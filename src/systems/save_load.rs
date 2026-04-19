use std::fs;
use std::path::Path;

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::components::{LocalPlayer, Player, Smelter};
use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::save::{self, LoadError};
use crate::tools::OwnedTools;

pub const SAVE_PATH: &str = "save.ron";

/// Startup system, ordered AFTER setup_world + UI spawners. If
/// `./save.ron` exists, load and apply. Resources are guaranteed present
/// since `setup_world` precedes us in the chained Startup tuple.
pub fn startup_load_system(
    grid: ResMut<Grid>,
    local_player: Single<(&mut Money, &mut Inventory, &mut OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&mut SmelterState, With<Smelter>>,
    player_q: Query<&mut Transform, With<Player>>,
) {
    if !Path::new(SAVE_PATH).exists() {
        info!("no save file found, starting fresh");
        return;
    }
    let (mut money, mut inventory, mut owned) = local_player.into_inner();
    if let Err(e) = try_load_and_apply(grid, &mut inventory, &mut money, &mut owned, smelter_q, player_q) {
        error!("save load failed: {:?}", e);
    } else {
        info!("save loaded");
    }
}

pub fn save_hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    grid: Res<Grid>,
    local_player: Single<(&Money, &Inventory, &OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&SmelterState, With<Smelter>>,
    player_q: Query<&Transform, With<Player>>,
) {
    if !keys.just_pressed(KeyCode::F5) { return; }
    let (money, inventory, owned) = local_player.into_inner();
    save_now(&grid, inventory, money, owned, &smelter_q, &player_q);
}

pub fn load_hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    grid: ResMut<Grid>,
    local_player: Single<(&mut Money, &mut Inventory, &mut OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&mut SmelterState, With<Smelter>>,
    player_q: Query<&mut Transform, With<Player>>,
) {
    if !keys.just_pressed(KeyCode::F9) { return; }
    let (mut money, mut inventory, mut owned) = local_player.into_inner();
    if let Err(e) = try_load_and_apply(grid, &mut inventory, &mut money, &mut owned, smelter_q, player_q) {
        error!("save load failed: {:?}", e);
    } else {
        info!("save loaded");
    }
}

pub fn auto_save_on_exit_system(
    mut exit_events: EventReader<AppExit>,
    grid: Res<Grid>,
    local_player: Single<(&Money, &Inventory, &OwnedTools), With<LocalPlayer>>,
    smelter_q: Query<&SmelterState, With<Smelter>>,
    player_q: Query<&Transform, With<Player>>,
) {
    if exit_events.read().next().is_none() { return; }
    info!("auto-saving on exit");
    let (money, inventory, owned) = local_player.into_inner();
    save_now(&grid, inventory, money, owned, &smelter_q, &player_q);
}

// --- helpers (private) -------------------------------------------------------

fn save_now(
    grid: &Grid,
    inventory: &Inventory,
    money: &Money,
    owned: &OwnedTools,
    smelter_q: &Query<&SmelterState, With<Smelter>>,
    player_q: &Query<&Transform, With<Player>>,
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
    let data = save::collect(grid, inventory, money, owned, smelter, pos);
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
    mut grid: ResMut<Grid>,
    inventory: &mut Inventory,
    money: &mut Money,
    owned: &mut OwnedTools,
    mut smelter_q: Query<&mut SmelterState, With<Smelter>>,
    mut player_q: Query<&mut Transform, With<Player>>,
) -> Result<(), LoadError> {
    let s = fs::read_to_string(SAVE_PATH).map_err(LoadError::Io)?;
    let data = save::deserialize_ron(&s)?;

    let Ok(mut smelter) = smelter_q.get_single_mut() else {
        warn!("apply: smelter entity missing; skipping smelter restore");
        // Apply the rest using a throwaway state
        let mut throwaway = SmelterState::default();
        let mut pos = [0.0_f32, 0.0_f32];
        save::apply(data, &mut grid, inventory, money, owned, &mut throwaway, &mut pos);
        if let Ok(mut player_xf) = player_q.get_single_mut() {
            player_xf.translation.x = pos[0];
            player_xf.translation.y = pos[1];
        }
        return Ok(());
    };
    let Ok(mut player_xf) = player_q.get_single_mut() else {
        warn!("apply: player entity missing; skipping player position restore");
        let mut pos = [0.0_f32, 0.0_f32];
        save::apply(data, &mut grid, inventory, money, owned, &mut smelter, &mut pos);
        return Ok(());
    };

    let mut pos = [player_xf.translation.x, player_xf.translation.y];
    save::apply(data, &mut grid, inventory, money, owned, &mut smelter, &mut pos);
    player_xf.translation.x = pos[0];
    player_xf.translation.y = pos[1];
    Ok(())
}
