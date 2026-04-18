use miningsim::grid::OreType;
use miningsim::inventory::Inventory;

#[test]
fn empty_inventory_returns_zero() {
    let inv = Inventory::default();
    assert_eq!(inv.get(OreType::Copper), 0);
}

#[test]
fn add_increments_count() {
    let mut inv = Inventory::default();
    inv.add(OreType::Copper, 3);
    assert_eq!(inv.get(OreType::Copper), 3);
    inv.add(OreType::Copper, 2);
    assert_eq!(inv.get(OreType::Copper), 5);
}

#[test]
fn remove_decrements_count_floored_at_zero() {
    let mut inv = Inventory::default();
    inv.add(OreType::Silver, 5);
    inv.remove(OreType::Silver, 2);
    assert_eq!(inv.get(OreType::Silver), 3);
    inv.remove(OreType::Silver, 100);
    assert_eq!(inv.get(OreType::Silver), 0);
}

#[test]
fn add_one_ore_does_not_affect_others() {
    let mut inv = Inventory::default();
    inv.add(OreType::Gold, 1);
    assert_eq!(inv.get(OreType::Copper), 0);
    assert_eq!(inv.get(OreType::Silver), 0);
}
