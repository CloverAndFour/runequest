# Shop API Reference

## Overview

Each town in the hex world has a persistent shop with tier-based inventory, dynamic pricing, and shared stock across all players. Shops are accessed through a fixed UI — no LLM involvement.

Shop state is global (`data/shops.json`), shared across all players. Shops are created lazily on first visit based on county tier.

## REST Endpoints

All endpoints require `Authorization: Bearer <JWT>`.

### View Shop

```
GET /api/shop?adventure_id=<uuid>
```

Returns the shop inventory at the player's current location. Requires the player to be at a town.

**Response (200):**
```json
{
  "shop_name": "Ironhaven Market",
  "tier": 3,
  "items": [
    {
      "item_id": "longsword",
      "name": "Longsword",
      "current_stock": 3,
      "base_stock": 5,
      "buy_price": 28,
      "sell_price": 16,
      "price_category": "above",
      "is_player_sold": false,
      "tier": 2,
      "item_type": "weapon"
    }
  ],
  "player_gold": 150,
  "player_inventory": [
    {
      "item_name": "Dagger",
      "item_id": "dagger",
      "quantity": 2,
      "sell_price": 1,
      "item_type": "weapon"
    }
  ]
}
```

**Error (400):** `{"error": "No shop at current location"}` or `{"error": "adventure_id required"}`

### Buy Item

```
POST /api/shop/buy
```

**Request:**
```json
{
  "adventure_id": "<uuid>",
  "item_id": "longsword",
  "quantity": 1
}
```

`quantity` defaults to 1 if omitted.

**Response (200):**
```json
{
  "success": true,
  "message": "Bought Longsword x1 for 28 gp",
  "item_name": "Longsword",
  "price_paid": 28,
  "gold_remaining": 122
}
```

**Failure:**
```json
{
  "success": false,
  "message": "Not enough gold (need 28, have 10)"
}
```

### Sell Item

```
POST /api/shop/sell
```

**Request:**
```json
{
  "adventure_id": "<uuid>",
  "item_name": "Longsword",
  "quantity": 1
}
```

**Response (200):**
```json
{
  "success": true,
  "message": "Sold Longsword x1 for 16 gp",
  "item_name": "Longsword",
  "gold_earned": 16,
  "gold_remaining": 138
}
```

## WebSocket Messages

### Client → Server

| Type | Fields | Description |
|------|--------|-------------|
| `view_shop` | — | Request shop inventory at current town |
| `shop_buy` | `item_id, quantity` | Buy item (quantity defaults to 1) |
| `shop_sell` | `item_name, quantity` | Sell item (quantity defaults to 1) |

### Server → Client

| Type | Fields | Description |
|------|--------|-------------|
| `shop_inventory` | `shop_name, tier, items[], player_gold, player_inventory[]` | Full shop view |
| `shop_buy_result` | `success, message, item_name?, price_paid?, gold_remaining?` | Buy outcome |
| `shop_sell_result` | `success, message, item_name?, gold_earned?, gold_remaining?` | Sell outcome |

## Price Categories

The `price_category` field indicates price relative to base:

| Category | Meaning | UI Color |
|----------|---------|----------|
| `above` | Stock depleted, price higher than base | Red |
| `normal` | Stock near base level | Gold |
| `below` | Stock surplus, price lower than base | Green |

## Notes

- `is_player_sold: true` items were sold to the shop by another player and will drain over time
- Buying from a shop at 0 stock will fail
- Selling an item the shop doesn't normally carry creates a temporary listing that drains at 1/hr
- The `sell_price` shown in responses is always 60% of the current `buy_price`
