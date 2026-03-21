use super::rarity::Rarity;

#[derive(Clone, Debug)]
pub(crate) struct ThrusterItem {
    pub rarity: Rarity,
    pub speed_mult: f32,
    pub accel_mult: f32,
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
        }
    }

    pub fn stat_summary(&self) -> String {
        format!("SPD:{:.2}x  ACC:{:.2}x", self.speed_mult, self.accel_mult)
    }
}
