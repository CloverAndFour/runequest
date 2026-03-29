//! WebSocket handler with game loop state machine.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::auth::AuthUser;
use crate::engine::adventure::AdventureState;
use crate::engine::character::{Class, Race, Stats};
use crate::engine::conditions::apply_turn_effects;
use crate::engine::dice::DiceRoller;
use crate::engine::executor::{execute_tool_call, ToolExecResult};
use crate::llm::client::XaiClient;
use crate::llm::pricing::{SessionCost, TokenUsage};
use crate::llm::prompts::{adventure_start_prompt, build_system_prompt};
use crate::llm::tools::build_tool_definitions;
use crate::llm::types::*;
use crate::storage::adventure_store::{AdventureStore, DisplayEvent, HistoryMessage};
use crate::storage::usage_logger::{UsageEntry, UsageLogger};
use crate::engine::combat::CombatantId;
use crate::web::protocol::{ActionInfo, ClientMsg, EnemyInfo, InitiativeInfo, ServerMsg};

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

        let response = handle_client_msg(client_msg, &session, &xai_client, &mut sender).await;

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
) -> anyhow::Result<()> {
    match msg {
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
            scenario,
            stats,
        } => {
            let race = parse_race(&race);
            let class = parse_class(&class);
            let stats = Stats {
                strength: stats.strength,
                dexterity: stats.dexterity,
                constitution: stats.constitution,
                intelligence: stats.intelligence,
                wisdom: stats.wisdom,
                charisma: stats.charisma,
            };

            let adventure = AdventureState::new(name, character_name, race, class, stats);
            let adventure_id = adventure.id.clone();
            let state_json = serde_json::to_value(&adventure)?;

            {
                let mut sess = session.lock().await;
                sess.store.create_adventure(adventure.clone())?;
                sess.adventure = Some(adventure);
                sess.messages.clear();
            }

            send_msg(sender, &ServerMsg::AdventureCreated { adventure_id, state: state_json }).await;
            start_adventure(session, xai_client, sender, &scenario).await?;
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

            let state_json = serde_json::to_value(&adventure)?;
            let adv_id = adventure.id.clone();

            // If no display_history.jsonl exists (old adventure), build from LLM history
            let display_events = if display_events.is_empty() {
                history
                    .iter()
                    .filter(|h| h.role == "assistant" && h.content.is_some() && h.tool_calls.is_none())
                    .map(|h| DisplayEvent {
                        event_type: "narrative".to_string(),
                        data: serde_json::json!({"text": h.content.as_deref().unwrap_or("")}),
                        timestamp: h.timestamp,
                    })
                    .collect::<Vec<_>>()
            } else {
                display_events
            };

            // Check if last display event was choices or dice roll (can restore without LLM)
            let last_event_type = display_events.last().map(|e| e.event_type.clone());
            let needs_resume = !matches!(last_event_type.as_deref(), Some("choices") | Some("dice_roll_request"));

            sess.adventure = Some(adventure);
            sess.messages = messages;
            drop(sess);

            send_msg(sender, &ServerMsg::AdventureLoaded { state: state_json }).await;
            if !display_events.is_empty() {
                send_msg(sender, &ServerMsg::ChatHistory { entries: display_events }).await;
            }

            // Only call LLM if the player wasn't mid-interaction
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
            continue_tool_loop(session, xai_client, sender).await?;
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
            run_game_turn(session, xai_client, sender, &content).await?;
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
                if let Some(id) = adv_id {
                    sess.store.append_message(
                        &id,
                        &HistoryMessage {
                            role: "tool".to_string(),
                            content: Some(format!("Player chose: {}", text)),
                            tool_calls: None,
                            tool_call_id: Some(pending.tool_call_id),
                            timestamp: chrono::Utc::now(),
                        },
                    )?;
                }
                drop(sess);
                continue_tool_loop(session, xai_client, sender).await?;
            } else {
                run_game_turn(session, xai_client, sender, &text).await?;
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
                    if let Some(id) = &adv_id {
                        sess.store.append_message(
                            id,
                            &HistoryMessage {
                                role: "assistant".to_string(),
                                content: Some(text),
                                tool_calls: None,
                                tool_call_id: None,
                                timestamp: chrono::Utc::now(),
                            },
                        )?;
                    }

                    // Send cost + state
                    send_cost_update(sender, &sess).await;

                    if let Some(ref adv) = sess.adventure {
                        let state = serde_json::to_value(adv)?;
                        send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                    }
                } else {
                    drop(sess);
                    continue_tool_loop(session, xai_client, sender).await?;
                }
            }
        }

        ClientMsg::GetCharacterSheet | ClientMsg::GetInventory | ClientMsg::GetQuests => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let state = serde_json::to_value(adv)?;
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
            }
        }

        ClientMsg::CombatAction { action_id, target, item_name } => {
            handle_combat_action(session, xai_client, sender, &action_id, target.as_deref(), item_name.as_deref()).await?;
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
    }

    Ok(())
}

async fn start_adventure(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    scenario: &Option<String>,
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

    continue_tool_loop(session, xai_client, sender).await
}

async fn run_game_turn(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    user_input: &str,
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

    continue_tool_loop(session, xai_client, sender).await
}

async fn continue_tool_loop(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
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
                    let state = serde_json::to_value(adv)?;
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

                    let exec_result = execute_tool_call(&mut adventure, &tc.function.name, &args);

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

                    let state = serde_json::to_value(&adventure)?;
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;

                    if adventure.character.hp <= 0 {
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

                let state = serde_json::to_value(&adventure)?;
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                send_msg(sender, &ServerMsg::CombatEnded { xp_reward: xp, victory: true }).await;

                sess.adventure = Some(adventure);
                sess.messages.push(ChatMessage::user("Combat is over. All enemies defeated. Narrate the victory and present choices for what to do next."));
                drop(sess);
                continue_tool_loop(session, xai_client, sender).await?;
                return Ok(());
            }

            let state = serde_json::to_value(&adventure)?;
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

    // Save state after action
    if action_id != "end_turn" {
        sess.store.save_adventure(&adventure)?;
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
