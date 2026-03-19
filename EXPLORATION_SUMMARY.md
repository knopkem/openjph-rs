# OpenJPH-RS Workspace Exploration Summary

## OVERVIEW

The OpenJPH-RS workspace is a **pure Rust port of OpenJPH** (C++ v0.26.3), implementing HTJ2K (JPEG 2000 Part 15) codec.

**Status:** Transform module framework is in place but currently STUB-ONLY with no implementation.

---

## 1. COMPLETE DIRECTORY STRUCTURE

```
/Users/macair/projects/dicom/openjph-rs/
├── Cargo.toml                          (workspace root)
├── Cargo.lock
├── openjph-core/                       (core codec library)
│   ├── Cargo.toml
│   ├── benches/
│   │   └── codec.rs
│   └── src/
│       ├── lib.rs                      (public API)
│       ├── types.rs                    (fundamental types)
│       ├── arch.rs                     (CPU detection, alignment)
│       ├── arg.rs
│       ├── error.rs
│       ├── file.rs
│       ├── mem.rs                      (memory utilities)
│       ├── message.rs
│       ├── codestream/                 (JPEG 2000 parsing)
│       │   ├── mod.rs
│       │   ├── local.rs
│       │   ├── bitbuffer_read.rs
│       │   ├── bitbuffer_write.rs
│       │   ├── tile.rs
│       │   ├── tile_comp.rs
│       │   ├── resolution.rs
│       │   ├── subband.rs
│       │   ├── precinct.rs
│       │   ├── codeblock.rs
│       │   ├── codeblock_fun.rs
│       │   └── simd/
│       │       └── mod.rs
│       ├── coding/                     (arithmetic coding)
│       │   ├── mod.rs
│       │   ├── common.rs
│       │   ├── tables.rs
│       │   ├── encoder.rs
│       │   ├── decoder32.rs
│       │   ├── decoder64.rs
│       │   └── simd/
│       │       └── mod.rs
│       ├── params/                     (JPEG 2000 parameters)
│       │   ├── mod.rs
│       │   └── local.rs
│       └── transform/                  ⭐ TARGET MODULE (STUBS ONLY)
│           ├── mod.rs
│           ├── wavelet.rs              (DWT 5/3, 9/7)
│           ├── colour.rs               (RCT, ICT)
│           └── simd/
│               └── mod.rs              (SIMD specializations)
├── openjph-cli/                        (command-line tools)
│   ├── Cargo.toml
│   └── src/
│       ├── compress.rs
│       └── expand.rs
├── tests/
└── target/
```

---

## 2. CARGO.TOML FILES

### Root Workspace Manifest
**File:** `/Users/macair/projects/dicom/openjph-rs/Cargo.toml`

```toml
[workspace]
members = ["openjph-core", "openjph-cli"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.78"
license = "BSD-2-Clause"
repository = "https://github.com/AousNaman/OpenJPH"
description = "Pure Rust implementation of HTJ2K (JPEG 2000 Part 15) codec"

[workspace.dependencies]
thiserror = "2"
anyhow = "1"
clap = { version = "4", features = ["derive"] }

[profile.release]
lto = true
opt-level = 3
```

### Core Library Manifest
**File:** `/Users/macair/projects/dicom/openjph-rs/openjph-core/Cargo.toml`

```toml
[package]
name = "openjph-core"
description = "HTJ2K (JPEG 2000 Part 15) codec library — pure Rust port of OpenJPH"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
thiserror = { workspace = true }

[dev-dependencies]
criterion = "0.5"

[features]
default = ["simd"]
simd = []
avx512 = ["simd"]

[[bench]]
name = "codec"
harness = false
```

### CLI Manifest
**File:** `/Users/macair/projects/dicom/openjph-rs/openjph-cli/Cargo.toml`

```toml
[package]
name = "openjph-cli"
description = "HTJ2K (JPEG 2000 Part 15) command-line compress/expand tools"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
openjph-core = { path = "../openjph-core" }
anyhow = { workspace = true }
clap = { workspace = true }

[[bin]]
name = "ojph_compress"
path = "src/compress.rs"

[[bin]]
name = "ojph_expand"
path = "src/expand.rs"
```

---

## 3. openjph-core/src/lib.rs

```rust
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
```

---

## 4. EXISTING TRANSFORM MODULE FILES

### openjph-core/src/transform/mod.rs

```rust
//! Wavelet and color transforms (DWT 5/3, 9/7, RCT, ICT).

pub(crate) mod wavelet;
pub(crate) mod colour;
pub(crate) mod simd;
```

**Status:** Framework only - declares submodules but no public API yet

### openjph-core/src/transform/wavelet.rs

