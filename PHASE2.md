# Phase 2: Infinite World

## Goal
Replace the static star field with a procedurally generated, infinite seamless world
the player can fly through indefinitely. Lay the groundwork for combat in Phase 3.

---

## World Model

### Chunks
- World divided into fixed-size chunks: **3200 × 3200 world units**
- A chunk is identified by integer grid coordinates `(cx, cy)`
- Chunk `(cx, cy)` covers world rect `[cx*SIZE .. (cx+1)*SIZE, cy*SIZE .. (cy+1)*SIZE]`
- Deterministic per-chunk seed: `hash(cx, cy, world_seed)` → reproducible content
- At any time, keep the **3×3 neighbourhood** around the player's chunk loaded (9 chunks max)
- Unload chunks that fall outside a **5×5** keep-alive radius to allow re-entry caching

### Chunk Classification (from seed)
| Type | Condition | Visual Tone |
|------|-----------|-------------|
| Deep Space | default | dark blue, sparse stars |
| Nebula | hostility > 0.6 | tinted purple/red background, denser particles |
| Planet | special roll ~8% | large circle landmark, landing ring |
| Derelict | special roll ~4% | wreckage shapes, high loot density |

Hostility score (0–1) drives enemy count and tier in Phase 3.

---

## Contents per Chunk

### Asteroids
- Count: `4 + (hostility * 8)` rounded, plus ±2 random variance
- Radius: 12–60 world units (uniform random)
- Position: random within chunk bounds, minimum spacing enforced
- Rotation: random angular velocity (visual only for now, no collision mass)
- Shape: drawn as irregular polygons (6–10 vertices with radius noise)

### Planet Markers (Planet chunks only)
- Single large circle (radius 120–200) near chunk center
- Colored ring indicating "landable" (white pulse ring)
- Name generated from seed (syllable table, e.g. "Veltara", "Koss IV")
- Approach within 180 units → show "[E] Land" prompt

### Background
- Star field generated per chunk (40–80 stars, varying brightness)
- Nebula chunks: additional 20–30 large low-alpha colored circles for atmosphere

---

## Implementation Plan

### New files
```
src/world/
    mod.rs       — ChunkCoord, World struct, load/unload logic
    chunk.rs     — Chunk struct (asteroids, planets, stars)
    gen.rs       — Deterministic generation functions
src/entities/
    asteroid.rs  — Asteroid struct (pos, radius, rotation, rot_speed, verts)
    planet.rs    — Planet struct (pos, radius, name, color)
```

### Changes to existing files
- `game.rs` — replace static `stars: Vec<Vec2>` with `World`; call `world.update(player_pos)` each frame
- `game.rs` draw — iterate loaded chunks, draw their contents
- Camera zoom stays the same (360-unit half-height)

### Generation algorithm (gen.rs)
```
fn chunk_seed(world_seed: u64, cx: i32, cy: i32) -> u64
fn gen_chunk(seed: u64, cx: i32, cy: i32) -> Chunk
    → classify type
    → gen_stars(seed)
    → gen_asteroids(seed, hostility)
    → optionally gen_planet(seed)
```
Use `quad-rand` seeded per chunk (save/restore RNG state around generation).

### Collision (asteroid only, Phase 2 scope)
- Circle vs circle: player radius ~12, asteroid radius as stored
- On collision: player takes 5 damage (Phase 3 health system), asteroid destroyed
- Keep it simple — no asteroid splitting yet (classic Asteroids split is Phase 3+)

---

## Out of Scope for Phase 2
- Enemies (Phase 3)
- Planet landing/shop (Phase 5)
- Loot drops (Phase 4)
- Asteroid splitting
- Minimap (can add at end of Phase 2 as stretch)
