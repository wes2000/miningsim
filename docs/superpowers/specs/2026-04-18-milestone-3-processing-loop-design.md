# Milestone 3 — Surface Base + First Processing Loop (Design Spec)

**Date:** 2026-04-18
**Status:** Draft (awaiting spec review)
**Parent roadmap:** [../../roadmap.md](../../roadmap.md)
**Prior milestone:** [2026-04-18-milestone-2-tool-progression-design.md](./2026-04-18-milestone-2-tool-progression-design.md)

## Purpose

Close the end-to-end core gameplay loop by adding a **surface processing
machine** the player can feed raw ore into and collect higher-value
**bars** from, then sell those bars at the existing shop for ~5× the raw
price. Together with the M2 tool-progression shop, this completes the
**dig → process → sell → upgrade → dig deeper** cycle promised by the
roadmap exit criterion.

The milestone also folds in two M2-review "before-M3" cleanups:
- Extract the world↔tile coordinate conversion into a single helper module.
- Make Buy buttons in the shop UI visibly indicate affordability.

## Scope

### In scope

- A new **`OreKind`** enum (`Copper, Silver, Gold`) replacing M1/M2's
  `OreType` (which carried a `None` sentinel variant the M2 review
  flagged as a type smell).
- A new **`ItemKind { Ore(OreKind), Bar(OreKind) }`** enum unifying raw
  and processed items.
- **`Tile.ore: Option<OreKind>`** replaces `Tile.ore: OreType`.
- **`Inventory`** is now keyed by `ItemKind` (so ores and bars share a
  single store).
- One **Smelter** machine entity placed on the surface (3 tiles left of
  spawn; mirroring the Shop on the right).
- Three recipes: each `OreKind` smelts 1:1 into the matching bar at a
  fixed time per item (initial 2.0 s; tunable in playtest).
- **Timed processing with output buffer:** the player clicks `Smelt All
  X`, machine drains the inventory of that ore, processes one item per
  tick interval, accumulates bars in an internal output buffer. Player
  must click `Collect All` to move bars from buffer → inventory. Player
  may walk away mid-process; tick continues.
- **Smelter UI panel** opened with `E` within ~2 tiles, mirroring the
  Shop interaction model. Closes on `E`/`Esc`/walk-away.
- **Bar prices** in the existing Shop: 5× the raw-ore price (Copper
  Bar=5c, Silver Bar=25c, Gold Bar=100c). Sell All sells ores AND bars
  in one button.
- **HUD** gains three bar rows (six item rows total).
- **`coords.rs`** module collects `world_to_tile`, `tile_min_world`,
  `tile_center_world` (kills the 6-site Y-inversion duplication).
- **Shop Buy button affordability state:** visually darker when the
  player can't afford the tool (or already owns it).
- All systems organized into named `SystemSet`s (`InputSet`, `WorldSet`,
  `MachineSet`, `UiSet`) since the Update chain has grown beyond
  hand-readable length.

### Out of scope (deferred)

- **Save/load** — its own focused mini-milestone after M3, ahead of M4.
- **Multiple smelters or chained recipes** (ore → dust → bar) — M5.
- **Conveyors / pallets / forklifts / automation** — M5.
- **Buying additional smelters with money / per-property machines** — M6.
- **Day cycle / pacing structure** — only the loop is required; per
  roadmap, optional in M3.
- **Smelter visual states** (idle/busy/output-ready tints, pulse, etc.) —
  M7 polish unless trivially cheap mid-iteration.
- **Bar drops as physical entities on the ground** (M5 conveyor outputs).
- **Multiple recipes queued at the same time** — single recipe slot per
  machine; new clicks rejected while busy.
- **Audio cues** on smelt-complete or pickup-collect — M7.
- **Localization, accessibility, gamepad input** — M7.

### Explicitly not designed for

- Persistent state across sessions (see save/load above).
- Networking / multiplayer (M4).

## Target platform & tech

Unchanged from M2:

- Bevy 0.15.x (pinned). Rust stable.
- Top-down 2D, single player.
- Desktop (Windows / macOS / Linux).
- Placeholder art only.

