# Potions, Consumables & Utility Items

## Overview

Consumables are single-use (or limited-use) items that are destroyed on use. They form the **logistics backbone** of RuneQuest -- the expendable fuel that powers dungeon runs, raid operations, and PvP engagements. With permanent death, every potion drunk and every scroll burned represents a real economic cost, creating continuous demand for crafters and supply chains.

**Design Principles:**

1. **Consumables are the crafter's recurring revenue.** Equipment is crafted once and lasts until death. Consumables are destroyed every use. This makes dedicated crafters essential support roles -- not just one-time equipment vendors, but ongoing suppliers.
2. **Organizational scaling through logistics.** T0-T2 dungeons need a handful of potions. T8+ raids need *hundreds* of consumables per attempt. This logistics challenge IS the coordination bottleneck at high tiers, not just combat skill.
3. **Gateway staircase applies.** Consumable recipes follow the same tier gateway rules as equipment. A T5 elixir needs T5 Enchanting gateway materials. No skill can bypass the staircase.
4. **High mixing.** Every consumable recipe at T2+ draws inputs from 2+ crafting skills. No single crafter can produce advanced consumables alone.
5. **Meaningful choices under permanent death.** Using a consumable means it is gone. Carrying too many reduces loot capacity. Carrying too few means death. Supply planning is a strategic decision.

---

## Consumable Categories

### 1. Potions (Alchemy Primary)

Liquid consumables brewed via the Alchemy skill. Covers healing, buffs, resistances, and alchemical weapons. Using a potion costs **1 Action** in combat (same as attacking). Only **one buff potion** and **one resistance potion** can be active simultaneously -- using a new one of the same category replaces the previous effect. **Healing potion cooldown:** 5 rounds between healing potion uses.

### 2. Scrolls (Enchanting Primary)

Enchanted parchment that releases a stored magical effect when activated. Covers teleportation, warding, communication, and identification. Using a scroll costs **1 Action** in combat. Some scrolls have a **channel time** (interruptible by damage).

### 3. Runes & Sigils (Runecrafting Primary)

Inscribed stones or surfaces that create persistent or triggered effects. Covers trap detection, warding passages, breach prevention, and dungeon utility. Placing a rune costs **1 Action**; the effect persists for a duration.

### 4. Divine Items (Theurgy Primary)

Holy or primordial consumables for corruption management and divine protection. Available only at high tiers (T8+) since Theurgy is the T10 gateway skill. Covers corruption cleansing, sanctification, and divine wards.

### 5. Alchemical Weapons (Alchemy + Smithing)

Throwable combat consumables dealing AoE damage or applying conditions. Costs **1 Action** to throw. Hits all enemies in the targeted area.

### 6. Mechanical Devices (Artificing Primary)

Reusable-but-limited devices for coordination, crafting logistics, and utility. Available at high tiers (T8+) since Artificing is the T9 gateway skill.

---

## Scaling Formulas

### Healing Potency

```
heal_amount(tier) = 2 * tier + 5    (T1+; T0 Crude Salve is hand-designed at 3 HP)
```

| Tier | Heal | Typical Enemy Damage/Hit | Hits Recovered | % Warrior HP |
|------|------|--------------------------|----------------|--------------|
| T0 | 3 | 2.5 | 1.2 | N/A |
| T1 | 7 | 6.2 | 1.1 | 47% |
| T2 | 9 | 7.2 | 1.3 | 45% |
| T3 | 11 | 9.2 | 1.2 | 39% |
| T4 | 13 | 10.2 | 1.3 | 34% |
| T5 | 15 | 12.2 | 1.2 | 29% |
| T6 | 17 | 13.2 | 1.3 | 25% |
| T7 | 19 | 15.2 | 1.3 | 18% |
| T8 | 21 | 18.2 | 1.2 | 14% |
| T9 | 23 | 23.8 | 1.0 | 11% |
| T10 | 25 | 27.8 | 0.9 | 8% |

> **Rebalanced via simulator v5.** Previous formula `3 * tier + 5` allowed potions to over-heal relative to incoming damage at mid tiers. Reduced to `2 * tier + 5` and cooldown increased from 3 to 5 rounds. Consumables now give 15-25% win rate improvement with a full loadout. The shallower formula keeps potions at ~1 enemy hit of recovery, making them useful emergency tools without trivializing permanent death.

**Design intent:** A healing potion buys 1-2 rounds of survival at every tier. At low tiers this is significant (solo play). At high tiers, 1-2 extra rounds buys time for the dedicated healer but does not substitute for one. The declining %HP curve forces reliance on party healing at T6+, which is the coordination bottleneck the game is built around.

**Potion cooldown:** 5-round cooldown between healing potion uses. You cannot drink another healing potion for 5 rounds after using one. This prevents degenerate chug-spam and forces tactical timing decisions.

**Previous formula (deprecated):** `tier^2 + 4*tier + 3` -- note the wiki previously listed this as `2*tier^2 + 3*tier + 3` which was a transcription error (the table values matched `tier^2 + 4*tier + 3`). Both are now deprecated in favor of the linear formula above.

### Buff Duration

```
buff_duration_rounds(tier) = 3 + floor(tier / 2)
```

| Tier | Duration (rounds) |
|------|-------------------|
| T0-T1 | 3 |
| T2-T3 | 4 |
| T4-T5 | 5 |
| T6-T7 | 6 |
| T8-T9 | 7 |
| T10 | 8 |

### Buff Potency

```
buff_bonus(tier) = 1 + floor(tier / 2)
```

| Tier | Stat Bonus |
|------|------------|
| T0-T1 | +1 |
| T2-T3 | +2 |
| T4-T5 | +3 |
| T6-T7 | +4 |
| T8-T9 | +5 |
| T10 | +6 |

### Alchemical Weapon Damage

```
alch_weapon_damage(tier) = "1d6 + tier * 2" (AoE, no to-hit roll, Dex save for half)
alch_weapon_save_dc(tier) = 10 + tier * 2
```

### Corruption Resistance (T7+)

```
corruption_rate_reduction(tier) = 0.40 + 0.05 * (tier - 7)
duration_rounds = 10
```

| Tier | Corruption Rate Reduced By | Effective Extra Rounds |
|------|---------------------------|----------------------|
| T7 | 40% | +40 rounds (100 to 140) |
| T8 | 45% | +30 rounds (67 to 97) |
| T9 | 50% | +25 rounds (50 to 75) |
| T10 | 55% | +18 rounds (33 to 51) |

**Design intent:** Corruption resistance potions extend dungeon time by roughly 50% but do NOT eliminate the rotation requirement. A T8 raid still needs 2 rotation waves (down from 3), and that reduction requires a steady supply of expensive potions.

---

## Complete Item Catalog

### Tier 0 -- Crude Remedies

Solo-craftable from gathered T0 materials. Designed to teach consumable mechanics.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Crude Salve | Potion | Heal 3 HP | Alchemy | 0 |
| Herbal Bandage | Potion | Heal 5 HP over 3 rounds (out of combat only) | Alchemy | 0 |
| Bitter Root Tea | Potion | Reduce next flee DC by 2 | Alchemy | 0 |

