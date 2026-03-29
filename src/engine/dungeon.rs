//! Seeded dungeon generator with deterministic room layouts, mob tables, traps, and treasure.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use super::combat::{Enemy, EnemyAttack};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dungeon {
    pub name: String,
    pub seed: u64,
    pub floors: Vec<Floor>,
    pub current_floor: usize,
    pub current_room: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Floor {
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub rooms: Vec<Room>,
    pub corridors: Vec<Corridor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Room {
    pub id: usize,
    pub name: String,
    pub room_type: RoomType,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub discovered: bool,
    pub visited: bool,
    pub cleared: bool,
    #[serde(default)]
    pub searched: bool,
    pub exits: Vec<Exit>,
    pub enemies: Vec<EnemyTemplate>,
    pub trap: Option<TrapTemplate>,
    pub treasure: RoomTreasure,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Corridor {
    pub from_room: usize,
    pub to_room: usize,
    pub cells: Vec<(u32, u32)>,
    pub discovered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Exit {
    pub direction: String,
    pub target_room: usize,
    pub target_floor: Option<usize>,
    pub locked: bool,
    pub key_item_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomType {
    Entrance,
    Combat,
    Trap,
    Treasure,
    Boss,
    Puzzle,
    Rest,
    Empty,
    Stairs,
}

impl Default for RoomType {
    fn default() -> Self {
        RoomType::Empty
    }
}

impl std::fmt::Display for RoomType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomType::Entrance => write!(f, "Entrance"),
            RoomType::Combat => write!(f, "Combat"),
            RoomType::Trap => write!(f, "Trap"),
            RoomType::Treasure => write!(f, "Treasure"),
            RoomType::Boss => write!(f, "Boss"),
            RoomType::Puzzle => write!(f, "Puzzle"),
            RoomType::Rest => write!(f, "Rest"),
            RoomType::Empty => write!(f, "Empty"),
            RoomType::Stairs => write!(f, "Stairs"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnemyTemplate {
    pub name: String,
    pub hp: i32,
    pub ac: i32,
    pub attacks: Vec<EnemyAttackTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnemyAttackTemplate {
    pub name: String,
    pub damage_dice: String,
    pub damage_modifier: i32,
    pub to_hit_bonus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrapTemplate {
    pub name: String,
    pub detection_dc: i32,
    pub save_stat: String,
    pub save_dc: i32,
    pub damage_dice: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoomTreasure {
    pub gold: u32,
    pub item_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// EnemyTemplate -> combat::Enemy conversion
// ---------------------------------------------------------------------------

impl EnemyTemplate {
    pub fn to_enemy(&self) -> Enemy {
        Enemy {
            name: self.name.clone(),
            hp: self.hp,
            max_hp: self.hp,
            ac: self.ac,
            attacks: self
                .attacks
                .iter()
                .map(|a| EnemyAttack {
                    name: a.name.clone(),
                    damage_dice: a.damage_dice.clone(),
                    damage_modifier: a.damage_modifier,
                    to_hit_bonus: a.to_hit_bonus,
                })
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Dungeon navigation helpers
// ---------------------------------------------------------------------------

impl Dungeon {
    pub fn current_room(&self) -> Option<&Room> {
        self.floors
            .get(self.current_floor)
            .and_then(|f| f.rooms.get(self.current_room))
    }

    pub fn current_room_mut(&mut self) -> Option<&mut Room> {
        let floor = self.current_floor;
        let room = self.current_room;
        self.floors
            .get_mut(floor)
            .and_then(|f| f.rooms.get_mut(room))
    }

    pub fn current_floor(&self) -> Option<&Floor> {
        self.floors.get(self.current_floor)
    }

    /// Move to a room via the given exit direction (case-insensitive).
    /// On success, marks the new room as discovered + visited and discovers the corridor.
    pub fn move_to_room(&mut self, direction: &str) -> Result<RoomMoveResult, String> {
        let dir_lower = direction.to_lowercase();

        let (target_room, target_floor, locked) = {
            let room = self
                .current_room()
                .ok_or_else(|| "No current room".to_string())?;
            let exit = room
                .exits
                .iter()
                .find(|e| e.direction.to_lowercase() == dir_lower)
                .ok_or_else(|| {
                    let available: Vec<&str> =
                        room.exits.iter().map(|e| e.direction.as_str()).collect();
                    format!(
                        "No exit '{}'. Available exits: {}",
                        direction,
                        available.join(", ")
                    )
                })?;
            (exit.target_room, exit.target_floor, exit.locked)
        };

        if locked {
            return Err("The door is locked.".to_string());
        }

        // Switch floor if needed
        let old_floor = self.current_floor;
        let old_room = self.current_room;
        if let Some(new_floor) = target_floor {
            self.current_floor = new_floor;
        }
        self.current_room = target_room;

        // Discover the corridor between old and new rooms
        let cf = self.current_floor;
        if old_floor == cf {
            if let Some(floor) = self.floors.get_mut(cf) {
                for corridor in &mut floor.corridors {
                    if (corridor.from_room == old_room && corridor.to_room == target_room)
                        || (corridor.from_room == target_room && corridor.to_room == old_room)
                    {
                        corridor.discovered = true;
                    }
                }
            }
        }

        // Mark new room as discovered + visited
        if let Some(room) = self.current_room_mut() {
            room.discovered = true;
            room.visited = true;
        }

        // Build the result
        let room = self.current_room().ok_or("Failed to access new room")?;
        Ok(RoomMoveResult {
            room_name: room.name.clone(),
            room_type: room.room_type.clone(),
            description: room.description.clone(),
            has_enemies: !room.enemies.is_empty() && !room.cleared,
            has_trap: room.trap.is_some() && !room.cleared,
            exits: room.exits.iter().map(|e| e.direction.clone()).collect(),
            floor: self.current_floor,
            room_id: self.current_room,
        })
    }

    /// Mark a specific room as discovered.
    pub fn discover_room(&mut self, floor: usize, room: usize) {
        if let Some(f) = self.floors.get_mut(floor) {
            if let Some(r) = f.rooms.get_mut(room) {
                r.discovered = true;
            }
        }
    }

    /// Unlock the boss door (remove locked flag from any exit with key_item_id "boss_key").
    pub fn unlock_boss_door(&mut self) {
        for floor in &mut self.floors {
            for room in &mut floor.rooms {
                for exit in &mut room.exits {
                    if exit.key_item_id.as_deref() == Some("boss_key") {
                        exit.locked = false;
                    }
                }
            }
        }
    }
}

/// Result of a successful room move.
#[derive(Debug, Clone)]
pub struct RoomMoveResult {
    pub room_name: String,
    pub room_type: RoomType,
    pub description: String,
    pub has_enemies: bool,
    pub has_trap: bool,
    pub exits: Vec<String>,
    pub floor: usize,
    pub room_id: usize,
}

// ---------------------------------------------------------------------------
// Name generation arrays
// ---------------------------------------------------------------------------

const PREFIXES: &[&str] = &[
    "Shadowed",
    "Forgotten",
    "Burning",
    "Cursed",
    "Silent",
    "Frozen",
    "Crimson",
    "Sunken",
    "Hollow",
    "Ancient",
    "Iron",
    "Bone",
    "Storm",
    "Emerald",
    "Obsidian",
];

const SUFFIXES: &[&str] = &[
    "Depths",
    "Catacombs",
    "Vaults",
    "Sanctum",
    "Labyrinth",
    "Crypts",
    "Halls",
    "Caverns",
    "Mines",
    "Tomb",
    "Fortress",
    "Spire",
    "Abyss",
    "Warren",
    "Citadel",
];

// ---------------------------------------------------------------------------
// Room name tables
// ---------------------------------------------------------------------------

const ENTRANCE_NAMES: &[&str] = &["Dungeon Entrance", "Entry Hall"];
const COMBAT_NAMES: &[&str] = &["Guard Post", "Patrol Room", "Lair", "Nest"];
const TRAP_NAMES: &[&str] = &["Trapped Corridor", "Rigged Chamber", "Danger Room"];
const TREASURE_NAMES: &[&str] = &["Treasury", "Vault", "Hoard Room", "Cache"];
const BOSS_NAMES: &[&str] = &["Boss Chamber", "Throne Room", "Dragon's Den"];
const REST_NAMES: &[&str] = &["Safe Haven", "Hidden Alcove", "Rest Chamber"];
const EMPTY_NAMES: &[&str] = &["Empty Chamber", "Dusty Room", "Abandoned Hall"];
const STAIRS_NAMES: &[&str] = &["Stairwell", "Descent", "Spiral Stairs"];
const PUZZLE_NAMES: &[&str] = &["Puzzle Room", "Riddle Chamber"];

fn room_name_for_type(rng: &mut StdRng, rt: &RoomType) -> String {
    let list = match rt {
        RoomType::Entrance => ENTRANCE_NAMES,
        RoomType::Combat => COMBAT_NAMES,
        RoomType::Trap => TRAP_NAMES,
        RoomType::Treasure => TREASURE_NAMES,
        RoomType::Boss => BOSS_NAMES,
        RoomType::Rest => REST_NAMES,
        RoomType::Empty => EMPTY_NAMES,
        RoomType::Stairs => STAIRS_NAMES,
        RoomType::Puzzle => PUZZLE_NAMES,
    };
    list.choose(rng).unwrap_or(&"Chamber").to_string()
}

// ---------------------------------------------------------------------------
// Room description pool
// ---------------------------------------------------------------------------

const ROOM_DESCRIPTIONS: &[&str] = &[
    "A damp stone room with moss-covered walls.",
    "Torch sconces line the walls, most long extinguished.",
    "The air here is thick with the smell of decay.",
    "Crumbling pillars support a vaulted ceiling overhead.",
    "Water drips steadily from cracks in the ceiling.",
    "Ancient runes glow faintly on the far wall.",
    "Cobwebs blanket every surface in this forgotten chamber.",
    "The floor is littered with bones and broken pottery.",
    "A cold draft whispers through unseen crevices.",
    "Faded tapestries hang in tatters along the walls.",
    "The stone floor is worn smooth by countless footsteps.",
    "A foul stench rises from a grated drain in the corner.",
    "Shadows dance at the edges of your torchlight.",
    "Iron chains dangle from rusted hooks in the ceiling.",
    "Claw marks score the walls at various heights.",
    "A thick layer of dust coats everything in this room.",
    "Strange mushrooms sprout from cracks in the masonry.",
    "The ceiling is low here, barely above head height.",
    "Soot stains the walls around a long-cold fireplace.",
    "An eerie silence fills this forgotten place.",
    "Broken furniture lies scattered across the floor.",
    "A shallow pool of dark water covers half the room.",
    "Crude drawings have been scratched into the walls.",
    "The air is warm and smells faintly of sulfur.",
];

fn room_description(rng: &mut StdRng) -> String {
    ROOM_DESCRIPTIONS
        .choose(rng)
        .unwrap_or(&"A nondescript chamber.")
        .to_string()
}

// ---------------------------------------------------------------------------
// Mob tables
// ---------------------------------------------------------------------------

fn floor1_mobs() -> Vec<EnemyTemplate> {
    vec![
        EnemyTemplate {
            name: "Giant Rat".into(),
            hp: 4,
            ac: 10,
            attacks: vec![EnemyAttackTemplate {
                name: "Bite".into(),
                damage_dice: "1d4".into(),
                damage_modifier: 0,
                to_hit_bonus: 2,
            }],
        },
        EnemyTemplate {
            name: "Goblin".into(),
            hp: 7,
            ac: 13,
            attacks: vec![EnemyAttackTemplate {
                name: "Scimitar".into(),
                damage_dice: "1d6".into(),
                damage_modifier: 1,
                to_hit_bonus: 4,
            }],
        },
        EnemyTemplate {
            name: "Skeleton".into(),
            hp: 13,
            ac: 13,
            attacks: vec![EnemyAttackTemplate {
                name: "Shortsword".into(),
                damage_dice: "1d6".into(),
                damage_modifier: 2,
                to_hit_bonus: 4,
            }],
        },
        EnemyTemplate {
            name: "Kobold".into(),
            hp: 5,
            ac: 12,
            attacks: vec![EnemyAttackTemplate {
                name: "Dagger".into(),
                damage_dice: "1d4".into(),
                damage_modifier: 2,
                to_hit_bonus: 4,
            }],
        },
    ]
}

fn floor2_mobs() -> Vec<EnemyTemplate> {
    vec![
        EnemyTemplate {
            name: "Orc".into(),
            hp: 15,
            ac: 13,
            attacks: vec![EnemyAttackTemplate {
                name: "Greataxe".into(),
                damage_dice: "1d12".into(),
                damage_modifier: 3,
                to_hit_bonus: 5,
            }],
        },
        EnemyTemplate {
            name: "Hobgoblin".into(),
            hp: 11,
            ac: 18,
            attacks: vec![EnemyAttackTemplate {
                name: "Longsword".into(),
                damage_dice: "1d8".into(),
                damage_modifier: 1,
                to_hit_bonus: 3,
            }],
        },
        EnemyTemplate {
            name: "Shadow".into(),
            hp: 16,
            ac: 12,
            attacks: vec![EnemyAttackTemplate {
                name: "Life Drain".into(),
                damage_dice: "2d6".into(),
                damage_modifier: 2,
                to_hit_bonus: 4,
            }],
        },
        EnemyTemplate {
            name: "Bugbear".into(),
            hp: 27,
            ac: 16,
            attacks: vec![EnemyAttackTemplate {
                name: "Morningstar".into(),
                damage_dice: "2d8".into(),
                damage_modifier: 2,
                to_hit_bonus: 4,
            }],
        },
    ]
}

fn floor3_mobs() -> Vec<EnemyTemplate> {
    vec![
        EnemyTemplate {
            name: "Ogre".into(),
            hp: 59,
            ac: 11,
            attacks: vec![EnemyAttackTemplate {
                name: "Greatclub".into(),
                damage_dice: "2d8".into(),
                damage_modifier: 4,
                to_hit_bonus: 6,
            }],
        },
        EnemyTemplate {
            name: "Wraith".into(),
            hp: 67,
            ac: 13,
            attacks: vec![EnemyAttackTemplate {
                name: "Life Drain".into(),
                damage_dice: "3d6".into(),
                damage_modifier: 3,
                to_hit_bonus: 6,
            }],
        },
    ]
}

fn boss_mobs() -> Vec<EnemyTemplate> {
    vec![
        EnemyTemplate {
            name: "Troll".into(),
            hp: 84,
            ac: 15,
            attacks: vec![EnemyAttackTemplate {
                name: "Claw".into(),
                damage_dice: "2d6".into(),
                damage_modifier: 4,
                to_hit_bonus: 7,
            }],
        },
        EnemyTemplate {
            name: "Young Dragon".into(),
            hp: 75,
            ac: 17,
            attacks: vec![EnemyAttackTemplate {
                name: "Bite".into(),
                damage_dice: "2d10".into(),
                damage_modifier: 4,
                to_hit_bonus: 8,
            }],
        },
    ]
}

fn mobs_for_floor(level: u32) -> Vec<EnemyTemplate> {
    match level {
        1 => floor1_mobs(),
        2 => floor2_mobs(),
        _ => floor3_mobs(),
    }
}

// ---------------------------------------------------------------------------
// Trap tables
// ---------------------------------------------------------------------------

fn all_traps() -> Vec<TrapTemplate> {
    vec![
        TrapTemplate {
            name: "Pit Trap".into(),
            detection_dc: 10,
            save_stat: "dex".into(),
            save_dc: 12,
            damage_dice: "1d6".into(),
            condition: None,
        },
        TrapTemplate {
            name: "Poison Dart".into(),
            detection_dc: 12,
            save_stat: "con".into(),
            save_dc: 13,
            damage_dice: "1d4".into(),
            condition: Some("Poisoned".into()),
        },
        TrapTemplate {
            name: "Flame Jet".into(),
            detection_dc: 14,
            save_stat: "dex".into(),
            save_dc: 14,
            damage_dice: "2d6".into(),
            condition: Some("Burning".into()),
        },
        TrapTemplate {
            name: "Blade Wall".into(),
            detection_dc: 15,
            save_stat: "dex".into(),
            save_dc: 15,
            damage_dice: "2d8".into(),
            condition: Some("Bleeding".into()),
        },
        TrapTemplate {
            name: "Poison Gas".into(),
            detection_dc: 16,
            save_stat: "con".into(),
            save_dc: 16,
            damage_dice: "3d6".into(),
            condition: Some("Poisoned".into()),
        },
    ]
}

fn traps_for_floor(level: u32) -> Vec<TrapTemplate> {
    let all = all_traps();
    match level {
        1 => all[0..2].to_vec(),
        2 => all[1..3].to_vec(),
        _ => all[3..5].to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Treasure tables
// ---------------------------------------------------------------------------

fn treasure_for_floor(rng: &mut StdRng, level: u32) -> RoomTreasure {
    match level {
        1 => {
            let gold = rng.gen_range(5..=25);
            let items: Vec<&str> = vec!["health_potion", "dagger", "leather_armor"];
            let item = items.choose(rng).unwrap_or(&"health_potion");
            RoomTreasure {
                gold,
                item_ids: vec![item.to_string()],
            }
        }
        2 => {
            let gold = rng.gen_range(25..=75);
            let items: Vec<&str> = vec![
                "health_potion",
                "chain_shirt",
                "rapier",
                "greater_health_potion",
            ];
            let item = items.choose(rng).unwrap_or(&"health_potion");
            RoomTreasure {
                gold,
                item_ids: vec![item.to_string()],
            }
        }
        _ => {
            let gold = rng.gen_range(75..=200);
            let items: Vec<&str> = vec![
                "half_plate",
                "ring_of_protection",
                "greater_health_potion",
            ];
            let item = items.choose(rng).unwrap_or(&"greater_health_potion");
            RoomTreasure {
                gold,
                item_ids: vec![item.to_string()],
            }
        }
    }
}

fn boss_treasure(rng: &mut StdRng) -> RoomTreasure {
    let gold = rng.gen_range(100..=300);
    let items: Vec<&str> = vec![
        "plate_armor",
        "cloak_of_protection",
        "greater_health_potion",
    ];
    let item = items.choose(rng).unwrap_or(&"greater_health_potion");
    RoomTreasure {
        gold,
        item_ids: vec![item.to_string()],
    }
}

// ---------------------------------------------------------------------------
// Room type weighted selection
// ---------------------------------------------------------------------------

fn random_room_type(rng: &mut StdRng) -> RoomType {
    // Combat: 30%, Empty: 25%, Trap: 15%, Treasure: 15%, Rest: 10%, Puzzle: 5%
    let roll = rng.gen_range(0..100);
    if roll < 30 {
        RoomType::Combat
    } else if roll < 55 {
        RoomType::Empty
    } else if roll < 70 {
        RoomType::Trap
    } else if roll < 85 {
        RoomType::Treasure
    } else if roll < 95 {
        RoomType::Rest
    } else {
        RoomType::Puzzle
    }
}

// ---------------------------------------------------------------------------
// Direction helpers
// ---------------------------------------------------------------------------

fn direction_from_to(ax: u32, ay: u32, aw: u32, ah: u32, bx: u32, by: u32, bw: u32, bh: u32) -> String {
    let acx = ax as f64 + aw as f64 / 2.0;
    let acy = ay as f64 + ah as f64 / 2.0;
    let bcx = bx as f64 + bw as f64 / 2.0;
    let bcy = by as f64 + bh as f64 / 2.0;

    let dx = bcx - acx;
    let dy = bcy - acy;

    if dx.abs() >= dy.abs() {
        if dx >= 0.0 {
            "East".to_string()
        } else {
            "West".to_string()
        }
    } else if dy >= 0.0 {
        "South".to_string()
    } else {
        "North".to_string()
    }
}

/// Make a direction unique among existing exits for this room.
fn unique_direction(existing: &[Exit], base: &str) -> String {
    if !existing.iter().any(|e| e.direction == base) {
        return base.to_string();
    }
    // Append a numeric suffix
    for i in 2..=20 {
        let candidate = format!("{} {}", base, i);
        if !existing.iter().any(|e| e.direction == candidate) {
            return candidate;
        }
    }
    base.to_string()
}

// ---------------------------------------------------------------------------
// Euclidean distance (room centers)
// ---------------------------------------------------------------------------

fn room_center(r: &Room) -> (f64, f64) {
    (r.x as f64 + r.w as f64 / 2.0, r.y as f64 + r.h as f64 / 2.0)
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

// ---------------------------------------------------------------------------
// Prim's MST for corridor connections
// ---------------------------------------------------------------------------

fn prims_mst(rooms: &[Room]) -> Vec<(usize, usize)> {
    let n = rooms.len();
    if n <= 1 {
        return vec![];
    }

    let centers: Vec<(f64, f64)> = rooms.iter().map(|r| room_center(r)).collect();
    let mut in_tree = vec![false; n];
    let mut edges = Vec::new();
    in_tree[0] = true;

    // min cost to connect each node to the tree
    let mut min_cost: Vec<f64> = vec![f64::MAX; n];
    let mut min_edge: Vec<usize> = vec![0; n];

    // Initialize from node 0
    for i in 1..n {
        min_cost[i] = dist(centers[0], centers[i]);
        min_edge[i] = 0;
    }

    for _ in 0..n - 1 {
        // Find closest node not in tree
        let mut best = usize::MAX;
        let mut best_cost = f64::MAX;
        for i in 0..n {
            if !in_tree[i] && min_cost[i] < best_cost {
                best_cost = min_cost[i];
                best = i;
            }
        }
        if best == usize::MAX {
            break;
        }
        in_tree[best] = true;
        edges.push((min_edge[best], best));

        // Update costs
        for i in 0..n {
            if !in_tree[i] {
                let d = dist(centers[best], centers[i]);
                if d < min_cost[i] {
                    min_cost[i] = d;
                    min_edge[i] = best;
                }
            }
        }
    }

    edges
}

// ---------------------------------------------------------------------------
// L-shaped corridor carving
// ---------------------------------------------------------------------------

fn carve_corridor(a: (u32, u32), b: (u32, u32), horizontal_first: bool) -> Vec<(u32, u32)> {
    let mut cells = Vec::new();

    if horizontal_first {
        // Horizontal segment
        let (min_x, max_x) = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
        for x in min_x..=max_x {
            cells.push((x, a.1));
        }
        // Vertical segment
        let (min_y, max_y) = if a.1 <= b.1 { (a.1, b.1) } else { (b.1, a.1) };
        for y in min_y..=max_y {
            cells.push((b.0, y));
        }
    } else {
        // Vertical segment first
        let (min_y, max_y) = if a.1 <= b.1 { (a.1, b.1) } else { (b.1, a.1) };
        for y in min_y..=max_y {
            cells.push((a.0, y));
        }
        // Horizontal segment
        let (min_x, max_x) = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
        for x in min_x..=max_x {
            cells.push((x, b.1));
        }
    }

    cells.sort();
    cells.dedup();
    cells
}

// ---------------------------------------------------------------------------
// Room overlap checking
// ---------------------------------------------------------------------------

fn rooms_overlap(r: &Room, x: u32, y: u32, w: u32, h: u32, padding: u32) -> bool {
    let r_left = r.x.saturating_sub(padding);
    let r_top = r.y.saturating_sub(padding);
    let r_right = r.x + r.w + padding;
    let r_bottom = r.y + r.h + padding;

    let left = x;
    let top = y;
    let right = x + w;
    let bottom = y + h;

    left < r_right && right > r_left && top < r_bottom && bottom > r_top
}

// ---------------------------------------------------------------------------
// Main generation function
// ---------------------------------------------------------------------------

pub fn generate_dungeon(seed: u64) -> Dungeon {
    let mut rng = StdRng::seed_from_u64(seed);

    // Generate name
    let prefix = PREFIXES.choose(&mut rng).unwrap_or(&"Dark");
    let suffix = SUFFIXES.choose(&mut rng).unwrap_or(&"Depths");
    let name = format!("The {} {}", prefix, suffix);

    let floor_room_counts = [8u32, 10, 12];
    let mut floors = Vec::new();

    for floor_idx in 0..3usize {
        let level = (floor_idx + 1) as u32;
        let width: u32 = 40;
        let height: u32 = 30;
        let target_rooms = floor_room_counts[floor_idx] as usize;

        // ---- Place rooms ----
        let mut rooms: Vec<Room> = Vec::new();
        for room_id in 0..target_rooms {
            let mut placed = false;
            for _ in 0..100 {
                let max_room_dim = 6u32;
                let rw = rng.gen_range(3..=6);
                let rh = rng.gen_range(3..=6);
                let rx = rng.gen_range(1..width.saturating_sub(max_room_dim));
                let ry = rng.gen_range(1..height.saturating_sub(max_room_dim));

                let overlaps = rooms.iter().any(|existing| {
                    rooms_overlap(existing, rx, ry, rw, rh, 2)
                });
                if overlaps {
                    continue;
                }

                rooms.push(Room {
                    id: room_id,
                    name: String::new(), // set later
                    room_type: RoomType::Empty, // set later
                    x: rx,
                    y: ry,
                    w: rw,
                    h: rh,
                    discovered: false,
                    visited: false,
                    cleared: false,
                    searched: false,
                    exits: Vec::new(),
                    enemies: Vec::new(),
                    trap: None,
                    treasure: RoomTreasure::default(),
                    description: String::new(),
                });
                placed = true;
                break;
            }
            if !placed {
                // Could not place room — skip
            }
        }

        // Fix room IDs in case some were skipped
        for (i, room) in rooms.iter_mut().enumerate() {
            room.id = i;
        }

        // ---- Assign room types ----
        if !rooms.is_empty() {
            // Floor 1, room 0 = Entrance
            if floor_idx == 0 {
                rooms[0].room_type = RoomType::Entrance;
            } else {
                // First room on floors 2+ gets a neutral type so the stairs arrive here
                rooms[0].room_type = RoomType::Empty;
            }

            // Last room on floor 3 = Boss
            if floor_idx == 2 {
                let last = rooms.len() - 1;
                rooms[last].room_type = RoomType::Boss;
            }

            // Pick one room per floor for stairs (not entrance, not boss, not room 0 on other floors)
            let mut stairs_assigned = false;
            if floor_idx < 2 {
                let candidates: Vec<usize> = (1..rooms.len())
                    .filter(|&i| {
                        rooms[i].room_type != RoomType::Entrance
                            && rooms[i].room_type != RoomType::Boss
                    })
                    .collect();
                if let Some(&stairs_idx) = candidates.choose(&mut rng) {
                    rooms[stairs_idx].room_type = RoomType::Stairs;
                    stairs_assigned = true;
                }
            }

            // Assign random types to remaining unset rooms
            for i in 0..rooms.len() {
                if rooms[i].room_type == RoomType::Empty
                    && !(floor_idx == 0 && i == 0)
                    && !(floor_idx == 2 && i == rooms.len() - 1)
                    && !(stairs_assigned && rooms[i].room_type == RoomType::Stairs)
                {
                    // Check again because the stairs room got set above
                    if rooms[i].room_type == RoomType::Empty {
                        rooms[i].room_type = random_room_type(&mut rng);
                    }
                }
            }
        }

        // ---- Build MST corridors ----
        let mst_edges = prims_mst(&rooms);

        // Add 1-2 random extra connections
        let extra_count = rng.gen_range(1..=2);
        let mut all_edges = mst_edges.clone();
        for _ in 0..extra_count {
            if rooms.len() < 2 {
                break;
            }
            let a = rng.gen_range(0..rooms.len());
            let b = rng.gen_range(0..rooms.len());
            if a != b {
                let edge = if a < b { (a, b) } else { (b, a) };
                if !all_edges.contains(&edge) {
                    all_edges.push(edge);
                }
            }
        }

        // Carve corridors
        let mut corridors = Vec::new();
        for &(from, to) in &all_edges {
            let ac = room_center(&rooms[from]);
            let bc = room_center(&rooms[to]);
            let horizontal_first = rng.gen_bool(0.5);
            let cells = carve_corridor(
                (ac.0 as u32, ac.1 as u32),
                (bc.0 as u32, bc.1 as u32),
                horizontal_first,
            );
            corridors.push(Corridor {
                from_room: from,
                to_room: to,
                cells,
                discovered: false,
            });
        }

        // ---- Create exits from corridors ----
        for corridor in &corridors {
            let from = corridor.from_room;
            let to = corridor.to_room;

            // Direction from -> to
            let dir_ft = direction_from_to(
                rooms[from].x, rooms[from].y, rooms[from].w, rooms[from].h,
                rooms[to].x, rooms[to].y, rooms[to].w, rooms[to].h,
            );
            // Reverse direction
            let dir_tf = direction_from_to(
                rooms[to].x, rooms[to].y, rooms[to].w, rooms[to].h,
                rooms[from].x, rooms[from].y, rooms[from].w, rooms[from].h,
            );

            let unique_ft = unique_direction(&rooms[from].exits, &dir_ft);
            rooms[from].exits.push(Exit {
                direction: unique_ft,
                target_room: to,
                target_floor: None,
                locked: false,
                key_item_id: None,
            });

            let unique_tf = unique_direction(&rooms[to].exits, &dir_tf);
            rooms[to].exits.push(Exit {
                direction: unique_tf,
                target_room: from,
                target_floor: None,
                locked: false,
                key_item_id: None,
            });
        }

        // ---- Populate rooms with content ----
        let mob_table = mobs_for_floor(level);
        let trap_table = traps_for_floor(level);

        for room in &mut rooms {
            // Name & description
            room.name = room_name_for_type(&mut rng, &room.room_type);
            room.description = room_description(&mut rng);

            match room.room_type {
                RoomType::Combat => {
                    // 1-3 random mobs from the floor table
                    let count = rng.gen_range(1..=3);
                    for _ in 0..count {
                        if let Some(mob) = mob_table.choose(&mut rng) {
                            room.enemies.push(mob.clone());
                        }
                    }
                }
                RoomType::Boss => {
                    let bosses = boss_mobs();
                    if let Some(boss) = bosses.choose(&mut rng) {
                        room.enemies.push(boss.clone());
                    }
                    room.treasure = boss_treasure(&mut rng);
                }
                RoomType::Trap => {
                    if let Some(trap) = trap_table.choose(&mut rng) {
                        room.trap = Some(trap.clone());
                    }
                    // Traps can also have a small treasure reward
                    if rng.gen_bool(0.5) {
                        room.treasure = treasure_for_floor(&mut rng, level);
                    }
                }
                RoomType::Treasure => {
                    room.treasure = treasure_for_floor(&mut rng, level);
                }
                RoomType::Entrance => {
                    // Entrance is safe
                    room.cleared = true;
                }
                RoomType::Rest => {
                    // Rest rooms are safe
                    room.cleared = true;
                }
                _ => {
                    // Empty, Puzzle, Stairs — no enemies/traps by default
                }
            }
        }

        // ---- Stairs exits (cross-floor) ----
        if floor_idx < 2 {
            if let Some(stairs_room_idx) = rooms.iter().position(|r| r.room_type == RoomType::Stairs) {
                rooms[stairs_room_idx].exits.push(Exit {
                    direction: "Descend".to_string(),
                    target_room: 0,
                    target_floor: Some(floor_idx + 1),
                    locked: false,
                    key_item_id: None,
                });
            }
        }

        // ---- Boss room locked door (floor 3 only) ----
        if floor_idx == 2 && rooms.len() >= 2 {
            let boss_idx = rooms.len() - 1;

            // Lock all exits leading INTO the boss room
            for room in rooms.iter_mut() {
                for exit in &mut room.exits {
                    if exit.target_room == boss_idx && exit.target_floor.is_none() {
                        exit.locked = true;
                        exit.key_item_id = Some("boss_key".to_string());
                    }
                }
            }

            // Place boss key in a treasure room on this floor (or any non-boss, non-entrance room)
            let key_candidates: Vec<usize> = rooms
                .iter()
                .enumerate()
                .filter(|(i, r)| {
                    *i != boss_idx
                        && r.room_type != RoomType::Entrance
                        && r.room_type != RoomType::Stairs
                })
                .map(|(i, _)| i)
                .collect();
            if let Some(&key_room_idx) = key_candidates.choose(&mut rng) {
                // Add the boss key as an item in that room's treasure
                rooms[key_room_idx]
                    .treasure
                    .item_ids
                    .push("boss_key".to_string());
            }
        }

        floors.push(Floor {
            level,
            width,
            height,
            rooms,
            corridors,
        });
    }

    // ---- Ascend exit on floors 2 and 3, room 0 ----
    // Floor 2 room 0 gets an Ascend exit to floor 1 stairs room
    for target_floor_idx in 0..2usize {
        let next_floor_idx = target_floor_idx + 1;
        // Find stairs room on the lower floor
        if let Some(stairs_room_idx) = floors[target_floor_idx]
            .rooms
            .iter()
            .position(|r| r.room_type == RoomType::Stairs)
        {
            if !floors[next_floor_idx].rooms.is_empty() {
                floors[next_floor_idx].rooms[0].exits.push(Exit {
                    direction: "Ascend".to_string(),
                    target_room: stairs_room_idx,
                    target_floor: Some(target_floor_idx),
                    locked: false,
                    key_item_id: None,
                });
            }
        }
    }

    let mut dungeon = Dungeon {
        name,
        seed,
        floors,
        current_floor: 0,
        current_room: 0,
    };

    // Mark entrance room as discovered and visited
    dungeon.discover_room(0, 0);
    if let Some(room) = dungeon.current_room_mut() {
        room.visited = true;
    }

    dungeon
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        let d1 = generate_dungeon(42);
        let d2 = generate_dungeon(42);
        assert_eq!(d1.name, d2.name);
        assert_eq!(d1.floors.len(), d2.floors.len());
        for (f1, f2) in d1.floors.iter().zip(d2.floors.iter()) {
            assert_eq!(f1.rooms.len(), f2.rooms.len());
            for (r1, r2) in f1.rooms.iter().zip(f2.rooms.iter()) {
                assert_eq!(r1.x, r2.x);
                assert_eq!(r1.y, r2.y);
                assert_eq!(r1.room_type, r2.room_type);
            }
        }
    }

    #[test]
    fn test_entrance_room_is_discovered() {
        let d = generate_dungeon(123);
        let room = d.current_room().unwrap();
        assert!(room.discovered);
        assert!(room.visited);
        assert_eq!(room.room_type, RoomType::Entrance);
    }

    #[test]
    fn test_three_floors() {
        let d = generate_dungeon(999);
        assert_eq!(d.floors.len(), 3);
        assert!(d.floors[0].rooms.len() >= 2);
        assert!(d.floors[1].rooms.len() >= 2);
        assert!(d.floors[2].rooms.len() >= 2);
    }

    #[test]
    fn test_boss_on_floor_3() {
        let d = generate_dungeon(7777);
        let floor3 = &d.floors[2];
        let last = &floor3.rooms[floor3.rooms.len() - 1];
        assert_eq!(last.room_type, RoomType::Boss);
        assert!(!last.enemies.is_empty());
    }

    #[test]
    fn test_name_format() {
        let d = generate_dungeon(55);
        assert!(d.name.starts_with("The "));
        assert!(d.name.split_whitespace().count() == 3);
    }

    #[test]
    fn test_move_to_room() {
        let mut d = generate_dungeon(42);
        let exits: Vec<String> = d
            .current_room()
            .unwrap()
            .exits
            .iter()
            .map(|e| e.direction.clone())
            .collect();
        assert!(!exits.is_empty(), "Entrance should have at least one exit");
        let result = d.move_to_room(&exits[0]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enemy_template_to_enemy() {
        let template = EnemyTemplate {
            name: "Test Mob".into(),
            hp: 10,
            ac: 12,
            attacks: vec![EnemyAttackTemplate {
                name: "Slash".into(),
                damage_dice: "1d6".into(),
                damage_modifier: 2,
                to_hit_bonus: 3,
            }],
        };
        let enemy = template.to_enemy();
        assert_eq!(enemy.name, "Test Mob");
        assert_eq!(enemy.hp, 10);
        assert_eq!(enemy.max_hp, 10);
        assert_eq!(enemy.ac, 12);
        assert_eq!(enemy.attacks.len(), 1);
        assert_eq!(enemy.attacks[0].damage_modifier, 2);
    }
}