```rust
//! Discrete wavelet transform (DWT) — 5/3 reversible and 9/7 irreversible.
```

**Status:** STUB - no implementation

### openjph-core/src/transform/colour.rs

```rust
//! Colour transforms — RCT (reversible) and ICT (irreversible).
```

**Status:** STUB - no implementation

### openjph-core/src/transform/simd/mod.rs

```rust
//! SIMD-accelerated transform routines.
```

**Status:** STUB - no implementation

---

## 5. openjph-core/src/types.rs

**File:** `/Users/macair/projects/dicom/openjph-rs/openjph-core/src/types.rs` (165 lines total)

### Numeric Type Aliases (C++ compatible naming)
```rust
pub type Ui8 = u8;    // Unsigned 8-bit
pub type Si8 = i8;    // Signed 8-bit
pub type Ui16 = u16;  // Unsigned 16-bit
pub type Si16 = i16;  // Signed 16-bit
pub type Ui32 = u32;  // Unsigned 32-bit
pub type Si32 = i32;  // Signed 32-bit
pub type Ui64 = u64;  // Unsigned 64-bit
pub type Si64 = i64;  // Signed 64-bit
```

### Version Constants
```rust
pub const OPENJPH_VERSION_MAJOR: u32 = 0;
pub const OPENJPH_VERSION_MINOR: u32 = 26;
pub const OPENJPH_VERSION_PATCH: u32 = 3;
```

### Codec Constants
```rust
pub const NUM_FRAC_BITS: u32 = 13;  // Fixed-point fractional bits
```

### Helper Functions
```rust
pub const fn div_ceil(a: u32, b: u32) -> u32        // ⌈a / b⌉
pub const fn ojph_max(a: i32, b: i32) -> i32
pub const fn ojph_min(a: i32, b: i32) -> i32
```

### Geometric Primitives
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}
impl Size {
    pub const fn new(w: u32, h: u32) -> Self
    pub const fn area(&self) -> u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}
impl Point {
    pub const fn new(x: u32, y: u32) -> Self
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub org: Point,
    pub siz: Size,
}
impl Rect {
    pub const fn new(org: Point, siz: Size) -> Self
    pub const fn area(&self) -> u64
}
```

---

## 6. openjph-core/src/arch.rs

**File:** `/Users/macair/projects/dicom/openjph-rs/openjph-core/src/arch.rs` (239 lines total)

### Alignment Constants
```rust
pub const BYTE_ALIGNMENT: u32 = 64;        // AVX-512 aligned (64 bytes)
pub const LOG_BYTE_ALIGNMENT: u32 = 6;    // log₂(64)
pub const OBJECT_ALIGNMENT: u32 = 8;
```

### x86-64 CPU Extension Detection
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i32)]
pub enum CpuExtLevel {
    Generic = 0,
    Mmx = 1,
    Sse = 2,
    Sse2 = 3,
    Sse3 = 4,
    Ssse3 = 5,
    Sse41 = 6,
    Sse42 = 7,
    Avx = 8,
    Avx2 = 9,
    Avx2Fma = 10,
    Avx512 = 11,
}

#[cfg(target_arch = "x86_64")]
pub fn get_cpu_ext_level() -> i32  // Runtime detection
```

### ARM CPU Extension Detection
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i32)]
pub enum ArmCpuExtLevel {
    Generic = 0,
    Neon = 1,
    Sve = 2,
    Sve2 = 3,
}

#[cfg(target_arch = "aarch64")]
pub fn get_cpu_ext_level() -> i32  // Always returns Neon for aarch64
```

### Bit Manipulation Helpers
```rust
pub const fn population_count(val: u32) -> u32
pub const fn count_leading_zeros(val: u32) -> u32
pub const fn count_leading_zeros_u64(val: u64) -> u32
pub const fn count_trailing_zeros(val: u32) -> u32
```

### Rounding Helpers
```rust
pub fn ojph_round(val: f32) -> i32        // Ties away from zero
pub fn ojph_trunc(val: f32) -> i32        // Toward zero
```

### Alignment Helper
```rust
pub const fn calc_aligned_size<T>(count: usize, alignment: u32) -> usize
// Calculate smallest multiple of alignment that holds count elements of type T
```

---

## 7. REFERENCES TO "transform", "colour", "color" IN CODEBASE

### Found References:

1. **openjph-core/src/transform/mod.rs** (header comment)
   - "Wavelet and color transforms (DWT 5/3, 9/7, RCT, ICT)"

2. **openjph-core/src/transform/wavelet.rs** (header)
   - "Discrete wavelet transform (DWT) — 5/3 reversible and 9/7 irreversible"

3. **openjph-core/src/transform/colour.rs** (header)
   - "Colour transforms — RCT (reversible) and ICT (irreversible)"

4. **openjph-core/src/mem.rs** (documentation)
   - Line 206: "A single line (row) of sample data used throughout the **wavelet** and coding pipeline"
   - Line 241: "LiftingBuf — lightweight wrapper for **wavelet** lifting steps"

### NOT FOUND in:
- `codestream/` module files
- `params/` module files
- `coding/` module files

**Conclusion:** The transform module is structurally defined but **completely disconnected** from the codestream processing pipeline. It's a planned feature with no integration yet.

---

## 8. CODESTREAM MODULE STRUCTURE

**File:** `/Users/macair/projects/dicom/openjph-rs/openjph-core/src/codestream/mod.rs`

```rust
//! JPEG 2000 codestream parser and generator.