**Total T0 consumables: 3**

### Tier 1 -- Basic Consumables

First potions requiring crafted materials. Still primarily solo-craftable by an alchemist.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Minor Healing Potion | Potion | Heal 7 HP | Alchemy | 1 |
| Antivenom | Potion | Cure poison condition | Alchemy | 1 |
| Smelling Salts | Potion | Cure stun condition | Alchemy | 1 |
| Torch Oil | Utility | Light radius +50% for 1 hour | Alchemy | 1 |

**Total T1 consumables: 4**

### Tier 2 -- Standard Consumables

First buff potions and alchemical weapons. Recipes start requiring 2-3 crafting skills.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Healing Potion | Potion | Heal 9 HP | Alchemy | 2 |
| Potion of Strength | Buff Potion | +2 attack bonus, 4 rounds | Alchemy | 2 |
| Potion of Iron Skin | Buff Potion | +2 AC, 4 rounds | Alchemy | 2 |
| Smoke Bomb | Alch. Weapon | Obscure 1 room, 3 rounds (disadvantage on ranged attacks) | Alchemy | 2 |
| Identify Scroll | Scroll | Reveal stats of one unknown item | Enchanting | 2 |

**Total T2 consumables: 5**

### Tier 3 -- Veteran Consumables

Town Portal Scrolls unlock. Trap detection becomes available. Recipes need 3 skills.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Greater Healing Potion | Potion | Heal 11 HP | Alchemy | 3 |
| Potion of Accuracy | Buff Potion | +2 to-hit bonus, 4 rounds | Alchemy | 3 |
| Poison Resistance Potion | Resistance | Advantage on poison saves, 4 rounds | Alchemy | 3 |
| Town Portal Scroll | Scroll | 3-sec channel, teleport to nearest town | Enchanting | 3 |
| Trap Detection Rune | Rune | Reveals all traps within 3 rooms, 10 min | Runecrafting | 3 |
| Alchemist Fire | Alch. Weapon | 1d6+6 fire AoE, Dex DC 16 half | Alchemy | 3 |

**Total T3 consumables: 6**

### Tier 4 -- Elite Consumables (Alchemy Gateway)

Alchemy gateway tier. Advanced potion brewing unlocks. Haste potion available. 3-4 skill inputs.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Superior Healing Potion | Potion | Heal 13 HP | Alchemy | 4 |
| Potion of Haste | Buff Potion | +1 bonus action for 2 rounds, +3 initiative | Alchemy | 4 |
| Elemental Resistance Potion | Resistance | Halve fire/cold/lightning damage, 5 rounds | Alchemy | 4 |
| Invisibility Potion | Utility | 3 rounds of stealth (broken by attacking/taking damage) | Alchemy | 4 |
| Trap Disarm Rune | Rune | Automatically disarms next triggered trap | Runecrafting | 4 |
| Frost Bomb | Alch. Weapon | 1d6+8 cold AoE + half movement 2 rounds, DC 18 | Alchemy | 4 |
| Warding Scroll | Scroll | Absorb next 20 damage on target, 5 rounds | Enchanting | 4 |

**Total T4 consumables: 7**

### Tier 5 -- Vanguard Consumables (Enchanting Gateway)

Multi-stat buffs. Signal flares for raid coordination. 4-5 skill inputs per recipe.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Potent Healing Elixir | Potion | Heal 15 HP | Alchemy | 5 |
| Elixir of Fortification | Buff Potion | +3 AC, +15 temp HP, 5 rounds | Alchemy | 5 |
| Elixir of Fury | Buff Potion | +3 attack bonus, +1d4 damage, 5 rounds | Alchemy | 5 |
| Dark Resistance Potion | Resistance | Advantage on saves vs undead/dark effects, 5 rounds | Alchemy | 5 |
| Signal Flare | Device | Visible flash across entire dungeon floor, coordination tool | Enchanting | 5 |
| Greater Town Portal Scroll | Scroll | 2-sec channel (faster), teleport to nearest town | Enchanting | 5 |
| Acid Vial | Alch. Weapon | 1d6+10 acid AoE, -2 AC to targets 3 rounds, DC 20 | Alchemy | 5 |

**Total T5 consumables: 7**

### Tier 6 -- Exalted Consumables (Tailoring Gateway)

Powerful single-target buffs. Communication scrolls for split-path dungeons. 5-6 skill inputs.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Arcane Healing Elixir | Potion | Heal 17 HP | Alchemy | 6 |
| Elixir of Mastery | Buff Potion | +4 to all combat rolls, 6 rounds | Alchemy | 6 |
| Arcane Resistance Elixir | Resistance | Halve all magical damage, 6 rounds | Alchemy | 6 |
| Communication Scroll | Scroll | Send message to any player in same dungeon instance | Enchanting | 6 |
| Greater Warding Scroll | Scroll | Absorb next 40 damage on target, 6 rounds | Enchanting | 6 |
| Sealing Sigil | Rune | Seal a passage for 30 min (blocks enemy respawn) | Runecrafting | 6 |
| Night Vision Elixir | Utility | See through magical darkness for 1 hour | Alchemy | 6 |

**Total T6 consumables: 7**

### Tier 7 -- Mythic Consumables (Jewelcrafting Gateway)

**Corruption resistance unlocks.** Critical tier for dungeon sustainability. 6-7 skill inputs.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Mythic Healing Draught | Potion | Heal 19 HP | Alchemy | 7 |
| Elixir of the Titan | Buff Potion | +4 ATK, +4 AC, +20 temp HP, 6 rounds | Alchemy | 7 |
| Corruption Resistance Potion | Resistance | Reduce corruption gain rate by 40% for 10 rounds | Alchemy | 7 |
| Warding Sigil | Rune | Seal a passage + prevent enemy respawn, 1 hour | Runecrafting | 7 |
| Breach Warning Rune | Rune | Alerts party if dungeon breach attempted within 30 min | Runecrafting | 7 |
| Mass Communication Scroll | Scroll | Broadcast message to all allies in dungeon | Enchanting | 7 |
| Stasis Bomb | Alch. Weapon | 1d6+14 AoE + paralyze 1 round (Con DC 24), single target | Alchemy | 7 |

**Total T7 consumables: 7**

### Tier 8 -- Legendary Consumables (Runecrafting Gateway)

Corruption management suite expands. Purification incense (Theurgy) enters. Dungeon-only material recipes. 7-8 skill inputs.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Legendary Healing Draught | Potion | Heal 21 HP | Alchemy | 8 |
| Elixir of Transcendence | Buff Potion | +5 all combat rolls, 7 rounds | Alchemy | 8 |
| Greater Corruption Resist | Resistance | Reduce corruption gain by 45%, 10 rounds | Alchemy | 8 |
| Purification Incense | Divine | Reduce current corruption by 15% (absolute) | Theurgy | 5 |
| Void Ward Rune | Rune | Corruption immunity for 5 rounds (single target) | Runecrafting | 8 |
| Breach Seal Scroll | Scroll | Prevent dungeon breach for 1 hour | Enchanting | 8 |
| Portable Alchemy Station | Device | Allows T4 alchemy recipes inside dungeons (3 uses) | Artificing | 5 |
| Mass Healing Bomb | Alch. Weapon | Heal all allies within 1 room for 15 HP | Alchemy | 8 |

