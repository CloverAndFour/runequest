# Skill System

## Overview

All characters have access to all **44 skills** (34 combat + 10 crafting). Skills have ranks 0-10 and per-skill XP. There are no classes that restrict skill access -- any character can train any skill.

## Rank Names and XP Thresholds

| Rank | Name | XP to Next Rank | Cumulative XP |
|---|---|---|---|
| 0 | Untrained | 50 | 0 |
| 1 | Novice | 100 | 50 |
| 2 | Apprentice | 300 | 150 |
| 3 | Journeyman | 800 | 450 |
| 4 | Adept | 2,000 | 1,250 |
| 5 | Expert | 5,000 | 3,250 |
| 6 | Master | 12,000 | 8,250 |
| 7 | Grandmaster | 30,000 | 20,250 |
| 8 | Legendary | 70,000 | 50,250 |
| 9 | Mythic | 150,000 | 120,250 |
| 10 | Transcendent | -- (max) | 270,250 |

## XP Sources

### Combat XP (Passive)
- **On successful attack hit:** +5 XP to weapon skill
  - Ranged weapons: Marksmanship
  - Finesse weapons: Blade Finesse
  - All others: Weapon Mastery
- **On taking damage and surviving:** +3 Fortitude XP

### Crafting XP (Chance-Based)
- **At-tier recipe:** 15% chance to improve crafting skill rank
- **Above-tier recipe:** 25% chance to improve crafting skill rank
- **Below-tier recipe:** 0% chance (no improvement)

### Gathering XP
- **Gather action:** +5-10 Survival XP per gather

### LLM-Driven XP
- `award_skill_xp` tool: The LLM can award XP to any skill based on narrative actions

## The 34 Combat Skills

### Warrior Family
| Skill ID | Name | Description |
|---|---|---|
| weapon_mastery | Weapon Mastery | Proficiency with melee weapons. Increases hit chance and damage. |
| shield_wall | Shield Wall | Defensive shield techniques. Increases AC when shield equipped. |
| fortitude | Fortitude | Physical resilience. Increases max HP and condition resistance. Unlocks **Second Wind** at rank 1. |

### Berserker Family
| Skill ID | Name | Description |
|---|---|---|
| rage | Rage | Fury in battle. Increases rage damage bonus and duration. Unlocks **Reckless Attack** at rank 1. |
| reckless_fury | Reckless Fury | Wild offensive strikes. Increases damage at the cost of defense. |
| primal_toughness | Primal Toughness | Raw physical endurance. Increases max HP and damage resistance. |

### Paladin Family
| Skill ID | Name | Description |
|---|---|---|
| holy_smite | Holy Smite | Divine radiant strikes. Increases smite damage, especially vs undead. |
| divine_shield | Divine Shield | Holy protection. Increases AC and resistance to dark magic. |
| lay_on_hands | Lay on Hands | Divine healing touch. Increases healing pool size. Unlocks **Lay on Hands** ability at rank 1. |

### Rogue Family
| Skill ID | Name | Description |
|---|---|---|
| blade_finesse | Blade Finesse | Precision strikes with light weapons. Increases sneak attack damage. |
| stealth | Stealth | Moving unseen. Increases hide effectiveness and surprise attack chance. Unlocks **Hide** at rank 1. |
| lockpicking | Lockpicking | Opening locks and disabling traps. Reduces DCs for mechanical challenges. |
| evasion | Evasion | Dodging area effects. Chance to halve or negate AoE damage. |

### Ranger Family
| Skill ID | Name | Description |
|---|---|---|
| marksmanship | Marksmanship | Ranged weapon accuracy. Increases hit chance and damage with bows. |
| tracking | Tracking | Reading the wilderness. Bonus to initiative and detecting hidden enemies. |
| beast_lore | Beast Lore | Knowledge of creatures. Bonus damage vs beasts and natural enemies. |
| survival | Survival | Living off the land. Improved trap detection and environmental resistance. |

### Monk Family
| Skill ID | Name | Description |
|---|---|---|
| martial_arts | Martial Arts | Unarmed combat mastery. Increases unarmed damage dice and to-hit. |
| ki_focus | Ki Focus | Inner energy control. Increases ki points and special ability power. |
| iron_body | Iron Body | Body hardening. Increases unarmored AC and condition resistance. |
| flurry | Flurry | Rapid strikes. Improves Flurry of Blows damage and accuracy. Unlocks **Flurry of Blows** at rank 1. |

### Mage Family
| Skill ID | Name | Description |
|---|---|---|
| evocation | Evocation | Destructive spell power. Increases spell damage and AoE radius. |
| abjuration | Abjuration | Protective wards. Increases shield/ward absorption. |
| spell_mastery | Spell Mastery | Arcane efficiency. Bonus spell slots and reduced misfire. |

### Warlock Family
| Skill ID | Name | Description |
|---|---|---|
| eldritch_blast | Eldritch Blast | Eldritch force mastery. Increases blast damage and adds effects. |
| curse_weaving | Curse Weaving | Dark enchantments. Increases hex/curse duration and potency. |
| soul_harvest | Soul Harvest | Life draining power. Heal on kills, life drain attacks. |

### Cleric Family
| Skill ID | Name | Description |
|---|---|---|
| healing | Healing | Divine restoration. Increases healing spell power. Unlocks **Healing Word** at rank 1. |
| blessing | Blessing | Holy buffs. Increases buff duration and strength. |
| turn_undead | Turn Undead | Divine repulsion of undead. Increases damage and fear radius vs undead. |

