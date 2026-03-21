pub(crate) mod chunk;
pub(crate) mod gen;

use crate::entities::asteroid::Asteroid;
use crate::entities::enemy::EnemyArchetype;
use chunk::{Chunk, ChunkCoord, ChunkType, CHUNK_SIZE};
use macroquad::prelude::*;
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

        // Collect all planet positions + radii for gravity pass
        let planets: Vec<(Vec2, f32)> = self
            .chunks
            .values()
            .filter_map(|c| c.planet.as_ref().map(|p| (p.pos, p.radius)))
            .collect();

        for chunk in self.chunks.values_mut() {
            chunk.update(dt);
            // Apply planet gravity to each asteroid
            if !planets.is_empty() {
                for asteroid in &mut chunk.asteroids {
                    for &(planet_pos, planet_radius) in &planets {
                        let to_planet = planet_pos - asteroid.pos;
                        let dist = to_planet.length();
                        // Only pull within range; avoid singularity inside planet
                        if dist < 1400.0 && dist > planet_radius {
                            let acc = 4_000.0 * planet_radius / (dist * dist);
                            asteroid.vel += to_planet.normalize() * acc * dt;
                        }
                    }
                }
            }
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

    /// Hit-test a projectile against all asteroids.
    /// Removes the first hit and returns Some((pos, base_radius, color)) for fragmentation.
    pub fn remove_asteroid_hit(&mut self, pos: Vec2, radius: f32) -> Option<(Vec2, f32, Color)> {
        for chunk in self.chunks.values_mut() {
            if let Some(idx) = chunk
                .asteroids
                .iter()
                .position(|a| a.pos.distance(pos) < a.collision_radius() + radius)
            {
                let a = chunk.asteroids.swap_remove(idx);
                let info = (a.pos, a.base_radius, a.color);
                // Spawn child fragments if large enough
                if a.base_radius > 18.0 {
                    let child_radius = a.base_radius * 0.45;
                    let count = if a.base_radius > 40.0 { 3 } else { 2 };
                    // Explosion speed scales with parent size
                    let blast_speed = 40.0 + a.base_radius * 1.5;
                    for i in 0..count {
                        let angle = i as f32 * std::f32::consts::TAU / count as f32
                            + quad_rand::gen_range(0.0_f32, 1.0);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        let offset = dir * a.base_radius * 0.6;
                        // Outward velocity + inherit parent drift
                        let frag_vel = a.vel + dir * blast_speed;
                        chunk.asteroids.push(Asteroid::new_fragment(
                            a.pos + offset,
                            child_radius,
                            a.color,
                            frag_vel,
                        ));
                    }
                }
                return Some(info);
            }
        }
        None
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

    /// Collect names of all planets in currently loaded chunks.
    pub fn known_planet_names(&self) -> Vec<String> {
        self.chunks
            .values()
            .filter_map(|c| c.planet.as_ref().map(|p| p.name.clone()))
            .collect()
    }

    /// Data for drawing the map: planet (pos, radius, color, name) for each loaded chunk.
    pub fn map_planets(&self) -> Vec<(Vec2, f32, Color, &str)> {
        self.chunks
            .values()
            .filter_map(|c| {
                c.planet
                    .as_ref()
                    .map(|p| (p.pos, p.radius, p.color, p.name.as_str()))
            })
            .collect()
    }

    /// Chunk grid info for the map: (chunk_origin, chunk_type_color) for each loaded chunk.
    pub fn map_chunks(&self) -> Vec<(Vec2, Color)> {
        self.chunks
            .values()
            .map(|c| {
                let origin = Vec2::new(
                    c.coord.cx as f32 * CHUNK_SIZE,
                    c.coord.cy as f32 * CHUNK_SIZE,
                );
                let color = match &c.chunk_type {
                    ChunkType::DeepSpace => Color::new(0.1, 0.1, 0.15, 0.3),
                    ChunkType::Nebula { tint } => {
                        Color::new(tint.r, tint.g, tint.b, 0.2)
                    }
                    ChunkType::Derelict => Color::new(0.5, 0.3, 0.1, 0.25),
                    ChunkType::HasPlanet => Color::new(0.15, 0.25, 0.4, 0.3),
                };
                (origin, color)
            })
            .collect()
    }

    fn unload_distant(&mut self) {
        let (pcx, pcy) = self.player_chunk;
        self.chunks
            .retain(|&(cx, cy), _| (cx - pcx).abs() <= 3 && (cy - pcy).abs() <= 3);
    }
}
