# Milestone 3 — Surface Base + First Processing Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [../specs/2026-04-18-milestone-3-processing-loop-design.md](../specs/2026-04-18-milestone-3-processing-loop-design.md)

**Goal:** Add a surface Smelter that processes raw ores into bars over time (with a Collect-All output buffer), wire bars through the existing shop at 5× the raw-ore price, and refactor the codebase to use a clean `ItemKind { Ore(OreKind), Bar(OreKind) }` type model — closing the dig → process → sell → upgrade loop and resolving M2's `OreType::None` sentinel debt at the same time.

**Architecture:** Three new pure modules (`items.rs`, `processing.rs`, `coords.rs`) and one new system module (`systems/smelter.rs`) sit on top of the M2 architecture. The big refactor is done as a single atomic Task 4 commit so the crate is broken for at most one task. M2 cleanups (coords helper extraction; Shop Buy affordability state) ship as Tasks 1–2 before any M3 feature work.

**Tech Stack:** Rust (stable), Bevy 0.15.x (pinned), built-in `cargo test`.

---

## Pre-flight: environment expectations

This plan assumes:
- Rust stable toolchain ≥ 1.82 (already installed; `rustc --version` ≥ 1.82).
- Working directory: `c:/Users/whann/Desktop/Games/miningsim` (existing git repo, branch `milestone-3` already created from `main`).
- `cargo test` currently passes 54/54 at `main`'s `58129e6` (M2 + post-merge cleanup).
- Author identity: commits use `--author="wes2000 <whannasch@gmail.com>"`. Do not modify global git config.

If any of these aren't true, stop and resolve before proceeding.

---

## File structure (target end state)

```
src/
  coords.rs                     # NEW: pure — world_to_tile, tile_min_world, tile_center_world, TILE_SIZE_PX
  items.rs                      # NEW: pure — OreKind, ItemKind, ALL_ORES, ALL_ITEMS
  processing.rs                 # NEW: pure — SmelterState, tick_smelter, start_smelting, collect_output, is_busy, SMELT_DURATION_S
  grid.rs                       # MODIFY: Tile.ore: Option<OreKind>; OreType removed
  inventory.rs                  # MODIFY: HashMap<ItemKind, u32>
  dig.rs                        # MODIFY: DigResult.ore: Option<OreKind>
  terrain_gen.rs                # MODIFY: Option<OreKind>; ore prob curves keyed by OreKind
  tools.rs                      # unchanged
  economy.rs                    # MODIFY: prices keyed by ItemKind; sell_all iterates ALL_ITEMS
  components.rs                 # MODIFY: drop OreType, add ItemKind on OreSprite/OreDrop;
                                #         add Smelter, SmelterUiRoot, SmelterButtonKind, SmelterStatusText markers
                                #         add SmelterUiOpen resource
  systems/
    setup.rs                    # MODIFY: import TILE_SIZE_PX from coords; spawn Smelter entity
    player.rs                   # MODIFY: ItemKind in OreDrop spawn; coords helpers
    chunk_lifecycle.rs          # MODIFY: coords helpers
    chunk_render.rs             # MODIFY: Option<OreKind>; coords helpers
    ore_drop.rs                 # MODIFY: ItemKind on OreDrop; pickup adds via ItemKind
    hud.rs                      # MODIFY: 6 item rows driven by ALL_ITEMS
    shop.rs                     # unchanged
    shop_ui.rs                  # MODIFY: Sell All iterates ALL_ITEMS; affordability state on Buy buttons
    smelter.rs                  # NEW: smelter_interact, walk_away, tick, panel spawn/sync/refresh, button handler
  app.rs                        # MODIFY: register smelter systems; group into named SystemSets
  lib.rs                        # MODIFY: pub mod coords, items, processing
  main.rs                       # unchanged
tests/
  coords.rs                     # NEW: round-trip + Y-inversion
  items.rs                      # NEW: enum + constants smoke
  processing.rs                 # NEW: tick_smelter math
  grid.rs                       # MODIFY: Option<OreKind>
  inventory.rs                  # MODIFY: ItemKind keys; mixed ore+bar round-trip
  terrain_gen.rs                # MODIFY: assertions in OreKind terms
  dig.rs                        # MODIFY: try_dig returns Option<OreKind>
  tools.rs                      # unchanged
  economy.rs                    # MODIFY: ItemKind prices, mixed-inventory sell_all
```

---

## Conventions

- Commit style: present-tense imperative. `--author="wes2000 <whannasch@gmail.com>"` on every commit.
- Pure modules follow TDD: failing test first → verify fail → implement minimum → verify pass → commit.
- Bevy systems are not unit-tested. `cargo build` + `cargo test` (regression on pure modules) is the verification gate; visual smoke tests are deferred to documented user checkpoints.
- `cargo run` blocks on the Bevy window — **subagents must not run it**. Use `cargo build` + `cargo test`; the human controller drives `cargo run` at smoke-test checkpoints.
- Bevy 0.15.x API drift: M2 hit several patch-version surprises (`children![]` macro absent, `ChildBuilder` vs `ChildSpawnerCommands`). Adapt minimally and document in the task report.
- The big refactor in Task 4 is **single-commit, atomic**. Before commit, the crate must build and `cargo test` must be green. Sub-edits during the task may temporarily break compile; that's fine inside the task.

---

## User smoke-test checkpoints

Two visual verification moments:

1. **After Task 4 (refactor)** — purely a regression check. Run the game; mining, shop, tools, all M2 behaviors should work identically. HUD has not yet grown to 6 rows (that's still 3 — bar rows arrive later when the smelter is plumbed).

   Wait — actually Task 4 also updates `hud.rs` to drive rows from `ALL_ITEMS` (6 rows). So at Task 4, HUD shows 6 rows but bar rows always show 0 (no source of bars yet). Confirm HUD layout looks reasonable with 6 rows.

2. **After Task 9 (smelter UI wired)** — full processing loop. Walk to Smelter, smelt ore, wait, collect, sell at shop for bar prices, buy tools.

3. **After Task 10 (final)** — exit-criteria walkthrough.

---

## Task 1: Extract `coords.rs` (TDD, M2 cleanup)

**Files:**
- Create: `src/coords.rs`
- Create: `tests/coords.rs`
- Modify: `src/lib.rs`
- Modify: `src/systems/setup.rs` — remove TILE_SIZE_PX + tile_center_world; import from coords
- Modify: `src/systems/player.rs`, `src/systems/chunk_lifecycle.rs`, `src/systems/chunk_render.rs` — replace inline conversions with calls to coords helpers

- [ ] **Step 1: Write failing tests in `tests/coords.rs`**

```rust
use bevy::math::{IVec2, Vec2};
use miningsim::coords::{self, TILE_SIZE_PX};

#[test]
fn tile_size_px_is_16() {
    assert_eq!(TILE_SIZE_PX, 16.0);
}

#[test]
fn tile_center_world_round_trip() {
    for &(x, y) in &[(0i32, 0), (1, 1), (5, 10), (-3, 4), (0, 200)] {
        let c = coords::tile_center_world(IVec2::new(x, y));
        assert_eq!(coords::world_to_tile(c), IVec2::new(x, y));
    }
}

#[test]
fn world_y_inversion() {
    // Tile (0, 0) is at the top of the map. Its world y is negative because Bevy y goes up.
    let c = coords::tile_center_world(IVec2::new(0, 0));
    assert_eq!(c.y, -8.0);
    let c2 = coords::tile_center_world(IVec2::new(0, 1));
    assert!(c2.y < c.y, "deeper tile should have more negative world y");
}

#[test]
fn tile_min_world_corners() {
    // Tile (3, 5) min corner: world.x = 3 * 16 = 48, world.y = -((5+1) * 16) = -96.
    let m = coords::tile_min_world(IVec2::new(3, 5));
    assert_eq!(m, Vec2::new(48.0, -96.0));
}

#[test]
fn world_to_tile_at_negative_world_x() {
    // World x = -1 maps to tile x = -1 (floor).
    assert_eq!(coords::world_to_tile(Vec2::new(-1.0, -8.0)).x, -1);
    // World x = 0 maps to tile x = 0.
    assert_eq!(coords::world_to_tile(Vec2::new(0.0, -8.0)).x, 0);
}
```

- [ ] **Step 2: Run tests to verify fail (compile error — module missing)**

```bash
cargo test --test coords 2>&1 | tail -10
```

- [ ] **Step 3: Implement `src/coords.rs`**

```rust
use bevy::math::{IVec2, Vec2};

pub const TILE_SIZE_PX: f32 = 16.0;

pub fn world_to_tile(world: Vec2) -> IVec2 {
    IVec2::new(
        (world.x / TILE_SIZE_PX).floor() as i32,
        ((-world.y) / TILE_SIZE_PX).floor() as i32,
    )
}

pub fn tile_min_world(tile: IVec2) -> Vec2 {
    Vec2::new(
        tile.x as f32 * TILE_SIZE_PX,
        -((tile.y + 1) as f32) * TILE_SIZE_PX,
    )
}

pub fn tile_center_world(tile: IVec2) -> Vec2 {
    Vec2::new(
        tile.x as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0,
        -(tile.y as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0),
    )
}
```

- [ ] **Step 4: Register the module in `src/lib.rs`**

Insert `pub mod coords;` in alphabetical order (likely between `components` and `dig`).

- [ ] **Step 5: Run new tests; expect 5/5 passing**

```bash
cargo test --test coords 2>&1 | tail -10
```

- [ ] **Step 6: Migrate call sites**

Replace inline `(world.x / TILE_SIZE_PX).floor() as i32` patterns and inline `Vec2::new(tx as f32 * TILE_SIZE_PX + ...)` patterns with `coords::world_to_tile(...)`, `coords::tile_min_world(...)`, `coords::tile_center_world(...)`.

Sites to update (verify by grep — there are at least 6):
- `src/systems/setup.rs`: delete the local `TILE_SIZE_PX` constant and the `tile_center_world` fn; import from `coords`. The `Player` and `Shop` spawn sites use `tile_center_world` — update those calls to use `IVec2::new(...)` instead of `(i32, i32)` tuples (signature changed).
- `src/systems/player.rs`: `dig_input_system` computes `tx`/`ty`/`tile_center` and `player_tile`. Replace with `coords::world_to_tile(cursor_world)` and `coords::world_to_tile(player_xf.translation.truncate())` and `coords::tile_center_world(target_tile)`. Also `collide_player_with_grid_system` has inline `tx0 = (min.x / TILE_SIZE_PX).floor() as i32` etc. — replace each call with `coords::world_to_tile(...)` (or keep inline for the AABB-corner case if it's clearer; use judgment).
- `src/systems/chunk_lifecycle.rs`: `world_to_chunk` calls `(world.x / TILE_SIZE_PX).floor() as i32` etc. Refactor to use `coords::world_to_tile`.
- `src/systems/chunk_render.rs`: per-tile `world_x = gx * TILE_SIZE_PX + TILE_SIZE_PX / 2.0` and `world_y = -(...)`. Replace with `coords::tile_center_world(IVec2::new(gx, gy))`.
- Any other site flagged by `cargo build` warnings or by the M2 final review (six sites total per the review).

Note: the function signature `tile_center_world` changed from `(i32, i32) -> Vec2` to `(IVec2) -> Vec2`. Update call sites accordingly.

- [ ] **Step 7: Build + full test regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result" | tail -10
```
Expected: 54 + 5 = 59 tests passing.

- [ ] **Step 8: Commit**

```bash
git add src/coords.rs src/lib.rs tests/coords.rs src/systems/setup.rs src/systems/player.rs src/systems/chunk_lifecycle.rs src/systems/chunk_render.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Extract coords module: world_to_tile, tile_min_world, tile_center_world"
```

---

## Task 2: Shop Buy buttons show affordability state

**Files:**
- Modify: `src/systems/shop_ui.rs`

- [ ] **Step 1: Update `update_shop_labels_system` to set `BackgroundColor` based on affordability**

Inside `update_shop_labels_system`, when iterating Buy buttons, in addition to the label refresh, also write the row's `BackgroundColor`. Since `BackgroundColor` is on the button itself (the `Node` carrying `ShopButtonKind`), update it via a separate query parameter:

```rust
pub fn update_shop_labels_system(
    money: Res<Money>,
    owned: Res<OwnedTools>,
    buttons_q: Query<(&ShopButtonKind, &Children, Entity)>,
    mut bg_q: Query<&mut BackgroundColor>,
    mut texts_q: Query<&mut Text>,
) {
    if !money.is_changed() && !owned.is_changed() { return; }
    for (kind, children, entity) in buttons_q.iter() {
        match kind {
            ShopButtonKind::SellAll => { /* static label, static color */ }
            ShopButtonKind::Buy(tool) => {
                let owned_already = owned.0.contains(tool);
                let price = economy::tool_buy_price(*tool);
                let affordable = money.0 >= price;

                let new_label = if owned_already {
                    format!("{} - OWNED", current_tool_display_name(*tool))
                } else {
                    format!("Buy {} - {}c", current_tool_display_name(*tool), price)
                };
                for c in children.iter() {
                    if let Ok(mut text) = texts_q.get_mut(*c) {
                        **text = new_label.clone();
                    }
                }

                // Background color signals interactability:
                //   normal (affordable, not owned) — slightly lit
                //   dimmed (broke or already owned) — darker
                let new_bg = if owned_already || !affordable {
                    Color::srgb(0.16, 0.16, 0.18)
                } else {
                    Color::srgb(0.22, 0.22, 0.28)
                };
                if let Ok(mut bg) = bg_q.get_mut(entity) {
                    *bg = BackgroundColor(new_bg);
                }
            }
        }
    }
}
```

`handle_shop_buttons_system` is unchanged — `try_buy` already returns `NotEnoughMoney` / `AlreadyOwned` on those cases and the call site discards the result.

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green; 59 tests still passing (no new tests).

- [ ] **Step 3: Commit**

```bash
git add src/systems/shop_ui.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Shop UI: dim Buy buttons when unaffordable or already owned"
```

---

## Task 3: `items.rs` pure module (TDD, additive)

**Files:**
- Create: `src/items.rs`
- Create: `tests/items.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Register the module**

Add `pub mod items;` to `src/lib.rs` in alphabetical order (likely between `inventory` and `lib`).

- [ ] **Step 2: Write failing tests in `tests/items.rs`**

```rust
use miningsim::items::{ItemKind, OreKind, ALL_ITEMS, ALL_ORES};

#[test]
fn all_ores_lists_three_kinds() {
    assert_eq!(ALL_ORES.len(), 3);
    assert!(ALL_ORES.contains(&OreKind::Copper));
    assert!(ALL_ORES.contains(&OreKind::Silver));
    assert!(ALL_ORES.contains(&OreKind::Gold));
}

#[test]
fn all_items_lists_six_combinations() {
    assert_eq!(ALL_ITEMS.len(), 6);
    for ore in ALL_ORES {
        assert!(ALL_ITEMS.contains(&ItemKind::Ore(ore)));
        assert!(ALL_ITEMS.contains(&ItemKind::Bar(ore)));
    }
}

#[test]
fn item_kind_round_trips_through_hashset() {
    use std::collections::HashSet;
    let s: HashSet<ItemKind> = ALL_ITEMS.iter().copied().collect();
    assert_eq!(s.len(), 6);
}

#[test]
fn ore_kind_and_item_kind_are_copy() {
    let o = OreKind::Copper;
    let _o2 = o;
    let _o3 = o;
    let i = ItemKind::Ore(OreKind::Silver);
    let _i2 = i;
    let _i3 = i;
}
```

- [ ] **Step 3: Run tests to verify fail**

```bash
cargo test --test items 2>&1 | tail -10
```

- [ ] **Step 4: Implement `src/items.rs`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OreKind {
    Copper,
    Silver,
    Gold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Ore(OreKind),
    Bar(OreKind),
}

pub const ALL_ORES: [OreKind; 3] = [OreKind::Copper, OreKind::Silver, OreKind::Gold];

pub const ALL_ITEMS: [ItemKind; 6] = [
    ItemKind::Ore(OreKind::Copper),
    ItemKind::Ore(OreKind::Silver),
    ItemKind::Ore(OreKind::Gold),
    ItemKind::Bar(OreKind::Copper),
    ItemKind::Bar(OreKind::Silver),
    ItemKind::Bar(OreKind::Gold),
];
```

- [ ] **Step 5: Run tests to verify pass**

```bash
cargo test --test items 2>&1 | tail -10
```
Expected: 4/4 passing.

- [ ] **Step 6: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: 59 + 4 = 63 tests passing.

- [ ] **Step 7: Commit**

```bash
git add src/items.rs src/lib.rs tests/items.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add items module: OreKind, ItemKind, ALL_ORES, ALL_ITEMS"
```

---

## Task 4: Big refactor — `OreType` → `OreKind` / `ItemKind` (atomic, sweeping)

**Files (all touched in one commit):**
- Modify: `src/grid.rs` — `Tile.ore: Option<OreKind>`; remove `OreType` enum entirely
- Modify: `src/dig.rs` — `DigResult.ore: Option<OreKind>`
- Modify: `src/terrain_gen.rs` — `Option<OreKind>`; `ore_probs` keyed by `OreKind`
- Modify: `src/inventory.rs` — `HashMap<ItemKind, u32>`
- Modify: `src/economy.rs` — `item_sell_price(ItemKind)`; `sell_all` iterates `ALL_ITEMS`
- Modify: `src/components.rs` — `OreSprite { ore: OreKind }`, `OreDrop { item: ItemKind }`
- Modify: `src/systems/player.rs` — drop spawn now carries `ItemKind::Ore(ore)`
- Modify: `src/systems/ore_drop.rs` — pickup adds via `ItemKind`
- Modify: `src/systems/chunk_render.rs` — `Option<OreKind>` rendering
- Modify: `src/systems/hud.rs` — 6 item rows from `ALL_ITEMS`; `ore_visual_color` becomes `item_color(ItemKind)` (or similar)
- Modify: `src/systems/shop_ui.rs` — `Sell All` semantics unchanged at the call site (it calls `economy::sell_all` which now iterates `ALL_ITEMS`)
- Modify: `tests/grid.rs`, `tests/inventory.rs`, `tests/dig.rs`, `tests/terrain_gen.rs`, `tests/economy.rs` — replace `OreType` with `OreKind` / `ItemKind` literals

This is the single biggest commit of the milestone. Work through it methodically: read the spec section "Components / modules in detail" for the new types and signatures.

- [ ] **Step 1: Write the new test bodies first**

In each affected `tests/*.rs`, rewrite assertions to use `OreKind` and `ItemKind`. For example, in `tests/grid.rs`:

```rust
// Was: g.set(1, 1, Tile { solid: true, layer: Layer::Stone, ore: OreType::Silver, damage: 2 });
g.set(1, 1, Tile { solid: true, layer: Layer::Stone, ore: Some(OreKind::Silver), damage: 2 });

// Was: assert_eq!(g.get(1, 1).unwrap().ore, OreType::None);
assert_eq!(g.get(1, 1).unwrap().ore, None);
```

In `tests/inventory.rs`:

```rust
// Was: inv.add(OreType::Copper, 3);
inv.add(ItemKind::Ore(OreKind::Copper), 3);
```

Similarly across `tests/dig.rs`, `tests/terrain_gen.rs`, `tests/economy.rs`.

Add ONE new mixed-inventory test to `tests/inventory.rs`:

```rust
#[test]
fn inventory_holds_ores_and_bars_distinctly() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Copper), 3);
    inv.add(ItemKind::Bar(OreKind::Copper), 2);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Copper)), 3);
    assert_eq!(inv.get(ItemKind::Bar(OreKind::Copper)), 2);
}
```

Add ONE new mixed-inventory test to `tests/economy.rs`:

```rust
#[test]
fn sell_all_sums_ores_and_bars() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Copper), 5);    //  5 *  1 =  5
    inv.add(ItemKind::Bar(OreKind::Copper), 3);    //  3 *  5 = 15
    inv.add(ItemKind::Bar(OreKind::Gold), 1);      //  1 * 100 = 100
    let mut money = Money::default();
    economy::sell_all(&mut inv, &mut money);
    assert_eq!(money.0, 120);
    for item in miningsim::items::ALL_ITEMS {
        assert_eq!(inv.get(item), 0);
    }
}
```

These tests should fail to compile until the implementation is updated. Don't try to run them yet; the crate is broken.

- [ ] **Step 2: Update `src/grid.rs`**

Remove the entire `OreType` enum. Update `Tile`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub solid: bool,
    pub layer: Layer,
    pub ore: Option<crate::items::OreKind>,
    pub damage: u8,
}

impl Default for Tile {
    fn default() -> Self {
        Self { solid: true, layer: Layer::Dirt, ore: None, damage: 0 }
    }
}
```

`OreType` is gone; downstream files will fail to compile until updated.

- [ ] **Step 3: Update `src/inventory.rs`**

```rust
use std::collections::HashMap;
use bevy::prelude::Resource;
use crate::items::ItemKind;

#[derive(Debug, Default, Resource)]
pub struct Inventory {
    counts: HashMap<ItemKind, u32>,
}

impl Inventory {
    pub fn add(&mut self, item: ItemKind, n: u32) {
        if n == 0 { return; }
        *self.counts.entry(item).or_insert(0) += n;
    }

    pub fn remove(&mut self, item: ItemKind, n: u32) {
        if let Some(c) = self.counts.get_mut(&item) {
            *c = c.saturating_sub(n);
        }
    }

    pub fn get(&self, item: ItemKind) -> u32 {
        *self.counts.get(&item).unwrap_or(&0)
    }
}
```

(Note: also fixes the M2 advisory about `remove` materializing zero entries — uses `get_mut` rather than `entry().or_insert(0)`.)

- [ ] **Step 4: Update `src/dig.rs`**

```rust
// In DigResult:
pub struct DigResult {
    pub status: DigStatus,
    pub ore: Option<crate::items::OreKind>,   // was OreType
}

// In try_dig: capture and pass through Option<OreKind>.
// On Broken: ore: tile.ore (which is now Option<OreKind>).
// Other branches: ore: None.
```

- [ ] **Step 5: Update `src/terrain_gen.rs`**

`maybe_assign_ore` operates on `&mut Option<OreKind>`. `ore_probs(layer)` returns `[(OreKind, f32); 3]` — but for None-producing layers (`Bedrock`, `Core`), there's no valid `OreKind` to put in the array. Two options:

(a) Refactor `ore_probs` to return `Vec<(OreKind, f32)>` and have empty for those layers; `maybe_assign_ore` skips empty.

(b) Keep `[(OreKind, f32); 3]` but use `OreKind::Copper` (or any) with prob `0.0` as a filler.

Pick (a) for clarity:

```rust
fn ore_probs(layer: Layer) -> Vec<(OreKind, f32)> {
    match layer {
        Layer::Dirt  => vec![(OreKind::Copper, 0.04),  (OreKind::Silver, 0.005), (OreKind::Gold, 0.0)],
        Layer::Stone => vec![(OreKind::Copper, 0.02),  (OreKind::Silver, 0.025), (OreKind::Gold, 0.003)],
        Layer::Deep  => vec![(OreKind::Copper, 0.005), (OreKind::Silver, 0.015), (OreKind::Gold, 0.02)],
        Layer::Core | Layer::Bedrock => vec![],
    }
}

fn maybe_assign_ore(tile: &mut Tile, rng: &mut StdRng) {
    let probs = ore_probs(tile.layer);
    if probs.is_empty() { return; }
    let r: f32 = rng.gen();
    let mut acc = 0.0;
    for (ore, p) in probs {
        acc += p;
        if r < acc {
            tile.ore = Some(ore);
            return;
        }
    }
    // tile.ore remains None
}
```

- [ ] **Step 6: Update `src/economy.rs`**

```rust
use crate::items::{ItemKind, OreKind, ALL_ITEMS};

pub fn item_sell_price(item: ItemKind) -> u32 {
    match item {
        ItemKind::Ore(OreKind::Copper) => 1,
        ItemKind::Ore(OreKind::Silver) => 5,
        ItemKind::Ore(OreKind::Gold)   => 20,
        ItemKind::Bar(OreKind::Copper) => 5,
        ItemKind::Bar(OreKind::Silver) => 25,
        ItemKind::Bar(OreKind::Gold)   => 100,
    }
}

pub fn sell_all(inv: &mut Inventory, money: &mut Money) {
    for item in ALL_ITEMS {
        let count = inv.get(item);
        if count == 0 { continue; }
        money.0 += item_sell_price(item) * count;
        inv.remove(item, count);
    }
}
```

`tool_buy_price` and `try_buy` unchanged.

- [ ] **Step 7: Update `src/components.rs`**

```rust
use crate::items::{ItemKind, OreKind};

#[derive(Component)]
pub struct OreSprite {
    pub ore: OreKind,    // was OreType — Bar variant doesn't need a sprite component
}

#[derive(Component)]
pub struct OreDrop {
    pub item: ItemKind,  // was ore: OreType
}
```

`SellAll` and `Buy(Tool)` shop button kinds unchanged. (Smelter component additions land in Task 6.)

- [ ] **Step 8: Update `src/systems/chunk_render.rs`**

Render condition was `if t.ore != OreType::None`; becomes `if let Some(ore) = t.ore`. The color helper takes `OreKind` instead of `OreType`. Also import from `coords` for the `tile_center_world` calls (already done in Task 1) — verify nothing was missed.

- [ ] **Step 9: Update `src/systems/hud.rs`**

```rust
use crate::items::{ItemKind, OreKind, ALL_ITEMS};

#[derive(Component)]
pub struct ItemCountText(pub ItemKind);   // was OreCountText(OreType)

pub fn item_color(item: ItemKind) -> Color {
    match item {
        ItemKind::Ore(OreKind::Copper) => Color::srgb(0.85, 0.45, 0.20),
        ItemKind::Ore(OreKind::Silver) => Color::srgb(0.85, 0.85, 0.92),
        ItemKind::Ore(OreKind::Gold)   => Color::srgb(0.95, 0.78, 0.25),
        ItemKind::Bar(OreKind::Copper) => Color::srgb(0.95, 0.55, 0.30),
        ItemKind::Bar(OreKind::Silver) => Color::srgb(0.95, 0.95, 1.00),
        ItemKind::Bar(OreKind::Gold)   => Color::srgb(1.00, 0.88, 0.40),
    }
}

pub fn current_tool_display_name(t: Tool) -> &'static str { /* unchanged */ }

pub fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, top: Val::Px(8.0), left: Val::Px(8.0), flex_direction: FlexDirection::Column, ..default() },
        ))
        .with_children(|root| {
            for item in ALL_ITEMS {
                spawn_item_row(root, item);
            }
            // money + current tool rows unchanged
        });
}

