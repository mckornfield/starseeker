use macroquad::prelude::*;

pub const PICKUP_RANGE: f32 = 38.0;

#[derive(Clone, Copy)]
pub enum LootKind {
    Credits(u32),
    WeaponShard, // placeholder — Phase 4 makes this a real component
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
        match self.kind {
            LootKind::Credits(_) => self.draw_credits(),
            LootKind::WeaponShard => self.draw_shard(),
        }
    }

    fn draw_credits(&self) {
        let r = 7.0;
        // Spinning square
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

    fn draw_shard(&self) {
        let r = 8.0;
        let pulse = (self.spin * 2.0).sin() * 0.3 + 0.7;
        draw_circle(
            self.pos.x,
            self.pos.y,
            r,
            Color::new(0.3, 0.7, 1.0, pulse * 0.5),
        );
        draw_circle_lines(self.pos.x, self.pos.y, r, 1.5, Color::new(0.4, 0.8, 1.0, pulse));
        draw_circle(self.pos.x, self.pos.y, 3.0, WHITE);
    }
}
