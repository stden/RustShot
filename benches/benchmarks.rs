use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use image::RgbaImage;
use std::hint::black_box;
use std::path::PathBuf;

use rustshot::capture::{self, Format};
use rustshot::cli;
use rustshot::record;
use rustshot::selection;
use rustshot::settings::{self, Settings};
use rustshot::tray;

fn patterned_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 256) as u8).collect()
}

fn benchmark_screens() -> Result<Vec<screenshots::Screen>, String> {
    screenshots::Screen::all().map_err(|e| format!("Failed to get screens: {e}"))
}

fn benchmark_skip(group_name: &str, reason: &str) {
    eprintln!("Skipping benchmark group '{group_name}': {reason}");
}

// ---------------------------------------------------------------------------
// capture::bgra_to_rgba
// ---------------------------------------------------------------------------

fn bench_bgra_to_rgba(c: &mut Criterion) {
    let mut group = c.benchmark_group("bgra_to_rgba");

    // 4 pixels (16 bytes)
    group.bench_function("4px", |b| {
        let src: Vec<u8> = (0..16).collect();
        b.iter(|| {
            let mut buf = src.clone();
            capture::bgra_to_rgba(black_box(&mut buf));
        });
    });

    // 1K pixels (4 096 bytes)
    group.bench_function("1K_px", |b| {
        let src: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
        b.iter(|| {
            let mut buf = src.clone();
            capture::bgra_to_rgba(black_box(&mut buf));
        });
    });

    // 1M pixels (4 194 304 bytes) — roughly a 1024x1024 image
    group.bench_function("1M_px", |b| {
        let src = patterned_bytes(4_194_304);
        b.iter(|| {
            let mut buf = src.clone();
            capture::bgra_to_rgba(black_box(&mut buf));
        });
    });

    group.bench_function("4k_frame", |b| {
        let src = patterned_bytes(3840 * 2160 * 4);
        b.iter(|| {
            let mut buf = src.clone();
            capture::bgra_to_rgba(black_box(&mut buf));
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::resolve_screen_index
// ---------------------------------------------------------------------------

fn bench_resolve_screen_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_screen_index");

    group.bench_function("none_requested", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(3), black_box(None)));
    });

    group.bench_function("valid_index", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(3), black_box(Some(1))));
    });

    group.bench_function("out_of_range", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(3), black_box(Some(5))));
    });

    group.bench_function("zero_screens", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(0), black_box(None)));
    });

    group.bench_function("large_valid_index", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(16), black_box(Some(15))));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::default_screen_index
// ---------------------------------------------------------------------------

