# Milestone 4 — Online Co-op Networking (MVP) Design Spec

**Date:** 2026-04-18
**Status:** Draft (awaiting spec review)
**Parent roadmap:** [../../roadmap.md](../../roadmap.md)
**Prior milestone:** [save/load mini-milestone](./2026-04-18-save-load-design.md)

## Purpose

Retrofit **2-player online co-op** onto the working singleplayer game so
two friends can dig the same property together over a direct IP
connection. **Minimum viable** scope for this milestone: 2 players,
direct IP, authoritative host, full per-component replication via
`bevy_replicon`, late-join via initial-state snapshot, graceful
disconnect (no reconnect). The roadmap's eventual end-state of "4
players, host/join via Steam/LAN/IP" decomposes into follow-on
mini-milestones (M4.1+) once this MVP is shipping.

## Scope

### In scope (MVP)
- **2 players per session** (host + 1 client; host is also a player).
- **Direct IP only.** CLI args:
  - `cargo run` → single-player (existing behavior — save/load active).
  - `cargo run -- host` → start as host on `DEFAULT_PORT = 5000`.
  - `cargo run -- host <port>` → start as host on `<port>`.
  - `cargo run -- join <addr>` → start as client connecting to `<addr>`
    (e.g. `127.0.0.1:5000`).
- **`bevy_replicon` for replication and transport.** Authoritative-server model.
- **Per-player components** (refactored from M3 Resources):
  `Money`, `Inventory`, `OwnedTools` migrate from `Resource` to
  `Component on Player entity`. New `LocalPlayer` and `RemotePlayer`
  marker components.
- **Shared resources** (single instance, replicated):
  `Grid`, `SmelterState` (on the singleton Smelter entity), the world
  itself.
- **Replicated entities and components:**
  - `Grid` resource → replicated.
  - `Player` entities (each with their own `Money`, `Inventory`,
    `OwnedTools`, `Transform`, `Velocity`, `Facing`, `Sprite`) → replicated.
  - `Smelter` entity + `SmelterState` → replicated.
  - `OreDrop` entities (in-flight drops) → replicated.
- **Client→server events:** `DigRequest`, `BuyToolRequest`,
  `SmeltAllRequest`, `CollectAllRequest`, `SellAllRequest`. Clients fire
  these instead of mutating shared state directly.
- **Smelter sharing:** trust-based. Both players can deposit into the
  same Smelter; whoever clicks `Collect All` gets all bars currently
  in the output. No per-depositor tracking.
- **Movement model:** client-side prediction for own player (snappy
  WASD); host validates and rebroadcasts; remote players show via
  replicated Transform.
- **Player visuals:** local player blue (existing), remote players
  orange. 12×12px squares. No name tags or customization.
- **Late-join:** when a client connects, replicon streams the host's
  full game state. Joining client sees the in-progress world.
- **Graceful disconnect:**
  - Host disconnects → client logs error, exits cleanly.
  - Client disconnects → host despawns the client's player entity, continues solo.
  - **No mid-session reconnect.**
- **`HashMap`/`HashSet` → `BTreeMap`/`BTreeSet` migration** on `Inventory`,
  `OwnedTools`, `SmelterState.output` (per save/load review). Required for
  deterministic serialization that replicon can diff cleanly.
- **`SaveLoadPlugin` and `MultiplayerPlugin` are mutually exclusive.** Save/load
  is single-player only; multiplayer sessions never write `save.ron`.

### Out of scope (deferred to M4.1+)
- 3+ players (will require interest management and scaling work).
- Steam Networking, LAN broadcast / discovery, lobby UI, friend invites.
- Mid-session reconnect / drop-in.
- Per-player customization (names, colors beyond local/remote).
- Anti-cheat / rate-limiting / authority validation beyond basic checks.
- Bandwidth optimization, delta encoding, compression.
- Loading a single-player save into a multiplayer session.
- Save/load INSIDE a multiplayer session.
- More than one Smelter (M5 brings conveyors + multi-machine).
- Schema-version handshake at connect time (assumed: peers run compatible builds).
- Lag compensation / rollback / interpolation polish.
- Spectator / observer slots.
- NAT traversal / hole punching.

### Explicitly not designed for
- Cheaters / hostile players (LAN/friend trust model).
- Cross-platform binary compatibility (peers should build from same git commit).
- Headless dedicated server (host is always a player).

