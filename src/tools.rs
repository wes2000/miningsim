use std::collections::BTreeSet;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use crate::grid::Layer;

// Variant order is load-bearing: derived `Ord` drives BTreeSet iteration in
// `OwnedTools.0`, replicated by bevy_replicon. Reordering changes diff output
// and on-disk save shape — bump SAVE_VERSION if changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tool {
    Shovel,
    Pickaxe,
    Jackhammer,
    Dynamite,
    /// One-time unlock for the belt-network build mode (M5a). Not a dig tool;
    /// `tool_tier` returns 0 so it never beats any layer's tier in
    /// `clicks_required`. Excluded from `best_applicable_tool`'s candidate array.
    BeltUnlock,
}

pub fn tool_tier(t: Tool) -> u8 {
    match t {
        Tool::Shovel => 1,
        Tool::Pickaxe => 2,
        Tool::Jackhammer => 3,
        Tool::Dynamite => 4,
        Tool::BeltUnlock => 0,
    }
}

pub fn layer_tier(l: Layer) -> Option<u8> {
    match l {
        Layer::Dirt => Some(1),
        Layer::Stone => Some(2),
        Layer::Deep => Some(3),
        Layer::Core => Some(4),
        Layer::Bedrock => None,
    }
}

pub fn clicks_required(tool: Tool, layer: Layer) -> Option<u8> {
    let lt = layer_tier(layer)?;
    let tt = tool_tier(tool);
    if tt < lt { return None; }
    let gap = (tt - lt).min(2);
    Some(3 - gap)
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct OwnedTools(pub BTreeSet<Tool>);

impl Default for OwnedTools {
    fn default() -> Self {
        let mut s = BTreeSet::new();
        s.insert(Tool::Shovel);
        Self(s)
    }
}

pub fn best_applicable_tool(owned: &OwnedTools, layer: Layer) -> Option<Tool> {
    [Tool::Dynamite, Tool::Jackhammer, Tool::Pickaxe, Tool::Shovel]
        .into_iter()
        .find(|t| owned.0.contains(t) && clicks_required(*t, layer).is_some())
}
