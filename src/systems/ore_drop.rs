use bevy::prelude::*;
use crate::components::{LocalPlayer, OreDrop, Player};
use crate::inventory::Inventory;
use crate::coords::TILE_SIZE_PX;

pub const VACUUM_RADIUS_TILES: f32 = 1.0;
pub const VACUUM_SPEED_PX_PER_S: f32 = 200.0;
pub const PICKUP_DISTANCE_PX: f32 = 6.0;

pub fn ore_drop_system(
    mut commands: Commands,
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut drops_q: Query<(Entity, &OreDrop, &mut Transform), Without<Player>>,
    local_inv: Single<&mut Inventory, With<LocalPlayer>>,
) {
    let Ok(player_xf) = player_q.get_single() else { return };
    let player_pos = player_xf.translation.truncate();
    let mut inv = local_inv.into_inner();

    for (entity, drop, mut t) in drops_q.iter_mut() {
        let to_player = player_pos - t.translation.truncate();
        let dist = to_player.length();
        if dist < PICKUP_DISTANCE_PX {
            inv.add(drop.item, 1);
            commands.entity(entity).despawn();
            continue;
        }
        if dist / TILE_SIZE_PX < VACUUM_RADIUS_TILES {
            let step = to_player.normalize() * VACUUM_SPEED_PX_PER_S * time.delta_secs();
            t.translation.x += step.x;
            t.translation.y += step.y;
        }
    }
}
