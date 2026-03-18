# Starseeker — Rust Technical Audit

**Audited:** 2026-03-17
**Auditor:** Automated structural review (all `.rs` sources read, Cargo.toml and CI inspected, `cargo clippy --all-targets` executed)
**Codebase snapshot:** commit `bbdd758` — Phase 3, ~1 200 lines of Rust across 19 source files

---

## 1) Architecture & Crate Structure — Scalability Challenges

### Workspace & Modularity

The project is a single binary crate with no workspace. The module tree is flat relative to `src/`:

```
main.rs
game.rs          ← 406 lines — god-struct orchestrator
player.rs
projectile.rs
input.rs
mobile.rs
entities/
    asteroid.rs
    enemy.rs
    loot.rs
    planet.rs
world/
    chunk.rs
    gen.rs
    mod.rs
items/
    loadout.rs
    rarity.rs
    thruster.rs
    weapon.rs
```

The structure is reasonable for the current Phase-3 scope. `entities/`, `items/`, and `world/` are proper sub-modules. However every sub-module re-exports everything as `pub`, creating a completely open surface even though nothing outside the binary is ever a consumer. There is no workspace, meaning adding a `starseeker-server`, `starseeker-editor`, or fuzz harness in future would require splitting the crate from scratch.

**`game.rs` is a monolith.** At 406 lines it owns: the main `Game` struct with 10 fields, update logic for every subsystem (input, world, six separate collision phases, loot pickup, camera), and two drawing methods plus a HUD. This single file handles physics, AI scheduling, item management, camera math, and UI rendering — responsibilities that should be distributed into systems or at least separate `impl` blocks in separate files.

The `spawn_queue` field on `World` (`world/mod.rs:13`) is a `pub` field that acts as a message bus between `World` and `Game`. This is an architectural smell: it leaks internal world state into the caller and prevents `World` from controlling the lifecycle of its own spawns.

### Dependency Injection

All configuration is hardcoded as module-level `const` values scattered across source files:

- `game.rs:11–16` — `PLAYER_RADIUS`, `PLAYER_MAX_HEALTH`, `ASTEROID_HIT_IFRAMES`, `ENEMY_CULL_DIST`
- `player.rs:6–9` — `ROTATION_SPEED`, `BASE_THRUST`, `DRAG`, `BASE_MAX_SPEED`
- `projectile.rs:3–4` — `LIFETIME`, `RADIUS`
- `entities/enemy.rs:4` — `DETECT_RANGE`
- `world/chunk.rs:6` — `CHUNK_SIZE`
- `world/gen.rs:7` — `WORLD_SEED`

There is no configuration file, no `GameConfig` struct, no builder pattern, and no trait abstraction over any subsystem. This means tuning a single value requires a recompile and there is no path to data-driven content.

Concrete types are used everywhere. `Enemy::update` takes a `&mut Vec<Projectile>` directly (`enemy.rs:57`), coupling the enemy AI to the exact projectile container type. A trait (`ProjectileEmitter` or similar) would decouple this.

---

## 2) Error Handling — Debugging & Reliability Risk

### Error Type Design

The project uses **zero** dedicated error types. There is no `thiserror`, no `anyhow`, no `Result`-returning functions anywhere in the codebase. Every operation either succeeds silently or panics.

### `unwrap()` / `expect()` Usage in Non-Test Code

| File | Line | Call | Risk |
|---|---|---|---|
| `src/world/gen.rs` | 242 | `.unwrap()` | `s1.chars().next()` on a `&str` from a `const` slice that is never empty — safe today, fragile on any future edit to `SYLS` that accidentally includes an empty string |
| `src/items/loadout.rs` | 25 | `.unwrap_or(true)` | Not a panic risk, but obscures the logic |
| `src/items/loadout.rs` | 38 | `.unwrap_or(true)` | Same as above |

The `gen.rs:242` unwrap deserves specific attention: `SYLS` is a hard-coded `const` slice and the call is `rng.range_usize(SYLS.len())` followed by `SYLS[idx].chars().next().unwrap()`. The invariant that a syllable is never empty is nowhere documented or asserted.

### Silent Error Swallowing

