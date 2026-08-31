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

/// A frame of FP16 scRGB pixels that straddles the tone curve's shoulder.
///
/// Half-precision bit patterns are written out directly so the fixture needs no
/// f16 encoder: 0.0, 0.25, 0.5, 0.75, 1.0, 0.5 again, then 2.0 and 8.0 as the
/// highlights. A pixel's level is the maximum of its three consecutive channels,
/// so the two levels above 1.0 put half of the eight pixel phases on the
/// highlight shoulder -- the branch with a `ln()` in it. `patterned_bytes` would
/// be the wrong fixture here: arbitrary bytes decode to NaNs, infinities and
/// enormous magnitudes, which the tone map rejects before it does any work, so
/// the measurement would describe a path nobody records through.
///
/// Deliberately a second copy of the fixture in `benches/benchmarks/recording.rs`
/// rather than a shared module: the gate is a separate Criterion target whose
/// recorded baselines must not move when the exploratory suite is edited.
fn scrgb_frame(width: u32, height: u32) -> Vec<u8> {
    const HALVES: [u16; 8] = [
        0x0000, 0x3400, 0x3800, 0x3a00, 0x3c00, 0x3800, 0x4000, 0x4800,
    ];
    let pixels = width as usize * height as usize;
    let mut frame = Vec::with_capacity(pixels * 8);
    for pixel in 0..pixels {
        for channel in 0..4 {
            frame.extend_from_slice(&HALVES[(pixel + channel) % HALVES.len()].to_le_bytes());
        }
    }
    frame
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
    // Steady-state cursor-move redraw. Deliberately declared before any
    // throughput is set on the group: an incremental repaint touches a crosshair
    // and its predecessor, not the frame, so a full-frame byte rate would be
    // meaningless here.
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

    // The screenshot conversion: turning the capture backend's bottom-up BGRA
    // into top-down RGBA is part of how long a PrintScreen takes.
    //
    // The ENTRY POINT here is production-dead -- the screenshot worker moved to
    // `rgba_to_bgra_bottom_up_in_place`, benchmarked further down -- and these two
    // have already been proposed for removal on that basis. They stay because the
    // KERNEL is not dead. `PixelSwizzle::convert` and `PixelSwizzle::convert_in_place`
    // are not two names for one loop: they dispatch to `swap_red_blue_into_avx2` /
    // `swap_red_blue_into_portable` and to `bgra_to_rgba_avx2` / `bgra_to_rgba_scalar`
    // respectively -- four separate functions sharing only the enum. The two-buffer
    // pair runs in production from `capture::platform::rgba_from_bgra_rows` (every
    // Windows Graphics Capture screenshot, row by row) and from
    // `record::frame::copy_region_bottom_up` on its `SourcePixelOrder::Rgba` arm (the
    // GDI recording fallback).
    //
    // Nothing else in this group reaches that pair: `bgra_to_rgba_4k` and both
    // in-place entries go through `convert_in_place`, and `recording_frame_copy_1080p`
    // takes the `SourcePixelOrder::Bgra` arm, which is a `copy_from_slice` with no
    // swizzle in it at all. So deleting these two would not stop gating dead code, it
    // would leave losing AVX2 in the two-buffer kernel invisible to the gate. The
    // in-place benchmarks cannot stand in: they would catch a broken
    // `PixelSwizzle::detect`, and nothing inside `swap_red_blue_into_avx2`.
    // Revisit only once both production callers named above are gone.
    //
    // 1080p is the sharper of the two on purpose and should outlive the 4K entry if
    // either is ever dropped: 16.6 MB of traffic is the more cache-resident case, and
    // therefore the one where a scalar fallback is not masked by DRAM bandwidth.
    //
    // Measured with both buffers preallocated, so an allocation in the timing loop
    // cannot hide the conversion itself.
    let captured_frame = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    let mut converted_frame = vec![0u8; captured_frame.len()];
    group.throughput(Throughput::Bytes(captured_frame.len() as u64));
    group.bench_function("screenshot_bgra_to_rgba_1080p", |b| {
        b.iter(|| {
            record::rgba_to_bgra_bottom_up_into(
                black_box(&captured_frame),
                black_box(&mut converted_frame),
                WIDTH_1080P,
                HEIGHT_1080P,
            )
            .unwrap();
        });
    });

    // 4K matters more than 1080p here: this conversion is per-pixel work on the
    // critical path of a single keypress, so it grows with monitor size.
    let captured_4k = patterned_bytes(WIDTH_4K as usize * HEIGHT_4K as usize * 4);
    let mut converted_4k = vec![0u8; captured_4k.len()];
    group.throughput(Throughput::Bytes(captured_4k.len() as u64));
    group.bench_function("screenshot_bgra_to_rgba_4k", |b| {
        b.iter(|| {
            record::rgba_to_bgra_bottom_up_into(
                black_box(&captured_4k),
                black_box(&mut converted_4k),
                WIDTH_4K,
                HEIGHT_4K,
            )
            .unwrap();
        });
    });

    // Overlay startup: converting the frozen screenshot into softbuffer pixels
    // is what the user waits through before the selection overlay appears. The
    // cold case allocates; the warm case is what each subsequent resize costs.
    let overlay_screenshot = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    group.throughput(Throughput::Bytes(overlay_screenshot.len() as u64));
    group.bench_function("selection_overlay_background_1080p", |b| {
        b.iter(|| {
            black_box(selection::benchmark_build_overlay_background(
                black_box(&overlay_screenshot),
                (WIDTH_1080P, HEIGHT_1080P),
                (WIDTH_1080P, HEIGHT_1080P),
            ));
        });
    });

    let mut overlay_background = selection::OverlayBackgroundBenchmark::new(
        overlay_screenshot,
        (WIDTH_1080P, HEIGHT_1080P),
        (WIDTH_1080P, HEIGHT_1080P),
    );
    group.bench_function("selection_overlay_background_rebuild_1080p", |b| {
        b.iter(|| black_box(overlay_background.rebuild()));
    });

    // Appended last on purpose: inserting a benchmark ahead of the three above
    // changes the cache state they start from, which shows up as a phantom
    // regression against their recorded baselines.
    group.throughput(Throughput::Bytes(
        u64::from(WIDTH_1080P) * u64::from(HEIGHT_1080P) * 4,
    ));
    group.bench_function("selection_overlay_first_frame_1080p", |b| {
        b.iter(|| {
            overlay.compose_full_frame(black_box((960, 540)));
            black_box(overlay.output());
        });
    });

    // The conversion the screenshot worker actually calls. The two-buffer entries
    // above cannot stand in for it: they are handed a preallocated destination, so
    // a change that gave this function a monitor-sized scratch buffer of its own
    // would leave their medians untouched.
    //
    // One buffer, patterned once and reused across iterations: the transform is its
    // own inverse, so successive iterations alternate between bottom-up BGRA and
    // top-down RGBA and every iteration does identical work. Re-cloning the frame
    // per iteration would time the clone instead of the conversion.
    let mut in_place_1080p = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    group.throughput(Throughput::Bytes(in_place_1080p.len() as u64));
    group.bench_function("screenshot_bgra_to_rgba_in_place_1080p", |b| {
        b.iter(|| {
            record::rgba_to_bgra_bottom_up_in_place(
                black_box(&mut in_place_1080p),
                WIDTH_1080P,
                HEIGHT_1080P,
            )
            .unwrap();
        });
    });

    // 4K is where this form is decided: the row swap moves the whole frame through
    // memory and the two swizzles walk it again, so replacing `swap_with_slice`
    // with a byte loop, or losing `PixelSwizzle`'s AVX2 path, shows up here first.
    let mut in_place_4k = patterned_bytes(WIDTH_4K as usize * HEIGHT_4K as usize * 4);
    group.throughput(Throughput::Bytes(in_place_4k.len() as u64));
    group.bench_function("screenshot_bgra_to_rgba_in_place_4k", |b| {
        b.iter(|| {
            record::rgba_to_bgra_bottom_up_in_place(
                black_box(&mut in_place_4k),
                WIDTH_4K,
                HEIGHT_4K,
            )
            .unwrap();
        });
    });

    // HDR recording's per-frame cost, and the only entry in this group that is
    // already over the budget its caller needs: roughly 30-42 ms depending on the
    // machine, against the 33.3 ms a 30 fps frame interval allows. It is gated
    // anyway, because "already too slow" is a reason to stop it drifting further,
    // not a reason to leave it unmeasured -- the budget comment in
    // `scripts/benchmark_guard.ps1` spells out why that number is a ceiling and not
    // an approval. Until now this path was benchmarked only in the exploratory
    // `benchmarks` target, which CI never runs and the guard cannot read.
    let scrgb_1080p = scrgb_frame(WIDTH_1080P, HEIGHT_1080P);
    let mut tone_mapped_1080p = vec![0u8; WIDTH_1080P as usize * HEIGHT_1080P as usize * 4];
    // Per pixel rather than per byte: the work is three half decodes, one tone map
    // and three table lookups per pixel, none of which scale with the 8-byte input
    // stride.
    group.throughput(Throughput::Elements(
        u64::from(WIDTH_1080P) * u64::from(HEIGHT_1080P),
    ));
    group.bench_function("recording_scrgb_tone_map_1080p", |b| {
        b.iter(|| {
            record::benchmark_copy_scrgb_frame_exact_fit(
                black_box(&scrgb_1080p),
                WIDTH_1080P,
                HEIGHT_1080P,
                black_box(1_000.0),
                black_box(&mut tone_mapped_1080p),
            )
            .unwrap();
        });
    });

    // Appended after every recorded baseline so these monitor-sized fixtures do
    // not change the cache state of established benchmarks. This is the common
    // selected-region recording path the exact-fit copy above cannot exercise:
    // a 1080p rectangle starts midway through a pitched 4K desktop frame.
    let source_row_pitch_4k = WIDTH_4K as usize * 4 + 256;
    let pitched_bgra_4k = patterned_bytes(source_row_pitch_4k * HEIGHT_4K as usize);
    let mut cropped_bgra_1080p = vec![0u8; WIDTH_1080P as usize * HEIGHT_1080P as usize * 4];
    group.throughput(Throughput::Bytes(cropped_bgra_1080p.len() as u64));
    group.bench_function("recording_region_copy_1080p_from_4k", |b| {
        b.iter(|| {
            record::benchmark_copy_bgra_region(
                black_box(&pitched_bgra_4k),
                source_row_pitch_4k,
                (WIDTH_4K, HEIGHT_4K),
                (960, 540, WIDTH_1080P, HEIGHT_1080P),
                (WIDTH_1080P, HEIGHT_1080P),
                black_box(&mut cropped_bgra_1080p),
            )
            .unwrap();
        });
    });

    // A 4K monitor makes overlay startup process four times the 1080p pixels.
    // Keep this separate from the warm rebuild benchmark: this cold path must
    // continue accounting for the allocation the first overlay open performs.
    let overlay_screenshot_4k = patterned_bytes(WIDTH_4K as usize * HEIGHT_4K as usize * 4);
    group.throughput(Throughput::Bytes(overlay_screenshot_4k.len() as u64));
    group.bench_function("selection_overlay_background_4k", |b| {
        b.iter(|| {
            black_box(selection::benchmark_build_overlay_background(
                black_box(&overlay_screenshot_4k),
                (WIDTH_4K, HEIGHT_4K),
                (WIDTH_4K, HEIGHT_4K),
            ));
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
