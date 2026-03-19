# OpenJPH-RS Codebase Exploration Summary

## Project Overview
**OpenJPH-RS** is a pure Rust port of the OpenJPH C++ library (v0.26.3), which implements HTJ2K (JPEG 2000 Part 15) codec functionality. The project is a 1:1 port focusing on block encoding/decoding, wavelet transforms, and color space transforms.

**Repository Structure:**
```
/Users/macair/projects/dicom/openjph-rs/
├── openjph-core/          # Main codec library (~11,300 LOC)
├── openjph-cli/           # Command-line tools
├── tests/                 # Test suite (currently empty)
└── Cargo.toml            # Workspace configuration
```

---

## 1. Directory Tree: openjph-core/src/

```
openjph-core/src/
├── lib.rs                      # Main library root, module declarations
├── types.rs                    # Core types (Size, Point, Rect, helpers)
├── error.rs                    # Error handling (OjphError, Result<T>)
├── message.rs                  # Message dispatch system (Info/Warn/Error)
├── arch.rs                     # CPU detection, bit manipulation, alignment
├── file.rs                     # I/O abstractions (InfileBase, OutfileBase)
├── arg.rs                      # Command-line argument handling
├── mem.rs                      # Memory utilities (AlignedVec, LineBuf)
│
├── params/                     # JPEG 2000 Marker Parameters
│   ├── mod.rs                  # Public re-exports
│   └── local.rs                # Internal param structures (SIZ, COD, QCD, etc.) [1993 LOC]
│
├── codestream/                 # Main codec/decoding pipeline
│   ├── mod.rs                  # Public Codestream API
│   ├── local.rs                # CodestreamLocal (main state machine) [420 LOC]
│   ├── tile.rs                 # Tile structure and layout [171 LOC]
│   ├── tile_comp.rs            # Tile-component (color plane) [123 LOC]
│   ├── resolution.rs           # Resolution levels in decomposition [159 LOC]
│   ├── subband.rs              # Subbands (LL, HL, LH, HH) [132 LOC]
│   ├── precinct.rs             # Precincts and tag trees [150 LOC]
│   ├── codeblock.rs            # Code-block structures [99 LOC]
│   ├── codeblock_fun.rs        # Code-block helper functions [57 LOC]
│   ├── bitbuffer_read.rs       # Bitstream reading [99 LOC]
│   ├── bitbuffer_write.rs      # Bitstream writing [122 LOC]
│   └── simd/
│       └── mod.rs             # SIMD dispatch stubs
│
├── coding/                     # HTJ2K Block Entropy Coding
│   ├── mod.rs                  # Module interface and dispatch [132 LOC]
│   ├── encoder.rs              # Block encoder (32-bit & 64-bit) [1576 LOC]
│   ├── decoder32.rs            # 32-bit block decoder [1308 LOC]
│   ├── decoder64.rs            # 64-bit block decoder [1302 LOC]
│   ├── common.rs               # Shared table generation/lookup [408 LOC]
│   ├── tables.rs               # VLC/UVLC source tables [823 LOC]
│   ├── simd/
│   │   └── mod.rs             # SIMD dispatch stubs
│
├── transform/                  # Wavelet & Color Transforms
│   ├── mod.rs                  # Module interface, function table dispatch [365 LOC]
│   ├── wavelet.rs              # 5/3 reversible & 9/7 irreversible DWT [1056 LOC]
│   ├── colour.rs               # RCT (reversible) & ICT (irreversible) [640 LOC]
│   └── simd/
│       └── mod.rs             # SIMD dispatch stubs
```

---

## 2. File Structure: tests/

```
tests/
├── unit/                       # Unit tests directory (currently empty)
├── common/                     # Common test utilities (currently empty)
└── fixtures/                   # Test fixtures (image data, codestreams)
```

---

## 3. Core Module Descriptions

### 3.1 types.rs (165 LOC)
**Purpose:** Fundamental geometric and numeric types.

**Key Types:**
- `Size`: 2D size (width × height)
- `Point`: 2D point (x, y)
- `Rect`: Axis-aligned rectangle (origin + size)
- Type aliases: `Ui8, Si8, Ui16, Si16, Ui32, Si32, Ui64, Si64`

