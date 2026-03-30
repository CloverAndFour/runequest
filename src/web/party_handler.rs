//! WebSocket handlers for party, group combat, traps, and PvP.

use crate::engine::party::*;
use crate::web::party_registry::{PartyRegistry, PendingInvite, PartyEvent};
use crate::web::presence::PresenceRegistry;
use crate::web::protocol::*;
use crate::engine::dice::DiceRoller;

use axum::extract::ws::Message;
use futures_util::SinkExt;

/// Send a ServerMsg over the WebSocket.
async fn send(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    msg: &ServerMsg,
) {
    let json = serde_json::to_string(msg).unwrap();
    let _ = sender.send(Message::Text(json.into())).await;
}

// ---------------------------------------------------------------------------
// Party Formation
// ---------------------------------------------------------------------------

pub async fn handle_send_party_invite(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    character_name: &str,
    character_class: &str,
    location: &str,
    target_username: &str,
    party_registry: &PartyRegistry,
    presence: &PresenceRegistry,
) {
    // Check target is online and at same location
    let target_presence = presence.get(target_username).await;
    match target_presence {
        None => {
            send(sender, &ServerMsg::PartyInviteSent {
                success: false,
                message: "Player not online".to_string(),
            }).await;
            return;
        }
        Some(ref p) => {
            let target_loc = p.location.as_deref().unwrap_or("");
            if target_loc.to_lowercase() != location.to_lowercase() {
                send(sender, &ServerMsg::PartyInviteSent {
                    success: false,
                    message: "Player not at your location".to_string(),
                }).await;
                return;
            }
        }
    }

    // Check neither is already in a party
    if party_registry.get_party_for_user(username).await.is_some() {
        // If we're in a party and we're the leader, we can invite
        let pid = party_registry.get_party_for_user(username).await.unwrap();
        let party = party_registry.get_party(&pid).await;
        if let Some(p) = party {
            if p.leader != username {
                send(sender, &ServerMsg::PartyInviteSent {
                    success: false,
                    message: "Only the party leader can invite".to_string(),
                }).await;
                return;
            }
            if p.is_full() {
                send(sender, &ServerMsg::PartyInviteSent {
                    success: false,
                    message: "Party is full (max 4)".to_string(),
                }).await;
                return;
            }
        }
    }
    if party_registry.get_party_for_user(target_username).await.is_some() {
        send(sender, &ServerMsg::PartyInviteSent {
            success: false,
            message: "That player is already in a party".to_string(),
        }).await;
        return;
    }

    // Send invite
    party_registry.add_invite(target_username, PendingInvite {
        from: username.to_string(),
        from_character: character_name.to_string(),
        from_class: character_class.to_string(),
        created_at: chrono::Utc::now(),
    }).await;

    // Notify target via presence channel
    use crate::web::presence::FriendEvent;
    // We'll use the existing FriendEvent channel to deliver the invite notification
    // by having the websocket select loop handle a new PartyEvent type.
    // For now, send directly via presence
    presence.send_to(target_username, FriendEvent::FriendPresence {
        username: String::new(), friend_code: String::new(), online: false,
        character_name: None, character_class: None, location: None,
    }).await;
    // The actual invite message needs to go through party broadcast or direct send.
    // We'll use a simpler approach: store the invite and let the target poll or
    // we push via the party_event channel. For now, we use a workaround by
    // embedding the invite in a FriendEvent.

    send(sender, &ServerMsg::PartyInviteSent {
        success: true,
        message: format!("Invite sent to {}", target_username),
    }).await;
}

