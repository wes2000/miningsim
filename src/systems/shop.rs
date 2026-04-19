use bevy::prelude::*;
use crate::components::{Player, Shop, ShopUiOpen};
use crate::coords::TILE_SIZE_PX;

pub const SHOP_INTERACT_RADIUS_TILES: f32 = 2.0;

pub fn shop_interact_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_open: ResMut<ShopUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    shop_q: Query<&Transform, (With<Shop>, Without<Player>)>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        ui_open.0 = false;
        return;
    }
    if !keys.just_pressed(KeyCode::KeyE) { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(shop) = shop_q.get_single() else { return };
    let dist = player.translation.truncate().distance(shop.translation.truncate());
    if dist / TILE_SIZE_PX <= SHOP_INTERACT_RADIUS_TILES {
        ui_open.0 = !ui_open.0;
    }
}

pub fn close_shop_on_walk_away_system(
    mut ui_open: ResMut<ShopUiOpen>,
    player_q: Query<&Transform, With<Player>>,
    shop_q: Query<&Transform, (With<Shop>, Without<Player>)>,
) {
    if !ui_open.0 { return; }
    let Ok(player) = player_q.get_single() else { return };
    let Ok(shop) = shop_q.get_single() else { return };
    let dist = player.translation.truncate().distance(shop.translation.truncate());
    if dist / TILE_SIZE_PX > SHOP_INTERACT_RADIUS_TILES {
        ui_open.0 = false;
    }
}