fn spawn_item_row(root: &mut ChildBuilder, item: ItemKind) {
    // mirror M2 spawn_ore_row but parameterized by ItemKind, color from item_color, label "0"
}

pub fn update_hud_system(
    inv: Res<Inventory>,
    money: Res<Money>,
    owned: Res<OwnedTools>,
    mut item_q: Query<(&mut Text, &ItemCountText), (Without<MoneyText>, Without<CurrentToolText>)>,
    mut money_q: Query<&mut Text, (With<MoneyText>, Without<ItemCountText>, Without<CurrentToolText>)>,
    mut tool_q: Query<&mut Text, (With<CurrentToolText>, Without<ItemCountText>, Without<MoneyText>)>,
) {
    if inv.is_changed() {
        for (mut text, marker) in item_q.iter_mut() {
            **text = inv.get(marker.0).to_string();
        }
    }
    // money + tool refresh blocks unchanged
}
```

The `ItemKind`/`OreKind` rename of marker components ripples into the `Without<...>` filters and into `update_smelter_panel_system` later — keep names consistent.

**Implementation gotcha:** the M2 `update_hud_system` had `Without<OreCountText>` filter clauses on its money and tool queries to maintain disjointness. After renaming `OreCountText` → `ItemCountText`, every `Without<OreCountText>` becomes `Without<ItemCountText>`. A simple search-and-replace catches it; do this BEFORE running `cargo build` so the borrow-checker doesn't complain about query aliasing.

- [ ] **Step 10: Update `src/systems/player.rs`**

In `dig_input_system`, the OreDrop spawn:

```rust
if result.status == DigStatus::Broken {
    if let Some(ore) = result.ore {
        commands.spawn((
            OreDrop { item: ItemKind::Ore(ore) },
            Sprite {
                color: hud::item_color(ItemKind::Ore(ore)),
                custom_size: Some(Vec2::splat(6.0)),
                ..default()
            },
            Transform::from_translation(tile_center.extend(5.0)),
        ));
    }
}
```

- [ ] **Step 11: Update `src/systems/ore_drop.rs`**

```rust
use crate::components::{OreDrop, Player};
// ... existing imports ...

