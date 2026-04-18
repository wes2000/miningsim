use bevy::prelude::*;
use crate::grid::OreType;
use crate::tools::Tool;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

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
