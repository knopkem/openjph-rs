//! Tile component processing.
//!
//! Port of `ojph_tile_comp.h/cpp`. A tile component represents one
//! color component within a tile.

use crate::error::Result;
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
    /// Stored image lines (encode path). Each entry is one row.
    pub lines: Vec<Vec<i32>>,
    /// Decoded image lines (decode path).
    pub decoded_lines: Vec<Vec<i32>>,
    /// Next line to pull for the decode path.
    pub cur_pull_line: u32,
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
            lines: Vec::new(),
            decoded_lines: Vec::new(),
            cur_pull_line: 0,
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
            lines: Vec::new(),
            decoded_lines: Vec::new(),
            cur_pull_line: 0,
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

    /// Push a line of image data (encode path).
    /// The line is level-shifted for unsigned components.
    pub fn push_line(&mut self, line: &[i32]) {
        let w = self.width() as usize;
        let mut row = vec![0i32; w];
        let copy_len = w.min(line.len());
        row[..copy_len].copy_from_slice(&line[..copy_len]);

        // Level-shift unsigned components
        if !self.is_signed {
            let shift = 1i32 << (self.bit_depth - 1);
            for v in &mut row {
                *v -= shift;
            }
        }
        self.lines.push(row);
    }

    /// Perform forward DWT on stored lines, populating subband coefficients.
    pub fn perform_dwt(&mut self) -> Result<()> {
        let num_decomps = self.num_resolutions - 1;

        if num_decomps == 0 {
            // No DWT: store lines directly in LL subband
            let w = self.width() as usize;
            let h = self.height() as usize;
            let mut coeffs = vec![0i32; w * h];
            for (y, row) in self.lines.iter().enumerate() {
                if y >= h { break; }
                let n = w.min(row.len());
                coeffs[y * w..y * w + n].copy_from_slice(&row[..n]);
            }
            self.resolutions[0].subbands[0].coeffs = coeffs;
            return Ok(());
        }

        // Multi-level DWT: apply iteratively
        let mut current: Vec<Vec<i32>> = self.lines.clone();
        for d in 0..num_decomps {
            let res_idx = (num_decomps - d) as usize; // highest resolution first
            let (ll, hl, lh, hh) = dwt53_forward_2d(&current);

            // Store detail subbands at this resolution level
            let res = &mut self.resolutions[res_idx];
            let sb_w = res.subbands[0].width() as usize;
            let sb_h = res.subbands[0].height() as usize;
            res.subbands[0].coeffs = flatten_2d(&hl, sb_w, sb_h);
            res.subbands[1].coeffs = flatten_2d(&lh, res.subbands[1].width() as usize,
                                                       res.subbands[1].height() as usize);
            res.subbands[2].coeffs = flatten_2d(&hh, res.subbands[2].width() as usize,
                                                       res.subbands[2].height() as usize);
            current = ll;
        }

        // Store final LL in resolution 0
        let ll_w = self.resolutions[0].subbands[0].width() as usize;
        let ll_h = self.resolutions[0].subbands[0].height() as usize;
        self.resolutions[0].subbands[0].coeffs = flatten_2d(&current, ll_w, ll_h);

        Ok(())
    }

    /// Perform inverse DWT, reconstructing decoded_lines from subband coefficients.
    pub fn perform_idwt(&mut self) -> Result<()> {
        let num_decomps = self.num_resolutions - 1;
        let w = self.width() as usize;
        let h = self.height() as usize;

        if num_decomps == 0 {
            // No DWT: extract directly from LL subband
            let sb_w = self.resolutions[0].subbands[0].width() as usize;
            let coeffs = &self.resolutions[0].subbands[0].coeffs;
            let mut lines = Vec::with_capacity(h);
            for y in 0..h {
                let mut row = vec![0i32; w];
                let n = w.min(sb_w);
                if y * sb_w + n <= coeffs.len() {
                    row[..n].copy_from_slice(&coeffs[y * sb_w..y * sb_w + n]);
                }
                lines.push(row);
            }
            self.decoded_lines = lines;
            self.undo_level_shift();
            return Ok(());
        }

        // Start from LL at resolution 0
        let ll_w = self.resolutions[0].subbands[0].width() as usize;
        let ll_h = self.resolutions[0].subbands[0].height() as usize;
        let mut current = unflatten_2d(&self.resolutions[0].subbands[0].coeffs, ll_w, ll_h);

        // Iterate from the lowest detail resolution to the highest
        for d in (0..num_decomps).rev() {
            let res_idx = (num_decomps - d) as usize;
            let res = &self.resolutions[res_idx];
            let hl = unflatten_2d(&res.subbands[0].coeffs,
                                  res.subbands[0].width() as usize,
                                  res.subbands[0].height() as usize);
            let lh = unflatten_2d(&res.subbands[1].coeffs,
                                  res.subbands[1].width() as usize,
                                  res.subbands[1].height() as usize);
            let hh = unflatten_2d(&res.subbands[2].coeffs,
                                  res.subbands[2].width() as usize,
                                  res.subbands[2].height() as usize);

            let out_w = if res_idx == num_decomps as usize { w } else {
                self.resolutions[res_idx].res_rect.siz.w as usize
            };
            let out_h = if res_idx == num_decomps as usize { h } else {
                self.resolutions[res_idx].res_rect.siz.h as usize
            };
            current = dwt53_inverse_2d(&current, &hl, &lh, &hh, out_w, out_h);
        }

        self.decoded_lines = current;
        self.undo_level_shift();
        Ok(())
    }

    /// Undo level shift for unsigned components.
    fn undo_level_shift(&mut self) {
        if !self.is_signed {
            let shift = 1i32 << (self.bit_depth - 1);
            let max_val = (1i32 << self.bit_depth) - 1;
            for row in &mut self.decoded_lines {
                for v in row.iter_mut() {
                    *v = (*v + shift).max(0).min(max_val);
                }
            }
        }
    }

    /// Pull the next decoded line (decode path).
    pub fn pull_line(&mut self) -> Option<Vec<i32>> {
        let idx = self.cur_pull_line as usize;
        if idx < self.decoded_lines.len() {
            self.cur_pull_line += 1;
            Some(self.decoded_lines[idx].clone())
        } else {
            None
        }
    }
}

