use crate::entities::asteroid::Asteroid;
use crate::entities::enemy::EnemyArchetype;
use crate::entities::planet::Planet;
use macroquad::prelude::*;

pub(crate) const CHUNK_SIZE: f32 = 3200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ChunkCoord {
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

pub(crate) struct StarPoint {
    pub pos: Vec2,
    pub brightness: f32,
    pub size: f32,
}

pub(crate) enum ChunkType {
    DeepSpace,
    Nebula { tint: Color },
    Derelict,
    HasPlanet,
}

pub(crate) struct Chunk {
    /// Retained for debugging/future use (planet shops will key on hostility).
    #[allow(dead_code)]
    pub coord: ChunkCoord,
    #[allow(dead_code)]
    pub hostility: f32,
    #[allow(dead_code)]
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
            draw_circle(
                s.pos.x,
                s.pos.y,
                s.size,
                Color::new(1.0, 1.0, 1.0, s.brightness),
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_coord_origin() {
        let c = ChunkCoord::from_world_pos(Vec2::new(0.0, 0.0));
        assert_eq!(c.cx, 0);
        assert_eq!(c.cy, 0);
    }

    #[test]
    fn chunk_coord_positive() {
        let c = ChunkCoord::from_world_pos(Vec2::new(CHUNK_SIZE * 2.5, CHUNK_SIZE * 1.1));
        assert_eq!(c.cx, 2);
        assert_eq!(c.cy, 1);
    }

    #[test]
    fn chunk_coord_negative() {
        let c = ChunkCoord::from_world_pos(Vec2::new(-1.0, -1.0));
        assert_eq!(c.cx, -1);
        assert_eq!(c.cy, -1);
    }

    #[test]
    fn chunk_coord_exact_boundary() {
        let c = ChunkCoord::from_world_pos(Vec2::new(CHUNK_SIZE, CHUNK_SIZE));
        assert_eq!(c.cx, 1);
        assert_eq!(c.cy, 1);
    }

    #[test]
    fn chunk_coord_just_before_boundary() {
        let c = ChunkCoord::from_world_pos(Vec2::new(CHUNK_SIZE - 0.01, 0.0));
        assert_eq!(c.cx, 0);
    }
}
