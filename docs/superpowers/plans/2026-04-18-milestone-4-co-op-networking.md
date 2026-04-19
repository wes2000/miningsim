# Milestone 4 — Co-op Networking (MVP) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [../specs/2026-04-18-milestone-4-co-op-networking-design.md](../specs/2026-04-18-milestone-4-co-op-networking-design.md)

**Goal:** Add 2-player direct-IP co-op via `bevy_replicon`. Authoritative-host model. Per-player Money/Inventory/OwnedTools (refactored from Resource to Component on Player). Trust-based shared smelter. Late-join via initial-state snapshot. Graceful disconnect.

**Architecture:** Three phases: (A) Resource→Component refactor, one resource per task, gameplay stays playable after each. (B) NetMode CLI + plugin selection scaffold (`SaveLoadPlugin` XOR `MultiplayerPlugin`). (C) Replicon integration: replicated component registration, client-fired event handlers on host, player lifecycle, disconnect handling.

**Tech Stack:** Rust (stable), Bevy 0.15.x (pinned), `bevy_replicon = "0.32"` ([docs.rs](https://docs.rs/crate/bevy_replicon/0.32.2)), `bevy_replicon_renet = "0.9"` (transport adapter, hard-pins `bevy_replicon ^0.32`).

---

## Pre-flight: environment expectations

This plan assumes:
- Rust stable toolchain ≥ 1.82.
- Working directory: `c:/Users/whann/Desktop/Games/miningsim` (existing git repo, branch `milestone-4` already created from `main`).
- `cargo test` currently passes 87/87 at `main`'s `ea6b9f1` (post-save/load merge).
- Author identity: commits use `--author="wes2000 <whannasch@gmail.com>"`. Do not modify global git config.

If any of these aren't true, stop and resolve before proceeding.

---

## Resolved open questions (from spec)

- **`bevy_replicon` version pin:** `bevy_replicon = "0.32"`, `bevy_replicon_renet = "0.9"` — these are the bevy-0.15-compatible releases. (Initially planned 0.30+0.5, but `bevy_replicon_renet 0.5` hard-depends on bevy 0.14; corrected during Task 7.) Notable API renames vs. earlier 0.x snippets in this plan: `ChannelKind`→`Channel`, `ReplicationPlugins`→`RepliconPlugins`, `ServerEvent`→`ClientConnected`/`ClientDisconnected` triggers, `ClientId` removed from prelude (clients are entities with `ConnectedClient`), serialization is `postcard` (not `bincode`), default visibility policy is `Blacklist`. Adapt subsequent tasks' code snippets to the 0.32 API where they reference older names.
- **Host self-ClientId:** replicon's host-as-server model does NOT assign a ClientId to the host's own player. We do NOT use the per-connection spawn handler (`spawn_player_for_new_clients`) for the host's player — instead, the host's local Player is spawned at `setup_world` with `LocalPlayer` already attached, just like single-player. The per-connection spawn handler only runs for actual remote client connections.

---

## File structure (target end state)

```
Cargo.toml                       # MODIFY: + bevy_replicon "0.30", + bevy_replicon_renet "0.5"
src/
  net.rs                         # NEW: pure — NetMode, DEFAULT_PORT, parse_args, CliParseError
  components.rs                  # MODIFY: + LocalPlayer, RemotePlayer markers
  inventory.rs                   # MODIFY: drop Resource derive; HashMap → BTreeMap
  economy.rs                     # MODIFY: drop Resource derive on Money
  tools.rs                       # MODIFY: drop Resource derive on OwnedTools; HashSet → BTreeSet; Tool needs Ord
  processing.rs                  # MODIFY: SmelterState.output: HashMap → BTreeMap
  items.rs                       # MODIFY: OreKind, ItemKind need Ord, PartialOrd
  save.rs                        # MODIFY: SAVE_VERSION = 2
  systems/
    setup.rs                     # MODIFY: spawn local Player with Money/Inventory/OwnedTools/LocalPlayer components
    hud.rs                       # MODIFY: query Single<…, With<LocalPlayer>>
    shop.rs, shop_ui.rs          # MODIFY: button handlers branch on NetMode (mutate vs fire event)
    smelter.rs                   # MODIFY: button handlers branch on NetMode; tick stays host-only
    save_load.rs                 # MODIFY: query LocalPlayer's components; wrap in SaveLoadPlugin
    player.rs                    # MODIFY: dig_input branches on NetMode; uses LocalPlayer
    ore_drop.rs                  # MODIFY: pickup adds to LocalPlayer's Inventory in single-player; host-side in multiplayer
    net_plugin.rs                # NEW: MultiplayerPlugin — replicon mount, mode dispatch, event handler registration
    net_events.rs                # NEW: DigRequest, BuyToolRequest, SmeltAllRequest, CollectAllRequest, SellAllRequest
    net_player.rs                # NEW: spawn_player_for_new_clients (host), mark_local_player_on_arrival (client), RemotePlayer rendering
  app.rs                         # MODIFY: branch on Res<NetMode> — load SaveLoadPlugin XOR MultiplayerPlugin
  main.rs                        # MODIFY: parse CLI args via net::parse_args; insert NetMode resource
  lib.rs                         # MODIFY: pub mod net; pub mod systems::{net_plugin, net_events, net_player}
tests/
  inventory.rs, tools.rs         # MODIFY: BTreeMap/BTreeSet API
  processing.rs                  # MODIFY: BTreeMap on output
  save.rs                        # MODIFY: SAVE_VERSION = 2; reuse existing 9 tests
  net.rs                         # NEW: ~8 parse_args tests
  net_events.rs                  # NEW: ~5 serde round-trip tests per event
```

---

## Conventions

- Commit style: present-tense imperative. `--author="wes2000 <whannasch@gmail.com>"` on every commit.
- Pure modules follow TDD: failing test → verify fail → implement → verify pass → commit.
- Bevy systems are not unit-tested. The two-window `cargo run -- host` / `cargo run -- join 127.0.0.1:5000` flow is the integration test.
- `cargo run` blocks on the Bevy window — **subagents must not run the binary**. Use `cargo build` + `cargo test`; the human controller drives `cargo run` at smoke-test checkpoints.
- Each commit must leave the crate building and `cargo test` green. The Resource → Component migration in Tasks 1–4 is sequenced so single-player gameplay works after every commit.

---

## User smoke-test checkpoints

Three checkpoints:

1. **After Task 4** (refactor complete) — single-player regression check. Mining, smelting, shop, save/load all behave identically to pre-M4. Critical — if this breaks, bail and fix before proceeding.
2. **After Task 6** (NetMode CLI works) — quick sanity check. `cargo run` still works (single-player + save/load). `cargo run -- host` and `cargo run -- join 127.0.0.1:5000` should at minimum launch the window without crashing (no networking yet — they just don't load `SaveLoadPlugin`).
3. **After Task 12** (full multiplayer flow) — two-window co-op test using the spec's full manual exit-criteria checklist.

Plus the final exit-criteria walkthrough in Task 14 before merge.

---

## Task 1: `Money` Resource → Component on Player + LocalPlayer marker

**Files:**
- Modify: `src/components.rs`
- Modify: `src/economy.rs`
- Modify: `src/systems/setup.rs`
- Modify: `src/systems/hud.rs`
- Modify: `src/systems/shop_ui.rs`
- Modify: `src/systems/smelter.rs` (only if it reads Money — verify)
- Modify: `src/systems/save_load.rs`

The first migration step. After this commit, single-player gameplay works identically; Money is just stored on the Player entity instead of as a Resource.

- [ ] **Step 1: Add LocalPlayer + RemotePlayer markers to `src/components.rs`**

Append:
```rust
/// The player entity controlled by this client. Exactly one in any session.
#[derive(Component)]
pub struct LocalPlayer;

/// A player entity replicated from another peer. Renders with a different sprite color.
#[derive(Component)]
pub struct RemotePlayer;
```

- [ ] **Step 2: Drop Resource derive from Money**

In `src/economy.rs`, change Money to drop `Resource`:
```rust
#[derive(Component, Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Money(pub u32);
```
(Was `#[derive(... Resource ...)]` — replace with `Component`.)

- [ ] **Step 3: Spawn Money on Player in setup_world**

In `src/systems/setup.rs`, the existing `commands.insert_resource(Money::default())` line goes away. The local Player spawn gains `LocalPlayer` and `Money::default()` components:

```rust
// Player
commands.spawn((
    Player,
    LocalPlayer,                     // NEW marker
    Velocity::default(),
    Facing::default(),
    Money::default(),                // NEW component (was Resource)
    Sprite { color: Color::srgb(0.30, 0.60, 0.90), custom_size: Some(Vec2::splat(12.0)), ..default() },
    Transform::from_translation(player_world.extend(10.0)),
));
```
Remove the `commands.insert_resource(Money::default())` line.

- [ ] **Step 4: Update HUD systems to query LocalPlayer's Money**

In `src/systems/hud.rs`:

`update_money_text_system` currently takes `money: Res<Money>`. Change to:
```rust
pub fn update_money_text_system(
    money_q: Query<&Money, (With<LocalPlayer>, Changed<Money>)>,
    mut text_q: Query<&mut Text, With<MoneyText>>,
) {
    let Ok(money) = money_q.get_single() else { return };
    if let Ok(mut text) = text_q.get_single_mut() {
        **text = format!("{}c", money.0);
    }
}
```

Note: `Changed<Money>` filter replaces `is_changed()`. The query yields the LocalPlayer's Money entity, and `Changed` ensures we only refresh when it actually changed.

- [ ] **Step 5: Update shop_ui to mutate LocalPlayer's Money**

In `src/systems/shop_ui.rs`:

`update_shop_labels_system` and `handle_shop_buttons_system` both reference Money. Change `Res<Money>` → query, `ResMut<Money>` → mut query:

```rust
pub fn update_shop_labels_system(
    money_q: Query<&Money, With<LocalPlayer>>,
    owned_q: Query<&OwnedTools, With<LocalPlayer>>,
    /* ... existing button query / texts query / bg query ... */
) {
    let Ok(money) = money_q.get_single() else { return };
    let Ok(owned) = owned_q.get_single() else { return };
    if !money.is_changed() && !owned.is_changed() { return; }
    /* ... existing button label/bg refresh logic, using `money` and `owned` instead of `money.0` from Res ... */
}
```

Wait — `Changed<>` filter on a query won't fire for "money or owned changed." Use `Or<(Changed<Money>, Changed<OwnedTools>)>` filter, or check `is_changed()` on the queried component. Simplest: keep the early-return pattern but use `is_changed()` calls:

Actually, since `Single<&Money, With<LocalPlayer>>` is the cleanest API in Bevy 0.15, prefer that style throughout this milestone. `Single<>` extracts a single matching entity and gives change detection on the component:

```rust
pub fn update_shop_labels_system(
    local_player: Single<(&Money, &OwnedTools), With<LocalPlayer>>,
    // ... button query / texts query / bg query ...
) {
    let (money, owned) = local_player.into_inner();
    // … existing logic, replacing Res<Money>.0 with money.0, etc.
}
```

If `Single<>` isn't available in the pinned Bevy 0.15 patch, fall back to `Query<…, With<LocalPlayer>>::get_single()`.

`handle_shop_buttons_system` similarly: replace `mut money: ResMut<Money>` and `mut owned: ResMut<OwnedTools>` with a mutable single query on LocalPlayer.

- [ ] **Step 6: Update save_load to query LocalPlayer's Money**

In `src/systems/save_load.rs`, the four systems currently take `Res<Money>` / `ResMut<Money>`. Change them to `Single<&Money, With<LocalPlayer>>` / `Single<&mut Money, With<LocalPlayer>>` (or query equivalents).

`save_now` and `try_load_and_apply` helper signatures change to take `&Money` / `&mut Money` directly (no longer Resources).

- [ ] **Step 7: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: build clean, 87 tests still passing. The economy unit tests don't change (Money is the same struct; the Resource-vs-Component distinction doesn't appear in pure-data tests).

- [ ] **Step 8: Commit**

```bash
git add src/components.rs src/economy.rs src/systems/setup.rs src/systems/hud.rs src/systems/shop_ui.rs src/systems/smelter.rs src/systems/save_load.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Migrate Money: Resource -> Component on Player; add LocalPlayer marker"
```

---

## Task 2: `Inventory` Resource → Component (HashMap → BTreeMap)

**Files:**
- Modify: `src/items.rs` (Ord derives)
- Modify: `src/inventory.rs`
- Modify: `src/systems/setup.rs`
- Modify: `src/systems/hud.rs`
- Modify: `src/systems/shop_ui.rs`
- Modify: `src/systems/smelter.rs`
- Modify: `src/systems/player.rs` (dig_input ore drop spawn doesn't touch inventory directly, but verify)
- Modify: `src/systems/ore_drop.rs` (pickup writes to LocalPlayer's Inventory)
- Modify: `src/systems/save_load.rs`
- Modify: `tests/inventory.rs` (API stays the same; only internal type changes)
- Modify: `tests/economy.rs` (uses Inventory)
- Modify: `tests/save.rs` (uses Inventory)
- Modify: `tests/dig.rs` (verify — likely doesn't touch Inventory)

- [ ] **Step 1: Derive Ord on OreKind, ItemKind**

In `src/items.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OreKind { Copper, Silver, Gold }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemKind { Ore(OreKind), Bar(OreKind) }
```

- [ ] **Step 2: Migrate Inventory in `src/inventory.rs`**

Drop `Resource` derive (gain `Component`). Swap `HashMap` → `BTreeMap`:

```rust
use std::collections::BTreeMap;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use crate::items::ItemKind;

#[derive(Component, Debug, Default, Clone, Serialize, Deserialize)]
pub struct Inventory {
    counts: BTreeMap<ItemKind, u32>,
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

API methods (`add`, `remove`, `get`) are unchanged — callers don't need to change.

- [ ] **Step 3: Spawn Inventory on Player in setup_world**

In `src/systems/setup.rs`, remove the `commands.insert_resource(Inventory::default())` line. Add `Inventory::default()` to the local Player spawn tuple:

```rust
commands.spawn((
    Player,
    LocalPlayer,
    Velocity::default(),
    Facing::default(),
    Money::default(),
    Inventory::default(),         // NEW component (was Resource)
    Sprite { color: ..., ..default() },
    Transform::from_translation(...),
));
```

- [ ] **Step 4: Update HUD inventory popup to query LocalPlayer's Inventory**

In `src/systems/hud.rs`, `update_inventory_popup_system` currently takes `inv: Res<Inventory>`. Change to:
```rust
pub fn update_inventory_popup_system(
    local_inv: Single<&Inventory, With<LocalPlayer>>,
    local_owned: Single<&OwnedTools, With<LocalPlayer>>,
    mut item_q: Query<(&mut Text, &ItemCountText), Without<ToolRowText>>,
    mut tool_q: Query<(&mut Text, &ToolRowText), Without<ItemCountText>>,
) {
    let inv = local_inv.into_inner();
    let owned = local_owned.into_inner();
    if inv.is_changed() {
        for (mut text, marker) in item_q.iter_mut() {
            **text = inv.get(marker.0).to_string();
        }
    }
    if owned.is_changed() {
        // existing strongest-tool logic
    }
}
```

Note: `Single::is_changed()` is the proper Bevy 0.15 way to detect change on a single-fetched component. If it's not available on `Single`, use `Query::single().is_changed()` via `Ref<>`.

- [ ] **Step 5: Update shop_ui Sell All handler to mutate LocalPlayer's Inventory**

`handle_shop_buttons_system` calls `economy::sell_all(&mut inv, &mut money)`. The `Res<Inventory>` parameter changes to a mutable single query on LocalPlayer.

- [ ] **Step 6: Update smelter button handlers**

`handle_smelter_buttons_system` in `src/systems/smelter.rs` calls `inv.remove(ItemKind::Ore(*ore), count)` and `inv.add(ItemKind::Bar(ore), n)`. Same change — mutate LocalPlayer's Inventory.

- [ ] **Step 7: Update ore_drop pickup**

In `src/systems/ore_drop.rs`, `ore_drop_system` calls `inv.add(drop.item, 1)`. Change Inventory parameter from `ResMut<Inventory>` to a mutable single query on LocalPlayer. (For multiplayer in later tasks, the host will adjudicate the actual pickup, but for single-player this is a direct mutation of the local player's inventory.)

- [ ] **Step 8: Update save_load**

Same `Res<Inventory>` → `Single<&Inventory, With<LocalPlayer>>` swap as Money in Task 1. `save_now` and `try_load_and_apply` helpers take `&Inventory` / `&mut Inventory` references.

- [ ] **Step 9: Update tests**

`tests/inventory.rs`, `tests/economy.rs`, `tests/save.rs`, `tests/dig.rs` — the API surface (`add`, `remove`, `get`) is unchanged. The only test changes are if any test asserts iteration order on the underlying collection (BTreeMap iterates sorted; HashMap was unordered). Spot-check by running tests; fix any that depend on iteration order.

- [ ] **Step 10: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 87 tests still passing.

- [ ] **Step 11: Commit**

```bash
git add src/items.rs src/inventory.rs src/systems/setup.rs src/systems/hud.rs src/systems/shop_ui.rs src/systems/smelter.rs src/systems/player.rs src/systems/ore_drop.rs src/systems/save_load.rs tests/
git commit --author="wes2000 <whannasch@gmail.com>" -m "Migrate Inventory: Resource -> Component on Player; HashMap -> BTreeMap"
```

---

## Task 3: `OwnedTools` Resource → Component (HashSet → BTreeSet)

**Files:**
- Modify: `src/tools.rs`
- Modify: `src/systems/setup.rs`
- Modify: `src/systems/hud.rs`
- Modify: `src/systems/shop_ui.rs`
- Modify: `src/systems/player.rs` (dig_input uses OwnedTools)
- Modify: `src/systems/save_load.rs`
- Modify: `tests/tools.rs` (BTreeSet API; same test bodies)
- Modify: `tests/economy.rs` (uses OwnedTools)
- Modify: `tests/save.rs` (uses OwnedTools)

- [ ] **Step 1: Derive Ord on Tool**

In `src/tools.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tool { Shovel, Pickaxe, Jackhammer, Dynamite }
```

Update `OwnedTools`:
```rust
use std::collections::BTreeSet;

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct OwnedTools(pub BTreeSet<Tool>);

impl Default for OwnedTools {
    fn default() -> Self {
        let mut s = BTreeSet::new();
        s.insert(Tool::Shovel);
        Self(s)
    }
}
```

`best_applicable_tool` already iterates a fixed `[Dynamite, Jackhammer, Pickaxe, Shovel]` array — no change needed.

- [ ] **Step 2: Spawn OwnedTools on Player in setup_world**

Remove the `commands.insert_resource(OwnedTools::default())` line. Add `OwnedTools::default()` to the local Player spawn tuple.

- [ ] **Step 3: Update consumers**

Same pattern as Tasks 1-2: `Res<OwnedTools>` / `ResMut<OwnedTools>` → `Single<&OwnedTools, With<LocalPlayer>>` / `Single<&mut OwnedTools, With<LocalPlayer>>`.

Sites:
- `hud::update_inventory_popup_system` (current-tool indicator)
- `shop_ui::update_shop_labels_system` (Buy button OWNED state)
- `shop_ui::handle_shop_buttons_system` (Buy → try_buy mutates OwnedTools)
- `player::dig_input_system` (best_applicable_tool reads OwnedTools)
- `save_load::*` (collect/apply)

- [ ] **Step 4: Update tests**

`tests/tools.rs` — API is unchanged; only internal collection type differs. Spot-check no test relies on HashSet iteration order.

- [ ] **Step 5: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 87 tests still passing.

- [ ] **Step 6: Commit**

```bash
git add src/tools.rs src/systems/setup.rs src/systems/hud.rs src/systems/shop_ui.rs src/systems/player.rs src/systems/save_load.rs tests/
git commit --author="wes2000 <whannasch@gmail.com>" -m "Migrate OwnedTools: Resource -> Component on Player; HashSet -> BTreeSet"
```

---

## Task 4: `SmelterState.output` HashMap → BTreeMap + bump SAVE_VERSION to 2

**Files:**
- Modify: `src/processing.rs`
- Modify: `src/save.rs`
- Modify: `tests/processing.rs`
- Modify: `tests/save.rs`

Small atomic commit. SmelterState is already a Component (M3); only its `output` field collection type and the SAVE_VERSION constant change.

- [ ] **Step 1: Migrate `SmelterState.output` to BTreeMap**

In `src/processing.rs`:
```rust
use std::collections::BTreeMap;

#[derive(Component, Debug, Default, Clone, Serialize, Deserialize)]
pub struct SmelterState {
    pub recipe: Option<OreKind>,
    pub time_left: f32,
    pub queue: u32,
    pub output: BTreeMap<OreKind, u32>,
}
```

`tick_smelter`, `start_smelting`, `collect_output`, `is_busy` need no logic changes — `BTreeMap` and `HashMap` share the same `entry().or_insert()`, `get`, `take()` API surface used here.

- [ ] **Step 2: Bump SAVE_VERSION**

In `src/save.rs`:
```rust
pub const SAVE_VERSION: u32 = 2;
```

This invalidates v1 saves on load (silent discard per the existing `LoadError::VersionMismatch` path).

- [ ] **Step 3: Update tests**

`tests/processing.rs` constructs `output: HashMap::new()` in some test setups — change to `BTreeMap::new()`. The 11 existing tests should still pass with no logic changes.

`tests/save.rs` — the `version_mismatch_is_detected` test asserts `expected = SAVE_VERSION`; that's already constant-driven, so no change needed. The round-trip test uses `HashMap::new()` to construct the SmelterState fixture — change to `BTreeMap::new()`.

- [ ] **Step 4: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 87 tests still passing.

- [ ] **Step 5: Commit**

```bash
git add src/processing.rs src/save.rs tests/processing.rs tests/save.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Migrate SmelterState.output to BTreeMap; bump SAVE_VERSION to 2"
```

---

## Smoke-test checkpoint #1 (after Task 4)

Human controller runs `cargo run`. Expected:
- Game launches, fresh world (or loaded from `save.ron` if v2 exists; v1 saves silently discarded).
- Mining, smelting, shop, save (F5), load (F9), auto-save on quit — all work identically to pre-M4.
- HUD shows correct ore/bar counts, money, current tool — read from the local Player's components.
- No regressions.

If anything is visibly broken, surface to controller for diagnosis before Task 5.

---

## Task 5: `net.rs` pure module — NetMode + parse_args (TDD)

**Files:**
- Create: `src/net.rs`
- Create: `tests/net.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Register module**

In `src/lib.rs`, add `pub mod net;` in alphabetical order (after `items`, before `processing`).

- [ ] **Step 2: Write failing tests in `tests/net.rs`**

```rust
use std::net::SocketAddr;
use miningsim::net::{self, CliParseError, NetMode, DEFAULT_PORT};

fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

#[test]
fn no_args_is_single_player() {
    assert_eq!(net::parse_args(&[]), Ok(NetMode::SinglePlayer));
}

#[test]
fn host_no_port_uses_default() {
    assert_eq!(net::parse_args(&s(&["host"])), Ok(NetMode::Host { port: DEFAULT_PORT }));
}

#[test]
fn host_with_port() {
    assert_eq!(net::parse_args(&s(&["host", "5050"])), Ok(NetMode::Host { port: 5050 }));
}

#[test]
fn host_with_bad_port() {
    assert_eq!(
        net::parse_args(&s(&["host", "abc"])),
        Err(CliParseError::BadPort("abc".to_string())),
    );
}

#[test]
fn join_with_addr() {
    let expected: SocketAddr = "192.168.1.5:5000".parse().unwrap();
    assert_eq!(net::parse_args(&s(&["join", "192.168.1.5:5000"])), Ok(NetMode::Client { addr: expected }));
}

#[test]
fn join_with_loopback() {
    let expected: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    assert_eq!(net::parse_args(&s(&["join", "127.0.0.1:5000"])), Ok(NetMode::Client { addr: expected }));
}

#[test]
fn join_missing_addr() {
    assert_eq!(
        net::parse_args(&s(&["join"])),
        Err(CliParseError::MissingArg("join requires an address")),
    );
}

#[test]
fn join_bad_addr() {
    assert!(matches!(
        net::parse_args(&s(&["join", "not-an-addr"])),
        Err(CliParseError::BadAddr(_)),
    ));
}

#[test]
fn unknown_command() {
    assert_eq!(
        net::parse_args(&s(&["whatever"])),
        Err(CliParseError::UnknownCommand("whatever".to_string())),
    );
}
```

- [ ] **Step 3: Run tests — expect compile failure**

```bash
cargo test --test net 2>&1 | tail -10
```

- [ ] **Step 4: Implement `src/net.rs`**

```rust
use std::net::SocketAddr;
use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub enum NetMode {
    SinglePlayer,
    Host { port: u16 },
    Client { addr: SocketAddr },
}

pub const DEFAULT_PORT: u16 = 5000;

#[derive(Debug, PartialEq, Eq)]
pub enum CliParseError {
    UnknownCommand(String),
    MissingArg(&'static str),
    BadAddr(String),
    BadPort(String),
}

/// Parse `std::env::args()`-style strings (excluding the binary name).
/// Accepts:
///   []                                       → SinglePlayer
///   ["host"]                                 → Host { port: DEFAULT_PORT }
///   ["host", "<port>"]                       → Host { port: <parsed> }
///   ["join", "<addr>"]                       → Client { addr: <parsed> }
pub fn parse_args(args: &[String]) -> Result<NetMode, CliParseError> {
    match args.first().map(String::as_str) {
        None => Ok(NetMode::SinglePlayer),
        Some("host") => {
            let port = match args.get(1) {
                None => DEFAULT_PORT,
                Some(p) => p.parse().map_err(|_| CliParseError::BadPort(p.clone()))?,
            };
            Ok(NetMode::Host { port })
        }
        Some("join") => {
            let addr_str = args.get(1).ok_or(CliParseError::MissingArg("join requires an address"))?;
            let addr = addr_str.parse().map_err(|_| CliParseError::BadAddr(addr_str.clone()))?;
            Ok(NetMode::Client { addr })
        }
        Some(other) => Err(CliParseError::UnknownCommand(other.to_string())),
    }
}
```

- [ ] **Step 5: Run tests — expect 9/9 passing**

```bash
cargo test --test net 2>&1 | tail -10
```

- [ ] **Step 6: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: 87 + 9 = 96 tests passing.

- [ ] **Step 7: Commit**

```bash
git add src/net.rs src/lib.rs tests/net.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add net module: NetMode enum + parse_args CLI parser"
```

---

## Task 6: SaveLoadPlugin + main.rs CLI parsing + app.rs branching

**Files:**
- Modify: `src/systems/save_load.rs` (wrap systems into `SaveLoadPlugin`)
- Modify: `src/main.rs` (parse args, insert NetMode resource)
- Modify: `src/app.rs` (branch on NetMode — load SaveLoadPlugin XOR nothing-yet)

After this task, single-player still works. `cargo run -- host` and `cargo run -- join ...` launch the window with no networking active (MultiplayerPlugin doesn't exist yet — Task 7 introduces it). The branching scaffold is in place.

- [ ] **Step 1: Wrap save_load systems into a Plugin**

In `src/systems/save_load.rs`, append:

```rust
use bevy::prelude::*;

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_system.after(/* whichever startup system spawns Smelter+Player+UI */))
            .add_systems(Update, (
                save_hotkey_system.in_set(crate::app::UiSet::SaveLoad),
                load_hotkey_system.in_set(crate::app::UiSet::SaveLoad),
                auto_save_on_exit_system.in_set(crate::app::UiSet::SaveLoad),
            ));
    }
}
```

The exact `.after(...)` and `.in_set(...)` registrations need to mirror what `app.rs` previously did directly. Check the M3.5 commit `4ec22cd` for the literal registration; preserve ordering.

- [ ] **Step 2: Update main.rs to parse CLI args**

```rust
use bevy::prelude::*;
use miningsim::app::MiningSimPlugin;
use miningsim::net::{self, NetMode};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let net_mode = match net::parse_args(&args) {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("CLI parse error: {:?} — falling back to single-player", err);
            NetMode::SinglePlayer
        }
    };

    App::new()
        .insert_resource(net_mode)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MiningSim".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MiningSimPlugin)
        .run();
}
```

- [ ] **Step 3: Update app.rs to branch on NetMode**

In `src/app.rs`, `MiningSimPlugin::build` removes the direct save_load system registrations (those moved into `SaveLoadPlugin`) and conditionally loads the right plugin:

```rust
impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        // ... existing SystemSet definitions, configure_sets, all the non-save-load systems ...

        // Mode-conditional plugin loading
        let net_mode = app.world().resource::<crate::net::NetMode>().clone();
        match net_mode {
            crate::net::NetMode::SinglePlayer => {
                app.add_plugins(crate::systems::save_load::SaveLoadPlugin);
            }
            crate::net::NetMode::Host { .. } | crate::net::NetMode::Client { .. } => {
                // MultiplayerPlugin lands in Task 7. For now, leave both branches no-op
                // for non-SinglePlayer modes — the game will run without networking
                // (and without save/load), which is the intentional intermediate state.
            }
        }
    }
}
```

Drop the previous direct `add_systems(Update, ... save_load systems ...)` calls — they're inside `SaveLoadPlugin` now.

- [ ] **Step 4: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 96 tests still passing.

- [ ] **Step 5: Commit**

```bash
git add src/systems/save_load.rs src/main.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Wrap save/load in SaveLoadPlugin; main.rs parses CLI; app.rs branches on NetMode"
```

---

## Smoke-test checkpoint #2 (after Task 6)

Human controller verifies:
- `cargo run` → single-player works as before. Save/load active.
- `cargo run -- host` → window opens, no crash, console shows nothing networking-related (because MultiplayerPlugin doesn't exist yet). Save/load is NOT active. Player can move and dig as a "lonely host" — useful sanity check that the branching works.
- `cargo run -- join 127.0.0.1:5000` → window opens, no crash, no networking. Same lonely-player behavior.
- `cargo run -- garbage` → eprintln warning; fallback to single-player. Window opens normally.

---

## Task 7: bevy_replicon deps + MultiplayerPlugin scaffold

**Files:**
- Modify: `Cargo.toml`
- Create: `src/systems/net_plugin.rs`
- Modify: `src/systems/mod.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Add deps to Cargo.toml**

```toml
[dependencies]
bevy = "0.15"
bevy_replicon = "0.32"
bevy_replicon_renet = "0.9"
rand = "0.8"
ron = "0.8"
serde = { version = "1", features = ["derive"] }
```

(Inserted in alphabetical order.)

- [ ] **Step 2: Create empty MultiplayerPlugin scaffold**

`src/systems/net_plugin.rs`:
```rust
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RepliconPlugins);
        app.add_plugins(RepliconRenetPlugins);
        // Replication registrations + event handlers added in subsequent tasks.
    }
}
```

- [ ] **Step 3: Register module**

In `src/systems/mod.rs`, add:
```rust
pub mod net_plugin;
```

- [ ] **Step 4: Wire into app.rs**

In `src/app.rs`, the multi-player branch from Task 6 now loads MultiplayerPlugin:
```rust
crate::net::NetMode::Host { .. } | crate::net::NetMode::Client { .. } => {
    app.add_plugins(crate::systems::net_plugin::MultiplayerPlugin);
}
```

- [ ] **Step 5: Build + test**

```bash
cargo build 2>&1 | tail -20
cargo test 2>&1 | grep "test result"
```
Expected: build clean (this will pull and compile bevy_replicon — first build will be slow, ~2-5 minutes). 96 tests passing.

If `bevy_replicon` 0.30's API differs from the spec's snippets (channel kind names, plugin names), adapt. The spec is approximate; check `bevy_replicon`'s 0.30 docs for the exact API shape.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/systems/net_plugin.rs src/systems/mod.rs src/app.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add bevy_replicon deps; MultiplayerPlugin scaffold"
```

---

## Task 8: `net_events.rs` — client-fired events (TDD)

**Files:**
- Create: `src/systems/net_events.rs`
- Create: `tests/net_events.rs`
- Modify: `src/systems/mod.rs`

- [ ] **Step 1: Register module**

In `src/systems/mod.rs`, add `pub mod net_events;` in alphabetical order.

- [ ] **Step 2: Write failing tests in `tests/net_events.rs`**

```rust
use bevy::math::IVec2;

use miningsim::items::OreKind;
use miningsim::systems::net_events::{
    BuyToolRequest, CollectAllRequest, DigRequest, SellAllRequest, SmeltAllRequest,
};
use miningsim::tools::Tool;

#[test]
fn dig_request_round_trips() {
    let original = DigRequest { target: IVec2::new(7, 12) };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: DigRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded.target, original.target);
}

#[test]
fn buy_tool_request_round_trips() {
    let original = BuyToolRequest { tool: Tool::Pickaxe };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: BuyToolRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded.tool, original.tool);
}

#[test]
fn smelt_all_request_round_trips() {
    let original = SmeltAllRequest { ore: OreKind::Silver };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: SmeltAllRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded.ore, original.ore);
}

#[test]
fn collect_all_request_round_trips() {
    let original = CollectAllRequest;
    let bytes = bincode::serialize(&original).expect("ser");
    let _decoded: CollectAllRequest = bincode::deserialize(&bytes).expect("de");
    // unit struct; existence of decoded value is success
}

#[test]
fn sell_all_request_round_trips() {
    let original = SellAllRequest;
    let bytes = bincode::serialize(&original).expect("ser");
    let _decoded: SellAllRequest = bincode::deserialize(&bytes).expect("de");
}
```

Add `bincode = "1.3"` to `[dev-dependencies]` in `Cargo.toml` for these tests:
```toml
[dev-dependencies]
bincode = "1.3"
```

- [ ] **Step 3: Run tests — expect compile failure**

```bash
cargo test --test net_events 2>&1 | tail -10
```

- [ ] **Step 4: Implement `src/systems/net_events.rs`**

```rust
use bevy::math::IVec2;
use bevy::prelude::Event;
use serde::{Deserialize, Serialize};

use crate::items::OreKind;
use crate::tools::Tool;

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DigRequest { pub target: IVec2 }

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BuyToolRequest { pub tool: Tool }

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SmeltAllRequest { pub ore: OreKind }

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectAllRequest;

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SellAllRequest;
```

- [ ] **Step 5: Run tests — expect 5/5 passing**

```bash
cargo test --test net_events 2>&1 | tail -10
```

- [ ] **Step 6: Full regression**

```bash
cargo test 2>&1 | grep "test result"
```
Expected: 96 + 5 = 101 tests passing.

- [ ] **Step 7: Commit**

```bash
git add src/systems/net_events.rs src/systems/mod.rs tests/net_events.rs Cargo.toml
git commit --author="wes2000 <whannasch@gmail.com>" -m "Add net_events module: client-fired request events with serde round-trip tests"
```

---

## Task 9: Replication setup in MultiplayerPlugin

**Files:**
- Modify: `src/systems/net_plugin.rs`

- [ ] **Step 1: Register replicated component types and client events**

Update `MultiplayerPlugin::build`:
```rust
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

use crate::components::Player;
use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::systems::net_events::{
    BuyToolRequest, CollectAllRequest, DigRequest, SellAllRequest, SmeltAllRequest,
};
use crate::tools::OwnedTools;

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RepliconPlugins);
        app.add_plugins(RepliconRenetPlugins);

        // Replicated components — host writes, all clients read
        app.replicate::<Player>()
           .replicate::<Grid>()                // Resource, not Component — verify replicon supports this; if not, wrap in a component on a singleton entity
           .replicate::<SmelterState>()
           .replicate::<Money>()
           .replicate::<Inventory>()
           .replicate::<OwnedTools>()
           .replicate::<Transform>();

        // Client-fired events
        app.add_client_event::<DigRequest>(ChannelKind::Ordered);
        app.add_client_event::<BuyToolRequest>(ChannelKind::Ordered);
        app.add_client_event::<SmeltAllRequest>(ChannelKind::Ordered);
        app.add_client_event::<CollectAllRequest>(ChannelKind::Ordered);
        app.add_client_event::<SellAllRequest>(ChannelKind::Ordered);

        // Mode-specific startup wiring lands in Task 12 (start_net_mode_system).
    }
}
```

**Note on `Grid` as a Resource:** if `bevy_replicon 0.30` doesn't support replicating Resources directly (only Components), the workaround is to wrap Grid in a singleton entity with a `GridContainer { grid: Grid }` component. Investigate during implementation; if the workaround is needed, that's a small additional refactor (5-10 lines) and a note in the task report.

- [ ] **Step 2: Build + test**

```bash
cargo build 2>&1 | tail -20
cargo test 2>&1 | grep "test result"
```
Expected: 101 tests still passing. Build may surface API mismatches with `bevy_replicon 0.30` — adapt names/signatures to the actual crate API and document changes.

- [ ] **Step 3: Commit**

```bash
git add src/systems/net_plugin.rs src/components.rs Cargo.toml
git commit --author="wes2000 <whannasch@gmail.com>" -m "MultiplayerPlugin: register replicated components and client events"
```

(Include `src/components.rs` and `Cargo.toml` if any auxiliary changes were needed for replicon API quirks.)

---

## Task 10: Server-side request handlers

**Files:**
- Modify: `src/systems/net_plugin.rs`

Five `Update` systems on the host that consume the corresponding `EventReader<XxxRequest>` and apply mutations server-side. Each handler routes the request to the player entity that fired it.

- [ ] **Step 1: Implement handlers**

In `src/systems/net_plugin.rs`, add:

```rust
use crate::components::Player;
use crate::dig::{self, DigStatus};
use crate::economy::{self, Money};
use crate::items::{ItemKind, OreKind};
use crate::processing::{self, SmelterState};
use crate::systems::coords::{tile_center_world, world_to_tile};
use crate::tools::{self, OwnedTools};

pub fn handle_dig_requests(
    mut events: EventReader<FromClient<DigRequest>>,
    mut grid: ResMut<Grid>,
    mut commands: Commands,
    mut player_q: Query<(Entity, &Transform, &OwnedTools, &mut Inventory), With<Player>>,
    /* ... entity-by-client-id lookup helper ... */
) {
    for FromClient { client_id, event } in events.read() {
        // Find the Player entity for this client_id. Replicon provides a way to map
        // ClientId to its connection's Player entity; either store that mapping ourselves
        // when spawning players in net_player.rs, or use replicon's built-in helper.

        let Some((player_entity, player_xf, owned, mut inv)) = find_player_for_client(*client_id, &mut player_q) else { continue };

        // Same gameplay logic as single-player dig:
        let player_tile = world_to_tile(player_xf.translation.truncate());
        if !dig::dig_target_valid(player_tile, event.target, /* DIG_REACH_TILES */ 2, &grid) { continue; }
        let Some(tile) = grid.get(event.target.x, event.target.y).copied() else { continue };
        let Some(tool) = tools::best_applicable_tool(owned, tile.layer) else { continue };
        let result = dig::try_dig(&mut grid, event.target, tool);
        if result.status == DigStatus::Broken {
            // Spawn OreDrop server-side; replicates to clients automatically
            if let Some(ore) = result.ore {
                let world_pos = tile_center_world(event.target);
                commands.spawn((
                    crate::components::OreDrop { item: ItemKind::Ore(ore) },
                    Sprite { /* ... */ },
                    Transform::from_translation(world_pos.extend(4.0)),
                    Replication,                  // mark for replication
                ));
            }
            // Mark chunk dirty (chunk lifecycle handles re-mesh on each peer locally)
        }
    }
}

pub fn handle_buy_tool_requests(
    mut events: EventReader<FromClient<BuyToolRequest>>,
    mut player_q: Query<(Entity, &mut Money, &mut OwnedTools), With<Player>>,
) {
    for FromClient { client_id, event } in events.read() {
        let Some((_, mut money, mut owned)) = find_player_for_client_buy(*client_id, &mut player_q) else { continue };
        let _ = economy::try_buy(event.tool, &mut money, &mut owned);
    }
}

pub fn handle_smelt_all_requests(
    mut events: EventReader<FromClient<SmeltAllRequest>>,
    mut player_q: Query<&mut Inventory, With<Player>>,
    mut smelter_q: Query<&mut SmelterState>,
) {
    for FromClient { client_id, event } in events.read() {
        let Some(mut inv) = find_inventory_for_client(*client_id, &mut player_q) else { continue };
        let Ok(mut state) = smelter_q.get_single_mut() else { continue };
        let count = inv.get(ItemKind::Ore(event.ore));
        if count == 0 || processing::is_busy(&state) { continue; }
        inv.remove(ItemKind::Ore(event.ore), count);
        processing::start_smelting(&mut state, event.ore, count);
    }
}

pub fn handle_collect_all_requests(
    mut events: EventReader<FromClient<CollectAllRequest>>,
    mut player_q: Query<&mut Inventory, With<Player>>,
    mut smelter_q: Query<&mut SmelterState>,
) {
    for FromClient { client_id, .. } in events.read() {
        let Some(mut inv) = find_inventory_for_client(*client_id, &mut player_q) else { continue };
        let Ok(mut state) = smelter_q.get_single_mut() else { continue };
        let drained = processing::collect_output(&mut state);
        for (ore, n) in drained {
            inv.add(ItemKind::Bar(ore), n);
        }
    }
}

pub fn handle_sell_all_requests(
    mut events: EventReader<FromClient<SellAllRequest>>,
    mut player_q: Query<(&mut Inventory, &mut Money), With<Player>>,
) {
    for FromClient { client_id, .. } in events.read() {
        let Some((mut inv, mut money)) = find_inventory_money_for_client(*client_id, &mut player_q) else { continue };
        economy::sell_all(&mut inv, &mut money);
    }
}

// Helper: maps replicon ClientId to the Player entity for that connection.
// Implementation depends on whether you store the mapping yourself (in a Resource keyed by ClientId)
// or use a replicon-provided lookup. Add the appropriate helper signature(s) above.
```

The exact `EventReader<FromClient<XxxRequest>>` API name is replicon-specific — verify against `bevy_replicon 0.30` docs and adapt.

- [ ] **Step 2: Register handlers in MultiplayerPlugin**

```rust
app.add_systems(Update, (
    handle_dig_requests,
    handle_buy_tool_requests,
    handle_smelt_all_requests,
    handle_collect_all_requests,
    handle_sell_all_requests,
).run_if(server_running));   // or whatever replicon's host-only condition is
```

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 101 tests still passing. Handlers are registered but not yet exercised (no clients fire events yet).

- [ ] **Step 4: Commit**

```bash
git add src/systems/net_plugin.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "MultiplayerPlugin: server-side handlers for dig/buy/smelt/collect/sell requests"
```

---

## Task 11: Branch UI handlers on NetMode (clients fire events)

**Files:**
- Modify: `src/systems/shop_ui.rs`
- Modify: `src/systems/smelter.rs`
- Modify: `src/systems/player.rs`

In multiplayer mode, the LocalPlayer's UI handlers fire request events instead of mutating state directly. In single-player mode, behavior is unchanged.

- [ ] **Step 1: Branch shop_ui button handler**

In `src/systems/shop_ui.rs`, `handle_shop_buttons_system`:

```rust
pub fn handle_shop_buttons_system(
    ui_open: Res<ShopUiOpen>,
    interaction_q: Query<(&Interaction, &ShopButtonKind), Changed<Interaction>>,
    mut local: Single<(&mut Inventory, &mut Money, &mut OwnedTools), With<LocalPlayer>>,
    mut sell_writer: EventWriter<SellAllRequest>,
    mut buy_writer: EventWriter<BuyToolRequest>,
    net_mode: Res<crate::net::NetMode>,
) {
    if !ui_open.0 { return; }
    let multiplayer = matches!(*net_mode, crate::net::NetMode::Host { .. } | crate::net::NetMode::Client { .. });
    for (interaction, kind) in interaction_q.iter() {
        if *interaction != Interaction::Pressed { continue; }
        match kind {
            ShopButtonKind::SellAll => {
                if multiplayer {
                    sell_writer.send(SellAllRequest);
                } else {
                    let (inv, money, _) = local.into_inner();
                    economy::sell_all(inv, money);
                }
            }
            ShopButtonKind::Buy(tool) => {
                if multiplayer {
                    buy_writer.send(BuyToolRequest { tool: *tool });
                } else {
                    let (_, money, owned) = local.into_inner();
                    let _ = economy::try_buy(*tool, money, owned);
                }
            }
        }
    }
}
```

(Note: `Single<>` borrowing semantics — if you need both branches in the loop, structure as `if let Ok((mut inv, ...))` outside the per-event loop OR use `Query<>` instead and call `.get_single_mut()` inside.)

- [ ] **Step 2: Branch smelter button handler**

Same pattern in `src/systems/smelter.rs`, `handle_smelter_buttons_system`. Single-player path mutates local Inventory + SmelterState directly; multiplayer path fires `SmeltAllRequest` / `CollectAllRequest`.

- [ ] **Step 3: Branch dig_input system**

In `src/systems/player.rs`, `dig_input_system`:

In single-player mode (existing behavior), call `dig::try_dig(&mut grid, target_tile, tool)` and handle the result locally.

In multiplayer mode, fire `DigRequest { target: target_tile }` instead. Skip the local Grid mutation entirely. The host will validate, mutate the replicated Grid, and the change comes back to the client via replication.

```rust
let net_mode = /* Res<NetMode> */;
let multiplayer = matches!(*net_mode, NetMode::Host { .. } | NetMode::Client { .. });

if multiplayer {
    dig_writer.send(DigRequest { target: target_tile });
    cooldown.0.reset();   // local cooldown still applies for input pacing
    return;
}

// single-player: existing flow
let result = dig::try_dig(&mut grid, target_tile, tool);
// ... existing match on result.status ...
```

Important: in multiplayer, the OreDrop spawn moves to the server-side `handle_dig_requests`. Don't spawn it on the client.

- [ ] **Step 4: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 101 tests still passing. Single-player gameplay unchanged. In multiplayer, button presses now fire events that the (Task 10) host handlers consume.

- [ ] **Step 5: Commit**

```bash
git add src/systems/shop_ui.rs src/systems/smelter.rs src/systems/player.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "UI handlers branch on NetMode: multiplayer fires events, single-player mutates locally"
```

---

## Task 12: Player lifecycle systems (`net_player.rs`) + start_net_mode

**Files:**
- Create: `src/systems/net_player.rs`
- Modify: `src/systems/mod.rs`
- Modify: `src/systems/net_plugin.rs`

- [ ] **Step 1: Create net_player.rs**

```rust
use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::components::{LocalPlayer, Player, RemotePlayer};
use crate::economy::Money;
use crate::inventory::Inventory;
use crate::tools::OwnedTools;
use crate::net::NetMode;

/// Server-side: when replicon reports a new client connecting, spawn a Player entity
/// with default per-player components. Tag it so we can route requests later.
pub fn spawn_player_for_new_clients(
    mut commands: Commands,
    mut events: EventReader<ServerEvent>,    // replicon's server-side event stream — verify name in 0.30
) {
    for event in events.read() {
        if let ServerEvent::ClientConnected { client_id } = event {
            commands.spawn((
                Player,
                ConnectedClient { id: *client_id },        // local marker for client-id lookup
                Money::default(),
                Inventory::default(),
                OwnedTools::default(),
                Transform::from_translation(/* spawn pocket position */ Vec3::ZERO),
                Sprite { color: Color::srgb(0.95, 0.55, 0.20), custom_size: Some(Vec2::splat(12.0)), ..default() },
                Replication,
            ));
            info!("client {:?} connected — spawned Player entity", client_id);
        }
    }
}

#[derive(Component)]
pub struct ConnectedClient { pub id: ClientId }

pub fn despawn_player_for_disconnected_clients(
    mut commands: Commands,
    mut events: EventReader<ServerEvent>,
    player_q: Query<(Entity, &ConnectedClient)>,
) {
    for event in events.read() {
        if let ServerEvent::ClientDisconnected { client_id, .. } = event {
            for (entity, conn) in player_q.iter() {
                if conn.id == *client_id {
                    commands.entity(entity).despawn();
                    info!("client {:?} disconnected — despawned Player", client_id);
                }
            }
        }
    }
}

/// Client-side: when our replicated Player entity arrives, tag it LocalPlayer.
/// Other Player entities get RemotePlayer.
pub fn mark_local_player_on_arrival(
    mut commands: Commands,
    new_players: Query<Entity, (With<Player>, Without<LocalPlayer>, Without<RemotePlayer>)>,
    client_id: Res<RepliconClient>,           // verify name in 0.30
    player_q: Query<(Entity, &ConnectedClient)>,
) {
    let Some(my_id) = client_id.id() else { return };
    for entity in new_players.iter() {
        if let Some((_, conn)) = player_q.get(entity).ok() {
            if conn.id == my_id {
                commands.entity(entity).insert(LocalPlayer);
            } else {
                commands.entity(entity).insert(RemotePlayer);
            }
        }
    }
}

/// Set sprite color based on Local/Remote status.
pub fn sync_remote_player_visuals(
    mut q: Query<(&mut Sprite, Has<LocalPlayer>, Has<RemotePlayer>), With<Player>>,
) {
    for (mut sprite, is_local, is_remote) in q.iter_mut() {
        sprite.color = if is_local {
            Color::srgb(0.30, 0.60, 0.90)        // blue
        } else if is_remote {
            Color::srgb(0.95, 0.55, 0.20)        // orange
        } else {
            sprite.color                          // unmarked — leave alone
        };
    }
}

/// Mode-specific startup: open server (host) or connect to address (client).
pub fn start_net_mode_system(/* … */) {
    // Implementation depends on bevy_replicon_renet 0.5's API for spawning a server / client.
    // Use Res<NetMode>, instantiate the appropriate transport handle, insert it as a Resource.
    // The actual API surface (RenetClient::new, RenetServer::new, ServerSocketAddress, etc.) is
    // small but version-specific — reference bevy_replicon_renet 0.5 docs.
}
```

**Note**: the spec calls `mark_local_player_on_arrival` — its real implementation depends on how `bevy_replicon 0.30` exposes "current client's network ID." If the host doesn't get a `RepliconClient::id()`, see the spec's resolved open question: **the host's Player is spawned with `LocalPlayer` directly in `setup_world`**, not via this arrival-hook. The arrival hook only runs on actual clients.

- [ ] **Step 2: Register module + systems**

In `src/systems/mod.rs`:
```rust
pub mod net_player;
```

In `src/systems/net_plugin.rs`, add to `MultiplayerPlugin::build`:
```rust
app.add_systems(Startup, net_player::start_net_mode_system);

app.add_systems(Update, (
    net_player::spawn_player_for_new_clients,
    net_player::despawn_player_for_disconnected_clients,
).run_if(server_running));

app.add_systems(Update, (
    net_player::mark_local_player_on_arrival,
).run_if(client_connected));     // or whatever replicon's client-side condition is

app.add_systems(Update, net_player::sync_remote_player_visuals);
```

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -20
cargo test 2>&1 | grep "test result"
```
Expected: 101 tests still passing.

- [ ] **Step 4: Commit**

```bash
git add src/systems/net_player.rs src/systems/mod.rs src/systems/net_plugin.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Player lifecycle: spawn-on-connect, mark Local/Remote, render visuals"
```

---

## Smoke-test checkpoint #3 (after Task 12)

Two-window test using the spec's full manual exit-criteria checklist (single-player regression, host launches alone, two-window co-op, late-join, save/load isolation).

```bash
# Window A:
cargo run -- host

# Window B (in another terminal):
cargo run -- join 127.0.0.1:5000
```

Tick each item from the spec's [Manual playtest exit-criteria](../specs/2026-04-18-milestone-4-co-op-networking-design.md). If any items fail, file a fix-task before Task 13.

---

## Task 13: Disconnect handling

**Files:**
- Modify: `src/systems/net_player.rs`

The host-side disconnect handling (despawn player) is in place from Task 12. This task adds the client-side: if the host drops, log + cleanly exit.

- [ ] **Step 1: Add `exit_on_host_disconnect` system**

```rust
pub fn exit_on_host_disconnect(
    mut events: EventReader<ClientEvent>,    // replicon's client-side event stream — verify name in 0.30
    mut exit: EventWriter<AppExit>,
) {
    for event in events.read() {
        if let ClientEvent::Disconnected { reason } = event {
            error!("disconnected from host: {:?}", reason);
            exit.send(AppExit::Success);
        }
    }
}
```

- [ ] **Step 2: Register in MultiplayerPlugin**

```rust
app.add_systems(Update, net_player::exit_on_host_disconnect.run_if(client_connected));
```

- [ ] **Step 3: Build + test**

```bash
cargo build 2>&1 | tail -10
cargo test 2>&1 | grep "test result"
```
Expected: 101 tests still passing.

- [ ] **Step 4: Manual disconnect test**

```bash
# Terminal A:
cargo run -- host

# Terminal B:
cargo run -- join 127.0.0.1:5000
```

In window A, close the window. Window B should log `error: disconnected from host: ...` and exit cleanly within ~1 second.

Restart window A as host. Reconnect window B. Same flow works again.

- [ ] **Step 5: Commit**

```bash
git add src/systems/net_player.rs src/systems/net_plugin.rs
git commit --author="wes2000 <whannasch@gmail.com>" -m "Client-side: clean exit on host disconnect"
```

---

## Task 14: Final playtest, roadmap update, merge to main

- [ ] **Step 1: Run full test suite**

```bash
cargo test
```
Expected: ~101 tests passing across all suites.

- [ ] **Step 2: Manual exit-criteria walkthrough**

Run the full multi-window playtest from the spec:

Single-player regression:
- [ ] `cargo run` → fresh world, F5/F9/AppExit work, M3 behaviors work.

Host-launches-alone:
- [ ] `cargo run -- host` → port 5000 open, host plays solo, clean shutdown.

Two-window co-op (full checklist):
- [ ] Player visuals (local blue, remote orange).
- [ ] Smooth movement across windows.
- [ ] Adjacent dig works for both.
- [ ] Same-tile conflict — exactly one wins.
- [ ] Per-player inventory (A's copper count diverges from B's).
- [ ] Smelter shared trust-based (B can collect A's deposit).
- [ ] Per-player money (independent coin counts).
- [ ] Per-player tool unlocks.

Late-join:
- [ ] Host plays solo, then client joins to in-progress world.

Disconnect:
- [ ] Host drop → client clean exit.
- [ ] Client drop → host continues solo.

Save/load isolation:
- [ ] F5/F9 do nothing in multiplayer; `save.ron` untouched.

Stability:
- [ ] 30-minute mixed session, no crashes.

- [ ] **Step 3: Append a Milestone 4 section to `docs/roadmap.md`**

```markdown
## Playtest Results — Milestone 4 (YYYY-MM-DD)

Exit-criteria met: [summary]

**What felt good:**
- ...

**What felt off:**
- ...

**Decisions for milestone 4.1+ / 5:**
- ...
```

- [ ] **Step 4: Commit playtest notes**

```bash
git add docs/roadmap.md
git commit --author="wes2000 <whannasch@gmail.com>" -m "Record milestone 4 playtest results"
```

- [ ] **Step 5: Merge to main + push**

```bash
git checkout main
git merge --no-ff milestone-4 -m "Merge milestone-4: 2-player co-op networking MVP"
git push origin main
git branch -d milestone-4
```

- [ ] **Step 6: Final code review (optional but recommended)**

Dispatch the `superpowers:code-reviewer` subagent against the merged `main` HEAD. Capture any "fix before M4.1 or M5" callouts before starting the next brainstorm.

Milestone 4 complete.
