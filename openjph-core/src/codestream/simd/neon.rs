//! NEON-accelerated codestream sample conversion routines for AArch64.

#![allow(dead_code)]

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;

/// NEON-accelerated signed-to-unsigned sample conversion.
///
/// Adds `2^(bit_depth-1)` to each sample.
#[cfg(target_arch = "aarch64")]
pub(crate) fn neon_convert_signed_to_unsigned(
    src: &[i32],
    dst: &mut [i32],
    width: u32,
    bit_depth: u32,
) {
    unsafe {
        neon_s2u_inner(src, dst, width, bit_depth);
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn neon_s2u_inner(src: &[i32], dst: &mut [i32], width: u32, bit_depth: u32) {
    let offset = 1i32 << (bit_depth - 1);
    let voffset = vdupq_n_s32(offset);
    let simd_count = width / 4;
    let remainder = width % 4;
    let mut sp = src.as_ptr();
    let mut dp = dst.as_mut_ptr();

    for _ in 0..simd_count {
        let v = vld1q_s32(sp);
        vst1q_s32(dp, vaddq_s32(v, voffset));
        sp = sp.add(4);
        dp = dp.add(4);
    }
    for i in 0..remainder as usize {
        *dp.add(i) = *sp.add(i) + offset;
    }
}

/// NEON-accelerated unsigned-to-signed sample conversion.
///
/// Subtracts `2^(bit_depth-1)` from each sample.
#[cfg(target_arch = "aarch64")]
pub(crate) fn neon_convert_unsigned_to_signed(
    src: &[i32],
    dst: &mut [i32],
    width: u32,
    bit_depth: u32,
) {
    unsafe {
        neon_u2s_inner(src, dst, width, bit_depth);
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn neon_u2s_inner(src: &[i32], dst: &mut [i32], width: u32, bit_depth: u32) {
    let offset = 1i32 << (bit_depth - 1);
    let voffset = vdupq_n_s32(offset);
    let simd_count = width / 4;
    let remainder = width % 4;
    let mut sp = src.as_ptr();
    let mut dp = dst.as_mut_ptr();

    for _ in 0..simd_count {
        let v = vld1q_s32(sp);
        vst1q_s32(dp, vsubq_s32(v, voffset));
        sp = sp.add(4);
        dp = dp.add(4);
    }
    for i in 0..remainder as usize {
        *dp.add(i) = *sp.add(i) - offset;
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[cfg(target_arch = "aarch64")]
mod tests {
    use super::*;
    use crate::codestream::simd::{gen_convert_signed_to_unsigned, gen_convert_unsigned_to_signed};

    #[test]
    fn neon_s2u_matches_scalar() {
        for width in [1, 3, 4, 7, 8, 16, 33, 64] {
            for bit_depth in [8, 12, 16] {
                let src: Vec<i32> = (0..width).map(|i| (i as i32) * 3 - 100).collect();
                let mut scalar_dst = vec![0i32; width];
                let mut neon_dst = vec![0i32; width];

                gen_convert_signed_to_unsigned(&src, &mut scalar_dst, width as u32, bit_depth);
                neon_convert_signed_to_unsigned(&src, &mut neon_dst, width as u32, bit_depth);

                assert_eq!(
                    scalar_dst, neon_dst,
                    "s2u mismatch width={width} bd={bit_depth}"
                );
            }
        }
    }

    #[test]
    fn neon_u2s_matches_scalar() {
        for width in [1, 3, 4, 7, 8, 16, 33, 64] {
            for bit_depth in [8, 12, 16] {
                let src: Vec<i32> = (0..width).map(|i| (i as i32) * 5 + 50).collect();
                let mut scalar_dst = vec![0i32; width];
                let mut neon_dst = vec![0i32; width];

                gen_convert_unsigned_to_signed(&src, &mut scalar_dst, width as u32, bit_depth);
                neon_convert_unsigned_to_signed(&src, &mut neon_dst, width as u32, bit_depth);

                assert_eq!(
                    scalar_dst, neon_dst,
                    "u2s mismatch width={width} bd={bit_depth}"
                );
            }
        }
    }

    #[test]
    fn neon_roundtrip_s2u_u2s() {
        let width = 64;
        let bit_depth = 8;
        let src: Vec<i32> = (0..width).map(|i| (i as i32) - 32).collect();
        let mut unsigned = vec![0i32; width];
        let mut back = vec![0i32; width];

        neon_convert_signed_to_unsigned(&src, &mut unsigned, width as u32, bit_depth);
        neon_convert_unsigned_to_signed(&unsigned, &mut back, width as u32, bit_depth);

        assert_eq!(src, back);
    }
}
