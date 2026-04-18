use bevy::prelude::*;
use crate::components::{ChunkDirty, TerrainChunk};
use crate::grid::{Grid, OreType};
use crate::marching_squares::build_chunk_mesh;
use crate::systems::setup::TILE_SIZE_PX;
use crate::systems::chunk_lifecycle::CHUNK_TILES;

/// Shared `ColorMaterial` handle for all terrain chunk meshes. Per-vertex
/// colors carry the layer tint; the material itself is plain white so the
/// vertex colors pass through unmodified.
#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<ColorMaterial>);

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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    chunk_mat: Option<Res<ChunkMaterial>>,
    dirty_q: Query<(Entity, &TerrainChunk), With<ChunkDirty>>,
    children_q: Query<&Children>,
) {
    let Some(grid) = grid else { return };

    // Lazily create the shared chunk material if setup hasn't yet.
    let mat_handle: Handle<ColorMaterial> = match chunk_mat {
        Some(r) => r.0.clone(),
        None => {
            let h = materials.add(ColorMaterial::default());
            commands.insert_resource(ChunkMaterial(h.clone()));
            h
        }
    };

    for (entity, chunk) in dirty_q.iter() {
        // Despawn previous children (old contour mesh + ore sprites).
        if let Ok(children) = children_q.get(entity) {
            for c in children.iter() {
                commands.entity(*c).despawn_recursive();
            }
        }

        let mesh_opt = build_chunk_mesh(&grid, chunk.coord);

        commands.entity(entity).with_children(|parent| {
            if let Some(mesh) = mesh_opt {
                let mesh_handle = meshes.add(mesh);
                parent.spawn((
                    Mesh2d(mesh_handle),
                    MeshMaterial2d(mat_handle.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ));
            }

            // Ore dots stay as small per-tile sprites on top of the contour.
            for ly in 0..CHUNK_TILES {
                for lx in 0..CHUNK_TILES {
                    let gx = chunk.coord.x * CHUNK_TILES + lx;
                    let gy = chunk.coord.y * CHUNK_TILES + ly;
                    let Some(t) = grid.get(gx, gy) else { continue };
                    if !t.solid { continue }
                    let Some(c) = ore_color(t.ore) else { continue };

                    let world_x = gx as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0;
                    let world_y = -(gy as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0);
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
        });

        commands.entity(entity).remove::<ChunkDirty>();
    }
}
