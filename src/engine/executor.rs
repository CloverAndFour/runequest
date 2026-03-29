//! Tool call executor — dispatches LLM tool calls to engine functions.

use serde_json::Value;

use super::adventure::AdventureState;
use super::combat::{Enemy, EnemyAttack};
use super::dice::DiceRoller;
use super::equipment::{get_item, EquipSlot};
use super::inventory::{Item, ItemType};
use crate::error::{RunequestError, Result};

/// Result of executing a tool call.
#[derive(Debug)]
pub enum ToolExecResult {
    /// Immediate result to return to LLM.
    Immediate(Value),
    /// Request the player to roll dice — server must wait for user input.
    PendingDiceRoll {
        dice_type: String,
        count: u32,
        modifier: i32,
        dc: i32,
        description: String,
        success_probability: f64,
    },
    /// Present choices to the player — server must wait for user selection.
    PendingChoices {
        choices: Vec<String>,
        allow_custom_input: bool,
        prompt: String,
    },
    /// Combat started — server enters combat mode.
    CombatStarted,
}

pub fn execute_tool_call(
    state: &mut AdventureState,
    tool_name: &str,
    args: &Value,
) -> Result<ToolExecResult> {
    state.updated_at = chrono::Utc::now();

    match tool_name {
        "roll_dice" => {
            let dice = args["dice_type"].as_str().unwrap_or("d20");
            let count = args["count"].as_u64().unwrap_or(1) as u32;
            let modifier = args["modifier"].as_i64().unwrap_or(0) as i32;
            let result = DiceRoller::roll(dice, count, modifier);
            Ok(ToolExecResult::Immediate(serde_json::to_value(&result)?))
        }

        "request_player_roll" => {
            let dice_type = args["dice_type"].as_str().unwrap_or("d20").to_string();
            let count = args["count"].as_u64().unwrap_or(1) as u32;
            let modifier = args["modifier"].as_i64().unwrap_or(0) as i32;
            let dc = args["dc"].as_i64().unwrap_or(10) as i32;
            let description = args["description"].as_str().unwrap_or("").to_string();
            let prob = DiceRoller::success_probability(&dice_type, count, modifier, dc);
            Ok(ToolExecResult::PendingDiceRoll {
                dice_type,
                count,
                modifier,
                dc,
                description,
                success_probability: prob,
            })
        }

        "ability_check" => {
            let stat = args["stat"].as_str().unwrap_or("strength");
            let dc = args["dc"].as_i64().unwrap_or(10) as i32;
            let desc = args["description"].as_str().unwrap_or("");
            let modifier = state
                .character
                .stats
                .modifier_for(stat)
                .unwrap_or(0);
            let result = DiceRoller::roll_with_dc("d20", 1, modifier, dc, desc);
            Ok(ToolExecResult::Immediate(serde_json::to_value(&result)?))
        }

        "saving_throw" => {
            let stat = args["stat"].as_str().unwrap_or("constitution");
            let dc = args["dc"].as_i64().unwrap_or(10) as i32;
            let desc = args["description"].as_str().unwrap_or("");
            let modifier = state
                .character
                .stats
                .modifier_for(stat)
                .unwrap_or(0);
            let result = DiceRoller::roll_with_dc("d20", 1, modifier, dc, desc);
            Ok(ToolExecResult::Immediate(serde_json::to_value(&result)?))
        }

        "attack_roll" => {
            let target = args["target"].as_str().unwrap_or("enemy");

            // Use equipped weapon from main hand
            let weapon = state.equipment.equipped_weapon();
            let (damage_dice, stat_mod, weapon_attack_bonus, weapon_name) = if let Some(w) = weapon {
                let stat_name = w.stats.damage_modifier_stat.as_deref().unwrap_or("str");
                let use_finesse = w.stats.is_finesse;
                let mod_val = if use_finesse {
                    // Finesse: use higher of STR or DEX
                    let str_mod = state.character.stats.modifier_for("str").unwrap_or(0);
                    let dex_mod = state.character.stats.modifier_for("dex").unwrap_or(0);
                    std::cmp::max(str_mod, dex_mod)
                } else {
                    state.character.stats.modifier_for(stat_name).unwrap_or(0)
                };
                let dice = w.stats.damage_dice.as_deref().unwrap_or("1d4").to_string();
                let atk_bonus = w.stats.attack_bonus;
                let name = w.display_name();
                (dice, mod_val, atk_bonus, name)
            } else {
                // Unarmed strike
                ("1d4".to_string(), state.character.stats.modifier_for("str").unwrap_or(0), 0, "Unarmed".to_string())
            };

            let prof = state.character.proficiency_bonus();
            let equip_bonuses = state.equipment.stat_bonuses();
            let total_attack_mod = stat_mod + prof + weapon_attack_bonus + equip_bonuses.attack_bonus;
            let attack = DiceRoller::roll("d20", 1, total_attack_mod);

            // Check against enemy AC if in combat
            let target_ac = state
                .combat
                .find_enemy_mut(target)
                .map(|e| e.ac)
                .unwrap_or(10);

            let hit = attack.total >= target_ac;
            let damage = if hit {
                let d = DiceRoller::roll(&damage_dice, 1, stat_mod);
                let dmg = std::cmp::max(d.total, 0);
                // Apply damage to enemy
                if let Some(enemy) = state.combat.find_enemy_mut(target) {
                    enemy.hp -= dmg;
                }
                dmg
            } else {
                0
            };

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "attack_roll": attack.total,
                "target_ac": target_ac,
                "hit": hit,
                "damage": damage,
                "weapon": weapon_name,
                "target": target,
            })))
        }

        "get_character_sheet" => {
            let equipped: Vec<Value> = state.equipment.all_equipped().iter().map(|item| {
                serde_json::json!({
                    "name": item.display_name(),
                    "slot": item.slot,
                    "item_type": item.item_type,
                })
            }).collect();

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "name": state.character.name,
                "race": state.character.race,
                "class": state.character.class,
                "level": state.character.level,
                "xp": state.character.xp,
                "xp_next": state.character.xp_for_next_level(),
                "hp": state.character.hp,
                "max_hp": state.character.max_hp,
                "ac": state.character.ac,
                "gold": state.character.gold,
                "stats": state.character.stats,
                "conditions": state.character.conditions,
                "proficiency_bonus": state.character.proficiency_bonus(),
                "equipped": equipped,
            })))
        }

        "update_hp" => {
            let delta = args["delta"].as_i64().unwrap_or(0) as i32;
            let reason = args["reason"].as_str().unwrap_or("unknown");
            state.character.hp = std::cmp::min(state.character.hp + delta, state.character.max_hp);
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "new_hp": state.character.hp,
                "max_hp": state.character.max_hp,
                "delta": delta,
                "reason": reason,
            })))
        }

        "add_item" => {
            let name = args["name"].as_str().unwrap_or("Unknown Item").to_string();
            let description = args["description"].as_str().unwrap_or("").to_string();
            let item_type_str = args["item_type"].as_str().unwrap_or("misc");
            let weight = args["weight"].as_f64().unwrap_or(1.0) as f32;

            let item_type = match item_type_str {
                "weapon" => ItemType::Weapon,
                "armor" => ItemType::Armor,
                "potion" => ItemType::Potion,
                "scroll" => ItemType::Scroll,
                _ => ItemType::Misc,
            };

            state.inventory.add(Item {
                id: name.to_lowercase().replace(' ', "_"),
                name: name.clone(),
                description,
                item_type,
                slot: None,
                rarity: super::equipment::Rarity::Common,
                weight,
                value_gp: 0,
                stats: Default::default(),
                enchantment: None,
                quantity: 1,
            });

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "added": name,
                "total_items": state.inventory.items.len(),
            })))
        }

        "remove_item" => {
            let name = args["name"].as_str().unwrap_or("");
            let removed = state.inventory.remove(name);
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "removed": removed.is_some(),
                "item": name,
            })))
        }

        "give_item" => {
            let item_id = args["item_id"].as_str().unwrap_or("");
            let quantity = args["quantity"].as_u64().unwrap_or(1) as u32;

            if let Some(mut item) = get_item(item_id) {
                item.quantity = quantity;
                let name = item.display_name();
                state.inventory.add(item);
                Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": true,
                    "item": name,
                    "quantity": quantity,
                    "total_items": state.inventory.items.len(),
                })))
            } else {
                Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": false,
                    "reason": format!("Item '{}' not found in database", item_id),
                })))
            }
        }

        "give_gold" => {
            let amount = args["amount"].as_u64().unwrap_or(0) as u32;
            let reason = args["reason"].as_str().unwrap_or("unknown");
            state.character.gold += amount;
            state.inventory.gold = state.character.gold;
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "gold_added": amount,
                "total_gold": state.character.gold,
                "reason": reason,
            })))
        }

        "equip_item" => {
            let item_name = args["item_name"].as_str().unwrap_or("");

            // Find the item in inventory
            let item = state.inventory.remove(item_name);
            if let Some(item) = item {
                if item.slot.is_none() {
                    // Can't equip — put it back
                    let name = item.name.clone();
                    state.inventory.add(item);
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "success": false,
                        "reason": format!("'{}' cannot be equipped (no equipment slot).", name),
                    })));
                }
                let item_display = item.display_name();
                let slot_name = item.slot.unwrap().display_name().to_string();
                match state.equipment.equip(item) {
                    Ok(displaced) => {
                        // Put displaced item back in inventory
                        if let Some(old_item) = displaced {
                            let old_name = old_item.display_name();
                            state.inventory.add(old_item);
                            // Recalculate AC
                            state.character.ac = state.character.calculate_ac(&state.equipment);
                            Ok(ToolExecResult::Immediate(serde_json::json!({
                                "success": true,
                                "equipped": item_display,
                                "slot": slot_name,
                                "displaced": old_name,
                                "new_ac": state.character.ac,
                            })))
                        } else {
                            state.character.ac = state.character.calculate_ac(&state.equipment);
                            Ok(ToolExecResult::Immediate(serde_json::json!({
                                "success": true,
                                "equipped": item_display,
                                "slot": slot_name,
                                "new_ac": state.character.ac,
                            })))
                        }
                    }
                    Err(msg) => {
                        // Equip failed — this shouldn't normally happen since we checked slot
                        Ok(ToolExecResult::Immediate(serde_json::json!({
                            "success": false,
                            "reason": msg,
                        })))
                    }
                }
            } else {
                Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": false,
                    "reason": format!("Item '{}' not found in inventory.", item_name),
                })))
            }
        }

        "unequip_slot" => {
            let slot_str = args["slot"].as_str().unwrap_or("");
            if let Some(slot) = EquipSlot::from_str(slot_str) {
                if let Some(item) = state.equipment.unequip(&slot) {
                    let name = item.display_name();
                    state.inventory.add(item);
                    state.character.ac = state.character.calculate_ac(&state.equipment);
                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "success": true,
                        "unequipped": name,
                        "slot": slot.display_name(),
                        "new_ac": state.character.ac,
                    })))
                } else {
                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "success": false,
                        "reason": format!("No item equipped in {} slot.", slot.display_name()),
                    })))
                }
            } else {
                Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": false,
                    "reason": format!("Unknown equipment slot: '{}'", slot_str),
                })))
            }
        }

        "use_ability" => {
            let ability_name = args["name"].as_str().unwrap_or("");
            let _target = args["target"].as_str().unwrap_or("");

            let result = if let Some(ability) = state
                .abilities
                .iter_mut()
                .find(|a| a.name.to_lowercase() == ability_name.to_lowercase())
            {
                if let (Some(remaining), Some(_per_rest)) =
                    (&mut ability.uses_remaining, ability.uses_per_rest)
                {
                    if *remaining > 0 {
                        *remaining -= 1;
                        serde_json::json!({
                            "used": ability_name,
                            "uses_remaining": *remaining,
                            "success": true,
                        })
                    } else {
                        serde_json::json!({
                            "used": ability_name,
                            "success": false,
                            "reason": "No uses remaining until rest",
                        })
                    }
                } else {
                    serde_json::json!({
                        "used": ability_name,
                        "success": true,
                        "note": "Ability has unlimited uses",
                    })
                }
            } else {
                serde_json::json!({
                    "success": false,
                    "reason": format!("Ability '{}' not found", ability_name),
                })
            };

            Ok(ToolExecResult::Immediate(result))
        }

        "award_xp" => {
            let amount = args["amount"].as_u64().unwrap_or(0) as u32;
            let reason = args["reason"].as_str().unwrap_or("");
            state.character.xp += amount;
            let leveled_up = state.character.check_level_up();
            if leveled_up {
                state.spell_slots = super::abilities::SpellSlots::for_class(
                    &state.character.class,
                    state.character.level,
                );
            }
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "xp_gained": amount,
                "total_xp": state.character.xp,
                "level": state.character.level,
                "leveled_up": leveled_up,
                "reason": reason,
            })))
        }

        "present_choices" => {
            let choices: Vec<String> = args
                .get("choices")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let allow_custom = args["allow_custom_input"].as_bool().unwrap_or(false);
            let prompt = args["prompt"].as_str().unwrap_or("What do you do?").to_string();

            Ok(ToolExecResult::PendingChoices {
                choices,
                allow_custom_input: allow_custom,
                prompt,
            })
        }

        "set_scene" => {
            let location = args["location"].as_str().unwrap_or("Unknown").to_string();
            let description = args["description"].as_str().unwrap_or("").to_string();
            state.current_scene.location = location.clone();
            state.current_scene.description = description.clone();
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "location": location,
                "description": description,
            })))
        }

        "start_combat" => {
            let enemies: Vec<Enemy> = args
                .get("enemies")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            let name = v["name"].as_str()?.to_string();
                            let hp = v["hp"].as_i64()? as i32;
                            let ac = v["ac"].as_i64().unwrap_or(10) as i32;
                            let attacks = v
                                .get("attacks")
                                .and_then(|a| a.as_array())
                                .map(|attacks| {
                                    attacks
                                        .iter()
                                        .filter_map(|a| {
                                            Some(EnemyAttack {
                                                name: a["name"].as_str()?.to_string(),
                                                damage_dice: a["damage_dice"]
                                                    .as_str()
                                                    .unwrap_or("1d6")
                                                    .to_string(),
                                                damage_modifier: a["damage_modifier"]
                                                    .as_i64()
                                                    .unwrap_or(0)
                                                    as i32,
                                                to_hit_bonus: a["to_hit_bonus"]
                                                    .as_i64()
                                                    .unwrap_or(3)
                                                    as i32,
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            Some(Enemy {
                                name,
                                hp,
                                max_hp: hp,
                                ac,
                                attacks,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let dex_mod = state.character.stats.modifier_for("dex").unwrap_or(0);
            state.combat.start(enemies, dex_mod);

            Ok(ToolExecResult::CombatStarted)
        }

        "end_combat" => {
            let xp_reward = args["xp_reward"].as_u64().unwrap_or(0) as u32;
            state.combat.end();
            state.character.xp += xp_reward;
            let leveled_up = state.character.check_level_up();

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "combat_ended": true,
                "xp_reward": xp_reward,
                "total_xp": state.character.xp,
                "leveled_up": leveled_up,
            })))
        }

        "add_quest" => {
            let name = args["name"].as_str().unwrap_or("Unknown Quest").to_string();
            let description = args["description"].as_str().unwrap_or("").to_string();
            state.quest_log.push(super::adventure::Quest {
                name: name.clone(),
                description,
                completed: false,
            });
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "quest_added": name,
                "total_quests": state.quest_log.len(),
            })))
        }

        "add_condition" => {
            let condition = args["condition"].as_str().unwrap_or("").to_string();
            let duration = args["duration"].as_str().unwrap_or("until cured").to_string();
            if !condition.is_empty() && !state.character.conditions.contains(&condition) {
                state.character.conditions.push(condition.clone());
            }
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "condition_added": condition,
                "duration": duration,
                "active_conditions": state.character.conditions,
            })))
        }

        "remove_condition" => {
            let condition = args["condition"].as_str().unwrap_or("");
            let cond_lower = condition.to_lowercase();
            let before = state.character.conditions.len();
            state.character.conditions.retain(|c| c.to_lowercase() != cond_lower);
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "condition_removed": condition,
                "was_present": state.character.conditions.len() < before,
                "active_conditions": state.character.conditions,
            })))
        }

        "complete_quest" => {
            let name = args["name"].as_str().unwrap_or("");
            let found = state
                .quest_log
                .iter_mut()
                .find(|q| q.name.to_lowercase() == name.to_lowercase());
            if let Some(quest) = found {
                quest.completed = true;
                Ok(ToolExecResult::Immediate(serde_json::json!({
                    "quest_completed": name,
                    "success": true,
                })))
            } else {
                Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": false,
                    "reason": format!("Quest '{}' not found", name),
                })))
            }
        }

        _ => Err(RunequestError::InvalidToolCall(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    }
}
