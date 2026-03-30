//! Skill system — per-skill XP with ranks 0-10.

use serde::{Deserialize, Serialize};

use super::character::Class;

/// A single skill with a rank from 0 (Untrained) to 10 (Transcendent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rank: u8,
    #[serde(default)]
    pub xp: u32,
    #[serde(default)]
    pub xp_to_next: u32,
}

/// A character's full set of skills.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillSet {
    pub skills: Vec<Skill>,
}

/// XP threshold to rank up from the given rank.
pub fn xp_for_rank(rank: u8) -> u32 {
    match rank {
        0 => 50,       // Untrained -> Novice
        1 => 100,      // Novice -> Apprentice
        2 => 300,      // Apprentice -> Journeyman
        3 => 800,      // Journeyman -> Adept
        4 => 2000,     // Adept -> Expert
        5 => 5000,     // Expert -> Master
        6 => 12000,    // Master -> Grandmaster
        7 => 30000,    // Grandmaster -> Legendary
        8 => 70000,    // Legendary -> Mythic
        9 => 150000,   // Mythic -> Transcendent
        _ => u32::MAX,
    }
}

impl SkillSet {
    /// Find a skill by ID.
    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.id == id)
    }

    /// Find a skill by ID (mutable).
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Skill> {
        self.skills.iter_mut().find(|s| s.id == id)
    }

    /// Improve a skill by 1 rank. Returns the new rank, or None if already max.
    pub fn improve(&mut self, id: &str) -> Option<u8> {
        if let Some(skill) = self.get_mut(id) {
            if skill.rank < 10 {
                skill.rank += 1;
                skill.xp_to_next = xp_for_rank(skill.rank);
                Some(skill.rank)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Award XP to a skill. Returns (new_rank, ranked_up) or None if skill not found.
    pub fn gain_xp(&mut self, skill_id: &str, amount: u32) -> Option<(u8, bool)> {
        let skill = self.get_mut(skill_id)?;
        if skill.rank >= 10 {
            return Some((skill.rank, false));
        }
        skill.xp += amount;
        skill.xp_to_next = xp_for_rank(skill.rank);
        let ranked_up = skill.xp >= skill.xp_to_next;
        if ranked_up {
            skill.xp -= skill.xp_to_next;
            skill.rank += 1;
            skill.xp_to_next = xp_for_rank(skill.rank);
        }
        Some((skill.rank, ranked_up))
    }

    /// Get a formatted summary of all skills.
    pub fn summary(&self) -> String {
        if self.skills.is_empty() {
            return "No skills".to_string();
        }
        self.skills
            .iter()
            .map(|s| format!("{}: {} ({})", s.name, rank_name(s.rank), s.rank))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Human-readable rank name.
pub fn rank_name(rank: u8) -> &'static str {
    match rank {
        0 => "Untrained",
        1 => "Novice",
        2 => "Apprentice",
        3 => "Journeyman",
        4 => "Adept",
        5 => "Expert",
        6 => "Master",
        7 => "Grandmaster",
        8 => "Legendary",
        9 => "Mythic",
        10 => "Transcendent",
        _ => "Unknown",
    }
}

/// Derive a class label from the character's highest-ranked combat skills.
/// This is for FLAVOR/DISPLAY only -- not used mechanically.
pub fn derived_class_label(skills: &SkillSet) -> &'static str {
    let families: &[(&[&str], &str)] = &[
        (&["weapon_mastery", "shield_wall", "fortitude"], "Warrior"),
        (&["rage", "reckless_fury", "primal_toughness"], "Berserker"),
        (&["holy_smite", "divine_shield", "lay_on_hands"], "Paladin"),
        (&["blade_finesse", "stealth", "lockpicking", "evasion"], "Rogue"),
        (&["marksmanship", "tracking", "beast_lore", "survival"], "Ranger"),
        (&["martial_arts", "ki_focus", "iron_body", "flurry"], "Monk"),
        (&["evocation", "abjuration", "spell_mastery"], "Mage"),
        (&["eldritch_blast", "curse_weaving", "soul_harvest"], "Warlock"),
        (&["healing", "blessing", "turn_undead"], "Cleric"),
        (&["inspire", "lore", "charm", "song_of_rest"], "Bard"),
    ];

    let mut best_label = "Adventurer";
    let mut best_total: u32 = 0;

    for (skill_ids, label) in families {
        let total: u32 = skill_ids.iter()
            .filter_map(|id| skills.get(id).map(|s| s.rank as u32))
            .sum();
        if total > best_total {
            best_total = total;
            best_label = label;
        }
    }

    if best_total == 0 { "Adventurer" } else { best_label }
}

fn skill(id: &str, name: &str, desc: &str, rank: u8) -> Skill {
    Skill {
        id: id.to_string(),
        name: name.to_string(),
        description: desc.to_string(),
        rank,
        xp: 0,
        xp_to_next: xp_for_rank(rank),
    }
}

/// Generate ALL skills at rank 0. Background applies starting ranks.
pub fn all_skills() -> SkillSet {
    let mut skills = vec![
        // Combat skills
        skill("weapon_mastery", "Weapon Mastery", "Proficiency with melee weapons. Increases hit chance and damage.", 0),
        skill("shield_wall", "Shield Wall", "Defensive shield techniques. Increases AC when shield equipped.", 0),
        skill("fortitude", "Fortitude", "Physical resilience. Increases max HP and condition resistance.", 0),
        skill("rage", "Rage", "Fury in battle. Increases rage damage bonus and duration.", 0),
        skill("reckless_fury", "Reckless Fury", "Wild offensive strikes. Increases damage at the cost of defense.", 0),
        skill("primal_toughness", "Primal Toughness", "Raw physical endurance. Increases max HP and damage resistance.", 0),
        skill("holy_smite", "Holy Smite", "Divine radiant strikes. Increases smite damage, especially vs undead.", 0),
        skill("divine_shield", "Divine Shield", "Holy protection. Increases AC and resistance to dark magic.", 0),
        skill("lay_on_hands", "Lay on Hands", "Divine healing touch. Increases healing pool size.", 0),
        skill("blade_finesse", "Blade Finesse", "Precision strikes with light weapons. Increases sneak attack damage.", 0),
        skill("stealth", "Stealth", "Moving unseen. Increases hide effectiveness and surprise attack chance.", 0),
        skill("lockpicking", "Lockpicking", "Opening locks and disabling traps. Reduces DCs for mechanical challenges.", 0),
        skill("evasion", "Evasion", "Dodging area effects. Chance to halve or negate AoE damage.", 0),
        skill("marksmanship", "Marksmanship", "Ranged weapon accuracy. Increases hit chance and damage with bows.", 0),
        skill("tracking", "Tracking", "Reading the wilderness. Bonus to initiative and detecting hidden enemies.", 0),
        skill("beast_lore", "Beast Lore", "Knowledge of creatures. Bonus damage vs beasts and natural enemies.", 0),
        skill("survival", "Survival", "Living off the land. Improved trap detection and environmental resistance.", 0),
        skill("martial_arts", "Martial Arts", "Unarmed combat mastery. Increases unarmed damage dice and to-hit.", 0),
        skill("ki_focus", "Ki Focus", "Inner energy control. Increases ki points and special ability power.", 0),
        skill("iron_body", "Iron Body", "Body hardening. Increases unarmored AC and condition resistance.", 0),
        skill("flurry", "Flurry", "Rapid strikes. Improves Flurry of Blows damage and accuracy.", 0),
        skill("evocation", "Evocation", "Destructive spell power. Increases spell damage and AoE radius.", 0),
        skill("abjuration", "Abjuration", "Protective wards. Increases shield/ward absorption.", 0),
        skill("spell_mastery", "Spell Mastery", "Arcane efficiency. Bonus spell slots and reduced misfire.", 0),
        skill("eldritch_blast", "Eldritch Blast", "Eldritch force mastery. Increases blast damage and adds effects.", 0),
        skill("curse_weaving", "Curse Weaving", "Dark enchantments. Increases hex/curse duration and potency.", 0),
        skill("soul_harvest", "Soul Harvest", "Life draining power. Heal on kills, life drain attacks.", 0),
        skill("healing", "Healing", "Divine restoration. Increases healing spell power.", 0),
        skill("blessing", "Blessing", "Holy buffs. Increases buff duration and strength.", 0),
        skill("turn_undead", "Turn Undead", "Divine repulsion of undead. Increases damage and fear radius vs undead.", 0),
        skill("inspire", "Inspire", "Motivating allies. Increases Bardic Inspiration bonus dice.", 0),
        skill("lore", "Lore", "Vast knowledge. Identify items, recall monster weaknesses, bonus to knowledge checks.", 0),
        skill("charm", "Charm", "Social manipulation. Improved NPC persuasion and enemy confusion.", 0),
        skill("song_of_rest", "Song of Rest", "Restorative melodies. Heals party during short rests.", 0),
    ];

    // Add crafting skills
    skills.extend(crafting_skills());

    SkillSet { skills }
}

/// Apply background starting skills to a skill set.
pub fn apply_background(skills: &mut SkillSet, background: &super::backgrounds::Background) {
    for (skill_id, rank) in background.starting_skills() {
        if let Some(s) = skills.get_mut(skill_id) {
            s.rank = s.rank.max(rank);
            s.xp_to_next = xp_for_rank(s.rank);
        }
    }
}

/// Generate starting skills for a character class (legacy -- kept for backwards compat).
/// All start at rank 1 (Novice).
pub fn starting_skills(class: &Class) -> SkillSet {
    let class_skill_ids: Vec<&str> = match class {
        Class::Warrior => vec!["weapon_mastery", "shield_wall", "fortitude"],
        Class::Berserker => vec!["rage", "reckless_fury", "primal_toughness"],
        Class::Paladin => vec!["holy_smite", "divine_shield", "lay_on_hands"],
        Class::Rogue => vec!["blade_finesse", "stealth", "lockpicking", "evasion"],
        Class::Ranger => vec!["marksmanship", "tracking", "beast_lore", "survival"],
        Class::Monk => vec!["martial_arts", "ki_focus", "iron_body", "flurry"],
        Class::Mage => vec!["evocation", "abjuration", "spell_mastery"],
        Class::Warlock => vec!["eldritch_blast", "curse_weaving", "soul_harvest"],
        Class::Cleric => vec!["healing", "blessing", "turn_undead"],
        Class::Bard => vec!["inspire", "lore", "charm", "song_of_rest"],
    };

    // Start with all skills at rank 0
    let mut skills = all_skills();
    // Set class skills to rank 1
    for skill_id in class_skill_ids {
        if let Some(s) = skills.get_mut(skill_id) {
            s.rank = 1;
            s.xp_to_next = xp_for_rank(1);
        }
    }
    skills
}

/// Crafting skills available to ALL characters regardless of class.
/// All start at rank 0 (Untrained).
pub fn crafting_skills() -> Vec<Skill> {
    vec![
        skill("leatherworking", "Leatherworking", "Working hides and leather into materials and armor.", 0),
        skill("smithing", "Smithing", "Forging metals into weapons, armor, and tools.", 0),
        skill("woodworking", "Woodworking", "Crafting wood into bows, staves, and construction materials.", 0),
        skill("alchemy", "Alchemy", "Brewing potions, poisons, and alchemical reagents.", 0),
        skill("enchanting", "Enchanting", "Infusing items with magical properties.", 0),
        skill("tailoring", "Tailoring", "Weaving cloth and magical fabrics into garments.", 0),
        skill("jewelcrafting", "Jewelcrafting", "Cutting gems and crafting precious jewelry.", 0),
        skill("runecrafting", "Runecrafting", "Inscribing magical runes and glyphs.", 0),
        skill("artificing", "Artificing", "Constructing complex magical mechanisms.", 0),
        skill("theurgy", "Theurgy", "Divine crafting of holy and primordial items.", 0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starting_skills_warrior() {
        let skills = starting_skills(&Class::Warrior);
        // All 34 combat skills + 10 crafting = 44 skills
        assert!(
            skills.skills.len() >= 40,
            "Expected >= 40 skills, got {}",
            skills.skills.len()
        );
        assert!(skills.get("weapon_mastery").is_some());
        assert!(skills.get("shield_wall").is_some());
        assert!(skills.get("fortitude").is_some());
        assert_eq!(skills.get("weapon_mastery").unwrap().rank, 1);
        // Non-class skills should be rank 0
        assert_eq!(skills.get("stealth").unwrap().rank, 0);
    }

    #[test]
    fn test_all_skills_count() {
        let skills = all_skills();
        // 34 combat + 10 crafting = 44
        assert_eq!(skills.skills.len(), 44);
        // All at rank 0
        for s in &skills.skills {
            assert_eq!(s.rank, 0, "Skill {} should be rank 0", s.id);
            assert_eq!(s.xp, 0);
            assert_eq!(s.xp_to_next, 50); // xp_for_rank(0) = 50
        }
    }

    #[test]
    fn test_gain_xp_basic() {
        let mut skills = all_skills();
        let result = skills.gain_xp("stealth", 30);
        assert_eq!(result, Some((0, false)));
        assert_eq!(skills.get("stealth").unwrap().xp, 30);
    }

    #[test]
    fn test_gain_xp_rank_up() {
        let mut skills = all_skills();
        let result = skills.gain_xp("stealth", 50);
        assert_eq!(result, Some((1, true)));
        assert_eq!(skills.get("stealth").unwrap().rank, 1);
        assert_eq!(skills.get("stealth").unwrap().xp, 0);
        assert_eq!(skills.get("stealth").unwrap().xp_to_next, 100); // xp_for_rank(1)
    }

    #[test]
    fn test_gain_xp_overflow() {
        let mut skills = all_skills();
        // Give 60 XP, only 50 needed for rank 0->1, so 10 carries over
        let result = skills.gain_xp("stealth", 60);
        assert_eq!(result, Some((1, true)));
        assert_eq!(skills.get("stealth").unwrap().xp, 10);
    }

    #[test]
    fn test_gain_xp_max_rank() {
        let mut skills = all_skills();
        // Manually set to rank 10
        skills.get_mut("stealth").unwrap().rank = 10;
        let result = skills.gain_xp("stealth", 100);
        assert_eq!(result, Some((10, false)));
    }

    #[test]
    fn test_gain_xp_unknown_skill() {
        let mut skills = all_skills();
        assert_eq!(skills.gain_xp("nonexistent", 100), None);
    }

    #[test]
    fn test_apply_background() {
        use super::super::backgrounds::Background;
        let mut skills = all_skills();
        apply_background(&mut skills, &Background::Hunter);
        assert_eq!(skills.get("marksmanship").unwrap().rank, 1);
        assert_eq!(skills.get("tracking").unwrap().rank, 1);
        assert_eq!(skills.get("stealth").unwrap().rank, 0); // Not a hunter skill
    }

    #[test]
    fn test_improve_skill() {
        let mut skills = starting_skills(&Class::Warrior);
        let new_rank = skills.improve("weapon_mastery");
        assert_eq!(new_rank, Some(2));
        assert_eq!(skills.get("weapon_mastery").unwrap().rank, 2);
    }

    #[test]
    fn test_improve_skill_max() {
        let mut skills = starting_skills(&Class::Warrior);
        // Improve to rank 10
        for _ in 0..9 {
            skills.improve("weapon_mastery");
        }
        assert_eq!(skills.get("weapon_mastery").unwrap().rank, 10);
        // Try to go beyond 10
        assert_eq!(skills.improve("weapon_mastery"), None);
    }

    #[test]
    fn test_improve_nonexistent_skill() {
        let mut skills = starting_skills(&Class::Warrior);
        assert_eq!(skills.improve("nonexistent"), None);
    }

    #[test]
    fn test_rank_name() {
        assert_eq!(rank_name(0), "Untrained");
        assert_eq!(rank_name(1), "Novice");
        assert_eq!(rank_name(5), "Expert");
        assert_eq!(rank_name(10), "Transcendent");
        assert_eq!(rank_name(11), "Unknown");
    }

    #[test]
    fn test_summary() {
        let skills = starting_skills(&Class::Mage);
        let summary = skills.summary();
        assert!(summary.contains("Evocation: Novice (1)"));
        assert!(summary.contains("Abjuration: Novice (1)"));
        assert!(summary.contains("Spell Mastery: Novice (1)"));
    }

    #[test]
    fn test_all_classes_have_skills() {
        let classes = vec![
            Class::Warrior,
            Class::Mage,
            Class::Rogue,
            Class::Cleric,
            Class::Ranger,
            Class::Berserker,
            Class::Paladin,
            Class::Monk,
            Class::Warlock,
            Class::Bard,
        ];
        for class in classes {
            let skills = starting_skills(&class);
            assert!(
                !skills.skills.is_empty(),
                "Class {:?} has no skills",
                class
            );
            for skill in &skills.skills {
                assert!(
                    skill.rank <= 1,
                    "Skill {} should start at rank 0 or 1, got {}",
                    skill.name,
                    skill.rank
                );
                assert!(!skill.id.is_empty());
                assert!(!skill.name.is_empty());
                assert!(!skill.description.is_empty());
            }
        }
    }

    #[test]
    fn test_empty_skillset_summary() {
        let skills = SkillSet::default();
        assert_eq!(skills.summary(), "No skills");
    }

    #[test]
    fn test_xp_thresholds() {
        assert_eq!(xp_for_rank(0), 50);
        assert_eq!(xp_for_rank(1), 100);
        assert_eq!(xp_for_rank(9), 150000);
        assert_eq!(xp_for_rank(10), u32::MAX);
    }
}
