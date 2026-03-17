use macroquad::prelude::*;
use crate::input::InputState;
use crate::mobile::MobileOverlay;
use crate::player::Player;
use crate::projectile::Projectile;
use crate::world::World;

pub struct Game {
    player: Player,
    projectiles: Vec<Projectile>,
    camera: Camera2D,
    world: World,
    mobile: MobileOverlay,
}

impl Game {
    pub fn new() -> Self {
        Self {
            player: Player::new(Vec2::ZERO),
            projectiles: Vec::new(),
            camera: Camera2D {
                zoom: vec2(1.0 / 640.0, 1.0 / 360.0),
                ..Default::default()
            },
            world: World::new(),
            mobile: MobileOverlay::new(),
        }
    }

    pub fn update(&mut self) {
        let dt = get_frame_time();

        // Merge keyboard + touch input
        let kb = InputState::from_keyboard();
        let touch = self.mobile.update();
        let input = kb.merge(&touch);

        self.player.update(dt, &input, &mut self.projectiles);
        self.projectiles.retain_mut(|p| p.update(dt));
        self.world.update(self.player.pos, dt);

        // Camera tracks player
        let aspect = screen_width() / screen_height();
        let half_h = 360.0;
        self.camera.target = self.player.pos;
        self.camera.zoom = vec2(1.0 / (half_h * aspect), 1.0 / half_h);
    }

    pub fn draw(&self) {
        clear_background(Color::new(0.02, 0.02, 0.06, 1.0));

        set_camera(&self.camera);

        self.world.draw();

        for p in &self.projectiles {
            p.draw();
        }

        self.player.draw();

        set_default_camera();

        // Screen-space HUD + mobile overlay
        self.draw_hud();
        self.mobile.draw();
    }

    fn draw_hud(&self) {
        let pad = 12.0;
        let fs = 18.0;
        draw_text("STARSEEKER", pad, pad + fs, fs, SKYBLUE);
        draw_text(&format!("FPS: {}", get_fps()), pad, pad + fs * 2.4, 14.0, GRAY);

        // Planet approach prompt
        if let Some(name) = self.world.nearby_planet_name(self.player.pos) {
            let msg = format!("[E] Land on {}", name);
            let tw = measure_text(&msg, None, 18, 1.0).width;
            draw_text(
                &msg,
                screen_width() * 0.5 - tw * 0.5,
                screen_height() * 0.5 + 60.0,
                18.0,
                YELLOW,
            );
        }

        draw_text(
            "W/↑ Thrust  S/↓ Brake  A/D Rotate  Space Main  Ctrl/Z Aux",
            pad,
            screen_height() - pad,
            13.0,
            DARKGRAY,
        );
    }
}