The entire game loop in `main.rs` is:

```rust
loop {
    game.update();
    game.draw();
    next_frame().await;
}
```

If `update()` or `draw()` panics, the WASM target will trap silently in the browser with no user-facing error message and no crash reporting. On native, stderr will receive a panic message but there is no `#[panic_handler]` customisation or logging integration.

`Player::update` (`player.rs:61`) calls `self.vel.normalize()` only when `self.vel.length() > max_speed` (which implies length > 0), so there is no NaN hazard there. However `projectile.rs:53` calls `self.vel.normalize()` unconditionally during `draw`:

```rust
let trail = self.pos - self.vel.normalize() * 6.0;
```

If a projectile is spawned with zero velocity (e.g., `Projectile::new_enemy` called with a zero direction vector), `normalize()` on a zero vector returns `Vec2::NAN` in `glam`, silently corrupting the draw position for that projectile. There is no guard and no documentation of the precondition.

### Missing Error Context

No structured context is attached to any failure path. When the game eventually grows save/load or asset loading, there will be no established pattern for propagating errors with context.

---

## 3) Type Safety & Correctness — Maintenance Deficits

### Primitive Obsession

The codebase uses raw `f32` for every domain quantity without newtype wrappers:

| Domain Concept | Represented As | Risk |
|---|---|---|
| Health points | `f32` (player `game.rs:20`, enemy `enemy.rs:17–18`) | Can silently become negative with no enforcement |
| Damage | `f32` (`projectile.rs:18`, `enemy.rs:53`) | Dimensionally identical to health — addition is silently legal |
| Speed (units/sec) | `f32` | Indistinguishable from acceleration or distance |
| Cooldown (seconds) | `f32` | Indistinguishable from lifetime |
| World position | `Vec2` | No coordinate-space tagging (world vs screen) |
| Rotation | `f32` radians | No `Radians` newtype to prevent degrees confusion |

The collision radius for each `EnemyArchetype` is duplicated in two places with different literal values:
- `enemy.rs` draw methods: Tank radius `18.0` (`draw_tank`, line 147), Ranged `13.0` (`draw_ranged`, line 173), Agile `10.0` (`draw_agile`, line 163)
- `game.rs:129–133`: hit-test radii `18.0` / `10.0` / `13.0` in a `match` block

There is no single source of truth for these radii. They will diverge.

### Floating-Point Equality for Game Logic

`player.rs:71` and `player.rs:88` gate firing on exact float equality:

```rust
if input.fire_main && self.main_cooldown == 0.0 {
```

`enemy.rs:75` and `enemy.rs:105` do the same for `fire_cooldown == 0.0`. Because cooldowns are clamped to `0.0` via `.max(0.0)`, this is *currently* safe, but it is a fragile pattern. Any change to the cooldown computation that produces a subnormal instead of exact `0.0` (e.g., introducing a frame-rate-dependent reset) would silently break firing.

### Exhaustive Matching / Catch-All Arms

`world/gen.rs:277–281` maps `rng.range_usize(3)` to archetypes:

```rust
match rng.range_usize(3) {
    0 => EnemyArchetype::Tank,
    1 => EnemyArchetype::Agile,
    _ => EnemyArchetype::Ranged,
```

The wildcard `_` arm means adding a fourth `EnemyArchetype` variant will not produce a compile error here; the new archetype will silently resolve to `Ranged`. The match should use exhaustive integer literals `2 =>` with `_ => unreachable!()`.

`world/gen.rs:101` uses a `_` arm for HSV sectoring in `hsv_to_color` (`items/weapon.rs:107`) which covers `5` — this is correct but is written as a silent catch-all rather than `5 =>`.

### `unsafe` Usage

No `unsafe` blocks are present in any source file. This is appropriate for a macroquad game.

---

## 4) Async & Concurrency — Safety & Performance Risk

### The macroquad Async Model

macroquad uses a cooperative async runtime where `next_frame().await` is the single yield point. The game loop in `main.rs` is effectively a synchronous loop:

```rust
loop {
    game.update();  // blocking sync
    game.draw();    // blocking sync
    next_frame().await;
}
```

