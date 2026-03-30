//! NPC model for tracked non-player characters.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NpcType {
    Fixed,
    #[default]
    Generated,
}

impl std::fmt::Display for NpcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NpcType::Fixed => write!(f, "fixed"),
            NpcType::Generated => write!(f, "generated"),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NpcFaction {
    #[default]
    Civilian,
    Guard,
    Criminal,
    Neutral,
    Merchant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcInteraction {
    pub summary: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    pub id: String,
    pub name: String,
    pub description: String,
    pub location: String,
    #[serde(default)]
    pub npc_type: NpcType,
    #[serde(default = "default_disposition")]
    pub disposition: String,
    #[serde(default)]
    pub quest_ids: Vec<String>,
    #[serde(default)]
    pub interactions: Vec<NpcInteraction>,
    #[serde(default)]
    pub combat_tier: f32,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
    #[serde(default)]
    pub ac: i32,
    #[serde(default)]
    pub attacks: Vec<super::combat::EnemyAttack>,
    #[serde(default)]
    pub faction: NpcFaction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
}

fn default_disposition() -> String {
    "neutral".to_string()
}

pub fn generate_npc_combat(tier: f32, faction: &NpcFaction) -> (i32, i32, Vec<super::combat::EnemyAttack>) {
    let base_tier = tier.round() as u32;
    let tier_adj: i32 = match faction {
        NpcFaction::Civilian => -1,
        NpcFaction::Merchant => 0,
        NpcFaction::Guard => 1,
        NpcFaction::Criminal => 0,
        NpcFaction::Neutral => 0,
    };
    let effective_tier = (base_tier as i32 + tier_adj).max(0) as u32;

    // Use monster stat curves for NPC combat stats
    let hp = (10 + effective_tier * 8) as i32;
    let ac = (10 + effective_tier) as i32;
    let attack = super::combat::EnemyAttack {
        name: match faction {
            NpcFaction::Guard => "Sword".to_string(),
            NpcFaction::Criminal => "Dagger".to_string(),
            NpcFaction::Civilian => "Fists".to_string(),
            _ => "Strike".to_string(),
        },
        damage_dice: if effective_tier < 2 { "d4".to_string() } else { "d6".to_string() },
        damage_modifier: effective_tier as i32 / 2,
        to_hit_bonus: (2 + effective_tier) as i32,
    };

    (hp, ac, vec![attack])
}
