# Milestone 1 — Core Dig Prototype Implementation Plan (Bevy)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [../specs/2026-04-18-milestone-1-core-dig-prototype-design.md](../specs/2026-04-18-milestone-1-core-dig-prototype-design.md)

**Goal:** Build a single-player Bevy 2D prototype where a player digs through procedurally generated, depth-banded, smooth-contoured 2D terrain, breaking tiles, picking up ore drops, and watching them stack in a HUD inventory — to answer "is digging fun?"

**Architecture:** Bevy ECS app. The `Grid` resource is the single source of truth for terrain. Pure modules (`grid`, `terrain_gen`, `inventory`, `dig`, `marching_squares`) are unit-tested with `cargo test` and have no Bevy dependencies (except where a mesh/IVec2 type is unavoidable). Bevy systems wrap these pure modules to integrate with input, rendering, and HUD. Strict downward dependency flow.

**Tech Stack:** Rust (stable), Bevy 0.15.x, `glam` (transitively via Bevy), `rand` for procgen, built-in `cargo test`.

---

## Pre-flight: environment expectations

This plan assumes:
- **Rust toolchain (stable)** installed and `cargo` reachable. Verify with `cargo --version`. Plan was written for `rustc 1.80+`.
- **Linker / build essentials** — On Windows, MSVC build tools (Visual Studio Build Tools "Desktop development with C++"). On Linux, `pkg-config`, `libudev-dev`, `libasound2-dev`, etc., per Bevy's [Linux dependencies](https://bevyengine.org/learn/quick-start/getting-started/setup/#installing-os-dependencies).
- **bash shell** (Git Bash on Windows). Commands use Unix-style paths and forward slashes.
- Working directory: `c:/Users/whann/Desktop/Games/miningsim` (existing git repo, branch `milestone-1`).
- Author identity: commits use `--author="wes2000 <whannasch@gmail.com>"`. Do not modify global git config.

If any of these aren't true, stop and resolve before proceeding.

---

## File structure (target end state)

```
Cargo.toml
.gitignore                    # already exists; will extend with /target
src/
  main.rs
  lib.rs
  app.rs
  grid.rs
  terrain_gen.rs
  inventory.rs
  dig.rs
  marching_squares.rs
  components.rs
  systems/
    mod.rs
    setup.rs
    player.rs
    camera.rs
    chunk_lifecycle.rs
    chunk_render.rs
    ore_drop.rs
    hud.rs
tests/
  grid.rs
  terrain_gen.rs
  inventory.rs
  dig.rs
docs/                          # already exists
```

Each pure module has in-file `#[cfg(test)] mod tests {}` AND a sibling
integration test in `tests/` exercising the public API. Bevy systems are
not unit-tested directly; they are exercised by manual smoke tests and
final playtest.

---

## Conventions used in this plan

- **Commit style:** present-tense imperative, short subject. Use the same `--author` flag every commit.
- **Test runs:** `cargo test --lib --tests` runs both in-file and integration tests for the library crate.
- **TDD discipline for pure modules:** write a failing test first, watch it fail (`cargo test <name>` reports failures), implement the minimum to pass, then commit. For Bevy systems (which require a windowed app to verify), replace "test" with "smoke test by running the binary" and document what to look for.
- **Bevy version assumption:** **0.15.x**. APIs have shifted across recent Bevy releases; if `cargo` resolves a different minor version, expect minor naming differences (`Sprite`/`SpriteBundle`, `Color::srgb`/`Color::rgb`, etc.). Adapt to current API; the algorithm and data flow remain valid.

---

## Task 1: Initialize the Cargo project & add Bevy

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`, `src/lib.rs`
- Modify: `.gitignore` (add `/target`)

- [ ] **Step 1: Verify Rust toolchain**

```bash
cd "c:/Users/whann/Desktop/Games/miningsim"
cargo --version
rustc --version
```
Expected: cargo 1.80 or later, rustc 1.80 or later. If absent, install via https://rustup.rs/ before proceeding.

- [ ] **Step 2: Initialize Cargo crate in the existing directory**

```bash
cargo init --name miningsim --vcs none --bin
```

`--vcs none` because the git repo already exists. `--bin` because we'll add `lib.rs` ourselves for tests.

- [ ] **Step 3: Add `/target` to .gitignore**

Append to `.gitignore`:
```
/target
Cargo.lock
```
(Yes, `Cargo.lock` is gitignored for libraries, but per Bevy convention, since this is a binary, the standard advice is to commit it. We'll commit it. Remove the `Cargo.lock` line if you prefer the binary convention — but starting without it keeps merge conflicts down during early prototyping.)

Decision for this plan: **gitignore `Cargo.lock`** during prototyping; revisit before milestone 4 (when reproducible netcode benefits from a locked dep tree).

- [ ] **Step 4: Replace generated `src/main.rs` and add `src/lib.rs`**

Replace `src/main.rs`:
```rust
use bevy::prelude::*;
use miningsim::app::MiningSimPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MiningSim — Milestone 1".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MiningSimPlugin)
        .run();
}
```

Create `src/lib.rs`:
```rust
pub mod app;
pub mod components;
pub mod dig;
pub mod grid;
pub mod inventory;
pub mod marching_squares;
pub mod systems;
pub mod terrain_gen;
```

(Some of these modules don't exist yet. They'll be added in subsequent tasks. The compiler will error here until they exist — that's expected; we add them as we go. To avoid that during Task 1, create empty stub files in Step 5.)

- [ ] **Step 5: Add stub modules so the crate compiles**

```bash
mkdir -p src/systems
touch src/grid.rs src/terrain_gen.rs src/inventory.rs src/dig.rs src/marching_squares.rs src/components.rs src/app.rs src/systems/mod.rs
```

In each stub, write the minimum to make the crate compile:

`src/grid.rs`, `src/terrain_gen.rs`, `src/inventory.rs`, `src/dig.rs`, `src/marching_squares.rs`, `src/components.rs`, `src/systems/mod.rs`: leave empty.

`src/app.rs`:
```rust
use bevy::prelude::*;

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, _app: &mut App) {
        // systems added in later tasks
    }
}
```

- [ ] **Step 6: Replace `Cargo.toml` with explicit Bevy dependency**

Overwrite `Cargo.toml`:
```toml
[package]
name = "miningsim"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = "0.15"
rand = "0.8"

[profile.dev]
opt-level = 1            # Bevy is much faster with some opt even in dev

