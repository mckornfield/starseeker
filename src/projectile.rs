use macroquad::prelude::*;

pub const PROJECTILE_SPEED: f32 = 600.0;
const LIFETIME: f32 = 2.0;
const RADIUS: f32 = 3.0;

#[derive(PartialEq, Clone, Copy)]
pub enum Owner {
    Player,
    Enemy,
}

pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub lifetime: f32,
    pub color: Color,
    pub owner: Owner,
}

impl Projectile {
    pub fn new(pos: Vec2, direction: Vec2, color: Color) -> Self {
        Self {
            pos,
            vel: direction * PROJECTILE_SPEED,
            lifetime: LIFETIME,
            color,
            owner: Owner::Player,
        }
    }

    pub fn new_enemy(pos: Vec2, direction: Vec2, color: Color) -> Self {
        Self {
            pos,
            vel: direction * (PROJECTILE_SPEED * 0.65),
            lifetime: LIFETIME * 1.2,
            color,
            owner: Owner::Enemy,
        }
    }

    /// Returns false when the projectile should be removed.
    pub fn update(&mut self, dt: f32) -> bool {
        self.pos += self.vel * dt;
        self.lifetime -= dt;
        self.lifetime > 0.0
    }

    pub fn draw(&self) {
        draw_circle(self.pos.x, self.pos.y, RADIUS, self.color);
        let trail = self.pos - self.vel.normalize() * 6.0;
        draw_circle(
            trail.x,
            trail.y,
            RADIUS * 0.5,
            Color::new(self.color.r, self.color.g, self.color.b, 0.35),
        );
    }
}
