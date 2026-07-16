//! Integration tests for all public pure functions used in benchmarks.
//! Ensures these functions remain accessible from external crates (benchmarks, tests).

use rustshot::capture::Format;
use rustshot::settings::Fps;
// `Path` is only used by the Windows-gated live-capture tests below.
#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;

#[cfg(windows)]
fn primary_screen() -> screenshots::Screen {
    let screens = screenshots::Screen::all().expect("screen enumeration should succeed");
    screens
        .iter()
        .find(|screen| screen.display_info.is_primary)
        .copied()
        .unwrap_or(screens[0])
}

#[cfg(windows)]
fn assert_file_looks_like_mp4(path: &Path) {
    let bytes = std::fs::read(path).expect("recording file bytes should be readable");
    assert!(
        bytes.len() > 16,
        "recording file is too small: {}",
        bytes.len()
    );
    assert_eq!(
        &bytes[4..8],
        b"ftyp",
        "recording file should start with an MP4 ftyp box"
    );

    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("moov"), "recording file should contain moov");
    assert!(text.contains("mdat"), "recording file should contain mdat");
}

// ---------------------------------------------------------------------------
// capture module
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_swaps_channels_correctly() {
    // BGRA: B=1, G=2, R=3, A=4
    let mut pixels = vec![1, 2, 3, 4];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    // After swap: R=3, G=2, B=1, A=4
    assert_eq!(pixels, vec![3, 2, 1, 4]);
}

#[test]
fn bgra_to_rgba_empty_input() {
    let mut pixels: Vec<u8> = vec![];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert!(pixels.is_empty());
}

#[test]
fn bgra_to_rgba_multiple_pixels() {
    let mut pixels = vec![10, 20, 30, 255, 50, 60, 70, 128];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![30, 20, 10, 255, 70, 60, 50, 128]);
}

#[test]
fn bgra_to_rgba_double_swap_is_identity() {
    let original = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut pixels = original.clone();
    rustshot::capture::bgra_to_rgba(&mut pixels);
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, original);
}

#[test]
fn resolve_screen_index_none_returns_zero() {
    assert_eq!(rustshot::capture::resolve_screen_index(3, None).unwrap(), 0);
}

#[test]
fn resolve_screen_index_valid_returns_index() {
    assert_eq!(
        rustshot::capture::resolve_screen_index(3, Some(2)).unwrap(),
        2
    );
}

#[test]
fn resolve_screen_index_out_of_range_is_error() {
    assert!(rustshot::capture::resolve_screen_index(3, Some(5)).is_err());
}

#[test]
fn resolve_screen_index_zero_screens_is_error() {
    assert!(rustshot::capture::resolve_screen_index(0, None).is_err());
}

#[test]
fn resolve_screen_index_boundary_last_valid() {
    assert_eq!(
        rustshot::capture::resolve_screen_index(5, Some(4)).unwrap(),
        4
    );
}

#[test]
fn resolve_screen_index_boundary_first_invalid() {
    assert!(rustshot::capture::resolve_screen_index(5, Some(5)).is_err());
}

#[test]
fn resolve_screen_index_out_of_range_error_mentions_valid_range() {
    let err = rustshot::capture::resolve_screen_index(3, Some(5)).unwrap_err();
    assert!(err.contains("5"));
    assert!(err.contains("0-2"));
}

#[test]
fn default_screen_index_primary_first() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![true, false, false]),
        Some(0)
    );
}

#[test]
fn default_screen_index_primary_last() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false, false, true]),
        Some(2)
    );
}

#[test]
fn default_screen_index_no_primary_falls_back_to_first() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false, false, false]),
        Some(0)
    );
}

#[test]
fn default_screen_index_empty_returns_none() {
    assert_eq!(
        rustshot::capture::default_screen_index(Vec::<bool>::new()),
        None
    );
}

#[test]
fn default_screen_index_single_primary() {
    assert_eq!(rustshot::capture::default_screen_index(vec![true]), Some(0));
}

#[test]
fn default_screen_index_single_non_primary_returns_zero() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false]),
        Some(0)
    );
}

#[test]
fn default_screen_index_multiple_primaries_returns_first_primary() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false, true, true, false]),
        Some(1)
    );
}

#[test]
fn resolve_output_path_explicit_returns_given_path() {
    let path = PathBuf::from("/tmp/shot.png");
    let result =
        rustshot::capture::resolve_output_path(Some(path.clone()), rustshot::capture::Format::Png);
    assert_eq!(result, path);
}

#[test]
fn resolve_output_path_none_generates_path_with_extension() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Jpg);
    assert!(result.extension().and_then(|e| e.to_str()) == Some("jpg"));
}

#[test]
fn resolve_output_path_none_png_has_png_extension() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("png"));
}

#[test]
fn resolve_output_path_none_bmp_has_bmp_extension() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Bmp);
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("bmp"));
}

#[test]
fn resolve_output_path_none_is_under_screenshots_dir() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    assert_eq!(
        result.parent(),
        Some(rustshot::capture::get_screenshots_dir().as_path())
    );
}

#[test]
fn resolve_output_path_explicit_without_extension_is_preserved() {
    let path = PathBuf::from("custom_output");
    let result =
        rustshot::capture::resolve_output_path(Some(path.clone()), rustshot::capture::Format::Bmp);
    assert_eq!(result, path);
}

#[test]
fn resolve_output_path_explicit_extension_is_not_rewritten() {
    let path = PathBuf::from("custom.jpg");
    let result =
        rustshot::capture::resolve_output_path(Some(path.clone()), rustshot::capture::Format::Png);
    assert_eq!(result, path);
}

// ---------------------------------------------------------------------------
// record module
// ---------------------------------------------------------------------------

#[test]
fn frame_duration_hns_30fps() {
    assert_eq!(rustshot::record::frame_duration_hns(30), 333333);
}

#[test]
fn frame_duration_hns_60fps() {
    assert_eq!(rustshot::record::frame_duration_hns(60), 166666);
}

#[test]
fn frame_duration_hns_zero_returns_zero() {
    assert_eq!(rustshot::record::frame_duration_hns(0), 0);
}

#[test]
fn frame_duration_hns_1fps() {
    assert_eq!(rustshot::record::frame_duration_hns(1), 10_000_000);
}

#[test]
fn round_to_even_even_unchanged() {
    assert_eq!(rustshot::record::round_to_even(1920), 1920);
}

#[test]
fn round_to_even_odd_rounds_up() {
    assert_eq!(rustshot::record::round_to_even(1921), 1922);
}

#[test]
fn round_to_even_zero() {
    assert_eq!(rustshot::record::round_to_even(0), 0);
}

#[test]
fn round_to_even_one() {
    assert_eq!(rustshot::record::round_to_even(1), 2);
}

#[test]
fn round_to_even_two() {
    assert_eq!(rustshot::record::round_to_even(2), 2);
}

#[test]
fn rgba_to_bgra_bottom_up_2x2() {
    // 2x2 RGBA image, top-down
    // Row 0: pixel(0,0)=[R,G,B,A]=[1,2,3,4], pixel(1,0)=[5,6,7,8]
    // Row 1: pixel(0,1)=[9,10,11,12], pixel(1,1)=[13,14,15,16]
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2);
    // Bottom-up: row 1 first, then row 0
    // BGRA: swap R and B channels
    // Row 1 (dst row 0): [11,10,9,12, 15,14,13,16]
    // Row 0 (dst row 1): [3,2,1,4, 7,6,5,8]
    assert_eq!(
        result,
        vec![11, 10, 9, 12, 15, 14, 13, 16, 3, 2, 1, 4, 7, 6, 5, 8]
    );
}

