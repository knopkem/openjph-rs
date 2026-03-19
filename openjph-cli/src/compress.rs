//! ojph_compress — HTJ2K compression CLI tool.
//!
//! Port of `ojph_compress.cpp`. Reads image files (PPM, PGM, YUV, DPX, RAWL)
//! and compresses them to JPEG 2000 Part 15 (HTJ2K) codestreams.

mod img_io;

use anyhow::{bail, Context, Result};
use clap::Parser;
use openjph_core::codestream::Codestream;
use openjph_core::file::J2cOutfile;
use openjph_core::params::CommentExchange;
use openjph_core::types::{Point, Size};

use img_io::ImageReader;

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

/// Parse a `{w,h}` size pair.
fn parse_size(s: &str) -> Result<Size, String> {
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Expected {{w,h}}, got '{}'", s));
    }
    let w = parts[0].trim().parse::<u32>().map_err(|e| e.to_string())?;
    let h = parts[1].trim().parse::<u32>().map_err(|e| e.to_string())?;
    Ok(Size::new(w, h))
}

/// Parse a `{x,y}` point pair.
fn parse_point(s: &str) -> Result<Point, String> {
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Expected {{x,y}}, got '{}'", s));
    }
    let x = parts[0].trim().parse::<u32>().map_err(|e| e.to_string())?;
    let y = parts[1].trim().parse::<u32>().map_err(|e| e.to_string())?;
    Ok(Point::new(x, y))
}

/// Parse a list of `{w,h}` size pairs separated by commas.
fn parse_size_list(s: &str) -> Result<Vec<Size>, String> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let sub = &s[start..=i];
                    result.push(parse_size(sub)?);
                    start = i + 1;
                }
            }
            ',' if depth == 0 => {
                start = i + 1;
            }
            _ => {}
        }
    }
    if result.is_empty() {
        return Err(format!("Expected {{w,h}} list, got '{}'", s));
    }
    Ok(result)
}

/// Parse a list of `{x,y}` point pairs.
fn parse_point_list(s: &str) -> Result<Vec<Point>, String> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let sub = &s[start..=i];
                    result.push(parse_point(sub)?);
                    start = i + 1;
                }
            }
            ',' if depth == 0 => {
                start = i + 1;
            }
            _ => {}
        }
    }
    if result.is_empty() {
        return Err(format!("Expected {{x,y}} list, got '{}'", s));
    }
    Ok(result)
}

/// HTJ2K image compression tool
#[derive(Parser, Debug)]
#[command(name = "ojph_compress", about = "Compress images to HTJ2K (JPEG 2000 Part 15) codestreams")]
struct Args {
    /// Input image file (PPM, PGM, YUV, DPX, RAWL)
    #[arg(short = 'i', long = "input")]
    input: String,

    /// Output codestream file (.j2c, .jph)
    #[arg(short = 'o', long = "output")]
    output: String,

    /// Number of DWT decomposition levels
    #[arg(long = "num_decomps", default_value_t = 5)]
    num_decomps: u32,

    /// Code block dimensions {w,h}
    #[arg(long = "block_size", value_parser = parse_size, default_value = "{64,64}")]
    block_size: Size,

    /// Precinct sizes {w,h},{w,h},...
    #[arg(long = "precincts", value_parser = parse_size_list)]
    precincts: Option<Vec<Size>>,

    /// Progression order: LRCP, RLCP, RPCL, PCRL, CPRL
    #[arg(long = "prog_order", default_value = "RPCL")]
    prog_order: String,

    /// Colour transform (true/false)
    #[arg(long = "colour_trans")]
    colour_trans: Option<bool>,

    /// Reversible (lossless) coding
    #[arg(long = "reversible", default_value_t = false)]
    reversible: bool,

    /// Quantization step for lossy compression
    #[arg(long = "qstep")]
    qstep: Option<f32>,

    /// Number of components (for RAW/YUV input)
    #[arg(long = "num_comps")]
    num_comps: Option<u32>,

