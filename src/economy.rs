use bevy::prelude::Resource;

use crate::grid::OreType;
use crate::inventory::Inventory;
use crate::tools::{OwnedTools, Tool};

#[derive(Debug, Default, Resource)]
pub struct Money(pub u32);

pub fn ore_sell_price(ore: OreType) -> u32 {
    match ore {
        OreType::None => 0,
        OreType::Copper => 1,
        OreType::Silver => 5,
        OreType::Gold => 20,
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
    for ore in [OreType::Copper, OreType::Silver, OreType::Gold] {
        let count = inv.get(ore);
        if count == 0 { continue; }
        money.0 += ore_sell_price(ore) * count;
        inv.remove(ore, count);
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
