use bevy::math::{IVec2, Vec2};
use miningsim::coords::{self, TILE_SIZE_PX};

#[test]
fn tile_size_px_is_16() {
    assert_eq!(TILE_SIZE_PX, 16.0);
}

#[test]
fn tile_center_world_round_trip() {
    for &(x, y) in &[(0i32, 0), (1, 1), (5, 10), (-3, 4), (0, 200)] {
        let c = coords::tile_center_world(IVec2::new(x, y));
        assert_eq!(coords::world_to_tile(c), IVec2::new(x, y));
    }
}

#[test]
fn world_y_inversion() {
    let c = coords::tile_center_world(IVec2::new(0, 0));
    assert_eq!(c.y, -8.0);
    let c2 = coords::tile_center_world(IVec2::new(0, 1));
    assert!(c2.y < c.y, "deeper tile should have more negative world y");
}

#[test]
fn tile_min_world_corners() {
    let m = coords::tile_min_world(IVec2::new(3, 5));
    assert_eq!(m, Vec2::new(48.0, -96.0));
}

#[test]
fn world_to_tile_at_negative_world_x() {
    assert_eq!(coords::world_to_tile(Vec2::new(-1.0, -8.0)).x, -1);
    assert_eq!(coords::world_to_tile(Vec2::new(0.0, -8.0)).x, 0);
}
