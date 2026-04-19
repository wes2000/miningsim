use bevy::prelude::*;
use crate::items::{ItemKind, OreKind};
use crate::tools::Tool;

#[derive(Component)]
pub struct Player;

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
