//! Tile component processing.
//!
//! Port of `ojph_tile_comp.h/cpp`. A tile component represents one
//! color component within a tile.

use crate::types::*;
use super::resolution::Resolution;

/// A tile component — one color plane within a tile.
///
/// Contains the resolution hierarchy for this component.
#[derive(Debug, Clone)]
pub struct TileComp {
    /// Component index (0-based).
    pub comp_num: u32,
    /// Rectangle of this tile-component in the image coordinate system.
    pub comp_rect: Rect,
    /// Number of resolution levels.
    pub num_resolutions: u32,
    /// Resolution levels, from lowest (0) to highest.
    pub resolutions: Vec<Resolution>,
    /// Whether reversible coding is used.
    pub reversible: bool,
    /// Component bit depth.
    pub bit_depth: u32,
    /// Whether the component is signed.
    pub is_signed: bool,
}

impl Default for TileComp {
    fn default() -> Self {
        Self {
            comp_num: 0,
            comp_rect: Rect::new(Point::new(0, 0), Size::new(0, 0)),
            num_resolutions: 0,
            resolutions: Vec::new(),
            reversible: true,
            bit_depth: 8,
            is_signed: false,
        }
    }
}

impl TileComp {
    /// Create a new tile component with the given parameters.
    pub fn new(
        comp_num: u32,
        comp_rect: Rect,
        num_decomps: u32,
        log_block_dims: Size,
        log_precinct_sizes: &[Size],
        reversible: bool,
        bit_depth: u32,
        is_signed: bool,
    ) -> Self {
        let num_resolutions = num_decomps + 1;
        let mut resolutions = Vec::with_capacity(num_resolutions as usize);

        for r in 0..num_resolutions {
            // Compute resolution rectangle
            let ds = 1u32 << (num_decomps - r);
            let res_rect = Rect::new(
                Point::new(
                    div_ceil(comp_rect.org.x, ds),
                    div_ceil(comp_rect.org.y, ds),
                ),
                Size::new(
                    div_ceil(comp_rect.org.x + comp_rect.siz.w, ds)
                        - div_ceil(comp_rect.org.x, ds),
                    div_ceil(comp_rect.org.y + comp_rect.siz.h, ds)
                        - div_ceil(comp_rect.org.y, ds),
                ),
            );

            let log_pp = if r < log_precinct_sizes.len() as u32 {
                log_precinct_sizes[r as usize]
            } else {
                Size::new(15, 15)
            };

            resolutions.push(Resolution::new(
                r,
                num_decomps,
                res_rect,
                log_pp,
                log_block_dims,
                reversible,
            ));
        }

        Self {
            comp_num,
            comp_rect,
            num_resolutions,
            resolutions,
            reversible,
            bit_depth,
            is_signed,
        }
    }

    /// Width of this tile component.
    #[inline]
    pub fn width(&self) -> u32 {
        self.comp_rect.siz.w
    }

    /// Height of this tile component.
    #[inline]
    pub fn height(&self) -> u32 {
        self.comp_rect.siz.h
    }

    /// Returns a reference to the resolution at the given level.
    pub fn get_resolution(&self, level: u32) -> Option<&Resolution> {
        self.resolutions.get(level as usize)
    }

    /// Returns a mutable reference to the resolution at the given level.
    pub fn get_resolution_mut(&mut self, level: u32) -> Option<&mut Resolution> {
        self.resolutions.get_mut(level as usize)
    }
}
