//! Codestream-local structures.
//!
//! Port of `ojph_codestream_local.h/cpp`. Contains the internal codestream
//! state machine that manages tiles, memory allocation, and I/O.

use crate::types::*;
use crate::error::{OjphError, Result};
use crate::file::{OutfileBase, InfileBase};
use crate::params::local::*;
use crate::mem::LineBuf;

use super::tile::Tile;

/// Internal codestream state.
///
/// This is the workhorse struct that holds all the codec state: SIZ, COD, QCD,
/// CAP, NLT, TLM, DFS parameters, the tile array, and I/O state.
#[derive(Debug)]
pub struct CodestreamLocal {
    // Parameter marker segments
    pub(crate) siz: ParamSiz,
    pub(crate) cod: ParamCod,
    pub(crate) qcd: ParamQcd,
    pub(crate) cap: ParamCap,
    pub(crate) nlt: ParamNlt,
    pub(crate) tlm: ParamTlm,
    pub(crate) dfs: ParamDfs,

    // Tile grid
    pub(crate) num_tiles: Size,
    pub(crate) tiles: Vec<Tile>,

    // Line exchange
    pub(crate) lines: Vec<LineBuf>,
    pub(crate) num_comps: u32,
    pub(crate) comp_size: Vec<Size>,
    pub(crate) recon_comp_size: Vec<Size>,
    pub(crate) employ_color_transform: bool,

    // Encode/decode state
    pub(crate) cur_line: u32,
    pub(crate) cur_comp: u32,
    pub(crate) cur_tile_row: u32,
    pub(crate) resilient: bool,
    pub(crate) skipped_res_for_read: u32,
    pub(crate) skipped_res_for_recon: u32,

    // Profile and settings
    pub(crate) planar: i32,
    pub(crate) profile: i32,
    pub(crate) tilepart_div: u32,
    pub(crate) need_tlm: bool,
}

impl Default for CodestreamLocal {
    fn default() -> Self {
        Self {
            siz: ParamSiz::default(),
            cod: ParamCod::default(),
            qcd: ParamQcd::default(),
            cap: ParamCap::default(),
            nlt: ParamNlt::default(),
            tlm: ParamTlm::default(),
            dfs: ParamDfs::default(),
            num_tiles: Size::new(0, 0),
            tiles: Vec::new(),
            lines: Vec::new(),
            num_comps: 0,
            comp_size: Vec::new(),
            recon_comp_size: Vec::new(),
            employ_color_transform: false,
            cur_line: 0,
            cur_comp: 0,
            cur_tile_row: 0,
            resilient: false,
            skipped_res_for_read: 0,
            skipped_res_for_recon: 0,
            planar: -1,
            profile: 0, // UNDEFINED
            tilepart_div: TILEPART_NO_DIVISIONS,
            need_tlm: false,
        }
    }
}

impl CodestreamLocal {
    /// Create a new codestream with default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all state for reuse.
    pub fn restart(&mut self) {
        *self = Self::default();
    }

    // ----- Accessors -----

    pub fn access_siz(&self) -> &ParamSiz { &self.siz }
    pub fn access_siz_mut(&mut self) -> &mut ParamSiz { &mut self.siz }
    pub fn access_cod(&self) -> &ParamCod { &self.cod }
    pub fn access_cod_mut(&mut self) -> &mut ParamCod { &mut self.cod }
    pub fn access_qcd(&self) -> &ParamQcd { &self.qcd }
    pub fn access_qcd_mut(&mut self) -> &mut ParamQcd { &mut self.qcd }
    pub fn access_nlt(&self) -> &ParamNlt { &self.nlt }
    pub fn access_nlt_mut(&mut self) -> &mut ParamNlt { &mut self.nlt }
    pub fn is_planar(&self) -> bool { self.planar != 0 }
    pub fn is_resilient(&self) -> bool { self.resilient }

    // ----- Configuration -----

    pub fn set_planar(&mut self, planar: i32) {
        self.planar = planar;
    }

    pub fn set_profile(&mut self, s: &str) -> Result<()> {
        match ProfileNum::from_str(s) {
            Some(p) => { self.profile = p as i32; Ok(()) }
            None => Err(OjphError::InvalidParam(format!("unknown profile: {}", s))),
        }
    }

    pub fn set_tilepart_divisions(&mut self, value: u32) {
        self.tilepart_div = value;
    }

    pub fn request_tlm_marker(&mut self, needed: bool) {
        self.need_tlm = needed;
    }

