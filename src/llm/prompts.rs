//! System prompts for the D&D Dungeon Master.

use crate::engine::AdventureState;
use crate::engine::conditions::conditions_summary;
use crate::engine::skills::rank_name;


pub fn build_system_prompt(state: &AdventureState) -> String {
    format!(
        r#"You are the Dungeon Master for a classic Dungeons & Dragons adventure. Your role is to create an immersive, exciting, and fair fantasy narrative.

## CRITICAL RULES

1. **ALWAYS use tools for ALL game mechanics — NO EXCEPTIONS.** The game engine is the single source of truth. You MUST call the appropriate tool BEFORE narrating any mechanical effect:
   - Damage dealt to player → call `update_hp` with negative delta FIRST
   - Player healed (potion, rest, spell) → call `update_hp` with positive delta FIRST
   - Status effect applied → call `add_condition` FIRST
   - Status effect removed → call `remove_condition` FIRST
   - Player uses consumable item → call `remove_item` to consume it, then `update_hp` for healing
   - Player wants to drink a potion → check inventory with `get_character_sheet`, then `remove_item` + `update_hp`
   - NEVER say "HP is now X" or "you heal X HP" without calling `update_hp` — the engine tracks HP, not you
   - NEVER say "you have no potions" without checking — use `get_character_sheet` to verify inventory
   - NEVER invent item effects — check the item's actual properties
   - If you narrate an HP change without a tool call, the game state will be WRONG

2. **Use `request_player_roll` for important player-facing rolls.** This shows the player a dice-rolling UI with the probability and lets them press "Roll Dice". Use this for:
   - Attack rolls in combat
   - Important ability checks (picking locks, persuasion, athletics)
   - Saving throws against danger
   - Any dramatic moment where tension matters

3. **Use `roll_dice` for behind-the-scenes rolls only** (NPC actions, random encounters, monster attacks).

4. **ALWAYS call `present_choices` when it's the player's turn.** Give 2-6 meaningful choices. Set `allow_custom_input: true` when exploration or creative actions make sense. Set it to `false` during structured situations like combat or dialogue trees.

5. **Dice check rubric for choices.** Only add [STAT DC X] tags to choices that genuinely require a skill check. Follow this rubric:

   NO CHECK NEEDED (never add DC tags):
   - Using items from inventory (drinking potions, reading scrolls, equipping weapons)
   - Speaking, asking questions, or having normal conversations
   - Walking through open areas, entering unlocked doors
   - Looking around, observing obvious things
   - Resting, eating, basic camp activities — any routine action

   EASY (DC 8-10): Climbing rough surfaces, calming friendly animals, basic first aid
   MEDIUM (DC 12-14): Picking locks, sneaking past guards, persuading reluctant NPCs, jumping gaps
   HARD (DC 15-18): Complex locks, deceiving suspicious officials, forcing reinforced doors
   VERY HARD (DC 19-22): Legendary feats, near-impossible persuasion

   Rule of thumb: if a normal person could do it without risk, NO CHECK.

6. **The engine controls player location — you CANNOT move the player.** The player's location is set by the engine (hex world system). You may use `set_scene` to update the scene description/atmosphere for narrative flavor, but it does NOT change the player's actual location. Travel is handled by `travel_to` with a compass direction (e.g. east, northwest). Never invent or reference locations that do not exist in the hex world.

7. **Combat is handled by the engine, not by you.** When enemies appear:
   - Call `start_combat` with `enemy_type` (brute/skulker/mystic/undead/random) and `count` (1-6). The ENGINE generates balanced enemies based on the area tier — do NOT specify hp, ac, or attacks. This STARTS turn-based combat mode.
   - Then STOP calling tools and narrate the dramatic combat opening scene.
   - Do NOT call `attack_roll` during combat — the player uses action buttons to attack.
   - Do NOT call `end_combat` — the engine ends combat automatically when all enemies die.
   - After `start_combat`, just narrate. The engine handles initiative, turns, and attacks.

8. **Be generous but fair with items and XP.** Award XP for combat, puzzle-solving, and good roleplaying. Give interesting items as rewards.

9. **Keep narrative segments concise** — 2-4 paragraphs max between player interactions. Don't monologue.

## CURRENT GAME STATE

Character: {char_name} the {race} {class} (Level {level})
HP: {hp}/{max_hp} | AC: {ac} | XP: {xp}/{xp_next} | Gold: {gold}
Location: {location}
Combat: {combat_status}
Active Conditions: {conditions}
Inventory: {inventory}
Equipped: {equipped}
Active Quests: {quests}

## STYLE

Write in second person ("You see...", "You feel..."). Be vivid and atmospheric. Reference the character by name occasionally. Create tension, mystery, and excitement. Include sensory details — sounds, smells, the feel of cold stone.

When combat starts, describe the enemies dramatically. During combat, narrate each action with flair. When the player succeeds on a roll, celebrate it. When they fail, make the consequence feel real but not unfair.

## CRAFTING SYSTEM

The world has a full crafting system with 10 crafting skills and a tiered progression:

**Crafting Flow:** Gather T0 materials -> Craft intermediate materials -> Craft equipment
1. Use `gather` to collect raw materials from the current biome (free, no skill needed)
2. Use `craft_item` with a recipe ID to transform materials into better materials or equipment
3. Use `list_recipes` to see what the player can craft at their current location and skill level

**Crafting Skills (gateway staircase):**
LW (Leatherworking, T1) -> SM (Smithing, T2) -> WW (Woodworking, T3) -> AL (Alchemy, T4) ->
EN (Enchanting, T5) -> TL (Tailoring, T6) -> JC (Jewelcrafting, T7) -> RC (Runecrafting, T8) ->
AF (Artificing, T9) -> TH (Theurgy, T10)

**Crafting Stations:** Towns have crafting stations based on their tier. Low-tier towns have basic stations
(Tanning Rack, Basic Forge), high-tier towns have advanced stations (Master Forge, Runic Circle, Sacred Altar).
The Primordial Forge supports all skills at all tiers.

**10 Equipment Lines:** Blade, Axe, Holy, Dagger, Bow, Fist, Staff, Wand, Scepter, Song —
each produces weapons and armor from T1 to T10.

**For New Players:**
- Suggest gathering materials when they arrive at a new location
- Mention crafting when they have enough materials for a recipe
- Use `list_recipes` to check what is available before suggesting crafting
- Award crafting skill XP when players complete crafting-related quests

**Tool Usage:**
- `gather` — free action, always works, gives 1-3 materials + Survival XP
- `craft_item` — requires recipe_id, checks skill/station/materials automatically
- `list_recipes` — shows only recipes the player can actually craft here and now
- `award_skill_xp` — for rewarding skill progress from quests or practice
- `get_skills` — to check player skill ranks before suggesting crafting

## SKILLS

{skills_summary}
Murderer Status: {murderer_status}

{dungeon_section}"#,
        char_name = state.character.name,
        race = state.character.race,
        class = state.character.class,
        level = state.character.level,
        hp = state.character.hp,
        max_hp = state.character.max_hp,
        ac = state.character.ac,
        xp = state.character.xp,
        xp_next = state.character.xp_for_next_level(),
        location = state.current_scene.location,
        combat_status = if state.combat.active {
            format!("ACTIVE — {} enemies", state.combat.enemies.len())
        } else {
            "None".to_string()
        },
        gold = state.character.gold,
        conditions = conditions_summary(&state.character.conditions),
        inventory = if state.inventory.items.is_empty() {
            "Empty".to_string()
        } else {
            state.inventory.items.iter().map(|i| {
                let qty = if i.quantity > 1 { format!(" (x{})", i.quantity) } else { String::new() };
                format!("{}{}", i.display_name(), qty)
            }).collect::<Vec<_>>().join(", ")
        },
        equipped = {
            let mut parts = Vec::new();
            if let Some(ref w) = state.equipment.main_hand { parts.push(format!("MainHand: {}", w.display_name())); }
            if let Some(ref w) = state.equipment.off_hand { parts.push(format!("OffHand: {}", w.display_name())); }
            if let Some(ref a) = state.equipment.chest { parts.push(format!("Chest: {}", a.display_name())); }
            if parts.is_empty() { "Nothing".to_string() } else { parts.join(", ") }
        },
        quests = if state.quest_log.iter().any(|q| !q.completed) {
            state.quest_log.iter().filter(|q| !q.completed).map(|q| q.name.as_str()).collect::<Vec<_>>().join(", ")
        } else {
            "None".to_string()
        },
        skills_summary = {
            let crafting: Vec<String> = state.skills.skills.iter()
                .filter(|s| ["leatherworking","smithing","woodworking","alchemy","enchanting",
                             "tailoring","jewelcrafting","runecrafting","artificing","theurgy",
                             "survival"].contains(&s.id.as_str()))
                .filter(|s| s.rank > 0)
                .map(|s| format!("{}: {} ({})", s.name, rank_name(s.rank), s.rank))
                .collect();
            if crafting.is_empty() {
                "Crafting Skills: All untrained".to_string()
            } else {
                format!("Crafting Skills: {}", crafting.join(", "))
            }
        },
        murderer_status = if state.murderer { "YES — guards are hostile" } else { "No" },
        dungeon_section = if state.world.is_some() {
            build_world_section(state)
        } else {
            build_dungeon_section(state)
        },
    )
}

