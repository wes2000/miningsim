//! Build-mode UX for conveyor belts (M5a Task 3).
//!
//! Per-peer local state. None of this is replicated — each peer toggles their
//! own build mode independently. Placement and removal go through the
//! single-player / host direct-spawn path here in M5a; Task 10 adds a
//! NetMode::Client branch that fires PlaceBeltRequest / RemoveBeltRequest
//! events instead.
//!
//! Systems:
//!   * `belt_build_toggle_system`     — B keypress toggles build mode (Esc exits).
//!                                      Gated on local player owning Tool::BeltUnlock.
//!   * `belt_build_rotate_system`     — Scroll wheel rotates cursor direction CW.
//!   * `belt_ghost_render_system`     — Spawn / move / despawn a translucent
//!                                      ghost belt entity following the cursor.
//!   * `belt_place_system`            — Left-click places a belt at cursor tile.
//!   * `belt_remove_system`           — Right-click despawns belt at cursor tile.
//!                                      Spills any item on it as an OreDrop sprite.
//!   * `belt_visual_recompute_system` — Rebuilds BeltVisual (corner kind) when
//!                                      a belt is added/removed/changed. Gated
//!                                      on Changed<BeltTile> + RemovedComponents.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::belt::{self, BeltDir, BeltTile, BeltVisual};
use crate::components::{BeltGhost, LocalPlayer, MainCamera, OreDrop, Shop, Smelter};
use crate::coords::{tile_center_world, world_to_tile, TILE_SIZE_PX};
use crate::grid::Grid;
use crate::tools::{OwnedTools, Tool};

/// Per-peer local state. `None` = not in build mode.
#[derive(Resource, Default)]
pub struct BeltBuildMode {
    pub cursor_dir: Option<BeltDir>,
}

pub fn belt_build_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut build_mode: ResMut<BeltBuildMode>,
    owned: Option<Single<&OwnedTools, With<LocalPlayer>>>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        let Some(owned) = owned else { return };
        if !owned.0.contains(&Tool::BeltUnlock) {
            return;
        }
        build_mode.cursor_dir = match build_mode.cursor_dir {
            None => Some(BeltDir::East),
            Some(_) => None,
        };
    }
    if keys.just_pressed(KeyCode::Escape) {
        build_mode.cursor_dir = None;
    }
}

pub fn belt_build_rotate_system(
    mut wheel: EventReader<MouseWheel>,
    mut build_mode: ResMut<BeltBuildMode>,
) {
    let Some(dir) = build_mode.cursor_dir else {
        wheel.clear();
        return;
    };
    let mut new_dir = dir;
    for ev in wheel.read() {
        if ev.y > 0.0 {
            new_dir = new_dir.rotate_cw();
        } else if ev.y < 0.0 {
            // Counter-clockwise = three CW rotations.
            new_dir = new_dir.rotate_cw().rotate_cw().rotate_cw();
        }
    }
    build_mode.cursor_dir = Some(new_dir);
}

pub fn belt_ghost_render_system(
    mut commands: Commands,
    build_mode: Res<BeltBuildMode>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    ghost_q: Query<Entity, With<BeltGhost>>,
) {
    let Some(dir) = build_mode.cursor_dir else {
        for e in ghost_q.iter() {
            commands.entity(e).despawn();
        }
        return;
    };

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
    let tile = world_to_tile(cursor_world);
    let center = tile_center_world(tile);

    let color = ghost_color(dir);

    if let Ok(existing) = ghost_q.get_single() {
        commands.entity(existing).insert((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                ..default()
            },
            Transform::from_translation(center.extend(8.0)),
        ));
    } else {
        commands.spawn((
            BeltGhost,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                ..default()
            },
            Transform::from_translation(center.extend(8.0)),
        ));
    }
}

fn ghost_color(dir: BeltDir) -> Color {
    let alpha = 0.40;
    match dir {
        BeltDir::North => Color::srgba(0.30, 0.80, 0.30, alpha),
        BeltDir::East  => Color::srgba(0.80, 0.80, 0.30, alpha),
        BeltDir::South => Color::srgba(0.80, 0.30, 0.30, alpha),
        BeltDir::West  => Color::srgba(0.30, 0.30, 0.80, alpha),
    }
}

pub(crate) fn belt_color(dir: BeltDir) -> Color {
    match dir {
        BeltDir::North => Color::srgb(0.20, 0.55, 0.20),
        BeltDir::East  => Color::srgb(0.60, 0.55, 0.20),
        BeltDir::South => Color::srgb(0.55, 0.20, 0.20),
        BeltDir::West  => Color::srgb(0.20, 0.20, 0.55),
    }
}

