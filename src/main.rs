use macroquad::prelude::*;

mod game;
mod player;
mod projectile;

#[macroquad::main("Starseeker")]
async fn main() {
    let mut game = game::Game::new();
    loop {
        game.update();
        game.draw();
        next_frame().await;
    }
}
