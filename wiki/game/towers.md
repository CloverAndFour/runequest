# Tower System

## Overview

10 towers placed at specific counties across the world map. Towers are **shared infinite dungeons** -- all players on the server share the same tower instance. Floors are deterministically generated from tower seed + floor number using ChaCha8 RNG.

Module: `src/engine/tower.rs`

## The 10 Towers

| Tower | ID | Base Tier | Seed | Recommended For |
|---|---|---|---|---|
| **Tower of Dawn** | tower_dawn | 2.0 | 1001 | New players (T2-T3) |
| **Ironspire** | ironspire | 3.5 | 1002 | Solo play (T3-T5) |
| **The Thornkeep** | thornkeep | 4.5 | 1003 | Small parties (T4-T6) |
| **Tidecaller Spire** | tidecaller | 5.0 | 1004 | Experienced parties (T5-T7) |
| **Shadowpillar** | shadowpillar | 5.5 | 1005 | Strong parties (T5-T7) |
| **The Nexus** | nexus | 6.0 | 1006 | Guild runs (T6-T8) |
| **Dragonwatch** | dragonwatch | 7.0 | 1007 | Raid content (T7-T9) |
| **Frostspire** | frostspire | 8.0 | 1008 | Endgame raids (T8-T10) |
| **The Abyss** | abyss | 8.5 | 1009 | Hardcore endgame (T8-T10+) |
| **Primordial Spire** | primordial_spire | 9.5 | 1010 | Server-first content (T9-T10+) |

## Floor Generation

### Tier Scaling
```
floor_tier = base_tier + floor_number * 0.2
```

Example (Tower of Dawn, base 2.0):
- Floor 0: Tier 2.0
- Floor 5: Tier 3.0
- Floor 10: Tier 4.0
- Floor 25: Tier 7.0
- Floor 40: Tier 10.0

Example (Primordial Spire, base 9.5):
- Floor 0: Tier 9.5
- Floor 3: Tier 10.1 (exceeds T10 -- true endgame)
- Floor 25: Tier 14.5 (far beyond normal content)

### Floor Size
```
width = min(8 + floor * 2, 50)
height = min(8 + floor * 2, 50)
```

- Floor 0: 8x8
- Floor 5: 18x18
- Floor 10: 28x28
- Floor 21+: 50x50 (capped)

### Special Floors

| Floor Pattern | Type | Description |
|---|---|---|
| Every 10th (10, 20, 30...) | **Safe Floor** | All rooms are Safe type. Rest, recover, prepare. |
| Every 5th ending in 4 (4, 9, 14, 19...) | **Boss Floor** | Center room contains a Boss enemy. |
| Floor 0 | **Entrance** | Room (0,0) is always Safe. |
| All floors | **Stairs** | Room (width-1, height-1) leads to next floor. |

### Room Types

| Type | Contents |
|---|---|
| Empty | Nothing |
| Combat | 1-3 enemies scaled to floor tier |
| Treasure | Gold + items (WIS check to find) |
| Trap | Detection DC (WIS), save DC, damage, conditions |
| Safe | Rest area, no enemies |
| Stairs | Leads to next floor |
| Boss | 1 boss enemy (on boss floors) |

## Shared Instances

**Critical feature:** Tower floors are shared across ALL players. When one player clears a combat room, it stays cleared for everyone. When one player opens a treasure chest, it is gone for everyone.

### Persistence
- Floor state saved as `data/towers/{tower_id}_{floor}.json`
- Shared across all players via `TowerStore`
- State includes: cleared rooms, opened chests, disarmed traps

### Floor Resets
Floors can reset when all players have left, allowing re-exploration. The deterministic generation means the same floor always has the same layout.

## PvP Inside Towers

Players can encounter each other inside towers. Since tower instances are shared, PvP challenges work normally inside towers. This makes high-tier towers dangerous not just from monsters but from other players.

## Tower Progression Strategy

1. **Early game (T1-T3):** Start with Tower of Dawn. First few floors are manageable solo.
2. **Mid game (T3-T5):** Move to Ironspire or The Thornkeep with a small party.
3. **Late game (T5-T7):** Tidecaller Spire through The Nexus require coordinated parties.
4. **Endgame (T7-T10):** Dragonwatch, Frostspire, The Abyss require raids.
5. **Beyond T10:** Primordial Spire floors quickly exceed T10, creating infinitely scaling challenge.

## LLM Tools

| Tool | Description |
|---|---|
| `enter_tower` | Enter a tower at the player's current location (must be at a tower county) |
| `tower_ascend` | Move to the next floor (must be at stairs room) |
| `exit_tower` | Leave the tower (returns to the world map county) |

The engine tracks each player's highest floor reached per tower.

## Finding Towers

Towers are placed at specific fixed counties during world generation. When a player is at a county with `has_tower: true`, the map view shows the tower name and the option to enter. Tower locations are visible on the hex map as distinct markers.
