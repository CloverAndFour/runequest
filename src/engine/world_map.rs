//! Global world map runtime bridge.
//!
//! The 251K-county world is generated ONCE at server start from a fixed seed.
//! Players store only their position (county coord) and discovered counties.

use std::collections::HashSet;
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use rand::Rng;

use super::worldgen::{self, HexCoord, County};
use super::combat::{Enemy, EnemyType};
use super::monsters::generate_monster;


/// Generate a descriptive hint string for a dungeon based on its tier.
fn dungeon_hint_for_tier(tier: u8) -> String {
    match tier {
        0 => "A shallow cave with vermin".to_string(),
        1 => "A dank cavern with basic monsters".to_string(),
        2 => "A dangerous dungeon with standard foes".to_string(),
        3 => "A veteran-level dungeon with tough enemies".to_string(),
        4 => "An elite dungeon with powerful guardians".to_string(),
        5..=6 => "A legendary dungeon pulsing with dark energy".to_string(),
        7..=8 => "A mythic dungeon of terrible power".to_string(),
        _ => "A primordial dungeon beyond mortal comprehension".to_string(),
    }
}

/// Fixed world seed. Same seed = same world on every server.
pub const WORLD_SEED: u64 = 42;
pub const WORLD_RADIUS: i32 = 289;

/// The global world map — generated once, shared by all players.
pub static WORLD: LazyLock<worldgen::WorldMap> = LazyLock::new(|| {
    worldgen::generate_world(WORLD_SEED, WORLD_RADIUS)
});

/// A player's position in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPosition {
    pub county_q: i32,
    pub county_r: i32,
    #[serde(default)]
    pub location_idx: u8,
}

impl PlayerPosition {
    pub fn coord(&self) -> HexCoord {
        HexCoord::new(self.county_q, self.county_r)
    }

    pub fn from_coord(coord: HexCoord) -> Self {
        Self { county_q: coord.q, county_r: coord.r, location_idx: 0 }
    }
}

impl Default for PlayerPosition {
    fn default() -> Self {
        // Default to Human spawn
        Self { county_q: 0, county_r: -(WORLD_RADIUS * 3 / 4), location_idx: 0 }
    }
}

/// Player's discovered areas.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryState {
    /// Counties the player has visited or scouted.
    pub discovered: HashSet<(i32, i32)>,
}

impl DiscoveryState {
    pub fn is_discovered(&self, coord: HexCoord) -> bool {
        self.discovered.contains(&(coord.q, coord.r))
    }

    pub fn discover(&mut self, coord: HexCoord) {
        self.discovered.insert((coord.q, coord.r));
        // Also discover all neighbors
        for n in coord.neighbors() {
            if WORLD.counties.contains_key(&n) {
                self.discovered.insert((n.q, n.r));
            }
        }
    }
}

// ========================================================================
// WORLD QUERY FUNCTIONS
// ========================================================================

/// Get county info at a coordinate.
pub fn get_county(coord: HexCoord) -> Option<&'static County> {
    WORLD.counties.get(&coord)
}

/// Get the current county for a player.
pub fn current_county(pos: &PlayerPosition) -> Option<&'static County> {
    get_county(pos.coord())
}

/// Get neighboring counties with basic info.
pub fn neighbors(coord: HexCoord) -> Vec<(HexCoord, &'static County)> {
    coord.neighbors().iter()
        .filter_map(|&n| WORLD.counties.get(&n).map(|c| (n, c)))
        .collect()
}

/// Named directions for hex movement.
pub fn direction_to_offset(direction: &str) -> Option<HexCoord> {
    match direction.to_lowercase().as_str() {
        "east" | "e" => Some(HexCoord::new(1, 0)),
        "west" | "w" => Some(HexCoord::new(-1, 0)),
        "northeast" | "ne" => Some(HexCoord::new(1, -1)),
        "northwest" | "nw" => Some(HexCoord::new(0, -1)),
        "southeast" | "se" => Some(HexCoord::new(0, 1)),
        "southwest" | "sw" => Some(HexCoord::new(-1, 1)),
        _ => None,
    }
}

/// Safe zone radius: counties within this distance from a race spawn
/// get reduced encounter rates.
const SAFE_ZONE_RADIUS: i32 = 5;
const INNER_SAFE_RADIUS: i32 = 2;

