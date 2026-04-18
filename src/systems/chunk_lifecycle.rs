use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::components::{ChunkDirty, MainCamera, TerrainChunk};
use crate::grid::Grid;
use crate::systems::setup::TILE_SIZE_PX;

pub const CHUNK_TILES: i32 = 16;
pub const CHUNK_MARGIN: i32 = 1;

pub fn chunk_lifecycle_system(
    mut commands: Commands,
    grid: Option<Res<Grid>>,
    cam_q: Query<&Transform, With<MainCamera>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
) {
    let Some(grid) = grid else { return };
    let Ok(cam) = cam_q.get_single() else { return };
    let Ok(win) = win_q.get_single() else { return };

    let half = Vec2::new(win.width(), win.height()) * 0.5;
    let cam_pos = cam.translation.truncate();
    let world_min = cam_pos - half;
    let world_max = cam_pos + half;

    // Y inverts between world (up-positive) and grid (down-positive), so
    // `world_min` / `world_max` map to chunk-space corners that are NOT
    // componentwise min/max. Normalize after mapping.
    let c_a = world_to_chunk(world_min);
    let c_b = world_to_chunk(world_max);
    let chunk_min = c_a.min(c_b) - IVec2::splat(CHUNK_MARGIN);
    let chunk_max = c_a.max(c_b) + IVec2::splat(CHUNK_MARGIN);

    let mut want = std::collections::HashSet::new();
    for cy in chunk_min.y..=chunk_max.y {
        for cx in chunk_min.x..=chunk_max.x {
            // skip chunks fully outside the grid
            if cx * CHUNK_TILES >= grid.width() as i32 { continue; }
            if cy * CHUNK_TILES >= grid.height() as i32 { continue; }
            if (cx + 1) * CHUNK_TILES <= 0 { continue; }
            if (cy + 1) * CHUNK_TILES <= 0 { continue; }
            want.insert(IVec2::new(cx, cy));
        }
    }

    let existing: std::collections::HashMap<IVec2, Entity> = chunks_q
        .iter()
        .map(|(e, c)| (c.coord, e))
        .collect();

    for coord in &want {
        if !existing.contains_key(coord) {
            commands.spawn((
                TerrainChunk { coord: *coord },
                ChunkDirty,
                Transform::from_xyz(0.0, 0.0, 0.0),
                Visibility::default(),
            ));
        }
    }
    for (coord, entity) in &existing {
        if !want.contains(coord) {
            commands.entity(*entity).despawn_recursive();
        }
    }
}

fn world_to_chunk(world: Vec2) -> IVec2 {
    let tx = (world.x / TILE_SIZE_PX).floor() as i32;
    // game Y inverts; underground tiles have larger y, in world they have negative y
    let ty = (-world.y / TILE_SIZE_PX).floor() as i32;
    IVec2::new(tx.div_euclid(CHUNK_TILES), ty.div_euclid(CHUNK_TILES))
}
