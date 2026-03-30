//! Adventure state aggregating all game components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::abilities::{starting_abilities, Ability, SpellSlots};
use super::character::{Character, Class, Race, Stats};
use super::combat::CombatState;
use super::dungeon::Dungeon;
use super::equipment::{get_item, Equipment};
use super::inventory::Inventory;
use super::skills::{self, SkillSet};
use super::world::{self, WorldMap};
use super::world_map::{PlayerPosition, DiscoveryState};

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
    #[serde(default)]
    pub equipment: Equipment,
    pub abilities: Vec<Ability>,
    pub spell_slots: SpellSlots,
    pub combat: CombatState,
    pub current_scene: Scene,
    pub quest_log: Vec<Quest>,
    #[serde(default)]
    pub dungeon: Option<Dungeon>,
    #[serde(default)]
    pub world: Option<WorldMap>,
    #[serde(default)]
    pub skills: SkillSet,
    #[serde(default)]
    pub world_position: PlayerPosition,
    #[serde(default)]
    pub discovery: DiscoveryState,
    #[serde(default)]
    pub murderer: bool,
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
        scenario: &Option<String>,
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
            // New classes get warrior-like starting equipment
            _ => {
                if let Some(item) = get_item("longsword") {
                    let _ = equipment.equip(item);
                }
                if let Some(item) = get_item("leather_armor") {
                    let _ = equipment.equip(item);
                }
            }
        }

        // Everyone starts with a health potion in their backpack
        if let Some(item) = get_item("health_potion") {
            inventory.add(item);
        }

        // Initialize skills based on class (before Character::new consumes class)
        let char_skills = skills::starting_skills(&class);

        let mut character = Character::new(char_name, race, class, stats);

        // Set AC from equipped gear
        character.ac = character.calculate_ac(&equipment);

        // Starting gold is set in Character::new (10 gp)
        // Also set it on inventory for display convenience
        inventory.gold = character.gold;

        // Create the world map based on scenario
        let world_map = world::create_world(scenario);

        // Initialize world position from race-specific hex spawn
        let world_position = PlayerPosition::from_coord(
            super::world_map::race_spawn_position(&format!("{}", character.race))
        );
        let mut discovery_state = DiscoveryState::default();
        discovery_state.discover(world_position.coord());

        // Set starting scene from hex world county
        let scene = if let Some(county) = super::world_map::current_county(&world_position) {
            Scene {
                location: county.name.clone(),
                description: format!("{} — {}", county.region, county.biome),
            }
        } else {
            let start_loc = &world_map.locations[world_map.current_location];
            Scene {
                location: start_loc.name.clone(),
                description: start_loc.description.clone(),
            }
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            character,
            inventory,
            equipment,
            abilities,
            spell_slots,
            combat: CombatState::new(),
            current_scene: scene,
            quest_log: Vec::new(),
            dungeon: None,
            world: Some(world_map),
            skills: char_skills,
            world_position,
            discovery: discovery_state,
            murderer: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
    pub fn new_with_background(
        name: String,
        char_name: String,
        race: Race,
        background: super::backgrounds::Background,
        scenario: &Option<String>,
    ) -> Self {
        let stats = Stats { strength: 10, dexterity: 10, constitution: 10, intelligence: 10, wisdom: 10, charisma: 10 };
        let con_mod = Stats::modifier(stats.constitution);
        let base_hp = 8 + con_mod;

        let mut character = Character {
            name: char_name,
            race,
            class: Class::Warrior, // Default, will be derived from skills
            level: 1,
            xp: 0,
            hp: base_hp,
            max_hp: base_hp,
            ac: 10,
            gold: background.starting_gold(),
            stats,
            conditions: Vec::new(),
            dead: false,
            background: Some(format!("{}", background)),
            murderer: false,
            kill_count: 0,
        };

        let mut inventory = Inventory::new();
        let mut equipment = Equipment::new();

        // Equip background starting items
        for item_id in background.starting_items() {
            if let Some(item) = get_item(item_id) {
                let _ = equipment.equip(item);
            }
        }

        character.ac = character.calculate_ac(&equipment);
        inventory.gold = character.gold;

        // Skills: all 44 at rank 0, then apply background
        let mut char_skills = skills::all_skills();
        skills::apply_background(&mut char_skills, &background);

        let abilities = Vec::new();
        let spell_slots = SpellSlots::default();

        // World
        let world_map = world::create_world(scenario);

        // Initialize world position from race-specific hex spawn
        let world_position = PlayerPosition::from_coord(
            super::world_map::race_spawn_position(&format!("{}", character.race))
        );
        let mut discovery_state = DiscoveryState::default();
        discovery_state.discover(world_position.coord());

        // Set starting scene from hex world county
        let scene = if let Some(county) = super::world_map::current_county(&world_position) {
            Scene {
                location: county.name.clone(),
                description: format!("{} — {}", county.region, county.biome),
            }
        } else {
            let start_loc = &world_map.locations[world_map.current_location];
            Scene {
                location: start_loc.name.clone(),
                description: start_loc.description.clone(),
            }
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            character,
            inventory,
            equipment,
            abilities,
            spell_slots,
            combat: CombatState::new(),
            current_scene: scene,
            quest_log: Vec::new(),
            dungeon: None,
            world: Some(world_map),
            skills: char_skills,
            world_position,
            discovery: discovery_state,
            murderer: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

}
