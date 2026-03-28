//! Dice rolling with true randomness.

use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiceResult {
    pub dice_type: String,
    pub rolls: Vec<u32>,
    pub total: i32,
    pub modifier: i32,
    pub dc: Option<i32>,
    pub success: Option<bool>,
    pub description: String,
}

pub struct DiceRoller;

impl DiceRoller {
    /// Parse dice type string (e.g., "d20", "d6") and return the number of sides.
    fn parse_sides(dice_type: &str) -> Option<u32> {
        let s = dice_type.trim().to_lowercase();
        let s = s.strip_prefix('d').unwrap_or(&s);
        s.parse().ok()
    }

    /// Roll dice.
    pub fn roll(dice_type: &str, count: u32, modifier: i32) -> DiceResult {
        let sides = Self::parse_sides(dice_type).unwrap_or(20);
        let mut rng = rand::thread_rng();
        let rolls: Vec<u32> = (0..count).map(|_| rng.gen_range(1..=sides)).collect();
        let sum: u32 = rolls.iter().sum();
        let total = sum as i32 + modifier;

        DiceResult {
            dice_type: format!("d{}", sides),
            rolls,
            total,
            modifier,
            dc: None,
            success: None,
            description: String::new(),
        }
    }

    /// Roll dice with a difficulty class.
    pub fn roll_with_dc(
        dice_type: &str,
        count: u32,
        modifier: i32,
        dc: i32,
        description: &str,
    ) -> DiceResult {
        let mut result = Self::roll(dice_type, count, modifier);
        result.dc = Some(dc);
        result.success = Some(result.total >= dc);
        result.description = description.to_string();
        result
    }

    /// Calculate probability of meeting or exceeding a DC.
    pub fn success_probability(dice_type: &str, count: u32, modifier: i32, dc: i32) -> f64 {
        let sides = Self::parse_sides(dice_type).unwrap_or(20) as i32;

        if count == 1 {
            // Simple case: single die
            let needed = dc - modifier;
            if needed <= 1 {
                return 1.0;
            }
            if needed > sides {
                return 0.0;
            }
            (sides - needed + 1) as f64 / sides as f64
        } else {
            // For multiple dice, use Monte Carlo simulation
            let mut rng = rand::thread_rng();
            let trials = 10_000;
            let mut successes = 0;
            for _ in 0..trials {
                let sum: i32 = (0..count).map(|_| rng.gen_range(1..=sides)).sum();
                if sum + modifier >= dc {
                    successes += 1;
                }
            }
            successes as f64 / trials as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_basic() {
        let result = DiceRoller::roll("d20", 1, 0);
        assert_eq!(result.rolls.len(), 1);
        assert!(result.rolls[0] >= 1 && result.rolls[0] <= 20);
        assert_eq!(result.total, result.rolls[0] as i32);
    }

    #[test]
    fn test_roll_with_modifier() {
        let result = DiceRoller::roll("d6", 2, 3);
        assert_eq!(result.rolls.len(), 2);
        let sum: u32 = result.rolls.iter().sum();
        assert_eq!(result.total, sum as i32 + 3);
    }

    #[test]
    fn test_roll_with_dc() {
        let result = DiceRoller::roll_with_dc("d20", 1, 5, 10, "Strength check");
        assert!(result.dc == Some(10));
        assert_eq!(result.success, Some(result.total >= 10));
    }

    #[test]
    fn test_probability_guaranteed() {
        let prob = DiceRoller::success_probability("d20", 1, 20, 1);
        assert!((prob - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_probability_impossible() {
        let prob = DiceRoller::success_probability("d20", 1, 0, 25);
        assert!((prob - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_sides() {
        assert_eq!(DiceRoller::parse_sides("d20"), Some(20));
        assert_eq!(DiceRoller::parse_sides("d6"), Some(6));
        assert_eq!(DiceRoller::parse_sides("D100"), Some(100));
    }
}