## Key design decisions

| Decision | Choice | Why |
|---|---|---|
| Processing scope | Single Smelter, single step (ore → bar) | Bare minimum to satisfy the M3 exit criterion. Two-stage chains and per-ore specialist machines are easy to add in M5 when conveyors arrive. |
| Interaction model | Panel + timed output buffer | "Leave it cooking and come back" is the moment-to-moment fun of a factory game; instant processing makes the timer meaningless. Mirrors the Shop UI plumbing exactly. |
| Item-type model | `ItemKind { Ore(OreKind), Bar(OreKind) }` refactor | Resolves M2's `OreType::None` sentinel debt; unifies raw + processed under one inventory; cleanest path forward for M5/M6 processed-good families. |
| Architecture | Pure `processing.rs` + Bevy `SmelterState` component | Mirrors M2's `tools.rs`+`Tool` and `economy.rs`+`Money` patterns. Tick math headlessly testable; M5 multi-machine retrofit is no-op at the data layer. |
| M2 cleanups | Folded in as M3 Task 1 (`coords.rs`) and Task 2 (affordability buttons) | Both block clean M3 work — `coords.rs` before machine placement adds another world-to-tile site; affordability before machines start to cost real money. |

## Architecture

### Module / file layout

```
src/
  grid.rs                       # MODIFY: Tile.ore: Option<OreKind>; OreType removed
  inventory.rs                  # MODIFY: HashMap<ItemKind, u32>
  dig.rs                        # MODIFY: DigResult.ore: Option<OreKind>
  terrain_gen.rs                # MODIFY: Option<OreKind>; ore prob curves keyed by OreKind
  tools.rs                      # unchanged
  economy.rs                    # MODIFY: prices keyed by ItemKind; sell_all iterates ALL_ITEMS
  items.rs                      # NEW: pub OreKind, pub ItemKind, ALL_ORES, ALL_ITEMS
  processing.rs                 # NEW: SmelterState, tick_smelter, start_smelting, collect_output, is_busy, SMELT_DURATION_S
  components.rs                 # MODIFY: Smelter, SmelterUiRoot, SmelterButtonKind, SmelterStatusText markers; SmelterUiOpen resource
  systems/
    coords.rs                   # NEW: world_to_tile, tile_min_world, tile_center_world
    setup.rs                    # MODIFY: spawn Smelter entity; use coords helpers
    player.rs                   # MODIFY: ItemKind in OreDrop spawn; coords helpers
    chunk_lifecycle.rs          # MODIFY: coords helpers
    chunk_render.rs             # MODIFY: Option<OreKind>; coords helpers
    ore_drop.rs                 # MODIFY: ItemKind on OreDrop; pickup adds to inventory by ItemKind
    hud.rs                      # MODIFY: 6 item rows driven by ALL_ITEMS
    shop.rs                     # unchanged
    shop_ui.rs                  # MODIFY: Sell All iterates ALL_ITEMS; affordability state on Buy buttons
    smelter.rs                  # NEW: smelter_interact, walk_away, tick, panel spawn/sync/refresh, button handler
  app.rs                        # MODIFY: register smelter systems; group all systems into named SystemSets
  lib.rs                        # MODIFY: pub mod items, processing
tests/
  grid.rs                       # MODIFY: Option<OreKind>
  inventory.rs                  # MODIFY: ItemKind keys; mixed ore+bar round-trip
  terrain_gen.rs                # MODIFY: assertions in OreKind terms
  dig.rs                        # MODIFY: try_dig returns Option<OreKind>
  tools.rs                      # unchanged
  economy.rs                    # MODIFY: ItemKind prices, mixed-inventory sell_all
  items.rs                      # NEW: enum/constants smoke
  processing.rs                 # NEW: tick_smelter math
  coords.rs                     # NEW: round-trip + Y-inversion
```

**New modules:** `items.rs`, `processing.rs`, `coords.rs`, `systems/smelter.rs`.

### Module boundary