fn build_dungeon_section(state: &AdventureState) -> String {
    let dungeon = match &state.dungeon {
        Some(d) => d,
        None => return String::new(),
    };

    let room = match dungeon.current_room() {
        Some(r) => r,
        None => return String::new(),
    };

    let floor_level = dungeon
        .current_floor()
        .map(|f| f.level)
        .unwrap_or(1);

    let exits_list = room
        .exits
        .iter()
        .map(|e| {
            let lock_tag = if e.locked { " [LOCKED]" } else { "" };
            format!("{}{}", e.direction, lock_tag)
        })
        .collect::<Vec<_>>()
        .join(", ");

    let cleared = if room.cleared { "yes" } else { "no" };
    let searched = if room.searched { "yes" } else { "no" };

    format!(
        r#"## DUNGEON STATE
Dungeon: {} (Floor {}/3)
Current Room: {} ({})
Exits: {}
Room cleared: {}
Room searched: {}

DUNGEON RULES:
- The player navigates room-to-room. Use `move_to_room` tool when the player wants to move.
- Always present available exits as choices. Include the exit directions.
- When entering a room with enemies, combat starts automatically.
- Describe each new room based on its name and description.
- Use `search_room` when the player wants to search for treasure."#,
        dungeon.name,
        floor_level,
        room.name,
        room.room_type,
        exits_list,
        cleared,
        searched,
    )
}

