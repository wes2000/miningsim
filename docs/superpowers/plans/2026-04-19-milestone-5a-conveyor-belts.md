# Milestone 5a — Conveyor Belts MVP — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [../specs/2026-04-19-milestone-5a-conveyor-belts-design.md](../specs/2026-04-19-milestone-5a-conveyor-belts-design.md)

**Goal:** Add single-direction conveyor belts that move items tile-by-tile, integrate with the smelter via a "direction-of-belt" rule, support build-mode placement (B + scroll-wheel + click), persist via save/load (SAVE_VERSION 2→3), and replicate cleanly in 2-player co-op.

**Architecture:** Five phases. (A) Pure module `src/belt.rs` with the data types and helper functions, TDD-verified by ~12 unit tests. (B) Single-player UI: shop unlock + build-mode + ghost cursor + place/remove. (C) Single-player gameplay: belt tick, OreDrop pickup, spillage, smelter I/O. (D) Save/load schema bump. (E) Multiplayer: client-fired events, replicated `BeltTile`, server-side handlers, client-side visual attachment.

**Tech Stack:** Rust (stable ≥ 1.82), Bevy 0.15.x (already pinned), `bevy_replicon = "0.32"`, `bevy_replicon_renet = "0.9"` (transport adapter, both already in Cargo.toml from M4).

---

## Pre-flight: environment expectations

This plan assumes:
- Working directory: `c:/Users/whann/Desktop/Games/miningsim` (existing git repo).
- Branch `main` is at the post-M4 HEAD (which includes M4 fix-ups). The first task creates a `milestone-5a` branch off of main.
- Rust stable toolchain ≥ 1.82.
- `cargo test` currently passes 101/101.
- Author identity: every commit uses `--author="wes2000 <whannasch@gmail.com>"`. Do not modify global git config. Never use `--no-verify` or `--no-gpg-sign` flags.

If any of these aren't true, stop and resolve before proceeding.

---

## File structure (target end state)

```
docs/superpowers/specs/2026-04-19-milestone-5a-conveyor-belts-design.md   # exists
docs/superpowers/plans/2026-04-19-milestone-5a-conveyor-belts.md          # exists (this file)
docs/roadmap.md                # MODIFY: append M5a playtest results section after merge

Cargo.toml                     # no change (replicon + renet already present)

src/
  belt.rs                      # NEW: pure — BeltDir, BeltTile, BeltVisual, helper fns
  components.rs                # MODIFY: + BeltGhost component
  tools.rs                     # MODIFY: + Tool::BeltUnlock variant
  economy.rs                   # MODIFY: + tool_buy_price arm for BeltUnlock
  save.rs                      # MODIFY: SaveData.belts field; SAVE_VERSION = 3
  lib.rs                       # MODIFY: pub mod belt;
  systems/
    mod.rs                     # MODIFY: + pub mod belt; + pub mod belt_ui;
    belt.rs                    # NEW: belt_tick, belt_pickup, belt_spillage, smelter_belt_io
    belt_ui.rs                 # NEW: build-mode input + ghost render
    save_load.rs               # MODIFY: collect/apply belts
    shop_ui.rs                 # MODIFY: render Belt Networks Buy button
    net_events.rs              # MODIFY: + PlaceBeltRequest, RemoveBeltRequest
    net_plugin.rs              # MODIFY: + replicate::<BeltTile>(); + add_client_event for both;
                               #         + handle_place_belt_requests + handle_remove_belt_requests
    net_player.rs              # MODIFY: + add_belt_visuals_on_arrival
  app.rs                       # MODIFY: register all new systems with correct set ordering
tests/
  belt.rs                      # NEW: ~12 pure-module tests
  save.rs                      # MODIFY: + 3 belt-roundtrip tests, SAVE_VERSION = 3
  net_events.rs                # MODIFY: + 2 belt-event serde tests
  tools.rs                     # MODIFY: BeltUnlock variant tests if any
  economy.rs                   # no change expected
```

Test count target: **~122 tests** (101 existing + 16 belt + 3 save + 2 net_events).

---

## Conventions

- Commit style: present-tense imperative.
- `--author="wes2000 <whannasch@gmail.com>"` on every commit.
- Pure modules follow TDD: failing test → verify fail → implement → verify pass → commit.
- Bevy systems are not unit-tested. Smoke checkpoints + manual playtest are the integration tests.
- `cargo run` blocks on the Bevy window — **subagents must not run the binary**. Use `cargo build` + `cargo test`; the human controller drives `cargo run` at smoke-test checkpoints.
- Each commit must leave the crate building and `cargo test` green.
- When following the M4 spec/plan patterns: pure modules don't import `bevy::prelude::*`; they import only what they need (`Component`, `IVec2`, etc.). Bevy systems live in `src/systems/`.

---

## User smoke-test checkpoints

Three checkpoints before final merge:

1. **After Task 3** (build-mode complete) — single-player place/remove belts visually. No item flow yet. Verifies: B-keybind toggle, scroll-wheel rotation, ghost rendering, click-to-place, right-click-remove, corner rendering.

2. **After Task 7** (single-player gameplay loop) — full single-player belt loop works. Place belts, dig adjacent to drop ore on a belt, ore advances, smelter consumes, bar comes out, spillage at dead-end works.

3. **After Task 10** (multiplayer) — two-window co-op: trust-based placement, replicated belt movement, client-fired place/remove requests.

Plus the final exit-criteria walkthrough in Task 11 before merge.

---

## Task 0: Branch + baseline check

**Files:** none

- [ ] **Step 1: Verify clean baseline**

```bash
git status
```
Expected: clean working tree on `main`. If not, surface to user before continuing.

- [ ] **Step 2: Create milestone-5a branch**

```bash
git checkout -b milestone-5a
git status
```
Expected: on `milestone-5a`, clean.

- [ ] **Step 3: Verify baseline tests pass**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: all suites pass; total = 101.

No commit (this is just baseline verification).

---

## Task 1: Pure module `src/belt.rs` (TDD)

**Files:**
- Create: `src/belt.rs`
- Create: `tests/belt.rs`
- Modify: `src/lib.rs` (add `pub mod belt;` in alphabetical order)

The pure-data foundation. After this task: `BeltDir`, `BeltTile`, `BeltVisual`, helper functions all defined and unit-tested. No Bevy systems involved.

- [ ] **Step 1: Register module**

In `src/lib.rs`, add `pub mod belt;` in alphabetical order (between `app` and `components`).

- [ ] **Step 2: Write failing tests in `tests/belt.rs`**

