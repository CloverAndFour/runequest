//! Battle simulator for the RuneQuest progression system.
//!
//! Runs thousands of simulated combats using d20 mechanics identical to
//! the live combat engine, to validate tier/class/enemy-type balance.
//!
//! v3: Further rebalancing - Bard Cutting Words, Monk/Ranger buffs,
//! monster HP growth aligned with player, Brute HP->AC shift,
//! Rogue hide nerf (3->2 rounds), Bard base damage buff.
//! (v2 changes preserved: compressed HP, Mage burst, Cleric 2x heal)

use rand::Rng;
use std::fmt;

// ========================================================================
// COMBAT ARCHETYPES
// ========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Archetype {
    Valor,
    Cunning,
    Arcana,
    Divine,
    Utility,
}

impl fmt::Display for Archetype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valor => write!(f, "Valor"),
            Self::Cunning => write!(f, "Cunning"),
            Self::Arcana => write!(f, "Arcana"),
            Self::Divine => write!(f, "Divine"),
            Self::Utility => write!(f, "Utility"),
        }
    }
}

// ========================================================================
// PLAYER CLASSES
// ========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimClass {
    Warrior,
    Berserker,
    Paladin,
    Rogue,
    Ranger,
    Monk,
    Mage,
    Warlock,
    Cleric,
    Bard,
}

impl SimClass {
    pub fn archetype(self) -> Archetype {
        match self {
            Self::Warrior | Self::Berserker | Self::Paladin => Archetype::Valor,
            Self::Rogue | Self::Ranger | Self::Monk => Archetype::Cunning,
            Self::Mage | Self::Warlock => Archetype::Arcana,
            Self::Cleric => Archetype::Divine,
            Self::Bard => Archetype::Utility,
        }
    }

    pub fn all() -> &'static [SimClass] {
        use SimClass::*;
        &[Warrior, Berserker, Paladin, Rogue, Ranger, Monk, Mage, Warlock, Cleric, Bard]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Warrior => "Warrior",
            Self::Berserker => "Berserker",
            Self::Paladin => "Paladin",
            Self::Rogue => "Rogue",
            Self::Ranger => "Ranger",
            Self::Monk => "Monk",
            Self::Mage => "Mage",
            Self::Warlock => "Warlock",
            Self::Cleric => "Cleric",
            Self::Bard => "Bard",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Warrior => "WAR",
            Self::Berserker => "BER",
            Self::Paladin => "PAL",
            Self::Rogue => "ROG",
            Self::Ranger => "RAN",
            Self::Monk => "MON",
            Self::Mage => "MAG",
            Self::Warlock => "WLK",
            Self::Cleric => "CLR",
            Self::Bard => "BRD",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "warrior" | "war" => Some(Self::Warrior),
            "berserker" | "ber" => Some(Self::Berserker),
            "paladin" | "pal" => Some(Self::Paladin),
            "rogue" | "rog" => Some(Self::Rogue),
            "ranger" | "ran" => Some(Self::Ranger),
            "monk" | "mon" => Some(Self::Monk),
            "mage" | "mag" => Some(Self::Mage),
            "warlock" | "wlk" => Some(Self::Warlock),
            "cleric" | "clr" => Some(Self::Cleric),
            "bard" | "brd" => Some(Self::Bard),
            _ => None,
        }
    }
}

impl fmt::Display for SimClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ========================================================================
// ENEMY TYPES
// ========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyType {
    Brute,
    Skulker,
    Mystic,
    Undead,
}

impl EnemyType {
    pub fn archetype(self) -> Archetype {
        match self {
            Self::Brute => Archetype::Valor,
            Self::Skulker => Archetype::Cunning,
            Self::Mystic => Archetype::Arcana,
            Self::Undead => Archetype::Divine,
        }
    }

    pub fn all() -> &'static [EnemyType] {
        &[EnemyType::Brute, EnemyType::Skulker, EnemyType::Mystic, EnemyType::Undead]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Brute => "Brute",
            Self::Skulker => "Skulker",
            Self::Mystic => "Mystic",
            Self::Undead => "Undead",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "brute" => Some(Self::Brute),
            "skulker" => Some(Self::Skulker),
            "mystic" => Some(Self::Mystic),
            "undead" => Some(Self::Undead),
            _ => None,
        }
    }
}

impl fmt::Display for EnemyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ========================================================================
// TYPE ADVANTAGE SYSTEM
// ========================================================================

pub fn type_damage_mult(class: SimClass, enemy: EnemyType) -> f64 {
    if enemy == EnemyType::Undead {
        return match class.archetype() {
            Archetype::Divine => 1.50,
            Archetype::Valor if class == SimClass::Paladin => 1.20,
            _ => 0.80,
        };
    }

    let atk = class.archetype();
    let def = enemy.archetype();

    match (atk, def) {
        (Archetype::Valor, Archetype::Cunning) => 1.20,
        (Archetype::Valor, Archetype::Arcana) => 0.80,
        (Archetype::Cunning, Archetype::Arcana) => 1.20,
        (Archetype::Cunning, Archetype::Valor) => 0.80,
        (Archetype::Arcana, Archetype::Valor) => 1.20,
        (Archetype::Arcana, Archetype::Cunning) => 0.80,
        (Archetype::Divine, _) => 0.90,
        (Archetype::Utility, _) => 1.00,
        _ => 1.00,
    }
}

