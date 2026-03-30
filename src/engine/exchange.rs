//! Exchange order book system — central market for player-to-player trading via limit orders.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderType { Buy, Sell }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub order_type: OrderType,
    pub player: String,
    pub item_id: String,
    pub item_name: String,
    pub quantity: u32,
    pub price_per_unit: u32,
    pub filled: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderBook {
    pub orders: Vec<Order>,
}

impl OrderBook {
    pub fn place_order(&mut self, order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining = order.quantity;

        // Try to match against existing orders
        let matching_orders: Vec<usize> = self.orders.iter().enumerate()
            .filter(|(_, o)| {
                o.item_id == order.item_id
                && o.order_type != order.order_type
                && o.player != order.player
                && match order.order_type {
                    OrderType::Buy => o.price_per_unit <= order.price_per_unit,
                    OrderType::Sell => o.price_per_unit >= order.price_per_unit,
                }
                && o.filled < o.quantity
            })
            .map(|(i, _)| i)
            .collect();

        // Execute matches (price-time priority — earlier orders first)
        for &idx in &matching_orders {
            if remaining == 0 { break; }
            let available = self.orders[idx].quantity - self.orders[idx].filled;
            let fill_qty = remaining.min(available);
            let price = self.orders[idx].price_per_unit;

            self.orders[idx].filled += fill_qty;
            remaining -= fill_qty;

            trades.push(Trade {
                buyer: if order.order_type == OrderType::Buy { order.player.clone() } else { self.orders[idx].player.clone() },
                seller: if order.order_type == OrderType::Sell { order.player.clone() } else { self.orders[idx].player.clone() },
                item_id: order.item_id.clone(),
                item_name: order.item_name.clone(),
                quantity: fill_qty,
                price_per_unit: price,
            });
        }

        // Remove fully filled orders
        self.orders.retain(|o| o.filled < o.quantity);

        // If not fully filled, add remainder as new order
        if remaining > 0 {
            let mut new_order = order;
            new_order.quantity = remaining;
            new_order.filled = 0;
            self.orders.push(new_order);
        }

        trades
    }

    pub fn cancel_order(&mut self, order_id: &str, player: &str) -> bool {
        if let Some(idx) = self.orders.iter().position(|o| o.id == order_id && o.player == player) {
            self.orders.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn list_orders(&self, item_id: Option<&str>) -> Vec<&Order> {
        self.orders.iter()
            .filter(|o| item_id.map_or(true, |id| o.item_id == id))
            .collect()
    }

    pub fn player_orders(&self, player: &str) -> Vec<&Order> {
        self.orders.iter().filter(|o| o.player == player).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub buyer: String,
    pub seller: String,
    pub item_id: String,
    pub item_name: String,
    pub quantity: u32,
    pub price_per_unit: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_order(order_type: OrderType, player: &str, item_id: &str, qty: u32, price: u32) -> Order {
        Order {
            id: uuid::Uuid::new_v4().to_string(),
            order_type,
            player: player.to_string(),
            item_id: item_id.to_string(),
            item_name: format!("Item {}", item_id),
            quantity: qty,
            price_per_unit: price,
            filled: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_no_match_different_items() {
        let mut book = OrderBook::default();
        let sell = make_order(OrderType::Sell, "alice", "iron", 5, 10);
        let buy = make_order(OrderType::Buy, "bob", "gold", 5, 10);
        let trades = book.place_order(sell);
        assert!(trades.is_empty());
        let trades = book.place_order(buy);
        assert!(trades.is_empty());
        assert_eq!(book.orders.len(), 2);
    }

    #[test]
    fn test_exact_match() {
        let mut book = OrderBook::default();
        let sell = make_order(OrderType::Sell, "alice", "iron", 5, 10);
        book.place_order(sell);
        let buy = make_order(OrderType::Buy, "bob", "iron", 5, 10);
        let trades = book.place_order(buy);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5);
        assert_eq!(trades[0].price_per_unit, 10);
        assert_eq!(book.orders.len(), 0);
    }

    #[test]
    fn test_partial_fill() {
        let mut book = OrderBook::default();
        let sell = make_order(OrderType::Sell, "alice", "iron", 10, 10);
        book.place_order(sell);
        let buy = make_order(OrderType::Buy, "bob", "iron", 3, 10);
        let trades = book.place_order(buy);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 3);
        assert_eq!(book.orders.len(), 1);
        assert_eq!(book.orders[0].quantity - book.orders[0].filled, 7);
    }

    #[test]
    fn test_cancel_order() {
        let mut book = OrderBook::default();
        let sell = make_order(OrderType::Sell, "alice", "iron", 5, 10);
        let oid = sell.id.clone();
        book.place_order(sell);
        assert!(book.cancel_order(&oid, "alice"));
        assert_eq!(book.orders.len(), 0);
        assert!(!book.cancel_order(&oid, "alice")); // already cancelled
    }

    #[test]
    fn test_same_player_no_self_trade() {
        let mut book = OrderBook::default();
        let sell = make_order(OrderType::Sell, "alice", "iron", 5, 10);
        book.place_order(sell);
        let buy = make_order(OrderType::Buy, "alice", "iron", 5, 10);
        let trades = book.place_order(buy);
        assert!(trades.is_empty());
        assert_eq!(book.orders.len(), 2);
    }
}
