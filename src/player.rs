use macroquad::prelude::*;
use crate::projectile::Projectile;

const ROTATION_SPEED: f32 = 3.0;   // rad/s
const THRUST: f32 = 280.0;         // units/s²
const DRAG: f32 = 0.97;
const MAX_SPEED: f32 = 420.0;

// Fire rates (seconds between shots per slot)
const MAIN_FIRE_RATE: f32 = 0.18;
const AUX_FIRE_RATE: f32 = 0.65;

pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub rotation: f32,  // radians; 0 = pointing up

    // Weapon cooldowns
    main_cooldown: f32,
    aux_cooldown: f32,
}

impl Player {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
            rotation: 0.0,
            main_cooldown: 0.0,
            aux_cooldown: 0.0,
        }
    }

    /// Updates state, appends any new projectiles to `projectiles`.
    pub fn update(&mut self, dt: f32, projectiles: &mut Vec<Projectile>) {
        // Rotation
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.rotation -= ROTATION_SPEED * dt;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.rotation += ROTATION_SPEED * dt;
        }

        // Thrust (forward only for now; reverse thruster in Phase 2)
        let forward = Vec2::new(self.rotation.sin(), -self.rotation.cos());
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            self.vel += forward * THRUST * dt;
        }

        // Drag + speed cap
        self.vel *= DRAG;
        if self.vel.length() > MAX_SPEED {
            self.vel = self.vel.normalize() * MAX_SPEED;
        }

        self.pos += self.vel * dt;

        // Cooldowns
        self.main_cooldown = (self.main_cooldown - dt).max(0.0);
        self.aux_cooldown = (self.aux_cooldown - dt).max(0.0);

        // Main weapons (Space) — twin bolts offset left/right of nose
        if is_key_down(KeyCode::Space) && self.main_cooldown == 0.0 {
            let right = Vec2::new(forward.y, -forward.x);
            let spawn_offset = 8.0;
            projectiles.push(Projectile::new(
                self.pos + right * spawn_offset,
                forward,
                SKYBLUE,
            ));
            projectiles.push(Projectile::new(
                self.pos - right * spawn_offset,
                forward,
                SKYBLUE,
            ));
            self.main_cooldown = MAIN_FIRE_RATE;
        }

        // Aux weapon (Left Ctrl / Z) — single heavy shot
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::Z))
            && self.aux_cooldown == 0.0
        {
            projectiles.push(Projectile::new(self.pos, forward, ORANGE));
            self.aux_cooldown = AUX_FIRE_RATE;
        }
    }

    pub fn draw(&self) {
        let size = 16.0;
        let forward = Vec2::new(self.rotation.sin(), -self.rotation.cos());
        let right = Vec2::new(forward.y, -forward.x);

        let tip = self.pos + forward * size;
        let left_wing = self.pos - forward * (size * 0.5) + right * (size * 0.65);
        let right_wing = self.pos - forward * (size * 0.5) - right * (size * 0.65);

        draw_triangle(tip, left_wing, right_wing, WHITE);
        draw_triangle_lines(tip, left_wing, right_wing, 1.0, LIGHTGRAY);

        // Engine glow
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            let exhaust = self.pos - forward * (size * 0.55);
            draw_circle(exhaust.x, exhaust.y, 5.0, ORANGE);
            draw_circle(exhaust.x, exhaust.y, 3.0, YELLOW);
        }
    }
}
