//! Combat state tracking.

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatState {
    pub active: bool,
    pub enemies: Vec<Enemy>,
    pub initiative_order: Vec<String>,
    pub current_turn: usize,
    pub round: u32,
}

impl CombatState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, enemies: Vec<Enemy>) {
        self.active = true;
        self.enemies = enemies;
        self.initiative_order = Vec::new();
        self.current_turn = 0;
        self.round = 1;
    }

    pub fn end(&mut self) {
        self.active = false;
        self.enemies.clear();
        self.initiative_order.clear();
        self.current_turn = 0;
        self.round = 0;
    }

    pub fn find_enemy_mut(&mut self, name: &str) -> Option<&mut Enemy> {
        let name_lower = name.to_lowercase();
        self.enemies
            .iter_mut()
            .find(|e| e.name.to_lowercase() == name_lower)
    }
}
