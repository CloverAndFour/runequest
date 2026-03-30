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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum EnemyType {
    Brute,
    Skulker,
    Mystic,
    Undead,
}

impl std::fmt::Display for EnemyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnemyType::Brute => write!(f, "Brute"),
            EnemyType::Skulker => write!(f, "Skulker"),
            EnemyType::Mystic => write!(f, "Mystic"),
            EnemyType::Undead => write!(f, "Undead"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub attacks: Vec<EnemyAttack>,
    #[serde(default)]
    pub enemy_type: Option<EnemyType>,
    #[serde(default)]
    pub tier: Option<u8>,
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
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub enemies: Vec<Enemy>,
    #[serde(default, alias = "initiative_order")]
    pub initiative: Vec<InitiativeEntry>,
    #[serde(default, alias = "current_turn")]
    pub current_turn_index: usize,
    #[serde(default)]
    pub round: u32,
    #[serde(default)]
    pub action_economy: ActionEconomy,
    #[serde(default)]
    pub player_dodging: bool,
    #[serde(default)]
    pub flee_attempts: u32,
    #[serde(default)]
    pub combat_log: Vec<String>,
    /// If set, boss enrages after this many rounds (T5+ dungeons).
    #[serde(default)]
    pub enrage_round: Option<u32>,
    /// True once enrage has triggered.
    #[serde(default)]
    pub enrage_active: bool,
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
        self.flee_attempts = 0;
        self.enrage_active = false;
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
        self.enrage_round = None;
        self.enrage_active = false;
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
        // Bounded loop to skip dead enemies (prevents infinite recursion)
        for _ in 0..self.initiative.len() + 1 {
            self.current_turn_index += 1;
            if self.current_turn_index >= self.initiative.len() {
                self.current_turn_index = 0;
                self.round += 1;
                self.combat_log.push(format!("Round {} begins.", self.round));
            }

            self.action_economy = ActionEconomy::new_turn();
            self.player_dodging = false;

            let combatant = self.initiative[self.current_turn_index].combatant.clone();
            if let CombatantId::Enemy(idx) = &combatant {
                if *idx < self.enemies.len() && self.enemies[*idx].hp <= 0 {
                    continue; // Skip dead enemy
                }
            }
            return combatant;
        }
        // Fallback: all enemies dead, return to player
        CombatantId::Player
    }

    /// Execute an enemy's turn using deterministic AI. Returns the result.
    pub fn execute_enemy_turn(&mut self, enemy_idx: usize, player: &mut Character) -> Option<EnemyTurnResult> {
        let enemy = self.enemies.get(enemy_idx)?;
        if enemy.hp <= 0 {
            return None;
        }

        // Pick best attack (highest to_hit_bonus), or use a default Strike if none defined
        let best_attack = if enemy.attacks.is_empty() {
            EnemyAttack {
                name: "Strike".to_string(),
                damage_dice: "1d6".to_string(),
                damage_modifier: (enemy.max_hp / 10).max(0),
                to_hit_bonus: (enemy.ac - 10).max(2),
            }
        } else {
            enemy.attacks.iter()
                .max_by_key(|a| a.to_hit_bonus)
                .cloned()
                .unwrap_or(EnemyAttack {
                    name: "Strike".to_string(),
                    damage_dice: "1d6".to_string(),
                    damage_modifier: 0,
                    to_hit_bonus: 3,
                })
        };

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
            player.apply_damage(dmg);
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

        let flee_dc = (10 + self.living_enemies().len() as i32 * 2 - self.flee_attempts as i32 * 2).max(5);
        actions.push(AvailableAction {
            id: "flee".to_string(),
            name: "Flee".to_string(),
            cost: "Action".to_string(),
            description: format!("Attempt to escape combat (DC {})", flee_dc),
            enabled: has_action,
        });

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

    /// Check if enrage should trigger this round, and apply AoE damage if so.
    /// Returns Some(damage_per_player) if enrage fires this round, None otherwise.
    pub fn check_enrage(&mut self) -> Option<i32> {
        if let Some(enrage_at) = self.enrage_round {
            if self.round >= enrage_at && !self.enrage_active {
                self.enrage_active = true;
                self.combat_log.push(format!(
                    "*** ENRAGE! The boss enters a frenzy after round {}! ***",
                    enrage_at
                ));
            }
            if self.enrage_active {
                // Find the boss (highest HP enemy) and calculate AoE damage
                let boss_max_hp = self.enemies.iter()
                    .filter(|e| e.hp > 0)
                    .map(|e| e.max_hp)
                    .max()
                    .unwrap_or(0);
                let aoe_damage = (boss_max_hp as f32 * 0.25).round() as i32;
                if aoe_damage > 0 {
                    self.combat_log.push(format!(
                        "Enrage AoE: {} damage to all players in the room!",
                        aoe_damage
                    ));
                }
                return Some(aoe_damage);
            }
        }
        None
    }

    /// Set up an enrage timer for boss combat in tiered dungeons.
    /// enrage_round = max(8, 25 - tier * 2)
    pub fn set_enrage_timer(&mut self, tier: u32) {
        if tier >= 5 {
            let enrage_at = (25u32.saturating_sub(tier * 2)).max(8);
            self.enrage_round = Some(enrage_at);
            self.combat_log.push(format!(
                "The boss will enrage after round {}!",
                enrage_at
            ));
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::character::{Character, Class, Race, Stats};

    fn test_character() -> Character {
        Character {
            name: "TestHero".to_string(),
            race: Race::Human,
            class: Class::Warrior,
            level: 1,
            hp: 20,
            max_hp: 20,
            ac: 15,
            gold: 0,
            xp: 0,
            stats: Stats {
                strength: 14,
                dexterity: 12,
                constitution: 14,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
            },
            conditions: vec![],
            dead: false,
            background: None,
            kill_count: 0,
            murderer: false,
        }
    }

    fn enemy_with_attacks() -> Enemy {
        Enemy {
            name: "Goblin".to_string(),
            hp: 8,
            max_hp: 8,
            ac: 13,
            attacks: vec![EnemyAttack {
                name: "Scimitar".to_string(),
                damage_dice: "1d6".to_string(),
                damage_modifier: 1,
                to_hit_bonus: 4,
            }],
            enemy_type: None,
            tier: None,
        }
    }

    fn enemy_without_attacks() -> Enemy {
        Enemy {
            name: "Goblin Scout".to_string(),
            hp: 8,
            max_hp: 8,
            ac: 13,
            attacks: vec![],
            enemy_type: None,
            tier: None,
        }
    }

    #[test]
    fn test_enemy_with_attacks_can_fight() {
        let mut combat = CombatState::new();
        combat.start(vec![enemy_with_attacks()], 0);
        let mut player = test_character();
        let result = combat.execute_enemy_turn(0, &mut player);
        assert!(result.is_some(), "Enemy with attacks should return a turn result");
        let r = result.unwrap();
        assert_eq!(r.enemy_name, "Goblin");
        assert_eq!(r.attack_name, "Scimitar");
    }

    #[test]
    fn test_enemy_without_attacks_still_fights() {
        let mut combat = CombatState::new();
        combat.start(vec![enemy_without_attacks()], 0);
        let mut player = test_character();
        let result = combat.execute_enemy_turn(0, &mut player);
        assert!(result.is_some(), "Enemy without attacks should still fight using default Strike");
        let r = result.unwrap();
        assert_eq!(r.enemy_name, "Goblin Scout");
        assert_eq!(r.attack_name, "Strike");
    }

    #[test]
    fn test_dead_enemy_cannot_fight() {
        let mut combat = CombatState::new();
        let mut dead_enemy = enemy_with_attacks();
        dead_enemy.hp = 0;
        combat.start(vec![dead_enemy], 0);
        let mut player = test_character();
        let result = combat.execute_enemy_turn(0, &mut player);
        assert!(result.is_none(), "Dead enemy should not attack");
    }

    #[test]
    fn test_damage_dice_in_range() {
        let mut combat = CombatState::new();
        combat.start(vec![enemy_with_attacks()], 0);
        let mut max_damage = 0;
        for _ in 0..200 {
            combat.enemies[0].hp = 8;
            let mut player = test_character();
            if let Some(result) = combat.execute_enemy_turn(0, &mut player) {
                if result.hit {
                    max_damage = std::cmp::max(max_damage, result.damage);
                }
            }
        }
        // 1d6+1 max is 7; should never exceed that
        assert!(max_damage <= 7, "Max damage {} exceeded 1d6+1 max of 7", max_damage);
    }
}
