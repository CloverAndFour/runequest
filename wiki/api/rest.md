# REST API Reference (Port 2998)

## Overview

All protected routes require `Authorization: Bearer <JWT>` header or `Authorization: Bearer <API_KEY>` (keys with `rq_` prefix).
CORS is fully open (any origin/method/headers).

Base URL: `http://host:2998` (or `https://host:2998` when TLS enabled)

## Authentication

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/auth/login` | No | `{username, password}` | `{token, username, role}` or 401 |
| POST | `/api/auth/change-password` | Yes | `{current_password, new_password}` | `{success: true}` or 400 `{error}` |
| POST | `/api/auth/api-keys` | Yes | `{name}` | `{id, name, key, prefix, created_at}` (key shown once) |
| GET | `/api/auth/api-keys` | Yes | -- | `[{id, name, prefix, created_at, last_used}]` |
| DELETE | `/api/auth/api-keys/:key_id` | Yes | -- | `{success: true}` |
| GET | `/health` | No | -- | `"ok"` |

### Change Password

`POST /api/auth/change-password`

Requires authentication. Verifies current password with argon2id before updating.

**Request:**
```json
{
  "current_password": "old-password",
  "new_password": "new-password-min-8-chars"
}
```

**Success response (200):**
```json
{ "success": true }
```

**Error response (400):**
```json
{ "error": "Current password is incorrect" }
{ "error": "New password must be at least 8 characters" }
```

### API Keys

API keys provide long-lived authentication tokens for programmatic access (AI agents, bots). Keys use the `rq_` prefix + 32 hex characters (128 bits entropy). Only the SHA-256 hash is stored server-side; the plaintext key is returned once at creation and cannot be retrieved again.

**Limits:** Max 10 keys per user.

**Authentication:** API keys work anywhere JWT tokens work -- `Authorization: Bearer rq_...` header or `?token=rq_...` query parameter. The auth middleware detects the `rq_` prefix to route through hash lookup instead of JWT validation.

**`last_used`:** Updated on each successful authentication with the key.

#### Create API Key

`POST /api/auth/api-keys`

**Request:**
```json
{ "name": "my-bot" }
```

**Response (200):**
```json
{
  "id": "uuid",
  "name": "my-bot",
  "key": "rq_a1b2c3d4e5f6...",
  "prefix": "rq_a1b2",
  "created_at": "2026-03-30T12:00:00Z"
}
```

The `key` field contains the full plaintext key. **This is the only time the key is shown.**

#### List API Keys

`GET /api/auth/api-keys`

**Response (200):**
```json
[
  {
    "id": "uuid",
    "name": "my-bot",
    "prefix": "rq_a1b2",
    "created_at": "2026-03-30T12:00:00Z",
    "last_used": "2026-03-30T15:30:00Z"
  }
]
```

No plaintext keys are returned. Use `prefix` to identify keys.

#### Revoke API Key

`DELETE /api/auth/api-keys/:key_id`

**Response (200):**
```json
{ "success": true }
```

## Adventures

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/adventures` | Yes | -- | `{adventures: [AdventureSummary]}` |
| POST | `/api/adventures` | Yes | `{name, character_name, race, background?, class?, backstory?, stats?, naked_start?}` | `GameResponse` |
| GET | `/api/adventures/:id` | Yes | -- | `GameResponse` |
| DELETE | `/api/adventures/:id` | Yes | -- | `{deleted: true}` |
| GET | `/api/adventures/:id/history` | Yes | -- | `{events: [DisplayEvent]}` |

### Character Creation Fields

Two creation paths:
- **Background path (new):** Provide `race` + `background` (string). Stats default to all-10.
- **Legacy class path:** Provide `race` + `class` (string) + `stats` object. Stats use 27-point buy.

`stats` object (legacy only): `{strength, dexterity, constitution, intelligence, wisdom, charisma}` (8-15 each)

`naked_start` (optional bool): Start with no equipment and 0 gold.

