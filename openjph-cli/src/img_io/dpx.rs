//! DPX (Digital Picture Exchange) reader.
//!
//! Supports 10-bit packed (Method A) and 16-bit samples in both big-endian
//! and little-endian byte orders. Read-only.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use super::ImageReader;
use std::any::Any;

/// DPX magic number (big-endian file): ASCII "SDPX".
const DPX_MAGIC_BE: u32 = 0x53445058;
/// DPX magic number (little-endian file): byte-swapped.
const DPX_MAGIC_LE: u32 = 0x58504453;

// ---------------------------------------------------------------------------
// Header helpers
// ---------------------------------------------------------------------------

fn read_u32(reader: &mut BufReader<File>, swap: bool) -> anyhow::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    let val = u32::from_be_bytes(buf);
    Ok(if swap { val.swap_bytes() } else { val })
}

fn read_u16(reader: &mut BufReader<File>, swap: bool) -> anyhow::Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    let val = u16::from_be_bytes(buf);
    Ok(if swap { val.swap_bytes() } else { val })
}

fn read_u8(reader: &mut BufReader<File>) -> anyhow::Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

pub struct DpxReader {
    reader: Option<BufReader<File>>,
    width: u32,
    height: u32,
    num_components: u32,
    bit_depth: u32,
    is_signed: bool,
    swap_bytes: bool,
    /// Offset to image data in the file.
    data_offset: u32,
    /// Packing method: 0 = no packing (16-bit), 1 = Method A (10-bit packed).
    packing: u16,
    /// Number of 32-bit words per scanline (for 10-bit packed).
    words_per_line: u32,
    /// Raw byte buffer for one scanline.
    raw_buf: Vec<u8>,
    /// Unpacked 16-bit sample buffer (all components interleaved).
    samples_buf: Vec<u16>,
    /// Output i32 buffer for one component's line.
    line_buf: Vec<i32>,
    /// Whether current line data has been read.
    cur_line_read: bool,
}

impl DpxReader {
    pub fn new() -> Self {
        Self {
            reader: None,
            width: 0,
            height: 0,
            num_components: 3,
            bit_depth: 10,
            is_signed: false,
            swap_bytes: false,
            data_offset: 0,
            packing: 0,
            words_per_line: 0,
            raw_buf: Vec::new(),
            samples_buf: Vec::new(),
            line_buf: Vec::new(),
            cur_line_read: false,
        }
    }
}

impl ImageReader for DpxReader {
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn open(&mut self, filename: &str) -> anyhow::Result<()> {
        let file = File::open(filename)
            .map_err(|e| anyhow::anyhow!("Cannot open '{}': {}", filename, e))?;
        let mut reader = BufReader::new(file);

        // Read magic number to determine endianness
        let mut magic_bytes = [0u8; 4];
        reader.read_exact(&mut magic_bytes)?;
        let magic = u32::from_be_bytes(magic_bytes);

        self.swap_bytes = match magic {
            DPX_MAGIC_BE => false,
            DPX_MAGIC_LE => true,
            _ => anyhow::bail!("Not a DPX file (invalid magic: 0x{:08X})", magic),
        };

        // File header: offset to image data (offset 4)
        self.data_offset = read_u32(&mut reader, self.swap_bytes)?;

        // Skip to image info header at offset 768
        reader.seek(SeekFrom::Start(768))?;
        let _orientation = read_u16(&mut reader, self.swap_bytes)?;
        let _num_elements = read_u16(&mut reader, self.swap_bytes)?;
        let width = read_u32(&mut reader, self.swap_bytes)?;
        let height = read_u32(&mut reader, self.swap_bytes)?;

        // Image element descriptor at offset 780
        reader.seek(SeekFrom::Start(780))?;
        let data_sign = read_u32(&mut reader, self.swap_bytes)?;
        self.is_signed = data_sign != 0;

        // Skip to descriptor byte at offset 800
        reader.seek(SeekFrom::Start(800))?;
        let descriptor = read_u8(&mut reader)?;
        let _transfer = read_u8(&mut reader)?;
        let _colorimetric = read_u8(&mut reader)?;
        let bit_depth = read_u8(&mut reader)? as u32;
        let packing = read_u16(&mut reader, self.swap_bytes)?;
        let _encoding = read_u16(&mut reader, self.swap_bytes)?;

        // Determine number of components from descriptor
        let num_components = match descriptor {
            // 1 = Red, 2 = Green, 3 = Blue, 4 = Alpha, etc.
            1 | 2 | 3 | 4 | 6 | 8 => 1u32,
            // 50 = RGB, 51 = RGBA, 52 = ABGR
            50 => 3u32,
            51 | 52 => 4u32,
            // Default: assume 3 components (RGB)
            _ => 3u32,
        };

        self.width = width;
        self.height = height;
        self.num_components = num_components;
        self.bit_depth = bit_depth;
        self.packing = packing;

        // Compute buffer sizes
        if bit_depth == 10 && packing != 0 {
            // 10-bit packed Method A: 3 samples per 32-bit word
            self.words_per_line = (num_components * width + 2) / 3;
            self.raw_buf = vec![0u8; self.words_per_line as usize * 4];
        } else if bit_depth == 16 || (bit_depth == 10 && packing == 0) {
            // 16-bit per sample
            let bytes_per_line = num_components * width * 2;
            self.raw_buf = vec![0u8; bytes_per_line as usize];
            self.words_per_line = (bytes_per_line + 3) / 4;
        } else if bit_depth == 8 {
            let bytes_per_line = num_components * width;
            self.raw_buf = vec![0u8; bytes_per_line as usize];
            self.words_per_line = (bytes_per_line + 3) / 4;
        } else {
            anyhow::bail!("Unsupported DPX bit depth: {}", bit_depth);
        }

        self.samples_buf = vec![0u16; (num_components * width) as usize];
        self.line_buf = vec![0i32; width as usize];
        self.cur_line_read = false;

        // Seek to image data
        reader.seek(SeekFrom::Start(self.data_offset as u64))?;
        self.reader = Some(reader);

        Ok(())
    }

