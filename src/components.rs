use bevy::prelude::*;
use crate::grid::OreType;
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
    pub ore: OreType,
}

#[derive(Component)]
pub struct OreDrop {
    pub ore: OreType,
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

#[derive(Component)]
pub struct CurrentToolText;

#[derive(bevy::prelude::Resource, Default)]
pub struct ShopUiOpen(pub bool);
