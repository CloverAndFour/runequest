//! In-memory party registry with event broadcasting.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::engine::party::*;

/// Events broadcast to party members.
#[derive(Debug, Clone)]
pub enum PartyEvent {
    MemberJoined { username: String, character_name: String, character_class: String },
    MemberLeft { username: String, reason: String },
    PartyDisbanded { reason: String },
    PartyInfo(Box<PartyInfoSnapshot>),
    NarrativeChunk { text: String },
    NarrativeEnd,
    StateUpdate { state: serde_json::Value },
    CombatStarted { enemies: Vec<PartyCombatEnemy>, initiative: Vec<PartyInitEntry> },
    CombatPhaseStart { deadline_ms: u64, round: u32 },
    CombatActionSubmitted { username: String },
    CombatAllReady,
    CombatResolution { results: Vec<serde_json::Value> },
    CombatEnemyPhase { results: Vec<serde_json::Value> },
    CombatEnded { victory: bool, xp_per_member: u32 },
    TrapResults { results: Vec<TrapMemberResult> },
    LocationChanged { location: String },
    DungeonEntered { dungeon_name: String },
    DungeonRoomChanged { room_name: String, room_type: String, description: String, exits: Vec<String> },
    // PvP
    PvpChallengeReceived { challenger: String, character_name: String },
    PvpStarted { opponent_name: String, opponent_class: String, opponent_hp: i32, opponent_ac: i32 },
    PvpTurnStart { your_turn: bool, round: u32 },
    PvpActionResult { result: serde_json::Value },
    PvpEnded { victory: bool, opponent: String },
}

