//! x86 AVX2-accelerated codestream sample conversion routines.
//!
//! All code gated behind `#[cfg(target_arch = "x86_64")]`.

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

/// AVX2-accelerated signed-to-unsigned sample conversion (8-wide).
#[cfg(target_arch = "x86_64")]
pub(crate) fn avx2_convert_signed_to_unsigned(
    src: &[i32],
    dst: &mut [i32],
    width: u32,
    bit_depth: u32,
) {
    unsafe {
        avx2_s2u_inner(src, dst, width, bit_depth);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn avx2_s2u_inner(src: &[i32], dst: &mut [i32], width: u32, bit_depth: u32) {
    let offset = 1i32 << (bit_depth - 1);
    let voffset = _mm256_set1_epi32(offset);
    let simd_count = width / 8;
    let remainder = width % 8;
    let mut sp = src.as_ptr();
    let mut dp = dst.as_mut_ptr();

    for _ in 0..simd_count {
        let v = _mm256_loadu_si256(sp as *const __m256i);
        _mm256_storeu_si256(dp as *mut __m256i, _mm256_add_epi32(v, voffset));
        sp = sp.add(8);
        dp = dp.add(8);
    }
    for i in 0..remainder as usize {
        *dp.add(i) = *sp.add(i) + offset;
    }
}

/// AVX2-accelerated unsigned-to-signed sample conversion (8-wide).
#[cfg(target_arch = "x86_64")]
pub(crate) fn avx2_convert_unsigned_to_signed(
    src: &[i32],
    dst: &mut [i32],
    width: u32,
    bit_depth: u32,
) {
    unsafe {
        avx2_u2s_inner(src, dst, width, bit_depth);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn avx2_u2s_inner(src: &[i32], dst: &mut [i32], width: u32, bit_depth: u32) {
    let offset = 1i32 << (bit_depth - 1);
    let voffset = _mm256_set1_epi32(offset);
    let simd_count = width / 8;
    let remainder = width % 8;
    let mut sp = src.as_ptr();
    let mut dp = dst.as_mut_ptr();

    for _ in 0..simd_count {
        let v = _mm256_loadu_si256(sp as *const __m256i);
        _mm256_storeu_si256(dp as *mut __m256i, _mm256_sub_epi32(v, voffset));
        sp = sp.add(8);
        dp = dp.add(8);
    }
    for i in 0..remainder as usize {
        *dp.add(i) = *sp.add(i) - offset;
    }
}
