//! Class abilities and spell slots.

use serde::{Deserialize, Serialize};

use super::character::Class;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub name: String,
    pub description: String,
    pub uses_per_rest: Option<u32>,
    pub uses_remaining: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpellSlots {
    pub level_1: u32,
    pub level_1_used: u32,
    pub level_2: u32,
    pub level_2_used: u32,
    pub level_3: u32,
    pub level_3_used: u32,
}

impl SpellSlots {
    pub fn for_class(class: &Class, level: u32) -> Self {
        match class {
            Class::Mage => match level {
                1 => Self { level_1: 2, ..Default::default() },
                2 => Self { level_1: 3, ..Default::default() },
                3 => Self { level_1: 4, level_2: 2, ..Default::default() },
                4 => Self { level_1: 4, level_2: 3, ..Default::default() },
                5.. => Self { level_1: 4, level_2: 3, level_3: 2, ..Default::default() },
                _ => Self::default(),
            },
            Class::Cleric => match level {
                1 => Self { level_1: 2, ..Default::default() },
                2 => Self { level_1: 3, ..Default::default() },
                3 => Self { level_1: 4, level_2: 2, ..Default::default() },
                4 => Self { level_1: 4, level_2: 3, ..Default::default() },
                5.. => Self { level_1: 4, level_2: 3, level_3: 2, ..Default::default() },
                _ => Self::default(),
            },
            Class::Ranger => match level {
                2 => Self { level_1: 2, ..Default::default() },
                3..=4 => Self { level_1: 3, ..Default::default() },
                5.. => Self { level_1: 4, level_2: 2, ..Default::default() },
                _ => Self::default(),
            },
            _ => Self::default(), // Warrior and Rogue have no spell slots
        }
    }

    pub fn reset(&mut self) {
        self.level_1_used = 0;
        self.level_2_used = 0;
        self.level_3_used = 0;
    }
}

pub fn starting_abilities(class: &Class) -> Vec<Ability> {
    match class {
        Class::Warrior => vec![
            Ability {
                name: "Second Wind".to_string(),
                description: "Regain 1d10 + level HP as a bonus action.".to_string(),
                uses_per_rest: Some(1),
                uses_remaining: Some(1),
            },
            Ability {
                name: "Fighting Style: Great Weapon".to_string(),
                description: "Reroll 1s and 2s on damage dice with two-handed weapons.".to_string(),
                uses_per_rest: None,
                uses_remaining: None,
            },
        ],
        Class::Mage => vec![
            Ability {
                name: "Arcane Recovery".to_string(),
                description: "Recover spell slots during a short rest (half your level, rounded up).".to_string(),
                uses_per_rest: Some(1),
                uses_remaining: Some(1),
            },
            Ability {
                name: "Fire Bolt".to_string(),
                description: "Ranged spell attack, 1d10 fire damage (cantrip).".to_string(),
                uses_per_rest: None,
                uses_remaining: None,
            },
        ],
        Class::Rogue => vec![
            Ability {
                name: "Sneak Attack".to_string(),
                description: "Extra 1d6 damage when you have advantage or an ally is nearby.".to_string(),
                uses_per_rest: None,
                uses_remaining: None,
            },
            Ability {
                name: "Cunning Action".to_string(),
                description: "Dash, Disengage, or Hide as a bonus action.".to_string(),
                uses_per_rest: None,
                uses_remaining: None,
            },
        ],
        Class::Cleric => vec![
            Ability {
                name: "Channel Divinity: Turn Undead".to_string(),
                description: "Undead within 30ft must make WIS save or flee for 1 minute.".to_string(),
                uses_per_rest: Some(1),
                uses_remaining: Some(1),
            },
            Ability {
                name: "Sacred Flame".to_string(),
                description: "Target must succeed DEX save or take 1d8 radiant damage (cantrip).".to_string(),
                uses_per_rest: None,
                uses_remaining: None,
            },
        ],
        Class::Ranger => vec![
            Ability {
                name: "Favored Enemy".to_string(),
                description: "Advantage on survival checks to track, and INT checks to recall info about, your favored enemy.".to_string(),
                uses_per_rest: None,
                uses_remaining: None,
            },
            Ability {
                name: "Natural Explorer".to_string(),
                description: "Difficult terrain doesn't slow your group. Advantage on initiative.".to_string(),
                uses_per_rest: None,
                uses_remaining: None,
            },
        ],
        // New classes - use generic abilities for now
        _ => vec![],
    }
}
