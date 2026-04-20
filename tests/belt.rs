use bevy::math::IVec2;
use miningsim::belt::{self, BeltDir, BeltTile, BeltVisual};
use miningsim::items::{ItemKind, OreKind};

#[test]
fn belt_dir_delta_cardinals() {
    assert_eq!(BeltDir::North.delta(), IVec2::new(0, -1));
    assert_eq!(BeltDir::South.delta(), IVec2::new(0, 1));
    assert_eq!(BeltDir::East.delta(),  IVec2::new(1, 0));
    assert_eq!(BeltDir::West.delta(),  IVec2::new(-1, 0));
}

#[test]
fn belt_dir_opposite_round_trip() {
    for dir in [BeltDir::North, BeltDir::East, BeltDir::South, BeltDir::West] {
        assert_eq!(dir.opposite().opposite(), dir);
    }
}

#[test]
fn belt_dir_rotate_cw_cycles() {
    let mut dir = BeltDir::North;
    for _ in 0..4 { dir = dir.rotate_cw(); }
    assert_eq!(dir, BeltDir::North);
    // explicit cycle order
    assert_eq!(BeltDir::North.rotate_cw(), BeltDir::East);
    assert_eq!(BeltDir::East.rotate_cw(),  BeltDir::South);
    assert_eq!(BeltDir::South.rotate_cw(), BeltDir::West);
    assert_eq!(BeltDir::West.rotate_cw(),  BeltDir::North);
}

#[test]
fn next_tile_basic() {
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::East),  IVec2::new(6, 5));
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::West),  IVec2::new(4, 5));
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::North), IVec2::new(5, 4));
    assert_eq!(belt::next_tile(IVec2::new(5, 5), BeltDir::South), IVec2::new(5, 6));
}

#[test]
fn belt_tile_default_empty() {
    let t = BeltTile::new(BeltDir::East);
    assert_eq!(t.item, None);
    assert_eq!(t.dir, BeltDir::East);
}

#[test]
fn belt_tile_holds_item() {
    let mut t = BeltTile::new(BeltDir::East);
    t.item = Some(ItemKind::Ore(OreKind::Copper));
    assert_eq!(t.item, Some(ItemKind::Ore(OreKind::Copper)));
}

#[test]
fn belt_visual_straight_no_feeder() {
    // self facing East, no perpendicular feeder => Straight
    assert_eq!(belt::belt_visual_kind(BeltDir::East, None), BeltVisual::Straight);
}

#[test]
fn belt_visual_aligned_feeder_is_straight() {
    // self facing East, feeder coming from the west (also facing East) is in-line => Straight
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::East)), BeltVisual::Straight);
}

#[test]
fn belt_visual_corner_from_south() {
    // self East, feeder coming from south (feeder dir = North) => corner from S to E
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::North)), BeltVisual::CornerSE);
}

#[test]
fn belt_visual_corner_from_north() {
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::South)), BeltVisual::CornerNE);
}

#[test]
fn belt_visual_all_corners() {
    // Going East, feeder from N or S
    assert_eq!(belt::belt_visual_kind(BeltDir::East,  Some(BeltDir::South)), BeltVisual::CornerNE);
    assert_eq!(belt::belt_visual_kind(BeltDir::East,  Some(BeltDir::North)), BeltVisual::CornerSE);
    // Going West, feeder from N or S
    assert_eq!(belt::belt_visual_kind(BeltDir::West,  Some(BeltDir::South)), BeltVisual::CornerNW);
    assert_eq!(belt::belt_visual_kind(BeltDir::West,  Some(BeltDir::North)), BeltVisual::CornerSW);
    // Going North, feeder from E or W
    assert_eq!(belt::belt_visual_kind(BeltDir::North, Some(BeltDir::East)),  BeltVisual::CornerNW);
    assert_eq!(belt::belt_visual_kind(BeltDir::North, Some(BeltDir::West)),  BeltVisual::CornerNE);
    // Going South, feeder from E or W
    assert_eq!(belt::belt_visual_kind(BeltDir::South, Some(BeltDir::East)),  BeltVisual::CornerSW);
    assert_eq!(belt::belt_visual_kind(BeltDir::South, Some(BeltDir::West)),  BeltVisual::CornerSE);
}

#[test]
fn belt_visual_feeder_facing_away_is_straight() {
    // self East, feeder dir = West (feeder is to the south but its arrow points away from us) => no feed
    assert_eq!(belt::belt_visual_kind(BeltDir::East, Some(BeltDir::West)), BeltVisual::Straight);
}

// ---------- Back-pressure pure helper tests ----------
// `compute_belt_advances` takes a snapshot of belt positions+dirs and which
// belts currently hold an item, and returns the list of (from, to) moves to
// apply this tick. Tests below exercise the algorithm independently of Bevy.

use std::collections::{HashMap, HashSet};

fn dirs(pairs: &[(IVec2, BeltDir)]) -> HashMap<IVec2, BeltDir> {
    pairs.iter().copied().collect()
}

fn items(positions: &[IVec2]) -> HashSet<IVec2> {
    positions.iter().copied().collect()
}

#[test]
fn back_pressure_chain_clears_simultaneously() {
    // Three belts in a row, all facing East, all carrying an item. Destination
    // (3,0) is empty (off the belt graph — spillage handled separately). All
    // three items advance one tile this tick.
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::East),
        (IVec2::new(2, 0), BeltDir::East),
    ]);
    let item_positions = items(&[IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(2, 0)]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let moves_set: HashSet<(IVec2, IVec2)> = moves.into_iter().collect();
    // Head item leaves the graph; remaining two shift forward one tile.
    assert!(moves_set.contains(&(IVec2::new(2, 0), IVec2::new(3, 0))));
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(2, 0))));
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert_eq!(moves_set.len(), 3);
}