```rust
use bevy::math::IVec2;
use miningsim::belt::{self, BeltDir, BeltTile, BeltVisual};
use miningsim::items::{ItemKind, OreKind};

#[test]
fn belt_dir_delta_cardinals() {
    assert_eq!(BeltDir::North.delta(), IVec2::new(0, -1));
    assert_eq!(BeltDir::South.delta(), IVec2::new(0, 1));
    assert_eq!(BeltDir::East.delta(),  IVec2::new(1, 0));
    assert_eq!(BeltDir::West.delta(),  IVec2::new(-1, 0));
}

#[test]
fn belt_dir_opposite_round_trip() {
    for dir in [BeltDir::North, BeltDir::East, BeltDir::South, BeltDir::West] {
        assert_eq!(dir.opposite().opposite(), dir);
    }
}

#[test]
fn belt_dir_rotate_cw_cycles() {
    let mut dir = BeltDir::North;
    for _ in 0..4 { dir = dir.rotate_cw(); }
    assert_eq!(dir, BeltDir::North);
    // explicit cycle order
    assert_eq!(BeltDir::North.rotate_cw(), BeltDir::East);
    assert_eq!(BeltDir::East.rotate_cw(),  BeltDir::South);
    assert_eq!(BeltDir::South.rotate_cw(), BeltDir::West);
    assert_eq!(BeltDir::West.rotate_cw(),  BeltDir::North);
}

#[test]
fn next_tile_basic() {
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::East),  IVec2::new(6, 5));
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::West),  IVec2::new(4, 5));
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::North), IVec2::new(5, 4));
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::South), IVec2::new(5, 6));
}

#[test]
fn belt_tile_default_empty() {
    let t = BeltTile::new(BeltDir::East);
    assert_eq!(t.item, None);
    assert_eq!(t.dir, BeltDir::East);
}

#[test]
fn belt_tile_holds_item() {
    let mut t = BeltTile::new(BeltDir::East);
    t.item = Some(ItemKind::Ore(OreKind::Copper));
    assert_eq!(t.item, Some(ItemKind::Ore(OreKind::Copper)));
}

#[test]
fn belt_visual_straight_no_feeder() {
    // self facing East, no perpendicular feeder => Straight
    assert_eq!(belt::belt_visual_kind(BeltDir::East, None), BeltVisual::Straight);
}

#[test]
fn belt_visual_aligned_feeder_is_straight() {
    // self facing East, feeder coming from the west (also facing East) is in-line => Straight
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::East)), BeltVisual::Straight);
}

#[test]
fn belt_visual_corner_from_south() {
    // self East, feeder coming from south (feeder dir = North) => corner from S to E
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::North)), BeltVisual::CornerSE);
}

#[test]
fn belt_visual_corner_from_north() {
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::South)), BeltVisual::CornerNE);
}

#[test]
fn belt_visual_all_corners() {
    // Going East, feeder from N or S
    assert_eq!(belt::belt_visual_kind(BeltDir::East,  Some(BeltDir::South)), BeltVisual::CornerNE);
    assert_eq!(belt::belt_visual_kind(BeltDir::East,  Some(BeltDir::North)), BeltVisual::CornerSE);
    // Going West, feeder from N or S
    assert_eq!(belt::belt_visual_kind(BeltDir::West,  Some(BeltDir::South)), BeltVisual::CornerNW);
    assert_eq!(belt::belt_visual_kind(BeltDir::West,  Some(BeltDir::North)), BeltVisual::CornerSW);
    // Going North, feeder from E or W
    assert_eq!(belt::belt_visual_kind(BeltDir::North, Some(BeltDir::East)),  BeltVisual::CornerNW);
    assert_eq!(belt::belt_visual_kind(BeltDir::North, Some(BeltDir::West)),  BeltVisual::CornerNE);
    // Going South, feeder from E or W
    assert_eq!(belt::belt_visual_kind(BeltDir::South, Some(BeltDir::East)),  BeltVisual::CornerSW);
    assert_eq!(belt::belt_visual_kind(BeltDir::South, Some(BeltDir::West)),  BeltVisual::CornerSE);
}

#[test]
fn belt_visual_feeder_facing_away_is_straight() {
    // self East, feeder dir = West (feeder is to the south but its arrow points away from us) => no feed
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::West)), BeltVisual::Straight);
}

// ---------- Back-pressure pure helper tests ----------
// `compute_belt_advances` takes a snapshot of belt positions+dirs and which
// belts currently hold an item, and returns the list of (from, to) moves to
// apply this tick. Tests below exercise the algorithm independently of Bevy.

use std::collections::{BTreeMap, BTreeSet};

fn dirs(pairs: &[(IVec2, BeltDir)]) -> BTreeMap<IVec2, BeltDir> {
    pairs.iter().copied().collect()
}

fn items(positions: &[IVec2]) -> BTreeSet<IVec2> {
    positions.iter().copied().collect()
}

#[test]
fn back_pressure_chain_clears_simultaneously() {
    // Three belts in a row, all facing East, all carrying an item. Destination
    // (3,0) is empty (off the belt graph — spillage handled separately). All
    // three items advance one tile this tick.
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::East),
        (IVec2::new(2, 0), BeltDir::East),
    ]);
    let item_positions = items(&[IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(2, 0)]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let mut moves_set: BTreeSet<(IVec2, IVec2)> = moves.into_iter().collect();
    // Head item leaves the graph; remaining two shift forward one tile.
    assert!(moves_set.contains(&(IVec2::new(2, 0), IVec2::new(3, 0))));
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(2, 0))));
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert_eq!(moves_set.len(), 3);
}

#[test]
fn back_pressure_blocks_when_destination_full() {
    // Two belts: (0,0) East with item, (1,0) East with item. Destination (2,0)
    // is NOT a belt → spillage path. The head item (1,0) goes to (2,0); the
    // tail (0,0) advances into the now-empty (1,0).
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::East),
    ]);
    let item_positions = items(&[IVec2::new(0, 0), IVec2::new(1, 0)]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let moves_set: BTreeSet<(IVec2, IVec2)> = moves.into_iter().collect();
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(2, 0))));
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert_eq!(moves_set.len(), 2);
}

#[test]
fn back_pressure_saturated_cycle_freezes() {
    // Four belts in a CW cycle, all carrying items, no slack → no moves possible.
    // (Players add slack by removing one belt or letting one item exit.)
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::South),
        (IVec2::new(1, 1), BeltDir::West),
        (IVec2::new(0, 1), BeltDir::North),
    ]);
    let item_positions = items(&[
        IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(1, 1), IVec2::new(0, 1),
    ]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    assert!(moves.is_empty(), "saturated cycle must freeze");
}

#[test]
fn back_pressure_cycle_with_slack_rotates() {
    // Same 4-belt CW cycle but only 3 items (one slot empty at (0,1)).
    // The remaining 3 items each advance one tile.
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::South),
        (IVec2::new(1, 1), BeltDir::West),
        (IVec2::new(0, 1), BeltDir::North),
    ]);
    let item_positions = items(&[
        IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(1, 1),
    ]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let moves_set: BTreeSet<(IVec2, IVec2)> = moves.into_iter().collect();
    // Each of the three items advances to its `next_tile`.
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(1, 1))));
    assert!(moves_set.contains(&(IVec2::new(1, 1), IVec2::new(0, 1))));
    assert_eq!(moves_set.len(), 3);
}
```

That's 16 tests total (12 from the original list + 4 back-pressure). Slight overshoot vs. the spec's "~12" target — accept the higher count; the algorithm is too important to leave unverified.

- [ ] **Step 3: Run tests — expect compile failure**

```bash
cargo test --test belt 2>&1 | tail -10
```
Expected: `error[E0432]: unresolved import miningsim::belt`.

- [ ] **Step 4: Implement `src/belt.rs`**

```rust
use bevy::math::IVec2;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use crate::items::ItemKind;

/// Cardinal direction a belt tile points in. Variant order is load-bearing
/// (drives any future BTreeMap iteration) — keep N,E,S,W and don't reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BeltDir {
    North,
    East,
    South,
    West,
}

impl BeltDir {
    /// Tile-space delta. Note: positive Y is "south" (deeper into terrain),
    /// matching the Grid convention used elsewhere in the codebase.
    pub fn delta(self) -> IVec2 {
        match self {
            BeltDir::North => IVec2::new(0, -1),
            BeltDir::East  => IVec2::new(1, 0),
            BeltDir::South => IVec2::new(0, 1),
            BeltDir::West  => IVec2::new(-1, 0),
        }
    }

    pub fn opposite(self) -> BeltDir {
        match self {
            BeltDir::North => BeltDir::South,
            BeltDir::South => BeltDir::North,
            BeltDir::East  => BeltDir::West,
            BeltDir::West  => BeltDir::East,
        }
    }

    /// Cycle clockwise: N → E → S → W → N.
    pub fn rotate_cw(self) -> BeltDir {
        match self {
            BeltDir::North => BeltDir::East,
            BeltDir::East  => BeltDir::South,
            BeltDir::South => BeltDir::West,
            BeltDir::West  => BeltDir::North,
        }
    }
}

/// One belt tile in the world. Component-on-entity. Replicated by replicon.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeltTile {
    pub item: Option<ItemKind>,
    pub dir: BeltDir,
}

impl BeltTile {
    pub fn new(dir: BeltDir) -> Self {
        Self { item: None, dir }
    }
}

/// Visual rendering kind for a belt tile. Locally derived, not replicated.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeltVisual {
    Straight,
    /// Corner from south-feed to east-out (or equivalent rotations).
    CornerSE,
    CornerNE,
    CornerSW,
    CornerNW,
}

/// Compute the visual kind for a belt given its own direction and an optional
/// "feeder" — the direction a perpendicular adjacent belt is pointing IF that
/// adjacent belt feeds into us. Returns Straight if no perpendicular feeder.
///
/// `feeder_dir` is the direction the feeder belt itself faces (its `dir`),
/// not the direction the feeder tile sits relative to us.
///
/// Rules: a feeder is "perpendicular" if its direction is perpendicular to
/// our direction. The feeder must point INTO us, which we encode by its dir
/// being the opposite of where it sits relative to us. Concretely: if we're
/// facing East and a belt south of us faces North (toward us), it's a feed.
/// The corner kind names the "approach direction" (where the item came from).
pub fn next_tile(pos: IVec2, dir: BeltDir) -> IVec2 {
    pos + dir.delta()
}

pub fn belt_visual_kind(self_dir: BeltDir, feeder_dir: Option<BeltDir>) -> BeltVisual {
    let Some(fd) = feeder_dir else { return BeltVisual::Straight };

    // Feeder must be perpendicular to self for a corner.
    let perpendicular = match (self_dir, fd) {
        (BeltDir::East,  BeltDir::North) | (BeltDir::East,  BeltDir::South) => true,
        (BeltDir::West,  BeltDir::North) | (BeltDir::West,  BeltDir::South) => true,
        (BeltDir::North, BeltDir::East)  | (BeltDir::North, BeltDir::West)  => true,
        (BeltDir::South, BeltDir::East)  | (BeltDir::South, BeltDir::West)  => true,
        _ => false,
    };
    if !perpendicular {
        return BeltVisual::Straight;
    }

    // Map (self_dir, feeder_dir) to one of the four corner kinds.
    // Feeder dir = North means feeder is south of us pointing up (item came from south)
    // Feeder dir = South means feeder is north of us pointing down (item came from north)
    // Feeder dir = East  means feeder is west of us pointing right (item came from west)
    // Feeder dir = West  means feeder is east of us pointing left (item came from east)
    match (self_dir, fd) {
        (BeltDir::East,  BeltDir::North) => BeltVisual::CornerSE,  // came from S, going E
        (BeltDir::East,  BeltDir::South) => BeltVisual::CornerNE,  // came from N, going E
        (BeltDir::West,  BeltDir::North) => BeltVisual::CornerSW,
        (BeltDir::West,  BeltDir::South) => BeltVisual::CornerNW,
        (BeltDir::North, BeltDir::East)  => BeltVisual::CornerNW,  // came from W, going N
        (BeltDir::North, BeltDir::West)  => BeltVisual::CornerNE,  // came from E, going N
        (BeltDir::South, BeltDir::East)  => BeltVisual::CornerSW,
        (BeltDir::South, BeltDir::West)  => BeltVisual::CornerSE,
        _ => BeltVisual::Straight,  // unreachable given the perpendicular check
    }
}

/// Pure back-pressure decision: given the current belt graph and which tiles
/// have items, return the list of (from, to) moves to apply this tick. A move
/// is included iff the destination tile, after applying all other moves this
/// tick, will be empty. Saturated cycles produce zero moves.
///
/// Algorithm: iterate to fixed point. Each round, collect moves whose
/// destination is currently empty (after prior rounds' moves). Apply them.
/// Repeat until no new moves. Bounded by N rounds for N belts.
///
/// `belts`: tile coord → belt direction (every belt entity present this tick).
/// `items_present`: tile coords that currently hold an item.
///
/// Note: items advancing to a tile that is NOT in `belts` (i.e. off the belt
/// graph — spillage destinations) ARE included in the returned moves. The
/// caller is responsible for spillage handling on those.
pub fn compute_belt_advances(
    belts: &std::collections::BTreeMap<bevy::math::IVec2, BeltDir>,
    items_present: &std::collections::BTreeSet<bevy::math::IVec2>,
) -> Vec<(bevy::math::IVec2, bevy::math::IVec2)> {
    let mut moves = Vec::new();
    // `current_items` is mutated as we apply moves; tracks who holds an item right now.
    let mut current_items: std::collections::BTreeSet<bevy::math::IVec2> =
        items_present.clone();

    loop {
        let mut round_moves = Vec::new();
        for (&pos, &dir) in belts.iter() {
            if !current_items.contains(&pos) { continue }
            let dest = next_tile(pos, dir);
            // Move-to is allowed if dest is not currently held by an item.
            // (Spillage destinations off the belt graph naturally don't hold items.)
            if !current_items.contains(&dest) {
                round_moves.push((pos, dest));
            }
        }
        if round_moves.is_empty() { break }
        for (from, to) in &round_moves {
            current_items.remove(from);
            current_items.insert(*to);
            moves.push((*from, *to));
        }
    }
    moves
}
```

