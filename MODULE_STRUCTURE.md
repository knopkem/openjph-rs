# OpenJPH-RS Transform Module Structure

## Current Module Exports

### From lib.rs
```rust
pub mod transform;  // Already exported in lib.rs (line 15)
```

### From types.rs (already exported via lib.rs)
```rust
pub use types::*;
```
Includes:
- `Ui8`, `Si8`, `Ui16`, `Si16`, `Ui32`, `Si32`, `Ui64`, `Si64`
- `Size`, `Point`, `Rect`
- `NUM_FRAC_BITS`, `OPENJPH_VERSION_*`
- Helper functions: `div_ceil`, `ojph_max`, `ojph_min`

## What You Have Available

### Memory Module (`crate::mem`)
```rust
pub struct AlignedVec<T> { ... }
pub struct LineBuf { ... }
pub enum LineBufData { I32(*mut i32), I64(*mut i64), F32(*mut f32), None }
pub struct LiftingBuf { ... }
pub struct MemFixedAllocator { ... }
pub struct MemElasticAllocator { ... }

pub const LFT_UNDEFINED: u32;
pub const LFT_32BIT: u32;
pub const LFT_64BIT: u32;
pub const LFT_INTEGER: u32;
pub const LFT_SIZE_MASK: u32;
```

### Architecture Module (`crate::arch`)
```rust
pub enum CpuExtLevel { Generic, Sse, Sse2, ..., Avx512 }
pub enum ArmCpuExtLevel { Generic, Neon, Sve, Sve2 }
pub fn get_cpu_ext_level() -> i32;

pub const BYTE_ALIGNMENT: u32;
pub const LOG_BYTE_ALIGNMENT: u32;
pub const OBJECT_ALIGNMENT: u32;

pub fn population_count(val: u32) -> u32;
pub fn count_leading_zeros(val: u32) -> u32;
pub fn count_leading_zeros_u64(val: u64) -> u32;
pub fn count_trailing_zeros(val: u32) -> u32;
pub fn ojph_round(val: f32) -> i32;
pub fn ojph_trunc(val: f32) -> i32;
pub fn calc_aligned_size<T>(count: usize, alignment: u32) -> usize;
```

### Error Module (`crate::error`)
```rust
pub enum OjphError { ... }
pub type Result<T> = std::result::Result<T, OjphError>;
```

## Proposed Module Structure

### transform/mod.rs (Public API Layer)
```rust
//! Wavelet and color transforms (DWT 5/3, 9/7, RCT, ICT).

pub(crate) mod wavelet;
pub(crate) mod colour;
pub(crate) mod simd;

// Public types
pub struct WaveletTransform { /* params for DWT */ }
pub struct ColorTransform { /* params for RCT/ICT */ }

// Public API functions
pub fn dwt_5_3_forward(...) -> Result<()>;
pub fn dwt_5_3_reverse(...) -> Result<()>;
pub fn dwt_9_7_forward(...) -> Result<()>;
pub fn dwt_9_7_reverse(...) -> Result<()>;
pub fn rct_forward(...) -> Result<()>;
pub fn rct_reverse(...) -> Result<()>;
pub fn ict_forward(...) -> Result<()>;
pub fn ict_reverse(...) -> Result<()>;
```

### transform/wavelet.rs (DWT Implementations)
```rust
//! Discrete wavelet transform (DWT) — 5/3 reversible and 9/7 irreversible.

use crate::{
    error::Result,
    mem::{LineBuf, LiftingBuf},
};

/// 5/3 reversible DWT - forward transform (analysis)
pub fn dwt_5_3_forward(
    in_buf: &LineBuf,
    out_lo: &mut LineBuf,  // Even-indexed samples
    out_hi: &mut LineBuf,  // Odd-indexed samples
) -> Result<()> {
    // Lifting-based implementation
    todo!()
}

/// 5/3 reversible DWT - inverse transform (synthesis)
pub fn dwt_5_3_reverse(
    in_lo: &LineBuf,
    in_hi: &LineBuf,
    out_buf: &mut LineBuf,
) -> Result<()> {
    todo!()
}

// ... similar for 9/7 ...
```

### transform/colour.rs (Color Transform Implementations)
```rust
//! Colour transforms — RCT (reversible) and ICT (irreversible).

use crate::{
    error::Result,
    mem::LineBuf,
};

/// Reversible Component Transform (3 components)
pub fn rct_forward(
    in_r: &LineBuf,
    in_g: &LineBuf,
    in_b: &LineBuf,
    out_y: &mut LineBuf,
    out_cb: &mut LineBuf,
    out_cr: &mut LineBuf,
) -> Result<()> {
    todo!()
}

pub fn rct_reverse(
    in_y: &LineBuf,
    in_cb: &LineBuf,
    in_cr: &LineBuf,
    out_r: &mut LineBuf,
    out_g: &mut LineBuf,
    out_b: &mut LineBuf,
) -> Result<()> {
    todo!()
}

/// Irreversible Component Transform (3 components)
pub fn ict_forward(...) -> Result<()> { todo!() }
pub fn ict_reverse(...) -> Result<()> { todo!() }
```

