use miningsim::grid::{Grid, Layer, Tile};
use miningsim::items::OreKind;

#[test]
fn new_grid_has_requested_dimensions() {
    let g = Grid::new(10, 20);
    assert_eq!(g.width(), 10);
    assert_eq!(g.height(), 20);
}

#[test]
fn new_grid_default_tiles_are_solid_dirt_no_ore() {
    let g = Grid::new(3, 3);
    let t = g.get(1, 1).expect("in bounds");
    assert!(t.solid);
    assert_eq!(t.layer, Layer::Dirt);
    assert_eq!(t.ore, None);
}

#[test]
fn set_and_get_round_trip() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: false, layer: Layer::Stone, ore: Some(OreKind::Silver), damage: 0 });
    let t = g.get(1, 1).unwrap();
    assert!(!t.solid);
    assert_eq!(t.layer, Layer::Stone);
    assert_eq!(t.ore, Some(OreKind::Silver));
}

#[test]
fn in_bounds_check() {
    let g = Grid::new(5, 5);
    assert!(g.in_bounds(0, 0));
    assert!(g.in_bounds(4, 4));
    assert!(!g.in_bounds(-1, 0));
    assert!(!g.in_bounds(0, -1));
    assert!(!g.in_bounds(5, 0));
    assert!(!g.in_bounds(0, 5));
}

#[test]
fn get_out_of_bounds_returns_none() {
    let g = Grid::new(3, 3);
    assert!(g.get(-1, 0).is_none());
    assert!(g.get(3, 0).is_none());
}

#[test]
#[should_panic]
fn set_out_of_bounds_panics() {
    let mut g = Grid::new(3, 3);
    g.set(5, 5, Tile { solid: true, layer: Layer::Dirt, ore: None, damage: 0 });
}

#[test]
fn new_tile_has_zero_damage() {
    let g = Grid::new(3, 3);
    assert_eq!(g.get(1, 1).unwrap().damage, 0);
}

#[test]
fn damage_round_trips_through_set() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: true, layer: Layer::Stone, ore: None, damage: 2 });
    assert_eq!(g.get(1, 1).unwrap().damage, 2);
}

#[test]
fn layer_core_variant_exists() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: true, layer: Layer::Core, ore: None, damage: 0 });
    assert_eq!(g.get(1, 1).unwrap().layer, Layer::Core);
}

#[test]
fn from_raw_round_trips_via_serde() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: true, layer: Layer::Stone, ore: Some(OreKind::Silver), damage: 2 });
    let s = ron::ser::to_string(&g).expect("serialize");
    let g2: Grid = ron::de::from_str(&s).expect("deserialize");
    assert_eq!(g2.width(), g.width());
    assert_eq!(g2.height(), g.height());
    assert_eq!(g2.get(1, 1), g.get(1, 1));
    assert_eq!(g2.get(0, 0), g.get(0, 0));
}

#[test]
#[should_panic]
fn from_raw_panics_on_length_mismatch() {
    let _ = Grid::from_raw(3, 3, vec![Tile::default(); 4]);
}
