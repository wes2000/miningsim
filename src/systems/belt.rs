use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::belt::{self, BeltTile};
use crate::coords::world_to_tile;

pub const BELT_TICK_SECONDS: f32 = 1.0;

#[derive(Resource)]
pub struct BeltTickTimer(pub Timer);

impl Default for BeltTickTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(BELT_TICK_SECONDS, TimerMode::Repeating))
    }
}

pub fn belt_tick_system(
    time: Res<Time>,
    mut timer: ResMut<BeltTickTimer>,
    mut belts_q: Query<(Entity, &Transform, &mut BeltTile)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() { return }

    let mut belt_dirs: HashMap<bevy::math::IVec2, belt::BeltDir> = HashMap::new();
    let mut entity_at: HashMap<bevy::math::IVec2, Entity> = HashMap::new();
    let mut item_at: HashMap<bevy::math::IVec2, crate::items::ItemKind> = HashMap::new();
    for (e, xf, bt) in belts_q.iter() {
        let pos = world_to_tile(xf.translation.truncate());
        belt_dirs.insert(pos, bt.dir);
        entity_at.insert(pos, e);
        if let Some(item) = bt.item {
            item_at.insert(pos, item);
        }
    }
    let items_present: HashSet<bevy::math::IVec2> = item_at.keys().copied().collect();

    let moves = belt::compute_belt_advances(&belt_dirs, &items_present);

    let mut new_item_for_entity: HashMap<Entity, Option<crate::items::ItemKind>> = HashMap::new();
    for (from, to) in &moves {
        if !entity_at.contains_key(to) { continue }
        let item = item_at.get(from).copied();
        if let Some(item) = item {
            if let Some(&from_e) = entity_at.get(from) {
                new_item_for_entity.insert(from_e, None);
            }
            if let Some(&to_e) = entity_at.get(to) {
                new_item_for_entity.insert(to_e, Some(item));
            }
        }
    }

    for (e, _, mut bt) in belts_q.iter_mut() {
        if let Some(&new_item) = new_item_for_entity.get(&e) {
            bt.item = new_item;
        }
    }
}