Note the helper deliberately uses `BTreeMap` and `BTreeSet` (deterministic iteration) rather than `HashMap`/`HashSet`, matching the M4 lesson on replicon determinism.

- [ ] **Step 5: Run tests — expect 16/16 passing**

```bash
cargo test --test belt 2>&1 | tail -10
```
Expected: `16 passed; 0 failed`.

- [ ] **Step 6: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: 101 + 16 = 117 tests passing.

- [ ] **Step 7: Commit**

```bash
git add src/belt.rs tests/belt.rs src/lib.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add belt pure module: BeltDir/BeltTile/BeltVisual + back-pressure helper + 16 unit tests"
```

---

## Task 2: `Tool::BeltUnlock` + shop button + price

**Files:**
- Modify: `src/tools.rs`
- Modify: `src/economy.rs`
- Modify: `src/systems/shop_ui.rs`
- Modify: `src/components.rs`
- Modify: `tests/tools.rs` (add a test if any tier-related coverage breaks; verify)

Add the unlockable tool entry that gates build mode. Buying it unlocks placement; that's it. No belt entities yet.

- [ ] **Step 1: Add Tool variant**

In `src/tools.rs`, append `BeltUnlock` to the `Tool` enum (variant order is load-bearing per the file's existing comment; **append at the end**, do NOT reorder existing variants):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tool {
    Shovel,
    Pickaxe,
    Jackhammer,
    Dynamite,
    BeltUnlock,
}
```

`tool_tier` doesn't apply (BeltUnlock isn't a dig tool). `best_applicable_tool` already iterates a fixed array `[Dynamite, Jackhammer, Pickaxe, Shovel]` — leave it unchanged. BeltUnlock is intentionally absent from that array because it's not a dig tool.

If `clicks_required` (in tools.rs) matches on `Tool` and is non-exhaustive, ensure it returns `None` for `BeltUnlock` (you can either add a `Tool::BeltUnlock => None` arm or rely on the existing wildcard). Verify by reading the current `clicks_required` body before editing.

- [ ] **Step 2: Add price**

In `src/economy.rs::tool_buy_price`, add the BeltUnlock arm:

```rust
pub fn tool_buy_price(tool: Tool) -> u32 {
    match tool {
        Tool::Shovel => 0,
        Tool::Pickaxe => 30,
        Tool::Jackhammer => 100,
        Tool::Dynamite => 300,
        Tool::BeltUnlock => 200,
    }
}
```

- [ ] **Step 3: Add shop Buy button**

In `src/systems/shop_ui.rs::spawn_shop_ui` (or wherever Buy buttons are constructed — find where the existing `ShopButtonKind::Buy(Tool::Pickaxe)` etc. are spawned and add an entry for `BeltUnlock`).

Read the existing button-construction code first, then mirror it. Look for a list/loop that adds a button per tool. Add `BeltUnlock` to that list.

The label can be `"Belt Networks"` (matches the spec's user-facing name).

- [ ] **Step 4: Verify shop refresh logic includes BeltUnlock**

`update_shop_labels_system` (in shop_ui.rs) probably iterates `ShopButtonKind::Buy(_)` entities and updates their labels based on owned/affordability state. Verify that no special-casing excludes `BeltUnlock` — it should be handled by the same generic logic.

- [ ] **Step 5: Build + test**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: 117 tests still passing. Build clean. Existing economy/tools tests should pass unchanged (the new variant is additive).

If `tests/economy.rs` exhaustively asserts on the Tool enum (e.g., a `match` over all variants), it will fail to compile — fix by adding a `BeltUnlock` arm with the expected price (200).

- [ ] **Step 6: Commit**

```bash
git add src/tools.rs src/economy.rs src/systems/shop_ui.rs src/components.rs tests/
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add Tool::BeltUnlock (200c); shop renders Belt Networks Buy button"
```

(Stage `tests/` blanket if tests needed updating; otherwise just the source files.)

---

## Task 3: Build-mode UI complete (B keybind + scroll wheel + click-place + click-remove)

**Files:**
- Create: `src/systems/belt_ui.rs`
- Modify: `src/systems/mod.rs` (add `pub mod belt_ui;`)
- Modify: `src/components.rs` (add `BeltGhost` Component marker)
- Modify: `src/app.rs` (register the new systems)

After this task: player can buy Belt Networks, press B to enter build mode, see a translucent ghost belt at the cursor, scroll-wheel to rotate, left-click to place a real belt, right-click to remove. **No item flow yet** — belts just sit there.

This is a bigger task than the M4 average because the build-mode UX is one cohesive flow that doesn't decompose well into smaller commits.

- [ ] **Step 1: Add `BeltGhost` Component to components.rs**

```rust
/// Visual marker for the translucent belt sprite shown at the cursor while in
/// build mode. Always exactly zero or one of these in the world.
#[derive(Component)]
pub struct BeltGhost;
```

- [ ] **Step 2: Register module**

In `src/systems/mod.rs`, add `pub mod belt_ui;` (alphabetical — between `belt` and `camera`).

- [ ] **Step 3: Create `src/systems/belt_ui.rs`**

```rust
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::belt::{self, BeltDir, BeltTile, BeltVisual};
use crate::components::{BeltGhost, LocalPlayer, MainCamera, Shop, Smelter, TerrainChunk};
use crate::coords::{tile_center_world, world_to_tile, TILE_SIZE_PX};
use crate::grid::Grid;
use crate::tools::{OwnedTools, Tool};

/// Per-peer local state. None = not in build mode. Some(dir) = in build mode
/// with the cursor showing a belt facing `dir`.
#[derive(Resource, Default)]
pub struct BeltBuildMode {
    pub cursor_dir: Option<BeltDir>,
}

/// Toggle build mode on B keypress. Exits on Esc as well. Gated on the local
/// player owning Tool::BeltUnlock — otherwise B does nothing.
pub fn belt_build_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut build_mode: ResMut<BeltBuildMode>,
    owned: Option<Single<&OwnedTools, With<LocalPlayer>>>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        let Some(owned) = owned else { return };
        if !owned.0.contains(&Tool::BeltUnlock) {
            return;
        }
        build_mode.cursor_dir = match build_mode.cursor_dir {
            None => Some(BeltDir::East),
            Some(_) => None,
        };
    }
    if keys.just_pressed(KeyCode::Escape) {
        build_mode.cursor_dir = None;
    }
}

/// Scroll wheel rotates the cursor direction clockwise. Only fires when in
/// build mode.
pub fn belt_build_rotate_system(
    mut wheel: EventReader<MouseWheel>,
    mut build_mode: ResMut<BeltBuildMode>,
) {
    let Some(dir) = build_mode.cursor_dir else {
        // Drain events even when not in build mode so they don't pile up.
        wheel.clear();
        return;
    };
    let mut new_dir = dir;
    for ev in wheel.read() {
        if ev.y > 0.0 {
            new_dir = new_dir.rotate_cw();
        } else if ev.y < 0.0 {
            // Counter-clockwise = three CW rotations.
            new_dir = new_dir.rotate_cw().rotate_cw().rotate_cw();
        }
    }
    build_mode.cursor_dir = Some(new_dir);
}

/// Spawn / move / despawn the ghost entity that follows the cursor.
pub fn belt_ghost_render_system(
    mut commands: Commands,
    build_mode: Res<BeltBuildMode>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    ghost_q: Query<Entity, With<BeltGhost>>,
) {
    let Some(dir) = build_mode.cursor_dir else {
        // Not in build mode — despawn any lingering ghost.
        for e in ghost_q.iter() {
            commands.entity(e).despawn();
        }
        return;
    };

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else {
        // Cursor off-window — keep ghost wherever it was.
        return;
    };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
    let tile = world_to_tile(cursor_world);
    let center = tile_center_world(tile);

    // Ghost color encodes direction (we'll polish to actual sprites later).
    let color = ghost_color(dir);

    if let Ok(existing) = ghost_q.get_single() {
        commands.entity(existing).insert((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                ..default()
            },
            Transform::from_translation(center.extend(8.0)),
        ));
    } else {
        commands.spawn((
            BeltGhost,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                ..default()
            },
            Transform::from_translation(center.extend(8.0)),
        ));
    }
}

fn ghost_color(dir: BeltDir) -> Color {
    // Translucent overlay color, hue-shifted by direction so player sees rotation.
    let alpha = 0.40;
    match dir {
        BeltDir::North => Color::srgba(0.30, 0.80, 0.30, alpha),
        BeltDir::East  => Color::srgba(0.80, 0.80, 0.30, alpha),
        BeltDir::South => Color::srgba(0.80, 0.30, 0.30, alpha),
        BeltDir::West  => Color::srgba(0.30, 0.30, 0.80, alpha),
    }
}

fn belt_color(dir: BeltDir) -> Color {
    // Solid color for placed belts (alpha 1.0). Same hue as ghost.
    match dir {
        BeltDir::North => Color::srgb(0.20, 0.55, 0.20),
        BeltDir::East  => Color::srgb(0.60, 0.55, 0.20),
        BeltDir::South => Color::srgb(0.55, 0.20, 0.20),
        BeltDir::West  => Color::srgb(0.20, 0.20, 0.55),
    }
}

