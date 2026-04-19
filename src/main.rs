use bevy::prelude::*;
use miningsim::app::MiningSimPlugin;
use miningsim::net::{self, NetMode};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let net_mode = match net::parse_args(&args) {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("CLI parse error: {:?} — falling back to single-player", err);
            NetMode::SinglePlayer
        }
    };

    App::new()
        .insert_resource(net_mode)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MiningSim".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MiningSimPlugin)
        .run();
}
