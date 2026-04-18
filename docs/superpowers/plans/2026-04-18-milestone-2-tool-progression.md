# Milestone 2 — Tool Progression + Tiered Ores Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [../specs/2026-04-18-milestone-2-tool-progression-design.md](../specs/2026-04-18-milestone-2-tool-progression-design.md)

**Goal:** Turn the M1 single-click-breaks-anything prototype into the full tool-progression arc: four tools × four tile tiers, with tier-gate + graduated damage; a surface shop that sells ore and buys tools; HUD money + current-tool indicator.

**Architecture:** Extend existing pure modules (`grid`, `dig`, `terrain_gen`) with the minimum new data (tile damage, Core layer). Put new concepts (`tools`, `economy`) in dedicated pure modules. Add shop + shop-UI as two new Bevy systems. Per-tile damage persists in the Grid and drives a semi-transparent overlay. Auto-tool-select each strike — no manual tool switching UI.

**Tech Stack:** Rust (stable), Bevy 0.15.x (pinned), `rand` (existing), Bevy 2D UI for the shop panel, built-in `cargo test` for unit tests.

---

## Pre-flight: environment expectations

This plan assumes:
- Rust stable toolchain (≥ 1.82 for Bevy 0.15). `rustup default stable` if in doubt.
- Working directory: `c:/Users/whann/Desktop/Games/miningsim` (existing git repo, branch `milestone-2` already created from `main`).
- `cargo test` currently passes 22/22 at `main`'s `3963e42`.
- Author identity: commits use `--author="wes2000 <whannasch@gmail.com>"`. Do not modify global git config.

If any of these aren't true, stop and resolve before proceeding.

---

## File structure (target end state)

```
src/
  grid.rs                       # MODIFY: Tile.damage: u8; Layer::Core added
  terrain_gen.rs                # MODIFY: deepest band → Core; outer ring stays Bedrock
  dig.rs                        # MODIFY: DigStatus variants; tool-aware try_dig; new dig_target_valid helper
  inventory.rs                  # unchanged
  tools.rs                      # NEW: Tool enum, OwnedTools, clicks_required, best_applicable_tool
  economy.rs                    # NEW: Money, prices, sell_all, try_buy
  components.rs                 # MODIFY: Shop, ShopUiRoot, ShopButtonKind, MoneyText, CurrentToolText markers; ShopUiOpen resource
  systems/
    setup.rs                    # MODIFY: init resources; spawn Shop entity
    player.rs                   # MODIFY: dig_input_system uses new helpers; cooldown reset rule
    chunk_render.rs             # MODIFY: spawn damage-overlay child sprite when damage > 0
    camera.rs, chunk_lifecycle.rs, ore_drop.rs     # unchanged
    hud.rs                      # MODIFY: ore_visual_color helper extracted + reused; Money row + CurrentTool row
    shop.rs                     # NEW: shop_interact_system, close_shop_on_walk_away_system
    shop_ui.rs                  # NEW: spawn_shop_ui, sync_shop_visibility_system, update_shop_labels_system, handle_shop_buttons_system
  app.rs                        # MODIFY: register new resources + systems
  lib.rs                        # MODIFY: pub mod tools; pub mod economy;
  main.rs                       # unchanged
tests/
  grid.rs                       # MODIFY: damage round-trip; Core variant
  terrain_gen.rs                # MODIFY: flip deep-band to Core; assert outer ring is Bedrock
  inventory.rs                  # unchanged
  dig.rs                        # MODIFY: tool-aware try_dig; dig_target_valid coverage
  tools.rs                      # NEW
  economy.rs                    # NEW
```

---

## Conventions

- Commit style: present-tense imperative. `--author="wes2000 <whannasch@gmail.com>"` on every commit.
- Pure modules follow TDD strictly: failing test first → verify fail → implement minimum → verify pass → commit.
- Bevy systems are not unit-tested; verify with `cargo build` + `cargo test` (for regressions on pure modules); visual verification deferred to the user at documented smoke-test checkpoints.
- Commands assume Git Bash / bash shell on Windows. Use Unix-style paths and forward slashes.
- `cargo run` blocks on the Bevy window — **subagents must not run it**. Use `cargo build` + `cargo test` for verification; the human controller drives `cargo run` at smoke-test checkpoints.
- Bevy 0.15.x API drift: if the plan's snippets don't compile against the resolved patch version (e.g. `children![]` macro unavailable), adapt to the equivalent API (`.with_children(|p| ...)`) and document the change in the task report. Do not silently paper over errors.

---

## User smoke-test checkpoints

Three visual verification moments in this milestone:

1. **After Task 9** — tool-aware dig works for the starting Shovel: 3 clicks per dirt tile, damage overlay visibly darkens between strikes, stone/deep/core clunk with no damage. This confirms the entire headless pipeline (grid + dig + tools + chunk render).
2. **After Task 12** — full shop loop: walk to shop, press `E`, Sell All, Buy Pickaxe, close, watch mining difficulty drop from 3 clicks to 2 on dirt and enable 3 clicks on stone. HUD shows money + current tool.
3. **After Task 14 (final)** — exit-criteria playtest: progression from Shovel to Dynamite, breaking Core, confirming the "barely-scratch → tear-through" feel.

---

## Task 1: Grid — add `damage` field + `Layer::Core` variant (TDD)

**Files:**
- Modify: `src/grid.rs`
- Modify: `tests/grid.rs`

- [ ] **Step 1: Update tests to cover new `damage` field and `Layer::Core`**

Append to `tests/grid.rs` (keep all existing tests unchanged):

```rust
#[test]
fn new_tile_has_zero_damage() {
    let g = Grid::new(3, 3);
    assert_eq!(g.get(1, 1).unwrap().damage, 0);
}

#[test]
fn damage_round_trips_through_set() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: true, layer: Layer::Stone, ore: OreType::None, damage: 2 });
    assert_eq!(g.get(1, 1).unwrap().damage, 2);
}

#[test]
fn layer_core_variant_exists() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: true, layer: Layer::Core, ore: OreType::None, damage: 0 });
    assert_eq!(g.get(1, 1).unwrap().layer, Layer::Core);
}
```

Existing tests that construct `Tile` by fields (e.g. `set_and_get_round_trip`) need `damage: 0` appended. Update `set_and_get_round_trip` and `set_out_of_bounds_panics` to include `damage: 0`. Leave the default-tile test alone — `Default` should fill in `damage: 0`.

- [ ] **Step 2: Run tests to verify they fail (compile error on missing field / variant)**

```bash
cargo test --test grid 2>&1 | tail -20
```
Expected: compile error mentioning `damage` field or `Core` variant.

- [ ] **Step 3: Update `src/grid.rs`**

Add the variant to `Layer`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Dirt,
    Stone,
    Deep,
    Core,     // NEW — deepest diggable band (Dynamite-only)
    Bedrock,  // map boundary, never breakable
}
```

Add `damage` to `Tile`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub solid: bool,
    pub layer: Layer,
    pub ore: OreType,
    pub damage: u8,  // strikes accumulated; 0 on fresh / broken tile
}

impl Default for Tile {
    fn default() -> Self {
        Self { solid: true, layer: Layer::Dirt, ore: OreType::None, damage: 0 }
    }
}
```

No other changes to `grid.rs` — everything else (Grid struct, width/height/get/set/in_bounds) is field-agnostic.

- [ ] **Step 4: Run tests to verify all pass**

```bash
cargo test --test grid 2>&1 | tail -5
```
Expected: all tests pass (previous 6 + 3 new = 9).

