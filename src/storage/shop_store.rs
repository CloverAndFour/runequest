//! Persistence for the shop system — JSON file with in-memory cache.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::engine::shop::ShopRegistry;

/// Persistent shop store with in-memory caching.
#[derive(Clone)]
pub struct ShopStore {
    path: PathBuf,
    cache: Arc<RwLock<ShopRegistry>>,
}

impl ShopStore {
    pub fn new(data_dir: &Path) -> Self {
        let path = data_dir.join("shops.json");
        let registry = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|data| serde_json::from_str(&data).ok())
                .unwrap_or_default()
        } else {
            ShopRegistry::default()
        };
        Self {
            path,
            cache: Arc::new(RwLock::new(registry)),
        }
    }

    /// Atomic read-modify-write operation on the shop registry.
    pub fn with_mut<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut ShopRegistry) -> R,
    {
        let mut reg = self
            .cache
            .write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let result = f(&mut reg);
        let json = serde_json::to_string(&*reg)?;
        std::fs::write(&self.path, json)?;
        Ok(result)
    }

    /// Read-only access to the shop registry.
    pub fn with_ref<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&ShopRegistry) -> R,
    {
        let reg = self
            .cache
            .read()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        Ok(f(&reg))
    }
}
