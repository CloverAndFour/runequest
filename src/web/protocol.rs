//! WebSocket message types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    ListAdventures,
    CreateAdventure {
        name: String,
        character_name: String,
        race: String,
        class: String,
        stats: StatsInput,
    },
    LoadAdventure {
        adventure_id: String,
    },
    DeleteAdventure {
        adventure_id: String,
    },
    SendMessage {
        content: String,
    },
    SelectChoice {
        index: usize,
        text: String,
    },
    RollDice,
    GetCharacterSheet,
    GetInventory,
    GetQuests,
}

#[derive(Debug, Deserialize)]
pub struct StatsInput {
    pub strength: u8,
    pub dexterity: u8,
    pub constitution: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub charisma: u8,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Connected {
        username: String,
    },
    AdventureList {
        adventures: Vec<crate::storage::adventure_store::AdventureSummary>,
    },
    AdventureLoaded {
        state: serde_json::Value,
    },
    AdventureCreated {
        adventure_id: String,
    },
    NarrativeChunk {
        text: String,
    },
    NarrativeEnd,
    DiceRollRequest {
        dice_type: String,
        count: u32,
        modifier: i32,
        dc: i32,
        description: String,
        success_probability: f64,
    },
    DiceRollResult {
        rolls: Vec<u32>,
        total: i32,
        dc: i32,
        success: bool,
        description: String,
    },
    PresentChoices {
        choices: Vec<String>,
        allow_custom_input: bool,
        prompt: String,
    },
    StateUpdate {
        state: serde_json::Value,
    },
    Error {
        code: String,
        message: String,
    },
}