**Total T8 consumables: 8**

### Tier 9 -- Eternal Consumables (Artificing Gateway)

Portable crafting stations. Divine sanctum wards. Recipes need 8-9 skills. Some require dungeon-only materials.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Eternal Healing Draught | Potion | Heal 23 HP | Alchemy | 9 |
| Elixir of Perfection | Buff Potion | +5 all combat rolls, +30 temp HP, 7 rounds | Alchemy | 9 |
| Supreme Corruption Resist | Resistance | Reduce corruption gain by 50%, 10 rounds | Alchemy | 9 |
| Greater Purification Incense | Divine | Reduce current corruption by 25% | Theurgy | 7 |
| Sanctified Ground Ward | Divine | Create 3-room safe zone vs undead, 30 min | Theurgy | 7 |
| Portable Crafting Station | Device | Allows T6 recipes (any skill) inside dungeons, 5 uses | Artificing | 7 |
| Coordination Beacon | Device | All allies on same floor see each other on map, 1 hour | Artificing | 7 |
| Superior Town Portal Scroll | Scroll | 1-sec channel, teleport + bring 1 adjacent ally | Enchanting | 9 |

**Total T9 consumables: 8**

### Tier 10 -- Primordial Consumables (Theurgy Gateway)

The ultimate consumables. Require ALL 10 crafting skills contributing materials. Dungeon-only ingredients mandatory. Guild-level production operations.

| Item | Type | Effect | Primary Skill | Rank |
|------|------|--------|---------------|------|
| Primordial Healing Draught | Potion | Heal 25 HP | Alchemy | 10 |
| Primordial Fury Elixir | Buff Potion | +6 ATK, +2 damage dice, +6 AC, 8 rounds | Alchemy | 10 |
| Primordial Corruption Ward | Resistance | Reduce corruption by 55%, 10 rounds | Alchemy | 10 |
| Primordial Purification Rite | Divine | Reduce corruption by 40%, affects all allies in room | Theurgy | 10 |
| Primordial Warding Array | Rune | Seal 5 passages + corruption immunity zone, 2 hours | Runecrafting | 10 |
| Titan Tears | Potion | Full heal to max HP (single use, dungeon-only mats) | Alchemy | 10 |
| Mobile Command Station | Device | Portable T8 crafting + coordination beacon + comms, 10 uses | Artificing | 9 |
| Primordial Portal Scroll | Scroll | Instant teleport, bring up to 5 allies, no channel | Enchanting | 10 |

**Total T10 consumables: 8**

**Grand Total: 70 consumable types across 11 tiers.**

---

## Recipe Database

### Recipe Notation

Each recipe lists:
- **Recipe ID** -- unique identifier
- **Skill** -- primary crafting skill required
- **Rank** -- minimum skill rank
- **Inputs** -- materials consumed (with source skill abbreviations)
- **Mixing Score** -- number of distinct crafting skills providing inputs

### Tier 0 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t0_salve` | Crude Salve | AL | 0 | Wild Herbs x3, Muddy Clay x1 | 0 (gathered) |
| `con_t0_bandage` | Herbal Bandage | AL | 0 | Plant Fiber x2, Wild Herbs x2 | 0 (gathered) |
| `con_t0_tea` | Bitter Root Tea | AL | 0 | Wild Herbs x2, Bone Dust x1 | 0 (gathered+drop) |

### Tier 1 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t1_heal` | Minor Healing Potion | AL | 1 | Herbal Paste x2 (AL), Ectoplasm x1 (Undead drop) | 1 |
| `con_t1_antivenom` | Antivenom | AL | 1 | Herbal Paste x1 (AL), Venom Sac x2 (Skulker drop), Bone Dust x1 | 1 |
| `con_t1_smelling_salts` | Smelling Salts | AL | 1 | Herbal Paste x1 (AL), Wisp Essence x2 (Mystic drop) | 1 |
| `con_t1_torch_oil` | Torch Oil | AL | 1 | Herbal Paste x1 (AL), Charcoal x3 | 1 |

### Tier 2 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t2_heal` | Healing Potion | AL | 2 | Refined Potion Base x1 (AL), Cured Hide x1 (LW), Enchanted Thread x1 (EN) | 3 |
| `con_t2_strength` | Potion of Strength | AL | 2 | Refined Potion Base x1 (AL), Iron Ingot x1 (SM), Wolf Pelt x1 (Brute drop) | 2 |
| `con_t2_ironskin` | Potion of Iron Skin | AL | 2 | Refined Potion Base x1 (AL), Hardened Leather x1 (LW), Iron Ingot x1 (SM) | 3 |
| `con_t2_smoke` | Smoke Bomb | AL | 2 | Refined Potion Base x1 (AL), Iron Nugget x1 (SM), Charcoal x3 | 2 |
| `con_t2_identify` | Identify Scroll | EN | 2 | Enchanted Thread x2 (EN), Woven Cloth x1 (TL), Arcane Crystal x1 (Mystic drop) | 2 |

### Tier 3 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t3_heal` | Greater Healing Potion | AL | 3 | Alchemical Catalyst x2 (AL), Reinforced Leather x1 (LW), Power Rune x1 (RC) | 3 |
| `con_t3_accuracy` | Potion of Accuracy | AL | 3 | Alchemical Catalyst x1 (AL), Jeweled Setting x1 (JC), Phase Silk x1 (Skulker drop) | 2 |
| `con_t3_poison_resist` | Poison Resistance Potion | AL | 3 | Alchemical Catalyst x2 (AL), Moonsilk x1 (TL), Wraith Dust x1 (Undead drop) | 2 |
| `con_t3_portal` | Town Portal Scroll | EN | 3 | Mana Weave x1 (EN), Alchemical Catalyst x1 (AL), Power Rune x1 (RC) | 3 |
| `con_t3_trap_detect` | Trap Detection Rune | RC | 3 | Power Rune x2 (RC), Alchemical Catalyst x1 (AL), Cut Gemstone x1 (JC) | 3 |
| `con_t3_fire` | Alchemist Fire | AL | 3 | Alchemical Catalyst x1 (AL), Steel Plate x1 (SM), Elemental Core x1 (Mystic drop) | 2 |

### Tier 4 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t4_heal` | Superior Healing Potion | AL | 4 | Alchemical Elixir Base x2 (AL), Alchemical Hide x1 (LW), Alchemical Rune x1 (RC) | 3 |
| `con_t4_haste` | Potion of Haste | AL | 4 | Alchemical Elixir Base x1 (AL), Alchemical Weave x1 (EN), Alchemical Gem x1 (JC), Phase Venom x1 (Skulker drop) | 3 |
| `con_t4_elem_resist` | Elemental Resistance Potion | AL | 4 | Alchemical Elixir Base x1 (AL), Alchemical Steel x1 (SM), Alchemical Silk x1 (TL), Elemental Heart x1 (Mystic drop) | 3 |
| `con_t4_invis` | Invisibility Potion | AL | 4 | Alchemical Elixir Base x1 (AL), Alchemical Silk x1 (TL), Alchemical Rune x1 (RC), Phase Venom x2 (Skulker drop) | 3 |
| `con_t4_trap_disarm` | Trap Disarm Rune | RC | 4 | Alchemical Rune x2 (RC), Alchemical Elixir Base x1 (AL), Alchemical Gem x1 (JC) | 3 |
| `con_t4_frost` | Frost Bomb | AL | 4 | Alchemical Elixir Base x1 (AL), Alchemical Steel x1 (SM), Alchemical Hardwood x1 (WW), Troll Blood x1 (Brute drop) | 3 |
| `con_t4_ward` | Warding Scroll | EN | 4 | Alchemical Weave x2 (EN), Alchemical Rune x1 (RC), Mummy Wrappings x1 (Undead drop) | 2 |

