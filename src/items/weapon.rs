use macroquad::prelude::*;
use super::rarity::Rarity;

const WEAPON_PREFIXES: &[&str] = &[
    "VOLT", "PLASMA", "ION", "PHASE", "CRYO",
    "FLUX", "NULL", "SOLAR", "DARK", "HYPER",
];
const WEAPON_NOUNS: &[&str] = &[
    "BLASTER", "CANNON", "RIFLE", "LANCE", "BOLT",
    "PULSAR", "VORTEX", "NOVA", "LANCER", "SPIKE",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WeaponSlot {
    Main,
    Aux,
}

#[derive(Clone)]
pub struct WeaponItem {
    pub name: String,
    pub rarity: Rarity,
    pub slot: WeaponSlot,
    pub damage: f32,
    pub fire_rate: f32,   // cooldown seconds (lower = faster)
    pub proj_speed: f32,
    pub proj_color: Color,
    pub spread: bool,     // true = twin bolts (main style)
}

impl WeaponItem {
    pub fn default_main() -> Self {
        Self {
            name: "TWIN BLASTER".to_string(),
            rarity: Rarity::Common,
            slot: WeaponSlot::Main,
            damage: 20.0,
            fire_rate: 0.18,
            proj_speed: 600.0,
            proj_color: SKYBLUE,
            spread: true,
        }
    }

    pub fn default_aux() -> Self {
        Self {
            name: "HEAVY SHOT".to_string(),
            rarity: Rarity::Common,
            slot: WeaponSlot::Aux,
            damage: 35.0,
            fire_rate: 0.65,
            proj_speed: 550.0,
            proj_color: ORANGE,
            spread: false,
        }
    }

    pub fn gen_main() -> Self { Self::gen_for_slot(WeaponSlot::Main) }
    pub fn gen_aux()  -> Self { Self::gen_for_slot(WeaponSlot::Aux)  }

    fn gen_for_slot(slot: WeaponSlot) -> Self {
        let rarity = Rarity::roll();
        let budget = rarity.budget_mult();

        let (base_dmg, base_rate, base_speed) = match slot {
            WeaponSlot::Main => (20.0_f32, 0.18_f32, 600.0_f32),
            WeaponSlot::Aux  => (35.0_f32, 0.65_f32, 550.0_f32),
        };

        let dmg_mult   = budget * quad_rand::gen_range(0.85_f32, 1.15_f32);
        let speed_mult = (0.8 + budget * 0.25) * quad_rand::gen_range(0.9_f32, 1.1_f32);
        let rate_div   = budget * quad_rand::gen_range(0.9_f32, 1.1_f32);

        let proj_color = hsv_to_color(quad_rand::gen_range(0.0_f32, 1.0));

        let prefix = WEAPON_PREFIXES[quad_rand::gen_range(0, WEAPON_PREFIXES.len() as u32) as usize];
        let noun   = WEAPON_NOUNS  [quad_rand::gen_range(0, WEAPON_NOUNS.len()   as u32) as usize];

        Self {
            name: format!("{} {}", prefix, noun),
            rarity,
            slot,
            damage:     base_dmg   * dmg_mult,
            fire_rate:  (base_rate / rate_div).max(0.05),
            proj_speed: base_speed * speed_mult,
            proj_color,
            spread: matches!(slot, WeaponSlot::Main),
        }
    }

    pub fn stat_summary(&self) -> String {
        format!("DMG:{:.0}  SPD:{:.0}", self.damage, self.proj_speed)
    }
}

/// Simple HSV→RGB conversion (S=1, V=1) for vivid procedural colors.
fn hsv_to_color(h: f32) -> Color {
    let h6 = h * 6.0;
    let i  = h6 as u32;
    let f  = h6 - i as f32;
    let (r, g, b) = match i % 6 {
        0 => (1.0,       f,   0.0),
        1 => (1.0 - f, 1.0,   0.0),
        2 => (0.0,     1.0,     f),
        3 => (0.0, 1.0 - f,   1.0),
        4 => (f,         0.0, 1.0),
        _ => (1.0,       0.0, 1.0 - f),
    };
    Color::new(r, g, b, 1.0)
}
