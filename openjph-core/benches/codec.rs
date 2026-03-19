//! Placeholder benchmark file — benchmarks will be added in later phases.

use criterion::{criterion_group, criterion_main, Criterion};

fn codec_bench(_c: &mut Criterion) {
    // TODO: add codec benchmarks
}

criterion_group!(benches, codec_bench);
criterion_main!(benches);
