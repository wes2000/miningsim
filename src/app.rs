use bevy::prelude::*;

use crate::systems::{camera, setup};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup::setup_world)
            .add_systems(Update, (
                crate::systems::player::read_input_system,
                crate::systems::player::apply_velocity_system,
                camera::camera_follow_system,
            ).chain());
    }
}
