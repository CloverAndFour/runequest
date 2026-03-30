//! Crafting graph system — materials, recipes, and balance analysis.
//!
//! Builds a directed acyclic graph (DAG) of crafting dependencies and analyzes:
//! - Total complexity to reach each tier per skill
//! - Mixing score (cross-skill dependency richness)
//! - Gateway constraint verification
//! - Combat-craft interdependency mapping
//! - Player count estimates per tier
//!
//! Usage:
//!   cargo run -- crafting --analyze
//!   cargo run -- crafting --recipe <item_id>
//!   cargo run -- crafting --tier <N>
//!   cargo run -- crafting --mixing

use std::collections::{HashMap, HashSet, BTreeMap};
use std::fmt;

use std::sync::LazyLock;
use super::inventory::{Item, ItemType};
use super::equipment::{Rarity, ItemStats};

pub static CRAFTING_GRAPH: LazyLock<CraftingGraph> = LazyLock::new(|| {
    let mut g = build_crafting_graph();
    g.analyze_usage();
    g
});

// ========================================================================
// CRAFTING SKILLS
// ========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CraftingSkill {
    Leatherworking,  // T1 gateway
    Smithing,        // T2 gateway
    Woodworking,     // T3 gateway
    Alchemy,         // T4 gateway
    Enchanting,      // T5 gateway
    Tailoring,       // T6 gateway
    Jewelcrafting,   // T7 gateway
    Runecrafting,    // T8 gateway
    Artificing,      // T9 gateway
    Theurgy,         // T10 gateway
}

impl CraftingSkill {
    pub fn all() -> &'static [CraftingSkill] {
        use CraftingSkill::*;
        &[Leatherworking, Smithing, Woodworking, Alchemy, Enchanting,
          Tailoring, Jewelcrafting, Runecrafting, Artificing, Theurgy]
    }

    pub fn gateway_tier(self) -> u8 {
        match self {
            Self::Leatherworking => 1,
            Self::Smithing => 2,
            Self::Woodworking => 3,
            Self::Alchemy => 4,
            Self::Enchanting => 5,
            Self::Tailoring => 6,
            Self::Jewelcrafting => 7,
            Self::Runecrafting => 8,
            Self::Artificing => 9,
            Self::Theurgy => 10,
        }
    }

    pub fn gateway_for_tier(tier: u8) -> Option<CraftingSkill> {
        Self::all().iter().find(|s| s.gateway_tier() == tier).copied()
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Leatherworking => "Leatherworking",
            Self::Smithing => "Smithing",
            Self::Woodworking => "Woodworking",
            Self::Alchemy => "Alchemy",
            Self::Enchanting => "Enchanting",
            Self::Tailoring => "Tailoring",
            Self::Jewelcrafting => "Jewelcrafting",
            Self::Runecrafting => "Runecrafting",
            Self::Artificing => "Artificing",
            Self::Theurgy => "Theurgy",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Leatherworking => "LW",
            Self::Smithing => "SM",
            Self::Woodworking => "WW",
            Self::Alchemy => "AL",
            Self::Enchanting => "EN",
            Self::Tailoring => "TL",
            Self::Jewelcrafting => "JC",
            Self::Runecrafting => "RC",
            Self::Artificing => "AF",
            Self::Theurgy => "TH",
        }
    }

    pub fn from_skill_id(id: &str) -> Option<CraftingSkill> {
        match id {
            "leatherworking" => Some(Self::Leatherworking),
            "smithing" => Some(Self::Smithing),
            "woodworking" => Some(Self::Woodworking),
            "alchemy" => Some(Self::Alchemy),
            "enchanting" => Some(Self::Enchanting),
            "tailoring" => Some(Self::Tailoring),
            "jewelcrafting" => Some(Self::Jewelcrafting),
            "runecrafting" => Some(Self::Runecrafting),
            "artificing" => Some(Self::Artificing),
            "theurgy" => Some(Self::Theurgy),
            _ => None,
        }
    }

    pub fn skill_id(self) -> &'static str {
        match self {
            Self::Leatherworking => "leatherworking",
            Self::Smithing => "smithing",
            Self::Woodworking => "woodworking",
            Self::Alchemy => "alchemy",
            Self::Enchanting => "enchanting",
            Self::Tailoring => "tailoring",
            Self::Jewelcrafting => "jewelcrafting",
            Self::Runecrafting => "runecrafting",
            Self::Artificing => "artificing",
            Self::Theurgy => "theurgy",
        }
    }
}

impl fmt::Display for CraftingSkill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ========================================================================
// MATERIALS
// ========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MaterialSource {
    /// Freely available in the world (gathering).
    Gathered,
    /// Dropped by monsters of a given type and minimum tier.
    MonsterDrop { monster_type: String, min_tier: u8 },
    /// Produced by a recipe.
    Crafted,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub id: String,
    pub name: String,
    pub tier: u8,
    pub source: MaterialSource,
    /// Which crafting skills use this as input (populated by analysis).
    pub used_by_skills: HashSet<CraftingSkill>,
}

// ========================================================================
// RECIPES
// ========================================================================

#[derive(Debug, Clone)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub skill: CraftingSkill,
    pub skill_rank: u8,
    pub tier: u8,
    pub inputs: Vec<(String, u32)>,   // (material_id, quantity)
    pub output: String,               // material_id
    pub output_qty: u32,
}

// ========================================================================
// CRAFTING GRAPH
// ========================================================================

#[derive(Debug, Clone)]
pub struct CraftingGraph {
    pub materials: HashMap<String, Material>,
    pub recipes: Vec<Recipe>,
}

impl CraftingGraph {
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            recipes: Vec::new(),
        }
    }

    pub fn add_material(&mut self, id: &str, name: &str, tier: u8, source: MaterialSource) {
        self.materials.insert(id.to_string(), Material {
            id: id.to_string(),
            name: name.to_string(),
            tier,
            source,
            used_by_skills: HashSet::new(),
        });
    }

    pub fn add_recipe(
        &mut self,
        id: &str, name: &str,
        skill: CraftingSkill, skill_rank: u8, tier: u8,
        inputs: &[(&str, u32)],
        output: &str, output_qty: u32,
    ) {
        // Mark the output as Crafted
        if let Some(mat) = self.materials.get_mut(output) {
            mat.source = MaterialSource::Crafted;
        }
        self.recipes.push(Recipe {
            id: id.to_string(),
            name: name.to_string(),
            skill,
            skill_rank,
            tier,
            inputs: inputs.iter().map(|(id, qty)| (id.to_string(), *qty)).collect(),
            output: output.to_string(),
            output_qty,
        });
    }

    /// Populate used_by_skills for all materials.
    pub fn analyze_usage(&mut self) {
        // Clear existing
        for mat in self.materials.values_mut() {
            mat.used_by_skills.clear();
        }
        // For each recipe, mark its inputs as used by that recipe's skill
        for recipe in &self.recipes {
            for (input_id, _) in &recipe.inputs {
                if let Some(mat) = self.materials.get_mut(input_id) {
                    mat.used_by_skills.insert(recipe.skill);
                }
            }
        }
    }

    /// Find which recipe produces a given material.
    pub fn recipe_for(&self, material_id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.output == material_id)
    }

    /// Get all recipes that use a given material as input.
    pub fn recipes_using(&self, material_id: &str) -> Vec<&Recipe> {
        self.recipes.iter()
            .filter(|r| r.inputs.iter().any(|(id, _)| id == material_id))
            .collect()
    }

    // ====================================================================
    // RECURSIVE ANALYSIS: Full dependency tree from T0 to target
    // ====================================================================

    /// Compute total production cost to make 1 unit of a material from T0.
    /// Returns (total_recipe_steps, materials_map, skills_needed, monster_kills_by_type).
    pub fn production_cost(&self, material_id: &str) -> ProductionCost {
        let mut cost = ProductionCost::default();
        let mut visited = HashSet::new();
        self.compute_cost(material_id, 1, &mut cost, &mut visited);
        cost
    }

    fn compute_cost(
        &self,
        material_id: &str,
        qty_needed: u32,
        cost: &mut ProductionCost,
        visited: &mut HashSet<String>,
    ) {
        // Avoid infinite loops
        if visited.contains(material_id) {
            return;
        }

        let mat = match self.materials.get(material_id) {
            Some(m) => m,
            None => return,
        };

        match &mat.source {
            MaterialSource::Gathered => {
                cost.gathered_materials.entry(material_id.to_string())
                    .and_modify(|q| *q += qty_needed)
                    .or_insert(qty_needed);
            }
            MaterialSource::MonsterDrop { monster_type, min_tier } => {
                cost.monster_kills.entry(format!("{}:T{}", monster_type, min_tier))
                    .and_modify(|q| *q += qty_needed)
                    .or_insert(qty_needed);
            }
            MaterialSource::Crafted => {
                if let Some(recipe) = self.recipe_for(material_id) {
                    // How many times must we run the recipe?
                    let batches = (qty_needed + recipe.output_qty - 1) / recipe.output_qty;
                    cost.recipe_steps += batches;
                    cost.skills_needed.insert(recipe.skill);

                    visited.insert(material_id.to_string());
                    // Recurse into inputs
                    for (input_id, input_qty) in &recipe.inputs {
                        self.compute_cost(input_id, input_qty * batches, cost, visited);
                    }
                    visited.remove(material_id);
                }
            }
        }
    }

    // ====================================================================
    // MIXING SCORE
    // ====================================================================

    /// Material mixing: avg number of different crafting skills that use each material.
    pub fn material_mixing_score(&self) -> f64 {
        let materials_with_uses: Vec<&Material> = self.materials.values()
            .filter(|m| !m.used_by_skills.is_empty())
            .collect();
        if materials_with_uses.is_empty() {
            return 0.0;
        }
        let total: usize = materials_with_uses.iter().map(|m| m.used_by_skills.len()).sum();
        total as f64 / materials_with_uses.len() as f64
    }

    /// Recipe mixing: avg number of different source skills required for each recipe's inputs.
    pub fn recipe_mixing_score(&self) -> f64 {
        if self.recipes.is_empty() {
            return 0.0;
        }
        let mut total_source_skills = 0.0;
        for recipe in &self.recipes {
            let mut source_skills = HashSet::new();
            for (input_id, _) in &recipe.inputs {
                if let Some(producer) = self.recipe_for(input_id) {
                    source_skills.insert(producer.skill);
                }
                // Non-crafted inputs count as "gathering" (a pseudo-skill)
                if let Some(mat) = self.materials.get(input_id) {
                    if !matches!(mat.source, MaterialSource::Crafted) {
                        source_skills.insert(CraftingSkill::Leatherworking); // placeholder for "gathering"
                    }
                }
            }
            total_source_skills += source_skills.len() as f64;
        }
        total_source_skills / self.recipes.len() as f64
    }

    /// Per-tier mixing: for each tier, how many distinct crafting skills are needed
    /// to produce ALL items at that tier.
    pub fn tier_skill_diversity(&self) -> BTreeMap<u8, usize> {
        let mut tier_skills: BTreeMap<u8, HashSet<CraftingSkill>> = BTreeMap::new();
        for recipe in &self.recipes {
            let cost = self.production_cost(&recipe.output);
            tier_skills.entry(recipe.tier)
                .or_default()
                .extend(cost.skills_needed.iter());
        }
        tier_skills.into_iter().map(|(t, s)| (t, s.len())).collect()
    }

    // ====================================================================
    // GATEWAY VERIFICATION
    // ====================================================================

    /// Verify the gateway constraint: for each tier, check that the gateway skill
    /// can reach that tier without needing any material of the same tier.
    pub fn verify_gateways(&self) -> Vec<GatewayViolation> {
        let mut violations = Vec::new();
        for tier in 1..=10u8 {
            if let Some(gateway) = CraftingSkill::gateway_for_tier(tier) {
                // Find gateway recipes at this tier
                let gateway_recipes: Vec<&Recipe> = self.recipes.iter()
                    .filter(|r| r.skill == gateway && r.tier == tier)
                    .collect();

                for recipe in gateway_recipes {
                    for (input_id, _) in &recipe.inputs {
                        if let Some(mat) = self.materials.get(input_id) {
                            if mat.tier >= tier {
                                violations.push(GatewayViolation {
                                    tier,
                                    gateway_skill: gateway,
                                    recipe_id: recipe.id.clone(),
                                    violating_input: input_id.clone(),
                                    input_tier: mat.tier,
                                });
                            }
                        }
                    }
                }
            }
        }
        violations
    }

    // ====================================================================
    // BALANCE REPORT
    // ====================================================================

    /// Compare production costs across all crafting skills at each tier.
    pub fn balance_report(&self) -> String {
        let mut out = String::new();

        out.push_str("========================================================================\n");
        out.push_str("  CRAFTING GRAPH BALANCE ANALYSIS\n");
        out.push_str("========================================================================\n\n");

        // Overall stats
        out.push_str(&format!("  Materials: {}  Recipes: {}\n",
            self.materials.len(), self.recipes.len()));
        out.push_str(&format!("  Material mixing score: {:.2} (avg skills using each material)\n",
            self.material_mixing_score()));
        out.push_str(&format!("  Recipe mixing score: {:.2} (avg source skills per recipe)\n\n",
            self.recipe_mixing_score()));

        // Gateway verification
        let violations = self.verify_gateways();
        if violations.is_empty() {
            out.push_str("  Gateway constraints: ALL VALID\n\n");
        } else {
            out.push_str(&format!("  Gateway constraints: {} VIOLATIONS!\n", violations.len()));
            for v in &violations {
                out.push_str(&format!("    T{} {}: recipe '{}' uses T{} input '{}'\n",
                    v.tier, v.gateway_skill, v.recipe_id, v.input_tier, v.violating_input));
            }
            out.push('\n');
        }

        // Tier skill diversity
        let diversity = self.tier_skill_diversity();
        out.push_str("  Tier skill diversity (crafting skills needed):\n");
        for (tier, count) in &diversity {
            let bar = "#".repeat(*count);
            out.push_str(&format!("    T{}: {:>2} skills {}\n", tier, count, bar));
        }
        out.push('\n');

        // Per-tier balance: compare production costs across skills
        out.push_str("  Per-tier production cost comparison:\n");
        out.push_str("  Tier | Skill      | Steps | Skills | Monsters | Gathered\n");
        out.push_str("  -----+------------+-------+--------+----------+---------\n");

        for tier in 1..=10u8 {
            let tier_recipes: Vec<&Recipe> = self.recipes.iter()
                .filter(|r| r.tier == tier)
                .collect();

            if tier_recipes.is_empty() {
                continue;
            }

            let mut tier_costs: Vec<(CraftingSkill, ProductionCost)> = Vec::new();
            for recipe in &tier_recipes {
                let cost = self.production_cost(&recipe.output);
                tier_costs.push((recipe.skill, cost));
            }

            for (skill, cost) in &tier_costs {
                let total_monsters: u32 = cost.monster_kills.values().sum();
                let total_gathered: u32 = cost.gathered_materials.values().sum();
                out.push_str(&format!(
                    "  T{:<3} | {:<10} | {:>5} | {:>6} | {:>8} | {:>7}\n",
                    tier, skill.short(), cost.recipe_steps,
                    cost.skills_needed.len(), total_monsters, total_gathered,
                ));
            }

            // Stats for this tier
            if tier_costs.len() > 1 {
                let avg_steps: f64 = tier_costs.iter().map(|(_, c)| c.recipe_steps as f64).sum::<f64>()
                    / tier_costs.len() as f64;
                let max_steps = tier_costs.iter().map(|(_, c)| c.recipe_steps).max().unwrap_or(0);
                let min_steps = tier_costs.iter().map(|(_, c)| c.recipe_steps).min().unwrap_or(0);
                let spread = if avg_steps > 0.0 {
                    (max_steps - min_steps) as f64 / avg_steps * 100.0
                } else { 0.0 };
                out.push_str(&format!(
                    "         avg={:.1} steps, spread={:.0}% (lower=better balanced)\n",
                    avg_steps, spread,
                ));
            }
        }

        // Monster drop mixing
        out.push_str("\n  Monster drop mixing (which skills use drops from each type):\n");
        let mut monster_skill_map: HashMap<String, HashSet<CraftingSkill>> = HashMap::new();
        for mat in self.materials.values() {
            if let MaterialSource::MonsterDrop { monster_type, .. } = &mat.source {
                for skill in &mat.used_by_skills {
                    monster_skill_map.entry(monster_type.clone())
                        .or_default()
                        .insert(*skill);
                }
            }
        }
        for (monster, skills) in &monster_skill_map {
            let skill_names: Vec<&str> = skills.iter().map(|s| s.short()).collect();
            out.push_str(&format!("    {}: {} skills ({})\n",
                monster, skills.len(), skill_names.join(", ")));
        }

        out
    }

    /// Detailed recipe lookup for a specific item.
    pub fn recipe_lookup(&self, material_id: &str) -> String {
        let mut out = String::new();

        let mat = match self.materials.get(material_id) {
            Some(m) => m,
            None => return format!("Material '{}' not found.\n", material_id),
        };

        out.push_str(&format!("=== {} (T{}) ===\n", mat.name, mat.tier));
        out.push_str(&format!("Source: {:?}\n\n", mat.source));

        if let Some(recipe) = self.recipe_for(material_id) {
            out.push_str(&format!("Recipe: {} ({})\n", recipe.name, recipe.skill));
            out.push_str(&format!("Requires {} rank {}\n", recipe.skill, recipe.skill_rank));
            out.push_str("Inputs:\n");
            for (input_id, qty) in &recipe.inputs {
                let input_name = self.materials.get(input_id)
                    .map(|m| format!("{} (T{})", m.name, m.tier))
                    .unwrap_or_else(|| input_id.clone());
                out.push_str(&format!("  {}x {}\n", qty, input_name));
            }
            out.push_str(&format!("Output: {}x {}\n\n", recipe.output_qty, mat.name));

            // Full production cost from T0
            let cost = self.production_cost(material_id);
            out.push_str("Full production cost from raw materials:\n");
            out.push_str(&format!("  Total recipe steps: {}\n", cost.recipe_steps));
            out.push_str(&format!("  Crafting skills needed: {}\n",
                cost.skills_needed.iter().map(|s| s.short()).collect::<Vec<_>>().join(", ")));
            if !cost.monster_kills.is_empty() {
                out.push_str("  Monster kills:\n");
                for (monster, qty) in &cost.monster_kills {
                    out.push_str(&format!("    {}x {}\n", qty, monster));
                }
            }
            if !cost.gathered_materials.is_empty() {
                out.push_str("  Gathered materials:\n");
                for (mat_id, qty) in &cost.gathered_materials {
                    let name = self.materials.get(mat_id)
                        .map(|m| m.name.as_str())
                        .unwrap_or(mat_id);
                    out.push_str(&format!("    {}x {}\n", qty, name));
                }
            }
        } else {
            out.push_str("(No recipe — raw material)\n");
        }

        // What can this material be used for?
        let consumers = self.recipes_using(material_id);
        if !consumers.is_empty() {
            out.push_str(&format!("\nUsed in {} recipes:\n", consumers.len()));
            for r in consumers {
                out.push_str(&format!("  {} ({}, T{})\n", r.name, r.skill, r.tier));
            }
        }

        out
    }

    /// Tier summary showing all recipes at a given tier.
    pub fn tier_report(&self, tier: u8) -> String {
        let mut out = String::new();

        out.push_str(&format!("=== Tier {} Recipes ===\n\n", tier));

        let gateway = CraftingSkill::gateway_for_tier(tier);
        if let Some(gw) = gateway {
            out.push_str(&format!("Gateway skill: {} (can reach T{} from T{} alone)\n\n",
                gw.name(), tier, tier - 1));
        }

        let tier_recipes: Vec<&Recipe> = self.recipes.iter()
            .filter(|r| r.tier == tier)
            .collect();

        if tier_recipes.is_empty() {
            out.push_str("  (no recipes defined for this tier)\n");
            return out;
        }

        for recipe in &tier_recipes {
            let is_gateway = gateway.map(|g| g == recipe.skill).unwrap_or(false);
            let marker = if is_gateway { " [GATEWAY]" } else { "" };
            out.push_str(&format!("  {} ({}){}\n", recipe.name, recipe.skill, marker));
            for (input_id, qty) in &recipe.inputs {
                let input = self.materials.get(input_id)
                    .map(|m| format!("{} (T{})", m.name, m.tier))
                    .unwrap_or_else(|| input_id.clone());
                out.push_str(&format!("    {}x {}\n", qty, input));
            }
            let cost = self.production_cost(&recipe.output);
            out.push_str(&format!("    → {} (total: {} steps, {} skills)\n\n",
                recipe.output, cost.recipe_steps, cost.skills_needed.len()));
        }

        out
    }

    // ====================================================================
    // EQUIPMENT END-TO-END ANALYSIS
    // ====================================================================

    /// All 10 equipment line names in order.
    pub fn equipment_lines() -> &'static [&'static str] {
        &["blade", "axe", "holy", "dagger", "bow", "fist", "staff", "wand", "scepter", "song"]
    }

    /// Equipment skill triplets (short codes) for each line.
    pub fn equipment_skills(line: &str) -> &'static [&'static str] {
        match line {
            "blade"   => &["SM", "LW", "EN"],
            "axe"     => &["SM", "LW", "WW"],
            "holy"    => &["SM", "RC", "TL"],
            "dagger"  => &["LW", "AL", "JC"],
            "bow"     => &["WW", "LW", "AL"],
            "fist"    => &["TL", "AL", "EN"],
            "staff"   => &["WW", "EN", "RC"],
            "wand"    => &["RC", "TL", "JC"],
            "scepter" => &["SM", "RC", "TL"],
            "song"    => &["WW", "TL", "JC"],
            _ => &[],
        }
    }

    /// Precompute all equipment costs (weapon and armor) for every line and tier.
    /// Returns a map: (line, tier) -> (weapon_cost, armor_cost).
    fn precompute_equipment_costs(&self) -> HashMap<(String, u8), (ProductionCost, ProductionCost)> {
        let mut result = HashMap::new();

        for line in Self::equipment_lines() {
            for tier in 1..=10u8 {
                let wid = format!("{}_weapon_t{}", line, tier);
                let aid = format!("{}_armor_t{}", line, tier);
                let wc = self.production_cost(&wid);
                let ac = self.production_cost(&aid);
                result.insert((line.to_string(), tier), (wc, ac));
            }
        }
        result
    }

    /// Full equipment balance report.
    pub fn equipment_report(&self) -> String {
        let mut out = String::new();

        out.push_str("========================================================================\n");
        out.push_str("  EQUIPMENT END-TO-END ANALYSIS\n");
        out.push_str("========================================================================\n\n");

        // Precompute all costs once
        let costs = self.precompute_equipment_costs();

        // Per-tier, per-line breakdown
        out.push_str("  Tier | Line    | Weapon | Armor  | Total  | Skills\n");
        out.push_str("  -----+---------+--------+--------+--------+-------\n");

        for tier in 1..=10u8 {
            let mut tier_totals: Vec<u32> = Vec::new();

            for line in Self::equipment_lines() {
                let (wc, ac) = costs.get(&(line.to_string(), tier)).unwrap();
                let total = wc.recipe_steps + ac.recipe_steps;
                let mut all_skills = wc.skills_needed.clone();
                all_skills.extend(&ac.skills_needed);
                let diversity = all_skills.len();
                tier_totals.push(total);

                out.push_str(&format!(
                    "  T{:<3} | {:<7} | {:>6} | {:>6} | {:>6} | {:>5}\n",
                    tier, line, wc.recipe_steps, ac.recipe_steps, total, diversity,
                ));
            }

            let avg = tier_totals.iter().map(|x| *x as u64).sum::<u64>() as f64 / tier_totals.len() as f64;
            let max_t = *tier_totals.iter().max().unwrap_or(&0);
            let min_t = *tier_totals.iter().min().unwrap_or(&0);
            let spread = if avg > 0.0 { (max_t - min_t) as f64 / avg * 100.0 } else { 0.0 };
            out.push_str(&format!(
                "         avg={:.1}, spread={:.0}%, min={}, max={}\n\n",
                avg, spread, min_t, max_t,
            ));
        }

        // Cross-equipment mixing (computed from precomputed costs)
        out.push_str("  CROSS-EQUIPMENT MIXING (crafting skill -> # equipment lines using it):\n");
        let mut skill_lines: HashMap<CraftingSkill, HashSet<String>> = HashMap::new();
        for line in Self::equipment_lines() {
            for tier in 1..=10u8 {
                let (wc, ac) = costs.get(&(line.to_string(), tier)).unwrap();
                for skill in wc.skills_needed.iter().chain(ac.skills_needed.iter()) {
                    skill_lines.entry(*skill).or_default().insert(line.to_string());
                }
            }
        }
        let mut mixing: Vec<(String, usize)> = skill_lines.iter()
            .map(|(skill, lines)| (skill.short().to_string(), lines.len()))
            .collect();
        mixing.sort();
        for (skill, count) in &mixing {
            let bar = "#".repeat(*count);
            out.push_str(&format!("    {:<4}: {:>2} lines {}\n", skill, count, bar));
        }
        out.push('\n');

        // Tier scaling
        out.push_str("  TIER SCALING (T(N) / T(N-1) avg cost ratio):\n");
        let mut prev_avg = 0.0f64;
        let mut scaling: Vec<(u8, f64)> = Vec::new();
        for tier in 1..=10u8 {
            let mut tier_totals: Vec<u32> = Vec::new();
            for line in Self::equipment_lines() {
                let (wc, ac) = costs.get(&(line.to_string(), tier)).unwrap();
                tier_totals.push(wc.recipe_steps + ac.recipe_steps);
            }
            let avg = tier_totals.iter().map(|x| *x as u64).sum::<u64>() as f64 / tier_totals.len() as f64;
            if tier > 1 && prev_avg > 0.0 {
                scaling.push((tier, avg / prev_avg));
            }
            prev_avg = avg;
        }
        for (tier, ratio) in &scaling {
            out.push_str(&format!("    T{}: {:.2}x\n", tier, ratio));
        }
        let avg_scale: f64 = if !scaling.is_empty() {
            scaling.iter().map(|(_, r)| r).sum::<f64>() / scaling.len() as f64
        } else { 0.0 };
        out.push_str(&format!("    avg={:.2}x\n\n", avg_scale));

        // Summary checks
        out.push_str("  BALANCE CHECKS:\n");

        // Check 1: Spread < 30% at every tier
        let mut all_tiers_ok = true;
        for tier in 1..=10u8 {
            let mut tier_totals: Vec<u32> = Vec::new();
            for line in Self::equipment_lines() {
                let (wc, ac) = costs.get(&(line.to_string(), tier)).unwrap();
                tier_totals.push(wc.recipe_steps + ac.recipe_steps);
            }
            let avg = tier_totals.iter().map(|x| *x as u64).sum::<u64>() as f64 / tier_totals.len() as f64;
            let max_t = *tier_totals.iter().max().unwrap_or(&0);
            let min_t = *tier_totals.iter().min().unwrap_or(&0);
            let spread = if avg > 0.0 { (max_t - min_t) as f64 / avg * 100.0 } else { 0.0 };
            if spread > 30.0 {
                out.push_str(&format!("    [FAIL] T{}: spread={:.0}% (>30%)\n", tier, spread));
                all_tiers_ok = false;
            }
        }
        if all_tiers_ok {
            out.push_str("    [PASS] All tiers have spread <30%\n");
        }

        // Check 2: All crafting skills feed 5+ equipment lines
        let mut all_skills_ok = true;
        for (skill, count) in &mixing {
            if *count < 5 {
                out.push_str(&format!("    [FAIL] {} feeds only {} lines (<5)\n", skill, count));
                all_skills_ok = false;
            }
        }
        if all_skills_ok {
            out.push_str("    [PASS] All crafting skills feed 5+ equipment lines\n");
        }

        // Check 3: Tier scaling consistency
        let scaling_spread = if scaling.len() > 1 {
            let max_s = scaling.iter().map(|(_, r)| *r).fold(f64::NEG_INFINITY, f64::max);
            let min_s = scaling.iter().map(|(_, r)| *r).fold(f64::INFINITY, f64::min);
            (max_s - min_s) / avg_scale * 100.0
        } else { 0.0 };
        if scaling_spread > 50.0 {
            out.push_str(&format!("    [WARN] Tier scaling spread={:.0}% (high variance)\n", scaling_spread));
        } else {
            out.push_str(&format!("    [PASS] Tier scaling consistent (spread={:.0}%)\n", scaling_spread));
        }

        // Check 4: All equipment recipes use 3+ different crafting skills
        let mut all_diverse = true;
        for line in Self::equipment_lines() {
            for tier in 1..=10u8 {
                let (wc, ac) = costs.get(&(line.to_string(), tier)).unwrap();
                let mut all_skills = wc.skills_needed.clone();
                all_skills.extend(&ac.skills_needed);
                let d = all_skills.len();
                if d < 3 {
                    out.push_str(&format!("    [FAIL] {} T{}: only {} skills (<3)\n", line, tier, d));
                    all_diverse = false;
                }
            }
        }
        if all_diverse {
            out.push_str("    [PASS] All equipment uses 3+ crafting skills\n");
        }

        out
    }
}