### Tier 5 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t5_heal` | Potent Healing Elixir | AL | 5 | Enchanted Elixir x2 (AL), Enchanted Hide x1 (LW), Enchanted Rune x1 (RC), Enchanted Gem x1 (JC) | 4 |
| `con_t5_fortify` | Elixir of Fortification | AL | 5 | Enchanted Elixir x1 (AL), Enchanted Steel x1 (SM), Enchanted Silk x1 (TL), Enchanted Gem x1 (JC), Naga Pearl x1 (Mystic drop) | 4 |
| `con_t5_fury` | Elixir of Fury | AL | 5 | Enchanted Elixir x1 (AL), Enchanted Steel x1 (SM), Enchanted Hardwood x1 (WW), Stalker Claw x1 (Skulker drop) | 3 |
| `con_t5_dark_resist` | Dark Resistance Potion | AL | 5 | Enchanted Elixir x1 (AL), Enchanted Rune x1 (RC), Enchanted Silk x1 (TL), Banshee Wail x2 (Undead drop) | 3 |
| `con_t5_flare` | Signal Flare | EN | 5 | Enchanted Mana Crystal x1 (EN), Enchanted Steel x1 (SM), Enchanted Elixir x1 (AL), Giant Sinew x1 (Brute drop) | 3 |
| `con_t5_portal_gt` | Greater Town Portal Scroll | EN | 5 | Enchanted Mana Crystal x2 (EN), Enchanted Elixir x1 (AL), Enchanted Rune x1 (RC) | 3 |
| `con_t5_acid` | Acid Vial | AL | 5 | Enchanted Elixir x1 (AL), Enchanted Steel x1 (SM), Enchanted Hardwood x1 (WW), Naga Pearl x1, Stalker Claw x1 | 3 |

### Tier 6 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t6_heal` | Arcane Healing Elixir | AL | 6 | Arcane Elixir x2 (AL), Arcane Hide x1 (LW), Arcane Rune x1 (RC), Arcane Gem x1 (JC), Arcane Tapestry x1 (TL) | 5 |
| `con_t6_mastery` | Elixir of Mastery | AL | 6 | Arcane Elixir x1 (AL), Arcane Steel x1 (SM), Arcane Weave x1 (EN), Arcane Gem x1 (JC), Golem Core x1 (Brute drop) | 4 |
| `con_t6_arcane_resist` | Arcane Resistance Elixir | AL | 6 | Arcane Elixir x1 (AL), Arcane Weave x1 (EN), Arcane Tapestry x1 (TL), Elder Crystal x2 (Mystic drop) | 3 |
| `con_t6_comms` | Communication Scroll | EN | 6 | Arcane Weave x2 (EN), Arcane Rune x1 (RC), Arcane Tapestry x1 (TL), Nightwalker Shade x1 (Skulker drop) | 3 |
| `con_t6_ward_gt` | Greater Warding Scroll | EN | 6 | Arcane Weave x2 (EN), Arcane Rune x1 (RC), Arcane Steel x1 (SM), Death Knight Shard x1 (Undead drop) | 3 |
| `con_t6_seal` | Sealing Sigil | RC | 6 | Arcane Rune x2 (RC), Arcane Weave x1 (EN), Arcane Hide x1 (LW), Golem Core x1 (Brute drop) | 3 |
| `con_t6_night_vision` | Night Vision Elixir | AL | 6 | Arcane Elixir x1 (AL), Arcane Gem x1 (JC), Nightwalker Shade x2 (Skulker drop), Arcane Tapestry x1 (TL) | 3 |

### Tier 7 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t7_heal` | Mythic Healing Draught | AL | 7 | Jeweled Elixir x2 (AL), Jeweled Hide x1 (LW), Jeweled Rune x1 (RC), Jeweled Tapestry x1 (TL), Precious Diadem x1 (JC) | 5 |
| `con_t7_titan` | Elixir of the Titan | AL | 7 | Jeweled Elixir x1 (AL), Jeweled Steel x1 (SM), Jeweled Weave x1 (EN), Jeweled Tapestry x1 (TL), Dragon Scale x1 (Brute drop), Beholder Eye x1 (Mystic drop) | 4 |
| `con_t7_corrupt_resist` | Corruption Resistance Potion | AL | 7 | Jeweled Elixir x2 (AL), Jeweled Rune x1 (RC), Jeweled Tapestry x1 (TL), Beholder Eye x1, Lich Phylactery x1 | 3 |
| `con_t7_ward_sigil` | Warding Sigil | RC | 7 | Jeweled Rune x2 (RC), Jeweled Weave x1 (EN), Jeweled Steel x1 (SM), Lich Phylactery x1 (Undead drop) | 3 |
| `con_t7_breach_rune` | Breach Warning Rune | RC | 7 | Jeweled Rune x2 (RC), Jeweled Hardwood x1 (WW), Jeweled Elixir x1 (AL), Gloom Silk x1 (Skulker drop) | 3 |
| `con_t7_mass_comms` | Mass Communication Scroll | EN | 7 | Jeweled Weave x2 (EN), Jeweled Rune x1 (RC), Jeweled Tapestry x1 (TL), Precious Diadem x1 (JC) | 4 |
| `con_t7_stasis` | Stasis Bomb | AL | 7 | Jeweled Elixir x1 (AL), Jeweled Steel x1 (SM), Precious Diadem x1 (JC), Dragon Scale x1, Beholder Eye x1 | 3 |

