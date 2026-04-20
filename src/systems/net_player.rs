//! Multiplayer player lifecycle, transport setup, and Grid-replication
//! re-mesh wiring (Task 12 + Task 10 follow-up + Task 13 client-disconnect).
//!
//! Responsibilities:
//!   * `start_net_mode_system`         — bind UDP / connect to host based on `NetMode`.
//!   * `spawn_player_for_new_clients`  — server observer; spawn a Player when a remote
//!                                       client connects.
//!   * `despawn_player_for_disconnected_clients` — server observer; clean up on disconnect.
//!   * `mark_local_player_on_arrival`  — client-side; tag arriving Players with
//!                                       `LocalPlayer` or `RemotePlayer` and add the
//!                                       per-Player components that replication doesn't
//!                                       carry (Sprite, Velocity, Facing).
//!   * `sync_remote_player_visuals`    — paint sprites blue (local) / orange (remote).
//!   * `add_shop_visuals_on_arrival`   — client-side; attach a Sprite to replicated Shop
//!                                       entities (Sprite isn't carried by replicon).
//!   * `add_smelter_visuals_on_arrival` — same, for Smelter.
//!   * `add_ore_drop_visuals_on_arrival` — same, for OreDrop (per-item color).
//!   * `apply_grid_snapshot`           — client-side; apply a one-shot GridSnapshot from
//!                                       the host, replacing the local Grid singleton.
//!   * `apply_tile_changed`            — client-side; apply incremental TileChanged deltas;
//!                                       early-returns if the Grid doesn't exist yet.
//!   * `exit_on_host_disconnect`       — client-side; log + emit `AppExit` when the
//!                                       host drops the connection.
//!
//! Approach for "which Player is mine?": `NetOwner(u64)` carries the renet
//! `client_id`. The server sets it from `NetworkId` on the just-connected client
//! entity. The client stores its own client_id in `LocalClientId` resource at
//! connection time; arriving Players match against that.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use bevy_replicon_renet::netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
    ServerConfig,
};
use bevy_replicon_renet::renet::{ConnectionConfig, RenetClient, RenetServer};
use bevy_replicon_renet::RenetChannelsExt;

use crate::components::{
    AuthoritativeTransform, ChunkDirty, Facing, LocalClientId, LocalPlayer, NetOwner, OwningClient,
    Player, RemotePlayer, TerrainChunk, Velocity,
};
use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::net::NetMode;
use crate::systems::net_events::{ClientPositionUpdate, GridSnapshot, TileChanged};
use crate::tools::OwnedTools;

/// Arbitrary game identifier — both ends MUST agree. ASCII "MINING_1".
pub const PROTOCOL_ID: u64 = 0x4D494E494E475F31;

const LOCAL_PLAYER_COLOR: Color = Color::srgb(0.30, 0.60, 0.90);
const REMOTE_PLAYER_COLOR: Color = Color::srgb(0.95, 0.55, 0.20);
const PLAYER_SPRITE_SIZE: f32 = 12.0;

/// Reserved "client_id" value used for the host's own local Player. Real renet
/// client IDs are `as_millis() as u64`; `u64::MAX` is well above the
/// millis-based range and won't collide with any real client_id (a 0 sentinel
/// would mis-tag the host's player as local on a client whose `as_millis()`
/// happened to return 0, or on a tester that passed 0 explicitly).
pub const HOST_NET_OWNER: u64 = u64::MAX;

/// How often the client ships its authoritative position to the host.
/// 10 Hz = one packet every 100 ms. At max player speed (120 px/s) that's
/// ~12 px = <1 tile of position staleness on the host side — well within
/// `DIG_REACH_TILES = 2.0`'s slack.
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

