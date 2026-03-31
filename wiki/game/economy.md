# Economy System

## Currency

**Gold Pieces (GP)** -- the universal currency.

### Gold Sources
- Quest rewards (LLM-driven)
- Selling items/materials to shops (60% of dynamic buy price)
- Trading with other players
- Exchange sales (limit orders)
- **Menial labour / odd jobs** (1-2 gold + tier bonus, available everywhere)

### Gold Sinks
- Buying from shops (dynamic pricing)
- Buying materials for crafting
- Exchange purchases
- Guild treasury donations

## Dynamic Shop System

Every town in the hex world has its own shop with dynamic pricing and shared inventory across all players.

### Shop Stock by County Tier

- **All towns:** Health Potions
- **T0 counties:** Basic items (dagger, leather armor)
- **T1 counties:** + longsword, chain mail, longbow
- **T2 counties:** + greatsword, breastplate, Greater Health Potions
- **T3+ counties:** + magical items (limited stock, 1.5x markup)
- **All towns:** Crafting materials (intermediate materials near county tier)
- **All towns:** Pre-made equipment at 3x markup (blade/bow/dagger/staff weapons + armor, up to T5)

**Tiers 6+ equipment is NEVER sold in shops.** Must be crafted.

### Dynamic Pricing Formula

```
buy_price = base_price * clamp(1.0 + (base_stock - current_stock) * sensitivity, 0.25, 3.0)
sell_price = buy_price * 60%
```

- `sensitivity` = 0.03 (same-tier items) or 0.05 (above-tier items)
- Prices rise when stock is depleted (players bought items)
- Prices drop when stock has surplus (players sold items)
- Floor: 25% of base price. Ceiling: 300% of base price.

### Shared Inventory

Shop state is global -- stored in `data/shops.json`, shared across all players. When one player sells an item, it appears for other players to buy. Non-base items (player-sold) gradually drain via lazy restock.

### Lazy Restock

Restocking is calculated on access (no background timer):
- Each hour elapsed since last access, every item moves 1 unit toward its target
- Base items restock toward `base_stock`
- Player-sold items drain toward 0 and are removed when empty

## Material Pricing by Tier (tier_to_value)

| Tier | GP Value | Rarity | Weapon Value (3x) | Armor Value (4x) |
|---|---|---|---|---|
| T0 | 1 gp | Common | 3 gp | 4 gp |
| T1 | 5 gp | Common | 15 gp | 20 gp |
| T2 | 15 gp | Uncommon | 45 gp | 60 gp |
| T3 | 50 gp | Uncommon | 150 gp | 200 gp |
| T4 | 175 gp | Rare | 525 gp | 700 gp |
| T5 | 600 gp | Rare | 1,800 gp | 2,400 gp |
| T6 | 2,100 gp | Epic | 6,300 gp | 8,400 gp |
| T7 | 7,500 gp | Epic | 22,500 gp | 30,000 gp |
| T8 | 26,000 gp | Legendary | 78,000 gp | 104,000 gp |
| T9 | 90,000 gp | Legendary | 270,000 gp | 360,000 gp |
| T10 | 300,000 gp | Legendary | 900,000 gp | 1,200,000 gp |

## Menial Labour (Odd Jobs)

Players can do odd jobs at any county for a small amount of gold and skill XP. This provides a reliable, always-available income source for new players or anyone low on gold.

### Endpoints

- **WS:** `work` -> `work_result { job, gold_earned, skill, skill_xp }` + `state_update`
- **REST:** `POST /api/adventures/:id/work` -> `{ job, gold_earned, skill, skill_xp, state }`
- Subject to fixed action cooldown (4s API / 1s browser)

### Jobs by Location

| Context | Available Jobs | Skill | Base Gold |
|---|---|---|---|
| Everywhere | Chopping firewood | Woodworking | 1 |
| Everywhere | Hauling cargo | Fortitude | 1 |
| Towns only | Serving tables at the tavern | Charm | 2 |
| Towns only | Sweeping the market square | Fortitude | 1 |
| Towns only | Running errands for merchants | Charm | 2 |
| Forest | Collecting kindling | Survival | 1 |
| Hills / Mountains | Breaking rocks | Smithing | 1 |
| Coast | Mending fishing nets | Leatherworking | 1 |
| Swamp | Draining ditches | Fortitude | 1 |
| Desert | Digging wells | Fortitude | 2 |
| Plains / other | Tending crops | Survival | 1 |

### Pay Scaling

- **Gold:** `base_gold + floor(county_tier / 2)`
  - T0 county: 1-2 gold
  - T4 county: 3-4 gold
  - T8 county: 5-6 gold
- **Skill XP:** 3-5 XP to the job's associated skill

### Design Rationale

Odd jobs exist to prevent soft-locking: a player who has lost all equipment and gold can always work their way back. The gold rate is intentionally low so it never competes with crafting/combat as a primary income source. The skill XP is a small bonus that encourages varied skill development.

## Player Trading

Two players at the **same world map location** can trade items and gold directly. See [Trading & Social](trading.md) for full details.

### Trade Flow
1. Player A sends trade request to Player B (must be same location)
2. Player B accepts
3. Both set offers (items + gold)
4. Both accept the final offer
5. Items and gold swap atomically

## Central Exchange System

Order book at 5 world locations. Players can place buy/sell limit orders for items. Orders match automatically using price-time priority. See [Trading & Social](trading.md) for full details.

### Exchange Locations (5)
Placed at specific tiers across the map: ~T1.5, T3, T5, T7, T9.

## Guild System

Player organizations with ranks, a shared treasury, and guild halls. Two guild types: Combat and Crafting. See [Trading & Social](trading.md) for full details.

### Guild Hall Locations
~3% of towns have guild halls. Notable ones include Crossroads Inn, Thornwall Village, Port Blackwater, and Frosthold.

### Guild Features
- **Treasury:** Shared gold pool (members donate)
- **Membership:** Up to 50 members with ranks (Leader, Officer, Member, Recruit)
- **Types:** Combat (hunting, PvP, dungeons) or Crafting (gathering, crafting, trade)
