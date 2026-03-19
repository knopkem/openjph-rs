//! Image I/O traits and format dispatch for the CLI tools.
//!
//! Supports PPM/PGM, YUV, DPX (read-only), and RAWL formats.

use std::any::Any;

pub mod ppm;
pub mod yuv;
pub mod dpx;
pub mod rawl;

/// Trait for reading image data line-by-line.
pub trait ImageReader {
    /// Upcast to `Any` for downcasting to concrete types.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Open the image file and parse its header.
    fn open(&mut self, filename: &str) -> anyhow::Result<()>;

    /// Read one line of samples for the given component.
    /// Returns a slice of `i32` samples for that row.
    fn read_line(&mut self, comp_num: u32) -> anyhow::Result<&[i32]>;

    /// Number of image components (e.g. 1 for grayscale, 3 for RGB).
    fn get_num_components(&self) -> u32;

    /// Bit depth of the given component.
    fn get_bit_depth(&self, comp_num: u32) -> u32;

    /// Whether samples for the given component are signed.
    fn is_signed(&self, comp_num: u32) -> bool;

    /// Image width (of the full-resolution grid).
    fn get_width(&self) -> u32;

    /// Image height (of the full-resolution grid).
    fn get_height(&self) -> u32;

    /// Per-component subsampling factors `(dx, dy)`.
    fn get_downsampling(&self, comp_num: u32) -> (u32, u32);

    /// Close the reader and release resources.
    fn close(&mut self);
}

/// Trait for writing image data line-by-line.
pub trait ImageWriter {
    /// Upcast to `Any` for downcasting to concrete types.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Open the output file and configure it.
    fn configure(
        &mut self,
        width: u32,
        height: u32,
        num_components: u32,
        bit_depth: u32,
    ) -> anyhow::Result<()>;

    /// Open the file for writing (call after configure).
    fn open(&mut self, filename: &str) -> anyhow::Result<()>;

    /// Write one line of samples for the given component.
    fn write_line(&mut self, comp_num: u32, data: &[i32]) -> anyhow::Result<()>;

    /// Flush and close the writer.
    fn close(&mut self) -> anyhow::Result<()>;
}

/// Detect image format from file extension and return a boxed reader.
pub fn create_reader(filename: &str) -> anyhow::Result<Box<dyn ImageReader>> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".ppm") || lower.ends_with(".pgm") {
        Ok(Box::new(ppm::PpmReader::new()))
    } else if lower.ends_with(".yuv") {
        Ok(Box::new(yuv::YuvReader::new()))
    } else if lower.ends_with(".dpx") {
        Ok(Box::new(dpx::DpxReader::new()))
    } else if lower.ends_with(".rawl") || lower.ends_with(".raw") {
        Ok(Box::new(rawl::RawlReader::new()))
    } else {
        anyhow::bail!("Unsupported input format: {}", filename)
    }
}

/// Detect image format from file extension and return a boxed writer.
pub fn create_writer(filename: &str) -> anyhow::Result<Box<dyn ImageWriter>> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".ppm") || lower.ends_with(".pgm") {
        Ok(Box::new(ppm::PpmWriter::new()))
    } else if lower.ends_with(".yuv") {
        Ok(Box::new(yuv::YuvWriter::new()))
    } else if lower.ends_with(".rawl") || lower.ends_with(".raw") {
        Ok(Box::new(rawl::RawlWriter::new()))
    } else {
        anyhow::bail!("Unsupported output format: {}", filename)
    }
}