fn bench_default_screen_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("default_screen_index");

    group.bench_function("primary_first", |b| {
        b.iter(|| capture::default_screen_index(black_box(vec![true, false, false])));
    });

    group.bench_function("primary_last", |b| {
        b.iter(|| capture::default_screen_index(black_box(vec![false, false, true])));
    });

    group.bench_function("no_primary", |b| {
        b.iter(|| capture::default_screen_index(black_box(vec![false, false, false])));
    });

    group.bench_function("primary_after_many_screens", |b| {
        let flags = vec![
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true,
        ];
        b.iter(|| capture::default_screen_index(black_box(flags.clone())));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::resolve_output_path
// ---------------------------------------------------------------------------

fn bench_resolve_output_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_output_path");

    group.bench_function("with_explicit_path", |b| {
        b.iter(|| {
            capture::resolve_output_path(
                black_box(Some(PathBuf::from("/tmp/shot.png"))),
                black_box(Format::Png),
            )
        });
    });

    group.bench_function("generate_default", |b| {
        b.iter(|| capture::resolve_output_path(black_box(None), black_box(Format::Png)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::resolve_output_path_in_dir
// ---------------------------------------------------------------------------

fn bench_resolve_output_path_in_dir(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_output_path_in_dir");

    let dir = PathBuf::from("/tmp/custom-shots");

    group.bench_function("with_explicit_path", |b| {
        b.iter(|| {
            capture::resolve_output_path_in_dir(
                black_box(Some(PathBuf::from("/tmp/shot.jpg"))),
                black_box(Format::Jpg),
                black_box(&dir),
            )
        });
    });

    group.bench_function("generate_default_png", |b| {
        b.iter(|| {
            capture::resolve_output_path_in_dir(
                black_box(None),
                black_box(Format::Png),
                black_box(&dir),
            )
        });
    });

    group.bench_function("generate_default_bmp", |b| {
        b.iter(|| {
            capture::resolve_output_path_in_dir(
                black_box(None),
                black_box(Format::Bmp),
                black_box(&dir),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::Format::extension
// ---------------------------------------------------------------------------

fn bench_format_extension(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_extension");

    group.bench_function("png", |b| {
        b.iter(|| {
            let ext = Format::Png.extension();
            black_box(ext);
        });
    });

    group.bench_function("jpg", |b| {
        b.iter(|| {
            let ext = Format::Jpg.extension();
            black_box(ext);
        });
    });

    group.bench_function("bmp", |b| {
        b.iter(|| {
            let ext = Format::Bmp.extension();
            black_box(ext);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::Format::to_image_format
// ---------------------------------------------------------------------------

fn bench_format_to_image_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_to_image_format");

    group.bench_function("png", |b| {
        b.iter(|| black_box(Format::Png).to_image_format());
    });

    group.bench_function("jpg", |b| {
        b.iter(|| black_box(Format::Jpg).to_image_format());
    });

    group.bench_function("bmp", |b| {
        b.iter(|| black_box(Format::Bmp).to_image_format());
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::get_screenshots_dir
// ---------------------------------------------------------------------------

fn bench_get_screenshots_dir(c: &mut Criterion) {
    c.bench_function("get_screenshots_dir", |b| {
        b.iter(capture::get_screenshots_dir);
    });
}

// ---------------------------------------------------------------------------
// capture::list_screens
// ---------------------------------------------------------------------------

fn bench_list_screens(c: &mut Criterion) {
    c.bench_function("list_screens", |b| {
        b.iter(capture::list_screens);
    });
}

// ---------------------------------------------------------------------------
// capture::benchmark_save_image_optimized / benchmark_ensure_parent_directory
// ---------------------------------------------------------------------------

fn bench_save_image_optimized(c: &mut Criterion) {
    let mut group = c.benchmark_group("save_image_optimized");
    let dir = std::env::temp_dir().join("rustshot_bench_save_image_optimized");
    std::fs::create_dir_all(&dir).ok();
    let image = RgbaImage::from_pixel(1920, 1080, image::Rgba([100, 150, 200, 255]));

    group.bench_function("png", |b| {
        let path = dir.join("save.png");
        b.iter(|| {
            capture::benchmark_save_image_optimized(
                black_box(&image),
                black_box(path.as_path()),
                black_box(Format::Png),
            )
            .unwrap();
        });
    });

    group.bench_function("jpg", |b| {
        let path = dir.join("save.jpg");
        b.iter(|| {
            capture::benchmark_save_image_optimized(
                black_box(&image),
                black_box(path.as_path()),
                black_box(Format::Jpg),
            )
            .unwrap();
        });
    });

    group.bench_function("bmp", |b| {
        let path = dir.join("save.bmp");
        b.iter(|| {
            capture::benchmark_save_image_optimized(
                black_box(&image),
                black_box(path.as_path()),
                black_box(Format::Bmp),
            )
            .unwrap();
        });
    });

    group.finish();
    std::fs::remove_dir_all(&dir).ok();
}

fn bench_ensure_parent_directory(c: &mut Criterion) {
    let mut group = c.benchmark_group("ensure_parent_directory");
    let root = std::env::temp_dir().join("rustshot_bench_ensure_parent_directory");

    group.bench_function("existing_parent", |b| {
        let existing_dir = root.join("existing");
        std::fs::create_dir_all(&existing_dir).ok();
        let path = existing_dir.join("shot.png");
        b.iter(|| {
            capture::benchmark_ensure_parent_directory(black_box(path.as_path())).unwrap();
        });
    });

    group.bench_function("create_nested_parent", |b| {
        let nested_root = root.join("recreate");
        let path = nested_root.join("deep/shot.png");
        b.iter_batched(
            || {
                let _ = std::fs::remove_dir_all(&nested_root);
                path.clone()
            },
            |path| {
                capture::benchmark_ensure_parent_directory(black_box(path.as_path())).unwrap();
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
    std::fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// record::frame_duration_hns
// ---------------------------------------------------------------------------

fn bench_frame_duration_hns(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_duration_hns");

    group.bench_function("30fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(30)));
    });

    group.bench_function("60fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(60)));
    });

    group.bench_function("0fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(0)));
    });

    group.bench_function("120fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(120)));
    });

    group.bench_function("240fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(240)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::round_to_even
// ---------------------------------------------------------------------------

fn bench_round_to_even(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_to_even");

    group.bench_function("even_value", |b| {
        b.iter(|| record::round_to_even(black_box(1920)));
    });

    group.bench_function("odd_value", |b| {
        b.iter(|| record::round_to_even(black_box(1921)));
    });

    group.bench_function("zero", |b| {
        b.iter(|| record::round_to_even(black_box(0)));
    });

    group.bench_function("one", |b| {
        b.iter(|| record::round_to_even(black_box(1)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::rgba_to_bgra_bottom_up
// ---------------------------------------------------------------------------

fn bench_rgba_to_bgra_bottom_up(c: &mut Criterion) {
    let mut group = c.benchmark_group("rgba_to_bgra_bottom_up");

    // 1x1
    {
        let data: Vec<u8> = vec![255, 0, 0, 255];
        group.bench_function("1x1", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(1), black_box(1)));
        });
    }

    // 64x64
    {
        let w: u32 = 64;
        let h: u32 = 64;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("64x64", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 640x480
    {
        let w: u32 = 640;
        let h: u32 = 480;
        let data: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i % 256) as u8).collect();
        group.bench_function("640x480", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 1920x1080
    {
        let w: u32 = 1920;
        let h: u32 = 1080;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("1920x1080", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 1919x1079 (odd dimensions)
    {
        let w: u32 = 1919;
        let h: u32 = 1079;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("1919x1079", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 2560x1440
    {
        let w: u32 = 2560;
        let h: u32 = 1440;
        let data: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i % 256) as u8).collect();
        group.bench_function("2560x1440", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 3840x2160
    {
        let w: u32 = 3840;
        let h: u32 = 2160;
        let data: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i % 256) as u8).collect();
        group.bench_function("3840x2160", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// record::pack_u32_pair
// ---------------------------------------------------------------------------

fn bench_pack_u32_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("pack_u32_pair");

    group.bench_function("frame_size_1920x1080", |b| {
        b.iter(|| record::pack_u32_pair(black_box(1920), black_box(1080)));
    });

    group.bench_function("fps_30_1", |b| {
        b.iter(|| record::pack_u32_pair(black_box(30), black_box(1)));
    });

    group.bench_function("max_values", |b| {
        b.iter(|| record::pack_u32_pair(black_box(u32::MAX), black_box(u32::MAX)));
    });

    group.bench_function("zero_zero", |b| {
        b.iter(|| record::pack_u32_pair(black_box(0), black_box(0)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::validate_recording_start
// ---------------------------------------------------------------------------

fn bench_validate_recording_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_recording_start");

    group.bench_function("valid_fps", |b| {
        b.iter(|| record::validate_recording_start(black_box(30)));
    });

    group.bench_function("zero_fps", |b| {
        b.iter(|| record::validate_recording_start(black_box(0)));
    });

    group.bench_function("high_fps", |b| {
        b.iter(|| record::validate_recording_start(black_box(240)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::get_recording_dir / get_recording_path / get_recording_path_in_dir
// ---------------------------------------------------------------------------

fn bench_get_recording_dir(c: &mut Criterion) {
    c.bench_function("get_recording_dir", |b| {
        b.iter(record::get_recording_dir);
    });
}

fn bench_get_recording_path(c: &mut Criterion) {
    c.bench_function("get_recording_path", |b| {
        b.iter(record::get_recording_path);
    });
}

fn bench_get_recording_path_in_dir(c: &mut Criterion) {
    let dir = PathBuf::from("/tmp/custom-recordings");
    c.bench_function("get_recording_path_in_dir", |b| {
        b.iter(|| record::get_recording_path_in_dir(black_box(&dir)));
    });
}

// ---------------------------------------------------------------------------
// settings::format_settings_json
// ---------------------------------------------------------------------------

fn bench_format_settings_json(c: &mut Criterion) {
    let s = Settings {
        screenshots_dir: PathBuf::from("/home/user/Pictures/RustShot"),
        recordings_dir: PathBuf::from("/home/user/Pictures/RustShot/recordings"),
        default_format: "png".to_string(),
        recording_fps: 30,
        auto_start: true,
    };
    let verbose = Settings {
        screenshots_dir: PathBuf::from("C:/Users/example/OneDrive - Company/Pictures/RustShot"),
        recordings_dir: PathBuf::from(
            "C:/Users/example/OneDrive - Company/Pictures/RustShot/recordings",
        ),
        default_format: "bmp".to_string(),
        recording_fps: 144,
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

fn bench_parse_settings_json(c: &mut Criterion) {
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

fn bench_settings_path(c: &mut Criterion) {
    c.bench_function("settings_path", |b| {
        b.iter(settings::settings_path);
    });
}

fn bench_log_file_path(c: &mut Criterion) {
    c.bench_function("log_file_path", |b| {
        b.iter(settings::log_file_path);
    });
}

fn bench_load_settings(c: &mut Criterion) {
    c.bench_function("load_settings", |b| {
        b.iter(settings::load_settings);
    });
}

fn bench_normalize_default_format(c: &mut Criterion) {
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

fn bench_normalize_recording_fps(c: &mut Criterion) {
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

fn bench_extract_json_string(c: &mut Criterion) {
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

fn bench_extract_json_number(c: &mut Criterion) {
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

fn bench_extract_json_bool(c: &mut Criterion) {
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

// ---------------------------------------------------------------------------
// cli helpers
// ---------------------------------------------------------------------------

fn bench_settings_capture_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("settings_capture_format");

    group.bench_function("png_fallback", |b| {
        b.iter(|| cli::settings_capture_format(black_box("gif")));
    });

    group.bench_function("jpg", |b| {
        b.iter(|| cli::settings_capture_format(black_box("jpg")));
    });

    group.bench_function("bmp", |b| {
        b.iter(|| cli::settings_capture_format(black_box("bmp")));
    });

    group.finish();
}

fn bench_cli_format_flag_present_in(c: &mut Criterion) {
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

fn bench_cli_path_helpers(c: &mut Criterion) {
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

fn bench_effective_cli_format(c: &mut Criterion) {
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

fn bench_should_run_cli_mode(c: &mut Criterion) {
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

fn bench_format_screen_list_output(c: &mut Criterion) {
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

fn bench_selection_stderr_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("selection_stderr_message");

    group.bench_function("cancelled", |b| {
        b.iter(|| cli::selection_stderr_message(black_box("Selection cancelled")));
    });

    group.bench_function("error", |b| {
        b.iter(|| cli::selection_stderr_message(black_box("Selection error: boom")));
    });

    group.finish();
}

fn bench_write_error_line(c: &mut Criterion) {
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

fn bench_child_process_is_running(c: &mut Criterion) {
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

// ---------------------------------------------------------------------------
// selection::build_region_from_drag
// ---------------------------------------------------------------------------

fn bench_build_region_from_drag(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_region_from_drag");

    group.bench_function("valid_region", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((0, 0)), black_box((1920, 1080)));
        });
    });

    group.bench_function("too_small", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((0, 0)), black_box((3, 3)));
        });
    });

    group.bench_function("reversed_coords", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((1920, 1080)), black_box((0, 0)));
        });
    });

    group.bench_function("large_region", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((0, 0)), black_box((3840, 2160)));
        });
    });

    group.bench_function("negative_coordinates", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((-1200, -800)), black_box((-20, -10)));
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::virtual_desktop_scale_factor
// ---------------------------------------------------------------------------

fn bench_virtual_desktop_scale_factor(c: &mut Criterion) {
    let mut group = c.benchmark_group("virtual_desktop_scale_factor");

    group.bench_function("primary_first", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(vec![
                (true, 1.0),
                (false, 2.0),
                (false, 1.5),
            ]));
        });
    });

    group.bench_function("no_primary", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(vec![
                (false, 1.0),
                (false, 2.0),
                (false, 1.5),
            ]));
        });
    });

    group.bench_function("single_screen", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(vec![(true, 1.5)]));
        });
    });

    group.bench_function("empty", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(Vec::<(bool, f32)>::new()));
        });
    });

    group.bench_function("many_monitors_primary_last", |b| {
        let screens = vec![
            (false, 1.0),
            (false, 1.25),
            (false, 1.5),
            (false, 2.0),
            (true, 1.75),
        ];
        b.iter(|| selection::virtual_desktop_scale_factor(black_box(screens.clone())));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::logical_to_physical
// ---------------------------------------------------------------------------

fn bench_logical_to_physical(c: &mut Criterion) {
    let mut group = c.benchmark_group("logical_to_physical");

    group.bench_function("scale_1.0", |b| {
        b.iter(|| selection::logical_to_physical(black_box(1920), black_box(1.0)));
    });

    group.bench_function("scale_1.5", |b| {
        b.iter(|| selection::logical_to_physical(black_box(1920), black_box(1.5)));
    });

    group.bench_function("scale_2.0", |b| {
        b.iter(|| selection::logical_to_physical(black_box(1920), black_box(2.0)));
    });

    group.bench_function("negative_value", |b| {
        b.iter(|| selection::logical_to_physical(black_box(-1920), black_box(1.5)));
    });

    group.bench_function("negative_125pct", |b| {
        b.iter(|| selection::logical_to_physical(black_box(-1440), black_box(1.25)));
    });

    group.bench_function("zero_200pct", |b| {
        b.iter(|| selection::logical_to_physical(black_box(0), black_box(2.0)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::find_screen_index_at_position
// ---------------------------------------------------------------------------

fn bench_find_screen_index_at_position(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_screen_index_at_position");

    group.bench_function("single_screen_hit", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32)];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((960, 540)),
            );
        });
    });

    group.bench_function("single_screen_miss", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32)];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((5000, 5000)),
            );
        });
    });

    group.bench_function("two_screens_hit_second", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32), (1920, 0, 2560u32, 1440u32)];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((2500, 500)),
            );
        });
    });

    group.bench_function("four_screens_hit_last", |b| {
        let screens = vec![
            (0, 0, 1920u32, 1080u32),
            (1920, 0, 1920u32, 1080u32),
            (0, 1080, 1920u32, 1080u32),
            (1920, 1080, 1920u32, 1080u32),
        ];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((2500, 1500)),
            );
        });
    });

    group.bench_function("empty_screens", |b| {
        let screens: Vec<(i32, i32, u32, u32)> = vec![];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((500, 500)),
            );
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::preferred_screen_index
// ---------------------------------------------------------------------------

fn bench_preferred_screen_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("preferred_screen_index");

    group.bench_function("cursor_on_screen", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, true),
            (1920, 0, 2560u32, 1440u32, false),
        ];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((2500, 500))),
            );
        });
    });

    group.bench_function("cursor_miss_fallback_primary", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, false),
            (1920, 0, 2560u32, 1440u32, true),
        ];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((9999, 9999))),
            );
        });
    });

    group.bench_function("no_cursor_fallback_primary", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, false),
            (1920, 0, 2560u32, 1440u32, true),
        ];
        b.iter(|| {
            selection::preferred_screen_index(black_box(screens.clone()), black_box(None));
        });
    });

    group.bench_function("no_cursor_no_primary_fallback_first", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, false),
            (1920, 0, 2560u32, 1440u32, false),
        ];
        b.iter(|| {
            selection::preferred_screen_index(black_box(screens.clone()), black_box(None));
        });
    });

    group.bench_function("three_screens_cursor_on_middle", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, true),
            (1920, 0, 2560u32, 1440u32, false),
            (4480, 0, 1920u32, 1080u32, false),
        ];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((3000, 700))),
            );
        });
    });

    group.bench_function("empty_screens", |b| {
        let screens: Vec<(i32, i32, u32, u32, bool)> = vec![];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((500, 500))),
            );
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::get_cursor_position
// ---------------------------------------------------------------------------

