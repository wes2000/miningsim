use bevy::prelude::*;
use crate::components::{Player, Smelter, SmelterUiOpen};
use crate::coords::TILE_SIZE_PX;
use crate::processing::{self, SmelterState};

pub const SMELTER_INTERACT_RADIUS_TILES: f32 = 2.0;

pub fn smelter_interact_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_open: ResMut<SmelterUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    smelter_q: Query<&Transform, (With<Smelter>, Without<Player>)>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        ui_open.0 = false;
        return;
    }
    if !keys.just_pressed(KeyCode::KeyE) { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(smelter) = smelter_q.get_single() else { return };
    let dist = player.translation.truncate().distance(smelter.translation.truncate());
    if dist / TILE_SIZE_PX <= SMELTER_INTERACT_RADIUS_TILES {
        ui_open.0 = !ui_open.0;
    }
}

pub fn close_smelter_on_walk_away_system(
    mut ui_open: ResMut<SmelterUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    smelter_q: Query<&Transform, (With<Smelter>, Without<Player>)>,
) {
    if !ui_open.0 { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(smelter) = smelter_q.get_single() else { return };
    let dist = player.translation.truncate().distance(smelter.translation.truncate());
    if dist / TILE_SIZE_PX > SMELTER_INTERACT_RADIUS_TILES {
        ui_open.0 = false;
    }
}

pub fn smelter_tick_system(
    time: Res<Time>,
    mut q: Query<&mut SmelterState>,
) {
    let dt = time.delta_secs();
    for mut state in q.iter_mut() {
        let _ = processing::tick_smelter(&mut state, dt);
        // Event return value is unused for M3; M4 (events bus) or M7 (audio) may consume it.
    }
}