pub fn ore_drop_system(
    /* same params ... */
    mut inv: ResMut<Inventory>,
) {
    /* same body, but pickup line: */
    inv.add(drop.item, 1);
    /* drop.item is ItemKind */
}
```

- [ ] **Step 12: Verify build, then run all tests**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 63 + 2 (new inventory test + new economy test) = 65 tests passing. Adjust expected count if other tests added/removed.

If `cargo build` fails, the most likely culprits:
- A site that still references `OreType` literal name → grep `OreType` to find leftovers, replace.
- A test that constructs `Tile { ore: OreType::None }` → change to `ore: None`.
- A `match t.ore` that has `OreType::None` arm → either drop the arm or change to `None`.
- A `match` on `ItemKind` is non-exhaustive → add the missing arm or use `_`.

Iterate until clean.

- [ ] **Step 13: Commit**

```bash
git add src/grid.rs src/inventory.rs src/dig.rs src/terrain_gen.rs src/economy.rs src/components.rs src/systems/player.rs src/systems/ore_drop.rs src/systems/chunk_render.rs src/systems/hud.rs src/systems/shop_ui.rs tests/grid.rs tests/inventory.rs tests/dig.rs tests/terrain_gen.rs tests/economy.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Refactor: OreType -> OreKind/ItemKind; Inventory keyed by ItemKind"
```

---

## Smoke-test checkpoint #1 (after Task 4)

Human runs `cargo run`. Expected:
- HUD now shows 6 item rows (3 ore + 3 bar). Bar rows always read `0` (no source of bars yet).
- Mining still works exactly as M2: dig dirt, ore drops, ore counts increment.
- Shop sell + buy still works.
- No regressions in dig, collision, tool tiers, damage overlay.

If something is visibly broken, surface to controller for diagnosis.

---

## Task 5: `processing.rs` pure module (TDD)

**Files:**
- Create: `src/processing.rs`
- Create: `tests/processing.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Register the module**

