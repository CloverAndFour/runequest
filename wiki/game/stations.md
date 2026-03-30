# Crafting Stations

## Overview

Crafting requires being at a town with an appropriate station. Each station supports specific crafting skills and has a maximum recipe tier it can handle. Stations are assigned to towns during world generation based on county tier and probability thresholds.

## The 12 Station Types

| Station | Skills Supported | Max Tier | Placement Rule | Approx Count in World |
|---|---|---|---|---|
| **Tanning Rack** | Leatherworking | T3 | All towns (T0+) | ~26,700 |
| **Basic Forge** | Smithing | T3 | Towns T1+ | ~26,400 |
| **Woodworking Bench** | Woodworking | T3 | Towns T1+ | ~26,400 |
| **Loom** | Tailoring | T3 | Towns T1+ | ~26,400 |
| **Herb Table** | Alchemy | T4 | Towns T2+ | ~24,100 |
| **Enchanting Altar** | Enchanting | T5 | 30% chance at T3+ | ~5,700 |
| **Jeweler's Bench** | Jewelcrafting | T5 | 30% chance at T3+ | ~5,500 |
| **Master Forge** | SM, LW, WW, TL | T7 | 20% chance at T4+ | ~1,900 |
| **Runic Circle** | Runecrafting | T8 | 15% chance at T5+ | ~680 |
| **Artificer's Workshop** | Artificing | T9 | 10% chance at T7+ | ~71 |
| **Sacred Altar** | Theurgy | T10 | 5% chance at T9+ | ~4-5 |
| **Primordial Forge** | All 10 skills | T10 | 2 fixed near T10 center | 2 |

## Station Details

### Tanning Rack
- **Skills:** Leatherworking
- **Max Tier:** T3
- **Placement:** Every town in the world, regardless of tier
- **Purpose:** Entry-level crafting. All new players can find one immediately.
- **Recipes:** Leather strips, hardened leather, basic leather goods

### Basic Forge
- **Skills:** Smithing
- **Max Tier:** T3
- **Placement:** Every T1+ town
- **Purpose:** Metal working. Required for the T2 gateway (Smithing).
- **Recipes:** Iron nuggets, iron ingots, steel plates

### Woodworking Bench
- **Skills:** Woodworking
- **Max Tier:** T3
- **Placement:** Every T1+ town
- **Purpose:** Wood and fiber crafting. Required for the T3 gateway (Woodworking).
- **Recipes:** Shaped wood, ironwood planks, hardwood beams

### Loom
- **Skills:** Tailoring
- **Max Tier:** T3
- **Placement:** Every T1+ town
- **Purpose:** Cloth and fabric crafting. Required for early tailoring.
- **Recipes:** Woven cloth, enchanted thread

### Herb Table
- **Skills:** Alchemy
- **Max Tier:** T4
- **Placement:** Every T2+ town
- **Purpose:** Potion and reagent crafting. Required for the T4 gateway (Alchemy).
- **Recipes:** Refined potion bases, alchemical extracts

### Enchanting Altar
- **Skills:** Enchanting
- **Max Tier:** T5
- **Placement:** 30% of T3+ towns
- **Purpose:** Magical infusion. Required for the T5 gateway (Enchanting).
- **Rarity note:** First station with a probability gate. Players may need to search for one.

### Jeweler's Bench
- **Skills:** Jewelcrafting
- **Max Tier:** T5
- **Placement:** 30% of T3+ towns
- **Purpose:** Gem cutting and precious metalwork.

### Master Forge
- **Skills:** Smithing, Leatherworking, Woodworking, Tailoring
- **Max Tier:** T7
- **Placement:** 20% of T4+ towns (cities)
- **Purpose:** Advanced version of the four basic stations. Handles T4-T7 recipes for SM, LW, WW, TL.
- **Key feature:** One station covers 4 skills, making T4+ cities crafting hubs.

### Runic Circle
- **Skills:** Runecrafting
- **Max Tier:** T8
- **Placement:** 15% of T5+ towns (rare magical sites)
- **Purpose:** Rune inscription and glyph crafting. Required for the T8 gateway (Runecrafting).

### Artificer's Workshop
- **Skills:** Artificing
- **Max Tier:** T9
- **Placement:** 10% of T7+ towns (very rare)
- **Purpose:** Magical mechanism construction. Required for the T9 gateway (Artificing).
- **Rarity note:** Only ~71 in the entire world. Finding one requires deep exploration into high-tier territory.

### Sacred Altar
- **Skills:** Theurgy
- **Max Tier:** T10
- **Placement:** 5% of T9+ towns (extremely rare)
- **Purpose:** Divine crafting. Required for the T10 gateway (Theurgy).
- **Rarity note:** Only 4-5 in the entire world, all deep in T9-T10 territory.

### Primordial Forge
- **Skills:** All 10 crafting skills
- **Max Tier:** T10
- **Placement:** Exactly 2, placed at fixed locations near the T10 center of the map
- **Purpose:** The ultimate crafting station. Can craft any recipe from any skill at any tier.
- **Rarity note:** Only 2 in the entire world. Reaching them requires crossing the most dangerous territory on the continent.

## Station Validation

When a player attempts to craft, the engine validates:
1. The player is at a town (not in the wilderness, dungeon, or tower)
2. The town has a station supporting the recipe's crafting skill
3. The station's max tier is >= the recipe's tier

If validation fails, the craft is rejected with an appropriate error message.

## Progression Implications

- **T1-T3:** Basic stations are everywhere. Any starting town has what you need.
- **T4-T5:** Need Herb Table (T2+) and Enchanting Altar/Jeweler's Bench (30% at T3+). Some travel required.
- **T6-T7:** Master Forge cities (20% at T4+) become essential hubs. ~1,900 in the world.
- **T8:** Runic Circles (15% at T5+). ~680 total. Significant exploration needed.
- **T9:** Artificer's Workshops (10% at T7+). ~71 total. Deep wilderness expedition.
- **T10:** Sacred Altars (5% at T9+) or Primordial Forges (2 total). Endgame destination.

## Frontend Display

When a player is at a town with crafting stations, the `map_view.current.stations` array lists all available stations with their type, display name, max tier, and supported skills. The UI shows a "Craft" button in the fixed actions bar when stations are present.