## Target platform & tech

Unchanged from prior milestones:
- Bevy 0.15.x (pinned). Rust stable.
- Top-down 2D, single property, two players.
- Desktop (Windows / macOS / Linux).
- New deps: `bevy_replicon` (latest 0.15-compatible), `bevy_replicon_renet` (transport adapter).

## Key design decisions

| Decision | Choice | Why |
|---|---|---|
| Player count | **2 (MVP)** | Going from 0 → 4-player co-op in one cycle is a multi-month effort. 2-player MVP unblocks "play with one friend over IP" in this cycle; M4.1+ scales up. |
| Coin / inventory model | **Per-player** (Money + Inventory + OwnedTools) | Independent progression keeps each player's economy distinct; pairs naturally with the `LocalPlayer` component pattern. |
| Smelter sharing | **Shared, trust-based collection** | Both players deposit into one Smelter; whoever clicks `Collect All` gets all current output. Communication-driven cooperation. |
| Netcode crate | **`bevy_replicon`** | Bevy-native, authoritative-server, automatic component replication, active development. Hand-rolling on `renet` directly would 2× the code volume. |
| Authority model | **Authoritative host** with per-event request handling | Standard for trust-required state (Grid, machine state, Money). Client-side prediction only for own movement (no authority on shared state). |
| Connection model | **Direct IP only** for M4 | No matchmaking / Steam / LAN scan in MVP. CLI args drive mode selection. |
| Refactor sequencing | **Sequential per-resource migration** (Money, then Inventory, then OwnedTools) | Each step keeps gameplay playable. Lower risk than a single atomic refactor. |
| Plugin organization | **Single `MultiplayerPlugin` that branches by `NetMode` internally** | Simpler than parallel `HostPlugin`/`ClientPlugin`. Mutually exclusive with `SaveLoadPlugin` based on `Res<NetMode>`. |
| Save/load × multiplayer | **Mutually exclusive plugins** | Save/load is single-player only in M4. Multiplayer-with-saves is M4.1+. |
| `HashMap` → `BTreeMap` | **Migrate** `Inventory.counts`, `OwnedTools.0`, `SmelterState.output` | Deterministic serialization required for replicon diff/delta. Flagged as M4-required by the save/load final review. |

## Architecture

### Module / file layout

```
Cargo.toml                       # MODIFY: + bevy_replicon, + bevy_replicon_renet, + bincode (for wire format)
src/
  net.rs                         # NEW: pure — NetMode, DEFAULT_PORT, parse_args, CliParseError
  components.rs                  # MODIFY: + LocalPlayer, RemotePlayer markers
  inventory.rs                   # MODIFY: drop Resource derive; HashMap → BTreeMap; ItemKind needs Ord
  economy.rs                     # MODIFY: drop Resource derive on Money (becomes Component)
  tools.rs                       # MODIFY: drop Resource derive on OwnedTools; HashSet → BTreeSet; Tool needs Ord
  processing.rs                  # MODIFY: SmelterState.output: HashMap → BTreeMap; OreKind needs Ord
  save.rs                        # MODIFY: bump SAVE_VERSION to 2 (encoding shifted with BTreeMap)
  systems/
    setup.rs                     # MODIFY: spawn local Player with Money/Inventory/OwnedTools/LocalPlayer components instead of inserting Resources
    hud.rs                       # MODIFY: query Single<…, With<LocalPlayer>> instead of Res<…>
    shop.rs, shop_ui.rs          # MODIFY: button handlers — single-player mutates LocalPlayer's components; multiplayer fires events
    smelter.rs                   # MODIFY: same — events vs direct mutation by NetMode
    save_load.rs                 # MODIFY: collect/apply against LocalPlayer's components; SaveLoadPlugin wraps the existing 4 systems
    net_plugin.rs                # NEW: MultiplayerPlugin — replicon setup, mode dispatch, event handler registration
    net_events.rs                # NEW: DigRequest, BuyToolRequest, SmeltAllRequest, CollectAllRequest, SellAllRequest
    net_replicate.rs             # NEW: replicated component registration helpers
    net_player.rs                # NEW: server-side player spawn-on-connect; client-side LocalPlayer assignment; RemotePlayer rendering
  app.rs                         # MODIFY: branch on Res<NetMode> — load SaveLoadPlugin XOR MultiplayerPlugin
  main.rs                        # MODIFY: parse CLI args via net::parse_args; insert NetMode resource
  lib.rs                         # MODIFY: pub mod net; pub mod systems::{net_plugin, net_events, net_replicate, net_player}
tests/
  inventory.rs, economy.rs, tools.rs   # MODIFY: BTreeMap/BTreeSet API; same test bodies otherwise
  processing.rs                  # MODIFY: BTreeMap on output; same test bodies otherwise
  save.rs                        # MODIFY: SAVE_VERSION = 2; expected ron output may shift slightly
  net.rs                         # NEW: parse_args matrix
  net_events.rs                  # NEW: serde round-trip per event type
```

