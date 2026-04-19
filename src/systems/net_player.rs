//! Multiplayer player lifecycle, transport setup, and Grid-replication
//! re-mesh wiring (Task 12 + Task 10 follow-up).
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
//!   * `mark_chunks_dirty_on_grid_change` — when Grid mutates (notably from a remote
//!                                       host's dig replicating to a client), flag every
//!                                       TerrainChunk so chunk_render rebuilds it.
//!
//! Approach for "which Player is mine?": `NetOwner(u64)` carries the renet
//! `client_id`. The server sets it from `NetworkId` on the just-connected client
//! entity. The client stores its own client_id in `LocalClientId` resource at
//! connection time; arriving Players match against that.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use bevy_replicon_renet::netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
    ServerConfig,
};
use bevy_replicon_renet::renet::{ConnectionConfig, RenetClient, RenetServer};

use crate::components::{
    ChunkDirty, Facing, LocalClientId, LocalPlayer, NetOwner, OwningClient, Player, RemotePlayer,
    TerrainChunk, Velocity,
};
use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::net::NetMode;
use crate::tools::OwnedTools;

/// Arbitrary game identifier — both ends MUST agree. ASCII "MINING_1".
pub const PROTOCOL_ID: u64 = 0x4D494E494E475F31;

const LOCAL_PLAYER_COLOR: Color = Color::srgb(0.30, 0.60, 0.90);
const REMOTE_PLAYER_COLOR: Color = Color::srgb(0.95, 0.55, 0.20);
const PLAYER_SPRITE_SIZE: f32 = 12.0;

/// Reserved "client_id" value used for the host's own local Player. Real renet
/// client IDs are `as_millis() as u64` and won't collide with this in practice.
pub const HOST_NET_OWNER: u64 = 0;

/// Transport setup. Runs once at Startup. NetMode::SinglePlayer is a no-op
/// (the MultiplayerPlugin isn't even loaded in that case, but be defensive).
pub fn start_net_mode_system(mut commands: Commands, net_mode: Res<NetMode>) {
    match net_mode.clone() {
        NetMode::SinglePlayer => {}
        NetMode::Host { port } => {
            if let Err(e) = setup_host(&mut commands, port) {
                error!("failed to start host on port {port}: {e}");
            }
        }
        NetMode::Client { addr } => {
            if let Err(e) = setup_client(&mut commands, addr) {
                error!("failed to connect client to {addr}: {e}");
            }
        }
    }
}

fn setup_host(commands: &mut Commands, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let server = RenetServer::new(ConnectionConfig::default());
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

fn setup_client(commands: &mut Commands, server_addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let client = RenetClient::new(ConnectionConfig::default());
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    // Client ID derives from current time: cheap unique ID for casual co-op.
    // Stored in a resource so we can identify our own Player when it arrives
    // via replication (server tags Players with NetOwner = client_id).
    let client_id = current_time.as_millis() as u64;
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
) {
    let client_entity = trigger.entity();
    // The NetworkId is required by replicon_renet's backend; if missing, fall
    // back to entity index (won't match anything client-side, but avoids crash).
    let net_owner = network_ids.get(client_entity).map(|n| n.get()).unwrap_or(0);
    info!("spawning player for connected client `{client_entity}` (network_id {net_owner})");
    commands.spawn((
        Player,
        OwningClient(client_entity),
        NetOwner(net_owner),
        Money::default(),
        Inventory::default(),
        OwnedTools::default(),
        // Spawn at world origin — joining-player position will improve when we
        // have a real spawn-finder; for now this is fine for smoke tests.
        Transform::from_xyz(0.0, 0.0, 10.0),
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
        (Entity, &NetOwner),
        (With<Player>, Without<LocalPlayer>, Without<RemotePlayer>),
    >,
) {
    let Some(local_id) = local_id else { return };
    for (entity, owner) in &arriving {
        let is_local = owner.0 == local_id.0;
        let mut ec = commands.entity(entity);
        if is_local {
            ec.insert((
                LocalPlayer,
                Velocity::default(),
                Facing::default(),
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

/// When the singleton Grid changes (typically because the host's dig
/// replicated to us as a client), flag every TerrainChunk dirty so
/// chunk_render rebuilds them. On the host this also fires after local digs,
/// but those tiles are already covered by the per-chunk dirtying inside
/// `handle_dig_requests`/local dig — re-dirtying them is harmless (the chunk
/// renderer is idempotent).
pub fn mark_chunks_dirty_on_grid_change(
    grid_q: Query<Ref<Grid>>,
    chunks: Query<Entity, With<TerrainChunk>>,
    mut commands: Commands,
) {
    let Ok(grid) = grid_q.get_single() else { return };
    if !grid.is_changed() {
        return;
    }
    for chunk in &chunks {
        commands.entity(chunk).insert(ChunkDirty);
    }
}