#[derive(Debug, Clone)]
pub struct PartyInfoSnapshot {
    pub id: String,
    pub leader: String,
    pub members: Vec<PartyMemberSnapshot>,
    pub state: String,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct PartyMemberSnapshot {
    pub username: String,
    pub character_name: String,
    pub character_class: String,
    pub hp: i32,
    pub max_hp: i32,
    pub ready: bool,
    pub incapacitated: bool,
}

struct PartyEntry {
    party: Party,
    tx: broadcast::Sender<PartyEvent>,
}

/// Shared registry for active parties, PvP challenges, and criminal status.
#[derive(Clone)]
pub struct PartyRegistry {
    parties: Arc<RwLock<HashMap<String, PartyEntry>>>,
    user_party: Arc<RwLock<HashMap<String, String>>>,
    pending_invites: Arc<RwLock<HashMap<String, Vec<PendingInvite>>>>,
    pvp_challenges: Arc<RwLock<Vec<PvpChallenge>>>,
    criminals: Arc<RwLock<Vec<CriminalStatus>>>,
    pvp_combats: Arc<RwLock<HashMap<String, PvpCombatState>>>,
}

#[derive(Debug, Clone)]
pub struct PendingInvite {
    pub from: String,
    pub from_character: String,
    pub from_class: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PartyRegistry {
    pub fn new() -> Self {
        Self {
            parties: Arc::new(RwLock::new(HashMap::new())),
            user_party: Arc::new(RwLock::new(HashMap::new())),
            pending_invites: Arc::new(RwLock::new(HashMap::new())),
            pvp_challenges: Arc::new(RwLock::new(Vec::new())),
            criminals: Arc::new(RwLock::new(Vec::new())),
            pvp_combats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // --- Party CRUD ---

    pub async fn create_party(&self, leader: PartyMember, location: String) -> (String, broadcast::Receiver<PartyEvent>) {
        let username = leader.username.clone();
        let party = Party::new(username.clone(), leader, location);
        let party_id = party.id.clone();
        let (tx, rx) = broadcast::channel(128);
        self.parties.write().await.insert(party_id.clone(), PartyEntry { party, tx });
        self.user_party.write().await.insert(username, party_id.clone());
        (party_id, rx)
    }

    pub async fn add_member(&self, party_id: &str, member: PartyMember) -> Option<broadcast::Receiver<PartyEvent>> {
        let mut parties = self.parties.write().await;
        let entry = parties.get_mut(party_id)?;
        if entry.party.is_full() || entry.party.has_member(&member.username) {
            return None;
        }
        let username = member.username.clone();
        let char_name = member.character_name.clone();
        let char_class = member.character_class.clone();
        entry.party.members.push(member);
        let rx = entry.tx.subscribe();
        let _ = entry.tx.send(PartyEvent::MemberJoined {
            username: username.clone(),
            character_name: char_name,
            character_class: char_class,
        });
        drop(parties);
        self.user_party.write().await.insert(username, party_id.to_string());
        Some(rx)
    }

    pub async fn remove_member(&self, username: &str, reason: &str) -> Option<String> {
        let party_id = self.user_party.write().await.remove(username)?;
        let mut parties = self.parties.write().await;
        let entry = parties.get_mut(&party_id)?;
        entry.party.remove_member(username);
        let _ = entry.tx.send(PartyEvent::MemberLeft {
            username: username.to_string(),
            reason: reason.to_string(),
        });
        // Disband if empty or only 1 member left
        if entry.party.members.is_empty() {
            parties.remove(&party_id);
            return Some(party_id);
        }
        if entry.party.members.len() == 1 {
            let last = entry.party.members[0].username.clone();
            let _ = entry.tx.send(PartyEvent::PartyDisbanded {
                reason: "Not enough members".to_string(),
            });
            parties.remove(&party_id);
            self.user_party.write().await.remove(&last);
            return Some(party_id);
        }
        // Notify new leader if changed
        let _ = entry.tx.send(PartyEvent::PartyInfo(Box::new(self.snapshot_party_inner(&entry.party))));
        Some(party_id)
    }

    pub async fn disband(&self, party_id: &str, reason: &str) {
        let mut parties = self.parties.write().await;
        if let Some(entry) = parties.remove(party_id) {
            let _ = entry.tx.send(PartyEvent::PartyDisbanded { reason: reason.to_string() });
            let mut up = self.user_party.write().await;
            for m in &entry.party.members {
                up.remove(&m.username);
            }
        }
    }

    pub async fn get_party_for_user(&self, username: &str) -> Option<String> {
        self.user_party.read().await.get(username).cloned()
    }

    pub async fn get_party(&self, party_id: &str) -> Option<Party> {
        self.parties.read().await.get(party_id).map(|e| e.party.clone())
    }

    pub async fn update_party<F>(&self, party_id: &str, f: F) where F: FnOnce(&mut Party) {
        if let Some(entry) = self.parties.write().await.get_mut(party_id) {
            f(&mut entry.party);
        }
    }

    pub async fn broadcast(&self, party_id: &str, event: PartyEvent) {
        if let Some(entry) = self.parties.read().await.get(party_id) {
            let _ = entry.tx.send(event);
        }
    }

    pub async fn subscribe(&self, party_id: &str) -> Option<broadcast::Receiver<PartyEvent>> {
        self.parties.read().await.get(party_id).map(|e| e.tx.subscribe())
    }

    fn snapshot_party_inner(&self, party: &Party) -> PartyInfoSnapshot {
        PartyInfoSnapshot {
            id: party.id.clone(),
            leader: party.leader.clone(),
            members: party.members.iter().map(|m| PartyMemberSnapshot {
                username: m.username.clone(),
                character_name: m.character_name.clone(),
                character_class: m.character_class.clone(),
                hp: m.hp,
                max_hp: m.max_hp,
                ready: m.ready,
                incapacitated: m.incapacitated,
            }).collect(),
            state: match &party.state {
                PartyState::Idle => "idle".to_string(),
                PartyState::InDungeon { .. } => "dungeon".to_string(),
                PartyState::InCombat(_) => "combat".to_string(),
            },
            location: party.location.clone(),
        }
    }

    pub async fn snapshot_party(&self, party_id: &str) -> Option<PartyInfoSnapshot> {
        self.parties.read().await.get(party_id).map(|e| self.snapshot_party_inner(&e.party))
    }

    // --- Invites ---

    pub async fn add_invite(&self, target: &str, invite: PendingInvite) {
        self.pending_invites.write().await
            .entry(target.to_string())
            .or_default()
            .push(invite);
    }

    pub async fn get_invites(&self, target: &str) -> Vec<PendingInvite> {
        self.pending_invites.read().await
            .get(target)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn remove_invite(&self, target: &str, from: &str) -> Option<PendingInvite> {
        let mut invites = self.pending_invites.write().await;
        let list = invites.get_mut(target)?;
        let pos = list.iter().position(|i| i.from == from)?;
        Some(list.remove(pos))
    }

    // --- PvP ---

    pub async fn add_pvp_challenge(&self, challenge: PvpChallenge) {
        self.pvp_challenges.write().await.push(challenge);
    }

    pub async fn find_pvp_challenge(&self, challenger: &str, target: &str) -> Option<PvpChallenge> {
        self.pvp_challenges.read().await.iter()
            .find(|c| c.challenger == challenger && c.target == target)
            .cloned()
    }

    pub async fn remove_pvp_challenge(&self, challenger: &str, target: &str) {
        self.pvp_challenges.write().await.retain(|c| !(c.challenger == challenger && c.target == target));
    }

    pub async fn start_pvp(&self, state: PvpCombatState) -> String {
        let key = format!("{}:{}", state.player_a, state.player_b);
        self.pvp_combats.write().await.insert(key.clone(), state);
        key
    }

    pub async fn get_pvp(&self, username: &str) -> Option<PvpCombatState> {
        let combats = self.pvp_combats.read().await;
        combats.values().find(|c| c.player_a == username || c.player_b == username).cloned()
    }

    pub async fn update_pvp<F>(&self, key: &str, f: F) where F: FnOnce(&mut PvpCombatState) {
        if let Some(state) = self.pvp_combats.write().await.get_mut(key) {
            f(state);
        }
    }

    pub async fn end_pvp(&self, username: &str) {
        self.pvp_combats.write().await.retain(|_, c| c.player_a != username && c.player_b != username);
    }

    // --- Criminal status ---

    pub async fn add_criminal(&self, username: &str) {
        let expires = chrono::Utc::now() + chrono::Duration::seconds(CRIMINAL_DURATION_SECS);
        let mut criminals = self.criminals.write().await;
        criminals.retain(|c| c.username != username);
        criminals.push(CriminalStatus {
            username: username.to_string(),
            expires_at: expires,
        });
    }

    pub async fn is_criminal(&self, username: &str) -> bool {
        let now = chrono::Utc::now();
        let criminals = self.criminals.read().await;
        criminals.iter().any(|c| c.username == username && c.expires_at > now)
    }

    pub async fn clean_expired_criminals(&self) {
        let now = chrono::Utc::now();
        self.criminals.write().await.retain(|c| c.expires_at > now);
    }
}
