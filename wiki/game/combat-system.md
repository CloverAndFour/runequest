# Combat System

## Overview

RuneQuest uses turn-based combat with individual initiative and a **30-second decision timer**. All combat capabilities derive from **skill ranks** and **equipment** — there are no classes, no ability scores, and no spell slots. The same system handles PvE, PvP, and mixed encounters, supporting asymmetric sides and mid-combat joining.

Permanent death makes every combat a genuine risk decision. The flee system ensures players always have an escape option, though escape is never free.

---

## Core Principles

1. **Skills, not classes.** Your combat options come from which skills you've trained. A character with Weapon Mastery 5 and Healing 3 can tank AND heal — there's no class restriction.
2. **Equipment matters.** Skills provide linear scaling; equipment provides exponential scaling. At high tiers, gear is the primary power differentiator.
3. **You learn by doing.** There is no character level or XP bar. Each combat action grants XP to the skill used — swing a sword to improve Weapon Mastery, cast a heal to improve Healing. This is the only progression system.
4. **Complexity grows with you.** At T0, you have 5 base actions. By T5, skill-unlocked abilities give you 10-15 options. By T10, you may have 25+ abilities to manage.
5. **30-second turns.** Act or lose your turn. Prevents stalling and keeps combat flowing.
6. **Permadeath demands escape.** Flee is always available. Failing costs you, but the option exists.
7. **AI-agent playable.** The API returns all available actions with costs, targets, and cooldowns. An agent needs zero game knowledge to select valid actions.

---

## Derived Combat Stats

There are no stored ability scores (no STR, DEX, CON, INT, WIS, CHA). All combat stats are computed from skill ranks + equipment bonuses.

### Primary Stats

| Stat | Description | Key Skill Contributors | Equipment Role |
|------|-------------|----------------------|----------------|
| Max HP | Health before death | Fortitude, Primal Toughness, Iron Body | Major — armor tiers drive HP scaling |
| Defense | Difficulty to hit | Shield Wall, Divine Shield, Iron Body, Evasion | Major — armor provides base Defense |
| Attack | Hit accuracy | Weapon Mastery, Blade Finesse, Marksmanship, Martial Arts | Major — weapon to-hit bonuses |
| Damage | Base damage dealt | Attack skills + Rage, Evocation | Major — weapon damage dice |
| Speed | Initiative + flee bonus | Evasion, Tracking, Stealth | Minor |
| Stamina | Physical ability resource | Fortitude, Primal Toughness, Weapon Mastery, Martial Arts | Minor |
| Mana | Magical ability resource | Spell Mastery, Ki Focus, Healing, Evocation | Minor |

### Computation Model

```
stat = base_value + sum(skill_rank * skill_weight[stat]) + equipment_bonus[stat]
```

**Skill contributions are linear.** Equipment contributions scale exponentially with equipment tier, following the tier progression formulas from the balance simulator (`exp_scale(base, 1.36, 1.46, tier, 6)`).

**Design intent:** At T0-T2, skills and equipment contribute roughly equally. At T5+, equipment dominates. At T8+, equipment is 70%+ of your stats. This makes crafting and gear acquisition the primary progression system at endgame.

### Stat Formulas

Constants are tuning targets validated through the combat simulator.

**Max HP:**
```
max_hp = 10
       + (fortitude_rank * 4)
       + (primal_toughness_rank * 4)
       + (iron_body_rank * 3)
       + (lay_on_hands_rank * 1)
       + equipment_hp
```

Sample values (skill + equipment combined):

| Tier | Low Investment | Tank Build | Notes |
|------|---------------|------------|-------|
| T0 | 10 HP | 10 HP | No skills, no gear |
| T1 | 18 HP | 22 HP | Rank 1 skills, T1 gear |
| T3 | 30 HP | 42 HP | Rank 3 core, T3 gear |
| T5 | 48 HP | 68 HP | Rank 5 core, T5 gear |
| T8 | 90 HP | 140 HP | Rank 7-8 core, T8 gear |
| T10 | 140 HP | 220 HP | Rank 9-10 core, T10 gear |

**Defense:**
```
defense = 8
        + (shield_wall_rank * 1)
        + (divine_shield_rank * 1)
        + (iron_body_rank * 1)        // unarmored only
        + floor(evasion_rank / 2)
        + equipment_defense
```

**Attack:**
```
attack = relevant_attack_skill_rank + equipment_attack_bonus
```

Relevant attack skill depends on the action used:
- Melee weapons: Weapon Mastery or Blade Finesse
- Ranged weapons: Marksmanship
- Unarmed: Martial Arts
- Spells: Evocation, Eldritch Blast, Holy Smite

**Speed:**
```
speed = evasion_rank + floor(tracking_rank / 2) + floor(stealth_rank / 2) + equipment_speed
```

**Stamina:**
```
max_stamina = 20 + sum(physical_skill_ranks * 2)
stamina_recovery_per_round = 3 + floor(highest_physical_skill_rank / 2)
```

Physical skills: weapon_mastery, shield_wall, fortitude, rage, reckless_fury, primal_toughness, blade_finesse, evasion, marksmanship, martial_arts, iron_body, flurry, survival, tracking

**Mana:**
```
max_mana = 10 + sum(magical_skill_ranks * 3)
mana_recovery_per_round = 2 + floor(highest_magical_skill_rank / 2)
```

Magical skills: evocation, abjuration, spell_mastery, eldritch_blast, curse_weaving, soul_harvest, healing, blessing, turn_undead, inspire, lore, charm, song_of_rest, ki_focus

---

## Turn Structure

### Initiative

At combat start, each combatant rolls initiative:

```
initiative = d20 + speed
```

Combatants act in descending initiative order. Ties: higher Speed first; still tied: random.

### Turn Phases

Each combatant's turn:

1. **Start of turn:** Stamina/Mana recovery, buff/debuff duration tick, DoT damage applies
2. **Decision phase (30 seconds):** Choose 1 Main Action + optionally 1 Quick Action
3. **Resolution:** Actions execute, targets take damage/effects
4. **End of turn:** Cooldowns tick down by 1

If the 30-second timer expires, the turn is skipped. Combat log: *"[Name] hesitates!"*

### Action Types

