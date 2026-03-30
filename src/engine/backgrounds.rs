//! Character backgrounds — starting skills, items, and gold.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    Farmhand,
    ApprenticeSmith,
    StreetUrchin,
    Hunter,
    Acolyte,
    Scholar,
    Merchant,
    Herbalist,
    Woodcutter,
    Drifter,
}

impl Background {
    pub fn all() -> &'static [Background] {
        &[
            Background::Farmhand,
            Background::ApprenticeSmith,
            Background::StreetUrchin,
            Background::Hunter,
            Background::Acolyte,
            Background::Scholar,
            Background::Merchant,
            Background::Herbalist,
            Background::Woodcutter,
            Background::Drifter,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Farmhand => "Farmhand",
            Self::ApprenticeSmith => "Apprentice Smith",
            Self::StreetUrchin => "Street Urchin",
            Self::Hunter => "Hunter",
            Self::Acolyte => "Acolyte",
            Self::Scholar => "Scholar",
            Self::Merchant => "Merchant",
            Self::Herbalist => "Herbalist",
            Self::Woodcutter => "Woodcutter",
            Self::Drifter => "Drifter",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Farmhand => "Raised working the fields and tending livestock. Hardy and practical.",
            Self::ApprenticeSmith => "Trained at the forge, shaping metal into tools and weapons.",
            Self::StreetUrchin => "Grew up on the streets, surviving by wits and quick fingers.",
            Self::Hunter => "Lived off the land, tracking game through the wilds.",
            Self::Acolyte => "Served at a temple, learning the ways of divine healing.",
            Self::Scholar => "Studied ancient texts and arcane theory in a great library.",
            Self::Merchant => "Traveled trade routes, haggling and making deals.",
            Self::Herbalist => "Gathered herbs and brewed remedies in a remote village.",
            Self::Woodcutter => "Felled trees and shaped timber in the deep forest.",
            Self::Drifter => "No fixed home, no trade. Just a restless wanderer.",
        }
    }

    /// Returns (skill_id, rank) pairs for starting skills from this background.
    pub fn starting_skills(&self) -> Vec<(&'static str, u8)> {
        match self {
            Self::Farmhand => vec![("fortitude", 1), ("leatherworking", 1)],
            Self::ApprenticeSmith => vec![("smithing", 1), ("weapon_mastery", 1)],
            Self::StreetUrchin => vec![("stealth", 1), ("lockpicking", 1)],
            Self::Hunter => vec![("marksmanship", 1), ("tracking", 1)],
            Self::Acolyte => vec![("healing", 1), ("blessing", 1)],
            Self::Scholar => vec![("lore", 1), ("enchanting", 1)],
            Self::Merchant => vec![("charm", 1), ("inspire", 1)],
            Self::Herbalist => vec![("alchemy", 1), ("survival", 1)],
            Self::Woodcutter => vec![("woodworking", 1), ("fortitude", 1)],
            Self::Drifter => vec![],
        }
    }

    /// Starting gold for this background.
    pub fn starting_gold(&self) -> u32 {
        match self {
            Self::Merchant => 20,
            Self::Drifter => 0,
            _ => 5,
        }
    }

    /// Starting equipment item IDs.
    pub fn starting_items(&self) -> Vec<&'static str> {
        match self {
            Self::Farmhand => vec!["spear"],
            Self::ApprenticeSmith => vec!["mace"],
            Self::StreetUrchin => vec!["dagger"],
            Self::Hunter => vec!["shortbow"],
            Self::Acolyte => vec!["quarterstaff"],
            Self::Scholar => vec!["spellbook"],
            Self::Merchant => vec![],
            Self::Herbalist => vec![],
            Self::Woodcutter => vec!["handaxe"],
            Self::Drifter => vec![],
        }
    }

    /// Parse a background from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace(' ', "_").as_str() {
            "farmhand" => Some(Self::Farmhand),
            "apprentice_smith" => Some(Self::ApprenticeSmith),
            "street_urchin" => Some(Self::StreetUrchin),
            "hunter" => Some(Self::Hunter),
            "acolyte" => Some(Self::Acolyte),
            "scholar" => Some(Self::Scholar),
            "merchant" => Some(Self::Merchant),
            "herbalist" => Some(Self::Herbalist),
            "woodcutter" => Some(Self::Woodcutter),
            "drifter" => Some(Self::Drifter),
            _ => None,
        }
    }
}

impl Default for Background {
    fn default() -> Self {
        Self::Drifter
    }
}

impl std::fmt::Display for Background {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_backgrounds_have_names() {
        for bg in Background::all() {
            assert!(!bg.name().is_empty());
            assert!(!bg.description().is_empty());
        }
    }

    #[test]
    fn test_starting_skills_valid() {
        for bg in Background::all() {
            let skills = bg.starting_skills();
            for (id, rank) in &skills {
                assert!(!id.is_empty());
                assert!(*rank > 0);
            }
        }
    }

    #[test]
    fn test_background_from_str() {
        assert_eq!(Background::from_str("farmhand"), Some(Background::Farmhand));
        assert_eq!(Background::from_str("Apprentice Smith"), Some(Background::ApprenticeSmith));
        assert_eq!(Background::from_str("street_urchin"), Some(Background::StreetUrchin));
        assert_eq!(Background::from_str("nonexistent"), None);
    }

    #[test]
    fn test_default_is_drifter() {
        assert_eq!(Background::default(), Background::Drifter);
    }

    #[test]
    fn test_merchant_has_most_gold() {
        assert_eq!(Background::Merchant.starting_gold(), 20);
        assert_eq!(Background::Drifter.starting_gold(), 0);
        assert_eq!(Background::Farmhand.starting_gold(), 5);
    }
}
