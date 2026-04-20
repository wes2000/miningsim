use bevy::math::IVec2;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use crate::items::ItemKind;

/// Cardinal direction a belt tile points in. Variant order is conventionally
/// N,E,S,W (clockwise from North) — keep this order, callers may rely on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BeltDir {
    North,
    East,
    South,
    West,
}

impl BeltDir {
    /// Tile-space delta. Note: positive Y is "south" (deeper into terrain),
    /// matching the Grid convention used elsewhere in the codebase.
    pub fn delta(self) -> IVec2 {
        match self {
            BeltDir::North => IVec2::new(0, -1),
            BeltDir::East  => IVec2::new(1, 0),
            BeltDir::South => IVec2::new(0, 1),
            BeltDir::West  => IVec2::new(-1, 0),
        }
    }

    pub fn opposite(self) -> BeltDir {
        match self {
            BeltDir::North => BeltDir::South,
            BeltDir::South => BeltDir::North,
            BeltDir::East  => BeltDir::West,
            BeltDir::West  => BeltDir::East,
        }
    }

    /// Cycle clockwise: N → E → S → W → N.
    pub fn rotate_cw(self) -> BeltDir {
        match self {
            BeltDir::North => BeltDir::East,
            BeltDir::East  => BeltDir::South,
            BeltDir::South => BeltDir::West,
            BeltDir::West  => BeltDir::North,
        }
    }
}

/// One belt tile in the world. Component-on-entity. Replicated by replicon.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeltTile {
    pub item: Option<ItemKind>,
    pub dir: BeltDir,
}

impl BeltTile {
    pub fn new(dir: BeltDir) -> Self {
        Self { item: None, dir }
    }
}

/// Visual rendering kind for a belt tile. Locally derived, not replicated.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeltVisual {
    Straight,
    /// Corner from south-feed to east-out (or equivalent rotations).
    CornerSE,
    CornerNE,
    CornerSW,
    CornerNW,
}

pub fn next_tile(pos: IVec2, dir: BeltDir) -> IVec2 {
    pos + dir.delta()
}

/// Validate a candidate belt placement. Pure — caller projects World state
/// into (is the tile a walkable floor within bounds, set of occupied tiles).
pub fn can_place_belt(
    tile: IVec2,
    in_bounds_and_floor: bool,
    occupied_tiles: &std::collections::HashSet<IVec2>,
) -> bool {
    in_bounds_and_floor && !occupied_tiles.contains(&tile)
}

pub fn belt_visual_kind(self_dir: BeltDir, feeder_dir: Option<BeltDir>) -> BeltVisual {
    let Some(fd) = feeder_dir else { return BeltVisual::Straight };

    // Feeder must be perpendicular to self for a corner.
    let perpendicular = match (self_dir, fd) {
        (BeltDir::East,  BeltDir::North) | (BeltDir::East,  BeltDir::South) => true,
        (BeltDir::West,  BeltDir::North) | (BeltDir::West,  BeltDir::South) => true,
        (BeltDir::North, BeltDir::East)  | (BeltDir::North, BeltDir::West)  => true,
        (BeltDir::South, BeltDir::East)  | (BeltDir::South, BeltDir::West)  => true,
        _ => false,
    };
    if !perpendicular {
        return BeltVisual::Straight;
    }

    // Map (self_dir, feeder_dir) to one of the four corner kinds.
    // Feeder dir = North means feeder is south of us pointing up (item came from south)
    // Feeder dir = South means feeder is north of us pointing down (item came from north)
    // Feeder dir = East  means feeder is west of us pointing right (item came from west)
    // Feeder dir = West  means feeder is east of us pointing left (item came from east)
    match (self_dir, fd) {
        (BeltDir::East,  BeltDir::North) => BeltVisual::CornerSE,  // came from S, going E
        (BeltDir::East,  BeltDir::South) => BeltVisual::CornerNE,  // came from N, going E
        (BeltDir::West,  BeltDir::North) => BeltVisual::CornerSW,
        (BeltDir::West,  BeltDir::South) => BeltVisual::CornerNW,
        (BeltDir::North, BeltDir::East)  => BeltVisual::CornerNW,  // came from W, going N
        (BeltDir::North, BeltDir::West)  => BeltVisual::CornerNE,  // came from E, going N
        (BeltDir::South, BeltDir::East)  => BeltVisual::CornerSW,
        (BeltDir::South, BeltDir::West)  => BeltVisual::CornerSE,
        _ => BeltVisual::Straight,  // unreachable given the perpendicular check
    }
}

