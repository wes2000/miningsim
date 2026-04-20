# Milestone 5b — Multiplayer Foundation Rework — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix M4's two pre-existing networking bugs that M5a smoke #3 exposed (full-Grid replication over unreliable UDP; missing client→host Transform sync), then re-land M5a's reverted Task 10 (belt multiplayer).

**Architecture:** Three-part sequence, each with its own smoke checkpoint. Part 1 replaces `.replicate::<Grid>()` with reliable `GridSnapshot` (initial) + `TileChanged` (delta) server events. Part 2 adds client-authoritative Transform sync via a periodic `ClientPositionUpdate` client event + a client-side `AuthoritativeTransform` stash that drops inbound Transform replication for LocalPlayer. Part 3 cherry-picks the reverted Task 10 commit (`a74b100`) on the repaired foundation.

**Tech Stack:** Rust 1.x; Bevy 0.15; `bevy_replicon = "0.32"` (0.32.2 installed); `bevy_replicon_renet = "0.9"`; `serde` + `bincode` + `ron`.

**Spec:** `docs/superpowers/specs/2026-04-20-milestone-5b-multiplayer-foundation-design.md`

**Baseline at plan start:** `main` is at commit `33adb80` (Merge milestone-5a). 124 tests pass. Single-player belts work end-to-end. Multiplayer is broken per the spec's problem statement.

---

## Task 0: Branch + baseline check

No commit. Just confirms the starting state.

- [ ] **Step 1: Branch from main**

```bash
cd c:/Users/whann/Desktop/Games/miningsim
git checkout main
git checkout -b milestone-5b
```

- [ ] **Step 2: Confirm tests pass on baseline**

```bash
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: `Total passing: 124`.

- [ ] **Step 3: Confirm single-player still works before we touch anything**

(Manual controller check — skip if confident from the recent M5a merge. If in doubt: `cargo run` → dig a hole, place a belt, observe item transport. Close cleanly.)

- [ ] **Step 4: Confirm clean working tree (modulo untracked logs)**

```bash
git status --short
```

Expected: either empty or only untracked `*.log` / `save.ron` / `run - *.bat` files from earlier sessions. No staged or unstaged modifications to tracked files.

---

## Part 1 — Grid delta replication

**Design (from spec):** replace full-component Grid replication with explicit delta events on the reliable/ordered channel. Initial state is sent as a one-shot `GridSnapshot` event per client connection; subsequent mutations fire `TileChanged` events broadcast to all clients.

Parts 1 and 2 are independent — Part 2 doesn't depend on Part 1's events existing. They're sequenced for testability: each landed separately makes one symptom go away.

## Task 1: Add `GridSnapshot` + `TileChanged` events (TDD)

**Files:**
- Modify: `src/systems/net_events.rs`
- Modify: `tests/net_events.rs`

After this task: both event structs exist with `Event` + `Serialize`/`Deserialize` derives, pass bincode round-trip tests. Not yet registered or emitted anywhere.

- [ ] **Step 1: Write failing tests first**

Append to `tests/net_events.rs` (before any trailing `}` or EOF):

```rust
// ---------- M5b server events (Grid delta replication) ----------

use miningsim::grid::{Grid, Layer, Tile};
use miningsim::systems::net_events::{GridSnapshot, TileChanged};

#[test]
fn tile_changed_round_trips() {
    let original = TileChanged {
        pos: IVec2::new(12, 40),
        tile: Tile {
            solid: false,
            layer: Layer::Stone,
            ore: Some(OreKind::Copper),
            damage: 2,
        },
    };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: TileChanged = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}

#[test]
fn grid_snapshot_round_trips() {
    // Construct a tiny non-default grid. We only need to prove the field shape
    // round-trips; Grid's own serde is already covered by tests/save.rs.
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: false, layer: Layer::Dirt, ore: None, damage: 0 });
    let original = GridSnapshot { grid: g };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: GridSnapshot = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded.grid.get(1, 1).map(|t| t.solid), Some(false));
    assert_eq!(decoded.grid.get(0, 0).map(|t| t.solid), Some(true));
    assert_eq!(decoded.grid.width(), 3);
    assert_eq!(decoded.grid.height(), 3);
}
```

Note: `use` statements go just below the existing `use miningsim::tools::Tool;` line (or at top of file) — place them alphabetically. The existing file imports `use bevy::math::IVec2;` and `use miningsim::items::OreKind;` already — reuse.

- [ ] **Step 2: Run tests — expect compile failure**

```bash
cargo test --test net_events 2>&1 | tail -10
```

Expected: `unresolved imports miningsim::systems::net_events::GridSnapshot, miningsim::systems::net_events::TileChanged`.

- [ ] **Step 3: Add events to `src/systems/net_events.rs`**

At the bottom of `src/systems/net_events.rs`, append:

```rust
// ---------- Server events (server → client) added in M5b ----------

/// Server → one specific client. Fired once per client connection, carrying
/// the full Grid. Replicon's ordered channel handles reliable delivery +
/// transparent fragmentation, so the ~80 KB payload reaches the client
/// intact. After this, the client tracks Grid via `TileChanged` deltas.
#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct GridSnapshot {
    pub grid: crate::grid::Grid,
}

/// Server → all clients. Broadcast after every successful tile mutation
/// (dig: damage or break). Ordered so reordering of two updates to the
/// same tile doesn't cause visual flicker (e.g., damage=1 arriving after
/// damage=2).
#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TileChanged {
    pub pos: IVec2,
    pub tile: crate::grid::Tile,
}
```

- [ ] **Step 4: Run tests — expect 7 + 2 = 9 passing for net_events**

```bash
cargo test --test net_events 2>&1 | tail -10
```

Expected: `test result: ok. 9 passed`.

- [ ] **Step 5: Full regression**

```bash
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: `Total passing: 126` (124 baseline + 2 new).

- [ ] **Step 6: Commit**

```bash
git add src/systems/net_events.rs tests/net_events.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add GridSnapshot + TileChanged server events with serde round-trip tests"
```

---

## Task 2: Register server events + host-side emitters

**Files:**
- Modify: `src/systems/net_plugin.rs`
- Modify: `src/systems/player.rs`

After this task: host registers both new events with replicon and fires them in the right places. Client-side handlers don't exist yet (next task), so clients receive events but drop them. `.replicate::<Grid>()` is STILL active (we remove it only after the client-side handlers work, in Task 4).

- [ ] **Step 1: Register server events in `MultiplayerPlugin::build`**

Open `src/systems/net_plugin.rs`. At the top, update the `net_events` import to pull in the new events:

```rust
use crate::systems::net_events::{
    BuyToolRequest, CollectAllRequest, DigRequest, GridSnapshot, SellAllRequest,
    SmeltAllRequest, TileChanged,
};
```

(Keep alphabetical order within the braces.)

In `MultiplayerPlugin::build`, after the existing `app.add_client_event::<SellAllRequest>(Channel::Ordered);` line (and before the server-handler `.add_systems` block), add:

```rust
// M5b server events — host-authoritative Grid sync.
app.add_server_event::<GridSnapshot>(Channel::Ordered);
app.add_server_event::<TileChanged>(Channel::Ordered);
```

- [ ] **Step 2: Add host-side observer `send_initial_grid_snapshot`**

Still in `net_plugin.rs`, at the end of the file, append:

```rust
/// Server observer: when a new client connects (replicon spawns an entity
/// with `ConnectedClient`), send them the full current Grid via a one-shot
/// `GridSnapshot` event. Runs only on the server (ConnectedClient entities
/// only exist server-side).
pub fn send_initial_grid_snapshot(
    trigger: Trigger<OnAdd, ConnectedClient>,
    grid_q: Query<&Grid>,
    mut writer: EventWriter<ToClients<GridSnapshot>>,
) {
    let client_entity = trigger.entity();
    let Ok(grid) = grid_q.get_single() else {
        warn!("send_initial_grid_snapshot: Grid singleton missing; client {client_entity} will see no terrain");
        return;
    };
    info!("sending initial grid snapshot to client {client_entity} ({}x{})", grid.width(), grid.height());
    writer.send(ToClients {
        mode: SendMode::Direct(client_entity),
        event: GridSnapshot { grid: grid.clone() },
    });
}
```

Note the imports already cover `ToClients` and `SendMode` if you add them — in Bevy 0.15 replicon 0.32 they live in the prelude re-export. Verify: the top of `net_plugin.rs` already has `use bevy_replicon::prelude::*;` which re-exports both.

- [ ] **Step 3: Register the observer in `MultiplayerPlugin::build`**

In `MultiplayerPlugin::build`, find the block with the existing observers:

```rust
app.add_observer(net_player::spawn_player_for_new_clients);
app.add_observer(net_player::despawn_player_for_disconnected_clients);
```

Immediately below those, add:

```rust
app.add_observer(send_initial_grid_snapshot);
```

(It lives in `net_plugin.rs` itself, not `net_player.rs` — hence the unqualified name.)

- [ ] **Step 4: Modify `handle_dig_requests` to broadcast `TileChanged`**

In `net_plugin.rs`, find `handle_dig_requests`. Add a new parameter:

```rust
pub fn handle_dig_requests(
    mut events: EventReader<FromClient<DigRequest>>,
    grid: Single<&mut Grid>,
    mut commands: Commands,
    player_q: Query<(Entity, &OwningClient, &Transform, &OwnedTools), With<Player>>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
    mut tile_writer: EventWriter<ToClients<TileChanged>>,   // NEW
) {
```

Inside the per-event loop, after the existing `if matches!(result.status, DigStatus::Broken | DigStatus::Damaged) { ... }` block that marks the chunk dirty (around the point where we already know the dig succeeded), add this block just BEFORE that one (we need to read the tile state post-mutation):

```rust
if matches!(result.status, DigStatus::Broken | DigStatus::Damaged) {
    // Broadcast the new tile state so all clients update their local Grid.
    if let Some(new_tile) = grid.get(event.target.x, event.target.y).copied() {
        tile_writer.send(ToClients {
            mode: SendMode::Broadcast,
            event: TileChanged { pos: event.target, tile: new_tile },
        });
    }
    // ... existing chunk-dirty marking stays below ...
}
```

Actual placement: find the existing `if matches!(result.status, DigStatus::Broken | DigStatus::Damaged) {` block and insert the `tile_writer.send(...)` at its top (before the existing chunk-dirty work). It only adds two lines inside that existing block — no structural change.

Reference final shape of that block:

```rust
if matches!(result.status, DigStatus::Broken | DigStatus::Damaged) {
    // NEW: broadcast the new tile state to clients.
    if let Some(new_tile) = grid.get(event.target.x, event.target.y).copied() {
        tile_writer.send(ToClients {
            mode: SendMode::Broadcast,
            event: TileChanged { pos: event.target, tile: new_tile },
        });
    }
    // Existing: mark the owning chunk dirty so chunk_render rebuilds the mesh.
    let chunk_coord = IVec2::new(
        event.target.x.div_euclid(CHUNK_TILES),
        event.target.y.div_euclid(CHUNK_TILES),
    );
    for (e, c) in chunks_q.iter() {
        if c.coord == chunk_coord {
            commands.entity(e).insert(ChunkDirty);
            break;
        }
    }
}
```

- [ ] **Step 5: Modify `dig_input_system` (host-local branch) to broadcast `TileChanged`**

Open `src/systems/player.rs`. Find `dig_input_system`. Its signature already takes `net_mode: Res<crate::net::NetMode>` and `mut dig_writer: EventWriter<DigRequest>`. Add a new parameter:

```rust
pub fn dig_input_system(
    // ... existing params ...
    net_mode: Res<crate::net::NetMode>,
    mut dig_writer: EventWriter<DigRequest>,
    mut tile_writer: EventWriter<bevy_replicon::prelude::ToClients<crate::systems::net_events::TileChanged>>,  // NEW
) {
```

Find the branch that runs on SinglePlayer and Host (i.e., after the `if matches!(*net_mode, crate::net::NetMode::Client { .. }) { ... return; }` early-out). In that branch, find the existing code that calls `dig::try_dig(&mut grid, target_tile, tool)` and handles `DigStatus::Broken | DigStatus::Damaged` (marks chunks dirty, etc).

Immediately after the `try_dig` call, if the status is `Broken` or `Damaged` AND we're in `NetMode::Host`, broadcast the tile change:

```rust
if matches!(result.status, DigStatus::Broken | DigStatus::Damaged) {
    // NEW: in Host mode, tell all clients about this tile change.
    // (SinglePlayer skips — no clients to notify.)
    if matches!(*net_mode, crate::net::NetMode::Host { .. }) {
        if let Some(new_tile) = grid.get(target_tile.x, target_tile.y).copied() {
            tile_writer.send(bevy_replicon::prelude::ToClients {
                mode: bevy_replicon::prelude::SendMode::Broadcast,
                event: crate::systems::net_events::TileChanged { pos: target_tile, tile: new_tile },
            });
        }
    }
    // ... existing cooldown.reset() + chunk-dirty marking stays below ...
}
```

**IMPORTANT — Single-player panic risk.** `EventWriter<ToClients<TileChanged>>` as a system param requires `Events<ToClients<TileChanged>>` to exist as a resource. That resource is only inserted by `MultiplayerPlugin::build` (which registers the event). In `NetMode::SinglePlayer`, `MultiplayerPlugin` is NOT loaded, so the resource is missing and Bevy will panic at schedule build or first-system-run.

This is the same constraint that made belt_ui.rs's `EventWriter<PlaceBeltRequest>` safe only in Host/Client modes (MultiplayerPlugin loads in both). We have three options:
  (a) Register `Events<ToClients<TileChanged>>` manually in all modes via `app.add_event::<ToClients<TileChanged>>()` at MiningSimPlugin top-level. Then the resource exists in SP too; the writer just never has anyone to deliver to.
  (b) Split `dig_input_system` into two systems — one SP/Client, one Host-only — with the Host one owning the EventWriter and registered only in MultiplayerPlugin.
  (c) Move the host-emission path into `handle_dig_requests` by having the host route its own digs through DigRequest events. Big refactor; out of scope.

**Pick (a)** — one line, matches how we've accepted similar patterns in belt_ui.rs (which works because MultiplayerPlugin always loads when Host/Client mode's NetMode branch is hit; SP just never takes the EventWriter path because `matches!(*net_mode, NetMode::Host)` returns false before the writer is touched).

Wait — rechecking (a). `EventWriter<T>` in Bevy requires `Events<T>` even if never called, because `SystemParam::get_param` accesses the resource at system-execution time. If the resource is missing, it panics.

