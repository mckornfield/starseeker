use macroquad::prelude::*;

pub const LAND_RANGE: f32 = 180.0;

pub struct Planet {
    pub pos: Vec2,
    pub radius: f32,
    pub name: String,
    pub color: Color,
}

impl Planet {
    pub fn draw(&self) {
        // Planet body
        draw_circle(self.pos.x, self.pos.y, self.radius, self.color);
        // Darker atmosphere ring
        draw_circle_lines(
            self.pos.x,
            self.pos.y,
            self.radius + 4.0,
            2.0,
            Color::new(self.color.r, self.color.g, self.color.b, 0.4),
        );
        // Subtle highlight
        draw_circle(
            self.pos.x - self.radius * 0.3,
            self.pos.y - self.radius * 0.3,
            self.radius * 0.25,
            Color::new(1.0, 1.0, 1.0, 0.08),
        );

        // Orbit ring (dashed look via thin circle)
        draw_circle_lines(
            self.pos.x,
            self.pos.y,
            self.radius + LAND_RANGE * 0.5,
            0.8,
            Color::new(1.0, 1.0, 1.0, 0.10),
        );

        // Name label
        let fs = 14.0_f32;
        let tw = measure_text(&self.name, None, fs as u16, 1.0).width;
        draw_text(
            &self.name,
            self.pos.x - tw * 0.5,
            self.pos.y + self.radius + 20.0,
            fs,
            Color::new(1.0, 1.0, 1.0, 0.7),
        );
    }

    pub fn is_in_range(&self, pos: Vec2) -> bool {
        self.pos.distance(pos) < self.radius + LAND_RANGE
    }
}