pub fn belt_place_system(
    mut commands: Commands,
    build_mode: Res<BeltBuildMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    grid_q: Option<Single<&Grid>>,
    belts_q: Query<&Transform, With<BeltTile>>,
    shops_q: Query<&Transform, With<Shop>>,
    smelters_q: Query<&Transform, With<Smelter>>,
    net_mode: Res<crate::net::NetMode>,
    mut place_writer: EventWriter<crate::systems::net_events::PlaceBeltRequest>,
) {
    let Some(dir) = build_mode.cursor_dir else { return };
    if !mouse.just_pressed(MouseButton::Left) { return }

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
    let tile = world_to_tile(cursor_world);

    if matches!(*net_mode, crate::net::NetMode::Client { .. }) {
        place_writer.send(crate::systems::net_events::PlaceBeltRequest { tile, dir });
        return;
    }

    let Some(grid) = grid_q else { return };
    let grid = grid.into_inner();
    let in_bounds_and_floor = grid.get(tile.x, tile.y).is_some_and(|g| !g.solid);
    let occupied: std::collections::HashSet<bevy::math::IVec2> = belts_q
        .iter()
        .chain(shops_q.iter())
        .chain(smelters_q.iter())
        .map(|xf| world_to_tile(xf.translation.truncate()))
        .collect();
    if !belt::can_place_belt(tile, in_bounds_and_floor, &occupied) { return }

    let center = tile_center_world(tile);
    commands.spawn((
        BeltTile::new(dir),
        BeltVisual::Straight,
        Sprite {
            color: belt_color(dir),
            custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
            ..default()
        },
        Transform::from_translation(center.extend(3.0)),
        bevy_replicon::prelude::Replicated,
    ));
}

pub fn belt_remove_system(
    mut commands: Commands,
    build_mode: Res<BeltBuildMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    win_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    belts_q: Query<(Entity, &Transform, &BeltTile)>,
    net_mode: Res<crate::net::NetMode>,
    mut remove_writer: EventWriter<crate::systems::net_events::RemoveBeltRequest>,
) {
    if build_mode.cursor_dir.is_none() { return }
    if !mouse.just_pressed(MouseButton::Right) { return }

    let Ok(win) = win_q.get_single() else { return };
    let Some(cursor_screen) = win.cursor_position() else { return };
    let Ok((cam, cam_xf)) = cam_q.get_single() else { return };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_xf, cursor_screen) else { return };
    let target = world_to_tile(cursor_world);

    if matches!(*net_mode, crate::net::NetMode::Client { .. }) {
        remove_writer.send(crate::systems::net_events::RemoveBeltRequest { tile: target });
        return;
    }

    for (e, xf, belt_tile) in belts_q.iter() {
        let pos = world_to_tile(xf.translation.truncate());
        if pos != target { continue }
        if let Some(item) = belt_tile.item {
            let center = tile_center_world(pos);
            commands.spawn((
                OreDrop { item },
                Sprite {
                    color: crate::systems::hud::item_color(item),
                    custom_size: Some(Vec2::splat(6.0)),
                    ..default()
                },
                Transform::from_translation(center.extend(4.0)),
            ));
        }
        commands.entity(e).despawn();
        return;
    }
}

pub fn belt_visual_recompute_system(
    changed_q: Query<(), Changed<BeltTile>>,
    mut removed: RemovedComponents<BeltTile>,
    mut belts_q: Query<(&Transform, &BeltTile, &mut BeltVisual)>,
    all_belts_q: Query<(&Transform, &BeltTile)>,
) {
    let any_removed = removed.read().count() > 0;
    if changed_q.is_empty() && !any_removed { return }

    use std::collections::HashMap;
    let map: HashMap<bevy::math::IVec2, BeltDir> = all_belts_q
        .iter()
        .map(|(xf, bt)| (world_to_tile(xf.translation.truncate()), bt.dir))
        .collect();

    for (xf, bt, mut visual) in belts_q.iter_mut() {
        let pos = world_to_tile(xf.translation.truncate());
        let feeder = perpendicular_feeder(pos, bt.dir, &map);
        let new_visual = belt::belt_visual_kind(bt.dir, feeder);
        if *visual != new_visual {
            *visual = new_visual;
        }
    }
}

fn perpendicular_feeder(
    pos: bevy::math::IVec2,
    self_dir: BeltDir,
    map: &std::collections::HashMap<bevy::math::IVec2, BeltDir>,
) -> Option<BeltDir> {
    let perps = match self_dir {
        BeltDir::East | BeltDir::West => [BeltDir::North, BeltDir::South],
        BeltDir::North | BeltDir::South => [BeltDir::East, BeltDir::West],
    };
    for perp in perps {
        let neighbor_pos = pos + perp.opposite().delta();
        let Some(neighbor_dir) = map.get(&neighbor_pos) else { continue };
        // For the neighbor to feed INTO us, its dir must point from neighbor_pos to pos.
        if neighbor_dir.delta() == perp.delta() {
            return Some(*neighbor_dir);
        }
    }
    None
}
