# Milestone 1 — Core Dig Prototype (Design Spec)

**Date:** 2026-04-18
**Status:** Draft (awaiting spec review)
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
- 3 ore types (copper near surface, silver mid, gold deep), rarer and more valuable with depth.
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

- **Engine:** Godot 4.x (latest stable at implementation time).
- **Language:** GDScript. C# / GDExtension only if profiling demands.
- **Perspective:** Top-down 2D.
- **Platforms:** Desktop (Windows/macOS/Linux), single player.
- **Art:** Placeholder flat-color tiles and simple player shape. Art pass is
  milestone 7.

## Key design decisions

Decisions already made during brainstorming, recorded here so the
implementation plan can reference them without re-litigating:

| Decision | Choice | Why |
|---|---|---|
| Terrain look & model | Smooth contour (marching squares) over a grid | Organic feel; grid underneath keeps netcode & procgen tractable. |
| Dig feel | Click-per-hit, one tile per swing | User preference. Compensate with audio/VFX punch; cap clicks-per-tile at 2–3 across all future tool tiers to manage RSI. |
| World orientation | Thin surface strip at top edge; underground below | Matches final game structure; lets "deeper = further from surface" pay off visually even in milestone 1. |
| Procgen style | Depth layers + ore veins, no natural caves | Visible progress as you descend; near-zero extra work over flat; sets up milestone 2 tool tiers at zero gameplay cost. |
| Ore pickup | Physical drop + ~1-tile auto-vacuum | Satisfying "pop," no hunting for pickups, reuses as a pattern for later pickup-able entities. |
| Rendering approach | Custom marching-squares renderer over an explicit grid data structure | Grid as first-class truth pays off every subsequent milestone; visual matches the chosen B-terrain direction. |

## Architecture

### Scene tree

```
Main (Node2D)
├── World (Node2D)
│   ├── Terrain (Node2D)            # owns grid data + chunk renderers
│   │   └── (ChunkRenderer × N, spawned lazily as camera moves)
│   ├── Ores (Node2D)               # embedded-ore sprites, placed per chunk
│   ├── Drops (Node2D)              # physical ore drops awaiting pickup
│   └── Player (CharacterBody2D)
├── Camera2D                        # follows Player, small deadzone
└── HUD (CanvasLayer)
    └── InventoryPanel (Control)
```

### Module boundaries

Dependencies flow strictly downward. Nothing reaches back up.

```
main → terrain → grid
main → terrain → chunk_renderer → grid
main → player  → terrain
main → player  → inventory
hud → inventory
drops → inventory
```

### File layout

```
scripts/
  main.gd                       # wires world, owns run's seed + inventory
  terrain/
    grid.gd                     # pure data: 2D tile array
    terrain_gen.gd              # pure functions: seed → Grid
    terrain.gd                  # Node2D wrapper: dig API, chunk lifecycle
    terrain_chunk.gd            # one chunk renderer: mesh + collision
  player/
    player.gd                   # CharacterBody2D: move, aim, dig
  items/
    ore_drop.gd                 # Area2D: vacuum toward player, hand to inventory
    inventory.gd                # RefCounted: dict + changed signal
  hud/
    inventory_panel.gd          # subscribes to inventory, redraws rows
scenes/
  main.tscn
  player.tscn
  ore_drop.tscn
  hud/inventory_panel.tscn
tests/
  unit/                         # headless GUT tests
  manual/dig_sandbox.tscn       # 20×20 fixed grid for rapid feel iteration
```

## Components

### Grid (`grid.gd`) — pure data
- 2D array of tile records: `{ solid: bool, layer: enum(DIRT|STONE|DEEP|BEDROCK), ore: enum(NONE|COPPER|SILVER|GOLD) }`.
- Methods: `get(x,y)`, `set(x,y,tile)`, `in_bounds(x,y)`, `size()`.
- No signals, no nodes, no drawing. Fully unit-testable.

