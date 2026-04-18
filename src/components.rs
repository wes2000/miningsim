use bevy::prelude::*;
use crate::grid::OreType;

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
