//! REST API server — stateless JSON endpoints on a separate port.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware as axum_mw,
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::auth::middleware::{require_auth, AuthMode, AuthState, AuthUser};
use crate::auth::{JwtManager, UserStore};
use crate::engine::adventure::AdventureState;
use crate::engine::character::{Class, Race, Stats};
use crate::engine::combat::{CombatantId, Enemy, EnemyAttack};
use crate::engine::conditions::apply_turn_effects;
use crate::engine::dice::DiceRoller;
use crate::engine::equipment;
use crate::engine::executor::{execute_tool_call, execute_tool_call_with_shop, ToolExecResult};
use crate::storage::shop_store::ShopStore;
use crate::engine::crafting::CRAFTING_GRAPH;
use crate::llm::client::XaiClient;
use crate::llm::pricing::{model_cost, TokenUsage};
use crate::llm::prompts::{adventure_start_prompt, build_system_prompt};
use crate::llm::tools::build_tool_definitions;
use crate::llm::types::*;
use crate::storage::adventure_store::{AdventureStore, DisplayEvent, HistoryMessage};
use crate::storage::usage_logger::{UsageEntry, UsageLogger};

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub struct ApiState {
    pub data_dir: PathBuf,
    pub xai_client: Arc<XaiClient>,
    pub default_model: String,
    pub auth_mode: AuthMode,
    pub user_store: Arc<UserStore>,
    pub jwt_manager: Arc<JwtManager>,
    pub shop_store: ShopStore,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct GameResponse {
    state: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    narrative: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending: Option<PendingInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    combat: Option<CombatInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost: Option<CostInfo>,
}

#[derive(Serialize)]
struct PendingInfo {
    pending_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dice_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modifier: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    success_probability: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    choices: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_custom_input: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
}

#[derive(Serialize)]
struct CombatInfo {
    active: bool,
    round: u32,
    current_turn: String,
    is_player_turn: bool,
    enemies: Vec<EnemyInfo>,
    available_actions: Vec<ActionInfo>,
    initiative_order: Vec<InitiativeInfo>,
}

#[derive(Serialize)]
struct CostInfo {
    prompt_tokens: u64,
    completion_tokens: u64,
    cost_usd: f64,
}

#[derive(Serialize)]
struct EnemyInfo {
    name: String,
    hp: i32,
    max_hp: i32,
    ac: i32,
    alive: bool,
}

#[derive(Serialize)]
struct ActionInfo {
    id: String,
    name: String,
    cost: String,
    description: String,
    enabled: bool,
}

#[derive(Serialize)]
struct InitiativeInfo {
    name: String,
    roll: i32,
    is_player: bool,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    username: String,
    role: String,
}

#[derive(Deserialize)]
struct CreateAdventureRequest {
    name: String,
    character_name: String,
    race: String,
    #[serde(default)]
    class: Option<String>,
    #[serde(default)]
    background: Option<String>,
    #[serde(default)]
    scenario: Option<String>,
    #[serde(default)]
    stats: Option<StatsInput>,
}

#[derive(Deserialize)]
struct StatsInput {
    strength: u8,
    dexterity: u8,
    constitution: u8,
    intelligence: u8,
    wisdom: u8,
    charisma: u8,
}

#[derive(Deserialize)]
struct MessageRequest {
    content: String,
}

#[derive(Deserialize)]
struct ChoiceRequest {
    index: usize,
    text: String,
}

#[derive(Deserialize)]
struct CombatActionRequest {
    action_id: String,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Deserialize)]
struct EquipRequest {
    item_name: String,
}

#[derive(Deserialize)]
struct UnequipRequest {
    slot: String,
}

#[derive(Deserialize)]
struct HpRequest {
    delta: i32,
    reason: String,
}

#[derive(Deserialize)]
struct ItemRequest {
    item_id: String,
}

#[derive(Deserialize)]
struct GoldRequest {
    amount: u32,
}

#[derive(Deserialize)]
struct XpRequest {
    amount: u32,
    reason: String,
}

#[derive(Deserialize)]
struct ConditionRequest {
    condition: String,
    action: String,
}

#[derive(Deserialize)]
struct CraftRequest {
    recipe_id: String,
}

#[derive(Deserialize)]
struct SkillRequest {
    action: String,
    #[serde(default)]
    skill_id: Option<String>,
    #[serde(default)]
    amount: Option<u32>,
}

#[derive(Deserialize)]
struct StartCombatRequest {
    enemies: Vec<EnemyInput>,
}

#[derive(Deserialize)]
struct EnemyInput {
    name: String,
    hp: i32,
    #[serde(default = "default_ac")]
    ac: i32,
    #[serde(default)]
    attacks: Vec<EnemyAttackInput>,
}

fn default_ac() -> i32 {
    10
}

#[derive(Deserialize)]
struct EnemyAttackInput {
    name: String,
    #[serde(default = "default_damage_dice")]
    damage_dice: String,
    #[serde(default)]
    damage_modifier: i32,
    #[serde(default = "default_to_hit")]
    to_hit_bonus: i32,
}

fn default_damage_dice() -> String {
    "1d6".to_string()
}

fn default_to_hit() -> i32 {
    3
}

#[derive(Deserialize)]
struct RollRequest {
    dice: String,
    #[serde(default = "default_count")]
    count: u32,
    #[serde(default)]
    modifier: i32,
    #[serde(default)]
    dc: Option<i32>,
}

fn default_count() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ApiError {
    error: String,
    code: String,
}

fn err_json(code: &str, msg: &str) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: msg.to_string(),
            code: code.to_string(),
        }),
    )
}

fn err_not_found(msg: &str) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            error: msg.to_string(),
            code: "not_found".to_string(),
        }),
    )
}

fn err_internal(msg: &str) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            error: msg.to_string(),
            code: "internal_error".to_string(),
        }),
    )
}

// ---------------------------------------------------------------------------
// Server entry point
// ---------------------------------------------------------------------------