[profile.dev.package."*"]
opt-level = 3            # Crank deps to release; only the local crate stays at -O1
```

The dev-profile tweaks are the standard Bevy guidance — without them, dev iteration on a Bevy app is painfully slow.

- [ ] **Step 7: Build & run**

```bash
cargo build
cargo run
```

Expected on first build: a long compile (Bevy + dependencies, several minutes). Then a window opens with a black screen titled "MiningSim — Milestone 1." Close it.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml .gitignore src/
git commit --author="wes2000 <whannasch@gmail.com>" -m "Initialize Bevy crate with empty MiningSimPlugin and stub modules"
```

---

## Task 2: Grid module (pure data) — TDD

**Files:**
- Modify: `src/grid.rs`
- Create: `tests/grid.rs`

- [ ] **Step 1: Write failing tests in `tests/grid.rs`**

```rust
use miningsim::grid::{Grid, Layer, OreType, Tile};

#[test]
fn new_grid_has_requested_dimensions() {
    let g = Grid::new(10, 20);
    assert_eq!(g.width(), 10);
    assert_eq!(g.height(), 20);
}

#[test]
fn new_grid_default_tiles_are_solid_dirt_no_ore() {
    let g = Grid::new(3, 3);
    let t = g.get(1, 1).expect("in bounds");
    assert!(t.solid);
    assert_eq!(t.layer, Layer::Dirt);
    assert_eq!(t.ore, OreType::None);
}

#[test]
fn set_and_get_round_trip() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: false, layer: Layer::Stone, ore: OreType::Silver });
    let t = g.get(1, 1).unwrap();
    assert!(!t.solid);
    assert_eq!(t.layer, Layer::Stone);
    assert_eq!(t.ore, OreType::Silver);
}

#[test]
fn in_bounds_check() {
    let g = Grid::new(5, 5);
    assert!(g.in_bounds(0, 0));
    assert!(g.in_bounds(4, 4));
    assert!(!g.in_bounds(-1, 0));
    assert!(!g.in_bounds(0, -1));
    assert!(!g.in_bounds(5, 0));
    assert!(!g.in_bounds(0, 5));
}

#[test]
fn get_out_of_bounds_returns_none() {
    let g = Grid::new(3, 3);
    assert!(g.get(-1, 0).is_none());
    assert!(g.get(3, 0).is_none());
}

#[test]
#[should_panic]
fn set_out_of_bounds_panics() {
    let mut g = Grid::new(3, 3);
    g.set(5, 5, Tile { solid: true, layer: Layer::Dirt, ore: OreType::None });
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test grid
```
Expected: compile error (Grid not defined).

- [ ] **Step 3: Implement Grid**

Replace `src/grid.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Dirt,
    Stone,
    Deep,
    Bedrock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OreType {
    None,
    Copper,
    Silver,
    Gold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub solid: bool,
    pub layer: Layer,
    pub ore: OreType,
}

impl Default for Tile {
    fn default() -> Self {
        Self { solid: true, layer: Layer::Dirt, ore: OreType::None }
    }
}

#[derive(Debug, bevy::prelude::Resource)]
pub struct Grid {
    width: u32,
    height: u32,
    tiles: Vec<Tile>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Grid dims must be positive");
        let tiles = vec![Tile::default(); (width * height) as usize];
        Self { width, height, tiles }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&Tile> {
        if !self.in_bounds(x, y) { return None; }
        Some(&self.tiles[self.idx(x, y)])
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Tile> {
        if !self.in_bounds(x, y) { return None; }
        let i = self.idx(x, y);
        Some(&mut self.tiles[i])
    }

    pub fn set(&mut self, x: i32, y: i32, t: Tile) {
        assert!(self.in_bounds(x, y), "set out of bounds: {},{}", x, y);
        let i = self.idx(x, y);
        self.tiles[i] = t;
    }

    fn idx(&self, x: i32, y: i32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }
}
```

The `Resource` derive comes from Bevy — pulling Bevy in here keeps the Grid usable as a Bevy resource directly. This is a pragmatic compromise: it's an annotation only and the rest of the module is pure logic.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --test grid
```
Expected: 6/6 passing.

- [ ] **Step 5: Commit**

```bash
git add src/grid.rs tests/grid.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add Grid data structure with unit tests"
```

---

## Task 3: Inventory module (pure data) — TDD

**Files:**
- Modify: `src/inventory.rs`
- Create: `tests/inventory.rs`

- [ ] **Step 1: Write failing tests in `tests/inventory.rs`**

```rust
use miningsim::grid::OreType;
use miningsim::inventory::Inventory;

#[test]
fn empty_inventory_returns_zero() {
    let inv = Inventory::default();
    assert_eq!(inv.get(OreType::Copper), 0);
}

#[test]
fn add_increments_count() {
    let mut inv = Inventory::default();
    inv.add(OreType::Copper, 3);
    assert_eq!(inv.get(OreType::Copper), 3);
    inv.add(OreType::Copper, 2);
    assert_eq!(inv.get(OreType::Copper), 5);
}

#[test]
fn remove_decrements_count_floored_at_zero() {
    let mut inv = Inventory::default();
    inv.add(OreType::Silver, 5);
    inv.remove(OreType::Silver, 2);
    assert_eq!(inv.get(OreType::Silver), 3);
    inv.remove(OreType::Silver, 100);
    assert_eq!(inv.get(OreType::Silver), 0);
}

