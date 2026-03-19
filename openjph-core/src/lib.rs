//! OpenJPH-RS: Pure Rust HTJ2K (JPEG 2000 Part 15) codec
//!
//! A 1:1 port of the OpenJPH C++ library (v0.26.3).

pub mod types;
pub mod error;
pub mod message;
pub mod arch;
pub mod mem;
pub mod file;
pub mod arg;
pub mod params;
pub mod codestream;
pub mod coding;
pub mod transform;

pub use types::*;
pub use error::{OjphError, Result};

#[cfg(test)]
mod pipeline_tests {
    use crate::codestream::Codestream;
    use crate::file::{MemOutfile, MemInfile};
    use crate::types::*;

    /// Encode an 8×8 image through the full pipeline, decode it, and verify
    /// the round-trip produces values within CUP tolerance.
    #[test]
    fn roundtrip_8x8_single_component() {
        let width = 8u32;
        let height = 8u32;

        // Use pixel value 131 (level-shifted: 3, magnitude=3, odd → exact with p=1)
        let image: Vec<Vec<i32>> = vec![vec![131i32; width as usize]; height as usize];

        // --- Encode ---
        let mut cs = Codestream::new();
        cs.access_siz_mut().set_image_extent(Point::new(width, height));
        cs.access_siz_mut().set_num_components(1);
        cs.access_siz_mut().set_comp_info(0, Point::new(1, 1), 8, false);
        cs.access_siz_mut().set_tile_size(Size::new(width, height));
        cs.access_cod_mut().set_num_decomposition(1);
        cs.access_cod_mut().set_reversible(true);
        cs.access_cod_mut().set_color_transform(false);
        cs.set_planar(0);

        let mut outfile = MemOutfile::new();
        cs.write_headers(&mut outfile, &[]).unwrap();

        for y in 0..height as usize {
            cs.exchange(&image[y], 0).unwrap();
        }
        cs.flush(&mut outfile).unwrap();

        let encoded = outfile.get_data().to_vec();
        assert!(encoded.len() > 20, "encoded data too small: {} bytes", encoded.len());

        // --- Decode ---
        let mut infile = MemInfile::new(&encoded);
        let mut cs2 = Codestream::new();
        cs2.read_headers(&mut infile).unwrap();
        cs2.create(&mut infile).unwrap();

        for y in 0..height as usize {
            let line = cs2.pull(0).expect("expected decoded line");
            assert_eq!(
                line, image[y],
                "mismatch at row {y}: got {:?}, expected {:?}",
                &line[..], &image[y][..]
            );
        }

        // No more lines
        assert!(cs2.pull(0).is_none());
    }

    /// Encode + decode a flat (constant value) image with exact roundtrip.
    #[test]
    fn roundtrip_8x8_constant() {
        let width = 8u32;
        let height = 8u32;
        // Pixel value 125: level-shifted = -3, magnitude 3 (odd), roundtrips exactly
        let image: Vec<Vec<i32>> = vec![vec![125i32; width as usize]; height as usize];

        let mut cs = Codestream::new();
        cs.access_siz_mut().set_image_extent(Point::new(width, height));
        cs.access_siz_mut().set_num_components(1);
        cs.access_siz_mut().set_comp_info(0, Point::new(1, 1), 8, false);
        cs.access_siz_mut().set_tile_size(Size::new(width, height));
        cs.access_cod_mut().set_num_decomposition(1);
        cs.access_cod_mut().set_reversible(true);
        cs.access_cod_mut().set_color_transform(false);
        cs.set_planar(0);

        let mut outfile = MemOutfile::new();
        cs.write_headers(&mut outfile, &[]).unwrap();
        for y in 0..height as usize {
            cs.exchange(&image[y], 0).unwrap();
        }
        cs.flush(&mut outfile).unwrap();

        let encoded = outfile.get_data().to_vec();
        let mut infile = MemInfile::new(&encoded);
        let mut cs2 = Codestream::new();
        cs2.read_headers(&mut infile).unwrap();
        cs2.create(&mut infile).unwrap();

        for y in 0..height as usize {
            let line = cs2.pull(0).expect("expected decoded line");
            assert_eq!(line, image[y], "mismatch at row {y}");
        }
    }

