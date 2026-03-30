//! Persistence for the guild system.

use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::engine::guild::Guild;

pub struct GuildStore {
    path: PathBuf,
}

impl GuildStore {
    pub fn new(data_dir: &Path) -> Self {
        let path = data_dir.join("guilds.json");
        Self { path }
    }

    pub fn load_all(&self) -> Result<Vec<Guild>> {
        if self.path.exists() {
            let data = std::fs::read_to_string(&self.path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn save_all(&self, guilds: &[Guild]) -> Result<()> {
        let json = serde_json::to_string_pretty(guilds)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn find_by_name(&self, name: &str) -> Result<Option<Guild>> {
        let guilds = self.load_all()?;
        let lower = name.to_lowercase();
        Ok(guilds.into_iter().find(|g| g.name.to_lowercase() == lower))
    }

    pub fn find_by_member(&self, username: &str) -> Result<Option<Guild>> {
        let guilds = self.load_all()?;
        Ok(guilds.into_iter().find(|g| g.is_member(username)))
    }
}
