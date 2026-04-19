use std::collections::HashMap;
use bevy::prelude::Component;
use crate::items::OreKind;

pub const SMELT_DURATION_S: f32 = 2.0;

#[derive(Component, Debug, Default)]
pub struct SmelterState {
    pub recipe: Option<OreKind>,
    pub time_left: f32,
    pub queue: u32,
    pub output: HashMap<OreKind, u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SmeltTickEvent {
    None,
    BarFinished(OreKind),
}

pub fn is_busy(state: &SmelterState) -> bool {
    state.recipe.is_some()
}

pub fn start_smelting(state: &mut SmelterState, ore: OreKind, count: u32) {
    if count == 0 || state.recipe.is_some() {
        return;
    }
    state.recipe = Some(ore);
    state.queue = count;
    state.time_left = SMELT_DURATION_S;
}

pub fn tick_smelter(state: &mut SmelterState, dt: f32) -> SmeltTickEvent {
    let Some(ore) = state.recipe else {
        return SmeltTickEvent::None;
    };
    state.time_left -= dt;
    if state.time_left > 0.0 {
        return SmeltTickEvent::None;
    }
    // Complete EXACTLY one item even if dt overshoots — predictable over realistic.
    *state.output.entry(ore).or_insert(0) += 1;
    state.queue -= 1;
    if state.queue > 0 {
        state.time_left = SMELT_DURATION_S;
    } else {
        state.recipe = None;
        state.time_left = 0.0;
    }
    SmeltTickEvent::BarFinished(ore)
}

pub fn collect_output(state: &mut SmelterState) -> HashMap<OreKind, u32> {
    std::mem::take(&mut state.output)
}
