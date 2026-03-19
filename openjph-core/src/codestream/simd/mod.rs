//! SIMD-accelerated codestream routines.
//!
//! Generic scalar sample conversion functions (port of ojph_codestream_gen.cpp)
//! and platform-specific SIMD implementations.

#[cfg(target_arch = "aarch64")]
pub(crate) mod neon;

#[cfg(target_arch = "x86_64")]
pub(crate) mod x86;

/// Generic signed-to-unsigned conversion for samples.
/// Converts signed integer samples to unsigned by adding an offset of 2^(bit_depth-1).
#[inline]
pub fn gen_convert_signed_to_unsigned(
    src: &[i32],
    dst: &mut [i32],
    width: u32,
    bit_depth: u32,
) {
    let offset = 1i32 << (bit_depth - 1);
    for i in 0..width as usize {
        dst[i] = src[i] + offset;
    }
}

/// Generic unsigned-to-signed conversion for samples.
/// Converts unsigned integer samples to signed by subtracting 2^(bit_depth-1).
#[inline]
pub fn gen_convert_unsigned_to_signed(
    src: &[i32],
    dst: &mut [i32],
    width: u32,
    bit_depth: u32,
) {
    let offset = 1i32 << (bit_depth - 1);
    for i in 0..width as usize {
        dst[i] = src[i] - offset;
    }
}