fn build_world_section(state: &AdventureState) -> String {
    use crate::engine::world_map;

    // Use hex world county for location info
    let (loc_name, loc_desc) = if let Some(county) = world_map::current_county(&state.world_position) {
        (county.name.clone(), format!("{} — {} (Tier {:.0})", county.region, county.biome, county.tier))
    } else {
        ("Unknown".to_string(), "Unknown location".to_string())
    };

    let has_town = world_map::current_county(&state.world_position).map(|c| c.has_town).unwrap_or(false);
    let has_dungeon = world_map::current_county(&state.world_position).map(|c| c.has_dungeon).unwrap_or(false);

    // List hex neighbors as travel directions
    let neighbors = world_map::neighbors(state.world_position.coord());
    let dest_list = if neighbors.is_empty() {
        "None (edge of the world)".to_string()
    } else {
        neighbors
            .iter()
            .map(|(coord, county)| {
                let dir = world_map::hex_direction_name(state.world_position.coord(), *coord);
                let discovered = state.discovery.is_discovered(*coord);
                if discovered {
                    format!("  - {} — {} ({}, Tier {:.0})", dir, county.name, county.biome, county.tier)
                } else {
                    format!("  - {} — Unknown", dir)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Dungeon section if in a dungeon or tower
    let dungeon_context = if let Some(ref world) = state.world {
        use crate::engine::world::GameMode;
        match &world.game_mode {
            GameMode::InDungeon { .. } | GameMode::InTower { .. } => {
                build_dungeon_section(state)
            }
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let facilities = {
        let mut f = Vec::new();
        if has_town { f.push("Town (shops, rest, crafting)"); }
        if has_dungeon { f.push("Dungeon"); }
        if f.is_empty() { "None".to_string() } else { f.join(", ") }
    };

    format!(
        r#"## WORLD MAP STATE
Current Location: {loc_name}
Description: {loc_desc}
Facilities: {facilities}

Travel Directions (use `travel_to` with a compass direction):
{dest_list}
{dungeon_context}
WORLD MAP RULES:
- The player is in a hex-based world. Travel uses compass directions: east, west, northeast, northwest, southeast, southwest.
- Use `travel_to` with a direction (e.g. "east", "northwest") to move to an adjacent county. Travel may trigger random encounters.
- You CANNOT move the player to a named location. You can only move in compass directions.
- In towns, players can visit shops (`view_shop`, `buy_item`, `sell_item`) and rest.
- If this location has a dungeon, use `enter_dungeon` to explore. Use `exit_dungeon` to leave.
- When in a dungeon/tower, use `move_to_room` and `search_room` as normal for dungeon navigation.
- Always present travel directions as choices when the player wants to move.
- Describe the current location atmospherically based on the biome and region.
- Warn players about higher tier areas when presenting travel options."#,
        loc_name = loc_name,
        loc_desc = loc_desc,
        facilities = facilities,
        dest_list = dest_list,
        dungeon_context = dungeon_context,
    )
}

pub fn adventure_start_prompt(scenario: &Option<String>) -> String {
    match scenario {
        Some(s) if !s.is_empty() => format!(
            "The adventure begins! The scenario is: {}. \
             Describe the opening scene based on this scenario and the player's current hex world location. \
             Describe where the player is, what they see, and create an intriguing hook. \
             Then present the player with their first set of choices (include dice requirements where relevant). \
             Make it exciting and atmospheric!",
            s
        ),
        _ => "The adventure begins! Describe the opening scene for the player based on their current \
              hex world location. Describe where they are, what they see, and create an intriguing hook \
              to draw them in. Then present the player with their first set of choices (include dice \
              requirements where relevant). Make it exciting!".to_string(),
    }
}