#[test]
fn add_one_ore_does_not_affect_others() {
    let mut inv = Inventory::default();
    inv.add(OreType::Gold, 1);
    assert_eq!(inv.get(OreType::Copper), 0);
    assert_eq!(inv.get(OreType::Silver), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test inventory
```
Expected: compile error.

- [ ] **Step 3: Implement Inventory**

Replace `src/inventory.rs`:
```rust
use std::collections::HashMap;
use crate::grid::OreType;

#[derive(Debug, Default, bevy::prelude::Resource)]
pub struct Inventory {
    counts: HashMap<OreType, u32>,
}

impl Inventory {
    pub fn add(&mut self, ore: OreType, n: u32) {
        if ore == OreType::None { return; }
        *self.counts.entry(ore).or_insert(0) += n;
    }

    pub fn remove(&mut self, ore: OreType, n: u32) {
        let c = self.counts.entry(ore).or_insert(0);
        *c = c.saturating_sub(n);
    }

    pub fn get(&self, ore: OreType) -> u32 {
        *self.counts.get(&ore).unwrap_or(&0)
    }
}
```

`OreType` needs `Hash` for `HashMap` keys. Update `src/grid.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OreType { ... }
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --test inventory
```
Expected: 4/4 passing.

- [ ] **Step 5: Commit**

```bash
git add src/inventory.rs src/grid.rs tests/inventory.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add Inventory resource with add/remove/get and unit tests"
```

---

## Task 4: TerrainGen module (pure functions) — TDD

**Files:**
- Modify: `src/terrain_gen.rs`
- Create: `tests/terrain_gen.rs`

- [ ] **Step 1: Write failing tests in `tests/terrain_gen.rs`**

```rust
use miningsim::grid::{Grid, Layer, OreType};
use miningsim::terrain_gen;

#[test]
fn generated_grid_has_requested_dimensions() {
    let g = terrain_gen::generate(80, 200, 12345);
    assert_eq!(g.width(), 80);
    assert_eq!(g.height(), 200);
}

#[test]
fn outermost_ring_is_bedrock() {
    let g = terrain_gen::generate(40, 60, 1);
    for x in 0..g.width() as i32 {
        assert_eq!(g.get(x, 0).unwrap().layer, Layer::Bedrock);
        assert_eq!(g.get(x, g.height() as i32 - 1).unwrap().layer, Layer::Bedrock);
    }
    for y in 0..g.height() as i32 {
        assert_eq!(g.get(0, y).unwrap().layer, Layer::Bedrock);
        assert_eq!(g.get(g.width() as i32 - 1, y).unwrap().layer, Layer::Bedrock);
    }
}

#[test]
fn surface_strip_is_walkable() {
    let g = terrain_gen::generate(40, 60, 1);
    for y in 1..=3i32 {
        for x in 1..(g.width() as i32 - 1) {
            assert!(!g.get(x, y).unwrap().solid, "surface tile {},{} should be non-solid", x, y);
        }
    }
}

#[test]
fn depth_layers_appear_in_order() {
    let g = terrain_gen::generate(40, 200, 1);
    assert_eq!(g.get(20, 10).unwrap().layer, Layer::Dirt);
    assert_eq!(g.get(20, 80).unwrap().layer, Layer::Stone);
    assert_eq!(g.get(20, 160).unwrap().layer, Layer::Deep);
}

#[test]
fn spawn_pocket_is_carved() {
    let g = terrain_gen::generate(40, 200, 1);
    let sp = terrain_gen::spawn_tile(&g);
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            let t = g.get(sp.0 + dx, sp.1 + dy).unwrap();
            assert!(!t.solid, "spawn pocket tile ({},{}) should be non-solid",
                    sp.0 + dx, sp.1 + dy);
        }
    }
    let floor_t = g.get(sp.0, sp.1 + 2).unwrap();
    assert!(floor_t.solid);
    assert_eq!(floor_t.ore, OreType::None);
}

#[test]
fn deterministic_for_same_seed() {
    let a = terrain_gen::generate(40, 60, 42);
    let b = terrain_gen::generate(40, 60, 42);
    for y in 0..a.height() as i32 {
        for x in 0..a.width() as i32 {
            assert_eq!(a.get(x, y), b.get(x, y), "tile {},{} mismatch", x, y);
        }
    }
}

