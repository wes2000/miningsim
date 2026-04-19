use bevy::prelude::*;
use crate::components::{Facing, MainCamera, Player, Shop, ShopUiOpen, Velocity};
use crate::coords::tile_center_world;
use crate::economy::Money;
use crate::inventory::Inventory;
use crate::terrain_gen;
use crate::tools::OwnedTools;

pub const MAP_W: u32 = 80;
pub const MAP_H: u32 = 200;

pub fn setup_world(mut commands: Commands) {
    let seed: u64 = rand::random();
    info!("world seed: {}", seed);     // logged so playtests can be reproduced
    let grid = terrain_gen::generate(MAP_W, MAP_H, seed);
    let sp = terrain_gen::spawn_tile(&grid);
    let player_world = tile_center_world(IVec2::new(sp.0, sp.1));

    commands.insert_resource(grid);
    commands.insert_resource(Inventory::default());
    commands.insert_resource(crate::systems::player::DigCooldown::default());
    commands.insert_resource(Money::default());
    commands.insert_resource(OwnedTools::default());
    commands.insert_resource(ShopUiOpen::default());

    // Player
    commands.spawn((
        Player,
        Velocity::default(),
        Facing::default(),
        Sprite {
            color: Color::srgb(0.30, 0.60, 0.90),
            custom_size: Some(Vec2::splat(12.0)),
            ..default()
        },
        Transform::from_translation(player_world.extend(10.0)),
    ));

    // Shop
    let shop_tile = IVec2::new(sp.0 + 3, sp.1);   // 3 tiles right of player spawn
    let shop_world = tile_center_world(shop_tile);
    commands.spawn((
        Shop,
        Sprite {
            color: Color::srgb(0.95, 0.80, 0.20),   // yellow placeholder
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        },
        Transform::from_translation(shop_world.extend(5.0)),
    ));

    // Camera
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_translation(player_world.extend(100.0)),
    ));
}
