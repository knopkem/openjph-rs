//! PPM/PGM image reader and writer.
//!
//! Supports P5 (PGM binary, grayscale) and P6 (PPM binary, RGB) formats
//! with 8-bit and 16-bit samples. 16-bit samples use big-endian byte order.
#![allow(dead_code)]

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use super::{ImageReader, ImageWriter};
use std::any::Any;

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

pub struct PpmReader {
    reader: Option<BufReader<File>>,
    width: u32,
    height: u32,
    num_components: u32,
    max_val: u32,
    bit_depth: u32,
    bytes_per_sample: u32,
    /// Raw byte buffer for one scanline (all components interleaved).
    temp_buf: Vec<u8>,
    /// Converted i32 line buffer (one component's worth of samples).
    line_buf: Vec<i32>,
    /// Full interleaved i32 buffer (all components for one scanline).
    interleaved_buf: Vec<i32>,
    /// Whether the current row's data has been read into interleaved_buf.
    cur_line_read: bool,
}

impl PpmReader {
    pub fn new() -> Self {
        Self {
            reader: None,
            width: 0,
            height: 0,
            num_components: 0,
            max_val: 0,
            bit_depth: 0,
            bytes_per_sample: 0,
            temp_buf: Vec::new(),
            line_buf: Vec::new(),
            interleaved_buf: Vec::new(),
            cur_line_read: false,
        }
    }
}

/// Skip whitespace and `#` comments in a PPM/PGM header.
fn skip_whitespace_and_comments(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            anyhow::bail!("Unexpected end of PPM/PGM header");
        }
        let ch = buf[0];
        if ch == b'#' {
            // Skip until end of line
            let mut line = String::new();
            reader.read_line(&mut line)?;
        } else if ch.is_ascii_whitespace() {
            reader.consume(1);
        } else {
            break;
        }
    }
    Ok(())
}

/// Read a positive decimal integer from the header.
fn read_header_int(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    skip_whitespace_and_comments(reader)?;
    let mut digits = Vec::new();
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() || !buf[0].is_ascii_digit() {
            break;
        }
        digits.push(buf[0]);
        reader.consume(1);
    }
    if digits.is_empty() {
        anyhow::bail!("Expected integer in PPM/PGM header");
    }
    let s = std::str::from_utf8(&digits)?;
    Ok(s.parse::<u32>()?)
}

impl ImageReader for PpmReader {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn open(&mut self, filename: &str) -> anyhow::Result<()> {
        let file = File::open(filename)
            .map_err(|e| anyhow::anyhow!("Cannot open '{}': {}", filename, e))?;
        let mut reader = BufReader::new(file);

        // Read magic number
        let mut magic = [0u8; 2];
        reader.read_exact(&mut magic)?;
        let (num_components, expected_ext) = match &magic {
            b"P5" => (1u32, ".pgm"),
            b"P6" => (3u32, ".ppm"),
            _ => anyhow::bail!(
                "Unsupported PPM/PGM magic: {:?}",
                std::str::from_utf8(&magic).unwrap_or("??")
            ),
        };

        // Validate extension
        let lower = filename.to_lowercase();
        if !lower.ends_with(expected_ext) {
            eprintln!(
                "Warning: file extension does not match PPM type (expected {})",
                expected_ext
            );
        }

        let width = read_header_int(&mut reader)?;
        let height = read_header_int(&mut reader)?;
        let max_val = read_header_int(&mut reader)?;

        if max_val == 0 {
            anyhow::bail!("PPM/PGM maxval cannot be 0");
        }

        // Consume the single whitespace byte after maxval
        let mut ws = [0u8; 1];
        reader.read_exact(&mut ws)?;

        let bit_depth = 32 - max_val.leading_zeros();
        let bytes_per_sample = if max_val > 255 { 2u32 } else { 1u32 };

        let num_ele_per_line = (num_components * width) as usize;
        let temp_buf_size = num_ele_per_line * bytes_per_sample as usize;

        self.reader = Some(reader);
        self.width = width;
        self.height = height;
        self.num_components = num_components;
        self.max_val = max_val;
        self.bit_depth = bit_depth;
        self.bytes_per_sample = bytes_per_sample;
        self.temp_buf = vec![0u8; temp_buf_size];
        self.line_buf = vec![0i32; width as usize];
        self.interleaved_buf = vec![0i32; num_ele_per_line];
        self.cur_line_read = false;

        Ok(())
    }