/// Left-click while in build mode places a belt at the cursor tile (single-
/// player + host path). In Client mode this fires `PlaceBeltRequest` instead;
/// that branch lands in Task 11.
pub fn belt_place_system(
    mut commands: Commands,
    build_mode: Res<BeltBuildMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    grid_q: Option<Single<&Grid>>,
    belts_q: Query<&Transform, With<BeltTile>>,
    shops_q: Query<&Transform, With<Shop>>,
    smelters_q: Query<&Transform, With<Smelter>>,
) {
    let Some(dir) = build_mode.cursor_dir else { return };
    if !mouse.just_pressed(MouseButton::Left) { return };

    let Some(grid) = grid_q else { return };
    let grid = grid.into_inner();

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
    let tile = world_to_tile(cursor_world);

    // Validate: tile in bounds, floor (not solid), no existing belt, no machine.
    if !validate_belt_placement(tile, grid, &belts_q, &shops_q, &smelters_q) {
        return;
    }

    let center = tile_center_world(tile);
    commands.spawn((
        BeltTile::new(dir),
        BeltVisual::Straight,  // recomputed by belt_visual_recompute_system below
        Sprite {
            color: belt_color(dir),
            custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
            ..default()
        },
        Transform::from_translation(center.extend(3.0)),
        bevy_replicon::prelude::Replicated,  // inert in single-player; needed in Host mode
    ));
}

/// Right-click while in build mode removes a belt at the cursor tile (single-
/// player + host path). Items currently on the removed belt spill as OreDrop
/// at that tile (server-side handler does the same in Task 11).
pub fn belt_remove_system(
    mut commands: Commands,
    build_mode: Res<BeltBuildMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    belts_q: Query<(Entity, &Transform, &BeltTile)>,
) {
    if build_mode.cursor_dir.is_none() { return };
    if !mouse.just_pressed(MouseButton::Right) { return };

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
    let target = world_to_tile(cursor_world);

    for (e, xf, belt_tile) in belts_q.iter() {
        let pos = world_to_tile(xf.translation.truncate());
        if pos == target {
            // Spill the item if any — spawn OreDrop at this tile.
            if let Some(item) = belt_tile.item {
                let center = tile_center_world(pos);
                commands.spawn((
                    crate::components::OreDrop { item },
                    Sprite {
                        color: crate::systems::hud::item_color(item),
                        custom_size: Some(Vec2::splat(6.0)),
                        ..default()
                    },
                    Transform::from_translation(center.extend(4.0)),
                ));
            }
            commands.entity(e).despawn();
            return;
        }
    }
}

fn validate_belt_placement(
    tile: bevy::math::IVec2,
    grid: &Grid,
    belts_q: &Query<&Transform, With<BeltTile>>,
    shops_q: &Query<&Transform, With<Shop>>,
    smelters_q: &Query<&Transform, With<Smelter>>,
) -> bool {
    // In bounds
    let Some(g) = grid.get(tile.x, tile.y) else { return false };
    // Not solid (must be floor)
    if g.solid { return false };
    // No existing belt
    for xf in belts_q.iter() {
        if world_to_tile(xf.translation.truncate()) == tile { return false };
    }
    // No machine
    for xf in shops_q.iter().chain(smelters_q.iter()) {
        if world_to_tile(xf.translation.truncate()) == tile { return false };
    }
    true
}

/// Recompute `BeltVisual` when belts are added, removed, or have their
/// direction changed. Gated on `Changed<BeltTile>` so we don't iterate every
/// frame in steady state. When ANY belt changes, we conservatively recompute
/// for every belt (the changed belt's neighbors might also need updates) —
/// fine at MVP scale; revisit with neighbor-only invalidation if perf matters.
pub fn belt_visual_recompute_system(
    changed_q: Query<(), Changed<BeltTile>>,
    removed: RemovedComponents<BeltTile>,
    mut belts_q: Query<(&Transform, &BeltTile, &mut BeltVisual)>,
    all_belts_q: Query<(&Transform, &BeltTile)>,
) {
    // Skip if nothing changed and no belts were removed.
    if changed_q.is_empty() && removed.is_empty() { return }

    use std::collections::BTreeMap;
    let map: BTreeMap<bevy::math::IVec2, BeltDir> = all_belts_q
        .iter()
        .map(|(xf, bt)| (world_to_tile(xf.translation.truncate()), bt.dir))
        .collect();

    for (xf, bt, mut visual) in belts_q.iter_mut() {
        let pos = world_to_tile(xf.translation.truncate());
        let feeder = perpendicular_feeder(pos, bt.dir, &map);
        let new_visual = belt::belt_visual_kind(bt.dir, feeder);
        if *visual != new_visual {
            *visual = new_visual;
        }
    }
}

/// For a belt at `pos` facing `self_dir`, look at the two perpendicular
/// neighbor tiles and return the first one whose belt points INTO us.
fn perpendicular_feeder(
    pos: bevy::math::IVec2,
    self_dir: BeltDir,
    map: &std::collections::BTreeMap<bevy::math::IVec2, BeltDir>,
) -> Option<BeltDir> {
    let perps = match self_dir {
        BeltDir::East | BeltDir::West => [BeltDir::North, BeltDir::South],
        BeltDir::North | BeltDir::South => [BeltDir::East, BeltDir::West],
    };
    for perp in perps {
        let neighbor_pos = pos + perp.opposite().delta();
        // Equivalently: neighbor sits in direction `perp.opposite()` from us.
        let Some(neighbor_dir) = map.get(&neighbor_pos) else { continue };
        // For the neighbor to feed INTO us, its dir must point from neighbor_pos to pos.
        // I.e., neighbor_dir.delta() == (pos - neighbor_pos) == -perp.opposite().delta() == perp.delta()
        if neighbor_dir.delta() == perp.delta() {
            return Some(*neighbor_dir);
        }
    }
    None
}
```

(That's a fairly large file; ~250 lines. It's the entire build-mode UX module.)

- [ ] **Step 4: Register systems in `src/app.rs`**

In `MiningSimPlugin::build`, insert the BeltBuildMode resource and register the five new systems. The placement and removal systems read mouse input — schedule them in `UiSet::Hud` (alongside other UI input systems) or wherever fits. The recompute system can run on Update unconditionally.

```rust
app.insert_resource(crate::systems::belt_ui::BeltBuildMode::default());

app.add_systems(Update, (
    crate::systems::belt_ui::belt_build_toggle_system,
    crate::systems::belt_ui::belt_build_rotate_system,
    crate::systems::belt_ui::belt_ghost_render_system,
    crate::systems::belt_ui::belt_place_system,
    crate::systems::belt_ui::belt_remove_system,
    crate::systems::belt_ui::belt_visual_recompute_system,
).in_set(crate::app::UiSet::Hud));
```

(Or split into a new `UiSet::Build` if `UiSet::Hud` becomes too crowded — verify the existing Hud set has room.)

- [ ] **Step 5: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 117 tests still passing. Build clean.

- [ ] **Step 6: Commit**

```bash
git add src/systems/belt_ui.rs src/systems/mod.rs src/components.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add belt build mode: B-toggle, scroll-rotate, click-place, right-click-remove"
```

---

## Smoke-test checkpoint #1 (after Task 3)

Human controller runs `cargo run`. Expected:

- Walk to shop, sell some ore, buy "Belt Networks" for 200c (button visible alongside Pickaxe etc.).
- Press B → translucent belt sprite (yellow, east-facing) appears at the mouse cursor.
- Scroll wheel up → cursor color cycles green (N), yellow (E), red (S), blue (W). Scroll down reverses.
- Move mouse → ghost follows.
- Left-click on a floor tile → solid belt sprite spawns at that tile, replacing the ghost color underneath.
- Right-click on a placed belt → it disappears.
- Press B again → ghost disappears, build mode exits.
- Press Esc while in build mode → also exits.
- Place several belts in an L-shape → corners visually distinct (you'll see them as different color tints once Task 3's `belt_visual_recompute_system` is wired; if all belts look the same straight color, that's a follow-up — sprites for the four corner kinds get polished post-MVP, but the data should already be flipping `BeltVisual` between Straight and corner kinds).

If any of these visibly fail, surface to controller for diagnosis before Task 4.

---

## Task 4: `belt_tick_system` — items advance one tile per second

**Files:**
- Modify: `src/systems/belt.rs` (NEW file — or add to existing? See below.)
- Modify: `src/systems/mod.rs` (add `pub mod belt;` if not already)
- Modify: `src/app.rs` (register `belt_tick_system` in a new `MachineSet::BeltTick` ordered before `MachineSet::SmelterTick`)

Note: there are two distinct `belt` files — `src/belt.rs` (pure module, Task 1) and `src/systems/belt.rs` (Bevy systems, this task). Both exist; they don't conflict because they're in different modules.

After this task: items placed manually on a belt advance one tile per second. No way to put items on a belt yet (Task 5 adds pickup) — so for now the implementer can hand-construct a test world or just verify the system compiles. Visual playtest happens at Smoke #2 after Task 7.

- [ ] **Step 1: Create `src/systems/belt.rs`**

The algorithm lives entirely in the pure `belt::compute_belt_advances` helper from Task 1. This system is just glue: snapshot world state into the helper's input shape, call the helper, apply the returned moves back to the World.

```rust
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::belt::{self, BeltTile};
use crate::coords::world_to_tile;

/// Belts tick every `BELT_TICK_SECONDS` (1.0). Items try to advance one tile
/// in their belt's direction. Back-pressure semantics live in the pure
/// `compute_belt_advances` helper.
pub const BELT_TICK_SECONDS: f32 = 1.0;

#[derive(Resource)]
pub struct BeltTickTimer(pub Timer);

impl Default for BeltTickTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(BELT_TICK_SECONDS, TimerMode::Repeating))
    }
}

