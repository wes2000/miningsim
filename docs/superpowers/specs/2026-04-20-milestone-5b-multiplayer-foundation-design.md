# Milestone 5b — Multiplayer Foundation Rework — Design Spec

**Date:** 2026-04-20
**Status:** Design complete; awaiting plan
**Predecessors:**
- Milestone 4 (co-op networking) — `docs/superpowers/specs/2026-04-18-milestone-4-co-op-networking-design.md`
- Milestone 5a (conveyor belts) — `docs/superpowers/specs/2026-04-19-milestone-5a-conveyor-belts-design.md`

---

## Goal

Fix the two pre-existing M4 networking bugs that M5a's smoke #3 exposed, then re-land M5a's Task 10 (belt multiplayer) on that repaired foundation.

After M5b, two-window co-op actually works end-to-end: both players see each other move, both see terrain changes as they happen, both place/use belts together.

M5b is pure infrastructure work — no new gameplay verbs. The payoff is that every future networked feature (M5c pallets/forklifts, M5d robots, M6 property system) stops being blocked on the same two bugs.

## Exit criteria

Two-window co-op (`cargo run -- host` + `cargo run -- join 127.0.0.1:5000`) passes smoke test #3 with the checklist below:

- **Visibility.** Host window shows the joining player's sprite (orange) moving smoothly. Client window shows the host's sprite (orange) moving smoothly. Each window shows itself as blue.
- **Grid sync.** Host digs → client sees the tile break within ~1 frame. Client digs → host sees the tile break within ~1 frame. Dig changes persist when the camera scrolls away and back.
- **Reach validation.** Client walks 10 tiles from spawn, tries to dig an adjacent tile → succeeds. Client tries to dig a tile 5 tiles away from their body → fails with no visible response (server-side reach check is authoritative).
- **Belt multiplayer.** Both players can buy "Belt Networks" independently. Client places a belt → host sees it. Host right-clicks it → despawns on both. Ore placed on a belt by either player advances and delivers on both screens. Bar pushed by the smelter onto an output belt visible to both.
- **Single-player unchanged.** F5 saves and F9 restores world state including belts, mined tiles, smelter contents. All 124 current tests still pass; +3 new serde tests + 2 re-landed unit tests land the count at **129 passing**.

In-scope failure modes we accept:
- Up to ~100 ms latency for a client to see their own player's position confirmed back from the server (client-side movement is authoritative and responsive, so this lag is invisible unless you're comparing pixel-level positions between windows).
- A misbehaving client can teleport its own player (trust model is "friends, not strangers" — unchanged from M4).

## Scope

| In scope | Out of scope (deferred) |
|----------|------------------------|
| Grid delta replication via `GridSnapshot` + `TileChanged` server events | Grid larger than 80×200 (M6 scope) |
| Client-authoritative Transform via `ClientPositionUpdate` client event | Client-side prediction / reconciliation (option 2b; deferred indefinitely) |
| Re-land Task 10 as-is (cherry-pick `a74b100`) | New belt-MP features beyond what Task 10 shipped |
| 3 new serde round-trip tests | Bandwidth metrics / logging |
| Smoke-test #3 reworked with expanded checklist | Anti-cheat / speed-cap / rate-limit on position events |
| Drop `.replicate::<Grid>()` and `mark_chunks_dirty_on_grid_change` | Replicon visibility API filtering (option ii; we use a local stash instead) |

## Architecture

M5b touches three independent concerns in sequence. Each can be smoke-tested before the next lands.

### Part 1 — Grid delta replication

