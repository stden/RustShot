use rustshot::capture::Format;
use rustshot::settings::Fps;
use std::path::PathBuf;

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
