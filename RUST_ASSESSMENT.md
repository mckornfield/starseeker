# Starseeker — Rust Technical Audit

**Audited:** 2026-03-21
**Auditor:** Automated structural review (all `.rs` sources read, Cargo.toml and CI inspected, `cargo clippy --all-targets` and `cargo test` executed)
**Codebase snapshot:** commit `021081d` — ~4 400 lines of Rust across 20 source files

> **Delta from prior audit (2026-03-17 @ bbdd758):** 40 tests added (from 0), CI pipeline fully replaced with quality gates, `game.rs` grew from 406 → 1 701 lines, `missions.rs` added (475 lines). Significant improvements in testing and CI; monolith risk has intensified.

---

## 1) Architecture & Crate Structure — Scalability Challenges

### Workspace & Modularity

The project remains a single binary crate with no workspace. Module tree:

```
main.rs
game.rs          ← 1 701 lines — severe god-module (was 406 at last audit, now 4× larger)
player.rs
input.rs
projectile.rs
mobile.rs
missions.rs      ← 475 lines — a second large monolith added since last audit
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
    mod.rs
    rarity.rs
    thruster.rs
    weapon.rs
```

`game.rs` has nearly quadrupled since the last audit. It now owns: game-state management, five distinct overlay/menu systems (sector map, inventory, planet menu, death screen, HUD), six collision passes, quest tracker rendering, camera management, loadout UI, and mobile layout. This file is the primary scalability risk in the codebase. Adding any new system (crafting, multiplayer, save/load) will require surgical edits in an already crowded 1 700-line file.

`missions.rs` at 475 lines contains shop generation, mission generation, mission log, and statistical tracking all in a single file. It is a smaller version of the same monolith pattern.

The `items/` module remains the best-structured part of the codebase: four focused files, clear separation of rarity, weapon, thruster, and loadout logic.

The `spawn_queue` field on `World` (`world/mod.rs`) continues to act as a public message bus between `World` and `Game`. This is an architectural smell that prevents `World` from controlling the lifecycle of its own spawns.

### Dependency Injection

All configuration remains hardcoded constants scattered across modules. There is still no `GameConfig` struct and no trait-based abstraction over any subsystem. `Enemy::update` takes a `&mut Vec<Projectile>` directly, coupling enemy AI to the concrete container type. The `gen.rs` world seed is a `const` with no path to data-driven or user-controlled seeding.

---

## 2) Error Handling — Debugging & Reliability Risk

### Error Type Design

The project still uses **zero** dedicated error types — no `thiserror`, no `anyhow`. Every operation either panics or succeeds silently. This is acceptable for a game binary (panics are visible to developers), but the pattern is unscalable once asset loading or save/load is added.

### `unwrap()` in Non-Test Code

| File | Location | Risk |
|---|---|---|
| `src/world/gen.rs:256` | `s1.chars().next().unwrap()` | Panics if `SYLS` ever contains an empty string — invariant is neither documented nor asserted |

All other `unwrap()` calls are inside `#[cfg(test)]` blocks, which is appropriate.

### `normalize()` on Potentially-Zero Vectors

`projectile.rs:53` calls `self.vel.normalize()` unconditionally during draw:

```rust
let trail = self.pos - self.vel.normalize() * 6.0;
```

In `glam`, `Vec2::normalize()` on a zero vector returns `Vec2::NAN`. A projectile spawned with zero direction will silently corrupt its draw position. A guard (`self.vel.length_squared() > f32::EPSILON`) is needed.

### Floating-Point Equality for Game Logic

`player.rs` and `enemy.rs` gate firing on `cooldown == 0.0`. Because cooldowns are clamped via `.max(0.0)`, this is currently safe. It remains a fragile pattern that should be replaced with `cooldown <= 0.0`.

---

## 3) Type Safety & Correctness — Maintenance Deficits

### Primitive Obsession

All domain quantities remain raw `f32` or `Vec2` with no newtype wrappers. Health, damage, speed, cooldown, rotation, and world/screen positions are all dimensionally identical to the compiler. This is unchanged from the prior audit. Key risks:

- Health can silently go below zero with no type enforcement
- World-space and screen-space `Vec2` are interchangeable at call sites
- Rotation in radians and rotation deltas are indistinguishable

### Enemy Hit Radius Duplication

Enemy collision radii are still duplicated between `game.rs` collision code and `enemy.rs` draw methods (Tank=18, Agile=10, Ranged=13). There is still no single source of truth. This will eventually diverge. An `EnemyArchetype::hit_radius()` method would resolve this.

