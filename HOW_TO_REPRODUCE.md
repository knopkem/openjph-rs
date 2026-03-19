# How to Reproduce the D2/D5 Reversible Lossless Decoding Tests

## Quick Reproduction

To reproduce all 28 reversible (5/3) tests from the verification:

```bash
cd /Users/macair/projects/dicom/openjph-rs
cargo test --test integration_encode_decode "rev" -- --nocapture 2>&1
```

## Individual Test Examples

### Test 1: D2 Reversible (Decomposition Level 2)
```bash
cargo test --test integration_encode_decode enc_rev53_decomp_2 -- --nocapture
```

**Expected Result**: ❌ FAIL with MSE=10,925.977

### Test 2: D5 Reversible (Decomposition Level 5)
```bash
cargo test --test integration_encode_decode enc_rev53_decomp_5 -- --nocapture
```

**Expected Result**: ❌ FAIL with MSE=10,925.977

### Test 3: Baseline (No DWT - should PASS)
```bash
cargo test --test integration_encode_decode enc_rev53_decomp_0 -- --nocapture
```

**Expected Result**: ✅ PASS with MSE=0.0

### Test 4: Large D5 Image
```bash
cargo test --test integration_encode_decode enc_rev53_256x256 -- --nocapture
```

**Expected Result**: ❌ FAIL with MSE=10,922.996

### Test 5: 16-bit Grayscale (Critical Case)
```bash
cargo test --test integration_encode_decode enc_rev53_16bit_gray -- --nocapture
```

**Expected Result**: ❌ FAIL with catastrophic MSE=2,078,828,000

## Full Test Suite

Run all reversible tests and capture results:

```bash
cargo test --test integration_encode_decode "rev" 2>&1 | tee reversible_test_results.log
```

## Analyzing Results

Extract just the pass/fail summary:

```bash
cargo test --test integration_encode_decode "rev" 2>&1 | grep -E "(test |FAILED|passed|failed)"
```

## Expected Output Pattern

All tests except `enc_rev53_decomp_0` should show:

```
assertion `left == right` failed: component 0: reversible roundtrip must be lossless (MSE=XXXX.XXX)
```

## Test Parameters Explained

Each test in the format `enc_rev53_*` or `dec_rev53_*`:

- **enc_** = Encode test (encode→decode roundtrip)
- **dec_** = Decode test (decode only)
- **rev53** = Reversible 5/3 DWT filter
- **decomp_N** = Decomposition level N (0-6)
- **64x64** = Image dimensions
- **16bit** = Bit depth (8/10/12/16)
- **gray/rgb** = Color space
- **tiles** = Tiled image

## Debugging Individual Components

### Test the 5/3 Filter Alone

To isolate the DWT53 issue, you'd want to:

```rust
#[test]
fn test_dwt53_forward_inverse_roundtrip() {
    let input = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let transformed = dwt53_forward(&input, ...);
    let reconstructed = dwt53_inverse(&transformed, ...);
    assert_eq!(input, reconstructed, "Must be lossless");
}
```

This would isolate whether the bug is in the forward or inverse transform.

## Cross-Platform Testing (Once Fixed)

After fixing the Rust implementation, verify C++ OpenJPH can decode the codestreams:

```bash
# Generate a reversible codestream with fixed Rust encoder
./target/release/ojph_compress -i test_image.pgm -o test_d5.j2c --reversible --num_decomps 5

# Decode with C++ OpenJPH
./target/release/ojph_expand -i test_d5.j2c -o test_decoded.pgm

# Compare original vs decoded (should be identical)
cmp test_image.pgm test_decoded.pgm && echo "LOSSLESS" || echo "LOSSY"
```

## CI/CD Integration

Add to your GitHub Actions or CI pipeline:

```yaml
- name: Verify D2/D5 Reversible Lossless
  run: |
    cd openjph-rs
    cargo test --test integration_encode_decode enc_rev53_decomp_2 -- --nocapture
    cargo test --test integration_encode_decode enc_rev53_decomp_5 -- --nocapture
```

## Success Criteria

✅ **After Fix**: All 28 tests should pass with MSE=0.0 and PAE=0

❌ **Before Fix**: 27/28 tests fail with non-zero MSE

## Related Files for Investigation

- `openjph-core/src/transform/dwt53.rs` - Primary suspect (5/3 filter)
- `openjph-core/src/codec/encoder.rs` - Encoding pipeline
- `openjph-core/src/codec/decoder.rs` - Decoding pipeline
- `openjph-core/tests/integration_encode_decode.rs` - Test definitions
- `openjph-core/tests/common/mse_pae.rs` - MSE/PAE computation

## Performance Baseline

Expected test execution time:
- Single test: ~100-200ms
- All 28 reversible tests: ~3-5 seconds (on modern hardware)
- Full integration suite: ~10-30 seconds

## Troubleshooting

### Test Compilation Issues
```bash
cargo clean
cargo build --release
cargo test --test integration_encode_decode --no-run
```

### View Full Test Output
```bash
RUST_BACKTRACE=1 cargo test --test integration_encode_decode enc_rev53_decomp_1 -- --nocapture
```

### Run with Verbose Output
```bash
cargo test --test integration_encode_decode enc_rev53_decomp_1 -- --nocapture --test-threads=1
```

---

**Test Environment**: Tested on macOS with Rust 1.94.0
**Last Updated**: March 19, 2024