`game.update()` and `game.draw()` are fully synchronous. There is no tokio, no rayon, no `std::thread`, and no `Arc`/`Mutex` usage. This is the correct approach for a single-threaded WASM game and carries no concurrency hazard.

**Implication for future scaling:** If the game ever needs background asset loading, world streaming off the main thread, or a save system, there is no established async pattern in the codebase to build on. The `async fn main` boundary is the only async surface; adding a blocking call inside `update()` (e.g., file I/O) would stall the entire render loop.

### Blocking in Async Contexts

`game.update()` calls `self.world.update()` which may call `gen::gen_chunk()` synchronously when the player crosses a chunk boundary (`world/mod.rs:34–36`). Chunk generation involves multiple Vec allocations and O(n) loops. This is a frame-time spike risk (see Section 9) but not a concurrency problem in the current single-threaded model.

### Shared State

All state is owned by the `Game` struct and accessed via `&mut self`. There is no shared mutable state. The `spawn_queue` field on `World` (`world/mod.rs:13`) is `pub` and drained directly by `Game` each frame — this is safe but is a coupling issue (see Section 1).

---

## 5) Testing — Barrier to Expansion & Refactoring

### Test Coverage

There are **zero tests** in the entire codebase. `grep` for `#[test]` and `#[cfg(test)]` returns no results across all 19 source files.

This is the single largest barrier to confident refactoring. Every system — rarity rolls, weapon generation, loadout upgrade logic, chunk generation determinism, collision geometry, cooldown clamping, world-coordinate conversion — is untested. Any change to these systems has no regression safety net.

### Missing Test Infrastructure

- No unit tests for `Rarity::roll()` distribution (statistical correctness)
- No unit tests for `Loadout::try_equip_weapon` upgrade logic (e.g., confirming a lower-rarity drop is rejected)
- No unit tests for `ChunkCoord::from_world_pos` coordinate math
- No unit tests for `ChunkRng` determinism (same `(cx, cy)` should always yield the same chunk)
- No unit tests for `wrap_angle` boundary cases
- No unit tests for the `hsv_to_color` conversion
- No integration tests for the `Game` struct lifecycle
- No proptest / quickcheck fuzzing for any numeric domain function
- No `cargo test` step in CI

The `Rarity::roll()` function is a probability-weighted RNG — its distribution is a critical game-balance property that should be validated with a statistical test.

### Test Isolation Quality

N/A — no tests exist to evaluate.

---

## 6) CI/CD & Tooling — Process Gaps

### GitHub Actions Coverage

One workflow exists: `.github/workflows/deploy.yml`. It performs only:

1. Checkout
2. Install `wasm32-unknown-unknown` target
3. `cargo build --release --target wasm32-unknown-unknown`
4. Copy artifacts to `dist/`
5. Deploy to GitHub Pages

**What is entirely absent:**

| Missing Step | Impact |
|---|---|
| `cargo fmt --check` | No formatting enforcement; diffs will be noisy |
| `cargo clippy -- -D warnings` | Warnings (e.g., `dead_code` on `chunk.rs:37–39`) are never fatal |
| `cargo test` | No test execution (though currently moot — no tests exist) |
| `cargo build` for native target | A native build failure would not be caught before WASM deploy |
| Coverage gate (llvm-cov / tarpaulin) | No coverage reporting |
| `cargo deny` | No license, duplicate-dep, or advisory checks |
| MSRV check | No `rust-version` declared in `Cargo.toml`, no toolchain pin |

The CI cache key is `${{ hashFiles('**/Cargo.lock') }}` which is correct.

The workflow YAML lives in the repository root under `workflows/` rather than `.github/workflows/`. The `.github/` symlink or directory structure may be unconventional depending on how the repo was initialized — the file was found at `.github/workflows/deploy.yml` via glob, so the canonical path is correct.

### Tooling Config Files

| File | Present | Notes |
|---|---|---|
| `rustfmt.toml` | No | Default rustfmt settings are used |
| `clippy.toml` | No | No lint configuration; no lints elevated to errors |
| `deny.toml` | No | No dependency policy enforcement |
| `rust-toolchain.toml` | No | No pinned toolchain; `dtolnay/rust-toolchain@stable` is used in CI but stable drifts |
| `.cargo/config.toml` | Not checked | May not exist |