**Helper Functions:**
- `div_ceil()`: Ceiling division
- `ojph_max()`, `ojph_min()`: Const-evaluable min/max
- Constants: `OPENJPH_VERSION_*`, `NUM_FRAC_BITS`

---

### 3.2 error.rs (40 LOC)
**Purpose:** Error handling model.

**Types:**
```rust
pub enum OjphError {
    Codec { code: u32, message: String },
    Io(std::io::Error),
    InvalidParam(String),
    Unsupported(String),
    AllocationFailed,
}
pub type Result<T> = std::result::Result<T, OjphError>;
```

---

### 3.3 arch.rs (239 LOC)
**Purpose:** CPU feature detection and bit manipulation.

**Key Components:**
- `CpuExtLevel`: x86-64 SIMD levels (Generic → SSE → SSE2 → ... → AVX-512)
- `ArmCpuExtLevel`: ARM SIMD levels (Generic → NEON → SVE → SVE2)
- Runtime detection via `get_cpu_ext_level()`
- Bit helpers: `population_count()`, `count_leading_zeros()`, `count_trailing_zeros()`
- Rounding: `ojph_round()`, `ojph_trunc()` (match C++ behavior)
- `calc_aligned_size<T>()`: Alignment calculation

**Constants:**
- `BYTE_ALIGNMENT = 64` (AVX-512)
- `LOG_BYTE_ALIGNMENT = 6`

---

### 3.4 file.rs (309 LOC)
**Purpose:** File I/O abstraction layer.

**Traits:**
- `OutfileBase`: Sequential/seekable output
- `InfileBase`: Sequential/seekable input

**Implementations:**
- `J2cOutfile`: File-backed output (wraps `std::fs::File`)
- `J2cInfile`: File-backed input (wraps `std::fs::File`)
- `MemOutfile`: In-memory output buffer
- `MemInfile`: In-memory read-only input

**SeekFrom:** Enum (Start, Current, End) matching standard library

---

### 3.5 mem.rs (Large, only first 150 LOC shown)
**Purpose:** Memory allocation and line buffer management.

**Key Types:**
- `AlignedVec<T>`: Heap-allocated buffer with guaranteed minimum alignment (default 64 bytes)
  - Custom `Layout` for aligned allocation
  - `resize()`, `len()`, `as_ptr()`, `as_mut_ptr()`
- `LineBuf`: Buffered line data for transform operations
  - Supports `I32`, `I64`, `F32` backing (union-like behavior)
- `LineBufData`: Enum for different data type storage
  
**Features:**
- Safe wrappers around `std::alloc`
- Zero-initialization support
- Implements `Deref`, `DerefMut`, `Index`, `IndexMut`

---

### 3.6 params/local.rs (1993 LOC - *Very Large*)
**Purpose:** JPEG 2000 marker segment parameters and I/O.

**Marker Codes:**
```
SOC (0xFF4F), SIZ (0xFF51), COD (0xFF52), QCD (0xFF5C),
COM (0xFF64), SOT (0xFF90), EOC (0xFFD9), etc.
```

**Key Structures:**
- `ParamSiz`: Image/tile size, components, bit depths, downsampling
- `ParamCod`: Coding style (decompositions, block dims, precincts, reversibility)
- `ParamQcd`: Quantization parameters (step sizes, exponents)
- `ParamCap`: Capabilities marker
- `ParamNlt`: Nonlinearity transform (inverse color transform, DC level shift)
- `ParamTlm`: Tile-Part Length Marker
- `ParamDfs`: Deformation marker (unused in most implementations)
- `ParamSot`: Start-of-Tile marker

**Enums:**
- `ProgressionOrder`: LRCP, RLCP, RPCL, PCRL, CPRL
- `ProfileNum`: Profile0, Profile1, Cinema2K/4K, Broadcast, IMF

**Big-Endian I/O Helpers:**
- `read_u8()`, `read_u16_be()`, `read_u32_be()`
- `write_u8()`, `write_u16_be()`, `write_u32_be()`

---

### 3.7 codestream/ - The Core Processing Pipeline

