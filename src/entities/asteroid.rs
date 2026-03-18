use macroquad::prelude::*;

pub(crate) struct Asteroid {
    pub pos: Vec2,
    pub base_radius: f32,
    pub rotation: f32,
    pub rot_speed: f32,
    /// Per-vertex angle offset in local frame (evenly spaced + jitter)
    pub vertex_angles: Vec<f32>,
    /// Per-vertex radius multiplier (0.6–1.0)
    pub vertex_radii: Vec<f32>,
    pub color: Color,
}

impl Asteroid {
    pub fn update(&mut self, dt: f32) {
        self.rotation += self.rot_speed * dt;
    }

    pub fn draw(&self) {
        let n = self.vertex_angles.len();
        for i in 0..n {
            let a1 = self.vertex_angles[i] + self.rotation;
            let a2 = self.vertex_angles[(i + 1) % n] + self.rotation;
            let r1 = self.vertex_radii[i] * self.base_radius;
            let r2 = self.vertex_radii[(i + 1) % n] * self.base_radius;
            let p1 = self.pos + Vec2::new(a1.cos(), a1.sin()) * r1;
            let p2 = self.pos + Vec2::new(a2.cos(), a2.sin()) * r2;
            draw_line(p1.x, p1.y, p2.x, p2.y, 1.5, self.color);
        }
    }

    pub fn collision_radius(&self) -> f32 {
        self.base_radius * 0.85
    }
}
