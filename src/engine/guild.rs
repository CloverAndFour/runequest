//! Guild system — player organizations for combat and crafting.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GuildType { Combat, Crafting }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GuildRank { Leader, Officer, Member, Recruit }

impl GuildRank {
    pub fn display(&self) -> &str {
        match self {
            GuildRank::Leader => "Leader",
            GuildRank::Officer => "Officer",
            GuildRank::Member => "Member",
            GuildRank::Recruit => "Recruit",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMember {
    pub username: String,
    pub rank: GuildRank,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub guild_type: GuildType,
    pub leader: String,
    pub members: Vec<GuildMember>,
    pub treasury_gold: u32,
    pub home_location: String,
    pub created_at: DateTime<Utc>,
    pub max_members: usize,
}

impl Guild {
    pub fn new(name: String, guild_type: GuildType, leader: String, location: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            guild_type,
            leader: leader.clone(),
            members: vec![GuildMember {
                username: leader,
                rank: GuildRank::Leader,
                joined_at: Utc::now(),
            }],
            treasury_gold: 0,
            home_location: location,
            created_at: Utc::now(),
            max_members: 50,
        }
    }

    pub fn add_member(&mut self, username: &str) -> Result<(), String> {
        if self.members.len() >= self.max_members {
            return Err("Guild is full".into());
        }
        if self.members.iter().any(|m| m.username == username) {
            return Err("Already a member".into());
        }
        self.members.push(GuildMember {
            username: username.to_string(),
            rank: GuildRank::Recruit,
            joined_at: Utc::now(),
        });
        Ok(())
    }

    pub fn remove_member(&mut self, username: &str) -> Result<(), String> {
        if username == self.leader {
            return Err("Leader cannot leave without transferring leadership".into());
        }
        if !self.members.iter().any(|m| m.username == username) {
            return Err("Not a member of this guild".into());
        }
        self.members.retain(|m| m.username != username);
        Ok(())
    }

    pub fn kick_member(&mut self, username: &str, by: &str) -> Result<(), String> {
        let kicker_rank = self.members.iter().find(|m| m.username == by)
            .map(|m| &m.rank).ok_or("You are not a member")?;
        if *kicker_rank != GuildRank::Leader && *kicker_rank != GuildRank::Officer {
            return Err("Only leader/officers can kick members".into());
        }
        if username == self.leader {
            return Err("Cannot kick the guild leader".into());
        }
        if !self.members.iter().any(|m| m.username == username) {
            return Err("Member not found".into());
        }
        self.members.retain(|m| m.username != username);
        Ok(())
    }

    pub fn promote(&mut self, username: &str, by: &str) -> Result<(), String> {
        let promoter_rank = self.members.iter().find(|m| m.username == by)
            .map(|m| &m.rank).ok_or("Not a member")?;
        if *promoter_rank != GuildRank::Leader && *promoter_rank != GuildRank::Officer {
            return Err("Only leader/officers can promote".into());
        }
        if let Some(member) = self.members.iter_mut().find(|m| m.username == username) {
            member.rank = match member.rank {
                GuildRank::Recruit => GuildRank::Member,
                GuildRank::Member => GuildRank::Officer,
                _ => return Err("Cannot promote further".into()),
            };
            Ok(())
        } else {
            Err("Member not found".into())
        }
    }

    pub fn donate_gold(&mut self, amount: u32) {
        self.treasury_gold += amount;
    }

    pub fn is_member(&self, username: &str) -> bool {
        self.members.iter().any(|m| m.username == username)
    }

    pub fn member_rank(&self, username: &str) -> Option<&GuildRank> {
        self.members.iter().find(|m| m.username == username).map(|m| &m.rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guild_creation() {
        let g = Guild::new("Test Guild".into(), GuildType::Combat, "alice".into(), "Crossroads Inn".into());
        assert_eq!(g.members.len(), 1);
        assert_eq!(g.leader, "alice");
        assert_eq!(g.members[0].rank, GuildRank::Leader);
    }

    #[test]
    fn test_add_member() {
        let mut g = Guild::new("Test Guild".into(), GuildType::Combat, "alice".into(), "Crossroads Inn".into());
        assert!(g.add_member("bob").is_ok());
        assert_eq!(g.members.len(), 2);
        assert!(g.add_member("bob").is_err()); // already a member
    }

    #[test]
    fn test_remove_member() {
        let mut g = Guild::new("Test Guild".into(), GuildType::Combat, "alice".into(), "Crossroads Inn".into());
        g.add_member("bob").unwrap();
        assert!(g.remove_member("bob").is_ok());
        assert_eq!(g.members.len(), 1);
        assert!(g.remove_member("alice").is_err()); // leader cannot leave
    }

    #[test]
    fn test_promote() {
        let mut g = Guild::new("Test Guild".into(), GuildType::Combat, "alice".into(), "Crossroads Inn".into());
        g.add_member("bob").unwrap();
        assert!(g.promote("bob", "alice").is_ok());
        assert_eq!(g.members[1].rank, GuildRank::Member);
        assert!(g.promote("bob", "alice").is_ok());
        assert_eq!(g.members[1].rank, GuildRank::Officer);
        assert!(g.promote("bob", "alice").is_err()); // cannot promote further
    }

    #[test]
    fn test_donate() {
        let mut g = Guild::new("Test Guild".into(), GuildType::Combat, "alice".into(), "Crossroads Inn".into());
        g.donate_gold(100);
        assert_eq!(g.treasury_gold, 100);
    }

    #[test]
    fn test_kick_member() {
        let mut g = Guild::new("Test Guild".into(), GuildType::Combat, "alice".into(), "Crossroads Inn".into());
        g.add_member("bob").unwrap();
        assert!(g.kick_member("bob", "alice").is_ok());
        assert_eq!(g.members.len(), 1);
        assert!(g.kick_member("alice", "alice").is_err()); // cannot kick leader
    }
}
