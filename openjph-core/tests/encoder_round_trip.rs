//! Lossless (reversible 5/3) HTJ2K encodes decode back to the exact samples.
//!
//! Single component, single tile, one layer, RPCL. The named cases are inputs
//! that once came back wrong because `BitBufferWrite::finalize` wrote an extra
//! padding byte after some packet headers (issue #2); the sweep covers sizes, precisions, signedness, decomposition levels, block
//! sizes and content.

use openjph_core::codestream::Codestream;
use openjph_core::file::{MemInfile, MemOutfile};
use openjph_core::types::{Point, Size};

#[derive(Clone, Copy, Debug)]
struct Params {
    width: u32,
    height: u32,
    precision: u32,
    signed: bool,
    levels: u32,
    block: u32,
}

fn encode(p: Params, samples: &[i32]) -> Vec<u8> {
    let mut cs = Codestream::new();
    let siz = cs.access_siz_mut();
    siz.set_image_extent(Point::new(p.width, p.height));
    siz.set_num_components(1);
    siz.set_comp_info(0, Point::new(1, 1), p.precision, p.signed);
    siz.set_tile_size(Size::new(p.width, p.height));
    let cod = cs.access_cod_mut();
    cod.set_num_decomposition(p.levels);
    cod.set_block_dims(p.block, p.block);
    cod.set_reversible(true);
    cod.set_color_transform(false);
    cod.set_progression_order("RPCL").unwrap();
    cs.set_planar(0);
    let mut out = MemOutfile::new();
    cs.write_headers(&mut out, &[]).unwrap();
    for line in samples.chunks(p.width as usize) {
        cs.exchange(line, 0).unwrap();
    }
    cs.flush(&mut out).unwrap();
    out.get_data().to_vec()
}

fn decode(bytes: &[u8]) -> Vec<i32> {
    let mut input = MemInfile::new(bytes);
    let mut cs = Codestream::new();
    cs.read_headers(&mut input).unwrap();
    cs.create(&mut input).unwrap();
    let mut samples = Vec::new();
    while let Some(line) = cs.pull(0) {
        samples.extend(line);
    }
    samples
}

/// Encode then decode; `Err` describes the first wrong sample.
fn round_trip(p: Params, samples: &[i32]) -> Result<(), String> {
    assert_eq!(samples.len(), (p.width * p.height) as usize);
    let decoded = decode(&encode(p, samples));
    if decoded.len() != samples.len() {
        return Err(format!(
            "{p:?}: decoded {} samples, want {}",
            decoded.len(),
            samples.len()
        ));
    }
    match decoded.iter().zip(samples).position(|(a, b)| a != b) {
        None => Ok(()),
        Some(i) => {
            let wrong = decoded.iter().zip(samples).filter(|(a, b)| a != b).count();
            Err(format!(
                "{p:?}: {wrong} wrong samples; first at {i}: got {}, want {}",
                decoded[i], samples[i]
            ))
        }
    }
}

fn assert_round_trip(p: Params, samples: &[i32]) {
    if let Err(e) = round_trip(p, samples) {
        panic!("{e}");
    }
}

fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// `count` samples uniform in `[0, 2^amplitude)`, minus `2^(amplitude-1)` when
/// `centered`.
fn noise(count: u32, amplitude: u32, centered: bool, seed: u64) -> Vec<i32> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            let v = (xorshift(&mut state) % (1u64 << amplitude)) as i64;
            (if centered {
                v - (1i64 << (amplitude - 1))
            } else {
                v
            }) as i32
        })
        .collect()
}

fn params(size: u32, precision: u32, signed: bool, levels: u32, block: u32) -> Params {
    Params {
        width: size,
        height: size,
        precision,
        signed,
        levels,
        block,
    }
}

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

#[test]
fn constant_60000_unsigned_16_bit() {
    let p = params(128, 16, false, 1, 64);
    assert_round_trip(p, &vec![60000; 128 * 128]);
}

/// The end-to-end reproduction from issue #2.
#[test]
fn constant_65535_unsigned_16_bit() {
    let p = params(128, 16, false, 1, 64);
    assert_round_trip(p, &vec![65535; 128 * 128]);
}

#[test]
fn ten_bit_noise_in_twelve_bit_unsigned_without_dwt() {
    let p = params(512, 12, false, 0, 64);
    assert_round_trip(p, &noise(512 * 512, 10, false, SEED));
}

