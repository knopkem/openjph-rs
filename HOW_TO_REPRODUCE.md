# How to Reproduce the Remaining OpenJPH-RS Issues

The current remaining blocker is the irreversible 9/7 path (`irv97`).

## Quick repro

```bash
cd /Users/macair/projects/dicom/openjph-rs
cargo test -p openjph-core irv97
```

Expected result:

- `tests/integration_encode_decode.rs` fails
- the binary currently reports `38 passed; 50 failed`
- the failures are the current `irv97` blocker surface

## Full-suite repro

```bash
cd /Users/macair/projects/dicom/openjph-rs
cargo test -p openjph-core
```

Expected result:

- Cargo stops at `tests/integration_encode_decode.rs`
- the failing tests are the `irv97` cases

## Representative individual repros

```bash
cargo test -p openjph-core --test integration_encode_decode dec_irv97_64x64_rgb -- --nocapture
cargo test -p openjph-core --test integration_encode_decode enc_irv97_decomp_0 -- --nocapture
cargo test -p openjph-core --test integration_encode_decode enc_irv97_16bit_gray -- --nocapture
cargo test -p openjph-core --test integration_encode_decode enc_irv97_tiles_33x33_d5 -- --nocapture
```

Representative current failure text:

```text
component 0: MSE 5464.9844 exceeds 1% of range² (650.25)
component 0: MSE 1053217000 exceeds 1% of range² (42948360)
```

## Current failure pattern

- many 8-bit RGB cases fail near `MSE 5464.9844`
- 16-bit cases fail near `MSE 1_039_004_000` to `1_053_217_000`
- the failures cover decode, encode, tiled, odd-size, and high-compression
  cases