// ========================================================================
// STAT GENERATION - EXPONENTIAL SCALING
// ========================================================================

fn exp_scale(base: f64, growth: f64, accel: f64, tier: u32, split: u32) -> f64 {
    let t = tier.clamp(1, 10) as f64;
    let s = split as f64;
    if tier <= split {
        base * growth.powf(t - 1.0)
    } else {
        let mid = base * growth.powf(s - 1.0);
        mid * accel.powf(t - s)
    }
}

fn linear_scale(base: f64, per_tier: f64, accel_per_tier: f64, tier: u32, split: u32) -> f64 {
    let t = tier.clamp(1, 10);
    if t <= split {
        base + per_tier * (t - 1) as f64
    } else {
        let mid = base + per_tier * (split - 1) as f64;
        mid + accel_per_tier * (t - split) as f64
    }
}

// --- Player Stats (v2: compressed HP range, boosted weak classes) ---

pub fn player_max_hp(class: SimClass, tier: u32) -> i32 {
    // v2: Tightened HP range. Berserker 20→16, Mage 9→13, Bard 11→13
    let (base, growth, accel) = match class {
        SimClass::Warrior   => (15.0, 1.36, 1.46),  // was 17
        SimClass::Berserker => (16.0, 1.36, 1.46),  // was 20, huge nerf
        SimClass::Paladin   => (14.0, 1.36, 1.46),  // was 16
        SimClass::Rogue     => (12.0, 1.36, 1.46),
        SimClass::Ranger    => (13.0, 1.36, 1.46),  // was 14
        SimClass::Monk      => (13.0, 1.36, 1.46),  // v3: was 12
        SimClass::Mage      => (13.0, 1.36, 1.46),  // was 9! huge buff
        SimClass::Warlock   => (12.0, 1.36, 1.46),  // was 11
        SimClass::Cleric    => (13.0, 1.36, 1.46),
        SimClass::Bard      => (13.0, 1.36, 1.46),  // was 11
    };
    exp_scale(base, growth, accel, tier, 6).round() as i32
}

pub fn player_ac(class: SimClass, tier: u32) -> i32 {
    // v2: Mage AC 12→13 (mage armor)
    let base = match class {
        SimClass::Warrior   => 15.0,
        SimClass::Berserker => 13.0,
        SimClass::Paladin   => 15.0,
        SimClass::Rogue     => 14.0,
        SimClass::Ranger    => 14.0,
        SimClass::Monk      => 14.0,
        SimClass::Mage      => 13.0,  // was 12, mage armor
        SimClass::Warlock   => 13.0,
        SimClass::Cleric    => 14.0,
        SimClass::Bard      => 13.0,
    };
    linear_scale(base, 1.0, 1.5, tier, 6).round() as i32
}

pub fn player_to_hit(class: SimClass, tier: u32) -> i32 {
    let base = match class {
        SimClass::Warrior   => 4.0,
        SimClass::Berserker => 4.0,
        SimClass::Paladin   => 3.0,
        SimClass::Rogue     => 5.0,
        SimClass::Ranger    => 5.0,
        SimClass::Monk      => 4.0,
        SimClass::Mage      => 4.0,   // was 3, better spell accuracy
        SimClass::Warlock   => 4.0,
        SimClass::Cleric    => 3.0,
        SimClass::Bard      => 4.0,
    };
    linear_scale(base, 1.0, 1.5, tier, 6).round() as i32
}

pub fn player_damage(class: SimClass, tier: u32) -> (u32, u32, i32) {
    let is_caster = matches!(class, SimClass::Mage | SimClass::Warlock);
    let is_heavy = matches!(class, SimClass::Warrior | SimClass::Berserker | SimClass::Paladin);

    let (sides, count) = match tier {
        1..=2 => if is_caster { (8, 1) } else { (6, 1) },
        3     => if is_caster { (8, 1) } else if is_heavy { (8, 1) } else { (6, 1) },
        4..=5 => if is_caster { (10, 1) } else { (8, 1) },
        6     => (10, 1),
        7     => (10, 1),
        8     => (12, 1),
        9     => if is_caster { (10, 2) } else { (8, 2) },
        _     => if is_caster { (10, 2) } else { (10, 2) },
    };

    // v2: Bard/Cleric base damage raised from 1→2
    let base_mod: f64 = match class {
        SimClass::Berserker => 3.0,   // was 4, nerfed
        SimClass::Mage      => 2.0,
        SimClass::Cleric    => 2.0,   // was 1
        SimClass::Bard      => 3.0,   // v3: was 2
        _                   => 3.0,
    };
    let flat_mod = linear_scale(base_mod, 1.0, 1.8, tier, 6).round() as i32;

    (sides, count, flat_mod)
}

// --- Monster Stats (v2: +1 to-hit, more HP for non-Brutes) ---

pub fn monster_max_hp(enemy_type: EnemyType, tier: u32) -> i32 {
    let base_hp = exp_scale(18.0, 1.36, 1.46, tier, 6).round() as i32;
    // v2: buffed Skulker/Mystic/Undead HP multipliers
    let mult = match enemy_type {
        EnemyType::Brute   => 1.20,
        EnemyType::Skulker => 0.85,   // was 0.75
        EnemyType::Mystic  => 0.95,   // was 0.85
        EnemyType::Undead  => 1.15,   // was 1.05
    };
    (base_hp as f64 * mult).round() as i32
}

