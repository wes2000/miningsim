use crate::grid::{Grid, Layer, OreType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigStatus {
    Ok,
    OutOfBounds,
    AlreadyEmpty,
    Bedrock,
}

#[derive(Debug, Clone, Copy)]
pub struct DigResult {
    pub status: DigStatus,
    pub ore: OreType,
}

pub fn try_dig(grid: &mut Grid, x: i32, y: i32) -> DigResult {
    let tile_opt = grid.get(x, y).copied();
    let Some(t) = tile_opt else {
        return DigResult { status: DigStatus::OutOfBounds, ore: OreType::None };
    };
    if t.layer == Layer::Bedrock {
        return DigResult { status: DigStatus::Bedrock, ore: OreType::None };
    }
    if !t.solid {
        return DigResult { status: DigStatus::AlreadyEmpty, ore: OreType::None };
    }
    let ore = t.ore;
    grid.set(x, y, crate::grid::Tile { solid: false, layer: t.layer, ore: OreType::None, damage: 0 });
    DigResult { status: DigStatus::Ok, ore }
}
