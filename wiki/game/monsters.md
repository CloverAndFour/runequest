# Monster System

## Overview

Module: `src/engine/monsters.rs`

Monsters are generated dynamically via `generate_monster(tier, enemy_type)` using simulator-validated stat curves. Two parameters determine everything:
- **Tier (0-10):** Stat scaling (HP, AC, attack bonus, damage)
- **Enemy type:** Brute, Skulker, Mystic, Undead -- name pool and type advantage interaction

## Base Stats per Tier

| Tier | HP Range | AC | To-Hit | Damage | Drop Tiers |
|---|---|---|---|---|---|
| T0 | 3-6 | 8 | +1 | 1d4 | T0 (60%) |
| T1 | 10-18 | 12 | +4 | 1d6+2 | T0-T1 |
| T2 | 18-30 | 13 | +5 | 1d6+3 | T0-T2 |
| T3 | 28-45 | 14 | +6 | 1d8+4 | T1-T3 |
| T4 | 40-60 | 15 | +7 | 1d8+5 | T2-T4 |
| T5 | 55-85 | 16 | +8 | 1d10+6 | T3-T5 |
| T6 | 75-117 | 17 | +9 | 1d10+7 | T4-T6 |
| T7 | 105-170 | 19 | +11 | 1d10+9 | T5-T7 |
| T8 | 150-250 | 20 | +12 | 1d12+11 | T6-T8 |
| T9 | 220-370 | 22 | +14 | 2d10+12 | T7-T9 |
| T10 | 320-550 | 24 | +15 | 2d12+14 | T8-T10 |

## Type Adjustments

| Type | HP Mult | AC Adj | Hit Adj | Dmg Adj | Profile |
|---|---|---|---|---|---|
| Brute | x1.20 | +1 | -1 | -1 | Tanky, lower damage |
| Skulker | x0.80 | +1 | +2 | +1 | Evasive, high burst |
| Mystic | x0.90 | -1 | +1 | 0 | Low AC, magic damage |
| Undead | x1.05 | 0 | 0 | 0 | Standard, type resistance |

## Named Monsters per Tier/Type

| Tier | Brute | Skulker | Mystic | Undead |
|---|---|---|---|---|
| T0 | Giant Rat | Cave Spider | Glow Wisp | Shambling Corpse |
| T1 | Kobold Thug | Giant Spider | Arcane Sprite | Skeleton |
| T2 | Goblin Warrior | Wolf | Fire Imp | Zombie |
| T3 | Orc Raider | Shadow Cat | Flame Elemental | Ghoul |
| T4 | Orc Warchief | Werewolf | Mind Flayer Spawn | Wraith |
| T5 | Hill Giant | Displacer Beast | Naga | Vampire Spawn |
| T6 | Stone Golem | Nightwalker | Elder Elemental | Death Knight |
| T7 | Fire Giant | Shadow Dragon | Beholder | Lich |
| T8 | Storm Giant | Void Stalker | Astral Devourer | Demilich |
| T9 | Titan Warrior | Dread Wraith Lord | Arch-Lich | Dracolich |
| T10 | Primordial Juggernaut | Primordial Lurker | Primordial Arcanum | Primordial Undying |

## Drop System

Module: `src/engine/drops.rs`

When combat ends in victory, `generate_drops()` is called BEFORE `combat.end()` clears the enemy list. This ensures the enemy data (tier, type) is still available for drop calculation. Drops are included in the `CombatEnded` message/response via the `drops: Vec<String>` field.

### Drop Chances

| Tier Difference (enemy tier - material tier) | Drop Chance |
|---|---|
| 0 (at-tier) | 60% |
| 1 (one tier below) | 30% |
| 2 (two tiers below) | 10% |
| 3+ | 0% |

Each dead enemy rolls independently against all matching materials in the crafting graph.

### Monster Drop Materials by Tier

**T0 Drops:**
- Brute: Rat Hide -- used by LW, SM
- Skulker: Spider Silk Strand -- used by TL, LW
- Mystic: Wisp Essence -- used by EN, JC, RC
- Undead: Bone Dust -- used by AL, RC

**T1 Drops:**
- Brute: Wolf Pelt -- used by LW
- Skulker: Venom Sac -- used by AL, WW
- Mystic: Mana Shard -- used by EN, AL
- Undead: Ectoplasm -- used by RC, AL

**(Pattern continues T2-T10 with increasingly rare and valuable drops)**

## Passive Combat Skill XP Awards

Players earn skill XP passively during combat:
- **On successful attack hit:** +5 XP to weapon skill
  - Ranged weapons (bows): Marksmanship
  - Finesse weapons (daggers, rapiers): Blade Finesse
  - All other weapons: Weapon Mastery
- **On taking damage and surviving:** +3 Fortitude XP

These passive awards are applied automatically by the combat engine, independent of any LLM tool calls.

## Organizational Scale by Tier

| Tier | Solo Feasibility | Party Size | Monster Examples |
|---|---|---|---|
| T0 | 1 (naked start) | 1 | Giant Rat, Cave Spider |
| T1-3 | 1 (solo) | 1-2 | Kobolds, Wolves, Orcs |
| T4-5 | 1-2 (challenging) | 3-5 | Werewolves, Giants |
| T6-7 | Party required | 5-15 | Golems, Shadow Dragons |
| T8 | Raid required | 20-40 | Storm Giants, Void Stalkers |
| T9 | Alliance required | 50-100+ | Titan Warriors, Dracolich |
| T10 | Server coalition | 200+ | Primordials |

## NPC Combat Stats

NPCs can also enter combat via `generate_npc_combat(tier, faction)`:
- Faction modifies effective tier: Guard +1, Civilian -1, others +0
- HP = 10 + effective_tier * 8, AC = 10 + effective_tier
- Attack weapon varies by faction (Guard = Sword, Criminal = Dagger, Civilian = Fists)

Guards attack murderers on sight. Guards defend non-murderer players being attacked.