Pure modules (`grid`, `inventory`, `dig`, `terrain_gen`, `items`,
`processing`, `tools`, `economy`, `coords`) take no Bevy systems or
queries. `SmelterState` derives `Component` so it can live on an entity,
but its mutation is exclusively via `processing::*` functions.

Dependency direction: `main → app → systems → pure modules`.

## Components / modules in detail

### `items.rs` (new, pure)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OreKind { Copper, Silver, Gold }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Ore(OreKind),
    Bar(OreKind),
}

pub const ALL_ORES: [OreKind; 3] = [OreKind::Copper, OreKind::Silver, OreKind::Gold];
pub const ALL_ITEMS: [ItemKind; 6] = [
    ItemKind::Ore(OreKind::Copper), ItemKind::Ore(OreKind::Silver), ItemKind::Ore(OreKind::Gold),
    ItemKind::Bar(OreKind::Copper), ItemKind::Bar(OreKind::Silver), ItemKind::Bar(OreKind::Gold),
];
```

### `grid.rs` (modified)

`Tile.ore: Option<OreKind>` (was `OreType`). `Tile::default().ore = None`.
Storage and bounds logic unchanged.

### `dig.rs` (modified)

`DigResult.ore: Option<OreKind>`. `try_dig` returns `Option<OreKind>` for
the ore captured on `Broken`. No other behavior change. The
`dig_target_valid` helper is unchanged.

### `terrain_gen.rs` (modified)

`maybe_assign_ore` operates on `&mut Option<OreKind>`. Per-layer prob
curves indexed by `OreKind` (the same probability table from M2,
relabeled).

### `inventory.rs` (modified, pure)

```rust
#[derive(Debug, Default, Resource)]
pub struct Inventory {
    counts: HashMap<ItemKind, u32>,
}

impl Inventory {
    pub fn add(&mut self, item: ItemKind, n: u32);
    pub fn remove(&mut self, item: ItemKind, n: u32);   // saturating
    pub fn get(&self, item: ItemKind) -> u32;
}
```

Bars and ores share the store. No `OreType::None` guard needed since the
type system no longer permits it.

### `processing.rs` (new, pure)

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

pub fn is_busy(state: &SmelterState) -> bool;

pub fn start_smelting(state: &mut SmelterState, ore: OreKind, count: u32);
//   No-op if state.recipe.is_some() (busy) or count == 0.
//   Sets recipe = Some(ore), queue = count, time_left = SMELT_DURATION_S.

pub fn tick_smelter(state: &mut SmelterState, dt: f32) -> SmeltTickEvent;
//   No-op if state.recipe.is_none().
//   Decrements time_left by dt. When time_left <= 0:
//     state.output[recipe] += 1, queue -= 1, and either
//       reset time_left = SMELT_DURATION_S (queue > 0) or
//       set recipe = None (queue == 0).
//   Returns BarFinished(ore) on completion, None otherwise.
//   Tick overshoot at very large dt completes EXACTLY ONE item, never more.

pub fn collect_output(state: &mut SmelterState) -> HashMap<OreKind, u32>;
//   Drains state.output and returns it.
```

### `economy.rs` (modified)

```rust
pub fn item_sell_price(item: ItemKind) -> u32 {
    match item {
        ItemKind::Ore(OreKind::Copper) => 1,
        ItemKind::Ore(OreKind::Silver) => 5,
        ItemKind::Ore(OreKind::Gold)   => 20,
        ItemKind::Bar(OreKind::Copper) => 5,    // 5x ore
        ItemKind::Bar(OreKind::Silver) => 25,   // 5x ore
        ItemKind::Bar(OreKind::Gold)   => 100,  // 5x ore
    }
}

pub fn sell_all(inv: &mut Inventory, money: &mut Money) {
    for item in items::ALL_ITEMS {
        let count = inv.get(item);
        if count == 0 { continue; }
        money.0 += item_sell_price(item) * count;
        inv.remove(item, count);
    }
}
```

`tool_buy_price` and `try_buy` unchanged.

### `coords.rs` (new, pure)