pub mod bitbuffer_read;
pub mod bitbuffer_write;
pub(crate) mod local;
pub(crate) mod tile;
pub(crate) mod tile_comp;
pub(crate) mod resolution;
pub(crate) mod subband;
pub(crate) mod precinct;
pub(crate) mod codeblock;
pub(crate) mod codeblock_fun;
pub(crate) mod simd;
```

### Processing Hierarchy
```
Tile (top-level processing unit)
├── TileComp (per-component processing)
│   ├── Resolution (pyramid level)
│   │   ├── Subband (LL, LH, HL, HH)
│   │   │   ├── Precinct
│   │   │   │   └── Codeblock (smallest unit)
```

**Status:** No imports or references to the `transform` module exist. Transform pipeline needs to be integrated here.

---

## 9. CRITICAL DATA STRUCTURES FOR TRANSFORM INTEGRATION

### LineBuf (Single Row of Sample Data)
**From:** `openjph-core/src/mem.rs` (lines 206-238)

```rust
/// Flags for LineBuf types
pub const LFT_UNDEFINED: u32 = 0x00;  // Not yet allocated
pub const LFT_32BIT: u32 = 0x04;      // 32-bit elements
pub const LFT_64BIT: u32 = 0x08;      // 64-bit elements
pub const LFT_INTEGER: u32 = 0x10;    // Integer (vs float)
pub const LFT_SIZE_MASK: u32 = 0x0F;  // Mask for size field

/// Discriminated pointer to sample data
#[derive(Debug, Clone, Copy)]
pub enum LineBufData {
    I32(*mut i32),   // 32-bit signed integer samples
    I64(*mut i64),   // 64-bit signed integer samples
    F32(*mut f32),   // 32-bit floating-point samples
    None,            // No buffer allocated yet
}

/// A single line (row) of sample data — primary interface for wavelet/coding pipeline
pub struct LineBuf {
    pub size: usize,        // Number of samples in this line
    pub pre_size: u32,      // Extra samples prepended (filter padding)
    pub flags: u32,         // LFT_* flag constants (type descriptor)
    pub data: LineBufData,  // Pointer into backing arena allocator
}
```

**Key Point:** The `data` pointer is **not owned** by LineBuf; it borrows from an arena allocator.

### LiftingBuf (Wavelet Lifting State)
**From:** `openjph-core/src/mem.rs` (lines 244-267)

```rust
/// A line buffer reference used during wavelet lifting steps
pub struct LiftingBuf {
    pub active: bool,           // Is this buffer slot currently active?
    pub line_idx: Option<usize>, // Index into external line-buffer array
}
```

**Design Note:** Uses index-based references instead of mutable references to avoid lifetime issues.

### AlignedVec<T> (AVX-512 Aligned Buffer)
**From:** `openjph-core/src/mem.rs` (lines 17-176)

```rust
/// Heap-allocated, contiguously-stored buffer with guaranteed 64-byte alignment
pub struct AlignedVec<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    alignment: usize,  // Defaults to BYTE_ALIGNMENT (64)
}

impl<T> AlignedVec<T> {
    pub fn new() -> Self                    // Default 64-byte alignment
    pub fn with_alignment(alignment: usize) -> Self  // Custom alignment
    pub fn resize(&mut self, count: usize) -> Result<()>
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
    pub fn as_ptr(&self) -> *const T
    pub fn as_mut_ptr(&mut self) -> *mut T
}

// Implements: Deref, DerefMut, Index, IndexMut, Drop, Default
```

### Memory Allocators
**From:** `openjph-core/src/mem.rs`

#### MemFixedAllocator (Two-phase bump allocator)
```rust
pub struct MemFixedAllocator {
    buf: Vec<u8>,
    offset: usize,
    alignment: usize,
}