/// Pure back-pressure decision: given the current belt graph and which tiles
/// have items, return the list of (from, to) moves to apply this tick.
///
/// **Algorithm (lockstep, two-pass):**
/// - Pass 1: every belt that holds an item declares its intended destination.
/// - Pass 2: a move succeeds iff its destination is "vacated this tick" —
///   either the destination didn't have an item to begin with, OR the
///   destination's item is also moving away this tick (cascade).
///
/// This handles three classes of motion in a single call:
/// - **Chain ending off-graph:** the head moves to its off-graph destination
///   (vacant), then the next item's destination is vacated, etc. All cascade.
/// - **Saturated cycle:** every item's destination is held by another item
///   that is also moving. All N items rotate one slot in lockstep.
/// - **Blocked at machine/wall:** if the head's destination has an item that
///   is NOT moving (e.g., another belt facing into the head, or a stationary
///   item whose destination is occupied without cascade), the chain stalls.
///
/// `belts`: tile coord → belt direction (every belt entity present this tick).
/// `items_present`: tile coords that currently hold an item.
///
/// Note: items advancing to a tile that is NOT in `belts` (i.e. off the belt
/// graph — spillage destinations) ARE included in the returned moves. The
/// caller is responsible for spillage handling on those.
///
/// Uses `HashMap`/`HashSet` rather than BTree variants because Bevy 0.15
/// `IVec2` doesn't implement `Ord`. The lockstep algorithm is deterministic
/// regardless of input iteration order (the cascade resolution is a pure
/// function of the input snapshot).
pub fn compute_belt_advances(
    belts: &std::collections::HashMap<bevy::math::IVec2, BeltDir>,
    items_present: &std::collections::HashSet<bevy::math::IVec2>,
) -> Vec<(bevy::math::IVec2, bevy::math::IVec2)> {
    use std::collections::HashMap;

    // Pass 1: each item-bearing belt declares its desired destination.
    let mut intended: HashMap<bevy::math::IVec2, bevy::math::IVec2> = HashMap::new();
    for (&pos, &dir) in belts.iter() {
        if items_present.contains(&pos) {
            intended.insert(pos, next_tile(pos, dir));
        }
    }

    // Pass 2: resolve which intended moves succeed. A move succeeds iff its
    // destination is vacated this tick — either the destination starts empty,
    // or the destination's own mover also succeeds (cascade), or the move is
    // part of a closed cycle where every member intends to move (lockstep
    // rotation). We compute this by alternating two sub-passes inside an outer
    // fixed-point loop:
    //   (a) propagation: a move can succeed if its destination's mover can.
    //   (b) cycle detection: walk intended-graph chains; if a chain loops
    //       back to its start, the whole closed cycle can move.
    // The outer loop catches "rho-shape" graphs (a tail feeding a cycle):
    // pass (b) marks the cycle, then pass (a) propagates back along the tail.
    let mut can_move: std::collections::HashSet<bevy::math::IVec2> = std::collections::HashSet::new();

    // Seed: any intended move whose destination starts empty (no item there).
    for (&from, &to) in intended.iter() {
        if !items_present.contains(&to) {
            can_move.insert(from);
        }
    }

    loop {
        // (a) Propagation sub-pass: a move can succeed if its destination's mover can.
        let mut grew = false;
        loop {
            let mut grew_inner = false;
            for (&from, &to) in intended.iter() {
                if can_move.contains(&from) { continue }
                if can_move.contains(&to) {
                    can_move.insert(from);
                    grew_inner = true;
                    grew = true;
                }
            }
            if !grew_inner { break }
        }

        // (b) Cycle-detection sub-pass: walk intended-graph chains from each
        //     unresolved start; if we loop back to start, the whole cycle moves.
        let unresolved: Vec<bevy::math::IVec2> = intended
            .keys()
            .filter(|p| !can_move.contains(p))
            .copied()
            .collect();
        for start in unresolved {
            if can_move.contains(&start) { continue }
            let mut path: Vec<bevy::math::IVec2> = Vec::new();
            let mut visited: std::collections::HashSet<bevy::math::IVec2> = std::collections::HashSet::new();
            let mut cur = start;
            loop {
                if visited.contains(&cur) {
                    if cur == start {
                        for p in &path {
                            can_move.insert(*p);
                            grew = true;
                        }
                    }
                    break;
                }
                visited.insert(cur);
                path.push(cur);
                let Some(&next) = intended.get(&cur) else { break };
                if !intended.contains_key(&next) { break }
                cur = next;
            }
        }

        if !grew { break }
    }

    // Y-merge arbitration: if multiple intended moves point at the same
    // destination, pick exactly one (the source with the lowest (x,y) sort
    // order — deterministic and reproducible). Drop the others. Without this,
    // the apply step would land two items on one tile.
    let mut moves_by_source: Vec<(bevy::math::IVec2, bevy::math::IVec2)> = intended
        .iter()
        .filter(|(from, _)| can_move.contains(from))
        .map(|(&from, &to)| (from, to))
        .collect();
    // Sort ascending so iter().next() gives the smallest source per destination.
    moves_by_source.sort_by_key(|(from, _)| (from.x, from.y));
    let mut destinations_taken: std::collections::HashSet<bevy::math::IVec2> = std::collections::HashSet::new();
    let moves: Vec<(bevy::math::IVec2, bevy::math::IVec2)> = moves_by_source
        .into_iter()
        .filter(|(_, to)| destinations_taken.insert(*to))
        .collect();
    moves
}