pub fn monster_ac(enemy_type: EnemyType, tier: u32) -> i32 {
    let base = match enemy_type {
        EnemyType::Brute   => 14.0,
        EnemyType::Skulker => 14.0,
        EnemyType::Mystic  => 12.0,
        EnemyType::Undead  => 12.0,
    };
    linear_scale(base, 1.0, 1.5, tier, 6).round() as i32
}

pub fn monster_to_hit(enemy_type: EnemyType, tier: u32) -> i32 {
    // v2: +1 across the board
    let base = match enemy_type {
        EnemyType::Brute   => 4.0,   // was 3
        EnemyType::Skulker => 6.0,   // was 5
        EnemyType::Mystic  => 5.0,   // was 4
        EnemyType::Undead  => 4.0,   // was 3
    };
    linear_scale(base, 1.0, 1.5, tier, 6).round() as i32
}

pub fn monster_damage(enemy_type: EnemyType, tier: u32) -> (u32, u32, i32) {
    let (sides, count) = match tier {
        1..=2 => (6, 1u32),
        3..=4 => (8, 1),
        5..=6 => (10, 1),
        7     => (10, 1),
        8     => (12, 1),
        9     => (10, 2),
        _     => (12, 2),
    };

    let base_mod: f64 = match enemy_type {
        EnemyType::Brute   => 2.0,
        EnemyType::Skulker => 4.0,
        EnemyType::Mystic  => 3.0,
        EnemyType::Undead  => 2.0,
    };
    let flat_mod = linear_scale(base_mod, 1.0, 1.8, tier, 6).round() as i32;

    (sides, count, flat_mod)
}

// ========================================================================
// COMBATANT STATE
// ========================================================================

#[derive(Debug, Clone)]
struct Combatant {
    hp: i32,
    max_hp: i32,
    ac: i32,
    to_hit: i32,
    dmg_sides: u32,
    dmg_count: u32,
    dmg_mod: i32,
    // Class abilities
    second_wind: i32,
    rage_dmg_bonus: i32,
    rage_resist: bool,
    sneak_attack_dice: u32,
    hide_rounds: i32,          // v2: limited rounds of advantage
    smite_damage: i32,
    lay_on_hands: i32,
    healing_word: i32,
    healing_word_uses: i32,    // v2: Cleric gets 2 uses
    hex_damage_dice: u32,
    arcane_burst_dice: u32,    // v2: Mage bonus d6 on every hit
    flurry_available: bool,
    inspire_bonus: i32,
    inspire_rounds: i32,
    cutting_words_penalty: i32,
    cutting_words_rounds: i32,
    ranged_bonus: i32,
}

impl Combatant {
    fn from_player(class: SimClass, tier: u32) -> Self {
        let hp = player_max_hp(class, tier);
        let ac = player_ac(class, tier);
        let to_hit = player_to_hit(class, tier);
        let (sides, count, dmg_mod) = player_damage(class, tier);
        let t = tier as i32;

        let mut c = Combatant {
            hp, max_hp: hp, ac, to_hit,
            dmg_sides: sides, dmg_count: count, dmg_mod,
            second_wind: 0, rage_dmg_bonus: 0, rage_resist: false,
            sneak_attack_dice: 0, hide_rounds: 0,
            smite_damage: 0, lay_on_hands: 0,
            healing_word: 0, healing_word_uses: 0,
            hex_damage_dice: 0, arcane_burst_dice: 0,
            flurry_available: false, inspire_bonus: 0, inspire_rounds: 0,
            cutting_words_penalty: 0, cutting_words_rounds: 0,
            ranged_bonus: 0,
        };

        match class {
            SimClass::Warrior => {
                c.second_wind = 5 + t;
            }
            SimClass::Berserker => {
                // v2: nerfed. rage_dmg was 2+t/2, now 1+t/3
                c.rage_dmg_bonus = 1 + t / 3;
                c.rage_resist = true;
            }
            SimClass::Paladin => {
                // v2: nerfed. smite was 2+t/2, now 1+t/3. LoH was 5t, now 3t.
                c.smite_damage = 1 + t / 3;
                c.lay_on_hands = 3 * t;
            }
            SimClass::Rogue => {
                c.sneak_attack_dice = ((t + 1) / 2) as u32;
                // v2: only 3 rounds of advantage (can't perma-hide in 1v1)
                c.hide_rounds = 2;
            }
            SimClass::Ranger => {
                c.ranged_bonus = 2 + t / 2;  // v3: was 1 + t/2
            }
            SimClass::Monk => {
                c.flurry_available = true;
            }
            SimClass::Mage => {
                // v2: Arcane Burst — bonus d6 on every hit (represents spell damage)
                c.arcane_burst_dice = ((t + 1) / 2) as u32;
            }
            SimClass::Warlock => {
                c.hex_damage_dice = ((t + 1) / 2) as u32;
            }
            SimClass::Cleric => {
                c.healing_word = 2 + t * 2;
                // v2: 2 uses instead of 1
                c.healing_word_uses = 2;
            }
            SimClass::Bard => {
                // v3: buffed. inspire + cutting words.
                c.inspire_bonus = 2 + t / 2;
                c.inspire_rounds = 5;
                c.cutting_words_penalty = 1 + t / 3;
                c.cutting_words_rounds = 5;
            }
        }

        c
    }

