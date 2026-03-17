use macroquad::prelude::*;
use crate::entities::asteroid::Asteroid;
use crate::entities::enemy::EnemyArchetype;
use crate::entities::planet::Planet;

pub const CHUNK_SIZE: f32 = 3200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub cx: i32,
    pub cy: i32,
}

impl ChunkCoord {
    pub fn from_world_pos(pos: Vec2) -> Self {
        Self {
            cx: (pos.x / CHUNK_SIZE).floor() as i32,
            cy: (pos.y / CHUNK_SIZE).floor() as i32,
        }
    }
}

pub struct StarPoint {
    pub pos: Vec2,
    pub brightness: f32,
    pub size: f32,
}

pub enum ChunkType {
    DeepSpace,
    Nebula { tint: Color },
    Derelict,
    HasPlanet,
}

pub struct Chunk {
    pub coord: ChunkCoord,
    pub hostility: f32,
    pub chunk_type: ChunkType,
    pub stars: Vec<StarPoint>,
    /// Nebula atmosphere blobs: (world_pos, radius, color)
    pub bg_blobs: Vec<(Vec2, f32, Color)>,
    pub asteroids: Vec<Asteroid>,
    pub planet: Option<Planet>,
    /// Enemy spawn specs — drained once by the World into its spawn queue
    pub enemy_spawns: Vec<(Vec2, EnemyArchetype)>,
}

impl Chunk {
    pub fn update(&mut self, dt: f32) {
        for a in &mut self.asteroids {
            a.update(dt);
        }
    }

    pub fn draw_background(&self) {
        for &(pos, radius, color) in &self.bg_blobs {
            draw_circle(pos.x, pos.y, radius, color);
        }
    }

    pub fn draw_stars(&self) {
        for s in &self.stars {
            draw_circle(s.pos.x, s.pos.y, s.size, Color::new(1.0, 1.0, 1.0, s.brightness));
        }
    }

    pub fn draw_content(&self) {
        for a in &self.asteroids {
            a.draw();
        }
        if let Some(planet) = &self.planet {
            planet.draw();
        }
    }
}
