# Shop API Reference

## Overview

Each town in the hex world has a persistent shop with tier-based inventory, dynamic pricing, and shared stock across all players. Shops are accessible through the REST API, WebSocket messages, and LLM tools — all backed by the same `ShopStore` engine.

**The engine controls all prices.** Neither the LLM nor the client can set or influence prices. Prices are calculated dynamically from shop state using the formula:

```
buy_price = base_price × clamp(1.0 + (base_stock - current_stock) × sensitivity, 0.25, 3.0)
sell_price = buy_price × 60%   (minimum 1 gold)
```

Shop state is global (`data/shops.json`), shared across all players. Shops are created lazily on first visit based on county tier.

## REST Endpoints

All endpoints require `Authorization: Bearer <JWT>`.

### View Shop

```
GET /api/adventures/:id/shop
```

Returns the shop inventory at the player's current location. Requires the player to be at a town.

**Response (200):**
```json
{
  "shop_name": "Ironhaven General Store",
  "tier": 3,
  "location": "Ironhaven",
  "items": [
    {
      "item_id": "longsword",
      "name": "Longsword",
      "description": "A double-edged blade, well-balanced for combat.",
      "buy_price": 28,
      "sell_price": 16,
      "current_stock": 3,
      "price_category": "above",
      "tier": 2
    }
  ],
  "player_gold": 150
}
```

**Error (400):** `{"error": "No shop here — this county has no town."}`
**Error (404):** `{"error": "Adventure not found"}`

### Buy Item

```
POST /api/adventures/:id/shop/buy
```

**Request:**
```json
{
  "item_id": "longsword",
  "quantity": 1
}
```

`quantity` defaults to 1 if omitted.

**Response (200) — success:**
```json
{
  "success": true,
  "message": "Bought Longsword x1 for 28 gold",
  "gold_remaining": 122
}
```

**Response (200) — failure:**
```json
{
  "success": false,
  "error": "Not enough gold (need 28, have 10)"
}
```

Other possible errors: `"Not enough stock (have 0, want 1)"`, `"Item 'xxx' not available"`, `"No shop at this location."`.

On success, the adventure state is persisted automatically.

### Sell Item

```
POST /api/adventures/:id/shop/sell
```

**Request:**
```json
{
  "item_name": "Longsword"
}
```

Item lookup is case-insensitive against the player's inventory.

**Response (200) — success:**
```json
{
  "success": true,
  "message": "Sold Longsword for 16 gold",
  "sell_price": 16,
  "gold_remaining": 138
}
```

**Response (200) — failure:**
```json
{
  "success": false,
  "error": "Item 'Longsword' not found in inventory"
}
```

On success, the adventure state is persisted automatically.

## WebSocket Messages

### Client → Server

| Type | Fields | Description |
|------|--------|-------------|
| `view_shop` | — | Request shop inventory at current town |
| `shop_buy` | `item_id` (string), `quantity` (u32, default 1) | Buy item from shop |
| `shop_sell` | `item_name` (string) | Sell item from inventory |

**Example:**
```json
{"type": "view_shop"}
{"type": "shop_buy", "item_id": "longsword", "quantity": 2}
{"type": "shop_sell", "item_name": "Dagger"}
```

### Server → Client

| Type | Fields | Description |
|------|--------|-------------|
| `shop_inventory` | `shop_name`, `tier`, `items[]`, `player_gold` | Full shop view |
| `shop_buy_result` | `success`, `item_name`, `price`, `gold_remaining`, `error?` | Buy outcome |
| `shop_sell_result` | `success`, `item_name`, `gold_earned`, `gold_remaining`, `error?` | Sell outcome |

On successful buy/sell, the server also sends a `state_update` with the full updated adventure state.

**Example `shop_inventory`:**
```json
{
  "type": "shop_inventory",
  "shop_name": "Ironhaven General Store",
  "tier": 3,
  "items": [
    {
      "item_id": "longsword",
      "name": "Longsword",
      "description": "A double-edged blade, well-balanced for combat.",
      "buy_price": 28,
      "sell_price": 16,
      "current_stock": 3,
      "price_category": "above",
      "tier": 2
    }
  ],
  "player_gold": 150
}
```

**Example `shop_buy_result`:**
```json
{
  "type": "shop_buy_result",
  "success": true,
  "item_name": "Bought Longsword x1 for 28 gold",
  "price": 28,
  "gold_remaining": 122
}
```

## ShopItemInfo Schema

| Field | Type | Description |
|-------|------|-------------|
| `item_id` | string | Equipment/material ID |
| `name` | string | Display name |
| `description` | string | Item description |
| `buy_price` | u32 | Current buy price (dynamic) |
| `sell_price` | u32 | Current sell price (60% of buy) |
| `current_stock` | u32 | Units available |
| `price_category` | string | `"above"`, `"normal"`, or `"below"` |
| `tier` | u8 | Item tier |

## Price Categories

| Category | Meaning | Cause |
|----------|---------|-------|
| `above` | Price higher than base | Stock depleted (players bought items) |
| `normal` | Price near base level | Stock at or near base level |
| `below` | Price lower than base | Stock surplus (players sold items) |

## LLM Tool Integration

The LLM tools `buy_item`, `sell_item`, and `view_shop` are still available and route through `execute_tool_call_with_shop()`. The executor receives `Option<&ShopStore>` and uses it to look up engine-controlled prices. The LLM cannot set or influence prices — it can only trigger buy/sell at whatever the engine's current price is.

## Notes

- Selling an item the shop doesn't normally carry creates a temporary listing with `sensitivity: 0.04` that drains at 1 unit/hour via lazy restock
- Buying from a shop at 0 stock will fail with an error
- Shop state is keyed by hex coordinates `"q_r"` — each town has one shop
- Shops are created lazily on first visit, initialized from the county's tier
