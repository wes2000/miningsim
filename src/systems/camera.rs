use bevy::prelude::*;
use crate::components::{LocalPlayer, MainCamera};

pub fn camera_follow_system(
    player_q: Query<&Transform, (With<LocalPlayer>, Without<MainCamera>)>,
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    let Ok(p) = player_q.get_single() else { return };
    let Ok(mut c) = cam_q.get_single_mut() else { return };
    let target = p.translation.truncate().extend(c.translation.z);
    let t = (time.delta_secs() * 6.0).clamp(0.0, 1.0);
    c.translation = c.translation.lerp(target, t);
}
