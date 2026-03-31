//! Unified action menu — computes available game actions from adventure state.

use serde::{Serialize, Deserialize};
use crate::engine::adventure::AdventureState;
use crate::engine::rate_limit;

/// A fixed (engine-only) action available to the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedAction {
    pub id: String,
    pub action: String,
    pub category: String,
    pub label: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub cooldown_ms: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

fn default_true() -> bool { true }

/// A combat action (from the existing combat system).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatActionInfo {
    pub id: String,
    pub name: String,
    pub cost: String,
    pub description: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
}

/// The complete action menu returned by GET /actions.
#[derive(Debug, Clone, Serialize)]
pub struct ActionMenu {
    pub fixed_actions: Vec<FixedAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combat_actions: Option<Vec<CombatActionInfo>>,
    /// Always null — LLM actions are lazy, requested via POST /action { action: "get_llm_choices" }
    pub llm_actions: Option<serde_json::Value>,
    pub state: serde_json::Value,
}

/// Build the complete action menu from current adventure state.
pub fn build_action_menu(
    adventure: &AdventureState,
    map_view: &serde_json::Value,
    state_json: serde_json::Value,
) -> ActionMenu {
    let mut fixed = Vec::new();
    let in_combat = adventure.combat.active;
    let is_dead = adventure.character.dead || adventure.character.hp <= 0;
    let in_dungeon = adventure.dungeon.is_some();
    let current = &map_view["current"];

    // Compute cooldowns
    let (llm_cd, fixed_cd) = rate_limit::remaining_cooldowns(
        adventure.last_llm_action_at,
        adventure.last_fixed_action_at,
    );

    let combat_reason = if in_combat { Some("Cannot do this during combat".to_string()) } else { None };
    let dead_reason = if is_dead { Some("Character is dead".to_string()) } else { None };

    if !is_dead && !in_combat && !in_dungeon {
        // ---- TRAVEL ----
        if let Some(dirs) = map_view.get("directions").and_then(|d| d.as_array()) {
            for dir in dirs {
                let direction = dir["direction"].as_str().unwrap_or("?");
                let name = dir["name"].as_str().unwrap_or("Unknown");
                let tier = dir["tier"].as_str().unwrap_or("?");
                let dir_lower = direction.to_lowercase();
                fixed.push(FixedAction {
                    id: format!("travel_{}", dir_lower),
                    action: "travel".into(),
                    category: "travel".into(),
                    label: format!("{}: {} (T{})", direction, name, tier),
                    params: serde_json::json!({ "direction": dir_lower }),
                    cooldown_ms: fixed_cd,
                    enabled: true,
                    disabled_reason: None,
                });
            }
        }

        // ---- TOWN ACTIONS ----
        if current["has_town"].as_bool().unwrap_or(false) {
            fixed.push(FixedAction {
                id: "shop_view".into(), action: "shop_view".into(),
                category: "town".into(), label: "Visit Shop".into(),
                params: serde_json::json!({}), cooldown_ms: 0, enabled: true,
                disabled_reason: None,
            });
        }
        if current["has_dungeon"].as_bool().unwrap_or(false) {
            fixed.push(FixedAction {
                id: "dungeon_enter".into(), action: "dungeon_enter".into(),
                category: "dungeon".into(), label: "Enter Dungeon".into(),
                params: serde_json::json!({}), cooldown_ms: fixed_cd, enabled: true,
                disabled_reason: None,
            });
        }
        if current["has_tower"].as_bool().unwrap_or(false) {
            let tower_name = current["tower_name"].as_str().unwrap_or("Tower");
            fixed.push(FixedAction {
                id: "tower_enter".into(), action: "tower_enter".into(),
                category: "tower".into(), label: format!("Enter {}", tower_name),
                params: serde_json::json!({}), cooldown_ms: fixed_cd, enabled: true,
                disabled_reason: None,
            });
        }
        if current["has_exchange"].as_bool().unwrap_or(false) {
            fixed.push(FixedAction {
                id: "exchange".into(), action: "send_message".into(),
                category: "town".into(), label: "Visit Exchange".into(),
                params: serde_json::json!({ "content": "Visit the exchange" }),
                cooldown_ms: llm_cd, enabled: true, disabled_reason: None,
            });
        }
        if current["has_guild_hall"].as_bool().unwrap_or(false) {
            fixed.push(FixedAction {
                id: "guild_hall".into(), action: "send_message".into(),
                category: "town".into(), label: "Guild Hall".into(),
                params: serde_json::json!({ "content": "Visit the guild hall" }),
                cooldown_ms: llm_cd, enabled: true, disabled_reason: None,
            });
        }

        // ---- RESOURCE ACTIONS (always available) ----
        fixed.push(FixedAction {
            id: "gather".into(), action: "gather".into(),
            category: "resource".into(), label: "Gather Resources".into(),
            params: serde_json::json!({}), cooldown_ms: fixed_cd, enabled: true,
            disabled_reason: None,
        });
        fixed.push(FixedAction {
            id: "work".into(), action: "work".into(),
            category: "resource".into(), label: "Do Odd Jobs".into(),
            params: serde_json::json!({}), cooldown_ms: fixed_cd, enabled: true,
            disabled_reason: None,
        });
    }

    // ---- DUNGEON ACTIONS ----
    if !is_dead && !in_combat && in_dungeon {
        // TODO: extract dungeon room exits when dungeon data is available
        // For now, provide basic dungeon actions
        fixed.push(FixedAction {
            id: "dungeon_retreat".into(), action: "dungeon_retreat".into(),
            category: "dungeon".into(), label: "Retreat from Dungeon".into(),
            params: serde_json::json!({}), cooldown_ms: fixed_cd, enabled: true,
            disabled_reason: None,
        });
        fixed.push(FixedAction {
            id: "dungeon_status".into(), action: "dungeon_status".into(),
            category: "dungeon".into(), label: "Dungeon Status".into(),
            params: serde_json::json!({}), cooldown_ms: 0, enabled: true,
            disabled_reason: None,
        });
    }

    // ---- EQUIPMENT ACTIONS ----
    if !is_dead {
        // Equippable items in inventory
        for item in &adventure.inventory.items {
            if let Some(ref slot) = item.slot {
                let slot_name = slot.display_name();
                fixed.push(FixedAction {
                    id: format!("equip_{}", item.id),
                    action: "equip".into(),
                    category: "equipment".into(),
                    label: format!("Equip {}", item.display_name()),
                    params: serde_json::json!({ "item_name": item.name }),
                    cooldown_ms: 0, enabled: !in_combat,
                    disabled_reason: combat_reason.clone(),
                });
            }
        }
        // Unequip from each occupied slot
        let slots = [
            ("head", &adventure.equipment.head),
            ("amulet", &adventure.equipment.amulet),
            ("main_hand", &adventure.equipment.main_hand),
            ("off_hand", &adventure.equipment.off_hand),
            ("chest", &adventure.equipment.chest),
            ("hands", &adventure.equipment.hands),
            ("ring1", &adventure.equipment.ring1),
            ("ring2", &adventure.equipment.ring2),
            ("legs", &adventure.equipment.legs),
            ("feet", &adventure.equipment.feet),
            ("back", &adventure.equipment.back),
        ];
        for (slot_name, item_opt) in &slots {
            if let Some(item) = item_opt {
                fixed.push(FixedAction {
                    id: format!("unequip_{}", slot_name),
                    action: "unequip".into(),
                    category: "equipment".into(),
                    label: format!("Unequip {} ({})", item.display_name(), slot_name),
                    params: serde_json::json!({ "slot": slot_name }),
                    cooldown_ms: 0, enabled: !in_combat,
                    disabled_reason: combat_reason.clone(),
                });
            }
        }
    }

    // ---- COMBAT ACTIONS ----
    let combat_actions = if in_combat && !is_dead {
        let has_weapon = adventure.equipment.equipped_weapon().is_some();
        let has_potion = adventure.inventory.items.iter()
            .any(|i| matches!(i.item_type, crate::engine::inventory::ItemType::Potion));
        let available = adventure.combat.available_actions(
            &adventure.character, has_weapon, has_potion,
        );
        let enemy_names: Vec<String> = adventure.combat.enemies.iter()
            .filter(|e| e.hp > 0)
            .map(|e| e.name.clone())
            .collect();

        Some(available.into_iter().map(|a| {
            let targets = if a.id == "attack" { Some(enemy_names.clone()) } else { None };
            CombatActionInfo {
                id: a.id, name: a.name, cost: a.cost,
                description: a.description, enabled: a.enabled, targets,
            }
        }).collect())
    } else {
        None
    };

    ActionMenu {
        fixed_actions: fixed,
        combat_actions,
        llm_actions: None,
        state: state_json,
    }
}
