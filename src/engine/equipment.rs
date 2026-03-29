//! Equipment slot system with item database.

use serde::{Deserialize, Serialize};

use super::inventory::Item;

/// Equipment slots for the character.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EquipSlot {
    Head,
    Amulet,
    MainHand,
    OffHand,
    Chest,
    Hands,
    Ring,
    Legs,
    Feet,
    Back,
}

impl EquipSlot {
    pub fn display_name(&self) -> &'static str {
        match self {
            EquipSlot::Head => "Head",
            EquipSlot::Amulet => "Amulet",
            EquipSlot::MainHand => "Main Hand",
            EquipSlot::OffHand => "Off Hand",
            EquipSlot::Chest => "Chest",
            EquipSlot::Hands => "Hands",
            EquipSlot::Ring => "Ring",
            EquipSlot::Legs => "Legs",
            EquipSlot::Feet => "Feet",
            EquipSlot::Back => "Back",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "head" => Some(EquipSlot::Head),
            "amulet" => Some(EquipSlot::Amulet),
            "main_hand" | "mainhand" | "main hand" => Some(EquipSlot::MainHand),
            "off_hand" | "offhand" | "off hand" => Some(EquipSlot::OffHand),
            "chest" => Some(EquipSlot::Chest),
            "hands" => Some(EquipSlot::Hands),
            "ring" | "ring1" | "ring2" => Some(EquipSlot::Ring),
            "legs" => Some(EquipSlot::Legs),
            "feet" => Some(EquipSlot::Feet),
            "back" => Some(EquipSlot::Back),
            _ => None,
        }
    }
}

impl std::fmt::Display for EquipSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Item rarity tiers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Default for Rarity {
    fn default() -> Self {
        Rarity::Common
    }
}

impl std::fmt::Display for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rarity::Common => write!(f, "Common"),
            Rarity::Uncommon => write!(f, "Uncommon"),
            Rarity::Rare => write!(f, "Rare"),
            Rarity::Epic => write!(f, "Epic"),
            Rarity::Legendary => write!(f, "Legendary"),
        }
    }
}

/// Magical enchantment on an item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enchantment {
    pub bonus: i32,
    pub element: Option<String>,
    pub name_prefix: String,
}

/// Stat block for an item — covers weapons, armor, and accessories.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ItemStats {
    pub ac_bonus: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ac_base: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage_dice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage_modifier_stat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    pub attack_bonus: i32,
    pub str_bonus: i32,
    pub dex_bonus: i32,
    pub con_bonus: i32,
    pub int_bonus: i32,
    pub wis_bonus: i32,
    pub cha_bonus: i32,
    pub hp_bonus: i32,
    pub speed_bonus: i32,
    pub is_two_handed: bool,
    pub is_finesse: bool,
    pub is_ranged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special: Option<String>,
}

/// Aggregate stat bonuses from all equipment.
#[derive(Debug, Clone, Default)]
pub struct StatBonuses {
    pub str_bonus: i32,
    pub dex_bonus: i32,
    pub con_bonus: i32,
    pub int_bonus: i32,
    pub wis_bonus: i32,
    pub cha_bonus: i32,
    pub hp_bonus: i32,
    pub speed_bonus: i32,
    pub attack_bonus: i32,
}

/// The character's equipped items across all slots.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Equipment {
    pub head: Option<Item>,
    pub amulet: Option<Item>,
    pub main_hand: Option<Item>,
    pub off_hand: Option<Item>,
    pub chest: Option<Item>,
    pub hands: Option<Item>,
    pub ring1: Option<Item>,
    pub ring2: Option<Item>,
    pub legs: Option<Item>,
    pub feet: Option<Item>,
    pub back: Option<Item>,
}