To be robust in SP, we must ensure `Events<ToClients<TileChanged>>` exists in all modes. Add to `src/app.rs::MiningSimPlugin::build`, near the top of the `build` fn:

```rust
// Ensure the Events resources for ToClients<T> exist in every NetMode so
// systems that declare `EventWriter<ToClients<T>>` as a param don't panic
// in SinglePlayer (where MultiplayerPlugin isn't loaded). Registering a
// duplicate add_event is a no-op in Bevy; no harm when MultiplayerPlugin
// later calls add_server_event (which internally also calls add_event).
app.add_event::<bevy_replicon::prelude::ToClients<crate::systems::net_events::TileChanged>>();
```

(Also add `ToClients<PlaceBeltRequest>` / `ToClients<RemoveBeltRequest>` when Task 10 re-lands — but those are server-event wrappers not used as EventWriters; the client events already have their own Events<T> registration.)

Actually wait — `PlaceBeltRequest` is a client event, not a server event, so its EventWriter is `EventWriter<PlaceBeltRequest>` not `EventWriter<ToClients<PlaceBeltRequest>>`. Client events' `Events<T>` resource is registered by replicon's `add_client_event` OR by us via `add_event` — need to audit.

Quick truth: `add_client_event::<T>` internally calls `add_event::<T>` for T, so the client's EventWriter<T> works. Similarly `add_server_event::<T>` adds `Events<ToClients<T>>`. In SP we skip both, so BOTH resources are missing, but belt_ui.rs only takes `EventWriter<PlaceBeltRequest>` (not `ToClients<...>`). So the belt case needs `add_event::<PlaceBeltRequest>()` to be safe in SP, and similarly for `RemoveBeltRequest`.

**Actually** — revisit M5a Task 10 (reverted). In the reverted state, belt_ui.rs was modified to take `EventWriter<PlaceBeltRequest>` which would panic in SP. Did SP work in M5a?

