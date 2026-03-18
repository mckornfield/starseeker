pub(crate) mod chunk;
pub(crate) mod gen;

use crate::entities::enemy::EnemyArchetype;
use chunk::{Chunk, ChunkCoord};
use macroquad::prelude::Vec2;
use std::collections::HashMap;

pub(crate) struct World {
    chunks: HashMap<(i32, i32), Chunk>,
    player_chunk: (i32, i32),
    /// New enemy spawns from freshly loaded chunks — drained by Game each frame.
    pub spawn_queue: Vec<(Vec2, EnemyArchetype)>,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            chunks: HashMap::new(),
            player_chunk: (i32::MAX, i32::MAX),
            spawn_queue: Vec::new(),
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

    /// Hit-test a projectile against all asteroids. Removes the first hit and returns true.
    pub fn remove_asteroid_hit(&mut self, pos: Vec2, radius: f32) -> bool {
        for chunk in self.chunks.values_mut() {
            if let Some(idx) = chunk
                .asteroids
                .iter()
                .position(|a| a.pos.distance(pos) < a.collision_radius() + radius)
            {
                chunk.asteroids.swap_remove(idx);
                return true;
            }
        }
        false
    }

    /// Returns true if the given circle overlaps any asteroid.
    pub fn overlaps_asteroid(&self, pos: Vec2, radius: f32) -> bool {
        for chunk in self.chunks.values() {
            for a in &chunk.asteroids {
                if a.pos.distance(pos) < a.collision_radius() + radius {
                    return true;
                }
            }
        }
        false
    }

    fn load_nearby(&mut self, player_pos: Vec2) {
        let coord = ChunkCoord::from_world_pos(player_pos);
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let key = (coord.cx + dx, coord.cy + dy);
                if !self.chunks.contains_key(&key) {
                    let mut chunk = gen::gen_chunk(key.0, key.1);
                    // Drain enemy spawns into queue
                    self.spawn_queue.append(&mut chunk.enemy_spawns);
                    self.chunks.insert(key, chunk);
                }
            }
        }
    }

    fn unload_distant(&mut self) {
        let (pcx, pcy) = self.player_chunk;
        self.chunks
            .retain(|&(cx, cy), _| (cx - pcx).abs() <= 3 && (cy - pcy).abs() <= 3);
    }
}