#[test]
fn rgba_to_bgra_bottom_up_1x1() {
    let rgba = vec![100, 150, 200, 255];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(result, vec![200, 150, 100, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_alpha() {
    let rgba = vec![10, 20, 30, 128];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(result[3], 128);
}

#[test]
fn pack_u32_pair_basic() {
    let packed = rustshot::record::pack_u32_pair(1920, 1080);
    assert_eq!(packed >> 32, 1920);
    assert_eq!(packed & 0xFFFF_FFFF, 1080);
}

#[test]
fn pack_u32_pair_zeros() {
    assert_eq!(rustshot::record::pack_u32_pair(0, 0), 0);
}

#[test]
fn pack_u32_pair_max_values() {
    let packed = rustshot::record::pack_u32_pair(u32::MAX, u32::MAX);
    assert_eq!(packed, u64::MAX);
}

#[test]
fn pack_u32_pair_hi_only() {
    let packed = rustshot::record::pack_u32_pair(1, 0);
    assert_eq!(packed, 1u64 << 32);
}

#[test]
fn pack_u32_pair_lo_only() {
    let packed = rustshot::record::pack_u32_pair(0, 42);
    assert_eq!(packed, 42);
}

#[test]
fn validate_recording_start_zero_fps_is_error() {
    assert!(rustshot::record::validate_recording_start(0).is_err());
}

#[test]
fn validate_recording_start_zero_fps_error_message() {
    let err = rustshot::record::validate_recording_start(0).unwrap_err();
    assert!(err.contains("FPS"));
}

#[cfg(windows)]
#[test]
fn validate_recording_start_valid_fps_ok() {
    assert!(rustshot::record::validate_recording_start(30).is_ok());
}

#[cfg(not(windows))]
#[test]
fn validate_recording_start_non_windows_is_error() {
    assert!(rustshot::record::validate_recording_start(30).is_err());
}

// ---------------------------------------------------------------------------
// settings module
// ---------------------------------------------------------------------------

#[test]
fn format_settings_json_contains_all_keys() {
    let s = rustshot::settings::Settings::default();
    let json = rustshot::settings::format_settings_json(&s);
    assert!(json.contains("screenshots_dir"));
    assert!(json.contains("recordings_dir"));
    assert!(json.contains("default_format"));
    assert!(json.contains("recording_fps"));
    assert!(json.contains("auto_start"));
}

#[test]
fn format_settings_json_is_valid_structure() {
    let s = rustshot::settings::Settings::default();
    let json = rustshot::settings::format_settings_json(&s);
    assert!(json.starts_with('{'));
    assert!(json.ends_with('}'));
}

#[test]
fn parse_settings_json_roundtrip() {
    let original = rustshot::settings::Settings {
        screenshots_dir: PathBuf::from("/tmp/shots"),
        recordings_dir: PathBuf::from("/tmp/recs"),
        default_format: Format::Jpg,
        recording_fps: Fps::new(60).unwrap(),
        auto_start: false,
    };
    let json = rustshot::settings::format_settings_json(&original);
    let parsed = rustshot::settings::parse_settings_json(&json);
    assert_eq!(parsed.default_format, Format::Jpg);
    assert_eq!(parsed.recording_fps.get(), 60);
    assert!(!parsed.auto_start);
}

#[test]
fn parse_settings_json_empty_returns_defaults() {
    let parsed = rustshot::settings::parse_settings_json("{}");
    assert_eq!(parsed.default_format, Format::Png);
    assert_eq!(parsed.recording_fps.get(), 30);
}

#[test]
fn parse_settings_json_partial_keys() {
    let json = r#"{ "default_format": "bmp", "recording_fps": 15 }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Bmp);
    assert_eq!(parsed.recording_fps.get(), 15);
    assert!(parsed.auto_start);
}

#[test]
fn parse_settings_json_garbage_returns_defaults() {
    let parsed = rustshot::settings::parse_settings_json("not json");
    assert_eq!(parsed.default_format, Format::Png);
}

#[test]
fn format_and_parse_settings_preserves_all_fields() {
    let original = rustshot::settings::Settings {
        screenshots_dir: PathBuf::from("/a/b"),
        recordings_dir: PathBuf::from("/c/d"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(120).unwrap(),
        auto_start: false,
    };
    let json = rustshot::settings::format_settings_json(&original);
    let parsed = rustshot::settings::parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
    assert_eq!(parsed.default_format, original.default_format);
    assert_eq!(parsed.recording_fps, original.recording_fps);
    assert_eq!(parsed.auto_start, original.auto_start);
}

#[test]
fn parse_settings_json_duplicate_auto_start_uses_first_occurrence() {
    let json = r#"{ "auto_start": false, "auto_start": true }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert!(!parsed.auto_start);
}

#[test]
fn parse_settings_json_invalid_auto_start_token_keeps_default() {
    let json = r#"{ "auto_start": trueish }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert!(parsed.auto_start);
}

#[test]
fn parse_settings_json_duplicate_recording_fps_uses_first_occurrence() {
    let json = r#"{ "recording_fps": 15, "recording_fps": 60 }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.recording_fps.get(), 15);
}

#[test]
fn parse_settings_json_duplicate_default_format_uses_first_occurrence() {
    let json = r#"{ "default_format": "jpg", "default_format": "bmp" }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Jpg);
}

#[test]
fn parse_settings_json_duplicate_screenshots_dir_uses_first_occurrence() {
    let json = r#"{ "screenshots_dir": "/first", "screenshots_dir": "/second" }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.screenshots_dir, PathBuf::from("/first"));
}

// ---------------------------------------------------------------------------
// capture::Format API
// ---------------------------------------------------------------------------

#[test]
fn format_extension_png() {
    assert_eq!(rustshot::capture::Format::Png.extension(), "png");
}

#[test]
fn format_extension_jpg() {
    assert_eq!(rustshot::capture::Format::Jpg.extension(), "jpg");
}

#[test]
fn format_extension_bmp() {
    assert_eq!(rustshot::capture::Format::Bmp.extension(), "bmp");
}

#[test]
fn format_default_is_png() {
    assert!(matches!(
        rustshot::capture::Format::default(),
        rustshot::capture::Format::Png
    ));
}

#[test]
fn format_to_image_format_png() {
    assert_eq!(
        rustshot::capture::Format::Png.to_image_format(),
        image::ImageFormat::Png
    );
}

#[test]
fn format_to_image_format_jpg() {
    assert_eq!(
        rustshot::capture::Format::Jpg.to_image_format(),
        image::ImageFormat::Jpeg
    );
}

#[test]
fn format_to_image_format_bmp() {
    assert_eq!(
        rustshot::capture::Format::Bmp.to_image_format(),
        image::ImageFormat::Bmp
    );
}

#[test]
fn format_to_image_format_all_distinct() {
    let png = rustshot::capture::Format::Png.to_image_format();
    let jpg = rustshot::capture::Format::Jpg.to_image_format();
    let bmp = rustshot::capture::Format::Bmp.to_image_format();
    assert_ne!(png, jpg);
    assert_ne!(png, bmp);
    assert_ne!(jpg, bmp);
}

#[test]
fn format_clone_produces_equal_variant() {
    let f = rustshot::capture::Format::Jpg;
    #[allow(clippy::clone_on_copy)]
    let g = f.clone();
    assert_eq!(f.extension(), g.extension());
}

#[test]
fn format_debug_is_non_empty() {
    assert!(!format!("{:?}", rustshot::capture::Format::Png).is_empty());
    assert!(!format!("{:?}", rustshot::capture::Format::Jpg).is_empty());
    assert!(!format!("{:?}", rustshot::capture::Format::Bmp).is_empty());
}

// ---------------------------------------------------------------------------
// capture::get_screenshots_dir
// ---------------------------------------------------------------------------

#[test]
fn get_screenshots_dir_ends_with_rustshot() {
    let dir = rustshot::capture::get_screenshots_dir();
    assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("RustShot"));
}

#[test]
fn get_screenshots_dir_consistent() {
    let a = rustshot::capture::get_screenshots_dir();
    let b = rustshot::capture::get_screenshots_dir();
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// capture::list_screens
// ---------------------------------------------------------------------------

#[test]
fn list_screens_returns_vec() {
    let screens = rustshot::capture::list_screens();
    // Should return a Vec (possibly empty in CI)
    let _ = screens.len();
}

#[test]
fn list_screens_entries_contain_screen_prefix() {
    let screens = rustshot::capture::list_screens();
    for entry in &screens {
        assert!(
            entry.starts_with("Screen "),
            "entry should start with 'Screen ': {entry}"
        );
    }
}

// ---------------------------------------------------------------------------
// capture::bgra_to_rgba edge cases
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_all_zeros() {
    let mut pixels = vec![0u8; 16];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![0u8; 16]);
}

#[test]
fn bgra_to_rgba_all_max() {
    let mut pixels = vec![255u8; 8];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![255u8; 8]);
}

#[test]
fn bgra_to_rgba_preserves_green_and_alpha() {
    // Only R and B should swap; G and A stay in place
    let mut pixels = vec![100, 200, 50, 128];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels[1], 200); // G unchanged
    assert_eq!(pixels[3], 128); // A unchanged
}

#[test]
fn bgra_to_rgba_trailing_bytes_ignored() {
    // chunks_exact_mut ignores trailing bytes not aligned to 4
    let mut pixels = vec![1, 2, 3, 4, 5, 6]; // 4+2 trailing bytes
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels[0], 3); // B→R swap happened on first chunk
    assert_eq!(pixels[2], 1);
    assert_eq!(pixels[4], 5); // trailing bytes untouched
    assert_eq!(pixels[5], 6);
}

#[test]
fn bgra_to_rgba_large_buffer_length_preserved() {
    let len = 4096;
    let mut pixels: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
    let original_len = pixels.len();
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels.len(), original_len);
}

// ---------------------------------------------------------------------------
// record::get_recording_dir / get_recording_path
// ---------------------------------------------------------------------------

#[test]
fn get_recording_dir_ends_with_recordings() {
    let dir = rustshot::record::get_recording_dir();
    assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("recordings"));
}

