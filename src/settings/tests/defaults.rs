//! Tests for defaults, derived traits, path helpers, and Format/Fps normalization.

use super::*;

#[test]
fn settings_defaults_are_stable() {
    let s = Settings::default();
    assert_eq!(s.default_format, Format::Png);
    assert_eq!(s.recording_fps, Fps::DEFAULT);
    assert!(s.auto_start);
    assert_eq!(
        s.screenshots_dir.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
    assert_eq!(
        s.recordings_dir.file_name().and_then(|n| n.to_str()),
        Some("recordings")
    );
}

#[test]
fn settings_and_log_paths_use_rustshot_directory() {
    // settings_path()/log_file_path() read SETTINGS_PATH_OVERRIDE.
    let _global = lock_global_overrides();
    let settings = settings_path();
    let log = log_file_path();

    assert_eq!(
        settings.file_name().and_then(|n| n.to_str()),
        Some("settings.json")
    );
    assert_eq!(
        settings
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|n| n.to_str()),
        Some("RustShot")
    );
    assert_eq!(
        log.file_name().and_then(|n| n.to_str()),
        Some("rustshot.log")
    );
    assert_eq!(
        log.parent()
            .and_then(|parent| parent.file_name())
            .and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

// --- normalize_default_format tests ---

#[test]
fn normalize_default_format_invalid_values_fall_back_to_png() {
    for value in ["gif", "", "PNG", "Jpg", " png"] {
        assert_eq!(
            Format::from_settings_str(value),
            Format::Png,
            "value: {value:?}"
        );
    }
}

// --- normalize_recording_fps tests ---

/// Previously `u32::MAX` passed straight through and only blew up later, at
/// record time. The `Fps` newtype makes an out-of-range rate unrepresentable,
/// so a hand-edited settings.json now falls back to the default instead.
#[test]
fn fps_new_rejects_out_of_range_but_from_settings_forgives() {
    assert!(Fps::new(0).is_err());
    assert!(Fps::new(Fps::MAX + 1).is_err());
    assert_eq!(Fps::new(Fps::MAX).unwrap().get(), Fps::MAX);
    for valid in [1, 60, Fps::MAX] {
        assert_eq!(Fps::from_settings(valid).get(), valid);
    }
    // Same inputs, forgiving constructor: no error, just the default.
    assert_eq!(Fps::from_settings(0), Fps::DEFAULT);
    assert_eq!(Fps::from_settings(Fps::MAX + 1), Fps::DEFAULT);
    assert_eq!(Fps::from_settings(u32::MAX), Fps::DEFAULT);
}

#[test]
fn fps_converts_only_through_the_validated_newtype() {
    let fps = Fps::try_from(60).unwrap();

    assert_eq!(u32::from(fps), 60);
    assert_eq!(
        Fps::try_from(0),
        Err("Recording FPS must be greater than 0".to_string())
    );
}

// -------------------------------------------------------------------------
// wide() helper tests
// -------------------------------------------------------------------------

#[test]
fn wide_encodes_utf16_and_appends_null() {
    let cases = [
        ("", vec![0]),
        ("AB", vec![0x41, 0x42, 0]),
        ("Rüst", vec![0x52, 0x00FC, 0x73, 0x74, 0]),
        ("🦀", vec![0xD83E, 0xDD80, 0]),
    ];

    for (input, expected) in cases {
        assert_eq!(wide(input), expected, "input: {input:?}");
    }
}

// -------------------------------------------------------------------------
// generate_timestamp tests
// -------------------------------------------------------------------------

#[test]
fn generate_timestamp_has_expected_filename_safe_shape() {
    let ts = generate_timestamp();
    let bytes = ts.as_bytes();

    assert_eq!(bytes.len(), 25, "timestamp: {ts}");
    assert_eq!(bytes[8], b'_', "timestamp: {ts}");
    assert_eq!(bytes[15], b'_', "timestamp: {ts}");
    assert!(
        bytes[..8]
            .iter()
            .chain(&bytes[9..15])
            .chain(&bytes[16..])
            .all(u8::is_ascii_digit),
        "timestamp: {ts}"
    );
}

// -------------------------------------------------------------------------
// normalize_default_format — all supported formats
// -------------------------------------------------------------------------

#[test]
fn normalize_default_format_all_supported() {
    assert_eq!(Format::from_settings_str("png").extension(), "png");
    assert_eq!(Format::from_settings_str("jpg").extension(), "jpg");
    assert_eq!(Format::from_settings_str("bmp").extension(), "bmp");
}
