//! Benchmarks — cli: argument parsing, app-mode policy, output helpers.

use criterion::Criterion;
use rustshot::capture::Format;
use rustshot::cli;
use std::hint::black_box;

// ---------------------------------------------------------------------------
// cli helpers
// ---------------------------------------------------------------------------

// Was `cli::settings_capture_format`; that mapper is gone, since parsing a
// settings format string is now `Format::from_settings_str`.
pub(crate) fn bench_settings_capture_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("settings_capture_format");

    group.bench_function("png_fallback", |b| {
        b.iter(|| Format::from_settings_str(black_box("gif")));
    });

    group.bench_function("jpg", |b| {
        b.iter(|| Format::from_settings_str(black_box("jpg")));
    });

    group.bench_function("bmp", |b| {
        b.iter(|| Format::from_settings_str(black_box("bmp")));
    });

    group.finish();
}

pub(crate) fn bench_cli_format_flag_present_in(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_format_flag_present_in");

    group.bench_function("long_flag", |b| {
        b.iter(|| cli::cli_format_flag_present_in(black_box(["rustshot", "--format", "png"])));
    });

    group.bench_function("attached_short_flag", |b| {
        b.iter(|| cli::cli_format_flag_present_in(black_box(["rustshot", "-fbmp"])));
    });

    group.bench_function("skip_output_value", |b| {
        b.iter(|| cli::cli_format_flag_present_in(black_box(["rustshot", "-o", "-fpng"])));
    });

    group.finish();
}

pub(crate) fn bench_cli_path_helpers(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_path_helpers");
    let png_path = std::path::Path::new("/tmp/shot.png");
    let jpg_path = std::path::Path::new("/tmp/shot.jpg");
    let webp_path = std::path::Path::new("/tmp/shot.webp");

    group.bench_function("output_extension_png", |b| {
        b.iter(|| cli::output_extension(black_box(Some(png_path))));
    });

    group.bench_function("format_from_extension_jpeg", |b| {
        b.iter(|| cli::format_from_extension(black_box("JPEG")));
    });

    group.bench_function("format_from_output_extension_jpg", |b| {
        b.iter(|| cli::format_from_output_extension(black_box(Some(jpg_path))));
    });

    group.bench_function("unsupported_output_extension_webp", |b| {
        b.iter(|| cli::unsupported_output_extension(black_box(Some(webp_path))));
    });

    group.finish();
}

pub(crate) fn bench_effective_cli_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("effective_cli_format");
    let bmp_path = std::path::Path::new("/tmp/shot.bmp");

    group.bench_function("explicit_cli_format", |b| {
        b.iter(|| {
            cli::effective_cli_format(
                black_box(Format::Bmp),
                black_box(true),
                black_box(Some(bmp_path)),
                black_box(Format::Jpg),
            )
        });
    });

    group.bench_function("output_extension_wins", |b| {
        b.iter(|| {
            cli::effective_cli_format(
                black_box(Format::Png),
                black_box(false),
                black_box(Some(bmp_path)),
                black_box(Format::Jpg),
            )
        });
    });

    group.bench_function("settings_fallback", |b| {
        b.iter(|| {
            cli::effective_cli_format(
                black_box(Format::Png),
                black_box(false),
                black_box(None),
                black_box(Format::Jpg),
            )
        });
    });

    group.finish();
}

pub(crate) fn bench_should_run_cli_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("should_run_cli_mode");

    group.bench_function("all_false", |b| {
        b.iter(|| {
            cli::should_run_cli_mode(
                black_box(false),
                black_box(false),
                black_box(false),
                black_box(false),
                black_box(false),
                black_box(false),
            )
        });
    });

    group.bench_function("format_explicit", |b| {
        b.iter(|| {
            cli::should_run_cli_mode(
                black_box(false),
                black_box(false),
                black_box(false),
                black_box(false),
                black_box(false),
                black_box(true),
            )
        });
    });

    group.bench_function("window_flag", |b| {
        b.iter(|| {
            cli::should_run_cli_mode(
                black_box(false),
                black_box(false),
                black_box(true),
                black_box(false),
                black_box(false),
                black_box(false),
            )
        });
    });

    group.finish();
}

pub(crate) fn bench_format_screen_list_output(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_screen_list_output");
    let short = vec!["Screen 0: 1920x1080", "Screen 1: 1280x720"];
    let many: Vec<String> = (0..16)
        .map(|index| format!("Screen {index}: 1920x1080"))
        .collect();

    group.bench_function("two_entries", |b| {
        b.iter(|| cli::format_screen_list_output(black_box(short.clone())));
    });

    group.bench_function("sixteen_entries", |b| {
        b.iter(|| cli::format_screen_list_output(black_box(many.clone())));
    });

    group.finish();
}

pub(crate) fn bench_selection_stderr_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("selection_stderr_message");

    group.bench_function("cancelled", |b| {
        b.iter(|| cli::selection_stderr_message(black_box("Selection cancelled")));
    });

    group.bench_function("error", |b| {
        b.iter(|| cli::selection_stderr_message(black_box("Selection error: boom")));
    });

    group.finish();
}

pub(crate) fn bench_write_error_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_error_line");

    group.bench_function("short_message", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            cli::write_error_line(black_box(&mut buf), black_box("boom")).unwrap();
            black_box(buf);
        });
    });

    group.bench_function("long_message", |b| {
        let message = "Selection error: screen capture failed because the monitor disappeared";
        b.iter(|| {
            let mut buf = Vec::new();
            cli::write_error_line(black_box(&mut buf), black_box(message)).unwrap();
            black_box(buf);
        });
    });

    group.finish();
}

pub(crate) fn bench_child_process_is_running(c: &mut Criterion) {
    let mut group = c.benchmark_group("child_process_is_running");

    group.bench_function("pending_child", |b| {
        b.iter(|| cli::child_process_is_running::<(), ()>(black_box(Ok(None))));
    });

    group.bench_function("exited_child", |b| {
        b.iter(|| cli::child_process_is_running::<(), ()>(black_box(Ok(Some(())))));
    });

    group.bench_function("try_wait_error", |b| {
        b.iter(|| cli::child_process_is_running::<(), ()>(black_box(Err(()))));
    });

    group.finish();
}
