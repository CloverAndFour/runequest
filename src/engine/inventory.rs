//! Item and inventory management.

use serde::{Deserialize, Serialize};

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
    pub name: String,
    pub description: String,
    pub item_type: ItemType,
    pub properties: serde_json::Value,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Inventory {
    pub items: Vec<Item>,
}

impl Inventory {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn remove(&mut self, name: &str) -> Option<Item> {
        let name_lower = name.to_lowercase();
        if let Some(pos) = self
            .items
            .iter()
            .position(|i| i.name.to_lowercase() == name_lower)
        {
            Some(self.items.remove(pos))
        } else {
            None
        }
    }

    pub fn find(&self, name: &str) -> Option<&Item> {
        let name_lower = name.to_lowercase();
        self.items
            .iter()
            .find(|i| i.name.to_lowercase() == name_lower)
    }

    pub fn total_weight(&self) -> f32 {
        self.items.iter().map(|i| i.weight).sum()
    }
}
