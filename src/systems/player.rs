use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::components::{AuthoritativeTransform, ChunkDirty, Facing, LocalPlayer, OreDrop, Player, TerrainChunk, Velocity};
use crate::coords::{self, TILE_SIZE_PX};
use crate::dig::{self, DigStatus};
use crate::grid::Grid;
use crate::items::ItemKind;
use crate::systems::chunk_lifecycle::CHUNK_TILES;
use crate::systems::hud::item_color;
use crate::systems::net_events::{DigRequest, TileChanged};
use bevy_replicon::prelude::{SendMode, ToClients};

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
    mut q: Query<(&mut Velocity, &mut Facing), With<LocalPlayer>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    // Snap facing to dominant input axis (tile space: +y = deeper).
    // World y is up-positive, tile y is down-positive — invert when mapping.
    let new_facing: Option<IVec2> = if dir == Vec2::ZERO {
        None
    } else if dir.y.abs() >= dir.x.abs() {
        Some(IVec2::new(0, if dir.y > 0.0 { -1 } else { 1 }))
    } else {
        Some(IVec2::new(if dir.x > 0.0 { 1 } else { -1 }, 0))
    };

    let velocity = if dir != Vec2::ZERO {
        dir.normalize() * PLAYER_SPEED_PX_PER_S
    } else {
        Vec2::ZERO
    };

    for (mut v, mut f) in q.iter_mut() {
        v.0 = velocity;
        if let Some(nf) = new_facing { f.0 = nf; }
    }
}

pub fn apply_velocity_system(
    time: Res<Time>,
    // `With<Player>` matches every Player entity — host's own LocalPlayer, the
    // host-side mirror of each remote client's Player, and client-side
    // LocalPlayer/RemotePlayer. Only those with `Velocity` are iterated. The
    // host-side mirrors of remote players are spawned without `Velocity` (see
    // `spawn_player_for_new_clients` in net_player.rs), so they're excluded —
    // their Transform is driven by the `handle_client_position_updates`
    // handler, not local integration. The `Option<&mut AuthoritativeTransform>`
    // tracks client-LocalPlayer only; host/SP players lack the component and
    // silently no-op.
    mut q: Query<(&Velocity, &mut Transform, Option<&mut AuthoritativeTransform>), With<Player>>,
) {
    let dt = time.delta_secs();
    for (v, mut t, auth) in q.iter_mut() {
        t.translation.x += v.0.x * dt;
        t.translation.y += v.0.y * dt;
        if let Some(mut auth) = auth {
            auth.0 = t.translation;
        }
    }
}

pub fn collide_player_with_grid_system(
    grid: Option<Single<&Grid>>,
    mut q: Query<(&mut Transform, Option<&mut AuthoritativeTransform>), With<LocalPlayer>>,
) {
    let Some(grid) = grid else { return };
    let grid = grid.into_inner();
    let Ok((mut t, auth)) = q.get_single_mut() else { return };

    // Resolve X then Y. Player AABB is [pos.xy ± PLAYER_HALF].
    // Convert world to tile coords. World y is negative-down; tile y is positive-down.
    for axis in [0u8, 1u8] {
        let p = t.translation;
        let min = Vec2::new(p.x - PLAYER_HALF, p.y - PLAYER_HALF);
        let max = Vec2::new(p.x + PLAYER_HALF, p.y + PLAYER_HALF);

        // tile range overlapping the AABB
        let t_min = coords::world_to_tile(Vec2::new(min.x, max.y));
        let t_max = coords::world_to_tile(Vec2::new(max.x, min.y));
        let tx0 = t_min.x;
        let tx1 = t_max.x;
        let ty0 = t_min.y;
        let ty1 = t_max.y;

        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let Some(tile) = grid.get(tx, ty) else { continue };
                if !tile.solid { continue }
                let tw_min = coords::tile_min_world(IVec2::new(tx, ty));
                let tw_max = tw_min + Vec2::splat(TILE_SIZE_PX);
                let overlap_x = (max.x.min(tw_max.x)) - (min.x.max(tw_min.x));
                let overlap_y = (max.y.min(tw_max.y)) - (min.y.max(tw_min.y));
                if overlap_x <= 0.0 || overlap_y <= 0.0 { continue }
                // Resolve along the AXIS WITH THE SMALLER OVERLAP (minimum
                // translation vector). Without this guard, walking vertically
                // into a wall would also push horizontally by the player's
                // full width, which felt like sideways bounce-back.
                if axis == 0 {
                    if overlap_x > overlap_y { continue; }
                    // push out along X
                    if t.translation.x < (tw_min.x + tw_max.x) * 0.5 {
                        t.translation.x -= overlap_x;
                    } else {
                        t.translation.x += overlap_x;
                    }
                } else {
                    if overlap_y > overlap_x { continue; }
                    if t.translation.y < (tw_min.y + tw_max.y) * 0.5 {
                        t.translation.y -= overlap_y;
                    } else {
                        t.translation.y += overlap_y;
                    }
                }
            }
        }
    }
    if let Some(mut auth) = auth {
        auth.0 = t.translation;
    }
}