### Conventional Commits

Commit messages follow an informal `feat:` / `fix:` / `chore:` prefix pattern (visible in git log) but there is no `commitlint` or similar enforcement. The pattern is inconsistent (some messages use em-dashes, some use `+`).

---

## 7) API Design & Documentation — Onboarding Cost

### `pub` vs `pub(crate)` Usage

Almost every type, field, function, and constant is `pub`. `grep` finds 159 `pub` occurrences across 19 source files; `pub(crate)` is used zero times. For a binary-only crate this is a minor concern (nothing is exposed to external consumers), but it eliminates the compiler's ability to flag unused public items and provides no encapsulation guidance to contributors.

Notable overly-public fields that should be private or `pub(crate)`:

| Field | File | Issue |
|---|---|---|
| `Player::pos`, `Player::vel`, `Player::rotation` | `player.rs:12–14` | Read widely; direct mutation from `Game` bypasses `Player` invariants |
| `Player::is_thrusting` | `player.rs:15` | Write-visible to `Game`; should only be set by `Player::update` |
| `Enemy::pos`, `Enemy::vel`, `Enemy::health`, `Enemy::max_health`, `Enemy::archetype` | `enemy.rs:14–19` | All directly read/mutated by `Game` collision code |
| `World::spawn_queue` | `world/mod.rs:13` | Pub field acting as a message bus |
| `Chunk::asteroids`, `Chunk::planet`, `Chunk::stars`, etc. | `chunk.rs:40–46` | All pub; callers can freely corrupt chunk state |

### Documentation

- Zero `///` doc comments on any public type, function, or method except for a handful of inline `//` comments in `game.rs` and `world/gen.rs`
- No `#[must_use]` attributes on any `Result`-returning or important-value-returning function (e.g., `Loadout::try_equip_weapon` returns `Option<(String, Rarity)>` and the caller *must* use it to show the pickup notice — missing `#[must_use]` means a future refactor could silently drop the return value)
- `Game::new()` does not implement `Default` despite being a no-argument constructor; `impl Default for Game` would be idiomatic
- `World::new()` and `MobileOverlay::new()` similarly lack `Default` impls

### README Quality

`README.md` is two lines:

```
# Starseeker
An experimental game for fusing asteroids and freelancer
```

There is no build instructions, no WASM setup guide, no architecture overview, no contribution guide, and no license declaration in the README (though a `LICENSE` file exists in the repo root).

---

## 8) Dependencies — Supply Chain & Bloat Risk

### Dependency Audit

`Cargo.toml` declares two direct dependencies:

```toml
[dependencies]
macroquad = "0.4"
quad-rand = "0.2"
```

`Cargo.lock` resolves to ~35 transitive dependencies including `image = "0.24.9"`, `fontdue`, `glam = "0.27"`, `miniquad`, `winapi`, and Android NDK bindings (`ndk-sys`). This is a typical macroquad dependency closure — no exotic or suspicious crates.

`cargo audit` is not installed in this environment so an advisory check could not be run. The dependency versions are not pinned to patch versions (e.g., `macroquad = "0.4"` will resolve `0.4.x`), which is standard semver practice but means patch-level yanks or semver-compatible breaking changes could affect reproducibility between `cargo update` runs.

### Cargo.lock

`Cargo.lock` is committed to the repository (confirmed via glob). This is correct for a binary crate and supports reproducible CI builds.

### MSRV

No `rust-version` field is declared in `Cargo.toml`. The minimum supported Rust version is undefined. The CI uses `dtolnay/rust-toolchain@stable` which will silently move forward. If a contributor uses an older toolchain, build failure gives no helpful "requires Rust X.Y" message.

### Version Constraints

Both direct dependencies use a major-version constraint with no upper bound (`"0.4"`, `"0.2"`). For `0.x` crates this is equivalent to a minor-version pin in semver, which is appropriate. No `[patch]` or `[replace]` sections are present.

### `cargo deny` / License Audit

