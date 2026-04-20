use bevy::math::IVec2;
use bevy::prelude::Event;
use serde::{Deserialize, Serialize};

use crate::belt::BeltDir;
use crate::items::OreKind;
use crate::tools::Tool;

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DigRequest {
    pub target: IVec2,
}

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BuyToolRequest {
    pub tool: Tool,
}

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SmeltAllRequest {
    pub ore: OreKind,
}

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectAllRequest;

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SellAllRequest;

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlaceBeltRequest {
    pub tile: IVec2,
    pub dir: BeltDir,
}

#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RemoveBeltRequest {
    pub tile: IVec2,
}