impl Equipment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a mutable reference to the slot.
    fn slot_mut(&mut self, slot: &EquipSlot) -> &mut Option<Item> {
        match slot {
            EquipSlot::Head => &mut self.head,
            EquipSlot::Amulet => &mut self.amulet,
            EquipSlot::MainHand => &mut self.main_hand,
            EquipSlot::OffHand => &mut self.off_hand,
            EquipSlot::Chest => &mut self.chest,
            EquipSlot::Hands => &mut self.hands,
            EquipSlot::Ring => &mut self.ring1,
            EquipSlot::Legs => &mut self.legs,
            EquipSlot::Feet => &mut self.feet,
            EquipSlot::Back => &mut self.back,
        }
    }

    /// Equip an item to its designated slot. If the slot is occupied, the displaced
    /// item is returned. For rings, tries ring1 first, then ring2.
    pub fn equip(&mut self, item: Item) -> std::result::Result<Option<Item>, String> {
        let slot = match &item.slot {
            Some(s) => *s,
            None => return Err(format!("'{}' cannot be equipped (no slot).", item.name)),
        };

        // Special ring handling: try ring1 first, then ring2
        if slot == EquipSlot::Ring {
            if self.ring1.is_none() {
                self.ring1 = Some(item);
                return Ok(None);
            } else if self.ring2.is_none() {
                self.ring2 = Some(item);
                return Ok(None);
            } else {
                // Both full — displace ring1
                let displaced = self.ring1.take();
                self.ring1 = Some(item);
                return Ok(displaced);
            }
        }

        // Two-handed weapon: also clear off-hand
        let mut extra_displaced: Option<Item> = None;
        if slot == EquipSlot::MainHand && item.stats.is_two_handed {
            extra_displaced = self.off_hand.take();
        }

        let target = self.slot_mut(&slot);
        let displaced = target.take();
        *target = Some(item);

        // If there's an extra displaced item from two-handed, we return the main displaced
        // and the caller should handle the extra. For simplicity, we only return one.
        // In practice the caller can check equipment state.
        if displaced.is_some() {
            Ok(displaced)
        } else {
            Ok(extra_displaced)
        }
    }

    /// Unequip an item from a slot, returning it.
    pub fn unequip(&mut self, slot: &EquipSlot) -> Option<Item> {
        // Special ring handling for unequip: try ring1, then ring2
        if *slot == EquipSlot::Ring {
            if self.ring1.is_some() {
                return self.ring1.take();
            } else {
                return self.ring2.take();
            }
        }
        self.slot_mut(slot).take()
    }

    /// Unequip ring from a specific ring slot (1 or 2).
    pub fn unequip_ring(&mut self, ring_num: u8) -> Option<Item> {
        match ring_num {
            1 => self.ring1.take(),
            2 => self.ring2.take(),
            _ => None,
        }
    }

    /// Total AC bonus from all equipped items EXCEPT chest armor (which provides base AC).
    pub fn total_ac_bonus(&self) -> i32 {
        let mut bonus = 0;
        let slots: [&Option<Item>; 10] = [
            &self.head,
            &self.amulet,
            &self.main_hand,
            &self.off_hand,
            &self.hands,
            &self.ring1,
            &self.ring2,
            &self.legs,
            &self.feet,
            &self.back,
        ];
        for slot in &slots {
            if let Some(item) = slot {
                bonus += item.stats.ac_bonus;
                if let Some(ref ench) = item.enchantment {
                    bonus += ench.bonus;
                }
            }
        }
        // Chest enchantment bonus (not ac_bonus, which is part of base)
        if let Some(ref chest) = self.chest {
            if let Some(ref ench) = chest.enchantment {
                bonus += ench.bonus;
            }
        }
        bonus
    }

    /// Aggregate stat bonuses from all equipped items.
    pub fn stat_bonuses(&self) -> StatBonuses {
        let mut b = StatBonuses::default();
        let all_items = self.all_equipped();
        for item in &all_items {
            b.str_bonus += item.stats.str_bonus;
            b.dex_bonus += item.stats.dex_bonus;
            b.con_bonus += item.stats.con_bonus;
            b.int_bonus += item.stats.int_bonus;
            b.wis_bonus += item.stats.wis_bonus;
            b.cha_bonus += item.stats.cha_bonus;
            b.hp_bonus += item.stats.hp_bonus;
            b.speed_bonus += item.stats.speed_bonus;
            b.attack_bonus += item.stats.attack_bonus;
        }
        b
    }

    /// Get equipped weapon from main hand (if it's a weapon).
    pub fn equipped_weapon(&self) -> Option<&Item> {
        self.main_hand.as_ref().filter(|item| {
            item.item_type == super::inventory::ItemType::Weapon
        })
    }

    /// Get all equipped items as a list of references.
    pub fn all_equipped(&self) -> Vec<&Item> {
        let slots: [&Option<Item>; 11] = [
            &self.head,
            &self.amulet,
            &self.main_hand,
            &self.off_hand,
            &self.chest,
            &self.hands,
            &self.ring1,
            &self.ring2,
            &self.legs,
            &self.feet,
            &self.back,
        ];
        slots.iter().filter_map(|s| s.as_ref()).collect()
    }

    /// Find an equipped item by name (case-insensitive).
    pub fn find_by_name(&self, name: &str) -> Option<(&Item, EquipSlot)> {
        let name_lower = name.to_lowercase();
        let slots_with_items: Vec<(&Option<Item>, EquipSlot)> = vec![
            (&self.head, EquipSlot::Head),
            (&self.amulet, EquipSlot::Amulet),
            (&self.main_hand, EquipSlot::MainHand),
            (&self.off_hand, EquipSlot::OffHand),
            (&self.chest, EquipSlot::Chest),
            (&self.hands, EquipSlot::Hands),
            (&self.ring1, EquipSlot::Ring),
            (&self.ring2, EquipSlot::Ring),
            (&self.legs, EquipSlot::Legs),
            (&self.feet, EquipSlot::Feet),
            (&self.back, EquipSlot::Back),
        ];
        for (opt_item, slot) in slots_with_items {
            if let Some(item) = opt_item {
                if item.name.to_lowercase() == name_lower || item.display_name().to_lowercase() == name_lower {
                    return Some((item, slot));
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Item database
// ---------------------------------------------------------------------------

use super::inventory::ItemType;

/// Look up a standard item from the built-in database by ID.
pub fn get_item(id: &str) -> Option<Item> {
    let items = item_database();
    items.into_iter().find(|i| i.id == id)
}

/// The full item database.
fn item_database() -> Vec<Item> {
    vec![
        // ===== WEAPONS =====
        // -- Simple Melee --
        Item {
            id: "club".into(),
            name: "Club".into(),
            description: "A simple wooden club.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 2.0,
            value_gp: 1,
            stats: ItemStats {
                damage_dice: Some("1d4".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("bludgeoning".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "dagger".into(),
            name: "Dagger".into(),
            description: "A simple dagger, light and easy to conceal.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 1.0,
            value_gp: 2,
            stats: ItemStats {
                damage_dice: Some("1d4".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("piercing".into()),
                is_finesse: true,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "handaxe".into(),
            name: "Handaxe".into(),
            description: "A small axe suitable for throwing or melee.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 2.0,
            value_gp: 5,
            stats: ItemStats {
                damage_dice: Some("1d6".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "mace".into(),
            name: "Mace".into(),
            description: "A heavy mace blessed by your deity.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 4.0,
            value_gp: 5,
            stats: ItemStats {
                damage_dice: Some("1d6".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("bludgeoning".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "quarterstaff".into(),
            name: "Quarterstaff".into(),
            description: "A wooden staff, useful for both walking and combat.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 4.0,
            value_gp: 2,
            stats: ItemStats {
                damage_dice: Some("1d6".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("bludgeoning".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "spear".into(),
            name: "Spear".into(),
            description: "A simple spear with a steel point.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 3.0,
            value_gp: 1,
            stats: ItemStats {
                damage_dice: Some("1d6".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("piercing".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Martial Melee --
        Item {
            id: "shortsword".into(),
            name: "Shortsword".into(),
            description: "A light, quick blade favored by rogues.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 2.0,
            value_gp: 10,
            stats: ItemStats {
                damage_dice: Some("1d6".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("piercing".into()),
                is_finesse: true,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "longsword".into(),
            name: "Longsword".into(),
            description: "A sturdy steel longsword.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 3.0,
            value_gp: 15,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "battleaxe".into(),
            name: "Battleaxe".into(),
            description: "A heavy single-edged axe designed for war.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 4.0,
            value_gp: 10,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "warhammer".into(),
            name: "Warhammer".into(),
            description: "A heavy hammer built for crushing armor.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 2.0,
            value_gp: 15,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("bludgeoning".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "rapier".into(),
            name: "Rapier".into(),
            description: "An elegant thrusting sword favored by duelists.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 2.0,
            value_gp: 25,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("piercing".into()),
                is_finesse: true,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "scimitar".into(),
            name: "Scimitar".into(),
            description: "A curved blade, swift and deadly.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 3.0,
            value_gp: 25,
            stats: ItemStats {
                damage_dice: Some("1d6".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("slashing".into()),
                is_finesse: true,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "greatsword".into(),
            name: "Greatsword".into(),
            description: "A massive two-handed blade of forged steel.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 6.0,
            value_gp: 50,
            stats: ItemStats {
                damage_dice: Some("2d6".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing".into()),
                is_two_handed: true,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "greataxe".into(),
            name: "Greataxe".into(),
            description: "An enormous axe requiring two hands to wield.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 7.0,
            value_gp: 30,
            stats: ItemStats {
                damage_dice: Some("1d12".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing".into()),
                is_two_handed: true,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "maul".into(),
            name: "Maul".into(),
            description: "A massive two-handed hammer.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 10.0,
            value_gp: 10,
            stats: ItemStats {
                damage_dice: Some("2d6".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("bludgeoning".into()),
                is_two_handed: true,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Ranged --
        Item {
            id: "shortbow".into(),
            name: "Shortbow".into(),
            description: "A compact bow for quick shots.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 2.0,
            value_gp: 25,
            stats: ItemStats {
                damage_dice: Some("1d6".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("piercing".into()),
                is_ranged: true,
                is_two_handed: true,
                range: Some("80/320".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "longbow".into(),
            name: "Longbow".into(),
            description: "A tall bow crafted from yew wood.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 2.0,
            value_gp: 50,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("piercing".into()),
                is_ranged: true,
                is_two_handed: true,
                range: Some("150/600".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "light_crossbow".into(),
            name: "Light Crossbow".into(),
            description: "A small crossbow, easy to use.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 5.0,
            value_gp: 25,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("piercing".into()),
                is_ranged: true,
                is_two_handed: true,
                range: Some("80/320".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "heavy_crossbow".into(),
            name: "Heavy Crossbow".into(),
            description: "A powerful crossbow that packs a punch.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Common,
            weight: 18.0,
            value_gp: 50,
            stats: ItemStats {
                damage_dice: Some("1d10".into()),
                damage_modifier_stat: Some("dex".into()),
                damage_type: Some("piercing".into()),
                is_ranged: true,
                is_two_handed: true,
                range: Some("100/400".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // ===== ARMOR =====
        // -- Light Armor --
        Item {
            id: "padded_armor".into(),
            name: "Padded Armor".into(),
            description: "Quilted layers of cloth and batting.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 8.0,
            value_gp: 5,
            stats: ItemStats {
                ac_base: Some(11), // 11 + DEX
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "leather_armor".into(),
            name: "Leather Armor".into(),
            description: "Supple leather molded to fit the body.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 10.0,
            value_gp: 10,
            stats: ItemStats {
                ac_base: Some(11), // 11 + DEX
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "studded_leather".into(),
            name: "Studded Leather".into(),
            description: "Tough leather reinforced with metal rivets.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 13.0,
            value_gp: 45,
            stats: ItemStats {
                ac_base: Some(12), // 12 + DEX
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Medium Armor --
        Item {
            id: "hide_armor".into(),
            name: "Hide Armor".into(),
            description: "Crude armor made from thick animal hides.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 12.0,
            value_gp: 10,
            stats: ItemStats {
                ac_base: Some(12), // 12 + DEX (max 2)
                special: Some("dex_cap_2".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "chain_shirt".into(),
            name: "Chain Shirt".into(),
            description: "A shirt of interlocking metal rings, lighter than full chain.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 20.0,
            value_gp: 50,
            stats: ItemStats {
                ac_base: Some(13), // 13 + DEX (max 2)
                special: Some("dex_cap_2".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "scale_mail".into(),
            name: "Scale Mail".into(),
            description: "Armor made of overlapping metal scales on a leather coat.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 45.0,
            value_gp: 50,
            stats: ItemStats {
                ac_base: Some(14), // 14 + DEX (max 2)
                special: Some("dex_cap_2".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "breastplate".into(),
            name: "Breastplate".into(),
            description: "A fitted metal chest piece covering the front torso.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 20.0,
            value_gp: 400,
            stats: ItemStats {
                ac_base: Some(14), // 14 + DEX (max 2)
                special: Some("dex_cap_2".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "half_plate".into(),
            name: "Half Plate".into(),
            description: "Plate armor covering the upper body, with leather below.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 40.0,
            value_gp: 750,
            stats: ItemStats {
                ac_base: Some(15), // 15 + DEX (max 2)
                special: Some("dex_cap_2".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Heavy Armor --
        Item {
            id: "ring_mail".into(),
            name: "Ring Mail".into(),
            description: "Leather armor with heavy rings sewn into it.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 40.0,
            value_gp: 30,
            stats: ItemStats {
                ac_base: Some(14), // flat 14, no DEX
                special: Some("no_dex".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "chain_mail".into(),
            name: "Chain Mail".into(),
            description: "Heavy armor made of interlocking metal rings.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 55.0,
            value_gp: 75,
            stats: ItemStats {
                ac_base: Some(16), // flat 16, no DEX
                special: Some("no_dex".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "splint_armor".into(),
            name: "Splint Armor".into(),
            description: "Armor made of narrow vertical strips of metal riveted to leather.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 60.0,
            value_gp: 200,
            stats: ItemStats {
                ac_base: Some(17), // flat 17, no DEX
                special: Some("no_dex".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "plate_armor".into(),
            name: "Plate Armor".into(),
            description: "Full plate armor, the finest protection money can buy.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Chest),
            rarity: Rarity::Common,
            weight: 65.0,
            value_gp: 1500,
            stats: ItemStats {
                ac_base: Some(18), // flat 18, no DEX
                special: Some("no_dex".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Shield --
        Item {
            id: "shield".into(),
            name: "Shield".into(),
            description: "A wooden shield emblazoned with a crest.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::OffHand),
            rarity: Rarity::Common,
            weight: 6.0,
            value_gp: 10,
            stats: ItemStats {
                ac_bonus: 2,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // ===== ACCESSORIES =====
        // -- Helmets --
        Item {
            id: "iron_helm".into(),
            name: "Iron Helm".into(),
            description: "A sturdy iron helmet.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Head),
            rarity: Rarity::Common,
            weight: 3.0,
            value_gp: 15,
            stats: ItemStats {
                ac_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "helm_of_awareness".into(),
            name: "Helm of Awareness".into(),
            description: "A gleaming helm that sharpens the wearer's senses.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Head),
            rarity: Rarity::Uncommon,
            weight: 3.0,
            value_gp: 200,
            stats: ItemStats {
                ac_bonus: 1,
                wis_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Amulets --
        Item {
            id: "amulet_of_health".into(),
            name: "Amulet of Health".into(),
            description: "A golden amulet that bolsters the wearer's vitality.".into(),
            item_type: ItemType::Misc,
            slot: Some(EquipSlot::Amulet),
            rarity: Rarity::Uncommon,
            weight: 0.5,
            value_gp: 250,
            stats: ItemStats {
                con_bonus: 2,
                hp_bonus: 5,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "amulet_of_protection".into(),
            name: "Amulet of Protection".into(),
            description: "A silver amulet inscribed with protective runes.".into(),
            item_type: ItemType::Misc,
            slot: Some(EquipSlot::Amulet),
            rarity: Rarity::Rare,
            weight: 0.5,
            value_gp: 500,
            stats: ItemStats {
                ac_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Rings --
        Item {
            id: "ring_of_protection".into(),
            name: "Ring of Protection".into(),
            description: "A silver ring that creates a faint magical barrier.".into(),
            item_type: ItemType::Misc,
            slot: Some(EquipSlot::Ring),
            rarity: Rarity::Uncommon,
            weight: 0.0,
            value_gp: 300,
            stats: ItemStats {
                ac_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "ring_of_strength".into(),
            name: "Ring of Strength".into(),
            description: "A thick iron ring that enhances physical power.".into(),
            item_type: ItemType::Misc,
            slot: Some(EquipSlot::Ring),
            rarity: Rarity::Uncommon,
            weight: 0.0,
            value_gp: 250,
            stats: ItemStats {
                str_bonus: 2,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "ring_of_evasion".into(),
            name: "Ring of Evasion".into(),
            description: "A slim platinum ring that quickens your reflexes.".into(),
            item_type: ItemType::Misc,
            slot: Some(EquipSlot::Ring),
            rarity: Rarity::Rare,
            weight: 0.0,
            value_gp: 500,
            stats: ItemStats {
                dex_bonus: 2,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Gloves --
        Item {
            id: "gauntlets_of_ogre_power".into(),
            name: "Gauntlets of Ogre Power".into(),
            description: "Heavy gauntlets that grant tremendous strength.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Hands),
            rarity: Rarity::Uncommon,
            weight: 2.0,
            value_gp: 400,
            stats: ItemStats {
                str_bonus: 2,
                attack_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "gloves_of_thievery".into(),
            name: "Gloves of Thievery".into(),
            description: "Thin leather gloves that enhance dexterous tasks.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Hands),
            rarity: Rarity::Uncommon,
            weight: 0.5,
            value_gp: 200,
            stats: ItemStats {
                dex_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Boots --
        Item {
            id: "boots_of_speed".into(),
            name: "Boots of Speed".into(),
            description: "Enchanted boots that quicken your stride.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Feet),
            rarity: Rarity::Rare,
            weight: 2.0,
            value_gp: 500,
            stats: ItemStats {
                speed_bonus: 10,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "boots_of_elvenkind".into(),
            name: "Boots of Elvenkind".into(),
            description: "Soft boots that muffle your footsteps.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Feet),
            rarity: Rarity::Uncommon,
            weight: 1.0,
            value_gp: 200,
            stats: ItemStats {
                dex_bonus: 1,
                special: Some("advantage_stealth".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Cloaks --
        Item {
            id: "cloak_of_protection".into(),
            name: "Cloak of Protection".into(),
            description: "A shimmering cloak that deflects harm.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Back),
            rarity: Rarity::Uncommon,
            weight: 1.0,
            value_gp: 350,
            stats: ItemStats {
                ac_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "cloak_of_elvenkind".into(),
            name: "Cloak of Elvenkind".into(),
            description: "A gray-green cloak that blends with natural surroundings.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Back),
            rarity: Rarity::Uncommon,
            weight: 1.0,
            value_gp: 300,
            stats: ItemStats {
                dex_bonus: 1,
                special: Some("advantage_stealth".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // -- Leg Armor --
        Item {
            id: "iron_greaves".into(),
            name: "Iron Greaves".into(),
            description: "Sturdy leg guards made of iron.".into(),
            item_type: ItemType::Armor,
            slot: Some(EquipSlot::Legs),
            rarity: Rarity::Common,
            weight: 4.0,
            value_gp: 20,
            stats: ItemStats {
                ac_bonus: 1,
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // ===== MAGIC WEAPONS =====
        Item {
            id: "flametongue_longsword".into(),
            name: "Longsword".into(),
            description: "A longsword wreathed in magical fire.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Rare,
            weight: 3.0,
            value_gp: 1000,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing+fire".into()),
                attack_bonus: 1,
                ..Default::default()
            },
            enchantment: Some(Enchantment {
                bonus: 0,
                element: Some("fire".into()),
                name_prefix: "Flametongue".into(),
            }),
            quantity: 1,
        },
        Item {
            id: "frostbrand_greatsword".into(),
            name: "Greatsword".into(),
            description: "A greatsword coated in a thin layer of frost.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Rare,
            weight: 6.0,
            value_gp: 1200,
            stats: ItemStats {
                damage_dice: Some("2d6".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing+cold".into()),
                attack_bonus: 1,
                is_two_handed: true,
                ..Default::default()
            },
            enchantment: Some(Enchantment {
                bonus: 0,
                element: Some("cold".into()),
                name_prefix: "Frostbrand".into(),
            }),
            quantity: 1,
        },
        Item {
            id: "vorpal_longsword".into(),
            name: "Longsword".into(),
            description: "An impossibly sharp blade that can sever heads.".into(),
            item_type: ItemType::Weapon,
            slot: Some(EquipSlot::MainHand),
            rarity: Rarity::Legendary,
            weight: 3.0,
            value_gp: 5000,
            stats: ItemStats {
                damage_dice: Some("1d8".into()),
                damage_modifier_stat: Some("str".into()),
                damage_type: Some("slashing".into()),
                attack_bonus: 3,
                special: Some("vorpal".into()),
                ..Default::default()
            },
            enchantment: Some(Enchantment {
                bonus: 0,
                element: None,
                name_prefix: "Vorpal".into(),
            }),
            quantity: 1,
        },

        // ===== POTIONS & CONSUMABLES =====
        Item {
            id: "health_potion".into(),
            name: "Health Potion".into(),
            description: "A vial of red liquid that restores 2d4+2 HP.".into(),
            item_type: ItemType::Potion,
            slot: None,
            rarity: Rarity::Common,
            weight: 0.5,
            value_gp: 50,
            stats: ItemStats {
                special: Some("heal_2d4+2".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "greater_health_potion".into(),
            name: "Greater Health Potion".into(),
            description: "A large vial of red liquid that restores 4d4+4 HP.".into(),
            item_type: ItemType::Potion,
            slot: None,
            rarity: Rarity::Uncommon,
            weight: 0.5,
            value_gp: 150,
            stats: ItemStats {
                special: Some("heal_4d4+4".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "scroll_of_fireball".into(),
            name: "Scroll of Fireball".into(),
            description: "A magical scroll that unleashes a burst of flame.".into(),
            item_type: ItemType::Scroll,
            slot: None,
            rarity: Rarity::Uncommon,
            weight: 0.1,
            value_gp: 200,
            stats: ItemStats {
                special: Some("cast_fireball".into()),
                ..Default::default()
            },
            enchantment: None,
            quantity: 1,
        },

        // ===== MISC =====
        Item {
            id: "spellbook".into(),
            name: "Spellbook".into(),
            description: "A leather-bound tome containing your arcane knowledge.".into(),
            item_type: ItemType::Misc,
            slot: None,
            rarity: Rarity::Common,
            weight: 3.0,
            value_gp: 50,
            stats: Default::default(),
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "thieves_tools".into(),
            name: "Thieves' Tools".into(),
            description: "A set of lockpicks, a small mirror, scissors, and a pair of pliers.".into(),
            item_type: ItemType::Misc,
            slot: None,
            rarity: Rarity::Common,
            weight: 1.0,
            value_gp: 25,
            stats: Default::default(),
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "torch".into(),
            name: "Torch".into(),
            description: "A simple wooden torch that provides light.".into(),
            item_type: ItemType::Misc,
            slot: None,
            rarity: Rarity::Common,
            weight: 1.0,
            value_gp: 1,
            stats: Default::default(),
            enchantment: None,
            quantity: 1,
        },
        Item {
            id: "rope_50ft".into(),
            name: "Rope (50 ft)".into(),
            description: "Fifty feet of hempen rope.".into(),
            item_type: ItemType::Misc,
            slot: None,
            rarity: Rarity::Common,
            weight: 10.0,
            value_gp: 1,
            stats: Default::default(),
            enchantment: None,
            quantity: 1,
        },
    ]
}

/// List all item IDs in the database (for reference/validation).
pub fn all_item_ids() -> Vec<String> {
    item_database().into_iter().map(|i| i.id).collect()
}