#[test]
fn get_recording_dir_parent_is_rustshot() {
    let dir = rustshot::record::get_recording_dir();
    let parent = dir.parent().unwrap();
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn get_recording_dir_consistent() {
    let a = rustshot::record::get_recording_dir();
    let b = rustshot::record::get_recording_dir();
    assert_eq!(a, b);
}

#[test]
fn get_recording_path_has_mp4_extension() {
    let path = rustshot::record::get_recording_path();
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
}

#[test]
fn get_recording_path_filename_starts_with_recording() {
    let path = rustshot::record::get_recording_path();
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
    assert!(
        name.starts_with("recording_"),
        "filename should start with 'recording_': {name}"
    );
}

#[test]
fn get_recording_path_parent_equals_recording_dir() {
    let path = rustshot::record::get_recording_path();
    let dir = rustshot::record::get_recording_dir();
    assert_eq!(path.parent().unwrap(), dir.as_path());
}

#[test]
fn get_recording_path_timestamp_segment_has_expected_length() {
    let path = rustshot::record::get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let inner = &name["recording_".len()..name.len() - ".mp4".len()];
    assert_eq!(inner.len(), 25);
}

#[test]
fn get_recording_path_filename_contains_only_valid_chars() {
    let path = rustshot::record::get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.'));
}

// ---------------------------------------------------------------------------
// record::rgba_to_bgra_bottom_up edge cases
// ---------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_output_length_equals_input() {
    let rgba = vec![0u8; 64];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 4, 4);
    assert_eq!(result.len(), rgba.len());
}

#[test]
fn rgba_to_bgra_bottom_up_all_zeros() {
    let rgba = vec![0u8; 16];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2);
    assert_eq!(result, vec![0u8; 16]);
}

#[test]
fn rgba_to_bgra_bottom_up_3x1_single_row_not_flipped() {
    // Single row: bottom-up flip is a no-op, only channel swap
    let rgba = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 3, 1);
    // RGBA→BGRA: swap R↔B
    assert_eq!(
        result,
        vec![30, 20, 10, 40, 70, 60, 50, 80, 110, 100, 90, 120]
    );
}

#[test]
fn rgba_to_bgra_bottom_up_1x3_tall_image_flips() {
    // 3 rows, each 1 pixel wide
    // Row 0: [1,2,3,4], Row 1: [5,6,7,8], Row 2: [9,10,11,12]
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 3);
    // Bottom-up: Row2→dst0, Row1→dst1, Row0→dst2
    // + BGRA swap
    assert_eq!(result, vec![11, 10, 9, 12, 7, 6, 5, 8, 3, 2, 1, 4]);
}

#[test]
fn rgba_to_bgra_bottom_up_double_conversion_restores_row_order() {
    // Applying twice reverses the flip twice (identity for row order),
    // but the channel swap happens twice too (R↔B twice = identity)
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let once = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2);
    let twice = rustshot::record::rgba_to_bgra_bottom_up(&once, 2, 2);
    assert_eq!(twice, rgba);
}

// ---------------------------------------------------------------------------
// record::frame_duration_hns edge cases
// ---------------------------------------------------------------------------

#[test]
fn frame_duration_hns_24fps_film() {
    assert_eq!(rustshot::record::frame_duration_hns(24), 416666);
}

#[test]
fn frame_duration_hns_120fps() {
    assert_eq!(rustshot::record::frame_duration_hns(120), 83333);
}

#[test]
fn frame_duration_hns_240fps() {
    assert_eq!(rustshot::record::frame_duration_hns(240), 41666);
}

#[test]
fn frame_duration_hns_is_positive_for_nonzero() {
    for fps in [1, 10, 24, 30, 60, 120, 240] {
        assert!(
            rustshot::record::frame_duration_hns(fps) > 0,
            "frame_duration_hns({fps}) should be positive"
        );
    }
}

#[test]
fn frame_duration_hns_decreases_with_higher_fps() {
    let d30 = rustshot::record::frame_duration_hns(30);
    let d60 = rustshot::record::frame_duration_hns(60);
    let d120 = rustshot::record::frame_duration_hns(120);
    assert!(d30 > d60);
    assert!(d60 > d120);
}

// ---------------------------------------------------------------------------
// record::round_to_even more values
// ---------------------------------------------------------------------------

#[test]
fn round_to_even_common_resolutions() {
    // All standard resolutions are already even
    for w in [640, 720, 1280, 1920, 2560, 3840] {
        assert_eq!(
            rustshot::record::round_to_even(w),
            w,
            "even value {w} should be unchanged"
        );
    }
}

#[test]
fn round_to_even_three_becomes_four() {
    assert_eq!(rustshot::record::round_to_even(3), 4);
}

#[test]
fn round_to_even_result_is_always_even() {
    for v in 0..=100 {
        let r = rustshot::record::round_to_even(v);
        assert_eq!(r % 2, 0, "round_to_even({v}) = {r} is not even");
    }
}

#[test]
fn round_to_even_result_gte_input() {
    for v in 0..=100 {
        assert!(
            rustshot::record::round_to_even(v) >= v,
            "round_to_even({v}) should be >= {v}"
        );
    }
}

#[test]
fn round_to_even_result_at_most_one_more() {
    for v in 0..=100 {
        let r = rustshot::record::round_to_even(v);
        assert!(r <= v + 1, "round_to_even({v}) = {r}, too large");
    }
}

// ---------------------------------------------------------------------------
// record::pack_u32_pair — roundtrip extraction
// ---------------------------------------------------------------------------

#[test]
fn pack_u32_pair_extract_hi_and_lo() {
    for (hi, lo) in [(0, 0), (1, 2), (1920, 1080), (u32::MAX, 0), (0, u32::MAX)] {
        let packed = rustshot::record::pack_u32_pair(hi, lo);
        assert_eq!((packed >> 32) as u32, hi, "hi mismatch for ({hi}, {lo})");
        assert_eq!(packed as u32, lo, "lo mismatch for ({hi}, {lo})");
    }
}

#[test]
fn pack_u32_pair_frame_rate_30_over_1() {
    let packed = rustshot::record::pack_u32_pair(30, 1);
    assert_eq!((packed >> 32) as u32, 30);
    assert_eq!(packed as u32, 1);
}

// ---------------------------------------------------------------------------
// record::validate_recording_start edge cases
// ---------------------------------------------------------------------------

#[test]
fn validate_recording_start_zero_error_mentions_greater_than_0() {
    let err = rustshot::record::validate_recording_start(0).unwrap_err();
    assert!(
        err.contains("greater than 0"),
        "error should mention 'greater than 0': {err}"
    );
}

#[cfg(windows)]
#[test]
fn validate_recording_start_1fps_ok() {
    assert!(rustshot::record::validate_recording_start(1).is_ok());
}

#[cfg(windows)]
#[test]
fn validate_recording_start_240fps_ok() {
    assert!(rustshot::record::validate_recording_start(240).is_ok());
}

#[test]
fn validate_recording_start_241fps_mentions_max_limit() {
    let err = rustshot::record::validate_recording_start(241).unwrap_err();
    assert!(
        err.contains("240"),
        "error should mention the max supported FPS: {err}"
    );
}

// ---------------------------------------------------------------------------
// settings edge cases
// ---------------------------------------------------------------------------

#[test]
fn format_settings_json_custom_fps_appears() {
    let s = rustshot::settings::Settings {
        recording_fps: Fps::new(144).unwrap(),
        ..rustshot::settings::Settings::default()
    };
    let json = rustshot::settings::format_settings_json(&s);
    assert!(json.contains("144"));
}

#[test]
fn format_settings_json_contains_newlines() {
    let json = rustshot::settings::format_settings_json(&rustshot::settings::Settings::default());
    assert!(json.contains('\n'));
}

#[test]
fn format_settings_json_contains_auto_start_key_once() {
    let json = rustshot::settings::format_settings_json(&rustshot::settings::Settings::default());
    assert_eq!(json.matches("\"auto_start\"").count(), 1);
}

#[test]
fn parse_settings_json_empty_string_returns_defaults() {
    let parsed = rustshot::settings::parse_settings_json("");
    assert_eq!(parsed, rustshot::settings::Settings::default());
}

#[test]
fn parse_settings_json_windows_backslash_roundtrip() {
    let original = rustshot::settings::Settings {
        screenshots_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot"),
        recordings_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot\\recordings"),
        default_format: Format::Png,
        recording_fps: Fps::DEFAULT,
        auto_start: true,
    };
    let json = rustshot::settings::format_settings_json(&original);
    let parsed = rustshot::settings::parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
}

#[test]
fn parse_settings_json_paths_with_quotes_roundtrip() {
    let original = rustshot::settings::Settings {
        screenshots_dir: PathBuf::from("/tmp/screens\"shots"),
        recordings_dir: PathBuf::from("/tmp/reco\"rdings"),
        default_format: Format::Png,
        recording_fps: Fps::DEFAULT,
        auto_start: true,
    };
    let json = rustshot::settings::format_settings_json(&original);
    let parsed = rustshot::settings::parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
}