    /// Image dimensions {w,h} (for RAW/YUV input)
    #[arg(long = "dims", value_parser = parse_size)]
    dims: Option<Size>,

    /// Bit depth (per component)
    #[arg(long = "bit_depth")]
    bit_depth: Option<u32>,

    /// Signed samples
    #[arg(long = "signed")]
    signed: Option<bool>,

    /// Subsampling format for YUV (e.g. 444, 422, 420, 400)
    #[arg(long = "downsamp")]
    downsamp: Option<String>,

    /// Add TLM marker
    #[arg(long = "tlm_marker", default_value_t = false)]
    tlm_marker: bool,

    /// Tile part divisions: R (resolutions), C (components), RC (both)
    #[arg(long = "tilepart_divs")]
    tilepart_divs: Option<String>,

    /// Tile offset {x,y}
    #[arg(long = "tile_offset", value_parser = parse_point)]
    tile_offset: Option<Point>,

    /// Tile dimensions {w,h}
    #[arg(long = "tile_size", value_parser = parse_size)]
    tile_size: Option<Size>,

    /// Image offset {x,y}
    #[arg(long = "image_offset", value_parser = parse_point)]
    image_offset: Option<Point>,

    /// Profile: IMF, BROADCAST
    #[arg(long = "profile")]
    profile: Option<String>,

    /// Comment marker text
    #[arg(long = "com")]
    com: Option<String>,
}

// ---------------------------------------------------------------------------
// Main compression logic
// ---------------------------------------------------------------------------