```rust
use bevy::math::{IVec2, Vec2};
use crate::systems::setup::TILE_SIZE_PX;     // or move TILE_SIZE_PX into coords.rs

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

`tile_center_world` (currently in setup.rs) moves here; setup.rs imports
from coords. The `TILE_SIZE_PX` constant either stays in setup and is
imported, or migrates here — implementation choice.

### `components.rs` (modified)

Add: `Smelter` (marker), `SmelterUiRoot` (marker),
`SmelterButtonKind { SmeltAll(OreKind), CollectAll }`, `SmelterStatusText`
(marker for the status line label), and `SmelterUiOpen(pub bool)` resource.

`OreSprite { ore: OreKind }` and `OreDrop { item: ItemKind }` (M3 only
ever drops `ItemKind::Ore(_)`; the `Bar` variant is reserved for M5
conveyor output).

### `systems/smelter.rs` (new)

Mirrors the shop systems pattern:

- `smelter_interact_system` — `E` toggles `SmelterUiOpen` when within
  2 tiles of the Smelter entity; `Esc` force-closes.
- `close_smelter_on_walk_away_system` — auto-close on distance.
- `smelter_tick_system` — each frame queries `&mut SmelterState`, calls
  `processing::tick_smelter`. Discards the returned event for M3 (M7
  polish or M4 networking might consume it).
- `spawn_smelter_ui` (Startup) — builds the panel hidden, with status
  label, three SmeltAll buttons, output line label, CollectAll button.
- `sync_smelter_visibility_system` — mirrors `SmelterUiOpen` state.
- `update_smelter_panel_system` — runs on `Changed<SmelterState>` or
  `Changed<Inventory>`. Refreshes status text, button enabled/disabled
  visuals, output line, CollectAll enabled state.
- `handle_smelter_buttons_system` — gates on `SmelterUiOpen.0`; on
  `SmeltAll(ore)` Pressed: read inventory count, remove all, call
  `start_smelting`. On `CollectAll` Pressed: `collect_output(&mut state)`,
  fold each `(ore, n)` into inventory as `ItemKind::Bar(ore)`.

### `systems/shop_ui.rs` (modified)

Existing system additions:

- `update_shop_labels_system` now also sets `BackgroundColor` on each
  Buy button row to a dimmer shade when not affordable
  (`money.0 < tool_buy_price(tool) || owned.contains(tool)`). Resolves
  M2 review item #3.
- `Sell All` handler now calls the M3 `economy::sell_all`, which
  iterates `ALL_ITEMS` (sells ores and bars in one operation).

### `systems/hud.rs` (modified)

`setup_hud` builds 6 item rows by iterating `items::ALL_ITEMS` instead
of three hardcoded ore rows. `OreCountText` marker becomes
`OreCountText(pub ItemKind)` (or rename to `ItemCountText`).

`update_hud_system` queries on `Changed<Inventory>`; for each row,
formats `inv.get(row.0)` into the label.

### `app.rs` (modified)

System set grouping using Bevy 0.15 `SystemSet`:

```rust
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InputSet { Read, Apply }   // movement, dig

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorldSet { Collide, Drops, ChunkLifecycle, ChunkRender }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MachineSet { ShopInteract, ShopUi, SmelterInteract, SmelterTick, SmelterUi }

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UiSet { Hud, Camera }
```

Concrete ordering uses `.in_set(...)` and `.chain()` between sets where
necessary (input → collide → tick → render → ui).

## Data flow

### Startup
1. `setup_world` generates Grid, inserts existing resources + `SmelterUiOpen::default()`.
2. Spawns Player (with Facing).
3. Spawns Shop entity (3 tiles right, yellow sprite).
4. Spawns **Smelter entity** (3 tiles left, orange sprite, `SmelterState::default()`).
5. Spawns Camera.
6. `spawn_shop_ui` and `spawn_smelter_ui` build their hidden panels.

### Dig (post-refactor — semantics identical to M2)
1. LMB or Space → tool-aware target tile, dig_target_valid gate, best_applicable_tool, try_dig.
2. On `Broken` with `Some(ore)`: spawn `OreDrop { item: ItemKind::Ore(ore) }`. Vacuum + pickup unchanged; pickup calls `inventory.add(ItemKind::Ore(ore), 1)`.

### Smelt — start
1. Player walks within 2 tiles of Smelter, presses `E`. `smelter_interact_system` toggles `SmelterUiOpen.0`.
2. `sync_smelter_visibility_system` reveals the panel.
3. `update_smelter_panel_system` populates status (`IDLE`), Smelt buttons (enabled iff ore count > 0 AND machine idle), output line, CollectAll state.
4. Player clicks `Smelt All Copper`. `handle_smelter_buttons_system`:
   - `n = inventory.get(ItemKind::Ore(Copper))`
   - `inventory.remove(ItemKind::Ore(Copper), n)`
   - `processing::start_smelting(&mut state, Copper, n)`

### Smelt — tick
1. `smelter_tick_system` calls `processing::tick_smelter(&mut state, time.delta_secs())` per frame.
2. When an item completes, `state.output[Copper] += 1`, queue decrements, timer resets or recipe clears.
3. Bevy detects `&mut SmelterState` mutation → next-frame
   `update_smelter_panel_system` refreshes status countdown and output count.

### Smelt — collect
1. Player clicks `Collect All`. Handler calls `processing::collect_output(&mut state)` (drains).
2. For each `(ore, n)`: `inventory.add(ItemKind::Bar(ore), n)`.
3. HUD bar rows update via `Changed<Inventory>`; smelter panel output line zeros.

### Sell
1. Player at Shop, clicks `Sell All`. `economy::sell_all(&mut inv, &mut money)` iterates `ALL_ITEMS`, drains every kind, credits money.
2. HUD money row + all six item rows update.

### Buy (M2 + affordability)
1. `update_shop_labels_system` recomputes affordability per Buy button on `Changed<Money>` / `Changed<OwnedTools>`. Sets dim background when unaffordable / owned.
2. Click on unaffordable button → `try_buy` returns `NotEnoughMoney` (no-op). Visual already communicates the cause.

## Cross-cutting invariants

- **Grid is the single source of truth for terrain.** Unchanged.
- **Inventory is the single source of truth for player items** — both ores and bars, keyed by `ItemKind`.
- **`SmelterState` is the single source of truth for machine state** — per entity. All mutations flow through `processing::*` functions; Bevy systems never touch `state.queue` or `state.output` directly.
- **Pure modules stay pure.** No system / query / commands inside `items`, `processing`, `coords`, `inventory`, `economy`, `dig`, `grid`, `terrain_gen`, `tools`.
- **No `Local<T>` system state holds game-relevant data** — all persistent state is `Resource` or `Component`. Save/load remains tractable.
- **`OreType::None` is gone.** The type system enforces "ore presence is `Option<OreKind>`."

## Edge cases & error handling

- **Smelt button clicked with 0 of that ore.** Panel disables it; `start_smelting` no-ops on count 0.
- **Smelt button clicked while busy.** Panel disables; `start_smelting` no-ops if `recipe.is_some()`.
- **Collect All on empty output.** Panel disables; `collect_output` returns empty map.
- **Player walks away mid-process.** Tick continues; bars accumulate.
- **Player closes panel mid-process.** Same — visibility doesn't gate tick.
- **Game closes mid-process.** State is lost (no save/load yet — by design).
- **Tick overshoot at low FPS.** When `dt > SMELT_DURATION_S`, `tick_smelter` completes EXACTLY one item per call (resets timer to full `SMELT_DURATION_S`, doesn't accumulate negative carryover). Predictable over realistic.
- **`SmelterState.output` overflow.** `u32` limit (~4B) — unreachable in practice.
- **Sell All on empty inventory.** Sums to zero, no state change.
- **Affordability state stale by one frame.** `Changed<Money>` triggers next-frame label refresh — imperceptible to player.
- **Coordinate edge cases.** `world_to_tile` at exact tile boundaries uses `floor`; behavior consistent across positive and negative tile coords. Tested explicitly.
- **Migration completeness.** Post-refactor, zero references to `OreType::None` or the `OreType` name should remain. Greppable.

### Explicitly not handled in M3
- Save / load.
- Multiple smelters or chained recipes.
- Bar drops as ground entities.
- Audio cues, particle effects.
- Concurrent recipe execution per machine.

## Testing approach

### Headless unit tests
- **`items`** — enum derives, `ALL_ORES` / `ALL_ITEMS` correctness.
- **`coords`** — round-trip `world_to_tile(tile_center_world(t)) == t`; Y-inversion correctness; positive and negative tile coords.
- **`processing`** — `tick_smelter` math: no-op when idle; `start_smelting(0)` and `start_smelting(while busy)` no-ops; one-item completion; multi-item queue draining; tick overshoot completes exactly one item; `collect_output` drains correctly.
- **`grid`** — `Option<OreKind>` round-trip; default tile `ore = None`.
- **`terrain_gen`** — assertions in `OreKind` terms; no `Bedrock` interior.
- **`dig`** — `try_dig` returns correct `Option<OreKind>`; existing damage / tier / LoS coverage unchanged in semantics.
- **`inventory`** — `ItemKind` keys; round-trip with mixed `Ore(_)` and `Bar(_)` entries.
- **`economy`** — `item_sell_price` matrix; `sell_all` mixed-inventory math; `try_buy` unchanged.
- **`tools`** — unchanged.

**Approximate test count target:** ~70 (M2's 54 + 7 processing + 4 coords + 1–2 items + small additions across grid/inventory/economy/dig).

### Bevy systems
Not unit-tested. Manual smoke-test consistent with M1/M2 policy.

### Manual playtest exit-criteria

- [ ] Game launches; orange Smelter visible 3 tiles left of spawn; yellow Shop still 3 tiles right.
- [ ] Smelter panel: `E` opens, `Esc` / `E` / walk-away closes.
- [ ] Mining ores still works (post-refactor regression check).
- [ ] HUD shows 6 item rows + money + current tool.
- [ ] Smelt All Copper drains inventory copper, status changes to `Smelting Copper Bar (2.0s, queue: N)`. Buttons disabled while busy.
- [ ] Status countdown ticks; queue decrements; output line increments.
- [ ] On queue empty: status returns to `IDLE`; CollectAll becomes enabled.
- [ ] CollectAll moves bars to inventory; HUD bar rows update.
- [ ] Walk-away during smelt: machine keeps cooking; come back to full output.
- [ ] Shop Sell All: drains both ores AND bars; coin count jumps; HUD ore + bar rows zero.
- [ ] Bar revenue >> raw-ore revenue (verify 5× math).
- [ ] Shop Buy buttons darker when unaffordable; click is silent no-op (label unchanged).
- [ ] Full loop: dig copper → smelt → collect → sell bars → buy Pickaxe → dig stone → smelt silver → sell → buy Jackhammer → dig deep → smelt gold → sell → buy Dynamite → dig core to bedrock floor.
- [ ] M2 behaviors (cardinal dig, LoS, tier-gate, damage overlay, spacebar facing, MTV collision) still work.
- [ ] No crashes over a 20-minute session.
- [ ] Processing *feels* like a real machine — leaving it cooking is satisfying.

### Explicitly not tested
- Pixel-perfect smelter panel layout.
- Smelter sprite art.
- Audio.
- Save / load (no persistence yet).

## Open questions deferred to implementation planning

- Whether `TILE_SIZE_PX` lives in `coords.rs` or stays in `systems/setup.rs`. Aesthetic; either works.
- Whether `OreCountText` is renamed to `ItemCountText` (the field type changes from `OreType` to `ItemKind` regardless).
- Smelter sprite color shade — orange seems fine but compare vs Shop yellow at runtime.
- Smelter panel position — likely centered like Shop; possibly offset so two open panels could coexist (Smelter and Shop), though there's no UX scenario where both are open simultaneously since they're far apart.
- Whether the smelter panel should also show the smelt-time per item as a hint in the buttons (e.g., `Smelt All Copper (×N · 2.0 s ea)`) — minor UX polish.

---

**Spec end.** Implementation plan to follow via the writing-plans skill once this spec is approved.
