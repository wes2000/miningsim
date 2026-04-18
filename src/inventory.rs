use std::collections::HashMap;
use crate::grid::OreType;

#[derive(Debug, Default, bevy::prelude::Resource)]
pub struct Inventory {
    counts: HashMap<OreType, u32>,
}

impl Inventory {
    pub fn add(&mut self, ore: OreType, n: u32) {
        if ore == OreType::None { return; }
        *self.counts.entry(ore).or_insert(0) += n;
    }

    pub fn remove(&mut self, ore: OreType, n: u32) {
        let c = self.counts.entry(ore).or_insert(0);
        *c = c.saturating_sub(n);
    }

    pub fn get(&self, ore: OreType) -> u32 {
        *self.counts.get(&ore).unwrap_or(&0)
    }
}
