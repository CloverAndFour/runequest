//! Combat loot drop system — generates crafting materials from defeated enemies.

use rand::Rng;

use super::combat::Enemy;
use super::crafting::{CRAFTING_GRAPH, MaterialSource, material_to_item};
use super::inventory::Item;

/// Generate material drops from defeated enemies.
/// Drop chance: 60% for at-tier materials, 30% for one tier below, 10% for two below.
pub fn generate_drops(enemies: &[Enemy]) -> Vec<Item> {
    let mut drops = Vec::new();
    let mut rng = rand::thread_rng();
    let graph = &*CRAFTING_GRAPH;

    for enemy in enemies {
        if enemy.hp > 0 { continue; }
        let Some(ref enemy_type) = enemy.enemy_type else { continue; };
        let tier = enemy.tier.unwrap_or(1);
        let type_str = enemy_type.to_string();

        for mat in graph.materials.values() {
            if let MaterialSource::MonsterDrop { monster_type, min_tier } = &mat.source {
                if monster_type == &type_str && *min_tier <= tier {
                    let tier_diff = tier.saturating_sub(*min_tier);
                    let chance = match tier_diff {
                        0 => 60,
                        1 => 30,
                        2 => 10,
                        _ => 0,
                    };
                    if chance > 0 && rng.gen_range(0..100u32) < chance {
                        if let Some(item) = material_to_item(graph, &mat.id) {
                            drops.push(item);
                        }
                    }
                }
            }
        }
    }
    drops
}