// ========================================================================
// ANALYSIS TYPES
// ========================================================================

#[derive(Debug, Default)]
pub struct ProductionCost {
    pub recipe_steps: u32,
    pub skills_needed: HashSet<CraftingSkill>,
    pub monster_kills: HashMap<String, u32>,
    pub gathered_materials: HashMap<String, u32>,
}

#[derive(Debug)]
pub struct GatewayViolation {
    pub tier: u8,
    pub gateway_skill: CraftingSkill,
    pub recipe_id: String,
    pub violating_input: String,
    pub input_tier: u8,
}

// ========================================================================
// GRAPH POPULATION: Define all materials and recipes
// ========================================================================

pub fn build_crafting_graph() -> CraftingGraph {
    let mut g = CraftingGraph::new();

    // ================================================================
    // T0 RAW MATERIALS (gathered or dropped by T0 monsters)
    // ================================================================
    // Gathering
    g.add_material("raw_hide_scraps", "Hide Scraps", 0, MaterialSource::Gathered);
    g.add_material("crude_thread", "Crude Thread", 0, MaterialSource::Gathered);
    g.add_material("scrap_metal", "Scrap Metal", 0, MaterialSource::Gathered);
    g.add_material("rough_stone", "Rough Stone", 0, MaterialSource::Gathered);
    g.add_material("green_wood", "Green Wood", 0, MaterialSource::Gathered);
    g.add_material("plant_fiber", "Plant Fiber", 0, MaterialSource::Gathered);
    g.add_material("wild_herbs", "Wild Herbs", 0, MaterialSource::Gathered);
    g.add_material("muddy_clay", "Muddy Clay", 0, MaterialSource::Gathered);
    g.add_material("raw_quartz", "Raw Quartz", 0, MaterialSource::Gathered);
    g.add_material("charcoal", "Charcoal", 0, MaterialSource::Gathered);

    // T0 monster drops (usable by MANY skills — high mixing)
    g.add_material("rat_hide", "Rat Hide", 0,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 0 });
    g.add_material("spider_silk_strand", "Spider Silk Strand", 0,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 0 });
    g.add_material("wisp_essence", "Wisp Essence", 0,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 0 });
    g.add_material("bone_dust", "Bone Dust", 0,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 0 });

    // ================================================================
    // TIER 1: Leatherworking is gateway
    // ================================================================

    // T1 materials (crafted)
    g.add_material("leather_strip", "Leather Strip", 1, MaterialSource::Crafted);
    g.add_material("cured_hide", "Cured Hide", 1, MaterialSource::Crafted);
    g.add_material("sinew_cord", "Sinew Cord", 1, MaterialSource::Crafted);
    g.add_material("iron_nugget", "Iron Nugget", 1, MaterialSource::Crafted);
    g.add_material("shaped_wood", "Shaped Wood", 1, MaterialSource::Crafted);
    g.add_material("herbal_paste", "Herbal Paste", 1, MaterialSource::Crafted);
    g.add_material("faint_enchant_dust", "Faint Enchanting Dust", 1, MaterialSource::Crafted);
    g.add_material("woven_cloth", "Woven Cloth", 1, MaterialSource::Crafted);
    g.add_material("polished_quartz", "Polished Quartz", 1, MaterialSource::Crafted);
    g.add_material("bone_charm", "Bone Charm", 1, MaterialSource::Crafted);

    // T1 monster drops
    g.add_material("wolf_pelt", "Wolf Pelt", 1,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 1 });
    g.add_material("venom_sac", "Venom Sac", 1,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 1 });
    g.add_material("mana_shard", "Mana Shard", 1,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 1 });
    g.add_material("ectoplasm", "Ectoplasm", 1,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 1 });

    // GATEWAY: Leatherworking T1 — uses ONLY T0 materials
    g.add_recipe("lw_t1_leather_strip", "Tan Leather Strip",
        CraftingSkill::Leatherworking, 1, 1,
        &[("rat_hide", 3), ("crude_thread", 1)],
        "leather_strip", 1);

    g.add_recipe("lw_t1_cured_hide", "Cure Hide",
        CraftingSkill::Leatherworking, 1, 1,
        &[("rat_hide", 2), ("wild_herbs", 1), ("rough_stone", 1)],
        "cured_hide", 1);

    g.add_recipe("lw_t1_sinew", "Braid Sinew Cord",
        CraftingSkill::Leatherworking, 1, 1,
        &[("rat_hide", 1), ("spider_silk_strand", 2), ("plant_fiber", 1)],
        "sinew_cord", 1);

    // Non-gateway T1: need leather_strip (T1 from LW gateway)
    g.add_recipe("sm_t1_iron_nugget", "Smelt Iron Nugget",
        CraftingSkill::Smithing, 1, 1,
        &[("scrap_metal", 3), ("charcoal", 2), ("leather_strip", 1)],
        "iron_nugget", 1);

    g.add_recipe("ww_t1_shaped_wood", "Shape Wood",
        CraftingSkill::Woodworking, 1, 1,
        &[("green_wood", 3), ("rough_stone", 1), ("sinew_cord", 1)],
        "shaped_wood", 1);

    g.add_recipe("al_t1_herbal_paste", "Brew Herbal Paste",
        CraftingSkill::Alchemy, 1, 1,
        &[("wild_herbs", 3), ("muddy_clay", 1), ("bone_dust", 1)],
        "herbal_paste", 1);

    g.add_recipe("en_t1_enchant_dust", "Distill Enchanting Dust",
        CraftingSkill::Enchanting, 1, 1,
        &[("wisp_essence", 2), ("raw_quartz", 1), ("leather_strip", 1)],
        "faint_enchant_dust", 1);

    g.add_recipe("tl_t1_woven_cloth", "Weave Cloth",
        CraftingSkill::Tailoring, 1, 1,
        &[("plant_fiber", 3), ("spider_silk_strand", 2), ("cured_hide", 1)],
        "woven_cloth", 1);

    g.add_recipe("jc_t1_polished_quartz", "Polish Quartz",
        CraftingSkill::Jewelcrafting, 1, 1,
        &[("raw_quartz", 3), ("rough_stone", 1), ("wisp_essence", 1)],
        "polished_quartz", 1);

    g.add_recipe("rc_t1_bone_charm", "Inscribe Bone Charm",
        CraftingSkill::Runecrafting, 1, 1,
        &[("bone_dust", 3), ("wisp_essence", 1), ("leather_strip", 1)],
        "bone_charm", 1);

    // ================================================================
    // TIER 2: Smithing is gateway
    // ================================================================

    g.add_material("iron_ingot", "Iron Ingot", 2, MaterialSource::Crafted);
    g.add_material("hardened_leather", "Hardened Leather", 2, MaterialSource::Crafted);
    g.add_material("ironwood_plank", "Ironwood Plank", 2, MaterialSource::Crafted);
    g.add_material("refined_potion_base", "Refined Potion Base", 2, MaterialSource::Crafted);
    g.add_material("enchanted_thread", "Enchanted Thread", 2, MaterialSource::Crafted);
    g.add_material("silk_bolt", "Silk Bolt", 2, MaterialSource::Crafted);
    g.add_material("cut_gemstone", "Cut Gemstone", 2, MaterialSource::Crafted);
    g.add_material("etched_rune", "Etched Rune", 2, MaterialSource::Crafted);

    // T2 monster drops
    g.add_material("tough_hide", "Tough Hide", 2,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 2 });
    g.add_material("shadow_thread", "Shadow Thread", 2,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 2 });
    g.add_material("arcane_crystal", "Arcane Crystal", 2,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 2 });
    g.add_material("dark_iron_ore", "Dark Iron Ore", 2,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 2 });

    // GATEWAY: Smithing T2 — uses ONLY T1 materials
    g.add_recipe("sm_t2_iron_ingot", "Forge Iron Ingot",
        CraftingSkill::Smithing, 2, 2,
        &[("iron_nugget", 3), ("shaped_wood", 1), ("sinew_cord", 1)],
        "iron_ingot", 1);

    // Non-gateway T2: need iron_ingot (T2 from SM gateway) + mixing
    g.add_recipe("lw_t2_hardened_leather", "Harden Leather",
        CraftingSkill::Leatherworking, 2, 2,
        &[("wolf_pelt", 2), ("herbal_paste", 1), ("iron_ingot", 1)],
        "hardened_leather", 1);

    g.add_recipe("ww_t2_ironwood", "Forge Ironwood Plank",
        CraftingSkill::Woodworking, 2, 2,
        &[("shaped_wood", 3), ("iron_ingot", 1), ("venom_sac", 1)],
        "ironwood_plank", 1);

    g.add_recipe("al_t2_potion_base", "Brew Refined Potion Base",
        CraftingSkill::Alchemy, 2, 2,
        &[("herbal_paste", 2), ("venom_sac", 1), ("iron_ingot", 1), ("mana_shard", 1)],
        "refined_potion_base", 1);

    g.add_recipe("en_t2_enchanted_thread", "Spin Enchanted Thread",
        CraftingSkill::Enchanting, 2, 2,
        &[("faint_enchant_dust", 2), ("mana_shard", 2), ("iron_ingot", 1)],
        "enchanted_thread", 1);

    g.add_recipe("tl_t2_silk_bolt", "Weave Silk Bolt",
        CraftingSkill::Tailoring, 2, 2,
        &[("woven_cloth", 2), ("shadow_thread", 2), ("iron_ingot", 1)],
        "silk_bolt", 1);

    g.add_recipe("jc_t2_cut_gemstone", "Cut Gemstone",
        CraftingSkill::Jewelcrafting, 2, 2,
        &[("polished_quartz", 2), ("arcane_crystal", 1), ("iron_ingot", 1)],
        "cut_gemstone", 1);

    g.add_recipe("rc_t2_etched_rune", "Etch Rune",
        CraftingSkill::Runecrafting, 2, 2,
        &[("bone_charm", 2), ("ectoplasm", 2), ("iron_ingot", 1), ("faint_enchant_dust", 1)],
        "etched_rune", 1);

    // ================================================================
    // TIER 3: Woodworking is gateway
    // ================================================================

    g.add_material("hardwood_beam", "Hardwood Beam", 3, MaterialSource::Crafted);
    g.add_material("steel_plate", "Steel Plate", 3, MaterialSource::Crafted);
    g.add_material("reinforced_leather", "Reinforced Leather", 3, MaterialSource::Crafted);
    g.add_material("alchemical_catalyst", "Alchemical Catalyst", 3, MaterialSource::Crafted);
    g.add_material("mana_weave", "Mana Weave", 3, MaterialSource::Crafted);
    g.add_material("moonsilk", "Moonsilk", 3, MaterialSource::Crafted);
    g.add_material("jeweled_setting", "Jeweled Setting", 3, MaterialSource::Crafted);
    g.add_material("power_rune", "Power Rune", 3, MaterialSource::Crafted);

    // T3 monster drops
    g.add_material("orc_tusk", "Orc Tusk", 3,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 3 });
    g.add_material("phase_silk", "Phase Silk", 3,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 3 });
    g.add_material("elemental_core", "Elemental Core", 3,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 3 });
    g.add_material("wraith_dust", "Wraith Dust", 3,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 3 });

    // GATEWAY: Woodworking T3 — uses ONLY T2 materials
    g.add_recipe("ww_t3_hardwood_beam", "Craft Hardwood Beam",
        CraftingSkill::Woodworking, 3, 3,
        &[("ironwood_plank", 3), ("hardened_leather", 1), ("enchanted_thread", 1)],
        "hardwood_beam", 1);

    // Non-gateway T3: need hardwood_beam + extensive mixing
    g.add_recipe("sm_t3_steel_plate", "Forge Steel Plate",
        CraftingSkill::Smithing, 3, 3,
        &[("iron_ingot", 3), ("hardwood_beam", 1), ("orc_tusk", 1), ("refined_potion_base", 1)],
        "steel_plate", 1);

    g.add_recipe("lw_t3_reinforced_leather", "Reinforce Leather",
        CraftingSkill::Leatherworking, 3, 3,
        &[("hardened_leather", 2), ("hardwood_beam", 1), ("phase_silk", 1), ("etched_rune", 1)],
        "reinforced_leather", 1);

    g.add_recipe("al_t3_catalyst", "Brew Alchemical Catalyst",
        CraftingSkill::Alchemy, 3, 3,
        &[("refined_potion_base", 2), ("elemental_core", 1), ("hardwood_beam", 1), ("cut_gemstone", 1)],
        "alchemical_catalyst", 1);

    g.add_recipe("en_t3_mana_weave", "Weave Mana Fabric",
        CraftingSkill::Enchanting, 3, 3,
        &[("enchanted_thread", 2), ("elemental_core", 1), ("hardwood_beam", 1), ("wraith_dust", 1)],
        "mana_weave", 1);

    g.add_recipe("tl_t3_moonsilk", "Spin Moonsilk",
        CraftingSkill::Tailoring, 3, 3,
        &[("silk_bolt", 2), ("hardwood_beam", 1), ("phase_silk", 2), ("orc_tusk", 1)],
        "moonsilk", 1);

    g.add_recipe("jc_t3_jeweled_setting", "Craft Jeweled Setting",
        CraftingSkill::Jewelcrafting, 3, 3,
        &[("cut_gemstone", 2), ("hardwood_beam", 1), ("orc_tusk", 1), ("wraith_dust", 1)],
        "jeweled_setting", 1);

    g.add_recipe("rc_t3_power_rune", "Inscribe Power Rune",
        CraftingSkill::Runecrafting, 3, 3,
        &[("etched_rune", 2), ("hardwood_beam", 1), ("wraith_dust", 1), ("orc_tusk", 1)],
        "power_rune", 1);


    // ================================================================
    // TIER 4: Alchemy is gateway
    // ================================================================

    // T4 monster drops
    g.add_material("troll_blood", "Troll Blood", 4,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 4 });
    g.add_material("phase_venom", "Phase Venom", 4,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 4 });
    g.add_material("elemental_heart", "Elemental Heart", 4,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 4 });
    g.add_material("mummy_wrappings", "Mummy Wrappings", 4,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 4 });

    // T4 crafted materials
    g.add_material("alchemical_elixir_base", "Alchemical Elixir Base", 4, MaterialSource::Crafted);
    g.add_material("alchemical_steel", "Alchemical Steel", 4, MaterialSource::Crafted);
    g.add_material("alchemical_hide", "Alchemical Hide", 4, MaterialSource::Crafted);
    g.add_material("alchemical_hardwood", "Alchemical Hardwood", 4, MaterialSource::Crafted);
    g.add_material("alchemical_weave", "Alchemical Weave", 4, MaterialSource::Crafted);
    g.add_material("alchemical_silk", "Alchemical Silk", 4, MaterialSource::Crafted);
    g.add_material("alchemical_gem", "Alchemical Gem", 4, MaterialSource::Crafted);
    g.add_material("alchemical_rune", "Alchemical Rune", 4, MaterialSource::Crafted);

    // GATEWAY: Alchemy T4 — uses ONLY T3 materials
    g.add_recipe("al_t4_elixir_base", "Brew Alchemical Elixir Base",
        CraftingSkill::Alchemy, 4, 4,
        &[("alchemical_catalyst", 3), ("mana_weave", 1), ("jeweled_setting", 1)],
        "alchemical_elixir_base", 1);

    // Non-gateway T4: use alchemical_elixir_base + own-skill T3 + monster drops
    g.add_recipe("sm_t4_alchemical_steel", "Forge Alchemical Steel",
        CraftingSkill::Smithing, 4, 4,
        &[("steel_plate", 2), ("alchemical_elixir_base", 1), ("troll_blood", 1), ("orc_tusk", 1)],
        "alchemical_steel", 1);

    g.add_recipe("lw_t4_alchemical_hide", "Treat Alchemical Hide",
        CraftingSkill::Leatherworking, 4, 4,
        &[("reinforced_leather", 2), ("alchemical_elixir_base", 1), ("troll_blood", 1), ("phase_venom", 1)],
        "alchemical_hide", 1);

    g.add_recipe("ww_t4_alchemical_hardwood", "Infuse Alchemical Hardwood",
        CraftingSkill::Woodworking, 4, 4,
        &[("hardwood_beam", 2), ("alchemical_elixir_base", 1), ("elemental_heart", 1), ("troll_blood", 1)],
        "alchemical_hardwood", 1);

    g.add_recipe("en_t4_alchemical_weave", "Enchant Alchemical Weave",
        CraftingSkill::Enchanting, 4, 4,
        &[("mana_weave", 2), ("alchemical_elixir_base", 1), ("elemental_heart", 1), ("mummy_wrappings", 1)],
        "alchemical_weave", 1);

    g.add_recipe("tl_t4_alchemical_silk", "Weave Alchemical Silk",
        CraftingSkill::Tailoring, 4, 4,
        &[("moonsilk", 2), ("alchemical_elixir_base", 1), ("phase_venom", 1), ("mummy_wrappings", 1)],
        "alchemical_silk", 1);

    g.add_recipe("jc_t4_alchemical_gem", "Cut Alchemical Gem",
        CraftingSkill::Jewelcrafting, 4, 4,
        &[("jeweled_setting", 2), ("alchemical_elixir_base", 1), ("troll_blood", 1), ("elemental_heart", 1)],
        "alchemical_gem", 1);

    g.add_recipe("rc_t4_alchemical_rune", "Inscribe Alchemical Rune",
        CraftingSkill::Runecrafting, 4, 4,
        &[("power_rune", 2), ("alchemical_elixir_base", 1), ("mummy_wrappings", 1), ("phase_venom", 1)],
        "alchemical_rune", 1);

    // ================================================================
    // TIER 5: Enchanting is gateway
    // ================================================================

    // T5 monster drops
    g.add_material("giant_sinew", "Giant Sinew", 5,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 5 });
    g.add_material("stalker_claw", "Stalker Claw", 5,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 5 });
    g.add_material("naga_pearl", "Naga Pearl", 5,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 5 });
    g.add_material("banshee_wail", "Banshee Wail", 5,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 5 });

    // T5 crafted materials
    g.add_material("enchanted_mana_crystal", "Enchanted Mana Crystal", 5, MaterialSource::Crafted);
    g.add_material("enchanted_steel", "Enchanted Steel", 5, MaterialSource::Crafted);
    g.add_material("enchanted_hide", "Enchanted Hide", 5, MaterialSource::Crafted);
    g.add_material("enchanted_hardwood", "Enchanted Hardwood", 5, MaterialSource::Crafted);
    g.add_material("enchanted_elixir", "Enchanted Elixir", 5, MaterialSource::Crafted);
    g.add_material("enchanted_silk", "Enchanted Silk", 5, MaterialSource::Crafted);
    g.add_material("enchanted_gem", "Enchanted Gem", 5, MaterialSource::Crafted);
    g.add_material("enchanted_rune", "Enchanted Rune", 5, MaterialSource::Crafted);

    // GATEWAY: Enchanting T5 — uses ONLY T4 materials
    g.add_recipe("en_t5_mana_crystal", "Crystallize Enchanted Mana",
        CraftingSkill::Enchanting, 5, 5,
        &[("alchemical_weave", 3), ("alchemical_gem", 1), ("alchemical_rune", 1)],
        "enchanted_mana_crystal", 1);

    // Non-gateway T5
    g.add_recipe("sm_t5_enchanted_steel", "Forge Enchanted Steel",
        CraftingSkill::Smithing, 5, 5,
        &[("alchemical_steel", 2), ("enchanted_mana_crystal", 1), ("giant_sinew", 1), ("troll_blood", 1)],
        "enchanted_steel", 1);

    g.add_recipe("lw_t5_enchanted_hide", "Treat Enchanted Hide",
        CraftingSkill::Leatherworking, 5, 5,
        &[("alchemical_hide", 2), ("enchanted_mana_crystal", 1), ("giant_sinew", 1), ("stalker_claw", 1)],
        "enchanted_hide", 1);

    g.add_recipe("ww_t5_enchanted_hardwood", "Shape Enchanted Hardwood",
        CraftingSkill::Woodworking, 5, 5,
        &[("alchemical_hardwood", 2), ("enchanted_mana_crystal", 1), ("giant_sinew", 1), ("naga_pearl", 1)],
        "enchanted_hardwood", 1);

    g.add_recipe("al_t5_enchanted_elixir", "Brew Enchanted Elixir",
        CraftingSkill::Alchemy, 5, 5,
        &[("alchemical_elixir_base", 2), ("enchanted_mana_crystal", 1), ("naga_pearl", 1), ("banshee_wail", 1)],
        "enchanted_elixir", 1);

    g.add_recipe("tl_t5_enchanted_silk", "Weave Enchanted Silk",
        CraftingSkill::Tailoring, 5, 5,
        &[("alchemical_silk", 2), ("enchanted_mana_crystal", 1), ("stalker_claw", 1), ("banshee_wail", 1)],
        "enchanted_silk", 1);

    g.add_recipe("jc_t5_enchanted_gem", "Facet Enchanted Gem",
        CraftingSkill::Jewelcrafting, 5, 5,
        &[("alchemical_gem", 2), ("enchanted_mana_crystal", 1), ("troll_blood", 1), ("naga_pearl", 1)],
        "enchanted_gem", 1);

    g.add_recipe("rc_t5_enchanted_rune", "Inscribe Enchanted Rune",
        CraftingSkill::Runecrafting, 5, 5,
        &[("alchemical_rune", 2), ("enchanted_mana_crystal", 1), ("banshee_wail", 1), ("giant_sinew", 1)],
        "enchanted_rune", 1);

    // ================================================================
    // TIER 6: Tailoring is gateway
    // ================================================================

    // T6 monster drops
    g.add_material("golem_core", "Golem Core", 6,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 6 });
    g.add_material("nightwalker_shade", "Nightwalker Shade", 6,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 6 });
    g.add_material("elder_crystal", "Elder Crystal", 6,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 6 });
    g.add_material("death_knight_shard", "Death Knight Shard", 6,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 6 });

    // T6 crafted materials
    g.add_material("arcane_tapestry", "Arcane Tapestry", 6, MaterialSource::Crafted);
    g.add_material("arcane_steel", "Arcane Steel", 6, MaterialSource::Crafted);
    g.add_material("arcane_hide", "Arcane Hide", 6, MaterialSource::Crafted);
    g.add_material("arcane_hardwood", "Arcane Hardwood", 6, MaterialSource::Crafted);
    g.add_material("arcane_elixir", "Arcane Elixir", 6, MaterialSource::Crafted);
    g.add_material("arcane_weave", "Arcane Weave", 6, MaterialSource::Crafted);
    g.add_material("arcane_gem", "Arcane Gem", 6, MaterialSource::Crafted);
    g.add_material("arcane_rune", "Arcane Rune", 6, MaterialSource::Crafted);

    // GATEWAY: Tailoring T6 — uses ONLY T5 materials
    g.add_recipe("tl_t6_arcane_tapestry", "Weave Arcane Tapestry",
        CraftingSkill::Tailoring, 6, 6,
        &[("enchanted_silk", 3), ("enchanted_hide", 1), ("enchanted_rune", 1)],
        "arcane_tapestry", 1);

    // Non-gateway T6
    g.add_recipe("sm_t6_arcane_steel", "Forge Arcane Steel",
        CraftingSkill::Smithing, 6, 6,
        &[("enchanted_steel", 2), ("arcane_tapestry", 1), ("golem_core", 1), ("death_knight_shard", 1)],
        "arcane_steel", 1);

    g.add_recipe("lw_t6_arcane_hide", "Treat Arcane Hide",
        CraftingSkill::Leatherworking, 6, 6,
        &[("enchanted_hide", 2), ("arcane_tapestry", 1), ("golem_core", 1), ("nightwalker_shade", 1)],
        "arcane_hide", 1);

    g.add_recipe("ww_t6_arcane_hardwood", "Shape Arcane Hardwood",
        CraftingSkill::Woodworking, 6, 6,
        &[("enchanted_hardwood", 2), ("arcane_tapestry", 1), ("golem_core", 1), ("elder_crystal", 1)],
        "arcane_hardwood", 1);

    g.add_recipe("al_t6_arcane_elixir", "Brew Arcane Elixir",
        CraftingSkill::Alchemy, 6, 6,
        &[("enchanted_elixir", 2), ("arcane_tapestry", 1), ("elder_crystal", 1), ("nightwalker_shade", 1)],
        "arcane_elixir", 1);

    g.add_recipe("en_t6_arcane_weave", "Enchant Arcane Weave",
        CraftingSkill::Enchanting, 6, 6,
        &[("enchanted_mana_crystal", 2), ("arcane_tapestry", 1), ("elder_crystal", 1), ("death_knight_shard", 1)],
        "arcane_weave", 1);

    g.add_recipe("jc_t6_arcane_gem", "Cut Arcane Gem",
        CraftingSkill::Jewelcrafting, 6, 6,
        &[("enchanted_gem", 2), ("arcane_tapestry", 1), ("golem_core", 1), ("nightwalker_shade", 1)],
        "arcane_gem", 1);

    g.add_recipe("rc_t6_arcane_rune", "Inscribe Arcane Rune",
        CraftingSkill::Runecrafting, 6, 6,
        &[("enchanted_rune", 2), ("arcane_tapestry", 1), ("death_knight_shard", 1), ("golem_core", 1)],
        "arcane_rune", 1);

    // ================================================================
    // TIER 7: Jewelcrafting is gateway
    // ================================================================

    // T7 monster drops
    g.add_material("dragon_scale", "Dragon Scale", 7,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 7 });
    g.add_material("gloom_silk", "Gloom Silk", 7,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 7 });
    g.add_material("beholder_eye", "Beholder Eye", 7,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 7 });
    g.add_material("lich_phylactery", "Lich Phylactery", 7,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 7 });

    // T7 crafted materials
    g.add_material("precious_diadem", "Precious Diadem", 7, MaterialSource::Crafted);
    g.add_material("jeweled_steel", "Jeweled Steel", 7, MaterialSource::Crafted);
    g.add_material("jeweled_hide", "Jeweled Hide", 7, MaterialSource::Crafted);
    g.add_material("jeweled_hardwood", "Jeweled Hardwood", 7, MaterialSource::Crafted);
    g.add_material("jeweled_elixir", "Jeweled Elixir", 7, MaterialSource::Crafted);
    g.add_material("jeweled_weave", "Jeweled Weave", 7, MaterialSource::Crafted);
    g.add_material("jeweled_tapestry", "Jeweled Tapestry", 7, MaterialSource::Crafted);
    g.add_material("jeweled_rune", "Jeweled Rune", 7, MaterialSource::Crafted);

    // GATEWAY: Jewelcrafting T7 — uses ONLY T6 materials
    g.add_recipe("jc_t7_precious_diadem", "Craft Precious Diadem",
        CraftingSkill::Jewelcrafting, 7, 7,
        &[("arcane_gem", 3), ("arcane_steel", 1), ("arcane_rune", 1)],
        "precious_diadem", 1);

    // Non-gateway T7
    g.add_recipe("sm_t7_jeweled_steel", "Forge Jeweled Steel",
        CraftingSkill::Smithing, 7, 7,
        &[("arcane_steel", 2), ("precious_diadem", 1), ("dragon_scale", 1), ("lich_phylactery", 1)],
        "jeweled_steel", 1);

    g.add_recipe("lw_t7_jeweled_hide", "Treat Jeweled Hide",
        CraftingSkill::Leatherworking, 7, 7,
        &[("arcane_hide", 2), ("precious_diadem", 1), ("dragon_scale", 1), ("gloom_silk", 1)],
        "jeweled_hide", 1);

    g.add_recipe("ww_t7_jeweled_hardwood", "Shape Jeweled Hardwood",
        CraftingSkill::Woodworking, 7, 7,
        &[("arcane_hardwood", 2), ("precious_diadem", 1), ("dragon_scale", 1), ("beholder_eye", 1)],
        "jeweled_hardwood", 1);

    g.add_recipe("al_t7_jeweled_elixir", "Brew Jeweled Elixir",
        CraftingSkill::Alchemy, 7, 7,
        &[("arcane_elixir", 2), ("precious_diadem", 1), ("beholder_eye", 1), ("gloom_silk", 1)],
        "jeweled_elixir", 1);

    g.add_recipe("en_t7_jeweled_weave", "Enchant Jeweled Weave",
        CraftingSkill::Enchanting, 7, 7,
        &[("arcane_weave", 2), ("precious_diadem", 1), ("beholder_eye", 1), ("lich_phylactery", 1)],
        "jeweled_weave", 1);

    g.add_recipe("tl_t7_jeweled_tapestry", "Weave Jeweled Tapestry",
        CraftingSkill::Tailoring, 7, 7,
        &[("arcane_tapestry", 2), ("precious_diadem", 1), ("gloom_silk", 1), ("dragon_scale", 1)],
        "jeweled_tapestry", 1);

    g.add_recipe("rc_t7_jeweled_rune", "Inscribe Jeweled Rune",
        CraftingSkill::Runecrafting, 7, 7,
        &[("arcane_rune", 2), ("precious_diadem", 1), ("lich_phylactery", 1), ("dragon_scale", 1)],
        "jeweled_rune", 1);

    // ================================================================
    // TIER 8: Runecrafting is gateway
    // ================================================================

    // T8 monster drops
    g.add_material("storm_essence", "Storm Essence", 8,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 8 });
    g.add_material("void_silk", "Void Silk", 8,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 8 });
    g.add_material("astral_fragment", "Astral Fragment", 8,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 8 });
    g.add_material("demilich_gem", "Demilich Gem", 8,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 8 });

    // T8 crafted materials
    g.add_material("runic_keystone", "Runic Keystone", 8, MaterialSource::Crafted);
    g.add_material("runic_steel", "Runic Steel", 8, MaterialSource::Crafted);
    g.add_material("runic_hide", "Runic Hide", 8, MaterialSource::Crafted);
    g.add_material("runic_hardwood", "Runic Hardwood", 8, MaterialSource::Crafted);
    g.add_material("runic_elixir", "Runic Elixir", 8, MaterialSource::Crafted);
    g.add_material("runic_weave", "Runic Weave", 8, MaterialSource::Crafted);
    g.add_material("runic_tapestry", "Runic Tapestry", 8, MaterialSource::Crafted);
    g.add_material("runic_gem", "Runic Gem", 8, MaterialSource::Crafted);

    // GATEWAY: Runecrafting T8 — uses ONLY T7 materials
    g.add_recipe("rc_t8_runic_keystone", "Inscribe Runic Keystone",
        CraftingSkill::Runecrafting, 8, 8,
        &[("jeweled_rune", 3), ("jeweled_weave", 1), ("jeweled_steel", 1)],
        "runic_keystone", 1);

    // Non-gateway T8
    g.add_recipe("sm_t8_runic_steel", "Forge Runic Steel",
        CraftingSkill::Smithing, 8, 8,
        &[("jeweled_steel", 2), ("runic_keystone", 1), ("storm_essence", 1), ("demilich_gem", 1)],
        "runic_steel", 1);

    g.add_recipe("lw_t8_runic_hide", "Treat Runic Hide",
        CraftingSkill::Leatherworking, 8, 8,
        &[("jeweled_hide", 2), ("runic_keystone", 1), ("storm_essence", 1), ("void_silk", 1)],
        "runic_hide", 1);

    g.add_recipe("ww_t8_runic_hardwood", "Shape Runic Hardwood",
        CraftingSkill::Woodworking, 8, 8,
        &[("jeweled_hardwood", 2), ("runic_keystone", 1), ("storm_essence", 1), ("astral_fragment", 1)],
        "runic_hardwood", 1);

    g.add_recipe("al_t8_runic_elixir", "Brew Runic Elixir",
        CraftingSkill::Alchemy, 8, 8,
        &[("jeweled_elixir", 2), ("runic_keystone", 1), ("astral_fragment", 1), ("void_silk", 1)],
        "runic_elixir", 1);

    g.add_recipe("en_t8_runic_weave", "Enchant Runic Weave",
        CraftingSkill::Enchanting, 8, 8,
        &[("jeweled_weave", 2), ("runic_keystone", 1), ("astral_fragment", 1), ("demilich_gem", 1)],
        "runic_weave", 1);

    g.add_recipe("tl_t8_runic_tapestry", "Weave Runic Tapestry",
        CraftingSkill::Tailoring, 8, 8,
        &[("jeweled_tapestry", 2), ("runic_keystone", 1), ("void_silk", 1), ("storm_essence", 1)],
        "runic_tapestry", 1);

    g.add_recipe("jc_t8_runic_gem", "Facet Runic Gem",
        CraftingSkill::Jewelcrafting, 8, 8,
        &[("precious_diadem", 2), ("runic_keystone", 1), ("demilich_gem", 1), ("storm_essence", 1)],
        "runic_gem", 1);

    // ================================================================
    // TIER 9: Artificing is gateway
    // ================================================================

    // T9 monster drops
    g.add_material("titan_bone", "Titan Bone", 9,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 9 });
    g.add_material("wraith_lord_cloak", "Wraith Lord Cloak", 9,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 9 });
    g.add_material("arch_lich_dust", "Arch-Lich Dust", 9,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 9 });
    g.add_material("dracolich_fang", "Dracolich Fang", 9,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 9 });

    // T9 crafted materials
    g.add_material("artificed_core", "Artificed Core", 9, MaterialSource::Crafted);
    g.add_material("artificed_steel", "Artificed Steel", 9, MaterialSource::Crafted);
    g.add_material("artificed_hide", "Artificed Hide", 9, MaterialSource::Crafted);
    g.add_material("artificed_hardwood", "Artificed Hardwood", 9, MaterialSource::Crafted);
    g.add_material("artificed_elixir", "Artificed Elixir", 9, MaterialSource::Crafted);
    g.add_material("artificed_weave", "Artificed Weave", 9, MaterialSource::Crafted);
    g.add_material("artificed_tapestry", "Artificed Tapestry", 9, MaterialSource::Crafted);
    g.add_material("artificed_gem", "Artificed Gem", 9, MaterialSource::Crafted);

    // GATEWAY: Artificing T9 — uses ONLY T8 materials
    g.add_recipe("af_t9_artificed_core", "Construct Artificed Core",
        CraftingSkill::Artificing, 9, 9,
        &[("runic_gem", 3), ("runic_steel", 1), ("runic_weave", 1)],
        "artificed_core", 1);

    // Non-gateway T9
    g.add_recipe("sm_t9_artificed_steel", "Forge Artificed Steel",
        CraftingSkill::Smithing, 9, 9,
        &[("runic_steel", 2), ("artificed_core", 1), ("titan_bone", 1), ("dracolich_fang", 1)],
        "artificed_steel", 1);

    g.add_recipe("lw_t9_artificed_hide", "Treat Artificed Hide",
        CraftingSkill::Leatherworking, 9, 9,
        &[("runic_hide", 2), ("artificed_core", 1), ("titan_bone", 1), ("wraith_lord_cloak", 1)],
        "artificed_hide", 1);

    g.add_recipe("ww_t9_artificed_hardwood", "Shape Artificed Hardwood",
        CraftingSkill::Woodworking, 9, 9,
        &[("runic_hardwood", 2), ("artificed_core", 1), ("titan_bone", 1), ("arch_lich_dust", 1)],
        "artificed_hardwood", 1);

    g.add_recipe("al_t9_artificed_elixir", "Brew Artificed Elixir",
        CraftingSkill::Alchemy, 9, 9,
        &[("runic_elixir", 2), ("artificed_core", 1), ("arch_lich_dust", 1), ("wraith_lord_cloak", 1)],
        "artificed_elixir", 1);

    g.add_recipe("en_t9_artificed_weave", "Enchant Artificed Weave",
        CraftingSkill::Enchanting, 9, 9,
        &[("runic_weave", 2), ("artificed_core", 1), ("arch_lich_dust", 1), ("dracolich_fang", 1)],
        "artificed_weave", 1);

    g.add_recipe("tl_t9_artificed_tapestry", "Weave Artificed Tapestry",
        CraftingSkill::Tailoring, 9, 9,
        &[("runic_tapestry", 2), ("artificed_core", 1), ("wraith_lord_cloak", 1), ("titan_bone", 1)],
        "artificed_tapestry", 1);

    g.add_recipe("jc_t9_artificed_gem", "Facet Artificed Gem",
        CraftingSkill::Jewelcrafting, 9, 9,
        &[("runic_gem", 2), ("artificed_core", 1), ("dracolich_fang", 1), ("titan_bone", 1)],
        "artificed_gem", 1);

    // ================================================================
    // TIER 10: Theurgy is gateway
    // ================================================================

    // T10 monster drops
    g.add_material("primordial_heart", "Primordial Heart", 10,
        MaterialSource::MonsterDrop { monster_type: "Brute".into(), min_tier: 10 });
    g.add_material("lurker_shadow", "Lurker Shadow", 10,
        MaterialSource::MonsterDrop { monster_type: "Skulker".into(), min_tier: 10 });
    g.add_material("arcanum_core", "Arcanum Core", 10,
        MaterialSource::MonsterDrop { monster_type: "Mystic".into(), min_tier: 10 });
    g.add_material("undying_essence", "Undying Essence", 10,
        MaterialSource::MonsterDrop { monster_type: "Undead".into(), min_tier: 10 });

    // T10 crafted materials
    g.add_material("divine_vessel", "Divine Vessel", 10, MaterialSource::Crafted);
    g.add_material("divine_steel", "Divine Steel", 10, MaterialSource::Crafted);
    g.add_material("divine_hide", "Divine Hide", 10, MaterialSource::Crafted);
    g.add_material("divine_hardwood", "Divine Hardwood", 10, MaterialSource::Crafted);
    g.add_material("divine_elixir", "Divine Elixir", 10, MaterialSource::Crafted);
    g.add_material("divine_weave", "Divine Weave", 10, MaterialSource::Crafted);
    g.add_material("divine_tapestry", "Divine Tapestry", 10, MaterialSource::Crafted);
    g.add_material("divine_gem", "Divine Gem", 10, MaterialSource::Crafted);

    // GATEWAY: Theurgy T10 — uses ONLY T9 materials
    g.add_recipe("th_t10_divine_vessel", "Consecrate Divine Vessel",
        CraftingSkill::Theurgy, 10, 10,
        &[("artificed_core", 3), ("artificed_weave", 1), ("artificed_gem", 1)],
        "divine_vessel", 1);

    // Non-gateway T10
    g.add_recipe("sm_t10_divine_steel", "Forge Divine Steel",
        CraftingSkill::Smithing, 10, 10,
        &[("artificed_steel", 2), ("divine_vessel", 1), ("primordial_heart", 1), ("undying_essence", 1)],
        "divine_steel", 1);

    g.add_recipe("lw_t10_divine_hide", "Treat Divine Hide",
        CraftingSkill::Leatherworking, 10, 10,
        &[("artificed_hide", 2), ("divine_vessel", 1), ("primordial_heart", 1), ("lurker_shadow", 1)],
        "divine_hide", 1);

    g.add_recipe("ww_t10_divine_hardwood", "Shape Divine Hardwood",
        CraftingSkill::Woodworking, 10, 10,
        &[("artificed_hardwood", 2), ("divine_vessel", 1), ("primordial_heart", 1), ("arcanum_core", 1)],
        "divine_hardwood", 1);

    g.add_recipe("al_t10_divine_elixir", "Brew Divine Elixir",
        CraftingSkill::Alchemy, 10, 10,
        &[("artificed_elixir", 2), ("divine_vessel", 1), ("arcanum_core", 1), ("lurker_shadow", 1)],
        "divine_elixir", 1);

    g.add_recipe("en_t10_divine_weave", "Enchant Divine Weave",
        CraftingSkill::Enchanting, 10, 10,
        &[("artificed_weave", 2), ("divine_vessel", 1), ("arcanum_core", 1), ("undying_essence", 1)],
        "divine_weave", 1);

    g.add_recipe("tl_t10_divine_tapestry", "Weave Divine Tapestry",
        CraftingSkill::Tailoring, 10, 10,
        &[("artificed_tapestry", 2), ("divine_vessel", 1), ("lurker_shadow", 1), ("primordial_heart", 1)],
        "divine_tapestry", 1);

    g.add_recipe("jc_t10_divine_gem", "Facet Divine Gem",
        CraftingSkill::Jewelcrafting, 10, 10,
        &[("artificed_gem", 2), ("divine_vessel", 1), ("undying_essence", 1), ("primordial_heart", 1)],
        "divine_gem", 1);


    // ================================================================
    // EQUIPMENT: End-product weapons and armor (10 lines x 10 tiers)
    // ================================================================

    // --- BLADE line: SM+LW+EN ---
    g.add_material("blade_weapon_t1", "Crude Sword", 1, MaterialSource::Crafted);
    g.add_material("blade_armor_t1", "Crude Plate", 1, MaterialSource::Crafted);
    g.add_material("blade_weapon_t2", "Iron Blade", 2, MaterialSource::Crafted);
    g.add_material("blade_armor_t2", "Iron Plate", 2, MaterialSource::Crafted);
    g.add_material("blade_weapon_t3", "Steel Longsword", 3, MaterialSource::Crafted);
    g.add_material("blade_armor_t3", "Steel Plate Armor", 3, MaterialSource::Crafted);
    g.add_material("blade_weapon_t4", "Dwarven Sword", 4, MaterialSource::Crafted);
    g.add_material("blade_armor_t4", "Dwarven Plate", 4, MaterialSource::Crafted);
    g.add_material("blade_weapon_t5", "Mithril Edge", 5, MaterialSource::Crafted);
    g.add_material("blade_armor_t5", "Mithril Plate", 5, MaterialSource::Crafted);
    g.add_material("blade_weapon_t6", "Runeblade", 6, MaterialSource::Crafted);
    g.add_material("blade_armor_t6", "Runeplate", 6, MaterialSource::Crafted);
    g.add_material("blade_weapon_t7", "Dragonsteel Sword", 7, MaterialSource::Crafted);
    g.add_material("blade_armor_t7", "Dragonsteel Plate", 7, MaterialSource::Crafted);
    g.add_material("blade_weapon_t8", "Voidforged Blade", 8, MaterialSource::Crafted);
    g.add_material("blade_armor_t8", "Voidforged Plate", 8, MaterialSource::Crafted);
    g.add_material("blade_weapon_t9", "Celestial Longsword", 9, MaterialSource::Crafted);
    g.add_material("blade_armor_t9", "Celestial Plate", 9, MaterialSource::Crafted);
    g.add_material("blade_weapon_t10", "Primordial Titansblade", 10, MaterialSource::Crafted);
    g.add_material("blade_armor_t10", "Primordial Titansguard", 10, MaterialSource::Crafted);

    g.add_recipe("eq_blade_weapon_t1", "Forge Crude Sword",
        CraftingSkill::Smithing, 1, 1,
        &[("iron_nugget", 2), ("leather_strip", 1), ("faint_enchant_dust", 1), ("venom_sac", 1)],
        "blade_weapon_t1", 1);
    g.add_recipe("eq_blade_armor_t1", "Craft Crude Plate",
        CraftingSkill::Smithing, 1, 1,
        &[("leather_strip", 2), ("iron_nugget", 1), ("faint_enchant_dust", 1), ("ectoplasm", 1)],
        "blade_armor_t1", 1);

    g.add_recipe("eq_blade_weapon_t2", "Forge Iron Blade",
        CraftingSkill::Smithing, 2, 2,
        &[("iron_ingot", 2), ("hardened_leather", 1), ("enchanted_thread", 1), ("arcane_crystal", 1)],
        "blade_weapon_t2", 1);
    g.add_recipe("eq_blade_armor_t2", "Craft Iron Plate",
        CraftingSkill::Smithing, 2, 2,
        &[("hardened_leather", 2), ("iron_ingot", 1), ("enchanted_thread", 1), ("wolf_pelt", 1)],
        "blade_armor_t2", 1);

    g.add_recipe("eq_blade_weapon_t3", "Forge Steel Longsword",
        CraftingSkill::Smithing, 3, 3,
        &[("steel_plate", 2), ("reinforced_leather", 1), ("mana_weave", 1), ("wraith_dust", 1)],
        "blade_weapon_t3", 1);
    g.add_recipe("eq_blade_armor_t3", "Craft Steel Plate Armor",
        CraftingSkill::Smithing, 3, 3,
        &[("reinforced_leather", 2), ("steel_plate", 1), ("mana_weave", 1), ("shadow_thread", 1)],
        "blade_armor_t3", 1);

    g.add_recipe("eq_blade_weapon_t4", "Forge Dwarven Sword",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_steel", 2), ("alchemical_hide", 1), ("alchemical_weave", 1), ("troll_blood", 1)],
        "blade_weapon_t4", 1);
    g.add_recipe("eq_blade_armor_t4", "Craft Dwarven Plate",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_hide", 2), ("alchemical_steel", 1), ("alchemical_weave", 1), ("elemental_core", 1)],
        "blade_armor_t4", 1);

    g.add_recipe("eq_blade_weapon_t5", "Forge Mithril Edge",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_steel", 2), ("enchanted_hide", 1), ("enchanted_mana_crystal", 1), ("stalker_claw", 1)],
        "blade_weapon_t5", 1);
    g.add_recipe("eq_blade_armor_t5", "Craft Mithril Plate",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_hide", 2), ("enchanted_steel", 1), ("enchanted_mana_crystal", 1), ("mummy_wrappings", 1)],
        "blade_armor_t5", 1);

    g.add_recipe("eq_blade_weapon_t6", "Forge Runeblade",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_steel", 2), ("arcane_hide", 1), ("arcane_weave", 1), ("elder_crystal", 1)],
        "blade_weapon_t6", 1);
    g.add_recipe("eq_blade_armor_t6", "Craft Runeplate",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_hide", 2), ("arcane_steel", 1), ("arcane_weave", 1), ("giant_sinew", 1)],
        "blade_armor_t6", 1);

    g.add_recipe("eq_blade_weapon_t7", "Forge Dragonsteel Sword",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_steel", 2), ("jeweled_hide", 1), ("jeweled_weave", 1), ("lich_phylactery", 1)],
        "blade_weapon_t7", 1);
    g.add_recipe("eq_blade_armor_t7", "Craft Dragonsteel Plate",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_hide", 2), ("jeweled_steel", 1), ("jeweled_weave", 1), ("nightwalker_shade", 1)],
        "blade_armor_t7", 1);

    g.add_recipe("eq_blade_weapon_t8", "Forge Voidforged Blade",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_steel", 2), ("runic_hide", 1), ("runic_weave", 1), ("storm_essence", 1)],
        "blade_weapon_t8", 1);
    g.add_recipe("eq_blade_armor_t8", "Craft Voidforged Plate",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_hide", 2), ("runic_steel", 1), ("runic_weave", 1), ("beholder_eye", 1)],
        "blade_armor_t8", 1);

    g.add_recipe("eq_blade_weapon_t9", "Forge Celestial Longsword",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_steel", 2), ("artificed_hide", 1), ("artificed_weave", 1), ("wraith_lord_cloak", 1)],
        "blade_weapon_t9", 1);
    g.add_recipe("eq_blade_armor_t9", "Craft Celestial Plate",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_hide", 2), ("artificed_steel", 1), ("artificed_weave", 1), ("demilich_gem", 1)],
        "blade_armor_t9", 1);

    g.add_recipe("eq_blade_weapon_t10", "Forge Primordial Titansblade",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_steel", 2), ("divine_hide", 1), ("divine_weave", 1), ("arcanum_core", 1)],
        "blade_weapon_t10", 1);
    g.add_recipe("eq_blade_armor_t10", "Craft Primordial Titansguard",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_hide", 2), ("divine_steel", 1), ("divine_weave", 1), ("titan_bone", 1)],
        "blade_armor_t10", 1);


    // --- AXE line: SM+LW+WW ---
    g.add_material("axe_weapon_t1", "Crude Hatchet", 1, MaterialSource::Crafted);
    g.add_material("axe_armor_t1", "Crude Hide Armor", 1, MaterialSource::Crafted);
    g.add_material("axe_weapon_t2", "Iron Axe", 2, MaterialSource::Crafted);
    g.add_material("axe_armor_t2", "Iron-studded Hide", 2, MaterialSource::Crafted);
    g.add_material("axe_weapon_t3", "Steel Greataxe", 3, MaterialSource::Crafted);
    g.add_material("axe_armor_t3", "Steel-braced Hide", 3, MaterialSource::Crafted);
    g.add_material("axe_weapon_t4", "Dwarven Cleaver", 4, MaterialSource::Crafted);
    g.add_material("axe_armor_t4", "Dwarven Hide", 4, MaterialSource::Crafted);
    g.add_material("axe_weapon_t5", "Mithril Axe", 5, MaterialSource::Crafted);
    g.add_material("axe_armor_t5", "Mithril Hide", 5, MaterialSource::Crafted);
    g.add_material("axe_weapon_t6", "Rune Greataxe", 6, MaterialSource::Crafted);
    g.add_material("axe_armor_t6", "Rune Hide Armor", 6, MaterialSource::Crafted);
    g.add_material("axe_weapon_t7", "Dragon Cleaver", 7, MaterialSource::Crafted);
    g.add_material("axe_armor_t7", "Dragonhide Armor", 7, MaterialSource::Crafted);
    g.add_material("axe_weapon_t8", "Voidcutter Axe", 8, MaterialSource::Crafted);
    g.add_material("axe_armor_t8", "Voidhide Armor", 8, MaterialSource::Crafted);
    g.add_material("axe_weapon_t9", "Celestial Greataxe", 9, MaterialSource::Crafted);
    g.add_material("axe_armor_t9", "Celestial Hide", 9, MaterialSource::Crafted);
    g.add_material("axe_weapon_t10", "Primordial Worldsplitter", 10, MaterialSource::Crafted);
    g.add_material("axe_armor_t10", "Primordial Beasthide", 10, MaterialSource::Crafted);

    g.add_recipe("eq_axe_weapon_t1", "Forge Crude Hatchet",
        CraftingSkill::Smithing, 1, 1,
        &[("iron_nugget", 2), ("leather_strip", 1), ("shaped_wood", 1), ("mana_shard", 1)],
        "axe_weapon_t1", 1);
    g.add_recipe("eq_axe_armor_t1", "Craft Crude Hide Armor",
        CraftingSkill::Smithing, 1, 1,
        &[("leather_strip", 2), ("iron_nugget", 1), ("shaped_wood", 1), ("wolf_pelt", 1)],
        "axe_armor_t1", 1);

    g.add_recipe("eq_axe_weapon_t2", "Forge Iron Axe",
        CraftingSkill::Smithing, 2, 2,
        &[("iron_ingot", 2), ("hardened_leather", 1), ("ironwood_plank", 1), ("dark_iron_ore", 1)],
        "axe_weapon_t2", 1);
    g.add_recipe("eq_axe_armor_t2", "Craft Iron-studded Hide",
        CraftingSkill::Smithing, 2, 2,
        &[("hardened_leather", 2), ("iron_ingot", 1), ("ironwood_plank", 1), ("venom_sac", 1)],
        "axe_armor_t2", 1);

    g.add_recipe("eq_axe_weapon_t3", "Forge Steel Greataxe",
        CraftingSkill::Smithing, 3, 3,
        &[("steel_plate", 2), ("reinforced_leather", 1), ("hardwood_beam", 1), ("orc_tusk", 1)],
        "axe_weapon_t3", 1);
    g.add_recipe("eq_axe_armor_t3", "Craft Steel-braced Hide",
        CraftingSkill::Smithing, 3, 3,
        &[("reinforced_leather", 2), ("steel_plate", 1), ("hardwood_beam", 1), ("arcane_crystal", 1)],
        "axe_armor_t3", 1);

    g.add_recipe("eq_axe_weapon_t4", "Forge Dwarven Cleaver",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_steel", 2), ("alchemical_hide", 1), ("alchemical_hardwood", 1), ("phase_venom", 1)],
        "axe_weapon_t4", 1);
    g.add_recipe("eq_axe_armor_t4", "Craft Dwarven Hide",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_hide", 2), ("alchemical_steel", 1), ("alchemical_hardwood", 1), ("wraith_dust", 1)],
        "axe_armor_t4", 1);

    g.add_recipe("eq_axe_weapon_t5", "Forge Mithril Axe",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_steel", 2), ("enchanted_hide", 1), ("enchanted_hardwood", 1), ("naga_pearl", 1)],
        "axe_weapon_t5", 1);
    g.add_recipe("eq_axe_armor_t5", "Craft Mithril Hide",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_hide", 2), ("enchanted_steel", 1), ("enchanted_hardwood", 1), ("troll_blood", 1)],
        "axe_armor_t5", 1);

    g.add_recipe("eq_axe_weapon_t6", "Forge Rune Greataxe",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_steel", 2), ("arcane_hide", 1), ("arcane_hardwood", 1), ("death_knight_shard", 1)],
        "axe_weapon_t6", 1);
    g.add_recipe("eq_axe_armor_t6", "Craft Rune Hide Armor",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_hide", 2), ("arcane_steel", 1), ("arcane_hardwood", 1), ("stalker_claw", 1)],
        "axe_armor_t6", 1);

    g.add_recipe("eq_axe_weapon_t7", "Forge Dragon Cleaver",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_steel", 2), ("jeweled_hide", 1), ("jeweled_hardwood", 1), ("dragon_scale", 1)],
        "axe_weapon_t7", 1);
    g.add_recipe("eq_axe_armor_t7", "Craft Dragonhide Armor",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_hide", 2), ("jeweled_steel", 1), ("jeweled_hardwood", 1), ("elder_crystal", 1)],
        "axe_armor_t7", 1);

    g.add_recipe("eq_axe_weapon_t8", "Forge Voidcutter Axe",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_steel", 2), ("runic_hide", 1), ("runic_hardwood", 1), ("void_silk", 1)],
        "axe_weapon_t8", 1);
    g.add_recipe("eq_axe_armor_t8", "Craft Voidhide Armor",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_hide", 2), ("runic_steel", 1), ("runic_hardwood", 1), ("lich_phylactery", 1)],
        "axe_armor_t8", 1);

    g.add_recipe("eq_axe_weapon_t9", "Forge Celestial Greataxe",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_steel", 2), ("artificed_hide", 1), ("artificed_hardwood", 1), ("arch_lich_dust", 1)],
        "axe_weapon_t9", 1);
    g.add_recipe("eq_axe_armor_t9", "Craft Celestial Hide",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_hide", 2), ("artificed_steel", 1), ("artificed_hardwood", 1), ("storm_essence", 1)],
        "axe_armor_t9", 1);

    g.add_recipe("eq_axe_weapon_t10", "Forge Primordial Worldsplitter",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_steel", 2), ("divine_hide", 1), ("divine_hardwood", 1), ("undying_essence", 1)],
        "axe_weapon_t10", 1);
    g.add_recipe("eq_axe_armor_t10", "Craft Primordial Beasthide",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_hide", 2), ("divine_steel", 1), ("divine_hardwood", 1), ("wraith_lord_cloak", 1)],
        "axe_armor_t10", 1);


    // --- HOLY line: SM+RC+TL ---
    g.add_material("holy_weapon_t1", "Crude Mace", 1, MaterialSource::Crafted);
    g.add_material("holy_armor_t1", "Crude Blessed Plate", 1, MaterialSource::Crafted);
    g.add_material("holy_weapon_t2", "Iron Mace", 2, MaterialSource::Crafted);
    g.add_material("holy_armor_t2", "Iron Blessed Plate", 2, MaterialSource::Crafted);
    g.add_material("holy_weapon_t3", "Steel Flanged Mace", 3, MaterialSource::Crafted);
    g.add_material("holy_armor_t3", "Steel Blessed Plate", 3, MaterialSource::Crafted);
    g.add_material("holy_weapon_t4", "Dwarven Warhammer", 4, MaterialSource::Crafted);
    g.add_material("holy_armor_t4", "Dwarven Blessed Plate", 4, MaterialSource::Crafted);
    g.add_material("holy_weapon_t5", "Mithril Mace", 5, MaterialSource::Crafted);
    g.add_material("holy_armor_t5", "Mithril Blessed Plate", 5, MaterialSource::Crafted);
    g.add_material("holy_weapon_t6", "Rune Mace", 6, MaterialSource::Crafted);
    g.add_material("holy_armor_t6", "Rune Blessed Plate", 6, MaterialSource::Crafted);
    g.add_material("holy_weapon_t7", "Dragonforged Mace", 7, MaterialSource::Crafted);
    g.add_material("holy_armor_t7", "Dragonforged Blessed Plate", 7, MaterialSource::Crafted);
    g.add_material("holy_weapon_t8", "Voidforged Mace", 8, MaterialSource::Crafted);
    g.add_material("holy_armor_t8", "Voidforged Blessed Plate", 8, MaterialSource::Crafted);
    g.add_material("holy_weapon_t9", "Celestial Mace", 9, MaterialSource::Crafted);
    g.add_material("holy_armor_t9", "Celestial Blessed Plate", 9, MaterialSource::Crafted);
    g.add_material("holy_weapon_t10", "Primordial Judgement", 10, MaterialSource::Crafted);
    g.add_material("holy_armor_t10", "Primordial Divineguard", 10, MaterialSource::Crafted);

    g.add_recipe("eq_holy_weapon_t1", "Forge Crude Mace",
        CraftingSkill::Smithing, 1, 1,
        &[("iron_nugget", 2), ("bone_charm", 1), ("woven_cloth", 1), ("ectoplasm", 1)],
        "holy_weapon_t1", 1);
    g.add_recipe("eq_holy_armor_t1", "Craft Crude Blessed Plate",
        CraftingSkill::Smithing, 1, 1,
        &[("bone_charm", 2), ("iron_nugget", 1), ("woven_cloth", 1), ("venom_sac", 1)],
        "holy_armor_t1", 1);

    g.add_recipe("eq_holy_weapon_t2", "Forge Iron Mace",
        CraftingSkill::Smithing, 2, 2,
        &[("iron_ingot", 2), ("etched_rune", 1), ("silk_bolt", 1), ("tough_hide", 1)],
        "holy_weapon_t2", 1);
    g.add_recipe("eq_holy_armor_t2", "Craft Iron Blessed Plate",
        CraftingSkill::Smithing, 2, 2,
        &[("etched_rune", 2), ("iron_ingot", 1), ("silk_bolt", 1), ("mana_shard", 1)],
        "holy_armor_t2", 1);

    g.add_recipe("eq_holy_weapon_t3", "Forge Steel Flanged Mace",
        CraftingSkill::Smithing, 3, 3,
        &[("steel_plate", 2), ("power_rune", 1), ("moonsilk", 1), ("phase_silk", 1)],
        "holy_weapon_t3", 1);
    g.add_recipe("eq_holy_armor_t3", "Craft Steel Blessed Plate",
        CraftingSkill::Smithing, 3, 3,
        &[("power_rune", 2), ("steel_plate", 1), ("moonsilk", 1), ("dark_iron_ore", 1)],
        "holy_armor_t3", 1);

    g.add_recipe("eq_holy_weapon_t4", "Forge Dwarven Warhammer",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_steel", 2), ("alchemical_rune", 1), ("alchemical_silk", 1), ("elemental_heart", 1)],
        "holy_weapon_t4", 1);
    g.add_recipe("eq_holy_armor_t4", "Craft Dwarven Blessed Plate",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_rune", 2), ("alchemical_steel", 1), ("alchemical_silk", 1), ("orc_tusk", 1)],
        "holy_armor_t4", 1);

    g.add_recipe("eq_holy_weapon_t5", "Forge Mithril Mace",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_steel", 2), ("enchanted_rune", 1), ("enchanted_silk", 1), ("banshee_wail", 1)],
        "holy_weapon_t5", 1);
    g.add_recipe("eq_holy_armor_t5", "Craft Mithril Blessed Plate",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_rune", 2), ("enchanted_steel", 1), ("enchanted_silk", 1), ("phase_venom", 1)],
        "holy_armor_t5", 1);

    g.add_recipe("eq_holy_weapon_t6", "Forge Rune Mace",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_steel", 2), ("arcane_rune", 1), ("arcane_tapestry", 1), ("golem_core", 1)],
        "holy_weapon_t6", 1);
    g.add_recipe("eq_holy_armor_t6", "Craft Rune Blessed Plate",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_rune", 2), ("arcane_steel", 1), ("arcane_tapestry", 1), ("naga_pearl", 1)],
        "holy_armor_t6", 1);

    g.add_recipe("eq_holy_weapon_t7", "Forge Dragonforged Mace",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_steel", 2), ("jeweled_rune", 1), ("jeweled_tapestry", 1), ("gloom_silk", 1)],
        "holy_weapon_t7", 1);
    g.add_recipe("eq_holy_armor_t7", "Craft Dragonforged Blessed Plate",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_rune", 2), ("jeweled_steel", 1), ("jeweled_tapestry", 1), ("death_knight_shard", 1)],
        "holy_armor_t7", 1);

    g.add_recipe("eq_holy_weapon_t8", "Forge Voidforged Mace",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_steel", 2), ("runic_gem", 1), ("runic_tapestry", 1), ("astral_fragment", 1)],
        "holy_weapon_t8", 1);
    g.add_recipe("eq_holy_armor_t8", "Craft Voidforged Blessed Plate",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_gem", 2), ("runic_steel", 1), ("runic_tapestry", 1), ("dragon_scale", 1)],
        "holy_armor_t8", 1);

    g.add_recipe("eq_holy_weapon_t9", "Forge Celestial Mace",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_steel", 2), ("artificed_gem", 1), ("artificed_tapestry", 1), ("dracolich_fang", 1)],
        "holy_weapon_t9", 1);
    g.add_recipe("eq_holy_armor_t9", "Craft Celestial Blessed Plate",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_gem", 2), ("artificed_steel", 1), ("artificed_tapestry", 1), ("void_silk", 1)],
        "holy_armor_t9", 1);

    g.add_recipe("eq_holy_weapon_t10", "Forge Primordial Judgement",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_steel", 2), ("divine_gem", 1), ("divine_tapestry", 1), ("primordial_heart", 1)],
        "holy_weapon_t10", 1);
    g.add_recipe("eq_holy_armor_t10", "Craft Primordial Divineguard",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_gem", 2), ("divine_steel", 1), ("divine_tapestry", 1), ("arch_lich_dust", 1)],
        "holy_armor_t10", 1);


    // --- DAGGER line: LW+AL+JC ---
    g.add_material("dagger_weapon_t1", "Crude Dagger", 1, MaterialSource::Crafted);
    g.add_material("dagger_armor_t1", "Crude Shadow Leather", 1, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t2", "Iron Dagger", 2, MaterialSource::Crafted);
    g.add_material("dagger_armor_t2", "Iron-trimmed Shadow Leather", 2, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t3", "Steel Stiletto", 3, MaterialSource::Crafted);
    g.add_material("dagger_armor_t3", "Steel-clasped Shadow Leather", 3, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t4", "Elven Dagger", 4, MaterialSource::Crafted);
    g.add_material("dagger_armor_t4", "Elven Shadow Leather", 4, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t5", "Mithril Dagger", 5, MaterialSource::Crafted);
    g.add_material("dagger_armor_t5", "Mithril Shadow Leather", 5, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t6", "Rune Dagger", 6, MaterialSource::Crafted);
    g.add_material("dagger_armor_t6", "Rune Shadow Leather", 6, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t7", "Dragon Fang Dagger", 7, MaterialSource::Crafted);
    g.add_material("dagger_armor_t7", "Dragon Shadow Leather", 7, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t8", "Voidstrike Dagger", 8, MaterialSource::Crafted);
    g.add_material("dagger_armor_t8", "Void Shadow Leather", 8, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t9", "Celestial Dagger", 9, MaterialSource::Crafted);
    g.add_material("dagger_armor_t9", "Celestial Shadow Leather", 9, MaterialSource::Crafted);
    g.add_material("dagger_weapon_t10", "Primordial Shadowfang", 10, MaterialSource::Crafted);
    g.add_material("dagger_armor_t10", "Primordial Nightveil", 10, MaterialSource::Crafted);

    g.add_recipe("eq_dagger_weapon_t1", "Forge Crude Dagger",
        CraftingSkill::Leatherworking, 1, 1,
        &[("leather_strip", 3), ("herbal_paste", 2), ("polished_quartz", 2), ("wolf_pelt", 1)],
        "dagger_weapon_t1", 1);
    g.add_recipe("eq_dagger_armor_t1", "Craft Crude Shadow Leather",
        CraftingSkill::Leatherworking, 1, 1,
        &[("herbal_paste", 3), ("leather_strip", 2), ("polished_quartz", 1), ("mana_shard", 1)],
        "dagger_armor_t1", 1);

    g.add_recipe("eq_dagger_weapon_t2", "Forge Iron Dagger",
        CraftingSkill::Leatherworking, 2, 2,
        &[("hardened_leather", 2), ("refined_potion_base", 1), ("cut_gemstone", 1), ("shadow_thread", 1)],
        "dagger_weapon_t2", 1);
    g.add_recipe("eq_dagger_armor_t2", "Craft Iron-trimmed Shadow Leather",
        CraftingSkill::Leatherworking, 2, 2,
        &[("refined_potion_base", 2), ("hardened_leather", 1), ("cut_gemstone", 1), ("ectoplasm", 1)],
        "dagger_armor_t2", 1);

    g.add_recipe("eq_dagger_weapon_t3", "Forge Steel Stiletto",
        CraftingSkill::Leatherworking, 3, 3,
        &[("reinforced_leather", 2), ("alchemical_catalyst", 1), ("jeweled_setting", 1), ("elemental_core", 1)],
        "dagger_weapon_t3", 1);
    g.add_recipe("eq_dagger_armor_t3", "Craft Steel-clasped Shadow Leather",
        CraftingSkill::Leatherworking, 3, 3,
        &[("alchemical_catalyst", 2), ("reinforced_leather", 1), ("jeweled_setting", 1), ("tough_hide", 1)],
        "dagger_armor_t3", 1);

    g.add_recipe("eq_dagger_weapon_t4", "Forge Elven Dagger",
        CraftingSkill::Leatherworking, 4, 4,
        &[("alchemical_hide", 2), ("alchemical_elixir_base", 1), ("alchemical_gem", 1), ("mummy_wrappings", 1)],
        "dagger_weapon_t4", 1);
    g.add_recipe("eq_dagger_armor_t4", "Craft Elven Shadow Leather",
        CraftingSkill::Leatherworking, 4, 4,
        &[("alchemical_elixir_base", 2), ("alchemical_hide", 1), ("alchemical_gem", 1), ("phase_silk", 1)],
        "dagger_armor_t4", 1);

    g.add_recipe("eq_dagger_weapon_t5", "Forge Mithril Dagger",
        CraftingSkill::Leatherworking, 5, 5,
        &[("enchanted_hide", 2), ("enchanted_elixir", 1), ("enchanted_gem", 1), ("giant_sinew", 1)],
        "dagger_weapon_t5", 1);
    g.add_recipe("eq_dagger_armor_t5", "Craft Mithril Shadow Leather",
        CraftingSkill::Leatherworking, 5, 5,
        &[("enchanted_elixir", 2), ("enchanted_hide", 1), ("enchanted_gem", 1), ("elemental_heart", 1)],
        "dagger_armor_t5", 1);

    g.add_recipe("eq_dagger_weapon_t6", "Forge Rune Dagger",
        CraftingSkill::Leatherworking, 6, 6,
        &[("arcane_hide", 2), ("arcane_elixir", 1), ("arcane_gem", 1), ("nightwalker_shade", 1)],
        "dagger_weapon_t6", 1);
    g.add_recipe("eq_dagger_armor_t6", "Craft Rune Shadow Leather",
        CraftingSkill::Leatherworking, 6, 6,
        &[("arcane_elixir", 2), ("arcane_hide", 1), ("arcane_gem", 1), ("banshee_wail", 1)],
        "dagger_armor_t6", 1);

    g.add_recipe("eq_dagger_weapon_t7", "Forge Dragon Fang Dagger",
        CraftingSkill::Leatherworking, 7, 7,
        &[("jeweled_hide", 2), ("jeweled_elixir", 1), ("precious_diadem", 1), ("beholder_eye", 1)],
        "dagger_weapon_t7", 1);
    g.add_recipe("eq_dagger_armor_t7", "Craft Dragon Shadow Leather",
        CraftingSkill::Leatherworking, 7, 7,
        &[("jeweled_elixir", 2), ("jeweled_hide", 1), ("precious_diadem", 1), ("golem_core", 1)],
        "dagger_armor_t7", 1);

    g.add_recipe("eq_dagger_weapon_t8", "Forge Voidstrike Dagger",
        CraftingSkill::Leatherworking, 8, 8,
        &[("runic_hide", 2), ("runic_elixir", 1), ("runic_gem", 1), ("demilich_gem", 1)],
        "dagger_weapon_t8", 1);
    g.add_recipe("eq_dagger_armor_t8", "Craft Void Shadow Leather",
        CraftingSkill::Leatherworking, 8, 8,
        &[("runic_elixir", 2), ("runic_hide", 1), ("runic_gem", 1), ("gloom_silk", 1)],
        "dagger_armor_t8", 1);

    g.add_recipe("eq_dagger_weapon_t9", "Forge Celestial Dagger",
        CraftingSkill::Leatherworking, 9, 9,
        &[("artificed_hide", 2), ("artificed_elixir", 1), ("artificed_gem", 1), ("titan_bone", 1)],
        "dagger_weapon_t9", 1);
    g.add_recipe("eq_dagger_armor_t9", "Craft Celestial Shadow Leather",
        CraftingSkill::Leatherworking, 9, 9,
        &[("artificed_elixir", 2), ("artificed_hide", 1), ("artificed_gem", 1), ("astral_fragment", 1)],
        "dagger_armor_t9", 1);

    g.add_recipe("eq_dagger_weapon_t10", "Forge Primordial Shadowfang",
        CraftingSkill::Leatherworking, 10, 10,
        &[("divine_hide", 2), ("divine_elixir", 1), ("divine_gem", 1), ("lurker_shadow", 1)],
        "dagger_weapon_t10", 1);
    g.add_recipe("eq_dagger_armor_t10", "Craft Primordial Nightveil",
        CraftingSkill::Leatherworking, 10, 10,
        &[("divine_elixir", 2), ("divine_hide", 1), ("divine_gem", 1), ("dracolich_fang", 1)],
        "dagger_armor_t10", 1);


    // --- BOW line: WW+LW+AL ---
    g.add_material("bow_weapon_t1", "Rough Bow", 1, MaterialSource::Crafted);
    g.add_material("bow_armor_t1", "Crude Ranger Leather", 1, MaterialSource::Crafted);
    g.add_material("bow_weapon_t2", "Yew Longbow", 2, MaterialSource::Crafted);
    g.add_material("bow_armor_t2", "Iron-clasped Ranger Leather", 2, MaterialSource::Crafted);
    g.add_material("bow_weapon_t3", "Steel-tipped Bow", 3, MaterialSource::Crafted);
    g.add_material("bow_armor_t3", "Steel-studded Ranger Leather", 3, MaterialSource::Crafted);
    g.add_material("bow_weapon_t4", "Elvenshade Bow", 4, MaterialSource::Crafted);
    g.add_material("bow_armor_t4", "Elven Ranger Leather", 4, MaterialSource::Crafted);
    g.add_material("bow_weapon_t5", "Mithril Bow", 5, MaterialSource::Crafted);
    g.add_material("bow_armor_t5", "Mithril Ranger Leather", 5, MaterialSource::Crafted);
    g.add_material("bow_weapon_t6", "Runewind Bow", 6, MaterialSource::Crafted);
    g.add_material("bow_armor_t6", "Rune Ranger Leather", 6, MaterialSource::Crafted);
    g.add_material("bow_weapon_t7", "Dragonwing Bow", 7, MaterialSource::Crafted);
    g.add_material("bow_armor_t7", "Dragon Ranger Leather", 7, MaterialSource::Crafted);
    g.add_material("bow_weapon_t8", "Voidhunter Bow", 8, MaterialSource::Crafted);
    g.add_material("bow_armor_t8", "Void Ranger Leather", 8, MaterialSource::Crafted);
    g.add_material("bow_weapon_t9", "Celestial Stag Bow", 9, MaterialSource::Crafted);
    g.add_material("bow_armor_t9", "Celestial Ranger Leather", 9, MaterialSource::Crafted);
    g.add_material("bow_weapon_t10", "Primordial Wilds Bow", 10, MaterialSource::Crafted);
    g.add_material("bow_armor_t10", "Primordial Wildstalker", 10, MaterialSource::Crafted);

    g.add_recipe("eq_bow_weapon_t1", "Forge Rough Bow",
        CraftingSkill::Woodworking, 1, 1,
        &[("shaped_wood", 2), ("leather_strip", 1), ("herbal_paste", 1), ("venom_sac", 1)],
        "bow_weapon_t1", 1);
    g.add_recipe("eq_bow_armor_t1", "Craft Crude Ranger Leather",
        CraftingSkill::Woodworking, 1, 1,
        &[("leather_strip", 2), ("shaped_wood", 2), ("herbal_paste", 1), ("ectoplasm", 1)],
        "bow_armor_t1", 1);

    g.add_recipe("eq_bow_weapon_t2", "Forge Yew Longbow",
        CraftingSkill::Woodworking, 2, 2,
        &[("ironwood_plank", 2), ("hardened_leather", 1), ("refined_potion_base", 1), ("arcane_crystal", 1)],
        "bow_weapon_t2", 1);
    g.add_recipe("eq_bow_armor_t2", "Craft Iron-clasped Ranger Leather",
        CraftingSkill::Woodworking, 2, 2,
        &[("hardened_leather", 2), ("ironwood_plank", 1), ("refined_potion_base", 1), ("wolf_pelt", 1)],
        "bow_armor_t2", 1);

    g.add_recipe("eq_bow_weapon_t3", "Forge Steel-tipped Bow",
        CraftingSkill::Woodworking, 3, 3,
        &[("hardwood_beam", 2), ("reinforced_leather", 1), ("alchemical_catalyst", 1), ("wraith_dust", 1)],
        "bow_weapon_t3", 1);
    g.add_recipe("eq_bow_armor_t3", "Craft Steel-studded Ranger Leather",
        CraftingSkill::Woodworking, 3, 3,
        &[("reinforced_leather", 2), ("hardwood_beam", 1), ("alchemical_catalyst", 1), ("shadow_thread", 1)],
        "bow_armor_t3", 1);

    g.add_recipe("eq_bow_weapon_t4", "Forge Elvenshade Bow",
        CraftingSkill::Woodworking, 4, 4,
        &[("alchemical_hardwood", 2), ("alchemical_hide", 1), ("alchemical_elixir_base", 1), ("troll_blood", 1)],
        "bow_weapon_t4", 1);
    g.add_recipe("eq_bow_armor_t4", "Craft Elven Ranger Leather",
        CraftingSkill::Woodworking, 4, 4,
        &[("alchemical_hide", 2), ("alchemical_hardwood", 1), ("alchemical_elixir_base", 1), ("elemental_core", 1)],
        "bow_armor_t4", 1);

    g.add_recipe("eq_bow_weapon_t5", "Forge Mithril Bow",
        CraftingSkill::Woodworking, 5, 5,
        &[("enchanted_hardwood", 2), ("enchanted_hide", 1), ("enchanted_elixir", 1), ("stalker_claw", 1)],
        "bow_weapon_t5", 1);
    g.add_recipe("eq_bow_armor_t5", "Craft Mithril Ranger Leather",
        CraftingSkill::Woodworking, 5, 5,
        &[("enchanted_hide", 2), ("enchanted_hardwood", 1), ("enchanted_elixir", 1), ("mummy_wrappings", 1)],
        "bow_armor_t5", 1);

    g.add_recipe("eq_bow_weapon_t6", "Forge Runewind Bow",
        CraftingSkill::Woodworking, 6, 6,
        &[("arcane_hardwood", 2), ("arcane_hide", 1), ("arcane_elixir", 1), ("elder_crystal", 1)],
        "bow_weapon_t6", 1);
    g.add_recipe("eq_bow_armor_t6", "Craft Rune Ranger Leather",
        CraftingSkill::Woodworking, 6, 6,
        &[("arcane_hide", 2), ("arcane_hardwood", 1), ("arcane_elixir", 1), ("giant_sinew", 1)],
        "bow_armor_t6", 1);

    g.add_recipe("eq_bow_weapon_t7", "Forge Dragonwing Bow",
        CraftingSkill::Woodworking, 7, 7,
        &[("jeweled_hardwood", 2), ("jeweled_hide", 1), ("jeweled_elixir", 1), ("lich_phylactery", 1)],
        "bow_weapon_t7", 1);
    g.add_recipe("eq_bow_armor_t7", "Craft Dragon Ranger Leather",
        CraftingSkill::Woodworking, 7, 7,
        &[("jeweled_hide", 2), ("jeweled_hardwood", 1), ("jeweled_elixir", 1), ("nightwalker_shade", 1)],
        "bow_armor_t7", 1);

    g.add_recipe("eq_bow_weapon_t8", "Forge Voidhunter Bow",
        CraftingSkill::Woodworking, 8, 8,
        &[("runic_hardwood", 2), ("runic_hide", 1), ("runic_elixir", 1), ("storm_essence", 1)],
        "bow_weapon_t8", 1);
    g.add_recipe("eq_bow_armor_t8", "Craft Void Ranger Leather",
        CraftingSkill::Woodworking, 8, 8,
        &[("runic_hide", 2), ("runic_hardwood", 1), ("runic_elixir", 1), ("beholder_eye", 1)],
        "bow_armor_t8", 1);

    g.add_recipe("eq_bow_weapon_t9", "Forge Celestial Stag Bow",
        CraftingSkill::Woodworking, 9, 9,
        &[("artificed_hardwood", 2), ("artificed_hide", 1), ("artificed_elixir", 1), ("wraith_lord_cloak", 1)],
        "bow_weapon_t9", 1);
    g.add_recipe("eq_bow_armor_t9", "Craft Celestial Ranger Leather",
        CraftingSkill::Woodworking, 9, 9,
        &[("artificed_hide", 2), ("artificed_hardwood", 1), ("artificed_elixir", 1), ("demilich_gem", 1)],
        "bow_armor_t9", 1);

    g.add_recipe("eq_bow_weapon_t10", "Forge Primordial Wilds Bow",
        CraftingSkill::Woodworking, 10, 10,
        &[("divine_hardwood", 2), ("divine_hide", 1), ("divine_elixir", 1), ("arcanum_core", 1)],
        "bow_weapon_t10", 1);
    g.add_recipe("eq_bow_armor_t10", "Craft Primordial Wildstalker",
        CraftingSkill::Woodworking, 10, 10,
        &[("divine_hide", 2), ("divine_hardwood", 1), ("divine_elixir", 1), ("titan_bone", 1)],
        "bow_armor_t10", 1);


    // --- FIST line: TL+AL+EN ---
    g.add_material("fist_weapon_t1", "Crude Wraps", 1, MaterialSource::Crafted);
    g.add_material("fist_armor_t1", "Crude Ki Robes", 1, MaterialSource::Crafted);
    g.add_material("fist_weapon_t2", "Iron-weighted Wraps", 2, MaterialSource::Crafted);
    g.add_material("fist_armor_t2", "Iron-hemmed Ki Robes", 2, MaterialSource::Crafted);
    g.add_material("fist_weapon_t3", "Steel-threaded Wraps", 3, MaterialSource::Crafted);
    g.add_material("fist_armor_t3", "Steel-clasped Ki Robes", 3, MaterialSource::Crafted);
    g.add_material("fist_weapon_t4", "Dwarven Knuckles", 4, MaterialSource::Crafted);
    g.add_material("fist_armor_t4", "Dwarven Ki Robes", 4, MaterialSource::Crafted);
    g.add_material("fist_weapon_t5", "Mithril Wraps", 5, MaterialSource::Crafted);
    g.add_material("fist_armor_t5", "Mithril Ki Robes", 5, MaterialSource::Crafted);
    g.add_material("fist_weapon_t6", "Rune Wraps", 6, MaterialSource::Crafted);
    g.add_material("fist_armor_t6", "Rune Ki Robes", 6, MaterialSource::Crafted);
    g.add_material("fist_weapon_t7", "Dragon Fist Wraps", 7, MaterialSource::Crafted);
    g.add_material("fist_armor_t7", "Dragon Ki Robes", 7, MaterialSource::Crafted);
    g.add_material("fist_weapon_t8", "Voidstrike Wraps", 8, MaterialSource::Crafted);
    g.add_material("fist_armor_t8", "Void Ki Robes", 8, MaterialSource::Crafted);
    g.add_material("fist_weapon_t9", "Celestial Wraps", 9, MaterialSource::Crafted);
    g.add_material("fist_armor_t9", "Celestial Ki Robes", 9, MaterialSource::Crafted);
    g.add_material("fist_weapon_t10", "Primordial Titanfist", 10, MaterialSource::Crafted);
    g.add_material("fist_armor_t10", "Primordial Ascension Robes", 10, MaterialSource::Crafted);

    g.add_recipe("eq_fist_weapon_t1", "Forge Crude Wraps",
        CraftingSkill::Tailoring, 1, 1,
        &[("woven_cloth", 2), ("herbal_paste", 1), ("faint_enchant_dust", 1), ("mana_shard", 1)],
        "fist_weapon_t1", 1);
    g.add_recipe("eq_fist_armor_t1", "Craft Crude Ki Robes",
        CraftingSkill::Tailoring, 1, 1,
        &[("herbal_paste", 2), ("woven_cloth", 1), ("faint_enchant_dust", 1), ("wolf_pelt", 1)],
        "fist_armor_t1", 1);

    g.add_recipe("eq_fist_weapon_t2", "Forge Iron-weighted Wraps",
        CraftingSkill::Tailoring, 2, 2,
        &[("silk_bolt", 2), ("refined_potion_base", 1), ("enchanted_thread", 1), ("dark_iron_ore", 1)],
        "fist_weapon_t2", 1);
    g.add_recipe("eq_fist_armor_t2", "Craft Iron-hemmed Ki Robes",
        CraftingSkill::Tailoring, 2, 2,
        &[("refined_potion_base", 2), ("silk_bolt", 1), ("enchanted_thread", 1), ("venom_sac", 1)],
        "fist_armor_t2", 1);

    g.add_recipe("eq_fist_weapon_t3", "Forge Steel-threaded Wraps",
        CraftingSkill::Tailoring, 3, 3,
        &[("moonsilk", 2), ("alchemical_catalyst", 1), ("mana_weave", 1), ("orc_tusk", 1)],
        "fist_weapon_t3", 1);
    g.add_recipe("eq_fist_armor_t3", "Craft Steel-clasped Ki Robes",
        CraftingSkill::Tailoring, 3, 3,
        &[("alchemical_catalyst", 2), ("moonsilk", 1), ("mana_weave", 1), ("arcane_crystal", 1)],
        "fist_armor_t3", 1);

    g.add_recipe("eq_fist_weapon_t4", "Forge Dwarven Knuckles",
        CraftingSkill::Tailoring, 4, 4,
        &[("alchemical_silk", 2), ("alchemical_elixir_base", 1), ("alchemical_weave", 1), ("phase_venom", 1)],
        "fist_weapon_t4", 1);
    g.add_recipe("eq_fist_armor_t4", "Craft Dwarven Ki Robes",
        CraftingSkill::Tailoring, 4, 4,
        &[("alchemical_elixir_base", 2), ("alchemical_silk", 1), ("alchemical_weave", 1), ("wraith_dust", 1)],
        "fist_armor_t4", 1);

    g.add_recipe("eq_fist_weapon_t5", "Forge Mithril Wraps",
        CraftingSkill::Tailoring, 5, 5,
        &[("enchanted_silk", 2), ("enchanted_elixir", 1), ("enchanted_mana_crystal", 1), ("naga_pearl", 1)],
        "fist_weapon_t5", 1);
    g.add_recipe("eq_fist_armor_t5", "Craft Mithril Ki Robes",
        CraftingSkill::Tailoring, 5, 5,
        &[("enchanted_elixir", 2), ("enchanted_silk", 1), ("enchanted_mana_crystal", 1), ("troll_blood", 1)],
        "fist_armor_t5", 1);

    g.add_recipe("eq_fist_weapon_t6", "Forge Rune Wraps",
        CraftingSkill::Tailoring, 6, 6,
        &[("arcane_tapestry", 2), ("arcane_elixir", 1), ("arcane_weave", 1), ("death_knight_shard", 1)],
        "fist_weapon_t6", 1);
    g.add_recipe("eq_fist_armor_t6", "Craft Rune Ki Robes",
        CraftingSkill::Tailoring, 6, 6,
        &[("arcane_elixir", 2), ("arcane_tapestry", 1), ("arcane_weave", 1), ("stalker_claw", 1)],
        "fist_armor_t6", 1);

    g.add_recipe("eq_fist_weapon_t7", "Forge Dragon Fist Wraps",
        CraftingSkill::Tailoring, 7, 7,
        &[("jeweled_tapestry", 2), ("jeweled_elixir", 1), ("jeweled_weave", 1), ("dragon_scale", 1)],
        "fist_weapon_t7", 1);
    g.add_recipe("eq_fist_armor_t7", "Craft Dragon Ki Robes",
        CraftingSkill::Tailoring, 7, 7,
        &[("jeweled_elixir", 2), ("jeweled_tapestry", 1), ("jeweled_weave", 1), ("elder_crystal", 1)],
        "fist_armor_t7", 1);

    g.add_recipe("eq_fist_weapon_t8", "Forge Voidstrike Wraps",
        CraftingSkill::Tailoring, 8, 8,
        &[("runic_tapestry", 2), ("runic_elixir", 1), ("runic_weave", 1), ("void_silk", 1)],
        "fist_weapon_t8", 1);
    g.add_recipe("eq_fist_armor_t8", "Craft Void Ki Robes",
        CraftingSkill::Tailoring, 8, 8,
        &[("runic_elixir", 2), ("runic_tapestry", 1), ("runic_weave", 1), ("lich_phylactery", 1)],
        "fist_armor_t8", 1);

    g.add_recipe("eq_fist_weapon_t9", "Forge Celestial Wraps",
        CraftingSkill::Tailoring, 9, 9,
        &[("artificed_tapestry", 2), ("artificed_elixir", 1), ("artificed_weave", 1), ("arch_lich_dust", 1)],
        "fist_weapon_t9", 1);
    g.add_recipe("eq_fist_armor_t9", "Craft Celestial Ki Robes",
        CraftingSkill::Tailoring, 9, 9,
        &[("artificed_elixir", 2), ("artificed_tapestry", 1), ("artificed_weave", 1), ("storm_essence", 1)],
        "fist_armor_t9", 1);

    g.add_recipe("eq_fist_weapon_t10", "Forge Primordial Titanfist",
        CraftingSkill::Tailoring, 10, 10,
        &[("divine_tapestry", 2), ("divine_elixir", 1), ("divine_weave", 1), ("undying_essence", 1)],
        "fist_weapon_t10", 1);
    g.add_recipe("eq_fist_armor_t10", "Craft Primordial Ascension Robes",
        CraftingSkill::Tailoring, 10, 10,
        &[("divine_elixir", 2), ("divine_tapestry", 1), ("divine_weave", 1), ("wraith_lord_cloak", 1)],
        "fist_armor_t10", 1);


    // --- STAFF line: WW+EN+RC ---
    g.add_material("staff_weapon_t1", "Crude Staff", 1, MaterialSource::Crafted);
    g.add_material("staff_armor_t1", "Crude Mage Robes", 1, MaterialSource::Crafted);
    g.add_material("staff_weapon_t2", "Ironshod Staff", 2, MaterialSource::Crafted);
    g.add_material("staff_armor_t2", "Iron-clasped Mage Robes", 2, MaterialSource::Crafted);
    g.add_material("staff_weapon_t3", "Steel-capped Staff", 3, MaterialSource::Crafted);
    g.add_material("staff_armor_t3", "Steel-trimmed Mage Robes", 3, MaterialSource::Crafted);
    g.add_material("staff_weapon_t4", "Dwarven Arcane Staff", 4, MaterialSource::Crafted);
    g.add_material("staff_armor_t4", "Dwarven Mage Robes", 4, MaterialSource::Crafted);
    g.add_material("staff_weapon_t5", "Mithril Staff", 5, MaterialSource::Crafted);
    g.add_material("staff_armor_t5", "Mithril Mage Robes", 5, MaterialSource::Crafted);
    g.add_material("staff_weapon_t6", "Rune Staff", 6, MaterialSource::Crafted);
    g.add_material("staff_armor_t6", "Rune Mage Robes", 6, MaterialSource::Crafted);
    g.add_material("staff_weapon_t7", "Dragonwood Staff", 7, MaterialSource::Crafted);
    g.add_material("staff_armor_t7", "Dragon Mage Robes", 7, MaterialSource::Crafted);
    g.add_material("staff_weapon_t8", "Voidtouched Staff", 8, MaterialSource::Crafted);
    g.add_material("staff_armor_t8", "Void Mage Robes", 8, MaterialSource::Crafted);
    g.add_material("staff_weapon_t9", "Celestial Staff", 9, MaterialSource::Crafted);
    g.add_material("staff_armor_t9", "Celestial Mage Robes", 9, MaterialSource::Crafted);
    g.add_material("staff_weapon_t10", "Primordial Worldstaff", 10, MaterialSource::Crafted);
    g.add_material("staff_armor_t10", "Primordial Arcanum Robes", 10, MaterialSource::Crafted);

    g.add_recipe("eq_staff_weapon_t1", "Forge Crude Staff",
        CraftingSkill::Woodworking, 1, 1,
        &[("shaped_wood", 2), ("faint_enchant_dust", 1), ("bone_charm", 1), ("ectoplasm", 1)],
        "staff_weapon_t1", 1);
    g.add_recipe("eq_staff_armor_t1", "Craft Crude Mage Robes",
        CraftingSkill::Woodworking, 1, 1,
        &[("faint_enchant_dust", 2), ("shaped_wood", 1), ("bone_charm", 1), ("venom_sac", 1)],
        "staff_armor_t1", 1);

    g.add_recipe("eq_staff_weapon_t2", "Forge Ironshod Staff",
        CraftingSkill::Woodworking, 2, 2,
        &[("ironwood_plank", 2), ("enchanted_thread", 1), ("etched_rune", 1), ("tough_hide", 1)],
        "staff_weapon_t2", 1);
    g.add_recipe("eq_staff_armor_t2", "Craft Iron-clasped Mage Robes",
        CraftingSkill::Woodworking, 2, 2,
        &[("enchanted_thread", 2), ("ironwood_plank", 1), ("etched_rune", 1), ("mana_shard", 1)],
        "staff_armor_t2", 1);

    g.add_recipe("eq_staff_weapon_t3", "Forge Steel-capped Staff",
        CraftingSkill::Woodworking, 3, 3,
        &[("hardwood_beam", 2), ("mana_weave", 1), ("power_rune", 1), ("phase_silk", 1)],
        "staff_weapon_t3", 1);
    g.add_recipe("eq_staff_armor_t3", "Craft Steel-trimmed Mage Robes",
        CraftingSkill::Woodworking, 3, 3,
        &[("mana_weave", 2), ("hardwood_beam", 1), ("power_rune", 1), ("dark_iron_ore", 1)],
        "staff_armor_t3", 1);

    g.add_recipe("eq_staff_weapon_t4", "Forge Dwarven Arcane Staff",
        CraftingSkill::Woodworking, 4, 4,
        &[("alchemical_hardwood", 2), ("alchemical_weave", 1), ("alchemical_rune", 1), ("elemental_heart", 1)],
        "staff_weapon_t4", 1);
    g.add_recipe("eq_staff_armor_t4", "Craft Dwarven Mage Robes",
        CraftingSkill::Woodworking, 4, 4,
        &[("alchemical_weave", 2), ("alchemical_hardwood", 1), ("alchemical_rune", 1), ("orc_tusk", 1)],
        "staff_armor_t4", 1);

    g.add_recipe("eq_staff_weapon_t5", "Forge Mithril Staff",
        CraftingSkill::Woodworking, 5, 5,
        &[("enchanted_hardwood", 2), ("enchanted_mana_crystal", 1), ("enchanted_rune", 1), ("banshee_wail", 1)],
        "staff_weapon_t5", 1);
    g.add_recipe("eq_staff_armor_t5", "Craft Mithril Mage Robes",
        CraftingSkill::Woodworking, 5, 5,
        &[("enchanted_mana_crystal", 2), ("enchanted_hardwood", 1), ("enchanted_rune", 1), ("phase_venom", 1)],
        "staff_armor_t5", 1);

    g.add_recipe("eq_staff_weapon_t6", "Forge Rune Staff",
        CraftingSkill::Woodworking, 6, 6,
        &[("arcane_hardwood", 2), ("arcane_weave", 1), ("arcane_rune", 1), ("golem_core", 1)],
        "staff_weapon_t6", 1);
    g.add_recipe("eq_staff_armor_t6", "Craft Rune Mage Robes",
        CraftingSkill::Woodworking, 6, 6,
        &[("arcane_weave", 2), ("arcane_hardwood", 1), ("arcane_rune", 1), ("naga_pearl", 1)],
        "staff_armor_t6", 1);

    g.add_recipe("eq_staff_weapon_t7", "Forge Dragonwood Staff",
        CraftingSkill::Woodworking, 7, 7,
        &[("jeweled_hardwood", 2), ("jeweled_weave", 1), ("jeweled_rune", 1), ("gloom_silk", 1)],
        "staff_weapon_t7", 1);
    g.add_recipe("eq_staff_armor_t7", "Craft Dragon Mage Robes",
        CraftingSkill::Woodworking, 7, 7,
        &[("jeweled_weave", 2), ("jeweled_hardwood", 1), ("jeweled_rune", 1), ("death_knight_shard", 1)],
        "staff_armor_t7", 1);

    g.add_recipe("eq_staff_weapon_t8", "Forge Voidtouched Staff",
        CraftingSkill::Woodworking, 8, 8,
        &[("runic_hardwood", 2), ("runic_weave", 1), ("runic_hide", 1), ("astral_fragment", 1)],
        "staff_weapon_t8", 1);
    g.add_recipe("eq_staff_armor_t8", "Craft Void Mage Robes",
        CraftingSkill::Woodworking, 8, 8,
        &[("runic_weave", 2), ("runic_hardwood", 1), ("runic_hide", 1), ("dragon_scale", 1)],
        "staff_armor_t8", 1);

    g.add_recipe("eq_staff_weapon_t9", "Forge Celestial Staff",
        CraftingSkill::Woodworking, 9, 9,
        &[("artificed_hardwood", 2), ("artificed_weave", 1), ("artificed_hide", 1), ("dracolich_fang", 1)],
        "staff_weapon_t9", 1);
    g.add_recipe("eq_staff_armor_t9", "Craft Celestial Mage Robes",
        CraftingSkill::Woodworking, 9, 9,
        &[("artificed_weave", 2), ("artificed_hardwood", 1), ("artificed_hide", 1), ("void_silk", 1)],
        "staff_armor_t9", 1);

    g.add_recipe("eq_staff_weapon_t10", "Forge Primordial Worldstaff",
        CraftingSkill::Woodworking, 10, 10,
        &[("divine_hardwood", 2), ("divine_weave", 1), ("divine_hide", 1), ("primordial_heart", 1)],
        "staff_weapon_t10", 1);
    g.add_recipe("eq_staff_armor_t10", "Craft Primordial Arcanum Robes",
        CraftingSkill::Woodworking, 10, 10,
        &[("divine_weave", 2), ("divine_hardwood", 1), ("divine_hide", 1), ("arch_lich_dust", 1)],
        "staff_armor_t10", 1);


    // --- WAND line: RC+TL+JC ---
    g.add_material("wand_weapon_t1", "Crude Wand", 1, MaterialSource::Crafted);
    g.add_material("wand_armor_t1", "Crude Dark Vestments", 1, MaterialSource::Crafted);
    g.add_material("wand_weapon_t2", "Iron-tipped Wand", 2, MaterialSource::Crafted);
    g.add_material("wand_armor_t2", "Iron-clasped Vestments", 2, MaterialSource::Crafted);
    g.add_material("wand_weapon_t3", "Steel-cored Wand", 3, MaterialSource::Crafted);
    g.add_material("wand_armor_t3", "Steel-trimmed Vestments", 3, MaterialSource::Crafted);
    g.add_material("wand_weapon_t4", "Dwarven Eldritch Wand", 4, MaterialSource::Crafted);
    g.add_material("wand_armor_t4", "Dwarven Dark Vestments", 4, MaterialSource::Crafted);
    g.add_material("wand_weapon_t5", "Mithril Wand", 5, MaterialSource::Crafted);
    g.add_material("wand_armor_t5", "Mithril Dark Vestments", 5, MaterialSource::Crafted);
    g.add_material("wand_weapon_t6", "Rune Wand", 6, MaterialSource::Crafted);
    g.add_material("wand_armor_t6", "Rune Dark Vestments", 6, MaterialSource::Crafted);
    g.add_material("wand_weapon_t7", "Dragonbone Wand", 7, MaterialSource::Crafted);
    g.add_material("wand_armor_t7", "Dragon Dark Vestments", 7, MaterialSource::Crafted);
    g.add_material("wand_weapon_t8", "Voidchannel Wand", 8, MaterialSource::Crafted);
    g.add_material("wand_armor_t8", "Void Dark Vestments", 8, MaterialSource::Crafted);
    g.add_material("wand_weapon_t9", "Celestial Wand", 9, MaterialSource::Crafted);
    g.add_material("wand_armor_t9", "Celestial Vestments", 9, MaterialSource::Crafted);
    g.add_material("wand_weapon_t10", "Primordial Dominus Wand", 10, MaterialSource::Crafted);
    g.add_material("wand_armor_t10", "Primordial Shadow Vestments", 10, MaterialSource::Crafted);

    g.add_recipe("eq_wand_weapon_t1", "Forge Crude Wand",
        CraftingSkill::Runecrafting, 1, 1,
        &[("bone_charm", 2), ("woven_cloth", 1), ("polished_quartz", 1), ("wolf_pelt", 1)],
        "wand_weapon_t1", 1);
    g.add_recipe("eq_wand_armor_t1", "Craft Crude Dark Vestments",
        CraftingSkill::Runecrafting, 1, 1,
        &[("woven_cloth", 2), ("bone_charm", 1), ("polished_quartz", 1), ("mana_shard", 1)],
        "wand_armor_t1", 1);

    g.add_recipe("eq_wand_weapon_t2", "Forge Iron-tipped Wand",
        CraftingSkill::Runecrafting, 2, 2,
        &[("etched_rune", 2), ("silk_bolt", 1), ("cut_gemstone", 1), ("shadow_thread", 1)],
        "wand_weapon_t2", 1);
    g.add_recipe("eq_wand_armor_t2", "Craft Iron-clasped Vestments",
        CraftingSkill::Runecrafting, 2, 2,
        &[("silk_bolt", 2), ("etched_rune", 1), ("cut_gemstone", 1), ("ectoplasm", 1)],
        "wand_armor_t2", 1);

    g.add_recipe("eq_wand_weapon_t3", "Forge Steel-cored Wand",
        CraftingSkill::Runecrafting, 3, 3,
        &[("power_rune", 2), ("moonsilk", 1), ("jeweled_setting", 1), ("elemental_core", 1)],
        "wand_weapon_t3", 1);
    g.add_recipe("eq_wand_armor_t3", "Craft Steel-trimmed Vestments",
        CraftingSkill::Runecrafting, 3, 3,
        &[("moonsilk", 2), ("power_rune", 1), ("jeweled_setting", 1), ("tough_hide", 1)],
        "wand_armor_t3", 1);

    g.add_recipe("eq_wand_weapon_t4", "Forge Dwarven Eldritch Wand",
        CraftingSkill::Runecrafting, 4, 4,
        &[("alchemical_rune", 2), ("alchemical_silk", 1), ("alchemical_gem", 1), ("mummy_wrappings", 1)],
        "wand_weapon_t4", 1);
    g.add_recipe("eq_wand_armor_t4", "Craft Dwarven Dark Vestments",
        CraftingSkill::Runecrafting, 4, 4,
        &[("alchemical_silk", 2), ("alchemical_rune", 1), ("alchemical_gem", 1), ("phase_silk", 1)],
        "wand_armor_t4", 1);

    g.add_recipe("eq_wand_weapon_t5", "Forge Mithril Wand",
        CraftingSkill::Runecrafting, 5, 5,
        &[("enchanted_rune", 2), ("enchanted_silk", 1), ("enchanted_gem", 1), ("giant_sinew", 1)],
        "wand_weapon_t5", 1);
    g.add_recipe("eq_wand_armor_t5", "Craft Mithril Dark Vestments",
        CraftingSkill::Runecrafting, 5, 5,
        &[("enchanted_silk", 2), ("enchanted_rune", 1), ("enchanted_gem", 1), ("elemental_heart", 1)],
        "wand_armor_t5", 1);

    g.add_recipe("eq_wand_weapon_t6", "Forge Rune Wand",
        CraftingSkill::Runecrafting, 6, 6,
        &[("arcane_rune", 2), ("arcane_tapestry", 1), ("arcane_gem", 1), ("nightwalker_shade", 1)],
        "wand_weapon_t6", 1);
    g.add_recipe("eq_wand_armor_t6", "Craft Rune Dark Vestments",
        CraftingSkill::Runecrafting, 6, 6,
        &[("arcane_tapestry", 2), ("arcane_rune", 1), ("arcane_gem", 1), ("banshee_wail", 1)],
        "wand_armor_t6", 1);

    g.add_recipe("eq_wand_weapon_t7", "Forge Dragonbone Wand",
        CraftingSkill::Runecrafting, 7, 7,
        &[("jeweled_rune", 2), ("jeweled_tapestry", 1), ("precious_diadem", 1), ("beholder_eye", 1)],
        "wand_weapon_t7", 1);
    g.add_recipe("eq_wand_armor_t7", "Craft Dragon Dark Vestments",
        CraftingSkill::Runecrafting, 7, 7,
        &[("jeweled_tapestry", 2), ("jeweled_rune", 1), ("precious_diadem", 1), ("golem_core", 1)],
        "wand_armor_t7", 1);

    g.add_recipe("eq_wand_weapon_t8", "Forge Voidchannel Wand",
        CraftingSkill::Runecrafting, 8, 8,
        &[("runic_weave", 2), ("runic_tapestry", 1), ("runic_gem", 1), ("demilich_gem", 1)],
        "wand_weapon_t8", 1);
    g.add_recipe("eq_wand_armor_t8", "Craft Void Dark Vestments",
        CraftingSkill::Runecrafting, 8, 8,
        &[("runic_tapestry", 2), ("runic_weave", 1), ("runic_gem", 1), ("gloom_silk", 1)],
        "wand_armor_t8", 1);

    g.add_recipe("eq_wand_weapon_t9", "Forge Celestial Wand",
        CraftingSkill::Runecrafting, 9, 9,
        &[("artificed_weave", 2), ("artificed_tapestry", 1), ("artificed_gem", 1), ("titan_bone", 1)],
        "wand_weapon_t9", 1);
    g.add_recipe("eq_wand_armor_t9", "Craft Celestial Vestments",
        CraftingSkill::Runecrafting, 9, 9,
        &[("artificed_tapestry", 2), ("artificed_weave", 1), ("artificed_gem", 1), ("astral_fragment", 1)],
        "wand_armor_t9", 1);

    g.add_recipe("eq_wand_weapon_t10", "Forge Primordial Dominus Wand",
        CraftingSkill::Runecrafting, 10, 10,
        &[("divine_weave", 2), ("divine_tapestry", 1), ("divine_gem", 1), ("lurker_shadow", 1)],
        "wand_weapon_t10", 1);
    g.add_recipe("eq_wand_armor_t10", "Craft Primordial Shadow Vestments",
        CraftingSkill::Runecrafting, 10, 10,
        &[("divine_tapestry", 2), ("divine_weave", 1), ("divine_gem", 1), ("dracolich_fang", 1)],
        "wand_armor_t10", 1);


    // --- SCEPTER line: SM+RC+TL ---
    g.add_material("scepter_weapon_t1", "Crude Scepter", 1, MaterialSource::Crafted);
    g.add_material("scepter_armor_t1", "Crude Priest Vestments", 1, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t2", "Iron Scepter", 2, MaterialSource::Crafted);
    g.add_material("scepter_armor_t2", "Iron-clasped Priest Vestments", 2, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t3", "Steel Holy Scepter", 3, MaterialSource::Crafted);
    g.add_material("scepter_armor_t3", "Steel-trimmed Priest Vestments", 3, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t4", "Dwarven Scepter", 4, MaterialSource::Crafted);
    g.add_material("scepter_armor_t4", "Dwarven Priest Vestments", 4, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t5", "Mithril Scepter", 5, MaterialSource::Crafted);
    g.add_material("scepter_armor_t5", "Mithril Priest Vestments", 5, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t6", "Rune Scepter", 6, MaterialSource::Crafted);
    g.add_material("scepter_armor_t6", "Rune Priest Vestments", 6, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t7", "Dragonforged Scepter", 7, MaterialSource::Crafted);
    g.add_material("scepter_armor_t7", "Dragon Priest Vestments", 7, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t8", "Voidforged Scepter", 8, MaterialSource::Crafted);
    g.add_material("scepter_armor_t8", "Void Priest Vestments", 8, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t9", "Celestial Scepter", 9, MaterialSource::Crafted);
    g.add_material("scepter_armor_t9", "Celestial Priest Vestments", 9, MaterialSource::Crafted);
    g.add_material("scepter_weapon_t10", "Primordial Divine Scepter", 10, MaterialSource::Crafted);
    g.add_material("scepter_armor_t10", "Primordial High Priest Vestments", 10, MaterialSource::Crafted);

    g.add_recipe("eq_scepter_weapon_t1", "Forge Crude Scepter",
        CraftingSkill::Smithing, 1, 1,
        &[("iron_nugget", 2), ("bone_charm", 1), ("woven_cloth", 1), ("venom_sac", 1)],
        "scepter_weapon_t1", 1);
    g.add_recipe("eq_scepter_armor_t1", "Craft Crude Priest Vestments",
        CraftingSkill::Smithing, 1, 1,
        &[("bone_charm", 2), ("iron_nugget", 1), ("woven_cloth", 1), ("ectoplasm", 1)],
        "scepter_armor_t1", 1);

    g.add_recipe("eq_scepter_weapon_t2", "Forge Iron Scepter",
        CraftingSkill::Smithing, 2, 2,
        &[("iron_ingot", 2), ("etched_rune", 1), ("silk_bolt", 1), ("arcane_crystal", 1)],
        "scepter_weapon_t2", 1);
    g.add_recipe("eq_scepter_armor_t2", "Craft Iron-clasped Priest Vestments",
        CraftingSkill::Smithing, 2, 2,
        &[("etched_rune", 2), ("iron_ingot", 1), ("silk_bolt", 1), ("wolf_pelt", 1)],
        "scepter_armor_t2", 1);

    g.add_recipe("eq_scepter_weapon_t3", "Forge Steel Holy Scepter",
        CraftingSkill::Smithing, 3, 3,
        &[("steel_plate", 2), ("power_rune", 1), ("moonsilk", 1), ("wraith_dust", 1)],
        "scepter_weapon_t3", 1);
    g.add_recipe("eq_scepter_armor_t3", "Craft Steel-trimmed Priest Vestments",
        CraftingSkill::Smithing, 3, 3,
        &[("power_rune", 2), ("steel_plate", 1), ("moonsilk", 1), ("shadow_thread", 1)],
        "scepter_armor_t3", 1);

    g.add_recipe("eq_scepter_weapon_t4", "Forge Dwarven Scepter",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_steel", 2), ("alchemical_rune", 1), ("alchemical_silk", 1), ("troll_blood", 1)],
        "scepter_weapon_t4", 1);
    g.add_recipe("eq_scepter_armor_t4", "Craft Dwarven Priest Vestments",
        CraftingSkill::Smithing, 4, 4,
        &[("alchemical_rune", 2), ("alchemical_steel", 1), ("alchemical_silk", 1), ("elemental_core", 1)],
        "scepter_armor_t4", 1);

    g.add_recipe("eq_scepter_weapon_t5", "Forge Mithril Scepter",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_steel", 2), ("enchanted_rune", 1), ("enchanted_silk", 1), ("stalker_claw", 1)],
        "scepter_weapon_t5", 1);
    g.add_recipe("eq_scepter_armor_t5", "Craft Mithril Priest Vestments",
        CraftingSkill::Smithing, 5, 5,
        &[("enchanted_rune", 2), ("enchanted_steel", 1), ("enchanted_silk", 1), ("mummy_wrappings", 1)],
        "scepter_armor_t5", 1);

    g.add_recipe("eq_scepter_weapon_t6", "Forge Rune Scepter",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_steel", 2), ("arcane_rune", 1), ("arcane_tapestry", 1), ("elder_crystal", 1)],
        "scepter_weapon_t6", 1);
    g.add_recipe("eq_scepter_armor_t6", "Craft Rune Priest Vestments",
        CraftingSkill::Smithing, 6, 6,
        &[("arcane_rune", 2), ("arcane_steel", 1), ("arcane_tapestry", 1), ("giant_sinew", 1)],
        "scepter_armor_t6", 1);

    g.add_recipe("eq_scepter_weapon_t7", "Forge Dragonforged Scepter",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_steel", 2), ("jeweled_rune", 1), ("jeweled_tapestry", 1), ("lich_phylactery", 1)],
        "scepter_weapon_t7", 1);
    g.add_recipe("eq_scepter_armor_t7", "Craft Dragon Priest Vestments",
        CraftingSkill::Smithing, 7, 7,
        &[("jeweled_rune", 2), ("jeweled_steel", 1), ("jeweled_tapestry", 1), ("nightwalker_shade", 1)],
        "scepter_armor_t7", 1);

    g.add_recipe("eq_scepter_weapon_t8", "Forge Voidforged Scepter",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_steel", 2), ("runic_elixir", 1), ("runic_tapestry", 1), ("storm_essence", 1)],
        "scepter_weapon_t8", 1);
    g.add_recipe("eq_scepter_armor_t8", "Craft Void Priest Vestments",
        CraftingSkill::Smithing, 8, 8,
        &[("runic_elixir", 2), ("runic_steel", 1), ("runic_tapestry", 1), ("beholder_eye", 1)],
        "scepter_armor_t8", 1);

    g.add_recipe("eq_scepter_weapon_t9", "Forge Celestial Scepter",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_steel", 2), ("artificed_elixir", 1), ("artificed_tapestry", 1), ("wraith_lord_cloak", 1)],
        "scepter_weapon_t9", 1);
    g.add_recipe("eq_scepter_armor_t9", "Craft Celestial Priest Vestments",
        CraftingSkill::Smithing, 9, 9,
        &[("artificed_elixir", 2), ("artificed_steel", 1), ("artificed_tapestry", 1), ("demilich_gem", 1)],
        "scepter_armor_t9", 1);

    g.add_recipe("eq_scepter_weapon_t10", "Forge Primordial Divine Scepter",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_steel", 2), ("divine_elixir", 1), ("divine_tapestry", 1), ("arcanum_core", 1)],
        "scepter_weapon_t10", 1);
    g.add_recipe("eq_scepter_armor_t10", "Craft Primordial High Priest Vestments",
        CraftingSkill::Smithing, 10, 10,
        &[("divine_elixir", 2), ("divine_steel", 1), ("divine_tapestry", 1), ("titan_bone", 1)],
        "scepter_armor_t10", 1);


    // --- SONG line: WW+TL+JC ---
    g.add_material("song_weapon_t1", "Crude Lute", 1, MaterialSource::Crafted);
    g.add_material("song_armor_t1", "Crude Performer Garb", 1, MaterialSource::Crafted);
    g.add_material("song_weapon_t2", "Wooden Lyre", 2, MaterialSource::Crafted);
    g.add_material("song_armor_t2", "Iron-clasped Performer Garb", 2, MaterialSource::Crafted);
    g.add_material("song_weapon_t3", "Steel-strung Lyre", 3, MaterialSource::Crafted);
    g.add_material("song_armor_t3", "Steel-trimmed Performer Garb", 3, MaterialSource::Crafted);
    g.add_material("song_weapon_t4", "Dwarven Warhorn", 4, MaterialSource::Crafted);
    g.add_material("song_armor_t4", "Dwarven Performer Garb", 4, MaterialSource::Crafted);
    g.add_material("song_weapon_t5", "Mithril Harp", 5, MaterialSource::Crafted);
    g.add_material("song_armor_t5", "Mithril Performer Garb", 5, MaterialSource::Crafted);
    g.add_material("song_weapon_t6", "Rune Lute", 6, MaterialSource::Crafted);
    g.add_material("song_armor_t6", "Rune Performer Garb", 6, MaterialSource::Crafted);
    g.add_material("song_weapon_t7", "Dragonsung Lyre", 7, MaterialSource::Crafted);
    g.add_material("song_armor_t7", "Dragon Performer Garb", 7, MaterialSource::Crafted);
    g.add_material("song_weapon_t8", "Voidecho Harp", 8, MaterialSource::Crafted);
    g.add_material("song_armor_t8", "Void Performer Garb", 8, MaterialSource::Crafted);
    g.add_material("song_weapon_t9", "Celestial Lyre", 9, MaterialSource::Crafted);
    g.add_material("song_armor_t9", "Celestial Performer Garb", 9, MaterialSource::Crafted);
    g.add_material("song_weapon_t10", "Primordial Worldsong", 10, MaterialSource::Crafted);
    g.add_material("song_armor_t10", "Primordial Maestro Garb", 10, MaterialSource::Crafted);

    g.add_recipe("eq_song_weapon_t1", "Forge Crude Lute",
        CraftingSkill::Woodworking, 1, 1,
        &[("shaped_wood", 2), ("woven_cloth", 1), ("polished_quartz", 1), ("mana_shard", 1)],
        "song_weapon_t1", 1);
    g.add_recipe("eq_song_armor_t1", "Craft Crude Performer Garb",
        CraftingSkill::Woodworking, 1, 1,
        &[("woven_cloth", 2), ("shaped_wood", 1), ("polished_quartz", 1), ("wolf_pelt", 1)],
        "song_armor_t1", 1);

    g.add_recipe("eq_song_weapon_t2", "Forge Wooden Lyre",
        CraftingSkill::Woodworking, 2, 2,
        &[("ironwood_plank", 2), ("silk_bolt", 1), ("cut_gemstone", 1), ("dark_iron_ore", 1)],
        "song_weapon_t2", 1);
    g.add_recipe("eq_song_armor_t2", "Craft Iron-clasped Performer Garb",
        CraftingSkill::Woodworking, 2, 2,
        &[("silk_bolt", 2), ("ironwood_plank", 1), ("cut_gemstone", 1), ("venom_sac", 1)],
        "song_armor_t2", 1);

    g.add_recipe("eq_song_weapon_t3", "Forge Steel-strung Lyre",
        CraftingSkill::Woodworking, 3, 3,
        &[("hardwood_beam", 2), ("moonsilk", 1), ("jeweled_setting", 1), ("orc_tusk", 1)],
        "song_weapon_t3", 1);
    g.add_recipe("eq_song_armor_t3", "Craft Steel-trimmed Performer Garb",
        CraftingSkill::Woodworking, 3, 3,
        &[("moonsilk", 2), ("hardwood_beam", 1), ("jeweled_setting", 1), ("arcane_crystal", 1)],
        "song_armor_t3", 1);

    g.add_recipe("eq_song_weapon_t4", "Forge Dwarven Warhorn",
        CraftingSkill::Woodworking, 4, 4,
        &[("alchemical_hardwood", 2), ("alchemical_silk", 1), ("alchemical_gem", 1), ("phase_venom", 1)],
        "song_weapon_t4", 1);
    g.add_recipe("eq_song_armor_t4", "Craft Dwarven Performer Garb",
        CraftingSkill::Woodworking, 4, 4,
        &[("alchemical_silk", 2), ("alchemical_hardwood", 1), ("alchemical_gem", 1), ("wraith_dust", 1)],
        "song_armor_t4", 1);

    g.add_recipe("eq_song_weapon_t5", "Forge Mithril Harp",
        CraftingSkill::Woodworking, 5, 5,
        &[("enchanted_hardwood", 2), ("enchanted_silk", 1), ("enchanted_gem", 1), ("naga_pearl", 1)],
        "song_weapon_t5", 1);
    g.add_recipe("eq_song_armor_t5", "Craft Mithril Performer Garb",
        CraftingSkill::Woodworking, 5, 5,
        &[("enchanted_silk", 2), ("enchanted_hardwood", 1), ("enchanted_gem", 1), ("troll_blood", 1)],
        "song_armor_t5", 1);

    g.add_recipe("eq_song_weapon_t6", "Forge Rune Lute",
        CraftingSkill::Woodworking, 6, 6,
        &[("arcane_hardwood", 2), ("arcane_tapestry", 1), ("arcane_gem", 1), ("death_knight_shard", 1)],
        "song_weapon_t6", 1);
    g.add_recipe("eq_song_armor_t6", "Craft Rune Performer Garb",
        CraftingSkill::Woodworking, 6, 6,
        &[("arcane_tapestry", 2), ("arcane_hardwood", 1), ("arcane_gem", 1), ("stalker_claw", 1)],
        "song_armor_t6", 1);

    g.add_recipe("eq_song_weapon_t7", "Forge Dragonsung Lyre",
        CraftingSkill::Woodworking, 7, 7,
        &[("jeweled_hardwood", 2), ("jeweled_tapestry", 1), ("precious_diadem", 1), ("dragon_scale", 1)],
        "song_weapon_t7", 1);
    g.add_recipe("eq_song_armor_t7", "Craft Dragon Performer Garb",
        CraftingSkill::Woodworking, 7, 7,
        &[("jeweled_tapestry", 2), ("jeweled_hardwood", 1), ("precious_diadem", 1), ("elder_crystal", 1)],
        "song_armor_t7", 1);

    g.add_recipe("eq_song_weapon_t8", "Forge Voidecho Harp",
        CraftingSkill::Woodworking, 8, 8,
        &[("runic_hardwood", 2), ("runic_tapestry", 1), ("runic_gem", 1), ("void_silk", 1)],
        "song_weapon_t8", 1);
    g.add_recipe("eq_song_armor_t8", "Craft Void Performer Garb",
        CraftingSkill::Woodworking, 8, 8,
        &[("runic_tapestry", 2), ("runic_hardwood", 1), ("runic_gem", 1), ("lich_phylactery", 1)],
        "song_armor_t8", 1);

    g.add_recipe("eq_song_weapon_t9", "Forge Celestial Lyre",
        CraftingSkill::Woodworking, 9, 9,
        &[("artificed_hardwood", 2), ("artificed_tapestry", 1), ("artificed_gem", 1), ("arch_lich_dust", 1)],
        "song_weapon_t9", 1);
    g.add_recipe("eq_song_armor_t9", "Craft Celestial Performer Garb",
        CraftingSkill::Woodworking, 9, 9,
        &[("artificed_tapestry", 2), ("artificed_hardwood", 1), ("artificed_gem", 1), ("storm_essence", 1)],
        "song_armor_t9", 1);

    g.add_recipe("eq_song_weapon_t10", "Forge Primordial Worldsong",
        CraftingSkill::Woodworking, 10, 10,
        &[("divine_hardwood", 2), ("divine_tapestry", 1), ("divine_gem", 1), ("undying_essence", 1)],
        "song_weapon_t10", 1);
    g.add_recipe("eq_song_armor_t10", "Craft Primordial Maestro Garb",
        CraftingSkill::Woodworking, 10, 10,
        &[("divine_tapestry", 2), ("divine_hardwood", 1), ("divine_gem", 1), ("wraith_lord_cloak", 1)],
        "song_armor_t10", 1);

    g.analyze_usage();
    g
}

// ========================================================================
// RUNTIME BRIDGE: Convert materials to inventory items
// ========================================================================

fn tier_to_rarity(tier: u8) -> Rarity {
    match tier {
        0..=1 => Rarity::Common,
        2..=3 => Rarity::Uncommon,
        4..=5 => Rarity::Rare,
        6..=7 => Rarity::Epic,
        _ => Rarity::Legendary,
    }
}

fn tier_to_value(tier: u8) -> u32 {
    match tier {
        0 => 1,
        1 => 5,
        2 => 15,
        3 => 50,
        4 => 175,
        5 => 600,
        6 => 2100,
        7 => 7500,
        8 => 26000,
        9 => 90000,
        10 => 300000,
        _ => 1,
    }
}

pub fn material_to_item(graph: &CraftingGraph, material_id: &str) -> Option<Item> {
    let mat = graph.materials.get(material_id)?;
    Some(Item {
        id: material_id.to_string(),
        name: mat.name.clone(),
        description: format!("Tier {} crafting material", mat.tier),
        item_type: ItemType::Material,
        slot: None,
        rarity: tier_to_rarity(mat.tier),
        weight: 0.1,
        value_gp: tier_to_value(mat.tier),
        stats: ItemStats::default(),
        enchantment: None,
        quantity: 1,
        tier: mat.tier,
        properties: None,
        image_id: None,
    })
}

/// Convert an equipment recipe output into a real equippable Item.
/// Returns None if the material_id is not an equipment item (e.g., intermediate materials).
pub fn equipment_to_item(material_id: &str) -> Option<Item> {
    // Equipment IDs follow pattern: {line}_{type}_t{tier}
    // e.g., "blade_weapon_t3", "bow_armor_t5"
    let parts: Vec<&str> = material_id.split('_').collect();
    if parts.len() < 3 { return None; }

    let line = parts[0];  // blade, axe, holy, dagger, bow, fist, staff, wand, scepter, song
    let eq_type = parts[1]; // weapon, sword, armor
    let tier_str = parts.last()?;
    let tier: u8 = tier_str.strip_prefix('t')?.parse().ok()?;

    // Only handle weapon/armor equipment types
    if eq_type != "weapon" && eq_type != "armor" && eq_type != "sword" {
        return None;
    }

    let graph = &*CRAFTING_GRAPH;
    let mat = graph.materials.get(material_id)?;

    let is_weapon = eq_type != "armor";

    if is_weapon {
        Some(make_weapon(line, tier, &mat.name))
    } else {
        Some(make_armor(line, tier, &mat.name))
    }
}

fn make_weapon(line: &str, tier: u8, name: &str) -> Item {
    use super::equipment::EquipSlot;

    // Weapon stats scale with tier (from simulator curves)
    let (dice, count, _dmg_mod, attack_bonus) = match tier {
        1 => ("d6", 1u32, 1i32, 0i32),
        2 => ("d6", 1, 2, 1),
        3 => ("d8", 1, 3, 1),
        4 => ("d8", 1, 4, 2),
        5 => ("d8", 1, 5, 2),
        6 => ("d10", 1, 6, 3),
        7 => ("d10", 1, 8, 3),
        8 => ("d12", 1, 10, 4),
        9 => ("d8", 2, 12, 5),
        10 => ("d10", 2, 14, 6),
        _ => ("d4", 1, 0, 0),
    };

    let is_ranged = matches!(line, "bow");
    let is_finesse = matches!(line, "dagger");
    let is_two_handed = matches!(line, "axe" | "bow" | "staff");
    let damage_stat = if is_ranged || is_finesse {
        "dex"
    } else if matches!(line, "staff" | "wand") {
        "int"
    } else {
        "str"
    };

    let damage_dice = if count > 1 {
        format!("{}{}", count, dice)
    } else {
        dice.to_string()
    };

    Item {
        id: material_id_for_weapon(line, tier),
        name: name.to_string(),
        description: format!("Tier {} {} weapon", tier, line),
        item_type: ItemType::Weapon,
        slot: Some(EquipSlot::MainHand),
        rarity: tier_to_rarity(tier),
        weight: if is_two_handed { 4.0 } else { 2.0 },
        value_gp: tier_to_value(tier) * 3,
        stats: ItemStats {
            damage_dice: Some(damage_dice),
            damage_modifier_stat: Some(damage_stat.to_string()),
            attack_bonus,
            is_ranged,
            is_finesse,
            is_two_handed,
            ..ItemStats::default()
        },
        enchantment: None,
        quantity: 1,
        tier,
        properties: None,
        image_id: None,
    }
}

fn material_id_for_weapon(line: &str, tier: u8) -> String {
    format!("{}_weapon_t{}", line, tier)
}

fn make_armor(line: &str, tier: u8, name: &str) -> Item {
    use super::equipment::EquipSlot;

    // Armor AC scales with tier; armor weight class depends on equipment line
    let (ac_base, special) = match line {
        // Heavy armor (no DEX bonus)
        "blade" | "axe" | "holy" | "scepter" => {
            let ac = 13 + tier as i32;
            (ac, Some("no_dex".to_string()))
        }
        // Medium armor (DEX capped at +2)
        "bow" | "dagger" => {
            let ac = 11 + tier as i32;
            (ac, Some("dex_cap_2".to_string()))
        }
        // Light armor (full DEX)
        _ => {
            let ac = 10 + tier as i32;
            (ac, None)
        }
    };

    Item {
        id: format!("{}_armor_t{}", line, tier),
        name: name.to_string(),
        description: format!("Tier {} {} armor", tier, line),
        item_type: ItemType::Armor,
        slot: Some(EquipSlot::Chest),
        rarity: tier_to_rarity(tier),
        weight: if special.as_deref() == Some("no_dex") { 15.0 } else { 5.0 },
        value_gp: tier_to_value(tier) * 4,
        stats: ItemStats {
            ac_base: Some(ac_base),
            special,
            ..ItemStats::default()
        },
        enchantment: None,
        quantity: 1,
        tier,
        properties: None,
        image_id: None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equipment_to_item_weapon() {
        let item = equipment_to_item("blade_weapon_t1").expect("Should produce blade weapon T1");
        assert_eq!(item.item_type, ItemType::Weapon);
        assert_eq!(item.tier, 1);
        assert!(item.slot.is_some());
        assert!(item.stats.damage_dice.is_some());
        let dice = item.stats.damage_dice.as_ref().unwrap();
        assert_eq!(dice, "d6"); // T1 = d6
        assert_eq!(item.stats.attack_bonus, 0); // T1 = 0 attack bonus
        assert!(!item.stats.is_ranged);
        assert!(!item.stats.is_finesse);
    }

    #[test]
    fn test_equipment_to_item_armor() {
        let item = equipment_to_item("blade_armor_t1").expect("Should produce blade armor T1");
        assert_eq!(item.item_type, ItemType::Armor);
        assert_eq!(item.tier, 1);
        assert!(item.slot.is_some());
        assert!(item.stats.ac_base.is_some());
        // Blade armor = heavy (13 + tier)
        assert_eq!(item.stats.ac_base.unwrap(), 14); // 13 + 1
        assert_eq!(item.stats.special.as_deref(), Some("no_dex"));
    }

    #[test]
    fn test_equipment_to_item_ranged() {
        let item = equipment_to_item("bow_weapon_t3").expect("Should produce bow weapon T3");
        assert_eq!(item.item_type, ItemType::Weapon);
        assert!(item.stats.is_ranged);
        assert!(item.stats.is_two_handed);
        assert_eq!(item.stats.damage_modifier_stat.as_deref(), Some("dex"));
    }

    #[test]
    fn test_equipment_to_item_finesse() {
        let item = equipment_to_item("dagger_weapon_t2").expect("Should produce dagger weapon T2");
        assert_eq!(item.item_type, ItemType::Weapon);
        assert!(item.stats.is_finesse);
        assert_eq!(item.stats.damage_modifier_stat.as_deref(), Some("dex"));
    }

    #[test]
    fn test_equipment_to_item_caster() {
        let item = equipment_to_item("staff_weapon_t5").expect("Should produce staff weapon T5");
        assert_eq!(item.item_type, ItemType::Weapon);
        assert!(item.stats.is_two_handed);
        assert_eq!(item.stats.damage_modifier_stat.as_deref(), Some("int"));
    }

    #[test]
    fn test_equipment_to_item_light_armor() {
        let item = equipment_to_item("fist_armor_t3").expect("Should produce fist armor T3");
        assert_eq!(item.item_type, ItemType::Armor);
        // Fist = light armor (10 + tier)
        assert_eq!(item.stats.ac_base.unwrap(), 13); // 10 + 3
        assert!(item.stats.special.is_none()); // Light = full DEX
    }

    #[test]
    fn test_equipment_to_item_medium_armor() {
        let item = equipment_to_item("bow_armor_t5").expect("Should produce bow armor T5");
        assert_eq!(item.item_type, ItemType::Armor);
        // Bow = medium armor (11 + tier)
        assert_eq!(item.stats.ac_base.unwrap(), 16); // 11 + 5
        assert_eq!(item.stats.special.as_deref(), Some("dex_cap_2"));
    }

    #[test]
    fn test_equipment_to_item_nonexistent() {
        assert!(equipment_to_item("nonexistent_weapon_t1").is_none());
    }

    #[test]
    fn test_equipment_to_item_intermediate_material() {
        // Intermediate materials should return None
        assert!(equipment_to_item("iron_ingot").is_none());
        assert!(equipment_to_item("cured_leather").is_none());
    }

    #[test]
    fn test_equipment_tier_scaling() {
        // Verify stats scale with tier
        let t1 = equipment_to_item("blade_weapon_t1").unwrap();
        let t5 = equipment_to_item("blade_weapon_t5").unwrap();
        let t10 = equipment_to_item("blade_weapon_t10").unwrap();
        
        // Attack bonus should increase
        assert!(t5.stats.attack_bonus > t1.stats.attack_bonus);
        assert!(t10.stats.attack_bonus > t5.stats.attack_bonus);
        
        // Value should increase
        assert!(t5.value_gp > t1.value_gp);
        assert!(t10.value_gp > t5.value_gp);
    }

    #[test]
    fn test_all_equipment_lines_produce_items() {
        let lines = vec!["blade", "axe", "holy", "dagger", "bow", "fist", "staff", "wand", "scepter", "song"];
        for line in &lines {
            let wep_id = format!("{}_weapon_t1", line);
            let arm_id = format!("{}_armor_t1", line);
            assert!(
                equipment_to_item(&wep_id).is_some(),
                "Missing weapon for line: {}", line
            );
            assert!(
                equipment_to_item(&arm_id).is_some(),
                "Missing armor for line: {}", line
            );
        }
    }
}
