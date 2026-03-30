//! Shop system — persistent per-town shops with dynamic pricing and shared inventory.
//!
//! Each town in the hex world gets its own independent shop, created lazily on first visit.
//! Prices rise when players buy (scarcity) and fall when players sell (surplus).
//! Shops restock 1 unit per hour toward their base inventory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::equipment::get_item;
use super::crafting;
use super::world_map;
use super::worldgen::HexCoord;

// ========================================================================
// DATA STRUCTURES
// ========================================================================

/// A single item slot in a shop's inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopItem {
    pub item_id: String,
    pub base_stock: u32,
    pub current_stock: u32,
    pub base_price: u32,
    pub sensitivity: f32,
    pub is_base_item: bool,
}

impl ShopItem {
    /// Current buy price based on stock deviation from base.
    pub fn buy_price(&self) -> u32 {
        let deviation = self.base_stock as f32 - self.current_stock as f32;
        let multiplier = (1.0 + deviation * self.sensitivity).max(0.25).min(3.0);
        let price = (self.base_price as f32 * multiplier).round() as u32;
        price.max(1)
    }

    /// Sell price: 60% of current buy price.
    pub fn sell_price(&self) -> u32 {
        let buy = self.buy_price();
        (buy * 60 / 100).max(1)
    }

    /// Price category for UI color coding.
    pub fn price_category(&self) -> &'static str {
        let deviation = self.base_stock as f32 - self.current_stock as f32;
        if deviation > 0.5 {
            "above"
        } else if deviation < -0.5 {
            "below"
        } else {
            "normal"
        }
    }
}

/// Complete state of a single town's shop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopState {
    pub coord_q: i32,
    pub coord_r: i32,
    pub name: String,
    pub tier: u8,
    pub items: HashMap<String, ShopItem>,
    pub last_restock: DateTime<Utc>,
}

impl ShopState {
    /// Create a new shop for a county, using the tier-based item pool.
    pub fn new_from_tier(q: i32, r: i32, county_name: &str, tier: f32) -> Self {
        let entries = world_map::generate_shop(tier);
        let tier_u8 = tier.round() as u8;
        let mut items = HashMap::new();

        let graph = &*crafting::CRAFTING_GRAPH;
        for entry in entries {
            let db_item = get_item(&entry.item_id)
                .or_else(|| crafting::material_to_item(graph, &entry.item_id))
                .or_else(|| crafting::equipment_to_item(&entry.item_id));
            if let Some(db_item) = db_item {
                let base_stock = entry.stock.unwrap_or(if db_item.tier > tier_u8 { 1 } else { 5 });
                let sensitivity = if db_item.tier > tier_u8 { 0.05 } else { 0.03 };
                items.insert(
                    entry.item_id.clone(),
                    ShopItem {
                        item_id: entry.item_id,
                        base_stock,
                        current_stock: base_stock,
                        base_price: (db_item.value_gp as f32 * entry.price_mult).max(1.0) as u32,
                        sensitivity,
                        is_base_item: true,
                    },
                );
            }
        }

        Self {
            coord_q: q,
            coord_r: r,
            name: format!("{} Market", county_name),
            tier: tier_u8,
            items,
            last_restock: Utc::now(),
        }
    }

    /// Apply lazy restocking based on elapsed time.
    /// For each hour elapsed, each item moves 1 unit toward its target stock.
    pub fn apply_restock(&mut self) {
        let now = Utc::now();
        let elapsed_hours = (now - self.last_restock).num_seconds() / 3600;
        if elapsed_hours <= 0 {
            return;
        }

        let restock_units = elapsed_hours as u32;
        let mut to_remove = Vec::new();

        for (item_id, item) in self.items.iter_mut() {
            let target = if item.is_base_item {
                item.base_stock
            } else {
                0
            };

            if item.current_stock < target {
                item.current_stock = (item.current_stock + restock_units).min(target);
            } else if item.current_stock > target {
                let decrease = restock_units.min(item.current_stock - target);
                item.current_stock -= decrease;
            }

            if !item.is_base_item && item.current_stock == 0 {
                to_remove.push(item_id.clone());
            }
        }

        for id in to_remove {
            self.items.remove(&id);
        }

        self.last_restock = now;
    }

