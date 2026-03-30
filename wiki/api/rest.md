# REST API Reference (Port 2998)

## Overview

All protected routes require `Authorization: Bearer <JWT>` header.
CORS is fully open (any origin/method/headers).

Base URL: `http://host:2998`

## Authentication

| Method | Path | Auth | Request Body | Response |
|---|---|---|---|---|
| POST | `/api/auth/login` | No | `{username, password}` | `{token, username, role}` or 401 |
| GET | `/health` | No | -- | `"ok"` |

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
