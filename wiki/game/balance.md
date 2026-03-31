# Balance Calculations

> Results from the battle simulator (v4, 4 iterations, 3000+ trials per matchup).

## Simulator Architecture

Module: `src/engine/simulator.rs`

The simulator uses the same d20 mechanics as the live combat engine:
- d20 + to_hit vs AC
- Natural 20 = critical (double dice)
- Natural 1 = auto-miss
- Type advantage multipliers applied to damage
- Class-specific abilities (Second Wind, Rage, Sneak Attack, etc.)

### Running Simulations

```bash
cargo run -- simulate --sweep --trials 3000     # Full class × enemy matrix
cargo run -- simulate --class warrior            # Single class report
cargo run -- simulate --stats                    # Stat tables
cargo run -- simulate --party-report             # 5-person party balance
```

## Player Stat Curves (from simulator v4)

### HP Scaling (exponential, split at T6)

Formula: `base × growth^(tier-1)` for T1-6, then `mid × accel^(tier-6)` for T7-10

| Class | Base HP | Growth (T1-6) | Accel (T7-10) |
|---|---|---|---|
| Warrior | 15 | 1.36 | 1.46 |
| Berserker | 16 | 1.36 | 1.46 |
| Paladin | 14 | 1.36 | 1.46 |
| Rogue | 12 | 1.36 | 1.46 |
| Ranger | 13 | 1.36 | 1.46 |
| Monk | 13 | 1.36 | 1.46 |
| Mage | 13 | 1.36 | 1.46 |
| Warlock | 12 | 1.36 | 1.46 |
| Cleric | 13 | 1.36 | 1.46 |
| Bard | 13 | 1.36 | 1.46 |

### AC Scaling (linear with acceleration)

Formula: `base + 1/tier` for T1-6, `base + 5 + 1.5/tier` for T7-10

| Class | Base AC |
|---|---|
| Warrior, Paladin | 15 |
| Rogue, Ranger, Monk, Cleric | 14 |
| Berserker, Warlock, Bard, Mage | 13 |

### To-Hit Scaling

Same formula as AC. Bases: ROG/RAN 5, WAR/BER/MON/WLK/BRD/MAG 4, PAL/CLR 3.

### Type Multipliers (validated v4)

- Advantage: **1.20x** (was 1.25 in v1-v3, reduced for balance)
- Disadvantage: **0.80x** (was 0.75, raised for balance)
- Divine vs Undead: **1.50x** (unchanged)
- Divine vs non-Undead: **0.90x** (intentional weakness)

## 1v1 Win Rate Targets

| Matchup | Target | Actual (v4) |
|---|---|---|
| Same tier, neutral type | ~50% | 50-60% |
| Same tier, type advantage | 60-70% | 60-70% |
| Same tier, type disadvantage | 35-40% | 35-45% |
| Cleric vs Undead | 70-80% | 76-90% |
| Cleric vs non-Undead | ~40% | 35-45% |
| Bard vs any | ~45% | 42-51% |
| +1 tier advantage | ~90% | 90-100% |
| -1 tier disadvantage | ~10% | 0-22% |

### Design rationale for Bard weakness
Bard is intentionally ~5% below average in combat. Their value comes from NPC negotiation, social encounters, lore, charm, and party utility (Inspire buffs the entire group in team combat). In 1v1, they're the weakest combatant. In a raid, they're essential.

## Solo Difficulty Curve

| Tier | 1v1 Win Rate | Assessment |
|---|---|---|
| T1-4 | 50-65% | Comfortable solo |
| T5-6 | 45-55% | Challenging solo |
| T7 | 40-50% | Hard solo, party recommended |
| T8 | 30-45% | Very hard solo |
| T9 | 20-35% | Nearly impossible solo |
| T10 | 15-30% | Requires party/raid |

**This is by design.** The MMORPG direction means solo play caps at ~T5-6. Higher tiers require teamwork and coordination.

## Cross-Tier Power

Each tier is a massive power jump:
- Player T(N) vs Monster T(N-1): **90-100% win rate**
- Player T(N) vs Monster T(N+1): **0-22% win rate**

One tier of equipment/skill difference is devastating. This validates the exponential growth design.

## Party Balance

Standard 5-person party (WAR+CLR+MAG+ROG+BRD) at 5x boss HP scaling:
- **100% win rate** across all tiers — boss HP needs 15-20x scaling for meaningful challenge
- Party combat simulation is a v2 priority

## Iteration History

| Version | Key Change | Outcome |
|---|---|---|
| v1 | Initial stat curves | Berserker 90%+, Mage 10% — massive imbalance |
| v2 | Compressed HP range, added Mage Arcane Burst, Bard Cutting Words | Mage fixed, Monk OP, Rogue still high |
| v3 | Nerfed Rogue hide rounds (3→2), buffed Ranger/Monk, matched monster growth to player | Monk still OP, type spread too wide |
| v4 | Reduced type multipliers (1.25→1.20), nerfed Monk flurry | **Balanced.** All classes 45-65% avg |


## Consumable Impact (v5)

> Calibrated via simulator v5. Target: 15-25% win rate improvement with full consumable loadout.

### Healing Potions

- **Formula:** `2 * tier + 5` HP
- **Cooldown:** 5 rounds between uses
- **Design intent:** Buys ~1 extra round of survival. Not enough to replace a dedicated healer at T6+.

### Buff Potions

- **To-hit bonus:** `1 + floor(tier / 3)`
- **Duration:** `3 + floor(tier / 2)` rounds
- **Design intent:** Meaningful accuracy boost without stacking to guaranteed hits.

### Alchemical Weapons

- **Damage (after saves):** `tier + 1` effective damage per use
- **Mechanic:** AoE, no to-hit roll, DEX save for half
- **Design intent:** Reliable chip damage. Strong in multi-target encounters, weak vs single bosses.

### Win Rate Impact (Full Loadout)

| Tier | No Consumables | Full Loadout | Improvement |
|------|----------------|--------------|-------------|
| T1-T3 | 50-60% | 65-80% | +15-20% |
| T4-T5 | 45-55% | 60-75% | +15-20% |
| T6-T7 | 40-50% | 55-70% | +15-20% |
| T8+ | 30-45% | 50-65% | +20-25% |

**Key finding:** Consumables cannot replace missing gear at T5+. A player missing one armor slot loses more AC than potions can compensate for. This makes the crafting supply chain essential, not optional.

**Balance lever:** The 5-round healing cooldown is the primary knob. At 3 rounds (old value), potions gave +30-40% win rate which trivialized mid-tier content. At 5 rounds, players must choose when to heal carefully.