pub fn belt_tick_system(
    time: Res<Time>,
    mut timer: ResMut<BeltTickTimer>,
    mut belts_q: Query<(Entity, &Transform, &mut BeltTile)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() { return }

    // Snapshot: build the (position → direction) map and (position → entity)
    // index, plus the set of tiles currently holding an item.
    let mut belt_dirs: BTreeMap<bevy::math::IVec2, belt::BeltDir> = BTreeMap::new();
    let mut entity_at: BTreeMap<bevy::math::IVec2, Entity> = BTreeMap::new();
    let mut item_at: BTreeMap<bevy::math::IVec2, crate::items::ItemKind> = BTreeMap::new();
    for (e, xf, bt) in belts_q.iter() {
        let pos = world_to_tile(xf.translation.truncate());
        belt_dirs.insert(pos, bt.dir);
        entity_at.insert(pos, e);
        if let Some(item) = bt.item {
            item_at.insert(pos, item);
        }
    }
    let items_present: BTreeSet<bevy::math::IVec2> = item_at.keys().copied().collect();

    // Pure helper computes the moves.
    let moves = belt::compute_belt_advances(&belt_dirs, &items_present);

    // Apply moves. We need to write to multiple BeltTile mutables; safest is
    // to first compute a (entity, new_item) plan, then apply with a single
    // iter_mut pass.
    use std::collections::HashMap;
    let mut new_item_for_entity: HashMap<Entity, Option<crate::items::ItemKind>> = HashMap::new();

    for (from, to) in &moves {
        let item = item_at.get(from).copied();
        if let Some(item) = item {
            if let Some(&from_e) = entity_at.get(from) {
                new_item_for_entity.insert(from_e, None);
            }
            // Destination may be off the belt graph (spillage destination); only set if it's a belt.
            if let Some(&to_e) = entity_at.get(to) {
                new_item_for_entity.insert(to_e, Some(item));
            }
            // Spillage of items moving off-graph is handled by `belt_spillage_system`
            // BEFORE this tick (see app.rs set ordering: BeltSpillage runs after
            // BeltTick on the *previous* frame; or run order can be adjusted).
            // For MVP simplicity: if the destination isn't a belt, simply leave the
            // item on the source — `belt_spillage_system` will spill it on its next
            // pass. That means a single dead-end belt's item takes 2 ticks to spill,
            // not 1. Acceptable.
        }
    }

    // To preserve "head of chain advances even when destination isn't a belt"
    // semantics, we need to also clear the source's item when the destination is
    // off-graph. Do that:
    for (from, to) in &moves {
        if !entity_at.contains_key(to) {
            // Dest is off-graph; clear the source so the item is "in transit" and
            // will be picked up by belt_spillage_system this tick.
            if let Some(&from_e) = entity_at.get(from) {
                new_item_for_entity.insert(from_e, None);
            }
            // We rely on belt_spillage_system having already run earlier this tick
            // OR running next tick to actually spawn the OreDrop. See Task 6.
        }
    }

    for (e, _, mut bt) in belts_q.iter_mut() {
        if let Some(&new_item) = new_item_for_entity.get(&e) {
            bt.item = new_item;
        }
    }
}
```

**Notes for the implementer:**
- All algorithmic correctness is in the pure helper — this system is just glue.
- The HashMap → BTreeMap distinction matters: snapshot uses `BTreeMap` to feed the deterministic pure helper; the writeback HashMap is local-only and doesn't affect determinism.
- Spillage interaction: `belt_spillage_system` (Task 6) runs AFTER `belt_tick_system` in the same tick (see app.rs set ordering). The above writeback correctly clears the source's item when the destination is off-graph; spillage detects "item moved off the belt" by computing `next_tile(pos, dir)` and seeing nothing there — but it needs to know the item WAS on this belt at start of tick. **Cleaner approach:** spillage uses its own snapshot at the start of the tick (run BEFORE belt_tick) and detects "this belt has an item AND its `next_tile` isn't a belt or smelter" → spill the item. Then belt_tick proceeds with the item already cleared. Tasks 4, 6 must coordinate; the cleanest set order is:
  1. **BeltPickup** (OreDrop → empty belt)
  2. **BeltSpillage** (items at dead-end belts → OreDrop, clear belt slot)
  3. **BeltTick** (advance remaining items)
  4. **SmelterBeltIo** (pull/push via direction-of-belt rule)
  5. **SmelterTick** (existing — process queue, produce bars)
  6. **SmelterUi** (existing — UI refresh)

  This is one tick later than the spec's nominal order but is logically equivalent and avoids the "where did this item come from" coordination problem. Update Task 6 and the smoke-checkpoint pointers accordingly.

- [ ] **Step 2: Register module + system**

In `src/systems/mod.rs`, add `pub mod belt;` (alphabetical position — between `belt_ui` and... hmm, `belt` is alphabetically before `belt_ui`. So insert in correct alpha order).

In `src/app.rs::MiningSimPlugin::build`:
- Insert `BeltTickTimer` resource: `app.insert_resource(crate::systems::belt::BeltTickTimer::default());`
- Add a new `MachineSet::BeltTick` variant in the SystemSet enum. Place it in `configure_sets` BEFORE `MachineSet::SmelterTick`.
- Register `belt_tick_system` in that set: `app.add_systems(Update, crate::systems::belt::belt_tick_system.in_set(MachineSet::BeltTick));`

(Per the spec's run-order: Pickup → Tick → SmelterIO → SmelterTick → Spillage. Each gets its own MachineSet variant; this task only adds BeltTick.)

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 117 tests still passing. Build clean.

- [ ] **Step 4: Commit**

```bash
git add src/systems/belt.rs src/systems/mod.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add belt_tick_system: items advance one tile per second with back-pressure"
```

---

## Task 5: `belt_pickup_system` — OreDrop on belt → belt slot

**Files:**
- Modify: `src/systems/belt.rs` (add the system)
- Modify: `src/app.rs` (add `MachineSet::BeltPickup` ordered BEFORE `MachineSet::BeltTick`)

After this task: an `OreDrop` floor sprite that lands on the same tile as an empty belt is consumed by the belt — `BeltTile.item` set, `OreDrop` despawned. Combined with Task 4, this means the player can dig adjacent to a belt and watch the ore advance.

- [ ] **Step 1: Append `belt_pickup_system` to `src/systems/belt.rs`**

```rust
use crate::components::OreDrop;

/// For each OreDrop, check if its tile coord matches an empty belt. If so,
/// transfer the item into the belt and despawn the OreDrop. If multiple drops
/// land on the same tile in the same frame, only the first one (in iter order)
/// is picked up; subsequent drops remain on the floor for the next tick.
pub fn belt_pickup_system(
    mut commands: Commands,
    drops_q: Query<(Entity, &Transform, &OreDrop)>,
    mut belts_q: Query<(&Transform, &mut BeltTile)>,
) {
    use std::collections::BTreeSet;
    // Snapshot which tiles already had a belt with no item — these are the
    // pickup candidates. We track which ones have been consumed this frame
    // so the second drop on the same tile doesn't double-write.
    let mut available_tiles: BTreeSet<bevy::math::IVec2> = belts_q
        .iter()
        .filter(|(_, bt)| bt.item.is_none())
        .map(|(xf, _)| world_to_tile(xf.translation.truncate()))
        .collect();

    for (drop_entity, drop_xf, drop_data) in drops_q.iter() {
        let drop_tile = world_to_tile(drop_xf.translation.truncate());
        if !available_tiles.contains(&drop_tile) { continue }

        // Find the belt entity at this tile and write the item.
        for (belt_xf, mut belt_tile) in belts_q.iter_mut() {
            let pos = world_to_tile(belt_xf.translation.truncate());
            if pos != drop_tile { continue }
            if belt_tile.item.is_some() { break } // shouldn't happen given the snapshot, defensive
            belt_tile.item = Some(drop_data.item);
            commands.entity(drop_entity).despawn();
            available_tiles.remove(&pos);   // mark consumed for subsequent drops this frame
            break;
        }
    }
}
```

- [ ] **Step 2: Register system in `src/app.rs`**

Add `MachineSet::BeltPickup` variant. In `configure_sets`, place it BEFORE `MachineSet::BeltTick`.

```rust
app.add_systems(Update, crate::systems::belt::belt_pickup_system
    .in_set(MachineSet::BeltPickup));
```

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 117 tests still passing. Build clean.

- [ ] **Step 4: Commit**

```bash
git add src/systems/belt.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add belt_pickup_system: OreDrop on belt tile transfers into belt slot"
```

---

## Task 6: `belt_spillage_system` — items at dead-end belts spill as OreDrop

**Files:**
- Modify: `src/systems/belt.rs` (add the system)
- Modify: `src/app.rs` (add `MachineSet::BeltSpillage` ordered AFTER `MachineSet::SmelterUi`)

After this task: when a belt's `next_tile` is not another belt and not a smelter, items reaching that belt spill off the end as a floor `OreDrop` sprite at the destination tile. Player can vacuum and sell.

- [ ] **Step 1: Append `belt_spillage_system` to `src/systems/belt.rs`**

```rust
use crate::components::Smelter;
use crate::coords::tile_center_world;
use crate::systems::hud::item_color;

