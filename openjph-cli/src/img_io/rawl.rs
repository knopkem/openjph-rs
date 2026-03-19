//! RAWL (raw linear) reader and writer.
//!
//! Simple sequential I/O of raw sample data. Format parameters (width, height,
//! components, bit depth, signedness) are specified externally via CLI arguments.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

use super::{ImageReader, ImageWriter};
use std::any::Any;

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

pub struct RawlReader {
    reader: Option<BufReader<File>>,
    width: u32,
    height: u32,
    num_components: u32,
    bit_depth: u32,
    bytes_per_sample: u32,
    is_signed: bool,
    /// Raw byte buffer.
    temp_buf: Vec<u8>,
    /// Converted i32 buffer.
    line_buf: Vec<i32>,
}

impl RawlReader {
    pub fn new() -> Self {
        Self {
            reader: None,
            width: 0,
            height: 0,
            num_components: 1,
            bit_depth: 8,
            bytes_per_sample: 1,
            is_signed: false,
            temp_buf: Vec::new(),
            line_buf: Vec::new(),
        }
    }

    /// Configure reader properties (must be called before open).
    pub fn set_img_props(
        &mut self,
        width: u32,
        height: u32,
        num_components: u32,
        bit_depth: u32,
        is_signed: bool,
    ) {
        self.width = width;
        self.height = height;
        self.num_components = num_components;
        self.bit_depth = bit_depth;
        self.is_signed = is_signed;
        self.bytes_per_sample = (bit_depth + 7) / 8;

        let max_samples = width as usize;
        self.temp_buf = vec![0u8; max_samples * self.bytes_per_sample as usize];
        self.line_buf = vec![0i32; max_samples];
    }
}

impl ImageReader for RawlReader {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn open(&mut self, filename: &str) -> anyhow::Result<()> {
        let file = File::open(filename)
            .map_err(|e| anyhow::anyhow!("Cannot open '{}': {}", filename, e))?;
        self.reader = Some(BufReader::new(file));
        Ok(())
    }

    fn read_line(&mut self, _comp_num: u32) -> anyhow::Result<&[i32]> {
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("File not open"))?;
        let w = self.width as usize;
        let bps = self.bytes_per_sample as usize;
        let byte_count = w * bps;

        reader.read_exact(&mut self.temp_buf[..byte_count])?;

        match bps {
            1 => {
                if self.is_signed {
                    for i in 0..w {
                        self.line_buf[i] = self.temp_buf[i] as i8 as i32;
                    }
                } else {
                    for i in 0..w {
                        self.line_buf[i] = self.temp_buf[i] as i32;
                    }
                }
            }
            2 => {
                // Little-endian 16-bit
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
            3 => {
                // Little-endian 24-bit
                for i in 0..w {
                    let b0 = self.temp_buf[i * 3] as u32;
                    let b1 = self.temp_buf[i * 3 + 1] as u32;
                    let b2 = self.temp_buf[i * 3 + 2] as u32;
                    let mut val = b0 | (b1 << 8) | (b2 << 16);
                    if self.is_signed && (val & 0x800000) != 0 {
                        val |= 0xFF000000; // sign extend
                    }
                    self.line_buf[i] = val as i32;
                }
            }
            4 => {
                // Little-endian 32-bit
                if self.is_signed {
                    for i in 0..w {
                        let b = &self.temp_buf[i * 4..i * 4 + 4];
                        self.line_buf[i] = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    }
                } else {
                    for i in 0..w {
                        let b = &self.temp_buf[i * 4..i * 4 + 4];
                        self.line_buf[i] = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i32;
                    }
                }
            }
            _ => anyhow::bail!("Unsupported bytes per sample: {}", bps),
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

    fn get_downsampling(&self, _comp_num: u32) -> (u32, u32) {
        (1, 1)
    }

    fn close(&mut self) {
        self.reader = None;
    }
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

pub struct RawlWriter {
    writer: Option<BufWriter<File>>,
    width: u32,
    bit_depth: u32,
    bytes_per_sample: u32,
    is_signed: bool,
    upper_val: i64,
    lower_val: i64,
    temp_buf: Vec<u8>,
}

impl RawlWriter {
    pub fn new() -> Self {
        Self {
            writer: None,
            width: 0,
            bit_depth: 8,
            bytes_per_sample: 1,
            is_signed: false,
            upper_val: 255,
            lower_val: 0,
            temp_buf: Vec::new(),
        }
    }

    /// Configure with signedness (must be called before open).
    pub fn configure_with_sign(&mut self, is_signed: bool, bit_depth: u32, width: u32) {
        self.is_signed = is_signed;
        self.bit_depth = bit_depth;
        self.width = width;
        self.bytes_per_sample = (bit_depth + 7) / 8;

        if is_signed {
            self.upper_val = (1i64 << (bit_depth - 1)) - 1;
            self.lower_val = -(1i64 << (bit_depth - 1));
        } else {
            self.upper_val = (1i64 << bit_depth) - 1;
            self.lower_val = 0;
        }

        self.temp_buf = vec![0u8; width as usize * self.bytes_per_sample as usize];
    }
}

impl ImageWriter for RawlWriter {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn configure(
        &mut self,
        width: u32,
        _height: u32,
        _num_components: u32,
        bit_depth: u32,
    ) -> anyhow::Result<()> {
        self.configure_with_sign(false, bit_depth, width);
        Ok(())
    }

    fn open(&mut self, filename: &str) -> anyhow::Result<()> {
        let file = File::create(filename)
            .map_err(|e| anyhow::anyhow!("Cannot create '{}': {}", filename, e))?;
        self.writer = Some(BufWriter::new(file));
        Ok(())
    }

    fn write_line(&mut self, _comp_num: u32, data: &[i32]) -> anyhow::Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("File not open"))?;
        let w = self.width as usize;
        let bps = self.bytes_per_sample as usize;

        match bps {
            1 => {
                for i in 0..w {
                    let val = (data[i] as i64).max(self.lower_val).min(self.upper_val);
                    self.temp_buf[i] = val as u8;
                }
            }
            2 => {
                for i in 0..w {
                    let val = (data[i] as i64).max(self.lower_val).min(self.upper_val) as u16;
                    self.temp_buf[i * 2] = (val & 0xFF) as u8;
                    self.temp_buf[i * 2 + 1] = (val >> 8) as u8;
                }
            }
            3 => {
                for i in 0..w {
                    let val = (data[i] as i64).max(self.lower_val).min(self.upper_val) as u32;
                    self.temp_buf[i * 3] = (val & 0xFF) as u8;
                    self.temp_buf[i * 3 + 1] = ((val >> 8) & 0xFF) as u8;
                    self.temp_buf[i * 3 + 2] = ((val >> 16) & 0xFF) as u8;
                }
            }
            4 => {
                for i in 0..w {
                    let val = (data[i] as i64).max(self.lower_val).min(self.upper_val) as u32;
                    let bytes = val.to_le_bytes();
                    self.temp_buf[i * 4..i * 4 + 4].copy_from_slice(&bytes);
                }
            }
            _ => anyhow::bail!("Unsupported bytes per sample: {}", bps),
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
