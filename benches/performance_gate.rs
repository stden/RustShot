//! Stable, deterministic benchmarks used by the pull-request performance gate.

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
use rustshot::{capture, record, selection};
use std::hint::black_box;
use std::time::Duration;

const WIDTH_1080P: u32 = 1920;
const HEIGHT_1080P: u32 = 1080;
const WIDTH_4K: u32 = 3840;
const HEIGHT_4K: u32 = 2160;

fn patterned_bytes(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index.wrapping_mul(31).wrapping_add(17)) & 0xff) as u8)
        .collect()
}

fn performance_gate(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_gate");
    group.sampling_mode(SamplingMode::Flat);

    let screenshot = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    let mut overlay = selection::OverlayRedrawBenchmark::new(
        screenshot,
        (WIDTH_1080P, HEIGHT_1080P),
        (WIDTH_1080P, HEIGHT_1080P),
    );
    group.throughput(Throughput::Bytes(
        u64::from(WIDTH_1080P) * u64::from(HEIGHT_1080P) * 4,
    ));
    group.bench_function("selection_overlay_redraw_1080p", |b| {
        b.iter(|| {
            overlay.redraw_crosshair(black_box((960, 540)));
            black_box(overlay.output());
        });
    });

    let mut bgra_4k = patterned_bytes(WIDTH_4K as usize * HEIGHT_4K as usize * 4);
    group.throughput(Throughput::Bytes(
        u64::from(WIDTH_4K) * u64::from(HEIGHT_4K) * 4,
    ));
    group.bench_function("bgra_to_rgba_4k", |b| {
        b.iter(|| capture::bgra_to_rgba(black_box(&mut bgra_4k)));
    });

    let native_frame = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    let mut media_foundation_frame = vec![0; native_frame.len()];
    group.throughput(Throughput::Bytes(native_frame.len() as u64));
    group.bench_function("recording_frame_copy_1080p", |b| {
        b.iter(|| {
            record::benchmark_copy_bgra_frame_exact_fit(
                black_box(&native_frame),
                WIDTH_1080P,
                HEIGHT_1080P,
                black_box(&mut media_foundation_frame),
            )
            .unwrap();
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(40)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .noise_threshold(0.05)
        .significance_level(0.05);
    targets = performance_gate
}
criterion_main!(benches);
