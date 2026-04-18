use bevy::prelude::IVec2;

use crate::grid::{Grid, OreType, Tile};
use crate::tools::{self, Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigStatus {
    Broken,
    Damaged,
    OutOfBounds,
    AlreadyEmpty,
    Blocked,
    UnderTier,
}

#[derive(Debug, Clone, Copy)]
pub struct DigResult {
    pub status: DigStatus,
    pub ore: OreType,
}

pub fn try_dig(grid: &mut Grid, target: IVec2, tool: Tool) -> DigResult {
    let x = target.x;
    let y = target.y;

    let tile = match grid.get(x, y) {
        None => return DigResult { status: DigStatus::OutOfBounds, ore: OreType::None },
        Some(t) => *t,
    };
    if !tile.solid {
        return DigResult { status: DigStatus::AlreadyEmpty, ore: OreType::None };
    }
    let Some(required) = tools::clicks_required(tool, tile.layer) else {
        return DigResult { status: DigStatus::UnderTier, ore: OreType::None };
    };

    let new_damage = tile.damage + 1;
    if new_damage >= required {
        // Break tile.
        let ore = tile.ore;
        grid.set(x, y, Tile { solid: false, layer: tile.layer, ore: OreType::None, damage: 0 });
        DigResult { status: DigStatus::Broken, ore }
    } else {
        grid.set(x, y, Tile { damage: new_damage, ..tile });
        DigResult { status: DigStatus::Damaged, ore: OreType::None }
    }
}

/// Cardinal-only + line-of-sight dig reach check, extracted for unit testing.
///
/// Returns true iff:
/// - `target` differs from `player_tile`,
/// - exactly one axis of delta is zero (cardinal, not diagonal),
/// - |delta| ≤ reach on the nonzero axis,
/// - every tile STRICTLY BETWEEN `player_tile` and `target` is non-solid.
///
/// Does NOT check whether `target` itself is solid (callers may want to mine
/// either solid or empty tiles depending on context). Does NOT check bounds;
/// out-of-bounds intermediates are treated as "non-solid" so that reach checks
/// near map edges don't spuriously reject.
pub fn dig_target_valid(player_tile: IVec2, target: IVec2, reach: i32, grid: &Grid) -> bool {
    let delta = target - player_tile;
    if delta == IVec2::ZERO { return false; }
    let is_cardinal = (delta.x == 0) ^ (delta.y == 0);
    if !is_cardinal { return false; }
    let dist = delta.x.abs().max(delta.y.abs());
    if dist > reach { return false; }

    let step = IVec2::new(delta.x.signum(), delta.y.signum());
    let mut probe = player_tile + step;
    while probe != target {
        // If a tile in the path is solid, LoS is blocked.
        if let Some(t) = grid.get(probe.x, probe.y) {
            if t.solid { return false; }
        }
        probe += step;
    }
    true
}