## Game Actions

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/adventures/:id/message` | Yes | `{content}` | `GameResponse` |
| POST | `/api/adventures/:id/choice` | Yes | `{index, text}` | `GameResponse` |
| POST | `/api/adventures/:id/roll` | Yes | -- | `GameResponse` |
| POST | `/api/adventures/:id/travel` | Yes | `{direction}` | `{county_name, county_tier, biome, region, encounter, state}` |
| POST | `/api/adventures/:id/work` | Yes | -- | `{job, gold_earned, skill, skill_xp, state}` |

## Travel (Fixed Action)

`POST /api/adventures/:id/travel`

Move to an adjacent county. This is a direct engine action (no LLM involved). Subject to fixed action cooldown (4s).

**Request:**
```json
{
  "direction": "east"
}
```

`direction` values: `"east"`, `"west"`, `"northeast"`, `"northwest"`, `"southeast"`, `"southwest"`

**Success response (200):**
```json
{
  "county_name": "Greenmeadow",
  "county_tier": 1.2,
  "biome": "Plains",
  "region": "Western Reaches",
  "encounter": false,
  "state": { ... }
}
```

If `encounter` is `true`, combat has been initiated with a random monster. The `state.combat` field will be populated.

**Error response (400):**
```json
{ "error": "Unknown direction 'north'. Use: east, west, northeast, northwest, southeast, southwest" }
```

## Work (Menial Labour)

`POST /api/adventures/:id/work`

Do an odd job for a small amount of gold and skill XP. Available at every county. Subject to fixed action cooldown (4s).

**Request:** No body required.

**Success response (200):**
```json
{
  "job": "Serving tables at the tavern",
  "gold_earned": 2,
  "skill": "charm",
  "skill_xp": 5,
  "state": { ... }
}
```

Jobs vary by location: towns offer tavern/merchant work, wilderness has biome-specific tasks (collecting kindling in forests, breaking rocks in hills, mending nets on coasts, etc.). Gold scales slightly with county tier (base + tier/2).

## Combat

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/adventures/:id/combat` | Yes | `{action_id, target?}` | `GameResponse` |

`action_id` values: `attack`, `dodge`, `dash`, `use_item`, `flee`, `second_wind`, `cunning_hide`, `healing_word`, `reckless_attack`, `lay_on_hands`, `flurry_of_blows`, `end_turn`

## Equipment

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/adventures/:id/equip` | Yes | `{item_name}` | `{result, state}` |
| POST | `/api/adventures/:id/unequip` | Yes | `{slot}` | `{result, state}` |

## Direct Engine Endpoints (Bypass LLM)

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/adventures/:id/engine/hp` | Yes | `{delta, reason}` | `GameResponse` |
| POST | `/api/adventures/:id/engine/item` | Yes | `{item_id}` | `GameResponse` |
| POST | `/api/adventures/:id/engine/gold` | Yes | `{amount}` | `GameResponse` |
| POST | `/api/adventures/:id/engine/xp` | Yes | `{amount, reason}` | `GameResponse` |
| POST | `/api/adventures/:id/engine/condition` | Yes | `{condition, action}` | `GameResponse` |
| POST | `/api/adventures/:id/engine/combat` | Yes | `{enemies: [{name, hp, ac, attacks}]}` | `GameResponse` |
| POST | `/api/adventures/:id/engine/roll` | Yes | `{dice, count?, modifier?, dc?}` | Roll result |
| POST | `/api/adventures/:id/engine/combat/simulate` | Yes | -- | `{simulation_log, combat_state}` |
| POST | `/api/adventures/:id/engine/skill` | Yes | `{action, skill_id?}` | `{result, state}` |

`action` for conditions: `"add"` or `"remove"`

Skill `action` values: `"get"` (list all skills) or `"improve"` (improve by `skill_id`)

## Quest & NPC (Direct Engine)

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/adventures/:id/engine/quest` | Yes | `{action, ...params}` | `{result, state}` |
| POST | `/api/adventures/:id/engine/npc` | Yes | `{action, ...params}` | `{result, state}` |

**Quest actions:** `add` (name, description, final_goal, next_step, reward), `complete` (name), `update_step` (quest_name, step_completed, new_next_step), `fail` (name)

**NPC actions:** `create` (name, description, location?, disposition?), `update` (npc_name, location?, disposition?, description?), `dismiss` (npc_name), `list` (location?), `log_interaction` (npc_name, summary)

## Item Database

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/items` | Yes | -- | `{items: [Item]}` |
| GET | `/api/items/:id` | Yes | -- | `{item: Item}` |

## Backgrounds

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/backgrounds` | Yes | -- | List of backgrounds with starting skills, items, gold |

## Crafting

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/recipes` | Yes | -- | `{recipes: [Recipe]}` |
| GET | `/api/recipes/:recipe_id` | Yes | -- | `Recipe` or 404 |
| GET | `/api/materials` | Yes | -- | `{materials: [Material]}` |
| POST | `/api/adventures/:id/craft` | Yes | `{recipe_id}` | `{result, state}` |

Query params for `/api/recipes`: `skill` (filter by skill ID), `tier` (filter by tier number).

Craft result: `{crafted, output, quantity, skill_progress}` on success, or `{error}` on failure.