/// Compute the hex distance from a coordinate to the nearest race spawn point.
fn distance_to_nearest_spawn(target: &HexCoord) -> i32 {
    let mut min_dist = i32::MAX;
    for county in WORLD.counties.values() {
        if county.race_spawn.is_some() {
            let dist = hex_distance(target, &county.coord);
            if dist < min_dist {
                min_dist = dist;
            }
        }
    }
    min_dist
}

/// Hex distance between two axial coordinates.
fn hex_distance(a: &HexCoord, b: &HexCoord) -> i32 {
    let dq = (a.q - b.q).abs();
    let dr = (a.r - b.r).abs();
    let ds = ((a.q + a.r) - (b.q + b.r)).abs();
    *[dq, dr, ds].iter().max().unwrap()
}

/// Move to an adjacent county. Returns the new county or error.
pub fn travel(pos: &mut PlayerPosition, discovery: &mut DiscoveryState, direction: &str) -> Result<TravelResult, String> {
    let offset = direction_to_offset(direction)
        .ok_or_else(|| format!("Unknown direction '{}'. Use: east, west, northeast, northwest, southeast, southwest", direction))?;

    let current = pos.coord();
    let target = HexCoord::new(current.q + offset.q, current.r + offset.r);

    let county = WORLD.counties.get(&target)
        .ok_or("Cannot travel there — edge of the known world")?;

    pos.county_q = target.q;
    pos.county_r = target.r;
    pos.location_idx = 0;
    discovery.discover(target);

    // Roll for encounter based on county tier, adjusted by proximity to spawn
    let mut rng = rand::thread_rng();
    let spawn_dist = distance_to_nearest_spawn(&target);
    let (encounter_chance, encounter_tier) = if spawn_dist <= INNER_SAFE_RADIUS {
        // Inner safe zone (0-2 hexes from spawn): very rare, weakest enemies
        (0.02_f32, 0_u32)
    } else if spawn_dist <= SAFE_ZONE_RADIUS {
        // Outer safe zone (3-5 hexes): reduced rate, T0 enemies regardless of county tier
        let rate = 0.03 + (spawn_dist as f32 - INNER_SAFE_RADIUS as f32) * 0.01;
        (rate, 0)
    } else {
        // Wilderness: normal encounter rate based on county tier
        let rate = 0.05 + county.tier * 0.04;
        (rate, county.tier.round() as u32)
    };
    let encounter = if rng.gen::<f32>() < encounter_chance {
        let enemy_types = [EnemyType::Brute, EnemyType::Skulker, EnemyType::Mystic, EnemyType::Undead];
        let enemy_type = enemy_types[rng.gen_range(0..enemy_types.len())];
        let enemy = generate_monster(encounter_tier, enemy_type);
        Some(vec![enemy])
    } else {
        None
    };

    Ok(TravelResult {
        county_name: county.name.clone(),
        county_tier: county.tier,
        biome: format!("{}", county.biome),
        region: county.region.clone(),
        has_town: county.has_town,
        has_dungeon: county.has_dungeon,
        has_tower: county.has_tower,
        tower_name: county.tower_name.clone(),
        has_exchange: county.has_exchange,
        has_guild_hall: county.has_guild_hall,
        encounter,
    })
}

#[derive(Debug, Clone)]
pub struct TravelResult {
    pub county_name: String,
    pub county_tier: f32,
    pub biome: String,
    pub region: String,
    pub has_town: bool,
    pub has_dungeon: bool,
    pub has_tower: bool,
    pub tower_name: Option<String>,
    pub has_exchange: bool,
    pub has_guild_hall: bool,
    pub encounter: Option<Vec<Enemy>>,
}