    /// Zero-decomposition (no DWT): test end-to-end with CUP-compatible values.
    #[test]
    fn roundtrip_8x8_no_dwt() {
        let width = 8u32;
        let height = 8u32;
        // Use constant value for no-DWT test (pixel=131, level-shift=3, magnitude=3, p=1)
        let image: Vec<Vec<i32>> = vec![vec![131i32; width as usize]; height as usize];

        let mut cs = Codestream::new();
        cs.access_siz_mut().set_image_extent(Point::new(width, height));
        cs.access_siz_mut().set_num_components(1);
        cs.access_siz_mut().set_comp_info(0, Point::new(1, 1), 8, false);
        cs.access_siz_mut().set_tile_size(Size::new(width, height));
        cs.access_cod_mut().set_num_decomposition(0);
        cs.access_cod_mut().set_reversible(true);
        cs.access_cod_mut().set_color_transform(false);
        cs.set_planar(0);

        let mut outfile = MemOutfile::new();
        cs.write_headers(&mut outfile, &[]).unwrap();
        for y in 0..height as usize {
            cs.exchange(&image[y], 0).unwrap();
        }
        cs.flush(&mut outfile).unwrap();

        let encoded = outfile.get_data().to_vec();
        let mut infile = MemInfile::new(&encoded);
        let mut cs2 = Codestream::new();
        cs2.read_headers(&mut infile).unwrap();
        cs2.create(&mut infile).unwrap();

        for y in 0..height as usize {
            let line = cs2.pull(0).expect("expected decoded line");
            assert_eq!(line, image[y], "mismatch at row {y}");
        }
    }

    /// Block coder roundtrip with corrected missing_msbs computation.
    #[test]
    fn block_coder_roundtrip_magnitude_3() {
        use crate::coding::encoder::encode_codeblock32;
        use crate::coding::decoder32::decode_codeblock32;

        // Magnitude 3 with p=1 should roundtrip exactly
        let mag = 3u32;
        let u32_val = (1u32 << 31) | mag;
        let buf = vec![u32_val; 16];
        let num_bits = 32 - mag.leading_zeros();
        let missing_msbs = 31 - num_bits; // p=1, msbs=29

        let enc = encode_codeblock32(&buf, missing_msbs, 1, 4, 4, 4).unwrap();
        assert!(enc.length >= 2);

        let mut coded = enc.data.clone();
        coded.resize(coded.len() + 16, 0);
        let mut decoded = vec![0u32; 16];
        decode_codeblock32(
            &mut coded, &mut decoded, missing_msbs, 1,
            enc.length, 0, 4, 4, 4, false,
        ).unwrap();

        for i in 0..16 {
            assert_eq!(decoded[i], buf[i],
                "mismatch at {}: got 0x{:08X}, expected 0x{:08X}", i, decoded[i], buf[i]);
        }
    }
}

#[cfg(test)]
mod debug_roundtrip_test {
    use super::*;
    use crate::codestream::Codestream;
    use crate::file::{MemOutfile, MemInfile};
    use crate::types::{Point, Size};

    #[test]
    fn debug_no_dwt_value100() {
        let width = 8u32;
        let height = 8u32;
        let pixels = vec![100i32; (width * height) as usize];

        let mut cs = Codestream::new();
        cs.access_siz_mut().set_image_extent(Point::new(width, height));
        cs.access_siz_mut().set_num_components(1);
        cs.access_siz_mut().set_comp_info(0, Point::new(1, 1), 8, false);
        cs.access_siz_mut().set_tile_size(Size::new(width, height));
        cs.access_cod_mut().set_num_decomposition(0);
        cs.access_cod_mut().set_reversible(true);
        cs.access_cod_mut().set_color_transform(false);
        cs.set_planar(0);

        let mut outfile = MemOutfile::new();
        cs.write_headers(&mut outfile, &[]).unwrap();
        for y in 0..height as usize {
            let start = y * width as usize;
            let end = start + width as usize;
            cs.exchange(&pixels[start..end], 0).unwrap();
        }
        cs.flush(&mut outfile).unwrap();

        let encoded = outfile.get_data().to_vec();
        let mut infile = MemInfile::new(&encoded);
        let mut cs2 = Codestream::new();
        cs2.read_headers(&mut infile).unwrap();
        cs2.create(&mut infile).unwrap();

        for y in 0..height as usize {
            let line = cs2.pull(0).expect("expected decoded line");
            for x in 0..width as usize {
                if line[x] != pixels[y * width as usize + x] {
                    panic!("Mismatch at ({},{}): expected {}, got {}",
                        x, y, pixels[y * width as usize + x], line[x]);
                }
            }
        }
    }
}