### Wildcard Match Arms in Generation

`world/gen.rs` uses `_ => EnemyArchetype::Ranged` in a `match rng.range_usize(3)`. Adding a fourth archetype will not produce a compiler error; it will silently be treated as `Ranged`. The arm should be `2 => EnemyArchetype::Ranged` with `_ => unreachable!()`.

### Clippy: `collapsible_if` (6 instances in `game.rs`)

CI runs `cargo clippy --all-targets -- -D warnings`. The following warnings exist in the current codebase and **will cause CI to fail on main**:

| Location | Warning |
|---|---|
| `game.rs:123` | `collapsible_if` |
| `game.rs:188` | `collapsible_if` |
| `game.rs:193` | `collapsible_if` |
| `game.rs:499` | `collapsible_if` |
| `game.rs:504` | `collapsible_if` |
| `game.rs:560` | `collapsible_if` |
| `game.rs:900` | `type_complexity` — "very complex type used" |
| `game.rs:1451` | `too_many_arguments` (8/7) |
| `game.rs:989` | `manual_arithmetic_check` |
| `items/loadout.rs:22` | `double_must_use` |
| `items/loadout.rs:45` | `double_must_use` |

These 11 warnings (reported as 11 unique + 11 duplicates) mean **CI is currently broken on main**. The `collapsible_if` and `double_must_use` warnings have autofix suggestions (`cargo clippy --fix`).

### `unsafe` Usage

No `unsafe` blocks exist anywhere in the source. Correct for a macroquad game.

---

## 4) Async & Concurrency — Safety & Performance Risk

No change from the prior audit. The macroquad cooperative async model is used correctly: fully synchronous `update()`/`draw()` with a single `next_frame().await` yield. No tokio, no rayon, no `Arc`/`Mutex`.

The risk noted previously remains: `world/mod.rs` calls `gen::gen_chunk()` synchronously on chunk boundary crossing, causing frame-time spikes on cold start. This is not a correctness issue but a production readiness concern.

---

## 5) Testing — Barrier to Expansion & Refactoring

### Significant Improvement: 40 Tests Added

The prior audit found zero tests. The current codebase has **40 unit tests** across 6 modules:

| Module | Tests | Coverage Focus |
|---|---|---|
| `items::rarity` | 5 | Ordering, labels, roll validity, statistical distribution |
| `items::loadout` | 9 | Equip/reject/upgrade/unequip logic for all slot types |
| `items::weapon` | 7 | Stat generation, HSV conversion |
| `missions` | 7 | Mission generation, log limits, tracking (kills/credits/visits), claim |
| `world::chunk` | 4 | `ChunkCoord` conversion (origin, positive, negative, boundary) |
| `entities::enemy` | 6 | Damage/death, `wrap_angle` (zero, in-range, negative, positive), hit radii |

This is a material improvement. The `loadout` tests are particularly thorough (empty slot, rejection, upgrade, force-equip, return-of-old-item). The statistical distribution test for `Rarity::roll` addresses the specific concern raised in the prior audit.

### Remaining Coverage Gaps

Despite 40 tests, several correctness-critical areas remain untested:

- **`projectile.rs`** — zero tests. The `normalize()` hazard is untested. Projectile lifetime and collision behavior are unverified.
- **`player.rs`** — zero tests. Thrust application, drag, max-speed clamping, stabilize logic, cooldown decrement, and firing guard are all untested.
- **`game.rs`** — structurally untestable. The `Game` struct mixes rendering and logic so tightly that no unit test can exercise collision logic, loot pickup, or sector generation without a full macroquad context. This is the consequence of the monolith: testability requires decomposition.
- **`world/gen.rs`** — `ChunkRng` determinism is untested (same `(cx, cy)` input should always produce the same chunk). The `gen_planet_name` syllable combinator is untested.
- **`missions.rs`** — shop offering determinism is untested. The seeded RNG (`MissionRng`) determinism for a given planet name is untested.
- **`entities/loot.rs`** — zero tests. Loot drop generation from enemy type is untested.

No property-based tests (`proptest`, `quickcheck`) exist. The `Rarity::roll` distribution test uses a single hard-coded count (10 000 rolls) rather than a property assertion.

### Test Organization

All tests use `#[cfg(test)] mod tests` within the source file, which is the correct Rust idiom. No `tests/` integration test directory exists. Test helper functions are file-local, which is correct. No `#[ignore]` tests exist.

