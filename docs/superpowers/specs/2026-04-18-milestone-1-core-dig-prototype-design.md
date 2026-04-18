# Milestone 1 — Core Dig Prototype (Design Spec)

**Date:** 2026-04-18
**Status:** Draft (Bevy rework — supersedes initial Godot draft)
**Parent roadmap:** [../../roadmap.md](../../roadmap.md)

## Purpose

Answer the single question: **is digging fun?** Nothing else. If the
moment-to-moment loop of moving, aiming, swinging, breaking tiles, and
watching ore drops pop and vacuum into inventory is satisfying, milestones
2+ have a foundation worth building on. If it isn't, we iterate here before
anything else is built.

## Scope

### In scope
- Single player, single local session.
- One procedurally generated property, fixed dimensions, fresh seed each run.
- Top-down 2D view.
- Smooth-contour destructible terrain (marching squares over a grid).
- Surface strip at the top edge; diggable underground below.
- Depth-banded layers (dirt / stone / deep rock / bedrock) purely as visual
  cues; hardness is uniform (one click breaks any non-bedrock tile).
- 3 ore types (copper near surface, silver mid, gold deep), rarer and more
  valuable with depth.
- Single generic pickaxe, click-per-hit, ~2-tile reach, ~0.15 s cooldown.
- Ores drop as physical pickup entities with ~1-tile auto-vacuum.
- Flat unlimited inventory surfaced in a minimal HUD.
- Placeholder art, minimal SFX/VFX.

### Out of scope (deferred to named milestones)
- Tool tiers and tile hardness variation — **milestone 2**.
- Factory, machines, recipes, money, shop — **milestone 3**.
- Networking, multiplayer, host/join — **milestone 4**.
- Conveyors, pallets, forklifts — **milestone 5**.
- Multiple properties, contracts, licenses — **milestone 6**.
- Character/warehouse customization, art pass, audio polish, tutorials,
  accessibility — **milestone 7**.
- Save/load — first introduced in milestone 3.
- Natural caves, cellular-automata carving — candidate for milestone 2 or 3.

