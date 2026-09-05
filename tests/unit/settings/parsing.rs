//! Tests for `parse_settings_json`: defaults, malformed input, and key handling.

use super::*;

#[test]
fn parse_settings_json_partial_keys() {
    let json = r#"{ "default_format": "bmp", "recording_fps": 15 }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Bmp);
    assert_eq!(parsed.recording_fps.get(), 15);
    assert!(parsed.auto_start); // default
}

#[test]
fn parse_settings_json_ignores_similar_key_names() {
    let json =
        r#"{ "default_format_backup": "bmp", "recording_fps_limit": 15, "auto_startup": false }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Png);
    assert_eq!(parsed.recording_fps.get(), 30);
    assert!(parsed.auto_start);
}

// --- malformed JSON parsing ---

#[test]
fn parse_settings_json_unterminated_string_returns_defaults() {
    let parsed = parse_settings_json(r#"{ "default_format": "jpg"#);
    assert_eq!(parsed.default_format, Format::Png);
    assert_eq!(parsed.recording_fps.get(), 30);
    assert!(parsed.auto_start);
}

#[test]
fn parse_settings_json_empty_string_returns_defaults() {
    let parsed = parse_settings_json("");
    assert_eq!(parsed, Settings::default());
}

#[test]
fn parse_settings_json_null_value_for_string_field_keeps_default() {
    let json = r#"{ "default_format": null }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Png);
}

#[test]
fn parse_settings_json_number_for_string_field_keeps_default() {
    let json = r#"{ "default_format": 42 }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Png);
}

#[test]
fn parse_settings_json_string_for_number_field_keeps_default() {
    let json = r#"{ "recording_fps": "sixty" }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.recording_fps.get(), 30);
}

#[test]
fn parse_settings_json_float_recording_fps_keeps_default() {
    let json = r#"{ "recording_fps": 30.5 }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.recording_fps.get(), 30);
}

#[test]
fn parse_settings_json_string_for_bool_field_keeps_default() {
    let json = r#"{ "auto_start": "yes" }"#;
    let parsed = parse_settings_json(json);
    assert!(parsed.auto_start);
}

#[test]
fn parse_settings_json_explicit_auto_start_false_overrides_default() {
    let json = r#"{ "auto_start": false, "recording_fps": 60 }"#;
    let parsed = parse_settings_json(json);
    assert!(!parsed.auto_start);
    assert_eq!(parsed.recording_fps.get(), 60);
}

#[test]
fn parse_settings_json_extra_commas_still_parses_valid_keys() {
    let json = r#"{ "default_format": "bmp", "recording_fps": 60, }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Bmp);
    assert_eq!(parsed.recording_fps.get(), 60);
}

// --- duplicate keys ---

#[test]
fn parse_settings_json_duplicate_key_uses_first_occurrence() {
    // The parser uses find() which returns the first match
    let json = r#"{ "default_format": "jpg", "default_format": "bmp" }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.default_format, Format::Jpg);
}

#[test]
fn parse_settings_json_duplicate_recording_fps_uses_first_occurrence() {
    let json = r#"{ "recording_fps": 15, "recording_fps": 60 }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.recording_fps.get(), 15);
}

#[test]
fn parse_settings_json_duplicate_auto_start_uses_first_occurrence() {
    let json = r#"{ "auto_start": false, "auto_start": true }"#;
    let parsed = parse_settings_json(json);
    assert!(!parsed.auto_start);
}

#[test]
fn parse_settings_json_duplicate_screenshots_dir_uses_first_occurrence() {
    let json = r#"{ "screenshots_dir": "/first", "screenshots_dir": "/second" }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.screenshots_dir, PathBuf::from("/first"));
}

#[test]
fn parse_settings_json_duplicate_recordings_dir_uses_first_occurrence() {
    let json = r#"{ "recordings_dir": "/first-recs", "recordings_dir": "/second-recs" }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(parsed.recordings_dir, PathBuf::from("/first-recs"));
}

#[test]
fn parse_settings_json_unescapes_backslashes_in_paths() {
    let json = r#"{ "screenshots_dir": "C:\\Users\\test\\Shots", "recordings_dir": "C:\\Users\\test\\Shots\\recordings" }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(
        parsed.screenshots_dir,
        PathBuf::from(r"C:\Users\test\Shots")
    );
    assert_eq!(
        parsed.recordings_dir,
        PathBuf::from(r"C:\Users\test\Shots\recordings")
    );
}

#[test]
fn parse_settings_json_invalid_bool_token_keeps_default() {
    let json = r#"{ "auto_start": trueish }"#;
    let parsed = parse_settings_json(json);
    assert!(parsed.auto_start);
}

// -------------------------------------------------------------------------
// parse_settings_json edge cases
// -------------------------------------------------------------------------

#[test]
fn parse_settings_json_normalizes_unknown_format_to_png() {
    let json = r#"{ "default_format": "webp" }"#;
    let s = parse_settings_json(json);
    assert_eq!(s.default_format, Format::Png);
}

#[test]
fn parse_settings_json_normalizes_zero_fps_to_30() {
    let json = r#"{ "recording_fps": 0 }"#;
    let s = parse_settings_json(json);
    assert_eq!(s.recording_fps.get(), 30);
}

#[test]
fn parse_settings_json_unicode_in_path() {
    let json = r#"{ "screenshots_dir": "/home/пользователь/снимки" }"#;
    let parsed = parse_settings_json(json);
    assert_eq!(
        parsed.screenshots_dir,
        PathBuf::from("/home/пользователь/снимки")
    );
}

#[test]
fn parse_settings_json_ignores_key_like_text_inside_string_values() {
    let json = concat!(
        "{",
        r#""screenshots_dir": "/tmp/\"recording_fps\": 999","#,
        r#""recording_fps": 30"#,
        "}"
    );

    let parsed = parse_settings_json(json);

    assert_eq!(
        parsed.screenshots_dir,
        PathBuf::from(r#"/tmp/"recording_fps": 999"#)
    );
    assert_eq!(parsed.recording_fps.get(), 30);
}

#[test]
fn parse_settings_json_ignores_nested_keys_when_top_level_key_exists() {
    let json = concat!(
        "{",
        r#""extra": { "recording_fps": 999 },"#,
        r#""recording_fps": 30"#,
        "}"
    );

    let parsed = parse_settings_json(json);

    assert_eq!(parsed.recording_fps.get(), 30);
}

#[test]
fn parse_settings_json_fps_boundary_values() {
    for fps in [1, 30, 60, 120, 240] {
        let json = format!(r#"{{ "recording_fps": {} }}"#, fps);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.recording_fps.get(), fps);
    }
}

// -------------------------------------------------------------------------
// parse_settings_json — extra/unknown keys ignored
// -------------------------------------------------------------------------

#[test]
fn parse_settings_json_ignores_unknown_keys() {
    let json = r#"{"unknown_key": "value", "default_format": "bmp"}"#;
    let settings = parse_settings_json(json);
    assert_eq!(settings.default_format, Format::Bmp);
}

#[test]
fn parse_settings_json_empty_object_returns_defaults() {
    let settings = parse_settings_json("{}");
    let defaults = Settings::default();
    assert_eq!(settings, defaults);
}

// -------------------------------------------------------------------------
// parse_settings_json — malformed JSON
// -------------------------------------------------------------------------

#[test]
fn parse_settings_json_completely_invalid_returns_defaults() {
    let settings = parse_settings_json("not json at all");
    assert_eq!(settings, Settings::default());
}

#[test]
fn parse_settings_json_null_returns_defaults() {
    let settings = parse_settings_json("null");
    assert_eq!(settings, Settings::default());
}
