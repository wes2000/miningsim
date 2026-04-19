use bevy::prelude::*;
use crate::systems::{camera, chunk_lifecycle, chunk_render, hud, ore_drop, player, setup, shop, shop_ui};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
                setup::setup_world,
                hud::setup_top_right_hud,
                hud::spawn_inventory_popup,
                shop_ui::spawn_shop_ui,
            ).chain())
           .add_systems(Update, (
                player::read_input_system,
                player::apply_velocity_system,
                player::collide_player_with_grid_system,
                player::dig_input_system,
                shop::shop_interact_system,
                shop::close_shop_on_walk_away_system,
                shop_ui::sync_shop_visibility_system,
                shop_ui::update_shop_labels_system,
                shop_ui::handle_shop_buttons_system,
                hud::toggle_inventory_popup_system,
                hud::sync_inventory_popup_visibility_system,
                ore_drop::ore_drop_system,
                chunk_lifecycle::chunk_lifecycle_system,
                chunk_render::chunk_remesh_system,
                camera::camera_follow_system,
                hud::update_money_text_system,
                hud::update_inventory_popup_system,
            ).chain());
    }
}
