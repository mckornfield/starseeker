use macroquad::prelude::*;

mod entities;
mod game;
mod input;
mod mobile;
mod player;
mod projectile;
mod world;

#[macroquad::main("Starseeker")]
async fn main() {
    let mut game = game::Game::new();
    loop {
        game.update();
        game.draw();
        next_frame().await;
    }
}