| Type | Per Turn | Examples |
|------|----------|---------|
| Main Action | 1 | Attack, Defend, Flee, Use Item, most abilities |
| Quick Action | 1 (optional) | Minor buffs, weapon swap, some abilities |
| Reaction | Automatic | Triggers on specific conditions (e.g., counter-attack on miss) |
| Passive | Always on | Stat bonuses, conditional effects |
| Free | Unlimited | End turn |

---

## Base Actions

Available to ALL characters regardless of skill ranks. This is the T0 experience.

| Action | Type | Resource | Description |
|--------|------|----------|-------------|
| Attack | Main | Free | Basic weapon attack. Roll d20 + Attack vs target Defense. |
| Defend | Main | Free | +4 Defense until your next turn. |
| Flee | Main | Free | Attempt to escape combat (see Flee section). |
| Use Item | Main | Free | Use a consumable (potion, bomb, scroll). Item cooldowns apply. |
| Swap Weapon | Quick | Free | Switch equipped weapon. Changes your attack archetype. |
| End Turn | Free | Free | End your turn immediately. |

---

## Skill-Unlocked Abilities

### Framework

Each combat skill provides **passive bonuses** at every rank AND unlocks **active abilities** at rank thresholds:

| Rank | Tier Equivalent | Ability Tier |
|------|----------------|-------------|
| 1 (Novice) | ~T1 | Basic: simple, low cost |
| 3 (Journeyman) | ~T3 | Intermediate: tactical, moderate cost |
| 5 (Expert) | ~T5 | Advanced: powerful, significant cost |
| 7 (Grandmaster) | ~T7-8 | Elite: fight-changing, high cost |
| 10 (Transcendent) | ~T10 | Ultimate: legendary power, very high cost |

Not every skill unlocks an ability at every threshold. Defensive skills (Fortitude, Primal Toughness) are primarily passive. Offensive skills (Evocation, Weapon Mastery) are ability-heavy.

### Ability Structure

```
Ability:
  name: string
  skill_id: string            # Which skill grants this
  rank_required: u8           # Minimum rank to use
  action_type: Main | Quick | Reaction
  cost: { stamina: u32, mana: u32 }
  cooldown: u8                # Rounds (0 = no cooldown)
  archetype: Valor | Cunning | Arcana | Faith | None
  targets: Self | SingleEnemy | SingleAlly | AllEnemies | AllAllies
  effect: ...                 # Damage, healing, buff, debuff, etc.
```

---

## Full Ability Catalog

### Warrior Family

#### Weapon Mastery

*Passive: +1 Attack, +1 Damage per rank with melee weapons*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Heavy Strike | Main | 5 Stam | 0 | Melee attack with +2 damage |
| 3 | Cleave | Main | 10 Stam | 3 | Melee attack hitting up to 2 enemies |
| 5 | Sundering Blow | Main | 15 Stam | 4 | Attack; target -2 Defense for 3 rounds |
| 7 | Executioner | Main | 25 Stam | 6 | +50% damage vs targets below 25% HP |
| 10 | Titan Strike | Main | 40 Stam | 8 | +100% damage; on kill, +2 Attack rest of combat |

#### Shield Wall

*Passive: +1 Defense per rank when shield equipped*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Raise Shield | Quick | 3 Stam | 0 | +2 Defense until next turn |
| 3 | Shield Bash | Main | 8 Stam | 2 | Light damage + target loses Quick Action |
| 5 | Phalanx | Main | 15 Stam | 5 | +4 Defense for 3 rounds; allies +1 Defense |
| 7 | Bulwark | Quick | 20 Stam | 6 | Absorb next attack targeting an ally |
| 10 | Fortress | Quick | 35 Stam | 10 | All allies +3 Defense for 3 rounds |

#### Fortitude

*Passive: +4 Max HP per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 3 | Rally | Quick | 0 | 5 | Heal 10% of your Max HP |
| 5 | Resist | Passive | -- | -- | Immune to first debuff per combat |
| 7 | Unbreakable | Reaction | 0 | Once | Survive one killing blow at 1 HP (once per combat) |

### Berserker Family

#### Rage

*Passive: +1 Damage per rank, -0.5 Defense per rank (floor)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Enrage | Quick | 5 Stam | 0 | +2 Damage, -1 Defense for 3 rounds |
| 3 | Bloodlust | Passive | -- | -- | +1 Damage per enemy killed this combat |
| 5 | Berserk Fury | Main | 15 Stam | 5 | Attack at +4 Damage; take +2 damage for 2 rounds |
| 7 | Unstoppable | Quick | 30 Stam | 8 | +4 Damage, +2 Attack, -3 Defense for 5 rounds; immune to crowd control |

#### Reckless Fury

*Passive: +1 Attack per rank, -0.5 Defense per rank (floor)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Wild Swing | Main | 5 Stam | 0 | Attack at -2 Attack, +3 Damage |
| 3 | Reckless Strike | Main | 10 Stam | 3 | Attack at +4 Attack; next attack against you gets +4 |
| 5 | Whirlwind | Main | 20 Stam | 5 | Attack ALL enemies at -1 Attack |
| 7 | Death Blow | Main | 35 Stam | 8 | Attack at +8 Damage; on kill, take another Main Action |

#### Primal Toughness

*Passive: +4 Max HP per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 3 | Thick Skin | Passive | -- | -- | Reduce all damage taken by 1 |
| 5 | Regenerate | Passive | -- | -- | Heal 2 HP at start of each turn |
| 7 | Last Stand | Reaction | 0 | Once | Survive killing blow at 1 HP (once per combat) |

### Paladin Family

#### Holy Smite

*Passive: +1 Attack, +1 Damage per rank vs Undead*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Smite | Main | 5 Mana | 0 | Attack with +1d4 Faith damage (+2d4 vs Undead) |
| 3 | Radiant Strike | Main | 12 Mana | 3 | Attack with +2d4 Faith damage (+3d4 vs Undead) |
| 5 | Judgment | Main | 20 Mana | 5 | All enemies take 1d6+rank Faith damage |
| 7 | Divine Wrath | Main | 35 Mana | 8 | Attack +3d6 Faith; Undead below 30% HP destroyed |

#### Divine Shield

*Passive: +1 Defense per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Holy Guard | Quick | 5 Mana | 0 | +2 Defense until next turn |
| 3 | Shield of Faith | Main | 12 Mana | 4 | Target ally +3 Defense for 3 rounds |
| 5 | Consecrate | Main | 20 Mana | 5 | All allies +2 Defense for 3 rounds |
| 7 | Divine Barrier | Quick | 30 Mana | 8 | Target ally absorbs next 30 damage |