---

## 6) CI/CD & Tooling — Significant Improvement, Active CI Breakage

### Pipeline Coverage

The prior audit found only a deploy workflow with no quality gates. A full `ci.yml` has been added:

| Step | Status |
|---|---|
| `cargo fmt --check` | ✅ Present |
| `cargo clippy --all-targets -- -D warnings` (native) | ✅ Present — **currently failing** (11 warnings) |
| `cargo clippy --target wasm32-unknown-unknown -- -D warnings` | ✅ Present |
| `cargo test` | ✅ Present |
| `cargo build` (native) | ✅ Present |
| `cargo build --release --target wasm32-unknown-unknown` | ✅ Present |
| Coverage gate (`llvm-cov` / tarpaulin) | ❌ Missing |
| `cargo audit` / `cargo deny` | ❌ Missing |

**The CI is currently broken.** The 11 clippy warnings in `game.rs` and `items/loadout.rs` will fail the `-- -D warnings` step. The commit `021081d` that introduced these warnings was pushed directly to `main`.

### Missing Tooling

| File | Present | Notes |
|---|---|---|
| `rustfmt.toml` | No | Default rustfmt used |
| `clippy.toml` | No | No custom lint config |
| `deny.toml` | No | No license/advisory policy |
| `rust-toolchain.toml` | No | `dtolnay/rust-toolchain@stable` used in CI but stable drifts |
| Coverage enforcement | No | `cargo test` runs but no threshold is gated |

### Conventional Commits

Commit messages follow an informal `feat:` prefix pattern. No `commitlint` or enforcement. Pattern is consistent across recent history.

---

## 7) API Design & Documentation — Onboarding Cost

### `pub` vs `pub(crate)`

`pub(crate)` is used in `mobile.rs` (`pub(crate) struct MobileOverlay`) — this is new and correct. However the broader codebase still exposes nearly everything as `pub` with no encapsulation boundary. Critical fields that enable invariant violation from outside their owning type:

- `Player::pos`, `Player::vel`, `Player::rotation`, `Player::is_thrusting`, `Player::main_cooldown`, `Player::aux_cooldown` — directly read and mutated by `Game`
- `Enemy::pos`, `Enemy::vel`, `Enemy::health`, `Enemy::archetype` — directly mutated by `Game` collision code
- `World::spawn_queue` — public Vec used as a message bus
- `Chunk::asteroids`, `Chunk::planet`, `Chunk::stars` — fully public collection fields

### `#[must_use]`

`items/loadout.rs` has `#[must_use]` on `try_equip_weapon` and `try_equip_thruster`, which is correct. However clippy flags these with `double_must_use` because the return types (`Result<...>` and `Result<...>`) are already `#[must_use]` — the attribute should be removed or given a message string.

### Documentation

Zero `///` doc comments on any public type, function, or method. No `#[deny(missing_docs)]`. No README build instructions. README is still two lines.

---

## 8) Dependencies — Supply Chain & Bloat Risk

No change from prior audit. Two direct dependencies (`macroquad = "0.4"`, `quad-rand = "0.2"`), `Cargo.lock` committed, no `cargo audit` or `deny.toml`. No MSRV declared in `Cargo.toml`.

---

## 9) Performance & Resource Management — Production Readiness

### Per-Frame Allocations (Unchanged)

The per-frame `Vec` allocation patterns identified in the prior audit remain:

- Dual projectile-collision collect passes in `Game::update` — could be unified into one `retain_mut`
- Loot drain/rebuild allocating two partition Vecs every frame — could be `retain`
- `draw_credits` allocating `Vec<Vec2>` for 4 points — should be `[Vec2; 4]`
- `format!("FPS: {}", get_fps())` and `format!("[{}]", tier)` allocating strings in draw hot path

### New: `game.rs` Draw Method Complexity

`game.rs` now has multiple `draw_*` methods (HUD, sector map, inventory, planet menu, death screen, quest tracker, loadout slots) inline in the same file as `update`. Every draw method has access to the full `&self` state, making it easy to accidentally couple rendering to mutable state or to make state changes in draw methods. The render/update boundary is convention only, not enforced by type.

### HashMap Draw Order

`World::draw` iterates `self.chunks: HashMap<(i32,i32), Chunk>` in non-deterministic order. Noted in the prior audit, still present.

### Logging

No logging anywhere. On WASM, any diagnostic requires `console_log!`. No structured logging, no spans, no observability for production debugging.

---

## 10) Refactoring Estimation & Summary