    fn from_monster(enemy_type: EnemyType, tier: u32) -> Self {
        let hp = monster_max_hp(enemy_type, tier);
        let ac = monster_ac(enemy_type, tier);
        let to_hit = monster_to_hit(enemy_type, tier);
        let (sides, count, dmg_mod) = monster_damage(enemy_type, tier);

        Combatant {
            hp, max_hp: hp, ac, to_hit,
            dmg_sides: sides, dmg_count: count, dmg_mod,
            second_wind: 0, rage_dmg_bonus: 0, rage_resist: false,
            sneak_attack_dice: 0, hide_rounds: 0,
            smite_damage: 0, lay_on_hands: 0,
            healing_word: 0, healing_word_uses: 0,
            hex_damage_dice: 0, arcane_burst_dice: 0,
            flurry_available: false, inspire_bonus: 0, inspire_rounds: 0,
            cutting_words_penalty: 0, cutting_words_rounds: 0,
            ranged_bonus: 0,
        }
    }
}

// ========================================================================
// DICE AND ATTACK ROLLS
// ========================================================================

fn roll_dice(rng: &mut impl Rng, sides: u32, count: u32) -> i32 {
    (0..count).map(|_| rng.gen_range(1..=sides) as i32).sum()
}

fn attack_roll(
    rng: &mut impl Rng,
    to_hit: i32,
    target_ac: i32,
    dmg_sides: u32,
    dmg_count: u32,
    dmg_mod: i32,
    bonus_damage: i32,
    type_mult: f64,
    advantage: bool,
) -> i32 {
    let roll1 = rng.gen_range(1..=20i32);
    let natural = if advantage {
        let roll2 = rng.gen_range(1..=20i32);
        std::cmp::max(roll1, roll2)
    } else {
        roll1
    };

    if natural == 1 {
        return 0;
    }

    let is_crit = natural == 20;
    let hits = is_crit || (natural + to_hit >= target_ac);

    if !hits {
        return 0;
    }

    let dice_count = if is_crit { dmg_count * 2 } else { dmg_count };
    let raw = roll_dice(rng, dmg_sides, dice_count) + dmg_mod + bonus_damage;
    let typed = (raw as f64 * type_mult).round() as i32;
    std::cmp::max(typed, 1)
}

// ========================================================================
// 1v1 COMBAT SIMULATION
// ========================================================================

fn simulate_1v1(class: SimClass, tier: u32, enemy_type: EnemyType, enemy_tier: u32) -> bool {
    let mut rng = rand::thread_rng();
    let mut player = Combatant::from_player(class, tier);
    let mut monster = Combatant::from_monster(enemy_type, enemy_tier);

    let type_mult = type_damage_mult(class, enemy_type);

    for _round in 0..50 {
        // === PLAYER TURN ===

        // Determine if player has advantage this round (Rogue hiding)
        let has_advantage = player.hide_rounds > 0;
        if player.hide_rounds > 0 {
            player.hide_rounds -= 1;
        }

        // Bard inspire tick
        let effective_to_hit = player.to_hit
            + if player.inspire_rounds > 0 { player.inspire_bonus } else { 0 };
        if player.inspire_rounds > 0 {
            player.inspire_rounds -= 1;
        }

        // Healing decision
        let mut used_action = false;

        // v2: Warrior second wind at 30% (was 40%)
        let heal_threshold = player.max_hp * 30 / 100;

        if player.hp <= heal_threshold {
            if player.second_wind > 0 {
                let heal = roll_dice(&mut rng, 10, 1) + (tier as i32);
                player.hp = std::cmp::min(player.hp + heal, player.max_hp);
                player.second_wind = 0;
                // Bonus action, can still attack
            } else if player.lay_on_hands > 0 {
                let need = player.max_hp - player.hp;
                let heal = std::cmp::min(player.lay_on_hands, need);
                player.hp += heal;
                player.lay_on_hands -= heal;
                used_action = true;
            }
        }

        // v2: Cleric Healing Word with multiple uses, bonus action at 50% HP
        if player.healing_word_uses > 0 && player.hp <= player.max_hp * 50 / 100 {
            let heal = roll_dice(&mut rng, 4, 1) + (tier as i32) * 2;
            player.hp = std::cmp::min(player.hp + heal, player.max_hp);
            player.healing_word_uses -= 1;
            // Bonus action, can still attack
        }

        if !used_action {
            let bonus_dmg = player.rage_dmg_bonus + player.smite_damage + player.ranged_bonus;
            let dmg = attack_roll(
                &mut rng, effective_to_hit, monster.ac,
                player.dmg_sides, player.dmg_count, player.dmg_mod,
                bonus_dmg, type_mult, has_advantage,
            );

            // Rogue sneak attack (only with advantage)
            let sneak_dmg = if dmg > 0 && player.sneak_attack_dice > 0 && has_advantage {
                (roll_dice(&mut rng, 6, player.sneak_attack_dice) as f64 * type_mult).round() as i32
            } else {
                0
            };

            // v2: Mage arcane burst (on every hit, no advantage needed)
            let arcane_dmg = if dmg > 0 && player.arcane_burst_dice > 0 {
                (roll_dice(&mut rng, 6, player.arcane_burst_dice) as f64 * type_mult).round() as i32
            } else {
                0
            };

            // Warlock hex (on every hit)
            let hex_dmg = if dmg > 0 && player.hex_damage_dice > 0 {
                (roll_dice(&mut rng, 6, player.hex_damage_dice) as f64 * type_mult).round() as i32
            } else {
                0
            };

            monster.hp -= dmg + sneak_dmg + arcane_dmg + hex_dmg;

            // Monk flurry
            if player.flurry_available && monster.hp > 0 {
                // v3: full damage modifier on flurry (was /2)
                let flurry_dmg = attack_roll(
                    &mut rng, effective_to_hit - 2, monster.ac,
                    player.dmg_sides, 1, player.dmg_mod * 2 / 3,
                    0, type_mult, false,
                );
                monster.hp -= flurry_dmg;
            }
        }

        if monster.hp <= 0 {
            return true;
        }

        // === MONSTER TURN ===
        // v3: Bard Cutting Words reduces monster effective to-hit
        let monster_effective_hit = if player.cutting_words_rounds > 0 {
            player.cutting_words_rounds -= 1;
            monster.to_hit - player.cutting_words_penalty
        } else {
            monster.to_hit
        };
        let raw_dmg = attack_roll(
            &mut rng, monster_effective_hit, player.ac,
            monster.dmg_sides, monster.dmg_count, monster.dmg_mod,
            0, 1.0, false,
        );

        // v2: Berserker rage resist now 0.90 (was 0.85)
        let actual_dmg = if player.rage_resist && raw_dmg > 0 {
            std::cmp::max((raw_dmg as f64 * 0.90).round() as i32, 1)
        } else {
            raw_dmg
        };

        player.hp -= actual_dmg;

        if player.hp <= 0 {
            return false;
        }
    }

    false
}