### Tier 8 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t8_heal` | Legendary Healing Draught | AL | 8 | Runic Elixir x2 (AL), Runic Hide x1 (LW), Runic Keystone x1 (RC), Runic Tapestry x1 (TL), Runic Gem x1 (JC), Runic Steel x1 (SM) | 6 |
| `con_t8_transcend` | Elixir of Transcendence | AL | 8 | Runic Elixir x1 (AL), Runic Steel x1 (SM), Runic Weave x1 (EN), Runic Tapestry x1 (TL), Runic Gem x1 (JC), Storm Essence x1, Astral Fragment x1 | 5 |
| `con_t8_corrupt_gt` | Greater Corruption Resist | AL | 8 | Runic Elixir x2 (AL), Runic Keystone x1 (RC), Runic Tapestry x1 (TL), Runic Gem x1 (JC), Void Silk x1, Demilich Gem x1 | 4 |
| `con_t8_purify` | Purification Incense | TH | 5 | Runic Elixir x1 (AL), Runic Keystone x1 (RC), Runic Tapestry x1 (TL), Runic Weave x1 (EN), Demilich Gem x2, Astral Fragment x1 | 4 |
| `con_t8_void_ward` | Void Ward Rune | RC | 8 | Runic Keystone x2 (RC), Runic Weave x1 (EN), Runic Elixir x1 (AL), Runic Gem x1 (JC), Void Silk x2 | 4 |
| `con_t8_breach_seal` | Breach Seal Scroll | EN | 8 | Runic Weave x2 (EN), Runic Keystone x1 (RC), Runic Hide x1 (LW), Runic Hardwood x1 (WW), Storm Essence x1 | 4 |
| `con_t8_portable_alch` | Portable Alchemy Station | AF | 5 | Runic Steel x2 (SM), Runic Hardwood x1 (WW), Runic Elixir x1 (AL), Runic Keystone x1 (RC), Runic Gem x1 (JC) | 5 |
| `con_t8_mass_heal` | Mass Healing Bomb | AL | 8 | Runic Elixir x3 (AL), Runic Weave x1 (EN), Runic Tapestry x1 (TL), Runic Gem x1 (JC), Astral Fragment x2 | 4 |

### Tier 9 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t9_heal` | Eternal Healing Draught | AL | 9 | Artificed Elixir x2 (AL), Artificed Hide x1 (LW), Artificed Keystone x1 (RC), Artificed Tapestry x1 (TL), Artificed Gem x1 (JC), Artificed Steel x1 (SM), Artificed Weave x1 (EN) | 7 |
| `con_t9_perfection` | Elixir of Perfection | AL | 9 | Artificed Elixir x1 (AL), Artificed Steel x1 (SM), Artificed Weave x1 (EN), Artificed Tapestry x1 (TL), Artificed Gem x1 (JC), Artificed Hardwood x1 (WW), Titan Bone x1, Dracolich Dust x1 | 6 |
| `con_t9_corrupt_sup` | Supreme Corruption Resist | AL | 9 | Artificed Elixir x2 (AL), Artificed Keystone x1 (RC), Artificed Tapestry x1 (TL), Artificed Gem x1 (JC), Artificed Weave x1 (EN), Void Silk x2, Demilich Gem x1 | 5 |
| `con_t9_purify_gt` | Greater Purification Incense | TH | 7 | Artificed Elixir x1 (AL), Artificed Keystone x1 (RC), Artificed Tapestry x1 (TL), Artificed Weave x1 (EN), Artificed Gem x1 (JC), Artificed Hardwood x1 (WW), Dracolich Dust x2 | 6 |
| `con_t9_sanctum` | Sanctified Ground Ward | TH | 7 | Artificed Keystone x1 (RC), Artificed Tapestry x1 (TL), Artificed Weave x1 (EN), Artificed Hide x1 (LW), Artificed Steel x1 (SM), Dracolich Dust x2, Titan Bone x1 | 5 |
| `con_t9_portable_craft` | Portable Crafting Station | AF | 7 | Artificed Mechanism x2 (AF), Artificed Steel x1 (SM), Artificed Hardwood x1 (WW), Artificed Weave x1 (EN), Artificed Keystone x1 (RC), Artificed Elixir x1 (AL), Artificed Gem x1 (JC) | 7 |
| `con_t9_beacon` | Coordination Beacon | AF | 7 | Artificed Mechanism x1 (AF), Artificed Weave x1 (EN), Artificed Gem x1 (JC), Artificed Keystone x1 (RC), Titan Bone x1, Astral Fragment x1 | 4 |
| `con_t9_portal_sup` | Superior Town Portal Scroll | EN | 9 | Artificed Weave x2 (EN), Artificed Keystone x1 (RC), Artificed Elixir x1 (AL), Artificed Tapestry x1 (TL), Artificed Gem x1 (JC), Titan Bone x1 | 5 |

### Tier 10 Recipes

| Recipe ID | Product | Skill | Rank | Inputs | Mixing |
|-----------|---------|-------|------|--------|--------|
| `con_t10_heal` | Primordial Healing Draught | AL | 10 | Divine Elixir x2 (AL), Divine Hide x1 (LW), Divine Keystone x1 (RC), Divine Tapestry x1 (TL), Divine Gem x1 (JC), Divine Steel x1 (SM), Divine Weave x1 (EN), Divine Hardwood x1 (WW) | 8 |
| `con_t10_fury` | Primordial Fury Elixir | AL | 10 | Divine Elixir x1 (AL), Divine Steel x1 (SM), Divine Weave x1 (EN), Divine Tapestry x1 (TL), Divine Gem x1 (JC), Divine Hardwood x1 (WW), Divine Mechanism x1 (AF), Primordial Essence x2 (dungeon-only) | 7 |
| `con_t10_corrupt_prim` | Primordial Corruption Ward | AL | 10 | Divine Elixir x2 (AL), Divine Keystone x1 (RC), Divine Tapestry x1 (TL), Divine Gem x1 (JC), Divine Weave x1 (EN), Divine Essence x1 (TH), Primordial Essence x2 (dungeon-only) | 6 |
| `con_t10_purify_rite` | Primordial Purification Rite | TH | 10 | Divine Essence x2 (TH), Divine Elixir x1 (AL), Divine Keystone x1 (RC), Divine Weave x1 (EN), Divine Tapestry x1 (TL), Divine Gem x1 (JC), Divine Steel x1 (SM), Primordial Essence x3 (dungeon-only) | 7 |
| `con_t10_ward_array` | Primordial Warding Array | RC | 10 | Divine Keystone x3 (RC), Divine Weave x1 (EN), Divine Essence x1 (TH), Divine Steel x1 (SM), Divine Tapestry x1 (TL), Divine Hardwood x1 (WW), Primordial Essence x2 (dungeon-only) | 6 |
| `con_t10_tears` | Titan Tears | AL | 10 | Divine Elixir x3 (AL), Divine Essence x1 (TH), Divine Gem x1 (JC), Divine Keystone x1 (RC), Divine Tapestry x1 (TL), Divine Mechanism x1 (AF), Primordial Heart x1 (T10 boss-only drop) | 6 |
| `con_t10_command` | Mobile Command Station | AF | 9 | Divine Mechanism x3 (AF), Divine Steel x1 (SM), Divine Hardwood x1 (WW), Divine Weave x1 (EN), Divine Keystone x1 (RC), Divine Elixir x1 (AL), Divine Gem x1 (JC), Divine Tapestry x1 (TL), Divine Essence x1 (TH) | 9 |
| `con_t10_portal_prim` | Primordial Portal Scroll | EN | 10 | Divine Weave x3 (EN), Divine Keystone x1 (RC), Divine Elixir x1 (AL), Divine Tapestry x1 (TL), Divine Gem x1 (JC), Divine Essence x1 (TH), Divine Mechanism x1 (AF), Primordial Essence x1 (dungeon-only) | 7 |

---

## Cross-Skill Dependency Analysis

### Mixing Score Summary

