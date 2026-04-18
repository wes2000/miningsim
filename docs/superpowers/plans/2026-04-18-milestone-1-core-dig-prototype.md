# Milestone 1 — Core Dig Prototype Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [../specs/2026-04-18-milestone-1-core-dig-prototype-design.md](../specs/2026-04-18-milestone-1-core-dig-prototype-design.md)

**Goal:** Build a single-player Godot 4 prototype where a player digs through procedurally generated, depth-banded, smooth-contoured 2D terrain, breaking tiles, picking up ore drops, and watching them stack in a HUD inventory — to answer "is digging fun?"

**Architecture:** A Godot 4 project organized into a pure data layer (Grid, TerrainGen, Inventory — fully unit-testable headlessly via GUT), a Godot-facing terrain wrapper that owns dig API and chunk lifecycle, and visual layers (chunk renderers using marching squares, player controller, ore drops, HUD). Strict downward dependency flow; the Grid is the single source of truth for terrain.

**Tech Stack:** Godot 4.3+ (latest stable), GDScript, [GUT](https://github.com/bitwes/Gut) (Godot Unit Testing) for headless tests, Git for version control.

---

## Pre-flight: environment expectations

This plan assumes:
- **Godot 4.3 or later** is installed and the CLI is reachable. On Windows, the CLI may be the same `.exe` you launch the editor with. Set `GODOT` env var or alias to the absolute path, e.g. `export GODOT="/c/Program Files/Godot/Godot_v4.3-stable_win64.exe"`. All test commands in this plan use `"$GODOT"`.
- **bash shell** (the project's working environment is Git Bash on Windows; commands use Unix-style paths and forward slashes).
- Working directory: `c:/Users/whann/Desktop/Games/miningsim` (the existing git repo, branch `main`).
- Author identity: commits use `--author="wes2000 <whannasch@gmail.com>"`. Do not modify global git config.

If any of these aren't true, stop and resolve before proceeding.

---

## File structure (target end state)

```
project.godot                          # Godot 4 project file
icon.svg                               # default project icon
addons/gut/                            # GUT testing addon (vendored)
scripts/
  main.gd
  terrain/
    grid.gd
    terrain_gen.gd
    terrain.gd
    terrain_chunk.gd
  player/
    player.gd
  items/
    ore_drop.gd
    inventory.gd
  hud/
    inventory_panel.gd
scenes/
  main.tscn
  player.tscn
  ore_drop.tscn
  hud/inventory_panel.tscn
tests/
  unit/
    test_grid.gd
    test_inventory.gd
    test_terrain_gen.gd
    test_terrain_dig.gd
  manual/
    dig_sandbox.tscn
    dig_sandbox.gd
docs/                                  # already exists
.gitignore                             # already exists; will extend
```

Each script holds one responsibility. Pure data files (`grid.gd`, `terrain_gen.gd`, `inventory.gd`) have no Godot node dependencies and are unit-tested. Visual files are smoke-tested manually via the sandbox or `main.tscn`.

---

## Conventions used in this plan

- **Commit style:** present-tense imperative, short subject; co-author trailer is optional for solo work but the user previously asked for it. Use the same `--author` flag every commit.
- **Test runs** assume `"$GODOT" --headless --path . -s addons/gut/gut_cmdln.gd -gtest=res://tests/unit/<file>.gd -gexit` (full headless GUT). Substitute `<file>` per task.
- **TDD discipline:** for every pure-data module, write a failing test first, watch it fail, then implement the minimum to pass, then commit. For visual/integration code, replace "test" with "smoke test in the editor" and document what to look for.

---

## Task 1: Initialize the Godot project

**Files:**
- Create: `project.godot`
- Create: `icon.svg`
- Modify: `.gitignore` (extend with Godot-specific entries — already present from earlier commit)

- [ ] **Step 1: Create the Godot project via the editor or CLI**

Open the Godot editor, choose "Import" → point to `c:/Users/whann/Desktop/Games/miningsim`, name the project "MiningSim", renderer: **Forward+** (or **Compatibility** for older hardware — either works for 2D), language: GDScript. Let Godot create `project.godot` and `icon.svg`.

Or via CLI from a fresh clone:
```bash
"$GODOT" --headless --path . --quit-after 1
```
(This won't create the project from scratch — easier to do via the editor on first run.)

- [ ] **Step 2: Configure the project for 2D**

In the editor: Project → Project Settings → Display → Window → set Viewport Width=1280, Height=720, Stretch Mode=`canvas_items`, Aspect=`keep`. Save and close.

- [ ] **Step 3: Verify .gitignore covers Godot artifacts**

Confirm `.gitignore` contains at least:
```
.godot/
*.import
export.cfg
export_presets.cfg
```
(Already added in the planning commit.)

- [ ] **Step 4: Commit**

```bash
cd "c:/Users/whann/Desktop/Games/miningsim"
git add project.godot icon.svg icon.svg.import 2>/dev/null || git add project.godot icon.svg
git commit --author="wes2000 <whannasch@gmail.com>" -m "Initialize Godot 4 project"
```

(`.import` files generated by Godot for icon import should also be staged.)

---

## Task 2: Vendor the GUT testing addon

**Files:**
- Create: `addons/gut/` (downloaded)
- Modify: `project.godot` (enable plugin)

- [ ] **Step 1: Download GUT 9.x**

Download the latest GUT 9.x release zip from https://github.com/bitwes/Gut/releases and extract its `addons/gut/` directory into `c:/Users/whann/Desktop/Games/miningsim/addons/gut/`.

- [ ] **Step 2: Enable the plugin**

Open the editor → Project → Project Settings → Plugins → enable "Gut". This writes a `[editor_plugins]` section into `project.godot`.

- [ ] **Step 3: Verify the headless test runner works**

Create an empty placeholder test file `tests/unit/test_smoke.gd`:

```gdscript
extends GutTest

func test_truth():
    assert_true(true)
```

Run:
```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gdir=res://tests/unit -gexit
```
Expected: GUT runs, reports 1/1 passing, exits 0. If the runner can't find `gut_cmdln.gd`, double-check the addon path.

- [ ] **Step 4: Delete the placeholder test**

```bash
rm tests/unit/test_smoke.gd
```

- [ ] **Step 5: Commit**

```bash
git add addons/ project.godot
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add GUT testing addon and verify headless runner"
```

---

## Task 3: Grid (pure data layer) — TDD

**Files:**
- Create: `scripts/terrain/grid.gd`
- Test: `tests/unit/test_grid.gd`

- [ ] **Step 1: Write failing tests for Grid**

Create `tests/unit/test_grid.gd`:

```gdscript
extends GutTest

const Grid = preload("res://scripts/terrain/grid.gd")

func test_new_grid_is_correct_size():
    var g = Grid.new(10, 20)
    assert_eq(g.width(), 10)
    assert_eq(g.height(), 20)

func test_new_grid_tiles_default_solid_dirt_no_ore():
    var g = Grid.new(3, 3)
    var t = g.get_tile(1, 1)
    assert_true(t.solid)
    assert_eq(t.layer, Grid.Layer.DIRT)
    assert_eq(t.ore, Grid.Ore.NONE)

func test_set_and_get_tile_round_trip():
    var g = Grid.new(3, 3)
    g.set_tile(1, 1, {"solid": false, "layer": Grid.Layer.STONE, "ore": Grid.Ore.SILVER})
    var t = g.get_tile(1, 1)
    assert_false(t.solid)
    assert_eq(t.layer, Grid.Layer.STONE)
    assert_eq(t.ore, Grid.Ore.SILVER)

func test_in_bounds():
    var g = Grid.new(5, 5)
    assert_true(g.in_bounds(0, 0))
    assert_true(g.in_bounds(4, 4))
    assert_false(g.in_bounds(-1, 0))
    assert_false(g.in_bounds(0, -1))
    assert_false(g.in_bounds(5, 0))
    assert_false(g.in_bounds(0, 5))

func test_get_tile_out_of_bounds_returns_null():
    var g = Grid.new(3, 3)
    assert_null(g.get_tile(-1, 0))
    assert_null(g.get_tile(3, 0))
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gtest=res://tests/unit/test_grid.gd -gexit
```
Expected: 5 failing tests (script not found / methods missing).

- [ ] **Step 3: Implement Grid**

Create `scripts/terrain/grid.gd`:

```gdscript
class_name Grid extends RefCounted

enum Layer { DIRT, STONE, DEEP, BEDROCK }
enum Ore   { NONE, COPPER, SILVER, GOLD }

var _w: int
var _h: int
var _tiles: Array  # Array of Dictionary

func _init(w: int, h: int) -> void:
    assert(w > 0 and h > 0, "Grid dimensions must be positive")
    _w = w
    _h = h
    _tiles = []
    _tiles.resize(w * h)
    for i in range(_tiles.size()):
        _tiles[i] = {"solid": true, "layer": Layer.DIRT, "ore": Ore.NONE}

func width() -> int: return _w
func height() -> int: return _h

func in_bounds(x: int, y: int) -> bool:
    return x >= 0 and y >= 0 and x < _w and y < _h

func get_tile(x: int, y: int):
    if not in_bounds(x, y):
        return null
    return _tiles[y * _w + x]

func set_tile(x: int, y: int, tile: Dictionary) -> void:
    assert(in_bounds(x, y), "set_tile out of bounds: %d,%d" % [x, y])
    _tiles[y * _w + x] = tile
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gtest=res://tests/unit/test_grid.gd -gexit
```
Expected: 5/5 passing.

- [ ] **Step 5: Commit**

```bash
git add scripts/terrain/grid.gd tests/unit/test_grid.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add Grid pure-data tile container with unit tests"
```

---

## Task 4: Inventory (pure data layer) — TDD

**Files:**
- Create: `scripts/items/inventory.gd`
- Test: `tests/unit/test_inventory.gd`

- [ ] **Step 1: Write failing tests for Inventory**

Create `tests/unit/test_inventory.gd`:

```gdscript
extends GutTest

const Inventory = preload("res://scripts/items/inventory.gd")
const Grid = preload("res://scripts/terrain/grid.gd")

func test_empty_inventory_count_is_zero():
    var inv = Inventory.new()
    assert_eq(inv.get_count(Grid.Ore.COPPER), 0)

func test_add_increments_count():
    var inv = Inventory.new()
    inv.add(Grid.Ore.COPPER, 3)
    assert_eq(inv.get_count(Grid.Ore.COPPER), 3)
    inv.add(Grid.Ore.COPPER, 2)
    assert_eq(inv.get_count(Grid.Ore.COPPER), 5)

func test_remove_decrements_count():
    var inv = Inventory.new()
    inv.add(Grid.Ore.SILVER, 5)
    inv.remove(Grid.Ore.SILVER, 2)
    assert_eq(inv.get_count(Grid.Ore.SILVER), 3)

func test_changed_signal_fires_on_add():
    var inv = Inventory.new()
    watch_signals(inv)
    inv.add(Grid.Ore.GOLD, 1)
    assert_signal_emitted_with_parameters(inv, "changed", [Grid.Ore.GOLD, 1])
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gtest=res://tests/unit/test_inventory.gd -gexit
```
Expected: 4 failing tests.

- [ ] **Step 3: Implement Inventory**

Create `scripts/items/inventory.gd`:

```gdscript
class_name Inventory extends RefCounted

signal changed(ore_type: int, new_count: int)

var _counts: Dictionary = {}

func add(ore_type: int, n: int) -> void:
    var c = _counts.get(ore_type, 0) + n
    _counts[ore_type] = c
    changed.emit(ore_type, c)

func remove(ore_type: int, n: int) -> void:
    var c = max(0, _counts.get(ore_type, 0) - n)
    _counts[ore_type] = c
    changed.emit(ore_type, c)

func get_count(ore_type: int) -> int:
    return _counts.get(ore_type, 0)
```

- [ ] **Step 4: Run tests to verify they pass**

Same command as Step 2. Expected: 4/4 passing.

- [ ] **Step 5: Commit**

```bash
git add scripts/items/inventory.gd tests/unit/test_inventory.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add Inventory with changed signal and unit tests"
```

---

## Task 5: TerrainGen (procedural generation) — TDD

**Files:**
- Create: `scripts/terrain/terrain_gen.gd`
- Test: `tests/unit/test_terrain_gen.gd`

- [ ] **Step 1: Write failing tests for TerrainGen**

Create `tests/unit/test_terrain_gen.gd`:

```gdscript
extends GutTest

const TerrainGen = preload("res://scripts/terrain/terrain_gen.gd")
const Grid = preload("res://scripts/terrain/grid.gd")

func test_generated_grid_has_requested_dimensions():
    var g = TerrainGen.generate(80, 200, 12345)
    assert_eq(g.width(), 80)
    assert_eq(g.height(), 200)

func test_outermost_ring_is_bedrock():
    var g = TerrainGen.generate(40, 60, 1)
    for x in range(g.width()):
        assert_eq(g.get_tile(x, 0).layer, Grid.Layer.BEDROCK, "top row x=%d" % x)
        assert_eq(g.get_tile(x, g.height() - 1).layer, Grid.Layer.BEDROCK, "bottom row x=%d" % x)
    for y in range(g.height()):
        assert_eq(g.get_tile(0, y).layer, Grid.Layer.BEDROCK, "left col y=%d" % y)
        assert_eq(g.get_tile(g.width() - 1, y).layer, Grid.Layer.BEDROCK, "right col y=%d" % y)

func test_surface_strip_is_walkable():
    var g = TerrainGen.generate(40, 60, 1)
    # rows 1..3 (just inside top bedrock) are non-solid surface
    for y in range(1, 4):
        for x in range(1, g.width() - 1):
            assert_false(g.get_tile(x, y).solid, "surface tile %d,%d should be non-solid" % [x, y])

func test_depth_layers_appear_in_order():
    var g = TerrainGen.generate(40, 200, 1)
    # below surface, dirt → stone → deep → (bedrock floor)
    assert_eq(g.get_tile(20, 10).layer, Grid.Layer.DIRT)
    assert_eq(g.get_tile(20, 80).layer, Grid.Layer.STONE)
    assert_eq(g.get_tile(20, 160).layer, Grid.Layer.DEEP)

func test_spawn_pocket_is_carved():
    var g = TerrainGen.generate(40, 200, 1)
    var sp = TerrainGen.spawn_tile(g)
    # 3x3 pocket centered on sp is non-solid
    for dy in range(-1, 2):
        for dx in range(-1, 2):
            assert_false(g.get_tile(sp.x + dx, sp.y + dy).solid,
                "spawn pocket tile %d,%d should be non-solid" % [sp.x + dx, sp.y + dy])
    # tile directly under the pocket is solid floor with no ore
    var floor_t = g.get_tile(sp.x, sp.y + 2)
    assert_true(floor_t.solid)
    assert_eq(floor_t.ore, Grid.Ore.NONE)

func test_deterministic_for_same_seed():
    var a = TerrainGen.generate(40, 60, 42)
    var b = TerrainGen.generate(40, 60, 42)
    for y in range(a.height()):
        for x in range(a.width()):
            assert_eq(a.get_tile(x, y), b.get_tile(x, y), "tile %d,%d mismatch" % [x, y])

func test_ore_distribution_in_tolerance():
    var g = TerrainGen.generate(80, 200, 7)
    var copper = 0
    var silver = 0
    var gold = 0
    for y in range(g.height()):
        for x in range(g.width()):
            var o = g.get_tile(x, y).ore
            if o == Grid.Ore.COPPER: copper += 1
            elif o == Grid.Ore.SILVER: silver += 1
            elif o == Grid.Ore.GOLD: gold += 1
    # Loose tolerance bands; tune later. Each ore should at least exist.
    assert_gt(copper, 50, "copper count")
    assert_gt(silver, 20, "silver count")
    assert_gt(gold, 5, "gold count")
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gtest=res://tests/unit/test_terrain_gen.gd -gexit
```
Expected: 7 failing tests.

- [ ] **Step 3: Implement TerrainGen**

Create `scripts/terrain/terrain_gen.gd`:

```gdscript
class_name TerrainGen extends RefCounted

const Grid = preload("res://scripts/terrain/grid.gd")

# Layer band fractions (of interior height, after surface strip)
const SURFACE_ROWS := 3       # rows 1..3 are walkable surface
const DIRT_FRAC := 0.30
const STONE_FRAC := 0.40
const DEEP_FRAC := 0.27
# Remaining rows form the bedrock floor.

# Per-layer ore probabilities (rough; tune via playtest).
const ORE_PROBS := {
    Grid.Layer.DIRT:  {Grid.Ore.COPPER: 0.04, Grid.Ore.SILVER: 0.005, Grid.Ore.GOLD: 0.0},
    Grid.Layer.STONE: {Grid.Ore.COPPER: 0.02, Grid.Ore.SILVER: 0.025, Grid.Ore.GOLD: 0.003},
    Grid.Layer.DEEP:  {Grid.Ore.COPPER: 0.005, Grid.Ore.SILVER: 0.015, Grid.Ore.GOLD: 0.02},
}

static func generate(width: int, height: int, seed: int) -> Grid:
    var rng = RandomNumberGenerator.new()
    rng.seed = seed
    var g = Grid.new(width, height)

    var interior_h = height - 2 - SURFACE_ROWS
    var dirt_end = 1 + SURFACE_ROWS + int(interior_h * DIRT_FRAC)
    var stone_end = dirt_end + int(interior_h * STONE_FRAC)
    var deep_end = stone_end + int(interior_h * DEEP_FRAC)

    for y in range(height):
        for x in range(width):
            var tile := {"solid": true, "layer": Grid.Layer.DIRT, "ore": Grid.Ore.NONE}
            if x == 0 or y == 0 or x == width - 1 or y == height - 1:
                tile.layer = Grid.Layer.BEDROCK  # bedrock ring
            elif y <= SURFACE_ROWS:
                tile.solid = false  # walkable surface strip
                tile.layer = Grid.Layer.DIRT
            elif y < dirt_end:
                tile.layer = Grid.Layer.DIRT
                _maybe_assign_ore(tile, rng)
            elif y < stone_end:
                tile.layer = Grid.Layer.STONE
                _maybe_assign_ore(tile, rng)
            elif y < deep_end:
                tile.layer = Grid.Layer.DEEP
                _maybe_assign_ore(tile, rng)
            else:
                tile.layer = Grid.Layer.BEDROCK  # bedrock floor band
            g.set_tile(x, y, tile)

    _carve_spawn_pocket(g)
    return g

static func spawn_tile(g: Grid) -> Vector2i:
    return Vector2i(g.width() / 2, SURFACE_ROWS + 1)

static func _maybe_assign_ore(tile: Dictionary, rng: RandomNumberGenerator) -> void:
    var probs: Dictionary = ORE_PROBS[tile.layer]
    var r = rng.randf()
    var acc = 0.0
    for ore in probs.keys():
        acc += probs[ore]
        if r < acc:
            tile.ore = ore
            return

static func _carve_spawn_pocket(g: Grid) -> void:
    var sp = spawn_tile(g)
    for dy in range(-1, 2):
        for dx in range(-1, 2):
            var t = g.get_tile(sp.x + dx, sp.y + dy)
            t.solid = false
            t.ore = Grid.Ore.NONE
    # ensure non-ore solid floor directly under pocket
    var floor_t = g.get_tile(sp.x, sp.y + 2)
    floor_t.solid = true
    floor_t.ore = Grid.Ore.NONE
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gtest=res://tests/unit/test_terrain_gen.gd -gexit
```
Expected: 7/7 passing. If the depth-layer test fails, double-check the band fractions add up close to 1.0 (they sum to 0.97; the residual feeds the bedrock floor band).

- [ ] **Step 5: Commit**

```bash
git add scripts/terrain/terrain_gen.gd tests/unit/test_terrain_gen.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add deterministic TerrainGen with layer bands and ore distribution"
```

---

## Task 6: Terrain dig API (no rendering) — TDD

**Files:**
- Create: `scripts/terrain/terrain.gd` (dig API only — chunk lifecycle added in Task 7)
- Test: `tests/unit/test_terrain_dig.gd`

- [ ] **Step 1: Write failing tests for Terrain.try_dig**

Create `tests/unit/test_terrain_dig.gd`:

```gdscript
extends GutTest

const Terrain = preload("res://scripts/terrain/terrain.gd")
const Grid = preload("res://scripts/terrain/grid.gd")

func _make_terrain() -> Terrain:
    var t = Terrain.new()
    var g = Grid.new(10, 10)
    # set everything to non-bedrock solid dirt (Grid default already)
    # add an ore tile at (3,3)
    g.set_tile(3, 3, {"solid": true, "layer": Grid.Layer.DIRT, "ore": Grid.Ore.COPPER})
    # add bedrock at (0,0)
    g.set_tile(0, 0, {"solid": true, "layer": Grid.Layer.BEDROCK, "ore": Grid.Ore.NONE})
    t.set_grid(g)
    return t

func test_dig_solid_tile_succeeds_returns_ore_type():
    var t = _make_terrain()
    var r = t.try_dig(Vector2i(3, 3))
    assert_eq(r.status, Terrain.DigStatus.OK)
    assert_eq(r.ore, Grid.Ore.COPPER)

func test_dig_clears_tile_in_grid():
    var t = _make_terrain()
    t.try_dig(Vector2i(3, 3))
    assert_false(t._grid.get_tile(3, 3).solid)

func test_dig_emits_tile_broken_signal():
    var t = _make_terrain()
    watch_signals(t)
    t.try_dig(Vector2i(3, 3))
    assert_signal_emitted(t, "tile_broken")

func test_dig_out_of_bounds_returns_oob():
    var t = _make_terrain()
    var r = t.try_dig(Vector2i(-1, 5))
    assert_eq(r.status, Terrain.DigStatus.OUT_OF_BOUNDS)

func test_dig_already_empty_returns_already_empty():
    var t = _make_terrain()
    t.try_dig(Vector2i(3, 3))
    var r = t.try_dig(Vector2i(3, 3))
    assert_eq(r.status, Terrain.DigStatus.ALREADY_EMPTY)

func test_dig_bedrock_returns_bedrock():
    var t = _make_terrain()
    var r = t.try_dig(Vector2i(0, 0))
    assert_eq(r.status, Terrain.DigStatus.BEDROCK)
    assert_true(t._grid.get_tile(0, 0).solid, "bedrock should remain solid")

func test_is_solid_query():
    var t = _make_terrain()
    assert_true(t.is_solid(Vector2i(3, 3)))
    t.try_dig(Vector2i(3, 3))
    assert_false(t.is_solid(Vector2i(3, 3)))

func test_world_to_tile_round_trip():
    var t = _make_terrain()
    t.tile_size_px = 16
    assert_eq(t.world_to_tile(Vector2(24, 40)), Vector2i(1, 2))
    assert_eq(t.tile_to_world(Vector2i(1, 2)), Vector2(24, 40))  # tile center
```

Note: tests touch `t._grid` directly to assert state — this is acceptable for unit tests of the wrapper. Once chunk rendering is added in Task 7, those tests still hold (we only ever mutate via `try_dig`).

- [ ] **Step 2: Run tests to verify they fail**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gtest=res://tests/unit/test_terrain_dig.gd -gexit
```
Expected: 8 failing tests.

- [ ] **Step 3: Implement Terrain (dig API only)**

Create `scripts/terrain/terrain.gd`:

```gdscript
class_name Terrain extends Node2D

const Grid = preload("res://scripts/terrain/grid.gd")

enum DigStatus { OK, OUT_OF_BOUNDS, ALREADY_EMPTY, BEDROCK }

signal tile_broken(tile: Vector2i, ore_type: int, world_pos: Vector2)

@export var tile_size_px: int = 16

var _grid: Grid

func set_grid(g: Grid) -> void:
    _grid = g

func is_solid(tile: Vector2i) -> bool:
    var t = _grid.get_tile(tile.x, tile.y)
    return t != null and t.solid

func try_dig(tile: Vector2i) -> Dictionary:
    var t = _grid.get_tile(tile.x, tile.y)
    if t == null:
        return {"status": DigStatus.OUT_OF_BOUNDS, "ore": Grid.Ore.NONE}
    if t.layer == Grid.Layer.BEDROCK:
        return {"status": DigStatus.BEDROCK, "ore": Grid.Ore.NONE}
    if not t.solid:
        return {"status": DigStatus.ALREADY_EMPTY, "ore": Grid.Ore.NONE}
    var ore = t.ore
    var new_t = {"solid": false, "layer": t.layer, "ore": Grid.Ore.NONE}
    _grid.set_tile(tile.x, tile.y, new_t)
    var wp = tile_to_world(tile)
    tile_broken.emit(tile, ore, wp)
    return {"status": DigStatus.OK, "ore": ore}

func world_to_tile(pos: Vector2) -> Vector2i:
    return Vector2i(int(pos.x) / tile_size_px, int(pos.y) / tile_size_px)

func tile_to_world(tile: Vector2i) -> Vector2:
    return Vector2(tile.x * tile_size_px + tile_size_px / 2.0,
                   tile.y * tile_size_px + tile_size_px / 2.0)
```

- [ ] **Step 4: Run tests to verify they pass**

Same command as Step 2. Expected: 8/8 passing.

- [ ] **Step 5: Commit**

```bash
git add scripts/terrain/terrain.gd tests/unit/test_terrain_dig.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add Terrain dig API with status enum and signal"
```

---

## Task 7: TerrainChunk renderer (marching squares, visual)

**Files:**
- Create: `scripts/terrain/terrain_chunk.gd`
- Modify: `scripts/terrain/terrain.gd` (add chunk lifecycle)

This task is visual — manually verified, not unit-tested.

- [ ] **Step 1: Implement TerrainChunk**

Create `scripts/terrain/terrain_chunk.gd`:

```gdscript
class_name TerrainChunk extends Node2D

const Grid = preload("res://scripts/terrain/grid.gd")

const CHUNK_TILES := 16

var _grid: Grid
var _chunk_x: int
var _chunk_y: int
var _tile_size: int
var _dirty: bool = true

@onready var _solid_polys: Node2D = Node2D.new()
@onready var _collision_body: StaticBody2D = StaticBody2D.new()

# Layer color palette
const LAYER_COLORS := {
    Grid.Layer.DIRT: Color(0.55, 0.42, 0.27),
    Grid.Layer.STONE: Color(0.42, 0.33, 0.22),
    Grid.Layer.DEEP: Color(0.29, 0.23, 0.15),
    Grid.Layer.BEDROCK: Color(0.16, 0.13, 0.10),
}

const ORE_COLORS := {
    Grid.Ore.COPPER: Color(0.85, 0.45, 0.20),
    Grid.Ore.SILVER: Color(0.85, 0.85, 0.92),
    Grid.Ore.GOLD: Color(0.95, 0.78, 0.25),
}

func setup(g: Grid, cx: int, cy: int, tile_size: int) -> void:
    _grid = g
    _chunk_x = cx
    _chunk_y = cy
    _tile_size = tile_size
    position = Vector2(cx * CHUNK_TILES * tile_size, cy * CHUNK_TILES * tile_size)
    add_child(_solid_polys)
    add_child(_collision_body)

func mark_dirty() -> void:
    _dirty = true

func _process(_dt: float) -> void:
    if _dirty:
        _remesh()
        _dirty = false

func _remesh() -> void:
    # clear previous draw + collision children
    for c in _solid_polys.get_children():
        c.queue_free()
    for c in _collision_body.get_children():
        c.queue_free()

    # MILESTONE 1 v1: per-tile rect rendering (placeholder for true marching squares).
    # This produces blocky output that already validates the data flow + collision; the
    # marching-squares pass is a follow-up refinement once the prototype is playable.
    # Switching to true marching squares affects ONLY this method.
    for ly in range(CHUNK_TILES):
        for lx in range(CHUNK_TILES):
            var gx = _chunk_x * CHUNK_TILES + lx
            var gy = _chunk_y * CHUNK_TILES + ly
            var t = _grid.get_tile(gx, gy)
            if t == null or not t.solid:
                continue
            var rect_pos = Vector2(lx * _tile_size, ly * _tile_size)
            var rect_size = Vector2(_tile_size, _tile_size)

            var poly = Polygon2D.new()
            poly.color = LAYER_COLORS.get(t.layer, Color.MAGENTA)
            poly.polygon = PackedVector2Array([
                rect_pos,
                rect_pos + Vector2(rect_size.x, 0),
                rect_pos + rect_size,
                rect_pos + Vector2(0, rect_size.y),
            ])
            _solid_polys.add_child(poly)

            var shape = CollisionShape2D.new()
            var rect_shape = RectangleShape2D.new()
            rect_shape.size = rect_size
            shape.shape = rect_shape
            shape.position = rect_pos + rect_size / 2
            _collision_body.add_child(shape)

            if t.ore != Grid.Ore.NONE:
                var ore_dot = Polygon2D.new()
                ore_dot.color = ORE_COLORS[t.ore]
                var c = rect_pos + rect_size / 2
                var r = _tile_size * 0.25
                ore_dot.polygon = PackedVector2Array([
                    c + Vector2(-r, -r),
                    c + Vector2(r, -r),
                    c + Vector2(r, r),
                    c + Vector2(-r, r),
                ])
                _solid_polys.add_child(ore_dot)
```

**Why blocky now, marching squares later:** The spec calls for smooth-contour marching squares. We start with per-tile rects so the data flow, collision, and ore visuals are validated end-to-end, *then* swap the `_remesh` body for a true marching-squares implementation in a follow-up task within this milestone. This keeps each commit small and individually testable.

- [ ] **Step 2: Add chunk lifecycle to Terrain**

Edit `scripts/terrain/terrain.gd` — add at the end of the script:

```gdscript
const TerrainChunk = preload("res://scripts/terrain/terrain_chunk.gd")
const CHUNK_MARGIN := 1  # extra chunks around camera rect

var _chunks: Dictionary = {}  # Vector2i(cx,cy) -> TerrainChunk

@export var camera_path: NodePath

func _process(_dt: float) -> void:
    if _grid == null or camera_path.is_empty():
        return
    var cam: Camera2D = get_node_or_null(camera_path)
    if cam == null:
        return
    var view = get_viewport_rect().size / cam.zoom
    var cam_rect = Rect2(cam.global_position - view / 2, view)
    var min_chunk = _world_to_chunk(cam_rect.position) - Vector2i(CHUNK_MARGIN, CHUNK_MARGIN)
    var max_chunk = _world_to_chunk(cam_rect.position + cam_rect.size) + Vector2i(CHUNK_MARGIN, CHUNK_MARGIN)
    var visible := {}
    for cy in range(min_chunk.y, max_chunk.y + 1):
        for cx in range(min_chunk.x, max_chunk.x + 1):
            var key = Vector2i(cx, cy)
            visible[key] = true
            if not _chunks.has(key):
                _spawn_chunk(cx, cy)
    for key in _chunks.keys():
        if not visible.has(key):
            _despawn_chunk(key)

func _world_to_chunk(pos: Vector2) -> Vector2i:
    var t = world_to_tile(pos)
    return Vector2i(t.x / TerrainChunk.CHUNK_TILES, t.y / TerrainChunk.CHUNK_TILES)

func _spawn_chunk(cx: int, cy: int) -> void:
    var chunk = TerrainChunk.new()
    add_child(chunk)
    chunk.setup(_grid, cx, cy, tile_size_px)
    _chunks[Vector2i(cx, cy)] = chunk

func _despawn_chunk(key: Vector2i) -> void:
    var chunk = _chunks[key]
    chunk.queue_free()
    _chunks.erase(key)

# call this from try_dig success path: replace the existing `tile_broken.emit(...)` line
# with this helper so chunk dirty-marking is centralized
func _on_tile_changed(tile: Vector2i) -> void:
    var ck = Vector2i(tile.x / TerrainChunk.CHUNK_TILES, tile.y / TerrainChunk.CHUNK_TILES)
    if _chunks.has(ck):
        _chunks[ck].mark_dirty()
```

Then in `try_dig`, just before `tile_broken.emit(...)`, add:
```gdscript
    _on_tile_changed(tile)
```

- [ ] **Step 3: Re-run dig unit tests to confirm no regressions**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gtest=res://tests/unit/test_terrain_dig.gd -gexit
```
Expected: 8/8 still passing. (The new `_on_tile_changed` is a no-op when `_chunks` is empty, which it is in unit tests.)

- [ ] **Step 4: Smoke-test in editor**

Create a quick scratch scene (don't commit) — Node2D root, add a Terrain instance, set its `tile_size_px=16`, then in a `_ready()` script call `set_grid(TerrainGen.generate(40, 60, 1))`. Add a Camera2D as a child, set `camera_path` on Terrain to point to it. Press Play.

Expected: a chunked, blocky map renders with visibly different colored bands (dirt/stone/deep) and small ore dots. Camera centers on origin; you may need to move the camera to see the surface area.

- [ ] **Step 5: Commit**

```bash
git add scripts/terrain/terrain_chunk.gd scripts/terrain/terrain.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add TerrainChunk renderer (per-tile placeholder) and chunk lifecycle"
```

---

## Task 8: Player controller (movement + collision)

**Files:**
- Create: `scripts/player/player.gd`
- Create: `scenes/player.tscn`

- [ ] **Step 1: Build the player scene**

In the editor: New Scene → CharacterBody2D root, name it `Player`. Add child `CollisionShape2D` with a `RectangleShape2D` of size (12, 12). Add child `Polygon2D` (the visual) with a 12×12 cyan square so the player is visible. Save as `scenes/player.tscn`. Attach `scripts/player/player.gd` (next step) to the root.

- [ ] **Step 2: Implement movement script**

Create `scripts/player/player.gd`:

```gdscript
class_name Player extends CharacterBody2D

@export var speed_px_per_s: float = 120.0

func _physics_process(_dt: float) -> void:
    var dir = Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down")
    velocity = dir * speed_px_per_s
    move_and_slide()
```

Note: WASD will arrive in Task 9 along with input map setup. Default arrows work via the built-in `ui_*` actions for the smoke test.

- [ ] **Step 3: Smoke-test movement**

Add the Player scene as a child of your scratch terrain test scene from Task 7. Play. Use arrow keys; the cyan square should move and stop against the rendered chunk collision.

Expected: smooth movement, proper collision against solid tiles, no z-order issues (player visible above terrain — adjust z-index if needed).

- [ ] **Step 4: Commit**

```bash
git add scripts/player/player.gd scenes/player.tscn
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add CharacterBody2D player with arrow-key movement"
```

---

## Task 9: Player dig input + WASD input map

**Files:**
- Modify: `scripts/player/player.gd`
- Modify: `project.godot` (input map)

- [ ] **Step 1: Define input actions**

In the editor: Project → Project Settings → Input Map. Add these actions and bindings:

| Action | Key |
|---|---|
| `move_left` | A |
| `move_right` | D |
| `move_up` | W |
| `move_down` | S |
| `dig` | Mouse Button 1 (left click) |

Save. This writes input bindings into `project.godot`.

- [ ] **Step 2: Replace input handling and add dig**

Edit `scripts/player/player.gd`:

```gdscript
class_name Player extends CharacterBody2D

const Grid = preload("res://scripts/terrain/grid.gd")

@export var speed_px_per_s: float = 120.0
@export var dig_reach_tiles: float = 2.0
@export var dig_cooldown_s: float = 0.15
@export var terrain_path: NodePath
@export var ore_drop_scene: PackedScene
@export var drops_parent_path: NodePath

var _cooldown_left: float = 0.0
var _terrain: Node = null
var _drops_parent: Node = null

func _ready() -> void:
    _terrain = get_node_or_null(terrain_path)
    _drops_parent = get_node_or_null(drops_parent_path)

func _physics_process(dt: float) -> void:
    var dir = Input.get_vector("move_left", "move_right", "move_up", "move_down")
    velocity = dir * speed_px_per_s
    move_and_slide()
    _cooldown_left = max(0.0, _cooldown_left - dt)
    if Input.is_action_pressed("dig") and _cooldown_left == 0.0:
        _try_dig()

func _try_dig() -> void:
    if _terrain == null:
        return
    var mouse_world = get_global_mouse_position()
    var tile = _terrain.world_to_tile(mouse_world)
    var tile_world_center = _terrain.tile_to_world(tile)
    var dist_tiles = (tile_world_center - global_position).length() / float(_terrain.tile_size_px)
    if dist_tiles > dig_reach_tiles:
        return
    var result = _terrain.try_dig(tile)
    _cooldown_left = dig_cooldown_s
    if result.status == _terrain.DigStatus.OK and result.ore != Grid.Ore.NONE:
        _spawn_drop(result.ore, tile_world_center)

func _spawn_drop(ore_type: int, world_pos: Vector2) -> void:
    if ore_drop_scene == null or _drops_parent == null:
        return
    var drop = ore_drop_scene.instantiate()
    drop.ore_type = ore_type
    drop.global_position = world_pos
    _drops_parent.add_child(drop)
```

- [ ] **Step 3: Smoke-test dig (will need OreDrop in next task — for now, watch tiles disappear)**

Set `ore_drop_scene` to `null` in the inspector. In your scratch scene, set Player's `terrain_path` to point at the Terrain node, leave `drops_parent_path` empty for now. Play; click on adjacent tiles. Expected: tiles vanish on click, bedrock doesn't, distant tiles ignored.

- [ ] **Step 4: Commit**

```bash
git add scripts/player/player.gd project.godot
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add WASD input map and click-to-dig with reach + cooldown"
```

---

## Task 10: OreDrop pickup entity

**Files:**
- Create: `scripts/items/ore_drop.gd`
- Create: `scenes/ore_drop.tscn`

- [ ] **Step 1: Build OreDrop scene**

New Scene → `Area2D` root, name `OreDrop`. Add `CollisionShape2D` with `CircleShape2D` radius=4. Add `Polygon2D` for visual (small diamond, color set in script per ore type). Save as `scenes/ore_drop.tscn`. Attach `scripts/items/ore_drop.gd`.

- [ ] **Step 2: Implement OreDrop**

Create `scripts/items/ore_drop.gd`:

```gdscript
class_name OreDrop extends Area2D

const Grid = preload("res://scripts/terrain/grid.gd")

const ORE_COLORS := {
    Grid.Ore.COPPER: Color(0.85, 0.45, 0.20),
    Grid.Ore.SILVER: Color(0.85, 0.85, 0.92),
    Grid.Ore.GOLD: Color(0.95, 0.78, 0.25),
}

@export var ore_type: int = Grid.Ore.NONE
@export var vacuum_radius_tiles: float = 1.0
@export var vacuum_speed_px_per_s: float = 200.0

var _player: Node2D = null
var _inventory  # Inventory ref (RefCounted)
var _tile_size_px: int = 16

func setup(player: Node2D, inv, tile_size_px: int) -> void:
    _player = player
    _inventory = inv
    _tile_size_px = tile_size_px

func _ready() -> void:
    var poly: Polygon2D = $Polygon2D
    if poly:
        poly.color = ORE_COLORS.get(ore_type, Color.MAGENTA)
    body_entered.connect(_on_body_entered)
    area_entered.connect(_on_area_entered)

func _process(dt: float) -> void:
    if _player == null:
        return
    var to_player = _player.global_position - global_position
    var dist_tiles = to_player.length() / float(_tile_size_px)
    if dist_tiles < vacuum_radius_tiles:
        global_position += to_player.normalized() * vacuum_speed_px_per_s * dt
        if to_player.length() < 6.0:
            _deliver()

func _on_body_entered(_b) -> void: _deliver()
func _on_area_entered(_a) -> void: _deliver()

func _deliver() -> void:
    if _inventory != null and ore_type != Grid.Ore.NONE:
        _inventory.add(ore_type, 1)
    queue_free()
```

- [ ] **Step 3: Wire OreDrop in Player**

Edit `scripts/player/player.gd` `_spawn_drop` to call `setup` after instantiation:

```gdscript
func _spawn_drop(ore_type: int, world_pos: Vector2) -> void:
    if ore_drop_scene == null or _drops_parent == null:
        return
    var drop = ore_drop_scene.instantiate()
    drop.ore_type = ore_type
    drop.global_position = world_pos
    _drops_parent.add_child(drop)
    drop.setup(self, _inventory_ref, _terrain.tile_size_px)
```

Also add an inventory holder field to Player:

```gdscript
var _inventory_ref  # set externally by main.gd
func set_inventory(inv) -> void:
    _inventory_ref = inv
```

**Ordering note:** `set_inventory` must be called before the first dig can occur. Task 12's `main.gd._ready` does this synchronously before the player can act, so there's no race. If you add an alternate entry point, replicate that ordering.

- [ ] **Step 4: Smoke-test pickup**

In your scratch scene, set Player's `ore_drop_scene` to `scenes/ore_drop.tscn`, add a `Drops` Node2D under World, point Player's `drops_parent_path` at it. Manually call `player.set_inventory(Inventory.new())` from a temporary `_ready` hook. Play; mine an ore tile, walk near the drop. Expected: drop lerps toward player, vanishes on contact, no errors.

- [ ] **Step 5: Commit**

```bash
git add scripts/items/ore_drop.gd scenes/ore_drop.tscn scripts/player/player.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add OreDrop pickup with auto-vacuum and inventory delivery"
```

---

## Task 11: HUD InventoryPanel

**Files:**
- Create: `scripts/hud/inventory_panel.gd`
- Create: `scenes/hud/inventory_panel.tscn`

- [ ] **Step 1: Build the HUD scene**

New Scene → `Control` root, name `InventoryPanel`. Add a `VBoxContainer` child. Inside it, add three `HBoxContainer`s — each with a `ColorRect` (16×16, ore color) and a `Label` (the count). Name them `RowCopper`, `RowSilver`, `RowGold`. Save as `scenes/hud/inventory_panel.tscn`. Attach script.

- [ ] **Step 2: Implement InventoryPanel script**

Create `scripts/hud/inventory_panel.gd`:

```gdscript
extends Control

const Grid = preload("res://scripts/terrain/grid.gd")

@onready var _row_copper: HBoxContainer = $VBoxContainer/RowCopper
@onready var _row_silver: HBoxContainer = $VBoxContainer/RowSilver
@onready var _row_gold:   HBoxContainer = $VBoxContainer/RowGold

var _inventory  # Inventory

func bind(inv) -> void:
    _inventory = inv
    if _inventory:
        _inventory.changed.connect(_on_changed)
        _refresh_all()

func _refresh_all() -> void:
    _set_row(_row_copper, _inventory.get_count(Grid.Ore.COPPER))
    _set_row(_row_silver, _inventory.get_count(Grid.Ore.SILVER))
    _set_row(_row_gold,   _inventory.get_count(Grid.Ore.GOLD))

func _on_changed(ore_type: int, new_count: int) -> void:
    match ore_type:
        Grid.Ore.COPPER: _set_row(_row_copper, new_count)
        Grid.Ore.SILVER: _set_row(_row_silver, new_count)
        Grid.Ore.GOLD:   _set_row(_row_gold,   new_count)

func _set_row(row: HBoxContainer, n: int) -> void:
    var label: Label = row.get_node("Label")
    label.text = str(n)
```

- [ ] **Step 3: Commit**

```bash
git add scripts/hud/inventory_panel.gd scenes/hud/inventory_panel.tscn
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add HUD InventoryPanel with per-ore rows"
```

---

## Task 12: Main scene wiring

**Files:**
- Create: `scripts/main.gd`
- Create: `scenes/main.tscn`
- Modify: `project.godot` (set main scene)

- [ ] **Step 1: Build main scene**

New Scene → `Node2D` root named `Main`. Children:
- `World` (Node2D)
  - `Terrain` (Node2D, attach `scripts/terrain/terrain.gd`, set `tile_size_px=16`)
  - `Ores` (Node2D)  *(reserved for milestone-2 use; can stay empty for now)*
  - `Drops` (Node2D)
  - Instance of `scenes/player.tscn` named `Player`
- `Camera2D` (set `position_smoothing_enabled = true`)
- `HUD` (CanvasLayer)
  - Instance of `scenes/hud/inventory_panel.tscn`

Set Terrain's `camera_path` to the Camera2D. Set Player's `terrain_path` to Terrain, `drops_parent_path` to Drops, `ore_drop_scene` to `scenes/ore_drop.tscn`.

Save as `scenes/main.tscn`.

- [ ] **Step 2: Implement main script**

Create `scripts/main.gd`:

```gdscript
extends Node2D

const Inventory = preload("res://scripts/items/inventory.gd")
const TerrainGen = preload("res://scripts/terrain/terrain_gen.gd")

const MAP_W := 80
const MAP_H := 200

@onready var _terrain = $World/Terrain
@onready var _player = $World/Player
@onready var _camera: Camera2D = $Camera2D
@onready var _hud_panel = $HUD/InventoryPanel

func _ready() -> void:
    var seed = randi()
    var grid = TerrainGen.generate(MAP_W, MAP_H, seed)
    _terrain.set_grid(grid)

    var inventory = Inventory.new()
    _player.set_inventory(inventory)
    _hud_panel.bind(inventory)

    var spawn_tile = TerrainGen.spawn_tile(grid)
    _player.global_position = _terrain.tile_to_world(spawn_tile)
    _camera.global_position = _player.global_position

func _process(_dt: float) -> void:
    _camera.global_position = _camera.global_position.lerp(_player.global_position, 0.15)
```

Attach the script to the Main root.

- [ ] **Step 3: Set as project main scene**

Project → Project Settings → Application → Run → Main Scene = `res://scenes/main.tscn`.

- [ ] **Step 4: Smoke-test the full loop**

Press F5. Expected:
- Map renders with banded layers.
- Player visible in the surface pocket.
- WASD moves the player; bedrock walls contain.
- LMB on adjacent solid tiles breaks them; bedrock doesn't.
- Ore tiles drop pickups, which vacuum into the inventory.
- HUD counts increment per ore type.

If anything misbehaves, that's a real bug — fix before committing.

- [ ] **Step 5: Commit**

```bash
git add scripts/main.gd scenes/main.tscn project.godot
git commit --author="wes2000 <whannasch@gmail.com>" -m "Wire main scene with terrain, player, drops, camera, and HUD"
```

---

## Task 13: Marching-squares pass on TerrainChunk

Replace the per-tile-rect placeholder in `TerrainChunk._remesh` with a marching-squares contour mesh. Done as a separate task so the previous milestone is already playable and we have a baseline to compare against.

**Files:**
- Modify: `scripts/terrain/terrain_chunk.gd`

- [ ] **Step 1: Document the marching-squares lookup**

Marching squares operates on a 2D scalar field. We treat each tile-corner as either inside-solid (1) or outside-solid (0), giving 16 cases per cell. Each case contributes a small polygon (or none) to the contour. References:
- https://en.wikipedia.org/wiki/Marching_squares
- For colored solids (this game's case), draw the *interior* polygon — the part of the cell that's "inside" the solid silhouette — colored by the dominant layer of contributing corner tiles.

- [ ] **Step 2: Replace `_remesh` with marching-squares pass**

The replacement uses corner samples between tiles, builds an interior polygon per cell from a 16-case lookup, and emits one Polygon2D per case (colored by the dominant solid corner's layer). Build the lookup as constants at the top of the script. Keep the collision body as before — block-sized rectangles per solid tile is good enough for collision; the visual is what changes.

(Implementation detail intentionally left for the engineer to derive from the standard 16-case marching-squares table — this is a well-known algorithm; importing a recipe verbatim would bloat the plan. Keep the function under ~100 lines.)

- [ ] **Step 3: Smoke-test**

Press F5. Expected: tile boundaries now look smoothed/curved rather than blocky; layer colors still correct; ore dots still visible; collision still feels right (collisions remain rect-sized so the player won't squeeze through smoothed corners — that's a known compromise, acceptable for milestone 1).

- [ ] **Step 4: If something looks wrong**, revert this commit and ship milestone 1 with the blocky placeholder. The "is digging fun" question can be answered either way; smooth contour is a quality polish, not a milestone requirement. Document the decision either way in the commit message.

- [ ] **Step 5: Commit**

```bash
git add scripts/terrain/terrain_chunk.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Render terrain with marching-squares contour"
```

---

## Task 14: Manual sandbox scene

**Files:**
- Create: `tests/manual/dig_sandbox.tscn`
- Create: `tests/manual/dig_sandbox.gd`

- [ ] **Step 1: Build sandbox scene**

Duplicate `scenes/main.tscn` to `tests/manual/dig_sandbox.tscn`. Attach `tests/manual/dig_sandbox.gd` to its root, replacing main's script.

- [ ] **Step 2: Implement sandbox script**

```gdscript
extends Node2D

const Inventory = preload("res://scripts/items/inventory.gd")
const TerrainGen = preload("res://scripts/terrain/terrain_gen.gd")

@onready var _terrain = $World/Terrain
@onready var _player  = $World/Player
@onready var _camera: Camera2D = $Camera2D
@onready var _hud_panel = $HUD/InventoryPanel

func _ready() -> void:
    # Tiny fixed-seed map for fast iteration on dig feel
    var grid = TerrainGen.generate(20, 20, 1)
    _terrain.set_grid(grid)
    var inv = Inventory.new()
    _player.set_inventory(inv)
    _hud_panel.bind(inv)
    var sp = TerrainGen.spawn_tile(grid)
    _player.global_position = _terrain.tile_to_world(sp)
    _camera.global_position = _player.global_position
```

Use this scene for rapid dig-feel iteration without waiting on the full 80×200 map to remesh.

- [ ] **Step 3: Commit**

```bash
git add tests/manual/dig_sandbox.tscn tests/manual/dig_sandbox.gd
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add tiny dig sandbox scene for feel iteration"
```

---

## Task 15: Final manual playtest & milestone wrap

- [ ] **Step 1: Run full unit suite**

```bash
"$GODOT" --headless --path . -s res://addons/gut/gut_cmdln.gd -gdir=res://tests/unit -gexit
```
Expected: all tests pass, exit 0.

- [ ] **Step 2: Manual exit-criteria walkthrough on `scenes/main.tscn`**

Run `main.tscn`. Tick each criterion only if observed:
- [ ] Game loads, banded layers visible.
- [ ] WASD movement and collision work.
- [ ] Clicking an adjacent solid tile breaks it.
- [ ] Bedrock cannot be broken.
- [ ] Ore tiles drop pickups; walking near vacuums them; HUD updates.
- [ ] Player can dig down to the bottom of the map.
- [ ] No crashes over a 15-minute session.
- [ ] Digging *feels* good (the milestone-defining subjective check).

- [ ] **Step 3: Document playtest result**

Append a short "Playtest results — milestone 1" section to `docs/roadmap.md` capturing what felt good, what felt off, and any decisions for milestone 2 (e.g. "click-per-hit feels fine at 0.15 s cooldown" or "needs more SFX punch").

- [ ] **Step 4: Commit playtest notes**

```bash
git add docs/roadmap.md
git commit --author="wes2000 <whannasch@gmail.com>" -m "Record milestone 1 playtest results"
```

- [ ] **Step 5: Push to GitHub**

```bash
git push origin main
```

Milestone 1 complete.
