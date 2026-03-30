# RuneQuest Architecture

> **This document is the authoritative reference for the RuneQuest project.**
> Claude MUST keep this document up to date after every change.
> If you discover this document is inaccurate, STOP work, fix it, then resume.

Last updated: 2026-03-30

## Overview

RuneQuest is a D&D adventure chat system powered by Grok 4.1 (xAI). It features a turn-based combat system inspired by Baldur's Gate 3, procedural dungeons, a world map, and an OSRS-themed pixel art frontend.

**Dual-server architecture:** Two servers run simultaneously sharing the same backend:
- **Port 2999** -- Web UI (SPA + WebSocket)
- **Port 2998** -- REST API (JSON endpoints)

Both interfaces are fully playable. Every game action available in the UI must also be available via the API.

## Project Structure

```
runequest/
  Cargo.toml                    # Rust project (edition 2021)
  .env                          # XAI_API_KEY, ports, bind addr (gitignored)
  deploy/runequest.service      # systemd unit file
  ARCHITECTURE.md               # This file
  data/
    jwt_secret.key              # Auto-generated HS256 secret
    users.json                  # User credentials (argon2id hashed)
    usage.jsonl                 # LLM cost/token log
    users/<username>/adventures/<uuid>/
      state.json                # Full adventure state snapshot
      history.jsonl             # LLM conversation history
      display_history.jsonl     # UI event replay log
  src/
    main.rs                     # CLI: serve + user management
    lib.rs                      # Module declarations
    error.rs                    # Error types (thiserror)
    auth/
      mod.rs                    # Re-exports
      jwt.rs                    # JWT creation/validation (HS256, 24h)
      middleware.rs             # Auth middleware (Bearer token or ?token= query)
      user_store.rs             # User CRUD, argon2id password hashing
    engine/
      mod.rs                    # Re-exports
      npc.rs                    # NPC model (Npc, NpcType, NpcInteraction)
      backgrounds.rs            # Background system (10 backgrounds, starting skills/items/gold)
      skills.rs                 # Skill system (per-skill XP, ranks 0-10, 44 total skills)
      monsters.rs               # Tier-based monster generation
      tower.rs                  # Tower system (shared infinite dungeons)
      simulator.rs              # Combat balance simulator
      abilities.rs              # Class abilities, spell slots
      adventure.rs              # AdventureState, game loop, tool call dispatch
      character.rs              # Character creation, stats, leveling
      combat.rs                 # Turn-based combat system
      conditions.rs             # Status effects (poisoned, burning, etc.)
      dice.rs                   # Dice rolling, DC checks, probability calc
      dungeon.rs                # Procedural dungeon generation
      equipment.rs              # Item database, equipment slots, AC calculation
      executor.rs               # Tool call -> engine mutation mapping
      inventory.rs              # Inventory management
      world.rs                  # Legacy world map (20-location, kept for backwards compat)
      worldgen.rs               # 251K county hex world generator (fixed seed)
      world_map.rs              # Hex world runtime bridge (travel, shops, map_view builder)
    llm/
      mod.rs                    # Re-exports
      client.rs                 # xAI API client (streaming + non-streaming)
      pricing.rs                # Token cost calculation per model
      prompts.rs                # Dynamic system prompt builder
      tools.rs                  # Tool definitions for Grok function calling
      types.rs                  # LLM request/response types
    storage/
      mod.rs                    # Re-exports
      adventure_store.rs        # Adventure persistence (state + history)
      friends_store.rs          # Friend lists, requests, chat persistence
      usage_logger.rs           # Token/cost logging
      tower_store.rs            # Tower floor persistence
    web/
      mod.rs                    # Re-exports
      api_server.rs             # REST API server (port 2998)
      presence.rs               # Online presence registry
      party_registry.rs           # In-memory party/invite/PvP registry
      party_handler.rs            # Party/combat/PvP WebSocket handlers
      trade_registry.rs           # In-memory player-to-player trade registry
      protocol.rs               # ClientMsg / ServerMsg enums
      server.rs                 # Web server (port 2999)
      static_files.rs           # Static file serving
      websocket.rs              # WebSocket handler
  static/
    index.html                  # SPA shell
    login.html                  # Login page (standalone, inline JS)
    favicon.svg                 # D20 icon
    css/
      theme.css                 # CSS variables, medieval/OSRS theme
      adventure.css             # Gameplay layout, combat, maps
      components.css            # Shared components (buttons, toasts, modals)
    js/
      api.js                    # Token management, authFetch wrapper
      app.js                    # SPA controller, WebSocket message routing
      ws.js                     # WebSocket manager with auto-reconnect
      adventure.js              # Gameplay UI (story, stats, items, skills, map, quests, crafting)
      friends.js                # Friends panel UI (friend list, chat, requests)
      party.js                    # Party panel, combat timer, PvP UI
      location-chat.js            # Location public chat UI
      combat.js                 # Combat UI (initiative bar, actions, enemy HP)
      select.js                 # Adventure select + character creation (race + background)
  tests/
    login.spec.ts               # Auth tests (5 tests, browser)
    engine.spec.ts              # Engine/API tests (34 tests, port 2998)
    adventure.spec.ts           # UI tests (17 tests, port 2999)
    world.spec.ts               # World map tests (port 2998)
  assets/
    backgrounds/                # Scene backgrounds
    dice/                       # Dice images
    icons/                      # UI icons
    ui/                         # UI elements
```

## REST API Endpoints (Port 2998)

All protected routes require `Authorization: Bearer <JWT>` header.
CORS is fully open (any origin/method/headers).

### Authentication

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/auth/login` | No | `{username, password}` | `{token, username, role}` or 401 |
| `GET` | `/health` | No | -- | `"ok"` |

### Adventures

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `GET` | `/api/adventures` | Yes | -- | `{adventures: [AdventureSummary]}` |
| `POST` | `/api/adventures` | Yes | `{name, character_name, race, background?, class?, backstory?, stats?}` | `GameResponse` |
| `GET` | `/api/adventures/:id` | Yes | -- | `GameResponse` |
| `DELETE` | `/api/adventures/:id` | Yes | -- | `{deleted: true}` |
| `GET` | `/api/adventures/:id/history` | Yes | -- | `{events: [DisplayEvent]}` |

`stats` object (legacy class path only): `{strength, dexterity, constitution, intelligence, wisdom, charisma}` (8-15 each, 27-point buy)

`naked_start` (optional bool): Start with no equipment and 0 gold.

### Game Actions

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/adventures/:id/message` | Yes | `{content}` | `GameResponse` |
| `POST` | `/api/adventures/:id/choice` | Yes | `{index, text}` | `GameResponse` |
| `POST` | `/api/adventures/:id/roll` | Yes | -- | `GameResponse` |

### Combat

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/adventures/:id/combat` | Yes | `{action_id, target?}` | `GameResponse` |

`action_id` values: `attack`, `dodge`, `dash`, `use_item`, `flee`, `second_wind`, `cunning_hide`, `healing_word`, `reckless_attack`, `lay_on_hands`, `flurry_of_blows`, `end_turn`

### Equipment

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/adventures/:id/equip` | Yes | `{item_name}` | `{result, state}` |
| `POST` | `/api/adventures/:id/unequip` | Yes | `{slot}` | `{result, state}` |

### Direct Engine Endpoints (bypass LLM)

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/adventures/:id/engine/hp` | Yes | `{delta, reason}` | `GameResponse` |
| `POST` | `/api/adventures/:id/engine/item` | Yes | `{item_id}` | `GameResponse` |
| `POST` | `/api/adventures/:id/engine/gold` | Yes | `{amount}` | `GameResponse` |
| `POST` | `/api/adventures/:id/engine/xp` | Yes | `{amount, reason}` | `GameResponse` |
| `POST` | `/api/adventures/:id/engine/condition` | Yes | `{condition, action}` | `GameResponse` |
| `POST` | `/api/adventures/:id/engine/combat` | Yes | `{enemies: [{name, hp, ac, attacks}]}` | `GameResponse` |
| `POST` | `/api/adventures/:id/engine/roll` | Yes | `{dice, count?, modifier?, dc?}` | Roll result |
| `POST` | `/api/adventures/:id/engine/combat/simulate` | Yes | -- | `{simulation_log, combat_state}` |
| `POST` | `/api/adventures/:id/engine/skill` | Yes | `{action, skill_id?}` | `{result, state}` |

`action` for conditions: `"add"` or `"remove"`

Skill `action` values: `"get"` (list all skills) or `"improve"` (improve a skill by `skill_id`)

### Quest & NPC (Direct Engine)

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/adventures/:id/engine/quest` | Yes | `{action, ...params}` | `{result, state}` |
| `POST` | `/api/adventures/:id/engine/npc` | Yes | `{action, ...params}` | `{result, state}` |

Quest actions: `add` (name, description, final_goal, next_step, reward), `complete` (name), `update_step` (quest_name, step_completed, new_next_step), `fail` (name)

NPC actions: `create` (name, description, location?, disposition?), `update` (npc_name, location?, disposition?, description?), `dismiss` (npc_name), `list` (location?), `log_interaction` (npc_name, summary)

### Item Database

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `GET` | `/api/items` | Yes | -- | `{items: [Item]}` |
| `GET` | `/api/items/:id` | Yes | -- | `{item: Item}` |


### Crafting

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `GET` | `/api/recipes` | Yes | -- | `{recipes: [Recipe]}` |
| `GET` | `/api/recipes/:recipe_id` | Yes | -- | `Recipe` or 404 |
| `GET` | `/api/materials` | Yes | -- | `{materials: [Material]}` |
| `POST` | `/api/adventures/:id/craft` | Yes | `{recipe_id}` | `{result, state}` |

Query parameters for `/api/recipes`: `skill` (filter by skill ID), `tier` (filter by tier number).

Craft result contains: `{crafted, output, quantity, skill_progress}` on success, or `{error}` on failure.

