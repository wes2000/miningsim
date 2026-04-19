use std::collections::{HashMap, HashSet};

use miningsim::economy::Money;
use miningsim::grid::{Grid, Layer, Tile};
use miningsim::inventory::Inventory;
use miningsim::items::{ItemKind, OreKind};
use miningsim::processing::SmelterState;
use miningsim::save::{self, LoadError, SaveData, SAVE_VERSION};
use miningsim::tools::{OwnedTools, Tool};

fn sample_save_data() -> SaveData {
    let mut grid = Grid::new(5, 5);
    grid.set(2, 2, Tile { solid: true, layer: Layer::Stone, ore: Some(OreKind::Copper), damage: 1 });

    let mut inventory = Inventory::default();
    inventory.add(ItemKind::Ore(OreKind::Copper), 5);
    inventory.add(ItemKind::Bar(OreKind::Gold), 2);

    let mut owned_tools = OwnedTools::default();
    owned_tools.0.insert(Tool::Pickaxe);

    let mut output = HashMap::new();
    output.insert(OreKind::Copper, 3);
    let smelter = SmelterState {
        recipe: Some(OreKind::Silver),
        time_left: 1.4,
        queue: 2,
        output,
    };

    SaveData {
        version: SAVE_VERSION,
        grid,
        inventory,
        money: Money(120),
        owned_tools,
        smelter,
        player_pos: [200.0, -75.0],
    }
}

#[test]
fn round_trip_via_ron() {
    let data = sample_save_data();
    let s = save::serialize_ron(&data).expect("serialize");
    let parsed = save::deserialize_ron(&s).expect("deserialize");
    assert_eq!(parsed.version, SAVE_VERSION);
    assert_eq!(parsed.money.0, 120);
    assert_eq!(parsed.player_pos, [200.0, -75.0]);
    assert_eq!(parsed.inventory.get(ItemKind::Ore(OreKind::Copper)), 5);
    assert_eq!(parsed.inventory.get(ItemKind::Bar(OreKind::Gold)), 2);
    assert!(parsed.owned_tools.0.contains(&Tool::Pickaxe));
    assert_eq!(parsed.smelter.recipe, Some(OreKind::Silver));
    assert_eq!(parsed.smelter.queue, 2);
    assert_eq!(*parsed.smelter.output.get(&OreKind::Copper).unwrap(), 3);
    assert_eq!(parsed.grid.get(2, 2).unwrap().damage, 1);
}

#[test]
fn collect_round_trips_state() {
    let data_a = sample_save_data();
    let collected = save::collect(
        &data_a.grid,
        &data_a.inventory,
        &data_a.money,
        &data_a.owned_tools,
        &data_a.smelter,
        data_a.player_pos,
    );
    assert_eq!(collected.money.0, data_a.money.0);
    assert_eq!(collected.player_pos, data_a.player_pos);
    assert_eq!(collected.smelter.queue, data_a.smelter.queue);
}

#[test]
fn apply_overwrites_destination_state() {
    let saved = sample_save_data();
    let s = save::serialize_ron(&saved).expect("ser");
    let loaded = save::deserialize_ron(&s).expect("de");

    let mut grid = Grid::new(5, 5);
    let mut inventory = Inventory::default();
    let mut money = Money::default();
    let mut owned = OwnedTools::default();
    let mut smelter = SmelterState::default();
    let mut pos = [0.0, 0.0];

    save::apply(loaded, &mut grid, &mut inventory, &mut money, &mut owned, &mut smelter, &mut pos);

    assert_eq!(money.0, 120);
    assert_eq!(pos, [200.0, -75.0]);
    assert_eq!(inventory.get(ItemKind::Ore(OreKind::Copper)), 5);
    assert!(owned.0.contains(&Tool::Pickaxe));
    assert_eq!(smelter.recipe, Some(OreKind::Silver));
    assert_eq!(grid.get(2, 2).unwrap().damage, 1);
}