    fn read_line(&mut self, comp_num: u32) -> anyhow::Result<&[i32]> {
        // Read raw data when processing component 0
        if comp_num == 0 || !self.cur_line_read {
            let reader = self.reader.as_mut().ok_or_else(|| anyhow::anyhow!("File not open"))?;

            if self.bit_depth == 10 && self.packing != 0 {
                // 10-bit packed Method A
                let byte_count = self.words_per_line as usize * 4;
                reader.read_exact(&mut self.raw_buf[..byte_count])?;

                let total_samples = (self.num_components * self.width) as usize;
                let mut sample_idx = 0;

                for word_idx in 0..self.words_per_line as usize {
                    let word = if self.swap_bytes {
                        let b = &self.raw_buf[word_idx * 4..word_idx * 4 + 4];
                        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                    } else {
                        let b = &self.raw_buf[word_idx * 4..word_idx * 4 + 4];
                        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
                    };

                    // Method A: R[31:22], G[21:12], B[11:2]
                    if sample_idx < total_samples {
                        self.samples_buf[sample_idx] = ((word >> 22) & 0x3FF) as u16;
                        sample_idx += 1;
                    }
                    if sample_idx < total_samples {
                        self.samples_buf[sample_idx] = ((word >> 12) & 0x3FF) as u16;
                        sample_idx += 1;
                    }
                    if sample_idx < total_samples {
                        self.samples_buf[sample_idx] = ((word >> 2) & 0x3FF) as u16;
                        sample_idx += 1;
                    }
                }
            } else if self.bit_depth == 16 {
                // 16-bit samples
                let byte_count = (self.num_components * self.width * 2) as usize;
                reader.read_exact(&mut self.raw_buf[..byte_count])?;

                let total_samples = (self.num_components * self.width) as usize;
                for i in 0..total_samples {
                    let val = if self.swap_bytes {
                        u16::from_le_bytes([self.raw_buf[i * 2], self.raw_buf[i * 2 + 1]])
                    } else {
                        u16::from_be_bytes([self.raw_buf[i * 2], self.raw_buf[i * 2 + 1]])
                    };
                    self.samples_buf[i] = val;
                }
            } else if self.bit_depth == 10 && self.packing == 0 {
                // 10-bit unpacked into 16-bit words
                let byte_count = (self.num_components * self.width * 2) as usize;
                reader.read_exact(&mut self.raw_buf[..byte_count])?;

                let total_samples = (self.num_components * self.width) as usize;
                for i in 0..total_samples {
                    let val = if self.swap_bytes {
                        u16::from_le_bytes([self.raw_buf[i * 2], self.raw_buf[i * 2 + 1]])
                    } else {
                        u16::from_be_bytes([self.raw_buf[i * 2], self.raw_buf[i * 2 + 1]])
                    };
                    self.samples_buf[i] = val & 0x3FF;
                }
            } else {
                // 8-bit
                let byte_count = (self.num_components * self.width) as usize;
                reader.read_exact(&mut self.raw_buf[..byte_count])?;

                for i in 0..byte_count {
                    self.samples_buf[i] = self.raw_buf[i] as u16;
                }
            }

            self.cur_line_read = true;
        }

        // De-interleave: extract comp_num
        let nc = self.num_components;
        let w = self.width as usize;
        for i in 0..w {
            self.line_buf[i] = self.samples_buf[i * nc as usize + comp_num as usize] as i32;
        }

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
