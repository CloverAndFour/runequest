# Trading & Social Systems

## Player-to-Player Trading

Two players at the **same world map location** can trade items and gold directly. Both must have an active adventure loaded.

### Trade Flow

1. Player A sends `send_trade_request { target_username }` -- must be at same location
2. Target receives `trade_request_received` notification
3. Target sends `accept_trade_request { from_username }` -- creates a TradeSession
4. Both receive `trade_started { partner }`
5. Either player sends `update_trade_offer { items, gold }` -- both see updated offers
6. When satisfied, each player sends `accept_trade`
7. **Important:** Acceptances reset if either player changes their offer
8. When both have accepted: items and gold swap atomically, both adventures saved
9. Both receive `trade_completed { items_gained, items_lost, gold_delta }`

### Trade Validation

- Both players must be at the same world location
- Both must be online with an active adventure
- Items must exist in inventory when trade executes
- Gold must be sufficient when trade executes
- One trade at a time per player

### Trade Cancellation

Either player can cancel at any time via `cancel_trade`. Trade is also cancelled if either player disconnects.

### Cross-User Notifications

Trade notifications use the PresenceRegistry's FriendEvent channel with `__TRADE_MSG__` prefixed JSON payloads piggybacked on LocationChat events.

## Central Exchange System

Order book available at specific world locations. Players place buy/sell limit orders for items. Orders match automatically.

### Exchange Locations

5 exchange locations across the map, placed at tier-appropriate positions (~T1.5, T3, T5, T7, T9). Counties with `has_exchange: true`.

### Order Types

| Type | Description | Escrow |
|---|---|---|
| **Limit Buy** | "Buy X of item Y at Z gp each" | Gold locked when order placed |
| **Limit Sell** | "Sell X of item Y at Z gp each" | Items locked when order placed |

### Matching Algorithm

- **Price-time priority:** Oldest matching order executes first
- **Automatic matching:** New orders immediately try to match against opposing orders
- **Partial fills:** Orders can be partially filled
- **Same-player protection:** Your own orders never match against each other
- Unfilled portions remain on the book until cancelled

### Exchange API

**REST Endpoints:**

| Method | Path | Description |
|---|---|---|
| GET | `/api/exchange/orders` | List all orders (optional `?item_id=` filter) |
| POST | `/api/exchange/orders` | Place a new order |
| GET | `/api/exchange/orders/mine` | List your orders |
| POST | `/api/exchange/orders/cancel` | Cancel an order |

**WebSocket Messages:**

Client -> Server:
- `place_exchange_order { item_id, order_type, quantity, price_per_unit }`
- `list_exchange_orders { item_id? }`
- `cancel_exchange_order { order_id }`
- `get_my_exchange_orders`

Server -> Client:
- `exchange_order_placed { order_id, trades[] }`
- `exchange_orders { orders[] }`
- `exchange_order_cancelled { order_id, refunded }`

### Exchange Persistence

Orders stored in `data/exchange.json` via `ExchangeStore`.

## Guild System

Player organizations with ranks, a shared treasury, and guild halls.

### Guild Types

| Type | Focus |
|---|---|
| **Combat** | Monster hunting, PvP, dungeon clearing |
| **Crafting** | Material gathering, crafting, trade |

### Guild Ranks

| Rank | Invite | Kick | Promote | Withdraw Treasury |
|---|---|---|---|---|
| **Leader** | Yes | Yes | Yes | Yes |
| **Officer** | Yes | Recruits only | Recruits -> Members | No |
| **Member** | No | No | No | No |
| **Recruit** | No | No | No | No |

### Guild Rules

- Maximum 50 members per guild
- One guild per player
- Must be at a guild hall location to create, join, or manage
- Leader can promote: Recruit -> Member -> Officer
- Leader/Officers can kick (not the leader)
- Anyone except the leader can leave
- Gold donations go to the shared treasury

### Guild Hall Locations

~3% of towns have guild halls (`has_guild_hall: true`). Notable locations include Crossroads Inn, Thornwall Village, Port Blackwater, and Frosthold.

### Guild API

**REST Endpoints:**

| Method | Path | Description |
|---|---|---|
| GET | `/api/guilds` | List all guilds |
| POST | `/api/guilds` | Create a guild |
| GET | `/api/guilds/mine` | Get your guild info |
| POST | `/api/guilds/join` | Join a guild |
| POST | `/api/guilds/leave` | Leave your guild |
| POST | `/api/guilds/donate` | Donate gold to treasury |
| POST | `/api/guilds/promote` | Promote a member |
| POST | `/api/guilds/kick` | Kick a member |

**WebSocket Messages:**

Client -> Server:
- `create_guild { name, guild_type }`
- `join_guild { guild_name }`
- `leave_guild`
- `donate_to_guild { gold }`
- `get_guild_info`
- `list_guilds`
- `promote_guild_member { username }`
- `kick_guild_member { username }`

Server -> Client:
- `guild_created { guild_id, name }`
- `guild_info { guild }`
- `guild_list { guilds[] }`
- `guild_member_joined { username, guild_name }`
- `guild_member_left { username, guild_name }`

### Guild Persistence

Stored in `data/guilds.json` via `GuildStore`.
