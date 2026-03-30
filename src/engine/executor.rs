//! Tool call executor — dispatches LLM tool calls to engine functions.

use serde_json::Value;

use super::adventure::AdventureState;
use super::combat::{Enemy, EnemyAttack, EnemyType};
use super::monsters::{generate_monster, generate_random_monster};
use super::crafting::{CRAFTING_GRAPH, material_to_item, equipment_to_item};
use super::dice::DiceRoller;
use super::equipment::{get_item, EquipSlot};
use super::inventory::{Item, ItemType};
use super::world_map;
use crate::error::{RunequestError, Result};
use crate::storage::shop_store::ShopStore;

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
    execute_tool_call_with_shop(state, tool_name, args, None)
}

pub fn execute_tool_call_with_shop(
    state: &mut AdventureState,
    tool_name: &str,
    args: &Value,
    shop_store: Option<&ShopStore>,
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
            // When combat mode is active, refuse — player must use BG3 action buttons
            if state.combat.active {
                return Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": "Combat is active. The player uses the combat action buttons to attack. Do NOT call attack_roll during combat — narrate what the combat UI will handle.",
                    "hint": "Describe the combat scene instead. The engine handles attacks via the combat action system."
                })));
            }
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
                tier: 0,
                image_id: None,
                properties: None,
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

            let item_opt = get_item(item_id)
                .or_else(|| material_to_item(&*CRAFTING_GRAPH, item_id))
                .or_else(|| equipment_to_item(item_id));

            if let Some(mut item) = item_opt {
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
            // Engine-controlled enemy generation: LLM provides flavor, engine determines stats.
            // Get tier from current county (world position) or dungeon tier.
            let tier = if let Some(ref dungeon) = state.dungeon {
                dungeon.tier
            } else {
                world_map::current_county(&state.world_position)
                    .map(|c| c.tier.round() as u32)
                    .unwrap_or(0)
            };

            let count = args.get("count")
                .and_then(|v| v.as_u64())
                .unwrap_or(1)
                .clamp(1, 6) as usize;

            let enemy_type_str = args.get("enemy_type")
                .and_then(|v| v.as_str())
                .unwrap_or("random");

            let enemies: Vec<Enemy> = (0..count).map(|_| {
                match enemy_type_str.to_lowercase().as_str() {
                    "brute" => generate_monster(tier, EnemyType::Brute),
                    "skulker" => generate_monster(tier, EnemyType::Skulker),
                    "mystic" => generate_monster(tier, EnemyType::Mystic),
                    "undead" => generate_monster(tier, EnemyType::Undead),
                    _ => generate_random_monster(tier),
                }
            }).collect();

            let enemy_names: Vec<String> = enemies.iter().map(|e| e.name.clone()).collect();
            let dex_mod = state.character.stats.modifier_for("dex").unwrap_or(0);
            state.combat.start(enemies, dex_mod);

            Ok(ToolExecResult::CombatStarted)
        }

        "end_combat" => {
            if !state.combat.active {
                return Ok(ToolExecResult::Immediate(serde_json::json!({"error": "No active combat to end."})));
            }
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

        "move_to_room" => {
            let direction = args["direction"].as_str().unwrap_or("").to_string();

            let dungeon = match &mut state.dungeon {
                Some(d) => d,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No dungeon exists in this adventure."
                    })));
                }
            };

            match dungeon.move_to_room(&direction) {
                Ok(result) => {
                    // Update scene
                    state.current_scene.location = result.room_name.clone();
                    state.current_scene.description = result.description.clone();

                    // Check for enemies (combat)
                    if result.has_enemies {
                        let enemies: Vec<Enemy> = {
                            let room = dungeon.current_room().unwrap();
                            room.enemies.iter().map(|t| t.to_enemy()).collect()
                        };
                        // Mark room as cleared so re-entry doesn't re-trigger
                        if let Some(room) = dungeon.current_room_mut() {
                            room.cleared = true;
                        }

                        let dex_mod = state.character.stats.modifier_for("dex").unwrap_or(0);
                        state.combat.start(enemies, dex_mod);

                        return Ok(ToolExecResult::CombatStarted);
                    }

                    // Check for traps
                    if result.has_trap {
                        let trap_info = {
                            let room = dungeon.current_room().unwrap();
                            room.trap.clone()
                        };

                        if let Some(trap) = trap_info {
                            // Auto-roll WIS-based detection
                            let wis_mod = state
                                .character
                                .stats
                                .modifier_for("wis")
                                .unwrap_or(0);
                            let detect_result =
                                DiceRoller::roll_with_dc("d20", 1, wis_mod, trap.detection_dc, "trap detection");
                            let detected = detect_result.total >= trap.detection_dc;

                            let mut trap_output = serde_json::json!({
                                "room": result.room_name,
                                "room_type": format!("{}", result.room_type),
                                "description": result.description,
                                "trap_name": trap.name,
                                "trap_detected": detected,
                                "detection_roll": detect_result.total,
                                "detection_dc": trap.detection_dc,
                                "exits": result.exits,
                                "floor": result.floor + 1,
                            });

                            if !detected {
                                // Failed detection — auto-roll saving throw
                                let save_mod = state
                                    .character
                                    .stats
                                    .modifier_for(&trap.save_stat)
                                    .unwrap_or(0);
                                let save_result = DiceRoller::roll_with_dc(
                                    "d20",
                                    1,
                                    save_mod,
                                    trap.save_dc,
                                    &format!("{} save vs {}", trap.save_stat, trap.name),
                                );
                                let saved = save_result.total >= trap.save_dc;

                                let damage_result = DiceRoller::roll(&trap.damage_dice, 1, 0);
                                let damage = if saved {
                                    damage_result.total / 2 // half damage on save
                                } else {
                                    damage_result.total
                                };
                                let damage = std::cmp::max(damage, 0);

                                state.character.hp -= damage;

                                // Apply condition if any and save failed
                                if !saved {
                                    if let Some(ref cond) = trap.condition {
                                        if !state.character.conditions.contains(cond) {
                                            state.character.conditions.push(cond.clone());
                                        }
                                    }
                                }

                                trap_output["save_roll"] = serde_json::json!(save_result.total);
                                trap_output["save_dc"] = serde_json::json!(trap.save_dc);
                                trap_output["save_stat"] = serde_json::json!(trap.save_stat);
                                trap_output["save_passed"] = serde_json::json!(saved);
                                trap_output["damage"] = serde_json::json!(damage);
                                trap_output["damage_dice"] = serde_json::json!(trap.damage_dice);
                                if let Some(ref cond) = trap.condition {
                                    if !saved {
                                        trap_output["condition_applied"] =
                                            serde_json::json!(cond);
                                    }
                                }
                                trap_output["hp_after"] = serde_json::json!(state.character.hp);
                                trap_output["max_hp"] =
                                    serde_json::json!(state.character.max_hp);
                            }

                            // Mark room cleared
                            if let Some(room) = dungeon.current_room_mut() {
                                room.cleared = true;
                            }

                            return Ok(ToolExecResult::Immediate(trap_output));
                        }
                    }

                    // Normal room — mark cleared if empty/rest/puzzle/stairs
                    match result.room_type {
                        super::dungeon::RoomType::Empty
                        | super::dungeon::RoomType::Rest
                        | super::dungeon::RoomType::Puzzle
                        | super::dungeon::RoomType::Stairs
                        | super::dungeon::RoomType::Entrance => {
                            if let Some(room) = dungeon.current_room_mut() {
                                room.cleared = true;
                            }
                        }
                        _ => {}
                    }

                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "room": result.room_name,
                        "room_type": format!("{}", result.room_type),
                        "description": result.description,
                        "exits": result.exits,
                        "floor": result.floor + 1,
                        "cleared": dungeon.current_room().map(|r| r.cleared).unwrap_or(false),
                    })))
                }
                Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": e,
                }))),
            }
        }

        "search_room" => {
            let dungeon = match &mut state.dungeon {
                Some(d) => d,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No dungeon exists in this adventure."
                    })));
                }
            };

            let floor_level = dungeon
                .current_floor()
                .map(|f| f.level)
                .unwrap_or(1);

            // WIS check DC scales with floor
            let search_dc = match floor_level {
                1 => 10,
                2 => 13,
                _ => 15,
            };

            let wis_mod = state.character.stats.modifier_for("wis").unwrap_or(0);
            let check = DiceRoller::roll_with_dc("d20", 1, wis_mod, search_dc, "search room");
            let passed = check.total >= search_dc;

            let room = match dungeon.current_room_mut() {
                Some(r) => r,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No current room."
                    })));
                }
            };

            if room.searched {
                return Ok(ToolExecResult::Immediate(serde_json::json!({
                    "searched": true,
                    "found": false,
                    "message": "You have already thoroughly searched this room.",
                })));
            }

            room.searched = true;

            if !passed {
                return Ok(ToolExecResult::Immediate(serde_json::json!({
                    "searched": true,
                    "found": false,
                    "check_roll": check.total,
                    "check_dc": search_dc,
                    "message": "You search the room but find nothing of interest.",
                })));
            }

            let treasure = room.treasure.clone();
            if treasure.gold == 0 && treasure.item_ids.is_empty() {
                return Ok(ToolExecResult::Immediate(serde_json::json!({
                    "searched": true,
                    "found": false,
                    "check_roll": check.total,
                    "check_dc": search_dc,
                    "message": "Your search is thorough but the room holds no valuables.",
                })));
            }

            // Award gold
            if treasure.gold > 0 {
                state.character.gold += treasure.gold;
                state.inventory.gold = state.character.gold;
            }

            // Award items
            let mut given_items = Vec::new();
            for item_id in &treasure.item_ids {
                if item_id == "boss_key" {
                    // Special key item — add as a custom item
                    let key_item = Item {
                        id: "boss_key".into(),
                        name: "Boss Key".into(),
                        description: "An ornate key that radiates dark energy. It unlocks the way to the dungeon's master.".into(),
                        item_type: ItemType::Misc,
                        slot: None,
                        rarity: super::equipment::Rarity::Rare,
                        weight: 0.5,
                        value_gp: 0,
                        stats: Default::default(),
                        enchantment: None,
                        quantity: 1,
                        tier: 0,
                        image_id: None,
                        properties: None,
                    };
                    state.inventory.add(key_item);
                    given_items.push("Boss Key".to_string());

                    // Unlock boss doors in the dungeon
                    // (need to re-borrow dungeon)
                } else if let Some(item) = get_item(item_id) {
                    let display = item.display_name();
                    state.inventory.add(item);
                    given_items.push(display);
                }
            }

            // Clear treasure so it can't be collected again
            if let Some(room) = dungeon.current_room_mut() {
                room.treasure.gold = 0;
                room.treasure.item_ids.clear();
            }

            // If we found a boss key, unlock the boss doors
            if given_items.iter().any(|n| n == "Boss Key") {
                dungeon.unlock_boss_door();
            }

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "searched": true,
                "found": true,
                "check_roll": check.total,
                "check_dc": search_dc,
                "gold_found": treasure.gold,
                "total_gold": state.character.gold,
                "items_found": given_items,
                "message": format!("You found {} gold and {} item(s)!", treasure.gold, given_items.len()),
            })))
        }

        // -----------------------------------------------------------------------
        // World map tools
        // -----------------------------------------------------------------------

        "travel_to" => {
            let location_str = args["location"].as_str().unwrap_or("");
            let world = match &mut state.world {
                Some(w) => w,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No world map exists in this adventure."
                    })));
                }
            };

            let location_id = match world.find_location(location_str) {
                Some(id) => id,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": format!("Location '{}' not found. Use view_map or check available destinations.", location_str),
                    })));
                }
            };

            match world.travel_to(location_id) {
                Ok(result) => {
                    state.current_scene.location = result.location_name.clone();
                    state.current_scene.description = result.description.clone();

                    if let Some(ref encounter) = result.encounter {
                        if !encounter.enemies.is_empty() {
                            let enemies: Vec<Enemy> = encounter.enemies.iter().map(|t| t.to_enemy()).collect();
                            let dex_mod = state.character.stats.modifier_for("dex").unwrap_or(0);
                            state.combat.start(enemies, dex_mod);

                            return Ok(ToolExecResult::CombatStarted);
                        }
                    }

                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "arrived": result.location_name,
                        "location_type": result.location_type,
                        "description": result.description,
                    })))
                }
                Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": e,
                }))),
            }
        }

        "enter_dungeon" => {
            let world = match &mut state.world {
                Some(w) => w,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No world map exists in this adventure."
                    })));
                }
            };

            let current_loc = world.current_location;
            match world.enter_dungeon(current_loc) {
                Ok(()) => {
                    // Copy the dungeon reference into adventure.dungeon for the
                    // existing dungeon navigation tools (move_to_room, search_room)
                    let dng = world.current_dungeon().unwrap().clone();
                    let loc_name = world.locations[current_loc].name.clone();

                    let room_info = if let Some(room) = dng.current_room() {
                        state.current_scene.location = format!("{} — {}", loc_name, room.name);
                        state.current_scene.description = room.description.clone();
                        serde_json::json!({
                            "room": room.name,
                            "description": room.description,
                            "exits": room.exits.iter().map(|e| e.direction.clone()).collect::<Vec<_>>(),
                        })
                    } else {
                        serde_json::json!({"room": "entrance"})
                    };

                    state.dungeon = Some(dng);

                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "entered_dungeon": loc_name,
                        "dungeon_name": state.dungeon.as_ref().unwrap().name,
                        "current_room": room_info,
                    })))
                }
                Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": e,
                }))),
            }
        }

        "exit_dungeon" => {
            let world = match &mut state.world {
                Some(w) => w,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No world map exists in this adventure."
                    })));
                }
            };

            // Sync dungeon state back to world before exiting
            if let super::world::GameMode::InDungeon { location_id } = world.game_mode {
                if let Some(ref dng) = state.dungeon {
                    world.dungeons.insert(location_id, dng.clone());
                }
            }

            world.exit_dungeon();
            state.dungeon = None;
            let loc = world.current_loc();
            state.current_scene.location = loc.name.clone();
            state.current_scene.description = loc.description.clone();
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "exited_dungeon": true,
                "location": loc.name,
                "description": loc.description,
            })))
        }

        "enter_tower" => {
            let world = match &mut state.world {
                Some(w) => w,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No world map exists in this adventure."
                    })));
                }
            };

            match world.enter_tower() {
                Ok(floor) => {
                    let key = world.tower_dungeon_key().unwrap();
                    let dng = world.dungeons.get(&key).unwrap().clone();

                    let room_info = if let Some(room) = dng.current_room() {
                        state.current_scene.location = format!("The Endless Tower — Floor {} — {}", floor, room.name);
                        state.current_scene.description = room.description.clone();
                        serde_json::json!({
                            "room": room.name,
                            "description": room.description,
                            "exits": room.exits.iter().map(|e| e.direction.clone()).collect::<Vec<_>>(),
                        })
                    } else {
                        serde_json::json!({"room": "entrance"})
                    };

                    state.dungeon = Some(dng);

                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "entered_tower": true,
                        "floor": floor,
                        "current_room": room_info,
                    })))
                }
                Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": e,
                }))),
            }
        }

        "tower_ascend" => {
            let world = match &mut state.world {
                Some(w) => w,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No world map exists in this adventure."
                    })));
                }
            };

            // Sync current dungeon state back
            if let Some(key) = world.tower_dungeon_key() {
                if let Some(ref dng) = state.dungeon {
                    world.dungeons.insert(key, dng.clone());
                }
            }

            match world.tower_ascend() {
                Ok(new_floor) => {
                    let key = world.tower_dungeon_key().unwrap();
                    let dng = world.dungeons.get(&key).unwrap().clone();

                    let room_info = if let Some(room) = dng.current_room() {
                        state.current_scene.location = format!("The Endless Tower — Floor {} — {}", new_floor, room.name);
                        state.current_scene.description = room.description.clone();
                        serde_json::json!({
                            "room": room.name,
                            "description": room.description,
                            "exits": room.exits.iter().map(|e| e.direction.clone()).collect::<Vec<_>>(),
                        })
                    } else {
                        serde_json::json!({"room": "entrance"})
                    };

                    state.dungeon = Some(dng);

                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "ascended": true,
                        "new_floor": new_floor,
                        "current_room": room_info,
                    })))
                }
                Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": e,
                }))),
            }
        }

        "exit_tower" => {
            let world = match &mut state.world {
                Some(w) => w,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No world map exists in this adventure."
                    })));
                }
            };

            world.exit_tower();
            state.dungeon = None;
            let loc = world.current_loc();
            state.current_scene.location = loc.name.clone();
            state.current_scene.description = loc.description.clone();
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "exited_tower": true,
                "location": loc.name,
                "description": loc.description,
            })))
        }

        "buy_item" => {
            let item_id = args["item_id"].as_str().unwrap_or("");
            let quantity = args["quantity"].as_u64().unwrap_or(1) as u32;
            let county = world_map::current_county(&state.world_position);
            match county {
                Some(county) if county.has_town => {
                    if let Some(shop_store) = shop_store {
                        let q = state.world_position.county_q;
                        let r = state.world_position.county_r;
                        let player_gold = state.character.gold;
                        let result = shop_store.with_mut(|reg: &mut crate::engine::shop::ShopRegistry| {
                            match reg.get_or_create(q, r) {
                                Some(shop) => shop.buy(item_id, quantity, player_gold),
                                None => Err("No shop at this location".to_string()),
                            }
                        });
                        match result {
                            Ok(buy_result) => match buy_result {
                                Ok((item_name, total_price)) => {
                                    state.character.gold -= total_price;
                                    state.inventory.gold = state.character.gold;
                                    if let Some(item) = super::equipment::get_item(item_id) {
                                        let mut inv_item = item.clone();
                                        inv_item.quantity = quantity;
                                        state.inventory.items.push(inv_item);
                                    }
                                    Ok(ToolExecResult::Immediate(serde_json::json!({
                                        "success": true,
                                        "message": format!("Bought {} x{} for {} gold", item_name, quantity, total_price),
                                        "gold_remaining": state.character.gold,
                                    })))
                                }
                                Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                                    "success": false,
                                    "error": e,
                                }))),
                            },
                            Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                                "success": false,
                                "error": format!("{}", e),
                            }))),
                        }
                    } else {
                        // Fallback: no shop store (shouldn't happen in normal flow)
                        Ok(ToolExecResult::Immediate(serde_json::json!({
                            "success": false,
                            "error": "Shop system unavailable.",
                        })))
                    }
                }
                _ => Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": false,
                    "error": "No shop at this location.",
                }))),
            }
        }

        "sell_item" => {
            let item_name = args["item_name"].as_str().unwrap_or("");
            let county = world_map::current_county(&state.world_position);
            let at_town = county.map(|c| c.has_town).unwrap_or(false);
            if !at_town {
                return Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": false,
                    "error": "No shop here — you must be at a town to sell items.",
                })));
            }
            // Find item in inventory by name (case-insensitive)
            let idx = state.inventory.items.iter().position(|i| i.name.to_lowercase() == item_name.to_lowercase());
            match idx {
                Some(idx) => {
                    let item = state.inventory.items.remove(idx);
                    if let Some(shop_store) = shop_store {
                        let q = state.world_position.county_q;
                        let r = state.world_position.county_r;
                        let base_value = super::equipment::get_item(&item.id).map(|i| i.value_gp).unwrap_or(1);
                        let sell_price = shop_store.with_mut(|reg: &mut crate::engine::shop::ShopRegistry| {
                            match reg.get_or_create(q, r) {
                                Some(shop) => shop.sell(&item.id, base_value, 1),
                                None => (base_value * 60 / 100).max(1),
                            }
                        }).unwrap_or(1);
                        state.character.gold += sell_price;
                        state.inventory.gold = state.character.gold;
                        Ok(ToolExecResult::Immediate(serde_json::json!({
                            "success": true,
                            "message": format!("Sold {} for {} gold", item.name, sell_price),
                            "sell_price": sell_price,
                            "gold_remaining": state.character.gold,
                        })))
                    } else {
                        // Fallback
                        let base_item = super::equipment::get_item(&item.id);
                        let sell_price = base_item.map(|i| std::cmp::max(i.value_gp / 2, 1)).unwrap_or(1);
                        state.character.gold += sell_price;
                        state.inventory.gold = state.character.gold;
                        Ok(ToolExecResult::Immediate(serde_json::json!({
                            "success": true,
                            "message": format!("Sold {} for {} gold", item.name, sell_price),
                            "sell_price": sell_price,
                            "gold_remaining": state.character.gold,
                        })))
                    }
                }
                None => Ok(ToolExecResult::Immediate(serde_json::json!({
                    "success": false,
                    "error": format!("Item '{}' not found in inventory", item_name),
                }))),
            }
        }

        "view_shop" => {
            let county = world_map::current_county(&state.world_position);
            if let Some(county) = county {
                if !county.has_town {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "No shop here — this county has no town."
                    })));
                }
                if let Some(shop_store) = shop_store {
                    let q = state.world_position.county_q;
                        let r = state.world_position.county_r;
                    let shop_data = shop_store.with_mut(|reg: &mut crate::engine::shop::ShopRegistry| {
                        match reg.get_or_create(q, r) {
                            Some(shop) => {
                                let items: Vec<serde_json::Value> = shop.items.values().filter_map(|si| {
                                    let equip = super::equipment::get_item(&si.item_id)?;
                                    Some(serde_json::json!({
                                        "item_id": si.item_id,
                                        "name": equip.name,
                                        "description": equip.description,
                                        "buy_price": si.buy_price(),
                                        "sell_price": si.sell_price(),
                                        "current_stock": si.current_stock,
                                        "price_category": si.price_category(),
                                        "tier": equip.tier,
                                    }))
                                }).collect();
                                Some((shop.name.clone(), shop.tier, items))
                            }
                            None => None,
                        }
                    });
                    match shop_data {
                        Ok(Some((name, tier, items))) => {
                            Ok(ToolExecResult::Immediate(serde_json::json!({
                                "shop_name": name,
                                "tier": tier,
                                "location": county.name,
                                "items": items,
                                "player_gold": state.character.gold,
                            })))
                        }
                        Ok(None) => Ok(ToolExecResult::Immediate(serde_json::json!({
                            "error": "No shop at this location.",
                        }))),
                        Err(e) => Ok(ToolExecResult::Immediate(serde_json::json!({
                            "error": format!("{}", e),
                        }))),
                    }
                } else {
                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": "Shop system unavailable.",
                    })))
                }
            } else {
                Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": "No shop at this location.",
                })))
            }
        }

        // -----------------------------------------------------------------------
        // Crafting tools
        // -----------------------------------------------------------------------

        "craft_item" => {
            let recipe_id = args["recipe_id"].as_str().unwrap_or("");
            let graph = &*CRAFTING_GRAPH;

            // Find recipe
            let recipe = match graph.recipes.iter().find(|r| r.id == recipe_id) {
                Some(r) => r,
                None => {
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": format!("Recipe {} not found", recipe_id),
                    })));
                }
            };

            // Check skill rank
            let skill_id = recipe.skill.skill_id();
            let player_rank = state.skills.get(skill_id).map(|s| s.rank).unwrap_or(0);
            if player_rank < recipe.skill_rank {
                return Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": format!("Requires {} rank {} but you have rank {}", recipe.skill.name(), recipe.skill_rank, player_rank),
                })));
            }

            // Check station availability
            let county = world_map::current_county(&state.world_position);
            let has_station = if let Some(c) = county {
                c.stations.iter().any(|st| {
                    st.supported_skills().contains(&skill_id) && st.max_tier() >= recipe.tier
                })
            } else {
                false
            };
            if !has_station {
                return Ok(ToolExecResult::Immediate(serde_json::json!({
                    "error": format!("No crafting station available for {} tier {} here. Travel to a town with the right station.", recipe.skill.name(), recipe.tier),
                })));
            }

            // Check all inputs
            for (mat_id, qty) in &recipe.inputs {
                let have = state.inventory.items.iter()
                    .filter(|i| i.id == *mat_id)
                    .map(|i| i.quantity)
                    .sum::<u32>();
                if have < *qty {
                    let mat_name = graph.materials.get(mat_id).map(|m| m.name.as_str()).unwrap_or(mat_id);
                    return Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": format!("Missing material: need {} x{} but have {}", mat_name, qty, have),
                    })));
                }
            }

            // Consume inputs
            for (mat_id, qty) in &recipe.inputs {
                for _ in 0..*qty {
                    state.inventory.remove_by_id(mat_id);
                }
            }

            // Produce output
            let output_id = &recipe.output;
            let output_item = equipment_to_item(output_id)
                .or_else(|| material_to_item(graph, output_id));

            let output_name = if let Some(mut item) = output_item {
                item.quantity = recipe.output_qty;
                let name = item.display_name();
                state.inventory.add_material(item);
                name
            } else {
                output_id.to_string()
            };

            // Skill XP chance: 15% at-level, 25% above-level
            let xp_chance = if player_rank > recipe.skill_rank { 0.25 } else { 0.15 };
            let mut rng = rand::thread_rng();
            let skill_progress = if rand::Rng::gen::<f64>(&mut rng) < xp_chance {
                let xp_amount = 10 + (recipe.tier as u32) * 5;
                let result = state.skills.gain_xp(skill_id, xp_amount);
                if let Some((new_rank, ranked_up)) = result {
                    Some(serde_json::json!({
                        "skill": skill_id,
                        "xp_gained": xp_amount,
                        "new_rank": new_rank,
                        "ranked_up": ranked_up,
                    }))
                } else {
                    None
                }
            } else {
                None
            };

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "crafted": true,
                "output": output_name,
                "quantity": recipe.output_qty,
                "recipe": recipe.name,
                "skill_progress": skill_progress,
            })))
        }

        "list_recipes" => {
            let skill_filter = args.get("skill").and_then(|v| v.as_str());
            let tier_filter = args.get("tier").and_then(|v| v.as_u64()).map(|t| t as u8);
            let graph = &*CRAFTING_GRAPH;

            // Get available stations at current location
            let county = world_map::current_county(&state.world_position);
            let stations: Vec<_> = county.map(|c| c.stations.clone()).unwrap_or_default();

            let recipes: Vec<serde_json::Value> = graph.recipes.iter()
                .filter(|r| {
                    // Filter by skill if specified
                    if let Some(sf) = skill_filter {
                        if r.skill.skill_id() != sf { return false; }
                    }
                    // Filter by tier if specified
                    if let Some(tf) = tier_filter {
                        if r.tier != tf { return false; }
                    }
                    // Filter by player skill rank
                    let player_rank = state.skills.get(r.skill.skill_id()).map(|s| s.rank).unwrap_or(0);
                    if player_rank < r.skill_rank { return false; }
                    // Filter by available stations
                    stations.iter().any(|st| {
                        st.supported_skills().contains(&r.skill.skill_id()) && st.max_tier() >= r.tier
                    })
                })
                .map(|r| {
                    let inputs: Vec<serde_json::Value> = r.inputs.iter().map(|(id, qty)| {
                        let name = graph.materials.get(id).map(|m| m.name.as_str()).unwrap_or(id.as_str());
                        serde_json::json!({"id": id, "name": name, "quantity": qty})
                    }).collect();
                    let output_name = graph.materials.get(&r.output).map(|m| m.name.as_str()).unwrap_or(r.output.as_str());
                    serde_json::json!({
                        "id": r.id,
                        "name": r.name,
                        "skill": r.skill.name(),
                        "skill_rank": r.skill_rank,
                        "tier": r.tier,
                        "inputs": inputs,
                        "output": r.output,
                        "output_name": output_name,
                        "output_qty": r.output_qty,
                    })
                })
                .collect();

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "recipes": recipes,
                "count": recipes.len(),
            })))
        }

        "gather" => {
            let county = world_map::current_county(&state.world_position);
            let biome_str = county.map(|c| format!("{}", c.biome)).unwrap_or_else(|| "Plains".to_string());

            // Biome-based gather pools
            let pool: &[&str] = match biome_str.as_str() {
                "Forest" => &["green_wood", "plant_fiber", "wild_herbs", "crude_thread"],
                "Hills" => &["rough_stone", "scrap_metal", "raw_quartz", "muddy_clay"],
                "Mountains" => &["scrap_metal", "rough_stone", "raw_quartz", "charcoal"],
                "Swamp" => &["muddy_clay", "plant_fiber", "wild_herbs", "crude_thread"],
                "Desert" => &["rough_stone", "raw_quartz", "charcoal", "scrap_metal"],
                "Tundra" => &["raw_hide_scraps", "crude_thread", "rough_stone", "charcoal"],
                "Coast" => &["plant_fiber", "muddy_clay", "crude_thread", "raw_quartz"],
                "Volcanic" => &["charcoal", "scrap_metal", "rough_stone", "raw_quartz"],
                _ /* Plains */ => &["raw_hide_scraps", "crude_thread", "plant_fiber", "green_wood"],
            };

            let graph = &*CRAFTING_GRAPH;
            let mut rng = rand::thread_rng();
            let count = rand::Rng::gen_range(&mut rng, 1..=3usize);
            let mut gathered = Vec::new();

            for _ in 0..count {
                let idx = rand::Rng::gen_range(&mut rng, 0..pool.len());
                let mat_id = pool[idx];
                if let Some(item) = material_to_item(graph, mat_id) {
                    gathered.push(serde_json::json!({"id": mat_id, "name": item.name.clone()}));
                    state.inventory.add_material(item);
                }
            }

            // Award 5-10 Survival XP
            let surv_xp = rand::Rng::gen_range(&mut rng, 5..=10u32);
            let surv_result = state.skills.gain_xp("survival", surv_xp);

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "gathered": gathered,
                "biome": biome_str,
                "survival_xp": surv_xp,
                "survival_rank_up": surv_result.map(|(_, up)| up).unwrap_or(false),
            })))
        }

        // -----------------------------------------------------------------------
        // Skill tools
        // -----------------------------------------------------------------------

        "award_skill_xp" => {
            let skill_id = args["skill_id"].as_str().unwrap_or("");
            let amount = args["amount"].as_u64().unwrap_or(0) as u32;

            match state.skills.gain_xp(skill_id, amount) {
                Some((new_rank, ranked_up)) => {
                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "skill": skill_id,
                        "xp_gained": amount,
                        "new_rank": new_rank,
                        "ranked_up": ranked_up,
                    })))
                }
                None => {
                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "error": format!("Skill {} not found", skill_id),
                    })))
                }
            }
        }

        "get_skills" => {
            let skills_json: Vec<serde_json::Value> = state.skills.skills.iter().map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "rank": s.rank,
                    "rank_name": super::skills::rank_name(s.rank),
                    "xp": s.xp,
                    "xp_to_next": s.xp_to_next,
                })
            }).collect();

            Ok(ToolExecResult::Immediate(serde_json::json!({
                "skills": skills_json,
            })))
        }

        "improve_skill" => {
            let skill_id = args["skill_id"].as_str().unwrap_or("");
            match state.skills.improve(skill_id) {
                Some(new_rank) => {
                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "success": true,
                        "skill": skill_id,
                        "new_rank": new_rank,
                        "rank_name": super::skills::rank_name(new_rank),
                    })))
                }
                None => {
                    Ok(ToolExecResult::Immediate(serde_json::json!({
                        "success": false,
                        "error": format!("Cannot improve skill {} (not found or already max rank)", skill_id),
                    })))
                }
            }
        }

        // -----------------------------------------------------------------------
        // World map info tool
        // -----------------------------------------------------------------------

        "get_map_info" => {
            let info = world_map::map_info(&state.world_position, &state.discovery);
            Ok(ToolExecResult::Immediate(info))
        }

        // -----------------------------------------------------------------------
        // Murderer flag tools
        // -----------------------------------------------------------------------

        "flag_murderer" => {
            state.murderer = true;
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "flagged": true,
                "message": "Player has been flagged as a murderer. Town guards will be hostile.",
            })))
        }

        "check_murderer" => {
            Ok(ToolExecResult::Immediate(serde_json::json!({
                "is_murderer": state.murderer,
            })))
        }

        _ => Err(RunequestError::InvalidToolCall(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    }
}
