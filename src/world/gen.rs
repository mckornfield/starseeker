use super::chunk::{Chunk, ChunkCoord, ChunkType, StarPoint, CHUNK_SIZE};
use crate::entities::asteroid::Asteroid;
use crate::entities::enemy::EnemyArchetype;
use crate::entities::planet::Planet;
use macroquad::prelude::*;

const WORLD_SEED: u64 = 0xdeadbeef_cafebabe;

// ── Deterministic per-chunk RNG (xorshift64 + splitmix64 seed mixing) ─────────

struct ChunkRng(u64);

impl ChunkRng {
    fn new(cx: i32, cy: i32) -> Self {
        let mut s = WORLD_SEED;
        s ^= (cx as i64 as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
        s ^= (cy as i64 as u64)
            .wrapping_mul(0x6c62_272e_07bb_0142)
            .rotate_left(32);
        // splitmix64 finalizer
        s ^= s >> 30;
        s = s.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        s ^= s >> 27;
        s = s.wrapping_mul(0x94d0_49bb_1331_11eb);
        s ^= s >> 31;
        if s == 0 {
            s = 1;
        }
        Self(s)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn f32(&mut self) -> f32 {
        (self.next() >> 11) as f32 / (1u64 << 53) as f32
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.f32() * (max - min)
    }

    fn range_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next() % max as u64) as usize
    }

    fn p(&mut self, probability: f32) -> bool {
        self.f32() < probability
    }
}

// ── Public entry point ─────────────────────────────────────────────────────────

