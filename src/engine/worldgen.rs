//! World map generator — produces a continent of 10,000+ counties with
//! smooth tier gradients, feature placement, and balance analysis.
//!
//! Uses a hex-grid layout with Perlin-like noise for organic terrain.
//! Race spawns placed at distant corners of the map.
//!
//! CLI: cargo run -- worldgen [--seed N] [--size N] [--analyze] [--map]

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use serde::{Serialize, Deserialize};
use std::fmt;

// ========================================================================
// CORE TYPES
// ========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HexCoord {
    pub q: i32, // column (axial coordinates)
    pub r: i32, // row
}

impl HexCoord {
    pub fn new(q: i32, r: i32) -> Self { Self { q, r } }

    /// Six neighbors in axial hex coordinates.
    pub fn neighbors(self) -> [HexCoord; 6] {
        [
            HexCoord::new(self.q + 1, self.r),
            HexCoord::new(self.q - 1, self.r),
            HexCoord::new(self.q, self.r + 1),
            HexCoord::new(self.q, self.r - 1),
            HexCoord::new(self.q + 1, self.r - 1),
            HexCoord::new(self.q - 1, self.r + 1),
        ]
    }

    /// Hex distance.
    pub fn distance(self, other: HexCoord) -> i32 {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = ((self.q + self.r) - (other.q + other.r)).abs();
        *[dq, dr, ds].iter().max().unwrap()
    }

    /// Convert to approximate (x, y) for rendering.
    pub fn to_pixel(self) -> (f64, f64) {
        let x = self.q as f64 + self.r as f64 * 0.5;
        let y = self.r as f64 * 0.866;
        (x, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    Plains,
    Forest,
    Hills,
    Mountains,
    Swamp,
    Desert,
    Tundra,
    Coast,
    Volcanic,
}

impl fmt::Display for Biome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plains => write!(f, "Plains"),
            Self::Forest => write!(f, "Forest"),
            Self::Hills => write!(f, "Hills"),
            Self::Mountains => write!(f, "Mountains"),
            Self::Swamp => write!(f, "Swamp"),
            Self::Desert => write!(f, "Desert"),
            Self::Tundra => write!(f, "Tundra"),
            Self::Coast => write!(f, "Coast"),
            Self::Volcanic => write!(f, "Volcanic"),
        }
    }
}

