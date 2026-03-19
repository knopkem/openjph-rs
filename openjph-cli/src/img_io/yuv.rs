//! YUV raw planar video reader and writer.
//!
//! Supports 444, 422, 420, and 400 subsampling formats.
//! Format parameters are typically passed from CLI arguments:
//! width, height, bit depth, and subsampling format.
#![allow(dead_code)]

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

use super::{ImageReader, ImageWriter};
use std::any::Any;

// ---------------------------------------------------------------------------
// Subsampling helpers
// ---------------------------------------------------------------------------

/// Subsampling factors `(dx, dy)` for each format.
pub fn subsampling_from_format(fmt: &str, num_comps: u32) -> anyhow::Result<Vec<(u32, u32)>> {
    if num_comps == 1 {
        return Ok(vec![(1, 1)]);
    }
    match fmt {
        "444" => Ok(vec![(1, 1), (1, 1), (1, 1)]),
        "422" => Ok(vec![(1, 1), (2, 1), (2, 1)]),
        "420" => Ok(vec![(1, 1), (2, 2), (2, 2)]),
        "400" => Ok(vec![(1, 1)]),
        _ => anyhow::bail!("Unknown YUV subsampling format: '{}'", fmt),
    }
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

pub struct YuvReader {
    reader: Option<BufReader<File>>,
    width: u32,
    height: u32,
    num_components: u32,
    bit_depth: u32,
    bytes_per_sample: u32,
    is_signed: bool,
    /// Per-component subsampling `(dx, dy)`.
    subsampling: Vec<(u32, u32)>,
    /// Per-component width.
    comp_widths: Vec<u32>,
    /// Per-component height.
    comp_heights: Vec<u32>,
    /// Raw byte buffer for reading one line of one component.
    temp_buf: Vec<u8>,
    /// Converted i32 line buffer.
    line_buf: Vec<i32>,
}

impl YuvReader {
    pub fn new() -> Self {
        Self {
            reader: None,
            width: 0,
            height: 0,
            num_components: 0,
            bit_depth: 0,
            bytes_per_sample: 0,
            is_signed: false,
            subsampling: Vec::new(),
            comp_widths: Vec::new(),
            comp_heights: Vec::new(),
            temp_buf: Vec::new(),
            line_buf: Vec::new(),
        }
    }

    /// Configure the YUV reader with image properties before opening.
    pub fn set_img_props(
        &mut self,
        width: u32,
        height: u32,
        num_components: u32,
        subsampling: &[(u32, u32)],
    ) {
        self.width = width;
        self.height = height;
        self.num_components = num_components;
        self.subsampling = subsampling.to_vec();

        self.comp_widths.clear();
        self.comp_heights.clear();
        for i in 0..num_components as usize {
            let (dx, dy) = if i < subsampling.len() {
                subsampling[i]
            } else {
                (1, 1)
            };
            self.comp_widths.push(width.div_ceil(dx));
            self.comp_heights.push(height.div_ceil(dy));
        }
    }

    /// Set bit depth and signedness.
    pub fn set_bit_depth(&mut self, bit_depth: u32, is_signed: bool) {
        self.bit_depth = bit_depth;
        self.is_signed = is_signed;
        self.bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };

        // Allocate temp buf for the widest component line
        let max_width = self.comp_widths.iter().copied().max().unwrap_or(self.width);
        self.temp_buf = vec![0u8; max_width as usize * self.bytes_per_sample as usize];
        self.line_buf = vec![0i32; max_width as usize];
    }
}

impl ImageReader for YuvReader {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn open(&mut self, filename: &str) -> anyhow::Result<()> {
        let file = File::open(filename)
            .map_err(|e| anyhow::anyhow!("Cannot open '{}': {}", filename, e))?;
        self.reader = Some(BufReader::new(file));
        Ok(())
    }

    fn read_line(&mut self, comp_num: u32) -> anyhow::Result<&[i32]> {
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("File not open"))?;
        let comp = comp_num as usize;
        let w = self.comp_widths[comp] as usize;
        let bps = self.bytes_per_sample as usize;
        let byte_count = w * bps;

        reader.read_exact(&mut self.temp_buf[..byte_count])?;

