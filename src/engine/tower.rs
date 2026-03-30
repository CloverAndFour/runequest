//! Tower system — shared infinite dungeons with deterministic floor generation.

use serde::{Serialize, Deserialize};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::combat::EnemyType;
use super::dungeon::SkillGate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerInfo {
    pub id: String,
    pub name: String,
    pub base_tier: f32,
    pub seed: u64,
    /// Minimum skill rank required to enter (0 = no requirement)
    pub entry_skill_rank: u8,
    /// Flavor description
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerFloor {
    pub tower_id: String,
    pub floor_number: u32,
    pub tier: f32,
    pub width: u32,
    pub height: u32,
    pub rooms: Vec<TowerRoom>,
    /// Guard floor: guards patrol all rooms, attack killers on sight.
    pub guard_floor: bool,
    /// Tier of guards on this floor (only relevant if guard_floor).
    pub guard_tier: f32,
    /// Skill gates on this floor (T2+ effective tier).
    #[serde(default)]
    pub skill_gates: Vec<SkillGate>,
    /// Whether the boss on this floor has been killed.
    #[serde(default)]
    pub boss_killed: bool,
    /// Whether first-clear bonus has been claimed.
    #[serde(default)]
    pub first_clear_claimed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerRoom {
    pub x: u32,
    pub y: u32,
    pub room_type: TowerRoomType,
    pub cleared: bool,
    pub enemies: Vec<TowerEnemy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TowerRoomType {
    Empty,
    Combat,
    Treasure,
    Trap,
    GuardPatrolled, // Guards patrol — attack killers on sight, defend non-killers
    Stairs,     // Go to next floor
    Boss,       // Floor boss
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerEnemy {
    pub name: String,
    pub tier: u8,
    pub enemy_type: String,
}

/// The 10 towers in the world.
pub fn tower_definitions() -> Vec<TowerInfo> {
    vec![
        TowerInfo { id: "tower_dawn".into(), name: "Tower of Dawn".into(), base_tier: 2.0, seed: 1001, entry_skill_rank: 0, description: "Beginner-friendly, gentle scaling".into() },
        TowerInfo { id: "ironspire".into(), name: "Ironspire".into(), base_tier: 3.5, seed: 1002, entry_skill_rank: 2, description: "Solo/duo training ground".into() },
        TowerInfo { id: "thornkeep".into(), name: "The Thornkeep".into(), base_tier: 4.5, seed: 1003, entry_skill_rank: 3, description: "First party content".into() },
        TowerInfo { id: "tidecaller".into(), name: "Tidecaller Spire".into(), base_tier: 5.0, seed: 1004, entry_skill_rank: 4, description: "Coordinated parties".into() },
        TowerInfo { id: "shadowpillar".into(), name: "Shadowpillar".into(), base_tier: 5.5, seed: 1005, entry_skill_rank: 4, description: "PvP-heavy, competitive".into() },
        TowerInfo { id: "nexus".into(), name: "The Nexus".into(), base_tier: 6.0, seed: 1006, entry_skill_rank: 5, description: "Guild territory wars".into() },
        TowerInfo { id: "dragonwatch".into(), name: "Dragonwatch".into(), base_tier: 7.0, seed: 1007, entry_skill_rank: 6, description: "Serious raid content".into() },
        TowerInfo { id: "frostspire".into(), name: "Frostspire".into(), base_tier: 8.0, seed: 1008, entry_skill_rank: 7, description: "Endgame guild content".into() },
        TowerInfo { id: "abyss".into(), name: "The Abyss".into(), base_tier: 8.5, seed: 1009, entry_skill_rank: 8, description: "Hardcore, high PvP".into() },
        TowerInfo { id: "primordial_spire".into(), name: "Primordial Spire".into(), base_tier: 9.5, seed: 1010, entry_skill_rank: 9, description: "Server-first pinnacle".into() },
    ]
}

/// Generate a tower floor deterministically from tower seed + floor number.
/// Floor tier = base_tier + floor_number * 0.2
/// Floor size grows: (8 + floor*2) x (8 + floor*2), max 50x50
/// Safe floors every 10 floors.
pub fn generate_floor(tower: &TowerInfo, floor_number: u32) -> TowerFloor {
    let seed = tower.seed.wrapping_add(floor_number as u64 * 7919);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let tier = tower.base_tier + floor_number as f32 * 0.2;
    let guard_floor = floor_number % 10 == 0 && floor_number > 0;

    let size = (8 + floor_number * 2).min(50);
    let width = size;
    let height = size;

    let mut rooms = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let room_type = if guard_floor {
                TowerRoomType::GuardPatrolled
            } else if x == 0 && y == 0 {
                TowerRoomType::GuardPatrolled // Entrance is always safe
            } else if x == width - 1 && y == height - 1 {
                TowerRoomType::Stairs
            } else if x == width / 2 && y == height / 2 && floor_number % 5 == 4 {
                TowerRoomType::Boss
            } else {
                let roll: f32 = rng.gen();
                if roll < 0.45 {
                    TowerRoomType::Combat
                } else if roll < 0.60 {
                    TowerRoomType::Treasure
                } else if roll < 0.70 {
                    TowerRoomType::Trap
                } else if roll < 0.75 {
                    TowerRoomType::GuardPatrolled
                } else {
                    TowerRoomType::Empty
                }
            };

            let enemies = if room_type == TowerRoomType::Combat || room_type == TowerRoomType::Boss {
                let enemy_count = if room_type == TowerRoomType::Boss { 1 } else { rng.gen_range(1..=3) };
                let monster_tier = tier.round() as u32;
                let types = [EnemyType::Brute, EnemyType::Skulker, EnemyType::Mystic, EnemyType::Undead];

                (0..enemy_count).map(|_| {
                    let et = types[rng.gen_range(0..types.len())];
                    TowerEnemy {
                        name: format!("T{} {}", monster_tier, et),
                        tier: monster_tier as u8,
                        enemy_type: format!("{}", et),
                    }
                }).collect()
            } else {
                Vec::new()
            };

            rooms.push(TowerRoom {
                x, y,
                room_type,
                cleared: false,
                enemies,
            });
        }
    }

    // Generate skill gates for higher-tier floors
    let effective_tier = tier.round() as u32;
    let skill_gates = if effective_tier >= 2 && !guard_floor {
        generate_tower_skill_gates(&mut rng, effective_tier, width, height)
    } else {
        Vec::new()
    };

    TowerFloor {
        tower_id: tower.id.clone(),
        floor_number,
        tier,
        width,
        height,
        rooms,
        guard_floor,
        guard_tier: if guard_floor { tier } else { 0.0 },
        skill_gates,
        boss_killed: false,
        first_clear_claimed: false,
    }
}

/// Get floor info summary as JSON value.
pub fn floor_summary(floor: &TowerFloor) -> serde_json::Value {
    let combat_rooms = floor.rooms.iter().filter(|r| r.room_type == TowerRoomType::Combat).count();
    let cleared_rooms = floor.rooms.iter().filter(|r| r.cleared).count();
    let has_boss = floor.rooms.iter().any(|r| r.room_type == TowerRoomType::Boss);

    serde_json::json!({
        "tower": floor.tower_id,
        "floor": floor.floor_number,
        "tier": format!("{:.1}", floor.tier),
        "size": format!("{}x{}", floor.width, floor.height),
        "total_rooms": floor.rooms.len(),
        "combat_rooms": combat_rooms,
        "cleared_rooms": cleared_rooms,
        "guard_floor": floor.guard_floor,
        "guard_tier": format!("{:.1}", floor.guard_tier),
        "first_clear_claimed": floor.first_clear_claimed,
        "boss_killed": floor.boss_killed,
        "skill_gates": floor.skill_gates.len(),
        "has_boss": has_boss,
    })
}


/// Calculate boss HP scaled by nearby player count.
/// Each additional player within 3 rooms adds 30% base HP.
pub fn boss_hp_scaled(base_hp: i32, nearby_players: u32) -> i32 {
    let multiplier = 1.0 + 0.3 * (nearby_players.saturating_sub(1) as f32);
    (base_hp as f32 * multiplier).round() as i32
}

/// Calculate teleport cost to a checkpoint floor.
pub fn checkpoint_teleport_cost(floor_number: u32) -> u32 {
    floor_number * 10
}

/// Check if a player meets the entry requirement for a tower.
pub fn meets_entry_requirement(tower: &TowerInfo, max_skill_rank: u8) -> bool {
    max_skill_rank >= tower.entry_skill_rank
}

/// Generate skill gates for a tower floor.
fn generate_tower_skill_gates(rng: &mut ChaCha8Rng, effective_tier: u32, width: u32, height: u32) -> Vec<SkillGate> {
    let skill_ids = [
        "weapon_mastery", "shield_wall", "fortitude", "lockpicking", "stealth",
        "evocation", "healing", "tracking", "smithing", "enchanting",
    ];
    let gate_count = (effective_tier / 3).min(3) as usize;
    let dc = 10 + effective_tier as i32 * 2;
    let min_rank = 1u8.max(effective_tier.saturating_sub(1) as u8);

    let mut gates = Vec::new();
    for i in 0..gate_count {
        let room_x: u32 = rng.gen_range(1..width.max(2));
        let room_y: u32 = rng.gen_range(1..height.max(2));
        let room_id = (room_y * width + room_x) as usize;
        gates.push(SkillGate {
            room_id,
            exit_index: 0,
            required_skill: skill_ids[i % skill_ids.len()].to_string(),
            required_rank: min_rank,
            dc,
        });
    }
    gates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tower_definitions_count() {
        assert_eq!(tower_definitions().len(), 10);
    }

    #[test]
    fn test_generate_floor_deterministic() {
        let tower = &tower_definitions()[0];
        let floor1 = generate_floor(tower, 1);
        let floor2 = generate_floor(tower, 1);
        // Same tower + same floor number = same layout
        assert_eq!(floor1.rooms.len(), floor2.rooms.len());
        assert_eq!(floor1.width, floor2.width);
        assert_eq!(floor1.tier, floor2.tier);
        for (a, b) in floor1.rooms.iter().zip(floor2.rooms.iter()) {
            assert_eq!(a.room_type, b.room_type);
            assert_eq!(a.enemies.len(), b.enemies.len());
        }
    }

    #[test]
    fn test_floor_size_growth() {
        let tower = &tower_definitions()[0];
        let f0 = generate_floor(tower, 0);
        assert_eq!(f0.width, 8);
        assert_eq!(f0.height, 8);
        let f5 = generate_floor(tower, 5);
        assert_eq!(f5.width, 18);
        assert_eq!(f5.height, 18);
        // Capped at 50
        let f100 = generate_floor(tower, 100);
        assert_eq!(f100.width, 50);
        assert_eq!(f100.height, 50);
    }

    #[test]
    fn test_safe_floor_every_10() {
        let tower = &tower_definitions()[0];
        let f10 = generate_floor(tower, 10);
        assert!(f10.guard_floor);
        assert!(f10.rooms.iter().all(|r| r.room_type == TowerRoomType::GuardPatrolled));
        let f5 = generate_floor(tower, 5);
        assert!(!f5.guard_floor);
    }

    #[test]
    fn test_entrance_is_safe() {
        let tower = &tower_definitions()[0];
        let f1 = generate_floor(tower, 1);
        let entrance = f1.rooms.iter().find(|r| r.x == 0 && r.y == 0).unwrap();
        assert_eq!(entrance.room_type, TowerRoomType::GuardPatrolled);
    }

    #[test]
    fn test_stairs_at_far_corner() {
        let tower = &tower_definitions()[0];
        let f1 = generate_floor(tower, 1);
        let stairs = f1.rooms.iter().find(|r| r.x == f1.width - 1 && r.y == f1.height - 1).unwrap();
        assert_eq!(stairs.room_type, TowerRoomType::Stairs);
    }

    #[test]
    fn test_boss_on_correct_floors() {
        let tower = &tower_definitions()[0];
        // floor 4 (4 % 5 == 4) should have a boss
        let f4 = generate_floor(tower, 4);
        assert!(f4.rooms.iter().any(|r| r.room_type == TowerRoomType::Boss));
        // floor 3 should not
        let f3 = generate_floor(tower, 3);
        assert!(!f3.rooms.iter().any(|r| r.room_type == TowerRoomType::Boss));
    }

    #[test]
    fn test_tier_calculation() {
        let tower = &tower_definitions()[0]; // base_tier 2.0
        let f0 = generate_floor(tower, 0);
        assert!((f0.tier - 2.0).abs() < f32::EPSILON);
        let f5 = generate_floor(tower, 5);
        assert!((f5.tier - 3.0).abs() < f32::EPSILON);
    }


    #[test]
    fn test_boss_hp_scaling() {
        assert_eq!(boss_hp_scaled(100, 1), 100);
        assert_eq!(boss_hp_scaled(100, 2), 130);
        assert_eq!(boss_hp_scaled(100, 10), 370);
    }

    #[test]
    fn test_checkpoint_cost() {
        assert_eq!(checkpoint_teleport_cost(0), 0);
        assert_eq!(checkpoint_teleport_cost(10), 100);
        assert_eq!(checkpoint_teleport_cost(50), 500);
    }

    #[test]
    fn test_entry_requirements() {
        let towers = tower_definitions();
        assert!(meets_entry_requirement(&towers[0], 0));
        assert!(!meets_entry_requirement(&towers[1], 1));
        assert!(meets_entry_requirement(&towers[1], 2));
        assert!(!meets_entry_requirement(&towers[9], 8));
        assert!(meets_entry_requirement(&towers[9], 9));
    }

    #[test]
    fn test_guard_floor_has_guard_tier() {
        let tower = &tower_definitions()[0];
        let f10 = generate_floor(tower, 10);
        assert!(f10.guard_floor);
        assert!(f10.guard_tier > 0.0);
        let f1 = generate_floor(tower, 1);
        assert!(!f1.guard_floor);
        assert!((f1.guard_tier - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_floor_summary() {
        let tower = &tower_definitions()[0];
        let f1 = generate_floor(tower, 1);
        let summary = floor_summary(&f1);
        assert_eq!(summary["tower"], "tower_dawn");
        assert_eq!(summary["floor"], 1);
        assert_eq!(summary["guard_floor"], false);
    }
}