impl Biome {
    pub fn char(self) -> char {
        match self {
            Self::Plains => '.',
            Self::Forest => 'f',
            Self::Hills => 'h',
            Self::Mountains => 'M',
            Self::Swamp => '~',
            Self::Desert => 'd',
            Self::Tundra => '*',
            Self::Coast => ',',
            Self::Volcanic => 'V',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftingStationType {
    TanningRack,        // LW, max T3
    BasicForge,         // SM, max T3
    WoodworkingBench,   // WW, max T3
    Loom,               // TL, max T3
    HerbTable,          // AL, max T4
    EnchantingAltar,    // EN, max T5
    JewelersBench,      // JC, max T5
    MasterForge,        // SM/LW/WW/TL, max T7
    RunicCircle,        // RC, max T8
    ArtificersWorkshop, // AF, max T9
    SacredAltar,        // TH, max T10
    PrimordialForge,    // All skills, max T10
}

impl CraftingStationType {
    pub fn name(self) -> &'static str {
        match self {
            Self::TanningRack => "Tanning Rack",
            Self::BasicForge => "Basic Forge",
            Self::WoodworkingBench => "Woodworking Bench",
            Self::Loom => "Loom",
            Self::HerbTable => "Herb Table",
            Self::EnchantingAltar => "Enchanting Altar",
            Self::JewelersBench => "Jeweler's Bench",
            Self::MasterForge => "Master Forge",
            Self::RunicCircle => "Runic Circle",
            Self::ArtificersWorkshop => "Artificer's Workshop",
            Self::SacredAltar => "Sacred Altar",
            Self::PrimordialForge => "Primordial Forge",
        }
    }

    pub fn max_tier(self) -> u8 {
        match self {
            Self::TanningRack | Self::BasicForge | Self::WoodworkingBench | Self::Loom => 3,
            Self::HerbTable => 4,
            Self::EnchantingAltar | Self::JewelersBench => 5,
            Self::MasterForge => 7,
            Self::RunicCircle => 8,
            Self::ArtificersWorkshop => 9,
            Self::SacredAltar | Self::PrimordialForge => 10,
        }
    }

    /// Which crafting skills can use this station.
    pub fn supported_skills(self) -> Vec<&'static str> {
        match self {
            Self::TanningRack => vec!["leatherworking"],
            Self::BasicForge => vec!["smithing"],
            Self::WoodworkingBench => vec!["woodworking"],
            Self::Loom => vec!["tailoring"],
            Self::HerbTable => vec!["alchemy"],
            Self::EnchantingAltar => vec!["enchanting"],
            Self::JewelersBench => vec!["jewelcrafting"],
            Self::MasterForge => vec!["smithing", "leatherworking", "woodworking", "tailoring"],
            Self::RunicCircle => vec!["runecrafting"],
            Self::ArtificersWorkshop => vec!["artificing"],
            Self::SacredAltar => vec!["theurgy"],
            Self::PrimordialForge => vec!["leatherworking", "smithing", "woodworking", "alchemy",
                "enchanting", "tailoring", "jewelcrafting", "runecrafting", "artificing", "theurgy"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Race {
    Human,
    Dwarf,
    Elf,
    Orc,
    Halfling,
    Gnome,
    Dragonborn,
    Faefolk,
    Goblin,
    Revenant,
}

impl fmt::Display for Race {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Human => write!(f, "Human"),
            Self::Dwarf => write!(f, "Dwarf"),
            Self::Elf => write!(f, "Elf"),
            Self::Orc => write!(f, "Orc"),
            Self::Halfling => write!(f, "Halfling"),
            Self::Revenant => write!(f, "Revenant"),
            Self::Goblin => write!(f, "Goblin"),
            Self::Faefolk => write!(f, "Faefolk"),
            Self::Dragonborn => write!(f, "Dragonborn"),
            Self::Gnome => write!(f, "Gnome"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct County {
    pub coord: HexCoord,
    pub tier: f32,
    pub biome: Biome,
    pub name: String,
    pub has_town: bool,
    pub has_dungeon: bool,
    pub dungeon_tier: Option<f32>,
    pub has_tower: bool,
    pub tower_name: Option<String>,
    pub race_spawn: Option<Race>,
    pub has_exchange: bool,
    pub has_guild_hall: bool,
    pub region: String,
    /// Number of sub-locations in this county (4-8)
    pub location_count: u8,
    /// Crafting stations available in this county
    pub stations: Vec<CraftingStationType>,
}

// ========================================================================
// NOISE FUNCTIONS (simple value noise, no external crate needed)
// ========================================================================

struct NoiseGen {
    perm: [u8; 256],
}

impl NoiseGen {
    fn new(rng: &mut impl Rng) -> Self {
        let mut perm = [0u8; 256];
        for (i, p) in perm.iter_mut().enumerate() {
            *p = i as u8;
        }
        // Fisher-Yates shuffle
        for i in (1..256).rev() {
            let j = rng.gen_range(0..=i);
            perm.swap(i, j);
        }
        Self { perm }
    }

    fn hash2d(&self, x: i32, y: i32) -> f32 {
        let xi = (x & 255) as usize;
        let yi = (y & 255) as usize;
        let h = self.perm[(self.perm[xi] as usize + yi) & 255];
        h as f32 / 255.0
    }

    fn smooth_noise(&self, x: f32, y: f32) -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        // Smoothstep
        let sx = fx * fx * (3.0 - 2.0 * fx);
        let sy = fy * fy * (3.0 - 2.0 * fy);

        let n00 = self.hash2d(ix, iy);
        let n10 = self.hash2d(ix + 1, iy);
        let n01 = self.hash2d(ix, iy + 1);
        let n11 = self.hash2d(ix + 1, iy + 1);

        let nx0 = n00 + sx * (n10 - n00);
        let nx1 = n01 + sx * (n11 - n01);
        nx0 + sy * (nx1 - nx0)
    }

    /// Multi-octave noise (fbm).
    fn fbm(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_amp = 0.0;

        for _ in 0..octaves {
            value += self.smooth_noise(x * frequency, y * frequency) * amplitude;
            max_amp += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        value / max_amp
    }
}

// ========================================================================
// MAP GENERATION
// ========================================================================

pub struct WorldMap {
    pub counties: HashMap<HexCoord, County>,
    pub seed: u64,
    pub radius: i32,
}

/// Generate a hex-grid world map.
/// `radius` determines map size: total counties ≈ 3*radius^2 + 3*radius + 1
/// For ~10,000 counties, radius ≈ 57. For ~15,000, radius ≈ 70.
pub fn generate_world(seed: u64, radius: i32) -> WorldMap {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let noise = NoiseGen::new(&mut rng);
    let noise2 = NoiseGen::new(&mut rng);
    let noise3 = NoiseGen::new(&mut rng);

    let mut counties = HashMap::new();

    // --- Step 1: Generate hex grid with tiers ---

    // Race spawn positions (spread across map edges)
    let spawn_positions: Vec<(HexCoord, Race)> = vec![
        (HexCoord::new(0, -(radius * 3 / 4)), Race::Human),
        (HexCoord::new(-(radius / 2), -(radius / 2)), Race::Elf),
        (HexCoord::new(radius / 2, -(radius / 2)), Race::Halfling),
        (HexCoord::new(-(radius * 2 / 3), 0), Race::Gnome),
        (HexCoord::new(radius * 2 / 3, 0), Race::Faefolk),
        (HexCoord::new(-(radius / 2), radius / 2), Race::Dwarf),
        (HexCoord::new(radius / 2, radius / 2), Race::Orc),
        (HexCoord::new(0, radius * 2 / 3), Race::Dragonborn),
        (HexCoord::new(radius / 3, -(radius * 2 / 3)), Race::Goblin),
        (HexCoord::new(-(radius / 6), radius / 2), Race::Revenant),
    ];

    // Center of the map (high tier)
    let center = HexCoord::new(0, 0);

    for q in -radius..=radius {
        for r in -radius..=radius {
            let s = -q - r;
            if s.abs() > radius { continue; }

            let coord = HexCoord::new(q, r);

            // Base tier from distance to center (higher = further from edges)
            let dist_center = coord.distance(center) as f32;
            let max_dist = radius as f32;
            let center_factor = dist_center / max_dist; // 0 at center, 1 at edge

            // Distance to nearest spawn (spawns are low tier)
            let min_spawn_dist = spawn_positions.iter()
                .map(|(pos, _)| coord.distance(*pos) as f32)
                .fold(f32::MAX, f32::min);

            // Tier: higher near center, lower near spawns
            let noise_scale = 0.08;
            let (px, py) = coord.to_pixel();
            let noise_val = noise.fbm(px as f32 * noise_scale, py as f32 * noise_scale, 4);

            // Base tier: ramp up from spawns toward center
            let spawn_factor = (min_spawn_dist / (radius as f32 * 0.7)).min(1.0);
            let raw_tier = spawn_factor * 12.0 * center_factor.powf(0.6)
                + noise_val * 3.5
                - 0.5; // shift down so edges are low

            let tier = raw_tier.clamp(0.0, 10.0);

            // Biome from noise
            let biome_noise = noise2.fbm(px as f32 * 0.05, py as f32 * 0.05, 3);
            let elev_noise = noise3.fbm(px as f32 * 0.04, py as f32 * 0.04, 3);

            let biome = if tier > 8.5 && elev_noise > 0.5 {
                Biome::Volcanic
            } else if elev_noise > 0.75 {
                Biome::Mountains
            } else if elev_noise > 0.6 {
                Biome::Hills
            } else if tier > 7.0 && biome_noise < 0.3 {
                Biome::Tundra
            } else if biome_noise < 0.25 {
                Biome::Swamp
            } else if biome_noise < 0.45 {
                Biome::Forest
            } else if biome_noise > 0.8 && tier > 3.0 {
                Biome::Desert
            } else if coord.distance(center) as f32 > radius as f32 * 0.85 {
                Biome::Coast
            } else {
                Biome::Plains
            };

            let location_count = rng.gen_range(4..=8u8);

            counties.insert(coord, County {
                coord,
                tier,
                biome,
                name: String::new(), // filled later
                has_town: false,
                has_dungeon: false,
                dungeon_tier: None,
                has_tower: false,
                tower_name: None,
                race_spawn: None,
                has_exchange: false,
                has_guild_hall: false,
                region: String::new(),
                location_count,
                stations: Vec::new(),
            });
        }
    }

    // --- Step 2: Smooth tiers so neighbors differ by ≤ 0.5 ---
    for _ in 0..50 {
        let coords: Vec<HexCoord> = counties.keys().cloned().collect();
        for &coord in &coords {
            let current = counties[&coord].tier;
            let neighbor_tiers: Vec<f32> = coord.neighbors().iter()
                .filter_map(|n| counties.get(n).map(|c| c.tier))
                .collect();
            if neighbor_tiers.is_empty() { continue; }

            let avg: f32 = neighbor_tiers.iter().sum::<f32>() / neighbor_tiers.len() as f32;
            // Blend toward neighbors slightly
            let smoothed = current * 0.7 + avg * 0.3;
            counties.get_mut(&coord).unwrap().tier = smoothed.clamp(0.0, 10.0);
        }
    }

    // --- Step 3: Place race spawns ---
    for (spawn_pos, race) in &spawn_positions {
        // Find closest county to intended spawn position
        let closest = counties.keys()
            .min_by_key(|c| c.distance(*spawn_pos))
            .cloned()
            .unwrap();

        // Set spawn and force low tier in a radius
        let spawn_radius = 12;
        let spawn_coords: Vec<HexCoord> = counties.keys().cloned().collect();
        for &coord in &spawn_coords {
            let dist = coord.distance(closest) as f32;
            let r = spawn_radius as f32;
            // Influence extends to 2x spawn_radius with gradual falloff
            let influence_radius = r * 1.5;
            if dist <= influence_radius {
                let c = counties.get_mut(&coord).unwrap();
                // Smooth cubic falloff: 1.0 at center, 0.0 at influence_radius
                let t = (dist / influence_radius).min(1.0);
                let blend = 1.0 - t * t * (3.0 - 2.0 * t); // smoothstep
                // Target tier at this distance (gradual ramp)
                let target = dist * 0.15;
                // Blend between target and existing tier
                c.tier = c.tier * (1.0 - blend) + target * blend;
                c.tier = c.tier.max(0.0);
                if dist < 1.0 {
                    c.race_spawn = Some(*race);
                    c.tier = 0.0;
                    c.has_town = true;
                    c.has_guild_hall = true;
                }
            }
        }
    }

    // --- Step 4: Place towns ---
    let coords: Vec<HexCoord> = counties.keys().cloned().collect();
    for &coord in &coords {
        if counties[&coord].has_town { continue; }
        // Town probability increases with lower tier (civilized areas)
        let tier = counties[&coord].tier;
        let town_chance = if tier < 2.0 { 0.25 }
            else if tier < 4.0 { 0.15 }
            else if tier < 6.0 { 0.08 }
            else if tier < 8.0 { 0.04 }
            else { 0.02 };
        if rng.gen::<f32>() < town_chance {
            counties.get_mut(&coord).unwrap().has_town = true;
        }
    }

    // --- Step 5: Place dungeons ---
    for &coord in &coords {
        let tier = counties[&coord].tier;
        // Dungeon probability: moderate everywhere
        let dung_chance = 0.12 + tier * 0.02;
        if rng.gen::<f32>() < dung_chance.min(0.30) {
            // Dungeon tier: normally distributed around county tier ± 1.5
            let dung_offset: f32 = (rng.gen::<f32>() + rng.gen::<f32>() + rng.gen::<f32>() - 1.5) * 2.0;
            let dung_tier = (tier + 0.5 + dung_offset).clamp(0.0, 12.0);
            let c = counties.get_mut(&coord).unwrap();
            c.has_dungeon = true;
            c.dungeon_tier = Some(dung_tier);
        }
    }

    // --- Step 6: Place ~10 towers ---
    let tower_names = [
        "Tower of Dawn", "Ironspire", "The Thornkeep", "Tidecaller Spire",
        "Shadowpillar", "Dragonwatch", "The Nexus", "Frostspire",
        "The Abyss", "Primordial Spire",
    ];

    // Place towers spread across tier ranges
    let tower_targets: Vec<f32> = vec![2.0, 3.5, 4.5, 5.0, 5.5, 6.5, 7.0, 8.0, 8.5, 9.5];
    let mut placed_towers: Vec<HexCoord> = Vec::new();

    for (i, &target_tier) in tower_targets.iter().enumerate() {
        if i >= tower_names.len() { break; }
        // Find best county: close to target tier, far from other towers
        let best = counties.keys()
            .filter(|c| {
                let t = counties[c].tier;
                (t - target_tier).abs() < 1.5
                    && !counties[c].has_tower
                    && placed_towers.iter().all(|p| c.distance(*p) > radius / 4)
            })
            .min_by(|a, b| {
                let da = (counties[a].tier - target_tier).abs();
                let db = (counties[b].tier - target_tier).abs();
                da.partial_cmp(&db).unwrap()
            })
            .cloned();

        if let Some(coord) = best {
            let c = counties.get_mut(&coord).unwrap();
            c.has_tower = true;
            c.tower_name = Some(tower_names[i].to_string());
            placed_towers.push(coord);
        }
    }

    // --- Step 7: Place exchanges and guild halls at major towns ---
    let mut towns_by_tier: Vec<(HexCoord, f32)> = counties.iter()
        .filter(|(_, c)| c.has_town)
        .map(|(coord, c)| (*coord, c.tier))
        .collect();
    towns_by_tier.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Exchanges at every ~3 tiers
    for &target in &[1.5, 3.0, 5.0, 7.0, 9.0] {
        if let Some(&(coord, _)) = towns_by_tier.iter()
            .min_by(|(_, t1), (_, t2)| (t1 - target).abs().partial_cmp(&(t2 - target).abs()).unwrap())
        {
            counties.get_mut(&coord).unwrap().has_exchange = true;
            counties.get_mut(&coord).unwrap().has_guild_hall = true;
        }
    }

    // Guild halls at more locations
    for &(coord, tier) in &towns_by_tier {
        if tier > 2.0 && rng.gen::<f32>() < 0.15 {
            counties.get_mut(&coord).unwrap().has_guild_hall = true;
        }
    }


    // --- Step 7b: Place crafting stations at towns ---
    for &coord in &coords {
        let county = &counties[&coord];
        if !county.has_town { continue; }
        let tier = county.tier;
        let mut stations = Vec::new();

        // Basic stations at all towns
        if tier >= 0.0 { stations.push(CraftingStationType::TanningRack); }
        if tier >= 1.0 {
            stations.push(CraftingStationType::BasicForge);
            stations.push(CraftingStationType::WoodworkingBench);
            stations.push(CraftingStationType::Loom);
        }
        if tier >= 2.0 { stations.push(CraftingStationType::HerbTable); }

        // Advanced stations with probability
        if tier >= 3.0 && rng.gen::<f32>() < 0.30 {
            stations.push(CraftingStationType::EnchantingAltar);
        }
        if tier >= 3.0 && rng.gen::<f32>() < 0.30 {
            stations.push(CraftingStationType::JewelersBench);
        }
        if tier >= 4.0 && rng.gen::<f32>() < 0.20 {
            stations.push(CraftingStationType::MasterForge);
        }
        if tier >= 5.0 && rng.gen::<f32>() < 0.15 {
            stations.push(CraftingStationType::RunicCircle);
        }
        if tier >= 7.0 && rng.gen::<f32>() < 0.10 {
            stations.push(CraftingStationType::ArtificersWorkshop);
        }
        if tier >= 9.0 && rng.gen::<f32>() < 0.05 {
            stations.push(CraftingStationType::SacredAltar);
        }

        counties.get_mut(&coord).unwrap().stations = stations;
    }

    // Place 2 Primordial Forges near the T10 center
    let mut primordial_candidates: Vec<_> = counties.iter()
        .filter(|(_, c)| c.tier >= 9.0 && c.has_town)
        .map(|(coord, c)| (*coord, c.tier))
        .collect();
    primordial_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for &(coord, _) in primordial_candidates.iter().take(2) {
        counties.get_mut(&coord).unwrap().stations.push(CraftingStationType::PrimordialForge);
    }

    // --- Step 8: Assign region names based on position ---
    for coord in counties.keys().cloned().collect::<Vec<_>>() {
        let (px, py) = coord.to_pixel();
        let r = radius as f64;
        let region = if py < -r * 0.4f64 {
            if px < -r * 0.3f64 { "Western Heartlands" }
            else if px > r * 0.3f64 { "Eastern Heartlands" }
            else { "Central Heartlands" }
        } else if py < 0.0f64 {
            if px < -r * 0.3f64 { "Western Marches" }
            else if px > r * 0.3f64 { "Eastern Frontier" }
            else { "The Crossroads" }
        } else if py < r * 0.4f64 {
            if px < -r * 0.3f64 { "Darkwood" }
            else if px > r * 0.3f64 { "Iron Coast" }
            else { "The Highlands" }
        } else {
            if px < -r * 0.3f64 { "Blightlands" }
            else if px > r * 0.3f64 { "Dragonspine" }
            else { "Frozen Reaches" }
        };
        counties.get_mut(&coord).unwrap().region = region.to_string();
    }

    // --- Step 9: Generate names ---
    let prefixes = ["Green", "Stone", "Oak", "Iron", "Silver", "Dark", "Red", "White",
        "Black", "Gold", "Frost", "Thorn", "Raven", "Wolf", "Bear", "Storm",
        "Shadow", "Bright", "Ash", "Copper", "Moss", "Elm", "Birch", "Amber"];
    let suffixes = ["vale", "ford", "holm", "wick", "mere", "dale", "croft", "haven",
        "reach", "fell", "moor", "brook", "ridge", "wood", "field", "gate",
        "bury", "ton", "keep", "watch", "march", "cross", "end", "stead"];

    for coord in counties.keys().cloned().collect::<Vec<_>>() {
        let hash = ((coord.q.wrapping_mul(73856093) ^ coord.r.wrapping_mul(19349663)) as u64)
            .wrapping_add(seed);
        let p = (hash % prefixes.len() as u64) as usize;
        let s = ((hash / 31) % suffixes.len() as u64) as usize;
        counties.get_mut(&coord).unwrap().name = format!("{}{}", prefixes[p], suffixes[s]);
    }

    WorldMap { counties, seed, radius }
}

// ========================================================================
// ANALYSIS & METRICS
// ========================================================================

pub struct MapMetrics {
    pub total_counties: usize,
    pub avg_tier: f32,
    pub max_neighbor_delta: f32,
    pub avg_neighbor_delta: f32,
    pub tier_histogram: BTreeMap<u8, usize>,
    pub towns: usize,
    pub dungeons: usize,
    pub towers: usize,
    pub exchanges: usize,
    pub guild_halls: usize,
    pub race_spawns: Vec<(Race, HexCoord, f32)>,
    pub min_path_spawn_to_t5: HashMap<String, usize>,
    pub min_path_spawn_to_t10: HashMap<String, usize>,
    pub avg_locations_per_county: f32,
    pub biome_counts: HashMap<Biome, usize>,
    pub dungeon_tier_histogram: BTreeMap<u8, usize>,
    pub station_counts: HashMap<String, usize>,
}

pub fn analyze_map(world: &WorldMap) -> MapMetrics {
    let mut max_delta: f32 = 0.0;
    let mut total_delta: f32 = 0.0;
    let mut delta_count: usize = 0;

    for (coord, county) in &world.counties {
        for neighbor in coord.neighbors() {
            if let Some(n) = world.counties.get(&neighbor) {
                let delta = (county.tier - n.tier).abs();
                if delta > max_delta { max_delta = delta; }
                total_delta += delta;
                delta_count += 1;
            }
        }
    }

    let mut tier_hist: BTreeMap<u8, usize> = BTreeMap::new();
    let mut biome_counts: HashMap<Biome, usize> = HashMap::new();
    let mut dungeon_hist: BTreeMap<u8, usize> = BTreeMap::new();
    let mut total_tier: f32 = 0.0;
    let mut total_locs: u32 = 0;

    let mut station_counts: HashMap<String, usize> = HashMap::new();
    for county in world.counties.values() {
        *tier_hist.entry(county.tier.floor() as u8).or_default() += 1;
        *biome_counts.entry(county.biome).or_default() += 1;
        total_tier += county.tier;
        total_locs += county.location_count as u32;
        if let Some(dt) = county.dungeon_tier {
            *dungeon_hist.entry(dt.floor() as u8).or_default() += 1;
        }
        for station in &county.stations {
            *station_counts.entry(format!("{:?}", station)).or_default() += 1;
        }
    }

    let spawns: Vec<(Race, HexCoord, f32)> = world.counties.values()
        .filter_map(|c| c.race_spawn.map(|r| (r, c.coord, c.tier)))
        .collect();

    // BFS from each spawn to find minimum path to T5 and T10
    let mut min_t5: HashMap<String, usize> = HashMap::new();
    let mut min_t10: HashMap<String, usize> = HashMap::new();
    for (race, spawn, _) in &spawns {
        let (d5, d10) = bfs_to_tier(world, *spawn);
        min_t5.insert(race.to_string(), d5);
        min_t10.insert(race.to_string(), d10);
    }

    MapMetrics {
        total_counties: world.counties.len(),
        avg_tier: total_tier / world.counties.len() as f32,
        max_neighbor_delta: max_delta,
        avg_neighbor_delta: if delta_count > 0 { total_delta / delta_count as f32 } else { 0.0 },
        tier_histogram: tier_hist,
        towns: world.counties.values().filter(|c| c.has_town).count(),
        dungeons: world.counties.values().filter(|c| c.has_dungeon).count(),
        towers: world.counties.values().filter(|c| c.has_tower).count(),
        exchanges: world.counties.values().filter(|c| c.has_exchange).count(),
        guild_halls: world.counties.values().filter(|c| c.has_guild_hall).count(),
        race_spawns: spawns,
        min_path_spawn_to_t5: min_t5,
        min_path_spawn_to_t10: min_t10,
        avg_locations_per_county: total_locs as f32 / world.counties.len() as f32,
        biome_counts,
        dungeon_tier_histogram: dungeon_hist,
        station_counts,
    }
}

fn bfs_to_tier(world: &WorldMap, start: HexCoord) -> (usize, usize) {
    let mut visited: HashSet<HexCoord> = HashSet::new();
    let mut queue: VecDeque<(HexCoord, usize)> = VecDeque::new();
    queue.push_back((start, 0));
    visited.insert(start);

    let mut first_t5 = usize::MAX;
    let mut first_t10 = usize::MAX;

    while let Some((coord, dist)) = queue.pop_front() {
        if let Some(county) = world.counties.get(&coord) {
            if county.tier >= 5.0 && first_t5 == usize::MAX { first_t5 = dist; }
            if county.tier >= 9.5 && first_t10 == usize::MAX { first_t10 = dist; }
        }
        if first_t5 < usize::MAX && first_t10 < usize::MAX { break; }

        for neighbor in coord.neighbors() {
            if world.counties.contains_key(&neighbor) && !visited.contains(&neighbor) {
                visited.insert(neighbor);
                queue.push_back((neighbor, dist + 1));
            }
        }
    }

    (first_t5, first_t10)
}

// ========================================================================
// REPORT & ASCII MAP
// ========================================================================

pub fn metrics_report(metrics: &MapMetrics) -> String {
    let mut out = String::new();

    out.push_str("========================================================================\n");
    out.push_str("  WORLD MAP ANALYSIS\n");
    out.push_str("========================================================================\n\n");

    out.push_str(&format!("  Total counties: {}\n", metrics.total_counties));
    out.push_str(&format!("  Average tier: {:.2}\n", metrics.avg_tier));
    out.push_str(&format!("  Avg locations per county: {:.1}\n\n", metrics.avg_locations_per_county));

    out.push_str("  SMOOTHNESS:\n");
    out.push_str(&format!("    Max neighbor delta: {:.3} (target: ≤ 0.50)\n", metrics.max_neighbor_delta));
    out.push_str(&format!("    Avg neighbor delta: {:.3} (target: 0.10-0.25)\n\n", metrics.avg_neighbor_delta));

    out.push_str("  TIER DISTRIBUTION:\n");
    for (tier, count) in &metrics.tier_histogram {
        let pct = *count as f32 / metrics.total_counties as f32 * 100.0;
        let bar = "#".repeat((pct * 0.5) as usize);
        out.push_str(&format!("    T{}-{}: {:>5} ({:>4.1}%) {}\n", tier, tier + 1, count, pct, bar));
    }

    out.push_str("\n  FEATURES:\n");
    out.push_str(&format!("    Towns: {} ({:.1}%)\n", metrics.towns,
        metrics.towns as f32 / metrics.total_counties as f32 * 100.0));
    out.push_str(&format!("    Dungeons: {} ({:.1}%)\n", metrics.dungeons,
        metrics.dungeons as f32 / metrics.total_counties as f32 * 100.0));
    out.push_str(&format!("    Towers: {}\n", metrics.towers));
    out.push_str(&format!("    Exchanges: {}\n", metrics.exchanges));
    out.push_str(&format!("    Guild halls: {}\n\n", metrics.guild_halls));

    out.push_str("  RACE SPAWNS & DISTANCES:\n");
    for (race, coord, tier) in &metrics.race_spawns {
        let d5 = metrics.min_path_spawn_to_t5.get(&race.to_string()).unwrap_or(&0);
        let d10 = metrics.min_path_spawn_to_t10.get(&race.to_string()).unwrap_or(&0);
        out.push_str(&format!("    {}: ({},{}) tier={:.1}, {} counties to T5, {} to T10\n",
            race, coord.q, coord.r, tier, d5, d10));
    }

    // Distance between spawns
    out.push_str("\n  INTER-SPAWN DISTANCES:\n");
    let spawns = &metrics.race_spawns;
    for i in 0..spawns.len() {
        for j in (i+1)..spawns.len() {
            let dist = spawns[i].1.distance(spawns[j].1);
            out.push_str(&format!("    {} ↔ {}: {} counties\n",
                spawns[i].0, spawns[j].0, dist));
        }
    }

    out.push_str("\n  BIOME DISTRIBUTION:\n");
    let mut biomes: Vec<_> = metrics.biome_counts.iter().collect();
    biomes.sort_by(|a, b| b.1.cmp(a.1));
    for (biome, count) in biomes {
        let pct = *count as f32 / metrics.total_counties as f32 * 100.0;
        out.push_str(&format!("    {}: {} ({:.1}%)\n", biome, count, pct));
    }

    out.push_str("\n  CRAFTING STATION DISTRIBUTION:\n");
    let mut stations: Vec<_> = metrics.station_counts.iter().collect();
    stations.sort_by(|a, b| b.1.cmp(a.1));
    for (station_type, count) in stations {
        out.push_str(&format!("    {}: {} counties\n", station_type, count));
    }

    out.push_str("\n  DUNGEON TIER DISTRIBUTION:\n");
    for (tier, count) in &metrics.dungeon_tier_histogram {
        let bar = "#".repeat((*count as f32 / 10.0).ceil() as usize);
        out.push_str(&format!("    T{}-{}: {:>4} {}\n", tier, tier + 1, count, bar));
    }

    out
}

/// Render an ASCII map. Each cell shows tier as a digit (0-9) or letter (A=10).
/// Race spawns marked with first letter of race. Towers marked with T.
pub fn ascii_map(world: &WorldMap, width: usize, height: usize) -> String {
    // Find bounding box
    let min_q = world.counties.keys().map(|c| c.q).min().unwrap_or(0);
    let max_q = world.counties.keys().map(|c| c.q).max().unwrap_or(0);
    let min_r = world.counties.keys().map(|c| c.r).min().unwrap_or(0);
    let max_r = world.counties.keys().map(|c| c.r).max().unwrap_or(0);

    let mut grid = vec![vec![' '; width]; height];

    for (coord, county) in &world.counties {
        // Map hex coords to grid coords
        let nx = (coord.q - min_q) as f32 / (max_q - min_q).max(1) as f32;
        let ny = (coord.r - min_r) as f32 / (max_r - min_r).max(1) as f32;
        // Offset for hex staggering
        let px = nx + ny * 0.3;
        let py = ny;

        let gx = (px * (width - 1) as f32).round() as usize;
        let gy = (py * (height - 1) as f32).round() as usize;

        if gx < width && gy < height {
            let ch = if county.race_spawn.is_some() {
                match county.race_spawn.unwrap() {
                    Race::Human => 'H',
                    Race::Dwarf => 'D',
                    Race::Elf => 'E',
                    Race::Orc => 'O',
                    Race::Halfling => 'L',
                    Race::Revenant => 'R',
                    Race::Goblin => 'g',
                    Race::Faefolk => 'F',
                    Race::Dragonborn => 'B',
                    Race::Gnome => 'G',
                }
            } else if county.has_tower {
                'T'
            } else if county.has_exchange {
                '$'
            } else {
                let t = county.tier.round() as u8;
                if t <= 9 { (b'0' + t) as char } else { 'X' }
            };
            grid[gy][gx] = ch;
        }
    }

    let mut out = String::new();
    out.push_str("  Legend: 0-9=tier  H=Human D=Dwarf E=Elf O=Orc L=Halfling G=Gnome B=Dragonborn F=Faefolk g=Goblin R=Revenant T=Tower $=Exchange\n\n");
    for row in &grid {
        let line: String = row.iter().collect();
        out.push_str(&line.trim_end());
        out.push('\n');
    }
    out
}