- [ ] **Step 5: Verify no other compile breakage**

```bash
cargo build 2>&1 | tail -10
```
Expected: build succeeds. Other modules (dig, terrain_gen) don't mention `damage` yet; default value fills in.

If downstream modules fail to compile because of exhaustive `match` over `Layer`, that's expected — they'll be fixed in Tasks 2, 3, 5. **For this task, ONLY update `grid.rs` and `tests/grid.rs`.** If `cargo build` fails with errors outside those files, note it in your report and STOP — the plan sequencing is wrong.

As of M1 HEAD, `dig.rs` and `terrain_gen.rs` use `if tile.layer == Layer::Bedrock` and exhaustive matches in `ore_probs` — adding `Core` triggers only an exhaustive-match compile error in `ore_probs` (handled in Task 2) and a `cargo check` warning anywhere else. `cargo build` at the end of this task may warn but not error.

- [ ] **Step 6: Commit**

```bash
git add src/grid.rs tests/grid.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add Tile.damage and Layer::Core variant"
```

---

## Task 2: TerrainGen — write Core band + keep Bedrock boundary (TDD, behavior-flip)

**Files:**
- Modify: `src/terrain_gen.rs`
- Modify: `tests/terrain_gen.rs`

The M1 test `depth_layers_appear_in_order` currently asserts the deepest sample is `Layer::Bedrock`. Under M2, the deepest diggable band is `Core`; Bedrock is only the outer ring. This task flips that assertion and adds a new assertion for the boundary ring.

- [ ] **Step 1: Update tests**

In `tests/terrain_gen.rs`, modify `depth_layers_appear_in_order`:

```rust
#[test]
fn depth_layers_appear_in_order() {
    let g = terrain_gen::generate(40, 200, 1);
    assert_eq!(g.get(20, 10).unwrap().layer, Layer::Dirt);
    assert_eq!(g.get(20, 80).unwrap().layer, Layer::Stone);
    assert_eq!(g.get(20, 140).unwrap().layer, Layer::Deep);
    assert_eq!(g.get(20, 180).unwrap().layer, Layer::Core);
}
```

Keep `outermost_ring_is_bedrock` untouched — it still passes because the boundary ring is still Bedrock.

Add a new test that interior tiles are never Bedrock (any deepest-band tile in the interior must be `Core`):

```rust
#[test]
fn interior_has_no_bedrock() {
    let g = terrain_gen::generate(40, 200, 1);
    for y in 1..(g.height() as i32 - 1) {
        for x in 1..(g.width() as i32 - 1) {
            assert_ne!(
                g.get(x, y).unwrap().layer,
                Layer::Bedrock,
                "interior tile ({},{}) should not be Bedrock", x, y
            );
        }
    }
}
```

Also update `ore_distribution_in_tolerance` if it scans the deep band by layer — the test as M1-written just counts ore, not layer, so it should still pass. Re-run to confirm. (If your grep shows no layer dependency, leave it alone.)

- [ ] **Step 2: Run tests to verify the flip fails**

```bash
cargo test --test terrain_gen 2>&1 | tail -15
```
Expected: `depth_layers_appear_in_order` fails (expects Core, still gets Bedrock); `interior_has_no_bedrock` fails (current gen writes Bedrock in the deepest band).

- [ ] **Step 3: Update `src/terrain_gen.rs`**

In `generate`, change the `else` branch of the interior-layer match so the deepest band writes `Layer::Core`, not `Layer::Bedrock`:

Current (M1):
```rust
} else {
    tile.layer = Layer::Bedrock;
}
```

Replace with:
```rust
} else {
    tile.layer = Layer::Core;
    maybe_assign_ore(&mut tile, &mut rng);
}
```

Wait — Core should NOT produce ore in M2 (per spec: "the reward for breaking through Core is 'you've reached the bottom of the map'"). Correct version:

```rust
} else {
    tile.layer = Layer::Core;
}
```

Also update `ore_probs` to handle `Layer::Core`. Bedrock currently returns `[(OreType::None, 0.0); 3]`; add the same for Core so exhaustive matching compiles:

```rust
fn ore_probs(layer: Layer) -> [(OreType, f32); 3] {
    match layer {
        Layer::Dirt  => [(OreType::Copper, 0.04),  (OreType::Silver, 0.005), (OreType::Gold, 0.0)],
        Layer::Stone => [(OreType::Copper, 0.02),  (OreType::Silver, 0.025), (OreType::Gold, 0.003)],
        Layer::Deep  => [(OreType::Copper, 0.005), (OreType::Silver, 0.015), (OreType::Gold, 0.02)],
        Layer::Core  => [(OreType::None, 0.0); 3],
        Layer::Bedrock => [(OreType::None, 0.0); 3],
    }
}
```

The boundary ring check (`if x == 0 || y == 0 || x == width-1 || y == height-1 { tile.layer = Layer::Bedrock; }`) is unchanged — that's exactly what we want.

- [ ] **Step 4: Run tests to verify pass**

```bash
cargo test --test terrain_gen 2>&1 | tail -10
```
Expected: all terrain_gen tests pass.

- [ ] **Step 5: Full test suite regression check**

```bash
cargo test 2>&1 | grep "test result" | tail -10
```
Expected: at least 22 tests still pass across suites.

- [ ] **Step 6: Commit**

```bash
git add src/terrain_gen.rs tests/terrain_gen.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "TerrainGen: deepest band is Core; interior has no Bedrock"
```

---

## Task 3: `tools.rs` pure module (TDD, new)

**Files:**
- Create: `src/tools.rs`
- Modify: `src/lib.rs`
- Create: `tests/tools.rs`

- [ ] **Step 1: Register the module**

Edit `src/lib.rs`, add `pub mod tools;` in alphabetical order:
```rust
pub mod inventory;
pub mod systems;
pub mod terrain_gen;
pub mod tools;
```