#[test]
fn apply_is_idempotent() {
    let saved = sample_save_data();

    let mut grid = Grid::new(5, 5);
    let mut inventory = Inventory::default();
    let mut money = Money::default();
    let mut owned = OwnedTools::default();
    let mut smelter = SmelterState::default();
    let mut pos = [0.0, 0.0];

    let s = save::serialize_ron(&saved).expect("ser");
    save::apply(save::deserialize_ron(&s).unwrap(), &mut grid, &mut inventory, &mut money, &mut owned, &mut smelter, &mut pos);
    let money_after_first = money.0;
    let pos_after_first = pos;
    let copper_after_first = inventory.get(ItemKind::Ore(OreKind::Copper));

    save::apply(save::deserialize_ron(&s).unwrap(), &mut grid, &mut inventory, &mut money, &mut owned, &mut smelter, &mut pos);
    assert_eq!(money.0, money_after_first);
    assert_eq!(pos, pos_after_first);
    assert_eq!(inventory.get(ItemKind::Ore(OreKind::Copper)), copper_after_first);
}

#[test]
fn version_mismatch_is_detected() {
    let mut data = sample_save_data();
    data.version = SAVE_VERSION.wrapping_add(1);
    let s = save::serialize_ron(&data).expect("ser");
    let result = save::deserialize_ron(&s);
    match result {
        Err(LoadError::VersionMismatch { found, expected }) => {
            assert_eq!(found, SAVE_VERSION.wrapping_add(1));
            assert_eq!(expected, SAVE_VERSION);
        }
        other => panic!("expected VersionMismatch, got {:?}", other),
    }
}

#[test]
fn malformed_ron_returns_parse_error() {
    let result = save::deserialize_ron("this is not valid ron");
    assert!(matches!(result, Err(LoadError::Parse(_))));
}

#[test]
fn inventory_round_trip_with_mixed_kinds() {
    let mut inv = Inventory::default();
    inv.add(ItemKind::Ore(OreKind::Copper), 5);
    inv.add(ItemKind::Bar(OreKind::Silver), 2);
    inv.add(ItemKind::Ore(OreKind::Gold), 1);
    let s = ron::ser::to_string(&inv).expect("ser");
    let inv2: Inventory = ron::de::from_str(&s).expect("de");
    assert_eq!(inv2.get(ItemKind::Ore(OreKind::Copper)), 5);
    assert_eq!(inv2.get(ItemKind::Bar(OreKind::Silver)), 2);
    assert_eq!(inv2.get(ItemKind::Ore(OreKind::Gold)), 1);
}

#[test]
fn owned_tools_round_trip() {
    let mut owned = OwnedTools::default();
    owned.0.insert(Tool::Pickaxe);
    owned.0.insert(Tool::Jackhammer);
    let s = ron::ser::to_string(&owned).expect("ser");
    let owned2: OwnedTools = ron::de::from_str(&s).expect("de");
    assert!(owned2.0.contains(&Tool::Shovel));
    assert!(owned2.0.contains(&Tool::Pickaxe));
    assert!(owned2.0.contains(&Tool::Jackhammer));
    assert!(!owned2.0.contains(&Tool::Dynamite));
}

#[test]
fn smelter_state_round_trip_with_active_recipe_and_output() {
    let mut output = HashMap::new();
    output.insert(OreKind::Copper, 7);
    output.insert(OreKind::Gold, 1);
    let s = SmelterState { recipe: Some(OreKind::Silver), time_left: 0.7, queue: 4, output };
    let ron_s = ron::ser::to_string(&s).expect("ser");
    let s2: SmelterState = ron::de::from_str(&ron_s).expect("de");
    assert_eq!(s2.recipe, Some(OreKind::Silver));
    assert_eq!(s2.queue, 4);
    assert_eq!(*s2.output.get(&OreKind::Copper).unwrap(), 7);
    assert_eq!(*s2.output.get(&OreKind::Gold).unwrap(), 1);
    assert!((s2.time_left - 0.7).abs() < 1e-6);
}
