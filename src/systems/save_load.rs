use std::fs;
use std::path::Path;

use bevy::app::AppExit;
use bevy::math::IVec2;
use bevy::prelude::*;

use crate::belt::{BeltTile, BeltVisual};
use crate::components::{LocalPlayer, Smelter};
use crate::coords::{tile_center_world, world_to_tile, TILE_SIZE_PX};
use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::save::{self, LoadError};
use crate::systems::belt_ui::belt_color;
use crate::tools::OwnedTools;

pub const SAVE_PATH: &str = "save.ron";

/// Startup system, ordered AFTER setup_world + UI spawners. If
/// `./save.ron` exists, load and apply. Resources are guaranteed present
/// since `setup_world` precedes us in the chained Startup tuple.
pub fn startup_load_system(
    mut commands: Commands,
    grid: Single<&mut Grid>,
    local_player: Single<(&mut Money, &mut Inventory, &mut OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&mut SmelterState, With<Smelter>>,
    player_q: Query<&mut Transform, With<LocalPlayer>>,
    existing_belts: Query<Entity, With<BeltTile>>,
) {
    if !Path::new(SAVE_PATH).exists() {
        info!("no save file found, starting fresh");
        return;
    }
    let mut grid = grid.into_inner();
    let (mut money, mut inventory, mut owned) = local_player.into_inner();
    if let Err(e) = try_load_and_apply(&mut commands, &mut grid, &mut inventory, &mut money, &mut owned, smelter_q, player_q, existing_belts) {
        error!("save load failed: {:?}", e);
    } else {
        info!("save loaded");
    }
}

pub fn save_hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    grid: Single<&Grid>,
    local_player: Single<(&Money, &Inventory, &OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&SmelterState, With<Smelter>>,
    player_q: Query<&Transform, With<LocalPlayer>>,
    belts_q: Query<(&Transform, &BeltTile)>,
) {
    if !keys.just_pressed(KeyCode::F5) { return; }
    let (money, inventory, owned) = local_player.into_inner();
    save_now(&grid, inventory, money, owned, &smelter_q, &player_q, &belts_q);
}

pub fn load_hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    grid: Single<&mut Grid>,
    local_player: Single<(&mut Money, &mut Inventory, &mut OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&mut SmelterState, With<Smelter>>,
    player_q: Query<&mut Transform, With<LocalPlayer>>,
    existing_belts: Query<Entity, With<BeltTile>>,
) {
    if !keys.just_pressed(KeyCode::F9) { return; }
    let mut grid = grid.into_inner();
    let (mut money, mut inventory, mut owned) = local_player.into_inner();
    if let Err(e) = try_load_and_apply(&mut commands, &mut grid, &mut inventory, &mut money, &mut owned, smelter_q, player_q, existing_belts) {
        error!("save load failed: {:?}", e);
    } else {
        info!("save loaded");
    }
}

pub fn auto_save_on_exit_system(
    mut exit_events: EventReader<AppExit>,
    grid: Single<&Grid>,
    local_player: Single<(&Money, &Inventory, &OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&SmelterState, With<Smelter>>,
    player_q: Query<&Transform, With<LocalPlayer>>,
    belts_q: Query<(&Transform, &BeltTile)>,
) {
    if exit_events.read().next().is_none() { return; }
    info!("auto-saving on exit");
    let (money, inventory, owned) = local_player.into_inner();
    save_now(&grid, inventory, money, owned, &smelter_q, &player_q, &belts_q);
}

// --- helpers (private) -------------------------------------------------------

fn save_now(
    grid: &Grid,
    inventory: &Inventory,
    money: &Money,
    owned: &OwnedTools,
    smelter_q: &Query<&SmelterState, With<Smelter>>,
    player_q: &Query<&Transform, With<LocalPlayer>>,
    belts_q: &Query<(&Transform, &BeltTile)>,
) {
    let Ok(smelter) = smelter_q.get_single() else {
        warn!("save_now: smelter entity missing; skipping save");
        return;
    };
    let Ok(player_xf) = player_q.get_single() else {
        warn!("save_now: player entity missing; skipping save");
        return;
    };
    let pos = [player_xf.translation.x, player_xf.translation.y];
    let belts: Vec<(IVec2, BeltTile)> = belts_q
        .iter()
        .map(|(xf, bt)| (world_to_tile(xf.translation.truncate()), *bt))
        .collect();
    let data = save::collect(grid, inventory, money, owned, smelter, pos, belts);
    let s = match save::serialize_ron(&data) {
        Ok(s) => s,
        Err(e) => {
            error!("save serialize failed: {:?}", e);
            return;
        }
    };
    if let Err(e) = fs::write(SAVE_PATH, s) {
        error!("save write failed: {:?}", e);
        return;
    }
    info!("game saved");
}

fn try_load_and_apply(
    commands: &mut Commands,
    grid: &mut Grid,
    inventory: &mut Inventory,
    money: &mut Money,
    owned: &mut OwnedTools,
    mut smelter_q: Query<&mut SmelterState, With<Smelter>>,
    mut player_q: Query<&mut Transform, With<LocalPlayer>>,
    existing_belts: Query<Entity, With<BeltTile>>,
) -> Result<(), LoadError> {
    let s = fs::read_to_string(SAVE_PATH).map_err(LoadError::Io)?;
    let data = save::deserialize_ron(&s)?;

    let loaded_belts = if let Ok(mut smelter) = smelter_q.get_single_mut() {
        if let Ok(mut player_xf) = player_q.get_single_mut() {
            let mut pos = [player_xf.translation.x, player_xf.translation.y];
            let belts = save::apply(data, grid, inventory, money, owned, &mut smelter, &mut pos);
            player_xf.translation.x = pos[0];
            player_xf.translation.y = pos[1];
            belts
        } else {
            warn!("apply: player entity missing; skipping player position restore");
            let mut pos = [0.0_f32, 0.0_f32];
            save::apply(data, grid, inventory, money, owned, &mut smelter, &mut pos)
        }
    } else {
        warn!("apply: smelter entity missing; skipping smelter restore");
        let mut throwaway = SmelterState::default();
        let mut pos = [0.0_f32, 0.0_f32];
        let belts = save::apply(data, grid, inventory, money, owned, &mut throwaway, &mut pos);
        if let Ok(mut player_xf) = player_q.get_single_mut() {
            player_xf.translation.x = pos[0];
            player_xf.translation.y = pos[1];
        }
        belts
    };

    // Despawn old belts and spawn belts from the load.
    for e in existing_belts.iter() {
        commands.entity(e).despawn();
    }
    for (pos, belt_tile) in loaded_belts {
        let center = tile_center_world(pos);
        commands.spawn((
            belt_tile,
            BeltVisual::Straight,
            Sprite {
                color: belt_color(belt_tile.dir),
                custom_size: Some(Vec2::splat(TILE_SIZE_PX)),
                ..default()
            },
            Transform::from_translation(center.extend(3.0)),
            bevy_replicon::prelude::Replicated,
        ));
    }
    Ok(())
}

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_system.after(crate::systems::setup::setup_world))
            .add_systems(Update, (
                save_hotkey_system.in_set(crate::app::UiSet::SaveLoad),
                load_hotkey_system.in_set(crate::app::UiSet::SaveLoad),
                auto_save_on_exit_system.in_set(crate::app::UiSet::SaveLoad),
            ));
    }
}