#[test]
fn ore_distribution_in_tolerance() {
    let g = terrain_gen::generate(80, 200, 7);
    let mut copper = 0;
    let mut silver = 0;
    let mut gold = 0;
    for y in 0..g.height() as i32 {
        for x in 0..g.width() as i32 {
            match g.get(x, y).unwrap().ore {
                OreType::Copper => copper += 1,
                OreType::Silver => silver += 1,
                OreType::Gold => gold += 1,
                OreType::None => {}
            }
        }
    }
    // Loose existence + relative-ordering assertions, kept brittle-resistant
    // to ore-prob tuning. We assert each ore exists at all and that copper
    // (most common in dirt) outnumbers gold (only generated in deep).
    assert!(copper > 0, "expected some copper");
    assert!(silver > 0, "expected some silver");
    assert!(gold > 0,   "expected some gold");
    assert!(copper > gold, "copper should be more common than gold ({} vs {})", copper, gold);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test terrain_gen
```
Expected: compile error.

- [ ] **Step 3: Implement TerrainGen**

Replace `src/terrain_gen.rs`:
```rust
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::grid::{Grid, Layer, OreType, Tile};

const SURFACE_ROWS: i32 = 3;
const DIRT_FRAC: f32 = 0.30;
const STONE_FRAC: f32 = 0.40;
const DEEP_FRAC: f32 = 0.27;

fn ore_probs(layer: Layer) -> [(OreType, f32); 3] {
    match layer {
        Layer::Dirt  => [(OreType::Copper, 0.04),  (OreType::Silver, 0.005), (OreType::Gold, 0.0)],
        Layer::Stone => [(OreType::Copper, 0.02),  (OreType::Silver, 0.025), (OreType::Gold, 0.003)],
        Layer::Deep  => [(OreType::Copper, 0.005), (OreType::Silver, 0.015), (OreType::Gold, 0.02)],
        Layer::Bedrock => [(OreType::None, 0.0); 3],
    }
}

pub fn generate(width: u32, height: u32, seed: u64) -> Grid {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut g = Grid::new(width, height);

    let interior_h = (height as i32) - 2 - SURFACE_ROWS;
    let dirt_end  = 1 + SURFACE_ROWS + (interior_h as f32 * DIRT_FRAC) as i32;
    let stone_end = dirt_end + (interior_h as f32 * STONE_FRAC) as i32;
    let deep_end  = stone_end + (interior_h as f32 * DEEP_FRAC) as i32;

    for y in 0..(height as i32) {
        for x in 0..(width as i32) {
            let mut tile = Tile::default();
            if x == 0 || y == 0 || x == width as i32 - 1 || y == height as i32 - 1 {
                tile.layer = Layer::Bedrock;
            } else if y <= SURFACE_ROWS {
                tile.solid = false;
                tile.layer = Layer::Dirt;
            } else if y < dirt_end {
                tile.layer = Layer::Dirt;
                maybe_assign_ore(&mut tile, &mut rng);
            } else if y < stone_end {
                tile.layer = Layer::Stone;
                maybe_assign_ore(&mut tile, &mut rng);
            } else if y < deep_end {
                tile.layer = Layer::Deep;
                maybe_assign_ore(&mut tile, &mut rng);
            } else {
                tile.layer = Layer::Bedrock;
            }
            g.set(x, y, tile);
        }
    }

    carve_spawn_pocket(&mut g);
    g
}

pub fn spawn_tile(g: &Grid) -> (i32, i32) {
    ((g.width() / 2) as i32, SURFACE_ROWS + 1)
}

fn maybe_assign_ore(tile: &mut Tile, rng: &mut StdRng) {
    let probs = ore_probs(tile.layer);
    let r: f32 = rng.gen();
    let mut acc = 0.0;
    for (ore, p) in probs {
        acc += p;
        if r < acc {
            tile.ore = ore;
            return;
        }
    }
}

fn carve_spawn_pocket(g: &mut Grid) {
    let sp = spawn_tile(g);
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            if let Some(t) = g.get_mut(sp.0 + dx, sp.1 + dy) {
                t.solid = false;
                t.ore = OreType::None;
            }
        }
    }
    if let Some(t) = g.get_mut(sp.0, sp.1 + 2) {
        t.solid = true;
        t.ore = OreType::None;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --test terrain_gen
```
Expected: 7/7 passing.

- [ ] **Step 5: Commit**

```bash
git add src/terrain_gen.rs tests/terrain_gen.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add deterministic TerrainGen with layer bands and ore distribution"
```

---

## Task 5: Dig module (pure logic) — TDD

**Files:**
- Modify: `src/dig.rs`
- Create: `tests/dig.rs`

- [ ] **Step 1: Write failing tests in `tests/dig.rs`**

```rust
use miningsim::grid::{Grid, Layer, OreType, Tile};
use miningsim::dig::{self, DigStatus};

fn make_grid() -> Grid {
    let mut g = Grid::new(10, 10);
    g.set(3, 3, Tile { solid: true, layer: Layer::Dirt, ore: OreType::Copper });
    g.set(0, 0, Tile { solid: true, layer: Layer::Bedrock, ore: OreType::None });
    g
}

#[test]
fn dig_solid_tile_returns_ok_with_ore() {
    let mut g = make_grid();
    let r = dig::try_dig(&mut g, 3, 3);
    assert_eq!(r.status, DigStatus::Ok);
    assert_eq!(r.ore, OreType::Copper);
}

#[test]
fn dig_clears_tile() {
    let mut g = make_grid();
    dig::try_dig(&mut g, 3, 3);
    assert!(!g.get(3, 3).unwrap().solid);
}

#[test]
fn dig_out_of_bounds_returns_oob() {
    let mut g = make_grid();
    let r = dig::try_dig(&mut g, -1, 5);
    assert_eq!(r.status, DigStatus::OutOfBounds);
}

#[test]
fn dig_already_empty_returns_already_empty() {
    let mut g = make_grid();
    dig::try_dig(&mut g, 3, 3);
    let r = dig::try_dig(&mut g, 3, 3);
    assert_eq!(r.status, DigStatus::AlreadyEmpty);
}

#[test]
fn dig_bedrock_returns_bedrock_and_keeps_solid() {
    let mut g = make_grid();
    let r = dig::try_dig(&mut g, 0, 0);
    assert_eq!(r.status, DigStatus::Bedrock);
    assert!(g.get(0, 0).unwrap().solid);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test dig
```
Expected: compile error.

- [ ] **Step 3: Implement Dig**

Replace `src/dig.rs`:
```rust
use crate::grid::{Grid, Layer, OreType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigStatus {
    Ok,
    OutOfBounds,
    AlreadyEmpty,
    Bedrock,
}

#[derive(Debug, Clone, Copy)]
pub struct DigResult {
    pub status: DigStatus,
    pub ore: OreType,
}

pub fn try_dig(grid: &mut Grid, x: i32, y: i32) -> DigResult {
    let tile_opt = grid.get(x, y).copied();
    let Some(t) = tile_opt else {
        return DigResult { status: DigStatus::OutOfBounds, ore: OreType::None };
    };
    if t.layer == Layer::Bedrock {
        return DigResult { status: DigStatus::Bedrock, ore: OreType::None };
    }
    if !t.solid {
        return DigResult { status: DigStatus::AlreadyEmpty, ore: OreType::None };
    }
    let ore = t.ore;
    grid.set(x, y, crate::grid::Tile { solid: false, layer: t.layer, ore: OreType::None });
    DigResult { status: DigStatus::Ok, ore }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --test dig
```
Expected: 5/5 passing.

- [ ] **Step 5: Commit**

```bash
git add src/dig.rs tests/dig.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add try_dig pure function with status enum and unit tests"
```

---

## Task 6: Components & systems module skeleton

**Files:**
- Modify: `src/components.rs`
- Modify: `src/systems/mod.rs`
- Create: `src/systems/setup.rs`, `src/systems/player.rs`, `src/systems/camera.rs`, `src/systems/chunk_lifecycle.rs`, `src/systems/chunk_render.rs`, `src/systems/ore_drop.rs`, `src/systems/hud.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Define components**

Replace `src/components.rs`:
```rust
use bevy::prelude::*;
use crate::grid::OreType;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct TerrainChunk {
    pub coord: IVec2,
}

#[derive(Component)]
pub struct ChunkDirty;

#[derive(Component)]
pub struct OreSprite {
    pub ore: OreType,
}

#[derive(Component)]
pub struct OreDrop {
    pub ore: OreType,
}

#[derive(Component)]
pub struct MainCamera;
```

- [ ] **Step 2: Stub the systems modules**

Replace `src/systems/mod.rs`:
```rust
pub mod camera;
pub mod chunk_lifecycle;
pub mod chunk_render;
pub mod hud;
pub mod ore_drop;
pub mod player;
pub mod setup;
```

In each system file, add a placeholder so the crate compiles:
```rust
use bevy::prelude::*;

// systems will be added in subsequent tasks
```

- [ ] **Step 3: Verify compile**

```bash
cargo build
```
Expected: success (warnings about unused imports are fine).

- [ ] **Step 4: Commit**

```bash
git add src/components.rs src/systems/
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add component markers and systems module skeleton"
```

---

## Task 7: Setup system + camera (visible window with a single sprite)

**Files:**
- Modify: `src/systems/setup.rs`, `src/systems/camera.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Implement setup system**

`src/systems/setup.rs`:
```rust
use bevy::prelude::*;
use crate::components::{MainCamera, Player, Velocity};
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::terrain_gen;

pub const TILE_SIZE_PX: f32 = 16.0;
pub const MAP_W: u32 = 80;
pub const MAP_H: u32 = 200;

pub fn setup_world(mut commands: Commands) {
    let seed: u64 = rand::random();
    info!("world seed: {}", seed);     // logged so playtests can be reproduced
    let grid = terrain_gen::generate(MAP_W, MAP_H, seed);
    let sp = terrain_gen::spawn_tile(&grid);
    let player_world = tile_center_world(sp.0, sp.1);

    commands.insert_resource(grid);
    commands.insert_resource(Inventory::default());

    // Player
    commands.spawn((
        Player,
        Velocity::default(),
        Sprite {
            color: Color::srgb(0.30, 0.60, 0.90),
            custom_size: Some(Vec2::splat(12.0)),
            ..default()
        },
        Transform::from_translation(player_world.extend(10.0)),
    ));

    // Camera
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_translation(player_world.extend(100.0)),
    ));
}

pub fn tile_center_world(x: i32, y: i32) -> Vec2 {
    Vec2::new(
        x as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0,
        // invert Y so deeper tiles render below in world (Bevy Y goes up)
        -(y as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0),
    )
}
```

(Note: the `Sprite { color, custom_size, .. }` form is Bevy 0.15's required-component pattern; older versions use `SpriteBundle`. Adapt as needed.)

- [ ] **Step 2: Implement camera follow**

`src/systems/camera.rs`:
```rust
use bevy::prelude::*;
use crate::components::{MainCamera, Player};

pub fn camera_follow_system(
    player_q: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    let Ok(p) = player_q.get_single() else { return };
    let Ok(mut c) = cam_q.get_single_mut() else { return };
    let target = p.translation.truncate().extend(c.translation.z);
    let t = (time.delta_secs() * 6.0).clamp(0.0, 1.0);
    c.translation = c.translation.lerp(target, t);
}
```

- [ ] **Step 3: Wire systems in MiningSimPlugin**

Replace `src/app.rs`:
```rust
use bevy::prelude::*;

use crate::systems::{camera, setup};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup::setup_world)
            .add_systems(Update, camera::camera_follow_system);
    }
}
```

- [ ] **Step 4: Run and smoke-test**

```bash
cargo run
```
Expected: window opens, you see a single small blue square (the player) on a black background. Camera centered on it. Close the window.

- [ ] **Step 5: Commit**

```bash
git add src/systems/setup.rs src/systems/camera.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add setup system + camera follow; player sprite visible"
```

---

## Task 8: Player movement (input → velocity → translation, no collision)

**Files:**
- Modify: `src/systems/player.rs`, `src/app.rs`

- [ ] **Step 1: Implement movement systems**

Replace `src/systems/player.rs`:
```rust
use bevy::prelude::*;
use crate::components::{Player, Velocity};

