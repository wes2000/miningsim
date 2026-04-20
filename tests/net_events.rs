use bevy::math::IVec2;

use miningsim::belt::BeltDir;
use miningsim::grid::{Grid, Layer, Tile};
use miningsim::items::OreKind;
use miningsim::systems::net_events::{
    BuyToolRequest, ClientPositionUpdate, CollectAllRequest, DigRequest, GridSnapshot,
    PlaceBeltRequest, RemoveBeltRequest, SellAllRequest, SmeltAllRequest, TileChanged,
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

#[test]
fn place_belt_request_round_trips() {
    let original = PlaceBeltRequest { tile: IVec2::new(7, 12), dir: BeltDir::North };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: PlaceBeltRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}

#[test]
fn remove_belt_request_round_trips() {
    let original = RemoveBeltRequest { tile: IVec2::new(3, 0) };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: RemoveBeltRequest = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}

// ---------- M5b server events (Grid delta replication) ----------

#[test]
fn tile_changed_round_trips() {
    let original = TileChanged {
        pos: IVec2::new(12, 40),
        tile: Tile {
            solid: false,
            layer: Layer::Stone,
            ore: Some(OreKind::Copper),
            damage: 2,
        },
    };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: TileChanged = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}

#[test]
fn grid_snapshot_round_trips() {
    let mut g = Grid::new(3, 3);
    g.set(1, 1, Tile { solid: false, layer: Layer::Dirt, ore: None, damage: 0 });
    let original = GridSnapshot { grid: g };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: GridSnapshot = bincode::deserialize(&bytes).expect("de");
    assert_eq!(
        decoded.grid.get(1, 1).copied(),
        Some(Tile { solid: false, layer: Layer::Dirt, ore: None, damage: 0 })
    );
    assert_eq!(decoded.grid.get(0, 0).copied(), Some(Tile::default()));
    assert_eq!(decoded.grid.width(), 3);
    assert_eq!(decoded.grid.height(), 3);
}

#[test]
fn client_position_update_round_trips() {
    let original = ClientPositionUpdate {
        pos: bevy::math::Vec2::new(123.5, -47.25),
        facing: IVec2::new(1, 0),
    };
    let bytes = bincode::serialize(&original).expect("ser");
    let decoded: ClientPositionUpdate = bincode::deserialize(&bytes).expect("de");
    assert_eq!(decoded, original);
}
