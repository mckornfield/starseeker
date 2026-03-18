use super::rarity::Rarity;
use macroquad::prelude::*;

const WEAPON_PREFIXES: &[&str] = &[
    "VOLT", "PLASMA", "ION", "PHASE", "CRYO", "FLUX", "NULL", "SOLAR", "DARK", "HYPER",
];
const WEAPON_NOUNS: &[&str] = &[
    "BLASTER", "CANNON", "RIFLE", "LANCE", "BOLT", "PULSAR", "VORTEX", "NOVA", "LANCER", "SPIKE",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WeaponSlot {
    Main,
    Aux,
}

#[derive(Clone)]
pub(crate) struct WeaponItem {
    pub name: String,
    pub rarity: Rarity,
    pub slot: WeaponSlot,
    pub damage: f32,
    pub fire_rate: f32, // cooldown seconds (lower = faster)
    pub proj_speed: f32,
    pub proj_color: Color,
    pub spread: bool, // true = twin bolts (main style)
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

    pub fn gen_main() -> Self {
        Self::gen_for_slot(WeaponSlot::Main)
    }
    pub fn gen_aux() -> Self {
        Self::gen_for_slot(WeaponSlot::Aux)
    }

    fn gen_for_slot(slot: WeaponSlot) -> Self {
        let rarity = Rarity::roll();
        let budget = rarity.budget_mult();

        let (base_dmg, base_rate, base_speed) = match slot {
            WeaponSlot::Main => (20.0_f32, 0.18_f32, 600.0_f32),
            WeaponSlot::Aux => (35.0_f32, 0.65_f32, 550.0_f32),
        };

        let dmg_mult = budget * quad_rand::gen_range(0.85_f32, 1.15_f32);
        let speed_mult = (0.8 + budget * 0.25) * quad_rand::gen_range(0.9_f32, 1.1_f32);
        let rate_div = budget * quad_rand::gen_range(0.9_f32, 1.1_f32);

        let proj_color = hsv_to_color(quad_rand::gen_range(0.0_f32, 1.0));

        let prefix =
            WEAPON_PREFIXES[quad_rand::gen_range(0, WEAPON_PREFIXES.len() as u32) as usize];
        let noun = WEAPON_NOUNS[quad_rand::gen_range(0, WEAPON_NOUNS.len() as u32) as usize];

        Self {
            name: format!("{} {}", prefix, noun),
            rarity,
            slot,
            damage: base_dmg * dmg_mult,
            fire_rate: (base_rate / rate_div).max(0.05),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_red_at_zero() {
        let c = hsv_to_color(0.0);
        assert!((c.r - 1.0).abs() < 1e-5);
        assert!(c.g.abs() < 1e-5);
        assert!(c.b.abs() < 1e-5);
    }

    #[test]
    fn hsv_green_at_one_third() {
        let c = hsv_to_color(1.0 / 3.0);
        assert!(c.r.abs() < 1e-5);
        assert!((c.g - 1.0).abs() < 1e-5);
        assert!(c.b.abs() < 1e-5);
    }

    #[test]
    fn hsv_blue_at_two_thirds() {
        let c = hsv_to_color(2.0 / 3.0);
        assert!(c.r.abs() < 1e-5);
        assert!(c.g.abs() < 1e-5);
        assert!((c.b - 1.0).abs() < 1e-5);
    }

    #[test]
    fn hsv_all_values_in_range() {
        for i in 0..100 {
            let h = i as f32 / 100.0;
            let c = hsv_to_color(h);
            assert!(c.r >= 0.0 && c.r <= 1.0, "r out of range at h={h}");
            assert!(c.g >= 0.0 && c.g <= 1.0, "g out of range at h={h}");
            assert!(c.b >= 0.0 && c.b <= 1.0, "b out of range at h={h}");
            assert!((c.a - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn default_main_weapon_has_spread() {
        let w = WeaponItem::default_main();
        assert!(w.spread);
        assert_eq!(w.slot, WeaponSlot::Main);
    }

    #[test]
    fn default_aux_weapon_no_spread() {
        let w = WeaponItem::default_aux();
        assert!(!w.spread);
        assert_eq!(w.slot, WeaponSlot::Aux);
    }

    #[test]
    fn generated_weapons_have_positive_stats() {
        quad_rand::srand(99);
        for _ in 0..50 {
            let w = WeaponItem::gen_main();
            assert!(w.damage > 0.0);
            assert!(w.fire_rate > 0.0);
            assert!(w.proj_speed > 0.0);
            assert!(!w.name.is_empty());
        }
    }
}