/// Transport setup. Runs once at Startup. NetMode::SinglePlayer is a no-op
/// (the MultiplayerPlugin isn't even loaded in that case, but be defensive).
pub fn start_net_mode_system(
    mut commands: Commands,
    net_mode: Res<NetMode>,
    channels: Res<RepliconChannels>,
    mut exit: EventWriter<AppExit>,
) {
    match net_mode.clone() {
        NetMode::SinglePlayer => {}
        NetMode::Host { port } => {
            if let Err(e) = setup_host(&mut commands, &channels, port) {
                // Fail loudly: without transport, replicon silently no-ops and
                // the user sees an apparently-running app that never replicates.
                // Emit AppExit::error so the launcher knows the run failed.
                error!("failed to start host on port {port}: {e}");
                exit.send(AppExit::error());
            }
        }
        NetMode::Client { addr } => {
            if let Err(e) = setup_client(&mut commands, &channels, addr) {
                error!("failed to connect client to {addr}: {e}");
                exit.send(AppExit::error());
            }
        }
    }
}

fn setup_host(
    commands: &mut Commands,
    channels: &RepliconChannels,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    // ConnectionConfig must declare channels matching what replicon registered
    // (one per replicated component + one per client/server event). The
    // default is empty, which silently drops all replicated state and events.
    let connection_config = ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..Default::default()
    };
    let server = RenetServer::new(connection_config);
    let public_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port);
    let socket = UdpSocket::bind(public_addr)?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let server_config = ServerConfig {
        current_time,
        max_clients: 8,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };
    let transport = NetcodeServerTransport::new(server_config, socket)?;
    commands.insert_resource(server);
    commands.insert_resource(transport);
    info!("hosting on {public_addr}");
    Ok(())
}

fn setup_client(
    commands: &mut Commands,
    channels: &RepliconChannels,
    server_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    // See `setup_host` — channels must match what replicon registered.
    let connection_config = ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..Default::default()
    };
    let client = RenetClient::new(connection_config);
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    // Client ID derives from current time: cheap unique ID for casual co-op.
    // Stored in a resource so we can identify our own Player when it arrives
    // via replication (server tags Players with NetOwner = client_id).
    let client_id = current_time.as_millis() as u64;
    debug_assert_ne!(
        client_id, HOST_NET_OWNER,
        "client_id collided with HOST_NET_OWNER sentinel"
    );
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: None,
    };
    let transport = NetcodeClientTransport::new(current_time, authentication, socket)?;
    commands.insert_resource(client);
    commands.insert_resource(transport);
    commands.insert_resource(LocalClientId(client_id));
    info!("connecting to {server_addr} as client_id={client_id}");
    Ok(())
}

/// Server-side observer: a remote client just connected (replicon spawns an
/// entity with `ConnectedClient` and `NetworkId`). Spawn that client's Player
/// with full per-player components + replication markers.
pub fn spawn_player_for_new_clients(
    trigger: Trigger<OnAdd, ConnectedClient>,
    mut commands: Commands,
    network_ids: Query<&NetworkId>,
    grid_q: Query<&crate::grid::Grid>,
) {
    let client_entity = trigger.entity();
    // The NetworkId is required by replicon_renet's backend; if missing, fall
    // back to entity index (won't match anything client-side, but avoids crash).
    let net_owner = network_ids.get(client_entity).map(|n| n.get()).unwrap_or(0);
    // Spawn the joining player at the host's spawn-pocket (same path setup_world
    // uses). Falls back to world origin if the Grid singleton isn't available —
    // shouldn't happen on host since setup_world always spawned it, but defensive.
    let spawn_world = grid_q
        .get_single()
        .map(|g| {
            let (sx, sy) = crate::terrain_gen::spawn_tile(g);
            crate::coords::tile_center_world(IVec2::new(sx, sy))
        })
        .unwrap_or(Vec2::ZERO);
    info!("spawning player for connected client `{client_entity}` (network_id {net_owner})");
    commands.spawn((
        Player,
        OwningClient(client_entity),
        NetOwner(net_owner),
        Money::default(),
        Inventory::default(),
        OwnedTools::default(),
        Transform::from_translation(spawn_world.extend(10.0)),
        Replicated,
    ));
}