Add `pub mod processing;` to `src/lib.rs` in alphabetical order.

- [ ] **Step 2: Write failing tests in `tests/processing.rs`**

```rust
use miningsim::items::OreKind;
use miningsim::processing::{self, SmelterState, SmeltTickEvent, SMELT_DURATION_S};

#[test]
fn default_state_is_idle() {
    let s = SmelterState::default();
    assert_eq!(s.recipe, None);
    assert_eq!(s.queue, 0);
    assert!(s.output.is_empty());
    assert!(!processing::is_busy(&s));
}

#[test]
fn start_smelting_sets_recipe_and_timer() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 5);
    assert_eq!(s.recipe, Some(OreKind::Copper));
    assert_eq!(s.queue, 5);
    assert_eq!(s.time_left, SMELT_DURATION_S);
    assert!(processing::is_busy(&s));
}

#[test]
fn start_smelting_with_zero_count_is_noop() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 0);
    assert_eq!(s.recipe, None);
    assert_eq!(s.queue, 0);
}

#[test]
fn start_smelting_while_busy_is_noop() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 3);
    processing::start_smelting(&mut s, OreKind::Silver, 7);
    // First recipe untouched; second ignored.
    assert_eq!(s.recipe, Some(OreKind::Copper));
    assert_eq!(s.queue, 3);
}

#[test]
fn tick_decrements_timer() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 1);
    let ev = processing::tick_smelter(&mut s, 0.5);
    assert_eq!(ev, SmeltTickEvent::None);
    assert_eq!(s.time_left, SMELT_DURATION_S - 0.5);
}

#[test]
fn full_tick_completes_one_item() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 2);
    let ev = processing::tick_smelter(&mut s, SMELT_DURATION_S);
    assert_eq!(ev, SmeltTickEvent::BarFinished(OreKind::Copper));
    assert_eq!(s.queue, 1);
    assert_eq!(*s.output.get(&OreKind::Copper).unwrap_or(&0), 1);
    assert_eq!(s.recipe, Some(OreKind::Copper));   // queue not empty -> still smelting
    assert_eq!(s.time_left, SMELT_DURATION_S);     // reset for next item
}

#[test]
fn last_item_in_queue_returns_to_idle() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Silver, 1);
    let _ = processing::tick_smelter(&mut s, SMELT_DURATION_S);
    assert_eq!(s.recipe, None);
    assert_eq!(s.queue, 0);
    assert!(!processing::is_busy(&s));
    assert_eq!(*s.output.get(&OreKind::Silver).unwrap_or(&0), 1);
}

#[test]
fn tick_overshoot_completes_exactly_one_item() {
    // dt = 100s but SMELT_DURATION_S is 2.0 — should complete exactly ONE item, not 50.
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Copper, 5);
    let ev = processing::tick_smelter(&mut s, 100.0);
    assert_eq!(ev, SmeltTickEvent::BarFinished(OreKind::Copper));
    assert_eq!(s.queue, 4);
    assert_eq!(*s.output.get(&OreKind::Copper).unwrap_or(&0), 1);
    assert_eq!(s.time_left, SMELT_DURATION_S);     // reset, not negative
}

#[test]
fn tick_when_idle_is_noop() {
    let mut s = SmelterState::default();
    let ev = processing::tick_smelter(&mut s, 5.0);
    assert_eq!(ev, SmeltTickEvent::None);
    assert_eq!(s.recipe, None);
    assert!(s.output.is_empty());
}

#[test]
fn collect_output_drains_and_returns() {
    let mut s = SmelterState::default();
    processing::start_smelting(&mut s, OreKind::Gold, 3);
    for _ in 0..3 { let _ = processing::tick_smelter(&mut s, SMELT_DURATION_S); }
    assert_eq!(*s.output.get(&OreKind::Gold).unwrap_or(&0), 3);
    let drained = processing::collect_output(&mut s);
    assert_eq!(*drained.get(&OreKind::Gold).unwrap_or(&0), 3);
    assert!(s.output.is_empty());
}

#[test]
fn collect_output_on_empty_returns_empty() {
    let mut s = SmelterState::default();
    let d = processing::collect_output(&mut s);
    assert!(d.is_empty());
}
```

