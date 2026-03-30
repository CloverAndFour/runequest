//! Image generation via xAI Grok Imagine API.
//!
//! Generates and caches entity images (items, monsters, NPCs) to disk.
//! Uses `grok-imagine-image` model at $0.02/image.

use base64::Engine;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs;

const XAI_API_BASE: &str = "https://api.x.ai/v1";
const IMAGE_MODEL: &str = "grok-imagine-image";

/// Style prefix for all image generation prompts.
const STYLE_PREFIX: &str = "Detailed fantasy RPG illustration, dark moody background, dramatic lighting. Digital painting style, rich colors, no text or labels.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageCategory {
    Item,
    Monster,
    Npc,
}

impl ImageCategory {
    pub fn dir_name(&self) -> &str {
        match self {
            Self::Item => "items",
            Self::Monster => "monsters",
            Self::Npc => "npcs",
        }
    }
}

/// Image generator with disk caching.
#[derive(Clone)]
pub struct ImageGenerator {
    client: Client,
    api_key: String,
    images_dir: PathBuf,
}

impl ImageGenerator {
    pub fn new(api_key: &str, data_dir: &Path) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            images_dir: data_dir.join("images"),
        }
    }

    /// Directory for a given category.
    fn category_dir(&self, category: ImageCategory) -> PathBuf {
        self.images_dir.join(category.dir_name())
    }

    /// Full file path for an image.
    pub fn path(&self, category: ImageCategory, key: &str) -> PathBuf {
        self.category_dir(category).join(format!("{}.jpg", sanitize_key(key)))
    }

    /// Check if an image already exists on disk.
    pub fn exists(&self, category: ImageCategory, key: &str) -> bool {
        self.path(category, key).exists()
    }

    /// Ensure category directories exist.
    pub async fn ensure_dirs(&self) -> std::io::Result<()> {
        for cat in &[ImageCategory::Item, ImageCategory::Monster, ImageCategory::Npc] {
            fs::create_dir_all(self.category_dir(*cat)).await?;
        }
        Ok(())
    }

    /// Generate an image and save to disk. Returns the file path on success.
    pub async fn generate_and_save(
        &self,
        category: ImageCategory,
        key: &str,
        prompt: &str,
    ) -> Result<PathBuf, String> {
        let sanitized = sanitize_key(key);
        let file_path = self.path(category, &sanitized);

        // Already cached
        if file_path.exists() {
            return Ok(file_path);
        }

        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| format!("mkdir: {}", e))?;
        }

        // Call xAI image generation API
        let body = serde_json::json!({
            "model": IMAGE_MODEL,
            "prompt": prompt,
            "n": 1,
            "response_format": "b64_json",
            "aspect_ratio": "1:1"
        });

        let response = self.client
            .post(format!("{}/images/generations", XAI_API_BASE))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("xAI image API returned {}: {}", status, text));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;

        let b64 = data.get("data")
            .and_then(|d| d.get(0))
            .and_then(|d| d.get("b64_json"))
            .and_then(|v| v.as_str())
            .ok_or("No b64_json in response")?;

        let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
            .map_err(|e| format!("Base64 decode error: {}", e))?;

        fs::write(&file_path, &bytes).await
            .map_err(|e| format!("Write error: {}", e))?;

        Ok(file_path)
    }

    /// Build a prompt for an item.
    pub fn item_prompt(name: &str, description: &str, rarity: &str, item_type: &str) -> String {
        format!(
            "{} A {} {}: {}. {}. Item icon, centered composition, single object.",
            STYLE_PREFIX, rarity, item_type, name, description
        )
    }

    /// Build a prompt for a monster.
    pub fn monster_prompt(name: &str, tier: u8, enemy_type: &str) -> String {
        format!(
            "{} Fantasy creature portrait: {}. A {} monster, tier {} power level. Menacing, battle-ready, full body visible.",
            STYLE_PREFIX, name, enemy_type, tier
        )
    }

    /// Build a prompt for an NPC.
    pub fn npc_prompt(name: &str, description: &str, disposition: &str) -> String {
        format!(
            "{} Fantasy character portrait: {}. {}. {} expression. Bust portrait, facing viewer.",
            STYLE_PREFIX, name, description, disposition
        )
    }
}

/// Normalize a name to a filesystem-safe key.
pub fn sanitize_key(name: &str) -> String {
    let s: String = name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    s.trim_matches('_').to_string()
}

/// Normalize a monster name to a cache key.
pub fn monster_key(name: &str) -> String {
    sanitize_key(name)
}

