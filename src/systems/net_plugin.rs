use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

use crate::app::InputSet;

use crate::components::{ChunkDirty, Facing, NetOwner, OreDrop, OwningClient, Player, Shop, Smelter, TerrainChunk};
use crate::coords::{tile_center_world, world_to_tile};
use crate::dig::{self, DigStatus};
use crate::economy::{self, Money};
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::items::ItemKind;
use crate::processing::{self, SmelterState};
use crate::systems::chunk_lifecycle::CHUNK_TILES;
use crate::systems::hud::item_color;
use crate::systems::net_events::{
    BuyToolRequest, ClientPositionUpdate, CollectAllRequest, DigRequest, GridSnapshot,
    SellAllRequest, SmeltAllRequest, TileChanged,
};
use crate::systems::net_player;
use crate::systems::player::DIG_REACH_TILES;
use crate::tools::{self, OwnedTools};

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RepliconPlugins);
        app.add_plugins(RepliconRenetPlugins);

        // Replicated components — host writes, all clients read.
        // Grid is NOT replicated here; it syncs via M5b GridSnapshot (full
        // on connect) and TileChanged (per-dig delta) server events instead.
        app.replicate::<Player>()
            .replicate::<NetOwner>()
            .replicate::<Shop>()
            .replicate::<Smelter>()
            .replicate::<SmelterState>()
            .replicate::<OreDrop>()
            .replicate::<Money>()
            .replicate::<Inventory>()
            .replicate::<OwnedTools>()
            .replicate::<Transform>();

        // Client-fired events (client → server). 0.32 uses `Channel`, not `ChannelKind`.
        app.add_client_event::<DigRequest>(Channel::Ordered);
        app.add_client_event::<BuyToolRequest>(Channel::Ordered);
        app.add_client_event::<SmeltAllRequest>(Channel::Ordered);
        app.add_client_event::<CollectAllRequest>(Channel::Ordered);
        app.add_client_event::<SellAllRequest>(Channel::Ordered);
        app.add_client_event::<ClientPositionUpdate>(Channel::Unreliable);

        // M5b server events — host-authoritative Grid sync.
        app.add_server_event::<GridSnapshot>(Channel::Ordered);
        app.add_server_event::<TileChanged>(Channel::Ordered);

        // Server-side request handlers. Run only when this app is acting as
        // the server (replicon's `server_running` condition). The host's own
        // local Player is NOT mutated here — the host UI/input keeps mutating
        // its own components directly via the single-player code path; only
        // events from REMOTE clients reach these handlers (their Player
        // entities carry an `OwningClient` component, inserted by Task 12).
        app.add_systems(
            Update,
            (
                handle_dig_requests,
                handle_buy_tool_requests,
                handle_smelt_all_requests,
                handle_collect_all_requests,
                handle_sell_all_requests,
                handle_client_position_updates,
            )
                .run_if(server_running),
        );

        // Transport setup (host: bind UDP; client: connect to addr).
        app.add_systems(Startup, net_player::start_net_mode_system);

        // Player lifecycle (server-only; observers fire only when ConnectedClient
        // entities are spawned/despawned, which only happens on the server side).
        app.add_observer(net_player::spawn_player_for_new_clients);
        app.add_observer(net_player::despawn_player_for_disconnected_clients);
        app.add_observer(send_initial_grid_snapshot);

        // Client-side: tag arriving Players LocalPlayer/RemotePlayer + add Sprite.
        app.add_systems(
            Update,
            net_player::mark_local_player_on_arrival.run_if(client_connected),
        );

        // Visual sync (idempotent; cheap to always run).
        app.add_systems(Update, net_player::sync_remote_player_visuals);

        // Client-side: Sprite isn't replicated by replicon, so when Shop /
        // Smelter / OreDrop entities arrive on the client they need their
        // visuals attached locally. The `Without<Sprite>` filter makes these
        // safe to run unconditionally on the host (where setup_world /
        // handle_dig_requests already attached Sprite at spawn time).
        app.add_systems(
            Update,
            (
                net_player::add_shop_visuals_on_arrival,
                net_player::add_smelter_visuals_on_arrival,
                net_player::add_ore_drop_visuals_on_arrival,
            ),
        );

        // M5b: client-side grid sync from server events.
        app.add_systems(
            Update,
            (
                net_player::apply_grid_snapshot,
                net_player::apply_tile_changed,
            )
                .chain()
                .run_if(client_connected),
        );

        // Client-side: clean exit when the host drops. No-op (early-returns)
        // when `RenetClient` isn't present, so it's safe in host/single-player.
        // NOT gated on `client_connected` — the whole point is to fire when
        // the connection is lost.
        app.add_systems(Update, net_player::exit_on_host_disconnect);

        // M5b: client-authoritative position sync.
        app.insert_resource(net_player::LocalPositionSyncTimer::default());
        app.add_systems(Update, net_player::send_local_position_system);

        // M5b Task 8: restore LocalPlayer Transform from AuthoritativeTransform
        // after replicon applies inbound server-origin updates in PreUpdate.
        // Runs at the top of Update (InputSet::ReadInput) so gameplay systems
        // see the correct client-authoritative position.
        app.add_systems(
            Update,
            net_player::restore_local_transform_from_authoritative
                .in_set(InputSet::ReadInput),
        );
    }
}

