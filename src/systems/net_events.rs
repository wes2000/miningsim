use bevy::math::{IVec2, Vec2};
use bevy::prelude::Event;
use serde::{Deserialize, Serialize};

use crate::belt::BeltDir;
use crate::grid::{Grid, Tile};
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

// ---------- Server events (server → client) added in M5b ----------

/// Server → one specific client. Fired once per client connection, carrying
/// the full Grid. Replicon's ordered channel handles reliable delivery +
/// transparent fragmentation, so the ~80 KB payload reaches the client
/// intact. After this, the client tracks Grid via `TileChanged` deltas.
#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct GridSnapshot {
    pub grid: Grid,
}

/// Server → all clients. Broadcast after every successful tile mutation
/// (dig: damage or break). Ordered so reordering of two updates to the
/// same tile doesn't cause visual flicker (e.g., damage=1 arriving after
/// damage=2).
#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TileChanged {
    pub pos: IVec2,
    pub tile: Tile,
}

// ---------- Client events (client → server) added in M5b ----------

/// Client → server. Fired at `POSITION_SYNC_HZ` (see `net_player.rs`) to
/// keep the server's view of this client's player position current. Used
/// for dig-reach validation and replication to OTHER clients via
/// `.replicate::<Transform>()`. Uses `Channel::Unreliable` because
/// position updates are supersedable — dropping a packet is cheaper
/// than retransmitting a stale position (the next 100 ms tick will
/// carry current truth anyway).
#[derive(Event, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClientPositionUpdate {
    pub pos: Vec2,
    pub facing: IVec2,
}
