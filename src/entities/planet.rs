use macroquad::prelude::*;

pub(crate) const LAND_RANGE: f32 = 180.0;

pub(crate) struct Planet {
    pub pos: Vec2,
    pub radius: f32,
    pub name: String,
    pub color: Color,
}

impl Planet {
    pub fn draw(&self) {
        // Planet body
        draw_circle(self.pos.x, self.pos.y, self.radius, self.color);
        // Dark terminator (shadow on one side)
        draw_circle(
            self.pos.x + self.radius * 0.18,
            self.pos.y + self.radius * 0.18,
            self.radius * 0.88,
            Color::new(0.0, 0.0, 0.0, 0.22),
        );
        // Thin bright rim
        draw_circle_lines(
            self.pos.x,
            self.pos.y,
            self.radius,
            1.5,
            Color::new(
                (self.color.r + 0.3).min(1.0),
                (self.color.g + 0.3).min(1.0),
                (self.color.b + 0.3).min(1.0),
                0.6,
            ),
        );
        // Approach range indicator (very faint)
        draw_circle_lines(
            self.pos.x,
            self.pos.y,
            self.radius + LAND_RANGE,
            0.7,
            Color::new(1.0, 1.0, 0.5, 0.08),
        );
        // Name label below planet
        let fs = 14.0_f32;
        let tw = measure_text(&self.name, None, fs as u16, 1.0).width;
        draw_text(
            &self.name,
            self.pos.x - tw * 0.5,
            self.pos.y + self.radius + 20.0,
            fs,
            Color::new(1.0, 1.0, 1.0, 0.65),
        );
    }

    pub fn is_in_range(&self, pos: Vec2) -> bool {
        self.pos.distance(pos) < self.radius + LAND_RANGE
    }
}