### TerrainGen (`terrain_gen.gd`) — pure functions
- `generate(width, height, seed) -> Grid`.
- Responsibilities, in order:
  1. Allocate grid, set outermost ring to solid bedrock (bounds barrier). This ring surrounds the entire map including the top, so the surface strip is bracketed by bedrock on left/right/top.
  2. Paint the rows immediately inside the top bedrock row (e.g. rows 1–3) as the surface strip: non-solid tiles the player can walk on freely, visually grass/dirt. The bedrock ring keeps the player contained.
  3. Fill interior cells as solid, assigning depth layer by row band.
  4. Sprinkle ore veins (small clusters) using per-layer probability curves — COPPER dominant near top, SILVER mid, GOLD deep.
  5. Carve a 3×3 spawn pocket just under the surface, ensure a solid non-ore floor tile underneath it.
- Deterministic for a given seed. No Godot node touched.

### Terrain (`terrain.gd`) — Godot-facing wrapper
- Holds a Grid and a dictionary of `chunk_coord → ChunkRenderer`.
- On each frame, computes the visible chunk rect from the camera plus a
  1-chunk margin. Spawns missing chunks; despawns chunks outside that rect.
- Public API:
  - `try_dig(tile: Vector2i) -> DigResult` — validates, clears, marks owning chunk dirty, emits `tile_broken(tile, ore_type, world_pos)`. Returns ore type (possibly NONE) on success, or a failure variant (OUT_OF_BOUNDS / ALREADY_EMPTY / BEDROCK).
  - `is_solid(tile: Vector2i) -> bool`.
  - `world_to_tile(pos: Vector2) -> Vector2i`, `tile_to_world(tile: Vector2i) -> Vector2`.
- **Invariant:** nothing outside Terrain mutates the Grid.

### TerrainChunk (`terrain_chunk.gd`)
- Renders one 16×16-tile slice.
- Owns: one `Polygon2D` (or MeshInstance2D) for the solid silhouette
  colored by layer, one `CollisionPolygon2D` on a `StaticBody2D` for
  collision, and a set of ore sprites placed at ore-tile positions.
- Reads its slice from Grid; uses marching-squares to emit the contour
  mesh. Reads one tile of overlap into neighbors so seams line up.
- Dirty flag; `_process` re-meshes only if dirty. Clean chunks are idle.

### Player (`player.gd`)
- `CharacterBody2D`, WASD → velocity, `move_and_slide()` against chunk
  collision polygons.
- LMB → target tile is the tile containing the cursor position
  (`Terrain.world_to_tile(get_global_mouse_position())`). If that tile is
  within reach (~2 tiles, measured Player-center to tile-center) and
  cooldown elapsed, call `Terrain.try_dig(tile)`.
- On successful dig with `ore_type != NONE`: instance an `OreDrop` at the
  tile's world position. Always: play placeholder hit SFX, trigger tiny
  camera shake. On failure (bedrock / out of reach): play "clunk" / "miss" SFX.
- Dig cooldown: ~0.15 s per-player.

### OreDrop (`ore_drop.gd`)
- `Area2D` with the ore-type sprite.
- Each frame: if Player within vacuum radius (~1 tile), lerp position toward Player.
- On overlap with Player body: `Inventory.add(ore_type, 1)`, `queue_free()`.

### Inventory (`inventory.gd`)
- `RefCounted` holding `Dictionary[ore_type: int]`.
- `add(ore, n)`, `remove(ore, n)`, `get_count(ore)`.
- Emits `changed(ore_type, new_count)` after every mutation.
- Owned by `main.gd`; passed into Player (for adds via drops) and HUD (for reads).

### InventoryPanel (`inventory_panel.gd`)
- `Control` with one row per ore type: icon + count.
- Subscribes to `Inventory.changed`, updates the affected row.
- Spartan for milestone 1; proper inventory UI arrives in later milestones.

### Main (`main.gd`)
- Picks a seed (random on new run; a dev override for reproducible testing).
- Creates Inventory, calls `TerrainGen.generate(80, 200, seed)`, hands
  Grid to Terrain, places Player at the generated spawn point, wires up
  signals.

## Data flow

### Startup
1. `Main._ready` picks a seed.
2. `Main` calls `TerrainGen.generate(80, 200, seed)` → Grid.
3. `Main` hands Grid to Terrain (no eager chunk spawn).
4. `Main` reads spawn pocket location from Grid, places Player there.
5. First `_process`: Terrain computes visible chunks from camera rect, spawns them; each meshes itself from Grid.