    pub fn enable_resilience(&mut self) {
        self.resilient = true;
    }

    pub fn restrict_input_resolution(&mut self, skipped_for_data: u32, skipped_for_recon: u32) {
        self.skipped_res_for_read = skipped_for_data;
        self.skipped_res_for_recon = skipped_for_recon;
        self.siz.set_skipped_resolutions(skipped_for_recon);
    }

    // ----- Tile grid computation -----

    /// Compute the tile grid from SIZ parameters.
    fn compute_tile_grid(&mut self) -> Result<()> {
        let ext = self.siz.get_image_extent();
        let toff = self.siz.get_tile_offset();
        let tsiz = self.siz.get_tile_size();

        self.num_tiles.w = div_ceil(ext.x - toff.x, tsiz.w);
        self.num_tiles.h = div_ceil(ext.y - toff.y, tsiz.h);

        if self.num_tiles.area() > 65535 {
            return Err(OjphError::Codec {
                code: 0x00030011,
                message: "the number of tiles cannot exceed 65535".into(),
            });
        }
        if self.num_tiles.area() == 0 {
            return Err(OjphError::Codec {
                code: 0x00030012,
                message: "the number of tiles cannot be 0".into(),
            });
        }
        Ok(())
    }

    // ----- Write path -----

    /// Write codestream headers to the output file.
    pub fn write_headers(
        &mut self,
        file: &mut dyn OutfileBase,
        comments: &[CommentExchange],
    ) -> Result<()> {
        // Validate parameters
        self.siz.check_validity()?;
        self.cod.check_validity(&self.siz)?;
        self.qcd.check_validity(&self.siz, &self.cod)?;
        self.cap.check_validity(&self.cod, &self.qcd);

        // Compute tile grid
        self.compute_tile_grid()?;

        // Write SOC
        file.write(&markers::SOC.to_be_bytes())?;

        // Write SIZ
        self.siz.write(file)?;

        // Write CAP
        self.cap.write(file)?;

        // Write COD
        self.cod.write(file)?;

        // Write COC segments
        let nc = self.siz.get_num_components() as u32;
        self.cod.write_coc(file, nc)?;

        // Write QCD
        self.qcd.write(file)?;

        // Write QCC segments
        self.qcd.write_qcc(file, nc)?;

        // Write NLT (if any)
        self.nlt.write(file)?;

        // Write comments
        for comment in comments {
            write_comment(file, comment)?;
        }

        // Build tiles
        self.build_tiles()?;

        // Write TLM if needed
        if self.need_tlm {
            let total_tps: u32 = self.tiles.iter()
                .map(|t| t.compute_num_tileparts(self.tilepart_div))
                .sum();
            self.tlm.init(total_tps);
        }
        if self.need_tlm {
            self.tlm.write(file)?;
        }

        Ok(())
    }

    // ----- Read path -----

    /// Read codestream headers from the input file.
    pub fn read_headers(&mut self, file: &mut dyn InfileBase) -> Result<()> {
        // Read SOC marker
        let mut marker_buf = [0u8; 2];
        file.read(&mut marker_buf)?;
        let marker = u16::from_be_bytes(marker_buf);
        if marker != markers::SOC {
            return Err(OjphError::Codec {
                code: 0x00030001,
                message: format!("Expected SOC marker (0xFF4F), got 0x{:04X}", marker),
            });
        }

        // Read remaining header markers
        loop {
            file.read(&mut marker_buf)?;
            let marker = u16::from_be_bytes(marker_buf);

            match marker {
                markers::SIZ => {
                    self.siz.read(file)?;
                }
                markers::CAP => {
                    self.cap.read(file)?;
                }
                markers::COD => {
                    self.cod.read(file)?;
                }
                markers::COC => {
                    let nc = self.siz.get_num_components() as u32;
                    let mut coc = ParamCod::default();
                    coc.read_coc(file, nc)?;
                    self.cod.children.push(coc);
                }
                markers::QCD => {
                    self.qcd.read(file)?;
                }
                markers::QCC => {
                    let nc = self.siz.get_num_components() as u32;
                    let mut qcc = ParamQcd::default();
                    qcc.read_qcc(file, nc)?;
                    self.qcd.children.push(qcc);
                }
                markers::NLT => {
                    self.nlt.read(file)?;
                }
                markers::COM => {
                    skip_marker_segment(file)?;
                }
                markers::TLM => {
                    skip_marker_segment(file)?;
                }
                markers::DFS => {
                    self.dfs.read(file)?;
                }
                markers::ATK => {
                    skip_marker_segment(file)?;
                }
                markers::SOT => {
                    // SOT signals start of tile data - stop reading main header
                    // Push the SOT marker back by seeking (handled at higher level)
                    break;
                }
                _ => {
                    // Unknown marker — skip it
                    if marker >= 0xFF30 && marker <= 0xFF3F {
                        // Zero-length marker
                        continue;
                    }
                    skip_marker_segment(file)?;
                }
            }
        }

        // Compute tile grid
        self.siz.set_skipped_resolutions(self.skipped_res_for_recon);
        self.compute_tile_grid()?;
        self.build_tiles()?;

        Ok(())
    }

