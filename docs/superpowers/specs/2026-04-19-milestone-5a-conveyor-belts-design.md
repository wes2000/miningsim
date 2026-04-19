# Milestone 5a — Conveyor Belts MVP — Design Spec

**Date:** 2026-04-19
**Status:** Design complete; awaiting plan
**Predecessors:** Milestone 4 (co-op networking) — `docs/superpowers/specs/2026-04-18-milestone-4-co-op-networking-design.md`

---

## Goal

Add **conveyor belts** as the foundational gameplay verb of M5 (Factory Automation). After M5a, a player can build an unattended ore-to-bar production line: drop raw ore on a belt, walk away, find bars at the other end.

This is the *first* of several M5 sub-milestones. M5b (multi-stage recipes + a second machine), M5c (pallets/forklifts), and M5d (warehouse robots) build on this foundation and are out of scope here.

## Exit criteria

A player can:

1. Buy "Belt Networks" from the shop (one-time unlock, 200c).
2. Press B to enter build mode, scroll-wheel to rotate the ghost cursor through 4 cardinal directions, left-click to place a belt, right-click to remove.
3. Drop ore on a belt tile (by digging adjacent to it, or by carrying ore over and tossing it on — see M4's existing `OreDrop` mechanic).
4. Watch the ore advance one tile per second along the belt.
5. See the smelter pull from any adjacent input belt (belt pointing INTO the smelter), process the ore over 2 seconds (existing rate), and push the resulting bar onto any adjacent output belt (belt pointing AWAY from the smelter).
6. Vacuum the bar where the belt ends and walk it to the shop to sell.
7. Save (F5) and load (F9) the world; belts and items-on-belts persist.

In multiplayer (host + 1 client):
- Both players can buy "Belt Networks" independently (per-player `OwnedTools`).
- Both players see each other's placed belts within ~1 frame of placement.
- Belt items advance on the host (authoritative); state replicates to client.
- Trust-based: either player can place or remove any belt regardless of who built it. Items on a removed belt spill as floor `OreDrop` sprites at that location.

## Scope

| In scope | Out of scope (deferred to M5b+) |
|----------|--------------------------------|
| Single-direction straight belts (4 cardinals) | Belt mergers / splitters / multi-input topology |
| L-corner rendering (auto-detected from neighbors) | Multi-belt-wide layouts |
| Hard-slot tiles (1 item/tile, 1 tile/sec) | Continuous-position smooth glide |
| Smelter ↔ belt I/O via "direction-of-belt" rule | Configurable per-side machine ports |
| Spillage off dead-end belts as `OreDrop` | Stockpile/chest entities as buffers |
| Build-mode UI (B keybind, scroll wheel, click) | Drag-to-paint multi-tile placement |
| One-time 200c shop unlock; free placement | Per-tile placement cost |
| Save/load (SAVE_VERSION 2 → 3) | Save/load in multiplayer (matches existing M4 deferral) |
| Multiplayer (host authoritative, replicated belts) | Multi-stage recipes, second machine type |
| ~17 new tests (~118 total) | Underground belts; belts in dig tunnels |

## Architecture

### Pure module: `src/belt.rs`

Pure data + functions. No Bevy systems. Unit-tested.

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeltTile {
    pub item: Option<crate::items::ItemKind>,
    pub dir: BeltDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BeltDir { North, East, South, West }

impl BeltDir {
    pub fn delta(self) -> bevy::math::IVec2 { /* (0,1)|(1,0)|(0,-1)|(-1,0) */ }
    pub fn opposite(self) -> BeltDir { /* ... */ }
    pub fn rotate_cw(self) -> BeltDir { /* N→E→S→W→N */ }
}

pub fn next_tile(pos: IVec2, dir: BeltDir) -> IVec2 { pos + dir.delta() }
```

`BeltVisual` is a separate Component on the same entity that the renderer uses to pick a sprite (Straight or one of four corner kinds). It's a pure derivation of the belt's own direction + the directions of any adjacent belts:

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeltVisual {
    Straight,           // arrow in self.dir
    CornerNE, CornerNW, CornerSE, CornerSW,  // L-bend
}

pub fn belt_visual_kind(self_dir: BeltDir, feeder_dir: Option<BeltDir>) -> BeltVisual {
    // If a feeder belt points into this tile from a side perpendicular to self.dir,
    // render a corner. Otherwise, render straight.
    // E.g., self_dir = East, feeder_dir = Some(South) → CornerSE (came up, going right).
}
```

**Variant order on `BeltDir` is load-bearing** (drives BTreeMap iteration in any future indexed structure). Documented inline.

### Bevy systems module: `src/systems/belt.rs`

```rust
// Run order each tick (1 second granularity, host-side only):
//   1. belt_pickup_system     — OreDrop floor sprites → belt slots
//   2. belt_tick_system       — items advance one tile (back-pressure)
//   3. smelter_belt_io_system — pull from input belts, push to output belts
//   4. smelter_tick_system    — (existing) process queue, produce bars
//   5. belt_spillage_system   — items at dead-end belts → OreDrop floor sprites
```

All five systems run with `.run_if(server_running)` in multiplayer; in single-player they run unconditionally.

**`belt_tick_system` algorithm:**
- Build a map of `IVec2 → (Entity, BeltTile)` for all belt tiles this frame.
- Two-pass to avoid order-dependence:
  - Pass 1: for each belt with `item.is_some()`, compute the destination tile (`next_tile(pos, dir)`). If the destination is another belt and its item is empty (or its own item will move out this tick), mark this item as "moving."
  - Pass 2: apply moves. An item moving requires: (1) destination belt exists, (2) destination's `item.is_none()` after pass 1 (i.e., the destination either had no item OR had one that's also moving).
- Mathematical equivalent: items advance iff the chain of belts in front of them all have items moving forward (or the front item has somewhere valid to go).

**`smelter_belt_io_system` algorithm:**
- For each Smelter entity, for each of the 4 cardinal sides:
  - Look up the adjacent tile (smelter position is on grid; belt tile's `IVec2` matches grid coordinates).
  - If adjacent belt's `dir` points TOWARD the smelter (i.e., `next_tile(belt_pos, belt.dir) == smelter_pos`), and belt has an `item`, AND `SmelterState.queue < QUEUE_MAX`: pull the item into queue, clear belt slot. Pull at most one item per smelter per tick (cardinal order: N, E, S, W).
  - If adjacent belt's `dir` points AWAY from the smelter (i.e., `next_tile(smelter_pos_treated_as_belt_origin, belt.dir) == ...`), formally: the belt's direction agrees with `(belt_pos - smelter_pos).into()` (the adjacent belt sits in direction X from the smelter and points further in direction X). And belt's `item.is_none()` AND `SmelterState.output` has any bars: push one bar to that belt slot. Push at most one bar per smelter per tick (cardinal order: N, E, S, W).

**`belt_pickup_system`:** for each `OreDrop` entity within an empty belt tile (matching grid position), transfer `OreDrop.item` into `BeltTile.item` and despawn the `OreDrop`. Belt tile must be empty.

**`belt_spillage_system`:** for each belt tile whose `next_tile(pos, dir)` is NOT a belt and NOT a smelter, AND whose item is not consumed by other I/O this tick: spawn an `OreDrop` at `tile_center_world(next_tile(pos, dir))` and clear the belt slot.

### Build-mode UI module: `src/systems/belt_ui.rs`

```rust
#[derive(Resource, Default)]
pub struct BeltBuildMode {
    pub cursor_dir: Option<BeltDir>,  // None = not in build mode
}

#[derive(Component)]
pub struct BeltGhost;
```

Systems:
- `belt_build_input_system` — handles B (toggle), Esc (exit), scroll wheel (rotate cursor_dir), left-click (place), right-click (remove). Gated on local player having `Tool::BeltUnlock` (silently no-ops the B keybind otherwise).
- `belt_ghost_render_system` — spawns/moves/despawns the `BeltGhost` entity following the cursor when build mode is active. Translucent belt sprite.

Build mode is **per-player local state** — Resource not replicated. Each peer toggles their own.

### Modified files

| File | Change |
|------|--------|
| `src/lib.rs` | `pub mod belt;` |
| `src/systems/mod.rs` | `pub mod belt; pub mod belt_ui;` |
| `src/components.rs` | Add `BeltGhost` Component (purely visual marker; pure module owns `BeltTile`/`BeltVisual`) |
| `src/tools.rs` | Add `Tool::BeltUnlock` variant (variant order is load-bearing — append at end) |
| `src/economy.rs` | `tool_buy_price(BeltUnlock) = 200` |
| `src/systems/shop_ui.rs` | "Belt Networks" Buy button appears alongside existing tool buys |
| `src/save.rs` | `SaveData.belts: Vec<(IVec2, BeltTile)>`; SAVE_VERSION 2 → 3 |
| `src/systems/save_load.rs` | `save::collect` queries belts; `save::apply` despawns + respawns belts |
| `src/systems/net_events.rs` | Add `PlaceBeltRequest { tile, dir }` and `RemoveBeltRequest { tile }` events |
| `src/systems/net_plugin.rs` | Register `replicate::<BeltTile>()`, `replicate::<BeltVisual>()`, `add_client_event` for both new events; add `handle_place_belt_requests` and `handle_remove_belt_requests` server-side handlers; add `add_belt_visuals_on_arrival` for client-side Sprite attachment |
| `src/systems/belt.rs` | (new) belt-tick + I/O systems |
| `src/systems/belt_ui.rs` | (new) build-mode input + ghost rendering |
| `src/app.rs` | Register all new systems into appropriate sets. Add `MachineSet::BeltTick` between `MachineSet::SmelterTick` and (existing) `MachineSet::SmelterUi`; insert `BeltSpillage` set after `MachineSet::SmelterUi`. Belt-UI input + ghost render go in `UiSet::Hud` or a new `UiSet::Build`. |

### Multiplayer

- `BeltTile` and `BeltVisual` derive `Component, Serialize, Deserialize, Clone, Copy` and are registered with replicon's `replicate::<T>()` in `MultiplayerPlugin::build`.
- New entities (belts) spawn server-side with the `Replicated` marker.
- The Bevy `Sprite` component is NOT replicated (per M4 fix). Add `add_belt_visuals_on_arrival(commands, q: Query<(Entity, &BeltTile, &BeltVisual), Added<BeltTile>>)` system in `net_player.rs` that attaches the belt sprite locally on the client.
- Two new client-fired events:
  ```rust
  #[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
  pub struct PlaceBeltRequest { pub tile: IVec2, pub dir: BeltDir }

  #[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
  pub struct RemoveBeltRequest { pub tile: IVec2 }
  ```
- Both events registered with `add_client_event(Channel::Ordered)`.
- `handle_place_belt_requests` and `handle_remove_belt_requests` server-side handlers run with `.run_if(server_running)`.
- Validation server-side (silent rejection on failure):
  - Place: tile must be in-bounds, must be floor (`!grid.get(x,y).solid`), must not already have a belt, must not have a Shop or Smelter sprite at that tile.
  - Remove: tile must have a belt; if the belt has an item, spawn an `OreDrop` at the belt's tile (position) before despawning the belt entity.
- Client-side `belt_build_input_system` branches on `NetMode`: in `Client` mode, fires the request event; in `SinglePlayer` or `Host` mode, mutates the world directly (matches the existing dig/buy/sell branching pattern).

### Save/load

- `SaveData` gains `belts: Vec<(IVec2, BeltTile)>`. SAVE_VERSION bumps 2 → 3.
- `save::collect` queries `Query<(&Transform, &BeltTile)>`, converts each Transform to a tile coord via `coords::world_to_tile`, builds the `Vec<(IVec2, BeltTile)>`.
- `save::apply` despawns existing belt entities then re-spawns from the saved list (with `BeltVisual` recomputed from `BeltTile.dir` + neighbors at spawn-time, and `Sprite` attached locally).
- v2 saves are silently discarded on load (existing `LoadError::VersionMismatch` path).
- Save/load only loaded in `NetMode::SinglePlayer` (existing pattern from M4).

## Data flow

### Single-player: ore from dig site to bars at shop

1. Player digs copper ore at tile (12, 5). `OreDrop` sprite spawns at `tile_center_world((12, 5))`.
2. Player walks adjacent and tosses (existing vacuum picks it up — wait, this is the existing flow; for the belt path, the player drops it on a belt tile by walking onto a belt tile while carrying ore... actually NO, per the design we picked option (a) "belts pick up ore drops from the world": the player digs *adjacent to* a belt tile, the OreDrop lands on the belt, `belt_pickup_system` consumes it).
3. So: player has placed a belt at (13, 5) facing East. They dig at (12, 5), the resulting `OreDrop` lands at world position `tile_center_world((12, 5))` — NOT on the belt. The player can either: (a) vacuum it up and drop it on the belt by some action (we haven't designed an explicit "drop on belt" action — the player would need to dig directly on a tile that's on a belt path), or (b) we revise: the player's dig can land the drop on an adjacent belt tile if one is right next to the dig site.
4. **Pragmatic resolution:** for MVP the player physically vacuums the ore (existing mechanic), then walks to the start of their belt. The first belt tile in their path picks up the ore from the player's inventory? No — that re-introduces a player→belt explicit action.
5. **Cleanest resolution:** the player drops ore by digging *directly above* (or adjacent to) a belt tile. The OreDrop then naturally falls within the belt tile's grid coord, and `belt_pickup_system` (which checks "OreDrop floor sprite within an empty belt tile") consumes it. **This is the path the player learns through play:** "if I want my ore on a belt, I dig right next to where the belt starts."

This is a slight revision from the brainstorm flow. The full reading: belts pick up `OreDrop` sprites that land on (or near, within tile coordinates) the belt tile. Players who want auto-feed dig adjacent to the belt entrance. Players who don't care can keep using the manual flow and walk ore to the smelter as today.

6. Continuing the example: ore at (12, 5) dig, belt at (13, 5) facing East. Belt at (13, 5) doesn't pick it up because (12, 5) ≠ (13, 5). Player learns to either dig at (13, 5) directly (impossible — belt is on it) or to dig at (14, 5) and place the belt to feed FROM that direction. The placement geometry teaches itself.

7. Smelter at (16, 5): belt chain (13,5) → (14,5) → (15,5) all facing East. Belt (15, 5) is east of (15,5) → next_tile((15,5), East) = (16, 5) = smelter position. So belt at (15, 5) is INPUT to the smelter (points INTO it).
8. Each tick: ore advances one tile. After 3 ticks ore arrives at (15, 5). Next tick: `smelter_belt_io_system` pulls it into smelter queue, belt (15, 5) is now empty.
9. Smelter processes for 2s (existing), bar appears in `SmelterState.output`.
10. Belt at (17, 5) facing East: next_tile((17,5), East) = (18, 5), so belt's direction "agrees with" the direction (17,5) - (16,5) = +X = East. Belt (17, 5) is OUTPUT of the smelter.
11. `smelter_belt_io_system` pushes the bar onto belt (17, 5).
12. Bar advances east, reaches end of belt, spills as `OreDrop` floor sprite.
13. Player vacuums bar, walks to shop, sells. Money += `item_sell_price(Bar(Copper))`.

### Multiplayer: client places a belt

1. Client is in build mode (Resource on client app, `cursor_dir = Some(East)`).
2. Client left-clicks tile (10, 8). `belt_build_input_system` (Client branch) fires `PlaceBeltRequest { tile: (10, 8), dir: East }`.
3. Replicon ships the event to host.
4. Host's `handle_place_belt_requests` validates (in-bounds, floor, no existing belt, no machine). Spawns `(BeltTile { item: None, dir: East }, BeltVisual::Straight, Transform::from_translation(tile_center_world((10,8))), Replicated)`.
5. Replicon ships the new entity (BeltTile + BeltVisual + Transform) to client.
6. Client's `add_belt_visuals_on_arrival` fires (`Added<BeltTile>` filter), inserts a Sprite component locally.
7. Client now sees the belt rendered.

### Multiplayer: trust-based removal

1. Client A right-clicks belt placed by client B (or host).
2. `RemoveBeltRequest { tile }` fires.
3. Host validates belt exists at tile. Inspects belt for items: if `item.is_some()`, spawn an `OreDrop` at the tile.
4. Despawn the belt entity. Both clients see the belt disappear.

## Edge cases

- **Belt loop:** player makes a circular belt (4 belts forming a square, all facing CW). Items orbit forever. **Acceptable in MVP.** No de-loop logic. Players will discover this and can either embrace it (storage carousel?) or remove a tile.
- **Belt collides with smelter spawn:** smelter is at (16, 5) from `setup_world`. Player tries to place a belt at (16, 5). Validation rejects (machine at tile). Silent no-op.
- **Belt placed in the middle of a dig pocket:** allowed (tile is floor). Player can later dig a tile under the belt? That's a separate concern: a belt's tile becomes `solid: true` again only if the player digs the tile away from underneath — but you can't dig a non-solid tile. So placing a belt on floor is permanent until the player removes it. ✓
- **Smelter consumes from multiple input belts:** see Section 4 — cardinal-order priority N → E → S → W, one item per tick per smelter.
- **Bar pushes to multiple output belts:** same priority, one bar per tick.
- **Dig path interrupted by belt:** player has a belt at (10, 5). They want to dig the floor at (10, 5)? Already not solid (it's floor). Can they dig at (10, 4) which is solid stone above? Yes — dig is independent of belt presence on adjacent floors. The belt at (10, 5) is unaffected.
- **Belt facing into a wall:** allowed. Items reaching the end fall off as spillage (`OreDrop` at the wall tile's center). Player will see ore drops piling up at the wall.
- **Item type mismatch on output:** smelter's output is bars. Player's output belt happens to lead into a smelter as input? `smelter_belt_io_system` would try to consume the bar as a recipe input. The smelter's recipe is `OreKind` (raw ore only); a bar is `ItemKind::Bar(_)` and would be ignored (smelter only accepts `ItemKind::Ore(_)` per its existing logic). The bar stays on the belt → back-pressure. Player learns through observation.
- **Removing a belt that's serving as the smelter's only input:** smelter starves naturally (no input → queue stays empty → no production). No special handling needed.
- **Race: client A places at (5,5), client B places at (5,5) same tick:** events arrive at host in some order. First request validates (no existing belt at (5,5)) and spawns. Second request fails validation (belt already at (5,5)) and silently no-ops. Both clients see the single belt.
- **Race: client A removes a belt as items are advancing through it:** `handle_remove_belt_requests` runs in Update set. If the request happens before `belt_tick_system`, the item is spilled; if after, the item moved out and spillage doesn't trigger. Either way nothing is silently lost.

## Testing strategy

**Pure-module unit tests in `tests/belt.rs` (~12 tests):**
- `belt_dir_delta_cardinals` — each direction's `delta()` matches expectation
- `belt_dir_opposite_round_trip` — `dir.opposite().opposite() == dir`
- `belt_dir_rotate_cw_cycles` — 4× rotate_cw returns to original
- `next_tile_basic` — `next_tile((0,0), East) == (1, 0)` etc.
- `belt_visual_straight_no_feeder` — no perpendicular feeder → `Straight`
- `belt_visual_corner_NE` — self_dir = East, feeder = South (going up-and-right from S to E) → `CornerSE`
- `belt_visual_corner_all_four` — exhaustive 4 corner kinds
- `belt_visual_feeder_aligned_means_straight` — if feeder is in-line (e.g., feeder = West, self = East), it's straight, not a corner
- `belt_back_pressure_blocks_when_destination_full` — pure helper for the back-pressure decision
- `belt_back_pressure_chain_clears_simultaneously` — 3 items in a row, all moving East, all advance one tile in the same tick
- `belt_back_pressure_loop_resolves` — circular belt, all items advance simultaneously (no deadlock)
- Plus 1-2 directed tests for the belt-direction-into-smelter "agrees with" geometry

**Save/load tests in `tests/save.rs` (~3 added):**
- `belts_round_trip_via_ron` — SaveData with belts in mixed directions and items round-trips
- `belts_apply_overwrites_destination_state` — apply replaces existing belts in destination world
- `belts_v2_save_discarded` — already covered by existing version-mismatch test, just verify it includes the belt path

**Net event serde tests in `tests/net_events.rs` (~2 added):**
- `place_belt_request_round_trips`
- `remove_belt_request_round_trips`

**Test count target:** 101 (existing) + 12 (belt pure) + 3 (save) + 2 (net_events) = **~118**.

**Bevy systems are not unit-tested** (existing convention). The four-mode smoke test (single-player + host + client + garbage CLI) plus the manual exit-criteria walkthrough is the integration test.

## Manual playtest exit criteria

### Single-player
- [ ] `cargo run`. Mine ~5 copper, sell. Walk to shop, see "Belt Networks" Buy button alongside existing tools. Click to buy (deducts 200c).
- [ ] Press B → ghost belt sprite appears at cursor with east-facing arrow.
- [ ] Scroll wheel → ghost rotates clockwise: N → E → S → W → N.
- [ ] Left-click an empty floor tile → belt spawned with arrow visible.
- [ ] Place ~6 belts in an L-shape: 3 east, then 3 north. Confirm corner tile renders as L (not straight).
- [ ] Drop ore by digging adjacent to the start of the belt. Watch the ore advance one tile per second.
- [ ] Place a belt that points INTO the smelter. Confirm smelter consumes the ore arriving on it (queue increments). Confirm the smelter starts processing (panel shows "smelting copper").
- [ ] Place an output belt pointing AWAY from the smelter. After ~2s confirm a bar appears on the output belt and starts advancing.
- [ ] Bar reaches the end of the output belt at a non-belt tile → spills as floor `OreDrop`.
- [ ] Vacuum bar, walk to shop, sell — money increases.
- [ ] Press F5 (save). Press F9 (load). Confirm belts and items-on-belts restored. Confirm smelter state restored.
- [ ] Right-click a belt with an item on it → belt despawns, `OreDrop` appears at that tile.
- [ ] Press Esc while in build mode → build mode exits.

### Multiplayer (two-window)
- [ ] Both players launch (`cargo run -- host` and `cargo run -- join 127.0.0.1:5000`). Both see each other's blue/orange sprites.
- [ ] Player A buys "Belt Networks" — only A's `OwnedTools` updates. Player B's shop panel still shows Belt Networks as buyable.
- [ ] Player A presses B → A enters build mode (A sees ghost). B does NOT see A's ghost.
- [ ] Player A places 5 belts. Player B sees them appear within ~1 frame each.
- [ ] Player B drops ore on a belt placed by A. Confirm ore advances on B's screen and A's screen identically.
- [ ] Player B right-clicks a belt placed by A → trust-based removal succeeds. Belt disappears on both screens.
- [ ] Items on a removed belt spill as `OreDrop` on both screens.
- [ ] Disconnect host → client cleanly exits (M4 behavior preserved).

### Stability
- [ ] 15-min mixed-mode session with belts in single-player AND a separate two-window co-op session: no crashes, no orphaned items, no stuck belts, no save corruption.

## Risks & open questions

1. **Tick rate vs. visual smoothness.** 1 tile/sec hard-snap may feel janky. If playtest reveals unbearable jerkiness, add a one-tick interpolation (item lerps from old tile to new tile over the tick duration) — small change, easy to add post-MVP. Keep the data model the same.
2. **Save schema for items-on-belts.** `BeltTile` has `item: Option<ItemKind>` which already serializes via existing serde derives. No new schema work — verify in unit test.
3. **Build-mode keybind conflict.** B is unused today (Tab is inventory popup, F5/F9 save/load, WASD movement, Space dig). No conflict expected — verify by grepping the input systems before commit.
4. **Replicon performance under heavy belt updates.** Each belt is a separate replicated entity. With 50+ belts containing items, replicon ships per-tick deltas for each. Estimate: 50 belts × ~16 bytes/BeltTile = ~800 bytes/tick = 800 B/s sustained. Tolerable. If we ever scale to 500+ belts, consider per-tile delta encoding (M5b+ concern).
5. **Edge case discovered during design:** the brainstorm flow assumed "belts pick up ore drops from the world" was self-evident, but the dig→drop→belt path requires the player to dig adjacent to a belt for the drop to land on it (Section "Data flow" #3-5). This is teachable through play and matches OFS's approach, but worth flagging in onboarding.
6. **Belt direction immutable post-placement.** No "rotate placed belt" action in MVP. Player must remove and re-place. Acceptable; tracked for post-MVP polish.

## Future hooks (M5b+)

- **Multi-stage recipes.** A second machine (e.g., "wire mill") consumes bars from a belt and produces wire. The belt I/O direction rule generalizes — `wire_mill_belt_io_system` follows the same pattern.
- **Mergers / splitters.** Special belt entities with multiple inputs or outputs. The hard-slot data model needs a tweak (a splitter has a "next destination" flip-flop bit), but the existing `belt_tick_system` skeleton handles it.
- **Stockpile entities.** Buffer items off-belt. Player UI to deposit/withdraw. Useful for contracts (M6).
- **Per-tile belt cost.** Shop offers belts as inventory items priced 5c each; placing consumes one. Multiplayer needs per-player belt-stock replication.
- **Continuous-position interpolation.** Smoother item glide at the cost of 100s of lines of position-update math.
- **Belt rotation post-placement.** Click+drag or shift-click to rotate without removing.
