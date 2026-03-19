# OpenJPH-RS Transform Module - Quick Reference

## Files Already Exist

```
openjph-core/src/
├── lib.rs                 → exports pub mod transform
├── types.rs               → Size, Point, Rect, NUM_FRAC_BITS
├── arch.rs                → BYTE_ALIGNMENT=64, CPU detection
├── mem.rs                 → LineBuf, AlignedVec, Allocators
└── transform/
    ├── mod.rs             → pub(crate) mod {wavelet, colour, simd}
    ├── wavelet.rs         → STUB - implement DWT 5/3 & 9/7
    ├── colour.rs          → STUB - implement RCT & ICT
    └── simd/mod.rs        → STUB - SIMD specializations
```

## Core Data Structures Available

### LineBuf - Row of Samples
```rust
pub struct LineBuf {
    pub size: usize,        // # of samples
    pub pre_size: u32,      // filter padding
    pub flags: u32,         // type: LFT_32BIT | LFT_64BIT | LFT_INTEGER
    pub data: LineBufData,  // I32(*mut i32) | I64(*mut i64) | F32(*mut f32) | None
}
```

### AlignedVec<T> - AVX-512 Aligned Buffer
```rust
pub struct AlignedVec<T> {
    // 64-byte aligned by default (BYTE_ALIGNMENT = 64)
}
impl<T> AlignedVec<T> {
    pub fn new() -> Self
    pub fn resize(&mut self, count: usize) -> Result<()>
    pub fn as_ptr(&self) -> *const T
    pub fn as_mut_ptr(&mut self) -> *mut T
}
```

### LiftingBuf - Wavelet Lifting State
```rust
pub struct LiftingBuf {
    pub active: bool,
    pub line_idx: Option<usize>,
}
```

## Constants & Helpers Available

### From arch.rs
```rust
const BYTE_ALIGNMENT: u32 = 64;
const LOG_BYTE_ALIGNMENT: u32 = 6;
fn get_cpu_ext_level() -> i32  // Runtime detection
```

### From types.rs
```rust
const NUM_FRAC_BITS: u32 = 13;
fn div_ceil(a: u32, b: u32) -> u32
fn ojph_round(val: f32) -> i32
fn ojph_trunc(val: f32) -> i32
```

## Integration Checklist

- [ ] Implement `wavelet.rs`: DWT 5/3 & 9/7 (reversible & irreversible)
- [ ] Implement `colour.rs`: RCT (reversible) & ICT (irreversible)
- [ ] Add SIMD variants in `simd/mod.rs` (x86-64 AVX2/AVX-512, ARM NEON)
- [ ] Export public API from `transform/mod.rs`
- [ ] Wire into `codestream/tile_comp.rs` or similar
- [ ] Add feature gates: `#[cfg(feature = "simd")]`, `#[cfg(feature = "avx512")]`
- [ ] Write tests for:
  - Wavelet reversibility (analyze → synthesize = identity)
  - Color transform invertibility
  - SIMD vs scalar consistency

## Processing Pipeline

```
Input Image
    ↓
Color Transform (RCT/ICT)
    ↓
Wavelet Analysis (DWT 5/3 or 9/7)
    ↓
Subbands (LL, LH, HL, HH)
    ↓ → to codestream module
    ↓
Codeblocks → Entropy Coding
```

## Memory Layout Expectations

- **Line buffers:** Pre-size padding + size samples
- **Alignment:** 64-byte (AVX-512 safe)
- **Ownership:** LineBuf borrows from arena allocators
- **Type flags:** Describe element width & signedness

## Workspace Features

```toml
[features]
default = ["simd"]
simd = []         # Enable SIMD paths
avx512 = ["simd"] # Enable AVX-512 specifically
```

Use: `#[cfg(feature = "simd")]` for conditional compilation

## Key Files Path Reference

| Purpose | Path |
|---------|------|
| Public API | `/openjph-core/src/transform/mod.rs` |
| Wavelet DWT | `/openjph-core/src/transform/wavelet.rs` |
| Color RCT/ICT | `/openjph-core/src/transform/colour.rs` |
| SIMD dispatch | `/openjph-core/src/transform/simd/mod.rs` |
| Data structures | `/openjph-core/src/mem.rs` |
| CPU detection | `/openjph-core/src/arch.rs` |
| Integration point | `/openjph-core/src/codestream/tile_comp.rs` |

## See Also

- Full analysis: `EXPLORATION_SUMMARY.md` (17 KB)
- Upstream C++: https://github.com/AousNaman/OpenJPH