    /// Buy an item. Returns (item_name, price_paid) on success.
    pub fn buy(&mut self, item_id: &str, quantity: u32, player_gold: u32) -> Result<(String, u32), String> {
        let item = self
            .items
            .get(item_id)
            .ok_or_else(|| format!("Item '{}' not available", item_id))?;

        if item.current_stock < quantity {
            return Err(format!(
                "Not enough stock (have {}, want {})",
                item.current_stock, quantity
            ));
        }

        let unit_price = item.buy_price();
        let total_price = unit_price * quantity;

        if player_gold < total_price {
            return Err(format!(
                "Not enough gold (need {}, have {})",
                total_price, player_gold
            ));
        }

        let item_name = get_item(item_id)
            .or_else(|| crafting::material_to_item(&*crafting::CRAFTING_GRAPH, item_id))
            .or_else(|| crafting::equipment_to_item(item_id))
            .map(|i| i.name.clone())
            .unwrap_or_else(|| item_id.to_string());

        let item = self.items.get_mut(item_id).unwrap();
        item.current_stock -= quantity;

        Ok((item_name, total_price))
    }

    /// Sell an item to the shop. Returns the total gold earned.
    pub fn sell(&mut self, item_id: &str, base_value: u32, quantity: u32) -> u32 {
        let sell_price_per_unit = if let Some(item) = self.items.get(item_id) {
            item.sell_price()
        } else {
            (base_value * 60 / 100).max(1)
        };

        if let Some(item) = self.items.get_mut(item_id) {
            item.current_stock += quantity;
        } else {
            self.items.insert(
                item_id.to_string(),
                ShopItem {
                    item_id: item_id.to_string(),
                    base_stock: 0,
                    current_stock: quantity,
                    base_price: base_value,
                    sensitivity: 0.04,
                    is_base_item: false,
                },
            );
        }

        sell_price_per_unit * quantity
    }
}

/// Registry of all shops in the world.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShopRegistry {
    pub shops: HashMap<String, ShopState>,
}

impl ShopRegistry {
    pub fn coord_key(q: i32, r: i32) -> String {
        format!("{}_{}", q, r)
    }

    /// Get or create a shop for a county (lazy initialization).
    /// Returns None if the county doesn't have a town.
    pub fn get_or_create(&mut self, q: i32, r: i32) -> Option<&mut ShopState> {
        let key = Self::coord_key(q, r);
        if !self.shops.contains_key(&key) {
            let county = world_map::get_county(HexCoord::new(q, r))?;
            if !county.has_town {
                return None;
            }
            let shop = ShopState::new_from_tier(q, r, &county.name, county.tier);
            self.shops.insert(key.clone(), shop);
        }

        if let Some(shop) = self.shops.get_mut(&key) {
            shop.apply_restock();
            Some(shop)
        } else {
            None
        }
    }
}