#[test]
fn full_range_twelve_bit_unsigned_noise() {
    let samples = noise(512 * 512, 12, false, SEED);
    for (levels, block) in [(0, 64), (3, 64), (0, 32), (3, 32)] {
        assert_round_trip(params(512, 12, false, levels, block), &samples);
    }
}

#[test]
fn full_range_sixteen_bit_noise_one_level() {
    let unsigned = noise(128 * 128, 16, false, SEED);
    assert_round_trip(params(128, 16, false, 1, 64), &unsigned);
    let signed = noise(128 * 128, 16, true, SEED);
    assert_round_trip(params(128, 16, true, 1, 64), &signed);
}

#[test]
fn twelve_bit_noise_in_sixteen_bit_signed_three_levels() {
    let samples = noise(512 * 512, 12, true, SEED);
    for block in [64, 32] {
        assert_round_trip(params(512, 16, true, 3, block), &samples);
    }
}

/// The smallest failing input found: 96x96, one 3x3 grid of 32x32 blocks.
#[test]
fn small_grid_of_32_blocks() {
    let p = params(96, 12, false, 0, 32);
    assert_round_trip(p, &noise(96 * 96, 12, false, SEED));
}

/// Slice `z` of `slices` of a synthetic CT-like phantom (a sphere in a
/// horizontal ramp), as unsigned 16-bit.
fn phantom(z: u32, slices: u32, size: u32) -> Vec<i32> {
    let n = size as i32;
    let zc = z as f64 / slices as f64 - 0.5;
    let radius = (0.16 - zc * zc).max(0.0).sqrt() * n as f64;
    let mut samples = Vec::with_capacity((size * size) as usize);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = ((x - n / 2) as f64, (y - n / 2) as f64);
            let inside = dx * dx + dy * dy <= radius * radius;
            samples.push(if inside {
                1024 + 60
            } else {
                1024 - 1000 + x * 200 / n
            });
        }
    }
    samples
}

#[test]
fn phantom_slices() {
    for size in [64, 128, 256, 512] {
        for z in [0, 30, 60] {
            let samples = phantom(z, 120, size);
            for levels in 0..=5 {
                assert_round_trip(params(size, 16, false, levels, 64), &samples);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Content {
    FullRangeNoise,
    LowAmplitudeNoise,
    ConstantMin,
    ConstantMax,
    Ramp,
}

fn content(p: Params, c: Content, seed: u64) -> Vec<i32> {
    let count = p.width * p.height;
    let (min, max) = if p.signed {
        (
            -(1i64 << (p.precision - 1)),
            (1i64 << (p.precision - 1)) - 1,
        )
    } else {
        (0, (1i64 << p.precision) - 1)
    };
    match c {
        Content::FullRangeNoise => noise(count, p.precision, p.signed, seed),
        Content::LowAmplitudeNoise => noise(count, 3, p.signed, seed),
        Content::ConstantMin => vec![min as i32; count as usize],
        Content::ConstantMax => vec![max as i32; count as usize],
        Content::Ramp => {
            let span = (p.width + p.height).saturating_sub(2).max(1) as i64;
            (0..count)
                .map(|i| {
                    let (x, y) = ((i % p.width) as i64, (i / p.width) as i64);
                    (min + (max - min) * (x + y) / span) as i32
                })
                .collect()
        }
    }
}

const CONTENTS: [Content; 5] = [
    Content::FullRangeNoise,
    Content::LowAmplitudeNoise,
    Content::ConstantMin,
    Content::ConstantMax,
    Content::Ramp,
];

/// Every combination; returns the failures.
fn sweep_sizes(sizes: &[(u32, u32)], max_levels: u32) -> Vec<String> {
    let mut failures = Vec::new();
    let mut seed = SEED;
    for &(width, height) in sizes {
        for precision in [8, 12, 16] {
            for signed in [false, true] {
                for levels in 0..=max_levels {
                    for block in [32, 64] {
                        for c in CONTENTS {
                            let p = Params {
                                width,
                                height,
                                precision,
                                signed,
                                levels,
                                block,
                            };
                            xorshift(&mut seed);
                            if let Err(e) = round_trip(p, &content(p, c, seed)) {
                                failures.push(format!("{c:?} {e}"));
                            }
                        }
                    }
                }
            }
        }
    }
    failures
}

fn assert_no_failures(failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{} failing cases:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn sweep() {
    let sizes = [
        (1, 1),
        (7, 13),
        (64, 64),
        (128, 128),
        (256, 256),
        (300, 200),
    ];
    assert_no_failures(sweep_sizes(&sizes, 5));
}