/// Find the Player entity owned by `client_entity` (the replicon
/// connected-client entity). Returns `None` for events whose sender has no
/// matching Player yet (race during connect / before Task 12 spawns one) or
/// for the host's own local Player (which has no `OwningClient` component).
// TODO: switch to a HashMap<Entity /*client*/, Entity /*player*/> resource updated by spawn/despawn observers if max_clients ever exceeds ~16.
fn player_entity_for_client(
    client_entity: Entity,
    q: &Query<(Entity, &OwningClient), With<Player>>,
) -> Option<Entity> {
    q.iter()
        .find_map(|(e, owning)| (owning.0 == client_entity).then_some(e))
}

pub fn handle_dig_requests(
    mut events: EventReader<FromClient<DigRequest>>,
    grid: Single<&mut Grid>,
    mut commands: Commands,
    player_q: Query<(Entity, &OwningClient, &Transform, &OwnedTools), With<Player>>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
    mut tile_writer: EventWriter<ToClients<TileChanged>>,   // NEW
) {
    let mut grid = grid.into_inner();
    for FromClient { client_entity, event } in events.read() {
        let Some((_, _, player_xf, owned)) = player_q
            .iter()
            .find(|(_, owning, _, _)| owning.0 == *client_entity)
        else {
            continue;
        };

        let player_tile = world_to_tile(player_xf.translation.truncate());
        if !dig::dig_target_valid(player_tile, event.target, DIG_REACH_TILES as i32, &grid) {
            continue;
        }
        let Some(tile) = grid.get(event.target.x, event.target.y).copied() else { continue };
        let Some(tool) = tools::best_applicable_tool(owned, tile.layer) else { continue };

        let result = dig::try_dig(&mut grid, event.target, tool);
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
        if result.status == DigStatus::Broken {
            if let Some(ore) = result.ore {
                let item = ItemKind::Ore(ore);
                let world_pos = tile_center_world(event.target);
                commands.spawn((
                    OreDrop { item },
                    Sprite {
                        color: item_color(item),
                        custom_size: Some(Vec2::splat(6.0)),
                        ..default()
                    },
                    Transform::from_translation(world_pos.extend(4.0)),
                    Replicated,
                ));
            }
        }
    }
}

pub fn handle_buy_tool_requests(
    mut events: EventReader<FromClient<BuyToolRequest>>,
    player_q: Query<(Entity, &OwningClient), With<Player>>,
    mut money_q: Query<(&mut Money, &mut OwnedTools), With<Player>>,
) {
    for FromClient { client_entity, event } in events.read() {
        let Some(e) = player_entity_for_client(*client_entity, &player_q) else { continue };
        let Ok((mut money, mut owned)) = money_q.get_mut(e) else { continue };
        let _ = economy::try_buy(event.tool, &mut money, &mut owned);
    }
}

pub fn handle_smelt_all_requests(
    mut events: EventReader<FromClient<SmeltAllRequest>>,
    player_q: Query<(Entity, &OwningClient), With<Player>>,
    mut inv_q: Query<&mut Inventory, With<Player>>,
    mut smelter_q: Query<&mut SmelterState>,
) {
    for FromClient { client_entity, event } in events.read() {
        let Some(e) = player_entity_for_client(*client_entity, &player_q) else { continue };
        let Ok(mut inv) = inv_q.get_mut(e) else { continue };
        let Ok(mut state) = smelter_q.get_single_mut() else { continue };
        let count = inv.get(ItemKind::Ore(event.ore));
        if count == 0 || processing::is_busy(&state) { continue; }
        inv.remove(ItemKind::Ore(event.ore), count);
        processing::start_smelting(&mut state, event.ore, count);
    }
}

pub fn handle_collect_all_requests(
    mut events: EventReader<FromClient<CollectAllRequest>>,
    player_q: Query<(Entity, &OwningClient), With<Player>>,
    mut inv_q: Query<&mut Inventory, With<Player>>,
    mut smelter_q: Query<&mut SmelterState>,
) {
    for FromClient { client_entity, .. } in events.read() {
        let Some(e) = player_entity_for_client(*client_entity, &player_q) else { continue };
        let Ok(mut inv) = inv_q.get_mut(e) else { continue };
        let Ok(mut state) = smelter_q.get_single_mut() else { continue };
        let drained = processing::collect_output(&mut state);
        for (ore, n) in drained {
            inv.add(ItemKind::Bar(ore), n);
        }
    }
}

pub fn handle_sell_all_requests(
    mut events: EventReader<FromClient<SellAllRequest>>,
    player_q: Query<(Entity, &OwningClient), With<Player>>,
    mut inv_money_q: Query<(&mut Inventory, &mut Money), With<Player>>,
) {
    for FromClient { client_entity, .. } in events.read() {
        let Some(e) = player_entity_for_client(*client_entity, &player_q) else { continue };
        let Ok((mut inv, mut money)) = inv_money_q.get_mut(e) else { continue };
        economy::sell_all(&mut inv, &mut money);
    }
}

/// Server-side: write the client's reported position onto its server-side
/// Player Transform. Trust-based — we don't validate or speed-cap. Mutation
/// triggers replicon's change detection, which broadcasts Transform updates
/// to all OTHER clients via the existing `.replicate::<Transform>()`.
pub fn handle_client_position_updates(
    mut events: EventReader<FromClient<ClientPositionUpdate>>,
    player_q: Query<(Entity, &OwningClient), With<Player>>,
    mut xf_q: Query<(&mut Transform, &mut Facing), With<Player>>,
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
