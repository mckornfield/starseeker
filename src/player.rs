use macroquad::prelude::*;
use crate::input::InputState;
use crate::projectile::Projectile;

const ROTATION_SPEED: f32 = 3.0;   // rad/s
const THRUST: f32 = 320.0;         // units/s²
const DRAG: f32 = 0.999;           // near-frictionless; Asteroids-style momentum
const MAX_SPEED: f32 = 650.0;

const MAIN_FIRE_RATE: f32 = 0.18;
const AUX_FIRE_RATE: f32 = 0.65;

pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub rotation: f32,      // radians; 0 = pointing up
    pub is_thrusting: bool, // set each update, read by draw

    main_cooldown: f32,
    aux_cooldown: f32,
}

impl Player {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
            rotation: 0.0,
            is_thrusting: false,
            main_cooldown: 0.0,
            aux_cooldown: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, input: &InputState, projectiles: &mut Vec<Projectile>) {
        if input.rotate_left {
            self.rotation -= ROTATION_SPEED * dt;
        }
        if input.rotate_right {
            self.rotation += ROTATION_SPEED * dt;
        }

        let forward = Vec2::new(self.rotation.sin(), -self.rotation.cos());
        self.is_thrusting = input.thrust;

        if input.thrust {
            self.vel += forward * THRUST * dt;
        }
        if input.brake {
            self.vel -= forward * THRUST * dt;
        }

        self.vel *= DRAG;
        if self.vel.length() > MAX_SPEED {
            self.vel = self.vel.normalize() * MAX_SPEED;
        }

        self.pos += self.vel * dt;

        self.main_cooldown = (self.main_cooldown - dt).max(0.0);
        self.aux_cooldown = (self.aux_cooldown - dt).max(0.0);

        // Main weapons — twin bolts offset left/right of nose
        if input.fire_main && self.main_cooldown == 0.0 {
            let right = Vec2::new(forward.y, -forward.x);
            projectiles.push(Projectile::new(self.pos + right * 8.0, forward, SKYBLUE));
            projectiles.push(Projectile::new(self.pos - right * 8.0, forward, SKYBLUE));
            self.main_cooldown = MAIN_FIRE_RATE;
        }

        // Aux weapon — single heavy shot
        if input.fire_aux && self.aux_cooldown == 0.0 {
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

        if self.is_thrusting {
            let exhaust = self.pos - forward * (size * 0.55);
            draw_circle(exhaust.x, exhaust.y, 5.0, ORANGE);
            draw_circle(exhaust.x, exhaust.y, 3.0, YELLOW);
        }
    }
}
