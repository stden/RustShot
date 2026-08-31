#[test]
fn benchmark_screen_count_from_result_returns_count_for_non_empty_screens() {
    let count = rustshot::capture::benchmark_screen_count_from_result::<u8>(Ok(vec![1, 2, 3]))
        .expect("non-empty screen list should produce a count");
    assert_eq!(count, 3);
}

#[test]
fn benchmark_first_screen_from_result_returns_first_screen() {
    let first = rustshot::capture::benchmark_first_screen_from_result(Ok(vec![10, 20, 30]))
        .expect("first screen should be returned");
    assert_eq!(first, 10);
}

#[test]
fn benchmark_rgb_image_from_rgba_produces_jpeg_compatible_image() {
    let rgba = image::RgbaImage::from_pixel(2, 2, image::Rgba([100, 150, 200, 255]));
    let rgb = rustshot::capture::benchmark_rgb_image_from_rgba(&rgba);

    let mut encoded = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 75);
    rgb.write_with_encoder(encoder)
        .expect("RGB benchmark images should encode as JPEG");

    assert!(encoded.len() > 2);
    assert_eq!(&encoded[..2], &[0xFF, 0xD8]);
}

// ---------------------------------------------------------------------------
// benchmark helper exports
// ---------------------------------------------------------------------------

#[test]
fn benchmark_ensure_parent_directory_creates_nested_parent() {
    let root = std::env::temp_dir().join(format!(
        "rustshot_benchmark_ensure_parent_{}",
        rustshot::settings::generate_timestamp()
    ));
    let path = root.join("nested/deeper/shot.png");

    rustshot::capture::benchmark_ensure_parent_directory(&path).unwrap();

    assert!(path.parent().unwrap().is_dir());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn benchmark_save_image_optimized_writes_png_file() {
    let root = std::env::temp_dir().join(format!(
        "rustshot_benchmark_save_image_{}",
        rustshot::settings::generate_timestamp()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("shot.png");
    let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));

    rustshot::capture::benchmark_save_image_optimized(
        &image,
        &path,
        rustshot::capture::Format::Png,
    )
    .unwrap();

    assert!(path.is_file());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn benchmark_build_default_icon_pixels_returns_full_rgba_buffer() {
    let pixels = rustshot::tray::benchmark_build_default_icon_pixels();
    assert_eq!(pixels.len(), 32 * 32 * 4);
}

// ---------------------------------------------------------------------------
// Performance-gate coverage
//
// The pull-request gate is worth exactly what it measures, and it enforces
// exactly what has a budget. Both halves drift silently. The gate went on
// benchmarking the two-buffer `rgba_to_bgra_bottom_up_into` after the screenshot
// worker moved to the single-buffer `rgba_to_bgra_bottom_up_in_place`, so the
// conversion on the hotkey path had no benchmark at all; the HDR tone map had a
// benchmark in the exploratory `benchmarks` target, which CI never runs and the
// guard cannot read. Neither failed anything, because a missing benchmark and a
// missing budget both look precisely like a green gate.
//
// These tests read the two files as text. `include_str!` is the point: editing
// either file recompiles this target, so the check cannot go stale, and it runs
// in `cargo test` on all three platforms rather than only in the pull-request
// benchmark job.
// ---------------------------------------------------------------------------

/// The Criterion target the pull-request performance gate runs.
const PERFORMANCE_GATE: &str = include_str!("../../benches/performance_gate.rs");

/// The budget table `scripts/benchmark_guard.ps1` enforces the gate against.
const BENCHMARK_GUARD: &str = include_str!("../../scripts/benchmark_guard.ps1");

/// Benchmark ids the gate declares, in declaration order.
///
/// Text rather than reflection: Criterion ids exist only at run time, and the
/// whole point is to compare them with a PowerShell array without running a
/// benchmark.
fn gate_benchmark_ids() -> Vec<&'static str> {
    PERFORMANCE_GATE
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("group.bench_function(\""))
        .filter_map(|rest| rest.split_once('"').map(|(id, _)| id))
        .collect()
}

/// Benchmark ids the guard budgets, with the `performance_gate/` prefix removed.
fn guard_budget_ids() -> Vec<&'static str> {
    BENCHMARK_GUARD
        .lines()
        .filter_map(|line| {
            line.trim_start()
                .strip_prefix("Benchmark = \"performance_gate/")
        })
        .filter_map(|rest| rest.split_once('"').map(|(id, _)| id))
        .collect()
}

/// Source belonging to one named Criterion benchmark, up to the next benchmark.
fn gate_benchmark_section(id: &str) -> &'static str {
    let marker = format!("group.bench_function(\"{id}\"");
    let (_, section) = PERFORMANCE_GATE
        .split_once(marker.as_str())
        .unwrap_or_else(|| panic!("performance gate does not declare benchmark {id:?}"));
    let end = section
        .find("group.bench_function(\"")
        .or_else(|| section.find("group.finish()"))
        .unwrap_or(section.len());
    &section[..end]
}

