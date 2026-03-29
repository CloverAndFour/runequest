//! BG3-inspired turn-based combat system with initiative, action economy, and deterministic enemy AI.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::character::{Character, Stats};
use super::dice::DiceRoller;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyAttack {
    pub name: String,
    pub damage_dice: String,
    pub damage_modifier: i32,
    pub to_hit_bonus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub attacks: Vec<EnemyAttack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CombatantId {
    Player,
    Enemy(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiativeEntry {
    pub combatant: CombatantId,
    pub name: String,
    pub roll: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionEconomy {
    pub actions: u32,
    pub bonus_actions: u32,
    pub movement_remaining: u32,
    pub reaction_available: bool,
}

impl ActionEconomy {
    pub fn new_turn() -> Self {
        Self {
            actions: 1,
            bonus_actions: 1,
            movement_remaining: 30,
            reaction_available: true,
        }
    }
}

/// Result of an enemy taking its turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyTurnResult {
    pub enemy_name: String,
    pub attack_name: String,
    pub attack_roll: i32,
    pub target_ac: i32,
    pub hit: bool,
    pub damage: i32,
    pub damage_type: String,
    pub player_hp_after: i32,
    pub player_max_hp: i32,
}

/// Result of a player combat action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatActionResult {
    pub action: String,
    pub success: bool,
    pub details: serde_json::Value,
}

/// Available combat action for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableAction {
    pub id: String,
    pub name: String,
    pub cost: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatState {
    pub active: bool,
    pub enemies: Vec<Enemy>,
    pub initiative: Vec<InitiativeEntry>,
    pub current_turn_index: usize,
    pub round: u32,
    pub action_economy: ActionEconomy,
    pub player_dodging: bool,
    pub combat_log: Vec<String>,
}

impl CombatState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start combat: populate enemies, roll initiative, sort order.
    pub fn start(&mut self, enemies: Vec<Enemy>, player_dex_mod: i32) {
        self.active = true;
        self.enemies = enemies;
        self.round = 1;
        self.player_dodging = false;
        self.combat_log.clear();

        // Roll initiative
        let mut rng = rand::thread_rng();
        let player_init = rng.gen_range(1..=20) + player_dex_mod;

        let mut entries = vec![InitiativeEntry {
            combatant: CombatantId::Player,
            name: "Player".to_string(),
            roll: player_init,
        }];

        for (i, enemy) in self.enemies.iter().enumerate() {
            let enemy_init = rng.gen_range(1..=20); // Enemies use flat d20 for simplicity
            entries.push(InitiativeEntry {
                combatant: CombatantId::Enemy(i),
                name: enemy.name.clone(),
                roll: enemy_init,
            });
        }

        // Sort descending by roll (higher goes first)
        entries.sort_by(|a, b| b.roll.cmp(&a.roll));
        self.initiative = entries;
        self.current_turn_index = 0;

        // Set up action economy for whoever goes first
        self.action_economy = ActionEconomy::new_turn();

        let first = self.initiative[0].name.clone();
        self.combat_log.push(format!("Round 1 begins. {} goes first.", first));
    }

    /// End combat entirely.
    pub fn end(&mut self) {
        self.active = false;
        self.enemies.clear();
        self.initiative.clear();
        self.current_turn_index = 0;
        self.round = 0;
        self.action_economy = ActionEconomy::default();
        self.player_dodging = false;
    }

    /// Get whose turn it currently is.
    pub fn current_combatant(&self) -> Option<&CombatantId> {
        self.initiative.get(self.current_turn_index).map(|e| &e.combatant)
    }

    pub fn current_combatant_name(&self) -> String {
        self.initiative.get(self.current_turn_index)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Is it the player's turn?
    pub fn is_player_turn(&self) -> bool {
        matches!(self.current_combatant(), Some(CombatantId::Player))
    }

    /// Advance to the next combatant's turn. Returns the new combatant.
    /// If we wrap around, increment the round.
    pub fn next_turn(&mut self) -> CombatantId {
        self.current_turn_index += 1;
        if self.current_turn_index >= self.initiative.len() {
            self.current_turn_index = 0;
            self.round += 1;
            self.combat_log.push(format!("Round {} begins.", self.round));
        }

        // Reset action economy for new turn
        self.action_economy = ActionEconomy::new_turn();
        self.player_dodging = false;

        // Remove dead enemies from initiative
        // (they stay in the enemies vec for reference but skip their turn)
        let combatant = self.initiative[self.current_turn_index].combatant.clone();
        if let CombatantId::Enemy(idx) = &combatant {
            if *idx < self.enemies.len() && self.enemies[*idx].hp <= 0 {
                // Dead enemy, skip to next
                return self.next_turn();
            }
        }

        combatant
    }

    /// Execute an enemy's turn using deterministic AI. Returns the result.
    pub fn execute_enemy_turn(&mut self, enemy_idx: usize, player: &mut Character) -> Option<EnemyTurnResult> {
        let enemy = self.enemies.get(enemy_idx)?;
        if enemy.hp <= 0 || enemy.attacks.is_empty() {
            return None;
        }

        // Pick best attack (highest to_hit_bonus)
        let best_attack = enemy.attacks.iter()
            .max_by_key(|a| a.to_hit_bonus)?
            .clone();

        // Roll to hit
        let mut rng = rand::thread_rng();
        let attack_roll_raw = rng.gen_range(1..=20);
        let attack_total = attack_roll_raw + best_attack.to_hit_bonus;

        // Check if player is dodging (disadvantage on attacks against them)
        let final_roll = if self.player_dodging {
            let second_roll = rng.gen_range(1..=20) + best_attack.to_hit_bonus;
            std::cmp::min(attack_total, second_roll) // Take lower
        } else {
            attack_total
        };

        let hit = final_roll >= player.ac;
        let damage = if hit {
            let dmg_result = DiceRoller::roll(&best_attack.damage_dice, 1, best_attack.damage_modifier);
            let dmg = std::cmp::max(dmg_result.total, 1);
            player.hp -= dmg;
            dmg
        } else {
            0
        };

        let enemy_name = enemy.name.clone();
        let result = EnemyTurnResult {
            enemy_name: enemy_name.clone(),
            attack_name: best_attack.name.clone(),
            attack_roll: final_roll,
            target_ac: player.ac,
            hit,
            damage,
            damage_type: "physical".to_string(),
            player_hp_after: player.hp,
            player_max_hp: player.max_hp,
        };

        if hit {
            self.combat_log.push(format!(
                "{} attacks with {} (rolled {} vs AC {}): HIT for {} damage!",
                enemy_name, best_attack.name, final_roll, player.ac, damage
            ));
        } else {
            self.combat_log.push(format!(
                "{} attacks with {} (rolled {} vs AC {}): MISS!",
                enemy_name, best_attack.name, final_roll, player.ac
            ));
        }

        Some(result)
    }

    /// Get available actions for the player based on current action economy and state.
    pub fn available_actions(&self, character: &Character, has_weapon: bool, has_potion: bool) -> Vec<AvailableAction> {
        let has_action = self.action_economy.actions > 0;
        let has_bonus = self.action_economy.bonus_actions > 0;

        let mut actions = vec![
            AvailableAction {
                id: "attack".to_string(),
                name: "Attack".to_string(),
                cost: "Action".to_string(),
                description: "Attack an enemy with your weapon".to_string(),
                enabled: has_action && has_weapon,
            },
            AvailableAction {
                id: "dodge".to_string(),
                name: "Dodge".to_string(),
                cost: "Action".to_string(),
                description: "Attacks against you have disadvantage until your next turn".to_string(),
                enabled: has_action,
            },
            AvailableAction {
                id: "dash".to_string(),
                name: "Dash".to_string(),
                cost: "Action".to_string(),
                description: "Double your movement this turn".to_string(),
                enabled: has_action,
            },
            AvailableAction {
                id: "use_item".to_string(),
                name: "Use Item".to_string(),
                cost: "Action".to_string(),
                description: "Use a potion or consumable item".to_string(),
                enabled: has_action && has_potion,
            },
        ];

        // Class-specific bonus actions
        match &character.class {
            super::character::Class::Warrior => {
                actions.push(AvailableAction {
                    id: "second_wind".to_string(),
                    name: "Second Wind".to_string(),
                    cost: "Bonus".to_string(),
                    description: format!("Heal 1d10+{} HP", character.level),
                    enabled: has_bonus && character.hp < character.max_hp,
                });
            }
            super::character::Class::Rogue => {
                actions.push(AvailableAction {
                    id: "cunning_hide".to_string(),
                    name: "Hide".to_string(),
                    cost: "Bonus".to_string(),
                    description: "Attempt to hide (grants advantage on next attack)".to_string(),
                    enabled: has_bonus,
                });
            }
            super::character::Class::Cleric => {
                actions.push(AvailableAction {
                    id: "healing_word".to_string(),
                    name: "Healing Word".to_string(),
                    cost: "Bonus".to_string(),
                    description: format!("Heal 1d4+{} HP (spell slot)", Stats::modifier(character.stats.wisdom).max(0)),
                    enabled: has_bonus && character.hp < character.max_hp,
                });
            }
            _ => {}
        }

        actions.push(AvailableAction {
            id: "end_turn".to_string(),
            name: "End Turn".to_string(),
            cost: "Free".to_string(),
            description: "End your turn and let the next combatant act".to_string(),
            enabled: true,
        });

        actions
    }

    /// Find an enemy by name (case-insensitive).
    pub fn find_enemy_mut(&mut self, name: &str) -> Option<&mut Enemy> {
        let name_lower = name.to_lowercase();
        self.enemies
            .iter_mut()
            .find(|e| e.name.to_lowercase() == name_lower)
    }

    /// Check if all enemies are dead.
    pub fn all_enemies_dead(&self) -> bool {
        self.enemies.iter().all(|e| e.hp <= 0)
    }

    /// Get living enemies count.
    pub fn living_enemies(&self) -> Vec<&Enemy> {
        self.enemies.iter().filter(|e| e.hp > 0).collect()
    }
}