pub async fn handle_accept_party_invite(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    adventure_id: &str,
    character_name: &str,
    character_class: &str,
    hp: i32,
    max_hp: i32,
    ac: i32,
    dex_mod: i32,
    from_username: &str,
    party_registry: &PartyRegistry,
) -> Option<tokio::sync::broadcast::Receiver<PartyEvent>> {
    // Remove the invite
    let invite = party_registry.remove_invite(username, from_username).await;
    if invite.is_none() {
        send(sender, &ServerMsg::Error {
            code: "no_invite".to_string(),
            message: "No pending invite from that player".to_string(),
        }).await;
        return None;
    }

    let member = PartyMember {
        username: username.to_string(),
        adventure_id: adventure_id.to_string(),
        character_name: character_name.to_string(),
        character_class: character_class.to_string(),
        hp, max_hp, ac, dex_mod,
        ready: false,
        disconnected: false,
        incapacitated: false,
    };

    // Check if inviter already has a party
    let existing_party = party_registry.get_party_for_user(from_username).await;
    let rx = if let Some(pid) = existing_party {
        party_registry.add_member(&pid, member).await
    } else {
        // Create new party with inviter as leader
        let leader = PartyMember {
            username: from_username.to_string(),
            adventure_id: String::new(), // Will be updated
            character_name: invite.as_ref().map(|i| i.from_character.clone()).unwrap_or_default(),
            character_class: invite.as_ref().map(|i| i.from_class.clone()).unwrap_or_default(),
            hp: 0, max_hp: 0, ac: 10, dex_mod: 0,
            ready: false, disconnected: false, incapacitated: false,
        };
        // Get leader's presence info for location
        let loc = String::new();
        let (pid, _leader_rx) = party_registry.create_party(leader, loc).await;
        party_registry.add_member(&pid, member).await
    };

    if let Some(rx) = rx {
        // Send party info to acceptor
        let pid = party_registry.get_party_for_user(username).await.unwrap_or_default();
        if let Some(snap) = party_registry.snapshot_party(&pid).await {
            send(sender, &ServerMsg::PartyInfo {
                party_id: snap.id,
                leader: snap.leader,
                members: snap.members.iter().map(|m| PartyMemberInfo {
                    username: m.username.clone(),
                    character_name: m.character_name.clone(),
                    character_class: m.character_class.clone(),
                    hp: m.hp,
                    max_hp: m.max_hp,
                    ready: m.ready,
                    incapacitated: m.incapacitated,
                }).collect(),
                state: snap.state,
                location: snap.location,
            }).await;
        }
        return Some(rx);
    }

    send(sender, &ServerMsg::Error {
        code: "party_full".to_string(),
        message: "Could not join party".to_string(),
    }).await;
    None
}

pub async fn handle_leave_party(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    party_registry: &PartyRegistry,
) {
    let result = party_registry.remove_member(username, "left voluntarily").await;
    if result.is_some() {
        send(sender, &ServerMsg::PartyDisbanded { reason: "You left the party".to_string() }).await;
    }
}

pub async fn handle_kick_member(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    target: &str,
    party_registry: &PartyRegistry,
) {
    let pid = match party_registry.get_party_for_user(username).await {
        Some(p) => p,
        None => return,
    };
    let party = match party_registry.get_party(&pid).await {
        Some(p) => p,
        None => return,
    };
    if party.leader != username {
        send(sender, &ServerMsg::Error {
            code: "not_leader".to_string(),
            message: "Only the party leader can kick members".to_string(),
        }).await;
        return;
    }
    party_registry.remove_member(target, "kicked by leader").await;
}

pub async fn handle_get_party_info(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    party_registry: &PartyRegistry,
) {
    let pid = match party_registry.get_party_for_user(username).await {
        Some(p) => p,
        None => {
            send(sender, &ServerMsg::Error {
                code: "no_party".to_string(),
                message: "You are not in a party".to_string(),
            }).await;
            return;
        }
    };
    if let Some(snap) = party_registry.snapshot_party(&pid).await {
        send(sender, &ServerMsg::PartyInfo {
            party_id: snap.id,
            leader: snap.leader,
            members: snap.members.iter().map(|m| PartyMemberInfo {
                username: m.username.clone(),
                character_name: m.character_name.clone(),
                character_class: m.character_class.clone(),
                hp: m.hp, max_hp: m.max_hp, ready: m.ready, incapacitated: m.incapacitated,
            }).collect(),
            state: snap.state,
            location: snap.location,
        }).await;
    }
}

// ---------------------------------------------------------------------------
// Party Combat
// ---------------------------------------------------------------------------

pub async fn handle_party_combat_action(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    action_id: &str,
    target: Option<&str>,
    party_registry: &PartyRegistry,
) {
    let pid = match party_registry.get_party_for_user(username).await {
        Some(p) => p,
        None => return,
    };

    let mut party = match party_registry.get_party(&pid).await {
        Some(p) => p,
        None => return,
    };

    if let PartyState::InCombat(ref mut combat) = party.state {
        if combat.phase != CombatPhase::PlayerDecision {
            send(sender, &ServerMsg::Error {
                code: "wrong_phase".to_string(),
                message: "Not in player decision phase".to_string(),
            }).await;
            return;
        }

        combat.submitted_actions.insert(username.to_string(), PartyCombatAction {
            username: username.to_string(),
            action_id: action_id.to_string(),
            target: target.map(|t| t.to_string()),
        });

        // Update in registry
        let actions_clone = combat.submitted_actions.clone();
        let living_usernames: Vec<String> = party.members.iter()
            .filter(|m| !m.incapacitated && !m.disconnected)
            .map(|m| m.username.clone())
            .collect();
        let all_ready = living_usernames.iter().all(|u| combat.submitted_actions.contains_key(u));

        party_registry.update_party(&pid, |p| {
            if let PartyState::InCombat(ref mut c) = p.state {
                c.submitted_actions = actions_clone;
            }
        }).await;

        send(sender, &ServerMsg::PartyCombatActionAck).await;
        party_registry.broadcast(&pid, PartyEvent::CombatActionSubmitted {
            username: username.to_string(),
        }).await;

        if all_ready {
            party_registry.broadcast(&pid, PartyEvent::CombatAllReady).await;
        }
    }
}

