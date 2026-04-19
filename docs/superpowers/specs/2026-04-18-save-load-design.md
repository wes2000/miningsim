# Save / Load — Mini-Milestone Design Spec

**Date:** 2026-04-18
**Status:** Draft (awaiting spec review)
**Parent roadmap:** [../../roadmap.md](../../roadmap.md)
**Prior milestone:** [2026-04-18-milestone-3-processing-loop-design.md](./2026-04-18-milestone-3-processing-loop-design.md)

## Purpose

Add persistent save/load so a player can quit and resume the dig → smelt
→ sell → upgrade loop on the same property. Single slot, hotkeys plus
auto-save on quit. **Focused mini-milestone** — kept deliberately tight
because (a) it's orthogonal to gameplay and (b) the next milestone (M4
networking) needs the same serde derives, so this work pays interest
forward immediately.

## Scope

### In scope
- One save slot at `./save.ron` (project-relative, RON format).
- `F5` saves; `F9` loads; closing the window auto-saves.
- On startup, if `save.ron` exists and parses, the game state is
  overwritten by the saved data after `setup_world` builds the fresh
  world. If it doesn't exist or fails to parse, the fresh world stands.
- Saved state: `Grid`, `Inventory`, `Money`, `OwnedTools`,
  `SmelterState`, player tile position.
- Skipped state (regenerated, default, or deliberately ephemeral):
  `DigCooldown` (Timer; reset to default), all panel `*UiOpen` resources
  (start closed), `OreDrop` entities (vanish), terrain chunks
  (re-spawned by lifecycle), camera position (snaps to player), HUD /
  Shop / Smelter entities (re-spawned by `setup_world`), Player
  `Velocity` and `Facing` (zero / default-down), world seed (the Grid
  itself is the post-procgen state).
- Schema versioning via a `version: u32` field. Version mismatch on
  load → log warning + ignore the save (no migration code yet).
- Hand-rolled `SaveData` struct + `serde` derives on the persistent
  pure-data types. No Bevy reflection.
- All file IO and Bevy glue lives in `systems/save_load.rs`; pure
  serialize/deserialize/collect/apply lives in `save.rs`.

### Out of scope (deferred)
- Multiple save slots / named saves.
- New Game / Save / Load menu UI.
- Backups (`.bak`) or rotated saves.
- Save-file metadata (timestamp, playtime, screenshot).
- Cross-platform user-data-dir (`%APPDATA%` / XDG dirs) — deferred
  until we cut a public build.
- Compressed save format (RON is fine at our sizes).
- Save-file migration between schema versions (mismatch is silently
  ignored for now).
- Concurrent / network-replicated save state (M4).

### Explicitly not designed for
- Save scumming protection / checksum / tamper detection.
- Multiplayer-replicated saves.
- Frequent autosave (every N seconds) — quit-only auto-save is enough.

## Target platform & tech

Unchanged from M3:
- Bevy 0.15.x (pinned). Rust stable.
- Top-down 2D, single player.
- Desktop (Windows / macOS / Linux).
- New deps: `serde = { version = "1", features = ["derive"] }`,
  `ron = "0.8"`.

## Key design decisions

| Decision | Choice | Why |
|---|---|---|
| Trigger model | `F5` save + `F9` load + auto-save on `AppExit` (option B) | Single slot is sufficient for one property / one player; auto-save on quit removes the "I forgot to save" footgun without a multi-slot UI we don't yet need. |
| File location + format | `./save.ron`, RON (option A) | Debug-readable during dev; OS user-data-dir is a one-line constant change at ship time. RON is the Bevy-ecosystem default and trivially serde-compatible. |
| Serialization style | Hand-rolled `SaveData` struct + serde derives (option B, vs. Bevy reflection) | Bevy reflection is overkill for one mini-milestone with five resources and one component. Hand-rolled keeps the pure-module discipline, is unit-testable headlessly, and is easy to evolve. |
| World seed storage | Not stored | The full Grid is saved; the seed is irrelevant once procgen has run. Keeps the save self-contained. |
| Player Facing / Velocity / cooldown timer | Not saved | All three are tiny continuity quirks at most — defaulting them on load is invisible to the player and avoids serializing Bevy's `Timer` (which doesn't derive Serialize). |

## Architecture

### Module / file layout

