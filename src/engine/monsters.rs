//! Monster generation by tier and type, using simulator-validated stat curves.

use rand::Rng;

use super::combat::{Enemy, EnemyAttack, EnemyType};

/// Tier-based monster stat table. Each entry: (min_hp, max_hp, ac, to_hit, dmg_dice, dmg_count, dmg_mod)
type TierRow = (i32, i32, i32, i32, &'static str, u32, i32);

const BASE_STATS: [TierRow; 11] = [
    // T0: vermin (rats, insects) — beatable unarmed
    (3, 6, 8, 1, "d4", 1, 0),
    // T1: basic enemies
    (10, 18, 12, 4, "d6", 1, 2),
    // T2: standard enemies
    (18, 30, 13, 5, "d6", 1, 3),
    // T3: veteran enemies
    (28, 45, 14, 6, "d8", 1, 4),
    // T4: elite enemies
    (40, 60, 15, 7, "d8", 1, 5),
    // T5: vanguard enemies
    (55, 85, 16, 8, "d10", 1, 6),
    // T6: exalted enemies
    (75, 117, 17, 9, "d10", 1, 7),
    // T7: mythic enemies
    (105, 170, 19, 11, "d10", 1, 9),
    // T8: legendary enemies
    (150, 250, 20, 12, "d12", 1, 11),
    // T9: eternal enemies
    (220, 370, 22, 14, "d10", 2, 12),
    // T10: primordial enemies
    (320, 550, 24, 15, "d12", 2, 14),
];

/// Named monster templates per tier and type.
const MONSTER_NAMES: [[&str; 4]; 11] = [
    // [Brute, Skulker, Mystic, Undead] per tier
    ["Giant Rat", "Cave Spider", "Glow Wisp", "Shambling Corpse"],           // T0
    ["Kobold Thug", "Giant Spider", "Arcane Sprite", "Skeleton"],            // T1
    ["Goblin Warrior", "Wolf", "Fire Imp", "Zombie"],                        // T2
    ["Orc Raider", "Shadow Cat", "Flame Elemental", "Ghoul"],               // T3
    ["Orc Warchief", "Werewolf", "Mind Flayer Spawn", "Wraith"],            // T4
    ["Hill Giant", "Displacer Beast", "Naga", "Vampire Spawn"],             // T5
    ["Stone Golem", "Nightwalker", "Elder Elemental", "Death Knight"],      // T6
    ["Fire Giant", "Shadow Dragon", "Beholder", "Lich"],                     // T7
    ["Storm Giant", "Void Stalker", "Astral Devourer", "Demilich"],         // T8
    ["Titan Warrior", "Dread Wraith Lord", "Arch-Lich", "Dracolich"],       // T9
    ["Primordial Juggernaut", "Primordial Lurker", "Primordial Arcanum", "Primordial Undying"], // T10
];

/// Attack names per enemy type.
const ATTACK_NAMES: [&str; 4] = ["Slam", "Strike", "Blast", "Drain"];

/// Generate a monster of a given tier (0-10) and type.
pub fn generate_monster(tier: u32, enemy_type: EnemyType) -> Enemy {
    let tier = tier.clamp(0, 10) as usize;
    let (min_hp, max_hp, base_ac, base_hit, dice, _dice_count, base_mod) = BASE_STATS[tier];

    let mut rng = rand::thread_rng();
    let hp = rng.gen_range(min_hp..=max_hp);

    // Type-based stat adjustments
    let (hp_mult, ac_adj, hit_adj, mod_adj) = match enemy_type {
        EnemyType::Brute   => (1.20, 1, -1, -1),  // Tankier, lower damage
        EnemyType::Skulker => (0.80, 1, 2, 1),     // Evasive, precise, bursty
        EnemyType::Mystic  => (0.90, -1, 1, 0),    // Moderate HP, low AC, magic damage
        EnemyType::Undead  => (1.05, 0, 0, 0),     // Standard stats, type resistance makes them tough
    };

    let final_hp = (hp as f64 * hp_mult).round() as i32;
    let final_ac = base_ac + ac_adj;
    let final_hit = base_hit + hit_adj;
    let final_mod = base_mod + mod_adj;

    let type_idx = match enemy_type {
        EnemyType::Brute => 0,
        EnemyType::Skulker => 1,
        EnemyType::Mystic => 2,
        EnemyType::Undead => 3,
    };

    let name = MONSTER_NAMES[tier][type_idx].to_string();
    let attack_name = ATTACK_NAMES[type_idx].to_string();

    Enemy {
        name,
        hp: final_hp,
        max_hp: final_hp,
        ac: final_ac,
        attacks: vec![EnemyAttack {
            name: attack_name,
            damage_dice: dice.to_string(),
            damage_modifier: final_mod,
            to_hit_bonus: final_hit,
        }],
        enemy_type: Some(enemy_type),
        tier: Some(tier as u8),
    }
}

/// Generate a random monster appropriate for a given tier.
pub fn generate_random_monster(tier: u32) -> Enemy {
    let mut rng = rand::thread_rng();
    let types = [EnemyType::Brute, EnemyType::Skulker, EnemyType::Mystic, EnemyType::Undead];
    let enemy_type = types[rng.gen_range(0..types.len())];
    generate_monster(tier, enemy_type)
}
