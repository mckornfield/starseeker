use super::rarity::Rarity;
use macroquad::prelude::Color;

#[derive(Clone, Debug)]
pub(crate) struct ThrusterItem {
    pub rarity: Rarity,
    pub speed_mult: f32,
    pub accel_mult: f32,
    pub color: Color,
}

impl ThrusterItem {
    pub fn gen() -> Self {
        let rarity = Rarity::roll();
        let extra = rarity.budget_mult() - 1.0;
        // Floor of 0.08 ensures even Common thrusters provide a small base benefit
        Self {
            rarity,
            speed_mult: 1.0 + 0.08 + extra * quad_rand::gen_range(0.4_f32, 0.9_f32),
            accel_mult: 1.0 + 0.08 + extra * quad_rand::gen_range(0.3_f32, 0.8_f32),
            color: hsv_to_color(quad_rand::gen_range(0.0_f32, 1.0)),
        }
    }

    pub fn stat_summary(&self) -> String {
        format!("SPD:{:.2}x  ACC:{:.2}x", self.speed_mult, self.accel_mult)
    }
}

/// Simple HSV→RGB (S=1, V=1) for vivid procedural exhaust colors.
fn hsv_to_color(h: f32) -> Color {
    let h6 = h * 6.0;
    let i = h6 as u32;
    let f = h6 - i as f32;
    let (r, g, b) = match i % 6 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    Color::new(r, g, b, 1.0)
}