// =========================================================================
// Simple 5/3 reversible DWT (scalar, for correctness)
// =========================================================================

/// Forward 1D 5/3 DWT on a signal of length n.
/// Returns (low, high) subbands.
fn dwt53_forward_1d(input: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let n = input.len();
    if n == 0 {
        return (vec![], vec![]);
    }
    if n == 1 {
        return (vec![input[0]], vec![]);
    }

    let low_len = (n + 1) / 2;
    let high_len = n / 2;

    // Split into even (s) and odd (d) samples
    let mut s: Vec<i32> = (0..low_len).map(|i| input[2 * i]).collect();
    let mut d: Vec<i32> = (0..high_len).map(|i| input[2 * i + 1]).collect();

    // Predict (high-pass): d[i] -= (s[i] + s[i+1]) >> 1
    for i in 0..high_len {
        let s_right = if i + 1 < low_len { s[i + 1] } else { s[low_len - 1] };
        d[i] -= (s[i] + s_right) >> 1;
    }

    // Update (low-pass): s[i] += (d[i-1] + d[i] + 2) >> 2
    for i in 0..low_len {
        let d_left = if i > 0 { d[i - 1] } else { d[0] };
        let d_right = if i < high_len { d[i] } else { d[high_len - 1] };
        s[i] += (d_left + d_right + 2) >> 2;
    }

    (s, d)
}

/// Inverse 1D 5/3 DWT — reconstructs from (low, high) subbands.
fn dwt53_inverse_1d(low: &[i32], high: &[i32]) -> Vec<i32> {
    let low_len = low.len();
    let high_len = high.len();
    let n = low_len + high_len;

    if n == 0 { return vec![]; }
    if n == 1 { return low.to_vec(); }

    let mut s = low.to_vec();
    let mut d = high.to_vec();

    // Undo update: s[i] -= (d[i-1] + d[i] + 2) >> 2
    for i in 0..low_len {
        let d_left = if i > 0 { d[i - 1] } else { d[0] };
        let d_right = if i < high_len { d[i] } else { d[high_len - 1] };
        s[i] -= (d_left + d_right + 2) >> 2;
    }

    // Undo predict: d[i] += (s[i] + s[i+1]) >> 1
    for i in 0..high_len {
        let s_right = if i + 1 < low_len { s[i + 1] } else { s[low_len - 1] };
        d[i] += (s[i] + s_right) >> 1;
    }

    // Interleave: even positions get s, odd get d
    let mut output = vec![0i32; n];
    for i in 0..low_len {
        output[2 * i] = s[i];
    }
    for i in 0..high_len {
        output[2 * i + 1] = d[i];
    }
    output
}

