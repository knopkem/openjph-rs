# How to Reproduce the Current OpenJPH-RS Status

This file supersedes the older rev53 failure instructions.

As of the 2026-03-19 audit:

- the old reversible 5/3 (`rev53`) failures are fixed
- the two external HTJ2K conformance fixtures previously cited as blockers now
  decode correctly
- the remaining red bucket is irreversible 9/7 (`irv97`)

## Quick status split

- `rev53`: green
- external HTJ2K fixture spot-checks: green
- full `openjph-core` suite: red because `integration_encode_decode` still has
  50 `irv97` failures

## Confirm the old rev53 repros are fixed

Run the old repro commands that used to fail:

```bash
cd /Users/macair/projects/dicom/openjph-rs

cargo test -p openjph-core --test integration_encode_decode enc_rev53_decomp_2
cargo test -p openjph-core --test integration_encode_decode enc_rev53_decomp_5
cargo test -p openjph-core --test integration_encode_decode enc_rev53_256x256
cargo test -p openjph-core --test integration_encode_decode enc_rev53_16bit_gray
```

Expected result now: all four commands exit successfully.

To run the whole reversible bucket:

```bash
cargo test -p openjph-core rev53
```

Expected result now: success. During the audit this subset was green.

## Reproduce the remaining failures

To run the currently failing irreversible bucket:

```bash
cd /Users/macair/projects/dicom/openjph-rs
cargo test -p openjph-core irv97
```

Expected result now:

- `integration_encode_decode` fails in 50 `irv97` cases
- many 8-bit RGB cases report MSE around `5464.9844`
- 16-bit cases report much larger MSE, around `1_039_004_000` to
  `1_053_217_000`

Representative individual repros:

```bash
cargo test -p openjph-core --test integration_encode_decode dec_irv97_64x64_rgb -- --nocapture
cargo test -p openjph-core --test integration_encode_decode enc_irv97_decomp_0 -- --nocapture
cargo test -p openjph-core --test integration_encode_decode enc_irv97_16bit_gray -- --nocapture
```

Representative current failure text:

```text
component 0: MSE 5464.9844 exceeds 1% of range² (650.25)
component 0: MSE 1053217000 exceeds 1% of range² (42948360)
```

## Full-suite check

```bash
cd /Users/macair/projects/dicom/openjph-rs
cargo test -p openjph-core
```

Expected result now:

- all `openjph-core` library/unit tests pass (`154 passed`)
- Cargo then stops in `tests/integration_encode_decode.rs`
- that binary reports `38 passed; 50 failed`
- the failed tests are the current `irv97` blocker surface

## External HTJ2K fixture spot-checks

The old blocker note claimed that these external fixtures still decoded
incorrectly:

- `dcmtk-rs/.../ds0_ht_12_b11.j2k`
- `dcmtk-rs/.../ds0_ht_11_b10.j2k`

During the 2026-03-19 audit they were rechecked with a temporary helper that
used `openjph_core::codestream::Codestream` directly and compared the decoded
pixels against the PGX references. Both matched.

There is no maintained in-tree command for that exact comparison yet, so treat
this as a verified spot-check rather than a polished regression workflow.

## Historical note

Older versions of this file said the `enc_rev53_*` commands above should fail.
That is no longer true. Those instructions are obsolete and were kept only long
enough to localize the reversible parity gap that has now been fixed.