**The problem (from M5a smoke #3):** `.replicate::<Grid>()` ships the full 80 KB Grid component snapshot on every single `try_dig` mutation. This message fragments into ~53 UDP packets; at typical dig cadence (0.15 s cooldown) the unreliable channel drops enough fragments that clients never reassemble a consistent grid. Client never sees any dig — host or its own.

**The fix:** replace full-component replication with explicit delta events on the reliable/ordered channel.

#### New events in `src/systems/net_events.rs`

```rust
/// Server → one specific client. Fired once when a client connects,
/// carrying the full Grid. After this, the client tracks Grid via
/// TileChanged deltas. Uses Channel::Ordered so replicon's reliable
/// delivery handles the ~80 KB fragmentation transparently.
#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct GridSnapshot {
    pub grid: Grid,
}

/// Server → all clients. Fired after every successful tile mutation
/// (dig: damage or break). Ordered so ordering is stable when a client
/// has high packet loss (wrong-order applies of the same tile would
/// visually flicker).
#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TileChanged {
    pub pos: IVec2,
    pub tile: Tile,
}
```

Both registered via `add_server_event::<T>(Channel::Ordered)` in `MultiplayerPlugin::build`. `Grid` and `Tile` already have `Serialize`/`Deserialize`.

#### Host-side changes

- **New system `send_initial_grid_snapshot` (server observer on `OnAdd, ConnectedClient`).** Reads the Grid singleton and fires `ToClients { mode: SendMode::Direct(client_entity), event: GridSnapshot { grid: grid.clone() } }`. Registered alongside `spawn_player_for_new_clients`; runs server-side.

- **`handle_dig_requests` (existing, in `net_plugin.rs`).** After every `try_dig` call that returns `Broken` or `Damaged`, emit a broadcast event:
  ```rust
  writer.send(ToClients {
      mode: SendMode::Broadcast,
      event: TileChanged { pos: event.target, tile: grid.get(event.target.x, event.target.y).copied().unwrap() },
  });
  ```
  Added as a new `EventWriter<ToClients<TileChanged>>` parameter to the handler.

- **`dig_input_system` (host-local path, in `player.rs`).** After every successful `try_dig` call in the non-Client branch, emit the same broadcast so remote clients see the host's own digs. Conditional on `NetMode::Host { .. }` — skip in SinglePlayer.

- **Remove `.replicate::<Grid>()`** from the `MultiplayerPlugin::build` replicate chain.

- **Delete `net_player::mark_chunks_dirty_on_grid_change`** and its registration. Per-tile events carry the chunk coordinate implicitly; no global Grid-is-changed path is needed or useful.

#### Client-side changes

- **New system `apply_grid_snapshot` in `net_player.rs`.** Reads `EventReader<GridSnapshot>`. On receipt:
  1. `commands.spawn(event.grid.clone())` — Grid becomes a singleton component on a fresh entity (no `Replicated` marker; never replicates back).
  2. Iterate the existing `TerrainChunk` entities via a `Query<Entity, With<TerrainChunk>>` parameter and insert `ChunkDirty` on each so the next `chunk_render` pass rebuilds meshes from the new grid.
  3. If a Grid singleton already exists (shouldn't on first snapshot, but defensive), despawn the old one before step 1.

- **New system `apply_tile_changed` in `net_player.rs`.** Reads `EventReader<TileChanged>`. For each event:
  1. If the Grid singleton doesn't exist yet (snapshot not applied), early-return without consuming (or drain the reader — see "Edge cases"). The snapshot will bring the client current; stray pre-snapshot mutations are already in the snapshot by the time it's sent.
  2. Write the tile into the Grid via `grid.set(pos.x, pos.y, tile)`.
  3. Compute the owning chunk coord (same math `handle_dig_requests` uses) and insert `ChunkDirty` on that chunk entity.

- Both systems gated on `client_connected` (same as other net_player client-side systems).

#### Edge cases

- **TileChanged arrives before GridSnapshot.** The ordered channel + the observer-fires-before-update-systems Bevy schedule ordering makes this unlikely, but not impossible across replicon's internal tick boundaries. Client's `apply_tile_changed` early-returns when `Grid` singleton doesn't exist; the `EventReader` still consumes the event (standard Bevy semantics). That's fine — the pre-snapshot mutations are already reflected in the snapshot the client is about to receive.

- **Host runs `apply_tile_changed` / `apply_grid_snapshot`.** Both are gated on `client_connected`, which is false on the host. Host never applies events to its own Grid (host already owns the authoritative Grid).

- **Initial chunk-dirty storm on snapshot.** Client marks every on-screen chunk dirty when the snapshot arrives. That causes a one-frame mesh rebuild burst. Acceptable — same work the old `.replicate::<Grid>()` initial sync caused, just routed differently.

### Part 2 — Client-authoritative movement

**The problem (from M5a smoke #3):** clients move their own `LocalPlayer` Transform locally. Replicon only replicates host → client for Transform, so the host's view of each client's Player stays frozen at the spawn tile. When a client tries to dig 7 tiles from spawn, `handle_dig_requests` sees `player_tile = spawn_tile` and the reach check (`dig_target_valid`, reach=2) silently rejects the dig.

**The fix:** client periodically ships its authoritative position to the host via a dedicated client event. Host writes it to the server-side Player Transform. `.replicate::<Transform>()` stays as-is, so other clients see this player via the normal host→client replication path.

This is *client-authoritative* — the client's reported position wins. Matches M4's documented "trust-based for friends" model. A misbehaving client can teleport; that's accepted for this game's audience.

#### New event in `net_events.rs`

```rust
/// Client → server. Fired at ~10 Hz (see `POSITION_SYNC_HZ` below)
/// whenever the client's LocalPlayer has a Transform/Facing. Uses
/// Channel::Unordered — we want packets to be reliable (so the host's
/// view doesn't drift to stale data) but we don't care if a mid-flight
/// packet arrives after a later one, because the later one will be
/// applied on top and become the current truth.
#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClientPositionUpdate {
    pub pos: Vec2,
    pub facing: IVec2,
}
```

Registered `add_client_event::<ClientPositionUpdate>(Channel::Unordered)` in `MultiplayerPlugin::build`.

#### Client-side changes

- **New resource `LocalPositionSyncTimer(Timer)` in `net_player.rs`**, 0.1 s repeating. `POSITION_SYNC_HZ = 10.0` named constant.

- **New system `send_local_position_system`** (gated internally on `NetMode::Client { .. }`; registered unconditionally in `MultiplayerPlugin`). Ticks timer, on finish sends `ClientPositionUpdate { pos, facing }` from the LocalPlayer's Transform + Facing. If LocalPlayer doesn't exist yet (pre-tagging), skip.

  Note: this uses a different gating pattern than Part 1's `apply_grid_snapshot`/`apply_tile_changed`, which use `.run_if(client_connected)` at registration. Here we want the system to exist in host mode too (so the schedule doesn't diverge between modes), but to do nothing when the host runs it — hence the inline `NetMode::Client` check. Both patterns are idiomatic in this codebase; the choice follows the existing convention in `exit_on_host_disconnect` (gated internally because it needs to check `Option<Res<RenetClient>>`, not a replicon condition).

- **New component `AuthoritativeTransform(Vec3)`** in `components.rs`. Not replicated. Attached to LocalPlayer during client-side tagging (`mark_local_player_on_arrival`).

- **Modify `apply_velocity_system`** (in `player.rs`): when moving a LocalPlayer, also write the new translation to `AuthoritativeTransform`. For non-LocalPlayer entities with `Velocity` (there are none today; host-side remote Players don't get `Velocity`), ignore. This is a narrow mutation — the query adds `Option<&mut AuthoritativeTransform>` and conditionally updates.

- **New system `restore_local_transform_from_authoritative`** in `net_player.rs`. Runs after replicon's replication apply (the `bevy_replicon` scheduling is documented; we'll use `.after(...)` targeting replicon's public schedule label, or place it in a set ordered after it). For each `(Transform, AuthoritativeTransform), With<LocalPlayer>`: if Transform differs from AuthoritativeTransform, write AuthoritativeTransform → Transform. Effectively drops any incoming Transform replication for the local player.

  Precise `.after()` target TBD during implementation — it must run after replicon writes replicated Transform but before rendering reads it. If no clean label exists, fall back to putting it in a late `Update` set ordered after `replicon::ClientSet`.

#### Host-side changes

- **New handler `handle_client_position_updates` in `net_plugin.rs`.** Registered in the existing `.run_if(server_running)` tuple. For each `FromClient { client_entity, event }`, look up the Player entity by `OwningClient`, then write `pos` into `Transform.translation.xy` and `facing` into `Facing`. `z` stays at its spawn value (`10.0`). Mutation triggers replicon's change detection → Transform ships to all other clients automatically.

- **No change to `handle_dig_requests`.** The reach check now sees a fresh `player_tile` (within ~100 ms of reality) and `DIG_REACH_TILES = 2.0` gives enough slack. Client digs that were silently rejected now succeed.

#### Edge cases

- **Client connects but LocalPlayer not yet tagged.** `send_local_position_system` skips if LocalPlayer missing. Host writes nothing; client's server-side Player Transform stays at spawn until the first sync fires. Acceptable — brief pre-input window.

- **Host's own player.** `send_local_position_system` checks `NetMode::Client` and returns early on host. Host's LocalPlayer Transform updates flow through the existing `apply_velocity_system` → `.replicate::<Transform>()` → client path. No change.

- **Client receives a Transform replication for itself anyway.** `restore_local_transform_from_authoritative` runs after and clobbers it. Client never visibly teleports.

- **Rapid input changes between 100 ms sync boundaries.** The host's view lags by up to 100 ms. Acceptable for this game. If `DIG_REACH_TILES` ever drops below ~1.5 this becomes too tight; at 2.0 a 100-ms lag at max 120 px/s = 12 px = <1 tile of slop.

- **Z coordinate / layering.** Host writes only `x, y` from the event; `z` stays at spawn. Remote Players on the client side receive full Transform via replication (including host-set z), which is correct for 2D sprite layering.

### Part 3 — Re-land Task 10 (belt multiplayer)

**The approach:** once Part 1 and Part 2 land and smoke-test clean, cherry-pick the reverted Task 10 commit:

```bash
git cherry-pick a74b100
```

Expected conflicts: `net_plugin.rs` will have collected new imports + handler registrations in Parts 1 and 2 that Task 10 also touched. Resolve by taking both sides of the imports + registering all handlers in the existing `.run_if(server_running)` tuple.

**What Task 10 restores:**
- `.replicate::<BeltTile>()` in the replication chain.
- `PlaceBeltRequest`/`RemoveBeltRequest` client events + their handlers.
- `belt_place_system` / `belt_remove_system` branch on `NetMode::Client` to fire events instead of mutating locally.
- `can_place_belt` pure helper in `src/belt.rs` + 2 unit tests in `tests/belt.rs`.
- `add_belt_visuals_on_arrival` client-side sprite attachment.

**No new design required** — the work was done; it just needed the foundation repaired beneath it.

**Cherry-pick fallback.** If `git cherry-pick a74b100` runs into non-trivial conflicts (e.g., one of Part 1/2's changes touched the same lines Task 10 touched in a way the three-way merge can't reconcile), abort the cherry-pick and reimplement Task 10 by hand from its diff (`git show a74b100`). The changes are small and self-contained — replication registration, 2 handlers, 1 client-side arrival system, a NetMode branch in belt_place/remove, and 2 tests — so manual re-apply is tractable. Either path should land the same final state; the commit message should note which path was used.

**Verify after cherry-pick:** `cargo test` → 129 passing (124 current + 3 new serde + 2 re-landed `can_place_belt`). Build clean. Single-player belts (smoke #1, #2) still pass.

## Sequencing

Three commits → three smoke checkpoints:

1. **Part 1: Grid delta.** Single-player still works (events + MultiplayerPlugin are inert in SP). Multiplayer smoke: both players dig → both see the changes within 1 frame. No belt MP yet.
2. **Part 2: Movement sync.** Multiplayer smoke: both players see each other move smoothly. Client digs 10 tiles from spawn → succeeds. No belt MP yet.
3. **Part 3: Task 10 re-land.** Full smoke #3 checklist — visibility + grid sync + reach validation + belt MP + single-player unchanged.

Each smoke checkpoint is a controller gate — do not proceed to the next part if the current one doesn't pass.

## Testing

| Layer | Count | Where |
|-------|-------|-------|
| Serde round-trip — `GridSnapshot` | 1 | `tests/net_events.rs` |
| Serde round-trip — `TileChanged` | 1 | `tests/net_events.rs` |
| Serde round-trip — `ClientPositionUpdate` | 1 | `tests/net_events.rs` |
| Re-landed `can_place_belt` unit tests | 2 | `tests/belt.rs` |
| **Total new / restored** | **5** | |

**No new integration tests** — the cross-system behavior (event registration, replicon wiring, observer scheduling) has no natural unit-test surface in this codebase; we verify it via the three smoke checkpoints. This matches M4's testing philosophy (`docs/roadmap.md` M4 playtest notes: "Pure-data tests cover the authoritative gameplay logic — replicon just routes who calls them").

## Files touched

| Path | Parts | Change |
|------|-------|--------|
| `src/systems/net_events.rs` | 1, 2 | +3 event structs |
| `src/systems/net_plugin.rs` | 1, 2, 3 | Drop `.replicate::<Grid>()`; add 3 event registrations, 3 new handlers; (re-land) add `BeltTile` replication + belt handlers |
| `src/systems/net_player.rs` | 1, 2, 3 | New systems: `send_initial_grid_snapshot`, `apply_grid_snapshot`, `apply_tile_changed`, `send_local_position_system`, `restore_local_transform_from_authoritative`; (re-land) `add_belt_visuals_on_arrival` |
| `src/systems/player.rs` | 1, 2 | `dig_input_system` fires `TileChanged` broadcast on host; `apply_velocity_system` updates `AuthoritativeTransform` for LocalPlayer |
| `src/components.rs` | 2 | +`AuthoritativeTransform` component |
| `src/systems/belt_ui.rs` | 3 (re-land) | NetMode branch in place/remove; `can_place_belt` integration |
| `src/belt.rs` | 3 (re-land) | `can_place_belt` pure helper |
| `tests/net_events.rs` | 1, 2 | +3 serde round-trip tests |
| `tests/belt.rs` | 3 (re-land) | +2 `can_place_belt` tests |
| `docs/roadmap.md` | end | +M5b playtest results section |

## What this does NOT change

- `SAVE_VERSION` stays at 3. Save format is unchanged.
- `NetMode` enum is unchanged.
- `handle_dig_requests`'s validation logic (reach, tool-tier) is unchanged; only its input (the Player Transform it reads) becomes accurate.
- All existing 124 tests keep passing without modification.
- Single-player behavior is bit-for-bit identical (no MultiplayerPlugin paths touch SP code).

## Known future work surfaced but not done

- **Grid resize to >80×200** — possible once deltas land, but gated on rendering/camera/chunk-system work unrelated to networking. M6 scope.
- **Anti-cheat on `ClientPositionUpdate`** — speed-cap, rate-limit, plausibility check. Unnecessary for friends-only; re-evaluate when we ship a real lobby.
- **Client-side prediction** — the 100 ms server-auth-movement option we explicitly rejected. Revisit only if competitive/PvP gameplay becomes a goal.
- **`Replicated` marker inventory** — `belt_place_system` spawns belts with `Replicated` even in SP (M5a choice; inert without replicon). After Part 3 this becomes active again, so no code change needed, but document the convention in CLAUDE.md when the file stabilizes.