// ========================================================================
// PARTY COMBAT SIMULATION
// ========================================================================

#[derive(Debug, Clone, Copy)]
pub enum PartyRole {
    Tank,
    Healer,
    MeleeDps,
    RangedDps,
    Buffer,
}

#[derive(Debug, Clone)]
pub struct PartyMember {
    pub class: SimClass,
    pub tier: u32,
    pub role: PartyRole,
}

fn simulate_party(
    party: &[PartyMember],
    boss_type: EnemyType,
    boss_tier: u32,
    party_scale: f64,
) -> bool {
    let mut rng = rand::thread_rng();

    let mut members: Vec<(Combatant, SimClass, PartyRole)> = party
        .iter()
        .map(|m| (Combatant::from_player(m.class, m.tier), m.class, m.role))
        .collect();

    let mut boss = Combatant::from_monster(boss_type, boss_tier);
    boss.hp = (boss.hp as f64 * party_scale).round() as i32;
    boss.max_hp = boss.hp;

    for _round in 0..100 {
        for i in 0..members.len() {
            if members[i].0.hp <= 0 {
                continue;
            }

            let class = members[i].1;
            let role = members[i].2;
            let type_mult = type_damage_mult(class, boss_type);

            match role {
                PartyRole::Healer => {
                    let lowest = (0..members.len())
                        .filter(|&j| members[j].0.hp > 0)
                        .min_by_key(|&j| members[j].0.hp * 100 / members[j].0.max_hp);

                    if let Some(idx) = lowest {
                        let pct = members[idx].0.hp * 100 / members[idx].0.max_hp;
                        if pct < 60 {
                            let heal = roll_dice(&mut rng, 8, 1) + members[i].0.to_hit + 2;
                            let cap = members[idx].0.max_hp;
                            members[idx].0.hp = std::cmp::min(members[idx].0.hp + heal, cap);
                            continue;
                        }
                    }
                    let dmg = attack_roll(
                        &mut rng, members[i].0.to_hit, boss.ac,
                        members[i].0.dmg_sides, members[i].0.dmg_count, members[i].0.dmg_mod,
                        0, type_mult, false,
                    );
                    boss.hp -= dmg;
                }
                PartyRole::Buffer => {
                    let bonus = members[i].0.inspire_bonus;
                    let dmg = attack_roll(
                        &mut rng, members[i].0.to_hit + bonus, boss.ac,
                        members[i].0.dmg_sides, members[i].0.dmg_count, members[i].0.dmg_mod,
                        0, type_mult, false,
                    );
                    boss.hp -= dmg;
                }
                _ => {
                    let bonus = members[i].0.rage_dmg_bonus
                        + members[i].0.smite_damage
                        + members[i].0.ranged_bonus;
                    // In party, Rogue gets advantage (ally adjacent = flanking)
                    let adv = members[i].0.sneak_attack_dice > 0;
                    let dmg = attack_roll(
                        &mut rng, members[i].0.to_hit, boss.ac,
                        members[i].0.dmg_sides, members[i].0.dmg_count, members[i].0.dmg_mod,
                        bonus, type_mult, adv,
                    );

                    let sneak = if dmg > 0 && members[i].0.sneak_attack_dice > 0 && adv {
                        (roll_dice(&mut rng, 6, members[i].0.sneak_attack_dice) as f64 * type_mult)
                            .round() as i32
                    } else { 0 };

                    let arcane = if dmg > 0 && members[i].0.arcane_burst_dice > 0 {
                        (roll_dice(&mut rng, 6, members[i].0.arcane_burst_dice) as f64 * type_mult)
                            .round() as i32
                    } else { 0 };

                    let hex = if dmg > 0 && members[i].0.hex_damage_dice > 0 {
                        (roll_dice(&mut rng, 6, members[i].0.hex_damage_dice) as f64 * type_mult)
                            .round() as i32
                    } else { 0 };

                    let flurry = if members[i].0.flurry_available && boss.hp > 0 {
                        attack_roll(
                            &mut rng, members[i].0.to_hit - 2, boss.ac,
                            members[i].0.dmg_sides, 1, members[i].0.dmg_mod / 2,
                            0, type_mult, false,
                        )
                    } else { 0 };

                    boss.hp -= dmg + sneak + arcane + hex + flurry;
                }
            }

            if boss.hp <= 0 {
                return true;
            }
        }

        // Boss attacks tank (or random alive)
        let target = (0..members.len())
            .find(|&i| members[i].0.hp > 0 && matches!(members[i].2, PartyRole::Tank))
            .or_else(|| {
                let alive: Vec<usize> =
                    (0..members.len()).filter(|&i| members[i].0.hp > 0).collect();
                if alive.is_empty() { None } else { Some(alive[rng.gen_range(0..alive.len())]) }
            });

        if let Some(idx) = target {
            let dmg = attack_roll(
                &mut rng, boss.to_hit, members[idx].0.ac,
                boss.dmg_sides, boss.dmg_count, boss.dmg_mod,
                0, 1.0, false,
            );

            let actual = if members[idx].0.rage_resist && dmg > 0 {
                std::cmp::max((dmg as f64 * 0.90).round() as i32, 1)
            } else { dmg };

            members[idx].0.hp -= actual;
        }

        if members.iter().all(|(c, _, _)| c.hp <= 0) {
            return false;
        }
    }

    false
}