#### Lay on Hands

*Passive: +2 Max HP per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 3 | Healing Touch | Main | 10 Mana | 3 | Heal target for 2d6+rank HP |
| 5 | Cleansing Touch | Quick | 15 Mana | 4 | Remove all debuffs from target ally |
| 7 | Martyr's Gift | Main | All Mana | 10 | Heal target 50% Max HP; you take 25% Max HP damage |

### Rogue Family

#### Blade Finesse

*Passive: +1 Attack per rank with light/finesse weapons*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Precise Strike | Main | 5 Stam | 0 | Attack with +3 Attack |
| 3 | Riposte | Reaction | 8 Stam | 3 | Counter-attack when an enemy misses you |
| 5 | Vital Strike | Main | 15 Stam | 4 | +4 Damage, ignores 2 Defense |
| 7 | Assassination | Main | 30 Stam | 8 | Double damage from Hidden state |

#### Stealth

*Passive: +1 to Flee checks per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Hide | Main | 5 Stam | 0 | Become Hidden; next attack gets +4 Attack |
| 3 | Ambush | Passive | -- | -- | First attack in combat deals +50% damage |
| 5 | Vanish | Quick | 10 Stam | 5 | Become Hidden mid-combat |
| 7 | Shadow Step | Quick | 20 Stam | 6 | Become Hidden + choose any target |

#### Lockpicking

*Passive: Pick locks and disable traps (dungeon utility)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 3 | Sabotage | Main | 8 Stam | 4 | Target -2 Defense for 3 rounds |
| 5 | Trap Mastery | Passive | -- | -- | Auto-detect traps; can set traps in combat |
| 7 | Exploit Mechanism | Main | 20 Stam | 6 | Target -4 Defense for 3 rounds; all allies +2 Attack vs target |

#### Evasion

*Passive: +1 Speed per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Dodge Roll | Quick | 3 Stam | 0 | +2 Defense until next turn |
| 3 | Sidestep | Passive | -- | -- | 15% chance to avoid any attack |
| 5 | Uncanny Dodge | Reaction | 10 Stam | 3 | Halve damage from one attack |
| 7 | Evasion Mastery | Passive | -- | -- | AoE abilities deal half damage to you |

### Ranger Family

#### Marksmanship

*Passive: +1 Attack, +1 Damage per rank with ranged weapons*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Aimed Shot | Main | 5 Stam | 0 | Ranged attack at +2 Attack |
| 3 | Double Shot | Main | 12 Stam | 3 | Two ranged attacks at -1 Attack each |
| 5 | Piercing Shot | Main | 15 Stam | 4 | Ranged attack ignoring 3 Defense |
| 7 | Killing Shot | Main | 25 Stam | 6 | +50% damage vs targets below 30% HP |

#### Tracking

*Passive: +1 Speed per 2 ranks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Mark Prey | Quick | 3 Stam | 0 | +1 Attack vs target for 3 rounds |
| 3 | Hunter's Eye | Passive | -- | -- | +2 initiative permanently |
| 5 | Expose Weakness | Quick | 10 Stam | 4 | All allies +2 Attack vs target for 3 rounds |
| 7 | True Hunt | Quick | 15 Stam | 6 | Mark all enemies; all allies +1 Attack and +1 Damage |

#### Beast Lore

*Passive: +1 Damage vs Brute-type per 2 ranks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 3 | Animal Instinct | Passive | -- | -- | Cannot be surprised; always act in ambush rounds |
| 5 | Call Beast | Main | 20 Stam | 8 | Summon beast companion for 3 rounds |
| 7 | Alpha Predator | Passive | -- | -- | +3 Damage vs all enemy types |

#### Survival

*Passive: +1 to Flee checks per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Field Medicine | Main | 5 Stam | 4 | Heal self 1d6+rank HP |
| 5 | Trap Sense | Passive | -- | -- | Auto-detect traps in dungeon rooms |
| 7 | Wilderness Mastery | Passive | -- | -- | +2 Defense in dungeons; immune to environmental damage |

### Monk Family

#### Martial Arts

*Passive: +1 Attack, +1 Damage per rank with unarmed attacks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Focused Strike | Main | 5 Stam | 0 | Unarmed attack at +2 Attack |
| 3 | Stunning Fist | Main | 10 Stam | 4 | On hit, target loses Quick Action next turn |
| 5 | Palm Strike | Main | 15 Stam | 4 | +4 Damage, bypasses 2 Defense |
| 7 | Thousand Fists | Main | 30 Stam | 8 | Three unarmed attacks at full Attack |

#### Ki Focus

*Passive: +3 Max Mana per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Inner Peace | Quick | 3 Mana | 0 | +1 Defense until next turn |
| 3 | Ki Barrier | Quick | 10 Mana | 4 | Absorb next 8 + rank*2 damage |
| 5 | Ki Surge | Quick | 15 Mana | 5 | Recover 10 Stamina instantly |
| 7 | Enlightenment | Quick | 25 Mana | 8 | All cooldowns reduced by 2 rounds |

#### Iron Body

*Passive: +3 Max HP per rank; +1 Defense per 2 ranks (unarmored only)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 3 | Stone Skin | Quick | 8 Stam | 4 | Next 3 attacks deal -3 damage to you |
| 5 | Diamond Body | Passive | -- | -- | Immune to poison; +2 vs magical effects |
| 7 | Adamantine Body | Quick | 20 Stam | 8 | -50% all damage taken for 2 rounds |

#### Flurry

*Passive: +1 Damage per rank with unarmed (stacks with Martial Arts)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Flurry Strike | Main | 5 Stam | 0 | Two unarmed attacks at -2 Attack each |
| 3 | Rapid Blows | Main | 10 Stam | 3 | Two unarmed attacks at -1 Attack each |
| 5 | Storm of Fists | Main | 20 Stam | 5 | Three unarmed attacks at -1 Attack each |
| 7 | Infinite Flurry | Main | 30 Stam | 8 | Four unarmed attacks at full Attack |

### Mage Family

#### Evocation