// ========================================================================
// TESTS
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(item_id: &str, base_stock: u32, base_price: u32) -> ShopItem {
        ShopItem {
            item_id: item_id.to_string(),
            base_stock,
            current_stock: base_stock,
            base_price,
            sensitivity: 0.04,
            is_base_item: true,
        }
    }

    #[test]
    fn test_buy_price_at_base_stock() {
        let item = make_item("sword", 5, 100);
        assert_eq!(item.buy_price(), 100);
    }

    #[test]
    fn test_buy_price_increases_when_depleted() {
        let mut item = make_item("sword", 5, 100);
        item.current_stock = 2; // 3 below base
        // deviation = 3, mult = 1 + 3*0.04 = 1.12
        assert_eq!(item.buy_price(), 112);
    }

    #[test]
    fn test_buy_price_decreases_when_surplus() {
        let mut item = make_item("sword", 5, 100);
        item.current_stock = 10; // 5 above base
        // deviation = -5, mult = 1 + (-5)*0.04 = 0.80
        assert_eq!(item.buy_price(), 80);
    }

    #[test]
    fn test_buy_price_clamped_floor() {
        let mut item = make_item("sword", 5, 100);
        item.current_stock = 100; // way above base
        // deviation = -95, mult clamped to 0.25
        assert_eq!(item.buy_price(), 25);
    }

    #[test]
    fn test_buy_price_clamped_ceiling() {
        let mut item = make_item("sword", 5, 100);
        item.current_stock = 0; // totally depleted
        item.sensitivity = 1.0; // extreme sensitivity
        // deviation = 5, mult = 1 + 5*1.0 = 6.0, clamped to 3.0
        assert_eq!(item.buy_price(), 300);
    }

    #[test]
    fn test_sell_price_is_60_percent() {
        let item = make_item("sword", 5, 100);
        assert_eq!(item.sell_price(), 60);
    }

    #[test]
    fn test_sell_price_minimum_1() {
        let item = make_item("junk", 5, 1);
        assert!(item.sell_price() >= 1);
    }

    #[test]
    fn test_price_category() {
        let mut item = make_item("sword", 5, 100);
        assert_eq!(item.price_category(), "normal");

        item.current_stock = 2;
        assert_eq!(item.price_category(), "above");

        item.current_stock = 10;
        assert_eq!(item.price_category(), "below");
    }

    #[test]
    fn test_restock_base_items() {
        let mut shop = ShopState {
            coord_q: 0,
            coord_r: 0,
            name: "Test Shop".into(),
            tier: 1,
            items: HashMap::new(),
            last_restock: Utc::now() - chrono::Duration::hours(3),
        };
        shop.items.insert(
            "sword".into(),
            ShopItem {
                item_id: "sword".into(),
                base_stock: 5,
                current_stock: 2, // bought 3
                base_price: 100,
                sensitivity: 0.04,
                is_base_item: true,
            },
        );

        shop.apply_restock();
        assert_eq!(shop.items["sword"].current_stock, 5); // restocked 3 units (capped at base)
    }

    #[test]
    fn test_restock_player_sold_items() {
        let mut shop = ShopState {
            coord_q: 0,
            coord_r: 0,
            name: "Test Shop".into(),
            tier: 1,
            items: HashMap::new(),
            last_restock: Utc::now() - chrono::Duration::hours(2),
        };
        shop.items.insert(
            "junk".into(),
            ShopItem {
                item_id: "junk".into(),
                base_stock: 0,
                current_stock: 5, // player sold 5
                base_price: 10,
                sensitivity: 0.04,
                is_base_item: false,
            },
        );

        shop.apply_restock();
        assert_eq!(shop.items["junk"].current_stock, 3); // drained 2 units
    }

    #[test]
    fn test_restock_removes_empty_player_items() {
        let mut shop = ShopState {
            coord_q: 0,
            coord_r: 0,
            name: "Test Shop".into(),
            tier: 1,
            items: HashMap::new(),
            last_restock: Utc::now() - chrono::Duration::hours(10),
        };
        shop.items.insert(
            "junk".into(),
            ShopItem {
                item_id: "junk".into(),
                base_stock: 0,
                current_stock: 3,
                base_price: 10,
                sensitivity: 0.04,
                is_base_item: false,
            },
        );

        shop.apply_restock();
        assert!(!shop.items.contains_key("junk")); // removed after draining to 0
    }

    #[test]
    fn test_buy_reduces_stock() {
        let mut shop = ShopState {
            coord_q: 0,
            coord_r: 0,
            name: "Test Shop".into(),
            tier: 1,
            items: HashMap::new(),
            last_restock: Utc::now(),
        };
        shop.items.insert("health_potion".into(), make_item("health_potion", 5, 50));

        let result = shop.buy("health_potion", 2, 1000);
        assert!(result.is_ok());
        let (_, price) = result.unwrap();
        assert_eq!(price, 100); // 50 * 2
        assert_eq!(shop.items["health_potion"].current_stock, 3);
    }

    #[test]
    fn test_buy_not_enough_gold() {
        let mut shop = ShopState {
            coord_q: 0,
            coord_r: 0,
            name: "Test Shop".into(),
            tier: 1,
            items: HashMap::new(),
            last_restock: Utc::now(),
        };
        shop.items.insert("sword".into(), make_item("sword", 5, 100));

        let result = shop.buy("sword", 1, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_sell_adds_stock() {
        let mut shop = ShopState {
            coord_q: 0,
            coord_r: 0,
            name: "Test Shop".into(),
            tier: 1,
            items: HashMap::new(),
            last_restock: Utc::now(),
        };
        shop.items.insert("sword".into(), make_item("sword", 5, 100));

        let gold = shop.sell("sword", 100, 3);
        assert_eq!(gold, 180); // 60% of 100 * 3
        assert_eq!(shop.items["sword"].current_stock, 8);
    }

    #[test]
    fn test_sell_new_item_creates_entry() {
        let mut shop = ShopState {
            coord_q: 0,
            coord_r: 0,
            name: "Test Shop".into(),
            tier: 1,
            items: HashMap::new(),
            last_restock: Utc::now(),
        };

        let gold = shop.sell("rare_gem", 200, 1);
        assert_eq!(gold, 120); // 60% of 200
        assert!(shop.items.contains_key("rare_gem"));
        assert!(!shop.items["rare_gem"].is_base_item);
        assert_eq!(shop.items["rare_gem"].current_stock, 1);
    }
}