It did — smoke #2 passed after Task 10 landed (the user ran `cargo run` which is SP). So either:
  (i) Bevy tolerates missing Events<T> when the system never actually sends,
  (ii) `add_client_event` was called even in SP (unlikely — MultiplayerPlugin isn't loaded in SP), or
  (iii) The SP path early-returned before accessing the EventWriter.

Looking at the M5a code: `belt_place_system` takes `EventWriter<PlaceBeltRequest>` as a direct param. Bevy's param resolution for `EventWriter<T>` accesses `ResMut<Events<T>>` — which panics at extraction time if missing. So (i) is false; (iii) is irrelevant (param extraction happens before the function body).

Hmm. Yet SP worked. That means `add_event::<PlaceBeltRequest>` was called somewhere even in SP. Looking at the code: no explicit `add_event` for it, and `add_client_event` is only in MultiplayerPlugin. So it should have panicked…

Possible explanation: Bevy's `EventWriter` might be lenient — check the actual error mode empirically. For this plan, prescribe the belt-and-suspenders approach:

At the top of `MiningSimPlugin::build`, register the Events<T> for the `ToClients<TileChanged>` wrapper so SP is safe. This is defensive and costs one line.

```rust
// In src/app.rs::MiningSimPlugin::build, very top:
app.add_event::<bevy_replicon::prelude::ToClients<crate::systems::net_events::TileChanged>>();
```

Do this in the SAME step as modifying `dig_input_system` so the change is atomic.

- [ ] **Step 6: Add defensive `add_event` registration in `MiningSimPlugin::build`**

In `src/app.rs`, inside `MiningSimPlugin::build`, at the very top (before the `add_systems(Startup, ...)` call):

```rust
// Ensure Events<ToClients<TileChanged>> exists in every NetMode.
// MultiplayerPlugin also registers this via `add_server_event`, but registering
// twice is a no-op. In SinglePlayer we need the resource for `dig_input_system`'s
// `EventWriter<ToClients<TileChanged>>` param even though no one is listening.
app.add_event::<bevy_replicon::prelude::ToClients<crate::systems::net_events::TileChanged>>();
```

- [ ] **Step 7: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: build clean, 126 passing.

- [ ] **Step 8: Quick single-player smoke**

```bash
cargo run
```

Walk around, dig a hole. Should behave identically to before. Close the window.

(We're not smoke-testing multiplayer yet — that's the end-of-Part-1 checkpoint, after Task 4.)

- [ ] **Step 9: Commit**

```bash
git add src/systems/net_plugin.rs src/systems/player.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Host-side: register GridSnapshot/TileChanged server events + fire on connect/dig"
```

---

## Task 3: Add client-side `apply_grid_snapshot` + `apply_tile_changed`

**Files:**
- Modify: `src/systems/net_player.rs`
- Modify: `src/systems/net_plugin.rs`

After this task: clients apply the incoming snapshot on connect and subsequent tile deltas. `.replicate::<Grid>()` is still active (two sources of truth for Grid updates — will drop in Task 4).

- [ ] **Step 1: Add `apply_grid_snapshot` in `net_player.rs`**

At the bottom of `net_player.rs`, append:

```rust
/// Client-side: receives the one-shot `GridSnapshot` sent by the host on
/// connection. Spawns the Grid singleton entity locally and marks every
/// existing TerrainChunk dirty so `chunk_render` rebuilds meshes from the
/// newly-available grid. Replaces any prior Grid singleton defensively
/// (shouldn't happen on the first snapshot, but handles the weird case
/// where a second snapshot arrives).
pub fn apply_grid_snapshot(
    mut commands: Commands,
    mut events: EventReader<crate::systems::net_events::GridSnapshot>,
    existing_grid: Query<Entity, With<crate::grid::Grid>>,
    chunks: Query<Entity, With<TerrainChunk>>,
) {
    for event in events.read() {
        info!("applying grid snapshot ({}x{})", event.grid.width(), event.grid.height());
        // Defensive: remove any existing Grid entity first.
        for e in existing_grid.iter() {
            commands.entity(e).despawn();
        }
        // Spawn fresh Grid singleton. No Replicated marker — client-local.
        commands.spawn(event.grid.clone());
        // Dirty every chunk so they re-mesh on the next chunk_render pass.
        for chunk in chunks.iter() {
            commands.entity(chunk).insert(ChunkDirty);
        }
    }
}
```

- [ ] **Step 2: Add `apply_tile_changed` in `net_player.rs`**

Append after `apply_grid_snapshot`:

```rust
/// Client-side: applies a single-tile delta from the host. Early-returns if
/// the Grid singleton doesn't exist yet (pre-snapshot race window); any lost
/// pre-snapshot events are already reflected in the snapshot that's arriving.
pub fn apply_tile_changed(
    mut commands: Commands,
    mut events: EventReader<crate::systems::net_events::TileChanged>,
    mut grid_q: Query<&mut crate::grid::Grid>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
) {
    let Ok(mut grid) = grid_q.get_single_mut() else {
        // No Grid yet — drain and drop. Snapshot will supersede.
        events.clear();
        return;
    };
    for event in events.read() {
        grid.set(event.pos.x, event.pos.y, event.tile);
        // Dirty only the owning chunk.
        let chunk_coord = IVec2::new(
            event.pos.x.div_euclid(crate::systems::chunk_lifecycle::CHUNK_TILES),
            event.pos.y.div_euclid(crate::systems::chunk_lifecycle::CHUNK_TILES),
        );
        for (e, c) in chunks_q.iter() {
            if c.coord == chunk_coord {
                commands.entity(e).insert(ChunkDirty);
                break;
            }
        }
    }
}
```

- [ ] **Step 3: Register both systems in `MultiplayerPlugin::build`**

In `net_plugin.rs`, find the existing block that registers `add_shop_visuals_on_arrival` / `add_smelter_visuals_on_arrival` / `add_ore_drop_visuals_on_arrival`. That block isn't gated on `client_connected` because the `Without<Sprite>` filter naturally no-ops on host.

For our new systems, we explicitly want them gated on `client_connected` — they apply replicated events which only fire on clients.

Add below the existing arrival-visual block, a new `add_systems` block:

```rust
// M5b: client-side grid sync from server events.
app.add_systems(
    Update,
    (
        net_player::apply_grid_snapshot,
        net_player::apply_tile_changed,
    )
        .run_if(client_connected),
);
```

Ordering: `apply_grid_snapshot` should logically come before `apply_tile_changed` within the same tick so a snapshot-then-delta sequence applies in order. Bevy's tuple ordering isn't guaranteed unless we chain. Use `.chain()`:

```rust
app.add_systems(
    Update,
    (
        net_player::apply_grid_snapshot,
        net_player::apply_tile_changed,
    )
        .chain()
        .run_if(client_connected),
);
```

- [ ] **Step 4: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: build clean, 126 passing (unchanged — no new tests, no regressions).

- [ ] **Step 5: Commit**

```bash
git add src/systems/net_player.rs src/systems/net_plugin.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Client-side: apply_grid_snapshot + apply_tile_changed event handlers"
```

---

## Task 4: Drop `.replicate::<Grid>()` + delete `mark_chunks_dirty_on_grid_change`

**Files:**
- Modify: `src/systems/net_plugin.rs`
- Modify: `src/systems/net_player.rs`

After this task: Part 1 is complete. Grid replication now flows ONLY through `GridSnapshot` + `TileChanged` events. The old unreliable full-snapshot path is gone.

- [ ] **Step 1: Remove `.replicate::<Grid>()` from the replication chain**

In `src/systems/net_plugin.rs`, find the `app.replicate::<...>()` chain in `MultiplayerPlugin::build`:

```rust
app.replicate::<Player>()
    .replicate::<NetOwner>()
    // ...
    .replicate::<Grid>()          // <-- REMOVE THIS LINE
    // ...
```

Delete the `.replicate::<Grid>()` line entirely. The chain continues with its other components.

- [ ] **Step 2: Delete `mark_chunks_dirty_on_grid_change` system function**

In `src/systems/net_player.rs`, find the `mark_chunks_dirty_on_grid_change` function. Delete the entire function body, including its doc comment and the `// SCALING:` block. Clean out any now-unused imports (`Ref<Grid>` in particular — verify whether anything else uses `Ref` first).

- [ ] **Step 3: Delete the registration in `net_plugin.rs`**

In `MultiplayerPlugin::build`, find and delete:

```rust
// When the singleton Grid changes via replication, re-mesh chunks.
// ... doc comment block ...
// SCALING: ...
app.add_systems(
    Update,
    net_player::mark_chunks_dirty_on_grid_change.run_if(client_connected),
);
```

Delete the whole `add_systems` block including its preceding comments.

- [ ] **Step 4: Update the `net_player.rs` module-level doc comment**

The module doc at the top of `net_player.rs` lists `mark_chunks_dirty_on_grid_change` in its Responsibilities list. Remove that bullet point.

- [ ] **Step 5: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: build clean (any `use crate::grid::Grid;` that's now unused in `net_player.rs` may trigger a warning — fix if the compiler complains, but don't pre-emptively delete; just react to what the build says). 126 passing.

- [ ] **Step 6: Single-player smoke**

```bash
cargo run
```

Dig a hole, confirm it visibly breaks. Mine an ore, pick it up. Save (F5), load (F9), confirm the hole persists. Close.

This verifies SP still works — the Grid replication paths were wrapped in MultiplayerPlugin-only code, but double-check.

- [ ] **Step 7: Commit**

```bash
git add src/systems/net_plugin.rs src/systems/net_player.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Drop .replicate::<Grid>() in favor of GridSnapshot + TileChanged events"
```

---

## Smoke-test checkpoint #A (after Task 4)

Human controller:

```
# Terminal A:
cargo run -- host

# Terminal B:
cargo run -- join 127.0.0.1:5000
```

Expected:

- Both windows open. Both players see their own LocalPlayer sprite (blue).
- Both windows see the OTHER player... well, maybe not yet — that's Part 2. For now, the joining client might see the host's sprite frozen at spawn (since client→host Transform still isn't synced). That's fine for this checkpoint.
- **Primary test:** Host digs a hole in their window. **Client sees the hole appear on their screen within ~1 frame.** This is the core Part 1 success condition.
- Client digs a hole directly adjacent to their (stuck-at-spawn, on host view) position — target should be within reach of the spawn tile. **Host window sees the hole appear.**
- Host log should NO LONGER show "Sending an unreliable message with 53 fragments" spam. If it does, something's still replicating Grid as a component — investigate before proceeding.
- Close cleanly (close host window; client should exit).

If digs don't cross the network in either direction, surface to controller. Do NOT continue to Part 2 with a broken Part 1.

---

## Part 2 — Client-authoritative movement

**Design (from spec):** client periodically sends its authoritative position via `ClientPositionUpdate` event. Host writes it onto the server-side Player's Transform (which then replicates to other clients via the existing `.replicate::<Transform>()`). Client ignores inbound Transform replications for its own LocalPlayer by restoring from a local `AuthoritativeTransform` stash each frame.

## Task 5: Add `ClientPositionUpdate` event + serde test + handler (TDD)

**Files:**
- Modify: `src/systems/net_events.rs`
- Modify: `tests/net_events.rs`
- Modify: `src/systems/net_plugin.rs`

After this task: event exists and is registered. Host handler writes incoming positions to the server-side Player. Client doesn't emit yet (Task 7 adds the emitter).

- [ ] **Step 1: Write failing test**

Append to `tests/net_events.rs`:

```rust
use miningsim::systems::net_events::ClientPositionUpdate;

#[test]
fn client_position_update_round_trips() {
    let original = ClientPositionUpdate {
        pos: bevy::math::Vec2::new(123.5, -47.25),
        facing: IVec2::new(1, 0),
    };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: ClientPositionUpdate = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}
```

Note: `Vec2` isn't imported yet in the test file — add `use bevy::math::Vec2;` near the other bevy imports, OR use the fully qualified path as shown above.

- [ ] **Step 2: Run test — expect compile failure**

```bash
cargo test --test net_events 2>&1 | tail -10
```

Expected: `unresolved import miningsim::systems::net_events::ClientPositionUpdate`.

- [ ] **Step 3: Add event in `net_events.rs`**

Append after the existing events (e.g., after `TileChanged`):

```rust
// ---------- Client events (client → server) added in M5b ----------

/// Client → server. Fired at `POSITION_SYNC_HZ` (see `net_player.rs`) to
/// keep the server's view of this client's player position current. Used
/// for dig-reach validation and replication to OTHER clients via
/// `.replicate::<Transform>()`. Unordered because later packets should
/// supersede earlier ones — we want newest position to win.
#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClientPositionUpdate {
    pub pos: bevy::math::Vec2,
    pub facing: IVec2,
}
```

- [ ] **Step 4: Run test — expect 10 passing for net_events**

```bash
cargo test --test net_events 2>&1 | tail -10
```

Expected: `test result: ok. 10 passed`.

- [ ] **Step 5: Register client event in `MultiplayerPlugin::build`**

In `src/systems/net_plugin.rs`, update the `net_events` import to include the new event:

```rust
use crate::systems::net_events::{
    BuyToolRequest, ClientPositionUpdate, CollectAllRequest, DigRequest, GridSnapshot,
    SellAllRequest, SmeltAllRequest, TileChanged,
};
```

Add after the existing `add_client_event` calls:

```rust
app.add_client_event::<ClientPositionUpdate>(Channel::Unordered);
```

- [ ] **Step 6: Add host-side handler `handle_client_position_updates`**

Append to `net_plugin.rs`:

```rust
/// Server-side: write the client's reported position onto its server-side
/// Player Transform. Trust-based — we don't validate or speed-cap. Mutation
/// triggers replicon's change detection, which broadcasts Transform updates
/// to all OTHER clients via the existing `.replicate::<Transform>()`.
pub fn handle_client_position_updates(
    mut events: EventReader<FromClient<ClientPositionUpdate>>,
    player_q: Query<(Entity, &OwningClient), With<Player>>,
    mut xf_q: Query<(&mut Transform, &mut crate::components::Facing), With<Player>>,
) {
    for FromClient { client_entity, event } in events.read() {
        let Some(e) = player_entity_for_client(*client_entity, &player_q) else { continue };
        let Ok((mut xf, mut facing)) = xf_q.get_mut(e) else { continue };
        xf.translation.x = event.pos.x;
        xf.translation.y = event.pos.y;
        // Don't touch z; it was set at spawn and drives sprite layering.
        facing.0 = event.facing;
    }
}
```

- [ ] **Step 7: Register handler in the `server_running` tuple**

Find the existing handler tuple in `MultiplayerPlugin::build`:

```rust
app.add_systems(
    Update,
    (
        handle_dig_requests,
        handle_buy_tool_requests,
        handle_smelt_all_requests,
        handle_collect_all_requests,
        handle_sell_all_requests,
    )
        .run_if(server_running),
);
```

Add `handle_client_position_updates` as the last item:

```rust
(
    handle_dig_requests,
    handle_buy_tool_requests,
    handle_smelt_all_requests,
    handle_collect_all_requests,
    handle_sell_all_requests,
    handle_client_position_updates,
)
```

- [ ] **Step 8: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: build clean, 127 passing (126 + 1 new).

- [ ] **Step 9: Commit**

```bash
git add src/systems/net_events.rs tests/net_events.rs src/systems/net_plugin.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add ClientPositionUpdate event + server handler (client→host position sync)"
```

---

## Task 6: Add `AuthoritativeTransform` component; maintain it from `apply_velocity_system`

**Files:**
- Modify: `src/components.rs`
- Modify: `src/systems/net_player.rs`
- Modify: `src/systems/player.rs`

After this task: LocalPlayer carries `AuthoritativeTransform(Vec3)` that the client's movement path keeps in sync with the authoritative (local) Transform. Nothing reads it yet — Task 8 adds the reader system.

- [ ] **Step 1: Add the component definition in `components.rs`**

In `src/components.rs`, alongside other Player-adjacent marker/data components (e.g., near `LocalPlayer` / `Velocity`), add:

```rust
/// Client-local authoritative Transform stash for LocalPlayer. Never
/// replicated. The client's `apply_velocity_system` writes to this
/// whenever it updates the player's Transform; `restore_local_transform_from_authoritative`
/// (in net_player.rs) reads it to overwrite any inbound Transform replication
/// for the LocalPlayer entity. Effectively makes Transform client-authoritative
/// for self, while keeping server-authoritative for remote players.
#[derive(Component, Debug, Clone, Copy)]
pub struct AuthoritativeTransform(pub bevy::math::Vec3);
```

- [ ] **Step 2: Attach `AuthoritativeTransform` in `mark_local_player_on_arrival`**

In `src/systems/net_player.rs`, find `mark_local_player_on_arrival`. In the `is_local` branch, add `AuthoritativeTransform` to the insert tuple:

```rust
if is_local {
    ec.insert((
        LocalPlayer,
        Velocity::default(),
        Facing::default(),
        AuthoritativeTransform(Vec3::ZERO),   // NEW - initialized to ZERO; apply_velocity_system catches it up
        Sprite {
            color: LOCAL_PLAYER_COLOR,
            custom_size: Some(Vec2::splat(PLAYER_SPRITE_SIZE)),
            ..default()
        },
    ));
}
```

Import `AuthoritativeTransform` at the top (add to the existing `use crate::components::{...}`).

**Initialization race note:** We insert with `Vec3::ZERO`. The very next frame, `apply_velocity_system` (Step 4 below) writes the Transform's actual position into `AuthoritativeTransform`. For exactly one frame between tagging and first apply, `AuthoritativeTransform` = ZERO, which would teleport the client's player to (0,0,0) if `restore_local_transform_from_authoritative` ran in that window. Solution: `restore_local_transform_from_authoritative` does NOT fire if `AuthoritativeTransform == Vec3::ZERO` (defensive), OR we initialize it to the Transform's current value on insertion. Pick the latter — cleaner, avoids magic sentinel.

Refactor: replace `AuthoritativeTransform(Vec3::ZERO)` with reading the current Transform. We need a query for the player's Transform in `mark_local_player_on_arrival`. But we're inserting components into the same entity we're iterating; doing both a read and a write inside one system is awkward but possible via `Query<(Entity, &Transform, &NetOwner), ...>`.

Modify the query:

```rust
pub fn mark_local_player_on_arrival(
    mut commands: Commands,
    local_id: Option<Res<LocalClientId>>,
    arriving: Query<
        (Entity, &NetOwner, &Transform),
        (With<Player>, Without<LocalPlayer>, Without<RemotePlayer>),
    >,
) {
```

Use the Transform in the insert:

```rust
for (entity, owner, xf) in &arriving {
    let is_local = owner.0 == local_id.0;
    let mut ec = commands.entity(entity);
    if is_local {
        ec.insert((
            LocalPlayer,
            Velocity::default(),
            Facing::default(),
            AuthoritativeTransform(xf.translation),   // seed with current server-provided spawn pos
            Sprite { ... },
        ));
    } else {
        ec.insert((
            RemotePlayer,
            Sprite { ... },
        ));
    }
}
```

- [ ] **Step 3: Update `apply_velocity_system` to write `AuthoritativeTransform` for LocalPlayer**

In `src/systems/player.rs`, find `apply_velocity_system`:

```rust
pub fn apply_velocity_system(
    time: Res<Time>,
    mut q: Query<(&Velocity, &mut Transform), With<Player>>,
) {
    // ...
}
```

Change the query to also pick up an optional `AuthoritativeTransform`:

```rust
pub fn apply_velocity_system(
    time: Res<Time>,
    mut q: Query<(&Velocity, &mut Transform, Option<&mut crate::components::AuthoritativeTransform>), With<Player>>,
) {
    let dt = time.delta_secs();
    for (v, mut t, auth) in q.iter_mut() {
        t.translation.x += v.0.x * dt;
        t.translation.y += v.0.y * dt;
        if let Some(mut auth) = auth {
            auth.0 = t.translation;
        }
    }
}
```

This way: entities with `AuthoritativeTransform` (i.e., LocalPlayer on client, nobody else) get their stash kept in lockstep with Transform. Host entities lack the component → no-op.

**Collision correction subtlety:** `collide_player_with_grid_system` (in player.rs) also modifies Transform after apply_velocity_system runs. It queries `With<LocalPlayer>`, so it only touches the client's own player. For `AuthoritativeTransform` to stay accurate, collision corrections must also propagate. Add the same pattern:

In `collide_player_with_grid_system`, change the query:

```rust
pub fn collide_player_with_grid_system(
    grid: Option<Single<&Grid>>,
    mut q: Query<(&mut Transform, Option<&mut crate::components::AuthoritativeTransform>), With<LocalPlayer>>,
) {
    // ... existing guards ...
    let Ok((mut t, auth)) = q.get_single_mut() else { return };

    // ... existing collision resolution that mutates `t.translation` ...

    // At the end of the function (after all collision axes are resolved):
    if let Some(mut auth) = auth {
        auth.0 = t.translation;
    }
}
```

Actually cleaner: since we're destructuring, just write `auth` at the end of the function. Don't intermix collision and auth writes — do collision on `t`, then sync `auth` once at the very end.

- [ ] **Step 4: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: 127 passing (no behavior change visible yet — AuthoritativeTransform is written but not read).

- [ ] **Step 5: Single-player smoke**

```bash
cargo run
```

Walk around. Collision with walls should feel identical. Close.

- [ ] **Step 6: Commit**

```bash
git add src/components.rs src/systems/net_player.rs src/systems/player.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add AuthoritativeTransform component; sync from apply_velocity + collide systems"
```

---

## Task 7: Add `send_local_position_system` + `LocalPositionSyncTimer`

**Files:**
- Modify: `src/systems/net_player.rs`
- Modify: `src/systems/net_plugin.rs`

After this task: client fires `ClientPositionUpdate` events at 10 Hz. Host now has a live, accurate view of the client's position. Server-side dig reach check becomes authoritative (was already written in Task 5; now it has real data to work with).

- [ ] **Step 1: Add constant + resource in `net_player.rs`**

Near the top of `net_player.rs`, below the existing constants (`LOCAL_PLAYER_COLOR`, etc.):

```rust
/// How often the client ships its authoritative position to the host.
/// 10 Hz = one packet every 100 ms. At max player speed (120 px/s) that's
/// ~12 px = <1 tile of position staleness on the host side — well within
/// DIG_REACH_TILES = 2.0's slack.
pub const POSITION_SYNC_HZ: f32 = 10.0;

/// Timer driving `send_local_position_system`. Inserted unconditionally in
/// `MultiplayerPlugin::build`; the system itself no-ops in non-Client modes.
#[derive(Resource)]
pub struct LocalPositionSyncTimer(pub Timer);

impl Default for LocalPositionSyncTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0 / POSITION_SYNC_HZ, TimerMode::Repeating))
    }
}
```

- [ ] **Step 2: Add `send_local_position_system`**

At the bottom of `net_player.rs`:

```rust
/// Client-side: every `POSITION_SYNC_HZ` ticks, ship our LocalPlayer's
/// Transform + Facing to the host via `ClientPositionUpdate`. Gated on
/// `NetMode::Client` internally rather than via run_if so the system
/// exists in the schedule in Host mode too (where it no-ops) — avoids
/// the registration divergence between modes.
pub fn send_local_position_system(
    time: Res<Time>,
    mut timer: ResMut<LocalPositionSyncTimer>,
    net_mode: Res<crate::net::NetMode>,
    player_q: Option<Single<(&Transform, &Facing), With<LocalPlayer>>>,
    mut writer: EventWriter<crate::systems::net_events::ClientPositionUpdate>,
) {
    if !matches!(*net_mode, crate::net::NetMode::Client { .. }) {
        return;
    }
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    let Some(p) = player_q else { return }; // LocalPlayer not tagged yet
    let (xf, facing) = p.into_inner();
    writer.send(crate::systems::net_events::ClientPositionUpdate {
        pos: xf.translation.truncate(),
        facing: facing.0,
    });
}
```

- [ ] **Step 3: Insert resource + register system in `MultiplayerPlugin::build`**

In `net_plugin.rs`, at the start of `MultiplayerPlugin::build` (after the `app.add_plugins(...)` calls for replicon), insert the timer:

```rust
app.insert_resource(net_player::LocalPositionSyncTimer::default());
```

Then add a new `add_systems` block for the sender:

```rust
// M5b: client-authoritative position sync.
app.add_systems(Update, net_player::send_local_position_system);
```

(No `.run_if(...)` — the internal `NetMode::Client` check handles gating.)

- [ ] **Step 4: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: 127 passing.

- [ ] **Step 5: Commit**

```bash
git add src/systems/net_player.rs src/systems/net_plugin.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add send_local_position_system: 10 Hz ClientPositionUpdate from client to host"
```

---

## Task 8: Add `restore_local_transform_from_authoritative`

**Files:**
- Modify: `src/systems/net_player.rs`
- Modify: `src/systems/net_plugin.rs`

After this task: Part 2 complete. Client's LocalPlayer never teleports back to the host's (stale) view of its position. Client movement feels responsive; host sees client's real position via the ClientPositionUpdate stream; other clients see both players move via the existing `.replicate::<Transform>()` (which on the host side ships Transform updates that `handle_client_position_updates` just wrote).

- [ ] **Step 1: Add `restore_local_transform_from_authoritative`**

At the bottom of `net_player.rs`:

```rust
/// Client-side: after replicon applies replicated Transform updates in
/// PreUpdate, restore the LocalPlayer's Transform from `AuthoritativeTransform`
/// so inbound server-origin updates for our own player don't clobber our
/// client-authoritative position. Runs on the Update schedule — by then,
/// PreUpdate (including `ClientSet::Receive`) is complete.
///
/// Ordering within Update: this must run BEFORE any system that reads
/// Transform for gameplay (dig_input_system, camera_follow_system, etc.).
/// Easiest way is to run it at the start of Update in its own SystemSet
/// ordered before InputSet::ReadInput. Alternatively, place it in
/// InputSet::ReadInput itself — it only touches LocalPlayer Transform, so
/// it's conceptually part of "input prep."
pub fn restore_local_transform_from_authoritative(
    mut q: Query<(&mut Transform, &crate::components::AuthoritativeTransform), With<LocalPlayer>>,
) {
    let Ok((mut xf, auth)) = q.get_single_mut() else { return };
    // Only write if there's an actual divergence. Bevy's change detection
    // means an unconditional write would dirty Transform every frame,
    // which could affect other Changed<Transform> filters elsewhere.
    if xf.translation != auth.0 {
        xf.translation = auth.0;
    }
}
```

- [ ] **Step 2: Register the system in `MultiplayerPlugin::build`**

Pick scheduling that runs in Update BEFORE any gameplay system reads Transform. The existing `InputSet::ReadInput` set is the earliest gameplay set. Place the restore system there too — it's morally part of input prep.

In `net_plugin.rs`, add to `MultiplayerPlugin::build`:

```rust
app.add_systems(
    Update,
    net_player::restore_local_transform_from_authoritative
        .in_set(crate::app::InputSet::ReadInput),
);
```

Within `InputSet::ReadInput`, tuple order isn't guaranteed across different `add_systems` calls, but our system doesn't depend on ordering with `read_input_system` — it just needs to run before `apply_velocity_system` (which is in `InputSet::ApplyInput`, the next set in the chain). That ordering is guaranteed by the `configure_sets(Update, (...).chain())` call in `app.rs`.

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: 127 passing.

- [ ] **Step 4: Single-player smoke**

```bash
cargo run
```

`restore_local_transform_from_authoritative` should be inert on host (LocalPlayer hasn't got AuthoritativeTransform — that's only added in `mark_local_player_on_arrival`, which is client-only). Walk around, make sure nothing feels weird.

Hmm — wait. On the HOST, `setup_world` spawns the host's player with `LocalPlayer` but without `AuthoritativeTransform`. So `restore_local_transform_from_authoritative`'s query (which requires both) doesn't match → system returns without touching. Good. On the HOST's own LocalPlayer, the Transform stays authoritative-from-input as before, and `AuthoritativeTransform` is simply absent. Correct behavior.

For SinglePlayer (same as Host without replicon), same story — system does nothing. Good.

- [ ] **Step 5: Commit**

```bash
git add src/systems/net_player.rs src/systems/net_plugin.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add restore_local_transform_from_authoritative: ignore inbound Transform for LocalPlayer"
```

---

## Smoke-test checkpoint #B (after Task 8)

Human controller, two-window:

```
# Terminal A:
cargo run -- host

# Terminal B:
cargo run -- join 127.0.0.1:5000
```

Expected:

- Both windows open and show both players as colored sprites (local blue, remote orange).
- Player A (host) walks around. Player B sees A's sprite move smoothly across B's screen.
- Player B (client) walks around. Player A sees B's sprite move smoothly across A's screen. (Position updates stream at 10 Hz, so motion on the other screen may look slightly less silky than local — acceptable at 100 ms granularity.)
- Client walks 10 tiles from spawn. Client attempts to dig an adjacent tile (space or left-click). **Client sees the tile break.** (Authoritative reach check on host now uses the client's real position, not spawn.)
- Client tries to dig a tile 5+ tiles from their position (e.g., click far away with mouse). Should silently reject (reach check enforces).
- Host digs → both see it (regression check for Part 1).
- Host log no longer spams fragment warnings (regression check for Part 1).
- Close host; client exits cleanly (M4 behavior preserved).

If any of movement, dig-beyond-spawn, or mutual visibility fails, surface to controller before proceeding to Task 9.

---

## Part 3 — Re-land Task 10 (belt multiplayer)

## Task 9: Cherry-pick Task 10 commit (or manual re-apply on conflict)

**Primary approach: cherry-pick.** Commit `a74b100` is the original Task 10. Its diff is pure addition (replication + handlers + client-side visual attachment + `can_place_belt` helper). The only file that Task 9 above also modifies is `net_plugin.rs` (imports, handler registrations); replicon-event registrations may conflict.

- [ ] **Step 1: Attempt cherry-pick**

```bash
git cherry-pick a74b100
```

Two outcomes:
- **Clean:** proceed to Step 3.
- **Conflict:** proceed to Step 2.

- [ ] **Step 2 (only if conflict): Resolve manually**

Likely conflict points in `net_plugin.rs`:

1. **Import block** — Task 10 added `PlaceBeltRequest, RemoveBeltRequest` to the `net_events` import. Our Task 5 already added `ClientPositionUpdate`. Combine both sets alphabetically:

```rust
use crate::systems::net_events::{
    BuyToolRequest, ClientPositionUpdate, CollectAllRequest, DigRequest, GridSnapshot,
    PlaceBeltRequest, RemoveBeltRequest, SellAllRequest, SmeltAllRequest, TileChanged,
};
```

2. **`add_client_event` block** — Task 10 adds two new ones; keep both:

```rust
app.add_client_event::<DigRequest>(Channel::Ordered);
app.add_client_event::<BuyToolRequest>(Channel::Ordered);
app.add_client_event::<SmeltAllRequest>(Channel::Ordered);
app.add_client_event::<CollectAllRequest>(Channel::Ordered);
app.add_client_event::<SellAllRequest>(Channel::Ordered);
app.add_client_event::<ClientPositionUpdate>(Channel::Unordered);
app.add_client_event::<PlaceBeltRequest>(Channel::Ordered);
app.add_client_event::<RemoveBeltRequest>(Channel::Ordered);
```

3. **Server handler tuple** — Task 10 adds `handle_place_belt_requests` and `handle_remove_belt_requests`. Combine with our Task 5's addition:

```rust
(
    handle_dig_requests,
    handle_buy_tool_requests,
    handle_smelt_all_requests,
    handle_collect_all_requests,
    handle_sell_all_requests,
    handle_client_position_updates,
    handle_place_belt_requests,
    handle_remove_belt_requests,
)
```

4. **Client-side arrival tuple** — Task 10 adds `net_player::add_belt_visuals_on_arrival`. Combine:

```rust
(
    net_player::add_shop_visuals_on_arrival,
    net_player::add_smelter_visuals_on_arrival,
    net_player::add_ore_drop_visuals_on_arrival,
    net_player::add_belt_visuals_on_arrival,
)
```

5. **Replication chain** — Task 10 adds `.replicate::<BeltTile>()`. We previously removed `.replicate::<Grid>()` in Task 4, so the chain is now missing Grid but should gain BeltTile. After resolution:

```rust
app.replicate::<Player>()
    .replicate::<NetOwner>()
    .replicate::<Shop>()
    .replicate::<Smelter>()
    .replicate::<SmelterState>()
    .replicate::<OreDrop>()
    .replicate::<Money>()
    .replicate::<Inventory>()
    .replicate::<OwnedTools>()
    .replicate::<BeltTile>()
    .replicate::<Transform>();
```

Run `git cherry-pick --continue` after resolving. If conflicts are messier than expected, abort and use the fallback:

```bash
git cherry-pick --abort
git show a74b100 > /tmp/task10.patch
# Hand-apply relevant hunks from /tmp/task10.patch, following the spec's
# "Cherry-pick fallback" note. Commit with: -m "Multiplayer: replicate BeltTile, server handlers, client branching, visual attachment"
```

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -20
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: build clean, **129 passing** (127 end-of-Task-8 + 2 re-landed `can_place_belt` tests from Task 10).

If the cherry-pick also brought back the 2 `can_place_belt` tests in `tests/belt.rs` (which it should), the count is 127 + 2 = 129.

- [ ] **Step 4: Quick regression check — single-player**

```bash
cargo run
```

Buy Belt Networks, place some belts, drop ore on one, watch it advance. All M5a SP behaviors intact.

- [ ] **Step 5: Commit**

If the cherry-pick succeeded cleanly, `git cherry-pick` already produced a commit (with the original commit message). If you resolved conflicts manually with `--continue`, that also produces a commit. If you took the fallback path (abort + hand-apply), create the commit:

```bash
git add -A
git commit --author="wes2000 <whannasch@gmail.com>" -m "Multiplayer: replicate BeltTile, server handlers, client branching, visual attachment (cherry-pick-fallback of a74b100)"
```

---

## Smoke-test checkpoint #C (after Task 9)

Full M5b exit-criteria smoke. Human controller, two-window:

```
# Terminal A:
cargo run -- host

# Terminal B:
cargo run -- join 127.0.0.1:5000
```

Checklist (all must pass to continue to Task 10):

- [ ] Both windows show both players (local blue, remote orange) and both move smoothly across screens.
- [ ] Host digs → client sees break within ~1 frame. Client digs (near them, anywhere) → host sees break within ~1 frame.
- [ ] Client walks 10+ tiles from spawn, digs an adjacent tile → succeeds. Dig-beyond-reach (far mouse-click target) silently rejects.
- [ ] Both players independently buy "Belt Networks" from the shop (200c each). Each player's Inventory UI shows only their own purchase.
- [ ] Player A presses B, scrolls to pick direction, left-clicks to place a belt. Player B sees that belt appear within ~1 frame.
- [ ] Player B walks to the belt, right-clicks it → belt disappears on both screens. If an item was on it, `OreDrop` spawns at the belt's former tile on both screens.
- [ ] Player A digs ore onto a belt they placed. Item advances one tile per second on both screens.
- [ ] Item reaches a belt adjacent to the smelter (pointing in) → smelter pulls it (queue increments on both screens). ~2 sec later smelter pushes a bar onto an adjacent out-pointing belt. Bar advances and spills at the end. Either player can vacuum it up.
- [ ] Host disconnects (close window) → client exits cleanly (AppExit::Success path).
- [ ] Reboot host standalone (`cargo run`), single-player: F5 saves, F9 restores world with belts + mined tiles intact. Matches M5a behavior.

If ANY item fails, surface to controller. Do not proceed to merge.

---

## Task 10: Roadmap + merge to main

**Files:**
- Modify: `docs/roadmap.md`

- [ ] **Step 1: Append M5b playtest results to `docs/roadmap.md`**

Add a new `## Playtest Results — Milestone 5b (YYYY-MM-DD)` section just before the `## What This Document Is Not` footer. Content outline (fill in from actual smoke #C results):

```markdown
## Playtest Results — Milestone 5b (YYYY-MM-DD)

Multiplayer foundation rework — fixed the two pre-existing M4 networking
bugs surfaced by M5a smoke #3, then re-landed M5a Task 10 (belt multiplayer)
on the repaired foundation.

**Grid delta replication.** Replaced `.replicate::<Grid>()` with
`GridSnapshot` (one-shot on client connect) + `TileChanged` (broadcast on
every dig mutation) server events, both on the reliable/ordered channel.
Per-dig bandwidth dropped from ~80 KB on unreliable UDP to ~16 B on
reliable ordered — a 5,000x reduction at zero cost to correctness. The
"53 fragments" warning from M5a smoke #3 is gone; clients now see each
other's terrain changes within one frame.

**Client-authoritative Transform.** Added `ClientPositionUpdate` client
event (10 Hz, unordered/reliable) so the host has a live view of each
connected client's position. Added `AuthoritativeTransform` client-local
component + `restore_local_transform_from_authoritative` system so the
client ignores inbound Transform replications for its own LocalPlayer.
Result: client movement feels responsive, host's dig-reach check is
accurate to within ~100 ms, remote peers see each other's movement via
the existing `.replicate::<Transform>()` flow.

**Belt MP re-landed.** Task 10's reverted commit (`a74b100`) cherry-picked
cleanly onto the repaired base. No changes needed to Task 10's design —
it was correct in isolation all along.

Test count: **129 passing** (124 post-M5a + 3 new serde round-trip + 2
re-landed `can_place_belt`).

**What felt good:**
- Delta replication is the right shape for mutation-heavy state. The
  same GridSnapshot + TileChanged pair generalizes naturally to any
  future grid-like data (machine networks, pipe layouts, etc.).
- Client-authoritative Transform with trust model matches M4's stated
  "friends, not strangers" posture. No anti-cheat machinery needed.
- AuthoritativeTransform + restore-in-ReadInput pattern is ~30 lines
  of client-side code and entirely self-contained. No replicon
  visibility API needed.

**What we deliberately deferred:**
- Client-side prediction / reconciliation (option 2b from brainstorm).
  Not needed at current feel quality; revisit only if competitive PvP
  becomes a goal.
- Anti-cheat on ClientPositionUpdate (speed-cap, rate-limit, plausibility).
  Unnecessary for friends; re-evaluate when a real lobby ships.
- Grid resize beyond 80×200. Delta replication scales, but the chunk
  system and camera work need their own pass. M6+ scope.

**Decisions for M5c+:**
- **Per-tile delta events are the default from here on.** When adding
  any new grid-like state (machine graphs, pipe networks), register it
  as `Initial<T>Snapshot` + `<T>Changed` server events. Don't default
  to `.replicate::<T>()` for bulk data.
- **ClientPositionUpdate's 10 Hz rate is tuned for current movement
  speed.** If player speed ever increases significantly, bump the rate.
  Re-evaluate at M6 (vehicles) and M7 (competitive multiplayer elements
  if added).
- **The `AuthoritativeTransform` pattern generalizes.** Any future
  client-owned replicated component (custom hats, held items, etc.)
  can use the same stash + restore approach if bidirectional sync is
  needed.
```

Replace `YYYY-MM-DD` with the actual completion date.

- [ ] **Step 2: Commit roadmap update**

```bash
git add docs/roadmap.md
git commit --author="wes2000 <whannasch@gmail.com>" -m "Roadmap: M5b playtest results (multiplayer foundation reworked)"
```

- [ ] **Step 3: Final test sweep**

```bash
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
```

Expected: 129 passing.

- [ ] **Step 4: Merge milestone-5b → main**

```bash
git checkout main
git merge --no-ff milestone-5b -m "Merge milestone-5b: multiplayer foundation rework"
```

Verify:

```bash
cargo test 2>&1 | grep "test result" | awk '{s += $4} END {print "Total passing:", s}'
git log --oneline main -3
```

Expected: 129 passing on main, merge commit visible in log.

- [ ] **Step 5: Push**

```bash
git push origin main
```

- [ ] **Step 6: Close milestone-5b (optional)**

Once the merge is verified on origin, the milestone-5b branch can be deleted locally + remotely, or kept for reference. Default: delete local, keep remote until you're sure nothing needs backtracking.

```bash
# Optional — only after confirming main works
git branch -d milestone-5b
```

---

## Summary

**10 tasks + 3 smoke checkpoints + 1 merge.**

- Tasks 0–4: Part 1 (Grid delta replication). End state: clients see each other's digs.
- Tasks 5–8: Part 2 (client-authoritative Transform). End state: clients see each other move; server reach check is accurate.
- Task 9: Part 3 (Task 10 re-land via cherry-pick). End state: belts work in MP.
- Task 10: Roadmap + merge.

**Test count progression:** 124 → 126 (Task 1) → 127 (Task 5) → 129 (Task 9).

**What this plan does NOT cover:**
- Changes to `SAVE_VERSION` (stays at 3).
- Changes to the `NetMode` enum.
- Any new gameplay features.
- Anti-cheat / rate-limiting / speed-caps.
- Client-side prediction.
- Grid delta compression / visibility culling.
- Any existing test modifications (all 124 baseline tests stay as-is).
