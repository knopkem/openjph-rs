# openjph-rs

A pure Rust port of [OpenJPH](https://github.com/aous72/OpenJPH) — a fast HTJ2K (JPEG 2000 Part 15) codec.

## Features

- **Pure Rust** — no C/C++ dependencies, no `unsafe` FFI
- **HTJ2K encoding and decoding** (ISO/IEC 15444-15)
- **Lossless** (reversible 5/3 DWT) and **lossy** (irreversible 9/7 DWT) compression
- **Multi-component** images with color transforms (RCT / ICT)
- **Configurable** — decomposition levels, block sizes, precinct sizes, progression orders
- **Profile support** — IMF, Broadcast, Cinema
- **NLT** (non-linearity) marker support for signed data
- **SIMD-ready** architecture with CPU feature detection
- **CLI tools** — `ojph_compress` and `ojph_expand`

## Crate Structure

| Crate | Description |
|-------|-------------|
| `openjph-core` | Core codec library |
| `openjph-cli` | Command-line tools |

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
openjph-core = { path = "openjph-core" }
```

### Encoding (Lossless)

```rust
use openjph_core::codestream::Codestream;
use openjph_core::file::MemOutfile;
use openjph_core::types::{Point, Size};

let (width, height) = (256u32, 256u32);
let pixels: Vec<i32> = (0..width * height).map(|i| (i % 256) as i32).collect();

let mut cs = Codestream::new();
cs.access_siz_mut().set_image_extent(Point::new(width, height));
cs.access_siz_mut().set_num_components(1);
cs.access_siz_mut().set_comp_info(0, Point::new(1, 1), 8, false);
cs.access_siz_mut().set_tile_size(Size::new(width, height));
cs.access_cod_mut().set_num_decomposition(5);
cs.access_cod_mut().set_reversible(true);
cs.access_cod_mut().set_color_transform(false);
cs.set_planar(0);

let mut outfile = MemOutfile::new();
cs.write_headers(&mut outfile, &[]).unwrap();

for y in 0..height as usize {
    let start = y * width as usize;
    cs.exchange(&pixels[start..start + width as usize], 0).unwrap();
}
cs.flush(&mut outfile).unwrap();

println!("Encoded {} bytes", outfile.len());
```

### Decoding

```rust
use openjph_core::codestream::Codestream;
use openjph_core::file::MemInfile;

fn decode(j2k_data: &[u8]) {
    let mut infile = MemInfile::new(j2k_data);
    let mut cs = Codestream::new();
    cs.read_headers(&mut infile).unwrap();

    let siz = cs.access_siz();
    let width = siz.get_width(0);
    let height = siz.get_height(0);
    println!("Image: {}×{}", width, height);

    cs.create(&mut infile).unwrap();

    for _y in 0..height {
        let line = cs.pull(0).expect("decoded line");
        // process line...
    }
}
```

## CLI Usage

### ojph_compress

Compress a PPM/PGM/YUV/DPX image to HTJ2K:

```bash
cargo run --bin ojph_compress -- \
    -i input.ppm \
    -o output.j2c \
    --num_decomps 5 \
    --block_size "{64,64}" \
    --reversible true
```

Options:
- `-i <file>` — input image (PPM, PGM, YUV, DPX, RAWL)
- `-o <file>` — output codestream (.j2c)
- `--num_decomps <n>` — DWT decomposition levels (default: 5)
- `--block_size "{w,h}"` — code block dimensions (default: {64,64})
- `--reversible <bool>` — lossless mode (default: true)
- `--qstep <f>` — quantization step (lossy mode)
- `--colour_trans <bool>` — enable color transform
- `--profile <name>` — IMF or BROADCAST
- `--precincts "{w,h},{w,h},..."` — precinct sizes per level
- `--prog_order <order>` — LRCP, RLCP, RPCL, PCRL, or CPRL
- `--tlm_marker <bool>` — write TLM marker

### ojph_expand

Decompress an HTJ2K codestream to PPM/PGM/YUV/RAWL:

```bash
cargo run --bin ojph_expand -- \
    -i input.j2c \
    -o output.ppm \
    --skip_res 0
```

Options:
- `-i <file>` — input codestream (.j2c, .jph)
- `-o <file>` — output image (PPM, PGM, YUV, RAWL)
- `--skip_res <n>` — skip resolution levels (default: 0)
- `--resilient` — enable error-tolerant decoding

## Architecture

```
openjph-core/src/
├── lib.rs           # Crate root, module re-exports
├── types.rs         # Numeric aliases, Size, Point, Rect
├── error.rs         # OjphError enum, Result alias
├── message.rs       # Diagnostic message dispatch
├── arch.rs          # CPU feature detection, alignment
├── mem.rs           # AlignedVec, LineBuf, arena allocators
├── file.rs          # I/O traits (OutfileBase, InfileBase)
├── arg.rs           # CLI argument parser
├── params/          # Marker segment types (SIZ, COD, QCD, ...)
├── codestream/      # Main Codestream encoder/decoder
│   ├── mod.rs       # Public Codestream struct
│   ├── local.rs     # Internal CodestreamLocal
│   ├── tile.rs      # Tile processing
│   ├── tile_comp.rs # Tile-component processing
│   ├── resolution.rs
│   ├── subband.rs
│   ├── precinct.rs
│   └── codeblock.rs
├── coding/          # HTJ2K block entropy coder
│   ├── encoder.rs   # Block encoder (CUP + VLC)
│   ├── decoder32.rs # 32-bit block decoder
│   └── decoder64.rs # 64-bit block decoder
└── transform/       # Wavelet and color transforms
    ├── wavelet.rs   # DWT 5/3 (reversible) and 9/7 (irreversible)
    └── colour.rs    # RCT and ICT color transforms
```

### Pipeline

**Encoding**: Image lines → Color transform (RCT/ICT) → DWT → Quantization → HTJ2K block encoder → Codestream

**Decoding**: Codestream → HTJ2K block decoder → Dequantization → Inverse DWT → Inverse color transform → Image lines

## Building

```bash
# Build the workspace
cargo build --release

# Run tests
cargo test

# Generate documentation
cargo doc --no-deps --open

# Run benchmarks
cargo bench -p openjph-core
```

### Features

- `simd` (default) — enable SIMD acceleration stubs
- `avx512` — enable AVX-512 specializations (requires `simd`)

## Testing

```bash
# All tests
cargo test

# Core library tests only
cargo test -p openjph-core

# Specific test
cargo test roundtrip_8x8
```

## Comparison with C++ OpenJPH

| Aspect | C++ OpenJPH | openjph-rs |
|--------|-------------|------------|
| Language | C++11/17 | Rust 2021 |
| Memory safety | Manual | Guaranteed |
| Build system | CMake | Cargo |
| Dependencies | None | `thiserror` only |
| SIMD | SSE2/AVX2/AVX-512/NEON | Architecture detection (stubs) |
| API style | pImpl classes | Struct + traits |
| Error handling | Exceptions | `Result<T, OjphError>` |
| Thread safety | Manual | `Send`/`Sync` where applicable |

## Status

This is a **v0.1.0** port of OpenJPH v0.26.3. The core encode/decode pipeline is functional for:

- Single-component 8-bit grayscale images
- Reversible (lossless) mode
- 0–5 DWT decomposition levels
- Standard block sizes (64×64)

### Roadmap

- [ ] Multi-component (RGB) encoding/decoding
- [ ] Irreversible (lossy) 9/7 DWT
- [ ] SIMD-optimized wavelet transforms
- [ ] SIMD-optimized block coding
- [ ] Tiled images with multiple tiles
- [ ] Higher bit-depth support (12-bit, 16-bit)
- [ ] File format wrappers (JP2, JPH)
- [ ] Streaming decode
- [ ] Performance benchmarks vs C++ OpenJPH

## License

BSD-2-Clause — same license as the original [OpenJPH](https://github.com/aous72/OpenJPH) project.

```
Copyright (c) 2019-2024, Aous Naman
Copyright (c) 2024, openjph-rs contributors
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.
```

## Acknowledgments

This project is a faithful port of [OpenJPH](https://github.com/aous72/OpenJPH) by Aous Naman. The original C++ implementation is the reference for all codec behavior.
