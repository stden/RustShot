//! Benchmarks — tray: icon construction.

use criterion::Criterion;
use rustshot::tray;

// ---------------------------------------------------------------------------
// tray::benchmark_build_default_icon_pixels
// ---------------------------------------------------------------------------

pub(crate) fn bench_build_default_icon_pixels(c: &mut Criterion) {
    c.bench_function("build_default_icon_pixels", |b| {
        b.iter(tray::benchmark_build_default_icon_pixels);
    });
}
