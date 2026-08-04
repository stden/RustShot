use rustshot::capture::Format;
use rustshot::settings::{Fps, Settings};
use std::path::PathBuf;

#[test]
fn format_and_parse_settings_preserves_all_fields() {
    let original = Settings {
        screenshots_dir: PathBuf::from("/a/b"),
        recordings_dir: PathBuf::from("/c/d"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(120).unwrap(),
        auto_start: false,
    };
    let parsed = rustshot::settings::parse_settings_json(
        &rustshot::settings::format_settings_json(&original),
    );
    assert_eq!(parsed, original);
}

#[test]
fn parse_settings_json_empty_string_returns_defaults() {
    assert_eq!(
        rustshot::settings::parse_settings_json(""),
        Settings::default()
    );
}

#[test]
fn parse_settings_json_partial_keys_preserves_other_defaults() {
    let parsed = rustshot::settings::parse_settings_json(
        r#"{ "default_format": "bmp", "recording_fps": 15 }"#,
    );
    assert_eq!(parsed.default_format, Format::Bmp);
    assert_eq!(parsed.recording_fps.get(), 15);
    assert!(parsed.auto_start);
}

#[test]
fn parse_settings_json_duplicate_auto_start_uses_first_occurrence() {
    let parsed =
        rustshot::settings::parse_settings_json(r#"{ "auto_start": false, "auto_start": true }"#);
    assert!(!parsed.auto_start);
}

#[test]
fn parse_settings_json_invalid_auto_start_token_keeps_default() {
    let parsed = rustshot::settings::parse_settings_json(r#"{ "auto_start": trueish }"#);
    assert!(parsed.auto_start);
}

#[test]
fn parse_settings_json_duplicate_recording_fps_uses_first_occurrence() {
    let parsed =
        rustshot::settings::parse_settings_json(r#"{ "recording_fps": 15, "recording_fps": 60 }"#);
    assert_eq!(parsed.recording_fps.get(), 15);
}

#[test]
fn parse_settings_json_duplicate_string_uses_first_occurrence() {
    let parsed = rustshot::settings::parse_settings_json(
        r#"{ "screenshots_dir": "/first", "screenshots_dir": "/second" }"#,
    );
    assert_eq!(parsed.screenshots_dir, PathBuf::from("/first"));
}

#[test]
fn parse_settings_json_windows_backslash_roundtrip() {
    let original = Settings {
        screenshots_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot"),
        recordings_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot\\recordings"),
        ..Settings::default()
    };
    let parsed = rustshot::settings::parse_settings_json(
        &rustshot::settings::format_settings_json(&original),
    );
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
}

#[test]
fn parse_settings_json_paths_with_quotes_roundtrip() {
    let original = Settings {
        screenshots_dir: PathBuf::from("/tmp/screens\"shots"),
        recordings_dir: PathBuf::from("/tmp/reco\"rdings"),
        ..Settings::default()
    };
    let parsed = rustshot::settings::parse_settings_json(
        &rustshot::settings::format_settings_json(&original),
    );
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
}

#[test]
fn parse_settings_json_null_string_field_keeps_default() {
    let parsed = rustshot::settings::parse_settings_json(r#"{ "default_format": null }"#);
    assert_eq!(parsed.default_format, Format::Png);
}

#[test]
fn parse_settings_json_string_for_number_keeps_default() {
    let parsed = rustshot::settings::parse_settings_json(r#"{ "recording_fps": "sixty" }"#);
    assert_eq!(parsed.recording_fps, Fps::DEFAULT);
}

#[test]
fn benchmark_normalize_default_format_unknown_falls_back_to_png() {
    assert_eq!(
        rustshot::settings::benchmark_normalize_default_format("gif"),
        Format::Png
    );
}

#[test]
fn benchmark_normalize_recording_fps_handles_zero_and_valid_values() {
    assert_eq!(
        rustshot::settings::benchmark_normalize_recording_fps(0),
        Fps::DEFAULT
    );
    assert_eq!(
        rustshot::settings::benchmark_normalize_recording_fps(144).get(),
        144
    );
}

#[test]
fn benchmark_extract_json_string_returns_value() {
    assert_eq!(
        rustshot::settings::benchmark_extract_json_string(
            r#"{ "screenshots_dir": "/tmp/shots" }"#,
            "screenshots_dir",
        ),
        Some("/tmp/shots".to_string())
    );
}

#[test]
fn benchmark_extract_json_number_returns_value() {
    assert_eq!(
        rustshot::settings::benchmark_extract_json_number(
            r#"{ "recording_fps": 144 }"#,
            "recording_fps",
        ),
        Some(144)
    );
}

#[test]
fn benchmark_extract_json_bool_returns_value() {
    assert_eq!(
        rustshot::settings::benchmark_extract_json_bool(r#"{ "auto_start": false }"#, "auto_start",),
        Some(false)
    );
}

#[test]
fn format_settings_json_each_key_appears_once() {
    let json = rustshot::settings::format_settings_json(&Settings::default());
    for key in [
        "screenshots_dir",
        "recordings_dir",
        "default_format",
        "recording_fps",
        "auto_start",
    ] {
        assert_eq!(json.matches(&format!("\"{key}\"")).count(), 1);
    }
}

#[test]
fn parse_settings_json_extra_comma_still_parses() {
    let parsed = rustshot::settings::parse_settings_json(
        r#"{ "default_format": "bmp", "recording_fps": 60, }"#,
    );
    assert_eq!(parsed.default_format, Format::Bmp);
    assert_eq!(parsed.recording_fps.get(), 60);
}