pub async fn handle_party_combat_ready(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    party_registry: &PartyRegistry,
) {
    // "Ready" without an action = dodge
    handle_party_combat_action(sender, username, "dodge", None, party_registry).await;
}

// ---------------------------------------------------------------------------
// PvP
// ---------------------------------------------------------------------------

pub async fn handle_pvp_challenge(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    username: &str,
    location: &str,
    target: &str,
    party_registry: &PartyRegistry,
    presence: &PresenceRegistry,
) {
    if username == target {
        send(sender, &ServerMsg::PvpChallengeSent {
            success: false, message: "Cannot challenge yourself".to_string(),
        }).await;
        return;
    }

    let target_pres = presence.get(target).await;
    match target_pres {
        None => {
            send(sender, &ServerMsg::PvpChallengeSent {
                success: false, message: "Player not online".to_string(),
            }).await;
            return;
        }
        Some(ref p) => {
            let tloc = p.location.as_deref().unwrap_or("");
            if tloc.to_lowercase() != location.to_lowercase() {
                send(sender, &ServerMsg::PvpChallengeSent {
                    success: false, message: "Player not at your location".to_string(),
                }).await;
                return;
            }
        }
    }

    // If target is a criminal, skip the accept step — auto-start
    if party_registry.is_criminal(target).await {
        send(sender, &ServerMsg::PvpChallengeSent {
            success: true, message: "Attacking criminal — combat starts!".to_string(),
        }).await;
        // Start PvP immediately (handled by caller who checks criminal status)
        return;
    }

    party_registry.add_pvp_challenge(PvpChallenge {
        challenger: username.to_string(),
        target: target.to_string(),
        location: location.to_string(),
        created_at: chrono::Utc::now(),
    }).await;

    // Notify target
    let char_name = presence.get(username).await
        .and_then(|p| p.character_name.clone())
        .unwrap_or_else(|| username.to_string());
    presence.send_to(target, crate::web::presence::FriendEvent::FriendPresence {
        username: String::new(), friend_code: String::new(), online: false,
        character_name: None, character_class: None, location: None,
    }).await;

    send(sender, &ServerMsg::PvpChallengeSent {
        success: true, message: format!("Challenge sent to {}", target),
    }).await;
}

// ---------------------------------------------------------------------------
// Dice helpers for party combat
// ---------------------------------------------------------------------------

pub fn roll_d20(modifier: i32) -> i32 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let roll: i32 = rng.gen_range(1..=20);
    roll + modifier
}

pub fn roll_damage(dice: &str, modifier: i32) -> i32 {
    let result = DiceRoller::roll(dice, 1, modifier);
    result.total.max(1)
}

