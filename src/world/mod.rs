pub mod chunk;
pub mod gen;

use std::collections::HashMap;
use macroquad::prelude::Vec2;
use chunk::{Chunk, ChunkCoord};

pub struct World {
    chunks: HashMap<(i32, i32), Chunk>,
    player_chunk: (i32, i32),
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            player_chunk: (i32::MAX, i32::MAX), // force initial load
        };
        world.load_nearby(Vec2::ZERO);
        world.player_chunk = (0, 0);
        world
    }

    pub fn update(&mut self, player_pos: Vec2, dt: f32) {
        let coord = ChunkCoord::from_world_pos(player_pos);
        let current = (coord.cx, coord.cy);

        if current != self.player_chunk {
            self.player_chunk = current;
            self.load_nearby(player_pos);
            self.unload_distant();
        }

        for chunk in self.chunks.values_mut() {
            chunk.update(dt);
        }
    }

    pub fn draw(&self) {
        // Background blobs first so stars render on top
        for chunk in self.chunks.values() {
            chunk.draw_background();
        }
        for chunk in self.chunks.values() {
            chunk.draw_stars();
        }
        for chunk in self.chunks.values() {
            chunk.draw_content();
        }
    }

    /// Returns the name of any planet the player is approaching.
    pub fn nearby_planet_name(&self, player_pos: Vec2) -> Option<&str> {
        for chunk in self.chunks.values() {
            if let Some(planet) = &chunk.planet {
                if planet.is_in_range(player_pos) {
                    return Some(&planet.name);
                }
            }
        }
        None
    }

    fn load_nearby(&mut self, player_pos: Vec2) {
        let coord = ChunkCoord::from_world_pos(player_pos);
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let key = (coord.cx + dx, coord.cy + dy);
                self.chunks
                    .entry(key)
                    .or_insert_with(|| gen::gen_chunk(key.0, key.1));
            }
        }
    }

    fn unload_distant(&mut self) {
        let (pcx, pcy) = self.player_chunk;
        self.chunks
            .retain(|&(cx, cy), _| (cx - pcx).abs() <= 3 && (cy - pcy).abs() <= 3);
    }
}