- [ ] **Step 3: Run tests to verify fail**

```bash
cargo test --test processing 2>&1 | tail -10
```

- [ ] **Step 4: Implement `src/processing.rs`**

```rust
use std::collections::HashMap;
use bevy::prelude::Component;
use crate::items::OreKind;

pub const SMELT_DURATION_S: f32 = 2.0;

#[derive(Component, Debug, Default)]
pub struct SmelterState {
    pub recipe: Option<OreKind>,
    pub time_left: f32,
    pub queue: u32,
    pub output: HashMap<OreKind, u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SmeltTickEvent {
    None,
    BarFinished(OreKind),
}

pub fn is_busy(state: &SmelterState) -> bool {
    state.recipe.is_some()
}

pub fn start_smelting(state: &mut SmelterState, ore: OreKind, count: u32) {
    if count == 0 || state.recipe.is_some() {
        return;
    }
    state.recipe = Some(ore);
    state.queue = count;
    state.time_left = SMELT_DURATION_S;
}

pub fn tick_smelter(state: &mut SmelterState, dt: f32) -> SmeltTickEvent {
    let Some(ore) = state.recipe else {
        return SmeltTickEvent::None;
    };
    state.time_left -= dt;
    if state.time_left > 0.0 {
        return SmeltTickEvent::None;
    }
    // Complete EXACTLY one item even if dt overshoots — predictable over realistic.
    *state.output.entry(ore).or_insert(0) += 1;
    state.queue -= 1;
    if state.queue > 0 {
        state.time_left = SMELT_DURATION_S;
    } else {
        state.recipe = None;
        state.time_left = 0.0;
    }
    SmeltTickEvent::BarFinished(ore)
}

pub fn collect_output(state: &mut SmelterState) -> HashMap<OreKind, u32> {
    std::mem::take(&mut state.output)
}
```

- [ ] **Step 5: Run tests to verify pass**

```bash
cargo test --test processing 2>&1 | tail -10
```
Expected: 11/11 passing.

- [ ] **Step 6: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: 65 + 11 = 76 tests passing.

- [ ] **Step 7: Commit**

```bash
git add src/processing.rs src/lib.rs tests/processing.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add processing module: SmelterState component + tick_smelter pure logic"
```

---

## Task 6: Smelter components, resource, entity spawn

**Files:**
- Modify: `src/components.rs`
- Modify: `src/systems/setup.rs`

- [ ] **Step 1: Add Smelter components and resource to `src/components.rs`**

Append:

```rust
use crate::items::OreKind;

#[derive(Component)]
pub struct Smelter;

#[derive(Component)]
pub struct SmelterUiRoot;

#[derive(Component)]
pub enum SmelterButtonKind {
    SmeltAll(OreKind),
    CollectAll,
}

#[derive(Component)]
pub struct SmelterStatusText;

#[derive(bevy::prelude::Resource, Default)]
pub struct SmelterUiOpen(pub bool);
```

- [ ] **Step 2: Spawn Smelter entity in `setup_world`**

In `src/systems/setup.rs`, add the Smelter spawn alongside the existing Shop spawn. Place it 3 tiles LEFT of the player spawn (mirroring Shop on the right):

```rust
use crate::components::{/* existing */, Smelter, SmelterUiOpen};
use crate::processing::SmelterState;

// In setup_world, after the Shop spawn block:
let smelter_tile = (sp.0 - 3, sp.1);
let smelter_world = coords::tile_center_world(IVec2::new(smelter_tile.0, smelter_tile.1));
commands.spawn((
    Smelter,
    SmelterState::default(),
    Sprite {
        color: Color::srgb(0.95, 0.50, 0.20),     // orange placeholder
        custom_size: Some(Vec2::splat(14.0)),
        ..default()
    },
    Transform::from_translation(smelter_world.extend(5.0)),
));

// Insert the SmelterUiOpen resource alongside the existing ShopUiOpen:
commands.insert_resource(SmelterUiOpen::default());
```

- [ ] **Step 3: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green; 76 tests still passing.

- [ ] **Step 4: Commit**

```bash
git add src/components.rs src/systems/setup.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Spawn Smelter entity with SmelterState; add Smelter UI markers and resource"
```

---

## Task 7: Smelter non-UI systems (interact + walk-away + tick)

**Files:**
- Create: `src/systems/smelter.rs`
- Modify: `src/systems/mod.rs`

- [ ] **Step 1: Add module declaration**

```rust
// src/systems/mod.rs
pub mod smelter;
```

- [ ] **Step 2: Create `src/systems/smelter.rs` with the three non-UI systems**

```rust
use bevy::prelude::*;
use crate::components::{Player, Smelter, SmelterUiOpen};
use crate::coords::TILE_SIZE_PX;
use crate::processing::{self, SmelterState};

pub const SMELTER_INTERACT_RADIUS_TILES: f32 = 2.0;

pub fn smelter_interact_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_open: ResMut<SmelterUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    smelter_q: Query<&Transform, (With<Smelter>, Without<Player>)>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        ui_open.0 = false;
        return;
    }
    if !keys.just_pressed(KeyCode::KeyE) { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(smelter) = smelter_q.get_single() else { return };
    let dist = player.translation.truncate().distance(smelter.translation.truncate());
    if dist / TILE_SIZE_PX <= SMELTER_INTERACT_RADIUS_TILES {
        ui_open.0 = !ui_open.0;
    }
}

pub fn close_smelter_on_walk_away_system(
    mut ui_open: ResMut<SmelterUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    smelter_q: Query<&Transform, (With<Smelter>, Without<Player>)>,
) {
    if !ui_open.0 { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(smelter) = smelter_q.get_single() else { return };
    let dist = player.translation.truncate().distance(smelter.translation.truncate());
    if dist / TILE_SIZE_PX > SMELTER_INTERACT_RADIUS_TILES {
        ui_open.0 = false;
    }
}

pub fn smelter_tick_system(
    time: Res<Time>,
    mut q: Query<&mut SmelterState>,
) {
    let dt = time.delta_secs();
    for mut state in q.iter_mut() {
        let _ = processing::tick_smelter(&mut state, dt);
        // Event return value is unused for M3; M4 (events bus) or M7 (audio) may consume it.
    }
}
```

**Note on potential collision with shop on `Esc`:** both `shop_interact_system` and `smelter_interact_system` close on `Esc`. Both run every frame; both set their UI to closed. No conflict — pressing `Esc` closes both panels.

