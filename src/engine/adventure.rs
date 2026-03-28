//! Adventure state aggregating all game components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::abilities::{starting_abilities, Ability, SpellSlots};
use super::character::{Character, Class, Race, Stats};
use super::combat::CombatState;
use super::inventory::{Inventory, Item, ItemType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub location: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub name: String,
    pub description: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdventureState {
    pub id: String,
    pub name: String,
    pub character: Character,
    pub inventory: Inventory,
    pub abilities: Vec<Ability>,
    pub spell_slots: SpellSlots,
    pub combat: CombatState,
    pub current_scene: Scene,
    pub quest_log: Vec<Quest>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AdventureState {
    pub fn new(
        name: String,
        char_name: String,
        race: Race,
        class: Class,
        stats: Stats,
    ) -> Self {
        let abilities = starting_abilities(&class);
        let spell_slots = SpellSlots::for_class(&class, 1);

        let mut inventory = Inventory::new();

        // Starting equipment based on class
        match &class {
            Class::Warrior => {
                inventory.add(Item {
                    name: "Longsword".to_string(),
                    description: "A sturdy steel longsword.".to_string(),
                    item_type: ItemType::Weapon,
                    properties: serde_json::json!({"damage": "1d8+STR", "type": "slashing"}),
                    weight: 3.0,
                });
                inventory.add(Item {
                    name: "Chain Mail".to_string(),
                    description: "Heavy armor made of interlocking metal rings.".to_string(),
                    item_type: ItemType::Armor,
                    properties: serde_json::json!({"ac": 16}),
                    weight: 55.0,
                });
            }
            Class::Mage => {
                inventory.add(Item {
                    name: "Quarterstaff".to_string(),
                    description: "A wooden staff, useful for both walking and combat.".to_string(),
                    item_type: ItemType::Weapon,
                    properties: serde_json::json!({"damage": "1d6+STR", "type": "bludgeoning"}),
                    weight: 4.0,
                });
                inventory.add(Item {
                    name: "Spellbook".to_string(),
                    description: "A leather-bound tome containing your arcane knowledge.".to_string(),
                    item_type: ItemType::Misc,
                    properties: serde_json::json!({}),
                    weight: 3.0,
                });
            }
            Class::Rogue => {
                inventory.add(Item {
                    name: "Shortsword".to_string(),
                    description: "A light, quick blade favored by rogues.".to_string(),
                    item_type: ItemType::Weapon,
                    properties: serde_json::json!({"damage": "1d6+DEX", "type": "piercing", "finesse": true}),
                    weight: 2.0,
                });
                inventory.add(Item {
                    name: "Thieves' Tools".to_string(),
                    description: "A set of lockpicks, a small mirror, scissors, and a pair of pliers.".to_string(),
                    item_type: ItemType::Misc,
                    properties: serde_json::json!({}),
                    weight: 1.0,
                });
            }
            Class::Cleric => {
                inventory.add(Item {
                    name: "Mace".to_string(),
                    description: "A heavy mace blessed by your deity.".to_string(),
                    item_type: ItemType::Weapon,
                    properties: serde_json::json!({"damage": "1d6+STR", "type": "bludgeoning"}),
                    weight: 4.0,
                });
                inventory.add(Item {
                    name: "Shield".to_string(),
                    description: "A wooden shield emblazoned with a holy symbol.".to_string(),
                    item_type: ItemType::Armor,
                    properties: serde_json::json!({"ac_bonus": 2}),
                    weight: 6.0,
                });
            }
            Class::Ranger => {
                inventory.add(Item {
                    name: "Longbow".to_string(),
                    description: "A tall bow crafted from yew wood.".to_string(),
                    item_type: ItemType::Weapon,
                    properties: serde_json::json!({"damage": "1d8+DEX", "type": "piercing", "range": "150/600"}),
                    weight: 2.0,
                });
                inventory.add(Item {
                    name: "Shortsword".to_string(),
                    description: "A backup melee weapon.".to_string(),
                    item_type: ItemType::Weapon,
                    properties: serde_json::json!({"damage": "1d6+DEX", "type": "piercing", "finesse": true}),
                    weight: 2.0,
                });
            }
        }

        // Everyone starts with a health potion
        inventory.add(Item {
            name: "Health Potion".to_string(),
            description: "A vial of red liquid that restores 2d4+2 HP.".to_string(),
            item_type: ItemType::Potion,
            properties: serde_json::json!({"healing": "2d4+2"}),
            weight: 0.5,
        });

        let character = Character::new(char_name, race, class, stats);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            character,
            inventory,
            abilities,
            spell_slots,
            combat: CombatState::new(),
            current_scene: Scene {
                location: "Unknown".to_string(),
                description: "Your adventure is about to begin...".to_string(),
            },
            quest_log: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
