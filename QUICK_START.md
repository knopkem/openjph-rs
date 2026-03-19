# OpenJPH-RS Quick Start Guide

## Project Layout

```
openjph-rs/
├── openjph-core/              # Main library (11,300 LOC)
│   ├── src/
│   │   ├── lib.rs             # Root module
│   │   ├── types.rs           # Size, Point, Rect, helpers
│   │   ├── error.rs           # OjphError, Result<T>
│   │   ├── arch.rs            # CPU detection, bit ops
│   │   ├── file.rs            # I/O traits & implementations
│   │   ├── mem.rs             # AlignedVec, LineBuf
│   │   ├── message.rs         # Message dispatch
│   │   ├── params/local.rs    # Marker parameters (1993 LOC)
│   │   ├── codestream/        # Pipeline hierarchy
│   │   │   ├── mod.rs         # Public Codestream API
│   │   │   ├── local.rs       # Internal state machine
│   │   │   ├── tile.rs        # Tiles
│   │   │   ├── tile_comp.rs   # Tile-components
│   │   │   ├── resolution.rs  # Resolution levels
│   │   │   ├── subband.rs     # Subbands (LL/HL/LH/HH)
│   │   │   ├── precinct.rs    # Precincts & tag trees
│   │   │   └── codeblock.rs   # Code-blocks
│   │   ├── coding/            # Block entropy coding (HTJ2K)
│   │   │   ├── encoder.rs     # 32/64-bit encoder (1576 LOC)
│   │   │   ├── decoder32.rs   # 32-bit decoder (1308 LOC)
│   │   │   ├── decoder64.rs   # 64-bit decoder (1302 LOC)
│   │   │   ├── common.rs      # Lookup table generation
│   │   │   └── tables.rs      # VLC/UVLC source data
│   │   └── transform/         # Wavelet & color transforms
│   │       ├── wavelet.rs     # 5/3 & 9/7 DWT (1056 LOC)
│   │       └── colour.rs      # RCT & ICT (640 LOC)
│   ├── Cargo.toml
│   └── benches/codec.rs
├── openjph-cli/               # Command-line tools
├── tests/                     # Test suite (currently empty)
└── Cargo.toml                 # Workspace config
```

## Key Modules

### Core Hierarchy (codestream/)
```
Codestream (public wrapper)
  └─ CodestreamLocal (state machine)
      ├─ Tile[num_tiles.w × num_tiles.h]
      │  └─ TileComp[num_comps]
      │     └─ Resolution[0..num_decomps]
      │        └─ Subband (LL or HL/LH/HH)
      │           └─ Precinct[num_precincts_x × num_precincts_y]
      │              └─ Codeblock (actual coded unit)
```

### Block Encoding (coding/encoder.rs)
- **Structure:** MagSgn (magnitude-sign) || MEL (exponential-Golomb) || VLC
- **Paths:** 32-bit (`encode_codeblock32`) and 64-bit (`encode_codeblock64`)
- **Passes:** 1 pass for HTJ2K (cleanup)
- **Byte-Stuffing:** Prevents 0xFF bytes for frame safety

### Block Decoding (coding/decoder32.rs / decoder64.rs)
- **Passes:** CUP (cleanup) → SPP (significance-prop) → MRP (magnitude-refinement)
- **Bitstreams:** Forward MEL/VLC, backward VLC/MRP, forward MagSgn
- **Returns:** `Ok(true)` success, `Ok(false)` non-fatal, `Err` fatal

### Wavelet Transform (transform/wavelet.rs)
- **Reversible (5/3):** 2 lifting steps, integer-only
- **Irreversible (9/7):** 4 lifting steps, floating-point
- **Paths:** 32-bit and 64-bit for reversible; float for irreversible

### Color Transform (transform/colour.rs)
- **RCT (Reversible):** Y = (R+2G+B)÷4, Cb = B-G, Cr = R-G
- **ICT (Irreversible):** Float-based color decorrelation
- **NLT Type 3:** DC level shift with sign flip

## Common Tasks

### 1. Creating a Codestream
```rust
use openjph_core::codestream::Codestream;
use openjph_core::params::*;

let mut cs = Codestream::new();
let siz = cs.access_siz_mut();
siz.set_image_extent(Point::new(1920, 1080));
siz.set_num_components(3);
siz.set_tile_size(Size::new(512, 512));
```

### 2. Reading Headers
```rust
use openjph_core::file::J2cInfile;

let mut infile = J2cInfile::open("image.j2k")?;
cs.read_headers(&mut infile)?;
let num_tiles = cs.get_num_tiles();
```

### 3. Writing Headers
```rust
use openjph_core::file::J2cOutfile;

let mut outfile = J2cOutfile::open("output.j2k")?;
cs.write_headers(&mut outfile, &[])?;
```

### 4. Accessing Parameters
```rust
let siz = cs.access_siz();
let width = siz.get_image_extent().x;

let cod = cs.access_cod();
let reversible = cod.get_coc(0).is_reversible();

let qcd = cs.access_qcd();
let layers = qcd.get_num_layers();
```

## Error Handling

