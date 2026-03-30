# Crafting Balance Analysis

> Results from the crafting graph analyzer (`cargo run -- crafting --analyze` and `cargo run -- crafting --equipment`).

## Graph Statistics

- **336 materials**, **282 recipes** (82 intermediate + 200 equipment) across T0-T10
- **Material mixing score: 2.37** (each material used by ~2.4 different skills)
- **Recipe mixing score: 2.71** (each recipe pulls from ~2.7 different source skills)
- **Gateway constraints: ALL VALID** (verified by analyzer)

## Intermediate Crafting Complexity per Tier

| Tier | Gateway Skill | Avg Recipe Steps | Growth from Previous | Crafting Skills Needed |
|---|---|---|---|---|
| T1 | Leatherworking | 1.5 | -- | 1-2 |
| T2 | Smithing | 14 | 9.3x | 3-5 |
| T3 | Woodworking | 110 | 7.9x | 5-6 |
| T4 | Alchemy | 766 | 7.0x | 6-7 |
| T5 | Enchanting | 5,309 | 6.9x | 7-8 |
| T6 | Tailoring | 37,124 | 7.0x | 8 |
| T7 | Jewelcrafting | 259,695 | 7.0x | 8 |
| T8 | Runecrafting | 1,811,392 | 7.0x | 8 |
| T9 | Artificing | 12,336,268 | 6.8x | 9 |
| T10 | Theurgy | 74,966,674 | 6.1x | 10 |

**Growth rate is remarkably consistent at ~7x per tier.** Average 6.98x, range 6.10-7.77x.

## Equipment End-to-End Costs

These are the total recipe steps to produce one complete equipment piece (weapon or armor) from T0 raw materials:

| Tier | Avg Total Steps | Min | Max | Spread |
|---|---|---|---|---|
| T1 | ~16 | ~13 | ~19 | 19% |
| T2 | ~110 | ~77 | ~143 | 30% |
| T3 | ~760 | ~640 | ~880 | <20% |
| T4 | ~5,300 | ~4,500 | ~6,100 | <20% |
| T5 | ~37,000 | ~31,000 | ~43,000 | <20% |
| T6 | ~260,000 | ~220,000 | ~300,000 | <20% |
| T7 | ~1,800,000 | ~1,500,000 | ~2,100,000 | <20% |
| T8 | ~12,600,000 | ~10,700,000 | ~14,500,000 | <20% |
| T9 | ~88,000,000 | ~74,000,000 | ~102,000,000 | <20% |
| T10 | ~625,000,000 | ~530,000,000 | ~720,000,000 | <20% |

At T10, producing a single Primordial-tier weapon from scratch requires ~625 million recipe executions across the entire supply chain. This is intentionally unreachable for solo players -- it requires a civilization of crafters working together.

## Equipment Balance Spread per Tier

The "spread" measures how much equipment lines differ in cost at the same tier. Lower = better balanced.

| Tier | Spread |
|---|---|
| T1 | 19% |
| T2 | 30% |
| T3-T10 | <20% each |

**Target: <30% spread.** All tiers pass. The gateway skill's equipment is always slightly cheaper, which is the intended "first mover" benefit.

## Cross-Equipment Skill Mixing

All 10 crafting skills feed all 10 equipment lines. No equipment line can be produced by a single crafter at T5+. Every piece requires 3-10 different crafting skills in its supply chain.

## Intermediate Balance Spread per Tier

| Tier | Spread |
|---|---|
| T1 | 67% (gateway much cheaper, expected) |
| T2 | 50% |
| T3 | 40% |
| T4 | 32% |
| T5 | 31% |
| T6-T8 | 30% |
| T9 | 31% |
| T10 | 34% |

## Monster Drop Mixing Analysis

**Target: each monster type feeds 4+ crafting skills.**

| Monster Type | Skills Using Drops | Count |
|---|---|---|
| Brute | SM, LW, WW, RC, TL, JC | 6 |
| Skulker | WW, LW, RC, AL, JC, TL | 6 |
| Mystic | EN, RC, WW, JC, AL | 5 |
| Undead | JC, TL, SM, RC, EN, AL | 6 |

All types exceed the 4+ minimum. **No siloing** -- killing any monster type produces materials useful across the entire crafting economy.

## Material Pricing (tier_to_value)

| Tier | Base GP Value | Rarity |
|---|---|---|
| T0 | 1 gp | Common |
| T1 | 5 gp | Common |
| T2 | 15 gp | Uncommon |
| T3 | 50 gp | Uncommon |
| T4 | 175 gp | Rare |
| T5 | 600 gp | Rare |
| T6 | 2,100 gp | Epic |
| T7 | 7,500 gp | Epic |
| T8 | 26,000 gp | Legendary |
| T9 | 90,000 gp | Legendary |
| T10 | 300,000 gp | Legendary |

**Weapons** sell for 3x material value. **Armor** sells for 4x material value. Shops sell pre-made equipment at 3x markup.

## Design Principles Validated

1. **Exponential complexity**: ~7x per tier, reaching 625M at T10
2. **Gateway staircase**: All constraints valid
3. **High mixing**: Material mixing 2.37, recipe mixing 2.71
4. **No siloing**: All monster types feed 5-6 skills
5. **Teamwork scaling**: T1 needs 1 skill, T10 needs all 10
6. **Equipment balance**: All tiers <30% spread

## Iteration History

1. **v1**: Initial T0-T3 recipes. JC and RC had T3 crafted inputs, causing cost explosion (118% spread)
2. **v2**: Replaced T3 crafted inputs with T2 crafted + T3 monster drops. Spread dropped to 40%. Added Brute drops to more recipes.
3. **v3**: Extended T4-T10 following established pattern. All tiers hit 30-34% spread on first pass.
4. **v4**: Added 200 equipment recipes (10 lines x 10 tiers x weapon/armor). Equipment spread <30% at all tiers.
