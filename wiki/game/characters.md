# Character Creation & Races

## Overview

RuneQuest uses a background-based character creation system with **no classes, no levels, and no point-buy stats**. Characters are defined by their race, background, skills, and equipment.

## Two Creation Paths

### Background-Based (Recommended)
Player picks **race + background**. This is the current system:
- All stats default to 10
- Background determines starting skills (2 at rank 1), items, and gold
- HP = 8 + CON modifier
- No class, no levels

### Class-Based (Legacy)
Player picks **race + class + stats**. Kept for backwards compatibility:
- 27-point buy stats (8-15 range per stat)
- Class determines starting skills, abilities, equipment
- Class-specific HP dice
- Character levels L1-L10

The legacy path is triggered by providing a `class` field during creation. If `background` is provided instead, the new system is used.

## The 10 Races

| Race | Starting Region | Map Direction |
|---|---|---|
| **Human** | South-center edge | Default/neutral |
| **Elf** | Southwest edge | Ancient forest territory |
| **Dwarf** | Northwest edge | Mountain foothills |
| **Orc** | Northeast edge | Savage borderlands |
| **Halfling** | Southeast edge | Rolling hills/farmland |
| **Gnome** | West edge | Inventor workshops |
| **Dragonborn** | North-center edge | Volcanic highlands |
| **Faefolk** | East edge | Enchanted wilds |
| **Goblin** | South-far-east edge | Scrubland tunnels |
| **Revenant** | North-inner edge | Haunted borderlands |

Each race spawns in a T0 safe zone approximately 18 counties in radius. The race determines ONLY the starting position on the hex map. There are no racial stat bonuses in the current system (all stats start at 10).

### Cross-Race Distance

The continent is massive:
- Nearest race pairs: Halfling-Goblin (43 counties)
- Farthest race pairs: Elf-Orc (576 counties)
- Average distance to T5 territory: ~60 counties
- Average distance to T10 center: ~130-280 counties

## The 10 Backgrounds

| Background | Starting Skills (Rank 1) | Starting Weapon | Starting Gold |
|---|---|---|---|
| **Farmhand** | Fortitude, Leatherworking | Spear | 5 gp |
| **Apprentice Smith** | Smithing, Weapon Mastery | Mace | 5 gp |
| **Street Urchin** | Stealth, Lockpicking | Dagger | 5 gp |
| **Hunter** | Marksmanship, Tracking | Shortbow | 5 gp |
| **Acolyte** | Healing, Blessing | Quarterstaff | 5 gp |
| **Scholar** | Lore, Enchanting | Spellbook | 5 gp |
| **Merchant** | Charm, Inspire | (none) | 20 gp |
| **Herbalist** | Alchemy, Survival | (none) | 5 gp |
| **Woodcutter** | Woodworking, Fortitude | Handaxe | 5 gp |
| **Drifter** | (none) | (none) | 0 gp |

### Background Selection Strategy

- **Combat-focused start:** Apprentice Smith (weapon mastery + smithing), Hunter (ranged), Street Urchin (stealth)
- **Crafting-focused start:** Farmhand (leatherworking gateway), Apprentice Smith (smithing gateway), Woodcutter (woodworking gateway), Herbalist (alchemy gateway)
- **Social/utility start:** Merchant (gold advantage), Scholar (enchanting head start), Acolyte (healing)
- **Hard mode:** Drifter (nothing -- true naked start)

## Stats

Six ability scores: STR, DEX, CON, INT, WIS, CHA.

### Base Values
- Background path: All stats start at **10** (modifier +0)
- Legacy class path: 27-point buy, range 8-15

### Stat Modifiers
- `modifier = (stat - 10) / 2` (integer division, rounds down)
- Stat 8 = -1, Stat 10 = +0, Stat 12 = +1, Stat 14 = +2, Stat 15 = +2

### Stat Uses

| Stat | Used For |
|---|---|
| STR | Melee attack/damage, carry weight |
| DEX | Ranged/finesse attack/damage, AC, initiative, flee checks |
| CON | HP calculation, condition resistance |
| INT | Arcane weapon damage (staff, wand) |
| WIS | Trap detection, Healing Word heal amount, Monk AC |
| CHA | Social interactions, some legacy class features |

## HP Calculation

### Background Path
- Starting HP: 8 + CON modifier
- No HP growth from leveling (no levels exist)
- HP increases from: equipment bonuses, Fortitude skill effects

### Legacy Class Path
- Starting HP: class hit die + CON modifier
- Per level: class HP gain + CON modifier (min 1)
- Full heal on level up

| Class | Hit Die | HP/Level |
|---|---|---|
| Berserker | d12 | 7 + CON |
| Warrior, Paladin, Ranger | d10 | 6 + CON |
| Rogue, Monk, Warlock, Bard, Cleric | d8 | 5 + CON |
| Mage | d6 | 4 + CON |

## Starting AC

AC is calculated from equipped armor:
- **No armor:** 10 + DEX modifier
- **Light armor:** base AC + full DEX modifier
- **Medium armor:** base AC + min(DEX modifier, 2)
- **Heavy armor:** base AC only (no DEX)
- **Monk special:** 10 + DEX modifier + WIS modifier (no armor)

Most backgrounds start with no armor (AC 10), except those whose starting weapon implies a fighting style.

## Derived Class Label

The `derived_class_label()` function computes a display-only class name from the character's highest-ranked combat skill family. This is purely cosmetic -- used in presence display and party info.

If no combat skills are trained, the label is "Adventurer".

## Murderer System

Players can be flagged as murderers for killing innocents:
- **Fields:** `murderer` (bool), `kill_count` (u32)
- Killing a non-murderer player or innocent NPC sets `murderer = true` and increments `kill_count`
- Killing a murderer does NOT flag (bounty hunting is legitimate)
- Guards in towns attack murderers on sight
- Guards defend non-murderer players being attacked
- NPC guard combat stats scale with local county tier

## Death

All death is **permanent**:
- HP <= 0 = character dies, adventure over
- No respawns, no resurrection
- Applies to both PvE and PvP
- All equipped items and inventory are lost with the character

## API

- **REST:** `GET /api/backgrounds` -- lists all backgrounds with starting skills, items, gold
- **REST:** `POST /api/adventures` -- create adventure with `race` and `background` fields
- **WebSocket:** `create_adventure` -- same fields as REST
