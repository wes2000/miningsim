use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layer {
    Dirt,
    Stone,
    Deep,
    Core,     // NEW — deepest diggable band (Dynamite-only)
    Bedrock,  // map boundary, never breakable
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub solid: bool,
    pub layer: Layer,
    pub ore: Option<crate::items::OreKind>,
    pub damage: u8,  // strikes accumulated; 0 on fresh / broken tile
}

impl Default for Tile {
    fn default() -> Self {
        Self { solid: true, layer: Layer::Dirt, ore: None, damage: 0 }
    }
}

#[derive(Debug, Clone, bevy::prelude::Resource, Serialize, Deserialize)]
pub struct Grid {
    width: u32,
    height: u32,
    tiles: Vec<Tile>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Grid dims must be positive");
        let tiles = vec![Tile::default(); (width * height) as usize];
        Self { width, height, tiles }
    }

    /// Build a Grid from existing tile data. Panics if `tiles.len() != width * height`.
    /// Used by save/load and tests; gameplay code should prefer `Grid::new(...)`.
    pub fn from_raw(width: u32, height: u32, tiles: Vec<Tile>) -> Self {
        assert!(width > 0 && height > 0, "Grid dims must be positive");
        let expected = (width as usize) * (height as usize);
        assert!(
            tiles.len() == expected,
            "from_raw: tile count {} doesn't match width*height {}",
            tiles.len(), expected,
        );
        Self { width, height, tiles }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&Tile> {
        if !self.in_bounds(x, y) { return None; }
        Some(&self.tiles[self.idx(x, y)])
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Tile> {
        if !self.in_bounds(x, y) { return None; }
        let i = self.idx(x, y);
        Some(&mut self.tiles[i])
    }

    pub fn set(&mut self, x: i32, y: i32, t: Tile) {
        assert!(self.in_bounds(x, y), "set out of bounds: {},{}", x, y);
        let i = self.idx(x, y);
        self.tiles[i] = t;
    }

    fn idx(&self, x: i32, y: i32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }
}
