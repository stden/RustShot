//! Tests for `format_settings_json` output and format/parse roundtrips.

use super::*;

// --- Windows backslash roundtrip ---

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

// -------------------------------------------------------------------------
// format_settings_json edge cases
// -------------------------------------------------------------------------

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

// -------------------------------------------------------------------------
// format/parse roundtrip with Windows backslash paths
// -------------------------------------------------------------------------

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

#[cfg(unix)]
#[test]
fn settings_roundtrip_preserves_paths_that_resemble_raw_path_encoding() {
    for suffix in ["", "2e", "2f746d70", "ff"] {
        let path = PathBuf::from(format!("\u{1e}rustshot-path-hex:{suffix}"));
        let original = Settings {
            screenshots_dir: path.clone(),
            recordings_dir: path,
            ..Settings::default()
        };
        assert_eq!(
            parse_settings_json(&format_settings_json(&original)),
            original
        );
    }
}

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