```
Cargo.toml                       # MODIFY: + serde { features = ["derive"] }, + ron
src/
  save.rs                        # NEW: pure — SaveData struct, SAVE_VERSION, collect/apply, serialize/deserialize, LoadError
  grid.rs                        # MODIFY: derive Serialize/Deserialize on Tile, Layer, Grid; add Grid::from_raw
  items.rs                       # MODIFY: derive Serialize/Deserialize on OreKind, ItemKind
  inventory.rs                   # MODIFY: derive Serialize/Deserialize on Inventory
  economy.rs                     # MODIFY: derive Serialize/Deserialize on Money
  tools.rs                       # MODIFY: derive Serialize/Deserialize on Tool, OwnedTools
  processing.rs                  # MODIFY: derive Serialize/Deserialize on SmelterState
  systems/
    save_load.rs                 # NEW: Bevy systems — F5 save, F9 load, AppExit auto-save, startup load-if-exists
    setup.rs                     # unchanged: setup_world stays unconditional; startup-load runs after it as a separate system
  app.rs                         # MODIFY: register save_load Startup + Update systems; chain startup-load AFTER all setup systems
  lib.rs                         # MODIFY: pub mod save
tests/
  save.rs                        # NEW: round-trip via RON; collect→serialize→deserialize→apply equality; version mismatch
```

**Module boundary:** `save.rs` is pure — `collect`/`apply` take plain
references to underlying types, no Bevy queries, no commands, no file IO.
Serialization happens via `serde::Serialize` / `Deserialize`. File IO and
Bevy-side glue (hotkeys, AppExit handler, startup-load decision) live in
`systems/save_load.rs`.

## Components / modules in detail

### `save.rs` (new, pure)

```rust
use serde::{Deserialize, Serialize};

use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::tools::OwnedTools;

pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveData {
    /// Schema version. Bump when SaveData layout changes; mismatched
    /// loads are silently discarded (no migration logic yet).
    pub version: u32,
    pub grid: Grid,
    pub inventory: Inventory,
    pub money: Money,
    pub owned_tools: OwnedTools,
    pub smelter: SmelterState,
    /// Player world position as `(x, y)`. Plain array avoids pulling
    /// Bevy types into the pure module.
    pub player_pos: [f32; 2],
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(ron::error::SpannedError),
    VersionMismatch { found: u32, expected: u32 },
}

pub fn collect(
    grid: &Grid,
    inventory: &Inventory,
    money: &Money,
    owned: &OwnedTools,
    smelter: &SmelterState,
    player_pos: [f32; 2],
) -> SaveData;

/// Idempotent: applying the same SaveData twice yields the same state.
pub fn apply(
    data: SaveData,
    grid: &mut Grid,
    inventory: &mut Inventory,
    money: &mut Money,
    owned: &mut OwnedTools,
    smelter: &mut SmelterState,
    player_pos: &mut [f32; 2],
);

pub fn serialize_ron(data: &SaveData) -> Result<String, ron::Error>;
pub fn deserialize_ron(s: &str) -> Result<SaveData, LoadError>;
//   Wraps ron::de + version check; returns VersionMismatch / Parse.
```

### Modified pure modules

Each gains `#[derive(Serialize, Deserialize)]` on its public types:

- **`grid.rs`** — `Tile`, `Layer`, `Grid`. Plus a new
  `pub fn Grid::from_raw(width: u32, height: u32, tiles: Vec<Tile>) -> Self`
  that panics on length mismatch. The struct's private fields will be
  exposed for serde via either `#[serde(into = "GridRepr", from = "GridRepr")]`
  with a small repr struct, or by changing field visibility — implementation
  choice.
- **`items.rs`** — `OreKind`, `ItemKind`. Both unit/tuple-style enums;
  serde derives are trivial.
- **`inventory.rs`** — `Inventory` (wraps `HashMap<ItemKind, u32>`).
  Adding the derives requires `ItemKind: Serialize + Deserialize` (added
  above). RON serializes `HashMap<EnumKind, _>` correctly via map syntax;
  no special wrapper needed.
- **`economy.rs`** — `Money(pub u32)`.
- **`tools.rs`** — `Tool`, `OwnedTools` (wraps `HashSet<Tool>`).
- **`processing.rs`** — `SmelterState` (`recipe: Option<OreKind>`,
  `time_left: f32`, `queue: u32`, `output: HashMap<OreKind, u32>`).