## Dungeon

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/adventures/:id/dungeon/enter` | Yes | `{seed?, tier?}` | `{result, dungeon, state}` |
| POST | `/api/adventures/:id/dungeon/move` | Yes | `{direction}` | `{result, room, floor, room_id, state}` |
| POST | `/api/adventures/:id/dungeon/skill-check` | Yes | `{direction, skill_id}` | `{result, skill, roll, dc, success}` |
| POST | `/api/adventures/:id/dungeon/activate-point` | Yes | `{puzzle_id, room_id}` | `{result, puzzle_id, activated_count, required_count, solved}` |
| POST | `/api/adventures/:id/dungeon/retreat` | Yes | -- | `{result, message, state}` |
| GET | `/api/adventures/:id/dungeon/status` | Yes | -- | `{in_dungeon, name?, tier?, ...}` |

`direction` values: `"North"`, `"South"`, `"East"`, `"West"`, `"Descend"`, `"Ascend"`

**dungeon/enter:** Generates a dungeon at the player's current county (must have `has_dungeon: true`). Optional `seed` overrides the dungeon seed; optional `tier` overrides the county's dungeon tier.

**dungeon/move:** Move in a cardinal direction or descend/ascend stairs. Returns the new room contents.

**dungeon/skill-check:** Attempt to pass a skill gate blocking a direction. Rolls the player's skill rank against the gate DC. On success the path opens.

**dungeon/activate-point:** Activate a puzzle activation point. Returns how many points are active and whether the puzzle is solved.

**dungeon/retreat:** Leave the dungeon and return to the world map.

**dungeon/status:** Returns current dungeon state: whether the player is in a dungeon, dungeon name, tier, current floor, room, and room contents.

## Tower

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/towers` | Yes | -- | `{towers: [{id, name, base_tier, ...}]}` |
| GET | `/api/towers/:tower_id/floor/:floor_num` | Yes | -- | `{floor: FloorSummary}` |
| POST | `/api/adventures/:id/tower/enter` | Yes | `{tower_id}` | `{result, tower_name, floor, tier, state}` |
| POST | `/api/adventures/:id/tower/move` | Yes | `{direction}` | Same as dungeon/move |
| POST | `/api/adventures/:id/tower/ascend` | Yes | -- | Same as dungeon/move (uses "Descend" direction internally) |
| POST | `/api/adventures/:id/tower/checkpoint` | Yes | `{floor}` | `{checkpoint_attuned, floor, teleport_cost}` |
| POST | `/api/adventures/:id/tower/teleport` | Yes | `{target_floor}` | `{teleport_available, target_floor, cost}` or error if insufficient gold |

**towers (list):** Returns all 10 towers with their ID, name, base tier, and location county.

**towers/:tower_id/floor/:floor_num:** Returns a summary of a specific floor (room count, cleared status, boss presence, etc.). Does not require the player to be in the tower.

**tower/enter:** Enter a tower. The player must be at a county with `has_tower: true` and the `tower_id` must match. Places the player on floor 0, room (0,0).

**tower/move:** Move within the current tower floor. Same direction values as dungeon/move.

**tower/ascend:** Ascend to the next floor. The player must be at the stairs room.

**tower/checkpoint:** Attune to a checkpoint on the current floor. Only available on safe floors (every 10th). Attuning is free; records the checkpoint for later teleportation.

**tower/teleport:** Teleport to a previously attuned checkpoint floor. Costs `floor_number * 10` gold. Returns an error if the player lacks sufficient gold or has not attuned to the target floor.

## Shop

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/adventures/:id/shop` | Yes | — | `{shop_name, tier, items[], player_gold}` |
| POST | `/api/adventures/:id/shop/buy` | Yes | `{item_id, quantity?}` | `{success, message, gold_remaining}` |
| POST | `/api/adventures/:id/shop/sell` | Yes | `{item_name}` | `{success, message, sell_price, gold_remaining}` |

See [Shop API Reference](shop.md) for full request/response schemas and examples.

## Friends

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/friends/code` | Yes | -- | `{tag}` |
| GET | `/api/friends` | Yes | -- | `{friends[], incoming_requests[], outgoing_requests[]}` |
| POST | `/api/friends/request` | Yes | `{friend_tag}` | `{success, message}` |
| POST | `/api/friends/accept` | Yes | `{username}` | `{success}` |
| POST | `/api/friends/decline` | Yes | `{username}` | `{success}` |
| DELETE | `/api/friends/:username` | Yes | -- | `{removed}` |
| POST | `/api/friends/chat` | Yes | `{to, text}` | `{from, text, ts}` |
| GET | `/api/friends/chat/:username` | Yes | -- | `{friend, messages[]}` |

Note: REST endpoints do not include real-time presence updates (use WebSocket for that).