/// All known monster template names for pre-generation.
pub fn all_monster_templates() -> Vec<(String, u8, String)> {
    use crate::engine::combat::EnemyType;

    let names: [[&str; 4]; 11] = [
        ["Giant Rat", "Cave Spider", "Glow Wisp", "Shambling Corpse"],
        ["Kobold Thug", "Giant Spider", "Arcane Sprite", "Skeleton"],
        ["Goblin Warrior", "Wolf", "Fire Imp", "Zombie"],
        ["Orc Raider", "Shadow Cat", "Flame Elemental", "Ghoul"],
        ["Orc Warchief", "Werewolf", "Mind Flayer Spawn", "Wraith"],
        ["Hill Giant", "Displacer Beast", "Naga", "Vampire Spawn"],
        ["Stone Golem", "Nightwalker", "Elder Elemental", "Death Knight"],
        ["Fire Giant", "Shadow Dragon", "Beholder", "Lich"],
        ["Storm Giant", "Void Stalker", "Astral Devourer", "Demilich"],
        ["Titan Warrior", "Dread Wraith Lord", "Arch-Lich", "Dracolich"],
        ["Primordial Juggernaut", "Primordial Lurker", "Primordial Arcanum", "Primordial Undying"],
    ];
    let types = [EnemyType::Brute, EnemyType::Skulker, EnemyType::Mystic, EnemyType::Undead];

    let mut templates = Vec::new();
    for (tier, row) in names.iter().enumerate() {
        for (ti, name) in row.iter().enumerate() {
            templates.push((
                name.to_string(),
                tier as u8,
                format!("{}", types[ti]),
            ));
        }
    }
    templates
}

/// Fixed NPC templates for pre-generation: (name, description, disposition).
pub fn fixed_npc_templates() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("Marta the Innkeeper", "A stout, warm-hearted woman running the busiest tavern in the region. She has seen countless adventurers come and go.", "friendly"),
        ("Gareth the Merchant", "A weathered retired adventurer with a keen eye for rare goods. His shop is filled with curiosities from distant lands.", "friendly"),
        ("Brynn Ironhand", "A scarred dwarf blacksmith with arms like tree trunks. The ring of his hammer echoes through the village day and night.", "neutral"),
        ("Elder Miriel", "An ancient elven sage with silver hair cascading to her waist. Her eyes hold the wisdom of centuries.", "friendly"),
        ("Warchief Gromm", "A grizzled orc veteran covered in battle scars. He tests every newcomer who enters his territory.", "suspicious"),
        ("Pippa Greenleaf", "A cheerful halfling herbalist with dirt-stained hands and a warm smile. Her garden is legendary.", "friendly"),
        ("Captain of the Guard", "A stern warrior in polished armor, maintaining order with an iron hand and a watchful eye.", "neutral"),
        ("The Healer", "A robed figure with gentle hands that glow with soft light. They ask no payment, only gratitude.", "friendly"),
        ("Ranger Scout", "A lean, weathered wilderness guide with sharp eyes. They know every trail and every danger in the wild.", "neutral"),
        ("Dockmaster Kael", "A shrewd half-orc who controls the trade with ledgers and an intimidating presence.", "neutral"),
        ("Guildmaster Thorne", "A distinguished veteran with a silver beard and a commanding presence. He manages the guild hall with wisdom.", "friendly"),
        ("The Keeper", "An ageless druid wrapped in living vines, tending an ancient shrine. Their voice echoes with the forest itself.", "neutral"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_key() {
        assert_eq!(sanitize_key("Longsword"), "longsword");
        assert_eq!(sanitize_key("Giant Rat"), "giant_rat");
        assert_eq!(sanitize_key("Orc Warchief"), "orc_warchief");
        assert_eq!(sanitize_key("../../etc/passwd"), "etc_passwd");
        assert_eq!(sanitize_key("health_potion"), "health_potion");
    }

    #[test]
    fn test_monster_templates_count() {
        let templates = all_monster_templates();
        assert_eq!(templates.len(), 44);
    }

    #[test]
    fn test_fixed_npc_templates() {
        let npcs = fixed_npc_templates();
        assert_eq!(npcs.len(), 12);
        for (name, desc, disp) in &npcs {
            assert!(!name.is_empty());
            assert!(!desc.is_empty());
            assert!(!disp.is_empty());
        }
    }

    #[test]
    fn test_prompts_not_empty() {
        let p = ImageGenerator::item_prompt("Longsword", "A fine blade", "Common", "Weapon");
        assert!(p.contains("Longsword"));
        assert!(p.contains("Detailed fantasy"));

        let p = ImageGenerator::monster_prompt("Giant Rat", 0, "Brute");
        assert!(p.contains("Giant Rat"));

        let p = ImageGenerator::npc_prompt("Marta", "An innkeeper", "friendly");
        assert!(p.contains("Marta"));
    }
}