Crafting checks: (1) skill rank requirement, (2) crafting station availability (player must be at a town with a station that supports the recipe's skill and tier), (3) material availability. On success, inputs are consumed, output is added to inventory, and there is a chance to improve the crafting skill (15% at-tier, 25% above-tier).

`list_recipes` is filtered by: (a) player's skill rank (only shows recipes at or below rank), (b) available crafting stations at current location.


### Dungeon

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/adventures/:id/dungeon/enter` | Yes | `{seed?, tier?}` | `{result, dungeon, state}` |
| `POST` | `/api/adventures/:id/dungeon/move` | Yes | `{direction}` | `{result, room, floor, room_id, state}` |
| `POST` | `/api/adventures/:id/dungeon/skill-check` | Yes | `{direction, skill_id}` | `{result, skill, roll, dc, success}` |
| `POST` | `/api/adventures/:id/dungeon/activate-point` | Yes | `{puzzle_id, room_id}` | `{result, puzzle_id, activated_count, required_count, solved}` |
| `POST` | `/api/adventures/:id/dungeon/retreat` | Yes | -- | `{result, message, state}` |
| `GET` | `/api/adventures/:id/dungeon/status` | Yes | -- | `{in_dungeon, name?, tier?, ...}` |

`direction` values: `"North"`, `"South"`, `"East"`, `"West"`, `"Descend"`, `"Ascend"`

### Tower

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `GET` | `/api/towers` | Yes | -- | `{towers: [{id, name, base_tier, ...}]}` |
| `GET` | `/api/towers/:tower_id/floor/:floor_num` | Yes | -- | `{floor: FloorSummary}` |
| `POST` | `/api/adventures/:id/tower/enter` | Yes | `{tower_id}` | `{result, tower_name, floor, tier, state}` |
| `POST` | `/api/adventures/:id/tower/move` | Yes | `{direction}` | Same as dungeon/move |
| `POST` | `/api/adventures/:id/tower/ascend` | Yes | -- | Same as dungeon/move (uses "Descend" direction internally) |
| `POST` | `/api/adventures/:id/tower/checkpoint` | Yes | `{floor}` | `{checkpoint_attuned, floor, teleport_cost}` |
| `POST` | `/api/adventures/:id/tower/teleport` | Yes | `{target_floor}` | `{teleport_available, target_floor, cost}` or error if insufficient gold |

`tower/enter` requires the player to be at a county with `has_tower: true`. The `tower_id` must match the tower at that county.

### GameResponse Format

```json
{
  "state": { /* full AdventureState + map_view */ },
  "narrative": "string or null",
  "pending": {
    "pending_type": "dice_roll" | "choices",
    // dice_roll: dice_type, count, modifier, dc, description, success_probability
    // choices: choices[], allow_custom_input, prompt
  },
  "combat": {
    "active": true,
    "round": 1,
    "current_turn": "player",
    "is_player_turn": true,
    "enemies": [{ "name", "hp", "max_hp", "ac" }],
    "available_actions": ["attack", "dodge", ...],
    "initiative_order": [{ "name", "initiative", "is_player" }]
  },
  "cost": { "prompt_tokens": 0, "completion_tokens": 0, "cost_usd": 0.0 }
}
```

#### `state.map_view` (injected into all state responses)

The `map_view` field is added to every state response (both REST and WebSocket). It provides a 3-ring hex neighborhood for the frontend hex grid map.

```json
{
  "hexes": [
    {
      "q": 0, "r": 0,           // relative coords (center = 0,0)
      "abs_q": 42, "abs_r": -217, // absolute world coords
      "current": true,
      "discovered": true,
      "name": "County Name",
      "tier": "2.0",
      "biome": "Forest",
      "has_town": true,
      "has_dungeon": false,
      "has_tower": false,
      "tower_name": null,
      "has_exchange": false,
      "has_guild_hall": false,
      "region": "Region Name"
    }
  ],
  "directions": [
    { "direction": "East", "name": "Neighbor County", "tier": "1.5", "biome": "Plains", "has_town": false }
  ],
  "current": {
    "name": "Current County", "tier": 2.0, "biome": "Forest", "region": "Region",
    "has_town": true, "has_dungeon": false, "has_tower": false, "tower_name": null,
    "has_exchange": false, "has_guild_hall": false,
    "stations": [
      {"type": "TanningRack", "name": "Tanning Rack", "max_tier": 3, "skills": ["leatherworking"]},
      {"type": "BasicForge", "name": "Basic Forge", "max_tier": 3, "skills": ["smithing"]}
    ]
  },
  "position": { "q": 42, "r": -217 }
}
```

Undiscovered hexes show `name: "???"`, `tier: "?"`, `biome: "unknown"`, and null for feature flags.

## WebSocket Protocol (Port 2999)

Connection: `ws://host:2999/ws?token=<JWT>`

### Client -> Server Messages

All messages are JSON with a `type` field (snake_case).

| Type | Fields | Description |
|------|--------|-------------|
| `list_adventures` | -- | Request adventure list |
| `create_adventure` | `name, character_name, race, background?, class?, backstory?` | Create new adventure |
| `load_adventure` | `adventure_id` | Load existing adventure |
| `delete_adventure` | `adventure_id` | Delete adventure |
| `send_message` | `content` | Free-text player input |
| `select_choice` | `index, text` | Respond to presented choices |
| `roll_dice` | -- | Execute pending dice roll |
| `get_character_sheet` | -- | Request state update |
| `get_inventory` | -- | Request state update |
| `get_quests` | -- | Request state update |
| `get_npcs` | -- | Request NPC list |
| `set_model` | `model` | Switch LLM model |
| `combat_action` | `action_id, target?, item_name?` | Combat action |
| `craft_item` | `recipe_id` | Craft an item using a recipe |
| `list_recipes` | `skill?, tier?` | List crafting recipes (optional filters) |
| `list_materials` | -- | List all crafting materials |

#### Dungeon

| Type | Fields | Description |
|------|--------|-------------|
| `dungeon_enter` | `seed?, tier?` | Enter a dungeon |
| `dungeon_move` | `direction` | Move to adjacent room |
| `dungeon_skill_check` | `direction, skill_id` | Attempt skill gate |
| `dungeon_activate_point` | `puzzle_id, room_id` | Activate puzzle point |
| `dungeon_retreat` | -- | Leave dungeon |
| `dungeon_status` | -- | Get dungeon state |

#### Tower

| Type | Fields | Description |
|------|--------|-------------|
| `tower_list` | -- | List all towers |
| `tower_enter` | `tower_id` | Enter a tower |
| `tower_move` | `direction` | Move in tower |
| `tower_ascend` | -- | Go to next floor |
| `tower_checkpoint` | `floor` | Attune checkpoint |
| `tower_teleport` | `target_floor` | Teleport to floor (costs gold) |
| `tower_floor_status` | `tower_id, floor` | Get floor info |


### Server -> Client Messages

| Type | Fields | Description |
|------|--------|-------------|
| `connected` | `username` | Auth confirmed |
| `adventure_list` | `adventures` | List of adventures |
| `adventure_loaded` | `state` | Adventure state loaded |
| `adventure_created` | `adventure_id, state` | New adventure created |
| `narrative_chunk` | `text` | Streaming narrative (80-byte chunks, 30ms delay) |
| `narrative_end` | -- | End of narrative stream |
| `dice_roll_request` | `dice_type, count, modifier, dc, description, success_probability` | Roll pending |
| `dice_roll_result` | `rolls, total, dc, success, description` | Roll outcome |
| `present_choices` | `choices, allow_custom_input, prompt` | Choices pending |
| `state_update` | `state` | Full state refresh |
| `cost_update` | `session_cost_usd, prompt_tokens, completion_tokens, today/week/month/total_cost_usd` | Cost stats |
| `condition_effects` | `effects` | Condition damage at turn start |
| `chat_history` | `entries` | Display history replay on load |
| `model_info` | `model, available_models` | Current model info |
| `combat_started` | `initiative_order, round` | Combat begins |
| `combat_turn_start` | `combatant, is_player, round, actions, bonus_actions, movement, available_actions, enemies` | Turn start |
| `combat_action_result` | `actor, action, description, roll?, hit?, damage?` | Action outcome |
| `combat_enemy_turn` | `enemy_name, attack_name, attack_roll, target_ac, hit, damage, player_hp, player_max_hp` | Enemy acts |
| `combat_ended` | `xp_reward, victory` | Combat ends |
| `state_changes` | `gold_delta, xp_delta, hp_delta, level_up, items_gained, items_lost, conditions_added, conditions_removed` | State diff after tool loop |
| `error` | `code, message` | Error message |

| `craft_result` | `recipe_name, output, quantity, skill_progress` | Crafting outcome |
| `recipe_list` | `recipes` | List of recipes |
| `material_list` | `materials` | List of materials |

#### Dungeon

| Type | Fields | Description |
|------|--------|-------------|
| `dungeon_entered` | `name, tier, floors, room` | Entered dungeon |
| `dungeon_room_changed` | `room, floor, room_id` | Moved to new room |
| `dungeon_skill_gate_result` | `skill, roll, dc, success` | Skill check result |
| `dungeon_puzzle_activation` | `puzzle_id, activated_count, required_count, solved` | Puzzle progress |
| `dungeon_retreated` | `message` | Left dungeon |
| `dungeon_status` | `status` | Current dungeon state |
| `corruption_tick` | `level, effects` | Corruption damage (T7+) |
| `path_cleared` | `path_index, mini_boss` | Split path cleared |
| `convergence_unlocked` | `convergence_room` | All paths cleared |
| `breach_warning` | `message` | Corruption breach warning |

#### Tower

| Type | Fields | Description |
|------|--------|-------------|
| `tower_list` | `towers[]` | Available towers |
| `tower_entered` | `tower_name, floor, tier` | Entered tower |
| `tower_floor_status` | `floor` | Floor details |
| `tower_player_nearby` | `player_name, room_x, room_y` | Nearby player |
| `tower_first_clear` | `tower, floor, player` | First clear achievement |

**Display Event Types:** narrative, choice_selected, dice_result, dice_roll_request, choices, user_message, combat_action, combat_enemy, combat_started, combat_ended, state_changes

### Fixed Actions Bar
When not in combat or a dungeon, the UI shows context-sensitive action buttons at the bottom of the story panel:
- Travel to connected locations
- Visit shop (if available)
- Enter dungeon/tower (if at dungeon/tower location)
- Talk to NPCs at current location

These send messages through the LLM for narrative context. The LLM is instructed not to duplicate these options in present_choices.

### Client Settings (localStorage)
Settings are stored in `localStorage` under key `rq_settings`:
- `punctuationHighlight` (bool, default false): Color-code sentence-ending punctuation for readability

### Loading States

A D20 spinner is shown:
- On initial adventure load
- After selecting a choice (while waiting for LLM response)
- After submitting custom input
- After clicking a fixed action

The spinner is automatically removed when the first narrative chunk arrives.

## Game Engine

### Character System

**Races:** Human, Elf, Dwarf, Orc, Halfling, Gnome, Dragonborn, Faefolk, Goblin, Revenant
**Classes (legacy):** Warrior, Mage, Rogue, Cleric, Ranger, Berserker, Paladin, Monk, Warlock, Bard
**Backgrounds (new):** Farmhand, Apprentice Smith, Street Urchin, Hunter, Acolyte, Scholar, Merchant, Herbalist, Woodcutter, Drifter
**Stats:** STR, DEX, CON, INT, WIS, CHA (default all-10 for background system; point-buy for legacy class system)

**Two creation paths:**
1. **Background-based (new):** Player picks race + background. Stats default to all-10. Background determines starting skills (2 skills at rank 1), starting items, and starting gold.
2. **Class-based (legacy):** Player picks race + class + stats. Class determines starting skills, abilities, spell slots, and equipment. Fully backwards compatible.

**HP formula:**
- Background path: 8 + CON mod (all backgrounds)
- Class path: Class hit die + CON modifier (Warrior 10, Berserker 12, Mage 6, etc.)

**Starting gold:**
- Background: Merchant=20gp, Drifter=0gp, others=5gp
- Class (legacy): 10gp

**Starting AC:** Calculated from equipped armor

**Stat validation:** The engine validates stat names in ability checks and saving throws. Invalid stats (like D&D 5e skills) return an error with a mapping to the correct ability score.

**Leveling:** XP thresholds: L2=300, L3=900, L4=2700, L5=6500, L6=14000, L7=23000, L8=34000, L9=48000, L10=64000
- HP gain per level: Warrior/Ranger/Berserker/Paladin 6+CON, Mage/Warlock 4+CON, Rogue/Cleric/Monk/Bard 5+CON (min 1)
- Full heal on level up
- Proficiency: +2 (L1-4), +3 (L5-8), +4 (L9-10)

### Equipment System

**10 slots:** Head, Amulet, MainHand, OffHand, Chest, Hands, Ring, Legs, Feet, Back

**AC calculation:**
- Light armor: base AC + full DEX mod
- Medium armor (dex_cap_2): base AC + min(DEX mod, 2)
- Heavy armor (no_dex): base AC only
- Unarmored: 10 + DEX mod
- Plus AC bonuses from all equipped items

**Rarity:** Common, Uncommon, Rare, Epic, Legendary

**Starting equipment by class:**
- Warrior: Longsword + Chain Mail + Shield
- Mage: Quarterstaff + Leather Armor + Spellbook (inventory)
- Rogue: Shortsword + Studded Leather + Thieves' Tools (inventory)
- Cleric: Mace + Scale Mail + Shield
- Ranger: Longbow + Chain Shirt + Shortsword (inventory)
- Berserker: Greataxe + Hide Armor
- Paladin: Longsword + Chain Mail + Shield
- Monk: Quarterstaff + Leather Armor
- Warlock: Quarterstaff + Leather Armor + Spellbook (inventory)
- Bard: Shortsword + Leather Armor + Lute (inventory)
- All: 1 Health Potion (inventory)

**Item database:** ~40+ hardcoded items including weapons, armor, shields, enchanted items, potions, scrolls. See `src/engine/equipment.rs` `get_item()`.

### Naked Start (T0)

Players can begin with `naked_start: true` during character creation. This starts them with:
- No equipment (all slots empty)
- 0 gold
- Unarmed damage: 1 + STR modifier (1d1 base)
- AC = 10 + DEX modifier (unarmored)
- **Monk Unarmored Defense:** AC = 10 + DEX modifier + WIS modifier

The `naked_start` parameter is accepted in both the REST API (`POST /api/adventures`) and WebSocket (`create_adventure`) character creation payloads.

### Item Tier System

Items have a `tier: u8` field (0-10) indicating power level. Existing items are classified into tiers:

| Tier | Name | Examples |
|------|------|----------|
| T0 | Unarmed | Unarmed/unequipped baseline |
| T1 | Basic | Club, Dagger, Leather Armor, Buckler |
| T2 | Standard | Longsword, Chain Mail, Scale Mail |
| T3 | Quality | Greatsword, plate-adjacent armor, magic accessories |
| T4 | Rare | Plate Armor, Flametongue, Frostbrand |
| T5 | Legendary | Vorpal Longsword |
| T6-T10 | (Reserved) | Future high-tier content |

Tiers inform monster generation balance and loot distribution.

### Combat Archetypes & Type Advantage

**Archetypes:** The combat archetype is determined by the EQUIPPED WEAPON, not the character's class. The `weapon_archetype()` function in `combat.rs` maps weapons to archetypes:

| Archetype | Weapon Types | Strong vs | Weak vs |
|-----------|-------------|-----------|---------|
| Valor | Melee weapons (sword, axe, mace, hammer, spear), unarmed | Cunning (Skulker) | Arcana (Mystic) |
| Cunning | Ranged weapons (bow), finesse weapons (dagger, shortsword, rapier) | Arcana (Mystic) | Valor (Brute) |
| Arcana | Spell-casting items (staff, wand, spellbook, tome) | Valor (Brute) | Cunning (Skulker) |
| Divine | (Reserved for holy items with Healing/Blessing skills) | Undead (+50%) | General (-10%) |
| Utility | (Reserved for instruments/social items) | Neutral | Neutral |

**Enemy types:** Brute, Skulker, Mystic, Undead

**Damage multipliers (applied to player attack damage):**

| Matchup | Multiplier |
|---------|------------|
| Advantage (e.g., Valor vs Skulker) | 1.20x |
| Disadvantage (e.g., Valor vs Mystic) | 0.80x |
| Divine vs Undead | 1.50x |
| Any non-Divine vs Undead | 0.80x |
| Divine vs non-Undead | 0.90x |
| Utility | 1.00x (neutral) |

The `Enemy` struct has an optional `enemy_type` field. The LLM can specify `enemy_type` in `start_combat`. When set, the type multiplier is applied to all player attack damage.

**Derived class label:** The `derived_class_label()` function in `skills.rs` computes a display-only class name from the character's highest-ranked skill family. This is used for presence display and party info, NOT for any mechanical purpose.

### Combat System (BG3-inspired)

**Initiative:** d20 + DEX mod (player), flat d20 (enemies). Sorted descending.
**Action economy per turn:** 1 action, 1 bonus action, 30ft movement, 1 reaction.

**Player actions:**
| Action | Type | Effect |
|--------|------|--------|
| Attack | Action | d20 + mods vs AC, weapon damage on hit |
| Dodge | Action | Enemies roll with disadvantage |
| Flee | Action | Attempt to escape combat. Roll d20+DEX vs DC (10 + living enemies×2 - prior attempts×2, min 5). Success ends combat with 0 XP. Failure wastes action but lowers DC by 2 next attempt. |
| Dash | Action | +30 movement |
| Use Item | Action | First potion in inventory (2d4+2 heal) |
| Second Wind | Bonus (requires Fortitude 1+) | 1d10 + level HP |
| Hide | Bonus (requires Stealth 1+) | Advantage on next attack |
| Healing Word | Bonus (requires Healing 1+) | 1d4 + WIS mod HP |
| Reckless Attack | Bonus (requires Rage 1+) | Advantage on attacks, but enemies have advantage on you |
| Lay on Hands | Bonus (requires Lay on Hands 1+) | Heal from divine pool (5 x level HP) |
| Flurry of Blows | Bonus (requires Flurry 1+) | Two bonus unarmed strikes |
| End Turn | Free | Advance to next combatant |

**Enemy AI:** Picks highest to_hit attack, rolls d20 + to_hit vs player AC.
**Death:** HP <= 0 = character dies, adventure over.
**Victory:** All enemies down. Awards 50 XP per enemy. LLM narrates victory.

Enemy names from `start_combat` are automatically truncated to 40 characters. Stats, descriptions, and flavor text are stripped from names.

### Conditions

| Condition | Effect |
|-----------|--------|
| Poisoned | Disadvantage on attacks/checks, 1d4 poison damage/turn |
| Burning / On Fire | 1d6 fire damage/turn |
| Bleeding | 1d4 damage/turn |
| Blinded | Disadvantage on attacks |
| Frightened | Disadvantage on checks/attacks |
| Stunned | Can't act, fails STR/DEX saves |
| Paralyzed | Can't act, auto-fail saves, melee crits |
| Exhaustion | Disadvantage on checks, speed halved |

### Hex World Map (251K Counties)

The game world is a massive hex grid of ~251,000 counties generated deterministically from a fixed seed (42) at server start. The global world map is shared by all players via `WORLD: LazyLock<WorldMap>`.

**Player state (stored per-adventure):**
- `position: PlayerPosition` -- hex coordinates (county_q, county_r) + location_idx
- `discovery: DiscoveryState` -- set of discovered county coordinates

**County properties:** name, tier (difficulty 0-10), biome, region, has_town, has_dungeon, dungeon_tier (hidden from players), has_tower, has_exchange, has_guild_hall, stations (list of CraftingStationType)

**Travel:** Directional (6 hex directions: east, west, NE, NW, SE, SW). Each move discovers the target county + its neighbors. Random encounters based on county tier.

**Shops:** Dynamically generated from county tier. Shops stock:
- Consumables (health potions, greater health potions at T2+)
- Crafting materials (intermediate materials at and near county tier)
- Pre-made equipment at a 3x markup (blade/bow/dagger/staff weapons and armor up to T5)
- Basic equipment items for low-tier towns (T0-T2)

Shop item lookups fall through: equipment database -> crafting material -> crafting equipment. This allows shops to sell crafted materials and equipment pieces.


**Crafting Stations:** Towns have crafting stations based on county tier. Each station supports specific crafting skills and has a maximum recipe tier.

| Station | Skills | Max Tier | Placement |
|---------|--------|----------|-----------|
| Tanning Rack | Leatherworking | T3 | All towns (T0+) |
| Basic Forge | Smithing | T3 | Towns T1+ |
| Woodworking Bench | Woodworking | T3 | Towns T1+ |
| Loom | Tailoring | T3 | Towns T1+ |
| Herb Table | Alchemy | T4 | Towns T2+ |
| Enchanting Altar | Enchanting | T5 | 30% chance at T3+ |
| Jeweler's Bench | Jewelcrafting | T5 | 30% chance at T3+ |
| Master Forge | SM/LW/WW/TL | T7 | 20% chance at T4+ |
| Runic Circle | Runecrafting | T8 | 15% chance at T5+ |
| Artificer's Workshop | Artificing | T9 | 10% chance at T7+ |
| Sacred Altar | Theurgy | T10 | 5% chance at T9+ |
| Primordial Forge | All 10 skills | T10 | 2 placed at highest-tier towns |

Station validation is enforced in `craft_item`: the player must be at a town with a station that supports the recipe's skill and is capable of the recipe's tier.

**Dungeons:** Generated on-demand from county seed via `generate_tiered_dungeon(seed, tier)`. Dungeon difficulty scales to the county's `dungeon_tier` (hidden from players). The `map_info` response includes a `dungeon_hint` field with a vague atmospheric description.

**Towers:** County towers generate infinite dungeon floors via `generate_tiered_dungeon`, each harder than the last (tier increases by +0.5 per floor).

**Race spawns:** Each race spawns in a different region of the hex map.

**Migration:** Old saves (with `world: Option<WorldMap>`) auto-migrate via `migrate_position()`, which assigns a spawn position based on character race.

### Quest System

Quests have structured state tracked by the engine:

**Quest fields:**
- `id` (UUID), `name`, `description`, `category` (Main/Side/Bounty)
- `final_goal` — ultimate objective shown to player
- `next_step` — current immediate objective (updated as progress is made)
- `reward` — `{ gold, xp, items[], description }` — auto-awarded on completion
- `steps_completed` — history of completed steps with timestamps
- `giver_npc_id` — linked NPC who gave the quest
- `status` — Active, Completed, Failed

**Quest tools:** `add_quest`, `complete_quest` (auto-awards rewards), `update_quest_step`, `fail_quest`

**Migration:** Old saves with `{name, description, completed}` format auto-migrate on load.

### NPC System

NPCs are persistent characters tracked across the adventure.

**NPC fields:**
- `id` (UUID), `name`, `description`, `location` (world map location name)
- `npc_type` — Fixed (permanent) or Generated (may leave)
- `disposition` — friendly, neutral, suspicious, hostile
- `quest_ids` — quests they've given
- `interactions` — interaction history `[{summary, timestamp}]`
- `faction` — NpcFaction enum: Civilian (default), Guard, Criminal, Neutral, Merchant
- `combat_tier` (f32), `hp` (i32), `max_hp` (i32), `ac` (i32) — combat stats
- `attacks` — Vec<EnemyAttack> for NPC combat encounters

**NPC combat stats** are generated via `generate_npc_combat(tier, faction)`:
- Faction modifies effective tier: Guard +1, Civilian -1, others +0
- HP = 10 + effective_tier * 8, AC = 10 + effective_tier
- Attack weapon varies by faction (Guard=Sword, Criminal=Dagger, Civilian=Fists)

**Persistence rules:**
- Fixed NPCs cannot be dismissed
- Generated NPCs with active quests cannot be dismissed
- Generated NPCs without active quests can be dismissed (they leave)

**NPC tools:** `create_npc`, `update_npc`, `dismiss_npc`, `list_npcs`, `log_npc_interaction`

### Murderer System

Players can be flagged as murderers for killing innocents.

**Character fields:** `murderer` (bool, default false), `kill_count` (u32, default 0)

**Rules:**
- Killing a non-murderer player or innocent NPC flags the killer as a murderer
- Killing a murderer does NOT flag (bounty hunting is legitimate)
- Guards in towns attack murderers on sight
- Guards defend non-murderer players being attacked
- NPC guards have combat stats based on the local county tier

**Murderer tools:** `flag_murderer` (sets murderer=true, increments kill_count), `check_murderer` (returns murderer status and kill_count)

**System prompt** includes PvP & Murder Rules and displays murderer status in game state.

### Skill System

All characters have access to all 44 skills (34 combat + 10 crafting). Skills have ranks 0-10 and per-skill XP.

**Rank names:** 0=Untrained, 1=Novice, 2=Apprentice, 3=Journeyman, 4=Adept, 5=Expert, 6=Master, 7=Grandmaster, 8=Legendary, 9=Mythic, 10=Transcendent

**Per-Skill XP thresholds (to rank up from rank N):**
- Rank 0: 50 XP, Rank 1: 100 XP, Rank 2: 300 XP, Rank 3: 800 XP, Rank 4: 2000 XP
- Rank 5: 5000 XP, Rank 6: 12000 XP, Rank 7: 30000 XP, Rank 8: 70000 XP, Rank 9: 150000 XP

**Combat skills (34):** Weapon Mastery, Shield Wall, Fortitude, Rage, Reckless Fury, Primal Toughness, Holy Smite, Divine Shield, Lay on Hands, Blade Finesse, Stealth, Lockpicking, Evasion, Marksmanship, Tracking, Beast Lore, Survival, Martial Arts, Ki Focus, Iron Body, Flurry, Evocation, Abjuration, Spell Mastery, Eldritch Blast, Curse Weaving, Soul Harvest, Healing, Blessing, Turn Undead, Inspire, Lore, Charm, Song of Rest

**Crafting skills (10):** Leatherworking, Smithing, Woodworking, Alchemy, Enchanting, Tailoring, Jewelcrafting, Runecrafting, Artificing, Theurgy

**Background starting skills:** Each background sets 2 skills to rank 1 (except Drifter which starts with none).

**Legacy class starting skills:** When using class-based creation, class-specific skills start at rank 1 (3-4 skills per class).

**Migration:**
- migrate_skills() -- populates skills for old saves without any
- migrate_crafting_skills() -- adds crafting skills to pre-crafting saves
- migrate_all_skills() -- adds any missing skills from the full 44-skill set and initializes xp_to_next

**Skill tools:** get_skills (includes xp/xp_to_next), improve_skill (instant rank up), award_skill_xp (XP-based progression)


### Crafting Graph & Equipment Production

**Module:** `src/engine/crafting.rs` (~2700 lines)

The crafting system is built on a directed acyclic graph (DAG) of materials and recipes spanning tiers T0-T10 with 10 crafting skills.

**Graph statistics:** 336 materials, 282 recipes (82 intermediate + 200 equipment).

**Intermediate crafting chain (T0-T10):**
- T0: 14 raw materials (gathered + monster drops)
- T1-T10: 8 crafted materials per tier per skill, plus 4 monster drops per tier
- Gateway skill per tier (can reach that tier from T(N-1) alone): LW(T1), SM(T2), WW(T3), AL(T4), EN(T5), TL(T6), JC(T7), RC(T8), AF(T9), TH(T10)

**Equipment production (end-product items):**
10 equipment lines, each producing a weapon + armor at every tier (T1-T10) = 200 equipment items total.

| Line | Skills | Weapon | Armor |
|------|--------|--------|-------|
| Blade | SM+LW+EN | Longsword | Heavy Plate |
| Axe | SM+LW+WW | Greataxe | Hide Armor |
| Holy | SM+RC+TL | Mace | Blessed Plate |
| Dagger | LW+AL+JC | Daggers | Shadow Leather |
| Bow | WW+LW+AL | Longbow | Ranger Leather |
| Fist | TL+AL+EN | Wraps | Ki Robes |
| Staff | WW+EN+RC | Arcane Staff | Mage Robes |
| Wand | RC+TL+JC | Eldritch Wand | Dark Vestments |
| Scepter | SM+RC+TL | Holy Scepter | Priest Vestments |
| Song | WW+TL+JC | Instrument | Performer Garb |

**Equipment naming tiers:** T1=Crude, T2=Iron, T3=Steel, T4=Dwarven/Elven, T5=Mithril, T6=Rune, T7=Dragon, T8=Void, T9=Celestial, T10=Primordial

**Balance metrics (all passing):**
- Per-tier spread: <30% at all tiers (T1: 19%, T2: 30%, T3-T10: <20%)
- Cross-equipment mixing: all 10 crafting skills feed all 10 equipment lines
- Tier scaling: ~7x cost increase per tier (avg=6.98x, range 6.10-7.77x)
- Skill diversity: every equipment piece requires 3-10 different crafting skills

**CLI analysis:**
```bash
cargo run -- crafting --analyze     # Full crafting graph balance report
cargo run -- crafting --equipment   # Equipment end-to-end balance report
cargo run -- crafting --recipe <id> # Lookup specific material/equipment recipe
cargo run -- crafting --tier <N>    # Show all recipes at a tier
cargo run -- crafting --mixing      # Show mixing scores
```

**Equipment material IDs:** `{line}_weapon_t{N}` and `{line}_armor_t{N}` (e.g., `blade_weapon_t1`, `bow_armor_t5`)

**Equipment crafting output:** When `craft_item` produces an equipment recipe output (weapon or armor), the `equipment_to_item()` function generates a fully equippable Item with proper stats:
- **Weapons:** damage dice, attack bonus, damage modifier stat, ranged/finesse/two-handed flags scale with tier
- **Armor:** AC base + weight class (heavy/medium/light) based on equipment line, scales with tier
- Falls back to `material_to_item()` for intermediate crafting materials

**Gather tool:** `gather` collects 1-3 T0 raw materials based on the current county biome and awards 5-10 Survival skill XP.

**Biome material pools:**
| Biome | Materials |
|-------|-----------|
| Plains | plant_fiber, wild_herbs, crude_thread |
| Forest | green_wood, wild_herbs, plant_fiber |
| Hills | rough_stone, scrap_metal, muddy_clay |
| Mountains | rough_stone, scrap_metal, raw_quartz |
| Swamp | muddy_clay, wild_herbs, plant_fiber |
| Coast | plant_fiber, rough_stone, raw_quartz |
| Desert | rough_stone, raw_quartz, scrap_metal |
| Tundra | rough_stone, raw_quartz, scrap_metal |
| Volcanic | scrap_metal, raw_quartz, charcoal |

**Passive combat skill XP:** Players earn skill XP during combat:
- On successful attack hit: +5 XP to weapon skill (marksmanship for ranged, blade_finesse for finesse, weapon_mastery otherwise)
- On taking damage and surviving: +3 Fortitude XP

### Background System

**Module:** src/engine/backgrounds.rs

10 backgrounds determining starting skills, equipment, and gold:

| Background | Starting Skills | Starting Items | Gold |
|---|---|---|---|
| Farmhand | Fortitude, Leatherworking | Spear | 5 |
| Apprentice Smith | Smithing, Weapon Mastery | Mace | 5 |
| Street Urchin | Stealth, Lockpicking | Dagger | 5 |
| Hunter | Marksmanship, Tracking | Shortbow | 5 |
| Acolyte | Healing, Blessing | Quarterstaff | 5 |
| Scholar | Lore, Enchanting | Spellbook | 5 |
| Merchant | Charm, Inspire | (none) | 20 |
| Herbalist | Alchemy, Survival | (none) | 5 |
| Woodcutter | Woodworking, Fortitude | Handaxe | 5 |
| Drifter | (none) | (none) | 0 |

**REST API:** GET /api/backgrounds -- lists all backgrounds with their starting skills, items, and gold.
**REST/WS creation:** POST /api/adventures and CreateAdventure WS message now accept optional background field (string). If background is provided, uses new system. If class is provided (or neither), uses legacy class system for backwards compatibility.

### Structured Choices

The `present_choices` tool supports both simple string choices and structured choices with mechanical check data:

**Simple choice:** `"Search the room"`
**Structured choice:** `{"text": "Disarm the trap", "check": {"stat": "dex", "dc": 12, "success_effect": "Trap disarmed", "failure_effect": "Trap triggers", "failure_damage": "1d6"}}`

When a player selects a structured choice with a `check`, the engine automatically:
1. Rolls the ability check (d20 + stat modifier vs DC)
2. Applies failure damage if the check fails
3. Passes the result to the LLM for narrative

The frontend displays check data as colored badges: `[DEX DC 12]` on choice buttons.

### Monster Generation

**Module:** `src/engine/monsters.rs`

`generate_monster(tier, enemy_type)` creates balanced enemies using simulator-validated stat curves. Monsters are generated based on two parameters:
- **Tier (0-10):** Determines stat scaling (HP, AC, attack bonus, damage)
- **Enemy type:** Brute, Skulker, Mystic, or Undead -- determines name pool and interacts with the type advantage system

Each tier/type combination has named monsters (e.g., a T2 Brute might be "Ogre", a T3 Mystic might be "Dark Sorcerer"). Stats are derived from curves validated by the battle simulator to ensure fair encounters at each tier.

### Battle Simulator

**Module:** `src/engine/simulator.rs`

CLI tool for validating combat balance across all class/tier/type combinations.

```bash
cargo run -- simulate [--sweep|--stats|--class <name>|--party-report]
```

| Flag | Description |
|------|-------------|
| `--sweep` | Run full sweep across all tiers and enemy types |
| `--stats` | Show stat curves for monsters at each tier |
| `--class <name>` | Simulate specific class across all matchups |
| `--party-report` | Full balance report for all classes |

Runs thousands of simulated combats to produce win-rate statistics. Used to derive and validate the stat curves in `monsters.rs` and the damage multipliers in the type advantage system.

### Dice System

- `roll(dice_type, count, modifier)` -- Returns DiceResult with individual rolls, total, and optional DC check
- `success_probability()` -- Exact for single die, Monte Carlo (10k trials) for multiple

### State Change Tracking
After each LLM tool loop completes, the engine captures a before/after snapshot of the player's state (HP, gold, XP, level, inventory, conditions) and computes a diff. Non-empty diffs are sent as `state_changes` messages and shown as colored badges in the UI (e.g., [+25 gold] [+50 XP] [-4 HP]).

### World Map

**World:** "The Realm of Eldara" -- 20 locations, 19 connections
**Location types:** Town, Dungeon, Wilderness, Tower, Camp, Landmark
**Danger levels:** 0 (safe), 1 (15% encounter), 2 (30%), 3+ (50%)
**Game modes:** WorldMap, InTown, InDungeon, InTower, Exploring

**Shops:** In towns with item_id references, stock limits, price multipliers. Buy at full price, sell at half.

**Scenario-based starting locations:**
- Default: Crossroads Inn
- Ruins/dungeon: Thornwall Village
- Dragon: Frosthold
- City/intrigue: Port Blackwater
- Wilderness: Dark Forest
- Haunted: Ravenmoor
- Destitute Start (naked_start: true, no equipment, 0 gold)


Locations may have `has_exchange: true` (exchange order book) or `has_guild_hall: true` (guild management). See Exchange System and Guild System sections below.

### Dungeon System

- **Seeded procedural generation** (deterministic per seed)
- **Tier-scaled difficulty:** Dungeons use the county's `dungeon_tier` (from worldgen) to scale monster stats, trap DCs, and treasure rewards. Higher tier = tougher enemies + better loot.
- **Floor count scales with tier:** Tier 0-2: 2 floors, Tier 3-4: 3 floors, Tier 5+: 4 floors.
- **Stat scaling per floor:** Each floor adds +0.5 effective tier. Monster HP scales by `1.0 + (effective_tier - 1.0) * 0.3`, AC and attack bonuses increase by `tier/2` and `tier/3` respectively.
- **Treasure scaling:** Gold is multiplied by `1.0 + tier * 0.5`.
- **Hidden tier:** The dungeon tier is NEVER shown to the player. Instead:
  - The LLM system prompt receives an atmosphere hint (e.g., "foreboding darkness" for tier 3-4, "crackles with dark energy" for tier 5-6).
  - The `map_info` response includes a `dungeon_hint` field with a vague description (e.g., "Locals mention an old cave nearby").
  - Players discover difficulty through exploration and combat.
- **Room types:** Entrance, Combat, Trap, Treasure, Boss, Puzzle, Rest, Empty, Stairs
- **BSP-like room placement** on a grid, connected by corridors
- **Traps:** Detection DC (WIS), save DC, damage, conditions (DCs scale with tier)
- **Treasure:** Gold + items per room (WIS check to find, DC scales by floor)
- **Boss doors:** Locked, require Boss Key from Treasure rooms
- **Navigation:** Move by cardinal direction + Descend/Ascend
- **Key functions:** `generate_dungeon(seed)` (base), `generate_tiered_dungeon(seed, tier)` (tier-scaled), `dungeon_hint_for_tier(tier)` (vague hint text)

**REST API:** 6 endpoints under `/api/adventures/:id/dungeon/` -- `enter`, `move`, `skill-check`, `activate-point`, `retreat`, `status`. See REST API Endpoints section above.

**WebSocket:** Client sends `dungeon_enter`, `dungeon_move`, `dungeon_skill_check`, `dungeon_activate_point`, `dungeon_retreat`, `dungeon_status`. Server responds with `dungeon_entered`, `dungeon_room_changed`, `dungeon_skill_gate_result`, `dungeon_puzzle_activation`, `dungeon_retreated`, `dungeon_status`, `corruption_tick`, `path_cleared`, `convergence_unlocked`, `breach_warning`.

### Tower System (Shared Infinite Dungeons)

10 towers placed at specific counties across the world map. All players share the same tower instance — floors are deterministically generated from tower seed + floor number using ChaCha8 RNG.

**Tower definitions** (`src/engine/tower.rs`):
| Tower | Base Tier | Seed |
|-------|-----------|------|
| Tower of Dawn | 2.0 | 1001 |
| Ironspire | 3.5 | 1002 |
| The Thornkeep | 4.5 | 1003 |
| Tidecaller Spire | 5.0 | 1004 |
| Shadowpillar | 5.5 | 1005 |
| The Nexus | 6.0 | 1006 |
| Dragonwatch | 7.0 | 1007 |
| Frostspire | 8.0 | 1008 |
| The Abyss | 8.5 | 1009 |
| Primordial Spire | 9.5 | 1010 |

**Floor generation:**
- **Tier:** `base_tier + floor_number * 0.2`
- **Size:** `(8 + floor*2) x (8 + floor*2)`, capped at 50x50
- **Safe floors:** Every 10th floor (floor 10, 20, 30, ...) — all rooms are Safe type
- **Boss floors:** Every 5th floor ending in 4 (floor 4, 9, 14, ...) — center room is Boss type
- **Entrance:** Room (0,0) is always Safe
- **Stairs:** Room (width-1, height-1) leads to next floor

**Room types:** Empty, Combat, Treasure, Trap, Safe, Stairs, Boss
- Combat rooms: 1-3 enemies scaled to floor tier
- Boss rooms: 1 boss enemy

**Persistence** (`src/storage/tower_store.rs`): Floor state saved as `data/towers/{tower_id}_{floor}.json`. Shared across all players.

**Guard floors:** Every 5th floor (5, 10, 15, ...) features a guard encounter that must be defeated to proceed. Guard difficulty scales with floor tier.

**Boss HP scaling:** Boss HP scales with floor number: `base_hp * (1.0 + floor * 0.15)`. Higher floors produce significantly tougher bosses.

**Checkpoints:** Players can attune to checkpoints on safe floors (every 10th floor). Attuning is free. Once attuned, the player can teleport back to that floor from the tower entrance.

**Teleportation:** Teleporting to a checkpoint costs gold: `floor_number * 10` gold. Requires sufficient gold in inventory.

**Entry requirements:** A player must be at a county with `has_tower: true` and the `tower_id` must match. No minimum tier requirement to enter, but higher-tier towers will be lethal for under-geared players.

**First-clear tracking:** The server tracks the first player to clear each floor of each tower. First clears are announced to all connected players via the `tower_first_clear` WebSocket message.

**LLM tools:** `enter_tower`, `tower_ascend`, `exit_tower`. Tracks highest floor reached.

**REST API:** 7 endpoints -- `GET /api/towers` (list), `GET /api/towers/:tower_id/floor/:floor_num` (floor info), and 5 under `/api/adventures/:id/tower/` -- `enter`, `move`, `ascend`, `checkpoint`, `teleport`. See REST API Endpoints section above.

**WebSocket:** Client sends `tower_list`, `tower_enter`, `tower_move`, `tower_ascend`, `tower_checkpoint`, `tower_teleport`, `tower_floor_status`. Server responds with `tower_list`, `tower_entered`, `tower_floor_status`, `tower_player_nearby`, `tower_first_clear`.

## LLM Integration

- **Provider:** xAI API (`https://api.x.ai/v1`)
- **Default model:** `grok-4-1-fast-reasoning`
- **Available models:** `grok-4-1-fast-reasoning`, `grok-4-1-fast-non-reasoning`
- **Pricing:** Fast: $0.20/M in, $0.50/M out. Full reasoning: $3.00/M in, $15.00/M out

**Tool loop:** Max 15 iterations. LLM calls tools -> engine executes -> results fed back -> repeat until LLM returns narrative or a pending action.

**44+ tool definitions** for the LLM including: roll_dice, request_player_roll, ability_check, saving_throw, attack_roll, get_character_sheet, update_hp, award_xp, add_item, remove_item, give_item, give_gold, equip_item, unequip_slot, use_ability, present_choices, set_scene, add_quest, complete_quest, add_condition, remove_condition, start_combat, end_combat, move_to_room, search_room, get_skills, improve_skill, award_skill_xp, travel_to, enter_dungeon, exit_dungeon, enter_tower, tower_ascend, exit_tower, buy_item, sell_item, view_shop, craft_item, list_recipes, gather, get_map_info, flag_murderer, check_murderer.

**Precomputed branching (WebSocket only):** When `request_player_roll` is pending, spawns two parallel LLM calls (success + failure). Uses the matching result instantly when the player rolls.

**System prompt:** Built dynamically from current game state including character stats, location, combat status, inventory, quests, dungeon state, world map, crafting skill ranks, and available crafting stations at the current location. Contains comprehensive crafting system guidance (~80 lines) covering the crafting flow, station types and tier caps, the gateway staircase, 10 equipment lines, new player onboarding, tool usage, and skill progression.

### Retry Guardrail

If the LLM returns a text response without calling `present_choices` during the tool loop (and no dice roll or combat is pending), the system automatically retries up to 2 times with a nudge message. This prevents adventures from getting stuck without player choices.


## Friends System

### Friend Codes

Each user account has a unique 6-digit friend code (e.g. `quinten#482193`). Codes are generated on account creation and lazily assigned to existing accounts on first access. Tags are in the format `username#NNNNNN`.

### Storage

**Per-user friend data** stored in `data/users/<username>/friends.json`:
```json
{
  "friends": ["other-user"],
  "outgoing_requests": ["pending-user"],
  "incoming_requests": ["requester"]
}
```

**Chat history** stored in `data/users/<username>/chats/<friend>.jsonl` (one JSON object per line):
```json
{"from": "quinten", "text": "hello!", "ts": "2026-03-29T12:00:00Z"}
```

### Online Presence

A shared in-memory `PresenceRegistry` tracks all connected WebSocket users:
- Username, friend code, character name, class, and current location
- Broadcast channel per user for pushing real-time friend events
- On connect: registers user, notifies friends of online status
- On disconnect: removes user, notifies friends of offline status
- On adventure load/create: updates character and location info, notifies friends

### REST API Endpoints (Port 2998)

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `GET` | `/api/friends/code` | Yes | -- | `{tag}` |
| `GET` | `/api/friends` | Yes | -- | `{friends[], incoming_requests[], outgoing_requests[]}` |
| `POST` | `/api/friends/request` | Yes | `{friend_tag}` | `{success, message}` |
| `POST` | `/api/friends/accept` | Yes | `{username}` | `{success}` |
| `POST` | `/api/friends/decline` | Yes | `{username}` | `{success}` |
| `DELETE` | `/api/friends/:username` | Yes | -- | `{removed}` |
| `POST` | `/api/friends/chat` | Yes | `{to, text}` | `{from, text, ts}` |
| `GET` | `/api/friends/chat/:username` | Yes | -- | `{friend, messages[]}` |

Note: REST API endpoints do not include real-time presence updates (use WebSocket for that).

### WebSocket Protocol

**Client -> Server:**

| Type | Fields | Description |
|------|--------|-------------|
| `get_friend_code` | -- | Get your username#code tag |
| `send_friend_request` | `friend_tag` | Send request by tag (e.g. "bob#123456") |
| `accept_friend_request` | `username` | Accept incoming request |
| `decline_friend_request` | `username` | Decline incoming request |
| `remove_friend` | `username` | Remove a friend |
| `get_friends` | -- | Get full friends list with presence |
| `send_chat` | `to, text` | DM a friend |
| `get_chat_history` | `friend, limit?` | Load chat history (default 50) |

**Server -> Client:**

| Type | Fields | Description |
|------|--------|-------------|
| `friend_code` | `tag` | Your tag (e.g. "quinten#482193") |
| `friends_list` | `friends[], incoming_requests[], outgoing_requests[]` | Full list with presence |
| `friend_presence` | `username, friend_code, online, character_name?, character_class?, location?` | Real-time status change |
| `friend_request_received` | `from_username, from_tag` | Incoming request notification |
| `friend_request_accepted` | `username, friend_code` | Request accepted notification |
| `friend_request_sent` | `success, message` | Result of your request |
| `friend_chat` | `from, text, ts` | Incoming chat message |
| `friend_chat_history` | `friend, messages[]` | Chat history response |

Each `friends[]` entry: `{username, friend_code, online, character_name?, character_class?, location?}`

### UI: Friends Panel

- **Toggle**: Button in story header (left side, people icon) opens/closes the panel
- **Layout**: 3-column when open (`260px | flex | 320px`), 2-column when collapsed
- **Mobile**: Slide-out drawer from left (280px, overlays content)
- **Sections**:
  - Your friend tag (click to copy)
  - Pending friend requests (accept/decline buttons)
  - Friends list (sorted: online first, then alphabetical)
  - Chat pane (replaces list when chatting with a friend)
- **Friend entry shows**: online dot, username#code, character name/class, location
- **Unread badges**: On individual friends and the toggle button
- **World map**: Online friends shown as green markers at their location

### Key Files

| File | Purpose |
|------|---------|
| `src/storage/friends_store.rs` | Friend list, request, and chat persistence |
| `src/web/presence.rs` | In-memory online presence registry |
| `src/web/protocol.rs` | Friend-related ClientMsg/ServerMsg variants |
| `src/web/websocket.rs` | WebSocket friend message handlers |
| `src/web/api_server.rs` | REST API friend endpoints |
| `static/js/friends.js` | Friends panel UI module |
| `static/css/adventure.css` | Friends panel CSS (appended) |


### Location Chat

Players at the same world map location can send public messages visible to all present players.

**Behavior:**
- Messages are ephemeral (in-memory ring buffer, last 50 per location, no disk persistence)
- Only players currently at a location can see messages
- Leaving a location means you can no longer see those messages
- On arrival at a location, recent chat history is sent

**REST API:**
| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/location/chat` | Yes | `{adventure_id, text}` | `{from, text, ts, location}` |
| `GET` | `/api/location/players?adventure_id=X` | Yes | -- | `{location, players[]}` |

**WebSocket Client -> Server:**
| Type | Fields | Description |
|------|--------|-------------|
| `send_location_chat` | `text` | Send public message at current location |
| `get_location_players` | -- | Request who's at your location |

**WebSocket Server -> Client:**
| Type | Fields | Description |
|------|--------|-------------|
| `location_chat` | `from, character_name, text, ts, location` | Public chat message |
| `location_presence_update` | `location, players[]` | Who's at this location |
| `location_chat_history` | `location, messages[]` | Recent messages on arrival |

**UI:** Toggle button (megaphone icon) in story header. Opens an overlay chat panel at bottom of story area.



## Party System

### Overview

Players can form parties of up to 4 members at the same world location. The party leader controls navigation (travel, dungeon entry, room movement). All members see shared narrative and participate in group combat with a 30-second timer system.

### Data Structures

**Party** (`src/engine/party.rs`):
- `id` (UUID), `leader` (username), `members` (Vec<PartyMember>), `location`, `state` (Idle/InDungeon/InCombat)
- `PartyMember`: username, adventure_id, character stats, ready flag, incapacitated flag, disconnected flag

**PartyRegistry** (`src/web/party_registry.rs`):
- In-memory registry with broadcast channels per party
- Quick user-to-party lookup
- Pending invite storage, PvP challenge tracking, criminal status tracking

### Party Lifecycle

1. **Invite**: Leader sends invite to player at same location
2. **Accept**: Target joins party (max 4 members)
3. **Navigation**: Leader's travel/dungeon actions replicate to all members
4. **Combat**: Timer-based group combat (see below)
5. **Leave/Kick**: Members can leave; leader can kick; party disbands at 1 member
6. **Disconnect**: Marked as disconnected, auto-dodge in combat, auto-removed after 5 min

### Group Combat (Timer-Based)

**Flow**: Party side vs Enemy side, alternating.

1. Combat triggers (room enemies, travel encounter)
2. Each member rolls d20+DEX for initiative
3. **Player Decision Phase (30s)**: All members choose actions simultaneously
4. **Resolution Phase**: Actions resolve in initiative order
5. **Enemy Phase**: Each enemy attacks a random living party member
6. Repeat until all enemies dead (victory) or all players incapacitated

**Death rules**: HP ≤ 0 = incapacitated (can be revived mid-combat). If still 0 HP when combat ends = **permanent death**. All members at 0 HP = TPK, all die.

**XP**: 50 per enemy, split equally among living members.

### Party Traps

When a party enters a trap room, each member rolls individually for detection (WIS) and saves. Damage/conditions apply per-member. LLM narrates the collective experience.

### REST API Endpoints

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `GET` | `/api/party` | Yes | -- | `{party_id, leader, members, state, location}` |
| `POST` | `/api/party/invite` | Yes | `{adventure_id, target_username}` | `{success, message}` |
| `POST` | `/api/party/accept` | Yes | `{adventure_id, from_username}` | `{success}` |
| `POST` | `/api/party/leave` | Yes | `{adventure_id}` | `{success}` |
| `POST` | `/api/party/combat/action` | Yes | `{adventure_id, action_id, target?}` | `{success}` |
| `POST` | `/api/party/combat/ready` | Yes | `{adventure_id}` | `{success}` |

### WebSocket Protocol

**Client -> Server:**
| Type | Fields | Description |
|------|--------|-------------|
| `send_party_invite` | `target_username` | Invite player at same location |
| `accept_party_invite` | `from_username` | Accept invite |
| `decline_party_invite` | `from_username` | Decline invite |
| `leave_party` | -- | Leave party |
| `kick_party_member` | `username` | Kick (leader only) |
| `get_party_info` | -- | Get party state |
| `party_combat_action` | `action_id, target?` | Submit combat action |
| `party_combat_ready` | -- | Ready up (defaults to dodge) |

**Server -> Client:**
| Type | Fields | Description |
|------|--------|-------------|
| `party_info` | `party_id, leader, members[], state, location` | Full party state |
| `party_invite_received` | `from_username, from_character, from_class` | Incoming invite |
| `party_invite_sent` | `success, message` | Invite result |
| `party_member_joined` | `username, character_name, character_class` | New member |
| `party_member_left` | `username, reason` | Member departed |
| `party_disbanded` | `reason` | Party dissolved |
| `party_narrative_chunk` | `text` | Shared narrative stream |
| `party_narrative_end` | -- | End of shared narrative |
| `party_combat_started` | `enemies[], initiative_order[], round` | Combat begins |
| `party_combat_phase_start` | `phase, deadline_ms, round, actions, enemies, party_hp` | 30s timer starts |
| `party_combat_action_ack` | -- | Your action was received |
| `party_combat_action_submitted` | `username` | Someone submitted |
| `party_combat_resolution` | `results[]` | Player actions resolved |
| `party_combat_enemy_phase` | `results[]` | Enemy attacks resolved |
| `party_combat_ended` | `victory, xp_per_member` | Combat over |
| `party_trap_results` | `results[]` | Per-member trap outcomes |

## PvP System

### Overview

Players at the same location can challenge each other to 1v1 duels. Death in PvP is **permanent**. Killing another player gives a 30-minute criminal flag. Criminals can be attacked without consent.

### Challenge Flow

1. Player A sends `pvp_challenge { target_username }`
2. Target receives `pvp_challenge_received`, can accept or decline
3. On accept: 1v1 combat using individual initiative (not timer-based)
4. HP ≤ 0 = **permanent death** (adventure over)

### Criminal System

- PK (killing in PvP) → 30-minute criminal flag
- Criminal icon visible in location player list and friends list
- Criminals can be attacked at same location WITHOUT accept step
- Criminal timer resets on each new PK

### Flee

Available each turn (except boss fights). Fleeing player exits combat and moves to a random connected location.

### REST API

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `POST` | `/api/pvp/challenge` | Yes | `{adventure_id, target_username}` | `{success, message}` |

### WebSocket Protocol

**Client -> Server:**
| Type | Fields | Description |
|------|--------|-------------|
| `pvp_challenge` | `target_username` | Challenge player |
| `accept_pvp_challenge` | `challenger` | Accept duel |
| `decline_pvp_challenge` | `challenger` | Decline duel |
| `pvp_action` | `action_id, target?` | Combat action |
| `pvp_flee` | -- | Flee from duel |

**Server -> Client:**
| Type | Fields | Description |
|------|--------|-------------|
| `pvp_challenge_received` | `challenger, character_name` | Incoming challenge |
| `pvp_challenge_sent` | `success, message` | Challenge result |
| `pvp_started` | `opponent_name, opponent_class, opponent_hp, opponent_ac` | Duel begins |
| `pvp_turn_start` | `your_turn, round, opponent_hp, opponent_max_hp, actions` | Your turn |
| `pvp_action_result` | `actor, action, description, roll?, hit?, damage?` | Action outcome |
| `pvp_ended` | `victory, opponent, criminal` | Duel over |
| `criminal_status_update` | `username, is_criminal` | Criminal flag change |

### Key Files

| File | Purpose |
|------|---------|
| `src/engine/party.rs` | Party, PartyCombatState, PvP data structures |
| `src/web/party_registry.rs` | In-memory party/invite/PvP/criminal registry |
| `src/web/party_handler.rs` | WebSocket handlers for party, combat, PvP |
| `static/js/party.js` | Party panel, combat timer, PvP UI |


## Trading System

### Overview

Two players at the same world location can trade items and gold. Both players must have an active adventure loaded. The trading uses a two-phase commit: both players set their offers, then both must accept before the trade executes.

### Trade Flow

1. Player A sends `send_trade_request { target_username }` -- must be at same location
2. Target receives notification of trade request
3. Target sends `accept_trade_request { from_username }` -- creates a TradeSession
4. Both players receive `trade_started { partner }`
5. Either player sends `update_trade_offer { items, gold }` -- both see updated offers
6. When satisfied, each player sends `accept_trade` -- acceptances reset if offers change
7. When both have accepted: items and gold are swapped, both adventures are saved
8. Both receive `trade_completed { items_gained, items_lost, gold_delta }`

### Validation

- Both players must be at the same world location
- Both players must be online with an active adventure
- Items must still exist in inventory when trade executes
- Gold must be sufficient when trade executes
- A player can only be in one trade at a time

### Cross-User Notifications

Trade notifications to the partner are sent via the PresenceRegistry's FriendEvent channel, using `__TRADE_MSG__` prefixed JSON payloads piggybacked on LocationChat events. The frontend should parse these specially.

### WebSocket Protocol

**Client -> Server:**
| Type | Fields | Description |
|------|--------|-------------|
| `send_trade_request` | `target_username` | Request trade (same location required) |
| `accept_trade_request` | `from_username` | Accept trade request |
| `decline_trade_request` | `from_username` | Decline trade request |
| `update_trade_offer` | `items: [{item_name, quantity}], gold` | Set your offer |
| `accept_trade` | -- | Accept current trade (both must accept) |
| `cancel_trade` | -- | Cancel active trade |
| `get_trade_status` | -- | Query trade state |

**Server -> Client:**
| Type | Fields | Description |
|------|--------|-------------|
| `trade_request_received` | `from_username` | Incoming trade request |
| `trade_started` | `partner` | Trade session created |
| `trade_offer_updated` | `your_offer, their_offer` | Offers changed |
| `trade_accepted` | `by` | One player accepted |
| `trade_completed` | `items_gained, items_lost, gold_delta` | Trade executed successfully |
| `trade_cancelled` | `reason` | Trade cancelled |
| `trade_status` | `active, partner?, your_offer?, their_offer?` | Current trade state |

### Key Files

| File | Purpose |
|------|---------|
| `src/web/trade_registry.rs` | In-memory trade session/request registry |
| `src/web/protocol.rs` | TradeItemInput struct, ClientMsg/ServerMsg trade variants |
| `src/web/websocket.rs` | Trade message handlers (in handle_client_msg) |


## Exchange System

### Overview

Central exchange/order book at specific world locations. Players can place buy/sell limit orders for items. Orders are matched automatically using price-time priority.

### Exchange Locations

Three locations have exchanges (`has_exchange: true`):
- **Crossroads Inn** (index 0)
- **Frosthold** (index 4)
- **Port Blackwater** (index 11)

### Order Types

- **Buy order**: Player escrows gold. Matched against sell orders at or below the buy price.
- **Sell order**: Player escrows items. Matched against buy orders at or above the sell price.

### Matching Algorithm

Price-time priority: orders are matched against existing opposing orders in FIFO order, filtered by price compatibility. Same-player orders never match. Partial fills are supported.

### REST API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/exchange/orders` | List all orders (optional `?item_id=` filter) |
| POST | `/api/exchange/orders` | Place a new order |
| GET | `/api/exchange/orders/mine` | List your orders |
| POST | `/api/exchange/orders/cancel` | Cancel an order |

**Place order request:**
```json
{
  "adventure_id": "uuid",
  "item_id": "iron_ingot",
  "order_type": "buy|sell",
  "quantity": 5,
  "price_per_unit": 10
}
```

**Place order response:**
```json
{
  "order_id": "uuid",
  "trades": [{"buyer": "...", "seller": "...", "item_id": "...", "quantity": 5, "price_per_unit": 10}],
  "state": { ... }
}
```

### WebSocket Protocol

**Client -> Server:**
- `place_exchange_order { item_id, order_type, quantity, price_per_unit }`
- `list_exchange_orders { item_id? }`
- `cancel_exchange_order { order_id }`
- `get_my_exchange_orders`

**Server -> Client:**
- `exchange_order_placed { order_id, trades[] }`
- `exchange_orders { orders[] }`
- `exchange_order_cancelled { order_id, refunded }`

### LLM Tools

- `place_exchange_order` -- Place buy/sell order (checks exchange location)
- `list_exchange_orders` -- View order book (checks exchange location)
- `cancel_exchange_order` -- Cancel an order (checks exchange location)

### Persistence

- `data/exchange.json` -- Single JSON file with all orders (via `ExchangeStore`)

### Key Files

| File | Purpose |
|------|---------|
| `src/engine/exchange.rs` | Order book logic, matching algorithm |
| `src/storage/exchange_store.rs` | JSON persistence for order book |

## Guild System

### Overview

Player organizations with ranks, a treasury, and guild halls. Two guild types: Combat and Crafting. Players can only be in one guild at a time.

### Guild Hall Locations

Four locations have guild halls (`has_guild_hall: true`):
- **Crossroads Inn** (index 0)
- **Thornwall Village** (index 1)
- **Frosthold** (index 4)
- **Port Blackwater** (index 11)

### Guild Ranks

- **Leader** -- Full control, can promote/kick
- **Officer** -- Can promote recruits/members, can kick
- **Member** -- Standard member
- **Recruit** -- New joiners

### Guild Operations

- **Create**: Must be at a guild hall, not already in a guild
- **Join**: Must be at a guild hall, not already in a guild
- **Leave**: Anyone except the leader can leave
- **Donate gold**: Deducted from player, added to guild treasury
- **Promote**: Leader/Officers can promote Recruit->Member->Officer
- **Kick**: Leader/Officers can kick (not the leader)

### REST API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/guilds` | List all guilds |
| POST | `/api/guilds` | Create a guild |
| GET | `/api/guilds/mine` | Get your guild info |
| POST | `/api/guilds/join` | Join a guild |
| POST | `/api/guilds/leave` | Leave your guild |
| POST | `/api/guilds/donate` | Donate gold to treasury |
| POST | `/api/guilds/promote` | Promote a member |
| POST | `/api/guilds/kick` | Kick a member |

### WebSocket Protocol

**Client -> Server:**
- `create_guild { name, guild_type }`
- `join_guild { guild_name }`
- `leave_guild`
- `donate_to_guild { gold }`
- `get_guild_info`
- `list_guilds`
- `promote_guild_member { username }`
- `kick_guild_member { username }`

**Server -> Client:**
- `guild_created { guild_id, name }`
- `guild_info { guild }`
- `guild_list { guilds[] }`
- `guild_member_joined { username, guild_name }`
- `guild_member_left { username, guild_name }`

### Persistence

- `data/guilds.json` -- Single JSON file with all guilds (via `GuildStore`)

### Key Files

| File | Purpose |
|------|---------|
| `src/engine/guild.rs` | Guild data structures, member management |
| `src/storage/guild_store.rs` | JSON persistence for guilds |



## Shop System

### Overview

Each town in the hex world has its own persistent shop with tier-based inventory, dynamic pricing, and shared stock across all players. Shops are accessed through a fixed UI panel (no LLM involvement). LLM tools `buy_item`, `sell_item`, and `view_shop` have been removed.

### Dynamic Pricing

```
buy_price = base_price * clamp(1.0 + (base_stock - current_stock) * sensitivity, 0.25, 3.0)
sell_price = buy_price * 60%
```

- `sensitivity` = 0.03 (same-tier items) or 0.05 (above-tier items)
- Prices rise when stock is depleted (players bought items)
- Prices drop when stock has surplus (players sold items)
- Floor: 25% of base price. Ceiling: 300% of base price

### Shared Inventory

Shop state is global — stored in `data/shops.json`, shared across all players. When one player sells an item, it appears for other players to buy. Non-base items (player-sold) gradually drain via restock.

### Lazy Restock

Restocking is calculated on access (no background timer):
- Each hour elapsed since last access, every item moves 1 unit toward its target
- Base items restock toward `base_stock`
- Player-sold items drain toward 0 and are removed when empty

### Persistence

- `data/shops.json` — All shop states, keyed by hex coordinates `"q_r"`
- `ShopStore` with `Arc<RwLock>` in-memory caching for concurrent access
- Shops are created lazily on first visit from county tier

### REST API Endpoints

| Method | Path | Auth | Request Body | Response |
|--------|------|------|-------------|----------|
| `GET` | `/api/shop?adventure_id=X` | Yes | -- | `{shop_name, tier, items[], player_gold, player_inventory[]}` |
| `POST` | `/api/shop/buy` | Yes | `{adventure_id, item_id, quantity}` | `{success, message, item_name?, price_paid?, gold_remaining?}` |
| `POST` | `/api/shop/sell` | Yes | `{adventure_id, item_name, quantity}` | `{success, message, item_name?, gold_earned?, gold_remaining?}` |

### WebSocket Protocol

**Client -> Server:**
| Type | Fields | Description |
|------|--------|-------------|
| `view_shop` | -- | Request shop inventory at current location |
| `shop_buy` | `item_id, quantity` | Buy from shop |
| `shop_sell` | `item_name, quantity` | Sell to shop |

**Server -> Client:**
| Type | Fields | Description |
|------|--------|-------------|
| `shop_inventory` | `shop_name, tier, items[], player_gold, player_inventory[]` | Full shop view |
| `shop_buy_result` | `success, message, item_name?, price_paid?, gold_remaining?` | Buy outcome |
| `shop_sell_result` | `success, message, item_name?, gold_earned?, gold_remaining?` | Sell outcome |

### UI: Shop

Modal overlay with Buy/Sell tabs. Items show image thumbnails, price (color-coded: red=scarce, green=surplus, gold=normal), stock count, and buy/sell buttons. Player-sold items are labeled.

### Key Files

| File | Purpose |
|------|---------|
| `src/engine/shop.rs` | Shop data model, pricing, restock, buy/sell logic |
| `src/storage/shop_store.rs` | JSON persistence with RwLock caching |
| `static/js/adventure.js` | Shop panel (renderShopPanel), Skills panel (renderSkills), Crafting panel (renderCraftingPanel), Equipment comparison (renderItemComparison) |
| `static/js/app.js` | RecipeList/CraftResult WebSocket handlers, crafting button in fixed actions, switchToCraftingTab |
| `static/css/adventure.css` | Shop, skills, crafting, and equipment comparison styles |

### UI: Skills Tab

The info panel has 5 tabs: Status, Items, Skills, Map, Quests. The Skills tab (renderSkills in adventure.js) displays all character skills split into two sections:
- **Combat Skills** (34): All non-crafting skills
- **Crafting Skills** (10): Leatherworking, Smithing, Woodworking, Alchemy, Enchanting, Tailoring, Jewelcrafting, Runecrafting, Artificing, Theurgy

Each skill row shows: name, rank name (Untrained through Transcendent), numeric rank, and an XP progress bar. Bar color changes by rank tier: blue (0-2), green (3-5), gold (6+).

### UI: Crafting Panel

When the player is at a location with crafting stations (`map_view.current.stations`), a "Craft" button appears in the fixed actions area. Clicking it:
1. Sends a `ListRecipes` WebSocket message to the server
2. Switches the info panel to a temporary "Crafting" tab

The crafting panel shows:
- Available crafting stations with tier limits
- Recipe cards showing: name, tier, required skill + rank, input materials with have/need counts
- Equipment comparison for equipment recipes (see below)
- "CRAFT" button on recipes where all requirements are met

WebSocket messages handled:
- `RecipeList` (server -> client): `{ recipes: [...] }` - stored in `gameState._recipes`, triggers panel re-render
- `CraftResult` (server -> client): `{ output, quantity, skill_progress }` - shown as narrative text, triggers recipe refresh

Craft action sent via: `window.rqWs.send({ type: 'CraftItem', recipe_id: '...' })`

### UI: Equipment Comparison

When a crafting recipe produces equipment with a slot (`recipe.output_item.slot`), the recipe card includes a side-by-side comparison between the new item and the currently equipped item in that slot.

Comparison shows: item name, damage dice + modifier, attack bonus, AC base, and tier for both items. If nothing is equipped in the slot, shows "Nothing equipped".

## Image Generation System

### Overview

Entity images are generated via xAI's Grok Imagine API (`grok-imagine-image` model, $0.02/image) and cached to disk. Images are generated for items, monsters, and NPCs in a detailed fantasy RPG art style.

### Storage

Images are cached at `{data_dir}/images/{category}/{key}.jpg`:
- `images/items/` — Item icons (keyed by item ID or sanitized name)
- `images/monsters/` — Monster portraits (keyed by sanitized monster name)
- `images/npcs/` — NPC portraits (keyed by sanitized NPC name or image_id)

### Image Serving Endpoint

| Method | Path | Auth | Response |
|--------|------|------|----------|
| `GET` | `/api/images/{category}/{id}` | No | JPEG image or 404 |

Available on both port 2998 and port 2999. Categories: `items`, `monsters`, `npcs`.
Cache headers: `Cache-Control: public, max-age=31536000, immutable`.
Path traversal protection: IDs sanitized to `[a-zA-Z0-9_-]` only.

### Pre-generation CLI

```bash
cargo run -- generate-images                    # Generate all missing images
cargo run -- generate-images --category items   # Only items
cargo run -- generate-images --dry-run          # Show what would be generated
cargo run -- generate-images --concurrency 5    # Increase parallelism
```

Generates images for: all item database entries (~57), all monster templates (44), and fixed NPC templates (12). Estimated cost: ~$2.30 for full generation.

### Background Generation

When the LLM creates entities during gameplay, images are generated asynchronously:
- `start_combat` → generates images for all enemies in the encounter
- `create_npc` → generates NPC portrait
- `add_item` → generates item icon for custom items

Generation is non-blocking (`tokio::spawn`). The frontend uses `onerror` fallbacks to show emoji/text when images are not yet available.

### Fixed NPCs

New adventures are seeded with 6 fixed NPCs at the player's starting location:
- Marta the Innkeeper (Merchant)
- Brynn Ironhand (Civilian)
- Captain of the Guard (Guard)
- The Healer (Civilian)
- Dockmaster Kael (Merchant)
- Guildmaster Thorne (Civilian)

These have stable `image_id` values used for pre-generated portraits.

### Frontend Integration

- **Inventory**: Item thumbnails (28x28px) with emoji fallback
- **Equipment slots**: Small thumbnails (22x22px) next to item names
- **Combat**: Enemy portraits (36x36px) in HP bars and target selector
- **NPCs**: Circular portraits (32x32px) in quest/NPC panel

### Data Model

- `Item.image_id: Option<String>` — For LLM-created items (known items use their `id` field)
- `Npc.image_id: Option<String>` — Image cache key for the NPC

### Key Files

| File | Purpose |
|------|---------|
| `src/llm/images.rs` | Image generation client, prompt building, disk cache |
| `src/web/server.rs` | Image serving endpoint (port 2999) |
| `src/web/api_server.rs` | Image serving endpoint (port 2998) |
| `src/engine/adventure.rs` | Fixed NPC creation during adventure setup |

## Auth System

- **Passwords:** Argon2id with random salt
- **JWT:** HS256, 24-hour expiry. Secret auto-generated (64 bytes, file perms 0o600)
- **Claims:** `{sub: username, role: "admin"|"user", exp, iat}`
- **Username rules:** 3-32 chars, lowercase alphanumeric + hyphens, no leading/trailing hyphens
- **Auth mode:** Enabled if `--require-auth` flag or any users exist. Disabled = default admin user
- **Middleware:** Token from `Authorization: Bearer` header or `?token=` query param
- **Timing attack mitigation:** 1-second sleep on failed auth

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `XAI_API_KEY` | (required) | xAI API key |
| `XAI_MODEL` | `grok-4-1-fast-reasoning` | LLM model |
| `RUNEQUEST_PORT` | 2999 | Web server port |
| `RUNEQUEST_API_PORT` | 2998 | REST API port |
| `RUNEQUEST_BIND_ADDR` | 0.0.0.0 | Bind address |
| `RUNEQUEST_DATA_DIR` | `$XDG_DATA_HOME/runequest` | Data directory |
| `RUNEQUEST_REQUIRE_AUTH` | false | Force auth |

## Build & Deploy

```bash
cargo build --release                    # Incremental release build
systemctl --user restart runequest       # Restart service
```

## Test Infrastructure

**Test user:** `test-user` / `test-password1`

| File | Tests | Target | Description |
|------|-------|--------|-------------|
| `tests/login.spec.ts` | 5 | Port 2999 | Login page, auth flow, session |
| `tests/engine.spec.ts` | 34 | Port 2998 | API: auth, adventures, character, equipment, dice, conditions, combat, items |
| `tests/adventure.spec.ts` | 17 | Port 2999 | UI: adventure select, character creation, race/background selection, tabs, narrative |
| `tests/world.spec.ts` | ? | Port 2998 | World map, travel, scenarios, dungeons |
| `src/engine/dice.rs` | 6 | Unit | Dice rolling, modifiers, DC checks, probability |

Run all: `cargo test && npx playwright test`