**Module boundary discipline preserved:**
- `net.rs` is pure (no Bevy systems / queries).
- All Bevy systems live in `systems/net_*.rs`.
- Existing pure modules (`grid`, `dig`, `terrain_gen`, `items`, `processing`, `tools`, `economy`, `inventory`, `coords`, `save`) gain only derive-level changes (Component, Ord) and one type swap (HashMap → BTreeMap). No new logic in pure modules.

## Components / modules in detail

### `net.rs` (new, pure)

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
pub fn parse_args(args: &[String]) -> Result<NetMode, CliParseError>;
```

### `components.rs` (modified)

```rust
/// The player entity controlled by this client. Exactly one in any session.
#[derive(Component)]
pub struct LocalPlayer;

/// A player entity replicated from another peer. Rendered with a different sprite color.
#[derive(Component)]
pub struct RemotePlayer;
```

### Modified pure modules

- **`inventory.rs`**: `Inventory` loses `Resource` derive, gains nothing else.
  `counts: HashMap<ItemKind, u32>` → `BTreeMap<ItemKind, u32>`. `add`/`remove`/`get`
  signatures unchanged.
- **`economy.rs`**: `Money(pub u32)` loses `Resource` derive (already derives `Component` for the Player-attached usage).
- **`tools.rs`**: `OwnedTools(pub HashSet<Tool>)` → `OwnedTools(pub BTreeSet<Tool>)`. `Tool` gains `Ord, PartialOrd` derives. `OwnedTools` loses `Resource` derive.
- **`processing.rs`**: `SmelterState.output: HashMap<OreKind, u32>` → `BTreeMap<OreKind, u32>`. `OreKind` gains `Ord, PartialOrd`. `SmelterState` keeps `Component` derive.
- **`items.rs`**: `OreKind`, `ItemKind` gain `Ord, PartialOrd` derives.

`Component` derives required on: `Money`, `Inventory`, `OwnedTools` (new — these become per-player components).

### `systems/setup.rs` (modified)

`setup_world` no longer inserts `Money::default()`, `Inventory::default()`, `OwnedTools::default()` as Resources. Instead, the local Player entity gets them as Components:

```rust
commands.spawn((
    Player,
    LocalPlayer,
    Velocity::default(),
    Facing::default(),
    Money::default(),
    Inventory::default(),
    OwnedTools::default(),
    Sprite { color: PLAYER_LOCAL_COLOR, custom_size: Some(Vec2::splat(12.0)), ..default() },
    Transform::from_translation(player_world.extend(10.0)),
));
```

`Smelter`, `Shop`, `Camera` spawns unchanged. `SmelterState` is still on the Smelter entity. `Grid` is still a Resource (shared world state, replicates as a Resource via replicon).

### `systems/hud.rs` (modified)

All systems that read `Res<Inventory>` / `Res<Money>` / `Res<OwnedTools>` change to:

```rust
fn update_money_text_system(
    local: Query<&Money, (With<LocalPlayer>, Changed<Money>)>,
    mut text_q: Query<&mut Text, With<MoneyText>>,
) { ... }
```

Same pattern for the inventory popup and tools section.

### `systems/shop_ui.rs`, `systems/smelter.rs` (modified)

Button handlers branch on `Res<NetMode>`:
- `SinglePlayer` → mutate the LocalPlayer's components directly (existing logic, just relocated from Resources to Components).
- `Host` / `Client` → fire the corresponding event (`BuyToolRequest`, `SellAllRequest`, `SmeltAllRequest`, `CollectAllRequest`).

### `systems/save_load.rs` (modified, wrapped into a Plugin)

`collect()` and `apply()` continue to operate on `Inventory`/`Money`/`OwnedTools` — but now sourced from the LocalPlayer's components (queried via `Single<&Inventory, With<LocalPlayer>>` etc.).

`SAVE_VERSION` bumps to **2** because the underlying serialized encoding shifts with the `HashMap` → `BTreeMap` migration. Pre-existing v1 saves are silently discarded as before.

The four save_load systems get pulled into `pub struct SaveLoadPlugin;` which `app.rs` mounts conditionally.

### `systems/net_plugin.rs` (new)

```rust
pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_replicon::prelude::RepliconPlugins);
        app.add_plugins(bevy_replicon_renet::RepliconRenetPlugins);

        // Replicate components
        app.replicate::<Grid>()
            .replicate::<SmelterState>()
            .replicate::<Money>()
            .replicate::<Inventory>()
            .replicate::<OwnedTools>()
            .replicate::<Transform>()
            .replicate::<Player>();

        // Client-fired events
        app.add_client_event::<DigRequest>(ChannelKind::Ordered);
        app.add_client_event::<BuyToolRequest>(ChannelKind::Ordered);
        app.add_client_event::<SmeltAllRequest>(ChannelKind::Ordered);
        app.add_client_event::<CollectAllRequest>(ChannelKind::Ordered);
        app.add_client_event::<SellAllRequest>(ChannelKind::Ordered);

        // Mode-specific startup (open server / connect client)
        app.add_systems(Startup, start_net_mode_system);

        // Server-side handlers (run only on host)
        app.add_systems(Update, (
            handle_dig_requests,
            handle_buy_tool_requests,
            handle_smelt_all_requests,
            handle_collect_all_requests,
            handle_sell_all_requests,
            spawn_player_for_new_clients,
            despawn_player_for_disconnected_clients,
        ).run_if(is_host));

        // Client-side: identify our Player, mark RemotePlayer entities
        app.add_systems(Update, (
            mark_local_player_on_arrival,
            mark_remote_players,
            sync_remote_player_visuals,
        ).run_if(is_client_or_host));   // both because host-as-player needs LocalPlayer marker too

        // Client-side: log + exit on disconnect
        app.add_systems(Update, exit_on_host_disconnect.run_if(is_client));
    }
}
```

### `systems/net_events.rs` (new)

```rust
#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct DigRequest { pub target: IVec2 }

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct BuyToolRequest { pub tool: Tool }

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct SmeltAllRequest { pub ore: OreKind }

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct CollectAllRequest;

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct SellAllRequest;
```

Each event also carries the originating `ClientId` (added by replicon's framing, not in the event type itself).

### `systems/net_player.rs` (new)

- `start_net_mode_system` — reads `Res<NetMode>`. On `Host { port }`, opens a replicon server. On `Client { addr }`, connects. On `SinglePlayer`, no-op (unreachable since this plugin isn't loaded in single-player).
- `spawn_player_for_new_clients` (host only) — when replicon reports a new client connection, spawn a Player entity with default components, tagged with the client's ID for request routing.
- `despawn_player_for_disconnected_clients` (host only) — remove the Player entity when its client disconnects.
- `mark_local_player_on_arrival` (client side) — when our Player entity replicates in, attach `LocalPlayer` marker. Other Player entities get `RemotePlayer`.
- `sync_remote_player_visuals` — set sprite color based on `LocalPlayer` vs `RemotePlayer` markers.
- `exit_on_host_disconnect` — listens for the disconnect event; logs and triggers `AppExit`.

### `app.rs` (modified)

```rust
fn build(&self, app: &mut App) {
    // ... existing setup ...
    let net_mode = app.world.resource::<NetMode>().clone();
    match net_mode {
        NetMode::SinglePlayer => app.add_plugins(SaveLoadPlugin),
        NetMode::Host { .. } | NetMode::Client { .. } => app.add_plugins(MultiplayerPlugin),
    };
    // ... rest ...
}
```

### `main.rs` (modified)

```rust
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let net_mode = net::parse_args(&args).unwrap_or_else(|err| {
        eprintln!("CLI parse error: {:?} — falling back to single-player", err);
        NetMode::SinglePlayer
    });

    App::new()
        .insert_resource(net_mode)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "MiningSim".into(), resolution: (1280., 720.).into(), ..default() }),
            ..default()
        }))
        .add_plugins(MiningSimPlugin)
        .run();
}
```

## Data flow

(See Section 3 of this conversation for the six scenarios — startup variants, movement, dig, smelt/collect/sell, conflict resolution, disconnect. Reproduced verbatim in the implementation plan to avoid drift.)

## Cross-cutting invariants

- **Host is the single source of truth** for shared state (Grid, SmelterState, OreDrops on the ground).
- **Per-player components live on the Player entity** owned (server-side) by the host, replicated to all clients. Each Player entity has exactly one set.
- **All shared-state mutations in multiplayer flow through events.** Clients don't write to replicated components directly; they fire requests, host validates and applies, replicon broadcasts results.
- **Client-side prediction is movement-only.** No prediction of dig outcomes, money, inventory, or tool changes — those wait for server confirmation.
- **`SaveLoadPlugin` and `MultiplayerPlugin` never coexist.** Selected by `Res<NetMode>` at app build time.
- **`BTreeMap`/`BTreeSet` everywhere we previously had `HashMap`/`HashSet`** for deterministic serialization that replicon's diff engine can rely on.
- **Same systems run on both peers** but write systems are gated by `.run_if(is_host)`. Read systems (HUD, render) run unconditionally on the local data.

## Edge cases & error handling

(See Section 4 of this conversation for connection failures, authority validation, late-join edge cases, replicon mismatches, save/load × multiplayer interaction, movement divergence, and host-as-player edge cases. Reproduced verbatim in the implementation plan to avoid drift.)

### Explicitly NOT handled in M4
- Reconnect after disconnect.
- Lag compensation / rollback.
- Bandwidth saturation / interest management.
- NAT traversal / port forwarding (requires manual configuration for WAN).
- Schema versioning between net peers.
- Spectators / observers.
- Anti-cheat heuristics, rate limits.
- Loading single-player save into multiplayer.

## Testing approach

### Headless unit tests (cargo test)
- **`net::parse_args`** — full CLI matrix (no args / host / host+port / join / errors).
- **`net_events`** — serde round-trip per event type (DigRequest / BuyToolRequest / SmeltAllRequest / CollectAllRequest / SellAllRequest).
- **`net_replicate`** — assertion that `MultiplayerPlugin`'s replicated component list contains the expected types.
- **Existing pure-module tests** — migrated to `BTreeMap`/`BTreeSet`. Test bodies almost unchanged; only the underlying collection type differs. `OwnedTools::default()` still contains just Shovel; `Inventory.add(...)` API unchanged.
- **`save.rs`** — `SAVE_VERSION = 2` constant updated; existing 9 tests migrate to whatever the v2 encoding looks like.

**Approximate test count target:** ~95 (87 from save/load + ~8 net.rs + a few in net_events).

### Bevy systems
Not unit-tested. The multiplayer flow is validated by manual two-window playtest (loopback `127.0.0.1:5000`).

### Manual playtest exit-criteria

(See Section 5 of this conversation for the full checklist — single-player regression, host-launches-alone, two-window co-op, late-join, disconnect, save/load isolation, stability. Reproduced verbatim in the implementation plan.)

## Open questions deferred to implementation planning

- Exact `bevy_replicon` version to pin against Bevy 0.15. Likely `bevy_replicon = "0.30"` or whatever the 0.15-compatible release tag is at implementation time.
- Whether to use `bincode` or a different binary encoding under replicon's transport. Replicon defaults to bincode — accept the default unless there's a reason to swap.
- Whether `Transform` replication needs interpolation smoothing on remote players. MVP: no — show raw replicated position. Add interpolation in M4.1 if jitter is visible.
- Whether `OreDrop` entities need their own server-side adjudication (vacuum to nearest player). Currently the vacuum_radius logic runs on the host; replication broadcasts result. That's the trust-based default; verify in playtest.
- Whether the host's player gets BOTH `LocalPlayer` AND its server-side identity. Yes — `LocalPlayer` is added by the same system that adds it on clients (when "our" Player entity becomes visible), where the host's "own client" is its own server-side ClientId.

---

**Spec end.** Implementation plan to follow via the writing-plans skill once this spec is approved.
