//! Marching-squares contour mesh builder for terrain chunks.
//!
//! Tile centers are sampled as the corners of marching-squares cells. A chunk
//! of N x N tiles produces N x N cells (the cell grid spans the area between
//! the corner tiles of adjacent chunks). Out-of-grid samples are treated as
//! solid bedrock so that map edges close cleanly.
//!
//! Corner / edge layout (world Y points up):
//!
//! ```text
//!     c0 ---- e0 ---- c1
//!      |              |
//!     e3              e1
//!      |              |
//!     c3 ---- e2 ---- c2
//! ```
//!
//! Bitmask: bit k set iff corner k is solid. CCW winding throughout.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use crate::grid::{Grid, Layer};
use crate::systems::chunk_lifecycle::CHUNK_TILES;
use crate::systems::setup::TILE_SIZE_PX;

fn layer_color(l: Layer) -> [f32; 4] {
    match l {
        Layer::Dirt    => [0.55, 0.42, 0.27, 1.0],
        Layer::Stone   => [0.42, 0.33, 0.22, 1.0],
        Layer::Deep    => [0.29, 0.23, 0.15, 1.0],
        Layer::Bedrock => [0.16, 0.13, 0.10, 1.0],
    }
}

/// Sample a tile-center corner. Out-of-grid is solid bedrock.
/// Returns (solid, layer_if_known).
fn sample(grid: &Grid, gx: i32, gy: i32) -> (bool, Option<Layer>) {
    match grid.get(gx, gy) {
        Some(t) => (t.solid, if t.solid { Some(t.layer) } else { None }),
        None => (true, None), // out-of-grid: solid bedrock, no preferred layer
    }
}

/// Build a single contour mesh for the chunk at `chunk_coord`.
/// Returns `None` if the resulting mesh would be empty.
pub fn build_chunk_mesh(grid: &Grid, chunk_coord: IVec2) -> Option<Mesh> {
    let cx = chunk_coord.x;
    let cy = chunk_coord.y;
    let n = CHUNK_TILES;

    // Pre-sample (n+1) x (n+1) corners.
    let stride = (n + 1) as usize;
    let mut solid = vec![false; stride * stride];
    let mut layers: Vec<Option<Layer>> = vec![None; stride * stride];
    for j in 0..=n {
        for i in 0..=n {
            let gx = cx * n + i;
            let gy = cy * n + j;
            let (s, ly) = sample(grid, gx, gy);
            let idx = (j as usize) * stride + (i as usize);
            solid[idx] = s;
            layers[idx] = ly;
        }
    }

    let s_at = |i: i32, j: i32| -> bool { solid[(j as usize) * stride + (i as usize)] };
    let l_at = |i: i32, j: i32| -> Option<Layer> { layers[(j as usize) * stride + (i as usize)] };

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();

    let ts = TILE_SIZE_PX;
    let half = ts * 0.5;

    // Convert grid-space (gx, gy) tile-center to world coords.
    let tile_world = |gx: f32, gy: f32| -> [f32; 3] {
        [gx * ts + half, -(gy * ts + half), 0.0]
    };

    for j in 0..n {
        for i in 0..n {
            let s0 = s_at(i,     j);
            let s1 = s_at(i + 1, j);
            let s2 = s_at(i + 1, j + 1);
            let s3 = s_at(i,     j + 1);

            let mask: u8 =
                  (s0 as u8)
                | ((s1 as u8) << 1)
                | ((s2 as u8) << 2)
                | ((s3 as u8) << 3);
            if mask == 0 { continue; }

            // Cell corner world positions (gx, gy are tile-center coords).
            let gx0 = (cx * n + i) as f32;
            let gy0 = (cy * n + j) as f32;
            let gx1 = gx0 + 1.0;
            let gy1 = gy0 + 1.0;
            let gxm = gx0 + 0.5;
            let gym = gy0 + 0.5;

            let c0 = tile_world(gx0, gy0);
            let c1 = tile_world(gx1, gy0);
            let c2 = tile_world(gx1, gy1);
            let c3 = tile_world(gx0, gy1);
            let e0 = tile_world(gxm, gy0); // top edge mid
            let e1 = tile_world(gx1, gym); // right edge mid
            let e2 = tile_world(gxm, gy1); // bottom edge mid
            let e3 = tile_world(gx0, gym); // left edge mid

            // Pick a color for this cell: prefer first in-grid solid corner.
            let pick_layer = || -> Layer {
                let cands = [
                    (s0, l_at(i,     j)),
                    (s1, l_at(i + 1, j)),
                    (s2, l_at(i + 1, j + 1)),
                    (s3, l_at(i,     j + 1)),
                ];
                for (s, ly) in cands.iter() {
                    if *s { if let Some(l) = ly { return *l; } }
                }
                Layer::Bedrock
            };
            let col = layer_color(pick_layer());

            let mut emit = |a: [f32;3], b: [f32;3], c: [f32;3]| {
                positions.push(a); positions.push(b); positions.push(c);
                colors.push(col); colors.push(col); colors.push(col);
                normals.push([0.0, 0.0, 1.0]);
                normals.push([0.0, 0.0, 1.0]);
                normals.push([0.0, 0.0, 1.0]);
            };

            // 16-case lookup, CCW winding (Y-up world).
            match mask {
                0 => {} // empty
                // single corner
                1  => emit(c0, e3, e0),
                2  => emit(c1, e0, e1),
                4  => emit(c2, e1, e2),
                8  => emit(c3, e2, e3),
                // two adjacent corners (half-cell trapezoid)
                3  => { emit(c0, e3, e1); emit(c0, e1, c1); }            // top half
                6  => { emit(c1, e0, e2); emit(c1, e2, c2); }            // right half
                12 => { emit(c2, e1, e3); emit(c2, e3, c3); }            // bottom half
                9  => { emit(c0, c3, e2); emit(c0, e2, e0); }            // left half
                // two opposite corners (ambiguous: emit both diagonal triangles)
                5  => { emit(c0, e3, e0); emit(c2, e1, e2); }
                10 => { emit(c1, e0, e1); emit(c3, e2, e3); }
                // three corners (pentagon = 3 triangles)
                14 => { emit(e3, c3, c2); emit(e3, c2, c1); emit(e3, c1, e0); } // c0 missing
                13 => { emit(c0, c3, c2); emit(c0, c2, e1); emit(c0, e1, e0); } // c1 missing
                11 => { emit(c0, c3, e2); emit(c0, e2, e1); emit(c0, e1, c1); } // c2 missing
                7  => { emit(c0, e3, e2); emit(c0, e2, c2); emit(c0, c2, c1); } // c3 missing
                // all four corners (full quad)
                15 => { emit(c3, c2, c1); emit(c3, c1, c0); }
                _ => unreachable!(),
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    let count = positions.len() as u32;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32((0..count).collect()));
    Some(mesh)
}