        if bps == 1 {
            if self.is_signed {
                for i in 0..w {
                    self.line_buf[i] = self.temp_buf[i] as i8 as i32;
                }
            } else {
                for i in 0..w {
                    self.line_buf[i] = self.temp_buf[i] as i32;
                }
            }
        } else {
            // 16-bit little-endian (YUV convention)
            if self.is_signed {
                for i in 0..w {
                    let lo = self.temp_buf[i * 2] as u16;
                    let hi = self.temp_buf[i * 2 + 1] as u16;
                    self.line_buf[i] = ((hi << 8) | lo) as i16 as i32;
                }
            } else {
                for i in 0..w {
                    let lo = self.temp_buf[i * 2] as u16;
                    let hi = self.temp_buf[i * 2 + 1] as u16;
                    self.line_buf[i] = ((hi << 8) | lo) as i32;
                }
            }
        }

        Ok(&self.line_buf[..w])
    }

    fn get_num_components(&self) -> u32 {
        self.num_components
    }

    fn get_bit_depth(&self, _comp_num: u32) -> u32 {
        self.bit_depth
    }

    fn is_signed(&self, _comp_num: u32) -> bool {
        self.is_signed
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn get_height(&self) -> u32 {
        self.height
    }

    fn get_downsampling(&self, comp_num: u32) -> (u32, u32) {
        let idx = comp_num as usize;
        if idx < self.subsampling.len() {
            self.subsampling[idx]
        } else {
            (1, 1)
        }
    }

    fn close(&mut self) {
        self.reader = None;
    }
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

pub struct YuvWriter {
    writer: Option<BufWriter<File>>,
    bit_depth: u32,
    bytes_per_sample: u32,
    num_components: u32,
    /// Per-component widths.
    comp_widths: Vec<u32>,
    /// Raw byte buffer for writing.
    temp_buf: Vec<u8>,
}

impl YuvWriter {
    pub fn new() -> Self {
        Self {
            writer: None,
            bit_depth: 0,
            bytes_per_sample: 0,
            num_components: 0,
            comp_widths: Vec::new(),
            temp_buf: Vec::new(),
        }
    }

    /// Configure with per-component widths (needed for subsampled outputs).
    pub fn configure_with_comp_widths(
        &mut self,
        bit_depth: u32,
        num_components: u32,
        comp_widths: &[u32],
    ) {
        self.bit_depth = bit_depth;
        self.bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };
        self.num_components = num_components;
        self.comp_widths = comp_widths.to_vec();

        let max_w = comp_widths.iter().copied().max().unwrap_or(0);
        self.temp_buf = vec![0u8; max_w as usize * self.bytes_per_sample as usize];
    }
}

impl ImageWriter for YuvWriter {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn configure(
        &mut self,
        width: u32,
        _height: u32,
        num_components: u32,
        bit_depth: u32,
    ) -> anyhow::Result<()> {
        // Default: all components same width (444)
        let comp_widths = vec![width; num_components as usize];
        self.configure_with_comp_widths(bit_depth, num_components, &comp_widths);
        Ok(())
    }

    fn open(&mut self, filename: &str) -> anyhow::Result<()> {
        let file = File::create(filename)
            .map_err(|e| anyhow::anyhow!("Cannot create '{}': {}", filename, e))?;
        self.writer = Some(BufWriter::new(file));
        Ok(())
    }

    fn write_line(&mut self, comp_num: u32, data: &[i32]) -> anyhow::Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("File not open"))?;
        let comp = comp_num as usize;
        let w = self.comp_widths[comp] as usize;
        let max_val = ((1i64 << self.bit_depth) - 1) as i32;
        let bps = self.bytes_per_sample as usize;

        if bps == 1 {
            for (i, &src) in data[..w].iter().enumerate() {
                let val = src.max(0).min(max_val);
                self.temp_buf[i] = val as u8;
            }
        } else {
            // 16-bit little-endian
            for (i, &src) in data[..w].iter().enumerate() {
                let val = src.max(0).min(max_val) as u16;
                self.temp_buf[i * 2] = (val & 0xFF) as u8;
                self.temp_buf[i * 2 + 1] = (val >> 8) as u8;
            }
        }

        writer.write_all(&self.temp_buf[..w * bps])?;
        Ok(())
    }

    fn close(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut w) = self.writer {
            w.flush()?;
        }
        self.writer = None;
        Ok(())
    }
}