### `systems/save_load.rs` (new)

```rust
use std::path::PathBuf;
use bevy::prelude::*;
use bevy::app::AppExit;

use crate::components::{Player, Smelter};
use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::save::{self, SaveData, LoadError};
use crate::tools::OwnedTools;

pub const SAVE_PATH: &str = "save.ron";

/// Startup system, ordered AFTER setup_world + UI spawners. If
/// `./save.ron` exists, load and apply.
pub fn startup_load_system(/* see below */);

/// Update system: F5 just-pressed → save current state.
pub fn save_hotkey_system(/* keys + state queries */);

/// Update system: F9 just-pressed → reload from disk.
pub fn load_hotkey_system(/* keys + state queries */);

/// Update system listening for AppExit; runs save before exit propagates.
pub fn auto_save_on_exit_system(/* AppExit reader + state queries */);

// Internal helpers (private; tested via the systems' integration paths):
fn save_now(/* refs to all state */) -> Result<(), std::io::Error>;
fn try_load_and_apply(/* mut refs to all state */) -> Result<(), LoadError>;
```

All four systems share the same query parameter shape: `Res<Grid>` + mut
or non-mut variants of `Inventory`/`Money`/`OwnedTools`, plus
`Query<&mut SmelterState, With<Smelter>>` and
`Query<&mut Transform, With<Player>>`. Rust query borrow-checker permits
this since each component is borrowed by exactly one system.

### `systems/setup.rs` (unchanged)

`setup_world` stays unconditional. It always builds a fresh world. Load
runs as a separate startup system ordered after it, so the fresh world is
either left alone (no save) or overwritten (valid save).

### `app.rs` (modified)

Startup chain extends to include the load step at the end:
```rust
.add_systems(Startup, (
    setup::setup_world,
    hud::setup_top_right_hud,
    hud::spawn_inventory_popup,
    shop_ui::spawn_shop_ui,
    smelter::spawn_smelter_ui,
    save_load::startup_load_system,
).chain())
```

Update chain adds three new systems in a dedicated `UiSet::SaveLoad`
variant (separate from HUD so future HUD reorderings don't accidentally
move file IO):

```rust
// app.rs SystemSet enum gains a SaveLoad variant on UiSet:
pub enum UiSet { Hud, SaveLoad, Camera }

// Update registration:
save_load::save_hotkey_system.in_set(UiSet::SaveLoad),
save_load::load_hotkey_system.in_set(UiSet::SaveLoad),
save_load::auto_save_on_exit_system.in_set(UiSet::SaveLoad),
```

`configure_sets` order: `... UiSet::Hud → UiSet::SaveLoad → UiSet::Camera`.
File IO runs after the HUD has had a chance to refresh on whatever the
last frame's input did, so an F5 captures the visible state.

### `Cargo.toml` (modified)

```toml
[dependencies]
bevy = "0.15"
rand = "0.8"
serde = { version = "1", features = ["derive"] }
ron = "0.8"
```

## Data flow

### Startup with no save file
1. App starts. `setup_world` builds the fresh world.
2. UI spawners (HUD top-right, inventory popup, shop UI, smelter UI) build their hidden roots.
3. `startup_load_system` checks `./save.ron`. Doesn't exist → `info!("no save file found, starting fresh")`. No-op.
4. Game runs normally.

### Startup with a valid save
1. Same through step 2 — fresh world built.
2. `startup_load_system` reads `./save.ron`, deserializes, version-checks, calls `apply()` — overwrites Grid, Inventory, Money, OwnedTools, SmelterState, Player Transform.
3. First Update tick: `chunk_lifecycle_system` runs against the (now-loaded) Grid. Since startup completes before any Update tick, **no chunks have been spawned yet** — lifecycle spawns them directly from the loaded grid. No visible "fresh-then-loaded" flash on startup. (Mid-session F9 IS subject to a 1-frame churn while existing chunks despawn and respawn — see below — acceptable.)
4. HUD update systems detect `Changed<Inventory>`, `Changed<Money>`, `Changed<OwnedTools>` and refresh.
5. Smelter panel update detects `Changed<SmelterState>` and refreshes.
6. Game continues from the saved state.