/// Resolve all submitted party combat actions, then enemy turns.
/// Returns (victory, all_dead) after resolving.
pub fn resolve_party_combat_round(
    party: &mut Party,
) -> (Vec<CombatResolutionEntry>, Vec<EnemyAttackResult>, bool, bool) {
    let mut player_results = Vec::new();
    let mut enemy_results = Vec::new();

    if let PartyState::InCombat(ref mut combat) = party.state {
        // Fill defaults for anyone who didn't submit
        for m in &party.members {
            if !m.incapacitated && !m.disconnected {
                if !combat.submitted_actions.contains_key(&m.username) {
                    combat.submitted_actions.insert(m.username.clone(), PartyCombatAction {
                        username: m.username.clone(),
                        action_id: "dodge".to_string(),
                        target: None,
                    });
                }
            }
        }

        // Resolve player actions in initiative order
        let init_order: Vec<PartyInitEntry> = combat.initiative_order.clone();
        for entry in &init_order {
            if !entry.is_player { continue; }
            let uname = match &entry.username { Some(u) => u.clone(), None => continue };
            let action = match combat.submitted_actions.get(&uname) {
                Some(a) => a.clone(),
                None => continue,
            };

            let member = match party.members.iter().find(|m| m.username == uname) {
                Some(m) => m.clone(),
                None => continue,
            };
            if member.incapacitated { continue; }

            match action.action_id.as_str() {
                "attack" => {
                    // Find target enemy
                    let target_name = action.target.clone().unwrap_or_else(|| {
                        combat.living_enemies().first().map(|(_, e)| e.name.clone()).unwrap_or_default()
                    });
                    let attack_roll = roll_d20(member.dex_mod + 2); // simplified: dex + proficiency
                    if let Some(enemy) = combat.find_enemy_mut(&target_name) {
                        let hit = attack_roll >= enemy.ac;
                        let damage = if hit { roll_damage("1d8", member.dex_mod).max(1) } else { 0 };
                        if hit { enemy.hp -= damage; if enemy.hp <= 0 { enemy.alive = false; } }
                        player_results.push(CombatResolutionEntry {
                            actor: member.character_name.clone(),
                            action: "attack".to_string(),
                            target: Some(target_name),
                            description: if hit {
                                format!("{} hits for {} damage!", member.character_name, damage)
                            } else {
                                format!("{} misses!", member.character_name)
                            },
                            roll: Some(attack_roll),
                            hit: Some(hit),
                            damage: if hit { Some(damage) } else { None },
                        });
                    }
                }
                "dodge" => {
                    player_results.push(CombatResolutionEntry {
                        actor: member.character_name.clone(),
                        action: "dodge".to_string(),
                        target: None,
                        description: format!("{} takes the Dodge action.", member.character_name),
                        roll: None, hit: None, damage: None,
                    });
                }
                "use_item" => {
                    // Heal with potion
                    let heal = roll_damage("2d4", 2);
                    if let Some(m) = party.members.iter_mut().find(|m| m.username == uname) {
                        m.hp = (m.hp + heal).min(m.max_hp);
                    }
                    player_results.push(CombatResolutionEntry {
                        actor: member.character_name.clone(),
                        action: "use_item".to_string(),
                        target: None,
                        description: format!("{} uses a potion, healing {} HP.", member.character_name, heal),
                        roll: None, hit: None, damage: Some(heal),
                    });
                }
                _ => {
                    player_results.push(CombatResolutionEntry {
                        actor: member.character_name.clone(),
                        action: action.action_id.clone(),
                        target: None,
                        description: format!("{} uses {}.", member.character_name, action.action_id),
                        roll: None, hit: None, damage: None,
                    });
                }
            }
        }

        // Check if all enemies dead
        if combat.all_enemies_dead() {
            return (player_results, enemy_results, true, false);
        }

        // Enemy phase: each living enemy attacks a random living player
        use rand::seq::SliceRandom;
        let living_players: Vec<String> = party.members.iter()
            .filter(|m| !m.incapacitated && !m.disconnected)
            .map(|m| m.username.clone())
            .collect();

        if living_players.is_empty() {
            return (player_results, enemy_results, false, true);
        }

        for enemy in &mut combat.enemies {
            if !enemy.alive || enemy.hp <= 0 { continue; }
            if enemy.attacks.is_empty() { continue; }

            let best_attack = enemy.attacks.iter()
                .max_by_key(|a| a.to_hit_bonus)
                .unwrap()
                .clone();

            let target_username = {
                let mut rng = rand::thread_rng();
                living_players.choose(&mut rng).cloned().unwrap_or_default()
            };

            let target_member = party.members.iter().find(|m| m.username == target_username);
            let target_ac = target_member.map(|m| m.ac).unwrap_or(10);
            let target_name = target_member.map(|m| m.character_name.clone()).unwrap_or_default();

            let attack_roll = roll_d20(best_attack.to_hit_bonus);
            let hit = attack_roll >= target_ac;
            let damage = if hit { roll_damage(&best_attack.damage_dice, best_attack.damage_modifier).max(1) } else { 0 };

            if hit {
                if let Some(m) = party.members.iter_mut().find(|m| m.username == target_username) {
                    m.hp -= damage;
                    if m.hp <= 0 { m.hp = 0; m.incapacitated = true; }
                }
            }

            let target_hp = party.members.iter().find(|m| m.username == target_username).map(|m| m.hp).unwrap_or(0);
            let target_max = party.members.iter().find(|m| m.username == target_username).map(|m| m.max_hp).unwrap_or(0);

            enemy_results.push(EnemyAttackResult {
                enemy_name: enemy.name.clone(),
                attack_name: best_attack.name.clone(),
                target: target_name,
                attack_roll,
                target_ac,
                hit,
                damage,
                target_hp,
                target_max_hp: target_max,
            });
        }

        // Clear submitted actions for next round
        combat.submitted_actions.clear();
        combat.round += 1;

        // Check TPK
        let all_dead = party.members.iter().all(|m| m.incapacitated || m.disconnected);

        (player_results, enemy_results, combat.all_enemies_dead(), all_dead)
    } else {
        (player_results, enemy_results, false, false)
    }
}
