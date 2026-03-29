//! Item and inventory management.

use serde::{Deserialize, Serialize};

use super::equipment::{Enchantment, EquipSlot, ItemStats, Rarity};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Weapon,
    Armor,
    Potion,
    Scroll,
    Misc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
    pub item_type: ItemType,
    pub slot: Option<EquipSlot>,
    pub rarity: Rarity,
    pub weight: f32,
    pub value_gp: u32,
    pub stats: ItemStats,
    pub enchantment: Option<Enchantment>,
    pub quantity: u32,
}

impl Item {
    /// Display name including enchantment prefix (e.g. "Flametongue Longsword").
    pub fn display_name(&self) -> String {
        if let Some(ref ench) = self.enchantment {
            if !ench.name_prefix.is_empty() {
                return format!("{} {}", ench.name_prefix, self.name);
            }
        }
        self.name.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Inventory {
    pub items: Vec<Item>,
    pub gold: u32,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            gold: 0,
        }
    }

    pub fn add(&mut self, item: Item) {
        // For stackable items (potions, scrolls), try to merge quantities
        if matches!(item.item_type, ItemType::Potion | ItemType::Scroll) {
            if let Some(existing) = self.items.iter_mut().find(|i| i.id == item.id) {
                existing.quantity += item.quantity;
                return;
            }
        }
        self.items.push(item);
    }

    pub fn remove(&mut self, name: &str) -> Option<Item> {
        let name_lower = name.to_lowercase();
        if let Some(pos) = self.items.iter().position(|i| {
            i.name.to_lowercase() == name_lower
                || i.display_name().to_lowercase() == name_lower
                || i.id.to_lowercase() == name_lower
        }) {
            let item = &mut self.items[pos];
            if item.quantity > 1 {
                item.quantity -= 1;
                let mut single = item.clone();
                single.quantity = 1;
                Some(single)
            } else {
                Some(self.items.remove(pos))
            }
        } else {
            None
        }
    }

    pub fn find(&self, name: &str) -> Option<&Item> {
        let name_lower = name.to_lowercase();
        self.items.iter().find(|i| {
            i.name.to_lowercase() == name_lower
                || i.display_name().to_lowercase() == name_lower
                || i.id.to_lowercase() == name_lower
        })
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut Item> {
        let name_lower = name.to_lowercase();
        self.items.iter_mut().find(|i| {
            i.name.to_lowercase() == name_lower
                || i.display_name().to_lowercase() == name_lower
                || i.id.to_lowercase() == name_lower
        })
    }

    pub fn total_weight(&self) -> f32 {
        self.items.iter().map(|i| i.weight * i.quantity as f32).sum()
    }
}