#### 3.7.1 mod.rs (151 LOC) - Public Codestream API
```rust
pub struct Codestream {
    inner: local::CodestreamLocal,
}
```

**Methods:**
- `new()`, `restart()`
- `access_siz()`, `access_cod()`, `access_qcd()`, `access_nlt()`
- Mutable variants: `access_siz_mut()`, etc.
- `enable_resilience()`, `set_planar()`, `set_profile()`
- `write_headers()`, `read_headers()`
- `get_num_tiles()`

---

#### 3.7.2 local.rs (420 LOC) - Internal State Machine
```rust
pub struct CodestreamLocal {
    siz, cod, qcd, cap, nlt, tlm, dfs: Param*,
    num_tiles: Size,
    tiles: Vec<Tile>,
    lines: Vec<LineBuf>,
    cur_line, cur_comp, cur_tile_row: u32,
    resilient, skipped_res_for_read/recon: bool/u32,
    ...
}
```

**Key Methods:**
- `compute_tile_grid()`: Calculates tile layout from parameters
- `write_headers()`: Outputs SOC, SIZ, CAP, COD, QCD, NLT, COM, TLM markers
- `read_headers()`: Parses incoming markers up to SOT
- `build_tiles()`: Creates tile objects from grid and parameters
- Helper: `skip_marker_segment()`, `write_comment()`

---

#### 3.7.3 tile.rs (171 LOC)
```rust
pub struct Tile {
    tile_idx: u32,
    tile_rect: Rect,
    tile_comps: Vec<TileComp>,
    num_comps: u32,
    employ_color_transform: bool,
    num_tileparts: u32,
    sot: ParamSot,
    is_complete: bool,
    cur_line, num_lines: u32,
    skipped_res_for_recon: u32,
}
```

**Methods:**
- `new()`: Creates tile from grid parameters
- `get_comp()`, `get_comp_mut()`: Access tile-components
- `compute_num_tileparts()`: Calculates tilepart divisions

---

#### 3.7.4 tile_comp.rs (123 LOC)
```rust
pub struct TileComp {
    comp_num: u32,
    comp_rect: Rect,
    num_resolutions: u32,
    resolutions: Vec<Resolution>,
    reversible: bool,
    bit_depth: u32,
    is_signed: bool,
}
```

**Methods:**
- `new()`: Creates component with resolution hierarchy
- `get_resolution()`, `get_resolution_mut()`
- `width()`, `height()`

---

#### 3.7.5 resolution.rs (159 LOC)
```rust
pub struct Resolution {
    res_num: u32,
    res_rect: Rect,
    log_precinct_size: Size,
    num_precincts_x/y: u32,
    subbands: Vec<Subband>,  // 1 (LL) or 3 (HL, LH, HH)
    log_block_dims: Size,
    is_lowest: bool,
    num_decomps: u32,
    reversible: bool,
}
```

**Hierarchy:**
- Resolution 0 = lowest (coarsest) with 1 LL subband
- Resolution 1..N = higher with 3 subbands each (HL, LH, HH)

---

#### 3.7.6 subband.rs (132 LOC)
```rust
pub enum SubbandType {
    LL = 0, HL = 1, LH = 2, HH = 3,
}

pub struct Subband {
    band_type: SubbandType,
    resolution_num: u32,
    band_rect: Rect,
    log_block_dims: Size,
    k_max: u32,              // Missing MSBs
    delta: f32,              // Quantization step (irreversible)
    reversible: bool,
    num_blocks_x/y: u32,
}
```

---

#### 3.7.7 precinct.rs (150 LOC)
```rust
pub struct TagTree {
    nodes: Vec<u32>,
    widths, heights: Vec<u32>,
    num_levels: u32,
}

pub struct Precinct {
    prec_rect: Rect,
    num_cbs_x/y: u32,
    inclusion_tree: TagTree,
    zero_bitplane_tree: TagTree,
    uses_sop, uses_eph: bool,
}
```

**TagTree:** Hierarchical structure for encoding/decoding inclusion and zero-bitplane info.

---