No `deny.toml` exists. The transitive tree includes `winapi = "0.3.9"` (MIT/Apache) and `image = "0.24.9"` (MIT). No AGPL or GPL-incompatible licenses are visible in the resolved tree, but this is not mechanically enforced.

---

## 9) Performance & Resource Management — Production Readiness

### Per-Frame Allocations

`game.update()` allocates multiple `Vec`s on every frame regardless of whether any collisions occur:

| Allocation | File/Lines | Frequency | Notes |
|---|---|---|---|
| `player_proj_positions: Vec<(usize, Vec2)>` | `game.rs:99–105` | Every frame | Collects all player projectiles into a new heap Vec |
| `player_proj_data: Vec<(usize, Vec2, f32)>` | `game.rs:118–124` | Every frame | Second collect of the same iterator with a different field |
| `remove_projs: Vec<usize>` (×2) | `game.rs:107`, `126` | Every frame | Two separate index-collection Vecs |
| `drops: Vec<LootDrop>` | `game.rs:146` | Every frame | Allocated even when no enemies die |
| `picked: Vec<LootDrop>` | `game.js:201` | Every frame | Partition of `loot_drops` into two new Vecs |
| `remaining: Vec<LootDrop>` | `game.js:202` | Every frame | Second partition Vec; `loot_drops` is drained and rebuilt every frame |

The loot pickup partition (`game.rs:201–210`) completely drains and rebuilds `self.loot_drops` every frame using two temporary Vecs. A `retain`-based approach or an index-based sweep would eliminate these allocations.

The two projectile collect+remove passes (`game.rs:99–143`) could be unified into a single `retain_mut` pass that handles both asteroid and enemy collisions simultaneously, eliminating two heap allocations.

`draw_credits` in `loot.rs:37` calls `.collect()` to build a `Vec<Vec2>` of 4 corner points every frame. A fixed-size array `[Vec2; 4]` should be used instead.

### String Allocations in Hot Path

`game.rs:394` calls `measure_text(&format!("[{}]", tier), None, 13, 1.0)` in `draw_loadout_slot` every frame, allocating a temporary format string for a GPU text measurement call. The `[CMN]` / `[UNC]` / `[RARE]` / `[EPIC]` strings are a finite set and could be `const` or computed once.

`game.rs:276` calls `format!("FPS: {}", get_fps())` every frame, allocating a string for an integer that changes at most once per second.

### Chunk Generation Performance Spike

`world/mod.rs:34–36` calls `load_nearby` synchronously when the player crosses a chunk boundary. `gen::gen_chunk` allocates `Vec<StarPoint>` (45–69 entries), `Vec<(Vec2, f32, Color)>` (3–6 blobs), `Vec<Asteroid>` (4–13 entries each with two inner `Vec<f32>` for vertex data), and a `Vec<(Vec2, EnemyArchetype)>`. Up to 25 new chunks could be generated in a single `update()` call on a cold start. This will cause a measurable frame spike.

### Logging and Observability

There is no logging anywhere in the codebase — no `log` crate, no `tracing`, no `eprintln!`. In WASM, any diagnostic output requires manual `console_log!` calls. There is no way to diagnose performance issues, unexpected game states, or chunk generation anomalies in production.

### HashMap Iteration Order

`World::draw` iterates `self.chunks: HashMap<(i32,i32), Chunk>` three times in sequence (`world/mod.rs:43–53`). `HashMap` iteration order is non-deterministic. While draw order rarely matters for space games, background blobs from a newly-loaded chunk may appear to flicker or z-fight depending on hash iteration order. A `BTreeMap` or sorted iteration would make draw order stable.

---

## 10) Refactoring Estimation & Summary

### Top 3 Risks

**Risk 1 — Zero test coverage on correctness-critical logic.**
`Loadout::try_equip_weapon`, `Rarity::roll`, `ChunkRng` determinism, and `wrap_angle` are all untested. Any refactor of these systems has no safety net. A bug in `Rarity::roll` could silently skew game balance without detection.

**Risk 2 — `game.rs` monolith with per-frame heap allocations.**
The 406-line `Game::update` function contains six distinct collision passes, each allocating temporary `Vec`s. This is both a performance hazard and a maintainability hazard: adding a new entity type requires modifying this single function in multiple places, and the per-frame allocation pattern will cause GC pressure at scale in WASM environments.

