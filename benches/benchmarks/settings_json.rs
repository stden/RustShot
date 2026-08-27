//! Benchmarks — settings: hand-rolled JSON parse/format and paths.

use criterion::Criterion;
use rustshot::capture::Format;
use rustshot::settings;
use rustshot::settings::Fps;
use rustshot::settings::Settings;
use std::hint::black_box;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// settings::format_settings_json
// ---------------------------------------------------------------------------

pub(crate) fn bench_format_settings_json(c: &mut Criterion) {
    let s = Settings {
        screenshots_dir: PathBuf::from("/home/user/Pictures/RustShot"),
        recordings_dir: PathBuf::from("/home/user/Pictures/RustShot/recordings"),
        default_format: Format::Png,
        recording_fps: Fps::DEFAULT,
        auto_start: true,
    };
    let verbose = Settings {
        screenshots_dir: PathBuf::from("C:/Users/example/OneDrive - Company/Pictures/RustShot"),
        recordings_dir: PathBuf::from(
            "C:/Users/example/OneDrive - Company/Pictures/RustShot/recordings",
        ),
        default_format: Format::Bmp,
        recording_fps: Fps::new(144).unwrap(),
        auto_start: false,
    };
    let mut group = c.benchmark_group("format_settings_json");
    group.bench_function("default_like", |b| {
        b.iter(|| settings::format_settings_json(black_box(&s)));
    });
    group.bench_function("verbose_paths", |b| {
        b.iter(|| settings::format_settings_json(black_box(&verbose)));
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// settings::parse_settings_json
// ---------------------------------------------------------------------------

pub(crate) fn bench_parse_settings_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_settings_json");

    let json_full = r#"{
  "screenshots_dir": "/home/user/Pictures/RustShot",
  "recordings_dir": "/home/user/Pictures/RustShot/recordings",
  "default_format": "png",
  "recording_fps": 30,
  "auto_start": true
}"#;
    let duplicate_heavy = r#"{
  "screenshots_dir": "/tmp/a",
  "screenshots_dir": "/tmp/b",
  "recordings_dir": "/tmp/a/recordings",
  "recordings_dir": "/tmp/b/recordings",
  "default_format": "jpg",
  "default_format": "png",
  "recording_fps": 15,
  "recording_fps": 60,
  "auto_start": false,
  "auto_start": true
}"#;
    let malformed_tokens = r#"{
  "screenshots_dir": "/tmp/rustshot",
  "recordings_dir": "/tmp/rustshot/recordings",
  "default_format": "trueish",
  "recording_fps": 0,
  "auto_start": falsehood
}"#;

    group.bench_function("full_valid_json", |b| {
        b.iter(|| settings::parse_settings_json(black_box(json_full)));
    });

    group.bench_function("empty_string", |b| {
        b.iter(|| settings::parse_settings_json(black_box("")));
    });

    group.bench_function("empty_object", |b| {
        b.iter(|| settings::parse_settings_json(black_box("{}")));
    });

    let json_partial = r#"{ "default_format": "jpg" }"#;
    group.bench_function("partial_json", |b| {
        b.iter(|| settings::parse_settings_json(black_box(json_partial)));
    });

    group.bench_function("duplicate_keys", |b| {
        b.iter(|| settings::parse_settings_json(black_box(duplicate_heavy)));
    });

    group.bench_function("malformed_tokens", |b| {
        b.iter(|| settings::parse_settings_json(black_box(malformed_tokens)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// settings::settings_path / log_file_path
// ---------------------------------------------------------------------------

pub(crate) fn bench_settings_path(c: &mut Criterion) {
    c.bench_function("settings_path", |b| {
        b.iter(settings::settings_path);
    });
}

pub(crate) fn bench_log_file_path(c: &mut Criterion) {
    c.bench_function("log_file_path", |b| {
        b.iter(settings::log_file_path);
    });
}

pub(crate) fn bench_load_settings(c: &mut Criterion) {
    c.bench_function("load_settings", |b| {
        b.iter(settings::load_settings);
    });
}

pub(crate) fn bench_normalize_default_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_default_format");

    group.bench_function("png", |b| {
        b.iter(|| settings::benchmark_normalize_default_format(black_box("png")));
    });

    group.bench_function("jpg", |b| {
        b.iter(|| settings::benchmark_normalize_default_format(black_box("jpg")));
    });

    group.bench_function("unknown", |b| {
        b.iter(|| settings::benchmark_normalize_default_format(black_box("gif")));
    });

    group.bench_function("mixed_case", |b| {
        b.iter(|| settings::benchmark_normalize_default_format(black_box("PNG")));
    });

    group.finish();
}

pub(crate) fn bench_normalize_recording_fps(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_recording_fps");

    group.bench_function("zero", |b| {
        b.iter(|| settings::benchmark_normalize_recording_fps(black_box(0)));
    });

    group.bench_function("thirty", |b| {
        b.iter(|| settings::benchmark_normalize_recording_fps(black_box(30)));
    });

    group.bench_function("high", |b| {
        b.iter(|| settings::benchmark_normalize_recording_fps(black_box(240)));
    });

    group.finish();
}

pub(crate) fn bench_extract_json_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_json_string");
    let json = r#"{ "screenshots_dir": "/tmp/rustshot" }"#;

    group.bench_function("hit", |b| {
        b.iter(|| {
            settings::benchmark_extract_json_string(black_box(json), black_box("screenshots_dir"))
        });
    });

    group.bench_function("miss", |b| {
        b.iter(|| {
            settings::benchmark_extract_json_string(black_box(json), black_box("recordings_dir"))
        });
    });

    group.finish();
}

pub(crate) fn bench_extract_json_number(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_json_number");
    let json = r#"{ "recording_fps": 144 }"#;

    group.bench_function("hit", |b| {
        b.iter(|| {
            settings::benchmark_extract_json_number(black_box(json), black_box("recording_fps"))
        });
    });

    group.bench_function("miss", |b| {
        b.iter(|| settings::benchmark_extract_json_number(black_box(json), black_box("missing")));
    });

    group.finish();
}

pub(crate) fn bench_extract_json_bool(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_json_bool");
    let json = r#"{ "auto_start": true }"#;

    group.bench_function("hit", |b| {
        b.iter(|| settings::benchmark_extract_json_bool(black_box(json), black_box("auto_start")));
    });

    group.bench_function("miss", |b| {
        b.iter(|| settings::benchmark_extract_json_bool(black_box(json), black_box("missing")));
    });

    group.finish();
}
