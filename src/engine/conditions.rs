//! Condition effects system — mechanical effects for status conditions.

use serde::{Deserialize, Serialize};

use super::adventure::AdventureState;
use super::dice::DiceRoller;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEffect {
    pub condition: String,
    pub description: String,
    pub damage_per_turn: Option<String>, // e.g. "1d4" poison damage
    pub stat_penalty: Option<(String, i32)>, // e.g. ("dexterity", -2)
    pub disadvantage_on: Option<String>, // e.g. "attack rolls"
}

/// Get the mechanical effect for a condition name.
pub fn condition_effect(name: &str) -> Option<ConditionEffect> {
    match name.to_lowercase().as_str() {
        "poisoned" => Some(ConditionEffect {
            condition: "Poisoned".to_string(),
            description: "Disadvantage on attack rolls and ability checks. Takes 1d4 poison damage at the start of each turn.".to_string(),
            damage_per_turn: Some("d4".to_string()),
            stat_penalty: None,
            disadvantage_on: Some("attack rolls and ability checks".to_string()),
        }),
        "burning" | "on fire" => Some(ConditionEffect {
            condition: "Burning".to_string(),
            description: "Takes 1d6 fire damage at the start of each turn until extinguished.".to_string(),
            damage_per_turn: Some("d6".to_string()),
            stat_penalty: None,
            disadvantage_on: None,
        }),
        "bleeding" => Some(ConditionEffect {
            condition: "Bleeding".to_string(),
            description: "Takes 1d4 damage at the start of each turn until healed.".to_string(),
            damage_per_turn: Some("d4".to_string()),
            stat_penalty: None,
            disadvantage_on: None,
        }),
        "blinded" => Some(ConditionEffect {
            condition: "Blinded".to_string(),
            description: "Can't see. Disadvantage on attack rolls. Attacks against you have advantage.".to_string(),
            damage_per_turn: None,
            stat_penalty: None,
            disadvantage_on: Some("attack rolls".to_string()),
        }),
        "frightened" => Some(ConditionEffect {
            condition: "Frightened".to_string(),
            description: "Disadvantage on ability checks and attack rolls while the source of fear is in sight.".to_string(),
            damage_per_turn: None,
            stat_penalty: None,
            disadvantage_on: Some("ability checks and attack rolls".to_string()),
        }),
        "stunned" => Some(ConditionEffect {
            condition: "Stunned".to_string(),
            description: "Can't move or take actions. Fails STR and DEX saves. Attacks against you have advantage.".to_string(),
            damage_per_turn: None,
            stat_penalty: None,
            disadvantage_on: Some("all actions".to_string()),
        }),
        "paralyzed" => Some(ConditionEffect {
            condition: "Paralyzed".to_string(),
            description: "Can't move or take actions. Auto-fails STR and DEX saves. Melee attacks against you are critical hits.".to_string(),
            damage_per_turn: None,
            stat_penalty: None,
            disadvantage_on: Some("all actions (auto-fail)".to_string()),
        }),
        "exhaustion" => Some(ConditionEffect {
            condition: "Exhaustion".to_string(),
            description: "Disadvantage on ability checks. Speed halved.".to_string(),
            damage_per_turn: None,
            stat_penalty: None,
            disadvantage_on: Some("ability checks".to_string()),
        }),
        _ => None,
    }
}

/// Apply start-of-turn condition effects. Returns a summary of what happened.
pub fn apply_turn_effects(state: &mut AdventureState) -> Vec<String> {
    let mut effects = Vec::new();

    for condition_name in state.character.conditions.clone() {
        if let Some(effect) = condition_effect(&condition_name) {
            if let Some(ref damage_dice) = effect.damage_per_turn {
                let result = DiceRoller::roll(damage_dice, 1, 0);
                let damage = std::cmp::max(result.total, 1);
                state.character.apply_damage(damage);
                effects.push(format!(
                    "{} deals {} damage (rolled {}). HP: {}/{}",
                    condition_name, damage, result.total,
                    state.character.hp, state.character.max_hp
                ));
            }
        }
    }

    effects
}

/// Build a summary of active condition effects for the LLM system prompt.
pub fn conditions_summary(conditions: &[String]) -> String {
    if conditions.is_empty() {
        return "None".to_string();
    }

    conditions.iter().map(|c| {
        if let Some(effect) = condition_effect(c) {
            format!("{}: {}", c, effect.description)
        } else {
            c.clone()
        }
    }).collect::<Vec<_>>().join("\n  ")
}
