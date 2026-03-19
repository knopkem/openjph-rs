//! Subband processing.
//!
//! Port of `ojph_subband.h/cpp`. A subband represents one frequency band
//! (LL, HL, LH, or HH) at a particular resolution level.

use crate::types::*;

/// Subband types: LL (lowpass-lowpass), HL, LH, HH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SubbandType {
    /// LL subband (only at the lowest resolution).
    LL = 0,
    /// HL subband (horizontal highpass, vertical lowpass).
    HL = 1,
    /// LH subband (horizontal lowpass, vertical highpass).
    LH = 2,
    /// HH subband (both highpass).
    HH = 3,
}

impl SubbandType {
    pub fn from_index(i: u32) -> Self {
        match i {
            0 => Self::LL,
            1 => Self::HL,
            2 => Self::LH,
            3 => Self::HH,
            _ => Self::LL,
        }
    }
}

/// A subband within a resolution level.
///
/// Contains the geometry and quantization parameters for one frequency band.
#[derive(Debug, Clone)]
pub struct Subband {
    /// Subband type (LL, HL, LH, HH).
    pub band_type: SubbandType,
    /// Resolution level that owns this subband (0 = lowest).
    pub resolution_num: u32,
    /// Subband rectangle in the coordinate space of the resolution.
    pub band_rect: Rect,
    /// Log2 of the codeblock dimensions used in this subband.
    pub log_block_dims: Size,
    /// Number of missing MSBs (Kmax from quantization).
    pub k_max: u32,
    /// Quantization step size delta (irreversible only).
    pub delta: f32,
    /// True if reversible coding.
    pub reversible: bool,
    /// Number of codeblocks in x direction.
    pub num_blocks_x: u32,
    /// Number of codeblocks in y direction.
    pub num_blocks_y: u32,
}

impl Default for Subband {
    fn default() -> Self {
        Self {
            band_type: SubbandType::LL,
            resolution_num: 0,
            band_rect: Rect::new(Point::new(0, 0), Size::new(0, 0)),
            log_block_dims: Size::new(0, 0),
            k_max: 0,
            delta: 1.0,
            reversible: true,
            num_blocks_x: 0,
            num_blocks_y: 0,
        }
    }
}

impl Subband {
    /// Create a new subband with basic parameters.
    pub fn new(
        band_type: SubbandType,
        resolution_num: u32,
        band_rect: Rect,
        log_block_dims: Size,
    ) -> Self {
        // Compute number of codeblocks
        let bw = 1u32 << log_block_dims.w;
        let bh = 1u32 << log_block_dims.h;
        let num_blocks_x = if band_rect.siz.w > 0 {
            div_ceil(band_rect.org.x + band_rect.siz.w, bw)
                - (band_rect.org.x / bw)
        } else {
            0
        };
        let num_blocks_y = if band_rect.siz.h > 0 {
            div_ceil(band_rect.org.y + band_rect.siz.h, bh)
                - (band_rect.org.y / bh)
        } else {
            0
        };
        Self {
            band_type,
            resolution_num,
            band_rect,
            log_block_dims,
            num_blocks_x,
            num_blocks_y,
            ..Default::default()
        }
    }

    /// Width of the subband.
    #[inline]
    pub fn width(&self) -> u32 {
        self.band_rect.siz.w
    }

    /// Height of the subband.
    #[inline]
    pub fn height(&self) -> u32 {
        self.band_rect.siz.h
    }

    /// True if this subband has zero area.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.band_rect.siz.w == 0 || self.band_rect.siz.h == 0
    }

    /// Total number of codeblocks.
    #[inline]
    pub fn total_blocks(&self) -> u32 {
        self.num_blocks_x * self.num_blocks_y
    }
}