pub const PLAYER_SPEED_PX_PER_S: f32 = 120.0;

pub fn read_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Velocity, With<Player>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO { dir = dir.normalize(); }
    for mut v in q.iter_mut() {
        v.0 = dir * PLAYER_SPEED_PX_PER_S;
    }
}

pub fn apply_velocity_system(
    time: Res<Time>,
    mut q: Query<(&Velocity, &mut Transform), With<Player>>,
) {
    let dt = time.delta_secs();
    for (v, mut t) in q.iter_mut() {
        t.translation.x += v.0.x * dt;
        t.translation.y += v.0.y * dt;
    }
}
```

- [ ] **Step 2: Register in plugin**

Edit `src/app.rs` `Update` registration:
```rust
.add_systems(Update, (
    crate::systems::player::read_input_system,
    crate::systems::player::apply_velocity_system,
    camera::camera_follow_system,
).chain())
```

- [ ] **Step 3: Smoke-test**

```bash
cargo run
```
Expected: WASD moves the blue square; camera follows.

- [ ] **Step 4: Commit**

```bash
git add src/systems/player.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add WASD movement system with velocity integration"
```

---

## Task 9: Chunk lifecycle + naive blocky rendering (validates data flow)

**Files:**
- Modify: `src/systems/chunk_lifecycle.rs`, `src/systems/chunk_render.rs`, `src/app.rs`

This task does NOT yet implement marching squares — it draws each solid tile as a colored square so we can verify Grid → on-screen pipeline before tackling contour meshing in Task 12.

- [ ] **Step 1: Implement chunk lifecycle system**

`src/systems/chunk_lifecycle.rs`:
```rust
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::components::{ChunkDirty, MainCamera, TerrainChunk};
use crate::grid::Grid;
use crate::systems::setup::TILE_SIZE_PX;

pub const CHUNK_TILES: i32 = 16;
pub const CHUNK_MARGIN: i32 = 1;

pub fn chunk_lifecycle_system(
    mut commands: Commands,
    grid: Option<Res<Grid>>,
    cam_q: Query<&Transform, With<MainCamera>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
) {
    let Some(grid) = grid else { return };
    let Ok(cam) = cam_q.get_single() else { return };
    let Ok(win) = win_q.get_single() else { return };

    let half = Vec2::new(win.width(), win.height()) * 0.5;
    let cam_pos = cam.translation.truncate();
    let world_min = cam_pos - half;
    let world_max = cam_pos + half;

    // Y inverts between world (up-positive) and grid (down-positive), so
    // world_min / world_max do NOT map to componentwise min/max in chunk
    // space. Map both corners and normalize.
    let c_a = world_to_chunk(world_min);
    let c_b = world_to_chunk(world_max);
    let chunk_min = c_a.min(c_b) - IVec2::splat(CHUNK_MARGIN);
    let chunk_max = c_a.max(c_b) + IVec2::splat(CHUNK_MARGIN);

    let mut want = std::collections::HashSet::new();
    for cy in chunk_min.y..=chunk_max.y {
        for cx in chunk_min.x..=chunk_max.x {
            // skip chunks fully outside the grid
            if cx * CHUNK_TILES >= grid.width() as i32 { continue; }
            if cy * CHUNK_TILES >= grid.height() as i32 { continue; }
            if (cx + 1) * CHUNK_TILES <= 0 { continue; }
            if (cy + 1) * CHUNK_TILES <= 0 { continue; }
            want.insert(IVec2::new(cx, cy));
        }
    }

    let existing: std::collections::HashMap<IVec2, Entity> = chunks_q
        .iter()
        .map(|(e, c)| (c.coord, e))
        .collect();

    for coord in &want {
        if !existing.contains_key(coord) {
            commands.spawn((
                TerrainChunk { coord: *coord },
                ChunkDirty,
                Transform::from_xyz(0.0, 0.0, 0.0),
                Visibility::default(),
            ));
        }
    }
    for (coord, entity) in &existing {
        if !want.contains(coord) {
            commands.entity(*entity).despawn_recursive();
        }
    }
}