fn bench_get_cursor_position(c: &mut Criterion) {
    c.bench_function("get_cursor_position", |b| {
        b.iter(selection::get_cursor_position);
    });
}

// ---------------------------------------------------------------------------
// selection::scaled_physical_dimension
// ---------------------------------------------------------------------------

fn bench_scaled_physical_dimension(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaled_physical_dimension");

    group.bench_function("scale_1_0", |b| {
        b.iter(|| selection::benchmark_scaled_physical_dimension(black_box(1920), black_box(1.0)));
    });

    group.bench_function("scale_1_333", |b| {
        b.iter(|| {
            selection::benchmark_scaled_physical_dimension(black_box(1000), black_box(1.333))
        });
    });

    group.bench_function("scale_1_5", |b| {
        b.iter(|| selection::benchmark_scaled_physical_dimension(black_box(1707), black_box(1.5)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::build_screen_layout_inputs_physical_sizes
// ---------------------------------------------------------------------------

fn bench_build_screen_layout_inputs_physical_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_screen_layout_inputs_physical_sizes");

    group.bench_function("single_monitor", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32, 1.0f32, true)];
        b.iter(|| {
            selection::benchmark_build_screen_layout_inputs_physical_sizes(
                black_box(screens.clone()),
                black_box(0),
                black_box((1920, 1080)),
            )
        });
    });

    group.bench_function("two_monitors_mixed_dpi", |b| {
        let screens = vec![
            (0, 0, 1920u32, 1080u32, 1.0f32, true),
            (1920, 0, 1707u32, 960u32, 1.5f32, false),
        ];
        b.iter(|| {
            selection::benchmark_build_screen_layout_inputs_physical_sizes(
                black_box(screens.clone()),
                black_box(1),
                black_box((2560, 1440)),
            )
        });
    });

    group.bench_function("three_monitors", |b| {
        let screens = vec![
            (-1707, 0, 1707u32, 960u32, 1.5f32, false),
            (0, 0, 1920u32, 1080u32, 1.0f32, true),
            (1920, 0, 1920u32, 1080u32, 1.0f32, false),
        ];
        b.iter(|| {
            selection::benchmark_build_screen_layout_inputs_physical_sizes(
                black_box(screens.clone()),
                black_box(1),
                black_box((1920, 1080)),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::capture_screenshot (end-to-end: capture screen + save to file)
// ---------------------------------------------------------------------------

fn bench_capture_screenshot(c: &mut Criterion) {
    let dir = std::env::temp_dir().join("rustshot_bench");
    std::fs::create_dir_all(&dir).ok();

    let screen_count = match capture::benchmark_screen_count_from_result(benchmark_screens()) {
        Ok(screen_count) => screen_count,
        Err(err) => {
            benchmark_skip("capture_screenshot", &err);
            std::fs::remove_dir_all(&dir).ok();
            return;
        }
    };
    let mut group = c.benchmark_group("capture_screenshot");

    // Benchmark: default screen (cursor-based or primary)
    group.bench_function("default_screen", |b| {
        b.iter(|| {
            let _ = capture::capture_screenshot_in_dir(
                black_box(None),
                black_box(Format::Png),
                black_box(None),
                black_box(&dir),
            );
        });
    });

    // Benchmark: each screen individually
    for i in 0..screen_count {
        group.bench_function(format!("screen_{}", i), |b| {
            b.iter(|| {
                let _ = capture::capture_screenshot_in_dir(
                    black_box(None),
                    black_box(Format::Png),
                    black_box(Some(i)),
                    black_box(&dir),
                );
            });
        });
    }

    // Benchmark: all screens sequentially
    group.bench_function("all_screens", |b| {
        b.iter(|| {
            for i in 0..screen_count {
                let _ = capture::capture_screenshot_in_dir(
                    black_box(None),
                    black_box(Format::Png),
                    black_box(Some(i)),
                    black_box(&dir),
                );
            }
        });
    });

    group.finish();

    // Cleanup
    std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// capture_screenshot_stages — per-stage profiling of the screenshot pipeline
// ---------------------------------------------------------------------------

fn bench_capture_screenshot_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("capture_screenshot_stages");

    // Stage 1: screen capture only (no save)
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => {
            group.bench_function("screen_capture_only", |b| {
                b.iter(|| {
                    let _ = black_box(screen.capture());
                });
            });
        }
        Err(err) => benchmark_skip("capture_screenshot_stages/screen_capture_only", &err),
    }

    // Stage 2: image conversion (RgbaImage::from_raw on a captured frame)
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => match screen.capture() {
            Ok(captured) => {
                let w = captured.width();
                let h = captured.height();
                let raw = captured.into_raw();
                group.bench_function("image_conversion", |b| {
                    b.iter(|| {
                        let _ = black_box(RgbaImage::from_raw(w, h, raw.clone()));
                    });
                });
            }
            Err(err) => benchmark_skip(
                "capture_screenshot_stages/image_conversion",
                &format!("Failed to capture screen: {err}"),
            ),
        },
        Err(err) => benchmark_skip("capture_screenshot_stages/image_conversion", &err),
    }

    // Stage 3-9: save benchmarks on a synthetic 1920x1080 image
    let synth = RgbaImage::from_pixel(1920, 1080, image::Rgba([100, 150, 200, 255]));
    let synth_rgb = capture::benchmark_rgb_image_from_rgba(&synth);
    let bench_dir = std::env::temp_dir().join("rustshot_bench_stages");
    std::fs::create_dir_all(&bench_dir).ok();

    // save_png_default — image.save_with_format(Png)
    group.bench_function("save_png_default", |b| {
        let path = bench_dir.join("bench_default.png");
        b.iter(|| {
            synth
                .save_with_format(black_box(&path), image::ImageFormat::Png)
                .unwrap();
        });
    });

    // save_png_fast — PngEncoder with CompressionType::Fast, FilterType::Sub
    group.bench_function("save_png_fast", |b| {
        let path = bench_dir.join("bench_fast.png");
        b.iter(|| {
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            use std::io::BufWriter;
            let file = std::fs::File::create(black_box(&path)).unwrap();
            let writer = BufWriter::new(file);
            let encoder =
                PngEncoder::new_with_quality(writer, CompressionType::Fast, FilterType::Sub);
            synth.write_with_encoder(encoder).unwrap();
        });
    });

    // save_png_best — PngEncoder with BestCompression, Adaptive filter
    group.bench_function("save_png_best", |b| {
        let path = bench_dir.join("bench_best.png");
        b.iter(|| {
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            use std::io::BufWriter;
            let file = std::fs::File::create(black_box(&path)).unwrap();
            let writer = BufWriter::new(file);
            let encoder =
                PngEncoder::new_with_quality(writer, CompressionType::Best, FilterType::Adaptive);
            synth.write_with_encoder(encoder).unwrap();
        });
    });

    // save_jpg_default — image.save_with_format(Jpeg)
    group.bench_function("save_jpg_default", |b| {
        let path = bench_dir.join("bench_default.jpg");
        b.iter(|| {
            synth_rgb
                .save_with_format(black_box(&path), image::ImageFormat::Jpeg)
                .unwrap();
        });
    });

    // save_jpg_q75 — JpegEncoder with quality 75
    group.bench_function("save_jpg_q75", |b| {
        let path = bench_dir.join("bench_q75.jpg");
        b.iter(|| {
            use image::codecs::jpeg::JpegEncoder;
            use std::io::BufWriter;
            let file = std::fs::File::create(black_box(&path)).unwrap();
            let writer = BufWriter::new(file);
            let encoder = JpegEncoder::new_with_quality(writer, 75);
            synth_rgb.write_with_encoder(encoder).unwrap();
        });
    });

    // save_bmp — image.save_with_format(Bmp)
    group.bench_function("save_bmp", |b| {
        let path = bench_dir.join("bench.bmp");
        b.iter(|| {
            synth
                .save_with_format(black_box(&path), image::ImageFormat::Bmp)
                .unwrap();
        });
    });

    group.finish();
    std::fs::remove_dir_all(&bench_dir).ok();
}

// ---------------------------------------------------------------------------
// recording_stages — per-stage profiling of the video recording pipeline
// ---------------------------------------------------------------------------

fn bench_recording_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("recording_stages");

    // Stage 1: screen capture raw (same as screenshot, confirms recording context)
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => {
            group.bench_function("screen_capture_raw", |b| {
                b.iter(|| {
                    let _ = black_box(screen.capture());
                });
            });
        }
        Err(err) => benchmark_skip("recording_stages/screen_capture_raw", &err),
    }

    // Stage 2: image_from_raw — RgbaImage::from_raw on a captured frame
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => match screen.capture() {
            Ok(captured) => {
                let w = captured.width();
                let h = captured.height();
                let raw = captured.into_raw();
                group.bench_function("image_from_raw", |b| {
                    b.iter(|| {
                        let _ = black_box(RgbaImage::from_raw(w, h, raw.clone()));
                    });
                });
            }
            Err(err) => benchmark_skip(
                "recording_stages/image_from_raw",
                &format!("Failed to capture screen: {err}"),
            ),
        },
        Err(err) => benchmark_skip("recording_stages/image_from_raw", &err),
    }

    // Stage 3: crop_region — crop a 800x600 region from a 1920x1080 frame
    {
        let frame = RgbaImage::from_pixel(1920, 1080, image::Rgba([100, 150, 200, 255]));
        group.bench_function("crop_region", |b| {
            b.iter(|| {
                let cropped =
                    image::imageops::crop_imm(black_box(&frame), 100, 100, 800, 600).to_image();
                black_box(cropped);
            });
        });
    }

    // Stage 4: pad_to_even — pad a cropped image to even dimensions
    {
        let cropped = RgbaImage::from_pixel(799, 599, image::Rgba([50, 100, 150, 255]));
        let even_w = record::round_to_even(799);
        let even_h = record::round_to_even(599);
        group.bench_function("pad_to_even", |b| {
            b.iter(|| {
                let mut padded = RgbaImage::new(even_w, even_h);
                image::imageops::replace(&mut padded, black_box(&cropped), 0, 0);
                black_box(padded);
            });
        });
    }

    // Stage 5: rgba_to_bgra_bottom_up — 1920x1080
    {
        let w: u32 = 1920;
        let h: u32 = 1080;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("rgba_to_bgra_bottom_up_1080p", |b| {
            b.iter(|| {
                let _ =
                    record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h));
            });
        });
    }

    // Full per-frame pipeline (capture + convert + crop + pad + color-convert), no WMF write
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => match screen.capture() {
            Ok(captured) => {
                let info = screen.display_info;
                // Use a region that's a subset of the screen
                let region_w = (info.width / 2).min(800);
                let region_h = (info.height / 2).min(600);
                let even_w = record::round_to_even(region_w);
                let even_h = record::round_to_even(region_h);
                let probe_raw = captured.into_raw();

                group.bench_function("recording_frame_pipeline", |b| {
                    b.iter(|| {
                        if let Ok(captured) = screen.capture() {
                            if let Some(img) = RgbaImage::from_raw(
                                captured.width(),
                                captured.height(),
                                captured.into_raw(),
                            ) {
                                let cropped =
                                    image::imageops::crop_imm(&img, 0, 0, region_w, region_h)
                                        .to_image();
                                let mut padded = RgbaImage::new(even_w, even_h);
                                image::imageops::replace(&mut padded, &cropped, 0, 0);
                                let bgra =
                                    record::rgba_to_bgra_bottom_up(padded.as_raw(), even_w, even_h);
                                black_box(bgra);
                            } else {
                                black_box(&probe_raw);
                            }
                        } else {
                            black_box(&probe_raw);
                        }
                    });
                });
            }
            Err(err) => benchmark_skip(
                "recording_stages/recording_frame_pipeline",
                &format!("Failed to capture screen: {err}"),
            ),
        },
        Err(err) => benchmark_skip("recording_stages/recording_frame_pipeline", &err),
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// tray::benchmark_build_default_icon_pixels
// ---------------------------------------------------------------------------

fn bench_build_default_icon_pixels(c: &mut Criterion) {
    c.bench_function("build_default_icon_pixels", |b| {
        b.iter(tray::benchmark_build_default_icon_pixels);
    });
}

// ---------------------------------------------------------------------------
// selection::build_physical_screen_layouts
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn bench_build_physical_screen_layouts(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_physical_screen_layouts");

    group.bench_function("single_1080p", |b| {
        let inputs = vec![(0, 0, 1920u32, 1080u32, 1920u32, 1080u32, true)];
        b.iter(|| selection::benchmark_build_physical_screen_layouts(black_box(&inputs)));
    });

    group.bench_function("two_horizontal", |b| {
        let inputs = vec![
            (0, 0, 1920, 1080, 1920, 1080, true),
            (1920, 0, 2560, 1440, 2560, 1440, false),
        ];
        b.iter(|| selection::benchmark_build_physical_screen_layouts(black_box(&inputs)));
    });

    group.bench_function("three_mixed_dpi", |b| {
        let inputs = vec![
            (-2560, 0, 1707, 960, 2560, 1440, false),
            (0, 0, 1920, 1080, 1920, 1080, true),
            (1920, 0, 1920, 1080, 1920, 1080, false),
        ];
        b.iter(|| selection::benchmark_build_physical_screen_layouts(black_box(&inputs)));
    });

    group.bench_function("eight_monitor_chain", |b| {
        let inputs = vec![
            (-11520, 600, 1920, 1080, 1920, 1080, false),
            (-9600, 500, 1920, 1080, 2400, 1350, false),
            (-7680, 400, 1920, 1080, 1920, 1080, false),
            (-5760, 300, 1920, 1080, 2880, 1620, false),
            (-3840, 200, 1920, 1080, 1920, 1080, false),
            (-1920, 100, 1920, 1080, 2400, 1350, false),
            (0, 0, 1920, 1080, 1920, 1080, true),
            (2000, 1200, 1920, 1080, 1920, 1080, false),
        ];
        b.iter(|| selection::benchmark_build_physical_screen_layouts(black_box(&inputs)));
    });

    group.finish();
}

fn bench_padded_recording_region(c: &mut Criterion) {
    let mut group = c.benchmark_group("padded_recording_region");
    let src_w = 1919u32;
    let src_h = 1079u32;
    let dst_w = 1920u32;
    let dst_h = 1080u32;
    let src = patterned_bytes((src_w * src_h * 4) as usize);
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];

    group.bench_function("rgba_1919x1079_to_1920x1080", |b| {
        b.iter(|| {
            record::rgba_region_to_bgra_bottom_up_into(
                black_box(&src),
                black_box(src_w),
                black_box(src_h),
                black_box(&mut out),
                black_box(dst_w),
                black_box(dst_h),
            );
        });
    });

    group.bench_function("bgra_1919x1079_to_1920x1080", |b| {
        b.iter(|| {
            record::bgra_top_down_region_to_bottom_up_into(
                black_box(&src),
                black_box(src_w),
                black_box(src_h),
                black_box(&mut out),
                black_box(dst_w),
                black_box(dst_h),
            );
        });
    });

    group.finish();
}