**Note on potential collision with shop on `E`:** if the player happens to be near both the shop and the smelter simultaneously (impossible given the spawn-3-left-shop-3-right placement), `E` would toggle BOTH. Not a problem in practice; if it ever becomes one, gate by which entity is nearer.

- [ ] **Step 3: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green. Smelter systems exist but are not yet registered in `MiningSimPlugin` — that lands in Task 9.

- [ ] **Step 4: Commit**

```bash
git add src/systems/smelter.rs src/systems/mod.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Smelter: interact-on-E, walk-away-close, tick-driven processing"
```

---

## Task 8: Smelter UI panel (spawn + visibility + labels + buttons)

**Files:**
- Modify: `src/systems/smelter.rs`

- [ ] **Step 1: Append the UI systems to `src/systems/smelter.rs`**

```rust
use crate::components::{
    SmelterButtonKind, SmelterStatusText, SmelterUiRoot,
};
use crate::economy::SMELT_DURATION_S;     // re-import if needed
use crate::inventory::Inventory;
use crate::items::{ItemKind, OreKind, ALL_ORES};
use crate::processing::is_busy;
use crate::systems::hud::current_tool_display_name;   // for the Smelt button label

pub fn spawn_smelter_ui(mut commands: Commands) {
    commands
        .spawn((
            SmelterUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(30.0),
                top: Val::Percent(20.0),
                width: Val::Px(420.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.10, 0.12, 0.92)),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("SMELTER"),
                TextFont { font_size: 24.0, ..default() },
            ));
            // Status line — updated by update_smelter_panel_system
            panel.spawn((
                Text::new("IDLE"),
                TextFont { font_size: 18.0, ..default() },
                SmelterStatusText,
            ));
            // SmeltAll buttons — one per ore
            for ore in ALL_ORES {
                spawn_smelter_button(panel, &smelt_button_label(ore, 0), SmelterButtonKind::SmeltAll(ore));
            }
            // CollectAll
            spawn_smelter_button(panel, "Collect All", SmelterButtonKind::CollectAll);
        });
}

fn smelt_button_label(ore: OreKind, count: u32) -> String {
    format!("Smelt All {} ({})", ore_display(ore), count)
}

fn ore_display(o: OreKind) -> &'static str {
    match o {
        OreKind::Copper => "Copper",
        OreKind::Silver => "Silver",
        OreKind::Gold   => "Gold",
    }
}

fn spawn_smelter_button(parent: &mut ChildBuilder, label: &str, kind: SmelterButtonKind) {
    parent.spawn((
        kind,
        Button,
        Node {
            padding: UiRect::all(Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            width: Val::Px(280.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.22, 0.22, 0.28)),
        BorderColor(Color::srgb(0.35, 0.35, 0.42)),
    )).with_children(|b| {
        b.spawn((
            Text::new(label),
            TextFont { font_size: 18.0, ..default() },
        ));
    });
}

pub fn sync_smelter_visibility_system(
    ui_open: Res<SmelterUiOpen>,
    mut q: Query<&mut Visibility, With<SmelterUiRoot>>,
) {
    if !ui_open.is_changed() { return; }
    if let Ok(mut vis) = q.get_single_mut() {
        *vis = if ui_open.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

pub fn update_smelter_panel_system(
    inv: Res<Inventory>,
    state_q: Query<&SmelterState>,
    status_q: Query<Entity, With<SmelterStatusText>>,
    buttons_q: Query<(&SmelterButtonKind, &Children, Entity)>,
    mut bg_q: Query<&mut BackgroundColor>,
    mut texts_q: Query<&mut Text>,
) {
    let Ok(state) = state_q.get_single() else { return };
    // Always refresh — SmelterState may have changed (tick mutates time_left every frame)
    // and inventory may have changed; combined gating is messy and the work is cheap.

    // Status line
    if let Ok(status_entity) = status_q.get_single() {
        if let Ok(mut text) = texts_q.get_mut(status_entity) {
            **text = match state.recipe {
                None => "IDLE".to_string(),
                Some(ore) => format!(
                    "Smelting {} Bar ({:.1}s, queue: {})",
                    ore_display(ore), state.time_left.max(0.0), state.queue
                ),
            };
        }
    }

    // Buttons
    for (kind, children, entity) in buttons_q.iter() {
        let (label, enabled) = match kind {
            SmelterButtonKind::SmeltAll(ore) => {
                let count = inv.get(ItemKind::Ore(*ore));
                let label = smelt_button_label(*ore, count);
                let enabled = count > 0 && !is_busy(state);
                (label, enabled)
            }
            SmelterButtonKind::CollectAll => {
                let total: u32 = state.output.values().sum();
                let label = format!("Collect All ({})", total);
                let enabled = total > 0;
                (label, enabled)
            }
        };
        // Update child Text label
        for c in children.iter() {
            if let Ok(mut text) = texts_q.get_mut(*c) {
                **text = label.clone();
            }
        }
        // Update background per enabled state
        let new_bg = if enabled {
            Color::srgb(0.22, 0.22, 0.28)
        } else {
            Color::srgb(0.16, 0.16, 0.18)
        };
        if let Ok(mut bg) = bg_q.get_mut(entity) {
            *bg = BackgroundColor(new_bg);
        }
    }
}

pub fn handle_smelter_buttons_system(
    ui_open: Res<SmelterUiOpen>,
    interaction_q: Query<(&Interaction, &SmelterButtonKind), Changed<Interaction>>,
    mut inv: ResMut<Inventory>,
    mut state_q: Query<&mut SmelterState>,
) {
    if !ui_open.0 { return; }
    let Ok(mut state) = state_q.get_single_mut() else { return };
    for (interaction, kind) in interaction_q.iter() {
        if *interaction != Interaction::Pressed { continue; }
        match kind {
            SmelterButtonKind::SmeltAll(ore) => {
                let count = inv.get(ItemKind::Ore(*ore));
                if count == 0 || processing::is_busy(&state) { continue; }
                inv.remove(ItemKind::Ore(*ore), count);
                processing::start_smelting(&mut state, *ore, count);
            }
            SmelterButtonKind::CollectAll => {
                let drained = processing::collect_output(&mut state);
                for (ore, n) in drained {
                    inv.add(ItemKind::Bar(ore), n);
                }
            }
        }
    }
}
```

(Watch for the `SMELT_DURATION_S` import line — I wrote `crate::economy::` but it's `crate::processing::SMELT_DURATION_S`. Fix during implementation; the value isn't actually needed in this file since labels show `time_left`, not the constant.)

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: green. Common Bevy 0.15 patch-version adaptations from M2 may apply (`children![]` → `.with_children`, `ChildBuilder` → `ChildSpawnerCommands`).

- [ ] **Step 3: Commit**

```bash
git add src/systems/smelter.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Smelter UI: spawn panel, sync visibility, refresh labels, handle buttons"
```

---

## Task 9: App wiring + named SystemSets

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Define SystemSets and rewrite `MiningSimPlugin`**