### F5 save mid-session
1. `save_hotkey_system` sees `keys.just_pressed(F5)`.
2. Reads Grid (Res), Inventory (Res), Money (Res), OwnedTools (Res), SmelterState (single Smelter component), Player Transform.
3. Calls `save::collect(...)` → `SaveData`.
4. `save::serialize_ron(&data)` → string.
5. Writes string to `./save.ron`.
6. `info!("game saved")` on success; `error!` on IO failure (game continues).

### F9 load mid-session
1. `load_hotkey_system` sees `keys.just_pressed(F9)`.
2. Reads `./save.ron`, deserializes, version-checks.
3. `apply()` mutates Grid + Inventory + Money + OwnedTools + SmelterState + Player Transform in place.
4. Same chunk-respawn / HUD-refresh ripples as startup-load.
5. `info!("save loaded")` on success; `warn!` / `error!` on parse / version mismatch / IO failure (current state untouched).

### Auto-save on quit
1. Player closes the window or invokes Alt-F4. Bevy emits `AppExit`.
2. `auto_save_on_exit_system` runs, observes the event, performs the F5 save flow.
3. Bevy processes AppExit at end of frame; the save system runs first.
4. Save IO failure logs `error!` — no popup, no retry — and the app exits anyway.

## Cross-cutting invariants

- **`apply()` is idempotent.** Calling `apply(data, ...)` twice yields the same state as one call. Lets F9-spam be safe and lets the round-trip test compare two applications.
- **`SaveData` carries `version` as the first field.** Future tooling can read it without parsing the rest.
- **Pure modules stay pure.** `save.rs` takes plain references; no Bevy queries / commands / Resources inside. File IO lives only in `systems/save_load.rs`.
- **`setup_world` always runs.** Load is purely an overwrite step. A corrupt or missing save never blocks the game from starting.
- **No save state diverges from runtime state by design.** What the player sees is what gets saved. Mid-vacuum drops, dig cooldown, panel visibility, etc., are deliberately ephemeral — documented in the spec and roadmap so the player can predict load behavior.
- **No backwards-compat migration in this milestone.** `version` field exists; mismatch is discarded. Future-us writes a migrator only when there's a real save worth preserving.

## Edge cases & error handling

### File-IO failures
- **Save file missing on startup-load.** `info!("no save file found, starting fresh")` → no-op.
- **Save file unreadable** (permissions, partial write). `error!("save load failed: {err}")` → fall through to fresh world.
- **Save fails to write** (disk full, locked file, permission). `error!("save failed: {err}")` → game continues.
- **Concurrent F5 within one frame** — `just_pressed` only fires once; no race.

### Parse failures
- **Malformed RON.** Reported as `LoadError::Parse`. On startup: fresh world; on F9: leave current state untouched.
- **Schema mismatch** (field added/removed since the save). RON's serde detects unknown / missing required fields — handled like any parse error.

### Version mismatch
- `version` ≠ `SAVE_VERSION` → `LoadError::VersionMismatch { found, expected }`. Logged as `warn!`. Same fallback behavior as parse error.

### Apply-time edge cases
- **Smelter entity missing when applying.** Defense: if `Query<&mut SmelterState>` is empty, log warning and skip the smelter restore. Other state still loads.
- **Multiple Smelter entities.** Apply to the first one; log warning.
- **Player position outside Grid bounds.** Apply blindly; collision resolves it next frame.
- **Loaded Grid dimensions ≠ setup-time `MAP_W`/`MAP_H`.** Grid is fully overwritten; chunk_lifecycle handles whatever dimensions are present.

### Mid-process saves
- **Save while smelter is busy.** SmelterState's full state serializes / restores → tick continues from exact saved fractional `time_left`.
- **Save while panel open.** Panel-open resources are not saved → all panels start closed on load.
- **Save with mid-vacuum drops.** OreDrops not saved → drops vanish on load. Standard "save doesn't capture pickups" convention.
- **Save during dig cooldown.** Timer not saved → resets on load → next click is responsive.

### State the player should expect to lose on load
- Mid-vacuum ore drops
- Dig cooldown progress (always immediately ready post-load)
- Open panels (always closed post-load)
- Player velocity (zero post-load)
- Player facing (default down post-load)