/// Forward 2D 5/3 DWT (one decomposition level).
/// Returns (ll, hl, lh, hh) as 2D arrays.
fn dwt53_forward_2d(data: &[Vec<i32>]) -> (Vec<Vec<i32>>, Vec<Vec<i32>>, Vec<Vec<i32>>, Vec<Vec<i32>>) {
    let height = data.len();
    let width = if height > 0 { data[0].len() } else { 0 };

    if height == 0 || width == 0 {
        return (vec![], vec![], vec![], vec![]);
    }

    let low_w = (width + 1) / 2;
    let high_w = width / 2;
    let low_h = (height + 1) / 2;
    let high_h = height / 2;

    // Step 1: Horizontal DWT on each row
    let mut h_low = Vec::with_capacity(height);
    let mut h_high = Vec::with_capacity(height);
    for row in data {
        let (l, h) = dwt53_forward_1d(row);
        h_low.push(l);
        h_high.push(h);
    }

    // Step 2: Vertical DWT on columns of h_low → LL, LH
    let mut ll = vec![vec![0i32; low_w]; low_h];
    let mut lh = vec![vec![0i32; low_w]; high_h];
    for x in 0..low_w {
        let col: Vec<i32> = h_low.iter().map(|row| {
            if x < row.len() { row[x] } else { 0 }
        }).collect();
        let (s, d) = dwt53_forward_1d(&col);
        for y in 0..low_h.min(s.len()) { ll[y][x] = s[y]; }
        for y in 0..high_h.min(d.len()) { lh[y][x] = d[y]; }
    }

    // Step 3: Vertical DWT on columns of h_high → HL, HH
    let mut hl = vec![vec![0i32; high_w]; low_h];
    let mut hh = vec![vec![0i32; high_w]; high_h];
    for x in 0..high_w {
        let col: Vec<i32> = h_high.iter().map(|row| {
            if x < row.len() { row[x] } else { 0 }
        }).collect();
        let (s, d) = dwt53_forward_1d(&col);
        for y in 0..low_h.min(s.len()) { hl[y][x] = s[y]; }
        for y in 0..high_h.min(d.len()) { hh[y][x] = d[y]; }
    }

    (ll, hl, lh, hh)
}

/// Inverse 2D 5/3 DWT — reconstructs image from subbands.
fn dwt53_inverse_2d(
    ll: &[Vec<i32>], hl: &[Vec<i32>], lh: &[Vec<i32>], hh: &[Vec<i32>],
    width: usize, height: usize,
) -> Vec<Vec<i32>> {
    if width == 0 || height == 0 {
        return vec![];
    }

    let low_w = (width + 1) / 2;
    let high_w = width / 2;

    // Step 1: Inverse vertical DWT on (LL, LH) → h_low columns
    let mut h_low = vec![vec![0i32; low_w]; height];
    for x in 0..low_w {
        let low_col: Vec<i32> = ll.iter().map(|row| {
            if x < row.len() { row[x] } else { 0 }
        }).collect();
        let high_col: Vec<i32> = lh.iter().map(|row| {
            if x < row.len() { row[x] } else { 0 }
        }).collect();
        let col = dwt53_inverse_1d(&low_col, &high_col);
        for y in 0..height.min(col.len()) {
            h_low[y][x] = col[y];
        }
    }

    // Step 2: Inverse vertical DWT on (HL, HH) → h_high columns
    let mut h_high = vec![vec![0i32; high_w]; height];
    for x in 0..high_w {
        let low_col: Vec<i32> = hl.iter().map(|row| {
            if x < row.len() { row[x] } else { 0 }
        }).collect();
        let high_col: Vec<i32> = hh.iter().map(|row| {
            if x < row.len() { row[x] } else { 0 }
        }).collect();
        let col = dwt53_inverse_1d(&low_col, &high_col);
        for y in 0..height.min(col.len()) {
            h_high[y][x] = col[y];
        }
    }

    // Step 3: Inverse horizontal DWT → output rows
    let mut output = Vec::with_capacity(height);
    for y in 0..height {
        let row = dwt53_inverse_1d(&h_low[y], &h_high[y]);
        let mut trimmed = vec![0i32; width];
        let n = width.min(row.len());
        trimmed[..n].copy_from_slice(&row[..n]);
        output.push(trimmed);
    }
    output
}

/// Flatten a 2D array to row-major 1D, clamping to (w, h).
fn flatten_2d(data: &[Vec<i32>], w: usize, h: usize) -> Vec<i32> {
    let mut out = vec![0i32; w * h];
    for y in 0..h.min(data.len()) {
        let n = w.min(data[y].len());
        out[y * w..y * w + n].copy_from_slice(&data[y][..n]);
    }
    out
}

/// Unflatten row-major 1D data to a 2D array.
fn unflatten_2d(data: &[i32], w: usize, h: usize) -> Vec<Vec<i32>> {
    let mut out = Vec::with_capacity(h);
    for y in 0..h {
        let start = y * w;
        let end = (start + w).min(data.len());
        let mut row = vec![0i32; w];
        if start < data.len() {
            let n = end - start;
            row[..n].copy_from_slice(&data[start..end]);
        }
        out.push(row);
    }
    out
}
