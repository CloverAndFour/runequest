# Crafting System

## Overview

10 crafting skills form a **gateway staircase** — each skill is the exclusive gateway to one tier. This forces cross-skill cooperation that scales exponentially with tier.

**Graph statistics:** 336 materials, 682 recipes (82 intermediate + 600 equipment).

## The 10 Crafting Skills

| Skill | Abbrev | Gateway Tier | Domain |
|---|---|---|---|
| **Leatherworking** | LW | T1 | Hides, leather, sinew |
| **Smithing** | SM | T2 | Metals, ores, alloys |
| **Woodworking** | WW | T3 | Lumber, natural fibers |
| **Alchemy** | AL | T4 | Herbs, reagents, potions |
| **Enchanting** | EN | T5 | Magical essences, mana |
| **Tailoring** | TL | T6 | Cloth, silk, magical fabrics |
| **Jewelcrafting** | JC | T7 | Gems, precious metals |
| **Runecrafting** | RC | T8 | Runes, glyphs, sigils |
| **Artificing** | AF | T9 | Mechanisms, constructs |
| **Theurgy** | TH | T10 | Divine/primordial creation |

All characters start with all 10 crafting skills at rank 0 (Untrained).

### Skill Improvement

Skills improve through crafting at-level or above-level recipes:
- **15% chance** per craft at current rank tier
- **25% chance** per craft above current rank tier
- **0% chance** from crafting below your rank

## The Gateway Staircase Rule

**For each tier N, the gateway skill can produce T(N) output using ONLY T(N-1) inputs. All other skills need at least one T(N) material (from the gateway) as input.**

This creates a mandatory progression chain:

| Tier | Gateway Skill | Can Reach From | Skills Required |
|---|---|---|---|
| T1 | Leatherworking | T0 raw materials | 1 |
| T2 | Smithing | T1 materials | 2-3 |
| T3 | Woodworking | T2 materials | 3-5 |
| T4 | Alchemy | T3 materials | 5-6 |
| T5 | Enchanting | T4 materials | 6-7 |
| T6 | Tailoring | T5 materials | 7-8 |
| T7 | Jewelcrafting | T6 materials | 8 |
| T8 | Runecrafting | T7 materials | 8 |
| T9 | Artificing | T8 materials | 9 |
| T10 | Theurgy | T9 materials | 10 |

Example chain:
- T1: Leatherworking makes Leather Strip from T0 raw materials
- T2: Smithing makes Iron Ingot — needs Leather Strip (from LW) as input
- T3: Woodworking makes Hardwood Beam — needs Iron Ingot (from SM) and other T2 products

## Material Sources

### Gathered (T0 only, 14 raw materials)
Collected via the `gather` tool. Yield: 1-3 materials per gather, +5-10 Survival XP.

**Biome pools:**
| Biome | Materials |
|---|---|
| Plains | Plant Fiber, Wild Herbs, Crude Thread |
| Forest | Green Wood, Wild Herbs, Plant Fiber |
| Hills | Rough Stone, Scrap Metal, Muddy Clay |
| Mountains | Rough Stone, Scrap Metal, Raw Quartz |
| Swamp | Muddy Clay, Wild Herbs, Plant Fiber |
| Coast | Plant Fiber, Rough Stone, Raw Quartz |
| Desert | Rough Stone, Raw Quartz, Scrap Metal |
| Tundra | Rough Stone, Raw Quartz, Scrap Metal |
| Volcanic | Scrap Metal, Raw Quartz, Charcoal |

### Monster Drops (T0-T10)
Dropped by defeated enemies. 4 drop types per tier (one per enemy type: Brute, Skulker, Mystic, Undead). See [Monster System](monsters.md) for drop tables.

### Crafted (T1-T10)
Produced by recipes. 8 crafted materials per tier per skill, plus intermediate products.

## Recipe Structure

Each recipe requires:
1. A **crafting skill** at a minimum rank
2. **Input materials** (consumed on craft)
3. A **crafting station** at the player's current town that supports the skill and tier
4. Produces an **output** material or equipment item

### Equipment Recipes (600 total)

10 equipment lines x 10 tiers x 6 slots (1 weapon + 5 armor) = 600 equipment recipes. Each produces a weapon or armor slot piece. Equipment IDs now use slot names: `{line}_{slot}_t{N}` (e.g., `blade_chest_t5`, `bow_head_t3`). See [Equipment Lines](equipment.md) for full details.

### Intermediate Recipes (82 total)

82 recipes that produce intermediate crafting materials used as inputs for equipment and higher-tier recipes.

## Crafting Stations

Crafting requires being at a town with an appropriate station. See [Crafting Stations](stations.md) for the full list of 12 station types, their tier caps, and world placement.

**Validation:** The engine checks three things before allowing a craft:
1. Player has the required skill rank
2. Player is at a town with a station supporting the recipe's skill and tier
3. Player has all required input materials

## Cross-Dependency Mixing

**Design principle:** Maximize cross-pollination. No skill is self-sufficient at T2+.

Every recipe at T2+ pulls inputs from 2-3 different crafting skills. Every monster type's drops feed 5-6 different crafting skills.

| Monster Type | Drops Feed Skills |
|---|---|
| Brute | SM, LW, WW, RC, TL, JC (6 skills) |
| Skulker | WW, LW, RC, AL, JC, TL (6 skills) |
| Mystic | EN, RC, WW, JC, AL (5 skills) |
| Undead | JC, TL, SM, RC, EN, AL (6 skills) |

## Combat-Craft Interdependency

Each combat specialization needs products from specific crafting skills:

| Specialization | Needs Products From |
|---|---|
| Warrior/Berserker | Blade line (SM+LW+EN) or Axe line (SM+LW+WW) |
| Paladin/Cleric | Holy line (SM+RC+TL) or Scepter line (SM+RC+TL) |
| Rogue | Dagger line (LW+AL+JC) |
| Ranger | Bow line (WW+LW+AL) |
| Monk | Fist line (TL+AL+EN) |
| Mage | Staff line (WW+EN+RC) |
| Warlock | Wand line (RC+TL+JC) |
| Bard | Song line (WW+TL+JC) |

## Recipe Database

Full listing available via CLI and API:
```bash
# CLI
cargo run -- crafting --analyze      # Full balance report
cargo run -- crafting --equipment    # Equipment cost analysis
cargo run -- crafting --recipe <id>  # Single recipe lookup
cargo run -- crafting --tier <N>     # All recipes at a tier
cargo run -- crafting --mixing       # Mixing score report

# API
GET /api/recipes
GET /api/recipes?skill=smithing&tier=2
GET /api/recipes/:recipe_id
GET /api/materials
```
