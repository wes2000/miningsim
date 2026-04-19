use bevy::math::{IVec2, Vec2};

pub const TILE_SIZE_PX: f32 = 16.0;

pub fn world_to_tile(world: Vec2) -> IVec2 {
    IVec2::new(
        (world.x / TILE_SIZE_PX).floor() as i32,
        ((-world.y) / TILE_SIZE_PX).floor() as i32,
    )
}

pub fn tile_min_world(tile: IVec2) -> Vec2 {
    Vec2::new(
        tile.x as f32 * TILE_SIZE_PX,
        -((tile.y + 1) as f32) * TILE_SIZE_PX,
    )
}

pub fn tile_center_world(tile: IVec2) -> Vec2 {
    Vec2::new(
        tile.x as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0,
        -(tile.y as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0),
    )
}
