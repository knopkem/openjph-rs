# OpenJPH-RS Lossless / Parity Verification Summary

This file originally documented a March 2024 investigation where reversible
5/3 (`rev53`) roundtrips were badly broken. That diagnosis was correct at the
time, but it is no longer the current status.

This updated summary reflects the 2026-03-19 audit.

## Executive summary

The previous `rev53` lossless failure report is obsolete.

Current verified status:

- reversible 5/3 (`rev53`) tests are now green
- the old failing `enc_rev53_*` repro commands now pass
- two external HTJ2K conformance fixtures previously cited as blockers now
  decode correctly against their PGX references
- the current failing surface is irreversible 9/7 (`irv97`), not reversible 5/3

## What was rechecked

### 1. Old rev53 repro commands

These commands were rerun:

```text
cargo test -p openjph-core --test integration_encode_decode enc_rev53_decomp_2
cargo test -p openjph-core --test integration_encode_decode enc_rev53_decomp_5
cargo test -p openjph-core --test integration_encode_decode enc_rev53_256x256
cargo test -p openjph-core --test integration_encode_decode enc_rev53_16bit_gray
```

All four succeeded during the audit.

### 2. Entire reversible bucket

```text
cargo test -p openjph-core rev53
```

This subset also succeeded during the audit.

### 3. External HTJ2K fixture spot-checks

The older migration blocker note called out these fixtures as still decoding
incorrectly:

- `ds0_ht_12_b11.j2k`
- `ds0_ht_11_b10.j2k`

Both were rechecked with a temporary helper that used
`openjph_core::codestream::Codestream` directly and compared the decoded output
to the PGX references from `dcmtk-rs`. Both matched exactly.

### 4. Full suite status

```text
cargo test -p openjph-core
```

Current observed outcome:

- all library/unit tests pass: `154 passed`
- `tests/integration_encode_decode.rs` then fails with `38 passed; 50 failed`
- the failures are all in the irreversible `irv97` group

## Current failure pattern

The repository no longer shows the old reversible D2/D5 corruption pattern.

The active failure pattern is instead:

- many 8-bit `irv97` RGB cases fail near `MSE 5464.9844`
- 16-bit `irv97` cases fail near `MSE 1_039_004_000` to `1_053_217_000`
- the failures cover decode, encode, tiled, odd-size, and high-compression cases

Representative failing tests:

- `dec_irv97_64x64_rgb`
- `dec_irv97_gray_tiles`
- `enc_irv97_decomp_0`
- `enc_irv97_16bit_gray`
- `enc_irv97_tiles_33x33_d6`

## Interpretation

### Resolved bucket

The earlier reversible parity gap was real and significant, but it has now been
closed far enough that:

- the old rev53 repros pass
- the whole `rev53` subset passes
- the two external HTJ2K spot-check fixtures decode correctly

### Remaining bucket

The codec is still not ready to replace the active `dcmtk-rs` HTJ2K backend,
because the irreversible 9/7 path still fails broadly.

So the migration blocker has moved:

- **old blocker**: reversible 5/3 / packet / DWT parity
- **current blocker**: irreversible 9/7 parity (`irv97`)

## Recommended next pass

1. Treat `irv97` as a dedicated parity bucket against local C++ OpenJPH.
2. Keep `rev53` green while working on the irreversible path.
3. After `irv97` is green, rerun:
   - `cargo test -p openjph-core`
   - the external HTJ2K fixture spot-checks
   - the downstream `dcmtk-rs` HTJ2K integration tests

## Historical note

The older raw artifacts in this directory (`D2D5_REVERSIBLE_TEST_REPORT.txt`
and `D2D5_TEST_RESULTS.csv`) are still useful as an archive of the original
bug, but they no longer describe the repository's current state.
