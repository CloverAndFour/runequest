//! Persistence for the exchange order book.

use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::engine::exchange::OrderBook;

pub struct ExchangeStore {
    path: PathBuf,
}

impl ExchangeStore {
    pub fn new(data_dir: &Path) -> Self {
        let path = data_dir.join("exchange.json");
        Self { path }
    }

    pub fn load(&self) -> Result<OrderBook> {
        if self.path.exists() {
            let data = std::fs::read_to_string(&self.path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(OrderBook::default())
        }
    }

    pub fn save(&self, book: &OrderBook) -> Result<()> {
        let json = serde_json::to_string_pretty(book)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}
