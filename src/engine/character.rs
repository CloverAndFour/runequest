//! Character stats and management.

use serde::{Deserialize, Serialize};

use super::equipment::Equipment;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Race {
    Human,
    Elf,
    Dwarf,
    Orc,
    Halfling,
}

impl std::fmt::Display for Race {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Race::Human => write!(f, "Human"),
            Race::Elf => write!(f, "Elf"),
            Race::Dwarf => write!(f, "Dwarf"),
            Race::Orc => write!(f, "Orc"),
            Race::Halfling => write!(f, "Halfling"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Class {
    Warrior,
    Mage,
    Rogue,
    Cleric,
    Ranger,
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Class::Warrior => write!(f, "Warrior"),
            Class::Mage => write!(f, "Mage"),
            Class::Rogue => write!(f, "Rogue"),
            Class::Cleric => write!(f, "Cleric"),
            Class::Ranger => write!(f, "Ranger"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub strength: u8,
    pub dexterity: u8,
    pub constitution: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub charisma: u8,
}

impl Stats {
    pub fn modifier(value: u8) -> i32 {
        (value as i32 - 10) / 2
    }

    pub fn get(&self, stat: &str) -> Option<u8> {
        match stat.to_lowercase().as_str() {
            "strength" | "str" => Some(self.strength),
            "dexterity" | "dex" => Some(self.dexterity),
            "constitution" | "con" => Some(self.constitution),
            "intelligence" | "int" => Some(self.intelligence),
            "wisdom" | "wis" => Some(self.wisdom),
            "charisma" | "cha" => Some(self.charisma),
            _ => None,
        }
    }

    pub fn modifier_for(&self, stat: &str) -> Option<i32> {
        self.get(stat).map(Self::modifier)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub race: Race,
    pub class: Class,
    pub level: u32,
    pub xp: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    #[serde(default)]
    pub gold: u32,
    pub stats: Stats,
    pub conditions: Vec<String>,
}

/// XP thresholds per level (index = level, value = XP needed for that level).
const XP_THRESHOLDS: &[u32] = &[
    0,     // Level 1
    300,   // Level 2
    900,   // Level 3
    2700,  // Level 4
    6500,  // Level 5
    14000, // Level 6
    23000, // Level 7
    34000, // Level 8
    48000, // Level 9
    64000, // Level 10
];

impl Character {
    pub fn new(name: String, race: Race, class: Class, stats: Stats) -> Self {
        let con_mod = Stats::modifier(stats.constitution);
        let base_hp = match class {
            Class::Warrior => 10 + con_mod,
            Class::Mage => 6 + con_mod,
            Class::Rogue => 8 + con_mod,
            Class::Cleric => 8 + con_mod,
            Class::Ranger => 10 + con_mod,
        };

        // AC starts at 10 (unarmored) — will be recalculated after equipping gear
        Self {
            name,
            race,
            class,
            level: 1,
            xp: 0,
            hp: base_hp,
            max_hp: base_hp,
            ac: 10,
            gold: 10, // Starting gold for all classes
            stats,
            conditions: Vec::new(),
        }
    }

    /// Calculate AC based on equipped gear and stats.
    ///
    /// Rules:
    /// - If chest armor has `ac_base`, that becomes the base AC.
    /// - Light armor (no special tag): base + full DEX modifier
    /// - Medium armor ("dex_cap_2"): base + min(DEX modifier, 2)
    /// - Heavy armor ("no_dex"): base only (no DEX)
    /// - No armor: 10 + DEX modifier
    /// - Add AC bonuses from all other equipment slots (shield, rings, cloaks, etc.)
    pub fn calculate_ac(&self, equipment: &Equipment) -> i32 {
        let dex_mod = Stats::modifier(self.stats.dexterity);

        let base_ac = if let Some(ref chest) = equipment.chest {
            if let Some(base) = chest.stats.ac_base {
                let special = chest.stats.special.as_deref().unwrap_or("");
                if special == "no_dex" {
                    base
                } else if special == "dex_cap_2" {
                    base + std::cmp::min(dex_mod, 2)
                } else {
                    // Light armor: full DEX
                    base + dex_mod
                }
            } else {
                // Armor with no ac_base (shouldn't happen for chest, but fallback)
                10 + dex_mod
            }
        } else {
            // Unarmored
            10 + dex_mod
        };

        base_ac + equipment.total_ac_bonus()
    }

    pub fn xp_for_next_level(&self) -> u32 {
        let next = self.level as usize;
        if next < XP_THRESHOLDS.len() {
            XP_THRESHOLDS[next]
        } else {
            u32::MAX
        }
    }

    pub fn check_level_up(&mut self) -> bool {
        if self.xp >= self.xp_for_next_level() && (self.level as usize) < XP_THRESHOLDS.len() {
            self.level += 1;
            let con_mod = Stats::modifier(self.stats.constitution);
            let hp_gain = match self.class {
                Class::Warrior => 6 + con_mod,
                Class::Mage => 4 + con_mod,
                Class::Rogue => 5 + con_mod,
                Class::Cleric => 5 + con_mod,
                Class::Ranger => 6 + con_mod,
            };
            let hp_gain = std::cmp::max(hp_gain, 1);
            self.max_hp += hp_gain;
            self.hp = self.max_hp; // Full heal on level up
            true
        } else {
            false
        }
    }

    pub fn proficiency_bonus(&self) -> i32 {
        match self.level {
            1..=4 => 2,
            5..=8 => 3,
            9..=10 => 4,
            _ => 4,
        }
    }
}