*Passive: +1 spell Damage per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Fire Bolt | Main | 5 Mana | 0 | Ranged: 1d6+rank fire damage |
| 3 | Fireball | Main | 15 Mana | 4 | AoE: 2d6+rank fire to all enemies |
| 5 | Chain Lightning | Main | 20 Mana | 5 | Hit 3 targets for 1d8+rank each |
| 7 | Meteor | Main | 40 Mana | 8 | AoE: 4d6+rank to all enemies |
| 10 | Cataclysm | Main | 60 Mana | 12 | AoE: 6d8+rank*2 to all enemies; allies take half |

#### Abjuration

*Passive: +1 Defense per rank (magical defense)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Arcane Shield | Quick | 5 Mana | 0 | +2 Defense until next turn |
| 3 | Ward | Main | 10 Mana | 4 | Absorb next 10 + rank*2 damage |
| 5 | Dispel | Main | 15 Mana | 3 | Remove one buff from target enemy |
| 7 | Antimagic Field | Main | 30 Mana | 8 | No magical abilities usable for 2 rounds (all combatants) |

#### Spell Mastery

*Passive: +2 Max Mana per rank; all spell costs -1 per 3 ranks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 3 | Arcane Recovery | Quick | 0 | 6 | Recover 15 Mana |
| 5 | Quicken Spell | Passive | -- | -- | Once per combat: cast a Main spell as Quick instead |
| 7 | Mastery | Passive | -- | -- | All spell cooldowns reduced by 1 (min 0) |

### Warlock Family

#### Eldritch Blast

*Passive: +1 Attack, +1 Damage per rank with force attacks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Eldritch Bolt | Main | 5 Mana | 0 | Ranged: 1d8+rank force damage |
| 3 | Eldritch Barrage | Main | 12 Mana | 3 | Two bolts at -1 Attack each |
| 5 | Eldritch Nova | Main | 20 Mana | 5 | AoE: 2d8+rank force to all enemies |
| 7 | Void Beam | Main | 35 Mana | 8 | Single: 4d8+rank, ignores 4 Defense |

#### Curse Weaving

*Passive: Cursed targets take +1 damage from all sources per 3 ranks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Hex | Quick | 5 Mana | 0 | Target takes +1 damage from all sources, 3 rounds |
| 3 | Weakening Curse | Main | 12 Mana | 4 | Target -3 Attack for 3 rounds |
| 5 | Doom Mark | Main | 20 Mana | 5 | Target takes +3 damage from all sources, 3 rounds |
| 7 | Soul Curse | Main | 30 Mana | 8 | Target +5 damage from all sources; on death, all allies heal 10% Max HP |

#### Soul Harvest

*Passive: Heal 5% of damage dealt per 3 ranks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Drain Life | Main | 5 Mana | 0 | Deal 1d6 damage, heal self for half |
| 3 | Leech | Passive | -- | -- | Heal 15% of damage dealt to cursed targets |
| 5 | Soul Siphon | Main | 20 Mana | 5 | Deal 2d8 damage, heal for full amount |
| 7 | Reaper's Harvest | Passive | -- | -- | On kill, heal 20% of your Max HP |

### Cleric Family

#### Healing

*Passive: +1 to all healing done per rank*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Mend | Main | 8 Mana | 0 | Heal target 1d6+rank HP |
| 3 | Restore | Main | 15 Mana | 3 | Heal target 2d6+rank HP, remove one debuff |
| 5 | Healing Wave | Main | 25 Mana | 5 | Heal ALL allies 1d6+rank HP |
| 7 | Miracle | Main | 50 Mana | 10 | Heal target 50% of their Max HP |

#### Blessing

*Passive: Your buffs last +1 round per 3 ranks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Bless | Quick | 5 Mana | 0 | Target ally +1 Attack for 2 rounds |
| 3 | Protection | Main | 12 Mana | 4 | Target ally +2 Defense for 3 rounds |
| 5 | Mass Blessing | Main | 20 Mana | 5 | All allies +2 Attack, +1 Defense for 3 rounds |
| 7 | Divine Favor | Quick | 30 Mana | 8 | Target ally's next ability costs no resources and ignores cooldown |

#### Turn Undead

*Passive: +2 Damage per rank vs Undead*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Rebuke | Main | 8 Mana | 0 | 1d8+rank Faith damage to one Undead |
| 3 | Turn | Main | 15 Mana | 4 | All Undead: check or flee for 2 rounds |
| 5 | Sanctify | Main | 20 Mana | 5 | All Undead take 1d4 damage/round for 3 rounds |
| 7 | Banish | Main | 35 Mana | 8 | Destroy one Undead below 25% HP |

### Bard Family

#### Inspire

*Passive: +1 to all ally buff values you apply per 3 ranks*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Encourage | Quick | 5 Mana | 0 | Target ally +1 Attack for 2 rounds |
| 3 | Battle Hymn | Main | 12 Mana | 4 | All allies +2 Attack for 2 rounds |
| 5 | Rallying Cry | Main | 20 Mana | 5 | All allies +2 Attack, +1 Defense for 3 rounds |
| 7 | Legendary Inspiration | Quick | 30 Mana | 8 | Target ally's next ability costs no resources |

#### Lore

*Passive: Reveal enemy type and approximate HP on sight*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Identify | Quick | 3 Mana | 0 | Reveal target's type, exact HP, and Defense |
| 3 | Exploit Knowledge | Quick | 8 Mana | 3 | All allies gain type advantage vs target for 2 rounds |
| 5 | Monster Lore | Passive | -- | -- | +2 Damage vs all enemy types |
| 7 | True Sight | Quick | 15 Mana | 5 | Reveal all enemies' full stats; reveal hidden enemies |

#### Charm

*Passive: +1 to social/NPC interactions per rank (non-combat utility)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Taunt | Main | 5 Mana | 0 | Target enemy must attack you next round |
| 3 | Confuse | Main | 12 Mana | 4 | Target: 30% chance to attack its own allies next round |
| 5 | Mesmerize | Main | 20 Mana | 5 | Target loses next turn entirely |
| 7 | Dominate | Main | 35 Mana | 8 | Control target enemy for 2 rounds |

#### Song of Rest

*Passive: Party heals +2 HP per rank between combats (out of combat)*

| Rank | Ability | Type | Cost | CD | Effect |
|------|---------|------|------|----|--------|
| 1 | Soothing Melody | Main | 8 Mana | 0 | Heal target ally 1d4+rank HP |
| 3 | Healing Song | Main | 15 Mana | 4 | Heal ALL allies 1d4+rank HP |
| 5 | Restorative Chorus | Main | 25 Mana | 5 | Heal all allies 1d6+rank HP, remove 1 debuff each |
| 7 | Eternal Song | Main | 40 Mana | 8 | All allies regen 3 HP/round for rest of combat |

