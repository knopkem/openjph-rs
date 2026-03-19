//! ojph_expand — HTJ2K decompression CLI tool.
//!
//! Port of `ojph_expand.cpp`. Reads JPEG 2000 Part 15 (HTJ2K) codestreams
//! and writes output images (PPM, PGM, YUV, RAWL).

mod img_io;

use anyhow::{bail, Result};
use clap::Parser;
use openjph_core::codestream::Codestream;
use openjph_core::file::J2cInfile;

use img_io::ImageWriter;

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

/// HTJ2K image decompression tool
#[derive(Parser, Debug)]
#[command(
    name = "ojph_expand",
    about = "Decompress HTJ2K (JPEG 2000 Part 15) codestreams to images"
)]
struct Args {
    /// Input codestream file (.j2c, .jph)
    #[arg(short = 'i', long = "input")]
    input: String,

    /// Output image file (PPM, PGM, YUV, RAWL)
    #[arg(short = 'o', long = "output")]
    output: String,

    /// Number of resolutions to skip (reduces decoded resolution)
    #[arg(long = "skip_res", default_value_t = 0)]
    skip_res: u32,

    /// Enable error resilience
    #[arg(long = "resilient", default_value_t = false)]
    resilient: bool,
}

// ---------------------------------------------------------------------------
// Main decompression logic
// ---------------------------------------------------------------------------

fn run() -> Result<()> {
    let args = Args::parse();

    // --- Read codestream headers ---
    let mut codestream = Codestream::new();

    let mut infile = J2cInfile::open(&args.input)
        .map_err(|e| anyhow::anyhow!("Cannot open '{}': {:?}", args.input, e))?;

    codestream
        .read_headers(&mut infile)
        .map_err(|e| anyhow::anyhow!("Failed to read codestream headers: {:?}", e))?;

    if args.resilient {
        codestream.enable_resilience();
    }

    if args.skip_res > 0 {
        codestream.restrict_input_resolution(args.skip_res, args.skip_res);
    }

    // --- Extract image parameters from SIZ ---
    // Extract all needed values first to avoid borrow conflicts
    let num_comps = codestream.access_siz().get_num_components() as u32;
    let employing_ct = codestream.access_cod().is_employing_color_transform();

    // Pre-extract per-component parameters
    let mut recon_widths = Vec::with_capacity(num_comps as usize);
    let mut recon_heights = Vec::with_capacity(num_comps as usize);
    let mut bit_depths = Vec::with_capacity(num_comps as usize);
    let mut downsamplings = Vec::with_capacity(num_comps as usize);
    let mut signed_flags = Vec::with_capacity(num_comps as usize);
    for c in 0..num_comps {
        let siz = codestream.access_siz();
        recon_widths.push(siz.get_recon_width(c));
        recon_heights.push(siz.get_recon_height(c));
        bit_depths.push(siz.get_bit_depth(c));
        downsamplings.push(siz.get_downsampling(c));
        signed_flags.push(siz.is_signed(c));
    }

    let lower_output = args.output.to_lowercase();

    // Determine output format and configure writer
    let mut writer: Box<dyn ImageWriter> = img_io::create_writer(&args.output)?;
    let is_planar;

    if lower_output.ends_with(".pgm") {
        if num_comps != 1 {
            bail!(
                ".pgm output requires exactly 1 component, got {}",
                num_comps
            );
        }
        writer.configure(recon_widths[0], recon_heights[0], 1, bit_depths[0])?;
        writer.open(&args.output)?;
        is_planar = false;
    } else if lower_output.ends_with(".ppm") {
        if num_comps != 3 {
            bail!(
                ".ppm output requires exactly 3 components, got {}",
                num_comps
            );
        }
        // Verify uniform downsampling
        let ds0 = downsamplings[0];
        for ds in &downsamplings[1..num_comps as usize] {
            if *ds != ds0 {
                bail!(".ppm output requires all components to have the same downsampling");
            }
        }
        writer.configure(recon_widths[0], recon_heights[0], 3, bit_depths[0])?;
        writer.open(&args.output)?;
        is_planar = false;
    } else if lower_output.ends_with(".yuv") {
        if num_comps != 1 && num_comps != 3 {
            bail!(".yuv output requires 1 or 3 components, got {}", num_comps);
        }
        if employing_ct {
            bail!(".yuv output does not support colour transform");
        }
        let max_bit_depth = bit_depths.iter().copied().max().unwrap_or(8);

        // Downcast to configure YuvWriter with per-component widths
        let yuv_writer = writer
            .as_any_mut()
            .downcast_mut::<img_io::yuv::YuvWriter>()
            .ok_or_else(|| anyhow::anyhow!("Internal error: expected YuvWriter"))?;
        yuv_writer.configure_with_comp_widths(max_bit_depth, num_comps, &recon_widths);
        writer.open(&args.output)?;
        is_planar = true;
    } else if lower_output.ends_with(".rawl") || lower_output.ends_with(".raw") {
        if num_comps != 1 {
            bail!(
                ".rawl output requires exactly 1 component, got {}",
                num_comps
            );
        }

        let rawl_writer = writer
            .as_any_mut()
            .downcast_mut::<img_io::rawl::RawlWriter>()
            .ok_or_else(|| anyhow::anyhow!("Internal error: expected RawlWriter"))?;
        rawl_writer.configure_with_sign(signed_flags[0], bit_depths[0], recon_widths[0]);
        writer.open(&args.output)?;
        is_planar = true;
    } else {
        bail!("Unsupported output format: {}", args.output);
    }

    codestream.set_planar(if is_planar { 1 } else { 0 });

    // --- Decompression loop ---
    // NOTE: The actual pull API on Codestream is not yet fully wired.
    // The real decompression loop will use codestream.create() + codestream.pull()
    // once available. For now we validate I/O setup.

    eprintln!(
        "ojph_expand: configured {}x{} ({} components) -> {}",
        recon_widths[0], recon_heights[0], num_comps, args.output,
    );
    eprintln!("Note: actual HTJ2K decoding will be completed when codestream.pull() is wired.");

    // TODO: Decompression loop
    // codestream.create()?;
    // if is_planar {
    //     for c in 0..num_comps {
    //         let height = siz.get_recon_height(c);
    //         for _ in 0..height {
    //             let (comp_num, line) = codestream.pull()?;
    //             writer.write_line(comp_num, line)?;
    //         }
    //     }
    // } else {
    //     let height = siz.get_recon_height(0);
    //     for _ in 0..height {
    //         for c in 0..num_comps {
    //             let (comp_num, line) = codestream.pull()?;
    //             writer.write_line(comp_num, line)?;
    //         }
    //     }
    // }

    writer.close()?;

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
