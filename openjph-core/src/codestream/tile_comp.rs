//! Tile component processing.
//!
//! Port of `ojph_tile_comp.h/cpp`. A tile component represents one
//! color component within a tile.

use super::resolution::Resolution;
use crate::error::Result;
use crate::types::*;

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
                Point::new(div_ceil(comp_rect.org.x, ds), div_ceil(comp_rect.org.y, ds)),
                Size::new(
                    div_ceil(comp_rect.org.x + comp_rect.siz.w, ds) - div_ceil(comp_rect.org.x, ds),
                    div_ceil(comp_rect.org.y + comp_rect.siz.h, ds) - div_ceil(comp_rect.org.y, ds),
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
                if y >= h {
                    break;
                }
                let n = w.min(row.len());
                coeffs[y * w..y * w + n].copy_from_slice(&row[..n]);
            }
            self.resolutions[0].subbands[0].coeffs = coeffs;
            return Ok(());
        }

        if self.reversible {
            analyze_resolution_reversible(&self.lines, &mut self.resolutions, num_decomps as usize);
            return Ok(());
        }

        // Multi-level DWT: apply iteratively
        let mut current: Vec<Vec<i32>> = self.lines.clone();
        for d in 0..num_decomps {
            let res_idx = (num_decomps - d) as usize; // highest resolution first
            let res_org = self.resolutions[res_idx].res_rect.org;
            let (ll, hl, lh, hh) = dwt53_forward_2d(&current, res_org.x, res_org.y);

            // Store detail subbands at this resolution level
            let res = &mut self.resolutions[res_idx];
            let sb_w = res.subbands[0].width() as usize;
            let sb_h = res.subbands[0].height() as usize;
            res.subbands[0].coeffs = flatten_2d(&hl, sb_w, sb_h);
            res.subbands[1].coeffs = flatten_2d(
                &lh,
                res.subbands[1].width() as usize,
                res.subbands[1].height() as usize,
            );
            res.subbands[2].coeffs = flatten_2d(
                &hh,
                res.subbands[2].width() as usize,
                res.subbands[2].height() as usize,
            );
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

        if self.reversible {
            self.decoded_lines =
                synthesize_resolution_reversible(&self.resolutions, num_decomps as usize);
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
            let res_org = res.res_rect.org;
            let hl = unflatten_2d(
                &res.subbands[0].coeffs,
                res.subbands[0].width() as usize,
                res.subbands[0].height() as usize,
            );
            let lh = unflatten_2d(
                &res.subbands[1].coeffs,
                res.subbands[1].width() as usize,
                res.subbands[1].height() as usize,
            );
            let hh = unflatten_2d(
                &res.subbands[2].coeffs,
                res.subbands[2].width() as usize,
                res.subbands[2].height() as usize,
            );

            let out_w = if res_idx == num_decomps as usize {
                w
            } else {
                self.resolutions[res_idx].res_rect.siz.w as usize
            };
            let out_h = if res_idx == num_decomps as usize {
                h
            } else {
                self.resolutions[res_idx].res_rect.siz.h as usize
            };
            current = dwt53_inverse_2d(&current, &hl, &lh, &hh, out_w, out_h, res_org.x, res_org.y);
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

fn make_i32_linebuf(data: &mut [i32]) -> crate::mem::LineBuf {
    crate::mem::LineBuf {
        size: data.len() - 1,
        pre_size: 1,
        flags: crate::mem::LFT_32BIT | crate::mem::LFT_INTEGER,
        data: crate::mem::LineBufData::I32(data[1..].as_mut_ptr()),
    }
}

fn rev53_atk() -> &'static crate::transform::ParamAtk {
    static ATK: std::sync::OnceLock<crate::transform::ParamAtk> = std::sync::OnceLock::new();
    ATK.get_or_init(|| {
        let mut atk = crate::transform::ParamAtk::default();
        atk.init_rev53();
        atk
    })
}

/// Forward 1D 5/3 DWT using the same wavelet kernel implementation as OpenJPH.
fn dwt53_forward_1d(input: &[i32], origin: u32) -> (Vec<i32>, Vec<i32>) {
    let width = input.len() as u32;
    if width == 0 {
        return (vec![], vec![]);
    }

    let even = (origin & 1) == 0;
    let low_len = ((width + if even { 1 } else { 0 }) / 2) as usize;
    let high_len = ((width + if even { 0 } else { 1 }) / 2) as usize;

    let mut src_data = vec![0i32; width as usize + 2];
    src_data[1..1 + input.len()].copy_from_slice(input);
    let src = make_i32_linebuf(&mut src_data);

    let mut low_data = vec![0i32; low_len + 2];
    let mut high_data = vec![0i32; high_len + 2];
    let mut low = make_i32_linebuf(&mut low_data);
    let mut high = make_i32_linebuf(&mut high_data);

    crate::transform::wavelet::gen_rev_horz_ana(
        rev53_atk(),
        &mut low,
        &mut high,
        &src,
        width,
        even,
    );

    (
        low_data[1..1 + low_len].to_vec(),
        high_data[1..1 + high_len].to_vec(),
    )
}

/// Inverse 1D 5/3 DWT using the same wavelet kernel implementation as OpenJPH.
fn dwt53_inverse_1d(low: &[i32], high: &[i32], origin: u32) -> Vec<i32> {
    let width = (low.len() + high.len()) as u32;
    if width == 0 {
        return vec![];
    }
    let even = (origin & 1) == 0;

    let mut low_data = vec![0i32; low.len() + 2];
    let mut high_data = vec![0i32; high.len() + 2];
    low_data[1..1 + low.len()].copy_from_slice(low);
    high_data[1..1 + high.len()].copy_from_slice(high);

    let mut low_buf = make_i32_linebuf(&mut low_data);
    let mut high_buf = make_i32_linebuf(&mut high_data);
    let mut dst_data = vec![0i32; width as usize + 2];
    let mut dst = make_i32_linebuf(&mut dst_data);

    crate::transform::wavelet::gen_rev_horz_syn(
        rev53_atk(),
        &mut dst,
        &mut low_buf,
        &mut high_buf,
        width,
        even,
    );

    dst_data[1..1 + width as usize].to_vec()
}

/// Forward 2D 5/3 DWT (one decomposition level).
/// `org_x` / `org_y` are the absolute origin of the signal (for parity).
/// Returns (ll, hl, lh, hh) as 2D arrays.
fn dwt53_forward_2d(
    data: &[Vec<i32>],
    org_x: u32,
    org_y: u32,
) -> (Vec<Vec<i32>>, Vec<Vec<i32>>, Vec<Vec<i32>>, Vec<Vec<i32>>) {
    let height = data.len();
    let width = if height > 0 { data[0].len() } else { 0 };

    if height == 0 || width == 0 {
        return (vec![], vec![], vec![], vec![]);
    }

    let odd_x = (org_x & 1) != 0;
    let odd_y = (org_y & 1) != 0;
    let low_w = if odd_x { width / 2 } else { (width + 1) / 2 };
    let high_w = if odd_x { (width + 1) / 2 } else { width / 2 };
    let low_h = if odd_y { height / 2 } else { (height + 1) / 2 };
    let high_h = if odd_y { (height + 1) / 2 } else { height / 2 };

    // Step 1: Horizontal DWT on each row
    let mut h_low = Vec::with_capacity(height);
    let mut h_high = Vec::with_capacity(height);
    for row in data {
        let (l, h) = dwt53_forward_1d(row, org_x);
        h_low.push(l);
        h_high.push(h);
    }

    // Step 2: Vertical DWT on columns of h_low → LL, LH
    let mut ll = vec![vec![0i32; low_w]; low_h];
    let mut lh = vec![vec![0i32; low_w]; high_h];
    for x in 0..low_w {
        let col: Vec<i32> = h_low
            .iter()
            .map(|row| if x < row.len() { row[x] } else { 0 })
            .collect();
        let (s, d) = dwt53_forward_1d(&col, org_y);
        for y in 0..low_h.min(s.len()) {
            ll[y][x] = s[y];
        }
        for y in 0..high_h.min(d.len()) {
            lh[y][x] = d[y];
        }
    }

    // Step 3: Vertical DWT on columns of h_high → HL, HH
    let mut hl = vec![vec![0i32; high_w]; low_h];
    let mut hh = vec![vec![0i32; high_w]; high_h];
    for x in 0..high_w {
        let col: Vec<i32> = h_high
            .iter()
            .map(|row| if x < row.len() { row[x] } else { 0 })
            .collect();
        let (s, d) = dwt53_forward_1d(&col, org_y);
        for y in 0..low_h.min(s.len()) {
            hl[y][x] = s[y];
        }
        for y in 0..high_h.min(d.len()) {
            hh[y][x] = d[y];
        }
    }

    (ll, hl, lh, hh)
}

/// Inverse 2D 5/3 DWT — reconstructs image from subbands.
/// `org_x` / `org_y` are the absolute origin of the output signal.
fn dwt53_inverse_2d(
    ll: &[Vec<i32>],
    hl: &[Vec<i32>],
    lh: &[Vec<i32>],
    hh: &[Vec<i32>],
    width: usize,
    height: usize,
    org_x: u32,
    org_y: u32,
) -> Vec<Vec<i32>> {
    if width == 0 || height == 0 {
        return vec![];
    }

    let odd_x = (org_x & 1) != 0;
    let low_w = if odd_x { width / 2 } else { (width + 1) / 2 };
    let high_w = if odd_x { (width + 1) / 2 } else { width / 2 };

    // Step 1: Inverse vertical DWT on (LL, LH) → h_low columns
    let mut h_low = vec![vec![0i32; low_w]; height];
    for x in 0..low_w {
        let low_col: Vec<i32> = ll
            .iter()
            .map(|row| if x < row.len() { row[x] } else { 0 })
            .collect();
        let high_col: Vec<i32> = lh
            .iter()
            .map(|row| if x < row.len() { row[x] } else { 0 })
            .collect();
        let col = dwt53_inverse_1d(&low_col, &high_col, org_y);
        for y in 0..height.min(col.len()) {
            h_low[y][x] = col[y];
        }
    }

    // Step 2: Inverse vertical DWT on (HL, HH) → h_high columns
    let mut h_high = vec![vec![0i32; high_w]; height];
    for x in 0..high_w {
        let low_col: Vec<i32> = hl
            .iter()
            .map(|row| if x < row.len() { row[x] } else { 0 })
            .collect();
        let high_col: Vec<i32> = hh
            .iter()
            .map(|row| if x < row.len() { row[x] } else { 0 })
            .collect();
        let col = dwt53_inverse_1d(&low_col, &high_col, org_y);
        for y in 0..height.min(col.len()) {
            h_high[y][x] = col[y];
        }
    }

    // Step 3: Inverse horizontal DWT → output rows
    let mut output = Vec::with_capacity(height);
    for y in 0..height {
        let row = dwt53_inverse_1d(&h_low[y], &h_high[y], org_x);
        let mut trimmed = vec![0i32; width];
        let n = width.min(row.len());
        trimmed[..n].copy_from_slice(&row[..n]);
        output.push(trimmed);
    }
    output
}

fn wrap_i32_line(data: &[i32]) -> crate::mem::LineBuf {
    crate::mem::LineBuf {
        size: data.len(),
        pre_size: 0,
        flags: crate::mem::LFT_32BIT | crate::mem::LFT_INTEGER,
        data: crate::mem::LineBufData::I32(data.as_ptr() as *mut i32),
    }
}

fn wrap_i32_line_mut(data: &mut [i32]) -> crate::mem::LineBuf {
    crate::mem::LineBuf {
        size: data.len(),
        pre_size: 0,
        flags: crate::mem::LFT_32BIT | crate::mem::LFT_INTEGER,
        data: crate::mem::LineBufData::I32(data.as_mut_ptr()),
    }
}

fn apply_rev_vert_step(
    step: &crate::transform::LiftingStep,
    sig: &[i32],
    other: &[i32],
    aug: &mut [i32],
    synthesis: bool,
) {
    let sig_buf = wrap_i32_line(sig);
    let other_buf = wrap_i32_line(other);
    let mut aug_buf = wrap_i32_line_mut(aug);
    crate::transform::wavelet::gen_rev_vert_step(
        step,
        &sig_buf,
        &other_buf,
        &mut aug_buf,
        aug.len() as u32,
        synthesis,
    );
}

fn analyze_resolution_reversible(lines: &[Vec<i32>], resolutions: &mut [Resolution], res_idx: usize) {
    if res_idx == 0 {
        let sb = &mut resolutions[0].subbands[0];
        sb.coeffs = flatten_2d(lines, sb.width() as usize, sb.height() as usize);
        return;
    }

    let (lower_resolutions, current_and_higher) = resolutions.split_at_mut(res_idx);
    let res = &mut current_and_higher[0];
    let width = res.res_rect.siz.w as usize;
    let height = res.res_rect.siz.h as usize;
    if width == 0 || height == 0 {
        analyze_resolution_reversible(&[], lower_resolutions, res_idx - 1);
        for sb in &mut res.subbands {
            sb.coeffs.clear();
        }
        return;
    }

    let atk = rev53_atk();
    let num_steps = atk.get_num_steps() as usize;
    let horz_origin = res.res_rect.org.x;
    let mut vert_even = (res.res_rect.org.y & 1) == 0;
    let mut cur_line = 0usize;
    let mut rows_to_produce = height;

    let mut ssp = vec![None::<Vec<i32>>; num_steps];
    let mut sig = None::<Vec<i32>>;
    let mut aug = None::<Vec<i32>>;

    let mut child_lines = Vec::new();
    let mut band1_lines = Vec::new();
    let mut band2_lines = Vec::new();
    let mut band3_lines = Vec::new();

    if height == 1 {
        let line = lines.get(0).cloned().unwrap_or_else(|| vec![0; width]);
        if vert_even {
            let (low, high) = dwt53_forward_1d(&line, horz_origin);
            child_lines.push(low);
            band1_lines.push(high);
        } else {
            let doubled: Vec<i32> = line.into_iter().map(|v| v << 1).collect();
            let (low, high) = dwt53_forward_1d(&doubled, horz_origin);
            band2_lines.push(low);
            band3_lines.push(high);
        }
    } else {
    while cur_line < height {
        let line = lines.get(cur_line).cloned().unwrap_or_else(|| vec![0; width]);
        if vert_even {
            sig = Some(line);
        } else {
            aug = Some(line);
        }
        cur_line += 1;

        if !vert_even && cur_line < height {
            vert_even = !vert_even;
            continue;
        }

        loop {
            for i in 0..num_steps {
                if let Some(ref mut aug_line) = aug {
                    if sig.is_some() || ssp[i].is_some() {
                        let sp1 = sig.as_ref().or(ssp[i].as_ref()).unwrap();
                        let sp2 = ssp[i].as_ref().or(sig.as_ref()).unwrap();
                        let step = atk.get_step((num_steps - i - 1) as u32);
                        apply_rev_vert_step(step, sp1, sp2, aug_line, false);
                    }
                }

                let t = aug.take();
                aug = ssp[i].take();
                ssp[i] = sig.take();
                sig = t;
            }

            if let Some(line) = aug.take() {
                let (low, high) = dwt53_forward_1d(&line, horz_origin);
                band2_lines.push(low);
                band3_lines.push(high);
                rows_to_produce -= 1;
            }
            if let Some(line) = sig.take() {
                let (low, high) = dwt53_forward_1d(&line, horz_origin);
                child_lines.push(low);
                band1_lines.push(high);
                rows_to_produce -= 1;
            }

            vert_even = !vert_even;
            if !(cur_line >= height && rows_to_produce > 0) {
                break;
            }
        }
    }
    }

    res.subbands[0].coeffs = flatten_2d(
        &band1_lines,
        res.subbands[0].width() as usize,
        res.subbands[0].height() as usize,
    );
    res.subbands[1].coeffs = flatten_2d(
        &band2_lines,
        res.subbands[1].width() as usize,
        res.subbands[1].height() as usize,
    );
    res.subbands[2].coeffs = flatten_2d(
        &band3_lines,
        res.subbands[2].width() as usize,
        res.subbands[2].height() as usize,
    );

    analyze_resolution_reversible(&child_lines, lower_resolutions, res_idx - 1);
}

fn synthesize_resolution_reversible(resolutions: &[Resolution], res_idx: usize) -> Vec<Vec<i32>> {
    if res_idx == 0 {
        let sb = &resolutions[0].subbands[0];
        return unflatten_2d(&sb.coeffs, sb.width() as usize, sb.height() as usize);
    }

    let res = &resolutions[res_idx];
    let width = res.res_rect.siz.w as usize;
    let height = res.res_rect.siz.h as usize;
    if width == 0 || height == 0 {
        return vec![];
    }

    let child_lines = synthesize_resolution_reversible(resolutions, res_idx - 1);
    let band1_lines = unflatten_2d(
        &res.subbands[0].coeffs,
        res.subbands[0].width() as usize,
        res.subbands[0].height() as usize,
    );
    let band2_lines = unflatten_2d(
        &res.subbands[1].coeffs,
        res.subbands[1].width() as usize,
        res.subbands[1].height() as usize,
    );
    let band3_lines = unflatten_2d(
        &res.subbands[2].coeffs,
        res.subbands[2].width() as usize,
        res.subbands[2].height() as usize,
    );

    let horz_origin = res.res_rect.org.x;
    let vert_even_init = (res.res_rect.org.y & 1) == 0;
    let atk = rev53_atk();
    let num_steps = atk.get_num_steps() as usize;

    if height == 1 {
        let mut line = if vert_even_init {
            let child = child_lines.first().map_or(&[][..], |v| v.as_slice());
            let band1 = band1_lines.first().map_or(&[][..], |v| v.as_slice());
            dwt53_inverse_1d(child, band1, horz_origin)
        } else {
            let band2 = band2_lines.first().map_or(&[][..], |v| v.as_slice());
            let band3 = band3_lines.first().map_or(&[][..], |v| v.as_slice());
            let mut line = dwt53_inverse_1d(band2, band3, horz_origin);
            for v in &mut line {
                *v >>= 1;
            }
            line
        };
        line.resize(width, 0);
        return vec![line];
    }

    let mut child_idx = 0usize;
    let mut band1_idx = 0usize;
    let mut band2_idx = 0usize;
    let mut band3_idx = 0usize;
    let mut cur_line = 0usize;
    let mut vert_even = vert_even_init;
    let mut ssp = vec![None::<Vec<i32>>; num_steps];
    let mut sig: Option<Vec<i32>> = None;
    let mut aug: Option<Vec<i32>> = None;
    let mut output = Vec::with_capacity(height);

    while output.len() < height {
        if let Some(line) = sig.take() {
            output.push(line);
            continue;
        }

        loop {
            if cur_line < height {
                if vert_even {
                    let child = child_lines.get(child_idx).map_or(&[][..], |v| v.as_slice());
                    let band1 = band1_lines.get(band1_idx).map_or(&[][..], |v| v.as_slice());
                    aug = Some(dwt53_inverse_1d(child, band1, horz_origin));
                    child_idx += 1;
                    band1_idx += 1;
                    vert_even = !vert_even;
                    cur_line += 1;
                    continue;
                } else {
                    let band2 = band2_lines.get(band2_idx).map_or(&[][..], |v| v.as_slice());
                    let band3 = band3_lines.get(band3_idx).map_or(&[][..], |v| v.as_slice());
                    sig = Some(dwt53_inverse_1d(band2, band3, horz_origin));
                    band2_idx += 1;
                    band3_idx += 1;
                    vert_even = !vert_even;
                    cur_line += 1;
                }
            }

            for i in 0..num_steps {
                if let Some(ref mut aug_line) = aug {
                    if sig.is_some() || ssp[i].is_some() {
                        let sp1 = sig.as_ref().or(ssp[i].as_ref()).unwrap();
                        let sp2 = ssp[i].as_ref().or(sig.as_ref()).unwrap();
                        apply_rev_vert_step(atk.get_step(i as u32), sp1, sp2, aug_line, true);
                    }
                }

                let t = aug.take();
                aug = ssp[i].take();
                ssp[i] = sig.take();
                sig = t;
            }

            if let Some(line) = aug.take() {
                output.push(line);
                break;
            }
            if let Some(line) = sig.take() {
                output.push(line);
                break;
            }
        }
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

#[cfg(test)]
mod tests {
    use super::{dwt53_forward_1d, dwt53_inverse_1d};
    use crate::mem::{LineBuf, LineBufData, LFT_32BIT, LFT_INTEGER};
    use crate::transform::{wavelet, ParamAtk};

    fn make_i32_buf(data: &mut [i32]) -> LineBuf {
        LineBuf {
            size: data.len() - 1,
            pre_size: 1,
            flags: LFT_32BIT | LFT_INTEGER,
            data: LineBufData::I32(data[1..].as_mut_ptr()),
        }
    }

    fn wavelet_forward(input: &[i32], origin: u32) -> (Vec<i32>, Vec<i32>) {
        let even = (origin & 1) == 0;
        let width = input.len() as u32;
        let l_width = ((width + if even { 1 } else { 0 }) / 2) as usize;
        let h_width = ((width + if even { 0 } else { 1 }) / 2) as usize;

        let mut atk = ParamAtk::default();
        atk.init_rev53();

        let mut src_data = vec![0i32; width as usize + 2];
        src_data[1..1 + input.len()].copy_from_slice(input);
        let src = make_i32_buf(&mut src_data);

        let mut low_data = vec![0i32; l_width + 2];
        let mut high_data = vec![0i32; h_width + 2];
        let mut low = make_i32_buf(&mut low_data);
        let mut high = make_i32_buf(&mut high_data);

        wavelet::gen_rev_horz_ana(&atk, &mut low, &mut high, &src, width, even);

        (
            low_data[1..1 + l_width].to_vec(),
            high_data[1..1 + h_width].to_vec(),
        )
    }

    fn wavelet_inverse(low: &[i32], high: &[i32], origin: u32) -> Vec<i32> {
        let even = (origin & 1) == 0;
        let width = (low.len() + high.len()) as u32;

        let mut atk = ParamAtk::default();
        atk.init_rev53();

        let mut low_data = vec![0i32; low.len() + 2];
        let mut high_data = vec![0i32; high.len() + 2];
        low_data[1..1 + low.len()].copy_from_slice(low);
        high_data[1..1 + high.len()].copy_from_slice(high);

        let mut low_buf = make_i32_buf(&mut low_data);
        let mut high_buf = make_i32_buf(&mut high_data);
        let mut dst_data = vec![0i32; width as usize + 2];
        let mut dst = make_i32_buf(&mut dst_data);

        wavelet::gen_rev_horz_syn(&atk, &mut dst, &mut low_buf, &mut high_buf, width, even);

        dst_data[1..1 + width as usize].to_vec()
    }

    #[test]
    fn tile_comp_forward_matches_wavelet_reference() {
        let samples: &[(&[i32], u32)] = &[
            (&[42], 0),
            (&[42], 1),
            (&[10, 20], 0),
            (&[10, 20], 1),
            (&[3, 7, 11, 15, 19], 0),
            (&[3, 7, 11, 15, 19], 1),
            (&[5, -4, 9, -2, 7, 1, -8], 0),
            (&[5, -4, 9, -2, 7, 1, -8], 1),
        ];

        for (input, origin) in samples {
            let (low, high) = dwt53_forward_1d(input, *origin);
            let (ref_low, ref_high) = wavelet_forward(input, *origin);
            assert_eq!(
                low, ref_low,
                "low mismatch for input={input:?} origin={origin}"
            );
            assert_eq!(
                high, ref_high,
                "high mismatch for input={input:?} origin={origin}"
            );
        }
    }

    #[test]
    fn tile_comp_inverse_matches_wavelet_reference() {
        let samples: &[(&[i32], u32)] = &[
            (&[42], 0),
            (&[42], 1),
            (&[10, 20], 0),
            (&[10, 20], 1),
            (&[3, 7, 11, 15, 19], 0),
            (&[3, 7, 11, 15, 19], 1),
            (&[5, -4, 9, -2, 7, 1, -8], 0),
            (&[5, -4, 9, -2, 7, 1, -8], 1),
        ];

        for (input, origin) in samples {
            let (low, high) = wavelet_forward(input, *origin);
            let output = dwt53_inverse_1d(&low, &high, *origin);
            let ref_output = wavelet_inverse(&low, &high, *origin);
            assert_eq!(
                output, ref_output,
                "inverse mismatch for input={input:?} origin={origin}"
            );
        }
    }
}
