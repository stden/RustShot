//! Tests for `format_settings_json` output and format/parse roundtrips.

use super::*;

#[test]
fn format_settings_json_contains_all_keys() {
    let s = Settings::default();
    let json = format_settings_json(&s);
    assert!(json.contains("screenshots_dir"));
    assert!(json.contains("recordings_dir"));
    assert!(json.contains("default_format"));
    assert!(json.contains("recording_fps"));
    assert!(json.contains("auto_start"));
}

#[test]
fn format_settings_json_contains_png() {
    let s = Settings::default();
    let json = format_settings_json(&s);
    assert!(json.contains("\"png\""));
}

#[test]
fn format_settings_json_contains_fps_30() {
    let s = Settings::default();
    let json = format_settings_json(&s);
    assert!(json.contains("30"));
}

#[test]
fn parse_settings_json_roundtrip() {
    let original = Settings {
        screenshots_dir: PathBuf::from("/tmp/shots"),
        recordings_dir: PathBuf::from("/tmp/recs"),
        default_format: Format::Jpg,
        recording_fps: Fps::new(60).unwrap(),
        auto_start: false,
    };
    let json = format_settings_json(&original);
    let parsed = parse_settings_json(&json);
    assert_eq!(parsed.default_format, Format::Jpg);
    assert_eq!(parsed.recording_fps.get(), 60);
    assert!(!parsed.auto_start);
}

#[test]
fn format_settings_json_auto_start_true() {
    let s = Settings {
        auto_start: true,
        ..Settings::default()
    };
    let json = format_settings_json(&s);
    assert!(json.contains("true"));
}

#[test]
fn format_settings_json_auto_start_false() {
    let s = Settings {
        auto_start: false,
        ..Settings::default()
    };
    let json = format_settings_json(&s);
    assert!(json.contains("false"));
}

#[test]
fn format_settings_json_contains_auto_start_key_once() {
    let json = format_settings_json(&Settings::default());
    assert_eq!(json.matches("\"auto_start\"").count(), 1);
}

// --- Windows backslash roundtrip ---

#[test]
fn format_and_parse_windows_backslash_paths() {
    let settings = Settings {
        screenshots_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot"),
        recordings_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot\\recordings"),
        default_format: Format::Png,
        recording_fps: Fps::DEFAULT,
        auto_start: true,
    };
    let json = format_settings_json(&settings);
    let parsed = parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, settings.screenshots_dir);
    assert_eq!(parsed.recordings_dir, settings.recordings_dir);
}

#[test]
fn format_and_parse_paths_with_embedded_quotes() {
    let settings = Settings {
        screenshots_dir: PathBuf::from("/tmp/screens\"shots"),
        recordings_dir: PathBuf::from("/tmp/reco\"rdings"),
        default_format: Format::Png,
        recording_fps: Fps::DEFAULT,
        auto_start: true,
    };
    let json = format_settings_json(&settings);
    let parsed = parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, settings.screenshots_dir);
    assert_eq!(parsed.recordings_dir, settings.recordings_dir);
}

// --- format_settings_json output structure ---

#[test]
fn format_settings_json_is_valid_json_structure() {
    let s = Settings::default();
    let json = format_settings_json(&s);
    assert!(json.starts_with('{'));
    assert!(json.ends_with('}'));
}

