use bevy::prelude::*;
use crate::systems::{camera, chunk_lifecycle, chunk_render, hud, ore_drop, player, setup};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup::setup_world, hud::setup_hud))
            .add_systems(Update, (
                player::read_input_system,
                player::apply_velocity_system,
                player::collide_player_with_grid_system,
                player::dig_input_system,
                ore_drop::ore_drop_system,
                chunk_lifecycle::chunk_lifecycle_system,
                chunk_render::chunk_remesh_system,
                camera::camera_follow_system,
                hud::update_hud_system,
            ).chain());
    }
}