pub async fn run_api_server(
    port: u16,
    bind_address: &str,
    data_dir: PathBuf,
    xai_client: Arc<XaiClient>,
    default_model: String,
    auth_mode: AuthMode,
    user_store: Arc<UserStore>,
    jwt_manager: Arc<JwtManager>,
    shop_store: ShopStore,
) -> anyhow::Result<()> {
    let api_state = Arc::new(ApiState {
        data_dir,
        xai_client,
        default_model,
        auth_mode: auth_mode.clone(),
        user_store,
        jwt_manager: jwt_manager.clone(),
        shop_store,
    });

    let auth_state = Arc::new(AuthState {
        auth_mode,
        jwt_manager,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes
    let public_routes = Router::new()
        .route("/api/auth/login", post(login_handler))
        .route("/health", get(|| async { "ok" }));

    // Protected routes
    let protected_routes = Router::new()
        // Adventure CRUD
        .route("/api/adventures", get(list_adventures))
        .route("/api/adventures", post(create_adventure))
        .route("/api/adventures/:id", get(get_adventure))
        .route("/api/adventures/:id", delete(delete_adventure))
        .route("/api/adventures/:id/history", get(get_history))
        // Game actions (LLM)
        .route("/api/adventures/:id/message", post(send_message))
        .route("/api/adventures/:id/choice", post(send_choice))
        .route("/api/adventures/:id/roll", post(roll_dice))
        // Combat
        .route("/api/adventures/:id/combat", post(combat_action))
        // Equipment
        .route("/api/adventures/:id/equip", post(equip_item))
        .route("/api/adventures/:id/unequip", post(unequip_item))
        // Direct engine endpoints
        .route("/api/adventures/:id/engine/hp", post(engine_hp))
        .route("/api/adventures/:id/engine/item", post(engine_item))
        .route("/api/adventures/:id/engine/gold", post(engine_gold))
        .route("/api/adventures/:id/engine/xp", post(engine_xp))
        .route("/api/adventures/:id/engine/condition", post(engine_condition))
        .route("/api/adventures/:id/engine/combat", post(engine_combat))
        .route("/api/adventures/:id/engine/roll", post(engine_roll))
        // Items
        .route("/api/items", get(list_items))
        .route("/api/items/:id", get(get_item_by_id))
        // Crafting
        .route("/api/recipes", get(list_recipes))
        .route("/api/recipes/:recipe_id", get(get_recipe))
        .route("/api/materials", get(list_materials))
        .route("/api/adventures/:id/craft", post(craft_item))
        .route("/api/adventures/:id/gather", post(gather_materials))
        // Skills
        .route("/api/adventures/:id/engine/skill", post(engine_skill))
        // Shop
        .route("/api/adventures/:id/shop", get(shop_view))
        .route("/api/adventures/:id/shop/buy", post(shop_buy))
        .route("/api/adventures/:id/shop/sell", post(shop_sell))
        // Backgrounds
        .route("/api/backgrounds", get(list_backgrounds))
        .layer(axum_mw::from_fn_with_state(
            auth_state.clone(),
            require_auth,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors)
        .with_state(api_state);

    let addr = format!("{}:{}", bind_address, port);
    eprintln!("RuneQuest REST API starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Shop endpoints
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ShopBuyRequest {
    item_id: String,
    #[serde(default = "default_shop_quantity")]
    quantity: u32,
}

#[derive(Deserialize)]
struct ShopSellRequest {
    item_name: String,
}

fn default_shop_quantity() -> u32 { 1 }

async fn shop_view(
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
) -> impl IntoResponse {
    let store = AdventureStore::new(&state.data_dir, &user.username);
    let adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Adventure not found"}))).into_response(),
    };

    let args = serde_json::json!({});
    let mut adv = adventure.clone();
    match execute_tool_call_with_shop(&mut adv, "view_shop", &args, Some(&state.shop_store)) {
        Ok(ToolExecResult::Immediate(result)) => Json(result).into_response(),
        Ok(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Unexpected result type"}))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn shop_buy(
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Json(body): Json<ShopBuyRequest>,
) -> impl IntoResponse {
    let store = AdventureStore::new(&state.data_dir, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Adventure not found"}))).into_response(),
    };

    let args = serde_json::json!({"item_id": body.item_id, "quantity": body.quantity});
    match execute_tool_call_with_shop(&mut adventure, "buy_item", &args, Some(&state.shop_store)) {
        Ok(ToolExecResult::Immediate(result)) => {
            let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            if success {
                store.save_adventure(&adventure).ok();
            }
            Json(result).into_response()
        }
        Ok(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Unexpected result type"}))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn shop_sell(
    Path(id): Path<String>,
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Json(body): Json<ShopSellRequest>,
) -> impl IntoResponse {
    let store = AdventureStore::new(&state.data_dir, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Adventure not found"}))).into_response(),
    };

    let args = serde_json::json!({"item_name": body.item_name});
    match execute_tool_call_with_shop(&mut adventure, "sell_item", &args, Some(&state.shop_store)) {
        Ok(ToolExecResult::Immediate(result)) => {
            let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            if success {
                store.save_adventure(&adventure).ok();
            }
            Json(result).into_response()
        }
        Ok(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Unexpected result type"}))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_store(state: &ApiState, username: &str) -> AdventureStore {
    AdventureStore::new(&state.data_dir, username)
}

fn build_combat_info(adventure: &AdventureState) -> Option<CombatInfo> {
    if !adventure.combat.active {
        return None;
    }
    let has_weapon = adventure.equipment.equipped_weapon().is_some();
    let has_potion = adventure
        .inventory
        .items
        .iter()
        .any(|i| i.item_type == crate::engine::inventory::ItemType::Potion);
    let actions = adventure
        .combat
        .available_actions(&adventure.character, has_weapon, has_potion);

    Some(CombatInfo {
        active: true,
        round: adventure.combat.round,
        current_turn: adventure.combat.current_combatant_name(),
        is_player_turn: adventure.combat.is_player_turn(),
        enemies: adventure
            .combat
            .enemies
            .iter()
            .map(|e| EnemyInfo {
                name: e.name.clone(),
                hp: e.hp,
                max_hp: e.max_hp,
                ac: e.ac,
                alive: e.hp > 0,
            })
            .collect(),
        available_actions: actions
            .into_iter()
            .map(|a| ActionInfo {
                id: a.id,
                name: a.name,
                cost: a.cost,
                description: a.description,
                enabled: a.enabled,
            })
            .collect(),
        initiative_order: adventure
            .combat
            .initiative
            .iter()
            .map(|e| InitiativeInfo {
                name: e.name.clone(),
                roll: e.roll,
                is_player: e.combatant == CombatantId::Player,
            })
            .collect(),
    })
}


fn build_state_with_map(adventure: &AdventureState) -> serde_json::Value {
    let mut state = serde_json::to_value(adventure).unwrap_or_default();
    if let serde_json::Value::Object(ref mut map) = state {
        map.insert("map_view".to_string(),
            crate::engine::world_map::build_map_view(&adventure.world_position, &adventure.discovery, false));
    }
    state
}

fn game_response(
    adventure: &AdventureState,
    narrative: Option<String>,
    pending: Option<PendingInfo>,
    cost: Option<CostInfo>,
) -> GameResponse {
    GameResponse {
        state: build_state_with_map(adventure),
        narrative,
        pending,
        combat: build_combat_info(adventure),
        cost,
    }
}

fn log_api_usage(state: &ApiState, username: &str, model: &str, usage: &TokenUsage) {
    let logger = UsageLogger::new(&state.data_dir);
    let cost_usd = model_cost(model, usage);
    let _ = logger.log(&UsageEntry {
        ts: chrono::Utc::now(),
        model: model.to_string(),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        cost_usd,
        username: username.to_string(),
    });
}

/// Load adventure history into ChatMessage format for the LLM.
fn load_messages_for_adventure(
    store: &AdventureStore,
    adventure: &AdventureState,
) -> Vec<ChatMessage> {
    let system = ChatMessage::system(&build_system_prompt(adventure));
    let mut messages = vec![system];
    if let Ok(history) = store.load_history(&adventure.id) {
        for h in &history {
            messages.push(ChatMessage {
                role: h.role.clone(),
                content: h.content.clone(),
                tool_calls: h
                    .tool_calls
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                tool_call_id: h.tool_call_id.clone(),
            });
        }
    }
    messages
}

// ---------------------------------------------------------------------------
// Core LLM tool loop (non-streaming)
// ---------------------------------------------------------------------------

struct ToolLoopResult {
    adventure: AdventureState,
    narrative: String,
    pending: Option<PendingInfo>,
    total_cost: Option<CostInfo>,
}

/// Runs the LLM tool loop, accumulating narrative text. Returns when the LLM
/// produces a final text response, or when a pending action is needed.
async fn run_tool_loop(
    state: &ApiState,
    store: &AdventureStore,
    mut adventure: AdventureState,
    mut messages: Vec<ChatMessage>,
    username: &str,
) -> Result<ToolLoopResult, (StatusCode, Json<ApiError>)> {
    let tools = build_tool_definitions();
    let model = state.default_model.clone();
    let max_iterations = 15;
    let mut narrative = String::new();
    let mut total_prompt = 0u64;
    let mut total_completion = 0u64;

    for _ in 0..max_iterations {
        let (response, usage) = state
            .xai_client
            .chat_with_tools(&messages, &tools, Some(&model))
            .await
            .map_err(|e| err_internal(&format!("LLM error: {}", e)))?;

        if let Some(ref u) = usage {
            total_prompt += u.prompt_tokens;
            total_completion += u.completion_tokens;
            log_api_usage(state, username, &model, u);
        }

        match response {
            XaiResponse::Text(text) => {
                if !text.is_empty() {
                    if !narrative.is_empty() {
                        narrative.push('\n');
                    }
                    narrative.push_str(&text);
                }

                messages.push(ChatMessage::assistant_text(&text));
                let _ = store.append_message(
                    &adventure.id,
                    &HistoryMessage {
                        role: "assistant".to_string(),
                        content: Some(text.clone()),
                        tool_calls: None,
                        tool_call_id: None,
                        timestamp: chrono::Utc::now(),
                    },
                );
                let _ = store.append_display_event(
                    &adventure.id,
                    &DisplayEvent {
                        event_type: "narrative".to_string(),
                        data: serde_json::json!({"text": text}),
                        timestamp: chrono::Utc::now(),
                    },
                );

                store
                    .save_adventure(&adventure)
                    .map_err(|e| err_internal(&format!("Save error: {}", e)))?;

                return Ok(ToolLoopResult {
                    adventure,
                    narrative,
                    pending: None,
                    total_cost: Some(CostInfo {
                        prompt_tokens: total_prompt,
                        completion_tokens: total_completion,
                        cost_usd: {
                            let u = TokenUsage {
                                prompt_tokens: total_prompt,
                                completion_tokens: total_completion,
                            };
                            model_cost(&model, &u)
                        },
                    }),
                });
            }

            XaiResponse::ToolCalls { tool_calls, text } => {
                if let Some(ref t) = text {
                    if !t.is_empty() {
                        if !narrative.is_empty() {
                            narrative.push('\n');
                        }
                        narrative.push_str(t);
                    }
                }

                messages.push(ChatMessage::assistant_tool_calls(tool_calls.clone()));
                let _ = store.append_message(
                    &adventure.id,
                    &HistoryMessage {
                        role: "assistant".to_string(),
                        content: text,
                        tool_calls: Some(serde_json::to_value(&tool_calls).unwrap_or_default()),
                        tool_call_id: None,
                        timestamp: chrono::Utc::now(),
                    },
                );

                for tc in &tool_calls {
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_default();

                    let exec_result =
                        execute_tool_call(&mut adventure, &tc.function.name, &args);

                    match exec_result {
                        Ok(ToolExecResult::Immediate(result)) => {
                            let result_str = serde_json::to_string(&result).unwrap_or_default();
                            messages.push(ChatMessage::tool_result(&tc.id, &result_str));
                            let _ = store.append_message(
                                &adventure.id,
                                &HistoryMessage {
                                    role: "tool".to_string(),
                                    content: Some(result_str),
                                    tool_calls: None,
                                    tool_call_id: Some(tc.id.clone()),
                                    timestamp: chrono::Utc::now(),
                                },
                            );
                        }

                        Ok(ToolExecResult::PendingDiceRoll {
                            dice_type,
                            count,
                            modifier,
                            dc,
                            description,
                            success_probability,
                        }) => {
                            store
                                .save_adventure(&adventure)
                                .map_err(|e| err_internal(&format!("Save error: {}", e)))?;

                            let _ = store.append_display_event(
                                &adventure.id,
                                &DisplayEvent {
                                    event_type: "dice_roll_request".to_string(),
                                    data: serde_json::json!({
                                        "dice_type": dice_type,
                                        "count": count,
                                        "modifier": modifier,
                                        "dc": dc,
                                        "description": description,
                                        "success_probability": success_probability,
                                        "tool_call_id": tc.id,
                                    }),
                                    timestamp: chrono::Utc::now(),
                                },
                            );

                            return Ok(ToolLoopResult {
                                adventure,
                                narrative,
                                pending: Some(PendingInfo {
                                    pending_type: "dice_roll".to_string(),
                                    dice_type: Some(dice_type),
                                    count: Some(count),
                                    modifier: Some(modifier),
                                    dc: Some(dc),
                                    description: Some(description),
                                    success_probability: Some(success_probability),
                                    choices: None,
                                    allow_custom_input: None,
                                    prompt: None,
                                }),
                                total_cost: Some(CostInfo {
                                    prompt_tokens: total_prompt,
                                    completion_tokens: total_completion,
                                    cost_usd: {
                                        let u = TokenUsage {
                                            prompt_tokens: total_prompt,
                                            completion_tokens: total_completion,
                                        };
                                        model_cost(&model, &u)
                                    },
                                }),
                            });
                        }

                        Ok(ToolExecResult::PendingChoices {
                            choices,
                            allow_custom_input,
                            prompt,
                        }) => {
                            store
                                .save_adventure(&adventure)
                                .map_err(|e| err_internal(&format!("Save error: {}", e)))?;

                            let _ = store.append_display_event(
                                &adventure.id,
                                &DisplayEvent {
                                    event_type: "choices".to_string(),
                                    data: serde_json::json!({
                                        "choices": &choices,
                                        "prompt": &prompt,
                                        "allow_custom_input": allow_custom_input,
                                        "tool_call_id": tc.id,
                                    }),
                                    timestamp: chrono::Utc::now(),
                                },
                            );

                            return Ok(ToolLoopResult {
                                adventure,
                                narrative,
                                pending: Some(PendingInfo {
                                    pending_type: "choices".to_string(),
                                    dice_type: None,
                                    count: None,
                                    modifier: None,
                                    dc: None,
                                    description: None,
                                    success_probability: None,
                                    choices: Some(choices),
                                    allow_custom_input: Some(allow_custom_input),
                                    prompt: Some(prompt),
                                }),
                                total_cost: Some(CostInfo {
                                    prompt_tokens: total_prompt,
                                    completion_tokens: total_completion,
                                    cost_usd: {
                                        let u = TokenUsage {
                                            prompt_tokens: total_prompt,
                                            completion_tokens: total_completion,
                                        };
                                        model_cost(&model, &u)
                                    },
                                }),
                            });
                        }

                        Ok(ToolExecResult::CombatStarted) => {
                            let combat_info =
                                "Combat has begun! Initiative order established. The engine will handle turn order and mechanics. Narrate the start of combat dramatically.";
                            messages.push(ChatMessage::tool_result(&tc.id, combat_info));
                            let _ = store.append_message(
                                &adventure.id,
                                &HistoryMessage {
                                    role: "tool".to_string(),
                                    content: Some(combat_info.to_string()),
                                    tool_calls: None,
                                    tool_call_id: Some(tc.id.clone()),
                                    timestamp: chrono::Utc::now(),
                                },
                            );

                            // Run enemy turns until player turn
                            run_enemy_turns(&mut adventure);

                            store
                                .save_adventure(&adventure)
                                .map_err(|e| err_internal(&format!("Save error: {}", e)))?;
                            // Continue the loop so LLM narrates the combat start
                        }

                        Err(e) => {
                            let err_msg = format!("Error: {}", e);
                            messages.push(ChatMessage::tool_result(&tc.id, &err_msg));
                        }
                    }
                }

                // Save after processing all tool calls in this iteration
                store
                    .save_adventure(&adventure)
                    .map_err(|e| err_internal(&format!("Save error: {}", e)))?;
            }
        }
    }

    // Hit iteration limit
    store.save_adventure(&adventure).ok();
    Ok(ToolLoopResult {
        adventure,
        narrative,
        pending: None,
        total_cost: Some(CostInfo {
            prompt_tokens: total_prompt,
            completion_tokens: total_completion,
            cost_usd: {
                let u = TokenUsage {
                    prompt_tokens: total_prompt,
                    completion_tokens: total_completion,
                };
                model_cost(&model, &u)
            },
        }),
    })
}

/// Advance enemy turns until it is the player's turn (or combat ends).
fn run_enemy_turns(adventure: &mut AdventureState) {
    let max_iters = 50; // safety limit
    for _ in 0..max_iters {
        if !adventure.combat.active {
            break;
        }
        match adventure.combat.current_combatant().cloned() {
            Some(CombatantId::Player) => break,
            Some(CombatantId::Enemy(idx)) => {
                let _ = adventure
                    .combat
                    .execute_enemy_turn(idx, &mut adventure.character);
                if adventure.character.hp <= 0 {
                    adventure.combat.end();
                    break;
                }
                adventure.combat.next_turn();
            }
            None => break,
        }
    }
}

// ---------------------------------------------------------------------------
// Auth endpoint
// ---------------------------------------------------------------------------

async fn login_handler(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match state.user_store.authenticate(&req.username, &req.password) {
        Ok(user) => {
            let role = user.role.to_string();
            match state.jwt_manager.create_token(&user.username, &role) {
                Ok(token) => Json(LoginResponse {
                    token,
                    username: user.username,
                    role,
                })
                .into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Adventure CRUD
// ---------------------------------------------------------------------------

async fn list_adventures(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    match store.list_adventures() {
        Ok(adventures) => Json(serde_json::json!({ "adventures": adventures })).into_response(),
        Err(e) => err_internal(&e.to_string()).into_response(),
    }
}

async fn create_adventure(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Json(req): Json<CreateAdventureRequest>,
) -> impl IntoResponse {
    let race = parse_race(&req.race);
    let class = parse_class(req.class.as_deref().unwrap_or("warrior"));
    let stats = if let Some(ref s) = req.stats {
        Stats {
            strength: s.strength,
            dexterity: s.dexterity,
            constitution: s.constitution,
            intelligence: s.intelligence,
            wisdom: s.wisdom,
            charisma: s.charisma,
        }
    } else {
        Stats { strength: 10, dexterity: 10, constitution: 10, intelligence: 10, wisdom: 10, charisma: 10 }
    };
    let adventure = if let Some(ref bg_str) = req.background {
        let bg = crate::engine::backgrounds::Background::from_str(bg_str)
            .unwrap_or_default();
        AdventureState::new_with_background(req.name, req.character_name, race, bg, &req.scenario)
    } else {
        AdventureState::new(req.name, req.character_name, race, class, stats, &req.scenario)
    };
    let store = make_store(&state, &user.username);

    if let Err(e) = store.create_adventure(adventure.clone()) {
        return err_internal(&e.to_string()).into_response();
    }

    // Build messages for the initial LLM call
    let system = ChatMessage::system(&build_system_prompt(&adventure));
    let start_prompt = adventure_start_prompt(&req.scenario);
    let user_msg = ChatMessage::user(&start_prompt);
    let messages = vec![system, user_msg];

    let _ = store.append_message(
        &adventure.id,
        &HistoryMessage {
            role: "user".to_string(),
            content: Some(start_prompt),
            tool_calls: None,
            tool_call_id: None,
            timestamp: chrono::Utc::now(),
        },
    );

    // Run the LLM tool loop to get the opening narrative
    match run_tool_loop(&state, &store, adventure, messages, &user.username).await {
        Ok(result) => Json(game_response(
            &result.adventure,
            Some(result.narrative),
            result.pending,
            result.total_cost,
        ))
        .into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_adventure(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    match store.load_adventure(&id) {
        Ok(adventure) => {
            Json(game_response(&adventure, None, None, None)).into_response()
        }
        Err(_) => err_not_found("Adventure not found").into_response(),
    }
}

async fn delete_adventure(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    match store.delete_adventure(&id) {
        Ok(()) => Json(serde_json::json!({"deleted": true})).into_response(),
        Err(e) => err_internal(&e.to_string()).into_response(),
    }
}

async fn get_history(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    match store.load_display_history(&id) {
        Ok(events) => Json(serde_json::json!({ "events": events })).into_response(),
        Err(e) => err_internal(&e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Game actions (LLM)
// ---------------------------------------------------------------------------

async fn send_message(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<MessageRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    // Apply condition effects
    let condition_effects = apply_turn_effects(&mut adventure);
    if !condition_effects.is_empty() {
        store.save_adventure(&adventure).ok();
    }

    let mut messages = load_messages_for_adventure(&store, &adventure);

    // Refresh system prompt with current state
    if !messages.is_empty() {
        messages[0] = ChatMessage::system(&build_system_prompt(&adventure));
    }

    // Add condition effects context if any
    if !condition_effects.is_empty() {
        let effects_text = format!(
            "[SYSTEM: Start-of-turn condition effects applied: {}]",
            condition_effects.join("; ")
        );
        messages.push(ChatMessage::system(&effects_text));
    }

    messages.push(ChatMessage::user(&req.content));
    let _ = store.append_message(
        &id,
        &HistoryMessage {
            role: "user".to_string(),
            content: Some(req.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            timestamp: chrono::Utc::now(),
        },
    );

    match run_tool_loop(&state, &store, adventure, messages, &user.username).await {
        Ok(result) => Json(game_response(
            &result.adventure,
            Some(result.narrative),
            result.pending,
            result.total_cost,
        ))
        .into_response(),
        Err(e) => e.into_response(),
    }
}

async fn send_choice(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<ChoiceRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    // Load history and find the pending choices tool_call_id from the last display event
    let display_events = store.load_display_history(&id).unwrap_or_default();
    let tool_call_id = display_events
        .iter()
        .rev()
        .find(|e| e.event_type == "choices")
        .and_then(|e| e.data.get("tool_call_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut messages = load_messages_for_adventure(&store, &adventure);

    if let Some(ref tc_id) = tool_call_id {
        let tool_result =
            ChatMessage::tool_result(tc_id, &format!("Player chose: {}", req.text));
        messages.push(tool_result);

        let _ = store.append_message(
            &id,
            &HistoryMessage {
                role: "tool".to_string(),
                content: Some(format!("Player chose: {}", req.text)),
                tool_calls: None,
                tool_call_id: Some(tc_id.clone()),
                timestamp: chrono::Utc::now(),
            },
        );
        let _ = store.append_display_event(
            &id,
            &DisplayEvent {
                event_type: "choice_selected".to_string(),
                data: serde_json::json!({"text": req.text, "index": req.index}),
                timestamp: chrono::Utc::now(),
            },
        );
    } else {
        // No pending tool call id found, treat as a plain user message
        messages.push(ChatMessage::user(&req.text));
        let _ = store.append_message(
            &id,
            &HistoryMessage {
                role: "user".to_string(),
                content: Some(req.text.clone()),
                tool_calls: None,
                tool_call_id: None,
                timestamp: chrono::Utc::now(),
            },
        );
    }

    match run_tool_loop(&state, &store, adventure, messages, &user.username).await {
        Ok(result) => Json(game_response(
            &result.adventure,
            Some(result.narrative),
            result.pending,
            result.total_cost,
        ))
        .into_response(),
        Err(e) => e.into_response(),
    }
}

async fn roll_dice(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    // Find the pending dice roll from the last display event
    let display_events = store.load_display_history(&id).unwrap_or_default();
    let pending = display_events
        .iter()
        .rev()
        .find(|e| e.event_type == "dice_roll_request");

    let pending_data = match pending {
        Some(e) => e.data.clone(),
        None => return err_json("no_pending_roll", "No pending dice roll").into_response(),
    };

    let dice_type = pending_data["dice_type"]
        .as_str()
        .unwrap_or("d20")
        .to_string();
    let count = pending_data["count"].as_u64().unwrap_or(1) as u32;
    let modifier = pending_data["modifier"].as_i64().unwrap_or(0) as i32;
    let dc = pending_data["dc"].as_i64().unwrap_or(10) as i32;
    let description = pending_data["description"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let tool_call_id = pending_data["tool_call_id"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Perform the roll
    let result = DiceRoller::roll_with_dc(&dice_type, count, modifier, dc, &description);
    let result_json = serde_json::to_string(&result).unwrap_or_default();

    let _ = store.append_display_event(
        &id,
        &DisplayEvent {
            event_type: "dice_result".to_string(),
            data: serde_json::json!({
                "rolls": result.rolls,
                "total": result.total,
                "dc": dc,
                "success": result.success,
                "description": description,
            }),
            timestamp: chrono::Utc::now(),
        },
    );

    let mut messages = load_messages_for_adventure(&store, &adventure);
    messages.push(ChatMessage::tool_result(&tool_call_id, &result_json));

    let _ = store.append_message(
        &id,
        &HistoryMessage {
            role: "tool".to_string(),
            content: Some(result_json),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
            timestamp: chrono::Utc::now(),
        },
    );

    match run_tool_loop(&state, &store, adventure, messages, &user.username).await {
        Ok(result) => Json(game_response(
            &result.adventure,
            Some(result.narrative),
            result.pending,
            result.total_cost,
        ))
        .into_response(),
        Err(e) => e.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Combat
// ---------------------------------------------------------------------------

async fn combat_action(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<CombatActionRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    if !adventure.combat.active {
        return err_json("no_combat", "No active combat").into_response();
    }
    if !adventure.combat.is_player_turn() {
        return err_json("not_your_turn", "It is not the player's turn").into_response();
    }

    #[allow(unused_assignments)]
    let mut action_description = String::new();

    match req.action_id.as_str() {
        "attack" => {
            if adventure.combat.action_economy.actions == 0 {
                return err_json("no_action", "No actions remaining").into_response();
            }
            adventure.combat.action_economy.actions -= 1;

            let target_name = req.target.as_deref().unwrap_or("enemy");
            let weapon = adventure.equipment.equipped_weapon();
            let (weapon_name, damage_dice, stat_mod, weapon_attack_bonus) =
                if let Some(w) = weapon {
                    let stat_name = w.stats.damage_modifier_stat.as_deref().unwrap_or("str");
                    let mod_val = if w.stats.is_finesse {
                        let str_mod =
                            adventure.character.stats.modifier_for("str").unwrap_or(0);
                        let dex_mod =
                            adventure.character.stats.modifier_for("dex").unwrap_or(0);
                        std::cmp::max(str_mod, dex_mod)
                    } else {
                        adventure
                            .character
                            .stats
                            .modifier_for(stat_name)
                            .unwrap_or(0)
                    };
                    let dice = w.stats.damage_dice.as_deref().unwrap_or("1d4").to_string();
                    (w.display_name(), dice, mod_val, w.stats.attack_bonus)
                } else {
                    (
                        "Unarmed".to_string(),
                        "1d4".to_string(),
                        adventure
                            .character
                            .stats
                            .modifier_for("str")
                            .unwrap_or(0),
                        0,
                    )
                };

            let prof = adventure.character.proficiency_bonus();
            let equip_atk = adventure.equipment.stat_bonuses().attack_bonus;
            let attack =
                DiceRoller::roll("d20", 1, stat_mod + prof + weapon_attack_bonus + equip_atk);

            let target_ac = adventure
                .combat
                .find_enemy_mut(target_name)
                .map(|e| e.ac)
                .unwrap_or(10);
            let hit = attack.total >= target_ac;
            let damage = if hit {
                let d = DiceRoller::roll(&damage_dice, 1, stat_mod);
                let dmg = std::cmp::max(d.total, 1);
                if let Some(enemy) = adventure.combat.find_enemy_mut(target_name) {
                    enemy.hp -= dmg;
                }
                dmg
            } else {
                0
            };

            action_description = if hit {
                format!(
                    "{} attacks {} with {} (rolled {} vs AC {}): HIT for {} damage!",
                    adventure.character.name,
                    target_name,
                    weapon_name,
                    attack.total,
                    target_ac,
                    damage
                )
            } else {
                format!(
                    "{} attacks {} with {} (rolled {} vs AC {}): MISS!",
                    adventure.character.name, target_name, weapon_name, attack.total, target_ac
                )
            };
            adventure.combat.combat_log.push(action_description.clone());

            // Check if all enemies dead
            if adventure.combat.all_enemies_dead() {
                let xp = adventure.combat.enemies.len() as u32 * 50;
                adventure.combat.end();
                adventure.character.xp += xp;
                adventure.character.check_level_up();
                store.save_adventure(&adventure).ok();

                // Ask LLM to narrate victory
                let messages = {
                    let mut m = load_messages_for_adventure(&store, &adventure);
                    m.push(ChatMessage::user(
                        "Combat is over. All enemies defeated. Narrate the victory and present choices for what to do next.",
                    ));
                    let _ = store.append_message(
                        &id,
                        &HistoryMessage {
                            role: "user".to_string(),
                            content: Some("Combat is over. All enemies defeated. Narrate the victory and present choices for what to do next.".to_string()),
                            tool_calls: None,
                            tool_call_id: None,
                            timestamp: chrono::Utc::now(),
                        },
                    );
                    m
                };

                match run_tool_loop(&state, &store, adventure, messages, &user.username).await {
                    Ok(result) => {
                        let mut narr = action_description;
                        if !result.narrative.is_empty() {
                            narr.push('\n');
                            narr.push_str(&result.narrative);
                        }
                        return Json(game_response(
                            &result.adventure,
                            Some(narr),
                            result.pending,
                            result.total_cost,
                        ))
                        .into_response();
                    }
                    Err(e) => return e.into_response(),
                }
            }
        }

        "dodge" => {
            if adventure.combat.action_economy.actions == 0 {
                return err_json("no_action", "No actions remaining").into_response();
            }
            adventure.combat.action_economy.actions -= 1;
            adventure.combat.player_dodging = true;
            action_description = format!(
                "{} takes the Dodge action. Attacks against them have disadvantage.",
                adventure.character.name
            );
            adventure.combat.combat_log.push(action_description.clone());
        }

        "dash" => {
            if adventure.combat.action_economy.actions == 0 {
                return err_json("no_action", "No actions remaining").into_response();
            }
            adventure.combat.action_economy.actions -= 1;
            adventure.combat.action_economy.movement_remaining += 30;
            action_description = format!(
                "{} dashes! Movement doubled this turn.",
                adventure.character.name
            );
            adventure.combat.combat_log.push(action_description.clone());
        }

        "use_item" => {
            if adventure.combat.action_economy.actions == 0 {
                return err_json("no_action", "No actions remaining").into_response();
            }
            let potion_idx = adventure
                .inventory
                .items
                .iter()
                .position(|i| i.item_type == crate::engine::inventory::ItemType::Potion);
            if let Some(idx) = potion_idx {
                adventure.combat.action_economy.actions -= 1;
                let potion_name = adventure.inventory.items[idx].name.clone();
                if adventure.inventory.items[idx].quantity > 1 {
                    adventure.inventory.items[idx].quantity -= 1;
                } else {
                    adventure.inventory.items.remove(idx);
                }
                let healing = DiceRoller::roll("d4", 2, 2);
                adventure.character.hp = std::cmp::min(
                    adventure.character.hp + healing.total,
                    adventure.character.max_hp,
                );
                action_description = format!(
                    "{} drinks {}! Healed {} HP (now {}/{})",
                    adventure.character.name,
                    potion_name,
                    healing.total,
                    adventure.character.hp,
                    adventure.character.max_hp
                );
                adventure.combat.combat_log.push(action_description.clone());
            } else {
                return err_json("no_item", "No potions available").into_response();
            }
        }

        "second_wind" => {
            if adventure.combat.action_economy.bonus_actions == 0 {
                return err_json("no_bonus", "No bonus actions remaining").into_response();
            }
            adventure.combat.action_economy.bonus_actions -= 1;
            let healing =
                DiceRoller::roll("d10", 1, adventure.character.level as i32);
            adventure.character.hp = std::cmp::min(
                adventure.character.hp + healing.total,
                adventure.character.max_hp,
            );
            action_description = format!(
                "{} uses Second Wind! Healed {} HP (now {}/{})",
                adventure.character.name,
                healing.total,
                adventure.character.hp,
                adventure.character.max_hp
            );
            adventure.combat.combat_log.push(action_description.clone());
        }

        "cunning_hide" => {
            if adventure.combat.action_economy.bonus_actions == 0 {
                return err_json("no_bonus", "No bonus actions remaining").into_response();
            }
            adventure.combat.action_economy.bonus_actions -= 1;
            action_description = format!(
                "{} hides in the shadows! Next attack has advantage.",
                adventure.character.name
            );
            adventure.combat.combat_log.push(action_description.clone());
        }

        "healing_word" => {
            if adventure.combat.action_economy.bonus_actions == 0 {
                return err_json("no_bonus", "No bonus actions remaining").into_response();
            }
            adventure.combat.action_economy.bonus_actions -= 1;
            let wis_mod = adventure
                .character
                .stats
                .modifier_for("wis")
                .unwrap_or(0);
            let healing = DiceRoller::roll("d4", 1, wis_mod);
            adventure.character.hp = std::cmp::min(
                adventure.character.hp + healing.total,
                adventure.character.max_hp,
            );
            action_description = format!(
                "{} casts Healing Word! Healed {} HP (now {}/{})",
                adventure.character.name,
                healing.total,
                adventure.character.hp,
                adventure.character.max_hp
            );
            adventure.combat.combat_log.push(action_description.clone());
        }


        "flee" => {
            if adventure.combat.action_economy.actions == 0 {
                return err_json("no_action", "No actions remaining").into_response();
            }
            adventure.combat.action_economy.actions -= 1;

            let dex_mod = adventure.character.stats.modifier_for("dex").unwrap_or(0);
            let living = adventure.combat.living_enemies().len() as i32;
            let flee_dc = (10 + living * 2 - adventure.combat.flee_attempts as i32 * 2).max(5);
            let roll = DiceRoller::roll("d20", 1, dex_mod);
            let success = roll.total >= flee_dc;

            if success {
                action_description = format!(
                    "{} attempts to flee (rolled {} vs DC {}): SUCCESS! Escaped combat!",
                    adventure.character.name, roll.total, flee_dc
                );
                adventure.combat.combat_log.push(action_description.clone());
                adventure.combat.end();
                store.save_adventure(&adventure).ok();

                // Ask LLM to narrate the escape
                let messages = {
                    let mut m = load_messages_for_adventure(&store, &adventure);
                    m.push(ChatMessage::user(
                        "The player successfully fled from combat. Narrate their narrow escape and present choices for what to do next.",
                    ));
                    let _ = store.append_message(
                        &id,
                        &HistoryMessage {
                            role: "user".to_string(),
                            content: Some("The player successfully fled from combat. Narrate their narrow escape and present choices for what to do next.".to_string()),
                            tool_calls: None,
                            tool_call_id: None,
                            timestamp: chrono::Utc::now(),
                        },
                    );
                    m
                };

                match run_tool_loop(&state, &store, adventure, messages, &user.username).await {
                    Ok(result) => {
                        let mut narr = action_description;
                        if !result.narrative.is_empty() {
                            narr.push('\n');
                            narr.push_str(&result.narrative);
                        }
                        return Json(game_response(
                            &result.adventure,
                            Some(narr),
                            result.pending,
                            result.total_cost,
                        ))
                        .into_response();
                    }
                    Err(e) => return e.into_response(),
                }
            } else {
                adventure.combat.flee_attempts += 1;
                let next_dc = (10 + living * 2 - adventure.combat.flee_attempts as i32 * 2).max(5);
                action_description = format!(
                    "{} attempts to flee (rolled {} vs DC {}): FAILED! The enemies block the escape. (Next attempt DC {})",
                    adventure.character.name, roll.total, flee_dc, next_dc
                );
                adventure.combat.combat_log.push(action_description.clone());
            }
        }

        "end_turn" => {
            adventure.combat.next_turn();
            run_enemy_turns(&mut adventure);
            store.save_adventure(&adventure).ok();
            return Json(game_response(
                &adventure,
                Some("Turn ended.".to_string()),
                None,
                None,
            ))
            .into_response();
        }

        _ => {
            return err_json("unknown_action", &format!("Unknown combat action: {}", req.action_id))
                .into_response();
        }
    }

    store.save_adventure(&adventure).ok();
    Json(game_response(
        &adventure,
        Some(action_description),
        None,
        None,
    ))
    .into_response()
}

// ---------------------------------------------------------------------------
// Equipment
// ---------------------------------------------------------------------------

async fn equip_item(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<EquipRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({"item_name": req.item_name});
    match execute_tool_call(&mut adventure, "equip_item", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Failed to equip item").into_response(),
    }
}

async fn unequip_item(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<UnequipRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({"slot": req.slot});
    match execute_tool_call(&mut adventure, "unequip_slot", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Failed to unequip item").into_response(),
    }
}

// ---------------------------------------------------------------------------
// Direct engine endpoints (bypass LLM)
// ---------------------------------------------------------------------------

async fn engine_hp(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<HpRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({"delta": req.delta, "reason": req.reason});
    match execute_tool_call(&mut adventure, "update_hp", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Failed to update HP").into_response(),
    }
}

async fn engine_item(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<ItemRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({"item_id": req.item_id});
    match execute_tool_call(&mut adventure, "give_item", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Failed to give item").into_response(),
    }
}

async fn engine_gold(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<GoldRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({"amount": req.amount, "reason": "API grant"});
    match execute_tool_call(&mut adventure, "give_gold", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Failed to give gold").into_response(),
    }
}

async fn engine_xp(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<XpRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({"amount": req.amount, "reason": req.reason});
    match execute_tool_call(&mut adventure, "award_xp", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Failed to award XP").into_response(),
    }
}

async fn engine_condition(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<ConditionRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let tool_name = match req.action.as_str() {
        "add" => "add_condition",
        "remove" => "remove_condition",
        _ => return err_json("invalid_action", "action must be 'add' or 'remove'").into_response(),
    };

    let args = serde_json::json!({"condition": req.condition});
    match execute_tool_call(&mut adventure, tool_name, &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Failed to modify condition").into_response(),
    }
}

async fn engine_combat(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<StartCombatRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let enemies: Vec<Enemy> = req
        .enemies
        .into_iter()
        .map(|e| Enemy {
            name: e.name,
            hp: e.hp,
            max_hp: e.hp,
            ac: e.ac,
            attacks: e
                .attacks
                .into_iter()
                .map(|a| EnemyAttack {
                    name: a.name,
                    damage_dice: a.damage_dice,
                    damage_modifier: a.damage_modifier,
                    to_hit_bonus: a.to_hit_bonus,
                })
                .collect(),
        
            enemy_type: None,
            tier: None,
        })
        .collect();

    let dex_mod = adventure
        .character
        .stats
        .modifier_for("dex")
        .unwrap_or(0);
    adventure.combat.start(enemies, dex_mod);

    // Run enemy turns until player turn
    run_enemy_turns(&mut adventure);
    store.save_adventure(&adventure).ok();

    Json(game_response(&adventure, Some("Combat started!".to_string()), None, None)).into_response()
}

async fn engine_roll(
    Extension(_user): Extension<AuthUser>,
    State(_state): State<Arc<ApiState>>,
    Path(_id): Path<String>,
    Json(req): Json<RollRequest>,
) -> impl IntoResponse {
    if let Some(dc) = req.dc {
        let result =
            DiceRoller::roll_with_dc(&req.dice, req.count, req.modifier, dc, "");
        Json(serde_json::to_value(&result).unwrap_or_default()).into_response()
    } else {
        let result = DiceRoller::roll(&req.dice, req.count, req.modifier);
        Json(serde_json::to_value(&result).unwrap_or_default()).into_response()
    }
}

// ---------------------------------------------------------------------------
// Item database endpoints
// ---------------------------------------------------------------------------

async fn list_items() -> impl IntoResponse {
    let ids = equipment::all_item_ids();
    let items: Vec<serde_json::Value> = ids
        .iter()
        .filter_map(|id| {
            equipment::get_item(id).map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "name": item.display_name(),
                    "description": item.description,
                    "item_type": item.item_type,
                    "slot": item.slot,
                    "rarity": item.rarity,
                    "weight": item.weight,
                    "value_gp": item.value_gp,
                    "stats": item.stats,
                    "enchantment": item.enchantment,
                })
            })
        })
        .collect();
    Json(serde_json::json!({ "items": items }))
}

async fn get_item_by_id(Path(item_id): Path<String>) -> impl IntoResponse {
    match equipment::get_item(&item_id) {
        Some(item) => Json(serde_json::json!({
            "id": item.id,
            "name": item.display_name(),
            "description": item.description,
            "item_type": item.item_type,
            "slot": item.slot,
            "rarity": item.rarity,
            "weight": item.weight,
            "value_gp": item.value_gp,
            "stats": item.stats,
            "enchantment": item.enchantment,
        }))
        .into_response(),
        None => err_not_found("Item not found").into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Crafting endpoints
// ---------------------------------------------------------------------------

async fn list_recipes(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let graph = &*CRAFTING_GRAPH;
    let skill_filter = params.get("skill").map(|s: &String| s.as_str());
    let tier_filter = params.get("tier").and_then(|t: &String| t.parse::<u8>().ok());

    let recipes: Vec<serde_json::Value> = graph.recipes.iter()
        .filter(|r| {
            if let Some(sf) = skill_filter {
                if r.skill.skill_id() != sf { return false; }
            }
            if let Some(tf) = tier_filter {
                if r.tier != tf { return false; }
            }
            true
        })
        .map(|r| {
            let inputs: Vec<serde_json::Value> = r.inputs.iter().map(|(id, qty)| {
                let name = graph.materials.get(id).map(|m| m.name.as_str()).unwrap_or(id.as_str());
                serde_json::json!({"id": id, "name": name, "quantity": qty})
            }).collect();
            let output_name = graph.materials.get(&r.output).map(|m| m.name.as_str()).unwrap_or(r.output.as_str());
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "skill": r.skill.name(),
                "skill_id": r.skill.skill_id(),
                "skill_rank": r.skill_rank,
                "tier": r.tier,
                "inputs": inputs,
                "output": r.output,
                "output_name": output_name,
                "output_qty": r.output_qty,
            })
        })
        .collect();

    Json(serde_json::json!({ "recipes": recipes }))
}

async fn get_recipe(Path(recipe_id): Path<String>) -> impl IntoResponse {
    let graph = &*CRAFTING_GRAPH;
    match graph.recipes.iter().find(|r| r.id == recipe_id) {
        Some(r) => {
            let inputs: Vec<serde_json::Value> = r.inputs.iter().map(|(id, qty)| {
                let name = graph.materials.get(id).map(|m| m.name.as_str()).unwrap_or(id.as_str());
                serde_json::json!({"id": id, "name": name, "quantity": qty})
            }).collect();
            let output_name = graph.materials.get(&r.output).map(|m| m.name.as_str()).unwrap_or(r.output.as_str());
            Json(serde_json::json!({
                "id": r.id,
                "name": r.name,
                "skill": r.skill.name(),
                "skill_id": r.skill.skill_id(),
                "skill_rank": r.skill_rank,
                "tier": r.tier,
                "inputs": inputs,
                "output": r.output,
                "output_name": output_name,
                "output_qty": r.output_qty,
            }))
            .into_response()
        }
        None => err_not_found("Recipe not found").into_response(),
    }
}

async fn list_materials() -> impl IntoResponse {
    let graph = &*CRAFTING_GRAPH;
    let materials: Vec<serde_json::Value> = graph.materials.values()
        .map(|m| serde_json::json!({
            "id": m.id,
            "name": m.name,
            "tier": m.tier,
            "source": format!("{:?}", m.source),
        }))
        .collect();
    Json(serde_json::json!({ "materials": materials }))
}

async fn craft_item(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<CraftRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({"recipe_id": req.recipe_id});
    match execute_tool_call(&mut adventure, "craft_item", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Crafting failed").into_response(),
    }
}


// ---------------------------------------------------------------------------
// Gather endpoint
// ---------------------------------------------------------------------------

async fn gather_materials(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let args = serde_json::json!({});
    match execute_tool_call(&mut adventure, "gather", &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Gathering failed").into_response(),
    }
}
// ---------------------------------------------------------------------------
// Skill endpoint
// ---------------------------------------------------------------------------

async fn engine_skill(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<SkillRequest>,
) -> impl IntoResponse {
    let store = make_store(&state, &user.username);
    let mut adventure = match store.load_adventure(&id) {
        Ok(a) => a,
        Err(_) => return err_not_found("Adventure not found").into_response(),
    };

    let (tool_name, args) = match req.action.as_str() {
        "get" => ("get_skills", serde_json::json!({})),
        "improve" => {
            let skill_id = match &req.skill_id {
                Some(s) => s.clone(),
                None => return err_json("missing_field", "skill_id required for improve").into_response(),
            };
            ("improve_skill", serde_json::json!({"skill_id": skill_id}))
        }
        "award_xp" => {
            let skill_id = match &req.skill_id {
                Some(s) => s.clone(),
                None => return err_json("missing_field", "skill_id required for award_xp").into_response(),
            };
            let amount = req.amount.unwrap_or(0);
            ("award_skill_xp", serde_json::json!({"skill_id": skill_id, "amount": amount}))
        }
        _ => return err_json("invalid_action", "action must be 'get', 'improve', or 'award_xp'").into_response(),
    };

    match execute_tool_call(&mut adventure, tool_name, &args) {
        Ok(ToolExecResult::Immediate(result)) => {
            store.save_adventure(&adventure).ok();
            Json(serde_json::json!({
                "result": result,
                "state": build_state_with_map(&adventure),
            }))
            .into_response()
        }
        _ => err_internal("Skill operation failed").into_response(),
    }
}

// ---------------------------------------------------------------------------
// Backgrounds endpoint
// ---------------------------------------------------------------------------

async fn list_backgrounds() -> impl IntoResponse {
    use crate::engine::backgrounds::Background;
    let backgrounds = Background::all();
    let result: Vec<serde_json::Value> = backgrounds.iter().map(|b| {
        serde_json::json!({
            "name": b.name(),
            "description": b.description(),
            "starting_gold": b.starting_gold(),
            "starting_skills": b.starting_skills(),
        })
    }).collect();
    Json(serde_json::json!({ "backgrounds": result }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_race(s: &str) -> Race {
    match s.to_lowercase().as_str() {
        "elf" => Race::Elf,
        "dwarf" => Race::Dwarf,
        "orc" => Race::Orc,
        "halfling" => Race::Halfling,
        _ => Race::Human,
    }
}

fn parse_class(s: &str) -> Class {
    match s.to_lowercase().as_str() {
        "mage" | "wizard" => Class::Mage,
        "rogue" | "thief" => Class::Rogue,
        "cleric" | "priest" => Class::Cleric,
        "ranger" => Class::Ranger,
        _ => Class::Warrior,
    }
}