/// Server-side observer: client disconnected. Despawn its Player.
pub fn despawn_player_for_disconnected_clients(
    trigger: Trigger<OnRemove, ConnectedClient>,
    mut commands: Commands,
    players: Query<(Entity, &OwningClient), With<Player>>,
) {
    let client_entity = trigger.entity();
    if let Some((player, _)) = players.iter().find(|(_, o)| o.0 == client_entity) {
        info!("despawning player {player} (client {client_entity} disconnected)");
        commands.entity(player).despawn();
    }
}

/// Client-side: when a Player arrives via replication, decide whether it's
/// "mine" by comparing `NetOwner.0` to our `LocalClientId.0`, and add the
/// per-player components replication doesn't carry (Sprite, Velocity, Facing).
/// Runs only on connected clients.
pub fn mark_local_player_on_arrival(
    mut commands: Commands,
    local_id: Option<Res<LocalClientId>>,
    arriving: Query<
        (Entity, &NetOwner, &Transform),
        (With<Player>, Without<LocalPlayer>, Without<RemotePlayer>),
    >,
) {
    let Some(local_id) = local_id else { return };
    for (entity, owner, xf) in &arriving {
        let is_local = owner.0 == local_id.0;
        let mut ec = commands.entity(entity);
        if is_local {
            ec.insert((
                LocalPlayer,
                Velocity::default(),
                Facing::default(),
                AuthoritativeTransform(xf.translation),   // seed from server spawn
                Sprite {
                    color: LOCAL_PLAYER_COLOR,
                    custom_size: Some(Vec2::splat(PLAYER_SPRITE_SIZE)),
                    ..default()
                },
            ));
        } else {
            ec.insert((
                RemotePlayer,
                Sprite {
                    color: REMOTE_PLAYER_COLOR,
                    custom_size: Some(Vec2::splat(PLAYER_SPRITE_SIZE)),
                    ..default()
                },
            ));
        }
    }
}

/// Keep sprite color in sync with Local/Remote markers. Cheap idempotent
/// resync — handles the rare case where a marker is added later than the
/// initial Sprite or a Sprite gets re-inserted.
pub fn sync_remote_player_visuals(
    mut sprites: Query<
        (&mut Sprite, Option<&LocalPlayer>, Option<&RemotePlayer>),
        With<Player>,
    >,
) {
    for (mut sprite, local, remote) in &mut sprites {
        let want = match (local.is_some(), remote.is_some()) {
            (true, _) => LOCAL_PLAYER_COLOR,
            (_, true) => REMOTE_PLAYER_COLOR,
            _ => continue,
        };
        if sprite.color != want {
            sprite.color = want;
        }
    }
}

/// Replicon doesn't ship `Sprite` over the wire. When a Shop entity arrives
/// via replication, attach the local visual. The `Without<Sprite>` filter
/// keeps this no-op on the host (setup_world spawns Shop with a Sprite
/// already attached).
pub fn add_shop_visuals_on_arrival(
    mut commands: Commands,
    new_shops: Query<Entity, (Added<crate::components::Shop>, Without<Sprite>)>,
) {
    for e in new_shops.iter() {
        commands.entity(e).insert(Sprite {
            color: Color::srgb(0.95, 0.80, 0.20),
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        });
    }
}

/// Same as `add_shop_visuals_on_arrival`, for Smelter.
pub fn add_smelter_visuals_on_arrival(
    mut commands: Commands,
    new_smelters: Query<Entity, (Added<crate::components::Smelter>, Without<Sprite>)>,
) {
    for e in new_smelters.iter() {
        commands.entity(e).insert(Sprite {
            color: Color::srgb(0.95, 0.50, 0.20),
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        });
    }
}

