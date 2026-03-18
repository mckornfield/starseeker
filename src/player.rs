use crate::input::InputState;
use crate::items::{Loadout, ThrusterItem, WeaponItem};
use crate::projectile::Projectile;
use macroquad::prelude::*;

const ROTATION_SPEED: f32 = 3.0;
const BASE_THRUST: f32 = 320.0;
const DRAG: f32 = 0.999;
const BASE_MAX_SPEED: f32 = 650.0;

pub(crate) struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub rotation: f32, // radians; 0 = pointing up
    pub is_thrusting: bool,
    pub loadout: Loadout,

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
            loadout: Loadout::starter(),
            main_cooldown: 0.0,
            aux_cooldown: 0.0,
        }
    }

    pub fn equip_weapon(&mut self, w: WeaponItem) -> Option<(String, crate::items::Rarity)> {
        self.loadout.try_equip_weapon(w)
    }

    pub fn equip_thruster(&mut self, t: ThrusterItem) -> Option<crate::items::Rarity> {
        self.loadout.try_equip_thruster(t)
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

        // Thruster stats
        let (thrust, max_speed) = if let Some(ref t) = self.loadout.thruster {
            (BASE_THRUST * t.accel_mult, BASE_MAX_SPEED * t.speed_mult)
        } else {
            (BASE_THRUST, BASE_MAX_SPEED)
        };

        if input.thrust {
            self.vel += forward * thrust * dt;
        }
        if input.brake {
            self.vel -= forward * thrust * dt;
        }

        self.vel *= DRAG;
        if self.vel.length() > max_speed {
            self.vel = self.vel.normalize() * max_speed;
        }

        self.pos += self.vel * dt;

        self.main_cooldown = (self.main_cooldown - dt).max(0.0);
        self.aux_cooldown = (self.aux_cooldown - dt).max(0.0);

        // Main weapon
        if input.fire_main && self.main_cooldown == 0.0 {
            if let Some(ref w) = self.loadout.main {
                let right = Vec2::new(forward.y, -forward.x);
                let speed = w.proj_speed;
                let dmg = w.damage;
                let color = w.proj_color;
                if w.spread {
                    projectiles.push(Projectile::new(
                        self.pos + right * 8.0,
                        forward,
                        speed,
                        dmg,
                        color,
                    ));
                    projectiles.push(Projectile::new(
                        self.pos - right * 8.0,
                        forward,
                        speed,
                        dmg,
                        color,
                    ));
                } else {
                    projectiles.push(Projectile::new(self.pos, forward, speed, dmg, color));
                }
                self.main_cooldown = w.fire_rate;
            }
        }

        // Aux weapon
        if input.fire_aux && self.aux_cooldown == 0.0 {
            if let Some(ref w) = self.loadout.aux {
                let speed = w.proj_speed;
                let dmg = w.damage;
                let color = w.proj_color;
                projectiles.push(Projectile::new(self.pos, forward, speed, dmg, color));
                self.aux_cooldown = w.fire_rate;
            }
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
            // Tint exhaust by thruster rarity if equipped
            let (inner, outer) = if let Some(ref t) = self.loadout.thruster {
                let rc = t.rarity.color();
                (Color::new(rc.r, rc.g, rc.b, 0.9), ORANGE)
            } else {
                (YELLOW, ORANGE)
            };
            draw_circle(exhaust.x, exhaust.y, 5.0, outer);
            draw_circle(exhaust.x, exhaust.y, 3.0, inner);
        }
    }
}
