# World Map System

## Overview

The continent of Eldara is a hex grid of **251,431 counties**, generated deterministically from a fixed seed (42) at server start. The map is GLOBAL -- shared by all players on the server. Players store only their position and set of discovered counties.

## Geography

### Hex Grid
- Axial coordinate system: `(q, r)` where `q` = column, `r` = row
- Each county has 6 neighbors (East, West, NE, NW, SE, SW)
- Map radius: 289 hexes from center
- Total: ~251K counties

### Tier Gradient
- Edges of the continent: T0-T2 (safe starting areas)
- Middle ring: T3-T5 (frontier/wilderness)
- Inner ring: T6-T8 (dangerous territory)
- Center: T9-T10 (Primordial Wastes)
- **Max neighbor delta: 0.243** -- extremely smooth transitions
- No two adjacent counties differ by more than ~0.25 tiers

### Biomes
| Biome | Coverage | Description | Gather Materials |
|---|---|---|---|
| Plains | 35% | Open grassland, farmland | Plant Fiber, Wild Herbs, Crude Thread |
| Forest | 26% | Trees, undergrowth | Green Wood, Wild Herbs, Plant Fiber |
| Hills | 17% | Elevated terrain | Rough Stone, Scrap Metal, Muddy Clay |
| Coast | 13% | Shoreline areas | Plant Fiber, Rough Stone, Raw Quartz |
| Swamp | 3% | Wetlands, bogs | Muddy Clay, Wild Herbs, Plant Fiber |
| Volcanic | 2% | Near T9-10 center | Scrap Metal, Raw Quartz, Charcoal |
| Mountains | 2% | High elevation | Rough Stone, Scrap Metal, Raw Quartz |
| Desert | 1% | Arid regions | Rough Stone, Raw Quartz, Scrap Metal |
| Tundra | 1% | Frozen north | Rough Stone, Raw Quartz, Scrap Metal |

## 10 Race Spawns

Each race starts at the edge of the continent in a T0 safe zone (~18 counties safety radius).

| Race | Map Position | Direction | Nearest Other Race |
|---|---|---|---|
| Human | (0, -217) | South-center | Goblin (54 counties) |
| Elf | (-145, -145) | Southwest | Human (65 counties) |
| Halfling | (145, -145) | Southeast | Goblin (43 counties) |
| Gnome | (-193, 0) | West | Elf (65 counties) |
| Faefolk | (193, 0) | East | Orc (65 counties) |
| Dwarf | (-145, 145) | Northwest | Revenant (96 counties) |
| Orc | (145, 145) | Northeast | Dragonborn (65 counties) |
| Dragonborn | (0, 193) | North-center | Revenant (96 counties) |
| Goblin | (96, -193) | South-far-east | Halfling (43 counties) |
| Revenant | (-48, 145) | North-inner | Dragonborn (96 counties) |

**Maximum cross-continental distance:** Elf to Orc = **576 counties**.

## Features

### Towns (~10.5% of counties)
- Town tier determines shop inventory, NPC strength, available services
- Higher-tier towns have better shops and stronger guards
- Towns have crafting stations based on county tier (see [Crafting Stations](stations.md))

### Crafting Stations in Towns

Station distribution across the world:

| Station | Approximate Count |
|---|---|
| Tanning Rack | ~26,700 |
| Basic Forge | ~26,400 |
| Woodworking Bench | ~26,400 |
| Loom | ~26,400 |
| Herb Table | ~24,100 |
| Enchanting Altar | ~5,700 |
| Jeweler's Bench | ~5,500 |
| Master Forge | ~1,900 |
| Runic Circle | ~680 |
| Artificer's Workshop | ~71 |
| Sacred Altar | ~4-5 |
| Primordial Forge | 2 |

### Dungeons (~21% of counties)
- Generated on-demand from county seed via `generate_tiered_dungeon(seed, tier)`
- Dungeon tier: normally distributed around county tier +/- 1.5
- **Hidden tiers** -- players discover difficulty through exploration
- Private instances (per player/party)
- Floor count: T0-2 = 2 floors, T3-4 = 3 floors, T5+ = 4 floors
- Room types: Entrance, Combat, Trap, Treasure, Boss, Puzzle, Rest, Empty, Stairs

### Towers (10 fixed locations)
- Shared between all players (unlike dungeons)
- Infinite floors, growing in size and difficulty
- See [Tower System](towers.md) for details

### Exchanges (~5 locations)
- Placed at tier-appropriate locations across the map (T1.5, T3, T5, T7, T9)
- Central order book for player trading

### Guild Halls (~3% of towns)
- Required for guild creation and management
- Located at specific towns

## Travel

- Moving between adjacent counties = 1 travel action
- Six directions: East, West, Northeast, Northwest, Southeast, Southwest
- **Encounter chance:** `5% + (county_tier * 4%)` -- T0 = 5%, T5 = 25%, T10 = 45%
- **Fog of war:** Undiscovered counties show as "Unknown" / "???"
- Visiting a county reveals it and all its neighbors
- Travel is leader-controlled in party play

## Generation Algorithm

The world is generated from seed 42 at server startup:
1. Perlin noise for terrain/biome
2. Distance-based tier gradient (edges low, center high)
3. 50-pass smoothing for gradual transitions
4. Race spawn zones with cubic falloff safety radius
5. Feature placement (towns, dungeons, towers) based on tier probabilities
6. Crafting station assignment based on county tier and probability thresholds
7. Exchange and guild hall placement at fixed tier thresholds

## Map View (Frontend)

The `map_view` field is injected into every state response. It provides a 3-ring hex neighborhood (37 hexes) for the frontend hex grid:
- Current county details including crafting stations
- Six directional neighbors with tier/biome/feature info
- Discovery state per hex
- Undiscovered hexes show "???" with null features
