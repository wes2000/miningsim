use std::collections::HashSet;
use bevy::prelude::Resource;

use crate::grid::Layer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    Shovel,
    Pickaxe,
    Jackhammer,
    Dynamite,
}

pub fn tool_tier(t: Tool) -> u8 {
    match t {
        Tool::Shovel => 1,
        Tool::Pickaxe => 2,
        Tool::Jackhammer => 3,
        Tool::Dynamite => 4,
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

#[derive(Debug, Resource)]
pub struct OwnedTools(pub HashSet<Tool>);

impl Default for OwnedTools {
    fn default() -> Self {
        let mut s = HashSet::new();
        s.insert(Tool::Shovel);
        Self(s)
    }
}

pub fn best_applicable_tool(owned: &OwnedTools, layer: Layer) -> Option<Tool> {
    [Tool::Dynamite, Tool::Jackhammer, Tool::Pickaxe, Tool::Shovel]
        .into_iter()
        .find(|t| owned.0.contains(t) && clicks_required(*t, layer).is_some())
}