### Dig
1. LMB → Player computes `target_tile` from cursor.
2. Player checks dig reach and cooldown.
3. Player calls `Terrain.try_dig(target_tile)`.
4. Terrain validates against Grid; on success, clears tile and marks the owning chunk dirty, emits `tile_broken`.
5. Player (on success with ore) spawns `OreDrop` at world pos.
6. Dirty chunk re-meshes next `_process`.

### Pickup
1. `OreDrop._process`: if within vacuum radius, lerp toward Player.
2. Body overlap → `Inventory.add(ore_type, 1)` → `Inventory.changed` fires.
3. `InventoryPanel` updates the row.
4. `OreDrop.queue_free()`.

### Movement + collision
Standard Godot: Player is `CharacterBody2D`, chunks own `CollisionPolygon2D`s on `StaticBody2D`, `move_and_slide()` handles the rest.

## Cross-cutting invariants

These are the properties that make milestones 4 (netcode) and 3 (save/load) tractable later. Violating them now means paying a large refactor cost when those milestones land, so they are load-bearing even in milestone 1:

1. **Grid is the single source of truth for terrain.** Rendering, collision, and gameplay queries all read from it; only `Terrain.try_dig` (and `TerrainGen.generate` at startup) writes to it.
2. **Pure functions where possible.** `TerrainGen` is fully pure. `Grid` has no side effects. Unit-testable headlessly.
3. **No component reaches up the tree.** Dependencies flow downward only.
4. **Deterministic procgen.** Same seed → same Grid. Networking and testing both rely on this.
5. **Dig action is idempotent on non-solid tiles.** Repeated or concurrent `try_dig` on the same tile will not double-spawn drops or double-emit signals.

## Edge cases & error handling

- **Digging outside the Grid:** `try_dig` returns `OUT_OF_BOUNDS`; no-op.
- **Map boundary:** Outermost tile ring is forced to bedrock in TerrainGen.
- **Spawn point safety:** TerrainGen guarantees a 3×3 empty pocket and a solid non-ore floor tile underneath it.
- **Chunk boundaries:** Marching-squares mesher reads one-tile overlap into neighbors' Grid slices directly; no neighbor-chunk spawning required for seam correctness.
- **Chunk lifecycle:** Chunks beyond `camera_rect + 1-chunk margin` are despawned. Dirty chunk can despawn safely — Grid is truth; respawn re-meshes.
- **Dig reach:** Fixed ~2 tiles from Player center, tile-center distance. Not player-configurable in milestone 1.
- **Rapid click spam:** ~0.15 s per-player cooldown. Also acts as SFX breathing room.
- **Drop overflow:** Hard cap of ~200 `OreDrop` instances in scene. If exceeded, oldest drop self-delivers directly to Inventory. Defensive; should not trigger in normal play.
- **Dig on already-broken tile:** Idempotent — returns `ALREADY_EMPTY`.

### Explicitly not handled in milestone 1
- Save-file corruption (no save).
- Network desync / reconciliation (no network).
- Concurrent-dig races (no multiplayer).
- Out-of-memory from huge maps (fixed 80×200 prevents this).
- Localization, accessibility settings, settings menu.

## Testing approach

### Headless unit tests (GUT or equivalent)
- `grid.gd`: set/get round-trip, bounds check, enum round-trip.
- `terrain_gen.gd`: deterministic for a fixed seed; spawn pocket always carved; bedrock ring present; depth layers in correct order; ore counts inside tolerance bands for per-layer probabilities.
- `terrain.gd` dig logic: exercised against a Grid without any ChunkRenderers; asserts tile cleared, correct signal emitted, correct ore returned, OUT_OF_BOUNDS handled, bedrock rejected, idempotent on empty tile.
- `inventory.gd`: add/remove math; `changed` signal fires with correct args.

### Manual sandbox
- `tests/manual/dig_sandbox.tscn` — 20×20 fixed grid, Player at known spot. Fast iteration for feel testing, independent of main.tscn.

### Manual playtest exit-criteria for milestone 1
- [ ] Game loads, shows banded layers.
- [ ] WASD movement and collision work.
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
- Whether to use `Polygon2D` vs `MeshInstance2D` for chunk rendering depends on whether we want shader flexibility for a future polish pass. Start with `Polygon2D`; upgrade only if a concrete need appears.
- Whether GUT is the right test framework or a minimal custom runner suffices for so few tests.