fn bench_overlay_frame_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlay_frame_update");
    let w = 1920usize;
    let h = 1080usize;
    let bg = vec![0x00336699u32; w * h];
    let mut buf = bg.clone();

    group.bench_function("drag_border_1080p", |b| {
        let mut damage = selection::OverlayDamage {
            surface_size: Some((w, h)),
            start_pos: Some((100, 100)),
            current_pos: Some((1500, 900)),
        };
        let mut toggle = false;
        b.iter(|| {
            toggle = !toggle;
            let end = if toggle { (1501, 901) } else { (1500, 900) };
            selection::update_selection_overlay_frame(
                black_box(&mut buf),
                black_box(w),
                black_box(h),
                black_box(&bg),
                black_box(w),
                black_box(Some((100, 100))),
                black_box(Some(end)),
                black_box(&mut damage),
            );
        });
    });

    group.bench_function("crosshair_1080p", |b| {
        let mut damage = selection::OverlayDamage {
            surface_size: Some((w, h)),
            start_pos: None,
            current_pos: Some((960, 540)),
        };
        let mut toggle = false;
        b.iter(|| {
            toggle = !toggle;
            let pos = if toggle { (961, 541) } else { (960, 540) };
            selection::update_selection_overlay_frame(
                black_box(&mut buf),
                black_box(w),
                black_box(h),
                black_box(&bg),
                black_box(w),
                black_box(None),
                black_box(Some(pos)),
                black_box(&mut damage),
            );
        });
    });

    group.finish();
}