(Actually M1's order is `app, components, dig, grid, inventory, systems, terrain_gen`. Insert `tools` after `terrain_gen` so `systems` stays last.)

- [ ] **Step 2: Write failing tests**

Create `tests/tools.rs`:

```rust
use std::collections::HashSet;
use miningsim::grid::Layer;
use miningsim::tools::{self, Tool, OwnedTools};

#[test]
fn tool_tiers_are_1_through_4() {
    assert_eq!(tools::tool_tier(Tool::Shovel), 1);
    assert_eq!(tools::tool_tier(Tool::Pickaxe), 2);
    assert_eq!(tools::tool_tier(Tool::Jackhammer), 3);
    assert_eq!(tools::tool_tier(Tool::Dynamite), 4);
}

#[test]
fn layer_tier_assigns_diggable_tiers_and_bedrock_is_none() {
    assert_eq!(tools::layer_tier(Layer::Dirt), Some(1));
    assert_eq!(tools::layer_tier(Layer::Stone), Some(2));
    assert_eq!(tools::layer_tier(Layer::Deep), Some(3));
    assert_eq!(tools::layer_tier(Layer::Core), Some(4));
    assert_eq!(tools::layer_tier(Layer::Bedrock), None);
}

#[test]
fn clicks_required_at_tier_is_three() {
    assert_eq!(tools::clicks_required(Tool::Shovel, Layer::Dirt), Some(3));
    assert_eq!(tools::clicks_required(Tool::Pickaxe, Layer::Stone), Some(3));
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Deep), Some(3));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Core), Some(3));
}

#[test]
fn clicks_required_one_above_tier_is_two() {
    assert_eq!(tools::clicks_required(Tool::Pickaxe, Layer::Dirt), Some(2));
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Stone), Some(2));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Deep), Some(2));
}

#[test]
fn clicks_required_two_or_more_above_tier_is_one() {
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Dirt), Some(1));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Stone), Some(1));
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Dirt), Some(1));
}

#[test]
fn clicks_required_under_tier_is_none() {
    assert_eq!(tools::clicks_required(Tool::Shovel, Layer::Stone), None);
    assert_eq!(tools::clicks_required(Tool::Pickaxe, Layer::Deep), None);
    assert_eq!(tools::clicks_required(Tool::Jackhammer, Layer::Core), None);
}

#[test]
fn clicks_required_bedrock_is_always_none() {
    assert_eq!(tools::clicks_required(Tool::Dynamite, Layer::Bedrock), None);
    assert_eq!(tools::clicks_required(Tool::Shovel, Layer::Bedrock), None);
}

#[test]
fn default_owned_tools_has_only_shovel() {
    let owned = OwnedTools::default();
    assert!(owned.0.contains(&Tool::Shovel));
    assert_eq!(owned.0.len(), 1);
}

#[test]
fn best_applicable_tool_picks_strongest() {
    let owned = OwnedTools(HashSet::from([Tool::Shovel, Tool::Pickaxe, Tool::Jackhammer]));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Dirt), Some(Tool::Jackhammer));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Stone), Some(Tool::Jackhammer));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Deep), Some(Tool::Jackhammer));
}

#[test]
fn best_applicable_tool_returns_none_when_no_owned_tool_can_break() {
    let owned = OwnedTools(HashSet::from([Tool::Shovel]));
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Stone), None);
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Core), None);
    assert_eq!(tools::best_applicable_tool(&owned, Layer::Bedrock), None);
}
```

- [ ] **Step 3: Run tests to verify fail (compile error)**

```bash
cargo test --test tools 2>&1 | tail -10
```
Expected: compile error — `tools` module doesn't exist yet.

- [ ] **Step 4: Implement `src/tools.rs`**

```rust
use std::collections::HashSet;
use bevy::prelude::Resource;

use crate::grid::Layer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    Shovel,
    Pickaxe,
    Jackhammer,
    Dynamite,
}

pub fn tool_tier(t: Tool) -> u8 {
    match t {
        Tool::Shovel => 1,
        Tool::Pickaxe => 2,
        Tool::Jackhammer => 3,
        Tool::Dynamite => 4,
    }
}

pub fn layer_tier(l: Layer) -> Option<u8> {
    match l {
        Layer::Dirt => Some(1),
        Layer::Stone => Some(2),
        Layer::Deep => Some(3),
        Layer::Core => Some(4),
        Layer::Bedrock => None,
    }
}

pub fn clicks_required(tool: Tool, layer: Layer) -> Option<u8> {
    let lt = layer_tier(layer)?;
    let tt = tool_tier(tool);
    if tt < lt { return None; }
    let gap = (tt - lt).min(2);
    Some(3 - gap)
}

#[derive(Debug, Resource)]
pub struct OwnedTools(pub HashSet<Tool>);

impl Default for OwnedTools {
    fn default() -> Self {
        let mut s = HashSet::new();
        s.insert(Tool::Shovel);
        Self(s)
    }
}

pub fn best_applicable_tool(owned: &OwnedTools, layer: Layer) -> Option<Tool> {
    [Tool::Dynamite, Tool::Jackhammer, Tool::Pickaxe, Tool::Shovel]
        .into_iter()
        .find(|t| owned.0.contains(t) && clicks_required(*t, layer).is_some())
}
```

- [ ] **Step 5: Run tests to verify pass**

```bash
cargo test --test tools 2>&1 | tail -10
```
Expected: 10/10 passing.

- [ ] **Step 6: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: all suites pass.

- [ ] **Step 7: Commit**

```bash
git add src/tools.rs src/lib.rs tests/tools.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add tools module: Tool enum, OwnedTools, clicks_required, best_applicable_tool"
```

---

## Task 4: `economy.rs` pure module (TDD, new)

**Files:**
- Create: `src/economy.rs`
- Modify: `src/lib.rs`
- Create: `tests/economy.rs`

- [ ] **Step 1: Register the module**

Edit `src/lib.rs`, insert `pub mod economy;` in alphabetical order:
```rust
pub mod dig;
pub mod economy;
pub mod grid;
```

- [ ] **Step 2: Write failing tests**

Create `tests/economy.rs`:

```rust
use miningsim::economy::{self, BuyResult, Money};
use miningsim::grid::OreType;
use miningsim::inventory::Inventory;
use miningsim::tools::{Tool, OwnedTools};

#[test]
fn ore_sell_prices_match_spec() {
    assert_eq!(economy::ore_sell_price(OreType::None), 0);
    assert_eq!(economy::ore_sell_price(OreType::Copper), 1);
    assert_eq!(economy::ore_sell_price(OreType::Silver), 5);
    assert_eq!(economy::ore_sell_price(OreType::Gold), 20);
}

#[test]
fn tool_buy_prices_match_spec() {
    assert_eq!(economy::tool_buy_price(Tool::Shovel), 0);
    assert_eq!(economy::tool_buy_price(Tool::Pickaxe), 30);
    assert_eq!(economy::tool_buy_price(Tool::Jackhammer), 100);
    assert_eq!(economy::tool_buy_price(Tool::Dynamite), 300);
}

#[test]
fn sell_all_converts_mixed_inventory_and_zeros_counts() {
    let mut inv = Inventory::default();
    inv.add(OreType::Copper, 5);    //  5 * 1 =  5
    inv.add(OreType::Silver, 3);    //  3 * 5 = 15
    inv.add(OreType::Gold, 2);      //  2 * 20 = 40
    let mut money = Money::default();
    economy::sell_all(&mut inv, &mut money);
    assert_eq!(money.0, 60);
    assert_eq!(inv.get(OreType::Copper), 0);
    assert_eq!(inv.get(OreType::Silver), 0);
    assert_eq!(inv.get(OreType::Gold), 0);
}

#[test]
fn sell_all_empty_inventory_is_no_op() {
    let mut inv = Inventory::default();
    let mut money = Money(10);
    economy::sell_all(&mut inv, &mut money);
    assert_eq!(money.0, 10);
}

#[test]
fn try_buy_succeeds_when_affordable() {
    let mut money = Money(50);
    let mut owned = OwnedTools::default();
    let r = economy::try_buy(Tool::Pickaxe, &mut money, &mut owned);
    assert_eq!(r, BuyResult::Ok);
    assert_eq!(money.0, 20);
    assert!(owned.0.contains(&Tool::Pickaxe));
}

#[test]
fn try_buy_returns_not_enough_money_when_poor() {
    let mut money = Money(10);
    let mut owned = OwnedTools::default();
    let r = economy::try_buy(Tool::Pickaxe, &mut money, &mut owned);
    assert_eq!(r, BuyResult::NotEnoughMoney);
    assert_eq!(money.0, 10);
    assert!(!owned.0.contains(&Tool::Pickaxe));
}

#[test]
fn try_buy_returns_already_owned_on_repeat_purchase() {
    let mut money = Money(100);
    let mut owned = OwnedTools::default();   // already has Shovel
    let r = economy::try_buy(Tool::Shovel, &mut money, &mut owned);
    assert_eq!(r, BuyResult::AlreadyOwned);
    assert_eq!(money.0, 100);
}

#[test]
fn try_buy_exact_cost_succeeds_and_zeros_money() {
    let mut money = Money(30);
    let mut owned = OwnedTools::default();
    let r = economy::try_buy(Tool::Pickaxe, &mut money, &mut owned);
    assert_eq!(r, BuyResult::Ok);
    assert_eq!(money.0, 0);
}
```

- [ ] **Step 3: Run tests to verify fail**

```bash
cargo test --test economy 2>&1 | tail -10
```
Expected: compile error.

- [ ] **Step 4: Implement `src/economy.rs`**

```rust
use bevy::prelude::Resource;

use crate::grid::OreType;
use crate::inventory::Inventory;
use crate::tools::{OwnedTools, Tool};

#[derive(Debug, Default, Resource)]
pub struct Money(pub u32);

pub fn ore_sell_price(ore: OreType) -> u32 {
    match ore {
        OreType::None => 0,
        OreType::Copper => 1,
        OreType::Silver => 5,
        OreType::Gold => 20,
    }
}

pub fn tool_buy_price(tool: Tool) -> u32 {
    match tool {
        Tool::Shovel => 0,
        Tool::Pickaxe => 30,
        Tool::Jackhammer => 100,
        Tool::Dynamite => 300,
    }
}

pub fn sell_all(inv: &mut Inventory, money: &mut Money) {
    for ore in [OreType::Copper, OreType::Silver, OreType::Gold] {
        let count = inv.get(ore);
        if count == 0 { continue; }
        money.0 += ore_sell_price(ore) * count;
        inv.remove(ore, count);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuyResult {
    Ok,
    AlreadyOwned,
    NotEnoughMoney,
}

pub fn try_buy(tool: Tool, money: &mut Money, owned: &mut OwnedTools) -> BuyResult {
    if owned.0.contains(&tool) {
        return BuyResult::AlreadyOwned;
    }
    let price = tool_buy_price(tool);
    if money.0 < price {
        return BuyResult::NotEnoughMoney;
    }
    money.0 -= price;
    owned.0.insert(tool);
    BuyResult::Ok
}
```

- [ ] **Step 5: Run tests to verify pass**

```bash
cargo test --test economy 2>&1 | tail -10
```
Expected: 8/8 passing.

- [ ] **Step 6: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: all suites pass.

- [ ] **Step 7: Commit**

```bash
git add src/economy.rs src/lib.rs tests/economy.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add economy module: Money, prices, sell_all, try_buy"
```

---

## Task 5: Dig module — tool-aware `try_dig` + `dig_target_valid` helper (TDD, rewrite)

**Files:**
- Modify: `src/dig.rs`
- Modify: `tests/dig.rs`

This task is the biggest single-module rewrite. We replace `try_dig(grid, x, y)` with `try_dig(grid, target: IVec2, tool: Tool)` and extract cardinal+LoS into the pure helper `dig_target_valid`. `DigStatus` grows new variants.

- [ ] **Step 1: Update tests in `tests/dig.rs`**

Replace the existing `tests/dig.rs` entirely:

```rust
use bevy::prelude::IVec2;
use miningsim::dig::{self, DigStatus};
use miningsim::grid::{Grid, Layer, OreType, Tile};
use miningsim::tools::Tool;

fn make_test_grid() -> Grid {
    let mut g = Grid::new(10, 10);
    // Fill interior with solid Dirt tiles (Grid default).
    // Override specific tiles:
    g.set(3, 3, Tile { solid: true, layer: Layer::Dirt,  ore: OreType::Copper, damage: 0 });
    g.set(4, 3, Tile { solid: true, layer: Layer::Stone, ore: OreType::None,   damage: 0 });
    g.set(5, 3, Tile { solid: true, layer: Layer::Deep,  ore: OreType::None,   damage: 0 });
    g.set(6, 3, Tile { solid: true, layer: Layer::Core,  ore: OreType::None,   damage: 0 });
    g.set(0, 0, Tile { solid: true, layer: Layer::Bedrock, ore: OreType::None, damage: 0 });
    g
}

// --- try_dig ---

#[test]
fn shovel_on_dirt_at_tier_takes_three_strikes() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    let r1 = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r1.status, DigStatus::Damaged);
    assert_eq!(g.get(3, 3).unwrap().damage, 1);
    let r2 = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r2.status, DigStatus::Damaged);
    assert_eq!(g.get(3, 3).unwrap().damage, 2);
    let r3 = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r3.status, DigStatus::Broken);
    assert_eq!(r3.ore, OreType::Copper);
    assert!(!g.get(3, 3).unwrap().solid);
    assert_eq!(g.get(3, 3).unwrap().damage, 0);
}

#[test]
fn pickaxe_on_dirt_one_above_tier_takes_two_strikes() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Damaged);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Broken);
}

#[test]
fn jackhammer_on_dirt_two_above_tier_takes_one_strike() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Jackhammer).status, DigStatus::Broken);
}

#[test]
fn shovel_on_stone_under_tier_returns_under_tier_no_damage() {
    let mut g = make_test_grid();
    let t = IVec2::new(4, 3);
    let r = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r.status, DigStatus::UnderTier);
    assert_eq!(g.get(4, 3).unwrap().damage, 0);
    assert!(g.get(4, 3).unwrap().solid);
}

#[test]
fn any_tool_on_bedrock_returns_under_tier_never_damages() {
    let mut g = make_test_grid();
    let t = IVec2::new(0, 0);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Dynamite).status, DigStatus::UnderTier);
    assert_eq!(g.get(0, 0).unwrap().damage, 0);
    assert!(g.get(0, 0).unwrap().solid);
}

#[test]
fn dynamite_on_core_at_tier_takes_three_strikes() {
    let mut g = make_test_grid();
    let t = IVec2::new(6, 3);
    for _ in 0..2 {
        assert_eq!(dig::try_dig(&mut g, t, Tool::Dynamite).status, DigStatus::Damaged);
    }
    assert_eq!(dig::try_dig(&mut g, t, Tool::Dynamite).status, DigStatus::Broken);
}

#[test]
fn tool_upgrade_mid_mining_breaks_immediately_when_threshold_met() {
    let mut g = make_test_grid();
    // Damage stone tile to 2 with pickaxe (3-click tier-match).
    let t = IVec2::new(4, 3);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Damaged);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Damaged);
    assert_eq!(g.get(4, 3).unwrap().damage, 2);
    // Now switch to Jackhammer (1-click required on stone). Tile should break.
    assert_eq!(dig::try_dig(&mut g, t, Tool::Jackhammer).status, DigStatus::Broken);
}

#[test]
fn dig_out_of_bounds_is_noop() {
    let mut g = make_test_grid();
    let r = dig::try_dig(&mut g, IVec2::new(-1, 5), Tool::Shovel);
    assert_eq!(r.status, DigStatus::OutOfBounds);
}

#[test]
fn dig_already_empty_is_noop_does_not_increment_damage() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    // Break tile completely.
    dig::try_dig(&mut g, t, Tool::Jackhammer);
    assert!(!g.get(3, 3).unwrap().solid);
    let r = dig::try_dig(&mut g, t, Tool::Jackhammer);
    assert_eq!(r.status, DigStatus::AlreadyEmpty);
    assert_eq!(g.get(3, 3).unwrap().damage, 0);
}

// --- dig_target_valid ---

#[test]
fn dig_target_valid_accepts_cardinal_within_reach() {
    let g = Grid::new(10, 10);   // all solid dirt
    let p = IVec2::new(5, 5);
    // All four cardinal directions, distance 1, with intermediate LoS clear (no intermediate tile).
    assert!(dig::dig_target_valid(p, IVec2::new(6, 5), 2, &g));
    assert!(dig::dig_target_valid(p, IVec2::new(4, 5), 2, &g));
    assert!(dig::dig_target_valid(p, IVec2::new(5, 6), 2, &g));
    assert!(dig::dig_target_valid(p, IVec2::new(5, 4), 2, &g));
}

#[test]
fn dig_target_valid_rejects_diagonal() {
    let g = Grid::new(10, 10);
    let p = IVec2::new(5, 5);
    assert!(!dig::dig_target_valid(p, IVec2::new(6, 6), 2, &g));
    assert!(!dig::dig_target_valid(p, IVec2::new(4, 4), 2, &g));
}

#[test]
fn dig_target_valid_rejects_beyond_reach() {
    let g = Grid::new(10, 10);
    let p = IVec2::new(5, 5);
    assert!(!dig::dig_target_valid(p, IVec2::new(8, 5), 2, &g));
}

#[test]
fn dig_target_valid_rejects_same_tile() {
    let g = Grid::new(10, 10);
    let p = IVec2::new(5, 5);
    assert!(!dig::dig_target_valid(p, IVec2::new(5, 5), 2, &g));
}

#[test]
fn dig_target_valid_rejects_when_intermediate_tile_is_solid() {
    let g = Grid::new(10, 10);  // all solid by default
    let p = IVec2::new(5, 5);
    // target is 2 tiles away, intermediate (6,5) is solid → rejected.
    assert!(!dig::dig_target_valid(p, IVec2::new(7, 5), 2, &g));
}

#[test]
fn dig_target_valid_accepts_reach_2_when_intermediate_is_empty() {
    let mut g = Grid::new(10, 10);
    // clear the intermediate tile
    let t = Tile { solid: false, layer: Layer::Dirt, ore: OreType::None, damage: 0 };
    g.set(6, 5, t);
    let p = IVec2::new(5, 5);
    assert!(dig::dig_target_valid(p, IVec2::new(7, 5), 2, &g));
}
```

- [ ] **Step 2: Run tests to verify compile failure**

```bash
cargo test --test dig 2>&1 | tail -20
```
Expected: compile error — `DigStatus::Damaged`, `DigStatus::UnderTier`, `dig::dig_target_valid`, `tools::Tool` params don't match.

- [ ] **Step 3: Implement new `src/dig.rs`**

Replace `src/dig.rs` entirely:

```rust
use bevy::prelude::IVec2;

use crate::grid::{Grid, Layer, OreType, Tile};
use crate::tools::{self, Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigStatus {
    Broken,
    Damaged,
    OutOfBounds,
    AlreadyEmpty,
    Blocked,
    UnderTier,
}

#[derive(Debug, Clone, Copy)]
pub struct DigResult {
    pub status: DigStatus,
    pub ore: OreType,
}

pub fn try_dig(grid: &mut Grid, target: IVec2, tool: Tool) -> DigResult {
    let x = target.x;
    let y = target.y;

    let tile = match grid.get(x, y) {
        None => return DigResult { status: DigStatus::OutOfBounds, ore: OreType::None },
        Some(t) => *t,
    };
    if !tile.solid {
        return DigResult { status: DigStatus::AlreadyEmpty, ore: OreType::None };
    }
    let Some(required) = tools::clicks_required(tool, tile.layer) else {
        return DigResult { status: DigStatus::UnderTier, ore: OreType::None };
    };

    let new_damage = tile.damage + 1;
    if new_damage >= required {
        // Break tile.
        let ore = tile.ore;
        grid.set(x, y, Tile { solid: false, layer: tile.layer, ore: OreType::None, damage: 0 });
        DigResult { status: DigStatus::Broken, ore }
    } else {
        grid.set(x, y, Tile { damage: new_damage, ..tile });
        DigResult { status: DigStatus::Damaged, ore: OreType::None }
    }
}

/// Cardinal-only + line-of-sight dig reach check, extracted for unit testing.
///
/// Returns true iff:
/// - `target` differs from `player_tile`,
/// - exactly one axis of delta is zero (cardinal, not diagonal),
/// - |delta| ≤ reach on the nonzero axis,
/// - every tile STRICTLY BETWEEN `player_tile` and `target` is non-solid.
///
/// Does NOT check whether `target` itself is solid (callers may want to mine
/// either solid or empty tiles depending on context). Does NOT check bounds;
/// out-of-bounds intermediates are treated as "non-solid" so that reach checks
/// near map edges don't spuriously reject.
pub fn dig_target_valid(player_tile: IVec2, target: IVec2, reach: i32, grid: &Grid) -> bool {
    let delta = target - player_tile;
    if delta == IVec2::ZERO { return false; }
    let is_cardinal = (delta.x == 0) ^ (delta.y == 0);
    if !is_cardinal { return false; }
    let dist = delta.x.abs().max(delta.y.abs());
    if dist > reach { return false; }

    let step = IVec2::new(delta.x.signum(), delta.y.signum());
    let mut probe = player_tile + step;
    while probe != target {
        // If a tile in the path is solid, LoS is blocked.
        if let Some(t) = grid.get(probe.x, probe.y) {
            if t.solid { return false; }
        }
        probe += step;
    }
    true
}
```

Design note: `DigStatus::Blocked` is defined but not yet returned by `try_dig` itself. That's fine — the dig input system will consult `dig_target_valid` first and gate before calling `try_dig`. A future caller could choose to return `Blocked` for path-occupied cases, but in M2 the gate happens upstream.

- [ ] **Step 4: Run tests to verify pass**

```bash
cargo test --test dig 2>&1 | tail -15
```
Expected: 15/15 passing.

- [ ] **Step 5: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: all suites pass. Note: `player.rs` currently calls the M1 `try_dig(&mut Grid, i32, i32)` signature, which this task broke. That will fail to compile — which is expected, and Task 9 fixes it.

**If `cargo build` fails because of `player.rs`:** that's expected for Tasks 5-8. Before committing, apply a minimal temporary patch to `src/systems/player.rs` to keep the crate compiling:

In `dig_input_system`, replace the `try_dig` call with a compiling stub until Task 9 wires the real flow:

```rust
// TEMPORARY (restored in Task 9): old M1 single-click behavior
let result_status = {
    let tile = grid.get(tx, ty).copied();
    match tile {
        None => dig::DigStatus::OutOfBounds,
        Some(t) if !t.solid => dig::DigStatus::AlreadyEmpty,
        Some(t) if t.layer == crate::grid::Layer::Bedrock => dig::DigStatus::UnderTier,
        Some(_) => dig::DigStatus::Broken,
    }
};
```

Remove the `dig::try_dig(&mut grid, tx, ty)` line. This scaffolds the system until Task 9 restores proper flow.

Actually — cleaner: replace the stub logic above with one real call using Shovel, to keep gameplay functional at the `main` commit boundary:

```rust
let result = dig::try_dig(&mut grid, bevy::prelude::IVec2::new(tx, ty), crate::tools::Tool::Shovel);
if result.status != dig::DigStatus::Broken { return; }
```

This makes Task 5's commit runnable (shovel only; clicks-per-tile = 3 on dirt; other layers clunk via UnderTier).

- [ ] **Step 6: Commit**

```bash
git add src/dig.rs tests/dig.rs src/systems/player.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Dig: tool-aware try_dig with damage; extract dig_target_valid helper"
```

---

## Task 6: Components + resources + shop entity spawn (small)

**Files:**
- Modify: `src/components.rs`
- Modify: `src/systems/setup.rs`

- [ ] **Step 1: Add new components + resource**

Append to `src/components.rs`:

```rust
use crate::tools::Tool;

#[derive(Component)]
pub struct Shop;

#[derive(Component)]
pub struct ShopUiRoot;

#[derive(Component)]
pub enum ShopButtonKind {
    SellAll,
    Buy(Tool),
}

#[derive(Component)]
pub struct MoneyText;

#[derive(Component)]
pub struct CurrentToolText;

#[derive(bevy::prelude::Resource, Default)]
pub struct ShopUiOpen(pub bool);
```

- [ ] **Step 2: Register resources + spawn Shop entity in setup**

Edit `src/systems/setup.rs`. Add imports:
```rust
use crate::components::{Shop, ShopUiOpen, MainCamera, Player, Velocity};
use crate::economy::Money;
use crate::tools::OwnedTools;
```

In `setup_world`, insert new resources alongside the existing ones:
```rust
commands.insert_resource(Money::default());
commands.insert_resource(OwnedTools::default());
commands.insert_resource(ShopUiOpen::default());
```

Spawn the Shop entity on the surface strip, offset from the player spawn pocket:
```rust
let shop_tile = (sp.0 + 3, sp.1);   // 3 tiles right of player spawn
let shop_world = tile_center_world(shop_tile.0, shop_tile.1);
commands.spawn((
    Shop,
    Sprite {
        color: Color::srgb(0.95, 0.80, 0.20),   // yellow placeholder
        custom_size: Some(Vec2::splat(14.0)),
        ..default()
    },
    Transform::from_translation(shop_world.extend(5.0)),
));
```

- [ ] **Step 3: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: build succeeds; tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/components.rs src/systems/setup.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Spawn Shop entity and init Money/OwnedTools/ShopUiOpen resources"
```

---

## Task 7: Chunk render — damage overlay sprite

**Files:**
- Modify: `src/systems/chunk_render.rs`

- [ ] **Step 1: Add damage overlay in `chunk_remesh_system`**

After the ore-sprite spawn inside the per-tile loop, add:
```rust
if t.damage > 0 {
    parent.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, t.damage as f32 * 0.2),
            custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
            ..default()
        },
        Transform::from_translation(Vec3::new(world_x, world_y, 0.25)),
    ));
}
```

Z = 0.25 puts the overlay above the layer-color sprite (z=0.0) and below the ore dot (z=0.5). That way the ore is still visible through partial damage.

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/systems/chunk_render.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Chunk render: draw damage overlay sprite for partially-mined tiles"
```

