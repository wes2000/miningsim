use bevy::prelude::*;

use crate::systems::{camera, setup};

pub struct MiningSimPlugin;

impl Plugin for MiningSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup::setup_world)
            .add_systems(Update, camera::camera_follow_system);
    }
}