impl MemFixedAllocator {
    pub fn pre_alloc_data(&mut self, size: usize, _count: usize)  // Phase 1: register need
    pub fn finalize(&mut self) -> Result<()>                      // Allocate backing buffer
    pub fn alloc_data(&mut self, size: usize) -> Result<*mut u8> // Phase 2: hand out slices
}
```

#### MemElasticAllocator (Arena with growing chunks)
```rust
pub struct MemElasticAllocator {
    chunks: Vec<Vec<u8>>,
    chunk_size: usize,
    cur_offset: usize,
}

impl MemElasticAllocator {
    pub fn new() -> Self
    pub fn with_chunk_size(chunk_size: usize) -> Self
    pub fn alloc_data(&mut self, size: usize) -> Result<*mut u8>
    pub fn reset(&mut self)
}
```

---

## 10. INTEGRATION RECOMMENDATIONS FOR YOUR TRANSFORM MODULE

### Current State
- ✅ Transform module framework exists (`pub(crate) mod transform;` in lib.rs)
- ✅ Submodules declared (wavelet, colour, simd)
- ❌ No public API functions
- ❌ No implementation code
- ❌ Not wired into codestream processing
- ✅ Memory infrastructure ready (LineBuf, AlignedVec, allocators)

### Integration Checklist

1. **Create Public API in `transform/mod.rs`**
   - Export wavelet transform functions
   - Export color transform functions
   - Define parameter structures for DWT/color transforms

2. **Implement Wavelet Transforms (`transform/wavelet.rs`)**
   - DWT 5/3 (reversible): analysis and synthesis
   - DWT 9/7 (irreversible): analysis and synthesis
   - Use `LineBuf` for row-at-a-time processing
   - Support both i32 and f32 sample types

3. **Implement Color Transforms (`transform/colour.rs`)**
   - RCT (Reversible Component Transform): 3-component
   - ICT (Irreversible Component Transform): 3-component
   - Handle component ordering and sign conventions

4. **Add SIMD Specializations (`transform/simd/mod.rs`)**
   - x86-64: AVX2 and AVX-512 variants (use `arch::get_cpu_ext_level()`)
   - ARM: NEON variants
   - Fallback to generic implementations
   - Use `BYTE_ALIGNMENT` (64) for proper register alignment

5. **Wire into Codestream Processing**
   - Likely in `codestream/tile_comp.rs` or similar
   - Call transform functions during:
     - **Encoding:** source image → color transform → wavelet decomposition → subbands
     - **Decoding:** codeblocks → subband reconstruction → wavelet synthesis → color inverse → output

6. **Feature Gating**
   - Respect `#[cfg(feature = "simd")]` for conditional SIMD builds
   - Respect `#[cfg(feature = "avx512")]` for AVX-512 code paths

### Data Flow Example

```
Input Image (RGB or component data)
    ↓
Color Transform (if needed)
    - RCT for reversible coding
    - ICT for irreversible coding
    ↓
Wavelet Analysis (DWT decomposition)
    - 5/3 for reversible
    - 9/7 for irreversible
    ↓
Subband Samples (to codestream module)
    ↓ [During decode]
    ↓
Wavelet Synthesis (DWT reconstruction)
    ↓
Color Inverse Transform
    ↓
Output Image
```

---

## 11. KEY INTEGRATION POINTS

### Files You'll Need to Modify/Create

1. **Create/Expand:** `openjph-core/src/transform/mod.rs`
   - Add public API functions
   - Export types and traits

2. **Implement:** `openjph-core/src/transform/wavelet.rs`
   - DWT functions using LineBuf

3. **Implement:** `openjph-core/src/transform/colour.rs`
   - Color transform functions

4. **Implement:** `openjph-core/src/transform/simd/mod.rs`
   - SIMD dispatch and implementations

5. **Modify:** `openjph-core/src/codestream/tile_comp.rs` (or similar)
   - Call transform functions at appropriate pipeline stage

### No Changes Needed To

- `types.rs` - already has required types
- `arch.rs` - already has CPU detection
- `mem.rs` - already has LineBuf, AlignedVec, allocators
- `error.rs` - use existing error types
- `lib.rs` - transform module already exported

---

## SUMMARY

You have a well-structured, memory-efficient foundation with:
- ✅ Aligned memory allocation (AVX-512 ready at 64 bytes)
- ✅ CPU feature detection (x86-64 and ARM)
- ✅ Line-based buffer interface (LineBuf)
- ✅ Arena allocators (MemFixedAllocator, MemElasticAllocator)
- ✅ Generic transform module skeleton
- ✅ Rust Edition 2021, stable features, clean error handling

What's missing:
- ❌ Wavelet DWT 5/3 and 9/7 implementations
- ❌ Color transform RCT and ICT implementations
- ❌ SIMD specializations
- ❌ Integration with codestream processing pipeline

**Ready to implement!**
