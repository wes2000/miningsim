use serde::{Deserialize, Serialize};

use crate::economy::Money;
use crate::grid::Grid;
use crate::inventory::Inventory;
use crate::processing::SmelterState;
use crate::tools::OwnedTools;

pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveData {
    /// Schema version. Bump when SaveData layout changes; mismatched
    /// loads are silently discarded (no migration logic yet).
    pub version: u32,
    pub grid: Grid,
    pub inventory: Inventory,
    pub money: Money,
    pub owned_tools: OwnedTools,
    pub smelter: SmelterState,
    /// Player world position as `(x, y)`. Plain array avoids pulling
    /// Bevy types into the pure module.
    pub player_pos: [f32; 2],
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(ron::error::SpannedError),
    VersionMismatch { found: u32, expected: u32 },
}

pub fn collect(
    grid: &Grid,
    inventory: &Inventory,
    money: &Money,
    owned: &OwnedTools,
    smelter: &SmelterState,
    player_pos: [f32; 2],
) -> SaveData {
    SaveData {
        version: SAVE_VERSION,
        grid: grid.clone(),
        inventory: inventory.clone(),
        money: *money,                 // Money is Copy
        owned_tools: owned.clone(),
        smelter: smelter.clone(),
        player_pos,
    }
}

pub fn apply(
    data: SaveData,
    grid: &mut Grid,
    inventory: &mut Inventory,
    money: &mut Money,
    owned: &mut OwnedTools,
    smelter: &mut SmelterState,
    player_pos: &mut [f32; 2],
) {
    *grid = data.grid;
    *inventory = data.inventory;
    *money = data.money;
    *owned = data.owned_tools;
    *smelter = data.smelter;
    *player_pos = data.player_pos;
}

pub fn serialize_ron(data: &SaveData) -> Result<String, ron::Error> {
    ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
}

pub fn deserialize_ron(s: &str) -> Result<SaveData, LoadError> {
    let data: SaveData = ron::de::from_str(s).map_err(LoadError::Parse)?;
    if data.version != SAVE_VERSION {
        return Err(LoadError::VersionMismatch {
            found: data.version,
            expected: SAVE_VERSION,
        });
    }
    Ok(data)
}