| Tier | Avg Mixing Score | Min | Max | Distinct Skills per Tier |
|------|-----------------|-----|-----|--------------------------|
| T0 | 0.0 | 0 | 0 | 1 (AL only) |
| T1 | 1.0 | 1 | 1 | 1 (AL only) |
| T2 | 2.4 | 2 | 3 | 4 (AL, SM, LW, EN) |
| T3 | 2.5 | 2 | 3 | 5 (AL, EN, RC, JC, SM) |
| T4 | 3.0 | 2 | 3 | 6 (AL, EN, RC, SM, TL, WW) |
| T5 | 3.4 | 3 | 4 | 7 (AL, EN, RC, SM, LW, TL, WW) |
| T6 | 3.4 | 3 | 5 | 6 (AL, EN, RC, LW, TL, JC) |
| T7 | 3.6 | 3 | 5 | 7 (AL, EN, RC, SM, LW, TL, JC) |
| T8 | 4.5 | 4 | 6 | 8 (AL, EN, RC, SM, LW, TL, JC, AF) |
| T9 | 5.6 | 4 | 7 | 9 (AL, EN, RC, SM, LW, TL, JC, AF, TH) |
| T10 | 7.0 | 6 | 9 | 10 (all skills) |

### Skill Usage in Consumable Recipes

How many consumable recipes use products from each skill:

| Skill | T0-T3 | T4-T6 | T7-T10 | Total | Role |
|-------|-------|-------|--------|-------|------|
| Alchemy (AL) | 10 | 18 | 27 | 55 | Primary for all potions/elixirs |
| Enchanting (EN) | 2 | 8 | 18 | 28 | Scrolls + high-tier secondary |
| Runecrafting (RC) | 2 | 7 | 20 | 29 | Runes + anti-corruption |
| Leatherworking (LW) | 1 | 4 | 10 | 15 | Containers, hide reagents |
| Smithing (SM) | 2 | 5 | 12 | 19 | Metal reagents, bomb casings |
| Tailoring (TL) | 1 | 5 | 16 | 22 | Wrappings, filters, fabric reagents |
| Jewelcrafting (JC) | 1 | 4 | 16 | 21 | Catalysts, gem reagents |
| Woodworking (WW) | 0 | 3 | 8 | 11 | Frames, handles, stabilizers |
| Artificing (AF) | 0 | 0 | 7 | 7 | Devices (T8+), mechanisms |
| Theurgy (TH) | 0 | 0 | 7 | 7 | Divine items (T8+), purification |

**Design validation:** Alchemy dominates as expected (it is the potion skill), but every other skill participates as secondary ingredient sources. No single crafter beyond T1 can produce consumables alone -- even a maxed Alchemist needs Leatherworkers, Runecrafters, and more.

### Monster Drop Usage in Consumables

| Monster Type | Drop Used In Consumables |
|-------------|--------------------------|
| Brute | Strength potions, fortification, fire bombs, signal flares |
| Skulker | Accuracy potions, haste, invisibility, frost bombs |
| Mystic | Elemental/arcane resistance, acid, identification, healing |
| Undead | Poison/dark resistance, warding, sealing, purification |

All four monster types feed consumable production. Guilds need diverse monster farming, not just one type.

---

## Simulation-Validated Balance (2026-03-30)

All balance data below is from Monte Carlo combat simulations (2000+ trials per matchup) using the same d20 mechanics as the live combat engine. Simulations run across all 10 classes vs all 4 enemy types at each tier.

### Solo 1v1 Balance — Rebalanced Healing Formula

Scenario: Player vs at-tier monster, 2 healing potions available, heal at 50% HP threshold.

| Tier | Baseline Win% | +Healing Win% | Delta | Target |
|------|--------------|--------------|-------|--------|
| T1 | 57% | 71% | +14% | 15-25% |
| T2 | 59% | 80% | +21% | 15-25% |
| T4 | 59% | 85% | +26% | 15-25% |
| T6 | 56% | 82% | +26% | 15-25% |
| T8 | 48% | 69% | +21% | 15-25% |
| T10 | 33% | 44% | +11% | 10-15% |

**Analysis:** The linear healing formula (`3*tier + 5`) keeps the delta within the 15-26% range at T1-T8, hitting the 20-30% target. At T10, the delta drops to +11% -- intentional, because T10 content requires coordinated parties, not solo potion-chugging. Death remains a real threat at every tier: even with 2 potions, a T10 solo player still dies 56% of the time.

### Buff Potion Balance

Scenario: Player vs at-tier monster, one buff potion active at combat start.

| Tier | ATK Buff Delta | DEF Buff Delta | Combined w/ Heal |
|------|---------------|---------------|-----------------|
| T1 | +3.5% | +3.0% | +16% |
| T2 | +8.4% | +7.0% | +27% |
| T4 | +14.0% | +12.2% | +36%* |
| T6 | +17.9% | +16.4% | +42%* |
| T8 | +20.9% | +19.8% | +50%* |
| T10 | +20.6% | +21.1% | +63%* |

*Combined values use old healing formula. With the rebalanced linear healing formula, combined Heal+Buff deltas are 15-30% lower.

**Analysis:** Buff potions alone give a meaningful but not overwhelming advantage (+3% to +21% across tiers). ATK buffs are slightly stronger than DEF buffs at most tiers. The buff formula (`1 + floor(tier/2)` bonus for `3 + floor(tier/2)` rounds) is correctly balanced -- no changes needed.

**Note:** Only one buff potion can be active at a time. The choice between ATK buff (+to-hit) and DEF buff (+AC) is a meaningful tactical decision with similar impact.

### PvP Balance

Scenario: Warrior vs Warrior mirror match. P1 has consumables, P2 does not. 3000 trials per scenario.

| Tier | Mirror (50% expected) | P1 Heal Only | P1 Buff Only | P1 Full |
|------|----------------------|-------------|-------------|---------|
| T1 | 56% | 72% | 62% | 77% |
| T2 | 55% | 86% | 67% | 93% |
| T4 | 56% | 93% | 70% | 98% |
| T6 | 55% | 96% | 73% | 99% |
| T8 | 53% | 99% | 77% | 100% |
| T10 | 52% | 99% | 77% | 100% |

> **These numbers use the old (deprecated) healing formula.** With the rebalanced linear formula and 3-round cooldown, the PvP advantage from healing potions is significantly reduced but still meaningful.

**PvP design context:**
- In practice, BOTH players in PvP typically have consumables -- the advantage cancels out
- Consumable advantage in PvP is **intentional** and balanced by economic cost: every potion used is permanently destroyed
- The criminal flag system provides the strategic deterrent against PvP, not consumable balance
- Guilds/escorts/numbers matter more than potion supply in open-world PvP
- A player burning 2 T8 healing potions (24 skill-hours to produce) in a PvP fight pays a real economic cost whether they win or lose

### Party Combat

Scenario: Party of 3 (Warrior/Cleric/Rogue) and Party of 5 (Warrior/Cleric/Rogue/Mage/Bard) vs boss at-tier.

| Size | Scale | T1 | T4 | T8 | T10 | Notes |
|------|-------|-----|-----|-----|------|-------|
| 3 | x2.5 | 100% | 100% | 100% | 100% | No deaths in any sim |
| 5 | x4.0 | 100% | 100% | 100% | 100% | No deaths in any sim |

