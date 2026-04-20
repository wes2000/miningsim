use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::belt::{self, BeltDir, BeltTile};
use crate::components::{OreDrop, Smelter};
use crate::coords::{tile_center_world, world_to_tile};
use crate::items::{ItemKind, OreKind};
use crate::processing::{self, SmelterState};
use crate::systems::hud::item_color;

const SMELTER_QUEUE_CAP: u32 = 8;

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

pub fn belt_pickup_system(
    mut commands: Commands,
    drops_q: Query<(Entity, &Transform, &OreDrop)>,
    mut belts_q: Query<(&Transform, &mut BeltTile)>,
) {
    let mut available_tiles: HashSet<bevy::math::IVec2> = belts_q
        .iter()
        .filter(|(_, bt)| bt.item.is_none())
        .map(|(xf, _)| world_to_tile(xf.translation.truncate()))
        .collect();

    for (drop_entity, drop_xf, drop_data) in drops_q.iter() {
        let drop_tile = world_to_tile(drop_xf.translation.truncate());
        if !available_tiles.contains(&drop_tile) { continue }

        for (belt_xf, mut belt_tile) in belts_q.iter_mut() {
            let pos = world_to_tile(belt_xf.translation.truncate());
            if pos != drop_tile { continue }
            if belt_tile.item.is_some() { break }
            belt_tile.item = Some(drop_data.item);
            commands.entity(drop_entity).despawn();
            available_tiles.remove(&pos);
            break;
        }
    }
}

pub fn belt_spillage_system(
    mut commands: Commands,
    mut belts_q: Query<(&Transform, &mut BeltTile)>,
    smelter_xf_q: Query<&Transform, With<Smelter>>,
) {
    let belt_positions: HashSet<bevy::math::IVec2> = belts_q
        .iter()
        .map(|(xf, _)| world_to_tile(xf.translation.truncate()))
        .collect();
    let smelter_positions: HashSet<bevy::math::IVec2> = smelter_xf_q
        .iter()
        .map(|xf| world_to_tile(xf.translation.truncate()))
        .collect();

    for (xf, mut bt) in belts_q.iter_mut() {
        let Some(item) = bt.item else { continue };
        let pos = world_to_tile(xf.translation.truncate());
        let dest = belt::next_tile(pos, bt.dir);
        if belt_positions.contains(&dest) || smelter_positions.contains(&dest) { continue }
        let dest_world = tile_center_world(dest);
        commands.spawn((
            OreDrop { item },
            Sprite {
                color: item_color(item),
                custom_size: Some(Vec2::splat(6.0)),
                ..default()
            },
            Transform::from_translation(dest_world.extend(4.0)),
        ));
        bt.item = None;
    }
}

pub fn smelter_belt_io_system(
    mut belts_q: Query<(&Transform, &mut BeltTile)>,
    mut smelters_q: Query<(&Transform, &mut SmelterState)>,
) {
    let belt_positions: HashMap<bevy::math::IVec2, BeltDir> = belts_q
        .iter()
        .map(|(xf, bt)| (world_to_tile(xf.translation.truncate()), bt.dir))
        .collect();

    for (smelter_xf, mut state) in smelters_q.iter_mut() {
        let smelter_pos = world_to_tile(smelter_xf.translation.truncate());

        // ---- Pull (input) ----
        for &side in &[BeltDir::North, BeltDir::East, BeltDir::South, BeltDir::West] {
            let neighbor_pos = smelter_pos + side.delta();
            let Some(neighbor_dir) = belt_positions.get(&neighbor_pos) else { continue };
            if *neighbor_dir != side.opposite() { continue }

            let mut consumed = false;
            for (belt_xf, mut belt_tile) in belts_q.iter_mut() {
                if world_to_tile(belt_xf.translation.truncate()) != neighbor_pos { continue }
                let Some(item) = belt_tile.item else { break };
                let ItemKind::Ore(ore) = item else { break };

                match state.recipe {
                    None => {
                        state.recipe = Some(ore);
                        state.queue = 1;
                        state.time_left = processing::SMELT_DURATION_S;
                        belt_tile.item = None;
                        consumed = true;
                    }
                    Some(current) if current == ore && state.queue < SMELTER_QUEUE_CAP => {
                        state.queue += 1;
                        belt_tile.item = None;
                        consumed = true;
                    }
                    _ => {}
                }
                break;
            }
            if consumed { break }
        }

        // ---- Push (output) ----
        let has_output = state.output.values().any(|&n| n > 0);
        if !has_output { continue }

        for &side in &[BeltDir::North, BeltDir::East, BeltDir::South, BeltDir::West] {
            let neighbor_pos = smelter_pos + side.delta();
            let Some(neighbor_dir) = belt_positions.get(&neighbor_pos) else { continue };
            if *neighbor_dir != side { continue }

            let mut pushed = false;
            for (belt_xf, mut belt_tile) in belts_q.iter_mut() {
                if world_to_tile(belt_xf.translation.truncate()) != neighbor_pos { continue }
                if belt_tile.item.is_some() { break }

                let mut to_drain: Option<OreKind> = None;
                for (&ore, &n) in state.output.iter() {
                    if n > 0 { to_drain = Some(ore); break }
                }
                let Some(ore) = to_drain else { break };
                if let Some(n) = state.output.get_mut(&ore) {
                    *n -= 1;
                    if *n == 0 { state.output.remove(&ore); }
                }
                belt_tile.item = Some(ItemKind::Bar(ore));
                pushed = true;
                break;
            }
            if pushed { break }
        }
    }
}
