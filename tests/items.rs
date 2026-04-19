use miningsim::items::{ItemKind, OreKind, ALL_ITEMS, ALL_ORES};

#[test]
fn all_ores_lists_three_kinds() {
    assert_eq!(ALL_ORES.len(), 3);
    assert!(ALL_ORES.contains(&OreKind::Copper));
    assert!(ALL_ORES.contains(&OreKind::Silver));
    assert!(ALL_ORES.contains(&OreKind::Gold));
}

#[test]
fn all_items_lists_six_combinations() {
    assert_eq!(ALL_ITEMS.len(), 6);
    for ore in ALL_ORES {
        assert!(ALL_ITEMS.contains(&ItemKind::Ore(ore)));
        assert!(ALL_ITEMS.contains(&ItemKind::Bar(ore)));
    }
}

#[test]
fn item_kind_round_trips_through_hashset() {
    use std::collections::HashSet;
    let s: HashSet<ItemKind> = ALL_ITEMS.iter().copied().collect();
    assert_eq!(s.len(), 6);
}

#[test]
fn ore_kind_and_item_kind_are_copy() {
    let o = OreKind::Copper;
    let _o2 = o;
    let _o3 = o;
    let i = ItemKind::Ore(OreKind::Silver);
    let _i2 = i;
    let _i3 = i;
}
