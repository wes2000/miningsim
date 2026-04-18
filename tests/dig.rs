use bevy::prelude::IVec2;
use miningsim::dig::{self, DigStatus};
use miningsim::grid::{Grid, Layer, OreType, Tile};
use miningsim::tools::Tool;

fn make_test_grid() -> Grid {
    let mut g = Grid::new(10, 10);
    // Fill interior with solid Dirt tiles (Grid default).
    // Override specific tiles:
    g.set(3, 3, Tile { solid: true, layer: Layer::Dirt,  ore: OreType::Copper, damage: 0 });
    g.set(4, 3, Tile { solid: true, layer: Layer::Stone, ore: OreType::None,   damage: 0 });
    g.set(5, 3, Tile { solid: true, layer: Layer::Deep,  ore: OreType::None,   damage: 0 });
    g.set(6, 3, Tile { solid: true, layer: Layer::Core,  ore: OreType::None,   damage: 0 });
    g.set(0, 0, Tile { solid: true, layer: Layer::Bedrock, ore: OreType::None, damage: 0 });
    g
}

// --- try_dig ---

#[test]
fn shovel_on_dirt_at_tier_takes_three_strikes() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    let r1 = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r1.status, DigStatus::Damaged);
    assert_eq!(g.get(3, 3).unwrap().damage, 1);
    let r2 = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r2.status, DigStatus::Damaged);
    assert_eq!(g.get(3, 3).unwrap().damage, 2);
    let r3 = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r3.status, DigStatus::Broken);
    assert_eq!(r3.ore, OreType::Copper);
    assert!(!g.get(3, 3).unwrap().solid);
    assert_eq!(g.get(3, 3).unwrap().damage, 0);
}

#[test]
fn pickaxe_on_dirt_one_above_tier_takes_two_strikes() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Damaged);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Broken);
}

#[test]
fn jackhammer_on_dirt_two_above_tier_takes_one_strike() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Jackhammer).status, DigStatus::Broken);
}

#[test]
fn shovel_on_stone_under_tier_returns_under_tier_no_damage() {
    let mut g = make_test_grid();
    let t = IVec2::new(4, 3);
    let r = dig::try_dig(&mut g, t, Tool::Shovel);
    assert_eq!(r.status, DigStatus::UnderTier);
    assert_eq!(g.get(4, 3).unwrap().damage, 0);
    assert!(g.get(4, 3).unwrap().solid);
}

#[test]
fn any_tool_on_bedrock_returns_under_tier_never_damages() {
    let mut g = make_test_grid();
    let t = IVec2::new(0, 0);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Dynamite).status, DigStatus::UnderTier);
    assert_eq!(g.get(0, 0).unwrap().damage, 0);
    assert!(g.get(0, 0).unwrap().solid);
}

#[test]
fn dynamite_on_core_at_tier_takes_three_strikes() {
    let mut g = make_test_grid();
    let t = IVec2::new(6, 3);
    for _ in 0..2 {
        assert_eq!(dig::try_dig(&mut g, t, Tool::Dynamite).status, DigStatus::Damaged);
    }
    assert_eq!(dig::try_dig(&mut g, t, Tool::Dynamite).status, DigStatus::Broken);
}

#[test]
fn tool_upgrade_mid_mining_breaks_immediately_when_threshold_met() {
    let mut g = make_test_grid();
    // Damage stone tile to 2 with pickaxe (3-click tier-match).
    let t = IVec2::new(4, 3);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Damaged);
    assert_eq!(dig::try_dig(&mut g, t, Tool::Pickaxe).status, DigStatus::Damaged);
    assert_eq!(g.get(4, 3).unwrap().damage, 2);
    // Now switch to Jackhammer (1-click required on stone). Tile should break.
    assert_eq!(dig::try_dig(&mut g, t, Tool::Jackhammer).status, DigStatus::Broken);
}

#[test]
fn dig_out_of_bounds_is_noop() {
    let mut g = make_test_grid();
    let r = dig::try_dig(&mut g, IVec2::new(-1, 5), Tool::Shovel);
    assert_eq!(r.status, DigStatus::OutOfBounds);
}

#[test]
fn dig_already_empty_is_noop_does_not_increment_damage() {
    let mut g = make_test_grid();
    let t = IVec2::new(3, 3);
    // Break tile completely.
    dig::try_dig(&mut g, t, Tool::Jackhammer);
    assert!(!g.get(3, 3).unwrap().solid);
    let r = dig::try_dig(&mut g, t, Tool::Jackhammer);
    assert_eq!(r.status, DigStatus::AlreadyEmpty);
    assert_eq!(g.get(3, 3).unwrap().damage, 0);
}

// --- dig_target_valid ---

#[test]
fn dig_target_valid_accepts_cardinal_within_reach() {
    let g = Grid::new(10, 10);   // all solid dirt
    let p = IVec2::new(5, 5);
    // All four cardinal directions, distance 1, with intermediate LoS clear (no intermediate tile).
    assert!(dig::dig_target_valid(p, IVec2::new(6, 5), 2, &g));
    assert!(dig::dig_target_valid(p, IVec2::new(4, 5), 2, &g));
    assert!(dig::dig_target_valid(p, IVec2::new(5, 6), 2, &g));
    assert!(dig::dig_target_valid(p, IVec2::new(5, 4), 2, &g));
}

#[test]
fn dig_target_valid_rejects_diagonal() {
    let g = Grid::new(10, 10);
    let p = IVec2::new(5, 5);
    assert!(!dig::dig_target_valid(p, IVec2::new(6, 6), 2, &g));
    assert!(!dig::dig_target_valid(p, IVec2::new(4, 4), 2, &g));
}

#[test]
fn dig_target_valid_rejects_beyond_reach() {
    let g = Grid::new(10, 10);
    let p = IVec2::new(5, 5);
    assert!(!dig::dig_target_valid(p, IVec2::new(8, 5), 2, &g));
}

#[test]
fn dig_target_valid_rejects_same_tile() {
    let g = Grid::new(10, 10);
    let p = IVec2::new(5, 5);
    assert!(!dig::dig_target_valid(p, IVec2::new(5, 5), 2, &g));
}

#[test]
fn dig_target_valid_rejects_when_intermediate_tile_is_solid() {
    let g = Grid::new(10, 10);  // all solid by default
    let p = IVec2::new(5, 5);
    // target is 2 tiles away, intermediate (6,5) is solid → rejected.
    assert!(!dig::dig_target_valid(p, IVec2::new(7, 5), 2, &g));
}

#[test]
fn dig_target_valid_accepts_reach_2_when_intermediate_is_empty() {
    let mut g = Grid::new(10, 10);
    // clear the intermediate tile
    let t = Tile { solid: false, layer: Layer::Dirt, ore: OreType::None, damage: 0 };
    g.set(6, 5, t);
    let p = IVec2::new(5, 5);
    assert!(dig::dig_target_valid(p, IVec2::new(7, 5), 2, &g));
}
