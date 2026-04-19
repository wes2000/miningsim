use bevy::math::IVec2;

use miningsim::items::OreKind;
use miningsim::systems::net_events::{
    BuyToolRequest, CollectAllRequest, DigRequest, SellAllRequest, SmeltAllRequest,
};
use miningsim::tools::Tool;

#[test]
fn dig_request_round_trips() {
    let original = DigRequest { target: IVec2::new(7, 12) };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: DigRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded.target, original.target);
}

#[test]
fn buy_tool_request_round_trips() {
    let original = BuyToolRequest { tool: Tool::Pickaxe };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: BuyToolRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded.tool, original.tool);
}

#[test]
fn smelt_all_request_round_trips() {
    let original = SmeltAllRequest { ore: OreKind::Silver };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: SmeltAllRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded.ore, original.ore);
}

#[test]
fn collect_all_request_round_trips() {
    let original = CollectAllRequest;
    let bytes = bincode::serialize(&original).expect("ser");
    let _decoded: CollectAllRequest = bincode::deserialize(&bytes).expect("de");
    // unit struct; existence of decoded value is success
}

#[test]
fn sell_all_request_round_trips() {
    let original = SellAllRequest;
    let bytes = bincode::serialize(&original).expect("ser");
    let _decoded: SellAllRequest = bincode::deserialize(&bytes).expect("de");
}