/// Get map info for the current position and neighbors.
pub fn map_info(pos: &PlayerPosition, discovery: &DiscoveryState) -> serde_json::Value {
    let debug = false;
    let coord = pos.coord();
    let county = get_county(coord);

    let neighbors: Vec<serde_json::Value> = coord.neighbors().iter()
        .filter_map(|&n| {
            let c = WORLD.counties.get(&n)?;
            let discovered = discovery.is_discovered(n);
            Some(serde_json::json!({
                "direction": hex_direction_name(coord, n),
                "name": if discovered { c.name.clone() } else { "Unknown".to_string() },
                "tier_hint": if discovered { format!("{:.0}", c.tier) } else { "?".to_string() },
                "biome": if discovered { format!("{}", c.biome) } else { "?".to_string() },
                "has_town": if discovered { Some(c.has_town) } else { None },
                "discovered": discovered,
            }))
        })
        .collect();

    serde_json::json!({
        "position": { "q": coord.q, "r": coord.r },
        "county": county.map(|c| serde_json::json!({
            "name": c.name,
            "tier": if debug { Some(c.tier) } else { None },
            "biome": format!("{}", c.biome),
            "region": c.region,
            "has_town": c.has_town,
            "has_dungeon": c.has_dungeon,
            "dungeon_hint": if c.has_dungeon {
                Some(dungeon_hint_for_tier(c.dungeon_tier.unwrap_or(c.tier) as u8))
            } else {
                None
            },
            "has_tower": c.has_tower,
            "tower_name": c.tower_name,
            "has_exchange": c.has_exchange,
            "has_guild_hall": c.has_guild_hall,
            "locations": c.location_count,
        })),
        "neighbors": neighbors,
        "discovered_count": discovery.discovered.len(),
    })
}

pub fn hex_direction_name(from: HexCoord, to: HexCoord) -> &'static str {
    let dq = to.q - from.q;
    let dr = to.r - from.r;
    match (dq, dr) {
        (1, 0) => "East",
        (-1, 0) => "West",
        (1, -1) => "Northeast",
        (0, -1) => "Northwest",
        (0, 1) => "Southeast",
        (-1, 1) => "Southwest",
        _ => "?",
    }
}