#### 3.7.8 codeblock.rs (99 LOC)
```rust
pub struct CodeblockEncState {
    pass1/2_bytes: u32,
    num_passes: u32,
    missing_msbs: u32,
    has_data: bool,
}

pub struct CodeblockDecState {
    pass1/2_len: u32,
    num_passes: u32,
    missing_msbs: u32,
}

pub struct Codeblock {
    cb_rect: Rect,
    log_block_dims: Size,
    enc_state: Option<CodeblockEncState>,
    dec_state: Option<CodeblockDecState>,
    coded_data: Vec<u8>,
}
```

---

### 3.8 coding/ - Block Entropy Coding (HTJ2K)

#### 3.8.1 mod.rs (132 LOC) - Interface & Dispatch
```rust
pub type EncodeCodeblock32Fn = fn(&[u32], missing_msbs, num_passes, w, h, stride) -> Result<EncodeResult>;
pub type DecodeCodeblock32Fn = fn(&mut [u8], &mut [u32], ...) -> Result<bool>;
// ... 64-bit variants
```

**Functions:**
- `init_block_encoder_tables()`, `init_block_decoder_tables()`
- `get_encode_codeblock32()`, `get_encode_codeblock64()`
- `get_decode_codeblock32()`, `get_decode_codeblock64()`

---

#### 3.8.2 encoder.rs (1576 LOC) - Block Encoding Engine
**Algorithm:** HTJ2K with MagSgn (magnitude-sign) + MEL (modified exponential-Golomb) + VLC bitstreams.

**Structures:**
```rust
struct EncodeResult {
    data: Vec<u8>,  // [MagSgn ‖ MEL ‖ VLC]
    length: u32,
}

struct MelEncoder { buf, pos, remaining_bits, tmp, run, k, threshold }
struct VlcEncoder { buf, pos, used_bits, tmp, last_greater_than_8f }
struct MsEncoder  { buf, pos, max_bits, used_bits, tmp }
```

**Public Functions:**
- `encode_codeblock32()`: 32-bit sample encoding (1 pass)
- `encode_codeblock64()`: 64-bit sample encoding (1 pass)

**Pass Logic:**
- Initial row (y=0): Uses table0, encodes with MEL prefix
- Non-initial rows (y≥2): Uses table1, direct UVLC

**Byte-Stuffing:** Prevents 0xFF bytes in output for framing safety.

---

#### 3.8.3 decoder32.rs (1308 LOC) - 32-bit Block Decoding

**Structures for bitstream reading:**
```rust
struct DecMelSt { data, pos, tmp, bits, unstuff, k, num_runs, runs }
struct RevStruct { data, pos, tmp, bits, unstuff, size }  // Backward VLC/MRP
struct FrwdStruct32 { data, pos, tmp, bits, unstuff, size }  // Forward MagSgn/SPP
```

**Three-Pass Decoding:**
1. **Cleanup Pass (CUP):** VLC + MEL + MagSgn
2. **Significance-Propagation Pass (SPP):** Forward bitstream
3. **Magnitude-Refinement Pass (MRP):** Backward bitstream (if num_passes > 2)

**Returns:** `Ok(true)` success, `Ok(false)` non-fatal, `Err` fatal.

---

#### 3.8.4 decoder64.rs (1302 LOC) - 64-bit Block Decoding
Nearly identical structure to `decoder32.rs` but operates on `u64` samples.

---

#### 3.8.5 common.rs (408 LOC) - Lookup Table Generation
```rust
pub struct DecoderTables {
    vlc_tbl0, vlc_tbl1: [u16; 1024],
    uvlc_tbl0: [u16; 320],
    uvlc_tbl1: [u16; 256],
    uvlc_bias: [u8; 320],
}

pub struct EncoderTables {
    vlc_tbl0, vlc_tbl1: [u16; 2048],
    uvlc_tbl: [UvlcEncEntry; 75],
}

pub struct UvlcEncEntry {
    pre, suf, ext: u8,
    pre_len, suf_len, ext_len: u8,
}
```

**Lazy Initialization:**
- `decoder_tables()`, `encoder_tables()` return static references
- Populated on first call via `OnceLock`
- Functions: `vlc_init_dec_tables()`, `vlc_init_enc_tables()`, etc.

---

