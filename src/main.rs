use bevy::prelude::*;
use miningsim::app::MiningSimPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MiningSim — Milestone 1".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MiningSimPlugin)
        .run();
}