### Explicitly NOT handled in this milestone
- `.bak` of previous save before overwrite
- Save-file metadata (timestamp / playtime / screenshot)
- Cross-platform user-data-dir
- Saving while unfocused / minimized (not racy; same code path)
- Compressed save format

## Testing approach

### Headless unit tests (cargo test)

In `tests/save.rs`:
- **Round-trip via RON.** Build a `SaveData` with non-default content (mixed inventory ores+bars, owned subset of tools, active smelt with output, non-zero player position, Grid with damaged tiles). `serialize_ron` → `deserialize_ron` → assert structural equality.
- **`apply()` correctness.** Build a `SaveData`, call `apply()` against fresh resources, assert each resource matches the SaveData fields.
- **`apply()` idempotence.** Apply same data twice; assert state matches single application.
- **Version mismatch detection.** Build a SaveData with `version != SAVE_VERSION`, deserialize, expect `LoadError::VersionMismatch { found, expected }`.
- **`Grid::from_raw` round-trip.** Construct via `Grid::new`, mutate tiles, serialize, deserialize, confirm equality.
- **`Grid::from_raw` length mismatch panics.** `#[should_panic]` test.
- **Inventory round-trip with mixed Ore/Bar.** Direct serde test on `Inventory` type.
- **OwnedTools round-trip.** Direct serde test on `OwnedTools`.
- **SmelterState round-trip with active recipe + non-empty output.** Direct serde test.

Approximate test count: **~9 new tests in `tests/save.rs`**, plus 1–2 small additions to `tests/grid.rs` for `from_raw`. Final total target ~85 (M3's 76 + ~9 new + some small migration).

### Bevy systems
Not unit-tested. Manual playtest validates IO behavior and integration.

### Manual playtest exit-criteria

- [ ] Game launches with no `save.ron` → fresh world; `info: no save file found, starting fresh`.
- [ ] Mine + smelt + buy a tool. Press F5 → `save.ron` appears; `info: game saved`.
- [ ] Open `save.ron` in a text editor — content is human-readable, version field = 1, recognizable inventory/money/smelter sections.
- [ ] Restart → world loads with the saved Grid (tunnels persist), inventory, money, owned tools, player position. `info: save loaded`.
- [ ] F9 mid-session after digging more → world reverts to the F5 state.
- [ ] Save while smelter is mid-process. Quit. Restart. Smelter resumes with the same time_left, queue, and output.
- [ ] Save with bars in the smelter output (uncollected). Load. Output buffer survives; Collect All works.
- [ ] Auto-save on quit: close window via X. Restart → state matches the moment of close.
- [ ] Hand-corrupt `save.ron` (delete a closing brace). Restart → `error: save load failed`; fresh world. No crash.
- [ ] Hand-edit `save.ron` to bump `version` to 999. Restart → `warn: save version 999, expected 1 — ignoring`; fresh world.
- [ ] Delete `save.ron` between sessions → fresh world.
- [ ] HUD bar count, ore count, money count all show correctly after load. Tab popup tools section shows correct active/owned/locked.
- [ ] No crashes over a 15-minute mixed save/load session.

### Explicitly not tested
- Cross-platform path conventions (we use `./save.ron` deliberately).
- Backwards compatibility with prior save versions (no migration logic).
- Save during a panel transition / mid-frame edge cases (auto-save fires once on AppExit).

## Open questions deferred to implementation planning

- Whether `Grid` exposes its inner state via field visibility or via a small `GridRepr` shim for serde. Aesthetic choice; both work.
- Whether `apply()` should also write a Bevy event (`SaveLoaded`) for downstream systems to observe. Not needed for M3.5; useful for M4 if peers want to know "the world just changed under me." Skip for now.
- Whether to expose `SAVE_PATH` as a configurable Resource for tests / dev override. Default `./save.ron` const is fine. **Decision for this milestone: tests stay purely in-memory** (round-trip via `serialize_ron`/`deserialize_ron` strings), so no `tempfile` dev-dep is needed. File IO is exercised only by manual playtest.
- `ron 0.8`'s `serialize_ron` returns `ron::Error` while `deserialize_ron`'s parse arm holds a `ron::error::SpannedError`. That's the official 0.8 API shape — different types for ser vs de — and both wrap into our `LoadError` cleanly. Implementer should not be surprised.

---

**Spec end.** Implementation plan to follow via the writing-plans skill once this spec is approved.
