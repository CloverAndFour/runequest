//! Action rate limiting — per-character cooldowns for game actions.

use chrono::{DateTime, Utc};

/// LLM actions via REST API: 6 seconds.
pub const LLM_COOLDOWN_MS: u64 = 6000;
/// Fixed actions via REST API: 4 seconds.
pub const FIXED_COOLDOWN_MS: u64 = 4000;
/// LLM actions via browser/WebSocket: 1 second.
pub const WS_LLM_COOLDOWN_MS: u64 = 1000;
/// Fixed actions via browser/WebSocket: 1 second.
pub const WS_FIXED_COOLDOWN_MS: u64 = 1000;
/// Equipment changes via REST API: 100ms.
pub const EQUIP_COOLDOWN_MS: u64 = 100;
/// Equipment changes via browser/WebSocket: 100ms.
pub const WS_EQUIP_COOLDOWN_MS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCategory {
    /// Triggers LLM call (send_message, select_choice, combat_action).
    Llm,
    /// Engine-only mutation (gather, craft, shop buy/sell, dungeon/tower moves).
    Fixed,
    /// Equipment changes (equip/unequip) — very short cooldown.
    Equipment,
    /// Read-only or admin — not rate-limited.
    ReadOnly,
}

/// Classify a WebSocket action tag (serde snake_case name) into its rate limit category.
pub fn classify_action(tag: &str) -> ActionCategory {
    match tag {
        // LLM actions
        "send_message" | "select_choice" | "combat_action" => ActionCategory::Llm,

        // Fixed/engine actions
        "gather" | "craft_item" | "shop_buy" | "shop_sell" | "travel"
        | "dungeon_move" | "dungeon_enter" | "dungeon_skill_check"
        | "dungeon_activate_point" | "dungeon_retreat"
        | "tower_move" | "tower_enter" | "tower_ascend" | "tower_teleport" => ActionCategory::Fixed,

        "equip_item" | "unequip_item" => ActionCategory::Equipment,

        // Everything else: read-only, admin, social
        _ => ActionCategory::ReadOnly,
    }
}

/// Check cooldown for WebSocket (browser) connections — shorter cooldowns.
pub fn check_cooldown_ws(
    category: ActionCategory,
    last_llm_at: Option<DateTime<Utc>>,
    last_fixed_at: Option<DateTime<Utc>>,
) -> Result<(), u64> {
    let (last_at, cooldown_ms) = match category {
        ActionCategory::Llm => (last_llm_at, WS_LLM_COOLDOWN_MS),
        ActionCategory::Fixed => (last_fixed_at, WS_FIXED_COOLDOWN_MS),
        ActionCategory::Equipment => (last_fixed_at, WS_EQUIP_COOLDOWN_MS),
        ActionCategory::ReadOnly => return Ok(()),
    };

    if let Some(last) = last_at {
        let elapsed_ms = Utc::now()
            .signed_duration_since(last)
            .num_milliseconds()
            .max(0) as u64;
        if elapsed_ms < cooldown_ms {
            return Err(cooldown_ms - elapsed_ms);
        }
    }
    Ok(())
}

/// Compute remaining WS cooldown milliseconds for both categories.
pub fn remaining_cooldowns_ws(
    last_llm_at: Option<DateTime<Utc>>,
    last_fixed_at: Option<DateTime<Utc>>,
) -> (u64, u64) {
    let now = Utc::now();
    let remaining = |last: Option<DateTime<Utc>>, cooldown_ms: u64| -> u64 {
        match last {
            Some(t) => {
                let elapsed = now.signed_duration_since(t).num_milliseconds().max(0) as u64;
                cooldown_ms.saturating_sub(elapsed)
            }
            None => 0,
        }
    };
    (
        remaining(last_llm_at, WS_LLM_COOLDOWN_MS),
        remaining(last_fixed_at, WS_FIXED_COOLDOWN_MS),
    )
}