fn world_to_chunk(world: Vec2) -> IVec2 {
    let tx = (world.x / TILE_SIZE_PX).floor() as i32;
    // game Y inverts; underground tiles have larger y, in world they have negative y
    let ty = (-world.y / TILE_SIZE_PX).floor() as i32;
    IVec2::new(tx.div_euclid(CHUNK_TILES), ty.div_euclid(CHUNK_TILES))
}
```

- [ ] **Step 2: Implement naive chunk render system**

`src/systems/chunk_render.rs`:
```rust
use bevy::prelude::*;
use crate::components::{ChunkDirty, TerrainChunk};
use crate::grid::{Grid, Layer, OreType};
use crate::systems::setup::TILE_SIZE_PX;
use crate::systems::chunk_lifecycle::CHUNK_TILES;

fn layer_color(l: Layer) -> Color {
    match l {
        Layer::Dirt    => Color::srgb(0.55, 0.42, 0.27),
        Layer::Stone   => Color::srgb(0.42, 0.33, 0.22),
        Layer::Deep    => Color::srgb(0.29, 0.23, 0.15),
        Layer::Bedrock => Color::srgb(0.16, 0.13, 0.10),
    }
}

fn ore_color(o: OreType) -> Option<Color> {
    match o {
        OreType::None   => None,
        OreType::Copper => Some(Color::srgb(0.85, 0.45, 0.20)),
        OreType::Silver => Some(Color::srgb(0.85, 0.85, 0.92)),
        OreType::Gold   => Some(Color::srgb(0.95, 0.78, 0.25)),
    }
}

pub fn chunk_remesh_system(
    mut commands: Commands,
    grid: Option<Res<Grid>>,
    dirty_q: Query<(Entity, &TerrainChunk), With<ChunkDirty>>,
    children_q: Query<&Children>,
) {
    let Some(grid) = grid else { return };
    for (entity, chunk) in dirty_q.iter() {
        // despawn previous children (tile sprites + ore sprites)
        if let Ok(children) = children_q.get(entity) {
            for c in children.iter() {
                commands.entity(*c).despawn_recursive();
            }
        }

        commands.entity(entity).with_children(|parent| {
            for ly in 0..CHUNK_TILES {
                for lx in 0..CHUNK_TILES {
                    let gx = chunk.coord.x * CHUNK_TILES + lx;
                    let gy = chunk.coord.y * CHUNK_TILES + ly;
                    let Some(t) = grid.get(gx, gy) else { continue };
                    if !t.solid { continue }

                    let world_x = gx as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0;
                    let world_y = -(gy as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0);

                    parent.spawn((
                        Sprite {
                            color: layer_color(t.layer),
                            custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(world_x, world_y, 0.0)),
                    ));

                    if let Some(c) = ore_color(t.ore) {
                        parent.spawn((
                            Sprite {
                                color: c,
                                custom_size: Some(Vec2::splat(TILE_SIZE_PX * 0.5)),
                                ..default()
                            },
                            Transform::from_translation(Vec3::new(world_x, world_y, 0.5)),
                        ));
                    }
                }
            }
        });

        commands.entity(entity).remove::<ChunkDirty>();
    }
}
```

- [ ] **Step 3: Register systems in plugin**

Update `src/app.rs`:
```rust
use bevy::prelude::*;
use crate::systems::{camera, chunk_lifecycle, chunk_render, player, setup};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup::setup_world)
            .add_systems(Update, (
                player::read_input_system,
                player::apply_velocity_system,
                chunk_lifecycle::chunk_lifecycle_system,
                chunk_render::chunk_remesh_system,
                camera::camera_follow_system,
            ).chain());
    }
}
```

- [ ] **Step 4: Smoke-test**

```bash
cargo run
```
Expected: blue player sits at the surface; below and around are colored tile bands (dirt brown, stone darker, deep darkest, bedrock near-black) with small dot ores. WASD moves player and the world chunks update around the camera.

- [ ] **Step 5: Commit**

```bash
git add src/systems/chunk_lifecycle.rs src/systems/chunk_render.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add chunk lifecycle + naive per-tile rendering"
```

---

## Task 10: Player AABB collision against the Grid

**Files:**
- Modify: `src/systems/player.rs`, `src/app.rs`

- [ ] **Step 1: Add collision resolution**

Append to `src/systems/player.rs`:
```rust
use crate::grid::Grid;
use crate::systems::setup::TILE_SIZE_PX;

pub const PLAYER_HALF: f32 = 6.0; // 12px sprite

