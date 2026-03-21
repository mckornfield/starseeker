use macroquad::prelude::Color;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
}

impl Rarity {
    /// Stat budget multiplier for this rarity tier.
    pub fn budget_mult(self) -> f32 {
        match self {
            Rarity::Common => 1.0,
            Rarity::Uncommon => 1.3,
            Rarity::Rare => 1.7,
            Rarity::Epic => 2.2,
        }
    }

    pub fn color(self) -> Color {
        match self {
            Rarity::Common => Color::new(0.75, 0.75, 0.75, 1.0),
            Rarity::Uncommon => Color::new(0.2, 0.9, 0.2, 1.0),
            Rarity::Rare => Color::new(0.3, 0.5, 1.0, 1.0),
            Rarity::Epic => Color::new(0.7, 0.2, 1.0, 1.0),
        }
    }

    pub fn base_price(self) -> u32 {
        match self {
            Rarity::Common => 50,
            Rarity::Uncommon => 150,
            Rarity::Rare => 400,
            Rarity::Epic => 1000,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rarity::Common => "CMN",
            Rarity::Uncommon => "UNC",
            Rarity::Rare => "RARE",
            Rarity::Epic => "EPIC",
        }
    }

    /// Weighted random roll: 50% Common, 30% Uncommon, 15% Rare, 5% Epic.
    pub fn roll() -> Self {
        let r = quad_rand::gen_range(0.0_f32, 1.0);
        if r < 0.50 {
            Rarity::Common
        } else if r < 0.80 {
            Rarity::Uncommon
        } else if r < 0.95 {
            Rarity::Rare
        } else {
            Rarity::Epic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_mult_increases_with_rarity() {
        assert!(Rarity::Common.budget_mult() < Rarity::Uncommon.budget_mult());
        assert!(Rarity::Uncommon.budget_mult() < Rarity::Rare.budget_mult());
        assert!(Rarity::Rare.budget_mult() < Rarity::Epic.budget_mult());
    }

    #[test]
    fn ordering_is_common_to_epic() {
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
    }

    #[test]
    fn labels_are_nonempty() {
        for r in [Rarity::Common, Rarity::Uncommon, Rarity::Rare, Rarity::Epic] {
            assert!(!r.label().is_empty());
        }
    }

    #[test]
    fn roll_returns_valid_rarity() {
        quad_rand::srand(42);
        for _ in 0..200 {
            let r = Rarity::roll();
            assert!(r >= Rarity::Common && r <= Rarity::Epic);
        }
    }

    #[test]
    fn roll_distribution_is_roughly_correct() {
        quad_rand::srand(12345);
        let n = 10_000;
        let mut counts = [0u32; 4];
        for _ in 0..n {
            match Rarity::roll() {
                Rarity::Common => counts[0] += 1,
                Rarity::Uncommon => counts[1] += 1,
                Rarity::Rare => counts[2] += 1,
                Rarity::Epic => counts[3] += 1,
            }
        }
        let pct = |c: u32| c as f32 / n as f32;
        // Allow ±8% tolerance for statistical variance
        assert!(
            pct(counts[0]) > 0.40 && pct(counts[0]) < 0.60,
            "Common: {}",
            pct(counts[0])
        );
        assert!(
            pct(counts[1]) > 0.20 && pct(counts[1]) < 0.40,
            "Uncommon: {}",
            pct(counts[1])
        );
        assert!(
            pct(counts[2]) > 0.07 && pct(counts[2]) < 0.23,
            "Rare: {}",
            pct(counts[2])
        );
        assert!(
            pct(counts[3]) > 0.01 && pct(counts[3]) < 0.12,
            "Epic: {}",
            pct(counts[3])
        );
    }
}