/// Check cooldown for REST API connections — longer cooldowns.
/// Returns `Ok(())` if allowed, `Err(remaining_ms)` if still on cooldown.
pub fn check_cooldown(
    category: ActionCategory,
    last_llm_at: Option<DateTime<Utc>>,
    last_fixed_at: Option<DateTime<Utc>>,
) -> Result<(), u64> {
    let (last_at, cooldown_ms) = match category {
        ActionCategory::Llm => (last_llm_at, LLM_COOLDOWN_MS),
        ActionCategory::Fixed => (last_fixed_at, FIXED_COOLDOWN_MS),
        ActionCategory::Equipment => (last_fixed_at, EQUIP_COOLDOWN_MS),
        ActionCategory::ReadOnly => return Ok(()),
    };

    if let Some(last) = last_at {
        let elapsed_ms = Utc::now()
            .signed_duration_since(last)
            .num_milliseconds()
            .max(0) as u64;
        if elapsed_ms < cooldown_ms {
            return Err(cooldown_ms - elapsed_ms);
        }
    }
    Ok(())
}

/// Compute remaining cooldown milliseconds for both categories.
pub fn remaining_cooldowns(
    last_llm_at: Option<DateTime<Utc>>,
    last_fixed_at: Option<DateTime<Utc>>,
) -> (u64, u64) {
    let now = Utc::now();
    let remaining = |last: Option<DateTime<Utc>>, cooldown_ms: u64| -> u64 {
        match last {
            Some(t) => {
                let elapsed = now.signed_duration_since(t).num_milliseconds().max(0) as u64;
                cooldown_ms.saturating_sub(elapsed)
            }
            None => 0,
        }
    };
    (
        remaining(last_llm_at, LLM_COOLDOWN_MS),
        remaining(last_fixed_at, FIXED_COOLDOWN_MS),
    )
}

/// Stamp the cooldown timestamp on an adventure after a successful action.
pub fn stamp_cooldown(
    adventure: &mut super::adventure::AdventureState,
    category: ActionCategory,
) {
    let now = Utc::now();
    match category {
        ActionCategory::Llm => adventure.last_llm_action_at = Some(now),
        ActionCategory::Fixed | ActionCategory::Equipment => adventure.last_fixed_action_at = Some(now),
        ActionCategory::ReadOnly => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_classify_actions() {
        assert_eq!(classify_action("send_message"), ActionCategory::Llm);
        assert_eq!(classify_action("select_choice"), ActionCategory::Llm);
        assert_eq!(classify_action("combat_action"), ActionCategory::Llm);
        assert_eq!(classify_action("gather"), ActionCategory::Fixed);
        assert_eq!(classify_action("craft_item"), ActionCategory::Fixed);
        assert_eq!(classify_action("shop_buy"), ActionCategory::Fixed);
        assert_eq!(classify_action("dungeon_move"), ActionCategory::Fixed);
        assert_eq!(classify_action("equip_item"), ActionCategory::Equipment);
        assert_eq!(classify_action("unequip_item"), ActionCategory::Equipment);
        assert_eq!(classify_action("tower_ascend"), ActionCategory::Fixed);
        assert_eq!(classify_action("list_adventures"), ActionCategory::ReadOnly);
        assert_eq!(classify_action("get_skills"), ActionCategory::ReadOnly);
        assert_eq!(classify_action("view_shop"), ActionCategory::ReadOnly);
    }

    #[test]
    fn test_cooldown_none_means_allowed() {
        assert!(check_cooldown(ActionCategory::Llm, None, None).is_ok());
        assert!(check_cooldown(ActionCategory::Fixed, None, None).is_ok());
    }

    #[test]
    fn test_cooldown_recent_action_blocked() {
        let just_now = Utc::now();
        assert!(check_cooldown(ActionCategory::Llm, Some(just_now), None).is_err());
        assert!(check_cooldown(ActionCategory::Fixed, None, Some(just_now)).is_err());
    }

    #[test]
    fn test_cooldown_old_action_allowed() {
        let long_ago = Utc::now() - Duration::seconds(30);
        assert!(check_cooldown(ActionCategory::Llm, Some(long_ago), None).is_ok());
        assert!(check_cooldown(ActionCategory::Fixed, None, Some(long_ago)).is_ok());
    }

    #[test]
    fn test_readonly_always_allowed() {
        let just_now = Utc::now();
        assert!(check_cooldown(ActionCategory::ReadOnly, Some(just_now), Some(just_now)).is_ok());
    }

    #[test]
    fn test_remaining_cooldowns() {
        let (llm, fixed) = remaining_cooldowns(None, None);
        assert_eq!(llm, 0);
        assert_eq!(fixed, 0);

        let just_now = Utc::now();
        let (llm, fixed) = remaining_cooldowns(Some(just_now), Some(just_now));
        assert!(llm > 5000 && llm <= 6000);
        assert!(fixed > 3000 && fixed <= 4000);
    }
}