```rust
use bevy::prelude::*;
use crate::systems::{camera, chunk_lifecycle, chunk_render, hud, ore_drop, player, setup, shop, shop_ui, smelter};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InputSet { ReadInput, ApplyInput }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorldSet { Collide, ChunkLifecycle, ChunkRender, Drops }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MachineSet { ShopProximity, ShopUi, SmelterProximity, SmelterTick, SmelterUi }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UiSet { Hud, Camera }

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
                setup::setup_world,
                hud::setup_hud,
                shop_ui::spawn_shop_ui,
                smelter::spawn_smelter_ui,
            ).chain())
            // Configure ordering between sets
            // Order matches M2's chained-tuple invariant:
            //   input → collide → machine interactions/UI → drops → chunks → hud → camera.
            //   Drops fires BEFORE chunk lifecycle/render so a tile broken this frame
            //   has its drop already in inventory before the HUD reads it.
            .configure_sets(Update, (
                InputSet::ReadInput,
                InputSet::ApplyInput,
                WorldSet::Collide,
                MachineSet::ShopProximity,
                MachineSet::SmelterProximity,
                MachineSet::SmelterTick,
                MachineSet::ShopUi,
                MachineSet::SmelterUi,
                WorldSet::Drops,
                WorldSet::ChunkLifecycle,
                WorldSet::ChunkRender,
                UiSet::Hud,
                UiSet::Camera,
            ).chain())
            .add_systems(Update, (
                player::read_input_system.in_set(InputSet::ReadInput),
                player::apply_velocity_system.in_set(InputSet::ApplyInput),
                player::dig_input_system.in_set(InputSet::ApplyInput),
                player::collide_player_with_grid_system.in_set(WorldSet::Collide),
                shop::shop_interact_system.in_set(MachineSet::ShopProximity),
                shop::close_shop_on_walk_away_system.in_set(MachineSet::ShopProximity),
                smelter::smelter_interact_system.in_set(MachineSet::SmelterProximity),
                smelter::close_smelter_on_walk_away_system.in_set(MachineSet::SmelterProximity),
                smelter::smelter_tick_system.in_set(MachineSet::SmelterTick),
                shop_ui::sync_shop_visibility_system.in_set(MachineSet::ShopUi),
                shop_ui::update_shop_labels_system.in_set(MachineSet::ShopUi),
                shop_ui::handle_shop_buttons_system.in_set(MachineSet::ShopUi),
                smelter::sync_smelter_visibility_system.in_set(MachineSet::SmelterUi),
                smelter::update_smelter_panel_system.in_set(MachineSet::SmelterUi),
                smelter::handle_smelter_buttons_system.in_set(MachineSet::SmelterUi),
                ore_drop::ore_drop_system.in_set(WorldSet::Drops),
                chunk_lifecycle::chunk_lifecycle_system.in_set(WorldSet::ChunkLifecycle),
                chunk_render::chunk_remesh_system.in_set(WorldSet::ChunkRender),
                hud::update_hud_system.in_set(UiSet::Hud),
                camera::camera_follow_system.in_set(UiSet::Camera),
            ));
    }
}
```

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green; 76 tests still passing.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "App: organize systems into named SystemSets; register Smelter pipeline"
```

---

## Smoke-test checkpoint #2 (after Task 9)

Human runs `cargo run`. Expected:
- Orange Smelter visible 3 tiles **left** of spawn; yellow Shop still 3 tiles right.
- Walk to Smelter, press `E` → smelter panel opens (dark semi-transparent, similar to shop).
- Status reads `IDLE`. Three Smelt All buttons (Copper / Silver / Gold). Collect All button.
- Buttons disabled (visibly dimmed) when player has 0 of that ore or output is empty.
- Mine some copper. Open smelter. `Smelt All Copper (5)` enabled.
- Click `Smelt All Copper` → inventory copper drops to 0; status changes to `Smelting Copper Bar (2.0s, queue: 5)`. All Smelt buttons disabled.
- Wait. Status countdown ticks; queue decrements as bars finish.
- After ~10s, queue empty, status returns to `IDLE`. Collect All becomes enabled with `(5)` count.
- Click `Collect All`. HUD copper-bar row shows 5; smelter output zeros.
- Walk to Shop. Sell All converts both ore + bar inventory to coins (5 copper bars × 5c = 25c).
- Buy Pickaxe — visibly affordable / unaffordable buttons are correct.
- Walk-away during smelt: machine keeps cooking. Come back, output is full.
- All M2 behaviors (cardinal dig, LoS, tier-gate, damage overlay, spacebar facing, MTV collision) still work.

If something's visibly wrong, surface to controller for diagnosis.

---

## Task 10: Final playtest, roadmap update, merge to main

- [ ] **Step 1: Run full test suite**

```bash
cargo test
```
Expected target: ~76 tests passing across all suites (M2's 54 + ~22 added in M3).

- [ ] **Step 2: Manual exit-criteria walkthrough**

Run `cargo run`. Walk through the spec's manual playtest checklist:
- [ ] Smelter visible on surface; panel opens with `E`, closes on `E`/`Esc`/walk-away.
- [ ] Smelt All Copper drains inventory copper, status shows queue + countdown.
- [ ] Buttons disabled while busy.
- [ ] Status returns to IDLE on queue empty; Collect All enables.
- [ ] Collect All transfers bars to inventory; HUD bar row updates.
- [ ] Walk-away mid-process keeps cooking.
- [ ] Shop Sell All sells both ores AND bars; coin count jumps.
- [ ] Bar revenue >> raw-ore revenue (5× math holds).
- [ ] Shop Buy buttons visibly dim when unaffordable / already owned.
- [ ] Full loop: dig copper → smelt → collect → sell bars → buy Pickaxe → dig stone → ... → buy Dynamite → break Core to bedrock floor.
- [ ] All M2 behaviors still work (cardinal dig, LoS, tier-gate, damage overlay, spacebar facing, MTV collision).
- [ ] No crashes over a 20-minute session.
- [ ] Processing *feels* like a real machine — leaving it cooking and coming back is satisfying.

- [ ] **Step 3: Append Playtest Results section to `docs/roadmap.md`**

Add under existing M1/M2 sections:

```markdown
## Playtest Results — Milestone 3 (YYYY-MM-DD)

Exit-criteria met: [summary]

**What felt good:**
- ...

**What felt off:**
- ...

**Decisions for milestone 3.5 / 4:**
- ...
```

Fill with actual observations.

- [ ] **Step 4: Commit playtest notes**

```bash
git add docs/roadmap.md
git commit --author="wes2000 <whannasch@gmail.com>" -m "Record milestone 3 playtest results"
```

- [ ] **Step 5: Merge to main + push**

```bash
git checkout main
git merge --no-ff milestone-3 -m "Merge milestone-3: surface base + first processing loop"
git push origin main
git branch -d milestone-3
```

- [ ] **Step 6: Final code review (optional but recommended)**

Dispatch the `superpowers:code-reviewer` subagent against the merged `main` HEAD. Capture any "fix before next milestone" callouts before starting the next brainstorm.

Milestone 3 complete.
