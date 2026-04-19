use bevy::prelude::Resource;

use crate::inventory::Inventory;
use crate::items::{ItemKind, OreKind, ALL_ITEMS};
use crate::tools::{OwnedTools, Tool};

#[derive(Debug, Default, Resource)]
pub struct Money(pub u32);

pub fn item_sell_price(item: ItemKind) -> u32 {
    match item {
        ItemKind::Ore(OreKind::Copper) => 1,
        ItemKind::Ore(OreKind::Silver) => 5,
        ItemKind::Ore(OreKind::Gold)   => 20,
        ItemKind::Bar(OreKind::Copper) => 5,
        ItemKind::Bar(OreKind::Silver) => 25,
        ItemKind::Bar(OreKind::Gold)   => 100,
    }
}

pub fn tool_buy_price(tool: Tool) -> u32 {
    match tool {
        Tool::Shovel => 0,
        Tool::Pickaxe => 30,
        Tool::Jackhammer => 100,
        Tool::Dynamite => 300,
    }
}

pub fn sell_all(inv: &mut Inventory, money: &mut Money) {
    for item in ALL_ITEMS {
        let count = inv.get(item);
        if count == 0 { continue; }
        money.0 += item_sell_price(item) * count;
        inv.remove(item, count);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuyResult {
    Ok,
    AlreadyOwned,
    NotEnoughMoney,
}

pub fn try_buy(tool: Tool, money: &mut Money, owned: &mut OwnedTools) -> BuyResult {
    if owned.0.contains(&tool) {
        return BuyResult::AlreadyOwned;
    }
    let price = tool_buy_price(tool);
    if money.0 < price {
        return BuyResult::NotEnoughMoney;
    }
    money.0 -= price;
    owned.0.insert(tool);
    BuyResult::Ok
}