/// For each belt with an item whose destination is NOT another belt and NOT
/// a smelter, spawn an OreDrop at the destination and clear the belt slot.
pub fn belt_spillage_system(
    mut commands: Commands,
    mut belts_q: Query<(&Transform, &mut BeltTile)>,
    smelter_xf_q: Query<&Transform, With<Smelter>>,
) {
    use std::collections::HashSet;
    let belt_positions: HashSet<bevy::math::IVec2> = belts_q
        .iter()
        .map(|(xf, _)| world_to_tile(xf.translation.truncate()))
        .collect();
    let smelter_positions: HashSet<bevy::math::IVec2> = smelter_xf_q
        .iter()
        .map(|xf| world_to_tile(xf.translation.truncate()))
        .collect();

    for (xf, mut bt) in belts_q.iter_mut() {
        let Some(item) = bt.item else { continue };
        let pos = world_to_tile(xf.translation.truncate());
        let dest = belt::next_tile(pos, bt.dir);
        // If dest is a belt or smelter, leave it alone — those have their own consumers.
        if belt_positions.contains(&dest) || smelter_positions.contains(&dest) { continue };
        // Spill: spawn OreDrop at dest, clear belt slot.
        let dest_world = tile_center_world(dest);
        commands.spawn((
            crate::components::OreDrop { item },
            Sprite {
                color: item_color(item),
                custom_size: Some(Vec2::splat(6.0)),
                ..default()
            },
            Transform::from_translation(dest_world.extend(4.0)),
        ));
        bt.item = None;
    }
}
```

- [ ] **Step 2: Register system + set in `src/app.rs`**

Add `MachineSet::BeltSpillage` variant. **Important per Task 4's note:** place it BEFORE `MachineSet::BeltTick` (spillage runs first, so items destined to leave the belt graph this tick get spilled before the tick attempts to advance them).

Final set order in `configure_sets`:
```
... existing input + collide ...
MachineSet::ShopProximity
MachineSet::SmelterProximity
MachineSet::BeltPickup        // Task 5
MachineSet::BeltSpillage      // Task 6 (this task)
MachineSet::BeltTick          // Task 4
MachineSet::SmelterBeltIo     // Task 7
MachineSet::SmelterTick       // existing
MachineSet::ShopUi            // existing
MachineSet::SmelterUi         // existing
... existing drops + chunks + hud + camera ...
```

```rust
app.add_systems(Update, crate::systems::belt::belt_spillage_system
    .in_set(MachineSet::BeltSpillage));
```

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 117 tests still passing.

- [ ] **Step 4: Commit**

```bash
git add src/systems/belt.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add belt_spillage_system: items at dead-end belts pop off as OreDrop"
```

---

## Task 7: `smelter_belt_io_system` — direction-of-belt rule

**Files:**
- Modify: `src/systems/belt.rs` (add the system)
- Modify: `src/app.rs` (add `MachineSet::SmelterBeltIo` between `MachineSet::BeltTick` and `MachineSet::SmelterTick`)

After this task: belts adjacent to a smelter integrate. Belt pointing INTO smelter feeds it (one item/tick into queue). Belt pointing AWAY pushes a bar onto it (one bar/tick from output buffer). Cardinal priority N → E → S → W.

- [ ] **Step 1: Append `smelter_belt_io_system` to `src/systems/belt.rs`**

Constants confirmed against the actual `src/processing.rs`:
- `processing::SMELT_DURATION_S` (NOT `SMELT_TIME_SECONDS`)
- `SmelterState.queue: u32` is uncapped in current code; for belt I/O we add a soft cap to prevent pathological pile-ups while the smelter is processing
- `SmelterState` accepts only one `recipe: Option<OreKind>` at a time; when busy with one ore, others must wait

```rust
use crate::belt::BeltDir;
use crate::items::{ItemKind, OreKind};
use crate::processing::{self, SmelterState};

/// Soft cap on per-smelter queue. Prevents the belt from feeding indefinitely
/// while the smelter is processing the same ore. 8 = roughly 16 seconds of
/// pre-buffered work at SMELT_DURATION_S = 2.0.
const SMELTER_QUEUE_CAP: u32 = 8;

/// For each smelter, scan 4 cardinal sides for adjacent belts.
/// - Belts pointing INTO the smelter feed it: pulls one ore per tick total.
///   - Idle smelter: starts smelting that ore (sets recipe + queue=1 + time_left).
///   - Busy with same recipe: queue += 1 (up to SMELTER_QUEUE_CAP).
///   - Busy with different recipe: do NOT pull (item stays on belt → back-pressure).
/// - Belts pointing AWAY get a bar pushed onto them: pushes one bar per tick.
/// Priority order: N, E, S, W on both pull and push (deterministic).
pub fn smelter_belt_io_system(
    mut belts_q: Query<(&Transform, &mut BeltTile)>,
    mut smelters_q: Query<(&Transform, &mut SmelterState)>,
) {
    use std::collections::BTreeMap;
    // Snapshot belt positions/dirs (we'll do a second iter_mut to write).
    let belt_positions: BTreeMap<bevy::math::IVec2, BeltDir> = belts_q
        .iter()
        .map(|(xf, bt)| (world_to_tile(xf.translation.truncate()), bt.dir))
        .collect();

    for (smelter_xf, mut state) in smelters_q.iter_mut() {
        let smelter_pos = world_to_tile(smelter_xf.translation.truncate());

        // ---- Pull (input) ----
        // At most one ore per smelter per tick; first matching side in N,E,S,W order.
        for &side in &[BeltDir::North, BeltDir::East, BeltDir::South, BeltDir::West] {
            let neighbor_pos = smelter_pos + side.delta();
            let Some(neighbor_dir) = belt_positions.get(&neighbor_pos) else { continue };
            // Belt points INTO smelter iff its dir == side.opposite()
            // (e.g., neighbor sits to my North; for it to feed me, its dir must be South).
            if *neighbor_dir != side.opposite() { continue }

            // Find the belt's actual entity to read its item and clear it.
            // We iterate the mutable query and match by tile position.
            let mut consumed = false;
            for (belt_xf, mut belt_tile) in belts_q.iter_mut() {
                if world_to_tile(belt_xf.translation.truncate()) != neighbor_pos { continue }
                let Some(item) = belt_tile.item else { break }; // empty belt
                let ItemKind::Ore(ore) = item else { break };   // bars don't feed the smelter

                // Decide based on smelter state.
                match state.recipe {
                    None => {
                        // Idle smelter: start smelting this ore.
                        state.recipe = Some(ore);
                        state.queue = 1;
                        state.time_left = processing::SMELT_DURATION_S;
                        belt_tile.item = None;
                        consumed = true;
                    }
                    Some(current) if current == ore && state.queue < SMELTER_QUEUE_CAP => {
                        // Same recipe, has capacity: extend the queue.
                        state.queue += 1;
                        belt_tile.item = None;
                        consumed = true;
                    }
                    _ => {
                        // Wrong recipe (busy with different ore) or queue cap hit:
                        // leave the item on the belt (back-pressure).
                    }
                }
                break;
            }
            if consumed { break }   // at most one input per smelter per tick
        }

        // ---- Push (output) ----
        // At most one bar per smelter per tick; first eligible side in N,E,S,W order.
        let has_output = state.output.values().any(|&n| n > 0);
        if !has_output { continue }

        for &side in &[BeltDir::North, BeltDir::East, BeltDir::South, BeltDir::West] {
            let neighbor_pos = smelter_pos + side.delta();
            let Some(neighbor_dir) = belt_positions.get(&neighbor_pos) else { continue };
            // Belt points AWAY from smelter iff its dir == side
            // (e.g., neighbor sits to my East and faces East — moving items further east).
            if *neighbor_dir != side { continue }

            // Find the belt and try to push a bar.
            let mut pushed = false;
            for (belt_xf, mut belt_tile) in belts_q.iter_mut() {
                if world_to_tile(belt_xf.translation.truncate()) != neighbor_pos { continue }
                if belt_tile.item.is_some() { break }   // belt slot occupied

                // Pick a bar — first non-zero ore in BTreeMap iteration order
                // (Copper < Silver < Gold per OreKind variant order).
                let mut to_drain: Option<OreKind> = None;
                for (&ore, &n) in state.output.iter() {
                    if n > 0 { to_drain = Some(ore); break }
                }
                let Some(ore) = to_drain else { break };
                if let Some(n) = state.output.get_mut(&ore) {
                    *n -= 1;
                    if *n == 0 { state.output.remove(&ore); }
                }
                belt_tile.item = Some(ItemKind::Bar(ore));
                pushed = true;
                break;
            }
            if pushed { break }   // at most one output per smelter per tick
        }
    }
}
```

**Algorithm notes:**
- Recipe-mismatch back-pressure (item stays on belt) is the M5a-MVP behavior — players will discover that putting different ore types into one smelter causes a jam. M5b can introduce per-ore smelters or a smelter that can switch recipes mid-stream.
- `SMELTER_QUEUE_CAP = 8` is a soft cap. The existing single-player UI path (`handle_smelter_buttons_system`) bypasses this cap (it sets `queue = count` directly via `start_smelting`). That's fine; the cap only governs the BELT pull rate.
- `state.output.remove(&ore)` when the count hits zero keeps the BTreeMap small. Optional but cleaner.

- [ ] **Step 2: Register system + set in `src/app.rs`**

Add `MachineSet::SmelterBeltIo` variant. Place it between `MachineSet::BeltTick` and `MachineSet::SmelterTick` in `configure_sets`.

```rust
app.add_systems(Update, crate::systems::belt::smelter_belt_io_system
    .in_set(MachineSet::SmelterBeltIo));