### Explicitly not designed for
- Scaling to multiplayer today (but structurally friendly to it — see
  [Cross-cutting Invariants](#cross-cutting-invariants)).
- Huge maps / streaming. Fixed 80×200 tiles.
- Mobile / touch input. Keyboard + mouse only.

## Target platform & tech

- **Engine:** [Bevy](https://bevyengine.org/), latest stable. Plan
  authored against **Bevy 0.15.x**; engineer adapts to current.
- **Language:** Rust (stable channel).
- **Build:** Cargo, single binary crate.
- **Perspective:** Top-down 2D, orthographic camera.
- **Platforms:** Desktop (Windows / macOS / Linux), single player.
- **Collision:** DIY tile-grid AABB. The player is a small AABB; collision
  is checked against the Grid each tick. No physics crate.
- **Rendering:** Bevy 2D primitives — meshes for terrain chunks, sprites
  for player and ore drops, `bevy_ui` for HUD.
- **Tests:** Rust's built-in `#[test]` framework, in-crate `tests/`
  directory for integration tests where needed. Pure data is testable
  without spinning up a Bevy `App`.
- **Art:** Placeholder solid-color meshes and sprites. Art pass is
  milestone 7.

## Key design decisions

Decisions made during brainstorming, recorded here so the implementation
plan can reference them without re-litigating:

| Decision | Choice | Why |
|---|---|---|
| Terrain look & model | Smooth contour (marching squares) over a grid | Organic feel; grid underneath keeps netcode & procgen tractable. |
| Dig feel | Click-per-hit, one tile per swing | User preference. Compensate with audio/VFX punch; cap clicks-per-tile at 2–3 across all future tool tiers to manage RSI. |
| World orientation | Thin surface strip at top edge; underground below | Matches final game structure; lets "deeper = further from surface" pay off visually even in milestone 1. |
| Procgen style | Depth layers + ore veins, no natural caves | Visible progress as you descend; near-zero extra work over flat; sets up milestone 2 tool tiers at zero gameplay cost. |
| Ore pickup | Physical drop + ~1-tile auto-vacuum | Satisfying "pop," no hunting for pickups, reuses as a pattern for later pickup-able entities. |
| Engine | Bevy (Rust ECS) | 100% code, no GUI in the loop, strong typing, ECS suits factory game; replaces earlier Godot choice. |
| Rendering approach | Custom marching-squares meshes per chunk over an explicit grid resource | Grid as first-class truth pays off every subsequent milestone; visual matches the chosen smooth-contour direction. |
| Collision approach | DIY tile-grid AABB resolution against the Grid | Fits the destructible-terrain model perfectly (collision data is implicit in the Grid; updates instantly when a tile breaks); avoids a heavy physics dependency. |

## Architecture

### ECS world layout

Bevy is ECS — there is no scene tree. Conceptually:

- **Resources** (singleton state):
  - `Grid` — the 2D tile array (single source of truth for terrain).
  - `Inventory` — flat dictionary of `OreType -> count`.
  - `WorldSeed` — the run's seed (generated in `setup_world` from `rand::random()` and logged at startup so playtests can be reproduced).
  - `DigCooldown` — `Timer` that gates per-player dig actions.
- **Entities + Components:**
  - **Player**: `Player` marker, `Transform`, `Sprite` (placeholder square), `Velocity`.
  - **TerrainChunk** (one per visible chunk): `TerrainChunk { coord: IVec2 }`, `Transform`, `Mesh2d` (visual), `ChunkDirty` marker (for re-meshing).
  - **OreSprite** (one per visible ore tile inside a chunk's region): `OreSprite { ore: OreType }`, `Transform`, `Sprite`.
  - **OreDrop**: `OreDrop { ore: OreType }`, `Transform`, `Sprite`, `VacuumTarget` (when within radius).
  - **MainCamera**: `Camera2d`, follows player.
  - **HudRoot** + UI children for inventory display.
- **Systems** (run per-tick or on schedule):
  - `setup_world` (Startup) — generate Grid, spawn Player, spawn Camera, spawn HUD.
  - `player_movement` (Update) — read input → set Velocity.
  - `player_collision` (Update, after movement) — resolve AABB overlap against Grid solid tiles.
  - `camera_follow` (Update) — interpolate camera toward player.
  - `dig_input` (Update) — on LMB pressed and cooldown elapsed, compute target tile, call dig logic.
  - `chunk_lifecycle` (Update) — spawn / despawn `TerrainChunk` entities based on camera-visible rect + 1-chunk margin.
  - `chunk_remesh` (Update, after dig and lifecycle) — for each `ChunkDirty`, regenerate mesh + the per-chunk ore sprite set; remove `ChunkDirty`.
  - `oredrop_vacuum` (Update) — for each `OreDrop`, lerp toward Player when within vacuum radius; on intersection, deliver to Inventory and despawn.
  - `inventory_hud` (Update, on `Inventory` changed) — refresh HUD text.

### Module boundaries

Dependencies flow strictly from the game crate down. Pure modules
(`grid`, `terrain_gen`, `inventory`, `dig`) take no Bevy types and are
fully unit-testable without an `App`.

```
main → app plugins → systems → pure modules
                              → bevy types (Resource/Component/Query)
```

### File layout

```
Cargo.toml
src/
  main.rs                       # App setup: add plugins, systems, resources
  lib.rs                        # Re-exports for tests
  app.rs                        # MiningSimPlugin wiring all milestone-1 pieces
  grid.rs                       # PURE: Grid struct, Tile, Layer, OreType
  terrain_gen.rs                # PURE: generate(width, height, seed) -> Grid
  inventory.rs                  # PURE: Inventory struct + tests
  dig.rs                        # PURE: try_dig(grid, tile) -> DigResult
  marching_squares.rs           # PURE: contour mesh from Grid slice
  systems/
    setup.rs                    # startup: spawn world entities
    player.rs                   # movement, collision, dig input
    camera.rs                   # follow system
    chunk_lifecycle.rs          # spawn/despawn chunks
    chunk_render.rs             # remesh dirty chunks
    ore_drop.rs                 # vacuum + delivery
    hud.rs                      # inventory UI
  components.rs                 # tag/marker components: Player, TerrainChunk, OreDrop, etc.
tests/
  grid.rs                       # integration: pure Grid tests
  terrain_gen.rs                # integration: pure procgen tests
  inventory.rs                  # integration: pure Inventory tests
  dig.rs                        # integration: pure dig tests
```

Each pure module has unit tests in-file (`#[cfg(test)] mod tests {}`).
Integration tests in `tests/` exercise public APIs only.

## Components / modules in detail

### `grid.rs` — pure data
- `pub enum Layer { Dirt, Stone, Deep, Bedrock }`
- `pub enum OreType { None, Copper, Silver, Gold }`
- `pub struct Tile { pub solid: bool, pub layer: Layer, pub ore: OreType }`
- `pub struct Grid { /* width, height, tiles: Vec<Tile> */ }`
  - `pub fn new(width: u32, height: u32) -> Self`
  - `pub fn width(&self) -> u32`, `height(&self) -> u32`
  - `pub fn in_bounds(&self, x: i32, y: i32) -> bool`
  - `pub fn get(&self, x: i32, y: i32) -> Option<&Tile>`
  - `pub fn set(&mut self, x: i32, y: i32, t: Tile)` (panics on OOB)
- No Bevy imports. Unit-testable.

### `terrain_gen.rs` — pure functions
- `pub fn generate(width: u32, height: u32, seed: u64) -> Grid`
- `pub fn spawn_tile(g: &Grid) -> IVec2` (or a plain `(i32, i32)`; choose to avoid Bevy import — use `glam::IVec2` which Bevy re-exports).
- Steps: bedrock ring → surface strip rows → depth-banded interior layers → ore vein sprinkling → 3×3 spawn pocket carving with non-ore floor underneath.
- Deterministic for a given seed. Uses `rand` crate with `StdRng::seed_from_u64`.

### `inventory.rs` — pure data
- `pub struct Inventory { /* HashMap<OreType, u32> */ }`
- Methods: `add(ore, n)`, `remove(ore, n)`, `get(ore) -> u32`.
- No signal mechanism in pure module — Bevy systems detect changes via the `Resource`'s `Changed<>` filter.

### `dig.rs` — pure functions
- `pub enum DigStatus { Ok, OutOfBounds, AlreadyEmpty, Bedrock }`
- `pub struct DigResult { pub status: DigStatus, pub ore: OreType }`
- `pub fn try_dig(grid: &mut Grid, tile: IVec2) -> DigResult`
- Pure transformation; no Bevy types.

### `marching_squares.rs` — pure functions
- `pub fn build_chunk_mesh(grid: &Grid, chunk: IVec2, chunk_tiles: u32, tile_size_px: f32) -> Mesh` — builds a Bevy `Mesh` from a slice of the Grid. (This module touches `bevy::render::mesh::Mesh` but no entities or systems; still single-responsibility.)
- 16-case lookup table of polygon shapes per cell.

### `components.rs` — Bevy markers
```rust
#[derive(Component)] pub struct Player;
#[derive(Component)] pub struct Velocity(pub Vec2);
#[derive(Component)] pub struct TerrainChunk { pub coord: IVec2 }
#[derive(Component)] pub struct ChunkDirty;
#[derive(Component)] pub struct OreSprite { pub ore: OreType }
#[derive(Component)] pub struct OreDrop { pub ore: OreType }
#[derive(Component)] pub struct MainCamera;
```

### `systems/player.rs`
- `player_movement_system` — reads `ButtonInput<KeyCode>` (W/A/S/D), sets `Velocity` on the Player entity.
- `player_apply_velocity_system` — integrates Velocity into Transform (delta-time scaled), then runs collision resolution against the Grid Resource (AABB-vs-tile-cells).
- `dig_input_system` — on LMB just-pressed and cooldown elapsed: compute target tile from cursor, check reach, call `dig::try_dig`, on success spawn an OreDrop entity and mark the owning chunk `ChunkDirty`.

### `systems/chunk_lifecycle.rs`
- `chunk_lifecycle_system` — each tick, computes visible chunk rect from camera transform + window viewport. Spawns missing `TerrainChunk` entities (with a default mesh) and inserts `ChunkDirty`. Despawns out-of-range chunks.

### `systems/chunk_render.rs`
- `chunk_remesh_system` — for each `(TerrainChunk, ChunkDirty)`, calls `marching_squares::build_chunk_mesh`, replaces the Mesh asset on the entity, removes `ChunkDirty`. Also despawns and re-spawns the chunk's child OreSprite entities.

### `systems/ore_drop.rs`
- `ore_drop_vacuum_system` — for each OreDrop, compute distance to the Player; if within vacuum radius, lerp toward player. On intersection (distance < threshold), call `inventory.add(ore, 1)`, despawn the OreDrop entity.

### `systems/hud.rs`
- `update_inventory_hud_system` — runs only when `Res<Inventory>` is `Changed`. Updates the three label texts.

### `systems/camera.rs`
- `camera_follow_system` — lerps the MainCamera toward the Player each frame.

### `systems/setup.rs`
- `setup_world` (Startup schedule) — generates the Grid via `terrain_gen::generate`, inserts as `Resource`. Spawns Player at `terrain_gen::spawn_tile`. Spawns MainCamera. Spawns HUD root and child labels.

### `app.rs`
- `pub struct MiningSimPlugin;` impl `Plugin for MiningSimPlugin` registers
  resources, components, and adds all systems to the right schedules
  (`Startup`, `Update`).

### `main.rs`
- Build a Bevy `App`, add `DefaultPlugins`, add `MiningSimPlugin`, run.

## Data flow

### Startup
1. Bevy app starts → `Startup` schedule runs `setup_world`.
2. `setup_world` calls `terrain_gen::generate(80, 200, seed)`, inserts the Grid as a Resource.
3. Inserts default `Inventory` Resource.
4. Spawns `Player` entity at the spawn-tile world position.
5. Spawns `MainCamera` entity (`Camera2d` required-component, Bevy 0.15+) positioned on the Player.
6. Spawns HUD root + three labels (one per ore).

### Per-frame
1. `chunk_lifecycle_system` — spawn/despawn chunks based on camera rect.
2. `chunk_remesh_system` — re-mesh dirty chunks (those with `ChunkDirty`).
3. `player_movement_system` — read input → `Velocity`.
4. `player_apply_velocity_system` — integrate + AABB-vs-Grid collision resolution.
5. `dig_input_system` — on LMB: try_dig → spawn OreDrop + mark chunk dirty.
6. `ore_drop_vacuum_system` — vacuum + deliver.
7. `update_inventory_hud_system` — runs only when Inventory changed.
8. `camera_follow_system` — lerp camera toward player.

System ordering uses Bevy's `.chain()` or `before/after` constraints where deterministic order matters (movement → collision; dig → chunk dirty marking → remesh).

## Cross-cutting invariants

These are the properties that make milestones 4 (netcode) and 3 (save/load) tractable later:

1. **Grid Resource is the single source of truth for terrain.** All rendering, collision, and gameplay queries read from it; only `dig::try_dig` (and `terrain_gen::generate` at startup) mutates it.
2. **Pure modules where possible.** `grid`, `terrain_gen`, `inventory`, `dig`, `marching_squares` (mesh builder) are pure and testable without Bevy `App`.
3. **No circular dependencies between systems.** Systems read shared resources; ordering is explicit via `.chain()` / `.after()` only where it matters.
4. **Deterministic procgen.** Same seed → same Grid. `bevy_replicon` and save/load both rely on this.
5. **Dig is idempotent on non-solid tiles.** `try_dig` on an already-empty tile returns `AlreadyEmpty` and does not double-spawn drops.

## Edge cases & error handling

- **Digging outside the Grid:** `try_dig` returns `OutOfBounds`; no-op.
- **Map boundary:** Outermost ring of tiles is forced to bedrock in `terrain_gen` so the player is contained without runtime bounds logic.
- **Spawn point safety:** `terrain_gen` carves a 3×3 empty pocket and ensures a non-ore solid floor tile underneath it.
- **Chunk boundaries:** Marching-squares mesher reads one-tile overlap into neighbors' Grid slices directly. Neighbors do not need to be spawned for seam correctness.
- **Chunk lifecycle:** Chunks beyond `camera_rect + 1-chunk margin` are despawned. Despawning a dirty chunk is fine — Grid is truth; respawn re-meshes.
- **Dig reach:** Fixed ~2 tiles, Player-center to tile-center, in tile units.
- **Rapid click spam:** ~0.15 s cooldown gates dig actions per-player.
- **Drop overflow:** Not specially handled. The 80×200 map's total ore tile count is bounded; pathological accumulation isn't expected in milestone 1. Revisit if a playtest produces lag from too many drops on screen.
- **Dig on already-broken tile:** `try_dig` returns `AlreadyEmpty`; no side effects.
- **Cursor outside window:** When Bevy returns no cursor position (window unfocused / off-screen), `dig_input_system` short-circuits without erroring.

### Explicitly not handled in milestone 1
- Save-file corruption (no save).
- Network desync / reconciliation (no network).
- Concurrent-dig races (no multiplayer).
- Out-of-memory from huge maps (fixed 80×200 prevents this).
- Localization, accessibility settings, settings menu.

## Testing approach

### Headless unit tests (`cargo test`)
- `grid` — set/get round-trip, bounds check, enum variants round-trip.
- `terrain_gen` — deterministic for a fixed seed; spawn pocket always carved; bedrock ring present; depth layers in correct order; ore counts inside tolerance bands.
- `inventory` — add/remove math.
- `dig` — exercised against a real Grid: tile cleared, correct ore returned, OutOfBounds handled, Bedrock rejected, idempotent on empty tile.

All four pure modules are testable without spinning up a Bevy `App`. They form the bulk of the test surface.

### Bevy `App` smoke tests (optional, lightweight)
A small integration test that builds a minimal `App` (no `DefaultPlugins`, just `MinimalPlugins` + `MiningSimPlugin`'s logic systems), inserts a tiny Grid, ticks the schedule once, and asserts that input simulation triggers the expected dig. Optional in milestone 1; added only if a regression bites us.

### Manual playtest exit-criteria for milestone 1
- [ ] Game window opens; map renders with banded layers.
- [ ] WASD movement and collision against terrain work.
- [ ] Clicking an adjacent solid tile breaks it.
- [ ] Bedrock cannot be broken.
- [ ] Ore tiles drop pickups; walking near vacuums them; HUD updates.
- [ ] Player can dig down to the bottom of the map.
- [ ] No crashes over a 15-minute session.
- [ ] Digging *feels* good. (Subjective. This is the milestone.)

### Explicitly not tested
- Visual regression (placeholder art).
- Performance benchmarks (unless manual play feels slow).
- Automated end-to-end session tests (nothing to assert in an open-ended loop).

## Open questions deferred to implementation planning
- Exact tile pixel size (16 vs 24 vs 32) will be validated in practice; 16 is the starting assumption.
- Chunk size (16×16 tiles = 256 px) is the starting assumption; may tune during performance testing.
- Whether to use `Mesh2d` with a `ColorMaterial` per layer or vertex-color a single mesh — starting with per-layer-color sub-meshes inside one chunk entity for clarity.
- Whether to gate cooldown via a `Resource<Timer>` or a per-Player component — starting with Resource since milestone 1 is single-player.