**Risk 3 — No CI quality gate beyond a successful WASM build.**
Clippy warnings, formatting drift, and test regressions are invisible to the merge process. The `dead_code` warning on `chunk.rs:37–39` (`coord`, `hostility`, `chunk_type` fields) already exists and is silently ignored. As the codebase grows, warning debt will compound.

### Key Tasks

1. **Add `cargo clippy -- -D warnings` and `cargo fmt --check` to CI** — blocks warns-as-errors before they accumulate.
2. **Add a `cargo test` CI step** — even with zero tests today, this gates future test regressions.
3. **Write unit tests for `Rarity`, `Loadout`, `ChunkCoord`, `wrap_angle`, `hsv_to_color`** — these are pure functions with no rendering dependency and are testable immediately.
4. **Eliminate per-frame Vec allocations in `Game::update`** — consolidate dual projectile collision passes into one `retain_mut`, replace loot drain/rebuild with `retain`, replace `draw_credits` Vec with a fixed array.
5. **Replace `pub` fields with `pub(crate)` or accessors** — specifically `Player::pos`/`vel`/`rotation`/`is_thrusting`, `Enemy` fields, `World::spawn_queue`, `Chunk` collection fields.
6. **Extract entity hit radii to `EnemyArchetype` method** — single source of truth for Tank=18, Agile=10, Ranged=13 instead of duplicating in `game.rs:129–133` and `enemy.rs` draw methods.
7. **Declare `rust-version` in `Cargo.toml`** and pin CI toolchain.
8. **Add `#[must_use]` to `try_equip_weapon` and `try_equip_thruster`**.
9. **Guard `vel.normalize()` in `projectile.rs:53`** — check `self.vel.length_squared() > 0.0` before normalizing.
10. **Address `dead_code` warnings** — either use or remove `Chunk::coord`, `Chunk::hostility`, `Chunk::chunk_type`.

### Time Estimates

**Minimal (critical gaps only):**
Address the three top risks: add CI lint/fmt/test gates, eliminate the worst per-frame allocations, and write a basic test suite for pure functions.
**Estimated: 1–2 developer days**

Breakdown:
- CI pipeline additions (clippy -D warnings, fmt check, test step, native build): 2–4 hours
- Eliminate per-frame Vec allocations in `Game::update` (loot partition, dual collision collect): 3–4 hours
- Fix `projectile.rs:53` normalize guard: 30 minutes
- Unit tests for `Rarity`, `Loadout`, `ChunkCoord`, `wrap_angle`: 3–4 hours

**Comprehensive (full compliance with production-quality standards):**
Full test coverage for all pure logic, `pub(crate)` encapsulation pass, newtype wrappers for domain quantities, extraction of `game.rs` into systems, configuration struct, `cargo deny` setup, MSRV declaration, README overhaul, and observability hooks.
**Estimated: 8–14 developer days**

Breakdown:
- Encapsulation pass (`pub` → `pub(crate)`, accessors): 1 day
- `game.rs` decomposition into separate system modules: 2–3 days
- Newtype wrappers (`Health`, `Damage`, `Radians`, `WorldPos`): 1–2 days
- Comprehensive test suite (unit + integration + property tests for RNG): 2–3 days
- Configuration struct + data-driven content hooks: 1–2 days
- `cargo deny`, `deny.toml`, MSRV, toolchain pin: 0.5 day
- README, doc comments, `#[must_use]` pass: 0.5–1 day

### Justification

The codebase is architecturally sound for a Phase-3 game prototype. The module boundaries are sensible, there is no `unsafe` code, and the macroquad async model is used correctly. The main liabilities are all process and discipline gaps rather than deep architectural flaws: absent tests make the correctness-critical item/loot logic invisible to regression, per-frame allocations are fixable with mechanical substitution (not redesign), and the CI pipeline adds quality gates in under a day. The comprehensive estimate reflects that extracting `game.rs` into proper systems is genuine refactoring work that touches every call site, and that newtype wrappers require propagating type changes through the full call graph.
