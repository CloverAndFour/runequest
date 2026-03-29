//! Adventure persistence — state snapshots + message history.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::engine::AdventureState;
use crate::error::{RunequestError, Result};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A display event — what the player actually saw in the story panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdventureSummary {
    pub id: String,
    pub name: String,
    pub character_name: String,
    pub race: String,
    pub class: String,
    pub level: u32,
    pub updated_at: String,
}

pub struct AdventureStore {
    base_path: PathBuf,
    username: String,
}

impl AdventureStore {
    pub fn new(base_path: &Path, username: &str) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
            username: username.to_string(),
        }
    }

    fn user_dir(&self) -> PathBuf {
        self.base_path
            .join("users")
            .join(&self.username)
            .join("adventures")
    }

    fn adventure_dir(&self, id: &str) -> PathBuf {
        self.user_dir().join(id)
    }

    pub fn list_adventures(&self) -> Result<Vec<AdventureSummary>> {
        let dir = self.user_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut summaries = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let state_path = entry.path().join("state.json");
                if state_path.exists() {
                    if let Ok(data) = std::fs::read_to_string(&state_path) {
                        match serde_json::from_str::<AdventureState>(&data) {
                            Ok(state) => {
                                summaries.push(AdventureSummary {
                                    id: state.id.clone(),
                                    name: state.name.clone(),
                                    character_name: state.character.name.clone(),
                                    race: state.character.race.to_string(),
                                    class: state.character.class.to_string(),
                                    level: state.character.level,
                                    updated_at: state.updated_at.to_rfc3339(),
                                });
                            }
                            Err(e) => {
                                eprintln!("Failed to parse adventure {}: {}", state_path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    pub fn load_adventure(&self, id: &str) -> Result<AdventureState> {
        let path = self.adventure_dir(id).join("state.json");
        if !path.exists() {
            return Err(RunequestError::AdventureNotFound(id.to_string()));
        }
        let data = std::fs::read_to_string(&path)?;
        let state: AdventureState = serde_json::from_str(&data)?;
        Ok(state)
    }

    pub fn save_adventure(&self, state: &AdventureState) -> Result<()> {
        let dir = self.adventure_dir(&state.id);
        std::fs::create_dir_all(&dir)?;

        let path = dir.join("state.json");
        let tmp_path = dir.join("state.json.tmp");
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    pub fn create_adventure(&self, state: AdventureState) -> Result<String> {
        let id = state.id.clone();
        self.save_adventure(&state)?;
        Ok(id)
    }

    pub fn delete_adventure(&self, id: &str) -> Result<()> {
        let dir = self.adventure_dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn append_message(&self, adventure_id: &str, msg: &HistoryMessage) -> Result<()> {
        let dir = self.adventure_dir(adventure_id);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("history.jsonl");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let line = serde_json::to_string(msg)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    pub fn load_history(&self, adventure_id: &str) -> Result<Vec<HistoryMessage>> {
        let path = self.adventure_dir(adventure_id).join("history.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&path)?;
        let messages: Vec<HistoryMessage> = data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(messages)
    }

    pub fn append_display_event(&self, adventure_id: &str, event: &DisplayEvent) -> Result<()> {
        let dir = self.adventure_dir(adventure_id);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("display_history.jsonl");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let line = serde_json::to_string(event)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    pub fn load_display_history(&self, adventure_id: &str) -> Result<Vec<DisplayEvent>> {
        let path = self.adventure_dir(adventure_id).join("display_history.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&path)?;
        let events: Vec<DisplayEvent> = data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(events)
    }
}
