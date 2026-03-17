use macroquad::prelude::Color;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
}

impl Rarity {
    /// Stat budget multiplier for this rarity tier.
    pub fn budget_mult(self) -> f32 {
        match self {
            Rarity::Common   => 1.0,
            Rarity::Uncommon => 1.3,
            Rarity::Rare     => 1.7,
            Rarity::Epic     => 2.2,
        }
    }

    pub fn color(self) -> Color {
        match self {
            Rarity::Common   => Color::new(0.75, 0.75, 0.75, 1.0),
            Rarity::Uncommon => Color::new(0.2,  0.9,  0.2,  1.0),
            Rarity::Rare     => Color::new(0.3,  0.5,  1.0,  1.0),
            Rarity::Epic     => Color::new(0.7,  0.2,  1.0,  1.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rarity::Common   => "CMN",
            Rarity::Uncommon => "UNC",
            Rarity::Rare     => "RARE",
            Rarity::Epic     => "EPIC",
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