```

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 117 tests still passing.

- [ ] **Step 4: Commit**

```bash
git add src/systems/belt.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add smelter_belt_io_system: direction-of-belt rule for smelter I/O"
```

---

## Smoke-test checkpoint #2 (after Task 7)

Human controller runs `cargo run`. Expected:

- Single-player works as before. M3/M4 behaviors unchanged.
- Buy "Belt Networks" (200c). Press B, place belts in a line ending at the smelter. Place at least one belt pointing INTO the smelter from one side, and at least one pointing AWAY from the smelter on a different side.
- Dig ore so a drop lands on a belt tile (place a belt where you'll be digging into).
- Watch ore advance one tile per second. Reaches the smelter input belt → smelter consumes it (panel shows queue increment).
- After ~2 seconds, smelter produces a bar, pushes it onto the output belt.
- Bar advances along output belt, reaches the end, spills as a floor `OreDrop`.
- Vacuum the bar, walk to shop, sell — money increases.
- Right-click a belt with an item on it → belt despawns AND `OreDrop` spawns at that tile.
- Loop test: build a circular belt (4 belts in a square, all CW). Drop one ore. It should orbit forever — confirms the back-pressure / movement algorithm.
- Save (F5), load (F9) — belts and items reset to the saved state... wait, this fails because Task 8 hasn't landed yet. Skip the save/load checks at this checkpoint; F5 will save without belts (silently dropped) and F9 will restore an empty world. **This is expected** — Task 8 fixes it.

If the visible loop fails, surface to controller before Task 8.

---

## Task 8: Save/load — SAVE_VERSION 2 → 3, belts in SaveData

**Files:**
- Modify: `src/save.rs`
- Modify: `src/systems/save_load.rs`
- Modify: `tests/save.rs`

After this task: belts and items-on-belts persist via F5/F9 and auto-save on quit.

- [ ] **Step 1: Bump SAVE_VERSION + add belts field**

In `src/save.rs`:

```rust
pub const SAVE_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub grid: Grid,
    pub inventory: Inventory,
    pub money: Money,
    pub owned_tools: OwnedTools,
    pub smelter: SmelterState,
    pub player_pos: [f32; 2],
    pub belts: Vec<(IVec2, BeltTile)>,    // NEW
}
```

(Add `use crate::belt::BeltTile;` and `use bevy::math::IVec2;` if not present.)

Update `save::collect` to take a belts iterator (or a Vec) and include it in the SaveData. Update the helper signature in `save_load.rs`'s caller to pass it.

**Pick this approach (don't waste cycles on alternatives):** `save::apply` returns `Vec<(IVec2, BeltTile)>` (the loaded belts), and the calling Bevy system (in save_load.rs) does the despawn-old + spawn-new with `Commands`. This keeps `save::apply` pure (no `&mut Commands` parameter, no Bevy entity churn inside the save module).

Concrete signature change:
```rust
// Before:
pub fn apply(data: SaveData, grid: &mut Grid, inventory: &mut Inventory, ...) { ... }
// After:
pub fn apply(data: SaveData, grid: &mut Grid, inventory: &mut Inventory, ...) -> Vec<(IVec2, BeltTile)> {
    // ... existing in-place mutations ...
    data.belts  // return the loaded belt list for the caller to spawn
}
```

- [ ] **Step 2: Update save_load.rs systems**

For each of the four save/load systems:
- `save_now` flow: query `Query<(&Transform, &BeltTile)>`, build `Vec<(IVec2, BeltTile)>` via `world_to_tile`, pass to `save::collect`.
- `load_apply` flow: despawn all existing belt entities (via `Query<Entity, With<BeltTile>>`), then spawn new belt entities from the loaded `Vec<(IVec2, BeltTile)>`. Each spawn adds: `(BeltTile, BeltVisual::Straight, Sprite { ... }, Transform::from_translation(tile_center_world(pos).extend(3.0)))`. The `belt_visual_recompute_system` will fix up corners on the next frame.

- [ ] **Step 3: Update existing save tests**

In `tests/save.rs`, add `belts: vec![]` to all SaveData constructors. Update `version_mismatch_is_detected` if it asserts on a specific version (it shouldn't if it uses the constant).

- [ ] **Step 4: Add 3 new save tests in `tests/save.rs`**

```rust
#[test]
fn belts_round_trip_via_ron() {
    let mut data = sample_save_data();
    data.belts = vec![
        (IVec2::new(3, 5), BeltTile { item: Some(ItemKind::Ore(OreKind::Copper)), dir: BeltDir::East }),
        (IVec2::new(4, 5), BeltTile { item: None, dir: BeltDir::North }),
    ];
    let s = save::serialize_ron(&data).expect("ser");
    let parsed = save::deserialize_ron(&s).expect("de");
    assert_eq!(parsed.belts.len(), 2);
    assert_eq!(parsed.belts[0].0, IVec2::new(3, 5));
    assert_eq!(parsed.belts[0].1.item, Some(ItemKind::Ore(OreKind::Copper)));
    assert_eq!(parsed.belts[0].1.dir, BeltDir::East);
}

#[test]
fn belts_empty_default_round_trips() {
    // The common case: no belts in the save.
    let data = sample_save_data();
    assert_eq!(data.belts.len(), 0);
    let s = save::serialize_ron(&data).expect("ser");
    let parsed = save::deserialize_ron(&s).expect("de");
    assert_eq!(parsed.belts.len(), 0);
}

#[test]
fn version_3_rejects_v2_saves() {
    // Construct a SaveData and force version=2 then try to deserialize.
    let mut data = sample_save_data();
    data.version = 2;
    let s = save::serialize_ron(&data).expect("ser");
    let result = save::deserialize_ron(&s);
    assert!(matches!(result, Err(save::LoadError::VersionMismatch { found: 2, expected: 3 })));
}
```

- [ ] **Step 5: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 117 + 3 = 120 tests passing. Existing save tests now updated for the new field.

- [ ] **Step 6: Commit**

```bash
git add src/save.rs src/systems/save_load.rs tests/save.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Save/load: persist belts; SAVE_VERSION 2 -> 3"
```

---

## Task 9: Net events — `PlaceBeltRequest` and `RemoveBeltRequest` (TDD)

**Files:**
- Modify: `src/systems/net_events.rs`
- Modify: `tests/net_events.rs`

After this task: two new client-fired events exist with derives required for replicon (`Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq`) and serde round-trip tests prove they serialize cleanly.

- [ ] **Step 1: Write failing tests in `tests/net_events.rs`**

Append:

```rust
use bevy::math::IVec2;
use miningsim::belt::BeltDir;
use miningsim::systems::net_events::{PlaceBeltRequest, RemoveBeltRequest};

#[test]
fn place_belt_request_round_trips() {
    let original = PlaceBeltRequest { tile: IVec2::new(7, 12), dir: BeltDir::North };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: PlaceBeltRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}

