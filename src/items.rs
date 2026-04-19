use serde::{Deserialize, Serialize};

// Variant order is load-bearing: derived `Ord` drives BTreeMap iteration in
// `Inventory.counts` and `SmelterState.output`, both replicated by bevy_replicon.
// Reordering changes diff output and on-disk save shape — bump SAVE_VERSION if changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OreKind {
    Copper,
    Silver,
    Gold,
}

// Variant order is load-bearing: see OreKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemKind {
    Ore(OreKind),
    Bar(OreKind),
}

pub const ALL_ORES: [OreKind; 3] = [OreKind::Copper, OreKind::Silver, OreKind::Gold];

pub const ALL_ITEMS: [ItemKind; 6] = [
    ItemKind::Ore(OreKind::Copper),
    ItemKind::Ore(OreKind::Silver),
    ItemKind::Ore(OreKind::Gold),
    ItemKind::Bar(OreKind::Copper),
    ItemKind::Bar(OreKind::Silver),
    ItemKind::Bar(OreKind::Gold),
];
