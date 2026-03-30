//! WebSocket handler with game loop state machine.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
fn build_state_with_map(adventure: &crate::engine::adventure::AdventureState) -> serde_json::Value {    let mut state = serde_json::to_value(adventure).unwrap_or_default();    if let serde_json::Value::Object(ref mut map) = state {        map.insert("map_view".to_string(),            crate::engine::world_map::build_map_view(&adventure.world_position, &adventure.discovery, false));    }    state}
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::auth::AuthUser;
use crate::engine::adventure::AdventureState;
use crate::engine::character::{Class, Race, Stats};
use crate::engine::conditions::apply_turn_effects;
use crate::engine::dice::DiceRoller;
use crate::engine::executor::{execute_tool_call, execute_tool_call_with_shop, ToolExecResult};
use crate::engine::dungeon::generate_tiered_dungeon;
use crate::engine::tower::{tower_definitions, generate_floor, floor_summary, meets_entry_requirement};
use crate::storage::shop_store::ShopStore;
use crate::llm::client::XaiClient;
use crate::llm::pricing::{SessionCost, TokenUsage};
use crate::llm::prompts::{adventure_start_prompt, build_system_prompt};
use crate::llm::tools::build_tool_definitions;
use crate::llm::types::*;
use crate::storage::adventure_store::{AdventureStore, DisplayEvent, HistoryMessage};
use crate::storage::usage_logger::{UsageEntry, UsageLogger};
use crate::engine::combat::CombatantId;
use crate::web::protocol::{ActionInfo, ClientMsg, EnemyInfo, InitiativeInfo, ServerMsg};
use crate::engine::crafting::CRAFTING_GRAPH;

const ALLOWED_MODELS: &[&str] = &[
    "grok-4-1-fast-reasoning",
    "grok-4-1-fast-non-reasoning",
];

struct Session {
    username: String,
    store: AdventureStore,
    usage_logger: UsageLogger,
    adventure: Option<AdventureState>,
    messages: Vec<ChatMessage>,
    pending_roll: Option<PendingRoll>,
    pending_choices: Option<PendingChoices>,
    model: String,
    session_cost: SessionCost,
    precomputed: Option<PrecomputedBranches>,
}

struct PendingRoll {
    dice_type: String,
    count: u32,
    modifier: i32,
    dc: i32,
    description: String,
    tool_call_id: String,
}

struct PendingChoices {
    tool_call_id: String,
}

struct PrecomputedBranches {
    success_text: Arc<Mutex<Option<String>>>,
    failure_text: Arc<Mutex<Option<String>>>,
    success_handle: JoinHandle<()>,
    failure_handle: JoinHandle<()>,
}

pub async fn handle_socket(
    socket: WebSocket,
    user: AuthUser,
    xai_client: Arc<XaiClient>,
    data_dir: std::path::PathBuf,
    default_model: String,
    shop_store: ShopStore,
) {
    let (mut sender, mut receiver) = socket.split();
    let store = AdventureStore::new(&data_dir, &user.username);
    let usage_logger = UsageLogger::new(&data_dir);

    let session = Arc::new(Mutex::new(Session {
        username: user.username.clone(),
        store,
        usage_logger,
        adventure: None,
        messages: Vec::new(),
        pending_roll: None,
        pending_choices: None,
        model: default_model.clone(),
        session_cost: SessionCost::new(),
        precomputed: None,
    }));

    let connected = ServerMsg::Connected {
        username: user.username.clone(),
    };
    let _ = sender
        .send(Message::Text(serde_json::to_string(&connected).unwrap().into()))
        .await;

    // Send model info
    let model_info = ServerMsg::ModelInfo {
        model: default_model,
        available_models: ALLOWED_MODELS.iter().map(|s| s.to_string()).collect(),
    };
    let _ = sender
        .send(Message::Text(serde_json::to_string(&model_info).unwrap().into()))
        .await;

    while let Some(Ok(msg)) = receiver.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let client_msg: ClientMsg = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let err = ServerMsg::Error {
                    code: "parse_error".to_string(),
                    message: format!("Invalid message: {}", e),
                };
                let _ = sender
                    .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                    .await;
                continue;
            }
        };

        let response = handle_client_msg(client_msg, &session, &xai_client, &mut sender, &shop_store).await;

        if let Err(e) = response {
            let err = ServerMsg::Error {
                code: "internal_error".to_string(),
                message: e.to_string(),
            };
            let _ = sender
                .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
                .await;
        }
    }
}

async fn send_msg(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    msg: &ServerMsg,
) {
    let json = serde_json::to_string(msg).unwrap();
    let _ = sender.send(Message::Text(json.into())).await;
}

async fn send_cost_update(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    sess: &Session,
) {
    let session_cost = sess.session_cost.cost_usd(&sess.model);
    let stats = sess.usage_logger.aggregate();
    send_msg(sender, &ServerMsg::CostUpdate {
        session_cost_usd: session_cost,
        prompt_tokens: sess.session_cost.total_prompt_tokens,
        completion_tokens: sess.session_cost.total_completion_tokens,
        today_cost_usd: stats.today.cost_usd,
        week_cost_usd: stats.week.cost_usd,
        month_cost_usd: stats.month.cost_usd,
        total_cost_usd: stats.total.cost_usd,
    }).await;
}

fn log_usage(sess: &Session, usage: &TokenUsage) {
    let cost_usd = crate::llm::pricing::model_cost(&sess.model, usage);
    let _ = sess.usage_logger.log(&UsageEntry {
        ts: chrono::Utc::now(),
        model: sess.model.clone(),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        cost_usd,
        username: sess.username.clone(),
    });
}

