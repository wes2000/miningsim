use bevy::prelude::*;
use crate::components::{MainCamera, Player, Velocity};
use crate::inventory::Inventory;
use crate::terrain_gen;

pub const TILE_SIZE_PX: f32 = 16.0;
pub const MAP_W: u32 = 80;
pub const MAP_H: u32 = 200;

pub fn setup_world(mut commands: Commands) {
    let seed: u64 = rand::random();
    info!("world seed: {}", seed);     // logged so playtests can be reproduced
    let grid = terrain_gen::generate(MAP_W, MAP_H, seed);
    let sp = terrain_gen::spawn_tile(&grid);
    let player_world = tile_center_world(sp.0, sp.1);

    commands.insert_resource(grid);
    commands.insert_resource(Inventory::default());
    commands.insert_resource(crate::systems::player::DigCooldown::default());

    // Player
    commands.spawn((
        Player,
        Velocity::default(),
        Sprite {
            color: Color::srgb(0.30, 0.60, 0.90),
            custom_size: Some(Vec2::splat(12.0)),
            ..default()
        },
        Transform::from_translation(player_world.extend(10.0)),
    ));

    // Camera
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_translation(player_world.extend(100.0)),
    ));
}

pub fn tile_center_world(x: i32, y: i32) -> Vec2 {
    Vec2::new(
        x as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0,
        // invert Y so deeper tiles render below in world (Bevy Y goes up)
        -(y as f32 * TILE_SIZE_PX + TILE_SIZE_PX / 2.0),
    )
}