```rust
use openjph_core::error::OjphError;

match result {
    Err(OjphError::Codec { code, message }) => {
        eprintln!("Codec error 0x{:08X}: {}", code, message);
    }
    Err(OjphError::Io(e)) => {
        eprintln!("I/O error: {}", e);
    }
    Err(OjphError::InvalidParam(s)) => {
        eprintln!("Invalid parameter: {}", s);
    }
    Err(OjphError::Unsupported(s)) => {
        eprintln!("Unsupported: {}", s);
    }
    Err(OjphError::AllocationFailed) => {
        eprintln!("Memory allocation failed");
    }
    Ok(_) => { /* success */ }
}
```

## Marker Codes (JPEG 2000)

| Marker | Code | Purpose |
|--------|------|---------|
| SOC | 0xFF4F | Start of Codestream |
| SIZ | 0xFF51 | Image/Tile Size |
| COD | 0xFF52 | Coding Style (Default) |
| COC | 0xFF53 | Coding Style (Component) |
| QCD | 0xFF5C | Quantization (Default) |
| QCC | 0xFF5D | Quantization (Component) |
| COM | 0xFF64 | Comment |
| SOT | 0xFF90 | Start of Tile |
| SOD | 0xFF93 | Start of Tile Data |
| EOC | 0xFFD9 | End of Codestream |

## CPU Detection

```rust
use openjph_core::arch::{get_cpu_ext_level, CpuExtLevel};

let level = get_cpu_ext_level();
match level as i32 {
    CpuExtLevel::Avx512 => println!("AVX-512 available"),
    CpuExtLevel::Avx2 => println!("AVX2 available"),
    _ => println!("Generic fallback"),
}
```

## Memory Management

### AlignedVec
```rust
use openjph_core::mem::AlignedVec;

let mut vec: AlignedVec<i32> = AlignedVec::new();
vec.resize(1024)?;  // Allocates 64-byte aligned
```

### LineBuf
```rust
use openjph_core::mem::LineBuf;

let mut line = LineBuf::new();
// Used in wavelet transform operations
```

## Progress Orders

```rust
use openjph_core::params::ProgressionOrder;

let orders = [
    ProgressionOrder::LRCP,  // Layer, Resolution, Component, Position
    ProgressionOrder::RLCP,  // Resolution, Layer, Component, Position
    ProgressionOrder::RPCL,  // Resolution, Position, Component, Layer
    ProgressionOrder::PCRL,  // Position, Component, Resolution, Layer
    ProgressionOrder::CPRL,  // Component, Position, Resolution, Layer
];
```

## Profiles

```rust
use openjph_core::params::ProfileNum;

let profile = match ProfileNum::from_str("IMF") {
    Some(p) => p,
    None => ProfileNum::Profile0,
};
```

## Development

### Building
```bash
cd openjph-rs
cargo build --release
```

### Testing (once tests are added)
```bash
cargo test
```

### Benchmarking
```bash
cargo bench --bench codec
```

### Features
```bash
# Default: SIMD infrastructure
cargo build

# With AVX-512 support (when implemented)
cargo build --features avx512
```

## Architecture Notes

### Single-Pass Encoding
- HTJ2K uses **single cleanup pass** (not classic JPEG 2000's 3-pass pipeline)
- All significance/magnitude info encoded in one pass
- MEL handles runs of insignificant samples efficiently

### Block Structure
- Codeblocks arranged in quad-row patterns (2×height)
- Quad-row enables efficient context formation and SIMD vectorization
- VLC and MEL bitstreams grow in opposite directions (merge at end)

### Color Transform Coupling
- Only first 3 components can have RCT applied
- Applied **before** wavelet transform (encoding) / **after** (decoding)
- Reversible (RCT) or Irreversible (ICT) choice affects quantization

### Precision Handling
- 32-bit path: up to 30 bits of data + sign bit
- 64-bit path: up to 62 bits of data + sign bit
- Missing MSBs tracked via `missing_msbs` parameter in quantization

## File I/O Trait Model

```rust
// Output
trait OutfileBase {
    fn write(&mut self, data: &[u8]) -> Result<usize>;
    fn tell(&self) -> i64;
    fn seek(&mut self, offset: i64, whence: SeekFrom) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

// Input
trait InfileBase {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn seek(&mut self, offset: i64, whence: SeekFrom) -> Result<()>;
    fn tell(&self) -> i64;
    fn eof(&self) -> bool;
}
```

**Implementations:**
- `J2cOutfile`, `J2cInfile`: File-backed
- `MemOutfile`, `MemInfile`: Memory-backed

## Message System

```rust
use openjph_core::message::{MsgLevel, MessageHandler, set_message_handler};

// Replace default stderr handler with custom
struct MyHandler;
impl MessageHandler for MyHandler {
    fn handle(&self, level: MsgLevel, code: u32, msg: &str) {
        // Custom handling
    }
}

set_message_handler(Some(Box::new(MyHandler)));
```

## Useful Constants

```rust
use openjph_core::types::*;

// Version
OPENJPH_VERSION_MAJOR = 0
OPENJPH_VERSION_MINOR = 26
OPENJPH_VERSION_PATCH = 3

// Numeric
NUM_FRAC_BITS = 13

// Architecture
BYTE_ALIGNMENT = 64         // AVX-512 aligned
LOG_BYTE_ALIGNMENT = 6
OBJECT_ALIGNMENT = 8
```

---

See `CODEBASE_ANALYSIS.md` for comprehensive module documentation.
