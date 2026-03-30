//! In-memory player-to-player trading registry.
//!
//! Follows the same Arc<RwLock<HashMap>> + broadcast pattern as PartyRegistry.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeItem {
    pub item_name: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TradeOffer {
    pub items: Vec<TradeItem>,
    pub gold: u32,
}

#[derive(Debug, Clone)]
pub enum TradeEvent {
    Started { partner: String },
    OfferUpdated { from: String, offer: TradeOffer },
    Accepted { by: String },
    Completed,
    Cancelled { by: String, reason: String },
}

pub struct TradeSession {
    pub id: String,
    pub player_a: String,
    pub player_b: String,
    pub offer_a: TradeOffer,
    pub offer_b: TradeOffer,
    pub accepted_a: bool,
    pub accepted_b: bool,
    pub tx: broadcast::Sender<TradeEvent>,
}

/// Shared registry for active trade sessions.
#[derive(Clone)]
pub struct TradeRegistry {
    /// Active trade sessions keyed by session ID
    sessions: Arc<RwLock<HashMap<String, TradeSession>>>,
    /// Map from username to active session ID
    user_sessions: Arc<RwLock<HashMap<String, String>>>,
    /// Pending trade requests: from -> to
    pending: Arc<RwLock<HashMap<String, String>>>,
}

impl TradeRegistry {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn request_trade(&self, from: &str, to: &str) -> Result<(), String> {
        let sessions = self.user_sessions.read().await;
        if sessions.contains_key(from) { return Err("Already in a trade".into()); }
        if sessions.contains_key(to) { return Err("Target is already trading".into()); }
        drop(sessions);

        self.pending.write().await.insert(from.to_string(), to.to_string());
        Ok(())
    }

    pub async fn accept_request(&self, accepter: &str, requester: &str) -> Result<broadcast::Receiver<TradeEvent>, String> {
        let mut pending = self.pending.write().await;
        match pending.get(requester) {
            Some(target) if target == accepter => {},
            _ => return Err("No pending trade request from this player".into()),
        }
        pending.remove(requester);
        drop(pending);

        // Double-check neither is now in a trade
        let sessions = self.user_sessions.read().await;
        if sessions.contains_key(requester) { return Err("Requester is now in another trade".into()); }
        if sessions.contains_key(accepter) { return Err("You are already in a trade".into()); }
        drop(sessions);

        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = broadcast::channel(32);

        let session = TradeSession {
            id: id.clone(),
            player_a: requester.to_string(),
            player_b: accepter.to_string(),
            offer_a: TradeOffer::default(),
            offer_b: TradeOffer::default(),
            accepted_a: false,
            accepted_b: false,
            tx,
        };

        self.sessions.write().await.insert(id.clone(), session);
        let mut user_sessions = self.user_sessions.write().await;
        user_sessions.insert(requester.to_string(), id.clone());
        user_sessions.insert(accepter.to_string(), id);

        Ok(rx)
    }

    pub async fn subscribe(&self, username: &str) -> Option<broadcast::Receiver<TradeEvent>> {
        let session_id = self.user_sessions.read().await.get(username).cloned()?;
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).map(|s| s.tx.subscribe())
    }

    pub async fn update_offer(&self, username: &str, offer: TradeOffer) -> Result<(), String> {
        let session_id = self.user_sessions.read().await.get(username).cloned()
            .ok_or("Not in a trade")?;
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&session_id).ok_or("Trade session not found")?;

        // Reset acceptances when offer changes
        session.accepted_a = false;
        session.accepted_b = false;

        if session.player_a == username {
            session.offer_a = offer.clone();
        } else {
            session.offer_b = offer.clone();
        }

        let _ = session.tx.send(TradeEvent::OfferUpdated {
            from: username.to_string(),
            offer,
        });
        Ok(())
    }

    pub async fn accept_trade(&self, username: &str) -> Result<bool, String> {
        let session_id = self.user_sessions.read().await.get(username).cloned()
            .ok_or("Not in a trade")?;
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&session_id).ok_or("Trade session not found")?;

        if session.player_a == username {
            session.accepted_a = true;
        } else {
            session.accepted_b = true;
        }

        let _ = session.tx.send(TradeEvent::Accepted { by: username.to_string() });

        // Both accepted?
        Ok(session.accepted_a && session.accepted_b)
    }

    pub async fn get_session_info(&self, username: &str) -> Option<(String, String, TradeOffer, TradeOffer, bool, bool)> {
        let session_id = self.user_sessions.read().await.get(username).cloned()?;
        let sessions = self.sessions.read().await;
        let session = sessions.get(&session_id)?;
        let partner = if session.player_a == username { &session.player_b } else { &session.player_a };
        let (my_offer, their_offer) = if session.player_a == username {
            (&session.offer_a, &session.offer_b)
        } else {
            (&session.offer_b, &session.offer_a)
        };
        Some((session_id.clone(), partner.clone(), my_offer.clone(), their_offer.clone(),
              session.accepted_a, session.accepted_b))
    }

    /// Remove the session and return (player_a, player_b, offer_a, offer_b).
    pub async fn complete_trade(&self, username: &str) -> Result<(String, String, TradeOffer, TradeOffer), String> {
        let session_id = self.user_sessions.read().await.get(username).cloned()
            .ok_or("Not in a trade")?;
        let mut sessions = self.sessions.write().await;
        let session = sessions.remove(&session_id).ok_or("Session not found")?;

        let _ = session.tx.send(TradeEvent::Completed);

        let mut user_sessions = self.user_sessions.write().await;
        user_sessions.remove(&session.player_a);
        user_sessions.remove(&session.player_b);

        Ok((session.player_a, session.player_b, session.offer_a, session.offer_b))
    }

    pub async fn cancel_trade(&self, username: &str, reason: &str) -> Result<(), String> {
        let session_id = self.user_sessions.read().await.get(username).cloned()
            .ok_or("Not in a trade")?;
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(&session_id) {
            let _ = session.tx.send(TradeEvent::Cancelled {
                by: username.to_string(),
                reason: reason.to_string(),
            });
            let mut user_sessions = self.user_sessions.write().await;
            user_sessions.remove(&session.player_a);
            user_sessions.remove(&session.player_b);
        }
        Ok(())
    }

    pub async fn has_pending_request(&self, to: &str) -> Option<String> {
        self.pending.read().await.iter()
            .find(|(_, target)| target.as_str() == to)
            .map(|(from, _)| from.clone())
    }

    pub async fn decline_request(&self, requester: &str) {
        self.pending.write().await.remove(requester);
    }

    pub async fn is_in_trade(&self, username: &str) -> bool {
        self.user_sessions.read().await.contains_key(username)
    }
}
