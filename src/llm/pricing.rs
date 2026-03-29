//! Token usage tracking and cost calculation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SessionCost {
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
}

impl SessionCost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, usage: &TokenUsage) {
        self.total_prompt_tokens += usage.prompt_tokens;
        self.total_completion_tokens += usage.completion_tokens;
    }

    pub fn cost_usd(&self, model: &str) -> f64 {
        let (input_per_m, output_per_m) = model_pricing(model);
        (self.total_prompt_tokens as f64 / 1_000_000.0) * input_per_m
            + (self.total_completion_tokens as f64 / 1_000_000.0) * output_per_m
    }
}

pub fn model_cost(model: &str, usage: &TokenUsage) -> f64 {
    let (input_per_m, output_per_m) = model_pricing(model);
    (usage.prompt_tokens as f64 / 1_000_000.0) * input_per_m
        + (usage.completion_tokens as f64 / 1_000_000.0) * output_per_m
}

fn model_pricing(model: &str) -> (f64, f64) {
    // (input per million, output per million) in USD
    // Grok 4.1 Fast (both reasoning and non-reasoning): $0.20 / $0.50
    // Grok 4 (full, non-fast): $3.00 / $15.00
    match model {
        m if m.contains("grok-4-1-fast") => (0.20, 0.50),
        m if m.contains("non-reasoning") => (0.20, 0.50),
        m if m.contains("reasoning") => (3.0, 15.0), // Full Grok 4
        _ => (0.20, 0.50), // Default to cheap rate
    }
}
