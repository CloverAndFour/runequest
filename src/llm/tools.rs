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
        tool("attack_roll", "Make an attack roll with the equipped main-hand weapon against a target. Uses the weapon currently in the Main Hand slot.", serde_json::json!({
            "type": "object",
            "properties": {
                "target": {"type": "string", "description": "Name of enemy target"}
            },
            "required": ["target"]
        })),
        tool("get_character_sheet", "Get the player's full character sheet including stats, HP, level, equipment, and gold.", serde_json::json!({
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
        tool("add_item", "Add a custom item to the player's inventory (backpack). For standard items, prefer give_item with an item_id from the database.", serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Item name"},
                "description": {"type": "string", "description": "Item description"},
                "item_type": {"type": "string", "enum": ["weapon", "armor", "potion", "scroll", "misc"]},
                "weight": {"type": "number", "description": "Weight in pounds", "default": 1.0}
            },
            "required": ["name", "description", "item_type"]
        })),
        tool("give_item", "Give the player a standard item from the item database by ID. The item goes into their inventory (backpack). The player manages their own equipment via the UI. Standard IDs include: longsword, shortsword, dagger, mace, quarterstaff, battleaxe, rapier, greatsword, greataxe, longbow, shortbow, light_crossbow, heavy_crossbow, leather_armor, studded_leather, chain_shirt, chain_mail, scale_mail, plate_armor, shield, health_potion, greater_health_potion, ring_of_protection, cloak_of_protection, boots_of_speed, gauntlets_of_ogre_power, amulet_of_health, flametongue_longsword, frostbrand_greatsword, vorpal_longsword, etc.", serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": {"type": "string", "description": "Item ID from the standard database (e.g. 'longsword', 'health_potion', 'ring_of_protection')"},
                "quantity": {"type": "integer", "description": "Number of items to give (for stackable items like potions)", "default": 1}
            },
            "required": ["item_id"]
        })),
        tool("give_gold", "Award gold pieces to the player.", serde_json::json!({
            "type": "object",
            "properties": {
                "amount": {"type": "integer", "description": "Number of gold pieces to give"},
                "reason": {"type": "string", "description": "Reason for gold award (e.g. 'loot from goblin', 'quest reward')"}
            },
            "required": ["amount"]
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
        tool("set_scene", "Update the scene description/atmosphere shown to the player. This does NOT change the player's location — location is controlled by the engine. Use this for narrative flavor only (e.g. describing the inside of a building, a campfire scene).", serde_json::json!({
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "Optional sub-location label for narrative (e.g. 'Inside the tavern', 'By the river'). Does NOT change the player's actual hex world position."},
                "description": {"type": "string", "description": "Scene description"}
            },
            "required": ["description"]
        })),
        tool("start_combat", "Start a combat encounter. The ENGINE generates tier-appropriate enemies — do NOT specify hp, ac, or attacks. Just say what type and how many.", serde_json::json!({
            "type": "object",
            "properties": {
                "enemy_type": {"type": "string", "enum": ["brute", "skulker", "mystic", "undead", "random"], "description": "Type of enemy. brute=tanky melee, skulker=evasive striker, mystic=magic user, undead=undead, random=engine picks."},
                "count": {"type": "integer", "description": "Number of enemies (1-6). Default 1.", "minimum": 1, "maximum": 6}
            }
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
        tool("move_to_room", "Move the player to an adjacent room in the dungeon via a named exit direction. This handles combat triggers, traps, and scene updates automatically. Always check the current room's available exits before calling this.", serde_json::json!({
            "type": "object",
            "properties": {
                "direction": {"type": "string", "description": "Exit direction to move through (e.g. 'North', 'East', 'South', 'West', 'Descend', 'Ascend')"}
            },
            "required": ["direction"]
        })),
        tool("search_room", "Search the current dungeon room for treasure and hidden items. Uses a WIS check (DC scales with floor level). Each room can only be searched once.", serde_json::json!({
            "type": "object",
            "properties": {}
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
        // -------------------------------------------------------------------
        // World map tools
        // -------------------------------------------------------------------
        tool("travel_to", "Travel to an adjacent hex county by compass direction. Valid directions: east, west, northeast, northwest, southeast, southwest. May trigger random encounters based on area tier.", serde_json::json!({
            "type": "object",
            "properties": {
                "direction": {"type": "string", "enum": ["east", "west", "northeast", "northwest", "southeast", "southwest"], "description": "Compass direction to travel"}
            },
            "required": ["direction"]
        })),
        tool("enter_dungeon", "Enter the dungeon at the player's current location. The location must be a Dungeon type. This generates the dungeon layout if it hasn't been visited before.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("exit_dungeon", "Exit the current dungeon and return to the world map at the dungeon's location.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("enter_tower", "Enter The Endless Tower. The player must be at The Endless Tower location. Each floor is a procedurally generated dungeon that gets harder as you ascend.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("tower_ascend", "Ascend to the next floor of The Endless Tower. Generates a new, harder dungeon floor.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("exit_tower", "Exit The Endless Tower and return to the world map. Progress is saved — the player can return later.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("view_shop", "View the shop inventory at the current town. Shows all available items, prices, and stock.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("buy_item", "Buy an item from the shop at the current town. Requires sufficient gold.", serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": {"type": "string", "description": "Item ID to purchase (e.g. 'health_potion', 'longsword')"}
            },
            "required": ["item_id"]
        })),
        tool("sell_item", "Sell an item from inventory at the current town. Items sell for half their base value.", serde_json::json!({
            "type": "object",
            "properties": {
                "item_name": {"type": "string", "description": "Name of the item in inventory to sell"}
            },
            "required": ["item_name"]
        })),
        // -------------------------------------------------------------------
        // Crafting tools
        // -------------------------------------------------------------------
        tool("craft_item", "Craft an item using a recipe. Requires: (1) sufficient crafting skill rank, (2) a crafting station at the current town that supports the skill and tier, (3) all input materials in inventory. On success, inputs are consumed and the output is added to inventory. There is a chance to gain crafting skill XP.", serde_json::json!({
            "type": "object",
            "properties": {
                "recipe_id": {"type": "string", "description": "Recipe ID to craft (e.g. 'cured_leather', 'iron_ingot')"}
            },
            "required": ["recipe_id"]
        })),
        tool("list_recipes", "List available crafting recipes. Filters by player skill rank and available crafting stations at the current location. Optionally filter by skill or tier.", serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {"type": "string", "description": "Filter by crafting skill ID (e.g. 'leatherworking', 'smithing')"},
                "tier": {"type": "integer", "description": "Filter by recipe tier (1-10)"}
            }
        })),
        tool("gather", "Gather raw materials from the current biome. Picks 1-3 random T0 materials based on the biome type (forest gives wood/herbs, mountains give stone/metal, etc.). Awards 5-10 Survival XP. No crafting station or skill rank required.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        // -------------------------------------------------------------------
        // Skill tools
        // -------------------------------------------------------------------
        tool("award_skill_xp", "Award XP to a specific skill. Skills rank up automatically when XP threshold is reached.", serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {"type": "string", "description": "Skill ID (e.g. 'weapon_mastery', 'stealth', 'leatherworking')"},
                "amount": {"type": "integer", "description": "XP amount to award"}
            },
            "required": ["skill_id", "amount"]
        })),
        tool("get_skills", "Return the player's full skill set with ranks, XP, and XP thresholds.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("improve_skill", "Directly improve a skill by 1 rank (bypass XP). Use sparingly for quest rewards or special events.", serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {"type": "string", "description": "Skill ID to improve"}
            },
            "required": ["skill_id"]
        })),
        // -------------------------------------------------------------------
        // Map and social tools
        // -------------------------------------------------------------------
        tool("get_map_info", "Get hex map info for the player's current position including neighbors, biome, tier, and points of interest.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("flag_murderer", "Flag the player as a murderer. Town guards will become hostile. Use when the player kills an innocent NPC.", serde_json::json!({
            "type": "object",
            "properties": {}
        })),
        tool("check_murderer", "Check if the player is flagged as a murderer.", serde_json::json!({
            "type": "object",
            "properties": {}
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
