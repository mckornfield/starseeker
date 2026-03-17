use macroquad::prelude::*;
use crate::items::{ThrusterItem, WeaponItem};

pub const PICKUP_RANGE: f32 = 38.0;

pub enum LootKind {
    Credits(u32),
    Weapon(WeaponItem),
    Thruster(ThrusterItem),
}

pub struct LootDrop {
    pub pos: Vec2,
    pub kind: LootKind,
    spin: f32,
}

impl LootDrop {
    pub fn new(pos: Vec2, kind: LootKind) -> Self {
        Self { pos, kind, spin: 0.0 }
    }

    pub fn update(&mut self, dt: f32) {
        self.spin += dt * 1.8;
    }

    pub fn draw(&self) {
        match &self.kind {
            LootKind::Credits(_)  => self.draw_credits(),
            LootKind::Weapon(w)   => self.draw_weapon(w),
            LootKind::Thruster(t) => self.draw_thruster(t),
        }
    }

    fn draw_credits(&self) {
        let r = 7.0;
        let corners: Vec<Vec2> = (0..4)
            .map(|i| {
                let a = self.spin + i as f32 * std::f32::consts::FRAC_PI_2;
                self.pos + Vec2::new(a.cos(), a.sin()) * r
            })
            .collect();
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            draw_line(a.x, a.y, b.x, b.y, 1.5, GOLD);
        }
        draw_circle(self.pos.x, self.pos.y, 2.5, Color::new(1.0, 0.85, 0.0, 0.8));
    }

    fn draw_weapon(&self, w: &WeaponItem) {
        let pulse = (self.spin * 2.0).sin() * 0.3 + 0.7;
        let rc = w.rarity.color();
        // Rarity glow ring
        draw_circle(self.pos.x, self.pos.y, 11.0, Color::new(rc.r, rc.g, rc.b, pulse * 0.25));
        draw_circle_lines(self.pos.x, self.pos.y, 11.0, 1.5, Color::new(rc.r, rc.g, rc.b, pulse));
        // Spinning diamond (4-point star outline) in weapon color
        let r = 6.0;
        let pts: [Vec2; 4] = std::array::from_fn(|i| {
            let a = self.spin + i as f32 * std::f32::consts::FRAC_PI_2;
            self.pos + Vec2::new(a.cos(), a.sin()) * r
        });
        let wc = w.proj_color;
        for i in 0..4 {
            draw_line(pts[i].x, pts[i].y, pts[(i + 1) % 4].x, pts[(i + 1) % 4].y, 1.5, wc);
        }
        draw_circle(self.pos.x, self.pos.y, 2.5, wc);
    }

    fn draw_thruster(&self, t: &ThrusterItem) {
        let pulse = (self.spin * 2.5).sin() * 0.3 + 0.7;
        let rc = t.rarity.color();
        // Rarity glow
        draw_circle(self.pos.x, self.pos.y, 11.0, Color::new(rc.r, rc.g, rc.b, pulse * 0.2));
        // Spinning triangle
        let r = 8.0;
        let pts: [Vec2; 3] = std::array::from_fn(|i| {
            let a = self.spin + i as f32 * std::f32::consts::TAU / 3.0;
            self.pos + Vec2::new(a.cos(), a.sin()) * r
        });
        draw_triangle_lines(pts[0], pts[1], pts[2], 1.5, Color::new(rc.r, rc.g, rc.b, pulse));
        draw_circle(self.pos.x, self.pos.y, 3.0, Color::new(rc.r, rc.g, rc.b, pulse));
    }
}
