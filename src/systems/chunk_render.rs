use bevy::prelude::*;
use crate::components::{ChunkDirty, TerrainChunk};
use crate::coords::{self, TILE_SIZE_PX};
use crate::grid::{Grid, Layer};
use crate::systems::chunk_lifecycle::CHUNK_TILES;
use crate::systems::hud::ore_visual_color;

fn layer_color(l: Layer) -> Color {
    match l {
        Layer::Dirt    => Color::srgb(0.55, 0.42, 0.27),
        Layer::Stone   => Color::srgb(0.42, 0.33, 0.22),
        Layer::Deep    => Color::srgb(0.29, 0.23, 0.15),
        Layer::Core    => Color::srgb(0.22, 0.18, 0.12),
        Layer::Bedrock => Color::srgb(0.16, 0.13, 0.10),
    }
}

pub fn chunk_remesh_system(
    mut commands: Commands,
    grid: Option<Single<&Grid>>,
    dirty_q: Query<(Entity, &TerrainChunk), With<ChunkDirty>>,
    children_q: Query<&Children>,
) {
    let Some(grid) = grid else { return };
    let grid = grid.into_inner();
    for (entity, chunk) in dirty_q.iter() {
        // despawn previous children (tile sprites + ore sprites)
        if let Ok(children) = children_q.get(entity) {
            for c in children.iter() {
                commands.entity(*c).despawn_recursive();
            }
        }

        commands.entity(entity).with_children(|parent| {
            for ly in 0..CHUNK_TILES {
                for lx in 0..CHUNK_TILES {
                    let gx = chunk.coord.x * CHUNK_TILES + lx;
                    let gy = chunk.coord.y * CHUNK_TILES + ly;
                    let Some(t) = grid.get(gx, gy) else { continue };
                    if !t.solid { continue }

                    let center = coords::tile_center_world(IVec2::new(gx, gy));
                    let world_x = center.x;
                    let world_y = center.y;

                    parent.spawn((
                        Sprite {
                            color: layer_color(t.layer),
                            custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(world_x, world_y, 0.0)),
                    ));

                    if let Some(ore) = t.ore {
                        parent.spawn((
                            Sprite {
                                color: ore_visual_color(ore),
                                custom_size: Some(Vec2::splat(TILE_SIZE_PX * 0.5)),
                                ..default()
                            },
                            Transform::from_translation(Vec3::new(world_x, world_y, 0.5)),
                        ));
                    }

                    if t.damage > 0 {
                        parent.spawn((
                            Sprite {
                                color: Color::srgba(0.0, 0.0, 0.0, t.damage as f32 * 0.2),
                                custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                                ..default()
                            },
                            Transform::from_translation(Vec3::new(world_x, world_y, 0.25)),
                        ));
                    }
                }
            }
        });

        commands.entity(entity).remove::<ChunkDirty>();
    }
}
