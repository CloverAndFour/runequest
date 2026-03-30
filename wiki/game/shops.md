# Shops

## Overview

Every town in the world has a shop. Shops sell tier-appropriate equipment and consumables. All shops have persistent, shared inventory — what one player buys or sells affects every other player.

## Accessing Shops

Click the **Visit Shop** button when at a town location. This opens a modal panel with Buy and Sell tabs.

## Dynamic Pricing

Shop prices adjust based on supply and demand.

### Formula

```
buy_price = base_price × clamp(1.0 + (base_stock - current_stock) × sensitivity, 0.25, 3.0)
sell_price = buy_price × 60%
```

### Price Sensitivity

| Item Relationship to Town Tier | Sensitivity |
|-------------------------------|-------------|
| Same tier or below | 0.03 per unit |
| Above town tier | 0.05 per unit |

### Examples

Starting with base_stock = 5, base_price = 100gp, sensitivity = 0.04:

| Scenario | Current Stock | Multiplier | Buy Price | Sell Price |
|----------|--------------|------------|-----------|------------|
| Untouched | 5 | 1.00× | 100 gp | 60 gp |
| 2 bought | 3 | 1.08× | 108 gp | 65 gp |
| 5 bought (depleted) | 0 | 1.20× | 120 gp | 72 gp |
| 5 sold (surplus) | 10 | 0.80× | 80 gp | 48 gp |
| 20 sold (heavy surplus) | 25 | 0.25× (floor) | 25 gp | 15 gp |

### Price Floor and Ceiling

- **Floor**: 25% of base price — even with massive surplus, items never become worthless
- **Ceiling**: 300% of base price — even with total depletion, prices are capped

## Restocking

Shops restock **1 unit per hour** toward their base inventory:

- **Base items** (shop's normal stock): If depleted by purchases, they reappear at 1/hour
- **Player-sold items**: If not part of the base inventory, they drain at 1/hour and eventually disappear

Restocking is calculated lazily — it happens when any player opens the shop, not on a background timer.

## Shared Inventory

Shop state is global across all players. This enables:

- **Player-to-player indirect trading**: Sell a rare item at one shop, another player can buy it
- **Market dynamics**: Heavy buying in one town raises prices; selling lowers them
- **Regional economics**: Each town has its own independent shop state

## Shop Inventory by Tier

Shop inventory is determined by the town's county tier. Higher-tier towns stock higher-tier items.

| Town Tier | Available Items |
|-----------|----------------|
| T1 | Dagger, Shortsword, Leather Armor, Shield, Health Potion |
| T2 | + Longsword, Chain Shirt, Chain Mail, Longbow, Rapier, Scale Mail, Studded Leather |
| T3 | + Greatsword, Greataxe, Breastplate, Half Plate |
| T4 | + Plate Armor, Flametongue Longsword |

Items one tier above the town's tier have limited stock (1) and a 1.5× price premium.

## Base Stock

- **Same-tier items**: Base stock of 5
- **Above-tier items**: Base stock of 1
- **Health Potions**: Always stocked (unlimited base stock at all tiers)

## UI

The shop panel is a modal overlay with:

- **Header**: Shop name, tier badge, gold display
- **Buy tab**: Items sorted by tier with image thumbnails, price (color-coded), stock count, buy button
- **Sell tab**: Player inventory items with sell prices and sell buttons
- **Price colors**: Red (scarce/expensive), Gold (normal), Green (surplus/cheap)
- **Player-sold label**: Items sold by other players are marked

## Design Principles

- **No LLM involvement**: Shops are entirely UI-driven for speed and consistency
- **Real economics**: Supply and demand create meaningful price signals
- **Cross-player interaction**: Shops are a shared resource, encouraging economic gameplay
- **Slow restock**: 1 unit/hour means player actions have lasting impact (~hours)
- **Per-town independence**: Each town is its own market; geographic arbitrage is possible
