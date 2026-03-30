# Combat System

## Overview

RuneQuest uses a BG3-inspired turn-based combat system. Combat archetype is determined by the **equipped weapon**, not by any class. There are no player classes in the mechanical sense — only skill specializations that serve as flavor labels.

## Weapon-Based Archetypes

The combat archetype is determined by `weapon_archetype()` in `combat.rs`, which inspects the currently equipped weapon:

| Archetype | Weapon Types | Determines |
|---|---|---|
| **Valor** | Swords, axes, maces, hammers, spears, unarmed | Type advantage vs Cunning enemies |
| **Cunning** | Bows, daggers, shortswords, rapiers (ranged/finesse) | Type advantage vs Arcana enemies |
| **Arcana** | Staves, wands, spellbooks, tomes | Type advantage vs Valor enemies |
| **Divine** | (Reserved: holy items with Healing/Blessing skills) | +50% vs Undead, -10% vs others |
| **Utility** | (Reserved: instruments/social items) | Neutral (1.0x) vs everything |

**Key insight:** Swapping your weapon changes your combat role instantly. A character can be Valor with a sword and Arcana with a staff.

## The Type Triangle

```
        VALOR
       /     \
 1.20x/       \0.80x
     /         \
CUNNING ──1.20x──> ARCANA
```

### Damage Multiplier Table

| Player Archetype | vs Brute | vs Skulker | vs Mystic | vs Undead |
|---|---|---|---|---|
| Valor | 1.00 | **1.20** | 0.80 | 0.80 |
| Cunning | 0.80 | 1.00 | **1.20** | 0.80 |
| Arcana | **1.20** | 0.80 | 1.00 | 0.80 |
| Divine | 0.90 | 0.90 | 0.90 | **1.50** |
| Utility | 1.00 | 1.00 | 1.00 | 1.00 |

### Enemy Types

| Type | Aligned Archetype | Profile | Examples |
|---|---|---|---|
| **Brute** | Valor | High HP, high AC, moderate damage | Orcs, Trolls, Giants, Golems |
| **Skulker** | Cunning | High evasion, burst damage | Wolves, Spiders, Assassins |
| **Mystic** | Arcana | Magic damage, lower AC | Elementals, Naga, Beholders |
| **Undead** | Divine | Damage resistance, life drain | Skeletons, Wraiths, Vampires |

## Derived Class Label (Flavor Only)

The `derived_class_label()` function in `skills.rs` computes a display-only class name from the character's highest-ranked skill family. This appears in presence display and party info. It has **zero mechanical effect**.

| Skill Family | Label |
|---|---|
| Weapon Mastery, Shield Wall, Fortitude | Warrior |
| Rage, Reckless Fury, Primal Toughness | Berserker |
| Holy Smite, Divine Shield, Lay on Hands | Paladin |
| Blade Finesse, Stealth, Lockpicking, Evasion | Rogue |
| Marksmanship, Tracking, Beast Lore, Survival | Ranger |
| Martial Arts, Ki Focus, Iron Body, Flurry | Monk |
| Evocation, Abjuration, Spell Mastery | Mage |
| Eldritch Blast, Curse Weaving, Soul Harvest | Warlock |
| Healing, Blessing, Turn Undead | Cleric |
| Inspire, Lore, Charm, Song of Rest | Bard |

If no combat skills are trained, the label is "Adventurer".

## Proficiency Bonus

Proficiency scales from the character's **highest skill rank** (not from a "level"):

| Highest Skill Rank | Proficiency |
|---|---|
| 0-4 | +2 |
| 5-8 | +3 |
| 9-10 | +4 |

For legacy class-based characters, proficiency is based on character level: L1-4 = +2, L5-8 = +3, L9-10 = +4.

## Initiative

- **Player:** d20 + DEX modifier
- **Enemies:** flat d20
- Sorted descending. Ties broken by DEX, then random.

## Action Economy Per Turn

| Resource | Amount |
|---|---|
| Actions | 1 |
| Bonus Actions | 1 |
| Movement | 30 ft |
| Reactions | 1 |

## Standard Actions (All Characters)