    /// Build tile structures from current parameters.
    fn build_tiles(&mut self) -> Result<()> {
        let ext = self.siz.get_image_extent();
        let ioff = self.siz.get_image_offset();
        let toff = self.siz.get_tile_offset();
        let tsiz = self.siz.get_tile_size();

        let mut tiles = Vec::with_capacity(self.num_tiles.area() as usize);

        for ty in 0..self.num_tiles.h {
            let y0 = toff.y + ty * tsiz.h;
            let y1 = y0 + tsiz.h;
            let tile_org_y = y0.max(ioff.y);
            let tile_h = y1.min(ext.y) - tile_org_y;

            for tx in 0..self.num_tiles.w {
                let x0 = toff.x + tx * tsiz.w;
                let x1 = x0 + tsiz.w;
                let tile_org_x = x0.max(ioff.x);
                let tile_w = x1.min(ext.x) - tile_org_x;

                let tile_rect = Rect::new(
                    Point::new(tile_org_x, tile_org_y),
                    Size::new(tile_w, tile_h),
                );

                let idx = ty * self.num_tiles.w + tx;
                tiles.push(Tile::new(
                    idx,
                    tile_rect,
                    &self.siz,
                    &self.cod,
                    &self.qcd,
                    self.skipped_res_for_recon,
                ));
            }
        }

        self.tiles = tiles;

        // Set up line buffers and component sizes
        self.num_comps = self.siz.get_num_components() as u32;
        self.lines = (0..self.num_comps).map(|_| LineBuf::new()).collect();
        self.comp_size = (0..self.num_comps).map(|c| {
            Size::new(self.siz.get_width(c), self.siz.get_height(c))
        }).collect();
        self.recon_comp_size = (0..self.num_comps).map(|c| {
            Size::new(self.siz.get_recon_width(c), self.siz.get_recon_height(c))
        }).collect();
        self.employ_color_transform = self.cod.is_employing_color_transform();

        self.cur_comp = 0;
        self.cur_line = 0;
        self.cur_tile_row = 0;

        Ok(())
    }
}

// =========================================================================
// Helper functions
// =========================================================================

/// Write a COM (comment) marker segment.
fn write_comment(file: &mut dyn OutfileBase, comment: &CommentExchange) -> Result<()> {
    let data_len = comment.data.len() as u16;
    let lcom = data_len + 4;
    file.write(&markers::COM.to_be_bytes())?;
    file.write(&lcom.to_be_bytes())?;
    file.write(&comment.rcom.to_be_bytes())?;
    file.write(&comment.data)?;
    Ok(())
}

/// Skip a marker segment by reading its length and consuming that many bytes.
fn skip_marker_segment(file: &mut dyn InfileBase) -> Result<()> {
    let mut lbuf = [0u8; 2];
    if file.read(&mut lbuf)? != 2 {
        return Err(OjphError::Codec {
            code: 0x00030099,
            message: "unexpected EOF reading marker segment length".into(),
        });
    }
    let length = u16::from_be_bytes(lbuf) as usize;
    if length < 2 {
        return Err(OjphError::Codec {
            code: 0x0003009A,
            message: "marker segment length too small".into(),
        });
    }
    let skip = length - 2;
    let mut buf = vec![0u8; skip.min(4096)];
    let mut remaining = skip;
    while remaining > 0 {
        let to_read = remaining.min(buf.len());
        let n = file.read(&mut buf[..to_read])?;
        if n == 0 {
            return Err(OjphError::Codec {
                code: 0x0003009B,
                message: "unexpected EOF skipping marker segment".into(),
            });
        }
        remaining -= n;
    }
    Ok(())
}