async fn handle_client_msg(
    msg: ClientMsg,
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    shop_store: &ShopStore,
) -> anyhow::Result<()> {
    match msg {
        ClientMsg::ViewShop => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                let args = serde_json::json!({});
                match execute_tool_call_with_shop(&mut adv, "view_shop", &args, Some(shop_store)) {
                    Ok(ToolExecResult::Immediate(result)) => {
                        let shop_name = result.get("shop_name").and_then(|v| v.as_str()).unwrap_or("Shop").to_string();
                        let tier = result.get("tier").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                        let items_val = result.get("items").cloned().unwrap_or(serde_json::json!([]));
                        let items: Vec<crate::web::protocol::ShopItemInfo> = serde_json::from_value(items_val).unwrap_or_default();
                        let player_gold = result.get("player_gold").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        sess.adventure = Some(adv);
                        send_msg(sender, &ServerMsg::ShopInventory { shop_name, tier, items, player_gold }).await;
                    }
                    Ok(_) => { sess.adventure = Some(adv); }
                    Err(e) => {
                        sess.adventure = Some(adv);
                        send_msg(sender, &ServerMsg::Error { code: "shop_error".to_string(), message: e.to_string() }).await;
                    }
                }
            } else {
                send_msg(sender, &ServerMsg::Error { code: "no_adventure".to_string(), message: "No active adventure".to_string() }).await;
            }
        }

        ClientMsg::ShopBuy { item_id, quantity } => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                let args = serde_json::json!({"item_id": item_id, "quantity": quantity});
                match execute_tool_call_with_shop(&mut adv, "buy_item", &args, Some(shop_store)) {
                    Ok(ToolExecResult::Immediate(result)) => {
                        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                        let item_name = result.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let error = result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let price = result.get("price").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let gold_remaining = result.get("gold_remaining").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        if success {
                            sess.store.save_adventure(&adv).ok();
                            let state_json = build_state_with_map(&adv);
                            send_msg(sender, &ServerMsg::StateUpdate { state: state_json }).await;
                        }
                        sess.adventure = Some(adv);
                        send_msg(sender, &ServerMsg::ShopBuyResult { success, item_name, price, gold_remaining, error }).await;
                    }
                    Ok(_) => { sess.adventure = Some(adv); }
                    Err(e) => {
                        sess.adventure = Some(adv);
                        send_msg(sender, &ServerMsg::Error { code: "shop_error".to_string(), message: e.to_string() }).await;
                    }
                }
            } else {
                send_msg(sender, &ServerMsg::Error { code: "no_adventure".to_string(), message: "No active adventure".to_string() }).await;
            }
        }

        ClientMsg::ShopSell { item_name } => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                let args = serde_json::json!({"item_name": item_name});
                match execute_tool_call_with_shop(&mut adv, "sell_item", &args, Some(shop_store)) {
                    Ok(ToolExecResult::Immediate(result)) => {
                        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                        let sold_name = result.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let error = result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let gold_earned = result.get("sell_price").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let gold_remaining = result.get("gold_remaining").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        if success {
                            sess.store.save_adventure(&adv).ok();
                            let state_json = build_state_with_map(&adv);
                            send_msg(sender, &ServerMsg::StateUpdate { state: state_json }).await;
                        }
                        sess.adventure = Some(adv);
                        send_msg(sender, &ServerMsg::ShopSellResult { success, item_name: sold_name, gold_earned, gold_remaining, error }).await;
                    }
                    Ok(_) => { sess.adventure = Some(adv); }
                    Err(e) => {
                        sess.adventure = Some(adv);
                        send_msg(sender, &ServerMsg::Error { code: "shop_error".to_string(), message: e.to_string() }).await;
                    }
                }
            } else {
                send_msg(sender, &ServerMsg::Error { code: "no_adventure".to_string(), message: "No active adventure".to_string() }).await;
            }
        }


        // ---------------------------------------------------------------
        // Dungeon messages
        // ---------------------------------------------------------------

        ClientMsg::DungeonEnter { seed, tier } => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if adv.dungeon.is_some() {
                    send_msg(sender, &ServerMsg::Error { code: "already_in_dungeon".into(), message: "Already in a dungeon".into() }).await;
                } else {
                    let s = seed.unwrap_or_else(|| {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
                    });
                    let t = tier.unwrap_or(0).min(10);
                    let dungeon = generate_tiered_dungeon(s, t);
                    let room_json = dungeon.current_room().map(|r| serde_json::json!({
                        "name": r.name, "type": format!("{}", r.room_type),
                        "description": r.description,
                        "exits": r.exits.iter().map(|e| &e.direction).collect::<Vec<_>>(),
                    })).unwrap_or(serde_json::json!(null));
                    let name = dungeon.name.clone();
                    let tier_val = dungeon.tier;
                    let floors = dungeon.floors.len();
                    adv.dungeon = Some(dungeon);
                    sess.store.save_adventure(&adv).ok();
                    send_msg(sender, &ServerMsg::DungeonEntered { name, tier: tier_val, floors, room: room_json }).await;
                    let state = build_state_with_map(&adv);
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                }
                sess.adventure = Some(adv);
            } else {
                send_msg(sender, &ServerMsg::Error { code: "no_adventure".into(), message: "No active adventure".into() }).await;
            }
        }

        ClientMsg::DungeonMove { direction } => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if let Some(ref mut dungeon) = adv.dungeon {
                    // Check skill gates
                    let cf = dungeon.current_floor;
                    let cr = dungeon.current_room;
                    let blocked = if let Some(floor) = dungeon.floors.get(cf) {
                        if let Some(room) = floor.rooms.get(cr) {
                            if let Some(ei) = room.exits.iter().position(|e| e.direction.to_lowercase() == direction.to_lowercase()) {
                                floor.skill_gates.iter().any(|g| g.room_id == cr && g.exit_index == ei)
                            } else { false }
                        } else { false }
                    } else { false };

                    if blocked {
                        send_msg(sender, &ServerMsg::Error { code: "skill_gate_locked".into(), message: "Exit is skill-gated. Use dungeon_skill_check first.".into() }).await;
                    } else {
                        match dungeon.move_to_room(&direction) {
                            Ok(result) => {
                                let room_json = serde_json::json!({
                                    "name": result.room_name, "type": format!("{}", result.room_type),
                                    "description": result.description, "has_enemies": result.has_enemies,
                                    "has_trap": result.has_trap, "exits": result.exits,
                                });
                                sess.store.save_adventure(&adv).ok();
                                send_msg(sender, &ServerMsg::DungeonRoomChanged { room: room_json, floor: result.floor, room_id: result.room_id }).await;
                                let state = build_state_with_map(&adv);
                                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                            }
                            Err(e) => {
                                send_msg(sender, &ServerMsg::Error { code: "move_failed".into(), message: e }).await;
                            }
                        }
                    }
                } else {
                    send_msg(sender, &ServerMsg::Error { code: "not_in_dungeon".into(), message: "Not in a dungeon".into() }).await;
                }
                sess.adventure = Some(adv);
            }
        }

        ClientMsg::DungeonSkillCheck { direction, skill_id } => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if let Some(ref mut dungeon) = adv.dungeon {
                    let cf = dungeon.current_floor;
                    let cr = dungeon.current_room;
                    let gate_info = if let Some(floor) = dungeon.floors.get(cf) {
                        if let Some(room) = floor.rooms.get(cr) {
                            if let Some(ei) = room.exits.iter().position(|e| e.direction.to_lowercase() == direction.to_lowercase()) {
                                floor.skill_gates.iter().find(|g| g.room_id == cr && g.exit_index == ei).cloned()
                            } else { None }
                        } else { None }
                    } else { None };

                    if let Some(gate) = gate_info {
                        let player_rank = adv.skills.get(&skill_id).map(|s| s.rank).unwrap_or(0);
                        if player_rank < gate.required_rank {
                            send_msg(sender, &ServerMsg::Error { code: "insufficient_rank".into(),
                                message: format!("Need {} rank {} but have {}", gate.required_skill, gate.required_rank, player_rank) }).await;
                        } else {
                            let roll = DiceRoller::roll("1d20", 1, player_rank as i32);
                            let success = roll.total >= gate.dc;
                            if success {
                                if let Some(floor) = dungeon.floors.get_mut(cf) {
                                    if let Some(room) = floor.rooms.get(cr) {
                                        if let Some(ei) = room.exits.iter().position(|e| e.direction.to_lowercase() == direction.to_lowercase()) {
                                            floor.skill_gates.retain(|g| !(g.room_id == cr && g.exit_index == ei));
                                        }
                                    }
                                }
                            }
                            sess.store.save_adventure(&adv).ok();
                            send_msg(sender, &ServerMsg::DungeonSkillGateResult {
                                skill: skill_id, roll: roll.total, dc: gate.dc, success,
                            }).await;
                        }
                    } else {
                        send_msg(sender, &ServerMsg::Error { code: "no_gate".into(), message: "No skill gate on that exit".into() }).await;
                    }
                } else {
                    send_msg(sender, &ServerMsg::Error { code: "not_in_dungeon".into(), message: "Not in a dungeon".into() }).await;
                }
                sess.adventure = Some(adv);
            }
        }

        ClientMsg::DungeonActivatePoint { puzzle_id, room_id } => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if let Some(ref mut dungeon) = adv.dungeon {
                    let cf = dungeon.current_floor;
                    if let Some(floor) = dungeon.floors.get_mut(cf) {
                        if let Some(puzzle) = floor.simultaneous_puzzles.iter_mut().find(|p| p.id == puzzle_id) {
                            if puzzle.solved {
                                send_msg(sender, &ServerMsg::Error { code: "already_solved".into(), message: "Puzzle already solved".into() }).await;
                            } else if let Some(point) = puzzle.activation_points.iter_mut().find(|ap| ap.room_id == room_id) {
                                use std::time::{SystemTime, UNIX_EPOCH};
                                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                                point.activated_by = Some(adv.character.name.clone());
                                point.activated_at = Some(now);

                                let activated: Vec<u64> = puzzle.activation_points.iter().filter_map(|ap| ap.activated_at).collect();
                                let all_ok = activated.len() >= puzzle.required_count as usize;
                                let in_window = if all_ok && activated.len() >= 2 {
                                    let mn = activated.iter().min().unwrap();
                                    let mx = activated.iter().max().unwrap();
                                    (mx - mn) <= puzzle.timer_window_ms as u64
                                } else { all_ok };
                                let solved = all_ok && in_window;
                                if solved { puzzle.solved = true; }
                                let ac = activated.len();
                                let rc = puzzle.required_count;

                                sess.store.save_adventure(&adv).ok();
                                send_msg(sender, &ServerMsg::DungeonPuzzleActivation {
                                    puzzle_id, activated_count: ac, required_count: rc, solved,
                                }).await;
                            } else {
                                send_msg(sender, &ServerMsg::Error { code: "no_point".into(), message: "No activation point in that room".into() }).await;
                            }
                        } else {
                            send_msg(sender, &ServerMsg::Error { code: "puzzle_not_found".into(), message: "No such puzzle".into() }).await;
                        }
                    }
                } else {
                    send_msg(sender, &ServerMsg::Error { code: "not_in_dungeon".into(), message: "Not in a dungeon".into() }).await;
                }
                sess.adventure = Some(adv);
            }
        }

        ClientMsg::DungeonRetreat => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if adv.combat.active {
                    send_msg(sender, &ServerMsg::Error { code: "in_combat".into(), message: "Cannot retreat during combat".into() }).await;
                } else if let Some(ref d) = adv.dungeon {
                    let msg = format!("You retreat from {}.", d.name);
                    adv.dungeon = None;
                    sess.store.save_adventure(&adv).ok();
                    send_msg(sender, &ServerMsg::DungeonRetreated { message: msg }).await;
                    let state = build_state_with_map(&adv);
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                } else {
                    send_msg(sender, &ServerMsg::Error { code: "not_in_dungeon".into(), message: "Not in a dungeon".into() }).await;
                }
                sess.adventure = Some(adv);
            }
        }

        ClientMsg::DungeonStatus => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let status = if let Some(ref d) = adv.dungeon {
                    let room = d.current_room();
                    let floor = d.current_floor();
                    serde_json::json!({
                        "in_dungeon": true, "name": d.name, "tier": d.tier,
                        "total_floors": d.floors.len(),
                        "current_floor": d.current_floor, "current_room": d.current_room,
                        "room": room.map(|r| serde_json::json!({
                            "name": r.name, "type": format!("{}", r.room_type),
                            "cleared": r.cleared,
                            "exits": r.exits.iter().map(|e| serde_json::json!({"direction": e.direction, "locked": e.locked})).collect::<Vec<_>>(),
                        })),
                        "floor_info": floor.map(|f| serde_json::json!({
                            "skill_gates": f.skill_gates.len(),
                            "puzzles": f.simultaneous_puzzles.len(),
                            "corruption_enabled": f.corruption_enabled,
                        })),
                    })
                } else {
                    serde_json::json!({"in_dungeon": false})
                };
                send_msg(sender, &ServerMsg::DungeonStatus { status }).await;
            }
        }

        // ---------------------------------------------------------------
        // Tower messages
        // ---------------------------------------------------------------

        ClientMsg::TowerList => {
            let towers = tower_definitions();
            let list: Vec<serde_json::Value> = towers.iter().map(|t| serde_json::json!({
                "id": t.id, "name": t.name, "base_tier": t.base_tier,
                "entry_skill_rank": t.entry_skill_rank, "description": t.description,
            })).collect();
            send_msg(sender, &ServerMsg::TowerList { towers: list }).await;
        }

        ClientMsg::TowerEnter { tower_id } => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if adv.dungeon.is_some() {
                    send_msg(sender, &ServerMsg::Error { code: "already_in_dungeon".into(), message: "Already in a dungeon/tower".into() }).await;
                } else {
                    let towers = tower_definitions();
                    if let Some(tower) = towers.iter().find(|t| t.id == tower_id) {
                        let max_rank = adv.skills.skills.iter().map(|s| s.rank).max().unwrap_or(0);
                        if !meets_entry_requirement(tower, max_rank) {
                            send_msg(sender, &ServerMsg::Error { code: "entry_denied".into(),
                                message: format!("{} requires skill rank {}+", tower.name, tower.entry_skill_rank) }).await;
                        } else {
                            let floor = generate_floor(tower, 0);
                            let tier = floor.tier.round() as u32;
                            let dungeon = generate_tiered_dungeon(tower.seed, tier);
                            let mut td = dungeon;
                            td.name = format!("{} — Floor 0", tower.name);
                            let tower_name = tower.name.clone();
                            let tier_str = format!("{:.1}", floor.tier);
                            adv.dungeon = Some(td);
                            sess.store.save_adventure(&adv).ok();
                            send_msg(sender, &ServerMsg::TowerEntered { tower_name, floor: 0, tier: tier_str }).await;
                            let state = build_state_with_map(&adv);
                            send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                        }
                    } else {
                        send_msg(sender, &ServerMsg::Error { code: "tower_not_found".into(), message: "Unknown tower".into() }).await;
                    }
                }
                sess.adventure = Some(adv);
            }
        }

        ClientMsg::TowerMove { direction } => {
            // Delegate to dungeon move logic
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if let Some(ref mut dungeon) = adv.dungeon {
                    match dungeon.move_to_room(&direction) {
                        Ok(result) => {
                            let room_json = serde_json::json!({
                                "name": result.room_name, "type": format!("{}", result.room_type),
                                "description": result.description, "has_enemies": result.has_enemies,
                                "exits": result.exits,
                            });
                            sess.store.save_adventure(&adv).ok();
                            send_msg(sender, &ServerMsg::DungeonRoomChanged { room: room_json, floor: result.floor, room_id: result.room_id }).await;
                        }
                        Err(e) => send_msg(sender, &ServerMsg::Error { code: "move_failed".into(), message: e }).await,
                    }
                } else {
                    send_msg(sender, &ServerMsg::Error { code: "not_in_tower".into(), message: "Not in a tower".into() }).await;
                }
                sess.adventure = Some(adv);
            }
        }

        ClientMsg::TowerAscend => {
            let mut sess = session.lock().await;
            if let Some(mut adv) = sess.adventure.take() {
                if let Some(ref mut dungeon) = adv.dungeon {
                    match dungeon.move_to_room("Descend") {
                        Ok(result) => {
                            let room_json = serde_json::json!({
                                "name": result.room_name, "type": format!("{}", result.room_type),
                                "description": result.description, "exits": result.exits,
                            });
                            sess.store.save_adventure(&adv).ok();
                            send_msg(sender, &ServerMsg::DungeonRoomChanged { room: room_json, floor: result.floor, room_id: result.room_id }).await;
                        }
                        Err(e) => send_msg(sender, &ServerMsg::Error { code: "ascend_failed".into(), message: e }).await,
                    }
                } else {
                    send_msg(sender, &ServerMsg::Error { code: "not_in_tower".into(), message: "Not in a tower".into() }).await;
                }
                sess.adventure = Some(adv);
            }
        }

        ClientMsg::TowerCheckpoint { floor } => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                if adv.dungeon.is_some() {
                    let cost = crate::engine::tower::checkpoint_teleport_cost(floor);
                    send_msg(sender, &ServerMsg::DungeonStatus {
                        status: serde_json::json!({"checkpoint_attuned": true, "floor": floor, "teleport_cost": cost}),
                    }).await;
                } else {
                    send_msg(sender, &ServerMsg::Error { code: "not_in_tower".into(), message: "Not in a tower".into() }).await;
                }
            }
        }

        ClientMsg::TowerTeleport { target_floor } => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let cost = crate::engine::tower::checkpoint_teleport_cost(target_floor);
                if adv.character.gold < cost {
                    send_msg(sender, &ServerMsg::Error { code: "insufficient_gold".into(),
                        message: format!("Need {} gold, have {}", cost, adv.character.gold) }).await;
                } else {
                    send_msg(sender, &ServerMsg::DungeonStatus {
                        status: serde_json::json!({"teleport_available": true, "target_floor": target_floor, "cost": cost}),
                    }).await;
                }
            }
        }

        ClientMsg::TowerFloorStatus { tower_id, floor } => {
            let towers = tower_definitions();
            if let Some(tower) = towers.iter().find(|t| t.id == tower_id) {
                let f = generate_floor(tower, floor);
                let summary = floor_summary(&f);
                send_msg(sender, &ServerMsg::TowerFloorStatus { floor: summary }).await;
            } else {
                send_msg(sender, &ServerMsg::Error { code: "tower_not_found".into(), message: "Unknown tower".into() }).await;
            }
        }

        ClientMsg::ListAdventures => {
            let sess = session.lock().await;
            let adventures = sess.store.list_adventures()?;
            send_msg(sender, &ServerMsg::AdventureList { adventures }).await;
        }

        ClientMsg::CreateAdventure {
            name,
            character_name,
            race,
            class,
            background,
            backstory,
            scenario,
            stats,
        } => {
            let race = parse_race(&race);
            let class = parse_class(class.as_deref().unwrap_or("warrior"));
            let stats = if let Some(s) = stats {
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

            let adventure = if let Some(ref bg_str) = background {
                let bg = crate::engine::backgrounds::Background::from_str(bg_str)
                    .unwrap_or_default();
                AdventureState::new_with_background(name, character_name, race, bg, &scenario)
            } else {
                AdventureState::new(name, character_name, race, class, stats, &scenario)
            };
            let adventure_id = adventure.id.clone();
            let state_json = build_state_with_map(&adventure);

            {
                let mut sess = session.lock().await;
                sess.store.create_adventure(adventure.clone())?;
                sess.adventure = Some(adventure);
                sess.messages.clear();
            }

            send_msg(sender, &ServerMsg::AdventureCreated { adventure_id, state: state_json }).await;
            start_adventure(session, xai_client, sender, &scenario, shop_store).await?;
        }

        ClientMsg::LoadAdventure { adventure_id } => {
            let mut sess = session.lock().await;
            let adventure = sess.store.load_adventure(&adventure_id)?;
            let history = sess.store.load_history(&adventure_id)?;
            let display_events = sess.store.load_display_history(&adventure_id)?;

            let system = ChatMessage::system(&build_system_prompt(&adventure));
            let mut messages = vec![system];
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

            let state_json = build_state_with_map(&adventure);
            let adv_id = adventure.id.clone();

            // Build narrative events from LLM history
            let has_narratives = display_events.iter().any(|e| e.event_type == "narrative");
            let display_events = if display_events.is_empty() || !has_narratives {
                // No display history or missing narratives — rebuild from LLM history
                history
                    .iter()
                    .filter_map(|h| {
                        if h.role == "assistant" {
                            if let Some(ref content) = h.content {
                                if !content.is_empty() {
                                    return Some(DisplayEvent {
                                        event_type: "narrative".to_string(),
                                        data: serde_json::json!({"text": content}),
                                        timestamp: h.timestamp,
                                    });
                                }
                            }
                        }
                        if h.role == "tool" {
                            if let Some(ref content) = h.content {
                                if content.starts_with("Player chose:") {
                                    return Some(DisplayEvent {
                                        event_type: "choice_selected".to_string(),
                                        data: serde_json::json!({"text": content.trim_start_matches("Player chose: ")}),
                                        timestamp: h.timestamp,
                                    });
                                }
                            }
                        }
                        None
                    })
                    .collect::<Vec<_>>()
            } else {
                display_events
            };

            let is_dead = adventure.character.dead || adventure.character.hp <= 0;
            let in_combat = adventure.combat.active;

            let last_event_type = display_events.last().map(|e| e.event_type.clone());
            let needs_resume = !matches!(last_event_type.as_deref(), Some("choices") | Some("dice_roll_request"));

            sess.adventure = Some(adventure);
            sess.messages = messages;
            drop(sess);

            send_msg(sender, &ServerMsg::AdventureLoaded { state: state_json }).await;
            if !display_events.is_empty() {
                send_msg(sender, &ServerMsg::ChatHistory { entries: display_events }).await;
            }

            // If character is dead, don't resume — send death message
            if is_dead {
                send_msg(sender, &ServerMsg::NarrativeChunk {
                    text: "Your character has fallen. This adventure is over.".to_string(),
                }).await;
                send_msg(sender, &ServerMsg::NarrativeEnd).await;
            }
            // If in combat, resume the combat turn instead of LLM resume
            else if in_combat {
                handle_combat_turn_start(session, sender).await?;
            }
            else {

            // Normal resume: call LLM
            if needs_resume {
                let mut sess = session.lock().await;
                let resume_prompt = "The adventurer returns after a break. Briefly recap the current situation in 1-2 sentences, then present choices for what to do next. Include dice requirements in choices where relevant.";
                sess.messages.push(ChatMessage::user(resume_prompt));
                sess.store.append_message(
                    &adv_id,
                    &HistoryMessage {
                        role: "user".to_string(),
                        content: Some(resume_prompt.to_string()),
                        tool_calls: None,
                        tool_call_id: None,
                        timestamp: chrono::Utc::now(),
                    },
                )?;
            }
            continue_tool_loop(session, xai_client, sender, shop_store).await?;
            } // end else (normal resume)
        }

        ClientMsg::DeleteAdventure { adventure_id } => {
            let mut sess = session.lock().await;
            sess.store.delete_adventure(&adventure_id)?;
            if sess
                .adventure
                .as_ref()
                .map(|a| a.id == adventure_id)
                .unwrap_or(false)
            {
                sess.adventure = None;
                sess.messages.clear();
            }
            let adventures = sess.store.list_adventures()?;
            send_msg(sender, &ServerMsg::AdventureList { adventures }).await;
        }

        ClientMsg::SendMessage { content } => {
            run_game_turn(session, xai_client, sender, &content, shop_store).await?;
        }

        ClientMsg::SelectChoice { index: _, text } => {
            let has_pending = {
                let sess = session.lock().await;
                sess.pending_choices.is_some()
            };

            if has_pending {
                let mut sess = session.lock().await;
                let pending = sess.pending_choices.take().unwrap();
                let tool_result =
                    ChatMessage::tool_result(&pending.tool_call_id, &format!("Player chose: {}", text));
                sess.messages.push(tool_result);

                let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
                if let Some(ref id) = adv_id {
                    sess.store.append_message(
                        id,
                        &HistoryMessage {
                            role: "tool".to_string(),
                            content: Some(format!("Player chose: {}", text)),
                            tool_calls: None,
                            tool_call_id: Some(pending.tool_call_id),
                            timestamp: chrono::Utc::now(),
                        },
                    )?;
                    let _ = sess.store.append_display_event(id, &DisplayEvent {
                        event_type: "choice_selected".to_string(),
                        data: serde_json::json!({"text": text}),
                        timestamp: chrono::Utc::now(),
                    });
                }
                drop(sess);
                continue_tool_loop(session, xai_client, sender, shop_store).await?;
            } else {
                run_game_turn(session, xai_client, sender, &text, shop_store).await?;
            }
        }

        ClientMsg::RollDice => {
            let mut sess = session.lock().await;
            if let Some(pending) = sess.pending_roll.take() {
                let result = DiceRoller::roll_with_dc(
                    &pending.dice_type,
                    pending.count,
                    pending.modifier,
                    pending.dc,
                    &pending.description,
                );

                let success = result.success.unwrap_or(false);

                send_msg(
                    sender,
                    &ServerMsg::DiceRollResult {
                        rolls: result.rolls.clone(),
                        total: result.total,
                        dc: pending.dc,
                        success,
                        description: pending.description.clone(),
                    },
                )
                .await;

                let result_json = serde_json::to_string(&result)?;
                let tool_result = ChatMessage::tool_result(&pending.tool_call_id, &result_json);
                sess.messages.push(tool_result);

                let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
                if let Some(id) = &adv_id {
                    sess.store.append_message(
                        id,
                        &HistoryMessage {
                            role: "tool".to_string(),
                            content: Some(result_json),
                            tool_calls: None,
                            tool_call_id: Some(pending.tool_call_id),
                            timestamp: chrono::Utc::now(),
                        },
                    )?;
                    let _ = sess.store.append_display_event(id, &DisplayEvent {
                        event_type: "dice_result".to_string(),
                        data: serde_json::json!({
                            "rolls": result.rolls, "total": result.total,
                            "dc": pending.dc, "success": success,
                            "description": pending.description,
                        }),
                        timestamp: chrono::Utc::now(),
                    });
                }

                // Check precomputed branches
                let precomputed_text = if let Some(ref pre) = sess.precomputed {
                    let branch = if success {
                        pre.success_text.lock().await.clone()
                    } else {
                        pre.failure_text.lock().await.clone()
                    };
                    branch
                } else {
                    None
                };

                // Abort precomputation tasks
                if let Some(pre) = sess.precomputed.take() {
                    pre.success_handle.abort();
                    pre.failure_handle.abort();
                }

                if let Some(text) = precomputed_text {
                    // Use precomputed narrative
                    drop(sess);
                    let chunks: Vec<&str> = text.as_bytes().chunks(80).map(|c| std::str::from_utf8(c).unwrap_or("")).collect();
                    for chunk in &chunks {
                        if !chunk.is_empty() {
                            send_msg(sender, &ServerMsg::NarrativeChunk { text: chunk.to_string() }).await;
                            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                        }
                    }
                    send_msg(sender, &ServerMsg::NarrativeEnd).await;

                    let mut sess = session.lock().await;
                    sess.messages.push(ChatMessage::assistant_text(&text));

                    let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
                    if let Some(ref id) = adv_id {
                        sess.store.append_message(
                            id,
                            &HistoryMessage {
                                role: "assistant".to_string(),
                                content: Some(text.clone()),
                                tool_calls: None,
                                tool_call_id: None,
                                timestamp: chrono::Utc::now(),
                            },
                        )?;
                        let _ = sess.store.append_display_event(id, &DisplayEvent {
                            event_type: "narrative".to_string(),
                            data: serde_json::json!({"text": text}),
                            timestamp: chrono::Utc::now(),
                        });
                    }

                    // Send cost + state
                    send_cost_update(sender, &sess).await;

                    if let Some(ref adv) = sess.adventure {
                        let state = build_state_with_map(&adv);
                        send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                    }
                } else {
                    drop(sess);
                    continue_tool_loop(session, xai_client, sender, shop_store).await?;
                }
            }
        }

        ClientMsg::GetCharacterSheet | ClientMsg::GetInventory | ClientMsg::GetQuests => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let state = build_state_with_map(&adv);
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
            }
        }

        ClientMsg::CombatAction { action_id, target, item_name } => {
            handle_combat_action(session, xai_client, sender, &action_id, target.as_deref(), item_name.as_deref(), shop_store).await?;
        }

        ClientMsg::SetModel { model } => {
            let model_str = model.clone();
            if ALLOWED_MODELS.contains(&model_str.as_str()) {
                let mut sess = session.lock().await;
                sess.model = model_str.clone();
                send_msg(sender, &ServerMsg::ModelInfo {
                    model: model_str,
                    available_models: ALLOWED_MODELS.iter().map(|s| s.to_string()).collect(),
                }).await;
            } else {
                send_msg(sender, &ServerMsg::Error {
                    code: "invalid_model".to_string(),
                    message: format!("Model '{}' not available", model),
                }).await;
            }
        }

        ClientMsg::GetNpcs => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let state = build_state_with_map(&adv);
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
            }
        }

        ClientMsg::CraftItem { recipe_id } => {
            let (result_opt, state_opt) = {
                let mut sess = session.lock().await;
                if let Some(ref mut adv) = sess.adventure {
                    let args = serde_json::json!({"recipe_id": recipe_id});
                    match execute_tool_call(adv, "craft_item", &args) {
                        Ok(ToolExecResult::Immediate(result)) => {
                            let adv_clone = adv.clone();
                            let _ = sess.store.save_adventure(&adv_clone);
                            let state = serde_json::to_value(&adv_clone).ok();
                            (Some(result), state)
                        }
                        _ => (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let Some(result) = result_opt {
                let crafted = result["crafted"].as_bool().unwrap_or(false);
                if crafted {
                    send_msg(sender, &ServerMsg::CraftResult {
                        recipe_name: result["recipe"].as_str().unwrap_or("").to_string(),
                        output: result["output"].as_str().unwrap_or("").to_string(),
                        quantity: result["quantity"].as_u64().unwrap_or(1) as u32,
                        skill_progress: result.get("skill_progress").cloned(),
                    }).await;
                } else {
                    send_msg(sender, &ServerMsg::Error {
                        code: "craft_failed".to_string(),
                        message: result["error"].as_str().unwrap_or("Crafting failed").to_string(),
                    }).await;
                }
                if let Some(state) = state_opt {
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                }
            } else {
                send_msg(sender, &ServerMsg::Error {
                    code: "craft_error".to_string(),
                    message: "Internal crafting error".to_string(),
                }).await;
            }
        }

        ClientMsg::ListRecipes { skill, tier } => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let mut args = serde_json::json!({});
                if let Some(s) = skill { args["skill"] = serde_json::json!(s); }
                if let Some(t) = tier { args["tier"] = serde_json::json!(t); }
                match execute_tool_call(&mut adv.clone(), "list_recipes", &args) {
                    Ok(ToolExecResult::Immediate(result)) => {
                        let recipes = result["recipes"].as_array()
                            .map(|a| a.clone())
                            .unwrap_or_default();
                        send_msg(sender, &ServerMsg::RecipeList { recipes }).await;
                    }
                    _ => {}
                }
            }
        }

        ClientMsg::ListMaterials => {
            let graph = &*CRAFTING_GRAPH;
            let materials: Vec<serde_json::Value> = graph.materials.values()
                .map(|m| serde_json::json!({
                    "id": m.id,
                    "name": m.name,
                    "tier": m.tier,
                    "source": format!("{:?}", m.source),
                }))
                .collect();
            send_msg(sender, &ServerMsg::MaterialList { materials }).await;
        }

        ClientMsg::Gather => {
            let (result_opt, state_opt) = {
                let mut sess = session.lock().await;
                if let Some(ref mut adv) = sess.adventure {
                    let args = serde_json::json!({});
                    match execute_tool_call(adv, "gather", &args) {
                        Ok(ToolExecResult::Immediate(result)) => {
                            let adv_clone = adv.clone();
                            let _ = sess.store.save_adventure(&adv_clone);
                            let state = serde_json::to_value(&adv_clone).ok();
                            (Some(result), state)
                        }
                        _ => (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let Some(result) = result_opt {
                let gathered = result["gathered"].as_array()
                    .map(|a| a.clone())
                    .unwrap_or_default();
                send_msg(sender, &ServerMsg::GatherResult {
                    gathered,
                    biome: result["biome"].as_str().unwrap_or("").to_string(),
                    survival_xp: result["survival_xp"].as_u64().unwrap_or(0) as u32,
                }).await;
                if let Some(state) = state_opt {
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                }
            }
        }

        ClientMsg::GetSkills => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let args = serde_json::json!({});
                match execute_tool_call(&mut adv.clone(), "get_skills", &args) {
                    Ok(ToolExecResult::Immediate(result)) => {
                        let skills = result["skills"].as_array()
                            .map(|a| a.clone())
                            .unwrap_or_default();
                        send_msg(sender, &ServerMsg::SkillList { skills }).await;
                    }
                    _ => {}
                }
            }
        }
        ClientMsg::EquipItem { item_name } => {
            let state_opt = {
                let mut sess = session.lock().await;
                if let Some(mut adv) = sess.adventure.take() {
                    let args = serde_json::json!({"item_name": item_name});
                    match execute_tool_call(&mut adv, "equip_item", &args) {
                        Ok(ToolExecResult::Immediate(_result)) => {
                            let st = build_state_with_map(&adv);
                            sess.store.save_adventure(&adv).ok();
                            sess.adventure = Some(adv);
                            Some(st)
                        }
                        _ => {
                            sess.adventure = Some(adv);
                            None
                        }
                    }
                } else {
                    None
                }
            };
            if let Some(state) = state_opt {
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
            } else {
                send_msg(sender, &ServerMsg::Error {
                    code: "equip_failed".to_string(),
                    message: "Failed to equip item".to_string(),
                }).await;
            }
        }

        ClientMsg::UnequipItem { slot } => {
            let state_opt = {
                let mut sess = session.lock().await;
                if let Some(mut adv) = sess.adventure.take() {
                    let args = serde_json::json!({"slot": slot});
                    match execute_tool_call(&mut adv, "unequip_slot", &args) {
                        Ok(ToolExecResult::Immediate(_result)) => {
                            let st = build_state_with_map(&adv);
                            sess.store.save_adventure(&adv).ok();
                            sess.adventure = Some(adv);
                            Some(st)
                        }
                        _ => {
                            sess.adventure = Some(adv);
                            None
                        }
                    }
                } else {
                    None
                }
            };
            if let Some(state) = state_opt {
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
            } else {
                send_msg(sender, &ServerMsg::Error {
                    code: "unequip_failed".to_string(),
                    message: "Failed to unequip item".to_string(),
                }).await;
            }
        }
    }

    Ok(())
}

async fn start_adventure(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    scenario: &Option<String>,
    shop_store: &ShopStore,
) -> anyhow::Result<()> {
    {
        let mut sess = session.lock().await;
        let (system_prompt, adv_id) = match &sess.adventure {
            Some(adv) => (build_system_prompt(adv), adv.id.clone()),
            None => return Ok(()),
        };

        let start_prompt = adventure_start_prompt(scenario);
        let system = ChatMessage::system(&system_prompt);
        let user_msg = ChatMessage::user(&start_prompt);
        sess.messages = vec![system, user_msg];

        sess.store.append_message(
            &adv_id,
            &HistoryMessage {
                role: "user".to_string(),
                content: Some(start_prompt),
                tool_calls: None,
                tool_call_id: None,
                timestamp: chrono::Utc::now(),
            },
        )?;
    }

    continue_tool_loop(session, xai_client, sender, shop_store).await
}

async fn run_game_turn(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    user_input: &str,
    shop_store: &ShopStore,
) -> anyhow::Result<()> {
    {
        let mut sess = session.lock().await;

        // Apply condition effects at start of turn
        let condition_effects = {
            if let Some(ref mut adventure) = sess.adventure {
                let effects = apply_turn_effects(adventure);
                if !effects.is_empty() {
                    Some((effects, serde_json::to_value(&*adventure)?))
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some((effects, state)) = condition_effects {
            send_msg(sender, &ServerMsg::ConditionEffects {
                effects: effects.clone(),
            }).await;
            let effects_text = format!(
                "[SYSTEM: Start-of-turn condition effects applied: {}]",
                effects.join("; ")
            );
            sess.messages.push(ChatMessage::system(&effects_text));
            send_msg(sender, &ServerMsg::StateUpdate { state }).await;
        }

        let system_prompt = match &sess.adventure {
            Some(adv) => Some(build_system_prompt(adv)),
            None => None,
        };
        if let Some(prompt) = system_prompt {
            if !sess.messages.is_empty() {
                sess.messages[0] = ChatMessage::system(&prompt);
            }
        }

        let user_msg = ChatMessage::user(user_input);
        sess.messages.push(user_msg);

        let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
        if let Some(id) = adv_id {
            sess.store.append_message(
                &id,
                &HistoryMessage {
                    role: "user".to_string(),
                    content: Some(user_input.to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    timestamp: chrono::Utc::now(),
                },
            )?;
        }
    }

    continue_tool_loop(session, xai_client, sender, shop_store).await
}

async fn continue_tool_loop(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    shop_store: &ShopStore,
) -> anyhow::Result<()> {
    let tools = build_tool_definitions();
    let max_iterations = 15;

    for _ in 0..max_iterations {
        let (messages, model) = {
            let sess = session.lock().await;
            (sess.messages.clone(), sess.model.clone())
        };

        let (response, usage) = xai_client.chat_with_tools(&messages, &tools, Some(&model)).await?;

        // Track usage
        if let Some(ref u) = usage {
            let mut sess = session.lock().await;
            sess.session_cost.add(u);
            log_usage(&sess, u);
        }

        match response {
            XaiResponse::Text(text) => {
                let chunks: Vec<&str> = text.as_bytes().chunks(80).map(|c| std::str::from_utf8(c).unwrap_or("")).collect();
                for chunk in &chunks {
                    if !chunk.is_empty() {
                        send_msg(sender, &ServerMsg::NarrativeChunk { text: chunk.to_string() }).await;
                        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    }
                }
                send_msg(sender, &ServerMsg::NarrativeEnd).await;

                let mut sess = session.lock().await;
                sess.messages.push(ChatMessage::assistant_text(&text));

                let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
                if let Some(ref id) = adv_id {
                    sess.store.append_message(
                        id,
                        &HistoryMessage {
                            role: "assistant".to_string(),
                            content: Some(text.clone()),
                            tool_calls: None,
                            tool_call_id: None,
                            timestamp: chrono::Utc::now(),
                        },
                    )?;
                    // Save display event
                    let _ = sess.store.append_display_event(id, &DisplayEvent {
                        event_type: "narrative".to_string(),
                        data: serde_json::json!({"text": text}),
                        timestamp: chrono::Utc::now(),
                    });
                }

                send_cost_update(sender, &sess).await;

                if let Some(ref adv) = sess.adventure {
                    let state = build_state_with_map(&adv);
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                }

                return Ok(());
            }

            XaiResponse::ToolCalls { tool_calls, text } => {
                if let Some(ref t) = text {
                    if !t.is_empty() {
                        send_msg(sender, &ServerMsg::NarrativeChunk { text: t.clone() }).await;
                    }
                }

                let mut sess = session.lock().await;
                sess.messages
                    .push(ChatMessage::assistant_tool_calls(tool_calls.clone()));

                let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
                if let Some(ref id) = adv_id {
                    sess.store.append_message(
                        id,
                        &HistoryMessage {
                            role: "assistant".to_string(),
                            content: text,
                            tool_calls: Some(serde_json::to_value(&tool_calls)?),
                            tool_call_id: None,
                            timestamp: chrono::Utc::now(),
                        },
                    )?;
                }

                for tc in &tool_calls {
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_default();

                    let mut adventure = match sess.adventure.take() {
                        Some(a) => a,
                        None => continue,
                    };

                    let exec_result = execute_tool_call_with_shop(&mut adventure, &tc.function.name, &args, Some(shop_store));

                    match exec_result {
                        Ok(ToolExecResult::Immediate(result)) => {
                            let result_str = serde_json::to_string(&result)?;
                            sess.messages.push(ChatMessage::tool_result(&tc.id, &result_str));

                            if let Some(ref id) = adv_id {
                                sess.store.append_message(
                                    id,
                                    &HistoryMessage {
                                        role: "tool".to_string(),
                                        content: Some(result_str),
                                        tool_calls: None,
                                        tool_call_id: Some(tc.id.clone()),
                                        timestamp: chrono::Utc::now(),
                                    },
                                )?;
                            }
                            sess.adventure = Some(adventure);
                        }

                        Ok(ToolExecResult::PendingDiceRoll {
                            dice_type,
                            count,
                            modifier,
                            dc,
                            description,
                            success_probability,
                        }) => {
                            send_msg(
                                sender,
                                &ServerMsg::DiceRollRequest {
                                    dice_type: dice_type.clone(),
                                    count,
                                    modifier,
                                    dc,
                                    description: description.clone(),
                                    success_probability,
                                },
                            )
                            .await;

                            sess.store.save_adventure(&adventure)?;
                            sess.pending_roll = Some(PendingRoll {
                                dice_type: dice_type.clone(),
                                count,
                                modifier,
                                dc,
                                description: description.clone(),
                                tool_call_id: tc.id.clone(),
                            });
                            sess.adventure = Some(adventure);

                            // Spawn precomputation tasks
                            let success_text = Arc::new(Mutex::new(None));
                            let failure_text = Arc::new(Mutex::new(None));

                            let mut success_msgs = sess.messages.clone();
                            let mut failure_msgs = sess.messages.clone();
                            let success_result = serde_json::json!({
                                "dice_type": dice_type, "rolls": [dc], "total": dc + modifier,
                                "modifier": modifier, "dc": dc, "success": true, "description": description
                            });
                            let failure_result = serde_json::json!({
                                "dice_type": dice_type, "rolls": [1], "total": 1 + modifier,
                                "modifier": modifier, "dc": dc, "success": false, "description": description
                            });

                            success_msgs.push(ChatMessage::tool_result(&tc.id, &serde_json::to_string(&success_result)?));
                            failure_msgs.push(ChatMessage::tool_result(&tc.id, &serde_json::to_string(&failure_result)?));

                            let s_text = success_text.clone();
                            let f_text = failure_text.clone();
                            let s_client = xai_client.clone();
                            let f_client = xai_client.clone();
                            let s_tools = tools.clone();
                            let f_tools = tools.clone();
                            let s_model = sess.model.clone();
                            let f_model = sess.model.clone();

                            let s_handle = tokio::spawn(async move {
                                if let Ok((XaiResponse::Text(t), _)) = s_client.chat_with_tools(&success_msgs, &s_tools, Some(&s_model)).await {
                                    *s_text.lock().await = Some(t);
                                }
                            });
                            let f_handle = tokio::spawn(async move {
                                if let Ok((XaiResponse::Text(t), _)) = f_client.chat_with_tools(&failure_msgs, &f_tools, Some(&f_model)).await {
                                    *f_text.lock().await = Some(t);
                                }
                            });

                            sess.precomputed = Some(PrecomputedBranches {
                                success_text,
                                failure_text,
                                success_handle: s_handle,
                                failure_handle: f_handle,
                            });

                            send_cost_update(sender, &sess).await;

                            return Ok(());
                        }

                        Ok(ToolExecResult::PendingChoices {
                            choices,
                            allow_custom_input,
                            prompt,
                        }) => {
                            // Save display event before moving data
                            if let Some(ref id) = adv_id {
                                let _ = sess.store.append_display_event(id, &DisplayEvent {
                                    event_type: "choices".to_string(),
                                    data: serde_json::json!({"choices": &choices, "prompt": &prompt, "allow_custom_input": allow_custom_input}),
                                    timestamp: chrono::Utc::now(),
                                });
                            }

                            send_msg(
                                sender,
                                &ServerMsg::PresentChoices {
                                    choices,
                                    allow_custom_input,
                                    prompt,
                                },
                            )
                            .await;

                            sess.store.save_adventure(&adventure)?;
                            sess.pending_choices = Some(PendingChoices {
                                tool_call_id: tc.id.clone(),
                            });
                            sess.adventure = Some(adventure);

                            send_cost_update(sender, &sess).await;

                            return Ok(());
                        }

                        Ok(ToolExecResult::CombatStarted) => {
                            sess.store.save_adventure(&adventure)?;

                            // Send combat started message
                            let init_order: Vec<InitiativeInfo> = adventure.combat.initiative.iter().map(|e| {
                                InitiativeInfo {
                                    name: e.name.clone(),
                                    roll: e.roll,
                                    is_player: e.combatant == CombatantId::Player,
                                }
                            }).collect();

                            send_msg(sender, &ServerMsg::CombatStarted {
                                initiative_order: init_order,
                                round: adventure.combat.round,
                            }).await;

                            // Tell LLM combat started
                            let combat_info = format!(
                                "Combat has begun! Initiative order established. The engine will handle turn order and mechanics. Narrate the start of combat dramatically."
                            );
                            sess.messages.push(ChatMessage::tool_result(&tc.id, &combat_info));
                            sess.adventure = Some(adventure);

                            // Now handle the first turn
                            drop(sess);
                            handle_combat_turn_start(session, sender).await?;
                            return Ok(());
                        }

                        Err(e) => {
                            let err_msg = format!("Error: {}", e);
                            sess.messages.push(ChatMessage::tool_result(&tc.id, &err_msg));
                            sess.adventure = Some(adventure);
                        }
                    }
                }

                if let Some(ref adventure) = sess.adventure {
                    sess.store.save_adventure(adventure)?;
                    let state = serde_json::to_value(adventure)?;
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                }

                drop(sess);
            }
        }
    }

    send_msg(
        sender,
        &ServerMsg::Error {
            code: "tool_loop_limit".to_string(),
            message: "Too many tool call iterations".to_string(),
        },
    )
    .await;

    Ok(())
}

/// Send the combat turn start message. Loops through enemy turns until it's the player's turn.
async fn handle_combat_turn_start(
    session: &Arc<Mutex<Session>>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
) -> anyhow::Result<()> {
    loop {
        let mut sess = session.lock().await;
        let mut adventure = match sess.adventure.take() {
            Some(a) => a,
            None => return Ok(()),
        };

        if !adventure.combat.active {
            sess.adventure = Some(adventure);
            return Ok(());
        }

        let combatant = adventure.combat.current_combatant().cloned();
        match combatant {
            Some(CombatantId::Player) => {
                let has_weapon = adventure.equipment.equipped_weapon().is_some();
                let has_potion = adventure.inventory.items.iter().any(|i| i.item_type == crate::engine::inventory::ItemType::Potion);
                let actions = adventure.combat.available_actions(&adventure.character, has_weapon, has_potion);
                let enemies: Vec<EnemyInfo> = adventure.combat.enemies.iter().map(|e| EnemyInfo {
                    name: e.name.clone(), hp: e.hp, max_hp: e.max_hp, ac: e.ac, alive: e.hp > 0,
                }).collect();

                send_msg(sender, &ServerMsg::CombatTurnStart {
                    combatant: adventure.character.name.clone(),
                    is_player: true,
                    round: adventure.combat.round,
                    actions: adventure.combat.action_economy.actions,
                    bonus_actions: adventure.combat.action_economy.bonus_actions,
                    movement: adventure.combat.action_economy.movement_remaining,
                    available_actions: actions.into_iter().map(|a| ActionInfo {
                        id: a.id, name: a.name, cost: a.cost, description: a.description, enabled: a.enabled,
                    }).collect(),
                    enemies,
                }).await;

                sess.adventure = Some(adventure);
                return Ok(());
            }
            Some(CombatantId::Enemy(idx)) => {
                let result = adventure.combat.execute_enemy_turn(idx, &mut adventure.character);
                if let Some(result) = result {
                    send_msg(sender, &ServerMsg::CombatEnemyTurn {
                        enemy_name: result.enemy_name,
                        attack_name: result.attack_name,
                        attack_roll: result.attack_roll,
                        target_ac: result.target_ac,
                        hit: result.hit,
                        damage: result.damage,
                        player_hp: result.player_hp_after,
                        player_max_hp: result.player_max_hp,
                    }).await;

                    let state = build_state_with_map(&adventure);
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;

                    if adventure.character.hp <= 0 {
                        adventure.character.dead = true;
                        adventure.combat.end();
                        sess.store.save_adventure(&adventure)?;
                        sess.adventure = Some(adventure);
                        send_msg(sender, &ServerMsg::CombatEnded { xp_reward: 0, victory: false }).await;
                        return Ok(());
                    }
                }

                adventure.combat.next_turn();
                sess.store.save_adventure(&adventure)?;
                sess.adventure = Some(adventure);
                drop(sess);
                // Loop continues to handle next combatant
                tokio::time::sleep(std::time::Duration::from_millis(500)).await; // Pause between enemy turns for readability
            }
            None => {
                sess.adventure = Some(adventure);
                return Ok(());
            }
        }
    }
}

/// Handle a player combat action.
async fn handle_combat_action(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    action_id: &str,
    target: Option<&str>,
    _item_name: Option<&str>,
    shop_store: &ShopStore,
) -> anyhow::Result<()> {
    let mut sess = session.lock().await;
    let mut adventure = match sess.adventure.take() {
        Some(a) => a,
        None => return Ok(()),
    };

    if !adventure.combat.active || !adventure.combat.is_player_turn() {
        sess.adventure = Some(adventure);
        send_msg(sender, &ServerMsg::Error {
            code: "not_your_turn".to_string(),
            message: "It's not your turn!".to_string(),
        }).await;
        return Ok(());
    }

    match action_id {
        "attack" => {
            if adventure.combat.action_economy.actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_action".to_string(), message: "No actions remaining".to_string() }).await;
                return Ok(());
            }
            adventure.combat.action_economy.actions -= 1;

            let target_name = target.unwrap_or("enemy");
            // Find weapon from equipment
            let weapon = adventure.equipment.equipped_weapon();
            let (weapon_name, damage_dice, stat_mod, weapon_attack_bonus) = if let Some(w) = weapon {
                let stat_name = w.stats.damage_modifier_stat.as_deref().unwrap_or("str");
                let mod_val = if w.stats.is_finesse {
                    let str_mod = adventure.character.stats.modifier_for("str").unwrap_or(0);
                    let dex_mod = adventure.character.stats.modifier_for("dex").unwrap_or(0);
                    std::cmp::max(str_mod, dex_mod)
                } else {
                    adventure.character.stats.modifier_for(stat_name).unwrap_or(0)
                };
                let dice = w.stats.damage_dice.as_deref().unwrap_or("1d4").to_string();
                (w.display_name(), dice, mod_val, w.stats.attack_bonus)
            } else {
                ("Unarmed".to_string(), "1d4".to_string(), adventure.character.stats.modifier_for("str").unwrap_or(0), 0)
            };

            let prof = adventure.character.proficiency_bonus();
            let equip_atk_bonus = adventure.equipment.stat_bonuses().attack_bonus;
            let attack = DiceRoller::roll("d20", 1, stat_mod + prof + weapon_attack_bonus + equip_atk_bonus);

            let target_ac = adventure.combat.find_enemy_mut(target_name).map(|e| e.ac).unwrap_or(10);
            let hit = attack.total >= target_ac;
            let damage = if hit {
                let d = DiceRoller::roll(&damage_dice, 1, stat_mod);
                let dmg = std::cmp::max(d.total, 1);
                if let Some(enemy) = adventure.combat.find_enemy_mut(target_name) {
                    enemy.hp -= dmg;
                }
                dmg
            } else { 0 };

            let desc = if hit {
                format!("{} attacks {} with {} (rolled {} vs AC {}): HIT for {} damage!", adventure.character.name, target_name, weapon_name, attack.total, target_ac, damage)
            } else {
                format!("{} attacks {} with {} (rolled {} vs AC {}): MISS!", adventure.character.name, target_name, weapon_name, attack.total, target_ac)
            };
            adventure.combat.combat_log.push(desc.clone());

            send_msg(sender, &ServerMsg::CombatActionResult {
                actor: adventure.character.name.clone(),
                action: "Attack".to_string(),
                description: desc,
                roll: Some(attack.total),
                hit: Some(hit),
                damage: if hit { Some(damage) } else { None },
            }).await;

            // Check if all enemies dead
            if adventure.combat.all_enemies_dead() {
                let xp = adventure.combat.enemies.len() as u32 * 50;
                adventure.combat.end();
                adventure.character.xp += xp;
                adventure.character.check_level_up();
                sess.store.save_adventure(&adventure)?;

                let state = build_state_with_map(&adventure);
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                send_msg(sender, &ServerMsg::CombatEnded { xp_reward: xp, victory: true }).await;

                sess.adventure = Some(adventure);
                sess.messages.push(ChatMessage::user("Combat is over. All enemies defeated. Narrate the victory and present choices for what to do next."));
                drop(sess);
                continue_tool_loop(session, xai_client, sender, shop_store).await?;
                return Ok(());
            }

            let state = build_state_with_map(&adventure);
            send_msg(sender, &ServerMsg::StateUpdate { state }).await;

            // Send updated actions
            let has_weapon = adventure.equipment.equipped_weapon().is_some();
            let has_potion = adventure.inventory.items.iter().any(|i| i.item_type == crate::engine::inventory::ItemType::Potion);
            let actions = adventure.combat.available_actions(&adventure.character, has_weapon, has_potion);
            let enemies: Vec<EnemyInfo> = adventure.combat.enemies.iter().map(|e| EnemyInfo {
                name: e.name.clone(), hp: e.hp, max_hp: e.max_hp, ac: e.ac, alive: e.hp > 0,
            }).collect();
            send_msg(sender, &ServerMsg::CombatTurnStart {
                combatant: adventure.character.name.clone(),
                is_player: true,
                round: adventure.combat.round,
                actions: adventure.combat.action_economy.actions,
                bonus_actions: adventure.combat.action_economy.bonus_actions,
                movement: adventure.combat.action_economy.movement_remaining,
                available_actions: actions.into_iter().map(|a| ActionInfo {
                    id: a.id, name: a.name, cost: a.cost, description: a.description, enabled: a.enabled,
                }).collect(),
                enemies,
            }).await;
        }

        "dodge" => {
            if adventure.combat.action_economy.actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_action".to_string(), message: "No actions remaining".to_string() }).await;
                return Ok(());
            }
            adventure.combat.action_economy.actions -= 1;
            adventure.combat.player_dodging = true;
            let desc = format!("{} takes the Dodge action. Attacks against them have disadvantage.", adventure.character.name);
            adventure.combat.combat_log.push(desc.clone());
            send_msg(sender, &ServerMsg::CombatActionResult {
                actor: adventure.character.name.clone(), action: "Dodge".to_string(),
                description: desc, roll: None, hit: None, damage: None,
            }).await;
        }

        "dash" => {
            if adventure.combat.action_economy.actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_action".to_string(), message: "No actions remaining".to_string() }).await;
                return Ok(());
            }
            adventure.combat.action_economy.actions -= 1;
            adventure.combat.action_economy.movement_remaining += 30;
            let desc = format!("{} dashes! Movement doubled this turn.", adventure.character.name);
            adventure.combat.combat_log.push(desc.clone());
            send_msg(sender, &ServerMsg::CombatActionResult {
                actor: adventure.character.name.clone(), action: "Dash".to_string(),
                description: desc, roll: None, hit: None, damage: None,
            }).await;
        }

        "use_item" => {
            if adventure.combat.action_economy.actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_action".to_string(), message: "No actions remaining".to_string() }).await;
                return Ok(());
            }
            // Find and use a potion
            let potion_idx = adventure.inventory.items.iter().position(|i| i.item_type == crate::engine::inventory::ItemType::Potion);
            if let Some(idx) = potion_idx {
                adventure.combat.action_economy.actions -= 1;
                let potion_name = adventure.inventory.items[idx].name.clone();
                // Decrement quantity or remove
                if adventure.inventory.items[idx].quantity > 1 {
                    adventure.inventory.items[idx].quantity -= 1;
                } else {
                    adventure.inventory.items.remove(idx);
                }
                let potion = potion_name;
                let healing = DiceRoller::roll("d4", 2, 2);
                adventure.character.hp = std::cmp::min(adventure.character.hp + healing.total, adventure.character.max_hp);
                let desc = format!("{} drinks {}! Healed {} HP (now {}/{})", adventure.character.name, potion, healing.total, adventure.character.hp, adventure.character.max_hp);
                adventure.combat.combat_log.push(desc.clone());
                send_msg(sender, &ServerMsg::CombatActionResult {
                    actor: adventure.character.name.clone(), action: "Use Item".to_string(),
                    description: desc, roll: None, hit: None, damage: Some(healing.total),
                }).await;
            }
        }

        "second_wind" => {
            if adventure.combat.action_economy.bonus_actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_bonus".to_string(), message: "No bonus actions remaining".to_string() }).await;
                return Ok(());
            }
            adventure.combat.action_economy.bonus_actions -= 1;
            let healing = DiceRoller::roll("d10", 1, adventure.character.level as i32);
            adventure.character.hp = std::cmp::min(adventure.character.hp + healing.total, adventure.character.max_hp);
            let desc = format!("{} uses Second Wind! Healed {} HP (now {}/{})", adventure.character.name, healing.total, adventure.character.hp, adventure.character.max_hp);
            adventure.combat.combat_log.push(desc.clone());
            send_msg(sender, &ServerMsg::CombatActionResult {
                actor: adventure.character.name.clone(), action: "Second Wind".to_string(),
                description: desc, roll: None, hit: None, damage: None,
            }).await;
        }

        "cunning_hide" => {
            if adventure.combat.action_economy.bonus_actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_bonus".to_string(), message: "No bonus actions remaining".to_string() }).await;
                return Ok(());
            }
            adventure.combat.action_economy.bonus_actions -= 1;
            let desc = format!("{} hides in the shadows! Next attack has advantage.", adventure.character.name);
            adventure.combat.combat_log.push(desc.clone());
            send_msg(sender, &ServerMsg::CombatActionResult {
                actor: adventure.character.name.clone(), action: "Hide".to_string(),
                description: desc, roll: None, hit: None, damage: None,
            }).await;
        }

        "healing_word" => {
            if adventure.combat.action_economy.bonus_actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_bonus".to_string(), message: "No bonus actions remaining".to_string() }).await;
                return Ok(());
            }
            adventure.combat.action_economy.bonus_actions -= 1;
            let wis_mod = adventure.character.stats.modifier_for("wis").unwrap_or(0);
            let healing = DiceRoller::roll("d4", 1, wis_mod);
            adventure.character.hp = std::cmp::min(adventure.character.hp + healing.total, adventure.character.max_hp);
            let desc = format!("{} casts Healing Word! Healed {} HP (now {}/{})", adventure.character.name, healing.total, adventure.character.hp, adventure.character.max_hp);
            adventure.combat.combat_log.push(desc.clone());
            send_msg(sender, &ServerMsg::CombatActionResult {
                actor: adventure.character.name.clone(), action: "Healing Word".to_string(),
                description: desc, roll: None, hit: None, damage: None,
            }).await;
        }


        "flee" => {
            if adventure.combat.action_economy.actions == 0 {
                send_msg(sender, &ServerMsg::Error { code: "no_action".to_string(), message: "No actions remaining".to_string() }).await;
                sess.adventure = Some(adventure);
                return Ok(());
            }
            adventure.combat.action_economy.actions -= 1;

            let dex_mod = adventure.character.stats.modifier_for("dex").unwrap_or(0);
            let living = adventure.combat.living_enemies().len() as i32;
            let flee_dc = (10 + living * 2 - adventure.combat.flee_attempts as i32 * 2).max(5);
            let roll = DiceRoller::roll("d20", 1, dex_mod);
            let success = roll.total >= flee_dc;

            if success {
                let desc = format!("{} attempts to flee (rolled {} vs DC {}): SUCCESS! Escaped combat!", adventure.character.name, roll.total, flee_dc);
                adventure.combat.combat_log.push(desc.clone());
                adventure.combat.end();
                sess.store.save_adventure(&adventure)?;

                let state = build_state_with_map(&adventure);
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                send_msg(sender, &ServerMsg::CombatActionResult {
                    actor: adventure.character.name.clone(), action: "Flee".to_string(),
                    description: desc, roll: Some(roll.total), hit: None, damage: None,
                }).await;
                send_msg(sender, &ServerMsg::CombatEnded { xp_reward: 0, victory: false }).await;

                sess.adventure = Some(adventure);
                sess.messages.push(ChatMessage::user("The player successfully fled from combat. Narrate their narrow escape and present choices for what to do next."));
                drop(sess);
                continue_tool_loop(session, xai_client, sender, shop_store).await?;
                return Ok(());
            } else {
                adventure.combat.flee_attempts += 1;
                let next_dc = (10 + living * 2 - adventure.combat.flee_attempts as i32 * 2).max(5);
                let desc = format!("{} attempts to flee (rolled {} vs DC {}): FAILED! The enemies block the escape. (Next attempt DC {})", adventure.character.name, roll.total, flee_dc, next_dc);
                adventure.combat.combat_log.push(desc.clone());
                send_msg(sender, &ServerMsg::CombatActionResult {
                    actor: adventure.character.name.clone(), action: "Flee".to_string(),
                    description: desc, roll: Some(roll.total), hit: Some(false), damage: None,
                }).await;
            }
        }

        "end_turn" => {
            adventure.combat.next_turn();
            sess.store.save_adventure(&adventure)?;
            sess.adventure = Some(adventure);
            drop(sess);
            return handle_combat_turn_start(session, sender).await;
        }

        _ => {
            send_msg(sender, &ServerMsg::Error {
                code: "unknown_action".to_string(),
                message: format!("Unknown combat action: {}", action_id),
            }).await;
        }
    }

    // Save state and send update after action
    if action_id != "end_turn" {
        sess.store.save_adventure(&adventure)?;
        let state = build_state_with_map(&adventure);
        send_msg(sender, &ServerMsg::StateUpdate { state }).await;
    }
    sess.adventure = Some(adventure);

    Ok(())
}

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
