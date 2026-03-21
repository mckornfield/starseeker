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
    pub is_braking: bool,
    pub is_stabilizing: bool,
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
            is_braking: false,
            is_stabilizing: false,
            loadout: Loadout::starter(),
            main_cooldown: 0.0,
            aux_cooldown: 0.0,
        }
    }

    pub fn equip_weapon(
        &mut self,
        w: WeaponItem,
    ) -> Result<(String, crate::items::Rarity, Option<WeaponItem>), WeaponItem> {
        self.loadout.try_equip_weapon(w)
    }

    pub fn equip_thruster(
        &mut self,
        t: ThrusterItem,
    ) -> Result<(crate::items::Rarity, Option<ThrusterItem>), ThrusterItem> {
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
        self.is_braking = input.brake;
        self.is_stabilizing = input.stabilize && self.vel.length_squared() > 1.0;

        // Thruster stats
        let (thrust, max_speed) = if let Some(ref t) = self.loadout.thruster {
            (BASE_THRUST * t.accel_mult, BASE_MAX_SPEED * t.speed_mult)
        } else {
            (BASE_THRUST, BASE_MAX_SPEED)
        };

        if input.stabilize {
            // Retro-thruster: strong exponential damping toward zero
            self.vel *= 1.0 - (dt * 4.0).min(1.0);
            if self.vel.length_squared() < 1.0 {
                self.vel = Vec2::ZERO;
            }
        } else {
            if input.thrust {
                self.vel += forward * thrust * dt;
            }
            if input.brake {
                self.vel -= forward * thrust * dt;
            }
        }

        self.vel *= DRAG;
        if self.speed() > max_speed {
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
                let vel = self.vel;
                if w.spread {
                    projectiles.push(Projectile::new(
                        self.pos + right * 8.0,
                        forward,
                        speed,
                        dmg,
                        color,
                        vel,
                    ));
                    projectiles.push(Projectile::new(
                        self.pos - right * 8.0,
                        forward,
                        speed,
                        dmg,
                        color,
                        vel,
                    ));
                } else {
                    projectiles.push(Projectile::new(self.pos, forward, speed, dmg, color, vel));
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
                projectiles.push(Projectile::new(self.pos, forward, speed, dmg, color, self.vel));
                self.aux_cooldown = w.fire_rate;
            }
        }
    }

    pub fn speed(&self) -> f32 {
        self.vel.length()
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

        if self.is_braking {
            let exhaust = self.pos + forward * (size * 0.85);
            draw_circle(exhaust.x, exhaust.y, 3.0, ORANGE);
            draw_circle(exhaust.x, exhaust.y, 1.8, YELLOW);
        }

        if self.is_stabilizing {
            // Retro-thruster ring: 8 puffs equally spaced around the ship
            let ring_r = size * 1.4;
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let dir = Vec2::new(angle.sin(), -angle.cos());
                let puff = self.pos + dir * ring_r;
                draw_circle(puff.x, puff.y, 4.0, Color::new(0.4, 0.8, 1.0, 0.55));
                draw_circle(puff.x, puff.y, 2.0, Color::new(0.9, 1.0, 1.0, 0.85));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputState;

    fn no_input() -> InputState {
        InputState::default()
    }

    fn input_with(f: impl Fn(&mut InputState)) -> InputState {
        let mut i = InputState::default();
        f(&mut i);
        i
    }

    #[test]
    fn new_player_starts_at_rest() {
        let p = Player::new(Vec2::new(100.0, 200.0));
        assert_eq!(p.pos, Vec2::new(100.0, 200.0));
        assert_eq!(p.vel, Vec2::ZERO);
        assert_eq!(p.rotation, 0.0);
        assert!(!p.is_thrusting);
        assert!(!p.is_braking);
        assert!(!p.is_stabilizing);
    }

    #[test]
    fn rotate_left_decreases_rotation() {
        let mut p = Player::new(Vec2::ZERO);
        let input = input_with(|i| i.rotate_left = true);
        p.update(0.1, &input, &mut vec![]);
        assert!(p.rotation < 0.0, "expected rotation < 0, got {}", p.rotation);
    }

    #[test]
    fn rotate_right_increases_rotation() {
        let mut p = Player::new(Vec2::ZERO);
        let input = input_with(|i| i.rotate_right = true);
        p.update(0.1, &input, &mut vec![]);
        assert!(p.rotation > 0.0, "expected rotation > 0, got {}", p.rotation);
    }

    #[test]
    fn thrust_increases_speed() {
        let mut p = Player::new(Vec2::ZERO);
        let input = input_with(|i| i.thrust = true);
        p.update(0.1, &input, &mut vec![]);
        assert!(p.speed() > 0.0, "thrust should accelerate the player");
        assert!(p.is_thrusting);
    }

    #[test]
    fn drag_slows_player_over_time() {
        let mut p = Player::new(Vec2::ZERO);
        p.vel = Vec2::new(100.0, 0.0);
        let initial_speed = p.speed();
        p.update(0.1, &no_input(), &mut vec![]);
        assert!(p.speed() < initial_speed, "drag should reduce speed each frame");
    }

    #[test]
    fn stabilize_damps_velocity_toward_zero() {
        let mut p = Player::new(Vec2::ZERO);
        p.vel = Vec2::new(200.0, 0.0);
        let input = input_with(|i| i.stabilize = true);
        p.update(0.1, &input, &mut vec![]);
        assert!(p.speed() < 200.0, "stabilize should reduce speed");
        assert!(p.is_stabilizing);
    }

    #[test]
    fn stabilize_does_not_set_flag_when_nearly_stopped() {
        let mut p = Player::new(Vec2::ZERO);
        p.vel = Vec2::new(0.5, 0.0); // below the length_squared > 1.0 threshold
        let input = input_with(|i| i.stabilize = true);
        p.update(0.016, &input, &mut vec![]);
        assert!(!p.is_stabilizing, "stabilize flag should be false when nearly stopped");
    }

    #[test]
    fn speed_clamped_to_max() {
        let mut p = Player::new(Vec2::ZERO);
        // Apply many frames of thrust to try to exceed max speed
        let input = input_with(|i| i.thrust = true);
        for _ in 0..200 {
            p.update(0.1, &input, &mut vec![]);
        }
        assert!(p.speed() <= BASE_MAX_SPEED + 1.0, "speed should not exceed max: {}", p.speed());
    }

    #[test]
    fn firing_main_weapon_spawns_projectiles() {
        let mut p = Player::new(Vec2::ZERO);
        let mut projectiles = vec![];
        let input = input_with(|i| i.fire_main = true);
        p.update(0.016, &input, &mut projectiles);
        assert!(!projectiles.is_empty(), "main weapon should spawn at least one projectile");
        assert!(projectiles.iter().all(|pr| pr.owner == crate::projectile::Owner::Player));
    }

    #[test]
    fn main_weapon_cooldown_prevents_rapid_fire() {
        let mut p = Player::new(Vec2::ZERO);
        let mut projectiles = vec![];
        let input = input_with(|i| i.fire_main = true);
        p.update(0.016, &input, &mut projectiles);
        let after_first = projectiles.len();
        // Fire again immediately — cooldown should block it
        p.update(0.001, &input, &mut projectiles);
        assert_eq!(projectiles.len(), after_first, "cooldown should block rapid re-fire");
    }

    #[test]
    fn position_updates_with_velocity() {
        let mut p = Player::new(Vec2::ZERO);
        p.vel = Vec2::new(100.0, 0.0);
        p.update(1.0, &no_input(), &mut vec![]);
        // With drag applied, pos should be close to 100 * DRAG
        assert!(p.pos.x > 90.0 && p.pos.x < 110.0, "pos.x should be ~100, got {}", p.pos.x);
    }
}
