use std::collections::HashMap;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use crate::items::ItemKind;

#[derive(Debug, Default, Clone, Resource, Serialize, Deserialize)]
pub struct Inventory {
    counts: HashMap<ItemKind, u32>,
}

impl Inventory {
    pub fn add(&mut self, item: ItemKind, n: u32) {
        if n == 0 { return; }
        *self.counts.entry(item).or_insert(0) += n;
    }

    pub fn remove(&mut self, item: ItemKind, n: u32) {
        if let Some(c) = self.counts.get_mut(&item) {
            *c = c.saturating_sub(n);
        }
    }

    pub fn get(&self, item: ItemKind) -> u32 {
        *self.counts.get(&item).unwrap_or(&0)
    }
}
