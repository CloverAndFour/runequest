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
    /// Parse a dice notation string and return (count, sides).
    /// Handles: "d20" -> (1, 20), "1d6" -> (1, 6), "2d8" -> (2, 8), "d6" -> (1, 6)
    fn parse_notation(dice_type: &str) -> (u32, u32) {
        let s = dice_type.trim().to_lowercase();
        if let Some(pos) = s.find('d') {
            let count_str = &s[..pos];
            let sides_str = &s[pos + 1..];
            let count = if count_str.is_empty() { 1 } else { count_str.parse().unwrap_or(1) };
            let sides = sides_str.parse().unwrap_or(20);
            (count, sides)
        } else {
            // No 'd' found, try parsing as just a number of sides
            (1, s.parse().unwrap_or(20))
        }
    }

    /// Parse dice type string (e.g., "d20", "d6", "1d6", "2d8") and return the number of sides.
    fn parse_sides(dice_type: &str) -> Option<u32> {
        let (_, sides) = Self::parse_notation(dice_type);
        Some(sides)
    }

    /// Roll dice from a full notation like "2d6" with an extra modifier.
    /// The count in the notation is multiplied by the count parameter.
    pub fn roll(dice_type: &str, count: u32, modifier: i32) -> DiceResult {
        let (notation_count, sides) = Self::parse_notation(dice_type);
        let count = notation_count * count;
        let mut rng = rand::thread_rng();
        let rolls: Vec<u32> = (0..count).map(|_| rng.gen_range(1..=sides)).collect();
        let sum: u32 = rolls.iter().sum();
        let total = sum as i32 + modifier;

        DiceResult {
            dice_type: format!("{}d{}", count, sides),
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
        // NdM notation
        assert_eq!(DiceRoller::parse_sides("1d6"), Some(6));
        assert_eq!(DiceRoller::parse_sides("2d8"), Some(8));
        assert_eq!(DiceRoller::parse_sides("3d10"), Some(10));
    }

    #[test]
    fn test_parse_notation() {
        assert_eq!(DiceRoller::parse_notation("d20"), (1, 20));
        assert_eq!(DiceRoller::parse_notation("d6"), (1, 6));
        assert_eq!(DiceRoller::parse_notation("1d6"), (1, 6));
        assert_eq!(DiceRoller::parse_notation("2d8"), (2, 8));
        assert_eq!(DiceRoller::parse_notation("3d10"), (3, 10));
    }

    #[test]
    fn test_roll_notation_count() {
        // "2d6" with count=1 should produce 2 rolls
        let result = DiceRoller::roll("2d6", 1, 0);
        assert_eq!(result.rolls.len(), 2);
        for r in &result.rolls {
            assert!(*r >= 1 && *r <= 6);
        }

        // "1d6" with count=1 should produce 1 roll
        let result = DiceRoller::roll("1d6", 1, 0);
        assert_eq!(result.rolls.len(), 1);
        assert!(result.rolls[0] >= 1 && result.rolls[0] <= 6);
        assert!(result.total >= 1 && result.total <= 6);
    }

    #[test]
    fn test_enemy_damage_dice_range() {
        // Simulate enemy damage roll: "1d6" should produce 1-6, not 1-20
        for _ in 0..100 {
            let result = DiceRoller::roll("1d6", 1, 0);
            assert!(result.total >= 1 && result.total <= 6,
                "1d6 rolled {}, expected 1-6", result.total);
        }
    }
}
