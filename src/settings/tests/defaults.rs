//! Tests for defaults, derived traits, path helpers, and Format/Fps normalization.

use super::*;

#[test]
fn settings_default_format_is_png() {
    let s = Settings::default();
    assert_eq!(s.default_format, Format::Png);
}

#[test]
fn settings_default_fps_is_30() {
    let s = Settings::default();
    assert_eq!(s.recording_fps.get(), 30);
}

#[test]
fn settings_default_auto_start_is_true() {
    let s = Settings::default();
    assert!(s.auto_start);
}

#[test]
fn settings_default_screenshots_dir_ends_with_rustshot() {
    let s = Settings::default();
    assert_eq!(
        s.screenshots_dir.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn settings_default_recordings_dir_ends_with_recordings() {
    let s = Settings::default();
    assert_eq!(
        s.recordings_dir.file_name().and_then(|n| n.to_str()),
        Some("recordings")
    );
}

#[test]
fn settings_path_ends_with_settings_json() {
    let p = settings_path();
    assert_eq!(
        p.file_name().and_then(|n| n.to_str()),
        Some("settings.json")
    );
}

#[test]
fn settings_path_parent_is_rustshot() {
    let p = settings_path();
    let parent = p.parent().unwrap();
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn settings_clone_works() {
    let s = Settings::default();
    let s2 = s.clone();
    assert_eq!(s, s2);
}

#[test]
fn settings_debug_is_non_empty() {
    let s = Settings::default();
    assert!(!format!("{:?}", s).is_empty());
}

// --- Settings equality ---

#[test]
fn settings_with_different_format_are_not_equal() {
    let mut a = Settings::default();
    let mut b = Settings::default();
    a.default_format = Format::Jpg;
    b.default_format = Format::Bmp;
    assert_ne!(a, b);
}

#[test]
fn settings_with_different_fps_are_not_equal() {
    let mut a = Settings::default();
    let mut b = Settings::default();
    a.recording_fps = Fps::new(30).unwrap();
    b.recording_fps = Fps::new(60).unwrap();
    assert_ne!(a, b);
}

#[test]
fn settings_with_different_auto_start_are_not_equal() {
    let mut a = Settings::default();
    let mut b = Settings::default();
    a.auto_start = true;
    b.auto_start = false;
    assert_ne!(a, b);
}

// --- settings_path consistency ---

#[test]
fn settings_path_is_consistent_across_calls() {
    let p1 = settings_path();
    let p2 = settings_path();
    assert_eq!(p1, p2);
}

#[test]
fn settings_path_has_at_least_two_components() {
    let p = settings_path();
    assert!(
        p.components().count() >= 2,
        "settings path should have at least parent/settings.json"
    );
}

#[test]
fn log_file_path_ends_with_rustshot_log() {
    let p = log_file_path();
    assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("rustshot.log"));
}

#[test]
fn log_file_path_parent_is_rustshot() {
    let p = log_file_path();
    let parent = p.parent().unwrap();
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn log_file_path_is_consistent_across_calls() {
    let p1 = log_file_path();
    let p2 = log_file_path();
    assert_eq!(p1, p2);
}

// --- normalize_default_format tests ---

#[test]
fn normalize_default_format_png_is_valid() {
    assert_eq!(Format::from_settings_str("png").extension(), "png");
}

#[test]
fn normalize_default_format_jpg_is_valid() {
    assert_eq!(Format::from_settings_str("jpg").extension(), "jpg");
}

#[test]
fn normalize_default_format_bmp_is_valid() {
    assert_eq!(Format::from_settings_str("bmp").extension(), "bmp");
}

#[test]
fn normalize_default_format_unknown_falls_back_to_png() {
    assert_eq!(Format::from_settings_str("gif").extension(), "png");
}

#[test]
fn normalize_default_format_empty_falls_back_to_png() {
    assert_eq!(Format::from_settings_str("").extension(), "png");
}

#[test]
fn normalize_default_format_case_sensitive() {
    assert_eq!(Format::from_settings_str("PNG").extension(), "png");
    assert_eq!(Format::from_settings_str("Jpg").extension(), "png");
}

// --- normalize_recording_fps tests ---

#[test]
fn normalize_recording_fps_zero_returns_30() {
    assert_eq!(Fps::from_settings(0).get(), 30);
}

#[test]
fn normalize_recording_fps_nonzero_returns_same() {
    assert_eq!(Fps::from_settings(60).get(), 60);
}

#[test]
fn normalize_recording_fps_one_is_valid() {
    assert_eq!(Fps::from_settings(1).get(), 1);
}

#[test]
fn normalize_recording_fps_max_is_accepted() {
    assert_eq!(Fps::from_settings(Fps::MAX).get(), Fps::MAX);
}

/// Previously `u32::MAX` passed straight through and only blew up later, at
/// record time. The `Fps` newtype makes an out-of-range rate unrepresentable,
/// so a hand-edited settings.json now falls back to the default instead.
#[test]
fn normalize_recording_fps_out_of_range_falls_back_to_default() {
    assert_eq!(Fps::from_settings(Fps::MAX + 1), Fps::DEFAULT);
    assert_eq!(Fps::from_settings(u32::MAX), Fps::DEFAULT);
}

#[test]
fn fps_new_rejects_out_of_range_but_from_settings_forgives() {
    assert!(Fps::new(0).is_err());
    assert!(Fps::new(Fps::MAX + 1).is_err());
    assert_eq!(Fps::new(Fps::MAX).unwrap().get(), Fps::MAX);
    // Same inputs, forgiving constructor: no error, just the default.
    assert_eq!(Fps::from_settings(0), Fps::DEFAULT);
    assert_eq!(Fps::from_settings(Fps::MAX + 1), Fps::DEFAULT);
}

// -------------------------------------------------------------------------
// normalize_default_format edge cases
// -------------------------------------------------------------------------

#[test]
fn normalize_default_format_jpeg_falls_back_to_png() {
    assert_eq!(Format::from_settings_str("jpeg").extension(), "png");
}

#[test]
fn normalize_default_format_whitespace_falls_back_to_png() {
    assert_eq!(Format::from_settings_str(" png").extension(), "png");
}

#[test]
fn normalize_default_format_tiff_falls_back_to_png() {
    assert_eq!(Format::from_settings_str("tiff").extension(), "png");
}

// -------------------------------------------------------------------------
// normalize_recording_fps edge cases
// -------------------------------------------------------------------------

#[test]
fn normalize_recording_fps_60_is_valid() {
    assert_eq!(Fps::from_settings(60).get(), 60);
}

#[test]
fn normalize_recording_fps_144_is_valid() {
    assert_eq!(Fps::from_settings(144).get(), 144);
}

// -------------------------------------------------------------------------
// Fps::MAX constant
// -------------------------------------------------------------------------

#[test]
fn max_recording_fps_is_240() {
    assert_eq!(Fps::MAX, 240);
}

// -------------------------------------------------------------------------
// wide() helper tests
// -------------------------------------------------------------------------

#[test]
fn wide_returns_null_terminated_vec() {
    let v = wide("hi");
    assert_eq!(*v.last().unwrap(), 0u16);
    assert_eq!(v.len(), 3); // 'h', 'i', '\0'
}

#[test]
fn wide_empty_string_is_single_null() {
    assert_eq!(wide(""), vec![0u16]);
}

#[test]
fn wide_ascii_chars() {
    let v = wide("AB");
    assert_eq!(v, vec![0x41, 0x42, 0]);
}

#[test]
fn wide_unicode_char() {
    let v = wide("\u{00E9}"); // e-acute
    assert_eq!(v.len(), 2); // one u16 char + null
    assert_eq!(v[0], 0x00E9);
    assert_eq!(v[1], 0);
}

#[test]
fn wide_surrogate_pair_emoji() {
    let v = wide("🦀");
    // Crab emoji U+1F980 needs a surrogate pair in UTF-16
    assert_eq!(v.len(), 3); // high surrogate + low surrogate + null
    assert_eq!(v[2], 0);
}

#[test]
fn wide_mixed_ascii_and_unicode() {
    let v = wide("Rüst");
    assert_eq!(v.len(), 5); // R, ü, s, t, null
    assert_eq!(v[0], b'R' as u16);
    assert_eq!(v[1], 0x00FC); // ü
    assert_eq!(v[4], 0);
}

#[test]
fn wide_long_string() {
    let s = "a".repeat(1000);
    let v = wide(&s);
    assert_eq!(v.len(), 1001); // 1000 chars + null
    assert_eq!(*v.last().unwrap(), 0);
}

// -------------------------------------------------------------------------
// generate_timestamp tests
// -------------------------------------------------------------------------

#[test]
fn generate_timestamp_has_correct_length() {
    let ts = generate_timestamp();
    assert_eq!(ts.len(), 25, "timestamp should be 25 chars: {ts}");
}

#[test]
fn generate_timestamp_has_two_underscores() {
    let ts = generate_timestamp();
    let underscores = ts.chars().filter(|&c| c == '_').count();
    assert_eq!(underscores, 2, "timestamp should have 2 underscores: {ts}");
}

#[test]
fn generate_timestamp_date_part_is_all_digits() {
    let ts = generate_timestamp();
    let date_part = &ts[..8];
    assert!(
        date_part.chars().all(|c| c.is_ascii_digit()),
        "date part should be all digits: {date_part}"
    );
}

#[test]
fn generate_timestamp_time_part_is_all_digits() {
    let ts = generate_timestamp();
    let time_part = &ts[9..15];
    assert!(
        time_part.chars().all(|c| c.is_ascii_digit()),
        "time part should be all digits: {time_part}"
    );
}

#[test]
fn generate_timestamp_nanos_part_is_9_digits() {
    let ts = generate_timestamp();
    let nanos_part = &ts[16..];
    assert_eq!(nanos_part.len(), 9);
    assert!(
        nanos_part.chars().all(|c| c.is_ascii_digit()),
        "nanos part should be all digits: {nanos_part}"
    );
}

#[test]
fn generate_timestamp_starts_with_20() {
    let ts = generate_timestamp();
    assert!(
        ts.starts_with("20"),
        "timestamp year should start with 20: {ts}"
    );
}

#[test]
fn generate_timestamp_two_calls_differ() {
    let ts1 = generate_timestamp();
    let ts2 = generate_timestamp();
    assert_eq!(ts1.len(), ts2.len());
}

#[test]
fn generate_timestamp_no_spaces_or_colons() {
    let ts = generate_timestamp();
    assert!(!ts.contains(' '), "timestamp should have no spaces");
    assert!(!ts.contains(':'), "timestamp should have no colons");
    assert!(!ts.contains('/'), "timestamp should have no slashes");
}

// -------------------------------------------------------------------------
// Settings derive trait tests
// -------------------------------------------------------------------------

#[test]
fn settings_clone_produces_equal_instance() {
    let s = Settings {
        screenshots_dir: PathBuf::from("/tmp/shots"),
        recordings_dir: PathBuf::from("/tmp/recs"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(60).unwrap(),
        auto_start: false,
    };
    let cloned = s.clone();
    assert_eq!(s, cloned);
}

#[test]
fn settings_partial_eq_detects_difference() {
    let s1 = Settings::default();
    let s2 = Settings {
        // Any valid non-default rate. The old value here was 999, which the
        // `Fps` newtype now makes unrepresentable.
        recording_fps: Fps::new(60).unwrap(),
        ..Settings::default()
    };
    assert_ne!(s1, s2);
}

#[test]
fn settings_debug_repr_is_non_empty() {
    let s = Settings::default();
    let debug = format!("{:?}", s);
    assert!(!debug.is_empty());
    assert!(debug.contains("Settings"));
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