// ========================================================================
// TRIAL RUNNERS
// ========================================================================

pub fn run_1v1_trials(
    class: SimClass, tier: u32,
    enemy_type: EnemyType, enemy_tier: u32,
    trials: u32,
) -> f64 {
    let wins: u32 = (0..trials)
        .map(|_| if simulate_1v1(class, tier, enemy_type, enemy_tier) { 1 } else { 0 })
        .sum();
    wins as f64 / trials as f64
}

pub fn run_party_trials(
    party: &[PartyMember],
    boss_type: EnemyType,
    boss_tier: u32,
    party_scale: f64,
    trials: u32,
) -> f64 {
    let wins: u32 = (0..trials)
        .map(|_| if simulate_party(party, boss_type, boss_tier, party_scale) { 1 } else { 0 })
        .sum();
    wins as f64 / trials as f64
}

// ========================================================================
// REPORT GENERATION
// ========================================================================

pub fn sweep_report(trials: u32) -> String {
    let mut out = String::new();

    out.push_str("========================================================================\n");
    out.push_str("  RUNEQUEST BATTLE SIMULATOR v4 — FULL SWEEP (1v1, same tier)\n");
    out.push_str("========================================================================\n");
    out.push_str(&format!("  Trials per matchup: {}\n", trials));
    out.push_str("  Target: ~50% neutral, ~65-70% advantage, ~30-35% disadvantage\n\n");

    for tier in 1..=10 {
        out.push_str(&format!("--- Tier {} {}\n", tier, "-".repeat(56)));
        out.push_str("  Class    | Brute   | Skulker | Mystic  | Undead  | Avg\n");
        out.push_str("  ---------+---------+---------+---------+---------+------\n");

        for &class in SimClass::all() {
            let mut rates = Vec::new();
            let mut row = format!("  {:<8} |", class.short());

            for &etype in EnemyType::all() {
                let rate = run_1v1_trials(class, tier, etype, tier, trials);
                let pct = (rate * 100.0).round() as i32;
                rates.push(rate);

                let indicator = match pct {
                    0..=35 => "!",
                    36..=44 => "-",
                    45..=55 => " ",
                    56..=69 => "+",
                    _ => "!",
                };
                row.push_str(&format!(" {:>3}%{:<1}  |", pct, indicator));
            }

            let avg = rates.iter().sum::<f64>() / rates.len() as f64;
            row.push_str(&format!(" {:>3}%", (avg * 100.0).round() as i32));

            out.push_str(&row);
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("Legend: (space)=on target  +=above target  -=below target  !=far off\n");
    out
}

pub fn class_report(class: SimClass, trials: u32) -> String {
    let mut out = String::new();

    out.push_str("========================================================================\n");
    out.push_str(&format!("  {} ({}) — 1v1 Win Rates by Tier\n", class.name(), class.archetype()));
    out.push_str("========================================================================\n");
    out.push_str(&format!("  Trials per matchup: {}\n\n", trials));
    out.push_str("  Tier | Brute   | Skulker | Mystic  | Undead  | Avg\n");
    out.push_str("  -----+---------+---------+---------+---------+------\n");

    for tier in 1..=10 {
        let mut rates = Vec::new();
        let mut row = format!("  T{:<3} |", tier);

        for &etype in EnemyType::all() {
            let rate = run_1v1_trials(class, tier, etype, tier, trials);
            let pct = (rate * 100.0).round() as i32;
            rates.push(rate);
            let indicator = match pct {
                0..=35 => "▼",
                36..=44 => "↓",
                45..=55 => "•",
                56..=64 => "↑",
                _ => "▲",
            };
            row.push_str(&format!(" {:>3}% {} |", pct, indicator));
        }

        let avg = rates.iter().sum::<f64>() / rates.len() as f64;
        row.push_str(&format!(" {:>3}%", (avg * 100.0).round() as i32));
        out.push_str(&row);
        out.push('\n');
    }

    out.push_str("\n  ▲ strong (65%+)  ↑ advantage (56-64%)  • neutral (45-55%)\n");
    out.push_str("  ↓ weak (36-44%)  ▼ very weak (<36%)\n");

    out.push_str(&format!("\n--- Cross-Tier ({}): T vs T-1 and T vs T+1 (Brute) ---\n", class.short()));
    out.push_str("  Tier | vs T-1  | vs T+1\n");
    out.push_str("  -----+---------+--------\n");
    for tier in 1..=10 {
        let vs_lower = if tier > 1 {
            let r = run_1v1_trials(class, tier, EnemyType::Brute, tier - 1, trials);
            format!("{:>3}%", (r * 100.0).round() as i32)
        } else {
            " N/A".to_string()
        };
        let vs_higher = if tier < 10 {
            let r = run_1v1_trials(class, tier, EnemyType::Brute, tier + 1, trials);
            format!("{:>3}%", (r * 100.0).round() as i32)
        } else {
            " N/A".to_string()
        };
        out.push_str(&format!("  T{:<3} |  {}   |  {}\n", tier, vs_lower, vs_higher));
    }

    out
}

pub fn matchup_detail(
    class: SimClass, tier: u32,
    enemy_type: EnemyType, enemy_tier: u32,
    trials: u32,
) -> String {
    let mut out = String::new();

    let hp = player_max_hp(class, tier);
    let ac = player_ac(class, tier);
    let hit = player_to_hit(class, tier);
    let (ds, dc, dm) = player_damage(class, tier);

    let mhp = monster_max_hp(enemy_type, enemy_tier);
    let mac = monster_ac(enemy_type, enemy_tier);
    let mhit = monster_to_hit(enemy_type, enemy_tier);
    let (mds, mdc, mdm) = monster_damage(enemy_type, enemy_tier);

    let tmult = type_damage_mult(class, enemy_type);
    let win_rate = run_1v1_trials(class, tier, enemy_type, enemy_tier, trials);

    out.push_str(&format!("=== {} T{} vs {} T{} ===\n\n", class.name(), tier, enemy_type.name(), enemy_tier));
    out.push_str(&format!("Player ({}, {}):\n", class.name(), class.archetype()));
    out.push_str(&format!("  HP: {}  AC: {}  To-Hit: +{}  Damage: {}d{}+{}\n", hp, ac, hit, dc, ds, dm));
    out.push_str(&format!("\nMonster ({}, {}):\n", enemy_type.name(), enemy_type.archetype()));
    out.push_str(&format!("  HP: {}  AC: {}  To-Hit: +{}  Damage: {}d{}+{}\n", mhp, mac, mhit, mdc, mds, mdm));
    out.push_str(&format!("\nType multiplier: {:.2}x\n", tmult));

    let p_hit = ((21 - (mac - hit)).clamp(2, 20) as f64) / 20.0;
    let m_hit = ((21 - (ac - mhit)).clamp(2, 20) as f64) / 20.0;
    let p_avg = (dc as f64 * (ds as f64 + 1.0) / 2.0 + dm as f64) * tmult;
    let m_avg = mdc as f64 * (mds as f64 + 1.0) / 2.0 + mdm as f64;
    let p_dpr = p_hit * p_avg;
    let m_dpr = m_hit * m_avg;

    out.push_str("\nTheoretical (base attack only, no abilities):\n");
    out.push_str(&format!("  Player: {:.0}% hit, {:.1} avg dmg, {:.1} DPR, kills in ~{:.1} rounds\n",
        p_hit * 100.0, p_avg, p_dpr, mhp as f64 / p_dpr));
    out.push_str(&format!("  Monster: {:.0}% hit, {:.1} avg dmg, {:.1} DPR, kills in ~{:.1} rounds\n",
        m_hit * 100.0, m_avg, m_dpr, hp as f64 / m_dpr));
    out.push_str(&format!("\nSimulation ({} trials): {:.1}% player win rate\n", trials, win_rate * 100.0));

    let target = if tmult > 1.1 { "65-70% (type advantage)" }
        else if tmult < 0.9 { "30-35% (type disadvantage)" }
        else { "~50% (neutral)" };
    out.push_str(&format!("Target: {}\n", target));

    out
}

pub fn stat_table_report() -> String {
    let mut out = String::new();

    out.push_str("========================================================================\n");
    out.push_str("  CHARACTER & MONSTER STAT TABLES (v3)\n");
    out.push_str("========================================================================\n\n");

    for &class in SimClass::all() {
        out.push_str(&format!("--- {} ({}) ---\n", class.name(), class.archetype()));
        out.push_str("  Tier |  HP  | AC | Hit | Damage      | Special\n");
        out.push_str("  -----+------+----+-----+-------------+-----------\n");

        for tier in 1..=10 {
            let hp = player_max_hp(class, tier);
            let ac = player_ac(class, tier);
            let hit = player_to_hit(class, tier);
            let (ds, dc, dm) = player_damage(class, tier);
            let dmg_str = format!("{}d{}+{}", dc, ds, dm);

            let special = match class {
                SimClass::Warrior => format!("SW:d10+{}", tier),
                SimClass::Berserker => format!("Rage+{} 90%", 1 + tier as i32 / 3),
                SimClass::Paladin => format!("Smite+{} LoH:{}", 1 + tier as i32 / 3, 3 * tier),
                SimClass::Rogue => format!("SA:{}d6 2rnd", (tier as i32 + 1) / 2),
                SimClass::Ranger => format!("+{} ranged", 2 + tier as i32 / 2),
                SimClass::Monk => "Flurry(-2)".to_string(),
                SimClass::Mage => format!("Burst:{}d6/hit", (tier as i32 + 1) / 2),
                SimClass::Warlock => format!("Hex:{}d6/hit", (tier as i32 + 1) / 2),
                SimClass::Cleric => format!("HW:d4+{}x2", tier as i32 * 2),
                SimClass::Bard => format!("Insp+{} CW-{} 5rnd", 2 + tier as i32 / 2, 1 + tier as i32 / 3),
            };

            out.push_str(&format!("  T{:<3} | {:>4} | {:>2} | +{:<2} | {:<11} | {}\n",
                tier, hp, ac, hit, dmg_str, special));
        }
        out.push('\n');
    }

    out.push_str("--- MONSTERS ---\n");
    for &etype in EnemyType::all() {
        out.push_str(&format!("\n{} ({}):\n", etype.name(), etype.archetype()));
        out.push_str("  Tier |  HP  | AC | Hit | Damage\n");
        out.push_str("  -----+------+----+-----+---------\n");
        for tier in 1..=10 {
            let hp = monster_max_hp(etype, tier);
            let ac = monster_ac(etype, tier);
            let hit = monster_to_hit(etype, tier);
            let (ds, dc, dm) = monster_damage(etype, tier);
            out.push_str(&format!("  T{:<3} | {:>4} | {:>2} | +{:<2} | {}d{}+{}\n",
                tier, hp, ac, hit, dc, ds, dm));
        }
    }

    out
}

pub fn default_party(tier: u32) -> Vec<PartyMember> {
    vec![
        PartyMember { class: SimClass::Warrior, tier, role: PartyRole::Tank },
        PartyMember { class: SimClass::Cleric, tier, role: PartyRole::Healer },
        PartyMember { class: SimClass::Mage, tier, role: PartyRole::RangedDps },
        PartyMember { class: SimClass::Rogue, tier, role: PartyRole::MeleeDps },
        PartyMember { class: SimClass::Bard, tier, role: PartyRole::Buffer },
    ]
}

pub fn party_report(trials: u32) -> String {
    let mut out = String::new();

    out.push_str("========================================================================\n");
    out.push_str("  PARTY BALANCE (5-person: WAR+CLR+MAG+ROG+BRD vs Boss)\n");
    out.push_str("========================================================================\n");
    out.push_str(&format!("  Trials: {}  Boss HP scale: 5.0x\n\n", trials));
    out.push_str("  Tier | Brute  | Skulker | Mystic | Undead | Avg\n");
    out.push_str("  -----+--------+---------+--------+--------+------\n");

    for tier in 1..=10 {
        let party = default_party(tier);
        let mut rates = Vec::new();
        let mut row = format!("  T{:<3} |", tier);

        for &etype in EnemyType::all() {
            let rate = run_party_trials(&party, etype, tier, 5.0, trials);
            let pct = (rate * 100.0).round() as i32;
            rates.push(rate);
            row.push_str(&format!(" {:>3}%  |", pct));
        }

        let avg = rates.iter().sum::<f64>() / rates.len() as f64;
        row.push_str(&format!(" {:>3}%", (avg * 100.0).round() as i32));
        out.push_str(&row);
        out.push('\n');
    }

    out
}

pub fn parse_party(spec: &str) -> Option<Vec<PartyMember>> {
    let mut members = Vec::new();
    for part in spec.split(',') {
        let parts: Vec<&str> = part.trim().split(':').collect();
        if parts.len() != 2 { return None; }
        let class = SimClass::from_str(parts[0])?;
        let tier: u32 = parts[1].parse().ok()?;
        let role = match class {
            SimClass::Warrior | SimClass::Paladin => PartyRole::Tank,
            SimClass::Cleric => PartyRole::Healer,
            SimClass::Bard => PartyRole::Buffer,
            SimClass::Ranger | SimClass::Mage | SimClass::Warlock => PartyRole::RangedDps,
            _ => PartyRole::MeleeDps,
        };
        members.push(PartyMember { class, tier, role });
    }
    if members.is_empty() { None } else { Some(members) }
}

pub fn parse_boss(spec: &str) -> Option<(EnemyType, u32)> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 2 { return None; }
    let etype = EnemyType::from_str(parts[0])?;
    let tier: u32 = parts[1].parse().ok()?;
    Some((etype, tier))
}
