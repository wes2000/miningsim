use miningsim::economy::{self, BuyResult, Money};
use miningsim::inventory::Inventory;
use miningsim::items::{ItemKind, OreKind};
use miningsim::tools::{Tool, OwnedTools};

#[test]
fn ore_sell_prices_match_spec() {
    assert_eq!(economy::item_sell_price(ItemKind::Ore(OreKind::Copper)), 1);
    assert_eq!(economy::item_sell_price(ItemKind::Ore(OreKind::Silver)), 5);
    assert_eq!(economy::item_sell_price(ItemKind::Ore(OreKind::Gold)), 20);
}

#[test]
fn tool_buy_prices_match_spec() {
    assert_eq!(economy::tool_buy_price(Tool::Shovel), 0);
    assert_eq!(economy::tool_buy_price(Tool::Pickaxe), 30);
    assert_eq!(economy::tool_buy_price(Tool::Jackhammer), 100);
    assert_eq!(economy::tool_buy_price(Tool::Dynamite), 300);
}

#[test]
fn sell_all_converts_mixed_inventory_and_zeros_counts() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Copper), 5);    //  5 * 1 =  5
    inv.add(ItemKind::Ore(OreKind::Silver), 3);    //  3 * 5 = 15
    inv.add(ItemKind::Ore(OreKind::Gold), 2);      //  2 * 20 = 40
    let mut money = Money::default();
    economy::sell_all(&mut inv, &mut money);
    assert_eq!(money.0, 60);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Copper)), 0);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Silver)), 0);
    assert_eq!(inv.get(ItemKind::Ore(OreKind::Gold)), 0);
}

#[test]
fn sell_all_empty_inventory_is_no_op() {
    let mut inv = Inventory::default();
    let mut money = Money(10);
    economy::sell_all(&mut inv, &mut money);
    assert_eq!(money.0, 10);
}

#[test]
fn try_buy_succeeds_when_affordable() {
    let mut money = Money(50);
    let mut owned = OwnedTools::default();
    let r = economy::try_buy(Tool::Pickaxe, &mut money, &mut owned);
    assert_eq!(r, BuyResult::Ok);
    assert_eq!(money.0, 20);
    assert!(owned.0.contains(&Tool::Pickaxe));
}

#[test]
fn try_buy_returns_not_enough_money_when_poor() {
    let mut money = Money(10);
    let mut owned = OwnedTools::default();
    let r = economy::try_buy(Tool::Pickaxe, &mut money, &mut owned);
    assert_eq!(r, BuyResult::NotEnoughMoney);
    assert_eq!(money.0, 10);
    assert!(!owned.0.contains(&Tool::Pickaxe));
}

#[test]
fn try_buy_returns_already_owned_on_repeat_purchase() {
    let mut money = Money(100);
    let mut owned = OwnedTools::default();   // already has Shovel
    let r = economy::try_buy(Tool::Shovel, &mut money, &mut owned);
    assert_eq!(r, BuyResult::AlreadyOwned);
    assert_eq!(money.0, 100);
}

#[test]
fn try_buy_exact_cost_succeeds_and_zeros_money() {
    let mut money = Money(30);
    let mut owned = OwnedTools::default();
    let r = economy::try_buy(Tool::Pickaxe, &mut money, &mut owned);
    assert_eq!(r, BuyResult::Ok);
    assert_eq!(money.0, 0);
}

#[test]
fn sell_all_sums_ores_and_bars() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Copper), 5);    //  5 *  1 =  5
    inv.add(ItemKind::Bar(OreKind::Copper), 3);    //  3 *  5 = 15
    inv.add(ItemKind::Bar(OreKind::Gold), 1);      //  1 * 100 = 100
    let mut money = Money::default();
    economy::sell_all(&mut inv, &mut money);
    assert_eq!(money.0, 120);
    for item in miningsim::items::ALL_ITEMS {
        assert_eq!(inv.get(item), 0);
    }
}
