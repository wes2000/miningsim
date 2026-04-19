use bevy::prelude::*;
// Replicated marker is inert in single-player (no replicon plugin); attached to
// Grid + Player so they replicate when MultiplayerPlugin is loaded in Host mode.
use bevy_replicon::prelude::Replicated;
use crate::components::{Facing, InventoryPopupOpen, LocalPlayer, MainCamera, NetOwner, Player, Shop, ShopUiOpen, Smelter, SmelterUiOpen, Velocity};
use crate::coords::tile_center_world;
use crate::economy::Money;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::terrain_gen;
use crate::tools::OwnedTools;

pub const MAP_W: u32 = 80;
pub const MAP_H: u32 = 200;

pub fn setup_world(mut commands: Commands, net_mode: Res<crate::net::NetMode>) {
    let is_client = matches!(*net_mode, crate::net::NetMode::Client { .. });

    // UI/input resources are needed in EVERY mode (single-player, host, client).
    commands.insert_resource(crate::systems::player::DigCooldown::default());
    commands.insert_resource(ShopUiOpen::default());
    commands.insert_resource(SmelterUiOpen::default());
    commands.insert_resource(InventoryPopupOpen::default());

    // World-state spawns (Grid, Player, Shop, Smelter) are ONLY done in
    // single-player or host mode. In client mode replicon ships these from
    // the host; spawning them locally too would cause duplicates and panic
    // `Single<&mut Grid>` queries (e.g., dig_input_system).
    //
    // In client mode no Grid yet → no known player_world, so the camera
    // starts at origin and `camera_follow_system` will catch up once the
    // LocalPlayer is tagged via `mark_local_player_on_arrival`.
    let camera_world = if !is_client {
        let seed: u64 = rand::random();
        info!("world seed: {}", seed);     // logged so playtests can be reproduced
        let grid = terrain_gen::generate(MAP_W, MAP_H, seed);
        let sp = terrain_gen::spawn_tile(&grid);
        let player_world = tile_center_world(IVec2::new(sp.0, sp.1));

        // Grid lives as a Component on a singleton entity so replicon can stream
        // it to clients (Task 9.5). Replicated marker is a no-op in single-player.
        commands.spawn((grid, Replicated));

        // Player. `NetOwner(HOST_NET_OWNER=u64::MAX)` + `Replicated` mark this as
        // the host's player so remote clients see it as a peer (RemotePlayer-tagged
        // sprite). Both are no-ops in single-player.
        commands.spawn((
            Player,
            LocalPlayer,
            NetOwner(crate::systems::net_player::HOST_NET_OWNER),
            Velocity::default(),
            Facing::default(),
            Money::default(),
            Inventory::default(),
            OwnedTools::default(),
            Sprite {
                color: Color::srgb(0.30, 0.60, 0.90),
                custom_size: Some(Vec2::splat(12.0)),
                ..default()
            },
            Transform::from_translation(player_world.extend(10.0)),
            Replicated,
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
            Replicated,
        ));

        // Smelter
        let smelter_tile = IVec2::new(sp.0 - 3, sp.1);   // 3 tiles left of player spawn
        let smelter_world = tile_center_world(smelter_tile);
        commands.spawn((
            Smelter,
            SmelterState::default(),
            Sprite {
                color: Color::srgb(0.95, 0.50, 0.20),   // orange placeholder
                custom_size: Some(Vec2::splat(14.0)),
                ..default()
            },
            Transform::from_translation(smelter_world.extend(5.0)),
            Replicated,
        ));

        player_world.extend(100.0)
    } else {
        Vec3::ZERO.with_z(100.0)
    };

    // Camera spawns in EVERY mode. Client needs a camera even before the
    // replicated Player arrives.
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_translation(camera_world),
    ));
}