/// Generate a shop inventory for a town based on its county tier.
/// Stocks consumables, crafting materials at/near tier, and a few pre-made equipment pieces.
pub fn generate_shop(county_tier: f32) -> Vec<ShopEntry> {
    let tier = county_tier.round() as u8;
    let graph = &*super::crafting::CRAFTING_GRAPH;
    let mut entries = Vec::new();

    // Always stock consumables
    entries.push(ShopEntry { item_id: "health_potion".into(), stock: None, price_mult: 1.0 });
    if tier >= 2 {
        entries.push(ShopEntry { item_id: "greater_health_potion".into(), stock: Some(3), price_mult: 1.0 });
    }

    // Stock crafting materials at and below county tier
    for mat in graph.materials.values() {
        if mat.tier <= tier && mat.tier >= tier.saturating_sub(1) {
            if matches!(mat.source, super::crafting::MaterialSource::Crafted) {
                // Sell intermediate crafting materials
                entries.push(ShopEntry {
                    item_id: mat.id.clone(),
                    stock: Some(if mat.tier == tier { 2 } else { 5 }),
                    price_mult: 1.0,
                });
            }
        }
    }

    // Stock a few pre-made equipment pieces at a markup
    let equipment_lines = ["blade", "bow", "dagger", "staff"];
    for line in &equipment_lines {
        let weapon_id = format!("{}_weapon_t{}", line, tier.min(5));
        let armor_id = format!("{}_armor_t{}", line, tier.min(5));
        if graph.materials.contains_key(&weapon_id) {
            entries.push(ShopEntry { item_id: weapon_id, stock: Some(1), price_mult: 3.0 });
        }
        if graph.materials.contains_key(&armor_id) {
            entries.push(ShopEntry { item_id: armor_id, stock: Some(1), price_mult: 3.0 });
        }
    }

    // Keep basic items for low-tier towns
    if tier <= 2 {
        let basic_pool: Vec<(&str, u8)> = vec![
            ("dagger", 1), ("shortsword", 1), ("leather_armor", 1), ("shield", 1),
            ("longsword", 2), ("chain_shirt", 2), ("chain_mail", 2), ("longbow", 2),
            ("rapier", 2), ("scale_mail", 2), ("studded_leather", 2),
        ];
        for (item_id, item_tier) in &basic_pool {
            if *item_tier <= tier + 1 {
                entries.push(ShopEntry {
                    item_id: item_id.to_string(),
                    stock: if *item_tier > tier { Some(1) } else { None },
                    price_mult: if *item_tier > tier { 1.5 } else { 1.0 },
                });
            }
        }
    }

    entries
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopEntry {
    pub item_id: String,
    pub stock: Option<u32>,
    pub price_mult: f32,
}

/// Find the spawn position for a given race.
pub fn race_spawn_position(race: &str) -> HexCoord {
    // Find the county with the matching race_spawn
    for county in WORLD.counties.values() {
        if let Some(ref spawn_race) = county.race_spawn {
            if format!("{}", spawn_race).to_lowercase() == race.to_lowercase() {
                return county.coord;
            }
        }
    }
    // Fallback to human spawn
    HexCoord::new(0, -(WORLD_RADIUS * 3 / 4))
}

/// Build a map view for the frontend: current county + 3 rings of neighbors.
pub fn build_map_view(pos: &PlayerPosition, discovery: &DiscoveryState, debug: bool) -> serde_json::Value {
    let center = pos.coord();
    let radius: i32 = 3;

    let mut hexes = Vec::new();

    // Collect all hexes within radius
    for dq in -radius..=radius {
        for dr in -radius..=radius {
            let ds = -dq - dr;
            if ds.abs() > radius {
                continue;
            }

            let coord = HexCoord::new(center.q + dq, center.r + dr);
            let is_current = dq == 0 && dr == 0;
            let discovered = discovery.is_discovered(coord);

            if let Some(county) = WORLD.counties.get(&coord) {
                hexes.push(serde_json::json!({
                    "q": dq,
                    "r": dr,
                    "abs_q": coord.q,
                    "abs_r": coord.r,
                    "current": is_current,
                    "discovered": discovered,
                    "name": if discovered { &county.name } else { "???" },
                    "tier": if debug && discovered { format!("{:.1}", county.tier) } else { "?".to_string() },
                    "biome": if discovered { format!("{}", county.biome) } else { "unknown".to_string() },
                    "has_town": if discovered { Some(county.has_town) } else { None },
                    "has_dungeon": if discovered { Some(county.has_dungeon) } else { None },
                    "has_tower": if discovered { Some(county.has_tower) } else { None },
                    "tower_name": if discovered { county.tower_name.as_deref() } else { None },
                    "has_exchange": if discovered { Some(county.has_exchange) } else { None },
                    "has_guild_hall": if discovered { Some(county.has_guild_hall) } else { None },
                    "region": if discovered { &county.region } else { "???" },
                }));
            }
        }
    }

    // Travel directions (immediate neighbors)
    let directions: Vec<serde_json::Value> = [
        ("East", 1, 0),
        ("West", -1, 0),
        ("Northeast", 1, -1),
        ("Northwest", 0, -1),
        ("Southeast", 0, 1),
        ("Southwest", -1, 1),
    ]
    .iter()
    .filter_map(|(name, dq, dr)| {
        let target = HexCoord::new(center.q + dq, center.r + dr);
        WORLD.counties.get(&target).map(|c| {
            let disc = discovery.is_discovered(target);
            serde_json::json!({
                "direction": name,
                "name": if disc { &c.name } else { "Unknown" },
                "tier": if debug && disc { format!("{:.1}", c.tier) } else { "?".to_string() },
                "biome": if disc { format!("{}", c.biome) } else { "?".to_string() },
                "has_town": if disc { Some(c.has_town) } else { None },
            })
        })
    })
    .collect();

    let current_county = WORLD.counties.get(&center);

    serde_json::json!({
        "hexes": hexes,
        "directions": directions,
        "current": current_county.map(|c| serde_json::json!({
            "name": c.name,
            "tier": if debug { Some(c.tier) } else { None },
            "biome": format!("{}", c.biome),
            "region": c.region,
            "has_town": c.has_town,
            "has_dungeon": c.has_dungeon,
            "has_tower": c.has_tower,
            "tower_name": c.tower_name,
            "has_exchange": c.has_exchange,
            "has_guild_hall": c.has_guild_hall,
            "stations": c.stations.iter().map(|s| serde_json::json!({
                "type": format!("{:?}", s),
                "name": s.name(),
                "max_tier": s.max_tier(),
                "skills": s.supported_skills(),
            })).collect::<Vec<_>>(),
        })),
        "position": { "q": center.q, "r": center.r },
    })
}