    fn read_line(&mut self, comp_num: u32) -> anyhow::Result<&[i32]> {
        // Read raw data when processing component 0 (or when data is stale)
        if comp_num == 0 || !self.cur_line_read {
            let reader = self
                .reader
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("File not open"))?;
            reader.read_exact(&mut self.temp_buf)?;
            self.cur_line_read = true;

            // Convert to i32
            let num_ele = (self.num_components * self.width) as usize;
            if self.bytes_per_sample == 1 {
                for i in 0..num_ele {
                    self.interleaved_buf[i] = self.temp_buf[i] as i32;
                }
            } else {
                // 16-bit big-endian
                for i in 0..num_ele {
                    let hi = self.temp_buf[i * 2] as u16;
                    let lo = self.temp_buf[i * 2 + 1] as u16;
                    self.interleaved_buf[i] = ((hi << 8) | lo) as i32;
                }
            }
        }

        // De-interleave: extract comp_num from interleaved data
        let nc = self.num_components;
        let w = self.width as usize;
        for i in 0..w {
            self.line_buf[i] = self.interleaved_buf[i * nc as usize + comp_num as usize];
        }

        // Mark data consumed after reading last component
        if comp_num == nc - 1 {
            self.cur_line_read = false;
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
        false
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

pub struct PpmWriter {
    writer: Option<BufWriter<File>>,
    width: u32,
    height: u32,
    num_components: u32,
    bit_depth: u32,
    bytes_per_sample: u32,
    /// Buffer for one interleaved scanline in i32 form.
    interleaved_buf: Vec<i32>,
    /// Raw byte buffer for writing.
    temp_buf: Vec<u8>,
    /// Track which component we're accumulating.
    cur_comp: u32,
}

impl PpmWriter {
    pub fn new() -> Self {
        Self {
            writer: None,
            width: 0,
            height: 0,
            num_components: 0,
            bit_depth: 0,
            bytes_per_sample: 0,
            interleaved_buf: Vec::new(),
            temp_buf: Vec::new(),
            cur_comp: 0,
        }
    }
}

impl ImageWriter for PpmWriter {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn configure(
        &mut self,
        width: u32,
        height: u32,
        num_components: u32,
        bit_depth: u32,
    ) -> anyhow::Result<()> {
        if num_components != 1 && num_components != 3 {
            anyhow::bail!("PPM/PGM supports 1 or 3 components, got {}", num_components);
        }
        self.width = width;
        self.height = height;
        self.num_components = num_components;
        self.bit_depth = bit_depth;
        self.bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };

        let num_ele = (num_components * width) as usize;
        self.interleaved_buf = vec![0i32; num_ele];
        self.temp_buf = vec![0u8; num_ele * self.bytes_per_sample as usize];
        self.cur_comp = 0;
        Ok(())
    }

    fn open(&mut self, filename: &str) -> anyhow::Result<()> {
        let file = File::create(filename)
            .map_err(|e| anyhow::anyhow!("Cannot create '{}': {}", filename, e))?;
        let mut writer = BufWriter::new(file);

        let magic = if self.num_components == 1 { "P5" } else { "P6" };
        let max_val = (1u32 << self.bit_depth) - 1;
        write!(
            writer,
            "{}\n{} {}\n{}\n",
            magic, self.width, self.height, max_val
        )?;

        self.writer = Some(writer);
        Ok(())
    }

    fn write_line(&mut self, comp_num: u32, data: &[i32]) -> anyhow::Result<()> {
        let w = self.width as usize;
        let nc = self.num_components as usize;
        let max_val = ((1i64 << self.bit_depth) - 1) as i32;

        // Interleave: place component data into the interleaved buffer
        for (i, &val) in data[..w].iter().enumerate() {
            let val = val.max(0).min(max_val);
            self.interleaved_buf[i * nc + comp_num as usize] = val;
        }

        self.cur_comp = comp_num + 1;

        // Flush the line when all components have been written
        if self.cur_comp >= self.num_components {
            let num_ele = nc * w;
            if self.bytes_per_sample == 1 {
                for i in 0..num_ele {
                    self.temp_buf[i] = self.interleaved_buf[i] as u8;
                }
            } else {
                // 16-bit big-endian
                for i in 0..num_ele {
                    let val = self.interleaved_buf[i] as u16;
                    self.temp_buf[i * 2] = (val >> 8) as u8;
                    self.temp_buf[i * 2 + 1] = (val & 0xFF) as u8;
                }
            }
            let writer = self
                .writer
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("File not open"))?;
            writer.write_all(&self.temp_buf[..num_ele * self.bytes_per_sample as usize])?;
            self.cur_comp = 0;
        }

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
