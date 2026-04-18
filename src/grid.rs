#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Dirt,
    Stone,
    Deep,
    Bedrock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OreType {
    None,
    Copper,
    Silver,
    Gold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub solid: bool,
    pub layer: Layer,
    pub ore: OreType,
}

impl Default for Tile {
    fn default() -> Self {
        Self { solid: true, layer: Layer::Dirt, ore: OreType::None }
    }
}

#[derive(Debug, bevy::prelude::Resource)]
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