| Action | Type | Effect |
|---|---|---|
| Attack | Action | d20 + stat_mod + proficiency + weapon_bonus vs AC. Weapon damage on hit. |
| Dodge | Action | Enemies attack with disadvantage until next turn. |
| Dash | Action | +30 movement this turn. |
| Use Item | Action | Consume first potion in inventory (Health Potion: 2d4+2 heal). |
| Flee | Action | d20 + DEX vs DC (10 + living_enemies x 2 - prior_attempts x 2, min 5). Success = escape, 0 XP. Failure = wasted action, DC drops by 2 next try. |
| End Turn | Free | Advance to next combatant. |

## Skill-Gated Bonus Actions

These bonus actions unlock when the character has the required skill at rank 1+:

| Bonus Action | Required Skill (Rank 1+) | Effect |
|---|---|---|
| Second Wind | Fortitude | Heal 1d10 + level HP. 1/rest. |
| Hide | Stealth | Gain advantage on next attack. |
| Healing Word | Healing | Heal 1d4 + WIS modifier HP. |
| Reckless Attack | Rage | Advantage on your attacks, but enemies have advantage on you. |
| Lay on Hands | Lay on Hands | Heal from divine pool (5 x level HP). |
| Flurry of Blows | Flurry | Two bonus unarmed strikes. 1 ki/use. |

## Damage Calculation

```
Attack roll: d20 + stat_modifier + proficiency_bonus + weapon_attack_bonus + equipment_bonuses
  vs target AC

If hit:
  Raw damage = weapon_damage_dice + stat_modifier
  Final damage = max(1, round(raw_damage * type_multiplier))

Critical hit (natural 20): double damage dice
Auto-miss (natural 1): always misses regardless of modifiers
```

### Damage Stat by Weapon

| Weapon Type | Damage Stat |
|---|---|
| Melee (sword, axe, mace, etc.) | STR |
| Ranged (bow) | DEX |
| Finesse (dagger) | DEX |
| Arcane (staff, wand) | INT |

### Unarmed Combat (T0)

- Damage: 1 + STR modifier (no dice roll, fixed)
- AC: 10 + DEX modifier (unarmored)
- Monk exception: AC = 10 + DEX modifier + WIS modifier

## Enemy AI

Each enemy picks its highest to_hit attack, rolls d20 + to_hit vs player AC. Simple but effective.

## Death

HP <= 0 = **permanent death**. The adventure is over. No respawns, no resurrection. This applies to both PvE and PvP.

## Victory Rewards

When all enemies reach 0 HP:
- **XP:** 50 XP per enemy defeated (split in party combat)
- **Material drops:** Generated based on enemy type and tier (see Monster System)
- Level-up check performed (legacy characters only)

## Passive Skill XP During Combat

Players earn skill XP passively during combat:
- **On successful attack hit:** +5 XP to the relevant weapon skill
  - Ranged weapons: Marksmanship
  - Finesse weapons: Blade Finesse
  - All others: Weapon Mastery
- **On taking damage and surviving:** +3 Fortitude XP

## Conditions

| Condition | Effect |
|---|---|
| Poisoned | Disadvantage on attacks/checks, 1d4 poison damage/turn |
| Burning / On Fire | 1d6 fire damage/turn |
| Bleeding | 1d4 damage/turn |
| Blinded | Disadvantage on attacks |
| Frightened | Disadvantage on checks/attacks |
| Stunned | Cannot act, fails STR/DEX saves |
| Paralyzed | Cannot act, auto-fail saves, melee attacks auto-crit |
| Exhaustion | Disadvantage on checks, speed halved |

## Party Combat (Timer-Based)

When in a party (up to 4 members), combat uses a simultaneous decision system:

1. Combat triggers (room enemies, travel encounter)
2. Each party member rolls d20 + DEX for initiative
3. **Player Decision Phase (30 seconds):** All members choose actions simultaneously
4. **Resolution Phase:** Actions resolve in initiative order
5. **Enemy Phase:** Each enemy attacks a random living party member
6. Repeat until all enemies dead or all players incapacitated

**Party death rules:** HP <= 0 = incapacitated (can be revived mid-combat). If still 0 HP when combat ends = permanent death. All members at 0 HP = TPK, everyone dies.

**Party XP:** 50 per enemy, split equally among living members.

## PvP Combat

1v1 duels between players at the same location. Uses individual initiative (not timer-based). Death is **permanent**. Killing another player gives a 30-minute criminal flag. Criminals can be attacked without consent.
