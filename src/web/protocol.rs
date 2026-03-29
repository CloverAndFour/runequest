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
        #[serde(default)]
        scenario: Option<String>,
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
    SetModel {
        model: String,
    },
    // Combat actions (BG3-style)
    CombatAction {
        action_id: String,
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        item_name: Option<String>,
    },
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
        state: serde_json::Value,
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
    CostUpdate {
        session_cost_usd: f64,
        prompt_tokens: u64,
        completion_tokens: u64,
        today_cost_usd: f64,
        week_cost_usd: f64,
        month_cost_usd: f64,
        total_cost_usd: f64,
    },
    ConditionEffects {
        effects: Vec<String>,
    },
    ChatHistory {
        entries: Vec<crate::storage::adventure_store::DisplayEvent>,
    },
    ModelInfo {
        model: String,
        available_models: Vec<String>,
    },
    // Combat messages
    CombatStarted {
        initiative_order: Vec<InitiativeInfo>,
        round: u32,
    },
    CombatTurnStart {
        combatant: String,
        is_player: bool,
        round: u32,
        actions: u32,
        bonus_actions: u32,
        movement: u32,
        available_actions: Vec<ActionInfo>,
        enemies: Vec<EnemyInfo>,
    },
    CombatActionResult {
        actor: String,
        action: String,
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        roll: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hit: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        damage: Option<i32>,
    },
    CombatEnemyTurn {
        enemy_name: String,
        attack_name: String,
        attack_roll: i32,
        target_ac: i32,
        hit: bool,
        damage: i32,
        player_hp: i32,
        player_max_hp: i32,
    },
    CombatEnded {
        xp_reward: u32,
        victory: bool,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct InitiativeInfo {
    pub name: String,
    pub roll: i32,
    pub is_player: bool,
}

#[derive(Debug, Serialize)]
pub struct ActionInfo {
    pub id: String,
    pub name: String,
    pub cost: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct EnemyInfo {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub alive: bool,
}
