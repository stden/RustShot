//! Tests for recording output paths: `get_recording_dir` / `get_recording_path*`.

use crate::record::*;

// -------------------------------------------------------------------------
// get_recording_dir tests
// -------------------------------------------------------------------------

#[test]
fn get_recording_dir_is_named_recordings_under_rustshot() {
    let dir = get_recording_dir();
    assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("recordings"));
    let parent = dir.parent().unwrap();
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn get_recording_dir_is_absolute_or_dot_relative() {
    let dir = get_recording_dir();
    let is_absolute = dir.is_absolute();
    let starts_with_dot = dir.to_string_lossy().starts_with('.');
    assert!(
        is_absolute || starts_with_dot,
        "path should be absolute or start with '.', got: {}",
        dir.display()
    );
}

#[test]
fn get_recording_dir_consistent_across_calls() {
    let dir1 = get_recording_dir();
    let dir2 = get_recording_dir();
    assert_eq!(
        dir1, dir2,
        "get_recording_dir must return the same path on repeated calls"
    );
}

#[test]
fn get_recording_dir_has_at_least_three_components() {
    let dir = get_recording_dir();
    let component_count = dir.components().count();
    assert!(
        component_count >= 3,
        "path should have at least 3 components (root/pictures/RustShot/recordings), got: {}",
        component_count
    );
}

// -------------------------------------------------------------------------
// get_recording_path tests
// -------------------------------------------------------------------------

#[test]
fn recording_path_is_under_recording_dir() {
    let path = get_recording_path();
    let parent = path.parent().expect("recording path must have a parent");
    let expected_dir = get_recording_dir();
    assert_eq!(
        parent, expected_dir,
        "parent of recording path should equal get_recording_dir()"
    );
}

#[test]
fn recording_path_filename_contains_digits() {
    let path = get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let has_digits = name.chars().any(|c| c.is_ascii_digit());
    assert!(
        has_digits,
        "filename should contain timestamp digits, got: {name}"
    );
}

#[test]
fn recording_path_filename_matches_pattern() {
    let path = get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    // Pattern: recording_YYYYMMDD_HHMMSS.mp4
    let re_like = name.starts_with("recording_") && name.ends_with(".mp4");
    assert!(
        re_like,
        "filename should match recording_*.mp4, got: {name}"
    );
    // Strip prefix and suffix, check remaining is digits and underscore
    let inner = &name["recording_".len()..name.len() - ".mp4".len()];
    assert!(
        inner.chars().all(|c| c.is_ascii_digit() || c == '_'),
        "timestamp portion should be digits and underscores, got: {inner}"
    );
}

#[test]
fn recording_path_timestamp_has_expected_length() {
    let path = get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    // "recording_YYYYMMDD_HHMMSS_NNNNNNNNN.mp4" → inner is 25 chars
    let inner = &name["recording_".len()..name.len() - ".mp4".len()];
    assert_eq!(
        inner.len(),
        25,
        "timestamp should be 25 chars (YYYYMMDD_HHMMSS_NNNNNNNNN), got {} chars: {inner}",
        inner.len()
    );
}

// -------------------------------------------------------------------------
// get_recording_path_in_dir tests
// -------------------------------------------------------------------------

#[test]
fn get_recording_path_in_dir_uses_provided_directory() {
    let dir = PathBuf::from("/tmp/custom-recordings");
    let path = get_recording_path_in_dir(&dir);
    assert_eq!(path.parent(), Some(dir.as_path()));
    assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("mp4"));
}

#[test]
fn get_recording_path_in_dir_filename_starts_with_recording() {
    let dir = PathBuf::from("/tmp/custom-recordings");
    let path = get_recording_path_in_dir(&dir);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with("recording_"));
    assert!(name.ends_with(".mp4"));
}

#[test]
fn get_recording_path_in_dir_two_calls_produce_different_paths() {
    let dir = PathBuf::from("/tmp/recs");
    let p1 = get_recording_path_in_dir(&dir);
    std::thread::sleep(std::time::Duration::from_millis(1));
    let p2 = get_recording_path_in_dir(&dir);
    // Timestamps include nanoseconds, so paths should differ
    assert_ne!(p1, p2);
}