#### 3.8.6 tables.rs (823 LOC) - VLC Source Data
```rust
pub struct VlcSrcEntry {
    c_q: u8,  // Context
    cwd: u8,  // Codeword
    cwd_len: u8,
    rho: u8,  // Significance pattern
    u_off: u8,
    e_k: u8,
    e_1: u8,
}

pub static VLC_SRC_TABLE0, VLC_SRC_TABLE1: &[VlcSrcEntry];
```

**Purpose:** Hard-coded VLC lookup entries (indexed by context and codeword prefix).

---

### 3.9 transform/ - Wavelet & Color Transforms

#### 3.9.1 mod.rs (365 LOC) - Interface & Function Dispatch
```rust
pub struct RevLiftingStep {
    a: i16,     // Coefficient Aatk
    b: i16,     // Additive Batk
    e: u8,      // Shift Eatk
}

pub struct IrvLiftingStep {
    a: f32,     // Coefficient (float)
}

pub enum LiftingStep {
    Reversible(RevLiftingStep),
    Irreversible(IrvLiftingStep),
}

pub struct ParamAtk {
    latk: u16,
    satk: u16,
    katk: f32,      // Scaling factor K
    natk: u8,       // Number of steps
    steps: Vec<LiftingStep>,
}
```

**Function Tables:**
```rust
pub struct WaveletTransformFns {
    rev_vert_step, irv_vert_step,
    rev_horz_ana, rev_horz_syn,
    irv_horz_ana, irv_horz_syn,
    irv_vert_times_k,
}

pub struct ColourTransformFns {
    rev_convert, irv_convert_to/from_integer/float,
    rev_convert_nlt_type3, irv_convert_*_nlt_type3,
    rct_forward, rct_backward,
    ict_forward, ict_backward,
}
```

**Lazy Initialization:**
- `init_wavelet_transform_functions()` → `WAVELET_FNS`
- `init_colour_transform_functions()` → `COLOUR_FNS`

**Standard Filters:**
- `ParamAtk::init_rev53()`: 5/3 reversible (2 steps)
- `ParamAtk::init_irv97()`: 9/7 irreversible (4 steps)

---

#### 3.9.2 wavelet.rs (1056 LOC) - DWT Implementations
**Generic implementations** for all combinations:
- Reversible: 32-bit and 64-bit
- Irreversible: 32-bit (float)

**Key Functions:**
```rust
fn gen_rev_vert_step32/64(step, sig, other, aug, repeat, synthesis)
fn gen_irv_vert_step(step, sig, other, aug, repeat, synthesis)
fn gen_rev_horz_ana/syn(atk, ldst/hdst, src, width, even)
fn gen_irv_horz_ana/syn(atk, ldst/hdst, src, width, even)
fn gen_irv_vert_times_k(k, aug, repeat)
```

**Lifting Step Operations:**
- Vertical: Applies to column data
- Horizontal: Applies to row data
- Synthesis vs. Analysis (forward/inverse)

---

#### 3.9.3 colour.rs (640 LOC) - Color Space Transforms
**Reversible (RCT):** Integer operations on Y, Cb, Cr
**Irreversible (ICT):** Float operations using fixed coefficients

**Constants:**
```rust
ALPHA_RF, ALPHA_GF, ALPHA_BF  // Forward coefficients
BETA_CBF, BETA_CRF            // Cb/Cr scaling
GAMMA_CR2R, GAMMA_CB2B, etc.  // Inverse coefficients
```

**Functions:**
```rust
gen_rev_convert(src, dst, shift, width)           // Reversible sample conversion
gen_rev_convert_nlt_type3(...)                    // NLT type 3 (sign flip)
gen_irv_convert_to_integer(...)                   // Float → i32 quantization
gen_irv_convert_to_float(...)                     // i32 → float dequantization
gen_rct_forward/backward(c0, c1, c2, d0, d1, d2) // RCT
gen_ict_forward/backward(...)                     // ICT
```

---

## 4. Key Data Flow

### Encoding Path
```
Input Image (samples)
    ↓
[Color Transform] (RCT or ICT)
    ↓
[Wavelet Transform] (5/3 or 9/7)
    ↓
[Quantization] (SIZ, QCD parameters)
    ↓
[Block Encoding] (per codeblock)
    - MagSgn, MEL, VLC bitstreams
    ↓
[Marker Writing] (SOC, SIZ, COD, QCD, etc.)
    ↓
[Compressed J2K Codestream]
```