---

## Attack Resolution

### Basic Attack

```
attack_roll = d20 + attack_bonus
attack_bonus = relevant_skill_rank + equipment_attack_bonus

hit = (attack_roll >= target_defense)
```

**Critical Hit:** Natural 20 = always hits; double all damage dice (not flat modifiers).

**Critical Miss:** Natural 1 = always misses. No additional penalty.

### Damage Calculation

```
damage = weapon_damage_dice + damage_bonus
damage_bonus = relevant_skill_rank + equipment_damage_bonus + ability_bonus

minimum_damage = 1 (attacks always deal at least 1 damage on hit)
```

### Type Advantage System

Weapon/skill archetypes create tactical rock-paper-scissors matchups:

| Archetype | Strong vs | Weak vs | Associated Attack Skills |
|-----------|-----------|---------|--------------------------|
| Valor | Cunning | Arcana | Weapon Mastery, Shield Wall, Rage, Reckless Fury |
| Cunning | Arcana | Valor | Blade Finesse, Stealth, Marksmanship, Tracking |
| Arcana | Valor | Cunning | Evocation, Abjuration, Eldritch Blast, Curse Weaving |
| Faith | Undead (2x bonus) | None | Holy Smite, Healing, Turn Undead, Blessing |

**Advantage (strong vs target type):** +2 Attack, +1 damage per damage die

**Disadvantage (weak vs target type):** -2 Attack

Your active archetype = the attack skill used for each action. Multi-skilled characters switch archetypes by using different abilities. Enemies have a fixed archetype based on their type:

| Enemy Type | Archetype | Stat Profile |
|-----------|-----------|-------------|
| Brute | Valor | +20% HP, +1 Defense, -1 Attack |
| Skulker | Cunning | -20% HP, +2 Attack, +1 Speed |
| Mystic | Arcana | Normal HP, +1 Attack, has AoE abilities |
| Undead | Undead | +10% HP, immune to debuffs, weak to Faith |

---

## Status Effects

### Buffs

| Effect | Description | Stacking Rule |
|--------|-------------|---------------|
| Defense Up | +N Defense | Same source: refresh. Different: stack. |
| Attack Up | +N Attack | Same source: refresh. Different: stack. |
| Damage Up | +N Damage | Same source: refresh. Different: stack. |
| Regeneration | Heal N HP/round | Stack up to 3 sources |
| Hidden | Next attack at +4 Attack | Removed on attacking or taking damage |
| Shield | Absorb N damage | Stack up to 2 sources |
| Marked | Allies get bonuses vs target | Refresh only (highest value) |

### Debuffs

| Effect | Description | Stacking Rule |
|--------|-------------|---------------|
| Defense Down | -N Defense | Stack up to 3 sources |
| Attack Down | -N Attack | Stack up to 3 sources |
| Cursed | Take +N damage from all sources | Highest value only |
| Stun | Lose Quick Action next turn | No stack |
| Confused | 30% chance to attack ally | No stack |
| Mesmerized | Lose entire next turn | No stack |
| DoT | Take N damage per round | All sources stack |
| Fleeing | Must attempt Flee next turn | No stack |

### Duration Rules

All effects have a round duration. They tick down at the **end** of the affected combatant's turn. An effect with "3 rounds" persists through 3 of the target's turns.

---

## Skill XP from Combat

**There is no character level.** All progression comes from per-skill XP. Combat is the primary way to gain combat skill XP. Each action you take in combat awards XP to the relevant skill(s).

### XP Awards per Action

