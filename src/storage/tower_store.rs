//! Tower floor persistence — stores shared tower floor state as JSON files.

use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::engine::tower::TowerFloor;

pub struct TowerStore {
    base_dir: PathBuf,
}

impl TowerStore {
    pub fn new(data_dir: &Path) -> Self {
        let base_dir = data_dir.join("towers");
        std::fs::create_dir_all(&base_dir).ok();
        Self { base_dir }
    }

    /// Load a floor from disk, or return None if it has not been generated yet.
    pub fn load_floor(&self, tower_id: &str, floor: u32) -> Result<Option<TowerFloor>> {
        let path = self.base_dir.join(format!("{}_{}.json", tower_id, floor));
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            Ok(Some(serde_json::from_str(&data)?))
        } else {
            Ok(None)
        }
    }

    /// Save a floor to disk (overwrites if it already exists).
    pub fn save_floor(&self, floor: &TowerFloor) -> Result<()> {
        let path = self.base_dir.join(format!("{}_{}.json", floor.tower_id, floor.floor_number));
        let json = serde_json::to_string_pretty(floor)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}
