use miningsim::inventory::Inventory;
use miningsim::items::{ItemKind, OreKind};

#[test]
fn empty_inventory_returns_zero() {
    let inv = Inventory::default();
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Copper)), 0);
}

#[test]
fn add_increments_count() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Copper), 3);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Copper)), 3);
    inv.add(ItemKind::Ore(OreKind::Copper), 2);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Copper)), 5);
}

#[test]
fn remove_decrements_count_floored_at_zero() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Silver), 5);
    inv.remove(ItemKind::Ore(OreKind::Silver), 2);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Silver)), 3);
    inv.remove(ItemKind::Ore(OreKind::Silver), 100);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Silver)), 0);
}

#[test]
fn add_one_ore_does_not_affect_others() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Gold), 1);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Copper)), 0);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Silver)), 0);
}

#[test]
fn inventory_holds_ores_and_bars_distinctly() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Copper), 3);
    inv.add(ItemKind::Bar(OreKind::Copper), 2);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Copper)), 3);
    assert_eq!(inv.get(ItemKind::Bar(OreKind::Copper)), 2);
}
