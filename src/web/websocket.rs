//! WebSocket handler with game loop state machine.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::auth::AuthUser;
use crate::engine::adventure::AdventureState;
use crate::engine::character::{Class, Race, Stats};
use crate::engine::dice::DiceRoller;
use crate::engine::executor::{execute_tool_call, ToolExecResult};
use crate::llm::client::XaiClient;
use crate::llm::prompts::{build_system_prompt, ADVENTURE_START_PROMPT};
use crate::llm::tools::build_tool_definitions;
use crate::llm::types::*;
use crate::storage::adventure_store::{AdventureStore, HistoryMessage};
use crate::web::protocol::{ClientMsg, ServerMsg};

/// Per-connection session state.
struct Session {
    user: AuthUser,
    store: AdventureStore,
    adventure: Option<AdventureState>,
    messages: Vec<ChatMessage>,
    pending_roll: Option<PendingRoll>,
    pending_choices: Option<PendingChoices>,
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

pub async fn handle_socket(
    socket: WebSocket,
    user: AuthUser,
    xai_client: Arc<XaiClient>,
    data_dir: std::path::PathBuf,
) {
    let (mut sender, mut receiver) = socket.split();
    let store = AdventureStore::new(&data_dir, &user.username);

    let session = Arc::new(Mutex::new(Session {
        user: user.clone(),
        store,
        adventure: None,
        messages: Vec::new(),
        pending_roll: None,
        pending_choices: None,
    }));

    let connected = ServerMsg::Connected {
        username: user.username.clone(),
    };
    let _ = sender
        .send(Message::Text(serde_json::to_string(&connected).unwrap().into()))
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
            start_adventure(session, xai_client, sender).await?;
        }

        ClientMsg::LoadAdventure { adventure_id } => {
            let mut sess = session.lock().await;
            let adventure = sess.store.load_adventure(&adventure_id)?;
            let history = sess.store.load_history(&adventure_id)?;

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
            sess.adventure = Some(adventure);
            sess.messages = messages;
            drop(sess);

            send_msg(sender, &ServerMsg::AdventureLoaded { state: state_json }).await;
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

                send_msg(
                    sender,
                    &ServerMsg::DiceRollResult {
                        rolls: result.rolls.clone(),
                        total: result.total,
                        dc: pending.dc,
                        success: result.success.unwrap_or(false),
                        description: pending.description.clone(),
                    },
                )
                .await;

                let result_json = serde_json::to_string(&result)?;
                let tool_result = ChatMessage::tool_result(&pending.tool_call_id, &result_json);
                sess.messages.push(tool_result);

                let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
                if let Some(id) = adv_id {
                    sess.store.append_message(
                        &id,
                        &HistoryMessage {
                            role: "tool".to_string(),
                            content: Some(result_json),
                            tool_calls: None,
                            tool_call_id: Some(pending.tool_call_id),
                            timestamp: chrono::Utc::now(),
                        },
                    )?;
                }

                drop(sess);
                continue_tool_loop(session, xai_client, sender).await?;
            }
        }

        ClientMsg::GetCharacterSheet | ClientMsg::GetInventory | ClientMsg::GetQuests => {
            let sess = session.lock().await;
            if let Some(ref adv) = sess.adventure {
                let state = serde_json::to_value(adv)?;
                send_msg(sender, &ServerMsg::StateUpdate { state }).await;
            }
        }
    }

    Ok(())
}

async fn start_adventure(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
) -> anyhow::Result<()> {
    {
        let mut sess = session.lock().await;
        // Extract what we need, then mutate
        let (system_prompt, adv_id) = match &sess.adventure {
            Some(adv) => (build_system_prompt(adv), adv.id.clone()),
            None => return Ok(()),
        };

        let system = ChatMessage::system(&system_prompt);
        let user_msg = ChatMessage::user(ADVENTURE_START_PROMPT);
        sess.messages = vec![system, user_msg];

        sess.store.append_message(
            &adv_id,
            &HistoryMessage {
                role: "user".to_string(),
                content: Some(ADVENTURE_START_PROMPT.to_string()),
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

        // Update system prompt with current state
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

/// The main tool-call loop: keeps calling the LLM until we get a text response
/// or hit a pending user action (dice roll or choice).
async fn continue_tool_loop(
    session: &Arc<Mutex<Session>>,
    xai_client: &Arc<XaiClient>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
) -> anyhow::Result<()> {
    let tools = build_tool_definitions();
    let max_iterations = 15;

    for _ in 0..max_iterations {
        let messages = {
            let sess = session.lock().await;
            sess.messages.clone()
        };

        let response = xai_client.chat_with_tools(&messages, &tools).await?;

        match response {
            XaiResponse::Text(text) => {
                // Stream the text to the frontend in chunks
                let chunks: Vec<&str> = text
                    .as_bytes()
                    .chunks(80)
                    .map(|c| std::str::from_utf8(c).unwrap_or(""))
                    .collect();

                for chunk in &chunks {
                    if !chunk.is_empty() {
                        send_msg(sender, &ServerMsg::NarrativeChunk { text: chunk.to_string() }).await;
                        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    }
                }
                send_msg(sender, &ServerMsg::NarrativeEnd).await;

                // Save assistant message
                let mut sess = session.lock().await;
                sess.messages.push(ChatMessage::assistant_text(&text));

                let adv_id = sess.adventure.as_ref().map(|a| a.id.clone());
                if let Some(id) = adv_id {
                    sess.store.append_message(
                        &id,
                        &HistoryMessage {
                            role: "assistant".to_string(),
                            content: Some(text),
                            tool_calls: None,
                            tool_call_id: None,
                            timestamp: chrono::Utc::now(),
                        },
                    )?;
                }

                // Send state update
                if let Some(ref adv) = sess.adventure {
                    let state = serde_json::to_value(adv)?;
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                }

                return Ok(());
            }

            XaiResponse::ToolCalls { tool_calls, text } => {
                // If there's text alongside tool calls, send it
                if let Some(ref t) = text {
                    if !t.is_empty() {
                        send_msg(sender, &ServerMsg::NarrativeChunk { text: t.clone() }).await;
                    }
                }

                // Save the assistant message with tool calls
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

                // Execute each tool call
                for tc in &tool_calls {
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_default();

                    // Take adventure out to avoid borrow conflicts
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

                            // Put adventure back
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
                                dice_type,
                                count,
                                modifier,
                                dc,
                                description,
                                tool_call_id: tc.id.clone(),
                            });
                            sess.adventure = Some(adventure);
                            return Ok(());
                        }

                        Ok(ToolExecResult::PendingChoices {
                            choices,
                            allow_custom_input,
                            prompt,
                        }) => {
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
                            return Ok(());
                        }

                        Err(e) => {
                            // Put adventure back and report error as tool result
                            let err_msg = format!("Error: {}", e);
                            sess.messages.push(ChatMessage::tool_result(&tc.id, &err_msg));
                            sess.adventure = Some(adventure);
                        }
                    }
                }

                // Save adventure state after processing all tools
                if let Some(ref adventure) = sess.adventure {
                    sess.store.save_adventure(adventure)?;
                    let state = serde_json::to_value(adventure)?;
                    send_msg(sender, &ServerMsg::StateUpdate { state }).await;
                }

                drop(sess);
                // Continue the loop — call LLM again with tool results
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