#[test]
fn remove_belt_request_round_trips() {
    let original = RemoveBeltRequest { tile: IVec2::new(3, 0) };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: RemoveBeltRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
cargo test --test net_events 2>&1 | tail -10
```
Expected: `unresolved import miningsim::systems::net_events::PlaceBeltRequest`.

- [ ] **Step 3: Append events to `src/systems/net_events.rs`**

```rust
use crate::belt::BeltDir;

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlaceBeltRequest { pub tile: bevy::math::IVec2, pub dir: BeltDir }

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RemoveBeltRequest { pub tile: bevy::math::IVec2 }
```

- [ ] **Step 4: Run tests — expect 7/7 passing (5 prior + 2 new)**

```bash
cargo test --test net_events 2>&1 | tail -10
```

- [ ] **Step 5: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: 120 + 2 = 122 tests passing.

- [ ] **Step 6: Commit**

```bash
git add src/systems/net_events.rs tests/net_events.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add PlaceBeltRequest + RemoveBeltRequest events with serde round-trip tests"
```

---

## Task 10: Multiplayer — replication + server handlers + client branching

**Files:**
- Modify: `src/systems/net_plugin.rs`
- Modify: `src/systems/belt_ui.rs` (branch placement/removal on NetMode)
- Modify: `src/systems/net_player.rs` (add `add_belt_visuals_on_arrival` and recompute BeltVisual on the client)

After this task: belts replicate from host to clients. Client placements/removals fire events validated server-side. Client-side belt sprites are attached on arrival.

This is the largest task in the plan because it spans three files and several distinct concerns. Decomposing further would create artificial commit boundaries; cleanest as one atomic commit.

- [ ] **Step 1: Register replication + events in MultiplayerPlugin**

In `src/systems/net_plugin.rs::MultiplayerPlugin::build`, add to the replicate chain:

```rust
.replicate::<crate::belt::BeltTile>()
```

(BeltVisual is NOT replicated — derived locally per the spec.)

Add the two new client events:

```rust
app.add_client_event::<PlaceBeltRequest>(Channel::Ordered);
app.add_client_event::<RemoveBeltRequest>(Channel::Ordered);
```

Import them: `use crate::systems::net_events::{..., PlaceBeltRequest, RemoveBeltRequest};`

- [ ] **Step 2: Add server-side handlers**

Add two new handler systems mirroring the existing `handle_*_requests` pattern:

```rust
pub fn handle_place_belt_requests(
    mut events: EventReader<FromClient<PlaceBeltRequest>>,
    mut commands: Commands,
    grid: Single<&crate::grid::Grid>,
    belts_q: Query<&Transform, With<BeltTile>>,
    shops_q: Query<&Transform, With<crate::components::Shop>>,
    smelters_q: Query<&Transform, With<crate::components::Smelter>>,
) {
    let grid = grid.into_inner();
    for FromClient { event, .. } in events.read() {
        // Validate (same rules as belt_ui::validate_belt_placement).
        let tile = event.tile;
        let Some(g) = grid.get(tile.x, tile.y) else { continue };
        if g.solid { continue };
        if belts_q.iter().any(|xf| world_to_tile(xf.translation.truncate()) == tile) { continue };
        if shops_q.iter().chain(smelters_q.iter()).any(|xf| world_to_tile(xf.translation.truncate()) == tile) { continue };

        let center = tile_center_world(tile);
        commands.spawn((
            BeltTile::new(event.dir),
            BeltVisual::Straight,
            Transform::from_translation(center.extend(3.0)),
            Replicated,
        ));
    }
}

pub fn handle_remove_belt_requests(
    mut events: EventReader<FromClient<RemoveBeltRequest>>,
    mut commands: Commands,
    belts_q: Query<(Entity, &Transform, &BeltTile)>,
) {
    for FromClient { event, .. } in events.read() {
        let target = event.tile;
        for (e, xf, bt) in belts_q.iter() {
            let pos = world_to_tile(xf.translation.truncate());
            if pos != target { continue };

            // Spill item if present.
            if let Some(item) = bt.item {
                let center = tile_center_world(pos);
                commands.spawn((
                    crate::components::OreDrop { item },
                    Sprite {
                        color: crate::systems::hud::item_color(item),
                        custom_size: Some(Vec2::splat(6.0)),
                        ..default()
                    },
                    Transform::from_translation(center.extend(4.0)),
                    Replicated,  // ore drop spawned by host should replicate to clients
                ));
            }
            commands.entity(e).despawn();
            break;
        }
    }
}
```

Register both with `.run_if(server_running)` in the existing handler tuple in `MultiplayerPlugin::build`.

- [ ] **Step 3: Branch belt_ui on NetMode + share validation**

The validation rules ("tile is in-bounds, floor, no existing belt, no machine") need to fire in two places: client-side `belt_place_system` (single-player + host direct-mutation path) and server-side `handle_place_belt_requests`. Both take Bevy queries as their natural input shape, so a Bevy-system-aware helper isn't pure.

**Strategy:** add a small pure helper in `src/belt.rs` that takes already-collected occupancy data:

```rust
// In src/belt.rs (append to Task 1's content):
use std::collections::BTreeSet;

/// Validate a candidate belt placement. Pure — caller projects World state
/// into the (occupied tiles, grid solidity) inputs.
pub fn can_place_belt(
    tile: bevy::math::IVec2,
    in_bounds_and_floor: bool,
    occupied_tiles: &BTreeSet<bevy::math::IVec2>,
) -> bool {
    in_bounds_and_floor && !occupied_tiles.contains(&tile)
}
```

(Add a 1-2 unit tests for `can_place_belt` to `tests/belt.rs` while you're here — bumps the count to ~18.)

Then both call sites project into that shape:
- `belt_place_system`: `let in_bounds_and_floor = grid.get(tile.x, tile.y).is_some_and(|g| !g.solid);` and build `occupied: BTreeSet<IVec2>` from belts + shops + smelters queries.
- `handle_place_belt_requests`: same projection from the server-side queries.

In `belt_place_system`, also add the NetMode branch. Final shape:

```rust
pub fn belt_place_system(
    mut commands: Commands,
    build_mode: Res<BeltBuildMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    grid_q: Option<Single<&Grid>>,
    belts_q: Query<&Transform, With<BeltTile>>,
    shops_q: Query<&Transform, With<Shop>>,
    smelters_q: Query<&Transform, With<Smelter>>,
    net_mode: Res<crate::net::NetMode>,
    mut place_writer: EventWriter<crate::systems::net_events::PlaceBeltRequest>,
) {
    let Some(dir) = build_mode.cursor_dir else { return };
    if !mouse.just_pressed(MouseButton::Left) { return }

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
    let tile = world_to_tile(cursor_world);

    if matches!(*net_mode, crate::net::NetMode::Client { .. }) {
        place_writer.send(crate::systems::net_events::PlaceBeltRequest { tile, dir });
        return;
    }

    // SinglePlayer / Host: validate + spawn directly.
    let Some(grid) = grid_q else { return };
    let grid = grid.into_inner();
    let in_bounds_and_floor = grid.get(tile.x, tile.y).is_some_and(|g| !g.solid);
    let occupied: std::collections::BTreeSet<bevy::math::IVec2> = belts_q.iter()
        .chain(shops_q.iter())
        .chain(smelters_q.iter())
        .map(|xf| world_to_tile(xf.translation.truncate()))
        .collect();
    if !belt::can_place_belt(tile, in_bounds_and_floor, &occupied) { return }

    let center = tile_center_world(tile);
    commands.spawn((
        BeltTile::new(dir),
        BeltVisual::Straight,  // recomputed by belt_visual_recompute_system next frame
        Sprite {
            color: belt_color(dir),
            custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
            ..default()
        },
        Transform::from_translation(center.extend(3.0)),
        bevy_replicon::prelude::Replicated,  // inert in single-player; needed in Host mode
    ));
}
```

Note: `belt_color` is the existing helper in belt_ui.rs. The `Replicated` import goes at the top of belt_ui.rs (the M4 lesson: `Replicated` is inert without RepliconPlugins, safe to add unconditionally — same pattern setup.rs uses for the host's player).

Same pattern for `belt_remove_system`: add `net_mode` + `RemoveBeltRequest` writer, branch on `Client` (fire event + return), otherwise existing direct-despawn path.

For Host mode: the host's local clicks go through the direct-spawn path (just like dig + buy + sell in M4). The server-side `handle_place_belt_requests` only fires for events from REMOTE clients (the host's own player has no `OwningClient`).

**Update Task 3's spawn:** the original Task 3 spawn snippet lacks `Replicated`. Add it there too (alternatively, fold this addition into Task 10 as a one-line tweak to belt_ui.rs's spawn tuple — either works). The plan is now showing the corrected version above; if Task 3 was already implemented, this edit lands as part of the Task 10 commit.

- [ ] **Step 4: Add `add_belt_visuals_on_arrival` system**

In `src/systems/net_player.rs`, add (mirroring `add_shop_visuals_on_arrival` etc.):

```rust
/// Replicon doesn't ship Sprite over the wire. When a BeltTile entity arrives
/// via replication, attach the local visual sprite. Direction-keyed color.
pub fn add_belt_visuals_on_arrival(
    mut commands: Commands,
    new_belts: Query<(Entity, &crate::belt::BeltTile), (Added<crate::belt::BeltTile>, Without<Sprite>)>,
) {
    for (e, bt) in new_belts.iter() {
        let color = belt_color_for_dir(bt.dir);
        commands.entity(e).insert((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(crate::coords::TILE_SIZE_PX)),
                ..default()
            },
            crate::belt::BeltVisual::Straight,  // recomputed on next frame
        ));
    }
}

fn belt_color_for_dir(dir: crate::belt::BeltDir) -> Color {
    use crate::belt::BeltDir;
    match dir {
        BeltDir::North => Color::srgb(0.20, 0.55, 0.20),
        BeltDir::East  => Color::srgb(0.60, 0.55, 0.20),
        BeltDir::South => Color::srgb(0.55, 0.20, 0.20),
        BeltDir::West  => Color::srgb(0.20, 0.20, 0.55),
    }
}
```

Register in `MultiplayerPlugin::build` (Update, no run-condition — `Added<BeltTile>` only fires when one is newly added).

- [ ] **Step 5: Build + test**

```bash
cargo build 2>&1 | tail -20
cargo test 2>&1 | grep "test result"
```
Expected: 122 tests still passing. Build clean.

- [ ] **Step 6: Commit**

```bash
git add src/systems/net_plugin.rs src/systems/belt_ui.rs src/systems/net_player.rs src/belt.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Multiplayer: replicate BeltTile, server handlers, client branching, visual attachment"
```

---

## Smoke-test checkpoint #3 (after Task 10)

Two-window test:

```bash
# Terminal A:
cargo run -- host

# Terminal B:
cargo run -- join 127.0.0.1:5000
```

Expected:

- Both players launch, both see each other's blue/orange sprites.
- Both buy "Belt Networks" independently. Each player's `OwnedTools` reflects only their own purchase.
- Player A presses B → A enters build mode (sees ghost). B does NOT see A's ghost.
- Player A places several belts. Player B sees them appear within ~1 frame.
- Player B drops ore on a belt placed by A. Ore advances on both screens.
- Smelter consumes the ore, produces a bar, pushes onto an output belt placed by either player.
- Player B right-clicks a belt placed by A → trust-based removal succeeds. Belt disappears on both screens.
- Items on the removed belt spill as `OreDrop` on both screens.
- Disconnect host → client cleanly exits (M4 behavior preserved).
- Save (F5) in single-player mode (after restarting) — belts persist via Task 8.

If any item fails, surface to controller for triage before Task 11.

---

## Task 11: Final playtest, roadmap update, merge to main

- [ ] **Step 1: Run full test suite**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: ~122 tests passing across all suites.

- [ ] **Step 2: Manual exit-criteria walkthrough**

Run the full multi-mode playtest from the spec's "Manual playtest exit criteria" section:

Single-player:
- [ ] `cargo run` → fresh world, F5/F9/AppExit work, M3/M4 behaviors work.
- [ ] Buy Belt Networks (200c). Build mode toggles, scroll-rotate, place, remove all work.
- [ ] Loop: dig adjacent to belt → ore on belt → smelter → bar → spillage → vacuum → sell.
- [ ] Save and load preserves belts and items-on-belts.

Two-window co-op (full M4 + M5a checklist):
- [ ] Both players see each other's belts in real time.
- [ ] Trust-based placement and removal.
- [ ] Items advance smoothly on both peers.
- [ ] Smelter shared (input from any player's belt; output to any player's belt).
- [ ] Per-player money/inventory/tools intact (M4 behavior).
- [ ] Disconnect drops cleanly.

Stability:
- [ ] 15-min mixed session, no crashes, no orphaned items, no stuck belts.

- [ ] **Step 3: Append a Milestone 5a section to `docs/roadmap.md`**

Match the format of the existing M1/M2/M3/M4 playtest results sections. Cover:

- Exit criteria met (one-paragraph summary)
- What felt good
- What felt off (and was fixed mid-flight, or remains as known issue)
- What we deliberately deferred (e.g., per-tile cost, mergers, multi-stage recipes — all going to M5b+)
- Decisions for the next milestone (M5b)

- [ ] **Step 4: Commit playtest notes**

```bash
git add docs/roadmap.md
git commit --author="wes2000 <whannasch@gmail.com>" -m "Record milestone 5a playtest results"
```

- [ ] **Step 5: Merge to main + push**

```bash
git checkout main
git merge --no-ff milestone-5a -m "Merge milestone-5a: conveyor belts MVP"
git push origin main
git branch -d milestone-5a
```

- [ ] **Step 6: Final code review (recommended)**

Dispatch the `superpowers:code-reviewer` subagent against the merged `main` HEAD with explicit scope: identify cross-cutting issues, hidden coupling, and "fix-before-M5b" callouts. Especially:
- Belt vs. system-set ordering correctness in app.rs
- Per-frame cost of belt_tick_system when there are 50+ belts
- Edge cases around BeltVisual recomputation (does it fire on every belt every frame? Should it be `Changed<BeltTile>`-gated?)
- Any new bare `Single<…>` instances that should be `Option<Single<…>>` per the M4 lesson

If any callouts surface, file as a follow-up commit before brainstorming M5b.

Milestone 5a complete.