pub fn collide_player_with_grid_system(
    grid: Option<Res<Grid>>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let Some(grid) = grid else { return };
    let Ok(mut t) = q.get_single_mut() else { return };

    // Resolve X then Y. Player AABB is [pos.xy ± PLAYER_HALF].
    // Convert world to tile coords. World y is negative-down; tile y is positive-down.
    for axis in [0u8, 1u8] {
        let p = t.translation;
        let min = Vec2::new(p.x - PLAYER_HALF, p.y - PLAYER_HALF);
        let max = Vec2::new(p.x + PLAYER_HALF, p.y + PLAYER_HALF);

        // tile range overlapping the AABB
        let tx0 = (min.x / TILE_SIZE_PX).floor() as i32;
        let tx1 = (max.x / TILE_SIZE_PX).floor() as i32;
        let ty0 = ((-max.y) / TILE_SIZE_PX).floor() as i32;
        let ty1 = ((-min.y) / TILE_SIZE_PX).floor() as i32;

        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let Some(tile) = grid.get(tx, ty) else { continue };
                if !tile.solid { continue }
                let tw_min = Vec2::new(
                    tx as f32 * TILE_SIZE_PX,
                    -((ty + 1) as f32) * TILE_SIZE_PX,
                );
                let tw_max = Vec2::new(
                    (tx + 1) as f32 * TILE_SIZE_PX,
                    -(ty as f32) * TILE_SIZE_PX,
                );
                let overlap_x = (max.x.min(tw_max.x)) - (min.x.max(tw_min.x));
                let overlap_y = (max.y.min(tw_max.y)) - (min.y.max(tw_min.y));
                if overlap_x <= 0.0 || overlap_y <= 0.0 { continue }
                if axis == 0 {
                    // push out along X
                    if t.translation.x < (tw_min.x + tw_max.x) * 0.5 {
                        t.translation.x -= overlap_x;
                    } else {
                        t.translation.x += overlap_x;
                    }
                } else {
                    if t.translation.y < (tw_min.y + tw_max.y) * 0.5 {
                        t.translation.y -= overlap_y;
                    } else {
                        t.translation.y += overlap_y;
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Register collision after movement**

Update `src/app.rs` Update chain:
```rust
.add_systems(Update, (
    player::read_input_system,
    player::apply_velocity_system,
    player::collide_player_with_grid_system,
    chunk_lifecycle::chunk_lifecycle_system,
    chunk_render::chunk_remesh_system,
    camera::camera_follow_system,
).chain())
```

- [ ] **Step 3: Smoke-test**

```bash
cargo run
```
Expected: player can no longer walk through solid tiles. Player can walk freely along the surface strip and within the spawn pocket. Bedrock walls contain.

- [ ] **Step 4: Commit**

```bash
git add src/systems/player.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add AABB-vs-grid collision resolution for player"
```

---

## Task 11: Dig input + OreDrop entities + HUD

**Files:**
- Modify: `src/systems/player.rs`, `src/systems/ore_drop.rs`, `src/systems/hud.rs`, `src/systems/setup.rs`, `src/app.rs`

This bundles dig input, drop entities, and HUD into one task because they form one indivisible playable loop and share the same smoke-test moment.

- [ ] **Step 1: Add dig cooldown resource**

In `src/systems/player.rs`, add:
```rust
use bevy::prelude::*;
use crate::components::{Player, Velocity, OreDrop, ChunkDirty, TerrainChunk};
use crate::dig::{self, DigStatus};
use crate::grid::{Grid, OreType};
use crate::inventory::Inventory;
use crate::systems::setup::TILE_SIZE_PX;
use crate::systems::chunk_lifecycle::CHUNK_TILES;
use bevy::window::PrimaryWindow;

#[derive(Resource)]
pub struct DigCooldown(pub Timer);

impl Default for DigCooldown {
    fn default() -> Self {
        Self(Timer::from_seconds(0.15, TimerMode::Once))
    }
}

pub const DIG_REACH_TILES: f32 = 2.0;

pub fn dig_input_system(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<crate::components::MainCamera>>,
    player_q: Query<&Transform, With<Player>>,
    mut grid: ResMut<Grid>,
    mut cooldown: ResMut<DigCooldown>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
    time: Res<Time>,
) {
    cooldown.0.tick(time.delta());
    if !mouse.pressed(MouseButton::Left) { return; }
    if !cooldown.0.finished() { return; }

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(player_xf) = player_q.get_single() else { return };

    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };

    let tx = (cursor_world.x / TILE_SIZE_PX).floor() as i32;
    let ty = ((-cursor_world.y) / TILE_SIZE_PX).floor() as i32;
    let tile_center = Vec2::new(
        tx as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0,
        -(ty as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0),
    );
    let dist_tiles =
        (tile_center - player_xf.translation.truncate()).length() / TILE_SIZE_PX;
    if dist_tiles > DIG_REACH_TILES { return; }

    let result = dig::try_dig(&mut grid, tx, ty);
    if result.status != DigStatus::Ok { return; }
    // Cooldown gates only successful swings — failed clicks (out of reach,
    // bedrock) shouldn't punish the player by stalling their next attempt.
    cooldown.0.reset();

    // mark owning chunk dirty
    let chunk_coord = IVec2::new(tx.div_euclid(CHUNK_TILES), ty.div_euclid(CHUNK_TILES));
    for (e, c) in chunks_q.iter() {
        if c.coord == chunk_coord {
            commands.entity(e).insert(ChunkDirty);
            break;
        }
    }

    // spawn ore drop
    if result.ore != OreType::None {
        let color = match result.ore {
            OreType::Copper => Color::srgb(0.85, 0.45, 0.20),
            OreType::Silver => Color::srgb(0.85, 0.85, 0.92),
            OreType::Gold   => Color::srgb(0.95, 0.78, 0.25),
            OreType::None   => Color::WHITE,
        };
        commands.spawn((
            OreDrop { ore: result.ore },
            Sprite {
                color,
                custom_size: Some(Vec2::splat(6.0)),
                ..default()
            },
            Transform::from_translation(tile_center.extend(5.0)),
        ));
    }
}
```

- [ ] **Step 2: Register DigCooldown resource in setup**

Append to `setup_world` in `src/systems/setup.rs`:
```rust
commands.insert_resource(crate::systems::player::DigCooldown::default());
```

- [ ] **Step 3: Implement OreDrop vacuum + delivery**

`src/systems/ore_drop.rs`:
```rust
use bevy::prelude::*;
use crate::components::{OreDrop, Player};
use crate::inventory::Inventory;
use crate::systems::setup::TILE_SIZE_PX;

pub const VACUUM_RADIUS_TILES: f32 = 1.0;
pub const VACUUM_SPEED_PX_PER_S: f32 = 200.0;
pub const PICKUP_DISTANCE_PX: f32 = 6.0;

pub fn ore_drop_system(
    mut commands: Commands,
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut drops_q: Query<(Entity, &OreDrop, &mut Transform), Without<Player>>,
    mut inv: ResMut<Inventory>,
) {
    let Ok(player_xf) = player_q.get_single() else { return };
    let player_pos = player_xf.translation.truncate();

    for (entity, drop, mut t) in drops_q.iter_mut() {
        let to_player = player_pos - t.translation.truncate();
        let dist = to_player.length();
        if dist < PICKUP_DISTANCE_PX {
            inv.add(drop.ore, 1);
            commands.entity(entity).despawn();
            continue;
        }
        if dist / TILE_SIZE_PX < VACUUM_RADIUS_TILES {
            let step = to_player.normalize() * VACUUM_SPEED_PX_PER_S * time.delta_secs();
            t.translation.x += step.x;
            t.translation.y += step.y;
        }
    }
}
```

- [ ] **Step 4: Implement HUD**

`src/systems/hud.rs`:
```rust
use bevy::prelude::*;
use crate::grid::OreType;
use crate::inventory::Inventory;

#[derive(Component)]
pub struct OreCountText(pub OreType);

pub fn setup_hud(mut commands: Commands) {
    let make_row = |ore: OreType, color: Color| (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        children![
            (
                Node {
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    margin: UiRect::right(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(color),
            ),
            (
                Text::new("0"),
                TextFont { font_size: 18.0, ..default() },
                OreCountText(ore),
            ),
        ],
    );

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            make_row(OreType::Copper, Color::srgb(0.85, 0.45, 0.20)),
            make_row(OreType::Silver, Color::srgb(0.85, 0.85, 0.92)),
            make_row(OreType::Gold,   Color::srgb(0.95, 0.78, 0.25)),
        ],
    ));
}

pub fn update_hud_system(
    inv: Res<Inventory>,
    mut q: Query<(&mut Text, &OreCountText)>,
) {
    if !inv.is_changed() { return; }
    for (mut text, marker) in q.iter_mut() {
        **text = inv.get(marker.0).to_string();
    }
}
```

(Bevy 0.15's UI uses required components and `children![]` macro; older versions used `NodeBundle`/`TextBundle`. Adapt to current API.)

- [ ] **Step 5: Wire all new systems**

Update `src/app.rs`:
```rust
use bevy::prelude::*;
use crate::systems::{camera, chunk_lifecycle, chunk_render, hud, ore_drop, player, setup};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup::setup_world, hud::setup_hud))
            .add_systems(Update, (
                player::read_input_system,
                player::apply_velocity_system,
                player::collide_player_with_grid_system,
                player::dig_input_system,
                ore_drop::ore_drop_system,
                chunk_lifecycle::chunk_lifecycle_system,
                chunk_render::chunk_remesh_system,
                camera::camera_follow_system,
                hud::update_hud_system,
            ).chain());
    }
}
```

- [ ] **Step 6: Smoke-test the full loop**

```bash
cargo run
```
Expected:
- Window opens, player on surface, banded layers below.
- WASD moves; collision works; bedrock contains.
- Click on adjacent solid tile within ~2 tiles → tile disappears.
- Bedrock click → no change.
- Ore tile click → small ore-colored drop appears at the tile center.
- Walk near the drop (~1 tile away) → it lerps to player → vanishes.
- HUD count for that ore increments.

- [ ] **Step 7: Commit**

```bash
git add src/systems/
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add dig input, ore drops with vacuum, and HUD inventory display"
```

---

## Task 12: Marching-squares contour rendering

Replace the per-tile rect renderer with a true marching-squares contour mesh per chunk. Done as a separate task so the previous milestone is already playable; if marching squares causes a regression, we revert just this commit and ship with the blocky placeholder.

**Files:**
- Modify: `src/marching_squares.rs`
- Modify: `src/systems/chunk_render.rs`

- [ ] **Step 1: Implement `marching_squares::build_chunk_mesh`**

Standard marching-squares: for each cell (corner samples = 4 tile-corners), look up 1 of 16 cases. Each case adds 0–2 polygons to the interior region. Build a single `Mesh` with vertex colors per layer, or one mesh per layer for clarity.

Module sketch:
```rust
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use crate::grid::{Grid, Layer};

pub fn build_chunk_mesh(
    grid: &Grid,
    chunk_origin_tile: IVec2,
    chunk_tiles: u32,
    tile_size_px: f32,
) -> Mesh {
    // Sample corners across (chunk_tiles+1) x (chunk_tiles+1).
    // For each cell of chunk_tiles x chunk_tiles, look up 16-case polygons.
    // Emit triangle fan/strip per polygon. Color = layer of strongest corner.
    // Detailed implementation derived from the standard marching-squares table.
    todo!("implement marching squares lookup and mesh build")
}
```

The detailed lookup table is well-known; engineer references e.g. https://en.wikipedia.org/wiki/Marching_squares . Keep the function under ~150 lines including the table.

- [ ] **Step 2: Switch chunk_render to use the mesh**

Replace the per-tile-sprite logic in `chunk_remesh_system` with:
1. Call `build_chunk_mesh(...)`.
2. Insert the resulting `Mesh` into `Assets<Mesh>`, get a `Handle<Mesh>`.
3. Spawn a single `Mesh2d` child of the chunk entity with the handle and a `MeshMaterial2d<ColorMaterial>` (or vertex colors).
4. Spawn ore sprites as separate children (these stay sprite-based — small dots at ore-tile centers).

- [ ] **Step 3: Smoke-test**

```bash
cargo run
```
Expected: tile boundaries now show smoothed/curved edges instead of hard grid steps. Layer colors still correct. Ore dots still visible. Collision still based on per-tile AABB (so collision corners remain "boxy" around the smoothed visual — known minor compromise; acceptable for milestone 1).

- [ ] **Step 4: If smooth contour breaks the prototype**

Revert just this commit:
```bash
git revert HEAD --no-edit
```
The per-tile placeholder is good enough to answer "is digging fun?" The smoothness is polish, not a milestone gate. Document the revert in the commit message.

- [ ] **Step 5: Commit (only if Step 3 looks correct)**

```bash
git add src/marching_squares.rs src/systems/chunk_render.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Render terrain chunks with marching-squares contour mesh"
```

---

## Task 13: Final manual playtest & milestone wrap

- [ ] **Step 1: Run full unit suite**

```bash
cargo test
```
Expected: all tests pass.

- [ ] **Step 2: Manual exit-criteria walkthrough**

Run `cargo run` and tick each criterion only if observed:
- [ ] Game window opens, banded layers visible.
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

- [ ] **Step 5: Open a PR for review (or merge directly to main if soloing)**

If using GitHub PRs:
```bash
git push -u origin milestone-1
gh pr create --title "Milestone 1: Core dig prototype" --body "$(cat <<'EOF'
## Summary
- Bevy 2D prototype with destructible terrain, WASD movement, click-to-dig, ore drops, HUD inventory.
- All Task-3..5 pure-module tests pass via cargo test.

## Test plan
- [x] cargo test → all green
- [x] cargo run → manual exit-criteria walkthrough complete
EOF
)"
```

Otherwise merge `milestone-1` into `main` directly.

Milestone 1 complete.
