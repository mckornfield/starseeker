use macroquad::prelude::*;

const LIFETIME: f32 = 2.0;
const RADIUS: f32 = 3.0;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum Owner {
    Player,
    Enemy,
}

pub(crate) struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub lifetime: f32,
    pub color: Color,
    pub owner: Owner,
    pub damage: f32,
}

impl Projectile {
    pub fn new(
        pos: Vec2,
        direction: Vec2,
        speed: f32,
        damage: f32,
        color: Color,
        carrier_vel: Vec2,
    ) -> Self {
        Self {
            pos,
            vel: direction * speed + carrier_vel,
            lifetime: LIFETIME,
            color,
            owner: Owner::Player,
            damage,
        }
    }

    pub fn new_enemy(pos: Vec2, direction: Vec2, color: Color) -> Self {
        Self {
            pos,
            vel: direction * 390.0,
            lifetime: LIFETIME * 1.2,
            color,
            owner: Owner::Enemy,
            damage: 15.0,
        }
    }

    /// Returns false when the projectile should be removed.
    pub fn update(&mut self, dt: f32) -> bool {
        self.pos += self.vel * dt;
        self.lifetime -= dt;
        self.lifetime > 0.0
    }

    #[allow(dead_code)]
    pub fn is_dead(&self) -> bool {
        self.lifetime <= 0.0
    }

    pub fn draw(&self) {
        draw_circle(self.pos.x, self.pos.y, RADIUS, self.color);
        let trail = if self.vel.length_squared() > 0.0 {
            self.pos - self.vel.normalize() * 6.0
        } else {
            self.pos
        };
        draw_circle(
            trail.x,
            trail.y,
            RADIUS * 0.5,
            Color::new(self.color.r, self.color.g, self.color.b, 0.35),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white() -> Color {
        Color::new(1.0, 1.0, 1.0, 1.0)
    }

    #[test]
    fn new_player_projectile_has_correct_owner_and_damage() {
        let p = Projectile::new(Vec2::ZERO, Vec2::new(0.0, -1.0), 500.0, 25.0, white(), Vec2::ZERO);
        assert_eq!(p.owner, Owner::Player);
        assert_eq!(p.damage, 25.0);
        assert!(!p.is_dead());
    }

    #[test]
    fn new_enemy_projectile_has_enemy_owner_and_fixed_damage() {
        let p = Projectile::new_enemy(Vec2::ZERO, Vec2::new(0.0, 1.0), white());
        assert_eq!(p.owner, Owner::Enemy);
        assert_eq!(p.damage, 15.0);
    }

    #[test]
    fn update_moves_position_along_velocity() {
        let mut p = Projectile::new(Vec2::ZERO, Vec2::new(1.0, 0.0), 100.0, 10.0, white(), Vec2::ZERO);
        p.update(0.5);
        assert!((p.pos.x - 50.0).abs() < 1e-4, "expected x≈50, got {}", p.pos.x);
        assert!(p.pos.y.abs() < 1e-4);
    }

    #[test]
    fn update_returns_true_while_lifetime_remains() {
        let mut p = Projectile::new(Vec2::ZERO, Vec2::new(0.0, -1.0), 500.0, 10.0, white(), Vec2::ZERO);
        assert!(p.update(0.1), "projectile should still be alive after 0.1s");
    }

    #[test]
    fn update_returns_false_when_lifetime_expires() {
        let mut p = Projectile::new(Vec2::ZERO, Vec2::new(0.0, -1.0), 500.0, 10.0, white(), Vec2::ZERO);
        let alive = p.update(LIFETIME + 0.01);
        assert!(!alive, "projectile should be dead after lifetime expires");
        assert!(p.is_dead());
    }

    #[test]
    fn zero_velocity_projectile_does_not_produce_nan_position() {
        let mut p = Projectile::new(Vec2::new(10.0, 20.0), Vec2::ZERO, 0.0, 5.0, white(), Vec2::ZERO);
        p.update(0.016);
        assert!(p.pos.x.is_finite() && p.pos.y.is_finite(), "pos should not be NaN for zero-vel projectile");
    }

    #[test]
    fn enemy_projectile_has_longer_lifetime_than_player() {
        let player_p = Projectile::new(Vec2::ZERO, Vec2::new(0.0, -1.0), 500.0, 10.0, white(), Vec2::ZERO);
        let enemy_p = Projectile::new_enemy(Vec2::ZERO, Vec2::new(0.0, 1.0), white());
        assert!(enemy_p.lifetime > player_p.lifetime);
    }

    #[test]
    fn carrier_velocity_added_to_projectile_velocity() {
        let carrier = Vec2::new(200.0, 0.0);
        let p = Projectile::new(Vec2::ZERO, Vec2::new(0.0, -1.0), 600.0, 10.0, white(), carrier);
        assert!((p.vel.x - 200.0).abs() < 1e-4, "carrier vel x should be inherited");
        assert!((p.vel.y - (-600.0)).abs() < 1e-4, "proj speed y should be unchanged");
    }
}
