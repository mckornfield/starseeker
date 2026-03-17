use macroquad::prelude::*;

pub const PROJECTILE_SPEED: f32 = 600.0;
const LIFETIME: f32 = 2.0;
const RADIUS: f32 = 3.0;

pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub lifetime: f32,
    pub color: Color,
}

impl Projectile {
    pub fn new(pos: Vec2, direction: Vec2, color: Color) -> Self {
        Self {
            pos,
            vel: direction * PROJECTILE_SPEED,
            lifetime: LIFETIME,
            color,
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
        // Faint trail dot
        draw_circle(
            self.pos.x - self.vel.normalize().x * 6.0,
            self.pos.y - self.vel.normalize().y * 6.0,
            RADIUS * 0.5,
            Color::new(self.color.r, self.color.g, self.color.b, 0.4),
        );
    }
}
