//! Tile processing.
//!
//! Port of `ojph_tile.h/cpp`. A tile is the top-level subdivision of the
//! image. Each tile contains tile-components.

use crate::types::*;
use crate::params::{ParamSiz, ParamCod, ParamQcd, ParamSot};
use super::tile_comp::TileComp;

/// A single tile within the codestream.
///
/// Contains the tile-components (one per image component).
#[derive(Debug, Clone)]
pub struct Tile {
    /// Tile index (raster order within the tile grid).
    pub tile_idx: u32,
    /// Tile rectangle in the image coordinate system.
    pub tile_rect: Rect,
    /// Tile components (one per image component).
    pub tile_comps: Vec<TileComp>,
    /// Number of components.
    pub num_comps: u32,
    /// Whether color transform is employed for this tile.
    pub employ_color_transform: bool,
    /// Number of tile parts.
    pub num_tileparts: u32,
    /// SOT marker data for this tile.
    pub sot: ParamSot,
    /// Whether the tile has been fully read/written.
    pub is_complete: bool,
    /// Current line being processed within this tile.
    pub cur_line: u32,
    /// Number of lines in this tile.
    pub num_lines: u32,
    /// Skipped resolution levels for reconstruction.
    pub skipped_res_for_recon: u32,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            tile_idx: 0,
            tile_rect: Rect::new(Point::new(0, 0), Size::new(0, 0)),
            tile_comps: Vec::new(),
            num_comps: 0,
            employ_color_transform: false,
            num_tileparts: 1,
            sot: ParamSot::default(),
            is_complete: false,
            cur_line: 0,
            num_lines: 0,
            skipped_res_for_recon: 0,
        }
    }
}

impl Tile {
    /// Create a new tile with the given parameters.
    pub fn new(
        tile_idx: u32,
        tile_rect: Rect,
        siz: &ParamSiz,
        cod: &ParamCod,
        _qcd: &ParamQcd,
        skipped_res_for_recon: u32,
    ) -> Self {
        let num_comps = siz.get_num_components() as u32;
        let mut tile_comps = Vec::with_capacity(num_comps as usize);

        for c in 0..num_comps {
            let ds = siz.get_downsampling(c);
            let comp_rect = Rect::new(
                Point::new(
                    div_ceil(tile_rect.org.x, ds.x),
                    div_ceil(tile_rect.org.y, ds.y),
                ),
                Size::new(
                    div_ceil(tile_rect.org.x + tile_rect.siz.w, ds.x)
                        - div_ceil(tile_rect.org.x, ds.x),
                    div_ceil(tile_rect.org.y + tile_rect.siz.h, ds.y)
                        - div_ceil(tile_rect.org.y, ds.y),
                ),
            );

            let cp = cod.get_coc(c);
            let num_decomps = cp.get_num_decompositions() as u32;
            let log_block_dims = cp.get_log_block_dims();

            // Collect precinct sizes
            let mut log_pp = Vec::with_capacity((num_decomps + 1) as usize);
            for r in 0..=num_decomps {
                log_pp.push(cp.get_log_precinct_size(r));
            }

            tile_comps.push(TileComp::new(
                c,
                comp_rect,
                num_decomps,
                log_block_dims,
                &log_pp,
                cp.is_reversible(),
                siz.get_bit_depth(c),
                siz.is_signed(c),
            ));
        }

        // Compute number of tile lines from highest resolution
        let num_lines = if skipped_res_for_recon > 0 {
            let ds = 1u32 << skipped_res_for_recon;
            div_ceil(tile_rect.siz.h, ds)
        } else {
            tile_rect.siz.h
        };

        Self {
            tile_idx,
            tile_rect,
            tile_comps,
            num_comps,
            employ_color_transform: cod.is_employing_color_transform(),
            num_tileparts: 1,
            sot: ParamSot::default(),
            is_complete: false,
            cur_line: 0,
            num_lines,
            skipped_res_for_recon,
        }
    }

    /// Width of this tile.
    #[inline]
    pub fn width(&self) -> u32 {
        self.tile_rect.siz.w
    }

    /// Height of this tile.
    #[inline]
    pub fn height(&self) -> u32 {
        self.tile_rect.siz.h
    }

    /// Returns a reference to the tile component at the given index.
    pub fn get_comp(&self, comp: u32) -> Option<&TileComp> {
        self.tile_comps.get(comp as usize)
    }

    /// Returns a mutable reference to the tile component at the given index.
    pub fn get_comp_mut(&mut self, comp: u32) -> Option<&mut TileComp> {
        self.tile_comps.get_mut(comp as usize)
    }

    /// Returns the number of tileparts for this tile, considering divisions.
    pub fn compute_num_tileparts(&self, tilepart_div: u32) -> u32 {
        if tilepart_div == 0 {
            return 1;
        }
        let mut tps = 1u32;
        if tilepart_div & super::super::params::local::TILEPART_COMPONENTS != 0 {
            tps *= self.num_comps;
        }
        if tilepart_div & super::super::params::local::TILEPART_RESOLUTIONS != 0 {
            // Use the max num_resolutions across all components
            let max_res = self.tile_comps.iter()
                .map(|tc| tc.num_resolutions)
                .max()
                .unwrap_or(1);
            tps *= max_res;
        }
        tps
    }
}
