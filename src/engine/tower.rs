//! Tower system — shared infinite dungeons with deterministic floor generation.

use serde::{Serialize, Deserialize};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::combat::EnemyType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerInfo {
    pub id: String,
    pub name: String,
    pub base_tier: f32,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerFloor {
    pub tower_id: String,
    pub floor_number: u32,
    pub tier: f32,
    pub width: u32,
    pub height: u32,
    pub rooms: Vec<TowerRoom>,
    pub safe_floor: bool,
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
    Safe,       // Rest area, no PvP
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
        TowerInfo { id: "tower_dawn".into(), name: "Tower of Dawn".into(), base_tier: 2.0, seed: 1001 },
        TowerInfo { id: "ironspire".into(), name: "Ironspire".into(), base_tier: 3.5, seed: 1002 },
        TowerInfo { id: "thornkeep".into(), name: "The Thornkeep".into(), base_tier: 4.5, seed: 1003 },
        TowerInfo { id: "tidecaller".into(), name: "Tidecaller Spire".into(), base_tier: 5.0, seed: 1004 },
        TowerInfo { id: "shadowpillar".into(), name: "Shadowpillar".into(), base_tier: 5.5, seed: 1005 },
        TowerInfo { id: "nexus".into(), name: "The Nexus".into(), base_tier: 6.0, seed: 1006 },
        TowerInfo { id: "dragonwatch".into(), name: "Dragonwatch".into(), base_tier: 7.0, seed: 1007 },
        TowerInfo { id: "frostspire".into(), name: "Frostspire".into(), base_tier: 8.0, seed: 1008 },
        TowerInfo { id: "abyss".into(), name: "The Abyss".into(), base_tier: 8.5, seed: 1009 },
        TowerInfo { id: "primordial_spire".into(), name: "Primordial Spire".into(), base_tier: 9.5, seed: 1010 },
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
    let safe_floor = floor_number % 10 == 0 && floor_number > 0;

    let size = (8 + floor_number * 2).min(50);
    let width = size;
    let height = size;

    let mut rooms = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let room_type = if safe_floor {
                TowerRoomType::Safe
            } else if x == 0 && y == 0 {
                TowerRoomType::Safe // Entrance is always safe
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
                    TowerRoomType::Safe
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

    TowerFloor {
        tower_id: tower.id.clone(),
        floor_number,
        tier,
        width,
        height,
        rooms,
        safe_floor,
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
        "safe_floor": floor.safe_floor,
        "has_boss": has_boss,
    })
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
        assert!(f10.safe_floor);
        assert!(f10.rooms.iter().all(|r| r.room_type == TowerRoomType::Safe));
        let f5 = generate_floor(tower, 5);
        assert!(!f5.safe_floor);
    }

    #[test]
    fn test_entrance_is_safe() {
        let tower = &tower_definitions()[0];
        let f1 = generate_floor(tower, 1);
        let entrance = f1.rooms.iter().find(|r| r.x == 0 && r.y == 0).unwrap();
        assert_eq!(entrance.room_type, TowerRoomType::Safe);
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
    fn test_floor_summary() {
        let tower = &tower_definitions()[0];
        let f1 = generate_floor(tower, 1);
        let summary = floor_summary(&f1);
        assert_eq!(summary["tower"], "tower_dawn");
        assert_eq!(summary["floor"], 1);
        assert_eq!(summary["safe_floor"], false);
    }
}