---

## Task 8: HUD extensions — ore_visual_color helper + Money + CurrentTool rows

**Files:**
- Modify: `src/systems/hud.rs`

- [ ] **Step 1: Extract shared color helper and add two new rows**

Replace `src/systems/hud.rs` with the expanded version. Key changes:
- `fn ore_visual_color(OreType) -> Color` becomes `pub` and replaces the ad-hoc matches in `hud.rs`, `chunk_render.rs`, `ore_drop.rs`, and (future) `shop_ui.rs`.
- `setup_hud` adds a Money row and a Current Tool row after the three ore rows.
- `update_hud_system` now also reacts to `Changed<Money>` and `Changed<OwnedTools>`.

Full replacement:

```rust
use bevy::prelude::*;
use crate::components::{MoneyText, CurrentToolText};
use crate::economy::Money;
use crate::grid::OreType;
use crate::inventory::Inventory;
use crate::tools::{self, OwnedTools, Tool};

#[derive(Component)]
pub struct OreCountText(pub OreType);

pub fn ore_visual_color(o: OreType) -> Color {
    match o {
        OreType::None   => Color::WHITE,
        OreType::Copper => Color::srgb(0.85, 0.45, 0.20),
        OreType::Silver => Color::srgb(0.85, 0.85, 0.92),
        OreType::Gold   => Color::srgb(0.95, 0.78, 0.25),
    }
}

pub fn current_tool_display_name(t: Tool) -> &'static str {
    match t {
        Tool::Shovel     => "Shovel",
        Tool::Pickaxe    => "Pickaxe",
        Tool::Jackhammer => "Jackhammer",
        Tool::Dynamite   => "Dynamite",
    }
}

pub fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|root| {
            // Existing ore rows
            spawn_ore_row(root, OreType::Copper);
            spawn_ore_row(root, OreType::Silver);
            spawn_ore_row(root, OreType::Gold);
            // New: Money row
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
            )).with_children(|row| {
                row.spawn((
                    Node {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                        margin: UiRect::right(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.9, 0.3)),  // coin yellow
                ));
                row.spawn((
                    Text::new("0c"),
                    TextFont { font_size: 18.0, ..default() },
                    MoneyText,
                ));
            });
            // New: Current tool row
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
            )).with_children(|row| {
                row.spawn((
                    Node {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                        margin: UiRect::right(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.6, 0.6, 0.9)),  // tool slot bg
                ));
                row.spawn((
                    Text::new("Shovel"),
                    TextFont { font_size: 18.0, ..default() },
                    CurrentToolText,
                ));
            });
        });
}

fn spawn_ore_row(root: &mut ChildBuilder, ore: OreType) {
    root.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(4.0)),
            ..default()
        },
    )).with_children(|row| {
        row.spawn((
            Node {
                width: Val::Px(16.0),
                height: Val::Px(16.0),
                margin: UiRect::right(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(ore_visual_color(ore)),
        ));
        row.spawn((
            Text::new("0"),
            TextFont { font_size: 18.0, ..default() },
            OreCountText(ore),
        ));
    });
}

pub fn update_hud_system(
    inv: Res<Inventory>,
    money: Res<Money>,
    owned: Res<OwnedTools>,
    mut ore_q: Query<(&mut Text, &OreCountText), (Without<MoneyText>, Without<CurrentToolText>)>,
    mut money_q: Query<&mut Text, (With<MoneyText>, Without<OreCountText>, Without<CurrentToolText>)>,
    mut tool_q: Query<&mut Text, (With<CurrentToolText>, Without<OreCountText>, Without<MoneyText>)>,
) {
    if inv.is_changed() {
        for (mut text, marker) in ore_q.iter_mut() {
            **text = inv.get(marker.0).to_string();
        }
    }
    if money.is_changed() {
        if let Ok(mut text) = money_q.get_single_mut() {
            **text = format!("{}c", money.0);
        }
    }
    if owned.is_changed() {
        if let Ok(mut text) = tool_q.get_single_mut() {
            // Strongest owned tool name
            let strongest = [Tool::Dynamite, Tool::Jackhammer, Tool::Pickaxe, Tool::Shovel]
                .into_iter()
                .find(|t| owned.0.contains(t))
                .unwrap_or(Tool::Shovel);
            **text = current_tool_display_name(strongest).to_string();
        }
    }
}
```