fn bench_crop_packed_region(c: &mut Criterion) {
    let mut group = c.benchmark_group("crop_packed_region");
    let w = 1920u32;
    let h = 1080u32;
    let packed: Vec<u32> = (0..(w * h))
        .map(|i| 0xFF00_0000 | (i & 0x00FF_FFFF))
        .collect();
    let region = selection::Region {
        physical_x: 200,
        physical_y: 150,
        physical_width: 960,
        physical_height: 540,
    };

    group.bench_function("960x540_from_1080p", |b| {
        b.iter(|| {
            selection::crop_packed_region(
                black_box(w),
                black_box(h),
                black_box(&packed),
                black_box(region),
            )
            .unwrap()
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::normalize_physical_screen_layouts
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn bench_normalize_physical_screen_layouts(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_physical_screen_layouts");

    group.bench_function("single_screen", |b| {
        let inputs = vec![(0, 0, 1920u32, 1080u32, 1920u32, 1080u32, true)];
        b.iter(|| selection::benchmark_normalize_physical_screen_layouts(black_box(&inputs)));
    });

    group.bench_function("two_horizontal_with_offset", |b| {
        let inputs = vec![
            (0, 0, 1920, 1080, 1920, 1080, true),
            (1920, 0, 2560, 1440, 2560, 1440, false),
        ];
        b.iter(|| selection::benchmark_normalize_physical_screen_layouts(black_box(&inputs)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::resolve_recording_selection
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn bench_resolve_recording_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_recording_selection");

    let layouts = vec![
        (0, 0, 1920, 1080, 0, 0, 1920u32, 1080u32),
        (1920, 0, 2560, 1440, 1920, 0, 2560u32, 1440u32),
    ];

    group.bench_function("region_in_first_screen", |b| {
        b.iter(|| {
            selection::benchmark_resolve_recording_selection(
                black_box(&layouts),
                black_box((100, 100, 800, 600)),
            )
        });
    });

    group.bench_function("region_in_second_screen", |b| {
        b.iter(|| {
            selection::benchmark_resolve_recording_selection(
                black_box(&layouts),
                black_box((2000, 100, 800, 600)),
            )
        });
    });

    group.bench_function("region_spanning_two_screens_error", |b| {
        b.iter(|| {
            selection::benchmark_resolve_recording_selection(
                black_box(&layouts),
                black_box((1800, 100, 400, 600)),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::ranges_overlap
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn bench_ranges_overlap(c: &mut Criterion) {
    let mut group = c.benchmark_group("ranges_overlap");

    group.bench_function("overlapping", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(0),
                black_box(100),
                black_box(50),
                black_box(100),
            )
        });
    });

    group.bench_function("disjoint", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(0),
                black_box(100),
                black_box(200),
                black_box(100),
            )
        });
    });

    group.bench_function("touching", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(0),
                black_box(100),
                black_box(100),
                black_box(100),
            )
        });
    });

    group.bench_function("negative_start", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(-100),
                black_box(200),
                black_box(50),
                black_box(100),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::monitor_scale_ratio
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn bench_monitor_scale_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("monitor_scale_ratio");

    group.bench_function("1x", |b| {
        b.iter(|| selection::benchmark_monitor_scale_ratio(black_box(1920), black_box(1920)));
    });

    group.bench_function("2x", |b| {
        b.iter(|| selection::benchmark_monitor_scale_ratio(black_box(1920), black_box(3840)));
    });

    group.bench_function("zero_logical", |b| {
        b.iter(|| selection::benchmark_monitor_scale_ratio(black_box(0), black_box(1920)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::scaled_gap
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn bench_scaled_gap(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaled_gap");

    group.bench_function("zero_gap", |b| {
        b.iter(|| selection::benchmark_scaled_gap(black_box(0), black_box(1920), black_box(1920)));
    });

    group.bench_function("positive_gap_2x", |b| {
        b.iter(|| {
            selection::benchmark_scaled_gap(black_box(100), black_box(1920), black_box(3840))
        });
    });

    group.bench_function("negative_gap", |b| {
        b.iter(|| {
            selection::benchmark_scaled_gap(black_box(-50), black_box(1920), black_box(1920))
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    // capture
    bench_bgra_to_rgba,
    bench_resolve_screen_index,
    bench_default_screen_index,
    bench_resolve_output_path,
    bench_resolve_output_path_in_dir,
    bench_format_extension,
    bench_format_to_image_format,
    bench_get_screenshots_dir,
    bench_list_screens,
    bench_save_image_optimized,
    bench_ensure_parent_directory,
    bench_capture_screenshot,
    bench_capture_screenshot_stages,
    // record
    bench_frame_duration_hns,
    bench_round_to_even,
    bench_rgba_to_bgra_bottom_up,
    bench_padded_recording_region,
    bench_pack_u32_pair,
    bench_validate_recording_start,
    bench_get_recording_dir,
    bench_get_recording_path,
    bench_get_recording_path_in_dir,
    bench_recording_stages,
    // settings
    bench_format_settings_json,
    bench_parse_settings_json,
    bench_settings_path,
    bench_log_file_path,
    bench_load_settings,
    bench_normalize_default_format,
    bench_normalize_recording_fps,
    bench_extract_json_string,
    bench_extract_json_number,
    bench_extract_json_bool,
    // cli
    bench_settings_capture_format,
    bench_cli_format_flag_present_in,
    bench_cli_path_helpers,
    bench_effective_cli_format,
    bench_should_run_cli_mode,
    bench_format_screen_list_output,
    bench_selection_stderr_message,
    bench_write_error_line,
    bench_child_process_is_running,
    // selection
    bench_build_region_from_drag,
    bench_virtual_desktop_scale_factor,
    bench_logical_to_physical,
    bench_find_screen_index_at_position,
    bench_preferred_screen_index,
    bench_get_cursor_position,
    bench_scaled_physical_dimension,
    bench_build_screen_layout_inputs_physical_sizes,
    bench_build_physical_screen_layouts,
    bench_normalize_physical_screen_layouts,
    bench_resolve_recording_selection,
    bench_ranges_overlap,
    bench_monitor_scale_ratio,
    bench_scaled_gap,
    bench_overlay_frame_update,
    bench_crop_packed_region,
    // tray
    bench_build_default_icon_pixels,
);

criterion_main!(benches);