### Bard Family
| Skill ID | Name | Description |
|---|---|---|
| inspire | Inspire | Motivating allies. Increases Bardic Inspiration bonus dice. |
| lore | Lore | Vast knowledge. Identify items, recall monster weaknesses, bonus to knowledge checks. |
| charm | Charm | Social manipulation. Improved NPC persuasion and enemy confusion. |
| song_of_rest | Song of Rest | Restorative melodies. Heals party during short rests. |

## The 10 Crafting Skills

| Skill ID | Name | Gateway Tier | Description |
|---|---|---|---|
| leatherworking | Leatherworking | T1 | Working hides and leather into materials and armor. |
| smithing | Smithing | T2 | Forging metals into weapons, armor, and tools. |
| woodworking | Woodworking | T3 | Crafting wood into bows, staves, and construction materials. |
| alchemy | Alchemy | T4 | Brewing potions, poisons, and alchemical reagents. |
| enchanting | Enchanting | T5 | Infusing items with magical properties. |
| tailoring | Tailoring | T6 | Weaving cloth and magical fabrics into garments. |
| jewelcrafting | Jewelcrafting | T7 | Cutting gems and crafting precious jewelry. |
| runecrafting | Runecrafting | T8 | Inscribing magical runes and glyphs. |
| artificing | Artificing | T9 | Constructing complex magical mechanisms. |
| theurgy | Theurgy | T10 | Divine crafting of holy and primordial items. |

## Skill to Bonus Action Mapping

These combat bonus actions unlock when the associated skill reaches rank 1:

| Skill | Unlocks | Effect |
|---|---|---|
| Fortitude (rank 1+) | Second Wind | Heal 1d10 + level HP. 1/rest. |
| Stealth (rank 1+) | Hide | Gain advantage on next attack. |
| Healing (rank 1+) | Healing Word | Heal 1d4 + WIS modifier HP. |
| Rage (rank 1+) | Reckless Attack | Advantage on attacks, enemies have advantage on you. |
| Lay on Hands (rank 1+) | Lay on Hands | Heal from divine pool (5 x level HP). |
| Flurry (rank 1+) | Flurry of Blows | Two bonus unarmed strikes. 1 ki/use. |

## Derived Class Label

The 10 skill families map to display-only class labels via `derived_class_label()`:

| Family Skills | Label |
|---|---|
| weapon_mastery, shield_wall, fortitude | Warrior |
| rage, reckless_fury, primal_toughness | Berserker |
| holy_smite, divine_shield, lay_on_hands | Paladin |
| blade_finesse, stealth, lockpicking, evasion | Rogue |
| marksmanship, tracking, beast_lore, survival | Ranger |
| martial_arts, ki_focus, iron_body, flurry | Monk |
| evocation, abjuration, spell_mastery | Mage |
| eldritch_blast, curse_weaving, soul_harvest | Warlock |
| healing, blessing, turn_undead | Cleric |
| inspire, lore, charm, song_of_rest | Bard |

The family with the highest total rank sum wins. Ties go to the first match. Zero total = "Adventurer".

## Proficiency Bonus from Skills

For background-path characters (no levels), proficiency is based on the character's highest skill rank:
- Rank 0-4: +2
- Rank 5-8: +3
- Rank 9-10: +4

For legacy class-path characters, proficiency is based on character level (L1-4 = +2, L5-8 = +3, L9-10 = +4).

## Background Starting Skills

Each background sets 2 skills to rank 1 (Novice) at character creation:

| Background | Skill 1 | Skill 2 |
|---|---|---|
| Farmhand | Fortitude | Leatherworking |
| Apprentice Smith | Smithing | Weapon Mastery |
| Street Urchin | Stealth | Lockpicking |
| Hunter | Marksmanship | Tracking |
| Acolyte | Healing | Blessing |
| Scholar | Lore | Enchanting |
| Merchant | Charm | Inspire |
| Herbalist | Alchemy | Survival |
| Woodcutter | Woodworking | Fortitude |
| Drifter | (none) | (none) |

## Legacy Class Starting Skills

When using class-based creation, 3-4 class-specific skills start at rank 1:

| Class | Starting Skills |
|---|---|
| Warrior | Weapon Mastery, Shield Wall, Fortitude |
| Berserker | Rage, Reckless Fury, Primal Toughness |
| Paladin | Holy Smite, Divine Shield, Lay on Hands |
| Rogue | Blade Finesse, Stealth, Lockpicking, Evasion |
| Ranger | Marksmanship, Tracking, Beast Lore, Survival |
| Monk | Martial Arts, Ki Focus, Iron Body, Flurry |
| Mage | Evocation, Abjuration, Spell Mastery |
| Warlock | Eldritch Blast, Curse Weaving, Soul Harvest |
| Cleric | Healing, Blessing, Turn Undead |
| Bard | Inspire, Lore, Charm, Song of Rest |

## UI Display

The Skills tab in the info panel shows all 44 skills split into:
- **Combat Skills (34):** All non-crafting skills
- **Crafting Skills (10):** The 10 crafting skills

Each skill row shows: name, rank name, numeric rank, and XP progress bar. Bar color: blue (rank 0-2), green (rank 3-5), gold (rank 6+).
