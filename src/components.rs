use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::items::{ItemKind, OreKind};
use crate::tools::Tool;

/// Marker on the player entity. Serde derives are required by replicon's replicate::<Player>(); the 0-byte payload signals "this entity is a player" to clients.
#[derive(Component, Serialize, Deserialize)]
pub struct Player;

/// The player entity controlled by this client. Exactly one in any session.
#[derive(Component)]
pub struct LocalPlayer;

/// A player entity replicated from another peer. Renders with a different sprite color.
#[derive(Component)]
pub struct RemotePlayer;

/// Server-side bookkeeping: links a Player entity to the replicon connection
/// (an `Entity` representing the connected client; see
/// `bevy_replicon::shared::backend::connected_client::ConnectedClient`) that
/// owns it. Inserted at player-spawn time on the host (Task 12). Absent on the
/// host's own local Player — the host mutates its own components directly via
/// the existing single-player code path. The server-side request handlers in
/// `MultiplayerPlugin` use this to route remote-client events to the correct
/// per-client Player entity.
///
/// Intentionally NOT replicated: the contained `Entity` is a server-side ID
/// that has no meaning on the client. Clients use [`NetOwner`] (carries the
/// renet client_id u64) to identify which Player is theirs.
#[derive(Component, Debug)]
pub struct OwningClient(pub Entity);

/// Replicated marker carrying the renet `client_id` (u64) of the player's
/// owning client. Server inserts it on every Player spawn (host's own local
/// Player gets `NetOwner(0)` so remote clients can render it as a remote peer).
/// Clients compare it against [`LocalClientId`] to decide LocalPlayer vs.
/// RemotePlayer when arriving Players are tagged.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetOwner(pub u64);

/// Resource on the client carrying the renet `client_id` we used at connect
/// time, so [`NetOwner`] arriving over replication can be matched against it.
/// Inserted by `start_net_mode_system` on the client. Absent on host/single-player.
#[derive(bevy::prelude::Resource, Debug, Clone, Copy)]
pub struct LocalClientId(pub u64);

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

/// Cardinal facing direction in TILE-space (positive y = deeper underground,
/// matching the grid convention, NOT Bevy's world Y).
/// One of (1,0), (-1,0), (0,1), (0,-1). Used by spacebar-dig to pick a
/// target tile relative to the player.
#[derive(Component)]
pub struct Facing(pub IVec2);

impl Default for Facing {
    fn default() -> Self { Self(IVec2::new(0, 1)) }   // down / deeper
}

#[derive(Component)]
pub struct TerrainChunk {
    pub coord: IVec2,
}

#[derive(Component)]
pub struct ChunkDirty;

#[derive(Component)]
pub struct OreSprite {
    pub ore: OreKind,
}

#[derive(Component)]
pub struct OreDrop {
    pub item: ItemKind,
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct Shop;

#[derive(Component)]
pub struct ShopUiRoot;

#[derive(Component)]
pub enum ShopButtonKind {
    SellAll,
    Buy(Tool),
}

#[derive(Component)]
pub struct MoneyText;

/// Marker on a tool-row label inside the inventory popup. Carries which Tool
/// the row represents so the refresh system can look up owned/locked state
/// per row without re-walking the children.
#[derive(Component)]
pub struct ToolRowText(pub Tool);

#[derive(Component)]
pub struct InventoryPopupRoot;

#[derive(bevy::prelude::Resource, Default)]
pub struct InventoryPopupOpen(pub bool);

#[derive(bevy::prelude::Resource, Default)]
pub struct ShopUiOpen(pub bool);

#[derive(Component)]
pub struct Smelter;

#[derive(Component)]
pub struct SmelterUiRoot;

#[derive(Component)]
pub enum SmelterButtonKind {
    SmeltAll(OreKind),
    CollectAll,
}

#[derive(Component)]
pub struct SmelterStatusText;

#[derive(bevy::prelude::Resource, Default)]
pub struct SmelterUiOpen(pub bool);
