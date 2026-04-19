use miningsim::grid::Layer;
use miningsim::items::OreKind;
use miningsim::terrain_gen;

#[test]
fn generated_grid_has_requested_dimensions() {
    let g = terrain_gen::generate(80, 200, 12345);
    assert_eq!(g.width(), 80);
    assert_eq!(g.height(), 200);
}

#[test]
fn outermost_ring_is_bedrock() {
    let g = terrain_gen::generate(40, 60, 1);
    for x in 0..g.width() as i32 {
        assert_eq!(g.get(x, 0).unwrap().layer, Layer::Bedrock);
        assert_eq!(g.get(x, g.height() as i32 - 1).unwrap().layer, Layer::Bedrock);
    }
    for y in 0..g.height() as i32 {
        assert_eq!(g.get(0, y).unwrap().layer, Layer::Bedrock);
        assert_eq!(g.get(g.width() as i32 - 1, y).unwrap().layer, Layer::Bedrock);
    }
}

#[test]
fn surface_strip_is_walkable() {
    let g = terrain_gen::generate(40, 60, 1);
    for y in 1..=3i32 {
        for x in 1..(g.width() as i32 - 1) {
            assert!(!g.get(x, y).unwrap().solid, "surface tile {},{} should be non-solid", x, y);
        }
    }
}

#[test]
fn depth_layers_appear_in_order() {
    let g = terrain_gen::generate(40, 200, 1);
    assert_eq!(g.get(20, 10).unwrap().layer, Layer::Dirt);
    assert_eq!(g.get(20, 80).unwrap().layer, Layer::Stone);
    assert_eq!(g.get(20, 140).unwrap().layer, Layer::Deep);
    assert_eq!(g.get(20, 195).unwrap().layer, Layer::Core);
}

#[test]
fn interior_has_no_bedrock() {
    let g = terrain_gen::generate(40, 200, 1);
    for y in 1..(g.height() as i32 - 1) {
        for x in 1..(g.width() as i32 - 1) {
            assert_ne!(
                g.get(x, y).unwrap().layer,
                Layer::Bedrock,
                "interior tile ({},{}) should not be Bedrock", x, y
            );
        }
    }
}

#[test]
fn spawn_pocket_is_carved() {
    let g = terrain_gen::generate(40, 200, 1);
    let sp = terrain_gen::spawn_tile(&g);
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            let t = g.get(sp.0 + dx, sp.1 + dy).unwrap();
            assert!(!t.solid, "spawn pocket tile ({},{}) should be non-solid",
                    sp.0 + dx, sp.1 + dy);
        }
    }
    let floor_t = g.get(sp.0, sp.1 + 2).unwrap();
    assert!(floor_t.solid);
    assert_eq!(floor_t.ore, None);
}

#[test]
fn deterministic_for_same_seed() {
    let a = terrain_gen::generate(40, 60, 42);
    let b = terrain_gen::generate(40, 60, 42);
    for y in 0..a.height() as i32 {
        for x in 0..a.width() as i32 {
            assert_eq!(a.get(x, y), b.get(x, y), "tile {},{} mismatch", x, y);
        }
    }
}

#[test]
fn ore_distribution_in_tolerance() {
    let g = terrain_gen::generate(80, 200, 7);
    let mut copper = 0;
    let mut silver = 0;
    let mut gold = 0;
    for y in 0..g.height() as i32 {
        for x in 0..g.width() as i32 {
            match g.get(x, y).unwrap().ore {
                Some(OreKind::Copper) => copper += 1,
                Some(OreKind::Silver) => silver += 1,
                Some(OreKind::Gold) => gold += 1,
                None => {}
            }
        }
    }
    // Loose existence + relative-ordering assertions, kept brittle-resistant
    // to ore-prob tuning. We assert each ore exists at all and that copper
    // (most common in dirt) outnumbers gold (only generated in deep).
    assert!(copper > 0, "expected some copper");
    assert!(silver > 0, "expected some silver");
    assert!(gold > 0,   "expected some gold");
    assert!(copper > gold, "copper should be more common than gold ({} vs {})", copper, gold);
}