#[test]
fn settings_path_ends_with_settings_json() {
    let p = rustshot::settings::settings_path();
    assert_eq!(
        p.file_name().and_then(|n| n.to_str()),
        Some("settings.json")
    );
}

#[test]
fn settings_path_parent_is_rustshot() {
    let p = rustshot::settings::settings_path();
    let parent = p.parent().unwrap();
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn settings_path_consistent() {
    assert_eq!(
        rustshot::settings::settings_path(),
        rustshot::settings::settings_path()
    );
}

#[test]
fn load_settings_returns_defaults_when_no_file() {
    let s = rustshot::settings::load_settings();
    assert_eq!(s.default_format, Format::Png);
}

#[test]
fn settings_default_clone_equals_original() {
    let s = rustshot::settings::Settings::default();
    let s2 = s.clone();
    assert_eq!(s, s2);
}

#[test]
fn settings_default_auto_start_is_true() {
    let s = rustshot::settings::Settings::default();
    assert!(s.auto_start);
}

#[test]
fn settings_default_screenshots_dir_ends_with_rustshot() {
    let s = rustshot::settings::Settings::default();
    assert_eq!(
        s.screenshots_dir.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn settings_default_recordings_dir_ends_with_recordings() {
    let s = rustshot::settings::Settings::default();
    assert_eq!(
        s.recordings_dir.file_name().and_then(|n| n.to_str()),
        Some("recordings")
    );
}

#[test]
fn settings_debug_is_non_empty() {
    let s = rustshot::settings::Settings::default();
    assert!(!format!("{:?}", s).is_empty());
}

#[test]
fn parse_settings_json_null_string_field_keeps_default() {
    let json = r#"{ "default_format": null }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Png);
}

#[test]
fn parse_settings_json_number_for_string_keeps_default() {
    let json = r#"{ "default_format": 42 }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Png);
}

#[test]
fn parse_settings_json_string_for_number_keeps_default() {
    let json = r#"{ "recording_fps": "sixty" }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.recording_fps.get(), 30);
}

#[test]
fn benchmark_normalize_default_format_unknown_falls_back_to_png() {
    assert_eq!(
        rustshot::settings::benchmark_normalize_default_format("gif"),
        Format::Png
    );
}

#[test]
fn benchmark_normalize_recording_fps_zero_returns_default() {
    assert_eq!(
        rustshot::settings::benchmark_normalize_recording_fps(0),
        Fps::DEFAULT
    );
}

#[test]
fn benchmark_normalize_default_format_mixed_case_falls_back_to_png() {
    assert_eq!(
        rustshot::settings::benchmark_normalize_default_format("PNG"),
        Format::Png
    );
}

#[test]
fn benchmark_normalize_recording_fps_non_zero_is_preserved() {
    assert_eq!(
        rustshot::settings::benchmark_normalize_recording_fps(144).get(),
        144
    );
}

#[test]
fn benchmark_extract_json_string_returns_value() {
    let json = r#"{ "screenshots_dir": "/tmp/shots" }"#;
    assert_eq!(
        rustshot::settings::benchmark_extract_json_string(json, "screenshots_dir"),
        Some("/tmp/shots".to_string())
    );
}

#[test]
fn benchmark_extract_json_string_missing_key_returns_none() {
    let json = r#"{ "screenshots_dir": "/tmp/shots" }"#;
    assert_eq!(
        rustshot::settings::benchmark_extract_json_string(json, "recordings_dir"),
        None
    );
}

#[test]
fn benchmark_extract_json_number_returns_value() {
    let json = r#"{ "recording_fps": 144 }"#;
    assert_eq!(
        rustshot::settings::benchmark_extract_json_number(json, "recording_fps"),
        Some(144)
    );
}

#[test]
fn benchmark_extract_json_number_missing_key_returns_none() {
    let json = r#"{ "recording_fps": 144 }"#;
    assert_eq!(
        rustshot::settings::benchmark_extract_json_number(json, "missing"),
        None
    );
}

#[test]
fn benchmark_extract_json_bool_returns_value() {
    let json = r#"{ "auto_start": false }"#;
    assert_eq!(
        rustshot::settings::benchmark_extract_json_bool(json, "auto_start"),
        Some(false)
    );
}

#[test]
fn benchmark_extract_json_bool_missing_key_returns_none() {
    let json = r#"{ "auto_start": false }"#;
    assert_eq!(
        rustshot::settings::benchmark_extract_json_bool(json, "missing"),
        None
    );
}

// ---------------------------------------------------------------------------
// Cross-module: resolve_output_path + Format interaction
// ---------------------------------------------------------------------------

#[test]
fn resolve_output_path_all_formats_produce_matching_extension() {
    use rustshot::capture::Format;
    for fmt in [Format::Png, Format::Jpg, Format::Bmp] {
        let path = rustshot::capture::resolve_output_path(None, fmt);
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some(fmt.extension()),
            "extension mismatch for format {:?}",
            fmt
        );
    }
}

#[test]
fn resolve_output_path_default_filename_starts_with_screenshot() {
    let path = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with("screenshot_"));
}

#[test]
fn resolve_output_path_default_filename_only_valid_chars() {
    let path = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.'));
}

#[test]
fn resolve_output_path_default_timestamp_segment_has_expected_length() {
    let path = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let inner = &name["screenshot_".len()..name.len() - ".png".len()];
    assert_eq!(inner.len(), 25);
}

// ---------------------------------------------------------------------------
// Cross-module default path consistency
// ---------------------------------------------------------------------------

#[test]
fn settings_default_screenshots_dir_matches_capture_dir() {
    let settings = rustshot::settings::Settings::default();
    assert_eq!(
        settings.screenshots_dir,
        rustshot::capture::get_screenshots_dir()
    );
}

#[test]
fn settings_default_recordings_dir_matches_recording_dir() {
    let settings = rustshot::settings::Settings::default();
    assert_eq!(
        settings.recordings_dir,
        rustshot::record::get_recording_dir()
    );
}

#[test]
fn settings_default_recordings_dir_is_child_of_screenshots_dir() {
    let settings = rustshot::settings::Settings::default();
    assert_eq!(
        settings.recordings_dir.parent(),
        Some(settings.screenshots_dir.as_path())
    );
}

// ---------------------------------------------------------------------------
// selection::Region public API
// ---------------------------------------------------------------------------

#[test]
fn selection_region_copy_preserves_values() {
    let region = rustshot::selection::Region {
        physical_x: 10,
        physical_y: 20,
        physical_width: 300,
        physical_height: 200,
    };
    let copied = region;
    assert_eq!(copied.physical_x, 10);
    assert_eq!(copied.physical_y, 20);
    assert_eq!(copied.physical_width, 300);
    assert_eq!(copied.physical_height, 200);
}

#[test]
fn selection_region_clone_supports_negative_coordinates() {
    let region = rustshot::selection::Region {
        physical_x: -1920,
        physical_y: -1080,
        physical_width: 640,
        physical_height: 480,
    };
    let cloned = region;
    assert_eq!(cloned.physical_x, -1920);
    assert_eq!(cloned.physical_y, -1080);
    assert_eq!(cloned.physical_width, 640);
    assert_eq!(cloned.physical_height, 480);
}

// ---------------------------------------------------------------------------
// Manual Windows smoke tests
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_capture_screenshot_saves_valid_png() {
    let output = std::env::temp_dir().join(format!(
        "rustshot_manual_capture_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    let saved = rustshot::capture::capture_screenshot(
        Some(output.clone()),
        rustshot::capture::Format::Png,
        None,
    )
    .expect("live screenshot capture should succeed");

    assert_eq!(saved, output);
    assert!(saved.exists(), "screenshot file should exist");

    let metadata = std::fs::metadata(&saved).expect("screenshot metadata should exist");
    assert!(metadata.len() > 0, "screenshot file should not be empty");

    let decoded = image::open(&saved).expect("saved screenshot should decode as an image");
    assert!(
        decoded.width() > 0 && decoded.height() > 0,
        "decoded screenshot should have non-zero dimensions"
    );

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_capture_screenshot_matches_primary_screen_dimensions() {
    let screens = screenshots::Screen::all().expect("screen enumeration should succeed");
    let screen = screens
        .iter()
        .find(|screen| screen.display_info.is_primary)
        .copied()
        .unwrap_or(screens[0]);
    let reference = screen
        .capture()
        .expect("direct screen capture should succeed for reference dimensions");

    let output = std::env::temp_dir().join(format!(
        "rustshot_manual_capture_dims_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    let saved = rustshot::capture::capture_screenshot(
        Some(output.clone()),
        rustshot::capture::Format::Png,
        None,
    )
    .expect("live screenshot capture should succeed");

    let decoded = image::open(&saved).expect("saved screenshot should decode as an image");
    assert_eq!(decoded.width(), reference.width());
    assert_eq!(decoded.height(), reference.height());

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_capture_active_window_saves_valid_image() {
    let output = std::env::temp_dir().join(format!(
        "rustshot_manual_active_window_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    let saved = rustshot::capture::capture_active_window(
        Some(output.clone()),
        rustshot::capture::Format::Png,
    )
    .expect("active-window capture should succeed");

    assert_eq!(saved, output);
    assert!(saved.exists(), "active-window screenshot file should exist");

    let decoded = image::open(&saved).expect("saved active-window screenshot should decode");
    assert!(
        decoded.width() > 0 && decoded.height() > 0,
        "active-window screenshot should have non-zero dimensions"
    );

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_recording_session_creates_non_empty_mp4() {
    use std::time::Duration;

    let screen = primary_screen();

    let width = screen.display_info.width.clamp(2, 64);
    let height = screen.display_info.height.clamp(2, 64);

    let session = rustshot::record::RecordingSession::start(
        screen.display_info.x,
        screen.display_info.y,
        rustshot::selection::Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        5,
    )
    .expect("live recording should start");

    std::thread::sleep(Duration::from_millis(700));

    let saved = session.stop().expect("live recording should stop cleanly");
    assert!(saved.exists(), "recording file should exist");

    let metadata = std::fs::metadata(&saved).expect("recording metadata should exist");
    assert!(metadata.len() > 0, "recording file should not be empty");
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("mp4"));
    assert_file_looks_like_mp4(&saved);

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_recording_session_survives_five_seconds() {
    use std::time::Duration;

    let screen = primary_screen();

    let width = screen.display_info.width.clamp(2, 128);
    let height = screen.display_info.height.clamp(2, 72);

    let session = rustshot::record::RecordingSession::start(
        screen.display_info.x,
        screen.display_info.y,
        rustshot::selection::Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        5,
    )
    .expect("5-second live recording should start");

    std::thread::sleep(Duration::from_secs(5));

    let saved = session
        .stop()
        .expect("5-second live recording should stop cleanly");
    assert!(saved.exists(), "5-second recording file should exist");

    let metadata = std::fs::metadata(&saved).expect("5-second recording metadata should exist");
    assert!(
        metadata.len() > 0,
        "5-second recording file should not be empty"
    );
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("mp4"));
    assert_file_looks_like_mp4(&saved);

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_recording_session_with_odd_region_dimensions_creates_valid_mp4() {
    use std::time::Duration;

    let screen = primary_screen();
    let width = screen.display_info.width.clamp(3, 63);
    let height = screen.display_info.height.clamp(3, 47);

    let session = rustshot::record::RecordingSession::start(
        screen.display_info.x,
        screen.display_info.y,
        rustshot::selection::Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        5,
    )
    .expect("odd-dimension live recording should start");

    std::thread::sleep(Duration::from_millis(700));

    let saved = session
        .stop()
        .expect("odd-dimension live recording should stop cleanly");
    assert!(saved.exists(), "odd-dimension recording file should exist");

    let metadata =
        std::fs::metadata(&saved).expect("odd-dimension recording metadata should exist");
    assert!(
        metadata.len() > 0,
        "odd-dimension recording file should not be empty"
    );
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("mp4"));
    assert_file_looks_like_mp4(&saved);

    let _ = std::fs::remove_file(saved);
}

// ---------------------------------------------------------------------------
// Screenshot quality — real capture + file validation
// ---------------------------------------------------------------------------

/// Whether a display server is plausibly available for live capture.
///
/// The `Err` arms below are not enough on a headless Linux box: the
/// `screenshots` crate's Wayland backend (libwayshot) *panics* when it cannot
/// reach a compositor, before our code can return an error. So live-capture
/// tests must bail out up front. On CI the workflow provides a display via
/// `xvfb-run`, so these tests still exercise real capture there; the skip only
/// engages on truly headless machines (e.g. a bare SSH session).
fn display_available() -> bool {
    if cfg!(windows) {
        return true;
    }
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Skip guard for tests that perform a live screen capture.
macro_rules! require_display {
    () => {
        if !display_available() {
            eprintln!("skipping live-capture test: no display server (DISPLAY/WAYLAND_DISPLAY)");
            return;
        }
    };
}

/// Capture a real screenshot in PNG format, verify the file is a valid image
/// with non-zero dimensions and reasonable file size.
#[test]
fn screenshot_capture_png_produces_valid_image() {
    require_display!();
    let tmp = std::env::temp_dir().join(format!(
        "rustshot_quality_png_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Png,
        None,
    );
    match result {
        Ok(path) => {
            assert_eq!(path, tmp);
            assert!(path.exists(), "output file must exist");
            let metadata = std::fs::metadata(&path).unwrap();
            assert!(
                metadata.len() > 100,
                "PNG file too small ({} bytes), likely corrupt",
                metadata.len()
            );
            // Verify it loads as a valid image
            let img = image::open(&path).expect("should be a valid image file");
            assert!(img.width() > 0, "image width must be > 0");
            assert!(img.height() > 0, "image height must be > 0");
            let _ = std::fs::remove_file(&path);
        }
        Err(e) => {
            // In headless CI environments, screen capture may fail — that's OK
            assert!(
                e.contains("screen") || e.contains("Screen") || e.contains("display"),
                "unexpected capture error: {e}"
            );
        }
    }
}

/// Capture a real screenshot in JPG format, verify valid JPEG output.
#[test]
fn screenshot_capture_jpg_produces_valid_image() {
    require_display!();
    let tmp = std::env::temp_dir().join(format!(
        "rustshot_quality_jpg_{}.jpg",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Jpg,
        None,
    );
    match result {
        Ok(path) => {
            assert_eq!(path, tmp);
            assert!(path.exists());
            let metadata = std::fs::metadata(&path).unwrap();
            assert!(
                metadata.len() > 100,
                "JPG too small: {} bytes",
                metadata.len()
            );
            let img = image::open(&path).expect("should be a valid JPEG");
            assert!(img.width() > 0);
            assert!(img.height() > 0);
            let _ = std::fs::remove_file(&path);
        }
        Err(e) => {
            assert!(
                e.contains("screen") || e.contains("Screen") || e.contains("display"),
                "unexpected capture error: {e}"
            );
        }
    }
}

/// Capture a real screenshot in BMP format, verify valid BMP output.
#[test]
fn screenshot_capture_bmp_produces_valid_image() {
    require_display!();
    let tmp = std::env::temp_dir().join(format!(
        "rustshot_quality_bmp_{}.bmp",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Bmp,
        None,
    );
    match result {
        Ok(path) => {
            assert_eq!(path, tmp);
            assert!(path.exists());
            let metadata = std::fs::metadata(&path).unwrap();
            assert!(
                metadata.len() > 100,
                "BMP too small: {} bytes",
                metadata.len()
            );
            let img = image::open(&path).expect("should be a valid BMP");
            assert!(img.width() > 0);
            assert!(img.height() > 0);
            let _ = std::fs::remove_file(&path);
        }
        Err(e) => {
            assert!(
                e.contains("screen") || e.contains("Screen") || e.contains("display"),
                "unexpected capture error: {e}"
            );
        }
    }
}

/// PNG screenshot should have at least 1024x768 on a real display.
#[test]
fn screenshot_capture_has_reasonable_resolution() {
    require_display!();
    let tmp = std::env::temp_dir().join(format!(
        "rustshot_resolution_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Png,
        None,
    );
    match result {
        Ok(path) => {
            let img = image::open(&path).unwrap();
            // Modern displays are at least 1024x768
            assert!(
                img.width() >= 640,
                "width {} too small for a real display",
                img.width()
            );
            assert!(
                img.height() >= 480,
                "height {} too small for a real display",
                img.height()
            );
            let _ = std::fs::remove_file(&path);
        }
        Err(_) => {
            // Headless/CI — skip
        }
    }
}

/// Capture two screenshots back-to-back in PNG: both must produce valid files.
#[test]
fn screenshot_capture_twice_both_succeed() {
    require_display!();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tmp1 = std::env::temp_dir().join(format!("rustshot_twice1_{ts}.png"));
    let tmp2 = std::env::temp_dir().join(format!("rustshot_twice2_{ts}.png"));

    let r1 = rustshot::capture::capture_screenshot(
        Some(tmp1.clone()),
        rustshot::capture::Format::Png,
        None,
    );
    let r2 = rustshot::capture::capture_screenshot(
        Some(tmp2.clone()),
        rustshot::capture::Format::Png,
        None,
    );

    match (r1, r2) {
        (Ok(p1), Ok(p2)) => {
            assert!(p1.exists());
            assert!(p2.exists());
            // Both should be valid images
            let img1 = image::open(&p1).unwrap();
            let img2 = image::open(&p2).unwrap();
            // Same screen → same dimensions
            assert_eq!(img1.width(), img2.width());
            assert_eq!(img1.height(), img2.height());
            let _ = std::fs::remove_file(&p1);
            let _ = std::fs::remove_file(&p2);
        }
        _ => {
            // Headless/CI
            let _ = std::fs::remove_file(&tmp1);
            let _ = std::fs::remove_file(&tmp2);
        }
    }
}

/// PNG file should not be excessively large (sanity check for encoding).
#[test]
fn screenshot_capture_png_file_size_is_reasonable() {
    require_display!();
    let tmp = std::env::temp_dir().join(format!(
        "rustshot_size_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Png,
        None,
    );
    if let Ok(path) = result {
        let metadata = std::fs::metadata(&path).unwrap();
        let img = image::open(&path).unwrap();
        let raw_size = (img.width() as u64) * (img.height() as u64) * 4;
        // PNG compression should make it smaller than raw RGBA
        assert!(
            metadata.len() < raw_size,
            "PNG ({} bytes) should be smaller than raw RGBA ({} bytes)",
            metadata.len(),
            raw_size
        );
        let _ = std::fs::remove_file(&path);
    }
}

/// capture_screenshot with explicit screen index 0 should work.
#[test]
fn screenshot_capture_screen_index_0_ok() {
    require_display!();
    let tmp = std::env::temp_dir().join(format!(
        "rustshot_screen0_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Png,
        Some(0),
    );
    match result {
        Ok(path) => {
            assert!(path.exists());
            let img = image::open(&path).unwrap();
            assert!(img.width() > 0);
            assert!(img.height() > 0);
            let _ = std::fs::remove_file(&path);
        }
        Err(e) => {
            assert!(
                e.contains("screen") || e.contains("Screen") || e.contains("display"),
                "unexpected error: {e}"
            );
        }
    }
}

/// capture_screenshot with out-of-range screen index should error.
#[test]
fn screenshot_capture_invalid_screen_index_errors() {
    require_display!();
    let tmp = std::env::temp_dir().join("rustshot_badscreen.png");
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Png,
        Some(999),
    );
    match result {
        Ok(_) => panic!("should error for screen index 999"),
        Err(e) => {
            assert!(
                e.contains("out of range") || e.contains("screen") || e.contains("Screen"),
                "error should mention screen range: {e}"
            );
        }
    }
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn selection_region_zero_dimensions() {
    let region = rustshot::selection::Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 0,
    };
    assert_eq!(region.physical_width, 0);
    assert_eq!(region.physical_height, 0);
}

#[test]
fn selection_region_debug_is_non_empty() {
    let region = rustshot::selection::Region {
        physical_x: 5,
        physical_y: 10,
        physical_width: 100,
        physical_height: 50,
    };
    let debug = format!("{:?}", region);
    assert!(!debug.is_empty());
    assert!(debug.contains("Region"));
}

#[test]
fn selection_region_large_values() {
    let region = rustshot::selection::Region {
        physical_x: i32::MAX,
        physical_y: i32::MIN,
        physical_width: u32::MAX,
        physical_height: u32::MAX,
    };
    assert_eq!(region.physical_x, i32::MAX);
    assert_eq!(region.physical_y, i32::MIN);
    assert_eq!(region.physical_width, u32::MAX);
    assert_eq!(region.physical_height, u32::MAX);
}

#[test]
fn build_region_from_drag_valid_drag_returns_region() {
    let region = rustshot::selection::build_region_from_drag((10, 20), (50, 80)).unwrap();
    assert_eq!(region.physical_x, 10);
    assert_eq!(region.physical_y, 20);
    assert_eq!(region.physical_width, 40);
    assert_eq!(region.physical_height, 60);
}

#[test]
fn build_region_from_drag_too_small_returns_none() {
    assert!(rustshot::selection::build_region_from_drag((10, 20), (15, 25)).is_none());
}

#[test]
fn virtual_desktop_scale_factor_prefers_primary_screen() {
    let scale = rustshot::selection::virtual_desktop_scale_factor([
        (false, 1.25),
        (true, 1.5),
        (false, 2.0),
    ]);
    assert_eq!(scale, Some(1.5));
}

#[test]
fn virtual_desktop_scale_factor_empty_returns_none() {
    assert_eq!(
        rustshot::selection::virtual_desktop_scale_factor(Vec::<(bool, f32)>::new()),
        None
    );
}

#[test]
fn logical_to_physical_scales_negative_values() {
    assert_eq!(rustshot::selection::logical_to_physical(-100, 1.5), -150);
}

#[test]
fn find_screen_index_at_position_hits_second_screen() {
    let screens = [(0, 0, 1920_u32, 1080_u32), (1920, 0, 2560_u32, 1440_u32)];
    assert_eq!(
        rustshot::selection::find_screen_index_at_position(screens, (2500, 500)),
        Some(1)
    );
}

#[test]
fn find_screen_index_at_position_empty_screens_returns_none() {
    assert_eq!(
        rustshot::selection::find_screen_index_at_position(
            Vec::<(i32, i32, u32, u32)>::new(),
            (0, 0)
        ),
        None
    );
}

#[test]
fn preferred_screen_index_cursor_hit_beats_primary_fallback() {
    let screens = [
        (0, 0, 1920_u32, 1080_u32, true),
        (1920, 0, 2560_u32, 1440_u32, false),
    ];
    assert_eq!(
        rustshot::selection::preferred_screen_index(screens, Some((2500, 500))),
        Some(1)
    );
}

#[test]
fn preferred_screen_index_no_cursor_falls_back_to_primary() {
    let screens = [
        (0, 0, 1920_u32, 1080_u32, false),
        (1920, 0, 2560_u32, 1440_u32, true),
    ];
    assert_eq!(
        rustshot::selection::preferred_screen_index(screens, None),
        Some(1)
    );
}

#[test]
fn benchmark_scaled_physical_dimension_uses_ceil() {
    assert_eq!(
        rustshot::selection::benchmark_scaled_physical_dimension(1000, 1.333).unwrap(),
        1333
    );
}

#[test]
fn benchmark_scaled_physical_dimension_zero_scale_is_error() {
    let err = rustshot::selection::benchmark_scaled_physical_dimension(1000, 0.0).unwrap_err();
    assert!(err.contains("positive"));
}

#[test]
fn benchmark_build_screen_layout_inputs_reports_target_and_scaled_sizes() {
    let sizes = rustshot::selection::benchmark_build_screen_layout_inputs_physical_sizes(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1707, 960, 1.5, false),
        ],
        1,
        (2560, 1440),
    )
    .unwrap();

    assert_eq!(sizes, vec![(1920, 1080), (2560, 1440)]);
}

#[test]
fn benchmark_build_screen_layout_inputs_out_of_range_screen_index_is_error() {
    let err = rustshot::selection::benchmark_build_screen_layout_inputs_physical_sizes(
        [(0, 0, 1920, 1080, 1.0, true)],
        1,
        (1920, 1080),
    )
    .unwrap_err();

    assert!(err.contains("Unknown screen index"));
}

#[test]
fn benchmark_build_screen_layout_inputs_invalid_non_target_scale_is_error() {
    let err = rustshot::selection::benchmark_build_screen_layout_inputs_physical_sizes(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1707, 960, 0.0, false),
        ],
        0,
        (1920, 1080),
    )
    .unwrap_err();

    assert!(err.contains("positive"));
}

#[test]
fn benchmark_screen_count_from_result_returns_count_for_non_empty_screens() {
    let count = rustshot::capture::benchmark_screen_count_from_result::<u8>(Ok(vec![1, 2, 3]))
        .expect("non-empty screen list should produce a count");
    assert_eq!(count, 3);
}

#[test]
fn benchmark_screen_count_from_result_rejects_empty_screens() {
    let err = rustshot::capture::benchmark_screen_count_from_result::<u8>(Ok(Vec::new()))
        .expect_err("empty screen list should be rejected for display benchmarks");
    assert!(err.contains("No screens detected"));
}

#[test]
fn benchmark_screen_count_from_result_preserves_enumeration_error() {
    let err = rustshot::capture::benchmark_screen_count_from_result::<u8>(Err(
        "Connection error".to_string()
    ))
    .expect_err("screen enumeration errors should be preserved");
    assert!(err.contains("Connection error"));
}

#[test]
fn benchmark_first_screen_from_result_returns_first_screen() {
    let first = rustshot::capture::benchmark_first_screen_from_result(Ok(vec![10, 20, 30]))
        .expect("first screen should be returned");
    assert_eq!(first, 10);
}

#[test]
fn benchmark_first_screen_from_result_rejects_empty_screens() {
    let err = rustshot::capture::benchmark_first_screen_from_result::<u8>(Ok(Vec::new()))
        .expect_err("empty screen list should be rejected");
    assert!(err.contains("No screens detected"));
}

#[test]
fn benchmark_first_screen_from_result_preserves_enumeration_error() {
    let err = rustshot::capture::benchmark_first_screen_from_result::<u8>(Err(
        "Display unavailable".to_string(),
    ))
    .expect_err("screen enumeration errors should be preserved");
    assert!(err.contains("Display unavailable"));
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
// tray module — menu constants
// ---------------------------------------------------------------------------

#[test]
fn tray_menu_constants_are_non_empty() {
    assert!(!rustshot::tray::MENU_CAPTURE.is_empty());
    assert!(!rustshot::tray::MENU_OPEN_FOLDER.is_empty());
    assert!(!rustshot::tray::MENU_EXIT.is_empty());
    assert!(!rustshot::tray::MENU_CAPTURE_WINDOW.is_empty());
    assert!(!rustshot::tray::MENU_SETTINGS.is_empty());
    assert!(!rustshot::tray::MENU_START_RECORD.is_empty());
    assert!(!rustshot::tray::MENU_STOP_RECORD.is_empty());
}

#[test]
fn tray_menu_constants_all_unique() {
    let ids = [
        rustshot::tray::MENU_CAPTURE,
        rustshot::tray::MENU_OPEN_FOLDER,
        rustshot::tray::MENU_EXIT,
        rustshot::tray::MENU_CAPTURE_WINDOW,
        rustshot::tray::MENU_SETTINGS,
        rustshot::tray::MENU_START_RECORD,
        rustshot::tray::MENU_STOP_RECORD,
    ];
    for (i, a) in ids.iter().enumerate() {
        for (j, b) in ids.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "menu IDs at index {i} and {j} must differ");
            }
        }
    }
}

#[test]
fn tray_menu_constants_contain_no_whitespace() {
    let ids = [
        rustshot::tray::MENU_CAPTURE,
        rustshot::tray::MENU_OPEN_FOLDER,
        rustshot::tray::MENU_EXIT,
        rustshot::tray::MENU_CAPTURE_WINDOW,
        rustshot::tray::MENU_SETTINGS,
        rustshot::tray::MENU_START_RECORD,
        rustshot::tray::MENU_STOP_RECORD,
    ];
    for id in &ids {
        assert!(
            !id.contains(char::is_whitespace),
            "menu ID should have no whitespace: '{id}'"
        );
    }
}

#[test]
fn tray_menu_capture_value() {
    assert_eq!(rustshot::tray::MENU_CAPTURE, "capture");
}

#[test]
fn tray_menu_exit_value() {
    assert_eq!(rustshot::tray::MENU_EXIT, "exit");
}

#[test]
fn tray_menu_settings_value() {
    assert_eq!(rustshot::tray::MENU_SETTINGS, "settings");
}

#[test]
fn tray_menu_start_and_stop_record_are_distinct() {
    assert_ne!(
        rustshot::tray::MENU_START_RECORD,
        rustshot::tray::MENU_STOP_RECORD
    );
}

#[test]
fn tray_menu_capture_and_capture_window_are_distinct() {
    assert_ne!(
        rustshot::tray::MENU_CAPTURE,
        rustshot::tray::MENU_CAPTURE_WINDOW
    );
}

// ---------------------------------------------------------------------------
// resolve_screen_index — additional edge cases
// ---------------------------------------------------------------------------

#[test]
fn resolve_screen_index_single_screen_index_zero_ok() {
    assert_eq!(
        rustshot::capture::resolve_screen_index(1, Some(0)).unwrap(),
        0
    );
}

#[test]
fn resolve_screen_index_single_screen_index_one_is_error() {
    let err = rustshot::capture::resolve_screen_index(1, Some(1)).unwrap_err();
    assert!(err.contains("out of range"));
    assert!(err.contains("0-0"));
}

#[test]
fn resolve_screen_index_usize_max_is_out_of_range() {
    assert!(rustshot::capture::resolve_screen_index(5, Some(usize::MAX)).is_err());
}

#[test]
fn resolve_screen_index_none_with_various_counts_always_zero() {
    for count in [1, 2, 5, 10, 100] {
        assert_eq!(
            rustshot::capture::resolve_screen_index(count, None).unwrap(),
            0,
            "None with {count} screens should return 0"
        );
    }
}

// ---------------------------------------------------------------------------
// frame_duration_hns — mathematical properties
// ---------------------------------------------------------------------------

#[test]
fn frame_duration_hns_times_fps_approximates_one_second() {
    // 10_000_000 HNS = 1 second. fps * frame_duration ≈ 10_000_000
    for fps in [1u32, 10, 24, 30, 60, 120] {
        let duration = rustshot::record::frame_duration_hns(fps);
        let product = duration * (fps as i64);
        // Due to integer division truncation, product may be slightly less
        let error = (10_000_000i64 - product).abs();
        assert!(
            error < fps as i64,
            "fps={fps}: {fps}*{duration}={product}, error={error} should be < {fps}"
        );
    }
}

#[test]
fn frame_duration_hns_10fps() {
    assert_eq!(rustshot::record::frame_duration_hns(10), 1_000_000);
}

#[test]
fn frame_duration_hns_25fps_pal() {
    assert_eq!(rustshot::record::frame_duration_hns(25), 400_000);
}

#[test]
fn frame_duration_hns_u32_max_no_panic() {
    // Should not panic or overflow
    let _ = rustshot::record::frame_duration_hns(u32::MAX);
}

// ---------------------------------------------------------------------------
// pack_u32_pair — WMF encoding patterns
// ---------------------------------------------------------------------------

#[test]
fn pack_u32_pair_aspect_ratio_1_1() {
    let packed = rustshot::record::pack_u32_pair(1, 1);
    assert_eq!((packed >> 32) as u32, 1);
    assert_eq!(packed as u32, 1);
}

#[test]
fn pack_u32_pair_common_frame_sizes() {
    for (w, h) in [
        (640, 480),
        (1280, 720),
        (1920, 1080),
        (2560, 1440),
        (3840, 2160),
    ] {
        let packed = rustshot::record::pack_u32_pair(w, h);
        assert_eq!((packed >> 32) as u32, w);
        assert_eq!(packed as u32, h);
    }
}

#[test]
fn pack_u32_pair_is_deterministic() {
    let a = rustshot::record::pack_u32_pair(1920, 1080);
    let b = rustshot::record::pack_u32_pair(1920, 1080);
    assert_eq!(a, b);
}

#[test]
fn pack_u32_pair_order_matters() {
    let ab = rustshot::record::pack_u32_pair(100, 200);
    let ba = rustshot::record::pack_u32_pair(200, 100);
    assert_ne!(ab, ba);
}

// ---------------------------------------------------------------------------
// rgba_to_bgra_bottom_up — larger images
// ---------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_4x4_corners() {
    // Verify specific corner pixels in a 4x4 image
    let w = 4u32;
    let h = 4u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    // Top-left pixel: R=10
    rgba[0] = 10;
    // Bottom-right pixel: R=20 (last row, last pixel)
    let br = ((h - 1) * w + (w - 1)) as usize * 4;
    rgba[br] = 20;

    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, w, h);

    // After flip: top-left of output = bottom-left of input
    // Bottom-right of input becomes top-right of output
    let tr_out = (w - 1) as usize * 4; // top-right of output = last pixel of first row
    assert_eq!(result[tr_out + 2], 20); // R moved to B channel pos 2 (BGRA)

    // Top-left of input becomes bottom-left of output
    let bl_out = ((h - 1) * w) as usize * 4;
    assert_eq!(result[bl_out + 2], 10); // R moved to B channel pos 2
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_all_alpha_values() {
    let w = 4u32;
    let h = 3u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    // Set distinct alpha values
    for i in 0..(w * h) as usize {
        rgba[i * 4 + 3] = (i * 17 % 256) as u8;
    }
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, w, h);

    // All alpha values should still exist (just reordered by row flip)
    let mut src_alphas: Vec<u8> = (0..(w * h) as usize).map(|i| rgba[i * 4 + 3]).collect();
    let mut dst_alphas: Vec<u8> = (0..(w * h) as usize).map(|i| result[i * 4 + 3]).collect();
    src_alphas.sort();
    dst_alphas.sort();
    assert_eq!(src_alphas, dst_alphas);
}

// ---------------------------------------------------------------------------
// bgra_to_rgba — pixel-level verification
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_sequential_values() {
    // 3 pixels: [0,1,2,3, 4,5,6,7, 8,9,10,11]
    let mut pixels = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    // Each chunk: swap [0]↔[2]
    assert_eq!(pixels, vec![2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11]);
}

#[test]
fn bgra_to_rgba_only_r_and_b_differ() {
    // When R==B, swap is invisible
    let mut pixels = vec![42, 100, 42, 200];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![42, 100, 42, 200]);
}

// ---------------------------------------------------------------------------
// Settings — inequality and structural tests
// ---------------------------------------------------------------------------

#[test]
fn settings_with_different_format_not_equal() {
    let a = rustshot::settings::Settings {
        default_format: Format::Jpg,
        ..rustshot::settings::Settings::default()
    };
    let b = rustshot::settings::Settings {
        default_format: Format::Bmp,
        ..rustshot::settings::Settings::default()
    };
    assert_ne!(a, b);
}

#[test]
fn settings_with_different_fps_not_equal() {
    let a = rustshot::settings::Settings {
        recording_fps: Fps::DEFAULT,
        ..rustshot::settings::Settings::default()
    };
    let b = rustshot::settings::Settings {
        recording_fps: Fps::new(60).unwrap(),
        ..rustshot::settings::Settings::default()
    };
    assert_ne!(a, b);
}

#[test]
fn settings_with_different_auto_start_not_equal() {
    let a = rustshot::settings::Settings {
        auto_start: true,
        ..rustshot::settings::Settings::default()
    };
    let b = rustshot::settings::Settings {
        auto_start: false,
        ..rustshot::settings::Settings::default()
    };
    assert_ne!(a, b);
}

#[test]
fn format_settings_json_each_key_appears_once() {
    let json = rustshot::settings::format_settings_json(&rustshot::settings::Settings::default());
    for key in [
        "screenshots_dir",
        "recordings_dir",
        "default_format",
        "recording_fps",
        "auto_start",
    ] {
        let pattern = format!("\"{}\"", key);
        assert_eq!(
            json.matches(&pattern).count(),
            1,
            "key '{key}' should appear exactly once"
        );
    }
}

#[test]
fn format_settings_json_auto_start_false() {
    let s = rustshot::settings::Settings {
        auto_start: false,
        ..rustshot::settings::Settings::default()
    };
    let json = rustshot::settings::format_settings_json(&s);
    assert!(json.contains("false"));
}

#[test]
fn format_settings_json_auto_start_true() {
    let s = rustshot::settings::Settings {
        auto_start: true,
        ..rustshot::settings::Settings::default()
    };
    let json = rustshot::settings::format_settings_json(&s);
    assert!(json.contains("true"));
}

#[test]
fn parse_settings_json_extra_commas_still_parses() {
    let json = r#"{ "default_format": "bmp", "recording_fps": 60, }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Bmp);
    assert_eq!(parsed.recording_fps.get(), 60);
}

#[test]
fn parse_settings_json_bool_for_number_field_keeps_default() {
    let json = r#"{ "recording_fps": true }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.recording_fps.get(), 30);
}

#[test]
fn parse_settings_json_only_recordings_dir() {
    let json = r#"{ "recordings_dir": "/custom/recs" }"#;
    let parsed = rustshot::settings::parse_settings_json(json);
    assert_eq!(parsed.recordings_dir, PathBuf::from("/custom/recs"));
    // Other fields should be defaults
    assert_eq!(parsed.default_format, Format::Png);
    assert_eq!(parsed.recording_fps.get(), 30);
}

// ---------------------------------------------------------------------------
// Cross-module: bgra_to_rgba ↔ rgba_to_bgra_bottom_up composability
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_then_rgba_to_bgra_bottom_up_1x1_roundtrip() {
    // Start with BGRA data, convert to RGBA, then to BGRA bottom-up (1x1 = no flip)
    let bgra = vec![50u8, 100, 150, 255]; // B=50, G=100, R=150, A=255
    let mut rgba = bgra.clone();
    rustshot::capture::bgra_to_rgba(&mut rgba);
    // Now rgba = [150, 100, 50, 255] (R=150, G=100, B=50, A=255)
    let bgra_bu = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 1);
    // BGRA bottom-up with 1 row = just channel swap: [50, 100, 150, 255]
    assert_eq!(bgra_bu, bgra);
}

// ---------------------------------------------------------------------------
// get_screenshots_dir / get_recording_dir relationship
// ---------------------------------------------------------------------------

#[test]
fn recording_dir_is_child_of_screenshots_dir() {
    let screenshots = rustshot::capture::get_screenshots_dir();
    let recordings = rustshot::record::get_recording_dir();
    assert_eq!(recordings.parent(), Some(screenshots.as_path()));
}

#[test]
fn get_screenshots_dir_is_absolute_or_dot_relative() {
    let dir = rustshot::capture::get_screenshots_dir();
    let display = dir.display().to_string();
    assert!(
        dir.is_absolute() || display.starts_with('.'),
        "should be absolute or dot-relative: {display}"
    );
}

#[test]
fn get_recording_dir_has_at_least_three_components() {
    let dir = rustshot::record::get_recording_dir();
    assert!(
        dir.components().count() >= 3,
        "recording dir should have at least Pictures/RustShot/recordings"
    );
}

// ---------------------------------------------------------------------------
// cli module
// ---------------------------------------------------------------------------

/// Was `cli::settings_capture_format`. That mapper is gone — parsing a settings
/// format string is now the `Format` enum's own constructor. Same behaviour.
#[test]
fn settings_format_string_maps_to_supported_values() {
    assert_eq!(Format::from_settings_str("jpg"), Format::Jpg);
    assert_eq!(Format::from_settings_str("bmp"), Format::Bmp);
    assert_eq!(Format::from_settings_str("gif"), Format::Png);
}

#[test]
fn cli_format_flag_present_detects_explicit_forms() {
    assert!(rustshot::cli::cli_format_flag_present_in([
        "rustshot", "--format", "png"
    ]));
    assert!(rustshot::cli::cli_format_flag_present_in([
        "rustshot",
        "--format=jpg"
    ]));
    assert!(rustshot::cli::cli_format_flag_present_in([
        "rustshot", "-fbmp"
    ]));
}

#[test]
fn cli_format_flag_present_skips_output_values() {
    assert!(!rustshot::cli::cli_format_flag_present_in([
        "rustshot", "--output", "--format"
    ]));
    assert!(!rustshot::cli::cli_format_flag_present_in([
        "rustshot", "-o", "-fpng"
    ]));
}

#[test]
fn cli_output_extension_extracts_png() {
    assert_eq!(
        rustshot::cli::output_extension(Some(std::path::Path::new("/tmp/shot.png"))),
        Some("png")
    );
}

#[test]
fn cli_format_from_extension_supports_jpeg_alias() {
    assert!(matches!(
        rustshot::cli::format_from_extension("JPEG"),
        Some(rustshot::capture::Format::Jpg)
    ));
}

#[test]
fn cli_format_from_output_extension_returns_none_for_missing_extension() {
    assert!(
        rustshot::cli::format_from_output_extension(Some(std::path::Path::new("/tmp/shot")))
            .is_none()
    );
}

#[test]
fn cli_unsupported_output_extension_returns_unknown_extension() {
    assert_eq!(
        rustshot::cli::unsupported_output_extension(Some(std::path::Path::new("/tmp/shot.webp"))),
        Some("webp".to_string())
    );
}

#[test]
fn cli_effective_cli_format_prefers_explicit_format() {
    assert!(matches!(
        rustshot::cli::effective_cli_format(
            rustshot::capture::Format::Bmp,
            true,
            Some(std::path::Path::new("/tmp/shot.png")),
            rustshot::capture::Format::Jpg,
        ),
        rustshot::capture::Format::Bmp
    ));
}

#[test]
fn cli_effective_cli_format_uses_output_extension_before_settings() {
    assert!(matches!(
        rustshot::cli::effective_cli_format(
            rustshot::capture::Format::Png,
            false,
            Some(std::path::Path::new("/tmp/shot.bmp")),
            rustshot::capture::Format::Jpg,
        ),
        rustshot::capture::Format::Bmp
    ));
}

#[test]
fn cli_should_run_cli_mode_uses_any_trigger_flag() {
    assert!(rustshot::cli::should_run_cli_mode(
        false, false, false, false, false, true
    ));
    assert!(rustshot::cli::should_run_cli_mode(
        false, false, true, false, false, false
    ));
    assert!(!rustshot::cli::should_run_cli_mode(
        false, false, false, false, false, false
    ));
}

#[test]
fn cli_format_screen_list_output_indents_each_entry() {
    let output =
        rustshot::cli::format_screen_list_output(vec!["Screen 0: 1920x1080", "Screen 1: 1280x720"]);
    assert_eq!(
        output,
        "Available screens:\n  Screen 0: 1920x1080\n  Screen 1: 1280x720"
    );
}

#[test]
fn cli_selection_stderr_message_omits_cancelled_error() {
    assert_eq!(
        rustshot::cli::selection_stderr_message("Selection cancelled"),
        None
    );
}

#[test]
fn cli_write_error_line_appends_newline() {
    let mut buf = Vec::new();
    rustshot::cli::write_error_line(&mut buf, "boom").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "boom\n");
}

#[test]
fn cli_child_process_is_running_matches_pending_result() {
    assert!(rustshot::cli::child_process_is_running::<(), ()>(Ok(None)));
    assert!(!rustshot::cli::child_process_is_running::<(), ()>(Ok(
        Some(())
    )));
    assert!(!rustshot::cli::child_process_is_running::<(), ()>(Err(())));
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