### Decoding Path
```
Compressed J2K Codestream
    ↓
[Marker Parsing] (SOC, SIZ, COD, QCD)
    ↓
[Block Decoding] (per codeblock)
    - VLC, MEL, MagSgn reconstruction
    ↓
[Dequantization]
    ↓
[Inverse Wavelet Transform] (5/3 or 9/7)
    ↓
[Inverse Color Transform] (RCT or ICT)
    ↓
Output Samples
```

---

## 5. Code Organization Summary

| Module | Lines | Purpose |
|--------|-------|---------|
| types.rs | 165 | Geometric & numeric types |
| error.rs | 40 | Error handling |
| arch.rs | 239 | CPU detection & bit manipulation |
| file.rs | 309 | I/O abstractions |
| mem.rs | ~500+ | Memory management & buffers |
| params/local.rs | 1993 | Marker parameters & parsing |
| codestream/ | 1363 | Tile/component/resolution hierarchy |
| coding/encoder.rs | 1576 | Block encoding (HTJ2K) |
| coding/decoder32.rs | 1308 | 32-bit block decoding |
| coding/decoder64.rs | 1302 | 64-bit block decoding |
| coding/common.rs | 408 | Lookup table generation |
| coding/tables.rs | 823 | VLC/UVLC source data |
| transform/wavelet.rs | 1056 | DWT implementations |
| transform/colour.rs | 640 | Color space transforms |
| **TOTAL** | **~11,300** | |

---

## 6. Feature Flags & Compilation

**Cargo.toml Features:**
```toml
[features]
default = ["simd"]
simd = []
avx512 = ["simd"]
```

**Dependencies:**
- `thiserror`: Error handling
- `criterion`: Benchmarking (dev)

**SIMD Status:**
- Dispatch infrastructure in place (`simd/mod.rs` stubs)
- Currently defaults to generic (non-SIMD) implementations
- Future: CPU-dispatched SIMD variants for x86-64 (SSE/AVX/AVX-512) and ARM (NEON/SVE)

---

## 7. Current Limitations & TODO

1. **SIMD:** Generic implementations only; SIMD dispatch framework awaits actual implementations
2. **Profiles:** Profile/capability markers supported but not enforced
3. **Error Recovery:** Resilience mode available but limited
4. **Tests:** Test suite structure exists but currently empty
5. **Documentation:** Inline comments present; external docs could expand

---

## 8. Architecture Patterns

### Trait-Based Abstraction
- `OutfileBase`, `InfileBase` for I/O (not tied to `std::io::Read/Write`)
- `MessageHandler` for diagnostic messages

### Lazy Initialization (OnceLock)
- Lookup tables (encoder/decoder)
- Function dispatch tables (wavelet, color transforms)

### Type Safety
- Enum-based lifting steps (Reversible vs. Irreversible)
- Strong typing for marker parameters

### Memory Safety
- Safe wrappers around `std::alloc`
- Aligned allocation for SIMD-ready buffers
- No unsafe code in user-facing APIs (minimal in internal codecs)

---

## 9. Test Structure

Currently, test directories exist but are empty:
```
tests/
├── unit/      # Will contain unit tests
├── common/    # Shared test utilities
└── fixtures/  # Test images/codestreams
```

**Note:** Development bench available at `openjph-core/benches/codec.rs`

---

## Summary

OpenJPH-RS is a **well-structured, complete Rust port** of OpenJPH that:
- Implements full HTJ2K (JPEG 2000 Part 15) block coding
- Provides 5/3 reversible and 9/7 irreversible wavelet transforms
- Handles reversible (RCT) and irreversible (ICT) color transforms
- Includes marker parsing and codestream I/O
- Maintains architectural compatibility with the C++ original
- Is organized for future SIMD optimization
- Provides trait-based abstractions for extensibility

The codebase is approximately **11,300 lines** of Rust across 33 source files, with proper separation of concerns, comprehensive error handling, and memory safety throughout.