| Action | Skill(s) Receiving XP | Base XP |
|--------|-----------------------|---------|
| Basic Attack (melee) | Weapon Mastery or Blade Finesse or Martial Arts | 2 |
| Basic Attack (ranged) | Marksmanship | 2 |
| Defend | Shield Wall (if shield) or Evasion (if no shield) or Iron Body (if unarmored) | 1 |
| Flee (success) | Stealth or Survival (player's choice) | 2 |
| Flee (failure) | Evasion | 1 |
| Use skill ability | The ability's parent skill | 2-4 (scales with rank required) |
| Kill an enemy | Primary attack skill used for killing blow | 3 |
| Take damage and survive | Fortitude or Primal Toughness or Iron Body (highest ranked) | 1 |
| Heal an ally | Healing, Lay on Hands, or Song of Rest | 2 |
| Apply a buff | Blessing, Inspire, or Divine Shield | 2 |
| Apply a debuff | Curse Weaving, Charm, or Lockpicking (Sabotage) | 2 |

### XP Scaling

Base XP is modified by encounter tier relative to skill rank:

```
xp_multiplier = max(0.1, 1.0 + (enemy_tier - skill_rank) * 0.25)
actual_xp = floor(base_xp * xp_multiplier)
```

| Enemy Tier vs Skill Rank | Multiplier | Effect |
|--------------------------|-----------|--------|
| 3+ tiers above | 1.75x | Big reward for punching up |
| 2 tiers above | 1.50x | Good reward |
| 1 tier above | 1.25x | Normal |
| Equal | 1.00x | Baseline |
| 1 tier below | 0.75x | Diminishing |
| 2 tiers below | 0.50x | Grinding inefficient |
| 3+ tiers below | 0.25x | Barely any XP |
| 5+ tiers below | 0.10x | Floor — never zero |

**Design intent:** You learn fastest by fighting at or above your level. Farming T1 rats with a rank 8 sword skill teaches you nothing. This prevents power-leveling exploits and encourages players to push into harder content.

### XP on Flee

Fleeing from combat grants XP only for the flee action itself (Stealth or Survival). You do NOT receive XP for actions taken before fleeing — flee means you abandoned the fight. This makes fleeing a genuine trade-off: survive, but lose all skill XP progress from this encounter.

### XP on Death

Dead characters are permanently deleted. All accumulated XP is lost. This is the core tension of RuneQuest.

### PvP XP

PvP grants normal skill XP for all combat actions. Killing another player grants XP to the killing blow skill. No bonus or penalty for PvP vs PvE — combat is combat.

### Passive Skill XP

Some skills gain XP from non-action events during combat:

| Event | Skill | XP |
|-------|-------|----|
| Get hit while shield equipped | Shield Wall | 1 |
| Get hit while unarmored | Iron Body | 1 |
| Resist a debuff (via Fortitude passive) | Fortitude | 2 |
| Sidestep triggers (Evasion passive) | Evasion | 2 |
| Counter-attack triggers (Riposte) | Blade Finesse | 2 |

### Skill XP Thresholds (Reference)

From the existing skill system — unchanged:

| Rank | XP to Next | Rank Name |
|------|-----------|-----------|
| 0 | 50 | Untrained → Novice |
| 1 | 100 | Novice → Apprentice |
| 2 | 300 | Apprentice → Journeyman |
| 3 | 800 | Journeyman → Adept |
| 4 | 2,000 | Adept → Expert |
| 5 | 5,000 | Expert → Master |
| 6 | 12,000 | Master → Grandmaster |
| 7 | 30,000 | Grandmaster → Legendary |
| 8 | 70,000 | Legendary → Mythic |
| 9 | 150,000 | Mythic → Transcendent |

---

## Flee & Retreat

### Flee from Combat

```
flee_dc = 10 + (living_enemies * 2) - (prior_flee_attempts * 2)    // minimum 5
flee_roll = d20 + speed + stealth_rank
```

| Result | Outcome |
|--------|---------|
| Success (roll >= DC) | Exit combat. No skill XP (except for the flee itself), no loot. You escape alive. |
| Failure (roll < DC) | Lose your Main Action. One random enemy gets a free attack at -2 Attack against you. |

Rules:
- Flee costs a Main Action
- Each failed attempt reduces DC by 2 (cumulative -- gets easier)
- Party members flee **individually** -- you can flee while allies still fight
- **Cannot flee from boss rooms** unless the boss is dead
- Fleeing from PvP: same rules, uses enemy player count for DC

### Tactical Retreat (Between Combats)

Outside active combat in a dungeon:
- Move toward entrance through cleared rooms (safe traversal)
- Moving through uncleared rooms: 30% chance of triggering a new encounter
- Reaching the dungeon entrance = safe exit with all collected loot

### Town Portal Scroll

- 3-second channel (interruptible by damage)
- Cannot use in combat or boss rooms
- Teleports to nearest town
- One-time use, consumed on use
- Craftable: Enchanting Rank 3

---

## PvE Combat

### Enemy AI

Enemies use the same turn system as players. Behavior is type-based:

| Type | Targeting | Behavior |
|------|-----------|----------|
| Brute | Highest damage dealer | Aggressive: always attacks, uses power abilities |
| Skulker | Lowest HP target | Hit-and-run: hides, targets wounded, finishes kills |
| Mystic | Most buffed/dangerous | Support-caster: debuffs first, then AoE, then single target |
| Undead | Nearest target | Relentless: attacks nearest, immune to crowd control |

### Enemy Complexity by Tier

| Tier | Enemy Capability |
|------|-----------------|
| T0-T2 | Basic attacks only. Predictable. |
| T3-T4 | 1-2 special abilities (e.g., Brute Power Attack, Skulker Sneak) |
| T5-T6 | 2-3 abilities including debuffs and AoE |
| T7-T8 | 3-4 abilities including reactions; packs coordinate focus fire |
| T9-T10 | Full ability suites; tactical coordination; synergy abilities between enemy types |

### Boss Mechanics

**Enrage Timer (T5+):**

```
enrage_round = max(8, 25 - tier * 2)
```

| Tier | Enrage Round | Solo DPS Achievable? |
|------|-------------|---------------------|
| T5 | 15 | Barely |
| T7 | 11 | No -- need 3-4 DPS |
| T9 | 7 | No -- need 10+ DPS |
| T10 | 5 | No -- need 20+ DPS |

After enrage: boss deals 25% of its Max HP as AoE damage per round to ALL combatants.

**Multi-Phase Bosses (T6+):**
- Phase transitions at HP thresholds (75%, 50%, 25%)
- Each phase may change abilities, spawn adds, or alter the environment
- Phase transitions heal the boss for 10% of that phase's HP threshold

**Enemy Synergies (T6+):**
Mixed-type packs buff each other:
- Brute + Mystic: Mystic shields the Brute (+2 Defense)
- Skulker + Undead: Undead creates darkness, Skulker gains Hidden
- Synergies break when one type in the pair dies -- rewards focused-fire tactics

---

## PvP Combat

### Initiation

Any player can attack any other player at any time:

1. **Neither in combat:** New combat starts with both combatants
2. **Target already in combat:** Attacker joins existing combat (mid-combat join)
3. **Both already in same combat:** Just target the other player on your turn
4. PvP cannot be refused. This is an open-world permadeath game.

### Killer Flag

- Killing a player marks you as a **killer** for 30 minutes (visible to all)
- Killing a killer does **NOT** mark you -- bounty hunting is encouraged
- Guards attack killers on sight
- Flag is visible to all nearby players

### Asymmetric & Multi-Side Combat

Sides can be any size: 1v1, 1v3, 5v5, 2v2v1 (three-way). Combatants are tagged with a **side** (party/guild/solo). You can target anyone not on your side.

Three-way+ combat happens when multiple unallied groups enter the same fight.

### Mid-Combat Joining

When a new combatant enters active combat:

1. They roll initiative
2. Inserted into initiative order at correct position
3. If their initiative would have already passed this round: they act at end of current round
4. Combat log: *"[Name] joins the battle!"*
5. Their side is determined by party/guild membership, or by who they attack first

### Mixed PvE + PvP

If PvP starts in a room with enemies:
- Enemies are their own side -- they don't ally with either player group
- Enemies target based on their AI (threat/proximity) regardless of PvP
- Creates chaotic multi-sided combat
- Clever players can use enemies as shields or bait

### Full Loot PvP

Killer loots **ALL** items, equipment, and gold from the victim's corpse. This is the primary economic motivation for PvP (and the deterrent -- your own gear is at risk).

### PvE Combat Drops

On PvE victory, `generate_drops()` (`src/engine/drops.rs`) is called BEFORE `combat.end()` clears the enemy list. Each defeated enemy rolls independently for material drops:

| Tier Offset | Chance | Example (T2 enemy) |
|---|---|---|
| At-tier (0) | 60% | T2 materials |
| One below (-1) | 30% | T1 materials |
| Two below (-2) | 10% | T0 materials |
| Three+ below | 0% | -- |

Monster-specific T0 drops include: Rat Hide (Brute), Spider Silk Strand (Skulker), Wisp Essence (Mystic), Bone Dust (Undead). Higher-tier monsters drop increasingly valuable crafting materials.

All drops are added to the player's inventory and listed in the `combat_ended` message's `drops` field. This is the primary source of crafting materials that cannot be gathered from the environment.

---

## Party Combat

### Multi-Combatant Initiative

All players, allies, and enemies share one initiative order. The 30-second timer per turn prevents stalling in large groups.

Example initiative for a 3v3 fight:
```
Round 1: Player_A (18) > Enemy_2 (16) > Player_C (14) > Enemy_1 (12) > Player_B (10) > Enemy_3 (7)
```

### Targeting

On your turn, choose a target from valid options:
- **Offensive abilities:** Any enemy combatant (or any non-ally in PvP)
- **Healing/buff abilities:** Self or any ally
- **AoE offensive:** All enemies
- **AoE support:** All allies

### Emergent Party Roles

Roles emerge from skill investment, not class selection:

| Role | Key Skills | Combat Function |
|------|-----------|----------------|
| Tank | Fortitude, Shield Wall, Primal Toughness | High HP/Defense, uses Taunt to pull aggro |
| Healer | Healing, Blessing, Song of Rest, Lay on Hands | Keeps party alive |
| Melee DPS | Weapon Mastery, Rage, Blade Finesse, Martial Arts | Single-target damage |
| Ranged DPS | Marksmanship, Evocation, Eldritch Blast | Ranged/AoE damage |
| Support | Inspire, Lore, Charm, Curse Weaving | Buffs allies, debuffs enemies |
| Hybrid | Mixed skills | Versatile but less specialized |

No restrictions prevent any combination. A tank who heals, a caster who picks locks, a bard who tanks -- all mechanically valid.

---

## Tier Scaling Summary

| Tier | Typical Actions | Combat Feel | Resource Pressure |
|------|----------------|-------------|-------------------|
| T0 | 5-6 (base only) | Tutorial: attack or flee | None |
| T1-T2 | 8-10 | Simple: a few abilities | Light |
| T3-T4 | 12-15 | Tactical: cooldown management | Moderate |
| T5-T6 | 15-20 | Complex: party coordination, enrage | Heavy |
| T7-T8 | 20-25 | Demanding: resource management, rotations | Severe |
| T9-T10 | 25+ | Mastery: everything matters | Extreme |

**Why T0 has fewer actions:** Characters start with all skills at rank 0. No skill-unlocked abilities exist at rank 0. Players only have the 5 base actions (Attack, Defend, Flee, Use Item, End Turn) plus Swap Weapon. As they gain rank 1 in their first skills, basic abilities unlock.

---

## API Reference

### Get Combat State

**`GET /api/combat/state`**

Returns full combat state for the requesting player. AI agents poll this on their turn.

```json
{
  "combat_id": "uuid",
  "active": true,
  "round": 3,
  "your_turn": true,
  "time_remaining_ms": 28500,
  "your_state": {
    "combatant_id": "player_abc",
    "hp": 45, "max_hp": 60,
    "stamina": 15, "max_stamina": 40,
    "mana": 8, "max_mana": 20,
    "defense": 14, "attack": 8,
    "speed": 5,
    "active_effects": [
      {"name": "Enrage", "rounds_remaining": 2, "description": "+2 Damage, -1 Defense"}
    ],
    "cooldowns": {"cleave": 2, "rally": 4}
  },
  "combatants": [
    {
      "id": "player_abc", "name": "Thorin", "side": "party_1",
      "hp": 45, "max_hp": 60, "defense": 14,
      "active_effects": []
    },
    {
      "id": "enemy_0", "name": "Orc Brute", "side": "enemies",
      "hp": 22, "max_hp": 40, "type": "brute", "defense": 12,
      "active_effects": []
    }
  ],
  "available_actions": [
    {
      "id": "attack", "name": "Attack", "type": "main",
      "cost": null, "cooldown_remaining": 0,
      "valid_targets": ["enemy_0", "enemy_1"],
      "description": "Basic weapon attack"
    },
    {
      "id": "heavy_strike", "name": "Heavy Strike", "type": "main",
      "cost": {"stamina": 5}, "cooldown_remaining": 0,
      "valid_targets": ["enemy_0", "enemy_1"],
      "description": "Melee attack with +2 damage"
    },
    {
      "id": "raise_shield", "name": "Raise Shield", "type": "quick",
      "cost": {"stamina": 3}, "cooldown_remaining": 0,
      "valid_targets": ["self"],
      "description": "+2 Defense until next turn"
    },
    {
      "id": "flee", "name": "Flee", "type": "main",
      "cost": null, "flee_dc": 12,
      "valid_targets": null,
      "description": "Attempt to escape (DC 12)"
    },
    {
      "id": "use_item", "name": "Use Item", "type": "main",
      "cost": null,
      "usable_items": [
        {"item_id": "healing_potion_t3", "name": "Healing Potion", "effect": "Heal 14 HP", "cooldown_remaining": 0}
      ]
    },
    {
      "id": "end_turn", "name": "End Turn", "type": "free"
    }
  ],
  "combat_log": [
    "Round 3 begins.",
    "Orc Brute attacks Thorin (14 vs Defense 14): HIT for 6 damage!"
  ]
}
```

### Submit Action

**`POST /api/combat/action`**

```json
{
  "action_id": "heavy_strike",
  "target": "enemy_0"
}
```

For item use:
```json
{
  "action_id": "use_item",
  "item_id": "healing_potion_t3",
  "target": "self"
}
```

Response: Updated combat state (same format as GET).

### WebSocket Messages

| Direction | Type | Description |
|-----------|------|-------------|
| S->C | `combat_started` | Combat initiated, full initial state |
| S->C | `combat_state_update` | Full state after any action resolves |
| S->C | `your_turn` | It's your turn; includes available_actions |
| S->C | `turn_timeout` | Timer expired, turn skipped |
| S->C | `combatant_joined` | New combatant entered mid-combat |
| S->C | `combatant_died` | A combatant was killed (permanent death) |
| S->C | `combat_ended` | Combat over; includes skill XP gains, `drops: Vec<String>` (material items gained), deaths |
| S->C | `flee_result` | Flee attempt outcome |
| C->S | `combat_action` | Submit action (same as POST body) |

---

## Implementation Notes

### Core Structs (Rust)

```rust
pub struct Combat {
    pub id: Uuid,
    pub round: u32,
    pub initiative_order: Vec<CombatantId>,
    pub current_turn: usize,
    pub turn_deadline_ms: u64,          // 30-second timer
    pub combatants: HashMap<CombatantId, Combatant>,
    pub sides: HashMap<CombatantId, Side>,
    pub combat_log: Vec<String>,
    pub enrage_round: Option<u32>,
    pub enrage_active: bool,
}

pub struct Combatant {
    pub id: CombatantId,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub defense: i32,
    pub attack: i32,
    pub damage_bonus: i32,
    pub speed: i32,
    pub stamina: i32,
    pub max_stamina: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub stamina_recovery: i32,
    pub mana_recovery: i32,
    pub active_effects: Vec<StatusEffect>,
    pub cooldowns: HashMap<String, u8>,
    pub abilities: Vec<AbilityInstance>,
    pub archetype: Archetype,
    pub is_player: bool,
    pub enemy_type: Option<EnemyType>,
    pub flee_attempts: u32,
}

pub struct AbilityInstance {
    pub id: String,
    pub name: String,
    pub skill_id: String,
    pub rank_required: u8,
    pub action_type: ActionType,
    pub stamina_cost: i32,
    pub mana_cost: i32,
    pub cooldown: u8,
    pub archetype: Archetype,
    pub target_type: TargetType,
    pub effect: AbilityEffect,
}

pub enum ActionType { Main, Quick, Reaction, Passive, Free }
pub enum Archetype { Valor, Cunning, Arcana, Faith, None }
pub enum TargetType { SelfOnly, SingleEnemy, SingleAlly, AllEnemies, AllAllies }
pub enum Side { Party(Uuid), Enemies, Solo(String) }

pub struct StatusEffect {
    pub name: String,
    pub effect_type: EffectType,
    pub value: i32,
    pub rounds_remaining: u8,
    pub source: CombatantId,
}
```

### Migration Plan

Files to replace/rewrite:

| File | Change |
|------|--------|
| `src/engine/combat.rs` | Complete rewrite: new Combat struct, turn system, action resolution |
| `src/engine/abilities.rs` | Replace: remove spell slots/class abilities, add skill ability catalog |
| `src/engine/character.rs` | Remove `Stats` struct (6 ability scores); remove `level` and `xp` fields; add stamina/mana; keep `Class` as display label only |
| `src/engine/skills.rs` | Add ability unlock data per skill (rank thresholds -> abilities); add combat XP award functions |
| `src/engine/equipment.rs` | Update stat references: AC -> Defense, ability score bonuses -> direct stat bonuses |
| `src/engine/monsters.rs` | Update to new stat model (Defense instead of AC); add enemy abilities by tier |
| `src/engine/simulator.rs` | Update to match new combat system for balance validation |
| `src/web/api_server.rs` | Add new combat endpoints (GET state, POST action) |
| `src/web/websocket.rs` | Add new combat WS message types |
| `src/web/protocol.rs` | Add new message type definitions |
| `static/` | Frontend combat UI rewrite |

---

## Design Rationale

### Why no character level?

Character levels are a vestige of D&D and most traditional RPGs. In RuneQuest, they add nothing:
- **Power comes from skills + equipment**, not a level number. A "level 10" with bad skills and bad gear is weaker than a "level 5" with focused skills and good equipment.
- **XP should be granular and meaningful.** Swinging a sword improves your sword skill. Casting a heal improves your healing. A single XP bar that fills from "doing anything combat-related" gives no feedback about WHAT you're getting better at.
- **Tiers replace levels.** The tier system (T0-T10) describes content difficulty and equipment grade, not character power. A character's effective tier is emergent from their skill portfolio and gear.
- **No level gates.** Without character levels, there are no artificial "you must be level X to enter" gates. Instead, skill rank requirements and equipment tier requirements serve as organic gates.
- **Permadeath is less punishing.** Losing a "level 30 character" feels worse than losing "a character with rank 5 Weapon Mastery and rank 3 Healing." Skills are a portfolio of investments, not a single monolithic number.

### Why remove ability scores?

D&D ability scores add a second layer of indirection between the player and their power. In RuneQuest, skills ARE the progression system. Adding STR/DEX/CON on top creates confusion ("do I invest in STR or Weapon Mastery?") and a balancing nightmare. Deriving stats directly from skills + equipment gives one clear, understandable progression path.

### Why stamina AND mana?

Two resource pools create meaningful build diversity. A pure fighter runs on stamina, a pure caster on mana, and hybrids must manage both. With a single pool, there's no mechanical distinction between "a fighter who heals" and "a healer who fights." Dual pools make specialization matter while still allowing hybridization.

### Why 30-second timer?

Permadeath games create "analysis paralysis" -- players freeze when stakes are high. The timer forces decisive action, keeps multiplayer combat flowing, and prevents griefing (AFK in party combat). AI agents respond in milliseconds, so the timer is irrelevant for them.

### Why keep type advantage?

Rock-paper-scissors archetypes force party composition diversity and weapon switching. At T4+ dungeons, enemies appear in mixed-type packs specifically to punish single-archetype parties. This drives the coordination complexity that is the game's core design goal.

### Why both cooldowns AND resource costs?

Cooldowns alone let you spam cheap abilities. Costs alone let you dump everything in round 1. Together they create pacing: you manage WHEN to use abilities (cooldowns) and HOW MANY (resources). This produces "rotation" gameplay that distinguishes skilled players from button-mashers.

### Why does failed flee cause an opportunity attack?

Permadeath demands an escape option, but free escape would be abused. If flee had no downside, players would attempt it every turn below 50% HP. The opportunity attack creates a genuine risk calculation: "Can I survive one more hit if I fail?" This produces dramatic moments without making escape impossible. The escalating DC reduction on repeated attempts ensures no one is trapped forever.

### Why no dodge-as-disadvantage?

The old system used D&D's advantage/disadvantage (roll 2d20, take best/worst). The new system uses flat modifiers (+4 Defense for Defend action). Flat modifiers are more transparent, easier for AI agents to evaluate, and simpler to reason about. A player knows exactly how much safer Defend makes them, rather than estimating probabilistic effects.