pub(crate) fn gen_chunk(cx: i32, cy: i32) -> Chunk {
    let mut rng = ChunkRng::new(cx, cy);
    let origin = Vec2::new(cx as f32 * CHUNK_SIZE, cy as f32 * CHUNK_SIZE);

    // Classify chunk
    let roll = rng.f32();
    let (chunk_type, planet) = if roll < 0.15 {
        let p = gen_planet(&mut rng, origin);
        (ChunkType::HasPlanet, Some(p))
    } else if roll < 0.19 {
        (ChunkType::Derelict, None)
    } else if roll < 0.45 {
        let tint = gen_nebula_tint(&mut rng);
        (ChunkType::Nebula { tint }, None)
    } else {
        (ChunkType::DeepSpace, None)
    };

    let hostility = match &chunk_type {
        ChunkType::Derelict => rng.range_f32(0.7, 1.0),
        ChunkType::Nebula { .. } => rng.range_f32(0.4, 0.9),
        ChunkType::HasPlanet => rng.range_f32(0.0, 0.25),
        ChunkType::DeepSpace => rng.range_f32(0.1, 0.7),
    };

    let stars = gen_stars(&mut rng, origin, &chunk_type);
    let bg_blobs = gen_bg_blobs(&mut rng, origin, &chunk_type);
    let asteroids = gen_asteroids(&mut rng, origin, hostility, &chunk_type);
    let enemy_spawns = gen_enemy_spawns(&mut rng, origin, hostility, &chunk_type);

    Chunk {
        coord: ChunkCoord { cx, cy },
        hostility,
        chunk_type,
        stars,
        bg_blobs,
        asteroids,
        planet,
        enemy_spawns,
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn gen_nebula_tint(rng: &mut ChunkRng) -> Color {
    const PALETTES: &[(f32, f32, f32)] = &[
        (0.55, 0.15, 0.80), // purple
        (0.15, 0.35, 0.90), // blue
        (0.85, 0.25, 0.20), // red
        (0.15, 0.75, 0.50), // teal
        (0.80, 0.60, 0.15), // amber
    ];
    let (r, g, b) = PALETTES[rng.range_usize(PALETTES.len())];
    Color::new(r, g, b, 1.0)
}

fn gen_bg_blobs(
    rng: &mut ChunkRng,
    origin: Vec2,
    chunk_type: &ChunkType,
) -> Vec<(Vec2, f32, Color)> {
    let tint = match chunk_type {
        ChunkType::Nebula { tint } => *tint,
        ChunkType::Derelict => Color::new(0.5, 0.3, 0.1, 1.0),
        _ => return vec![],
    };
    let count = rng.range_usize(4) + 3; // 3–6 blobs
    (0..count)
        .map(|_| {
            let pos = origin
                + Vec2::new(
                    rng.range_f32(0.0, CHUNK_SIZE),
                    rng.range_f32(0.0, CHUNK_SIZE),
                );
            let radius = rng.range_f32(CHUNK_SIZE * 0.15, CHUNK_SIZE * 0.45);
            let alpha = rng.range_f32(0.03, 0.07);
            (pos, radius, Color::new(tint.r, tint.g, tint.b, alpha))
        })
        .collect()
}

fn gen_stars(rng: &mut ChunkRng, origin: Vec2, chunk_type: &ChunkType) -> Vec<StarPoint> {
    let count = match chunk_type {
        ChunkType::Nebula { .. } => rng.range_usize(25) + 45, // 45-69
        ChunkType::DeepSpace => rng.range_usize(25) + 25,     // 25-49
        _ => rng.range_usize(20) + 15,                        // 15-34
    };
    (0..count)
        .map(|_| StarPoint {
            pos: origin
                + Vec2::new(
                    rng.range_f32(0.0, CHUNK_SIZE),
                    rng.range_f32(0.0, CHUNK_SIZE),
                ),
            brightness: rng.range_f32(0.25, 1.0),
            size: rng.range_f32(0.5, 2.2),
        })
        .collect()
}

fn gen_asteroids(
    rng: &mut ChunkRng,
    origin: Vec2,
    hostility: f32,
    chunk_type: &ChunkType,
) -> Vec<Asteroid> {
    let count = match chunk_type {
        ChunkType::HasPlanet => (rng.range_f32(1.0, 5.0)) as usize,
        _ => {
            let base = 4.0 + hostility * 9.0;
            let variance = rng.range_f32(-2.0, 2.0);
            ((base + variance).max(1.0)) as usize
        }
    };
    (0..count).map(|_| gen_asteroid(rng, origin)).collect()
}

fn gen_asteroid(rng: &mut ChunkRng, origin: Vec2) -> Asteroid {
    let pos = origin
        + Vec2::new(
            rng.range_f32(80.0, CHUNK_SIZE - 80.0),
            rng.range_f32(80.0, CHUNK_SIZE - 80.0),
        );
    let base_radius = rng.range_f32(12.0, 62.0);
    let rot_speed = rng.range_f32(-0.6, 0.6);
    let rotation = rng.range_f32(0.0, std::f32::consts::TAU);

    let n_verts = rng.range_usize(5) + 6; // 6–10
    let mut vertex_angles = Vec::with_capacity(n_verts);
    let mut vertex_radii = Vec::with_capacity(n_verts);
    for i in 0..n_verts {
        let base_angle = i as f32 * std::f32::consts::TAU / n_verts as f32;
        vertex_angles.push(base_angle + rng.range_f32(-0.25, 0.25));
        vertex_radii.push(rng.range_f32(0.55, 1.0));
    }

    let v = rng.range_f32(0.45, 0.78);
    let color = Color::new(
        v * rng.range_f32(0.9, 1.1).min(1.0),
        v * rng.range_f32(0.85, 1.0),
        v * rng.range_f32(0.75, 0.95),
        1.0,
    );

    Asteroid {
        pos,
        base_radius,
        rotation,
        rot_speed,
        vertex_angles,
        vertex_radii,
        color,
    }
}

fn gen_planet(rng: &mut ChunkRng, origin: Vec2) -> Planet {
    let pos = origin
        + Vec2::new(
            rng.range_f32(500.0, CHUNK_SIZE - 500.0),
            rng.range_f32(500.0, CHUNK_SIZE - 500.0),
        );
    let radius = rng.range_f32(110.0, 220.0);

    const PLANET_COLORS: &[(f32, f32, f32)] = &[
        (0.25, 0.45, 0.90), // blue
        (0.65, 0.38, 0.18), // orange/brown
        (0.35, 0.68, 0.38), // green
        (0.68, 0.68, 0.48), // tan
        (0.48, 0.28, 0.72), // purple
        (0.75, 0.45, 0.45), // rose
    ];
    let (r, g, b) = PLANET_COLORS[rng.range_usize(PLANET_COLORS.len())];
    let color = Color::new(r, g, b, 1.0);
    let name = gen_planet_name(rng);

    Planet {
        pos,
        radius,
        name,
        color,
    }
}

fn gen_planet_name(rng: &mut ChunkRng) -> String {
    const SYLS: &[&str] = &[
        "vel", "tar", "koss", "mir", "eth", "zar", "dun", "fel", "mor", "ael", "vex", "tor", "yss",
        "nox", "dra", "kel", "sor", "ith",
    ];
    const SUFS: &[&str] = &[
        "a", "is", "us", "on", "ix", "ar", "en", "ia", "or", "um", "ax", "ys",
    ];

    let s1 = SYLS[rng.range_usize(SYLS.len())];
    let s2 = SUFS[rng.range_usize(SUFS.len())];

    let first = s1.chars().next().unwrap().to_ascii_uppercase();
    let base = format!("{}{}{}", first, &s1[1..], s2);

    if rng.p(0.35) {
        format!("{} {}", base, rng.range_usize(6) + 2)
    } else {
        base
    }
}

fn gen_enemy_spawns(
    rng: &mut ChunkRng,
    origin: Vec2,
    hostility: f32,
    chunk_type: &ChunkType,
) -> Vec<(Vec2, EnemyArchetype)> {
    // No enemies near planets; derelicts are extra dangerous
    let count = match chunk_type {
        ChunkType::HasPlanet => return vec![],
        ChunkType::Derelict => (rng.range_f32(6.0, 12.0)) as usize,
        _ => {
            if hostility < 0.2 {
                return vec![];
            }
            (hostility * 10.0 + rng.range_f32(-1.0, 2.0)).max(0.0) as usize
        }
    };

    // Pick 1-3 cluster centers within the chunk interior, then scatter enemies
    // within a tight radius around each center for a pack feel.
    let cluster_count = rng.range_usize(3).max(1); // 1-3 clusters
    let clusters: Vec<Vec2> = (0..cluster_count)
        .map(|_| {
            origin
                + Vec2::new(
                    rng.range_f32(300.0, CHUNK_SIZE - 300.0),
                    rng.range_f32(300.0, CHUNK_SIZE - 300.0),
                )
        })
        .collect();

    (0..count)
        .map(|i| {
            let center = clusters[i % cluster_count];
            let angle = rng.range_f32(0.0, std::f32::consts::TAU);
            let radius = rng.range_f32(0.0, 220.0);
            let pos = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
            let archetype = match rng.range_usize(3) {
                0 => EnemyArchetype::Tank,
                1 => EnemyArchetype::Agile,
                2 => EnemyArchetype::Ranged,
                _ => unreachable!("range_usize(3) only yields 0, 1, or 2"),
            };
            (pos, archetype)
        })
        .collect()
}
