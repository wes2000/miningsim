use miningsim::grid::{Grid, Layer, OreType, Tile};
use miningsim::dig::{self, DigStatus};

fn make_grid() -> Grid {
    let mut g = Grid::new(10, 10);
    g.set(3, 3, Tile { solid: true, layer: Layer::Dirt, ore: OreType::Copper });
    g.set(0, 0, Tile { solid: true, layer: Layer::Bedrock, ore: OreType::None });
    g
}

#[test]
fn dig_solid_tile_returns_ok_with_ore() {
    let mut g = make_grid();
    let r = dig::try_dig(&mut g, 3, 3);
    assert_eq!(r.status, DigStatus::Ok);
    assert_eq!(r.ore, OreType::Copper);
}

#[test]
fn dig_clears_tile() {
    let mut g = make_grid();
    dig::try_dig(&mut g, 3, 3);
    assert!(!g.get(3, 3).unwrap().solid);
}

#[test]
fn dig_out_of_bounds_returns_oob() {
    let mut g = make_grid();
    let r = dig::try_dig(&mut g, -1, 5);
    assert_eq!(r.status, DigStatus::OutOfBounds);
}

#[test]
fn dig_already_empty_returns_already_empty() {
    let mut g = make_grid();
    dig::try_dig(&mut g, 3, 3);
    let r = dig::try_dig(&mut g, 3, 3);
    assert_eq!(r.status, DigStatus::AlreadyEmpty);
}

#[test]
fn dig_bedrock_returns_bedrock_and_keeps_solid() {
    let mut g = make_grid();
    let r = dig::try_dig(&mut g, 0, 0);
    assert_eq!(r.status, DigStatus::Bedrock);
    assert!(g.get(0, 0).unwrap().solid);
}
