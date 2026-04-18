use bevy::prelude::*;
use crate::components::{Player, Velocity};

pub const PLAYER_SPEED_PX_PER_S: f32 = 120.0;

pub fn read_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Velocity, With<Player>>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO { dir = dir.normalize(); }
    for mut v in q.iter_mut() {
        v.0 = dir * PLAYER_SPEED_PX_PER_S;
    }
}

pub fn apply_velocity_system(
    time: Res<Time>,
    mut q: Query<(&Velocity, &mut Transform), With<Player>>,
) {
    let dt = time.delta_secs();
    for (v, mut t) in q.iter_mut() {
        t.translation.x += v.0.x * dt;
        t.translation.y += v.0.y * dt;
    }
}
