# OpenJPH-RS Remaining Test Failures

The current remaining blocker is the irreversible 9/7 path (`irv97`).

## Current failing surface

Observed from `cargo test -p openjph-core` and `cargo test -p openjph-core irv97`:

- the failing binary is `tests/integration_encode_decode.rs`
- it currently reports `38 passed; 50 failed`
- the failures are the `irv97` cases

Representative failures:

- `dec_irv97_64x64_rgb`
- `dec_irv97_gray_tiles`
- `enc_irv97_decomp_0`
- `enc_irv97_16bit_gray`
- `enc_irv97_tiles_33x33_d5`

Representative MSE values:

- many 8-bit RGB failures: about `5464.9844`
- 16-bit failures: about `1_039_004_000` to `1_053_217_000`

## Repro commands

```bash
cd /Users/macair/projects/dicom/openjph-rs
cargo test -p openjph-core irv97
cargo test -p openjph-core
```

## Migration implication for `dcmtk-rs`

Do not switch the active HTJ2K backend to `openjph-rs` yet.

The remaining migration blocker is the broad `irv97` parity gap. After that
parity pass succeeds, the full suite and downstream HTJ2K integration should be
rerun before retrying the backend swap.
