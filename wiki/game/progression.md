# Progression System

## Core Design: No Levels, No Point-Buy

RuneQuest does **not** have player levels, experience levels, or point-buy stat allocation. Progression comes from three sources:

1. **Skills** — Per-skill XP, 44 independent skills (34 combat + 10 crafting)
2. **Equipment** — Crafted gear across 10 tiers
3. **Organizational scale** — Higher tiers require more players cooperating

## Character Creation

### Races (10)

| Race | Starting Region | Map Position |
|---|---|---|
| Human | South-center | (0, -217) approx |
| Elf | Southwest | (-145, -145) approx |
| Dwarf | Northwest | (-145, 145) approx |
| Orc | Northeast | (145, 145) approx |
| Halfling | Southeast | (145, -145) approx |
| Gnome | West | (-193, 0) approx |
| Dragonborn | North-center | (0, 193) approx |
| Faefolk | East | (193, 0) approx |
| Goblin | South-far-east | (96, -193) approx |
| Revenant | North-inner | (-48, 145) approx |

Each race spawns in a T0 safe zone at the edge of the continent.

### Backgrounds (10)

| Background | Starting Skills (Rank 1) | Starting Items | Gold |
|---|---|---|---|
| Farmhand | Fortitude, Leatherworking | Spear | 5 gp |
| Apprentice Smith | Smithing, Weapon Mastery | Mace | 5 gp |
| Street Urchin | Stealth, Lockpicking | Dagger | 5 gp |
| Hunter | Marksmanship, Tracking | Shortbow | 5 gp |
| Acolyte | Healing, Blessing | Quarterstaff | 5 gp |
| Scholar | Lore, Enchanting | Spellbook | 5 gp |
| Merchant | Charm, Inspire | (none) | 20 gp |
| Herbalist | Alchemy, Survival | (none) | 5 gp |
| Woodcutter | Woodworking, Fortitude | Handaxe | 5 gp |
| Drifter | (none) | (none) | 0 gp |

### Stats

All stats default to **10** (base). Stats are modified by:
- **Race bonuses** (if any)
- **Equipment bonuses** (from worn gear)
- **Skill effects** (certain skills at high ranks)

There is **no point-buy** for the background creation path. Legacy class-based characters use 27-point buy (8-15 range per stat).

### HP Formula

- **Background path:** 8 + CON modifier (all backgrounds)
- **Legacy class path:** Class hit die + CON modifier

## Tier System (T0-T10)

Everything in the game scales with tiers: equipment, monsters, crafting materials, world areas.

| Tier | Scale Name | Players Required | Real-World Analogy |
|---|---|---|---|
| 0 | **Raw** | 1 (naked start) | Learning to walk |
| 1 | **Novice** | 1 (solo) | Apprentice |
| 2 | **Journeyman** | 1-2 | Professional |
| 3 | **Veteran** | 2-3 | Small workshop |
| 4 | **Elite** | 3-5 (party) | Small business |
| 5 | **Vanguard** | 5-10 | Specialized team |
| 6 | **Exalted** | 10-20 (raid) | Company |
| 7 | **Mythic** | 20-40 (guild) | Corporation |
| 8 | **Legendary** | 40-80 (large guild) | Government agency |
| 9 | **Eternal** | 80-200 (alliance) | National program |
| 10 | **Primordial** | 200+ (coalition) | Manhattan Project |

## Per-Skill XP

Each of the 44 skills has its own independent XP counter. XP is earned through:
- **Combat:** Passive weapon skill XP on hit (+5), Fortitude XP on taking damage (+3)
- **Crafting:** Chance-based improvement on recipe execution (15% at-tier, 25% above-tier)
- **Gathering:** +5-10 Survival XP per gather action
- **Engine tools:** `award_skill_xp` (LLM-driven rewards)

### XP Thresholds Per Rank

| From Rank | To Rank | XP Required |
|---|---|---|
| 0 (Untrained) | 1 (Novice) | 50 |
| 1 (Novice) | 2 (Apprentice) | 100 |
| 2 (Apprentice) | 3 (Journeyman) | 300 |
| 3 (Journeyman) | 4 (Adept) | 800 |
| 4 (Adept) | 5 (Expert) | 2,000 |
| 5 (Expert) | 6 (Master) | 5,000 |
| 6 (Master) | 7 (Grandmaster) | 12,000 |
| 7 (Grandmaster) | 8 (Legendary) | 30,000 |
| 8 (Legendary) | 9 (Mythic) | 70,000 |
| 9 (Mythic) | 10 (Transcendent) | 150,000 |

**Total XP to reach Transcendent (rank 10):** 270,250 XP per skill.

## Death is Permanent

All death is permanent:
- HP <= 0 in PvE combat = character dies, adventure over
- HP <= 0 in PvP combat = character dies, adventure over
- No respawns, no resurrection mechanics
- All equipped items and inventory are lost

This makes progression meaningful and creates real stakes at every tier.

## Legacy Character System

For backwards compatibility, the game supports class-based character creation (Warrior, Mage, Rogue, Cleric, Ranger, Berserker, Paladin, Monk, Warlock, Bard) with:
- 27-point buy stats
- Character levels (L1-L10)
- Class-specific HP dice and abilities
- XP thresholds: L2=300, L3=900, L4=2700, L5=6500, L6=14000, L7=23000, L8=34000, L9=48000, L10=64000

New characters should use the background system.

## Design Principles

1. **Organizational scale IS the progression.** At T9-10 the question is not "how strong is your character?" but "how large and coordinated is your alliance?"
2. **Coordination over grinding.** Progression requires diverse skill cooperation, not repetitive killing.
3. **Exponential growth.** Each tier is ~7x harder than the previous.
4. **Death is permanent.** Real stakes at every level.
5. **No artificial gates.** Any character can train any skill. Specialization is a choice, not a lock.
