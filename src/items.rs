use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OreKind {
    Copper,
    Silver,
    Gold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
