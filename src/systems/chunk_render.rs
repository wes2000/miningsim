use bevy::prelude::*;
use crate::components::{ChunkDirty, TerrainChunk};
use crate::grid::{Grid, Layer, OreType};
use crate::systems::setup::TILE_SIZE_PX;
use crate::systems::chunk_lifecycle::CHUNK_TILES;

fn layer_color(l: Layer) -> Color {
    match l {
        Layer::Dirt    => Color::srgb(0.55, 0.42, 0.27),
        Layer::Stone   => Color::srgb(0.42, 0.33, 0.22),
        Layer::Deep    => Color::srgb(0.29, 0.23, 0.15),
        Layer::Bedrock => Color::srgb(0.16, 0.13, 0.10),
    }
}

fn ore_color(o: OreType) -> Option<Color> {
    match o {
        OreType::None   => None,
        OreType::Copper => Some(Color::srgb(0.85, 0.45, 0.20)),
        OreType::Silver => Some(Color::srgb(0.85, 0.85, 0.92)),
        OreType::Gold   => Some(Color::srgb(0.95, 0.78, 0.25)),
    }
}

pub fn chunk_remesh_system(
    mut commands: Commands,
    grid: Option<Res<Grid>>,
    dirty_q: Query<(Entity, &TerrainChunk), With<ChunkDirty>>,
    children_q: Query<&Children>,
) {
    let Some(grid) = grid else { return };
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

                    let world_x = gx as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0;
                    let world_y = -(gy as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0);

                    parent.spawn((
                        Sprite {
                            color: layer_color(t.layer),
                            custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(world_x, world_y, 0.0)),
                    ));

                    if let Some(c) = ore_color(t.ore) {
                        parent.spawn((
                            Sprite {
                                color: c,
                                custom_size: Some(Vec2::splat(TILE_SIZE_PX * 0.5)),
                                ..default()
                            },
                            Transform::from_translation(Vec3::new(world_x, world_y, 0.5)),
                        ));
                    }
                }
            }
        });

        commands.entity(entity).remove::<ChunkDirty>();
    }
}
