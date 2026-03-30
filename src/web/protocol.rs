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
        #[serde(default)]
        class: Option<String>,
        #[serde(default)]
        background: Option<String>,
        #[serde(default)]
        backstory: Option<String>,
        #[serde(default)]
        scenario: Option<String>,
        #[serde(default)]
        stats: Option<StatsInput>,
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
    // NPC
    GetNpcs,
    // Crafting
    CraftItem {
        recipe_id: String,
    },
    ListRecipes {
        #[serde(default)]
        skill: Option<String>,
        #[serde(default)]
        tier: Option<u8>,
    },
    ListMaterials,
    Gather,
    // Skills
    // Equipment (UI)
    EquipItem {
        item_name: String,
    },
    UnequipItem {
        slot: String,
    },
    GetSkills,
    // Shop
    ViewShop,
    ShopBuy {
        item_id: String,
        #[serde(default = "default_quantity")]
        quantity: u32,
    },
    ShopSell {
        item_name: String,
    },
    // Dungeon
    DungeonEnter {
        #[serde(default)]
        seed: Option<u64>,
        #[serde(default)]
        tier: Option<u32>,
    },
    DungeonMove {
        direction: String,
    },
    DungeonSkillCheck {
        direction: String,
        skill_id: String,
    },
    DungeonActivatePoint {
        puzzle_id: String,
        room_id: usize,
    },
    DungeonRetreat,
    DungeonStatus,
    // Tower
    TowerList,
    TowerEnter {
        tower_id: String,
    },
    TowerMove {
        direction: String,
    },
    TowerAscend,
    TowerCheckpoint {
        floor: u32,
    },
    TowerTeleport {
        target_floor: u32,
    },
    TowerFloorStatus {
        tower_id: String,
        floor: u32,
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
    // Crafting results
    CraftResult {
        recipe_name: String,
        output: String,
        quantity: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        skill_progress: Option<serde_json::Value>,
    },
    RecipeList {
        recipes: Vec<serde_json::Value>,
    },
    MaterialList {
        materials: Vec<serde_json::Value>,
    },
    GatherResult {
        gathered: Vec<serde_json::Value>,
        biome: String,
        survival_xp: u32,
    },
    SkillList {
        skills: Vec<serde_json::Value>,
    },
    // Shop results
    ShopInventory {
        shop_name: String,
        tier: u8,
        items: Vec<ShopItemInfo>,
        player_gold: u32,
    },
    ShopBuyResult {
        success: bool,
        item_name: String,
        price: u32,
        gold_remaining: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    ShopSellResult {
        success: bool,
        item_name: String,
        gold_earned: u32,
        gold_remaining: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    // Dungeon messages
    DungeonEntered {
        name: String,
        tier: u32,
        floors: usize,
        room: serde_json::Value,
    },
    DungeonRoomChanged {
        room: serde_json::Value,
        floor: usize,
        room_id: usize,
    },
    DungeonSkillGateResult {
        skill: String,
        roll: i32,
        dc: i32,
        success: bool,
    },
    DungeonPuzzleActivation {
        puzzle_id: String,
        activated_count: usize,
        required_count: u32,
        solved: bool,
    },
    DungeonRetreated {
        message: String,
    },
    DungeonStatus {
        status: serde_json::Value,
    },
    CorruptionTick {
        level: f32,
        effects: serde_json::Value,
    },
    PathCleared {
        path_index: usize,
        mini_boss: String,
    },
    ConvergenceUnlocked {
        convergence_room: usize,
    },
    BreachWarning {
        message: String,
    },
    // Tower messages
    TowerList {
        towers: Vec<serde_json::Value>,
    },
    TowerEntered {
        tower_name: String,
        floor: u32,
        tier: String,
    },
    TowerFloorStatus {
        floor: serde_json::Value,
    },
    TowerPlayerNearby {
        player_name: String,
        room_x: u32,
        room_y: u32,
    },
    TowerFirstClear {
        tower: String,
        floor: u32,
        player: String,
    },
    StateChanges {
        #[serde(skip_serializing_if = "Option::is_none")]
        gold_delta: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        xp_delta: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hp_delta: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        level_up: Option<bool>,
        #[serde(default)]
        items_gained: Vec<String>,
        #[serde(default)]
        items_lost: Vec<String>,
        #[serde(default)]
        conditions_added: Vec<String>,
        #[serde(default)]
        conditions_removed: Vec<String>,
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

fn default_quantity() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopItemInfo {
    pub item_id: String,
    pub name: String,
    pub description: String,
    pub buy_price: u32,
    pub sell_price: u32,
    pub current_stock: u32,
    pub price_category: String,
    pub tier: u8,
}
