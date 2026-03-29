//! Tool definitions for the D&D engine.

use super::types::{ToolDef, ToolFunctionDef};

pub fn build_tool_definitions() -> Vec<ToolDef> {
    vec![
        tool("roll_dice", "Roll dice (for NPC/behind-the-scenes rolls only, not for player-facing rolls). Returns the actual random results.", serde_json::json!({
            "type": "object",
            "properties": {
                "dice_type": {"type": "string", "description": "Dice type, e.g. d4, d6, d8, d10, d12, d20, d100"},
                "count": {"type": "integer", "description": "Number of dice to roll", "default": 1},
                "modifier": {"type": "integer", "description": "Modifier to add to the total", "default": 0}
            },
            "required": ["dice_type"]
        })),
        tool("request_player_roll", "Request the player to roll dice. This pauses the story and shows a dice-rolling UI to the player. Use this for important checks where the player should feel the tension of the roll.", serde_json::json!({
            "type": "object",
            "properties": {
                "dice_type": {"type": "string", "description": "Dice type, e.g. d20"},
                "count": {"type": "integer", "description": "Number of dice", "default": 1},
                "modifier": {"type": "integer", "description": "Modifier", "default": 0},
                "dc": {"type": "integer", "description": "Difficulty class to beat"},
                "description": {"type": "string", "description": "Description of what this roll is for, e.g. 'Strength check to force open the door'"}
            },
            "required": ["dice_type", "dc", "description"]
        })),
        tool("ability_check", "Perform an ability check (automatic roll, not player-facing). Uses the character's stat modifier.", serde_json::json!({
            "type": "object",
            "properties": {
                "stat": {"type": "string", "description": "Stat to check: str, dex, con, int, wis, cha"},
                "dc": {"type": "integer", "description": "Difficulty class"},
                "description": {"type": "string", "description": "What the check is for"}
            },
            "required": ["stat", "dc"]
        })),
        tool("saving_throw", "Perform a saving throw (automatic roll). Uses the character's stat modifier.", serde_json::json!({
            "type": "object",
            "properties": {
                "stat": {"type": "string", "description": "Stat for save: str, dex, con, int, wis, cha"},
                "dc": {"type": "integer", "description": "Difficulty class"},
                "description": {"type": "string", "description": "What the save is for"}
            },
            "required": ["stat", "dc"]
        })),
        tool("attack_roll", "Make an attack roll with a weapon against a target.", serde_json::json!({
            "type": "object",
            "properties": {
                "weapon": {"type": "string", "description": "Name of weapon in inventory"},
                "target": {"type": "string", "description": "Name of enemy target"}
            },
            "required": ["weapon", "target"]
        })),
        tool("get_character_sheet", "Get the player's full character sheet including stats, HP, level, etc.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("update_hp", "Change the player's HP (positive for healing, negative for damage).", serde_json::json!({
            "type": "object",
            "properties": {
                "delta": {"type": "integer", "description": "HP change (positive = heal, negative = damage)"},
                "reason": {"type": "string", "description": "Reason for HP change"}
            },
            "required": ["delta", "reason"]
        })),
        tool("add_item", "Add an item to the player's inventory.", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Item name"},
                "description": {"type": "string", "description": "Item description"},
                "item_type": {"type": "string", "enum": ["weapon", "armor", "potion", "scroll", "misc"]},
                "properties": {"type": "object", "description": "Item-specific properties (damage, ac_bonus, etc.)"},
                "weight": {"type": "number", "description": "Weight in pounds", "default": 1.0}
            },
            "required": ["name", "description", "item_type"]
        })),
        tool("remove_item", "Remove an item from the player's inventory.", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Item name to remove"}
            },
            "required": ["name"]
        })),
        tool("use_ability", "Use a class ability or spell.", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Ability name"},
                "target": {"type": "string", "description": "Target of the ability"}
            },
            "required": ["name"]
        })),
        tool("award_xp", "Award experience points to the player.", serde_json::json!({
            "type": "object",
            "properties": {
                "amount": {"type": "integer", "description": "XP amount"},
                "reason": {"type": "string", "description": "Reason for XP award"}
            },
            "required": ["amount", "reason"]
        })),
        tool("present_choices", "Present the player with a set of choices. You MUST call this when it's the player's turn to act.", serde_json::json!({
            "type": "object",
            "properties": {
                "choices": {"type": "array", "items": {"type": "string"}, "description": "1-6 choices for the player", "maxItems": 6},
                "allow_custom_input": {"type": "boolean", "description": "Whether the player can also type a custom action", "default": false},
                "prompt": {"type": "string", "description": "The prompt/question to display above the choices", "default": "What do you do?"}
            },
            "required": ["choices"]
        })),
        tool("set_scene", "Set the current scene/location. Updates the scene info shown to the player.", serde_json::json!({
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "Location name"},
                "description": {"type": "string", "description": "Scene description"}
            },
            "required": ["location", "description"]
        })),
        tool("start_combat", "Start a combat encounter with enemies.", serde_json::json!({
            "type": "object",
            "properties": {
                "enemies": {"type": "array", "items": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "hp": {"type": "integer"},
                        "ac": {"type": "integer"},
                        "attacks": {"type": "array", "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "damage_dice": {"type": "string"},
                                "damage_modifier": {"type": "integer"},
                                "to_hit_bonus": {"type": "integer"}
                            }
                        }}
                    },
                    "required": ["name", "hp"]
                }}
            },
            "required": ["enemies"]
        })),
        tool("end_combat", "End the current combat encounter.", serde_json::json!({
            "type": "object",
            "properties": {
                "xp_reward": {"type": "integer", "description": "XP to award for the combat", "default": 0}
            }
        })),
        tool("add_quest", "Add a quest to the player's quest log.", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Quest name"},
                "description": {"type": "string", "description": "Quest description"}
            },
            "required": ["name", "description"]
        })),
        tool("complete_quest", "Mark a quest as completed.", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Quest name to complete"}
            },
            "required": ["name"]
        })),
        tool("add_condition", "Apply a status condition to the player (e.g., poisoned, blinded, frightened, stunned, paralyzed, exhaustion, burning, bleeding). ALWAYS use this when the player is affected by a status effect.", serde_json::json!({
            "type": "object",
            "properties": {
                "condition": {"type": "string", "description": "Condition name (e.g., 'Poisoned', 'Blinded', 'Frightened')"},
                "duration": {"type": "string", "description": "How long it lasts (e.g., '1 hour', '3 rounds', 'until cured')", "default": "until cured"}
            },
            "required": ["condition"]
        })),
        tool("remove_condition", "Remove a status condition from the player (e.g., when poison is cured, fear ends, etc.).", serde_json::json!({
            "type": "object",
            "properties": {
                "condition": {"type": "string", "description": "Condition name to remove"}
            },
            "required": ["condition"]
        })),
    ]
}

fn tool(name: &str, description: &str, parameters: serde_json::Value) -> ToolDef {
    ToolDef {
        tool_type: "function".to_string(),
        function: ToolFunctionDef {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
        },
    }
}
