//! World map system — locations, travel, shops, tower.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::dungeon::{generate_dungeon, Dungeon, EnemyTemplate, EnemyAttackTemplate};
use super::equipment::get_item;
use super::inventory::Inventory;

// === Data Structures ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMap {
    pub name: String,
    pub locations: Vec<WorldLocation>,
    pub connections: Vec<WorldConnection>,
    pub current_location: usize,
    #[serde(default)]
    pub game_mode: GameMode,
    #[serde(default)]
    pub dungeons: HashMap<usize, Dungeon>,
    #[serde(default)]
    pub tower: Option<Tower>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldLocation {
    pub id: usize,
    pub name: String,
    pub location_type: LocationType,
    pub x: f32,
    pub y: f32,
    pub description: String,
    #[serde(default)]
    pub discovered: bool,
    #[serde(default)]
    pub visited: bool,
    #[serde(default)]
    pub dungeon_seed: Option<DungeonSeedType>,
    #[serde(default)]
    pub dungeon_cleared: bool,
    #[serde(default)]
    pub shops: Vec<Shop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConnection {
    pub from: usize,
    pub to: usize,
    pub distance: u32,
    pub danger_level: u32,
    pub path_name: String,
    #[serde(default)]
    pub discovered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum GameMode {
    #[default]
    WorldMap,
    InTown { location_id: usize },
    InDungeon { location_id: usize },
    InTower { floor: u32 },
    Exploring { location_id: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LocationType {
    Town, Dungeon, Wilderness, Landmark, Camp, Tower,
}

impl Default for LocationType {
    fn default() -> Self { Self::Wilderness }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DungeonSeedType {
    Fixed(u64),
    Random(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shop {
    pub name: String,
    pub items: Vec<ShopItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopItem {
    pub item_id: String,
    pub stock: Option<u32>,
    pub price_multiplier: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tower {
    pub base_seed: u64,
    pub highest_floor_reached: u32,
    pub current_floor: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TravelResult {
    pub location_name: String,
    pub location_type: String,
    pub description: String,
    pub encounter: Option<TravelEncounter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TravelEncounter {
    pub encounter_type: String,
    pub description: String,
    pub enemies: Vec<EnemyTemplate>,
}

// === World Creation ===

pub fn create_world(scenario: &Option<String>) -> WorldMap {
    let mut rng = rand::thread_rng();

    let locations = vec![
        loc(0, "Crossroads Inn", LocationType::Town, 0.4, 0.5, "A bustling inn at the heart of all major trade routes.",
            None, vec![shop("General Store", &[("potion_healing",None,1.0),("leather_armor",None,1.0),("shortsword",None,1.0),("dagger",None,1.0)])]),
        loc(1, "Thornwall Village", LocationType::Town, 0.3, 0.4, "A quiet village nestled at the forest's edge.",
            None, vec![shop("Village Smith", &[("longsword",None,1.0),("chain_shirt",None,1.0),("shield",None,1.0),("potion_healing",None,1.0)])]),
        loc(2, "Dark Forest", LocationType::Wilderness, 0.25, 0.3, "Ancient trees block the sunlight.", None, vec![]),
        loc(3, "Frozen Crypts", LocationType::Dungeon, 0.2, 0.2, "Ice-covered burial chambers.", Some(DungeonSeedType::Random(rng.gen())), vec![]),
        loc(4, "Frosthold", LocationType::Town, 0.15, 0.15, "A fortified northern trading post.",
            None, vec![shop("Northern Outfitter", &[("battleaxe",None,1.0),("scale_mail",None,1.0),("potion_healing",None,1.0),("potion_greater_healing",None,1.2)])]),
        loc(5, "Mountain Pass", LocationType::Wilderness, 0.3, 0.1, "A treacherous path through the peaks.", None, vec![]),
        loc(6, "Dragon Peak", LocationType::Dungeon, 0.4, 0.05, "A volcanic mountain where a dragon lairs.", Some(DungeonSeedType::Fixed(1001)), vec![]),
        loc(7, "Ancient Ruins", LocationType::Dungeon, 0.55, 0.45, "Crumbling elven ruins.", Some(DungeonSeedType::Fixed(2002)), vec![]),
        loc(8, "Ravenmoor", LocationType::Town, 0.35, 0.6, "A fog-shrouded hamlet by the marshes.",
            None, vec![shop("Apothecary", &[("potion_healing",Some(5),0.9),("potion_greater_healing",None,0.9),("antidote",Some(3),0.9)])]),
        loc(9, "Haunted Manor", LocationType::Dungeon, 0.3, 0.7, "A cursed estate where the dead walk.", Some(DungeonSeedType::Fixed(3003)), vec![]),
        loc(10, "Marshlands", LocationType::Wilderness, 0.25, 0.75, "Treacherous bogs with poisonous mist.", None, vec![]),
        loc(11, "Port Blackwater", LocationType::Town, 0.65, 0.6, "A sprawling port city of merchants and thieves.",
            None, vec![shop("Grand Bazaar", &[("rapier",None,1.0),("studded_leather",None,1.0),("longbow",None,1.0),("chain_mail",None,1.0),("potion_healing",None,1.0),("ring_protection",Some(1),1.0)])]),
        loc(12, "Merchant Road", LocationType::Wilderness, 0.55, 0.55, "The main trade route, not always safe.", None, vec![]),
        loc(13, "Sea Caves", LocationType::Dungeon, 0.75, 0.65, "Tidal caves used by pirates.", Some(DungeonSeedType::Random(rng.gen())), vec![]),
        loc(14, "Sunken Temple", LocationType::Dungeon, 0.8, 0.75, "An underwater temple to a forgotten god.", Some(DungeonSeedType::Fixed(4004)), vec![]),
        loc(15, "The Endless Tower", LocationType::Tower, 0.45, 0.1, "A tower stretching beyond sight.", None, vec![]),
        loc(16, "Mountain Camp", LocationType::Camp, 0.35, 0.08, "A windswept camp for weary travelers.", None, vec![]),
        loc(17, "Forest Shrine", LocationType::Landmark, 0.15, 0.35, "An ancient shrine humming with magic.", None, vec![]),
        loc(18, "Bandit Hideout", LocationType::Dungeon, 0.2, 0.35, "A cave system used by robbers.", Some(DungeonSeedType::Random(rng.gen())), vec![]),
        loc(19, "Wizard's Retreat", LocationType::Landmark, 0.1, 0.1, "A secluded tower of an eccentric wizard.",
            None, vec![shop("Arcane Emporium", &[("ring_protection",Some(1),1.5),("cloak_protection",Some(1),1.5),("amulet_health",Some(1),1.5)])]),
    ];

    let connections = vec![
        conn(0,1,1,0,"Village Road"), conn(0,8,1,0,"South Road"), conn(0,7,2,1,"Ruins Trail"),
        conn(0,12,1,1,"Trade Road"), conn(1,2,2,1,"Forest Path"), conn(2,3,1,2,"Frozen Trail"),
        conn(3,4,2,2,"Northern Road"), conn(4,5,2,2,"Mountain Trail"), conn(5,6,1,3,"Dragon's Path"),
        conn(5,15,1,2,"Tower Road"), conn(5,16,1,1,"Camp Trail"), conn(8,9,1,2,"Manor Road"),
        conn(8,10,2,2,"Marsh Path"), conn(12,11,2,1,"Coast Road"), conn(11,13,1,2,"Cave Trail"),
        conn(13,14,1,3,"Sunken Path"), conn(2,17,1,1,"Shrine Trail"), conn(2,18,1,2,"Bandit Trail"),
        conn(4,19,1,1,"Wizard's Path"),
    ];

    let (start, pre) = scenario_start(scenario);
    let mut world = WorldMap {
        name: "The Realm of Eldara".to_string(),
        locations, connections, current_location: start,
        game_mode: GameMode::WorldMap, dungeons: HashMap::new(), tower: None,
    };

    for &id in &pre { if id < world.locations.len() { world.locations[id].discovered = true; } }
    if start < world.locations.len() { world.locations[start].discovered = true; world.locations[start].visited = true; }
    for c in &mut world.connections {
        if pre.contains(&c.from) && pre.contains(&c.to) { c.discovered = true; }
    }
    world
}

fn scenario_start(scenario: &Option<String>) -> (usize, Vec<usize>) {
    match scenario.as_deref().unwrap_or("") {
        s if s.contains("ruins") || s.contains("dungeon") || s.contains("lost") => (1, vec![0,1,2,7]),
        s if s.contains("dragon") => (4, vec![4,5,6,16]),
        s if s.contains("city") || s.contains("intrigue") => (11, vec![0,11,12,13]),
        s if s.contains("wilderness") || s.contains("survival") => (2, vec![2]),
        s if s.contains("haunted") || s.contains("manor") => (8, vec![0,8,9,10]),
        _ => (0, vec![0,1,8]),
    }
}

// === WorldMap Methods ===

impl WorldMap {
    pub fn current_loc(&self) -> &WorldLocation { &self.locations[self.current_location] }

    pub fn connections_from(&self, id: usize) -> Vec<(usize, &WorldConnection)> {
        self.connections.iter().filter_map(|c| {
            if c.from == id { Some((c.to, c)) } else if c.to == id { Some((c.from, c)) } else { None }
        }).collect()
    }

    pub fn reachable_names(&self) -> Vec<(String, String)> {
        self.connections_from(self.current_location).iter().map(|(to, c)| {
            (self.locations[*to].name.clone(), c.path_name.clone())
        }).collect()
    }

    pub fn find_location(&self, name: &str) -> Option<usize> {
        let lower = name.to_lowercase();
        self.locations.iter().position(|l| l.name.to_lowercase().contains(&lower))
    }

    pub fn travel_to(&mut self, target: usize) -> Result<TravelResult, String> {
        if target >= self.locations.len() { return Err("Invalid location".to_string()); }
        let danger = {
            let c = self.connections.iter_mut().find(|c|
                (c.from == self.current_location && c.to == target) || (c.to == self.current_location && c.from == target)
            ).ok_or("No path exists")?;
            c.discovered = true;
            c.danger_level
        };

        let mut rng = rand::thread_rng();
        let enc_chance = match danger { 0=>0, 1=>15, 2=>30, _=>50 };
        let encounter = if rng.gen_range(0..100) < enc_chance {
            Some(gen_encounter(danger, &mut rng))
        } else { None };

        self.current_location = target;
        self.locations[target].discovered = true;
        self.locations[target].visited = true;

        let connected: Vec<usize> = self.connections_from(target).iter().map(|(to,_)| *to).collect();
        for &id in &connected { if id < self.locations.len() { self.locations[id].discovered = true; } }
        for c in &mut self.connections {
            if (c.from == target || c.to == target) && (connected.contains(&c.from) || connected.contains(&c.to)) {
                c.discovered = true;
            }
        }

        self.game_mode = if self.locations[target].location_type == LocationType::Town {
            GameMode::InTown { location_id: target }
        } else { GameMode::WorldMap };

        let l = &self.locations[target];
        Ok(TravelResult { location_name: l.name.clone(), location_type: format!("{:?}", l.location_type), description: l.description.clone(), encounter })
    }

    pub fn enter_dungeon(&mut self, loc_id: usize) -> Result<(), String> {
        let loc = self.locations.get(loc_id).ok_or("Invalid location")?;
        if loc.location_type != LocationType::Dungeon { return Err("Not a dungeon".to_string()); }
        if !self.dungeons.contains_key(&loc_id) {
            let seed = match &loc.dungeon_seed {
                Some(DungeonSeedType::Fixed(s)) => *s,
                Some(DungeonSeedType::Random(s)) => *s,
                None => rand::random(),
            };
            self.dungeons.insert(loc_id, generate_dungeon(seed));
        }
        self.game_mode = GameMode::InDungeon { location_id: loc_id };
        Ok(())
    }

    pub fn exit_dungeon(&mut self) { self.game_mode = GameMode::WorldMap; }

    pub fn current_dungeon(&self) -> Option<&Dungeon> {
        if let GameMode::InDungeon { location_id } = &self.game_mode { self.dungeons.get(location_id) } else { None }
    }

    pub fn current_dungeon_mut(&mut self) -> Option<&mut Dungeon> {
        if let GameMode::InDungeon { location_id } = &self.game_mode { let id = *location_id; self.dungeons.get_mut(&id) } else { None }
    }

    pub fn enter_tower(&mut self) -> Result<u32, String> {
        if self.tower.is_none() {
            self.tower = Some(Tower { base_seed: rand::random(), highest_floor_reached: 0, current_floor: None });
        }
        let tower = self.tower.as_mut().unwrap();
        let floor = tower.current_floor.unwrap_or(1);
        tower.current_floor = Some(floor);
        let seed = tower.base_seed + floor as u64;
        self.dungeons.insert(1000 + floor as usize, generate_dungeon(seed));
        self.game_mode = GameMode::InTower { floor };
        Ok(floor)
    }

    pub fn tower_ascend(&mut self) -> Result<u32, String> {
        let tower = self.tower.as_mut().ok_or("No tower")?;
        let cur = tower.current_floor.ok_or("Not in tower")?;
        self.dungeons.remove(&(1000 + cur as usize));
        let next = cur + 1;
        tower.current_floor = Some(next);
        if next > tower.highest_floor_reached { tower.highest_floor_reached = next; }
        let seed = tower.base_seed + next as u64;
        self.dungeons.insert(1000 + next as usize, generate_dungeon(seed));
        self.game_mode = GameMode::InTower { floor: next };
        Ok(next)
    }

    pub fn exit_tower(&mut self) {
        if let Some(ref mut t) = self.tower {
            if let Some(f) = t.current_floor { self.dungeons.remove(&(1000 + f as usize)); }
            t.current_floor = None;
        }
        self.game_mode = GameMode::WorldMap;
    }

    pub fn tower_dungeon(&self) -> Option<&Dungeon> {
        if let GameMode::InTower { floor } = &self.game_mode { self.dungeons.get(&(1000 + *floor as usize)) } else { None }
    }

    pub fn tower_dungeon_mut(&mut self) -> Option<&mut Dungeon> {
        if let GameMode::InTower { floor } = &self.game_mode { let k = 1000 + *floor as usize; self.dungeons.get_mut(&k) } else { None }
    }

    pub fn buy_item(&mut self, loc_id: usize, item_id: &str, gold: &mut u32, inv: &mut Inventory) -> Result<String, String> {
        let shop = self.locations.get(loc_id).and_then(|l| l.shops.first()).ok_or("No shop")?;
        let si = shop.items.iter().find(|i| i.item_id == item_id).ok_or("Not in stock")?;
        if let Some(0) = si.stock { return Err("Out of stock".to_string()); }
        let item = get_item(item_id).ok_or("Unknown item")?;
        let price = (item.value_gp as f32 * si.price_multiplier) as u32;
        if *gold < price { return Err(format!("Need {} gp, have {}", price, gold)); }
        *gold -= price;
        let name = item.name.clone();
        inv.add(item);
        if let Some(s) = self.locations.get_mut(loc_id).and_then(|l| l.shops.first_mut()) {
            if let Some(si) = s.items.iter_mut().find(|i| i.item_id == item_id) {
                if let Some(ref mut st) = si.stock { *st = st.saturating_sub(1); }
            }
        }
        Ok(format!("Bought {} for {} gp", name, price))
    }

    pub fn sell_item(&self, item_name: &str, gold: &mut u32, inv: &mut Inventory) -> Result<String, String> {
        let item = inv.find(item_name).ok_or("Not in inventory")?;
        let price = item.value_gp / 2;
        let name = item.display_name();
        inv.remove(item_name).ok_or("Failed to remove")?;
        *gold += price;
        Ok(format!("Sold {} for {} gp", name, price))
    }
}

// === Helpers ===

fn loc(id: usize, name: &str, lt: LocationType, x: f32, y: f32, desc: &str, seed: Option<DungeonSeedType>, shops: Vec<Shop>) -> WorldLocation {
    WorldLocation { id, name: name.to_string(), location_type: lt, x, y, description: desc.to_string(), discovered: false, visited: false, dungeon_seed: seed, dungeon_cleared: false, shops }
}

fn conn(from: usize, to: usize, dist: u32, danger: u32, name: &str) -> WorldConnection {
    WorldConnection { from, to, distance: dist, danger_level: danger, path_name: name.to_string(), discovered: false }
}

fn shop(name: &str, items: &[(&str, Option<u32>, f32)]) -> Shop {
    Shop { name: name.to_string(), items: items.iter().map(|(id,stock,mult)| ShopItem { item_id: id.to_string(), stock: *stock, price_multiplier: *mult }).collect() }
}

fn gen_encounter(danger: u32, rng: &mut impl Rng) -> TravelEncounter {
    let mobs: Vec<(&str,i32,i32,&str,&str,i32,i32)> = match danger {
        1 => vec![("Wolf",11,13,"Bite","d6",1,4),("Bandit",11,12,"Shortsword","d6",2,4)],
        2 => vec![("Bandit Captain",20,15,"Longsword","d8",3,5),("Orc Raider",15,13,"Greataxe","d12",3,5)],
        _ => vec![("Troll",50,15,"Claw","2d6",4,7),("Wyvern",45,13,"Bite","2d8",4,7)],
    };
    let m = &mobs[rng.gen_range(0..mobs.len())];
    let enemy = EnemyTemplate { name: m.0.to_string(), hp: m.1, ac: m.2, attacks: vec![
        EnemyAttackTemplate { name: m.3.to_string(), damage_dice: m.4.to_string(), damage_modifier: m.5, to_hit_bonus: m.6 }
    ]};
    TravelEncounter { encounter_type: "combat".to_string(), description: format!("Ambushed by a {}!", m.0), enemies: vec![enemy] }
}

impl WorldMap {
    pub fn get_shop(&self, location_id: usize) -> Option<&Shop> {
        self.locations.get(location_id)?.shops.first()
    }

    pub fn current_location_info(&self) -> serde_json::Value {
        let loc = self.current_loc();
        let reachable = self.reachable_names();
        serde_json::json!({
            "name": loc.name,
            "type": format!("{:?}", loc.location_type),
            "description": loc.description,
            "destinations": reachable.iter().map(|(name, path)| format!("{} (via {})", name, path)).collect::<Vec<_>>(),
        })
    }

    pub fn reachable_locations(&self) -> Vec<(String, String)> {
        self.reachable_names()
    }

    pub fn format_shop_info(&self, location_id: usize) -> String {
        let shop = match self.locations.get(location_id).and_then(|l| l.shops.first()) {
            Some(s) => s,
            None => return "No shop here.".to_string(),
        };
        let mut info = format!("{}:\n", shop.name);
        for si in &shop.items {
            if let Some(item) = get_item(&si.item_id) {
                let price = (item.value_gp as f32 * si.price_multiplier) as u32;
                let stock = si.stock.map(|s| format!(" ({})", s)).unwrap_or_default();
                info.push_str(&format!("  - {} ({} gp){}\n", item.name, price, stock));
            }
        }
        info
    }

    pub fn tower_dungeon_key(&self) -> Option<usize> {
        match &self.game_mode {
            GameMode::InTower { floor } => Some(1000 + *floor as usize),
            _ => None,
        }
    }
}

impl Default for WorldMap {
    fn default() -> Self { create_world(&None) }
}