pub fn dig_input_system(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<crate::components::MainCamera>>,
    player_q: Query<(&Transform, &Facing), With<LocalPlayer>>,
    grid: Option<Single<&mut Grid>>,
    mut cooldown: ResMut<DigCooldown>,
    chunks_q: Query<(Entity, &TerrainChunk)>,
    owned_tools: Option<Single<&crate::tools::OwnedTools, With<crate::components::LocalPlayer>>>,
    time: Res<Time>,
    net_mode: Res<crate::net::NetMode>,
    mut dig_writer: EventWriter<DigRequest>,
    mut tile_writer: EventWriter<ToClients<TileChanged>>,
) {
    let (Some(grid), Some(owned_tools)) = (grid, owned_tools) else { return };
    cooldown.0.tick(time.delta());
    let mut grid = grid.into_inner();

    // Two trigger paths. Mouse wins if both are held (more specific aim).
    let mouse_held = mouse.pressed(MouseButton::Left);
    let space_held = keys.pressed(KeyCode::Space);
    if !mouse_held && !space_held { return; }
    if !cooldown.0.finished() { return; }

    let Ok((player_xf, facing)) = player_q.get_single() else { return };
    let player_tile = coords::world_to_tile(player_xf.translation.truncate());

    // Target tile depends on which trigger fired. Mouse takes precedence.
    let target_tile = if mouse_held {
        let Ok(win) = win_q.get_single() else { return };
        let Some(cursor_screen) = win.cursor_position() else { return };
        let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
        let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
        coords::world_to_tile(cursor_world)
    } else {
        // Spacebar: dig the tile immediately in front of the player, in the
        // current facing direction (set by the last WASD press).
        player_tile + facing.0
    };

    let tile_center = coords::tile_center_world(target_tile);

    let reach = DIG_REACH_TILES as i32;

    // Cardinal + reach + line-of-sight gate. No cooldown reset on rejection.
    if !dig::dig_target_valid(player_tile, target_tile, reach, &grid) { return; }

    // Look up tile layer to pick the best tool.
    let Some(tile) = grid.get(target_tile.x, target_tile.y).copied() else { return; };
    let Some(tool) = crate::tools::best_applicable_tool(*owned_tools, tile.layer) else {
        // Player owns nothing that can break this layer. Clunk; no cooldown reset.
        return;
    };

    if matches!(*net_mode, crate::net::NetMode::Client { .. }) {
        dig_writer.send(DigRequest { target: target_tile });
        cooldown.0.reset();
        return;
    }

    let result = dig::try_dig(&mut grid, target_tile, tool);
    match result.status {
        DigStatus::Broken | DigStatus::Damaged => {
            cooldown.0.reset();
            // NEW: in Host mode, tell all clients about this tile change.
            // (SinglePlayer skips — no clients to notify.)
            if matches!(*net_mode, crate::net::NetMode::Host { .. }) {
                if let Some(new_tile) = grid.get(target_tile.x, target_tile.y).copied() {
                    tile_writer.send(ToClients {
                        mode: SendMode::Broadcast,
                        event: TileChanged { pos: target_tile, tile: new_tile },
                    });
                }
            }
            // Mark owning chunk dirty.
            let chunk_coord = IVec2::new(
                target_tile.x.div_euclid(CHUNK_TILES),
                target_tile.y.div_euclid(CHUNK_TILES),
            );
            for (e, c) in chunks_q.iter() {
                if c.coord == chunk_coord {
                    commands.entity(e).insert(ChunkDirty);
                    break;
                }
            }
            // Spawn ore drop only on full break.
            if result.status == DigStatus::Broken {
                if let Some(ore) = result.ore {
                    let item = ItemKind::Ore(ore);
                    commands.spawn((
                        OreDrop { item },
                        Sprite {
                            color: item_color(item),
                            custom_size: Some(Vec2::splat(6.0)),
                            ..default()
                        },
                        Transform::from_translation(tile_center.extend(4.0)),
                    ));
                }
            }
        }
        _ => { /* OutOfBounds / AlreadyEmpty / UnderTier / Blocked — no cooldown reset */ }
    }
}