**Issue identified:** Boss HP scaling (2.5x for 3, 4.0x for 5) is too low. Parties trivially defeat bosses at all tiers regardless of consumables. This is a **separate balance issue** from consumables -- the boss scaling formula needs independent tuning. Recommended: increase boss HP multiplier to ~4-5x for parties of 3, ~7-8x for parties of 5, and add multi-attack for bosses at T5+.

Consumables in party play reduce fight duration by 15-25% (fewer healing pauses) but do not affect win/loss outcome.

### Corruption Resistance Validation

Theoretical analysis from the original design holds. No simulation changes needed.

Without consumables, T8 gives about 67 rounds before corruption wipe.
With Greater Corruption Resist (45% reduction): about 97 rounds.
With Void Ward Rune (5 rounds immunity) used 3x: about 82 rounds.
With both: about 112 rounds.

Net effect: extends dungeon time by 50-67%, reducing rotation waves from 3 to 2. The logistics challenge shifts form, not magnitude.

### Consumables vs. Permanent Death

Every consumed item is permanently gone. This creates a **replacement cost** that scales with tier:

| Tier | Avg Materials per Consumable | Avg Skill-Hours to Produce 1 | Real Cost Feel |
|------|------------------------------|------------------------------|----------------|
| T0 | 4 | 0.1 | Trivial |
| T1 | 3 | 0.3 | Minor |
| T2 | 3 | 0.5 | Noticeable |
| T3 | 4 | 1.0 | Significant |
| T4 | 4 | 2.0 | Costly |
| T5 | 5 | 3.5 | Expensive |
| T6 | 5 | 5.0 | Very expensive |
| T7 | 6 | 8.0 | Precious |
| T8 | 7 | 12.0 | Rare |
| T9 | 8 | 20.0 | Extremely rare |
| T10 | 9+ | 40.0+ | Priceless |

**Implication for PvP:** Killing a player carrying 10 T7 healing potions is like destroying 80 skill-hours of crafting work. This makes PvP ganking at dungeon exits economically devastating -- and makes guard/escort logistics critical.

---

## Economic Implications

### Supply Chain per Dungeon Tier

| Dungeon Tier | Players | Consumables per Run | Crafters Needed | Material Cost |
|-------------|---------|--------------------|-----------------|----|
| T0-T2 | 1-2 | 3-5 | 0 (self-craft) | 15 raw mats |
| T3-T4 | 2-5 | 10-25 | 1 dedicated | 80 mats from 3+ skills |
| T5-T6 | 5-15 | 30-80 | 2-3 dedicated | 300 mats from 5+ skills |
| T7 | 20-35 | 80-180 | 4-6 dedicated | 900 mats from 7+ skills |
| T8 | 40-60 | 200-400 | 8-12 dedicated | 2500 mats from 8+ skills |
| T9 | 80-150 | 500-1000 | 15-25 dedicated | 7000 mats from 9+ skills |
| T10 | 200+ | 2000-5000 | 40-60 dedicated | 30000+ mats from all skills |

### T8 Raid Consumable Budget (Worked Example)

**Scenario:** 50-player T8 dungeon run, 8 floors, estimated 10-12 hours.

**Per Player (estimated):**
- 5x Legendary Healing Draught (29 HP each)
- 2x Elixir of Transcendence (+5 all rolls)
- 3x Greater Corruption Resist (45% reduction, extends to 2 rotation waves)
- 1x Void Ward Rune (boss phase emergency)
- 1x Town Portal Scroll (emergency exit)

**Bulk total:**
- 250 healing draughts x 7 mats each = 1,750 crafting materials
- 100 buff elixirs x 7 mats = 700 materials
- 150 corruption resist x 6 mats = 900 materials
- 50 void ward runes x 6 mats = 300 materials
- 50 portal scrolls x 5 mats = 250 materials
- **Total: approx 3,900 crafting materials, 600 consumable items**

**Crafter requirements:**
- 4 Alchemists (Rank 8) producing 125 potions each
- 3 Runecrafters (Rank 8) producing runes and void wards
- 2 Enchanters (Rank 8) producing scrolls
- 1 Theurgy specialist (Rank 5) for purification incense
- **10 dedicated crafters minimum** just for consumables

**Material sourcing:**
- Runic-tier crafted materials need T7 materials, which need T6, which need T5...
- Each material traces back through the gateway staircase
- The 3,900 T8 materials ultimately require 15,000+ raw/intermediate materials across 8+ crafting skills
- Monster drops needed: Storm Essence, Void Silk, Astral Fragment, Demilich Gem -- all T8 dungeon drops
- **Circular dependency:** You need T8 dungeon runs to get T8 drops to make T8 consumables for T8 dungeon runs

This circular dependency is intentional -- guilds must bootstrap by running T7 content to fund initial T8 attempts, gradually accumulating T8 drops across multiple partial clears.

### T10 Coalition Supply Operation

**Scenario:** 250-player T10 assault, 15 floors, multi-day campaign.

**Consumable estimate (conservative):**
- 3,000 Primordial Healing Draughts
- 500 Primordial Fury Elixirs (strike team DPS phases)
- 750 Primordial Corruption Wards
- 200 Primordial Purification Rites (room-wide cleanse)
- 100 Primordial Warding Arrays (passage control)
- 50 Primordial Portal Scrolls (evacuation)
- 10 Mobile Command Stations (in-dungeon crafting)
- 200 Titan Tears (boss fight full heals)
- **Approx 4,810 consumables total**

At 8-9 materials per T10 consumable, that is 40,000+ T10 crafting materials. Each of those traces back through ALL 10 gateway tiers. The total material tree is astronomical.

**Required:** A dedicated crafting guild (or crafting wing of a coalition) with 40-60 crafters across all 10 skills, operating supply chains that take weeks to prepare for a single T10 attempt.

---

## Organization Scaling

### Solo Play (T0-T2)

A single player with Alchemy rank 1-2 can self-supply:
- Craft Crude Salves and Minor Healing Potions from gathered materials
- T2 potions need 2-3 skills -- a versatile character with Alchemy + one secondary crafting skill can manage
- No external supply chain needed

### Small Party (T3-T5)

A dedicated crafter covers the party:
- One player focuses Alchemy + 1-2 secondary crafting skills
- Other party members farm materials as a side activity
- Town Portal Scrolls require an Enchanter (separate player or dual-spec)
- Party begins needing a "logistics role" distinct from combat roles

### Raid Group (T6-T8)

Dedicated crafting squad:
- 8-12 crafters with diverse specializations
- Pre-raid crafting sessions to stockpile consumables
- Material farming runs to lower-tier dungeons
- Crafter escorts during raid (portable alchemy stations at T8)
- **Crafters are as important as fighters** -- a raid without consumables is a raid without healing

### Coalition (T9-T10)

Guild-level alchemy operations:
- 40-60 crafters organized into supply teams
- Multi-week preparation for single attempts
- In-dungeon crafting stations with escort teams
- Material trade agreements between guilds
- Crafter succession plans (permanent death applies to crafters too)
- A dead master Alchemist is a strategic loss comparable to losing a raid leader

---

## Integration with Dungeon Mechanics

