# OpenJPH-RS Remaining Parity Summary

The current remaining blocker is the irreversible 9/7 path (`irv97`).

## Current observed status

Running:

```text
cargo test -p openjph-core
```

currently fails in:

```text
tests/integration_encode_decode.rs
```

with:

```text
38 passed; 50 failed
```

Running:

```text
cargo test -p openjph-core irv97
```

reproduces the same blocker bucket directly.

## Failure pattern

Representative failing tests:

- `dec_irv97_64x64_rgb`
- `dec_irv97_gray_tiles`
- `enc_irv97_decomp_0`
- `enc_irv97_16bit_gray`
- `enc_irv97_tiles_33x33_d6`

Representative current MSE values:

- many 8-bit RGB failures: about `5464.9844`
- 16-bit failures: about `1_039_004_000` to `1_053_217_000`

The failures span:

- decode and encode
- tiled and untiled images
- odd-size images
- 8-bit and 16-bit coverage
- high-compression cases

## Recommendation

Treat `irv97` as the next dedicated parity pass against local C++ OpenJPH.

Until that bucket is green:

- `openjph-rs` should not replace the active `dcmtk-rs` HTJ2K backend
- `cargo test -p openjph-core` should be expected to fail
- downstream HTJ2K migration work should remain paused
