# OpenJPH-RS Verification Status

This file used to summarize an early `rev53` failure investigation. That
conclusion is now obsolete.

## Current verified status

The 2026-03-19 audit found:

- `cargo test -p openjph-core rev53` passes
- the old repro commands
  `enc_rev53_decomp_2`, `enc_rev53_decomp_5`, `enc_rev53_256x256`, and
  `enc_rev53_16bit_gray` all pass
- direct `openjph-core` spot-checks against the external HTJ2K fixtures
  `ds0_ht_12_b11.j2k` and `ds0_ht_11_b10.j2k` both match their PGX references
- `cargo test -p openjph-core` still fails because
  `tests/integration_encode_decode.rs` has 50 failing `irv97` cases

## What is green now

### Reversible 5/3 (`rev53`)

The previously reported reversible D2/D5 lossless breakage is fixed.

Observed green checks during the audit:

```text
cargo test -p openjph-core --test integration_encode_decode enc_rev53_decomp_2
cargo test -p openjph-core --test integration_encode_decode enc_rev53_decomp_5
cargo test -p openjph-core --test integration_encode_decode enc_rev53_256x256
cargo test -p openjph-core --test integration_encode_decode enc_rev53_16bit_gray
cargo test -p openjph-core rev53
```

### External HTJ2K conformance spot-checks

The earlier blocker report cited two real HTJ2K fixtures as still decoding
incorrectly. That specific claim is no longer current. Both fixtures were
rechecked with `openjph-core` directly and matched their PGX references.

## What is still red

### Irreversible 9/7 (`irv97`)

The remaining blocker surface is currently the irreversible path.

Current observed failure shape:

- `cargo test -p openjph-core irv97` fails
- the failing test binary is `tests/integration_encode_decode.rs`
- it currently reports `38 passed; 50 failed`
- many 8-bit RGB cases fail at roughly `MSE 5464.9844`
- 16-bit cases fail at roughly `MSE 1_039_004_000` to `1_053_217_000`

Representative failures:

- `dec_irv97_64x64_rgb`
- `dec_irv97_gray_tiles`
- `enc_irv97_decomp_0`
- `enc_irv97_16bit_gray`
- `enc_irv97_tiles_33x33_d5`

## Migration implication for `dcmtk-rs`

The backend swap to `openjph-rs` should still stay paused, but the reason has
changed.

It is no longer blocked by the earlier reversible D2/D5 failure report or by
the two cited external conformance fixtures. It is still blocked by the broader
`irv97` parity gap and the lack of a fresh end-to-end toolkit validation pass
after that bucket is fixed.

## Historical artifacts in this directory

The following files are retained as historical snapshots from the original
reversible investigation and should not be treated as current status:

- `D2D5_REVERSIBLE_TEST_REPORT.txt`
- `D2D5_TEST_RESULTS.csv`

They were useful for the original rev53 debugging pass, but they no longer
describe the current state of the repository.