Then update `chunk_render.rs` and `ore_drop.rs` to call `crate::systems::hud::ore_visual_color` instead of their local ORE_COLORS constants. Delete the duplicate constants.

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/systems/hud.rs src/systems/chunk_render.rs src/systems/ore_drop.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "HUD: extract ore_visual_color; add Money + CurrentTool rows"
```

---

## Task 9: Player dig system — wire dig_target_valid + best_applicable_tool + cooldown rule

**Files:**
- Modify: `src/systems/player.rs`

- [ ] **Step 1: Rewrite `dig_input_system`**

Replace the body of `dig_input_system` with the tool-aware flow. Remove the Task-5 temporary stub. Final shape:

```rust
pub fn dig_input_system(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<crate::components::MainCamera>>,
    player_q: Query<&Transform, With<Player>>,
    mut grid: ResMut<Grid>,
    mut cooldown: ResMut<DigCooldown>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
    owned_tools: Res<crate::tools::OwnedTools>,
    time: Res<Time>,
) {
    cooldown.0.tick(time.delta());
    let dig_held = mouse.pressed(MouseButton::Left) || keys.pressed(KeyCode::Space);
    if !dig_held { return; }
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

    let player_tile = IVec2::new(
        (player_xf.translation.x / TILE_SIZE_PX).floor() as i32,
        ((-player_xf.translation.y) / TILE_SIZE_PX).floor() as i32,
    );
    let target_tile = IVec2::new(tx, ty);
    let reach = DIG_REACH_TILES as i32;

    // Cardinal + reach + line-of-sight gate. No cooldown reset on rejection.
    if !dig::dig_target_valid(player_tile, target_tile, reach, &grid) { return; }

    // Look up tile layer to pick the best tool.
    let Some(tile) = grid.get(tx, ty).copied() else { return; };
    let Some(tool) = crate::tools::best_applicable_tool(&owned_tools, tile.layer) else {
        // Player owns nothing that can break this layer. Clunk; no cooldown reset.
        return;
    };

    let result = dig::try_dig(&mut grid, target_tile, tool);
    match result.status {
        DigStatus::Broken | DigStatus::Damaged => {
            cooldown.0.reset();
            // Mark owning chunk dirty.
            let chunk_coord = IVec2::new(tx.div_euclid(CHUNK_TILES), ty.div_euclid(CHUNK_TILES));
            for (e, c) in chunks_q.iter() {
                if c.coord == chunk_coord {
                    commands.entity(e).insert(ChunkDirty);
                    break;
                }
            }
            // Spawn ore drop only on full break.
            if result.status == DigStatus::Broken && result.ore != OreType::None {
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
        _ => { /* OutOfBounds / AlreadyEmpty / UnderTier / Blocked — no cooldown reset */ }
    }
}
```

Ensure imports cover `DigStatus` and other needed items:
```rust
use crate::dig::{self, DigStatus};
```

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/systems/player.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Player dig: tool-aware flow with dig_target_valid, best_applicable_tool, cooldown on damage-or-break only"
```

---

## Smoke-test checkpoint #1 (after Task 9)

At this point, the human controller should run `cargo run` and confirm:

- Shovel-on-dirt: 3 clicks per tile; damage overlay visibly darkens between strikes.
- Shovel-on-stone / deep / core / bedrock: no damage, clunk (or at least no visible change, no ore drop).
- Digging a tile: final strike removes the damage overlay and the tile; ore drops work as M1.
- Cardinal / LoS / reach gates work identically to M1.

If something is visibly broken, surface to controller for diagnosis.

---

## Task 10: Shop proximity + toggle systems

**Files:**
- Create: `src/systems/shop.rs`
- Modify: `src/systems/mod.rs`

- [ ] **Step 1: Create `src/systems/shop.rs`**

```rust
use bevy::prelude::*;
use crate::components::{Player, Shop, ShopUiOpen};
use crate::systems::setup::TILE_SIZE_PX;

pub const SHOP_INTERACT_RADIUS_TILES: f32 = 2.0;

pub fn shop_interact_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_open: ResMut<ShopUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    shop_q: Query<&Transform, (With<Shop>, Without<Player>)>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        ui_open.0 = false;
        return;
    }
    if !keys.just_pressed(KeyCode::KeyE) { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(shop) = shop_q.get_single() else { return };
    let dist = player.translation.truncate().distance(shop.translation.truncate());
    if dist / TILE_SIZE_PX <= SHOP_INTERACT_RADIUS_TILES {
        ui_open.0 = !ui_open.0;
    }
}

pub fn close_shop_on_walk_away_system(
    mut ui_open: ResMut<ShopUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    shop_q: Query<&Transform, (With<Shop>, Without<Player>)>,
) {
    if !ui_open.0 { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(shop) = shop_q.get_single() else { return };
    let dist = player.translation.truncate().distance(shop.translation.truncate());
    if dist / TILE_SIZE_PX > SHOP_INTERACT_RADIUS_TILES {
        ui_open.0 = false;
    }
}
```

- [ ] **Step 2: Register the module in `src/systems/mod.rs`**

```rust
pub mod shop;
pub mod shop_ui;       // will land in Task 11; forward-declare the module here now
```

(Leave `shop_ui` module file absent for now — Rust will error. Either add an empty `src/systems/shop_ui.rs` stub here, or wait to add both lines in Task 11. Cleaner option: add both module declarations NOW and create an empty `shop_ui.rs` stub containing only `use bevy::prelude::*;`. Then Task 11 fills in content.)

Empty stub `src/systems/shop_ui.rs`:
```rust
use bevy::prelude::*;
// systems will be added in Task 11
```

- [ ] **Step 3: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green. Shop system exists but is not yet registered in MiningSimPlugin; that happens in Task 12.

- [ ] **Step 4: Commit**

```bash
git add src/systems/shop.rs src/systems/shop_ui.rs src/systems/mod.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Shop systems: interact on E + auto-close on walk-away"
```

---

## Task 11: Shop UI — spawn panel + sync visibility + update labels + handle buttons

**Files:**
- Modify: `src/systems/shop_ui.rs`

- [ ] **Step 1: Implement the UI**

Replace `src/systems/shop_ui.rs`:

```rust
use bevy::prelude::*;
use crate::components::{ShopButtonKind, ShopUiOpen, ShopUiRoot};
use crate::economy::{self, BuyResult, Money};
use crate::grid::OreType;
use crate::inventory::Inventory;
use crate::tools::{OwnedTools, Tool};
use crate::systems::hud::{ore_visual_color, current_tool_display_name};

pub fn spawn_shop_ui(mut commands: Commands) {
    commands
        .spawn((
            ShopUiRoot,
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
            // Title
            panel.spawn((
                Text::new("SHOP"),
                TextFont { font_size: 24.0, ..default() },
            ));
            // Sell All button
            spawn_button(panel, "Sell All Ore", ShopButtonKind::SellAll);
            // Divider text
            panel.spawn((
                Text::new("Tools:"),
                TextFont { font_size: 18.0, ..default() },
            ));
            // Buy Pickaxe / Jackhammer / Dynamite
            spawn_buy_row(panel, Tool::Pickaxe);
            spawn_buy_row(panel, Tool::Jackhammer);
            spawn_buy_row(panel, Tool::Dynamite);
        });
}

fn spawn_button(parent: &mut ChildBuilder, label: &str, kind: ShopButtonKind) {
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
    )).with_children(|button| {
        button.spawn((
            Text::new(label),
            TextFont { font_size: 18.0, ..default() },
        ));
    });
}

fn spawn_buy_row(parent: &mut ChildBuilder, tool: Tool) {
    let price = economy::tool_buy_price(tool);
    let label = format!("Buy {} — {}c", current_tool_display_name(tool), price);
    spawn_button(parent, &label, ShopButtonKind::Buy(tool));
}

pub fn sync_shop_visibility_system(
    ui_open: Res<ShopUiOpen>,
    mut q: Query<&mut Visibility, With<ShopUiRoot>>,
) {
    if !ui_open.is_changed() { return; }
    if let Ok(mut vis) = q.get_single_mut() {
        *vis = if ui_open.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

pub fn update_shop_labels_system(
    money: Res<Money>,
    owned: Res<OwnedTools>,
    buttons_q: Query<(&ShopButtonKind, &Children)>,
    mut texts_q: Query<&mut Text>,
) {
    if !money.is_changed() && !owned.is_changed() { return; }
    for (kind, children) in buttons_q.iter_mut() {
        match kind {
            ShopButtonKind::SellAll => { /* static label */ }
            ShopButtonKind::Buy(tool) => {
                let new_label = if owned.0.contains(tool) {
                    format!("{} — OWNED", current_tool_display_name(*tool))
                } else {
                    let price = economy::tool_buy_price(*tool);
                    format!("Buy {} — {}c", current_tool_display_name(*tool), price)
                };
                for c in children.iter() {
                    if let Ok(mut text) = texts_q.get_mut(*c) {
                        **text = new_label.clone();
                    }
                }
            }
        }
    }
}

pub fn handle_shop_buttons_system(
    ui_open: Res<ShopUiOpen>,
    interaction_q: Query<(&Interaction, &ShopButtonKind), Changed<Interaction>>,
    mut inv: ResMut<Inventory>,
    mut money: ResMut<Money>,
    mut owned: ResMut<OwnedTools>,
) {
    // Defense-in-depth: Bevy does not deliver Interaction events for hidden UI,
    // but guard here in case system ordering changes or the UI is force-hidden
    // mid-frame.
    if !ui_open.0 { return; }
    for (interaction, kind) in interaction_q.iter() {
        if *interaction != Interaction::Pressed { continue; }
        match kind {
            ShopButtonKind::SellAll => {
                economy::sell_all(&mut inv, &mut money);
            }
            ShopButtonKind::Buy(tool) => {
                let _ = economy::try_buy(*tool, &mut money, &mut owned);
                // BuyResult::Ok / NotEnoughMoney / AlreadyOwned handled silently;
                // UI labels update via Changed<Money> / Changed<OwnedTools>.
            }
        }
    }
}
```

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green. Common Bevy 0.15 patch-version adaptations:
- `children![]` macro unavailable → use `.with_children` (already done in plan code).
- `ChildBuilder` may be `ChildSpawnerCommands` on some versions — use whichever the compiler expects.
- `Interaction` needs `Button` component — already in `spawn_button`.
- `BorderColor` import path varies; try `bevy::ui::BorderColor` or via prelude.

If any API adaptation is required, document in the task report.

- [ ] **Step 3: Commit**

```bash
git add src/systems/shop_ui.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Shop UI: spawn panel, sync visibility, update labels, handle buttons"
```

---

## Task 12: App wiring — register all new resources + systems

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Update `MiningSimPlugin`**

Replace `src/app.rs`:

```rust
use bevy::prelude::*;
use crate::systems::{camera, chunk_lifecycle, chunk_render, hud, ore_drop, player, setup, shop, shop_ui};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
                setup::setup_world,
                hud::setup_hud,
                shop_ui::spawn_shop_ui,
            ).chain())
           .add_systems(Update, (
                player::read_input_system,
                player::apply_velocity_system,
                player::collide_player_with_grid_system,
                player::dig_input_system,
                shop::shop_interact_system,
                shop::close_shop_on_walk_away_system,
                shop_ui::sync_shop_visibility_system,
                shop_ui::update_shop_labels_system,
                shop_ui::handle_shop_buttons_system,
                ore_drop::ore_drop_system,
                chunk_lifecycle::chunk_lifecycle_system,
                chunk_render::chunk_remesh_system,
                camera::camera_follow_system,
                hud::update_hud_system,
            ).chain());
    }
}
```

- [ ] **Step 2: Build + regression**

```bash
cargo build 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "App: register shop + shop_ui systems and resources"
```

---

## Smoke-test checkpoint #2 (after Task 12)

Human controller runs `cargo run`. Expected:

- Yellow shop square visible near spawn on the surface strip.
- Walk within 2 tiles + press `E` → shop panel appears (dark semi-transparent, centered-ish).
- Sell All button + three Buy buttons with current prices.
- Mine some copper, press `E` near shop, click Sell All — HUD money counter jumps.
- Buy Pickaxe for 30c → button row changes to `Pickaxe — OWNED`; HUD current tool updates.
- Close shop with `E` again, `Esc`, or walk away.
- Mining dirt now takes 2 strikes; stone takes 3 strikes.

Expected quirks allowed:
- Shop panel layout may look rough (placeholder art).
- Button hover styling absent — click still registers.

Report anomalies to controller before Task 13.

---

## Task 13: Hardness-visualization / playtest polish (optional)

Only implement if Checkpoint #2 reveals something the team wants fixed before the final playtest. Examples:
- SFX on clunk vs hit vs break (use Bevy `AudioPlayer` + a generated sine tone, or skip and add to M7 TODO).
- Panel layout improvements.
- Tuning ore or tool prices based on feel.

If nothing jumps out, skip this task and go straight to Task 14.

Commit any changes with an appropriate message.

---

## Task 14: Final playtest, roadmap update, merge to main

- [ ] **Step 1: Run full test suite**

```bash
cargo test
```
Expected target per plan tallies: 9 grid + 4 inventory + 8 terrain_gen + 15 dig + 10 tools + 8 economy ≈ **54 tests** passing (the spec's "~37" is stale; the plan added more tests than initially estimated). Actual count may drift ±2 with any test consolidation; as long as all pass, the count is fine.

- [ ] **Step 2: Manual exit-criteria walkthrough**

Run `cargo run`. Tick off per the spec's manual-playtest checklist:
- [ ] Game launches; shop visible near spawn.
- [ ] Shovel on dirt: 3 strikes; damage overlay darkens between strikes.
- [ ] Shovel on stone: clunk, no damage.
- [ ] Shovel on bedrock: clunk, no damage.
- [ ] Shop `E` open/close, `Esc` close, walk-away close all work.
- [ ] Sell All converts all ore to coins; HUD updates.
- [ ] Buy Pickaxe (30c) → OWNED; dirt now 2 strikes, stone now 3.
- [ ] Buy Jackhammer (100c) → OWNED; dirt 1 strike, stone 2, deep 3.
- [ ] Buy Dynamite (300c) → OWNED; Core 3 strikes, Deep 2, Stone 1, Dirt 1.
- [ ] Break through Core to bedrock floor; Bedrock boundary ring remains unbreakable.
- [ ] Current-tool HUD indicator updates.
- [ ] Partial damage persists across walking away and returning.
- [ ] No crashes over a ~20-minute session.
- [ ] Progression *feels* like "barely scratch → tear through."

- [ ] **Step 3: Add Playtest Results section to `docs/roadmap.md`**

Append under the existing M1 playtest section:

```markdown
## Playtest Results — Milestone 2 (YYYY-MM-DD)

Exit-criteria met: [summary]

**What felt good:**
- ...

**What felt off:**
- ...

**Decisions for milestone 3:**
- ...
```

Fill with actual observations — what worked, what didn't, what you'd change.

- [ ] **Step 4: Commit playtest notes**

```bash
git add docs/roadmap.md
git commit --author="wes2000 <whannasch@gmail.com>" -m "Record milestone 2 playtest results"
```

- [ ] **Step 5: Merge to main + push**

```bash
git checkout main
git merge --no-ff milestone-2 -m "Merge milestone-2: tool progression + tiered ores"
git push origin main
git branch -d milestone-2
```

- [ ] **Step 6: Final code review (optional but recommended)**

Dispatch the `superpowers:code-reviewer` subagent with the merged `main` HEAD commit as context — same pattern as M1's final review. Capture any "fix before M3" callouts before starting M3 brainstorm.

Milestone 2 complete.
