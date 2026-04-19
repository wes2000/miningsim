use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::items::{ItemKind, OreKind};
use crate::tools::Tool;

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
#[derive(Component, Debug)]
pub struct OwningClient(pub Entity);

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
