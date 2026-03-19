//! Tile processing.
//!
//! Port of `ojph_tile.h/cpp`. A tile is the top-level subdivision of the
//! image. Each tile contains tile-components.

use crate::types::*;
use crate::error::{OjphError, Result};
use crate::file::{OutfileBase, InfileBase};
use crate::params::{ParamSiz, ParamCod, ParamQcd, ParamSot};
use crate::params::local::markers;
use super::tile_comp::TileComp;
use super::codeblock::CodeblockDecState;
use super::bitbuffer_write::BitBufferWrite;

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

    // ===================================================================
    // Encode path
    // ===================================================================

    /// Push a line of image data to the specified component.
    pub fn push_line(&mut self, line: &[i32], comp_num: u32) -> Result<()> {
        let tc = self.tile_comps.get_mut(comp_num as usize)
            .ok_or_else(|| OjphError::InvalidParam(
                format!("component {} out of range", comp_num)
            ))?;
        tc.push_line(line);
        Ok(())
    }

    /// Encode and write this tile's data to the output file.
    ///
    /// Performs DWT on all components, encodes codeblocks, then writes
    /// SOT + SOD + packet data.
    pub fn encode_and_write(&mut self, file: &mut dyn OutfileBase) -> Result<()> {
        // 1. DWT + codeblock encoding for each component
        for tc in &mut self.tile_comps {
            tc.perform_dwt()?;
            for res in &mut tc.resolutions {
                for sb in &mut res.subbands {
                    sb.encode_codeblocks()?;
                }
            }
        }

        // 2. Build all packet data into a buffer
        let packet_data = self.build_all_packets()?;

        // 3. Write SOT — payload_len is the tile data size AFTER the SOD marker
        self.sot.init(0, self.tile_idx as u16, 0, 1);
        self.sot.write(file, packet_data.len() as u32)?;

        // 4. Write SOD
        file.write(&markers::SOD.to_be_bytes())?;

        // 5. Write packet data
        file.write(&packet_data)?;

        self.is_complete = true;
        Ok(())
    }

    /// Build all packets for this tile in LRCP order.
    fn build_all_packets(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let num_layers = 1u32; // single quality layer
        let max_res = self.tile_comps.iter()
            .map(|tc| tc.num_resolutions)
            .max()
            .unwrap_or(1);

        // LRCP: Layer → Resolution → Component → Position(precinct)
        for _layer in 0..num_layers {
            for r in 0..max_res {
                for c in 0..self.num_comps {
                    let tc = &self.tile_comps[c as usize];
                    if r >= tc.num_resolutions {
                        // Write empty packet for this component at this resolution
                        data.push(0x00);
                        continue;
                    }
                    let res = &tc.resolutions[r as usize];
                    // One packet per precinct. For simplicity, 1 precinct per resolution.
                    let packet = Self::write_packet(res)?;
                    data.extend_from_slice(&packet);
                }
            }
        }
        Ok(data)
    }

    /// Write a single packet (header + body) for one precinct at a resolution.
    fn write_packet(res: &super::resolution::Resolution) -> Result<Vec<u8>> {
        let mut header = BitBufferWrite::new();
        let mut body: Vec<u8> = Vec::new();

        // Check if any codeblock has data
        let has_any_data = res.subbands.iter().any(|sb| {
            sb.codeblocks.iter().any(|cb| {
                cb.enc_state.as_ref().map_or(false, |s| s.has_data)
            })
        });

        if !has_any_data {
            // Empty packet: single 0 bit, then byte-align
            header.write(0, 1);
            header.finalize();
            return Ok(header.into_data());
        }

        // Non-empty packet
        header.write(1, 1);

        for sb in &res.subbands {
            for cb in &sb.codeblocks {
                let enc = match &cb.enc_state {
                    Some(e) => e,
                    None => {
                        // Not included
                        header.write(0, 1); // inclusion = not included
                        continue;
                    }
                };

                if !enc.has_data {
                    header.write(0, 1); // inclusion = not included
                    continue;
                }

                // Inclusion: included at this layer (tag tree value=0)
                // For a single-node tag tree, just write 1
                header.write(1, 1);

                // Zero bitplanes: code missing_msbs as (missing_msbs) zeros + 1
                for _ in 0..enc.missing_msbs {
                    header.write(0, 1);
                }
                header.write(1, 1);

                // Number of coding passes (1 pass = "0")
                header.write(0, 1);

                // Pass length coding with Lblock
                let len = enc.pass1_bytes;
                let lblock = 3u32;
                let bits_needed = if len > 0 { 32 - len.leading_zeros() } else { 1 };
                let delta = if bits_needed > lblock { bits_needed - lblock } else { 0 };

                // Code delta as unary: (delta) ones + 0
                for _ in 0..delta {
                    header.write(1, 1);
                }
                header.write(0, 1);

                // Code length in (lblock + delta) bits
                header.write(len, lblock + delta);

                // Append coded data to body
                let data_len = enc.pass1_bytes as usize;
                if data_len <= cb.coded_data.len() {
                    body.extend_from_slice(&cb.coded_data[..data_len]);
                }
            }
        }

        header.finalize();
        let mut packet = header.into_data();
        packet.extend_from_slice(&body);
        Ok(packet)
    }

    // ===================================================================
    // Decode path
    // ===================================================================

    /// Read tile data from the file (after SOT marker has been consumed).
    /// Expects the file position to be at the SOT payload (after the 0xFF90 marker).
    pub fn read_tile_data(&mut self, file: &mut dyn InfileBase) -> Result<()> {
        // Read SOT payload
        self.sot.read(file, false)?;

        // Compute how many bytes remain (including SOD marker)
        let payload_including_sod = self.sot.get_payload_length();
        if payload_including_sod < 2 {
            return Err(OjphError::Codec {
                code: 0x00060001,
                message: "tile payload too small for SOD marker".into(),
            });
        }

        // Read SOD marker
        let mut marker_buf = [0u8; 2];
        file.read(&mut marker_buf)?;
        let marker = u16::from_be_bytes(marker_buf);
        if marker != markers::SOD {
            return Err(OjphError::Codec {
                code: 0x00060002,
                message: format!("expected SOD marker (0xFF93), got 0x{:04X}", marker),
            });
        }

        // Read tile data
        let data_len = (payload_including_sod - 2) as usize;
        let mut tile_data = vec![0u8; data_len];
        if data_len > 0 {
            let mut read_so_far = 0;
            while read_so_far < data_len {
                let n = file.read(&mut tile_data[read_so_far..])?;
                if n == 0 { break; }
                read_so_far += n;
            }
        }

        // Parse packets from tile data
        self.parse_all_packets(&tile_data)?;

        // Decode codeblocks and perform IDWT
        for tc in &mut self.tile_comps {
            for res in &mut tc.resolutions {
                for sb in &mut res.subbands {
                    sb.decode_codeblocks()?;
                }
            }
            tc.perform_idwt()?;
        }

        self.is_complete = true;
        Ok(())
    }

    /// Parse all packets from a tile data buffer (LRCP order).
    fn parse_all_packets(&mut self, data: &[u8]) -> Result<()> {
        let mut pos = 0usize;
        let num_layers = 1u32;
        let max_res = self.tile_comps.iter()
            .map(|tc| tc.num_resolutions)
            .max()
            .unwrap_or(1);

        for _layer in 0..num_layers {
            for r in 0..max_res {
                for c in 0..self.num_comps {
                    let tc = &self.tile_comps[c as usize];
                    if r >= tc.num_resolutions {
                        // Parse empty packet
                        if pos < data.len() {
                            pos += 1; // skip the 0x00 byte
                        }
                        continue;
                    }

                    let consumed = self.parse_packet(c, r, &data[pos..])?;
                    pos += consumed;
                }
            }
        }
        Ok(())
    }

    /// Parse a single packet from data. Returns number of bytes consumed.
    fn parse_packet(&mut self, comp: u32, res_num: u32, data: &[u8]) -> Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        // Simple bit reader that reads one bit at a time from MSB
        let mut byte_pos = 0usize;
        let mut bit_pos = 8u32; // bits remaining in current byte
        let mut cur_byte = 0u8;
        let mut unstuff = false;

        let mut read_bit = |byte_pos: &mut usize, bit_pos: &mut u32,
                            cur_byte: &mut u8, unstuff: &mut bool| -> u32 {
            if *bit_pos == 0 || *byte_pos == 0 && *bit_pos == 8 {
                if *byte_pos < data.len() {
                    *cur_byte = data[*byte_pos];
                    *byte_pos += 1;
                    *bit_pos = if *unstuff { 7 } else { 8 };
                    *unstuff = *cur_byte == 0xFF;
                } else {
                    return 0;
                }
            }
            *bit_pos -= 1;
            ((*cur_byte >> *bit_pos) & 1) as u32
        };

        let mut read_bits = |n: u32, bp: &mut usize, bitp: &mut u32,
                             cb: &mut u8, us: &mut bool| -> u32 {
            let mut val = 0u32;
            for _ in 0..n {
                val = (val << 1) | read_bit(bp, bitp, cb, us);
            }
            val
        };

        // Non-empty indicator
        let non_empty = read_bit(&mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);
        if non_empty == 0 {
            return Ok(byte_pos);
        }

        // Collect codeblock info for body parsing
        struct CbInfo {
            sb_idx: usize,
            cb_idx: usize,
            pass_len: u32,
            missing_msbs: u32,
            num_passes: u32,
        }
        let mut cb_infos: Vec<CbInfo> = Vec::new();

        let tc = &self.tile_comps[comp as usize];
        let res = &tc.resolutions[res_num as usize];
        let num_subbands = res.subbands.len();

        for sb_idx in 0..num_subbands {
            let sb = &res.subbands[sb_idx];
            for cb_idx in 0..sb.codeblocks.len() {
                let included = read_bit(&mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);
                if included == 0 {
                    continue;
                }

                // Read zero bitplanes (unary: zeros then 1)
                let mut missing_msbs = 0u32;
                loop {
                    let bit = read_bit(&mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);
                    if bit == 1 { break; }
                    missing_msbs += 1;
                }

                // Read number of coding passes
                let pass_bit = read_bit(&mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);
                let num_passes = if pass_bit == 0 {
                    1u32
                } else {
                    let bit2 = read_bit(&mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);
                    if bit2 == 0 { 2 } else {
                        let extra = read_bits(2, &mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);
                        3 + extra
                    }
                };

                // Read pass length: delta_lblock (unary) + length
                let mut lblock = 3u32;
                loop {
                    let bit = read_bit(&mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);
                    if bit == 0 { break; }
                    lblock += 1;
                }

                let pass_len = read_bits(lblock, &mut byte_pos, &mut bit_pos, &mut cur_byte, &mut unstuff);

                cb_infos.push(CbInfo {
                    sb_idx,
                    cb_idx,
                    pass_len,
                    missing_msbs,
                    num_passes,
                });
            }
        }

        // Body starts at the current byte_pos (header was byte-stuffed and padded)
        let mut body_offset = byte_pos;
        for info in &cb_infos {
            let len = info.pass_len as usize;
            let coded_data = if body_offset + len <= data.len() {
                data[body_offset..body_offset + len].to_vec()
            } else {
                let available = data.len().saturating_sub(body_offset);
                let mut d = vec![0u8; len];
                let copy = available.min(len);
                d[..copy].copy_from_slice(&data[body_offset..body_offset + copy]);
                d
            };
            body_offset += len;

            let tc = &mut self.tile_comps[comp as usize];
            let sb = &mut tc.resolutions[res_num as usize].subbands[info.sb_idx];
            let cb = &mut sb.codeblocks[info.cb_idx];
            cb.coded_data = coded_data;
            cb.dec_state = Some(CodeblockDecState {
                pass1_len: info.pass_len,
                pass2_len: 0,
                num_passes: info.num_passes,
                missing_msbs: info.missing_msbs,
            });
        }

        Ok(body_offset)
    }

    /// Pull a decoded line for the given component.
    pub fn pull_line(&mut self, comp_num: u32) -> Option<Vec<i32>> {
        self.tile_comps.get_mut(comp_num as usize)
            .and_then(|tc| tc.pull_line())
    }
}
