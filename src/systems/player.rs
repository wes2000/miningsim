use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::components::{ChunkDirty, OreDrop, Player, TerrainChunk, Velocity};
use crate::dig::{self, DigStatus};
use crate::grid::{Grid, OreType};
use crate::systems::chunk_lifecycle::CHUNK_TILES;
use crate::systems::hud::ore_visual_color;
use crate::systems::setup::TILE_SIZE_PX;

#[derive(Resource)]
pub struct DigCooldown(pub Timer);

impl Default for DigCooldown {
    fn default() -> Self {
        Self(Timer::from_seconds(0.15, TimerMode::Once))
    }
}

pub const DIG_REACH_TILES: f32 = 2.0;

pub const PLAYER_SPEED_PX_PER_S: f32 = 120.0;
pub const PLAYER_HALF: f32 = 6.0; // 12px sprite

pub fn read_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Velocity, With<Player>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO { dir = dir.normalize(); }
    for mut v in q.iter_mut() {
        v.0 = dir * PLAYER_SPEED_PX_PER_S;
    }
}

pub fn apply_velocity_system(
    time: Res<Time>,
    mut q: Query<(&Velocity, &mut Transform), With<Player>>,
) {
    let dt = time.delta_secs();
    for (v, mut t) in q.iter_mut() {
        t.translation.x += v.0.x * dt;
        t.translation.y += v.0.y * dt;
    }
}

pub fn collide_player_with_grid_system(
    grid: Option<Res<Grid>>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let Some(grid) = grid else { return };
    let Ok(mut t) = q.get_single_mut() else { return };

    // Resolve X then Y. Player AABB is [pos.xy ± PLAYER_HALF].
    // Convert world to tile coords. World y is negative-down; tile y is positive-down.
    for axis in [0u8, 1u8] {
        let p = t.translation;
        let min = Vec2::new(p.x - PLAYER_HALF, p.y - PLAYER_HALF);
        let max = Vec2::new(p.x + PLAYER_HALF, p.y + PLAYER_HALF);

        // tile range overlapping the AABB
        let tx0 = (min.x / TILE_SIZE_PX).floor() as i32;
        let tx1 = (max.x / TILE_SIZE_PX).floor() as i32;
        let ty0 = ((-max.y) / TILE_SIZE_PX).floor() as i32;
        let ty1 = ((-min.y) / TILE_SIZE_PX).floor() as i32;

        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let Some(tile) = grid.get(tx, ty) else { continue };
                if !tile.solid { continue }
                let tw_min = Vec2::new(
                    tx as f32 * TILE_SIZE_PX,
                    -((ty + 1) as f32) * TILE_SIZE_PX,
                );
                let tw_max = Vec2::new(
                    (tx + 1) as f32 * TILE_SIZE_PX,
                    -(ty as f32) * TILE_SIZE_PX,
                );
                let overlap_x = (max.x.min(tw_max.x)) - (min.x.max(tw_min.x));
                let overlap_y = (max.y.min(tw_max.y)) - (min.y.max(tw_min.y));
                if overlap_x <= 0.0 || overlap_y <= 0.0 { continue }
                if axis == 0 {
                    // push out along X
                    if t.translation.x < (tw_min.x + tw_max.x) * 0.5 {
                        t.translation.x -= overlap_x;
                    } else {
                        t.translation.x += overlap_x;
                    }
                } else {
                    if t.translation.y < (tw_min.y + tw_max.y) * 0.5 {
                        t.translation.y -= overlap_y;
                    } else {
                        t.translation.y += overlap_y;
                    }
                }
            }
        }
    }
}

pub fn dig_input_system(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<crate::components::MainCamera>>,
    player_q: Query<&Transform, With<Player>>,
    mut grid: ResMut<Grid>,
    mut cooldown: ResMut<DigCooldown>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
    owned_tools: Res<crate::tools::OwnedTools>,
    time: Res<Time>,
) {
    cooldown.0.tick(time.delta());
    let dig_held = mouse.pressed(MouseButton::Left) || keys.pressed(KeyCode::Space);
    if !dig_held { return; }
    if !cooldown.0.finished() { return; }

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(player_xf) = player_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };

    let tx = (cursor_world.x / TILE_SIZE_PX).floor() as i32;
    let ty = ((-cursor_world.y) / TILE_SIZE_PX).floor() as i32;
    let tile_center = Vec2::new(
        tx as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0,
        -(ty as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0),
    );

    let player_tile = IVec2::new(
        (player_xf.translation.x / TILE_SIZE_PX).floor() as i32,
        ((-player_xf.translation.y) / TILE_SIZE_PX).floor() as i32,
    );
    let target_tile = IVec2::new(tx, ty);
    let reach = DIG_REACH_TILES as i32;

    // Cardinal + reach + line-of-sight gate. No cooldown reset on rejection.
    if !dig::dig_target_valid(player_tile, target_tile, reach, &grid) { return; }

    // Look up tile layer to pick the best tool.
    let Some(tile) = grid.get(tx, ty).copied() else { return; };
    let Some(tool) = crate::tools::best_applicable_tool(&owned_tools, tile.layer) else {
        // Player owns nothing that can break this layer. Clunk; no cooldown reset.
        return;
    };

    let result = dig::try_dig(&mut grid, target_tile, tool);
    match result.status {
        DigStatus::Broken | DigStatus::Damaged => {
            cooldown.0.reset();
            // Mark owning chunk dirty.
            let chunk_coord = IVec2::new(tx.div_euclid(CHUNK_TILES), ty.div_euclid(CHUNK_TILES));
            for (e, c) in chunks_q.iter() {
                if c.coord == chunk_coord {
                    commands.entity(e).insert(ChunkDirty);
                    break;
                }
            }
            // Spawn ore drop only on full break.
            if result.status == DigStatus::Broken && result.ore != OreType::None {
                commands.spawn((
                    OreDrop { ore: result.ore },
                    Sprite {
                        color: ore_visual_color(result.ore),
                        custom_size: Some(Vec2::splat(6.0)),
                        ..default()
                    },
                    Transform::from_translation(tile_center.extend(5.0)),
                ));
            }
        }
        _ => { /* OutOfBounds / AlreadyEmpty / UnderTier / Blocked — no cooldown reset */ }
    }
}