/// Same as `add_shop_visuals_on_arrival`, for OreDrop. Color depends on the
/// item kind, matching what `dig_input_system` / `handle_dig_requests` apply
/// at spawn time.
pub fn add_ore_drop_visuals_on_arrival(
    mut commands: Commands,
    new_drops: Query<
        (Entity, &crate::components::OreDrop),
        (Added<crate::components::OreDrop>, Without<Sprite>),
    >,
) {
    for (e, drop) in new_drops.iter() {
        commands.entity(e).insert(Sprite {
            color: crate::systems::hud::item_color(drop.item),
            custom_size: Some(Vec2::splat(6.0)),
            ..default()
        });
    }
}

/// Client-side: detect host disconnect and exit the app cleanly.
///
/// Polls the renet client connection state each frame. When it transitions
/// from connected to disconnected (host closed window, crashed, network
/// dropped), log the reason and emit `AppExit::Success`. The `Local<bool>`
/// latch prevents spamming the log + exit event during the (possibly several)
/// frames between `is_disconnected()` becoming true and the app actually
/// exiting.
///
/// No-op when `RenetClient` isn't present (single-player or host mode), so
/// this can be registered unconditionally on the MultiplayerPlugin.
pub fn exit_on_host_disconnect(
    client: Option<Res<RenetClient>>,
    mut exit: EventWriter<AppExit>,
    mut already_logged: Local<bool>,
) {
    let Some(client) = client else { return };
    if client.is_disconnected() && !*already_logged {
        match client.disconnect_reason() {
            Some(reason) => error!("disconnected from host: {:?}", reason),
            None => error!("disconnected from host (no reason given)"),
        }
        exit.send(AppExit::Success);
        *already_logged = true;
    }
}

/// Client-side: receives the one-shot `GridSnapshot` sent by the host on
/// connection. Spawns the Grid singleton entity locally and marks every
/// existing TerrainChunk dirty so `chunk_render` rebuilds meshes from the
/// newly-available grid. Replaces any prior Grid singleton defensively
/// (shouldn't happen on the first snapshot, but handles the weird case
/// where a second snapshot arrives).
pub fn apply_grid_snapshot(
    mut commands: Commands,
    mut events: EventReader<GridSnapshot>,
    existing_grid: Query<Entity, With<Grid>>,
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

/// Client-side: applies a single-tile delta from the host. Early-returns if
/// the Grid singleton doesn't exist yet (pre-snapshot race window); any lost
/// pre-snapshot events are already reflected in the snapshot that's arriving.
pub fn apply_tile_changed(
    mut commands: Commands,
    mut events: EventReader<TileChanged>,
    mut grid_q: Query<&mut Grid>,
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

/// Client-side: every `POSITION_SYNC_HZ` ticks, ship our LocalPlayer's
/// Transform + Facing to the host via `ClientPositionUpdate`. Gated on
/// `NetMode::Client` internally rather than via `.run_if(...)` so the
/// system exists in the schedule in Host mode too (where it no-ops) —
/// avoids the registration divergence between modes. Follows the same
/// pattern as `exit_on_host_disconnect`.
pub fn send_local_position_system(
    time: Res<Time>,
    mut timer: ResMut<LocalPositionSyncTimer>,
    net_mode: Res<NetMode>,
    player_q: Option<Single<(&Transform, &Facing), With<LocalPlayer>>>,
    mut writer: EventWriter<ClientPositionUpdate>,
) {
    // Tick the timer unconditionally so it stays in sync even in non-Client
    // modes. If we only ticked inside the Client gate, a reconnect flow that
    // toggles through Host → Client could fire the first event instantly
    // rather than waiting the full 100 ms.
    timer.0.tick(time.delta());
    if !matches!(*net_mode, NetMode::Client { .. }) {
        return;
    }
    if !timer.0.just_finished() {
        return;
    }
    let Some(p) = player_q else { return }; // LocalPlayer not tagged yet
    let (xf, facing) = p.into_inner();
    writer.send(ClientPositionUpdate {
        pos: xf.translation.truncate(),
        facing: facing.0,
    });
}