#[test]
fn format_settings_json_custom_values() {
    let s = Settings {
        screenshots_dir: PathBuf::from("/custom/shots"),
        recordings_dir: PathBuf::from("/custom/recs"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(120).unwrap(),
        auto_start: false,
    };
    let json = format_settings_json(&s);
    assert!(json.contains("/custom/shots"));
    assert!(json.contains("/custom/recs"));
    assert!(json.contains("\"bmp\""));
    assert!(json.contains("120"));
    assert!(json.contains("false"));
}

#[test]
fn format_settings_json_escapes_backslashes_in_paths() {
    let s = Settings {
        screenshots_dir: PathBuf::from(r"C:\Users\test\Shots"),
        recordings_dir: PathBuf::from(r"C:\Users\test\Shots\recordings"),
        ..Settings::default()
    };
    let json = format_settings_json(&s);
    assert!(json.contains(r#""screenshots_dir": "C:\\Users\\test\\Shots""#));
    assert!(json.contains(r#""recordings_dir": "C:\\Users\\test\\Shots\\recordings""#));
}

// --- parse_settings_json roundtrip with all fields changed ---

#[test]
fn parse_settings_json_full_roundtrip_all_fields() {
    let original = Settings {
        screenshots_dir: PathBuf::from("/a/b/c"),
        recordings_dir: PathBuf::from("/d/e/f"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(120).unwrap(),
        auto_start: false,
    };
    let json = format_settings_json(&original);
    let parsed = parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
    assert_eq!(parsed.default_format, original.default_format);
    assert_eq!(parsed.recording_fps, original.recording_fps);
    assert_eq!(parsed.auto_start, original.auto_start);
}

#[test]
fn parse_settings_json_preserves_unc_paths() {
    let original = Settings {
        screenshots_dir: PathBuf::from(r"\\server\share\shots"),
        recordings_dir: PathBuf::from(r"\\server\share\shots\recordings"),
        ..Settings::default()
    };

    let json = format_settings_json(&original);
    let parsed = parse_settings_json(&json);

    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
}

// --- format_settings_json structure ---

#[test]
fn format_settings_json_contains_newlines() {
    let s = Settings::default();
    let json = format_settings_json(&s);
    assert!(json.contains('\n'), "JSON should be pretty-printed");
}

#[test]
fn format_settings_json_contains_at_least_five_colons() {
    let s = Settings::default();
    let json = format_settings_json(&s);
    let colon_count = json.chars().filter(|&c| c == ':').count();
    assert!(
        colon_count >= 5,
        "should have at least 5 colons (one per key-value pair), got {colon_count}"
    );
}

// -------------------------------------------------------------------------
// format_settings_json edge cases
// -------------------------------------------------------------------------

#[test]
fn format_settings_json_includes_every_key() {
    let s = Settings::default();
    let json = format_settings_json(&s);
    assert!(json.contains("screenshots_dir"));
    assert!(json.contains("recordings_dir"));
    assert!(json.contains("default_format"));
    assert!(json.contains("recording_fps"));
    assert!(json.contains("auto_start"));
}

#[test]
fn format_settings_json_escapes_backslashes() {
    let s = Settings {
        screenshots_dir: PathBuf::from("C:\\Users\\test"),
        ..Settings::default()
    };
    let json = format_settings_json(&s);
    assert!(json.contains("C:\\\\Users\\\\test"));
}

#[test]
fn format_settings_json_output_is_valid_roundtrip() {
    let original = Settings {
        screenshots_dir: PathBuf::from("/tmp/a"),
        recordings_dir: PathBuf::from("/tmp/b"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(240).unwrap(),
        auto_start: false,
    };
    let json = format_settings_json(&original);
    let loaded = parse_settings_json(&json);
    assert_eq!(loaded.default_format, original.default_format);
    assert_eq!(loaded.recording_fps, original.recording_fps);
    assert_eq!(loaded.auto_start, original.auto_start);
}

#[test]
fn format_parse_roundtrip_with_unicode_paths() {
    let original = Settings {
        screenshots_dir: PathBuf::from("/home/用户/截图"),
        recordings_dir: PathBuf::from("/home/用户/录像"),
        default_format: Format::Jpg,
        recording_fps: Fps::new(24).unwrap(),
        auto_start: true,
    };
    let json = format_settings_json(&original);
    let parsed = parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
    assert_eq!(parsed.default_format, Format::Jpg);
    assert_eq!(parsed.recording_fps.get(), 24);
}

// -------------------------------------------------------------------------
// format_settings_json + parse_settings_json roundtrip with edge cases
// -------------------------------------------------------------------------

#[test]
fn format_parse_roundtrip_with_backslash_heavy_path() {
    let original = Settings {
        screenshots_dir: PathBuf::from(r"C:\Users\test\Documents\My Pictures"),
        recordings_dir: PathBuf::from(r"D:\Videos\Screen Recordings"),
        default_format: Format::Png,
        recording_fps: Fps::DEFAULT,
        auto_start: true,
    };
    let json = format_settings_json(&original);
    let parsed = parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
    assert_eq!(parsed.recording_fps, original.recording_fps);
    assert_eq!(parsed.auto_start, original.auto_start);
}

#[test]
fn format_parse_roundtrip_preserves_auto_start_false() {
    let original = Settings {
        auto_start: false,
        ..Settings::default()
    };
    let json = format_settings_json(&original);
    let parsed = parse_settings_json(&json);
    assert!(!parsed.auto_start);
}

// -------------------------------------------------------------------------
// format/parse roundtrip with Windows backslash paths
// -------------------------------------------------------------------------

#[test]
fn format_parse_roundtrip_preserves_backslash_paths() {
    let original = Settings {
        screenshots_dir: PathBuf::from(r"C:\Users\test\Pictures\RustShot"),
        recordings_dir: PathBuf::from(r"C:\Users\test\Videos\recordings"),
        ..Settings::default()
    };
    let json = format_settings_json(&original);
    let parsed = parse_settings_json(&json);
    assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
    assert_eq!(parsed.recordings_dir, original.recordings_dir);
}

#[test]
fn format_settings_json_backslash_is_escaped() {
    let s = Settings {
        screenshots_dir: PathBuf::from(r"C:\path"),
        ..Settings::default()
    };
    let json = format_settings_json(&s);
    // Backslash should be escaped as \\
    assert!(
        json.contains(r"C:\\path"),
        "backslash should be escaped: {json}"
    );
}

// -------------------------------------------------------------------------
// Requirement 12.7: format_settings_json each key appears exactly once
// -------------------------------------------------------------------------

#[test]
fn format_settings_json_each_key_appears_exactly_once() {
    let json = format_settings_json(&Settings::default());
    let keys = [
        "\"screenshots_dir\"",
        "\"recordings_dir\"",
        "\"default_format\"",
        "\"recording_fps\"",
        "\"auto_start\"",
    ];
    for key in keys {
        assert_eq!(
            json.matches(key).count(),
            1,
            "key {key} should appear exactly once in: {json}"
        );
    }
}

// -------------------------------------------------------------------------
// Property-based test: Settings JSON roundtrip
// -------------------------------------------------------------------------

use proptest::prelude::*;

proptest! {
    /// **Validates: Requirements 17.5, 12.1**
    ///
    /// For all valid Settings with supported format strings and positive FPS,
    /// `parse_settings_json(format_settings_json(s))` produces equivalent Settings.
    #[test]
    fn prop_settings_json_roundtrip(
        // The path strategy deliberately includes characters that need JSON
        // escaping — double quotes, backslashes, and control characters
        // (\x00-\x1f covers \n, \r, \t, NUL) — so the roundtrip exercises the
        // escaper, not just the happy path.
        screenshots_dir in r#"[a-zA-Z0-9/_ \\"\x00-\x1f]{1,30}"#,
        recordings_dir in r#"[a-zA-Z0-9/_ \\"\x00-\x1f]{1,30}"#,
        default_format in prop_oneof!["png", "jpg", "bmp"],
        // The strategy is bounded by the type's own invariant: an `Fps`
        // above `Fps::MAX` is not constructible, so there is nothing to
        // generate outside this range.
        recording_fps in 1u32..=Fps::MAX,
        auto_start in any::<bool>(),
    ) {
        let original = Settings {
            screenshots_dir: PathBuf::from(&screenshots_dir),
            recordings_dir: PathBuf::from(&recordings_dir),
            default_format: Format::from_settings_str(&default_format),
            recording_fps: Fps::new(recording_fps).expect("strategy is bounded by Fps::MAX"),
            auto_start,
        };

        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);

        prop_assert_eq!(
            &parsed.screenshots_dir, &original.screenshots_dir,
            "screenshots_dir mismatch after roundtrip"
        );
        prop_assert_eq!(
            &parsed.recordings_dir, &original.recordings_dir,
            "recordings_dir mismatch after roundtrip"
        );
        prop_assert_eq!(
            &parsed.default_format, &original.default_format,
            "default_format mismatch after roundtrip"
        );
        prop_assert_eq!(
            parsed.recording_fps, original.recording_fps,
            "recording_fps mismatch after roundtrip"
        );
        prop_assert_eq!(
            parsed.auto_start, original.auto_start,
            "auto_start mismatch after roundtrip"
        );
    }
}