### Town Portal Scrolls in Dungeons

Already specified in the [Dungeons & Towers](dungeons-and-towers.md) design:
- **Channel time:** 3 seconds (T3), 2 seconds (T5), 1 second (T9), instant (T10)
- **Restrictions:** Cannot use in boss rooms, during active combat, or during simultaneous puzzles
- **Effect:** Teleport to nearest town. T9+ version can bring adjacent allies.
- **Critical use case:** Emergency evacuation when corruption is high, or extracting a crafter/specialist safely

### Corruption Management in T7+ Dungeons

The corruption/consumable interaction creates a strategic planning layer:

**Without consumables:**

| Tier | Rounds to 100% | Rotation Waves |
|------|----------------|----------------|
| T7 | 100 | 2 |
| T8 | 67 | 3 |
| T9 | 50 | 4 |
| T10 | 33 | 5+ |

**With full consumable suite:**

| Tier | Effective Rounds | Rotation Waves | Consumables per Wave |
|------|-----------------|----------------|---------------------|
| T7 | approx 140 | 1-2 | 3 per player (resist + purify) |
| T8 | approx 97 | 2 | 4 per player (resist + void ward + purify) |
| T9 | approx 75 | 2-3 | 5 per player |
| T10 | approx 51 | 3-4 | 6 per player |

Consumables halve the rotation requirement but double the logistics requirement. The organizational challenge shifts form, not magnitude.

### Mid-Dungeon Crafting at T8+

The T8+ Portable Alchemy Station and T9+ Portable Crafting Station enable in-dungeon consumable production:
- Limited uses (3-10)
- Limited tier cap (T4 for portable alchemy, T6 for portable crafting)
- Cannot produce the highest-tier consumables inside the dungeon
- **Purpose:** Replenish basic consumables during extended multi-day runs, not replace pre-raid stockpiling

### Crafting Barriers and Consumables

Existing crafting barriers (T8+) require crafted items from dungeon-only materials. Consumable crafters in the raid serve double duty:
- Produce barrier items (primary role)
- Produce emergency consumables (secondary role)
- **This makes crafter escorts the most critical logistics in T8+ content**

### Split-Path Coordination

Communication Scrolls (T6+) and Signal Flares (T5+) are designed for split-path dungeons:
- **Signal Flare:** Visible to all players on the floor -- "we are at the activation point, begin countdown"
- **Communication Scroll:** Send message to any player in the dungeon -- coordinate between split paths
- **Mass Communication Scroll (T7+):** Broadcast to all allies -- essential for T9+ 4-5 path splits
- **Coordination Beacon (T9+):** All allies see each other positions on the map -- tactical awareness for complex maneuvers

These consumables turn coordination from "have everyone on Discord" to "have enough scrolls and beacons to maintain situational awareness."

---

## Appendix: Dungeon-Only Materials

Certain T9+ and T10 consumable recipes require materials that ONLY drop inside dungeons and cannot be gathered, crafted, or traded on the open market outside of dungeon instances.

| Material | Source | Used In |
|----------|--------|---------|
| Primordial Essence | T10 monster drops (any type, inside T10 dungeons) | All T10 consumables |
| Primordial Heart | T10 boss kill (1-3 per kill, shared among raid) | Titan Tears |
| Void Shards | T8+ dungeon-specific monster drop | Void Ward Rune crafting barrier |

**Design intent:** These cannot be stockpiled from safe farming. You MUST enter high-tier dungeons to get the materials, creating the bootstrapping loop described in Economic Implications.

---

## Key Balance Invariants

These invariants must be maintained across any future balance changes. They are validated by the combat simulator.

1. **Healing potions recover 1-2 enemy hits of HP** at every tier. If a potion recovers 3+ hits, it is too strong.
2. **Solo 1v1 with 2 healing potions should improve win rate by 15-25%**, not more. If the delta exceeds 30%, the healing formula is too generous.
3. **Death rate at T8+ solo should remain above 30%** even with full consumables. Permanent death must remain a credible threat.
4. **Buff potions should give +10-20% solo win rate** at mid/high tiers. Buffs are meaningful but not fight-deciding.
5. **Consumables should NOT change party win/loss outcomes** at current content difficulty. They should reduce fight duration and casualties, not flip the result.
6. **Healing potion HP% should decline with tier**: ~50% at T1-T2, ~30-40% at T4-T6, ~15-25% at T8-T10. This forces healer reliance at high tiers.
7. **PvP consumable advantage is balanced by economic cost**, not by weakening the items. A prepared player SHOULD beat an unprepared one -- preparation is the game.
8. **Corruption resistance extends dungeon time by ~50%, never eliminates rotation.** If consumables reduce rotation waves by more than 2, they are too strong.

---

## Design Rationale

### Why healing potions do not fully heal

At high tiers, healing potions heal roughly 30-50% of typical HP. Full heals would make permanent death trivially avoidable -- just chug a potion. Partial heals create a decision: "Am I safe enough to stay, or should I retreat?" This makes death feel like a consequence of poor judgment rather than insufficient potion supply.

**Exception:** Titan Tears (T10, full heal) exists because at T10 the COST of producing one is so enormous that using it IS the meaningful decision. It takes a boss kill worth of materials to make, so burning one IS burning a boss kill.

### Why one buff + one resistance potion limit

Without limits, wealthy players could stack 5+ buff potions and become effectively invincible. The one-buff/one-resistance limit creates a CHOICE: "Do I want +5 attack or +5 AC? Fire resistance or corruption resistance?" Strategic consumable selection is more interesting than "activate everything."

### Why high-tier recipes need so many skills

Because the organizational challenge of PRODUCING consumables should parallel the challenge of USING them. If a solo crafter could mass-produce T8 healing potions, raids would never run out. By requiring 6-8 different crafting skills per recipe, production itself becomes a coordination problem. The crafter guild must cooperate as closely as the fighter guild.

### Why corruption resistance, not corruption immunity

Corruption immunity would eliminate the rotation mechanic entirely. Corruption resistance REDUCES rotation waves but does not remove them. The result: guilds must BOTH stockpile consumables AND maintain rotation reserves. Neither consumables nor pure numbers alone solve the problem -- you need both. This maximizes the organizational challenge.

### Why Town Portal Scrolls at T3

T3 is the first tier where party play becomes necessary (2-3 players) and dungeon floors are long enough (3 floors, 10 rooms each) that a failed run represents meaningful lost time. Portal scrolls let players make a risk/retreat decision rather than feeling trapped. Earlier tiers are short enough that walking out is viable. Later tiers need faster channels because combat corruption pressure is higher.

### Why crafters are as important as fighters

In most MMOs, crafters are a convenience. In RuneQuest, they are a strategic resource:
- **Consumables are destroyed on use** -- continuous demand, not one-time
- **Permanent death destroys ALL gear** -- replacement demand is real
- **High-tier recipes need specialized crafters** -- not fungible labor
- **Crafter death is strategic loss** -- a dead Rank 8 Alchemist takes months to replace
- **In-dungeon crafting barriers** -- crafters must physically enter dangerous dungeons
- This makes "protect the crafter" a genuine tactical concern, and "crafter assassination" a viable PvP strategy
