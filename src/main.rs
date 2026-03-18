use macroquad::prelude::*;

mod entities;
mod game;
mod input;
mod items;
mod missions;
mod mobile;
mod player;
mod projectile;
mod world;

#[macroquad::main("Starseeker")]
async fn main() {
    let screenshot_mode = std::env::args().any(|a| a == "--screenshot");
    let mut game = game::Game::new();
    // Start at interval so first screenshot fires on the first eligible frame
    let mut screenshot_timer: f32 = 5.0;
    loop {
        game.update();
        game.draw();
        if screenshot_mode {
            screenshot_timer += get_frame_time();
            if screenshot_timer >= 5.0 {
                screenshot_timer = 0.0;
                get_screen_data().export_png("/tmp/starseeker_screen.png");
            }
        }
        next_frame().await;
    }
}
