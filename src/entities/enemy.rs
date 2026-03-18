use crate::projectile::Projectile;
use macroquad::prelude::*;

const DETECT_RANGE: f32 = 700.0;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum EnemyArchetype {
    Tank,
    Agile,
    Ranged,
}

impl EnemyArchetype {
    /// Collision/hit radius for this archetype — single source of truth.
    pub fn hit_radius(self) -> f32 {
        match self {
            EnemyArchetype::Tank => 18.0,
            EnemyArchetype::Agile => 10.0,
            EnemyArchetype::Ranged => 13.0,
        }
    }
}

pub(crate) struct Enemy {
    pub pos: Vec2,
    pub vel: Vec2,
    pub rotation: f32,
    pub health: f32,
    pub max_health: f32,
    pub archetype: EnemyArchetype,
    fire_cooldown: f32,
    patrol_dir: Vec2,
    patrol_timer: f32,
    orbit_sign: f32,
}

impl Enemy {
    pub fn new(pos: Vec2, archetype: EnemyArchetype) -> Self {
        let health = match archetype {
            EnemyArchetype::Tank => 80.0,
            EnemyArchetype::Agile => 25.0,
            EnemyArchetype::Ranged => 40.0,
        };
        // Use pos to seed a stable orbit sign per-enemy
        let orbit_sign = if (pos.x as i32 + pos.y as i32) % 2 == 0 {
            1.0
        } else {
            -1.0
        };
        Self {
            pos,
            vel: Vec2::ZERO,
            rotation: 0.0,
            health,
            max_health: health,
            archetype,
            fire_cooldown: 0.0,
            patrol_dir: Vec2::X,
            patrol_timer: 0.0,
            orbit_sign,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.health <= 0.0
    }

    pub fn take_damage(&mut self, damage: f32) {
        self.health -= damage;
    }

    pub fn update(&mut self, dt: f32, player_pos: Vec2, projectiles: &mut Vec<Projectile>) {
        self.fire_cooldown = (self.fire_cooldown - dt).max(0.0);
        self.patrol_timer = (self.patrol_timer - dt).max(0.0);

        let dist = self.pos.distance(player_pos);
        let to_player = if dist > 1.0 {
            (player_pos - self.pos) / dist
        } else {
            Vec2::X
        };

        match self.archetype {
            EnemyArchetype::Tank => {
                if dist < DETECT_RANGE {
                    self.vel += to_player * 140.0 * dt;
                } else {
                    self.patrol(dt, 55.0);
                }
                if dist < 420.0 && self.fire_cooldown == 0.0 {
                    projectiles.push(Projectile::new_enemy(self.pos, to_player, RED));
                    self.fire_cooldown = 1.8;
                }
                self.vel *= 0.97;
                self.vel = self.vel.clamp_length_max(160.0);
            }

            EnemyArchetype::Agile => {
                if dist < DETECT_RANGE {
                    let perp = Vec2::new(-to_player.y, to_player.x) * self.orbit_sign;
                    self.vel += (to_player + perp * 0.45).normalize() * 480.0 * dt;
                } else {
                    self.patrol(dt, 140.0);
                }
                self.vel *= 0.985;
                self.vel = self.vel.clamp_length_max(480.0);
            }

            EnemyArchetype::Ranged => {
                if dist < DETECT_RANGE * 1.3 {
                    let ideal = 480.0_f32;
                    let push = if dist < ideal - 50.0 {
                        -to_player
                    } else if dist > ideal + 50.0 {
                        to_player
                    } else {
                        Vec2::ZERO
                    };
                    self.vel += push * 180.0 * dt;
                    if dist < DETECT_RANGE && self.fire_cooldown == 0.0 {
                        projectiles.push(Projectile::new_enemy(self.pos, to_player, VIOLET));
                        self.fire_cooldown = 1.2;
                    }
                } else {
                    self.patrol(dt, 70.0);
                }
                self.vel *= 0.975;
                self.vel = self.vel.clamp_length_max(200.0);
            }
        }

        // Smoothly aim rotation toward velocity
        if self.vel.length() > 10.0 {
            let target = self.vel.x.atan2(-self.vel.y);
            let diff = wrap_angle(target - self.rotation);
            self.rotation += diff * (dt * 8.0).min(1.0);
        }

        self.pos += self.vel * dt;
    }

    fn patrol(&mut self, dt: f32, thrust: f32) {
        if self.patrol_timer <= 0.0 {
            let a = (self.pos.x * 0.001 + self.pos.y * 0.0013 + get_time() as f32 * 0.3).sin()
                * std::f32::consts::TAU;
            self.patrol_dir = Vec2::new(a.cos(), a.sin());
            self.patrol_timer = 2.5;
        }
        self.vel += self.patrol_dir * thrust * dt;
    }

    pub fn draw(&self) {
        match self.archetype {
            EnemyArchetype::Tank => self.draw_tank(),
            EnemyArchetype::Agile => self.draw_agile(),
            EnemyArchetype::Ranged => self.draw_ranged(),
        }
        self.draw_health_bar();
    }

    fn draw_tank(&self) {
        let r = self.archetype.hit_radius();
        draw_circle(
            self.pos.x,
            self.pos.y,
            r * 0.55,
            Color::new(0.5, 0.05, 0.05, 0.8),
        );
        draw_circle_lines(self.pos.x, self.pos.y, r, 2.5, RED);
        draw_circle_lines(
            self.pos.x,
            self.pos.y,
            r * 0.6,
            1.0,
            Color::new(1.0, 0.3, 0.3, 0.5),
        );
        // Cross hatch
        draw_line(
            self.pos.x - r,
            self.pos.y,
            self.pos.x + r,
            self.pos.y,
            1.5,
            Color::new(1.0, 0.3, 0.3, 0.4),
        );
        draw_line(
            self.pos.x,
            self.pos.y - r,
            self.pos.x,
            self.pos.y + r,
            1.5,
            Color::new(1.0, 0.3, 0.3, 0.4),
        );
    }

    fn draw_agile(&self) {
        let size = self.archetype.hit_radius();
        let fwd = Vec2::new(self.rotation.sin(), -self.rotation.cos());
        let right = Vec2::new(fwd.y, -fwd.x);
        let tip = self.pos + fwd * size;
        let lw = self.pos - fwd * (size * 0.6) + right * (size * 0.55);
        let rw = self.pos - fwd * (size * 0.6) - right * (size * 0.55);
        draw_triangle(tip, lw, rw, ORANGE);
        draw_triangle_lines(tip, lw, rw, 1.0, Color::new(1.0, 0.6, 0.0, 0.6));
    }

    fn draw_ranged(&self) {
        let r = self.archetype.hit_radius();
        let up = self.pos + Vec2::new(0.0, -r);
        let dn = self.pos + Vec2::new(0.0, r);
        let lt = self.pos + Vec2::new(-r * 0.65, 0.0);
        let rt = self.pos + Vec2::new(r * 0.65, 0.0);
        draw_triangle(up, rt, dn, Color::new(0.5, 0.1, 0.8, 0.5));
        draw_triangle(up, lt, dn, Color::new(0.5, 0.1, 0.8, 0.5));
        draw_line(up.x, up.y, rt.x, rt.y, 1.5, VIOLET);
        draw_line(rt.x, rt.y, dn.x, dn.y, 1.5, VIOLET);
        draw_line(dn.x, dn.y, lt.x, lt.y, 1.5, VIOLET);
        draw_line(lt.x, lt.y, up.x, up.y, 1.5, VIOLET);
        draw_circle(self.pos.x, self.pos.y, 3.5, Color::new(0.7, 0.3, 1.0, 0.7));
    }

    fn draw_health_bar(&self) {
        if self.health >= self.max_health {
            return;
        }
        let w = 30.0;
        let h = 3.0;
        let x = self.pos.x - w * 0.5;
        let y = self.pos.y - 28.0;
        draw_rectangle(x, y, w, h, Color::new(0.4, 0.0, 0.0, 0.7));
        let fill = (self.health / self.max_health).max(0.0);
        draw_rectangle(x, y, w * fill, h, Color::new(0.1, 0.85, 0.2, 0.9));
    }
}

fn wrap_angle(mut a: f32) -> f32 {
    use std::f32::consts::PI;
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn wrap_angle_zero() {
        assert!((wrap_angle(0.0)).abs() < 1e-6);
    }

    #[test]
    fn wrap_angle_positive_wrap() {
        let result = wrap_angle(3.0 * PI);
        assert!(result > -PI && result <= PI, "got {result}");
        assert!((result - PI).abs() < 1e-5);
    }

    #[test]
    fn wrap_angle_negative_wrap() {
        let result = wrap_angle(-3.0 * PI);
        assert!((-PI..=PI).contains(&result), "got {result}");
    }

    #[test]
    fn wrap_angle_already_in_range() {
        let v = 1.0_f32;
        assert!((wrap_angle(v) - v).abs() < 1e-6);
    }

    #[test]
    fn hit_radius_values() {
        assert_eq!(EnemyArchetype::Tank.hit_radius(), 18.0);
        assert_eq!(EnemyArchetype::Agile.hit_radius(), 10.0);
        assert_eq!(EnemyArchetype::Ranged.hit_radius(), 13.0);
    }

    #[test]
    fn enemy_takes_damage_and_dies() {
        let mut e = Enemy::new(Vec2::ZERO, EnemyArchetype::Agile);
        assert!(!e.is_dead());
        e.take_damage(25.0);
        assert!(e.is_dead());
    }
}