### Top 3 Risks

**Risk 1 — CI is currently broken by clippy warnings introduced on main.**
The `cargo clippy --all-targets -- -D warnings` step will fail on `021081d`. Eleven warnings (6× `collapsible_if`, `type_complexity`, `too_many_arguments`, `manual_arithmetic_check`, 2× `double_must_use`) need to be resolved before main is green. This is a process discipline issue: the quality gate exists but was bypassed by direct push. Several have `--fix` autofixes.

**Risk 2 — `game.rs` monolith at 1 701 lines makes game logic untestable and expansion expensive.**
Since the last audit, `game.rs` grew by 1 295 lines (4×). Every new feature (sector map, HUD, inventory, planet menu, quest tracker, death screen) was added directly into the same file. No logic in `game.rs` can be unit-tested without a macroquad rendering context. Any new system — crafting, multiplayer, persistence — must be grafted into this single file. The longer decomposition is deferred, the more expensive it becomes.

**Risk 3 — Coverage gaps in `projectile.rs`, `player.rs`, and `game.rs` collision logic.**
The 40 tests cover item systems and pure utility functions well, but the physics core (projectile lifetime, player thrust/drag/cooldown, collision detection) is entirely untested. A refactor of velocity clamping, damage application, or the `normalize()` hazard in `projectile.rs` has no regression safety net.

### Key Tasks

1. **Fix the 11 clippy warnings immediately** — run `cargo clippy --fix` for the auto-fixable ones (`collapsible_if`, `double_must_use`), address `too_many_arguments` and `type_complexity` manually. This unblocks CI.
2. **Extract systems from `game.rs`** — at minimum, split into `game/update.rs`, `game/draw.rs`, and `game/collision.rs`. Target: `game.rs` under 400 lines, with collision and HUD drawing in their own `impl` blocks or sub-modules.
3. **Add `projectile.rs` unit tests** — test `Projectile::new`, verify lifetime decrement, guard the `normalize()` zero-velocity hazard, verify `is_dead()`.
4. **Add `player.rs` unit tests** — test drag application, thrust clamping, cooldown decrement, firing guard, stabilize braking.
5. **Eliminate per-frame Vec allocations** — `retain_mut` for projectile collision, `retain` for loot pickup, `[Vec2; 4]` for draw_credits, `const` for tier label strings.
6. **Guard `vel.normalize()` in `projectile.rs:53`** — `if self.vel.length_squared() > f32::EPSILON`.
7. **Add `EnemyArchetype::hit_radius() -> f32`** — eliminates the duplicated literal values.
8. **Replace `_ => EnemyArchetype::Ranged` with `2 => ... _ => unreachable!()`** in `world/gen.rs`.
9. **Add `rust-version` to `Cargo.toml`** and `rust-toolchain.toml` to pin the toolchain.
10. **Add coverage step to CI** (`cargo-llvm-cov` or `tarpaulin`) with a minimum threshold.

### Time Estimates

**Minimal (unblock CI + highest-risk gaps):**
- Fix 11 clippy warnings: 1–2 hours (several are `--fix` autofixes)
- Add projectile and player unit tests: 2–3 hours
- Guard `normalize()` zero-velocity hazard: 30 minutes
- `EnemyArchetype::hit_radius()` extraction: 1 hour

*Estimated: 1 developer day*

**Comprehensive (sustainable codebase):**
- `game.rs` decomposition into sub-modules: 2–4 days
- Full test coverage for physics and collision: 2–3 days
- Per-frame allocation cleanup: 1 day
- `pub` → `pub(crate)` encapsulation pass: 1 day
- `cargo deny`, MSRV, coverage gate, `rust-toolchain.toml`: 0.5 day
- Newtype wrappers for `Health`, `Damage`, `Radians`, `WorldPos`: 1–2 days

*Estimated: 8–12 developer days*

### Justification

Since the last audit, the project has made meaningful progress on the two highest-priority gaps: tests and CI. Forty tests covering item systems and utility logic provide a real regression safety net for game balance and loadout logic. The CI pipeline now enforces formatting, linting, testing, and both build targets on every PR — this is the correct long-term foundation.

The outstanding risks are process discipline (CI was broken by a direct push) and architectural momentum (`game.rs` is growing faster than it's being decomposed). Addressing the clippy warnings restores the quality gate in under a day. The more consequential investment — decomposing `game.rs` and adding physics-layer tests — is what enables confident feature development in Phase 4 and beyond without accumulating hidden regression risk.
