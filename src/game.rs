use macroquad::prelude::*;
use crate::player::Player;
use crate::projectile::Projectile;

pub struct Game {
    player: Player,
    projectiles: Vec<Projectile>,
    camera: Camera2D,
    stars: Vec<Vec2>,   // static background star field
}

impl Game {
    pub fn new() -> Self {
        // Seed a fixed star field around origin for Phase 1
        let mut stars = Vec::with_capacity(200);
        // Simple deterministic spread — real generation per chunk in Phase 2
        for i in 0..200u32 {
            let angle = (i as f32 * 137.508f32).to_radians(); // golden angle
            let radius = (i as f32 * 30.0).sqrt() * 80.0;
            stars.push(Vec2::new(angle.cos() * radius, angle.sin() * radius));
        }

        Self {
            player: Player::new(Vec2::ZERO),
            projectiles: Vec::new(),
            camera: Camera2D {
                zoom: vec2(1.0 / 640.0, 1.0 / 360.0),
                ..Default::default()
            },
            stars,
        }
    }

    pub fn update(&mut self) {
        let dt = get_frame_time();

        self.player.update(dt, &mut self.projectiles);

        // Update projectiles; remove dead ones
        self.projectiles.retain_mut(|p| p.update(dt));

        // Track camera to player
        self.camera.target = self.player.pos;

        // Adjust zoom to keep a consistent world-space viewport
        let aspect = screen_width() / screen_height();
        let half_h = 360.0;
        self.camera.zoom = vec2(1.0 / (half_h * aspect), 1.0 / half_h);
    }

    pub fn draw(&self) {
        clear_background(Color::new(0.02, 0.02, 0.06, 1.0));

        set_camera(&self.camera);

        // Star field
        for star in &self.stars {
            draw_circle(star.x, star.y, 1.0, Color::new(1.0, 1.0, 1.0, 0.6));
        }

        // Projectiles
        for p in &self.projectiles {
            p.draw();
        }

        // Player
        self.player.draw();

        set_default_camera();

        // HUD (screen space)
        self.draw_hud();
    }

    fn draw_hud(&self) {
        let pad = 12.0;
        let font_size = 18.0;
        draw_text("STARSEEKER", pad, pad + font_size, font_size, SKYBLUE);
        draw_text(
            &format!("FPS: {}", get_fps()),
            pad,
            pad + font_size * 2.5,
            14.0,
            GRAY,
        );
        draw_text(
            "W/↑ Thrust   S/↓ Brake   A/D Rotate   Space Main   Ctrl/Z Aux",
            pad,
            screen_height() - pad,
            13.0,
            DARKGRAY,
        );
    }
}
