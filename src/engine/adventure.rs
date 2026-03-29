//! Adventure state aggregating all game components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::abilities::{starting_abilities, Ability, SpellSlots};
use super::character::{Character, Class, Race, Stats};
use super::combat::CombatState;
use super::equipment::{get_item, Equipment};
use super::inventory::Inventory;

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
    pub equipment: Equipment,
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
        let mut equipment = Equipment::new();

        // Starting equipment based on class — equip from database items
        match &class {
            Class::Warrior => {
                if let Some(item) = get_item("longsword") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("chain_mail") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("shield") {
                    let _ = equipment.equip(item);
                }
            }
            Class::Mage => {
                if let Some(item) = get_item("quarterstaff") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("leather_armor") {
                    let _ = equipment.equip(item);
                }
                // Spellbook goes in inventory (backpack)
                if let Some(item) = get_item("spellbook") {
                    inventory.add(item);
                }
            }
            Class::Rogue => {
                if let Some(item) = get_item("shortsword") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("studded_leather") {
                    let _ = equipment.equip(item);
                }
                // Thieves' tools in inventory
                if let Some(item) = get_item("thieves_tools") {
                    inventory.add(item);
                }
            }
            Class::Cleric => {
                if let Some(item) = get_item("mace") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("scale_mail") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("shield") {
                    let _ = equipment.equip(item);
                }
            }
            Class::Ranger => {
                if let Some(item) = get_item("longbow") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("chain_shirt") {
                    let _ = equipment.equip(item);
                }
                // Backup shortsword in inventory
                if let Some(item) = get_item("shortsword") {
                    inventory.add(item);
                }
            }
        }

        // Everyone starts with a health potion in their backpack
        if let Some(item) = get_item("health_potion") {
            inventory.add(item);
        }

        let mut character = Character::new(char_name, race, class, stats);

        // Set AC from equipped gear
        character.ac = character.calculate_ac(&equipment);

        // Starting gold is set in Character::new (10 gp)
        // Also set it on inventory for display convenience
        inventory.gold = character.gold;

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            character,
            inventory,
            equipment,
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
