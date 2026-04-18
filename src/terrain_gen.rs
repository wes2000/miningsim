use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::grid::{Grid, Layer, OreType, Tile};

const SURFACE_ROWS: i32 = 3;
const DIRT_FRAC: f32 = 0.30;
const STONE_FRAC: f32 = 0.40;
const DEEP_FRAC: f32 = 0.27;

fn ore_probs(layer: Layer) -> [(OreType, f32); 3] {
    match layer {
        Layer::Dirt  => [(OreType::Copper, 0.04),  (OreType::Silver, 0.005), (OreType::Gold, 0.0)],
        Layer::Stone => [(OreType::Copper, 0.02),  (OreType::Silver, 0.025), (OreType::Gold, 0.003)],
        Layer::Deep  => [(OreType::Copper, 0.005), (OreType::Silver, 0.015), (OreType::Gold, 0.02)],
        Layer::Core => [(OreType::None, 0.0); 3],
        Layer::Bedrock => [(OreType::None, 0.0); 3],
    }
}

pub fn generate(width: u32, height: u32, seed: u64) -> Grid {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut g = Grid::new(width, height);

    let interior_h = (height as i32) - 2 - SURFACE_ROWS;
    let dirt_end  = 1 + SURFACE_ROWS + (interior_h as f32 * DIRT_FRAC) as i32;
    let stone_end = dirt_end + (interior_h as f32 * STONE_FRAC) as i32;
    let deep_end  = stone_end + (interior_h as f32 * DEEP_FRAC) as i32;

    for y in 0..(height as i32) {
        for x in 0..(width as i32) {
            let mut tile = Tile::default();
            if x == 0 || y == 0 || x == width as i32 - 1 || y == height as i32 - 1 {
                tile.layer = Layer::Bedrock;
            } else if y <= SURFACE_ROWS {
                tile.solid = false;
                tile.layer = Layer::Dirt;
            } else if y < dirt_end {
                tile.layer = Layer::Dirt;
                maybe_assign_ore(&mut tile, &mut rng);
            } else if y < stone_end {
                tile.layer = Layer::Stone;
                maybe_assign_ore(&mut tile, &mut rng);
            } else if y < deep_end {
                tile.layer = Layer::Deep;
                maybe_assign_ore(&mut tile, &mut rng);
            } else {
                tile.layer = Layer::Core;
            }
            g.set(x, y, tile);
        }
    }

    carve_spawn_pocket(&mut g);
    g
}

pub fn spawn_tile(g: &Grid) -> (i32, i32) {
    ((g.width() / 2) as i32, SURFACE_ROWS + 1)
}

fn maybe_assign_ore(tile: &mut Tile, rng: &mut StdRng) {
    let probs = ore_probs(tile.layer);
    let r: f32 = rng.gen();
    let mut acc = 0.0;
    for (ore, p) in probs {
        acc += p;
        if r < acc {
            tile.ore = ore;
            return;
        }
    }
}

fn carve_spawn_pocket(g: &mut Grid) {
    let sp = spawn_tile(g);
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            if let Some(t) = g.get_mut(sp.0 + dx, sp.1 + dy) {
                t.solid = false;
                t.ore = OreType::None;
            }
        }
    }
    if let Some(t) = g.get_mut(sp.0, sp.1 + 2) {
        t.solid = true;
        t.ore = OreType::None;
    }
}