## Location Chat

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/location/chat` | Yes | `{adventure_id, text}` | `{from, text, ts, location}` |
| GET | `/api/location/players?adventure_id=X` | Yes | -- | `{location, players[]}` |

## Party

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/party` | Yes | -- | `{party_id, leader, members, state, location}` |
| POST | `/api/party/invite` | Yes | `{adventure_id, target_username}` | `{success, message}` |
| POST | `/api/party/accept` | Yes | `{adventure_id, from_username}` | `{success}` |
| POST | `/api/party/leave` | Yes | `{adventure_id}` | `{success}` |
| POST | `/api/party/combat/action` | Yes | `{adventure_id, action_id, target?}` | `{success}` |
| POST | `/api/party/combat/ready` | Yes | `{adventure_id}` | `{success}` |

## PvP

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/pvp/challenge` | Yes | `{adventure_id, target_username}` | `{success, message}` |

## Exchange

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/exchange/orders` | Yes | -- | List orders (optional `?item_id=` filter) |
| POST | `/api/exchange/orders` | Yes | `{adventure_id, item_id, order_type, quantity, price_per_unit}` | `{order_id, trades[], state}` |
| GET | `/api/exchange/orders/mine` | Yes | -- | Your orders |
| POST | `/api/exchange/orders/cancel` | Yes | `{order_id}` | Cancel result |

## Guilds

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| GET | `/api/guilds` | Yes | -- | List all guilds |
| POST | `/api/guilds` | Yes | `{adventure_id, name, guild_type}` | `{guild_id, name}` |
| GET | `/api/guilds/mine` | Yes | -- | Your guild info |
| POST | `/api/guilds/join` | Yes | `{adventure_id, guild_name}` | Success/failure |
| POST | `/api/guilds/leave` | Yes | `{adventure_id}` | Success/failure |
| POST | `/api/guilds/donate` | Yes | `{adventure_id, gold}` | Donation result |
| POST | `/api/guilds/promote` | Yes | `{adventure_id, username}` | Promotion result |
| POST | `/api/guilds/kick` | Yes | `{adventure_id, username}` | Kick result |

## Rate Limiting

All game actions are subject to per-character cooldowns. Admin users (role=admin) bypass all limits.

### Action Categories & Cooldowns

| Category | Cooldown | Actions |
|----------|----------|---------|
| LLM | 6 seconds | `/message`, `/choice`, `/combat` |
| Fixed | 4 seconds | `/travel`, `/work`, `/craft`, `/shop/buy`, `/shop/sell`, `/dungeon/*`, `/tower/*` |
| Equipment | 100ms | `/equip`, `/unequip` |
| Read-only | None | `GET` endpoints, `/shop` (view), `/dungeon/status` |

### Cooldown Response

When an action is attempted before the cooldown expires:

```
HTTP 429 Too Many Requests
```

```json
{
  "error": "Action on cooldown",
  "code": "cooldown",
  "remaining_ms": 3421
}
```

The `remaining_ms` field tells the client exactly how long to wait before retrying.

## GameResponse Format

All game action endpoints return this format:

```json
{
  "state": {
    "character": { "name", "race", "class", "level", "xp", "hp", "max_hp", "stats", "equipment" },
    "inventory": [],
    "quests": [],
    "npcs": [],
    "skills": { "skills": [] },
    "gold": 0,
    "conditions": [],
    "map_view": { "hexes": [], "directions": [], "current": {}, "position": {} }
  },
  "narrative": "string or null",
  "pending": {
    "pending_type": "dice_roll | choices",
    "dice_type": "d20", "count": 1, "modifier": 0, "dc": 12,
    "description": "...", "success_probability": 0.45,
    "choices": [], "allow_custom_input": true, "prompt": "..."
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

### state.map_view

Injected into every state response. Provides a 3-ring hex neighborhood (37 hexes):

```json
{
  "hexes": [
    { "q": 0, "r": 0, "abs_q": 42, "abs_r": -217, "current": true, "discovered": true,
      "name": "County Name", "tier": "2.0", "biome": "Forest", "has_town": true,
      "has_dungeon": false, "has_tower": false, "tower_name": null,
      "has_exchange": false, "has_guild_hall": false, "region": "Region Name" }
  ],
  "directions": [
    { "direction": "East", "name": "Neighbor", "tier": "1.5", "biome": "Plains", "has_town": false }
  ],
  "current": {
    "name": "Current County", "tier": 2.0, "biome": "Forest", "region": "Region",
    "has_town": true, "has_dungeon": false, "has_tower": false,
    "has_exchange": false, "has_guild_hall": false,
    "stations": [
      {"type": "TanningRack", "name": "Tanning Rack", "max_tier": 3, "skills": ["leatherworking"]}
    ]
  },
  "position": { "q": 42, "r": -217 }
}
```

Undiscovered hexes show `name: "???"`, `tier: "?"`, `biome: "unknown"`, and null for features.
