//! Party system: data structures for party formation, group combat, and PvP.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_PARTY_SIZE: usize = 4;
pub const COMBAT_TIMER_SECS: u64 = 30;
pub const CRIMINAL_DURATION_SECS: i64 = 1800; // 30 minutes

// ---------------------------------------------------------------------------
// Party
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: String,
    pub leader: String,
    pub members: Vec<PartyMember>,
    pub location: String,
    pub state: PartyState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    pub username: String,
    pub adventure_id: String,
    pub character_name: String,
    pub character_class: String,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub dex_mod: i32,
    pub ready: bool,
    pub disconnected: bool,
    pub incapacitated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartyState {
    Idle,
    InDungeon {
        dungeon_name: String,
        current_floor: usize,
        current_room: usize,
    },
    InCombat(PartyCombatState),
}

impl Default for PartyState {
    fn default() -> Self {
        PartyState::Idle
    }
}

// ---------------------------------------------------------------------------
// Party Combat
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyCombatState {
    pub round: u32,
    pub phase: CombatPhase,
    pub phase_deadline: DateTime<Utc>,
    pub submitted_actions: HashMap<String, PartyCombatAction>,
    pub enemies: Vec<PartyCombatEnemy>,
    pub initiative_order: Vec<PartyInitEntry>,
    pub combat_log: Vec<String>,
    pub flee_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CombatPhase {
    PlayerDecision,
    Resolution,
    EnemyTurn,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyCombatAction {
    pub username: String,
    pub action_id: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyCombatEnemy {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub ac: i32,
    pub attacks: Vec<EnemyAttackDef>,
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyAttackDef {
    pub name: String,
    pub damage_dice: String,
    pub damage_modifier: i32,
    pub to_hit_bonus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyInitEntry {
    pub name: String,
    pub username: Option<String>, // None for enemies
    pub roll: i32,
    pub is_player: bool,
    pub enemy_idx: Option<usize>,
}

// ---------------------------------------------------------------------------
// Party Trap Results
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrapMemberResult {
    pub username: String,
    pub character_name: String,
    pub detected: bool,
    pub detection_roll: i32,
    pub detection_dc: i32,
    pub saved: bool,
    pub save_roll: i32,
    pub save_dc: i32,
    pub damage: i32,
    pub condition: Option<String>,
    pub hp_after: i32,
}

// ---------------------------------------------------------------------------
// PvP
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvpChallenge {
    pub challenger: String,
    pub target: String,
    pub location: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvpCombatState {
    pub player_a: String,
    pub player_b: String,
    pub current_turn: String,
    pub round: u32,
    pub flee_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriminalStatus {
    pub username: String,
    pub expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

impl Party {
    pub fn new(leader_username: String, leader_member: PartyMember, location: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            leader: leader_username,
            members: vec![leader_member],
            location,
            state: PartyState::Idle,
            created_at: Utc::now(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.members.len() >= MAX_PARTY_SIZE
    }

    pub fn has_member(&self, username: &str) -> bool {
        self.members.iter().any(|m| m.username == username)
    }

    pub fn get_member(&self, username: &str) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.username == username)
    }

    pub fn get_member_mut(&mut self, username: &str) -> Option<&mut PartyMember> {
        self.members.iter_mut().find(|m| m.username == username)
    }

    pub fn living_members(&self) -> Vec<&PartyMember> {
        self.members.iter().filter(|m| !m.incapacitated && !m.disconnected).collect()
    }

    pub fn all_incapacitated(&self) -> bool {
        self.members.iter().all(|m| m.incapacitated || m.disconnected)
    }

    pub fn remove_member(&mut self, username: &str) -> bool {
        let before = self.members.len();
        self.members.retain(|m| m.username != username);
        if self.leader == username && !self.members.is_empty() {
            self.leader = self.members[0].username.clone();
        }
        self.members.len() < before
    }
}

impl PartyCombatState {
    pub fn all_enemies_dead(&self) -> bool {
        self.enemies.iter().all(|e| !e.alive || e.hp <= 0)
    }

    pub fn living_enemies(&self) -> Vec<(usize, &PartyCombatEnemy)> {
        self.enemies.iter().enumerate().filter(|(_, e)| e.alive && e.hp > 0).collect()
    }

    pub fn find_enemy_mut(&mut self, name: &str) -> Option<&mut PartyCombatEnemy> {
        self.enemies.iter_mut().find(|e| e.name.to_lowercase() == name.to_lowercase() && e.alive)
    }

    pub fn all_players_submitted(&self, living_members: &[&PartyMember]) -> bool {
        living_members.iter().all(|m| self.submitted_actions.contains_key(&m.username))
    }
}