#[test]
fn performance_gate_benchmark_ids_parse() {
    let gate = gate_benchmark_ids();
    // Without this, a parser that silently matches nothing would make both
    // coverage tests below pass by comparing two empty lists -- the same kind of
    // green-for-no-reason the gate itself was suffering from.
    assert!(
        gate.contains(&"bgra_to_rgba_4k"),
        "the gate's bench_function calls must stay parseable, found {gate:?}"
    );
    assert!(
        !guard_budget_ids().is_empty(),
        "the guard's budget table must stay parseable"
    );
    // Counting as well as sampling: both parsers key on a literal prefix, so a
    // single rustfmt-wrapped call or a switch to `BenchmarkId` would drop that id
    // from BOTH lists and leave the two cross-checks below comparing equal-but-
    // incomplete sets. These two assertions are what turn that silent hole into a
    // failure.
    assert_eq!(
        gate.len(),
        PERFORMANCE_GATE.matches("bench_function(").count(),
        "every bench_function call must be parseable as a plain string literal"
    );
    assert_eq!(
        guard_budget_ids().len(),
        BENCHMARK_GUARD.matches("Benchmark =").count(),
        "every budget entry must name a performance_gate/ benchmark"
    );
}

#[test]
fn every_performance_gate_benchmark_has_a_budget() {
    let budgets = guard_budget_ids();
    for id in gate_benchmark_ids() {
        assert!(
            budgets.contains(&id),
            "scripts/benchmark_guard.ps1 has no budget for performance_gate/{id}: \
             CI would measure it on every pull request and enforce nothing"
        );
    }
}

#[test]
fn every_budget_names_a_benchmark_the_gate_declares() {
    let gate = gate_benchmark_ids();
    for id in guard_budget_ids() {
        assert!(
            gate.contains(&id),
            "scripts/benchmark_guard.ps1 budgets performance_gate/{id}, which no \
             benchmark declares; the guard would abort with 'Missing Criterion \
             result' twenty minutes into the benchmark job instead of failing here"
        );
    }
}

#[test]
fn performance_gate_measures_the_in_place_screenshot_conversion() {
    // Asserted on the call, not on the bare name: the name already appears in a
    // comment in the gate, so the loose form passes while nothing is measured.
    assert!(
        PERFORMANCE_GATE.contains("record::rgba_to_bgra_bottom_up_in_place("),
        "benches/performance_gate.rs must call the conversion \
         capture_monitor_image_on_worker uses; the two-buffer form it also \
         benchmarks has no production caller left"
    );
}
/// The two-buffer conversion, pinned for the reason that is not its name.
///
/// `rgba_to_bgra_bottom_up_into` has had no production caller since the screenshot
/// worker moved to the in-place form, so it reads like runner time spent on dead
/// code and has been proposed for removal on exactly that basis. What its two
/// benchmarks actually measure is `PixelSwizzle::convert`, which dispatches to
/// `swap_red_blue_into_avx2` -- a DIFFERENT function from `convert_in_place`'s
/// `bgra_to_rgba_avx2` -- and that one runs per row for every Windows Graphics
/// Capture screenshot (`capture::platform::rgba_from_bgra_rows`) and in the GDI
/// recording fallback (`copy_region_bottom_up`'s `SourcePixelOrder::Rgba` arm).
///
/// No other benchmark in the gate reaches it: `bgra_to_rgba_4k` and both in-place
/// entries use `convert_in_place`, and `recording_frame_copy_1080p` takes the `Bgra`
/// arm, which is a `copy_from_slice`. Removing those two benchmarks therefore does
/// not remove coverage of dead code -- it removes the gate's only coverage of a live
/// AVX2 kernel on the hotkey path, and leaves a green gate saying so.
#[test]
fn performance_gate_measures_the_two_buffer_swizzle() {
    // Asserted on the call rather than the bare name, for the same reason as the
    // test above: the name now appears in the gate's explanatory comment, so the
    // loose form would pass while nothing was being measured.
    assert!(
        PERFORMANCE_GATE.contains("record::rgba_to_bgra_bottom_up_into("),
        "benches/performance_gate.rs must keep a benchmark on the two-buffer \
         conversion: it is the gate's only measurement of PixelSwizzle::convert, \
         which capture::platform::rgba_from_bgra_rows runs for every WGC screenshot \
         and copy_region_bottom_up runs for the GDI recording fallback"
    );
}

#[test]
fn performance_gate_measures_the_hdr_tone_map() {
    assert!(
        PERFORMANCE_GATE.contains("record::benchmark_copy_scrgb_frame_exact_fit("),
        "benches/performance_gate.rs must call the FP16 scRGB tone map: it is the \
         heaviest per-pixel path in the recorder and the only one already over its \
         frame interval, and the guard reads no Criterion target but this one"
    );
}

#[test]
fn performance_gate_measures_the_pitched_recording_region_copy() {
    let section = gate_benchmark_section("recording_region_copy_1080p_from_4k");
    assert!(
        section.contains("record::benchmark_copy_bgra_region("),
        "the selected-region gate must call the production pitched BGRA crop adapter; \
         an exact-fit copy would omit both source pitch and crop-offset costs"
    );
}

#[test]
fn performance_gate_measures_the_4k_overlay_cold_build() {
    let section = gate_benchmark_section("selection_overlay_background_4k");
    assert!(
        section.contains("selection::benchmark_build_overlay_background("),
        "the 4K overlay gate must call the production cold-build adapter"
    );
    assert!(
        section.contains("black_box(&overlay_screenshot_4k)"),
        "the 4K overlay gate must process its monitor-sized source instead of a \
         smaller fixture"
    );
}