### transform/simd/mod.rs (SIMD Dispatch)
```rust
//! SIMD-accelerated transform routines.

use crate::arch::get_cpu_ext_level;

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    mod avx2 { /* implementations */ }
    mod avx512 { /* implementations */ }
}

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    mod neon { /* implementations */ }
    mod sve { /* implementations */ }
}

pub fn dwt_5_3_forward_simd(...) -> Result<()> {
    // Runtime dispatch based on CPU features
    match get_cpu_ext_level() {
        level if level >= CpuExtLevel::Avx512 as i32 => {
            #[cfg(all(target_arch = "x86_64", feature = "avx512"))]
            return x86_64::avx512::dwt_5_3_forward(...);
        }
        level if level >= CpuExtLevel::Avx2 as i32 => {
            #[cfg(all(target_arch = "x86_64", feature = "simd"))]
            return x86_64::avx2::dwt_5_3_forward(...);
        }
        _ => {
            // Fallback to scalar
            super::wavelet::dwt_5_3_forward(...)
        }
    }
}
```

## Integration Point: codestream/tile_comp.rs

Current status: No transform references

Proposed integration:
```rust
use crate::transform;

impl TileComponent {
    fn decompose(&mut self) -> Result<()> {
        // After reading input samples, apply transforms
        
        // 1. Color transform (if needed)
        if self.params.color_transform_enabled {
            transform::rct_forward(&r, &g, &b, ...)?;
        }
        
        // 2. Wavelet decomposition (per level)
        for level in 0..num_levels {
            let dwt_op = if reversible {
                transform::dwt_5_3_forward
            } else {
                transform::dwt_9_7_forward
            };
            dwt_op(&input, &output_lo, &output_hi)?;
        }
        
        Ok(())
    }
}
```

## Type Compatibility

### Using LineBuf with transforms
```rust
// LineBuf stores discriminated pointer
match buf.data {
    LineBufData::I32(ptr) => {
        // Process i32 samples
        unsafe { /* lifting operations */ }
    }
    LineBufData::F32(ptr) => {
        // Process f32 samples (for 9/7)
        unsafe { /* lifting operations */ }
    }
    _ => return Err(OjphError::InvalidFormat),
}

// Check element size
if (buf.flags & LFT_SIZE_MASK) == LFT_32BIT {
    // 32-bit elements
}
```

### Feature Gates
```rust
#[cfg(feature = "simd")]
use crate::transform::simd;

#[cfg(all(feature = "avx512", target_arch = "x86_64"))]
unsafe {
    // AVX-512 specific code
}

#[cfg(all(feature = "simd", target_arch = "aarch64"))]
// ARM NEON code
```

## Summary of Dependencies

**Your transform module will need:**

```rust
use crate::{
    error::{OjphError, Result},
    mem::{AlignedVec, LineBuf, LineBufData, LiftingBuf, 
          LFT_32BIT, LFT_64BIT, LFT_INTEGER, LFT_SIZE_MASK},
    arch::{get_cpu_ext_level, CpuExtLevel, ArmCpuExtLevel,
           BYTE_ALIGNMENT, ojph_round},
    types::{NUM_FRAC_BITS, div_ceil, ojph_max, ojph_min},
};

// No external crate dependencies for basic implementation
// Only thiserror (already a workspace dependency)
```

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dwt_5_3_reversibility() {
        // Forward then reverse should equal identity
        let input = create_test_line(256);
        let mut forward = LineBuf::new();
        let mut lo = LineBuf::new();
        let mut hi = LineBuf::new();
        
        dwt_5_3_forward(&input, &mut lo, &mut hi)?;
        dwt_5_3_reverse(&lo, &hi, &mut forward)?;
        
        assert_eq!(input, forward);
    }

    #[test]
    fn rct_invertibility() {
        // Forward then reverse should equal identity
        // Test with various color values
    }

    #[cfg(feature = "simd")]
    #[test]
    fn simd_vs_scalar_consistency() {
        // SIMD results should match scalar implementation
    }
}
```

## Next Steps

1. ✏️ Implement scalar versions first (wavelet.rs, colour.rs)
2. 🧪 Write comprehensive tests
3. ⚡ Add SIMD variants (simd/mod.rs)
4. 🔌 Integrate with codestream processing
5. 📊 Performance profiling & benchmarking
6. 📚 Documentation & examples