fn run() -> Result<()> {
    let args = Args::parse();

    let image_offset = args.image_offset.unwrap_or(Point::new(0, 0));
    let tile_offset = args.tile_offset.unwrap_or(Point::new(0, 0));

    // --- Create the appropriate image reader based on file extension ---
    let lower_input = args.input.to_lowercase();
    let mut reader: Box<dyn ImageReader> = img_io::create_reader(&args.input)?;

    let is_planar;

    if lower_input.ends_with(".yuv") {
        // YUV requires external dimensions
        let dims = args.dims.context("-dims required for YUV input")?;
        let num_comps = args.num_comps.unwrap_or(3);
        let bit_depth = args.bit_depth.unwrap_or(8);
        let is_signed = args.signed.unwrap_or(false);
        let fmt = args.downsamp.as_deref().unwrap_or("420");

        let subsampling = img_io::yuv::subsampling_from_format(fmt, num_comps)?;

        // Downcast to configure YuvReader
        let yuv_reader = reader
            .as_any_mut()
            .downcast_mut::<img_io::yuv::YuvReader>()
            .context("Internal error: expected YuvReader")?;
        yuv_reader.set_img_props(dims.w, dims.h, num_comps, &subsampling);
        yuv_reader.set_bit_depth(bit_depth, is_signed);
        reader.open(&args.input)?;
        is_planar = true;
    } else if lower_input.ends_with(".rawl") || lower_input.ends_with(".raw") {
        // RAWL requires external dimensions
        let dims = args.dims.context("-dims required for RAWL input")?;
        let num_comps = args.num_comps.unwrap_or(1);
        let bit_depth = args.bit_depth.unwrap_or(8);
        let is_signed = args.signed.unwrap_or(false);

        let rawl_reader = reader
            .as_any_mut()
            .downcast_mut::<img_io::rawl::RawlReader>()
            .context("Internal error: expected RawlReader")?;
        rawl_reader.set_img_props(dims.w, dims.h, num_comps, bit_depth, is_signed);
        reader.open(&args.input)?;
        is_planar = true;
    } else {
        // PPM, PGM, DPX — self-describing formats
        reader.open(&args.input)?;
        is_planar = false;
    }

    let num_comps = reader.get_num_components();
    let width = reader.get_width();
    let height = reader.get_height();

    // --- Configure codestream ---
    let mut codestream = Codestream::new();

    // SIZ parameters
    {
        let siz = codestream.access_siz_mut();
        siz.set_image_extent(Point::new(
            image_offset.x + width,
            image_offset.y + height,
        ));
        siz.set_image_offset(image_offset);
        siz.set_tile_offset(tile_offset);

        if let Some(ts) = args.tile_size {
            siz.set_tile_size(ts);
        }

        siz.set_num_components(num_comps);
        for c in 0..num_comps {
            let (dx, dy) = reader.get_downsampling(c);
            siz.set_comp_info(
                c,
                Point::new(dx, dy),
                reader.get_bit_depth(c),
                reader.is_signed(c),
            );
        }
    }

    // COD parameters
    {
        let cod = codestream.access_cod_mut();
        cod.set_num_decomposition(args.num_decomps);
        cod.set_block_dims(args.block_size.w, args.block_size.h);
        cod.set_reversible(args.reversible);

        if let Some(ref precincts) = args.precincts {
            cod.set_precinct_size(precincts.len() as i32, precincts);
        }

        cod.set_progression_order(&args.prog_order)
            .map_err(|e| anyhow::anyhow!("Invalid progression order '{}': {:?}", args.prog_order, e))?;

        if let Some(ct) = args.colour_trans {
            cod.set_color_transform(ct);
        }
    }

    // QCD: quantization step for lossy
    if !args.reversible {
        if let Some(qstep) = args.qstep {
            codestream.access_qcd_mut().set_delta(qstep);
        }
    }

    // Configuration flags
    codestream.set_planar(if is_planar { 1 } else { 0 });
    codestream.request_tlm_marker(args.tlm_marker);

    if let Some(ref tp) = args.tilepart_divs {
        let val = match tp.as_str() {
            "R" => 1,
            "C" => 2,
            "RC" => 3,
            _ => bail!("Invalid tilepart_divs: '{}' (expected R, C, or RC)", tp),
        };
        codestream.set_tilepart_divisions(val);
    }

    if let Some(ref profile) = args.profile {
        codestream.set_profile(profile)
            .map_err(|e| anyhow::anyhow!("Invalid profile '{}': {:?}", profile, e))?;
    }

    // --- Write headers ---
    let mut outfile = J2cOutfile::open(&args.output)
        .map_err(|e| anyhow::anyhow!("Cannot create output '{}': {:?}", args.output, e))?;

    let mut comments = Vec::new();
    if let Some(ref com_str) = args.com {
        let mut ce = CommentExchange::default();
        ce.set_string(com_str);
        comments.push(ce);
    }

    codestream.write_headers(&mut outfile, &comments)
        .map_err(|e| anyhow::anyhow!("Failed to write codestream headers: {:?}", e))?;

    // --- Compression loop ---
    // NOTE: The actual exchange/push API on Codestream is not yet fully wired.
    // For now, we read all image data to validate I/O works, and log progress.
    // The real compression loop will use codestream.exchange() once available.

    if is_planar {
        // Planar: read each component fully, one row at a time
        for c in 0..num_comps {
            let (_, dy) = reader.get_downsampling(c);
            let comp_height = openjph_core::types::div_ceil(height, dy);
            for _row in 0..comp_height {
                let _line = reader.read_line(c)?;
                // TODO: codestream.exchange(line, next_comp) once available
            }
        }
    } else {
        // Interleaved: read line-by-line, component-by-component
        for _row in 0..height {
            for c in 0..num_comps {
                let _line = reader.read_line(c)?;
                // TODO: codestream.exchange(line, next_comp) once available
            }
        }
    }

    // TODO: codestream.flush() and codestream.close() once available
    reader.close();

    eprintln!(
        "ojph_compress: read {}x{} image ({} components, {} bpp) -> {}",
        width, height, num_comps,
        reader.get_bit_depth(0),
        args.output,
    );
    eprintln!("Note: actual HTJ2K encoding will be completed when codestream.exchange() is wired.");

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