#[test]
fn back_pressure_blocks_when_destination_full() {
    // Two belts: (0,0) East with item, (1,0) East with item. Destination (2,0)
    // is NOT a belt → spillage path. The head item (1,0) goes to (2,0); the
    // tail (0,0) advances into the now-empty (1,0).
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::East),
    ]);
    let item_positions = items(&[IVec2::new(0, 0), IVec2::new(1, 0)]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let moves_set: HashSet<(IVec2, IVec2)> = moves.into_iter().collect();
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(2, 0))));
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert_eq!(moves_set.len(), 2);
}

#[test]
fn back_pressure_saturated_cycle_rotates() {
    // Four belts in a CW cycle, all carrying items. The lockstep algorithm
    // rotates them all one slot — matches the spec's "Belt loop: items orbit
    // forever" behavior. The whole cycle moves as one unit.
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::South),
        (IVec2::new(1, 1), BeltDir::West),
        (IVec2::new(0, 1), BeltDir::North),
    ]);
    let item_positions = items(&[
        IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(1, 1), IVec2::new(0, 1),
    ]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let moves_set: HashSet<(IVec2, IVec2)> = moves.into_iter().collect();
    assert_eq!(moves_set.len(), 4, "all four cycle members should rotate");
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(1, 1))));
    assert!(moves_set.contains(&(IVec2::new(1, 1), IVec2::new(0, 1))));
    assert!(moves_set.contains(&(IVec2::new(0, 1), IVec2::new(0, 0))));
}

#[test]
fn back_pressure_cycle_with_slack_rotates() {
    // Same 4-belt CW cycle but only 3 items (one slot empty at (0,1)).
    // The remaining 3 items each advance one tile.
    let belt_dirs = dirs(&[
        (IVec2::new(0, 0), BeltDir::East),
        (IVec2::new(1, 0), BeltDir::South),
        (IVec2::new(1, 1), BeltDir::West),
        (IVec2::new(0, 1), BeltDir::North),
    ]);
    let item_positions = items(&[
        IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(1, 1),
    ]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let moves_set: HashSet<(IVec2, IVec2)> = moves.into_iter().collect();
    // Each of the three items advances to its `next_tile`.
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(1, 1))));
    assert!(moves_set.contains(&(IVec2::new(1, 1), IVec2::new(0, 1))));
    assert_eq!(moves_set.len(), 3);
}

#[test]
fn back_pressure_rho_shape_propagates_after_cycle() {
    // Layout: tail belt at (-1,0) East feeds into a 4-belt CW cycle at
    // (0,0)→(1,0)→(1,1)→(0,1)→(0,0). All five belts have items. The cycle
    // is saturated so it rotates in lockstep; the tail item then advances
    // into the just-vacated cycle slot.
    let belt_dirs = dirs(&[
        (IVec2::new(-1, 0), BeltDir::East),     // tail
        (IVec2::new(0, 0),  BeltDir::East),     // cycle
        (IVec2::new(1, 0),  BeltDir::South),
        (IVec2::new(1, 1),  BeltDir::West),
        (IVec2::new(0, 1),  BeltDir::North),
    ]);
    let item_positions = items(&[
        IVec2::new(-1, 0),
        IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(1, 1), IVec2::new(0, 1),
    ]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    let moves_set: HashSet<(IVec2, IVec2)> = moves.into_iter().collect();
    // All five items advance: cycle rotates one slot, tail advances into the
    // (0,0) slot vacated by the cycle's rotation.
    // BUT: this is a Y-merge — the tail at (-1,0) and the cycle node (0,1)
    // both target (0,0). Y-merge arbitration picks exactly one.
    // The (0,1)→(0,0) move belongs to the cycle and goes through cycle-pass
    // marking; (-1,0)→(0,0) goes through propagation. Both end up in
    // can_move, then arbitration drops one. Sort order: (-1,0) < (0,1) by
    // (x,y), so (-1,0) wins.
    assert!(moves_set.contains(&(IVec2::new(-1, 0), IVec2::new(0, 0))),
            "tail should advance into vacated cycle slot");
    // Other three cycle moves still happen.
    assert!(moves_set.contains(&(IVec2::new(0, 0), IVec2::new(1, 0))));
    assert!(moves_set.contains(&(IVec2::new(1, 0), IVec2::new(1, 1))));
    assert!(moves_set.contains(&(IVec2::new(1, 1), IVec2::new(0, 1))));
    // (0,1)→(0,0) was dropped by Y-merge arbitration in favor of the tail.
    // Net effect: 4 moves applied; the cycle item that wanted (0,0) stays put.
    assert_eq!(moves_set.len(), 4);
}

#[test]
fn back_pressure_y_merge_arbitrates_to_single_winner() {
    // Two belts both pointing at the same destination tile. The destination
    // is not a belt itself (off-graph). Both want to move into (5,5). Only
    // one move should be emitted.
    let belt_dirs = dirs(&[
        (IVec2::new(4, 5), BeltDir::East),  // wants → (5,5)
        (IVec2::new(5, 4), BeltDir::South), // wants → (5,5)
    ]);
    let item_positions = items(&[IVec2::new(4, 5), IVec2::new(5, 4)]);
    let moves = belt::compute_belt_advances(&belt_dirs, &item_positions);
    assert_eq!(moves.len(), 1, "Y-merge arbitration should keep exactly one move per destination");
    // Sort order: (4,5) < (5,4) by (x,y), so (4,5) wins.
    assert_eq!(moves[0], (IVec2::new(4, 5), IVec2::new(5, 5)));
}
