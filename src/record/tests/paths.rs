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
fn get_recording_dir_parent_is_rustshot() {
    let dir = get_recording_dir();
    let parent = dir.parent().expect("recordings dir must have a parent");
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot"),
        "parent of recordings dir should be named RustShot"
    );
}

#[test]
fn get_recording_dir_grandparent_is_pictures_or_dot() {
    let dir = get_recording_dir();
    let rustshot = dir.parent().expect("recordings must have parent");
    let grandparent = rustshot.parent().expect("RustShot must have parent");
    let name = grandparent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".");
    // Grandparent is either the OS pictures dir (some name) or "." as fallback
    // We cannot predict the exact name, but it must be non-empty
    assert!(
        !name.is_empty(),
        "grandparent directory name should not be empty, got: {name}"
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

#[test]
fn get_recording_dir_ends_with_recordings() {
    let dir = get_recording_dir();
    assert_eq!(
        dir.file_name().and_then(|n| n.to_str()),
        Some("recordings"),
        "last component of dir must be 'recordings'"
    );
}

// -------------------------------------------------------------------------
// get_recording_path tests
// -------------------------------------------------------------------------

#[test]
fn recording_output_path_has_mp4_extension() {
    let path = get_recording_path();
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
}

#[test]
fn recording_path_filename_starts_with_recording() {
    let path = get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with("recording_"), "got: {name}");
}

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
fn recording_path_two_calls_same_prefix_and_suffix() {
    let p1 = get_recording_path();
    let p2 = get_recording_path();
    let n1 = p1.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let n2 = p2.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(
        n1.starts_with("recording_") && n1.ends_with(".mp4"),
        "first path should start with 'recording_' and end with '.mp4', got: {n1}"
    );
    assert!(
        n2.starts_with("recording_") && n2.ends_with(".mp4"),
        "second path should start with 'recording_' and end with '.mp4', got: {n2}"
    );
}

#[test]
fn recording_path_filename_only_valid_chars() {
    let path = get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    for ch in name.chars() {
        assert!(
            ch.is_alphanumeric() || ch == '_' || ch == '.',
            "filename contains unexpected character '{ch}' in: {name}"
        );
    }
}

// -------------------------------------------------------------------------
// Recording path naming convention tests (TC-RECERR-006 area)
// -------------------------------------------------------------------------

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

// --- recording path consistency ---

#[test]
fn get_recording_path_parent_equals_get_recording_dir() {
    let path = get_recording_path();
    let parent = path.parent().unwrap();
    assert_eq!(parent, get_recording_dir().as_path());
}

#[test]
fn get_recording_path_filename_timestamp_is_25_chars() {
    let path = get_recording_path();
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
    // "recording_YYYYMMDD_HHMMSS_NNNNNNNNN" → stem after "recording_" is 25 chars
    let ts = &name["recording_".len()..];
    assert_eq!(ts.len(), 25);
    assert!(ts.chars().all(|c| c.is_ascii_digit() || c == '_'));
}

// =========================================================================
// get_recording_path_in_dir — additional coverage
// =========================================================================

#[test]
fn get_recording_path_in_dir_respects_custom_dir() {
    let dir = PathBuf::from("/my/custom/dir");
    let path = get_recording_path_in_dir(&dir);
    assert_eq!(path.parent().unwrap(), dir.as_path());
}

#[test]
fn get_recording_path_in_dir_always_mp4() {
    let dir = PathBuf::from("/tmp");
    let path = get_recording_path_in_dir(&dir);
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
}

#[test]
fn get_recording_path_in_dir_stem_has_recording_prefix() {
    let dir = PathBuf::from("/tmp");
    let path = get_recording_path_in_dir(&dir);
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap();
    assert!(stem.starts_with("recording_"));
}

#[test]
fn get_recording_path_in_dir_uses_subsecond_timestamp_precision() {
    let dir = PathBuf::from("/tmp");
    let path = get_recording_path_in_dir(&dir);
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap();
    let ts = &stem["recording_".len()..];
    assert_eq!(ts.len(), 25);
}

// --- get_recording_path_in_dir edge cases ---

#[test]
fn get_recording_path_in_dir_filename_contains_underscore_separator() {
    let dir = PathBuf::from("/tmp/recs");
    let path = get_recording_path_in_dir(&dir);
    let filename = path.file_stem().unwrap().to_str().unwrap();
    assert!(
        filename.contains('_'),
        "filename should contain underscore separators"
    );
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

// -------------------------------------------------------------------------
// get_recording_path / get_recording_dir additional tests
// -------------------------------------------------------------------------

#[test]
fn get_recording_path_has_mp4_extension() {
    let path = get_recording_path();
    assert_eq!(path.extension().unwrap(), "mp4");
}

#[test]
fn get_recording_path_parent_is_recording_dir() {
    let path = get_recording_path();
    let dir = get_recording_dir();
    assert_eq!(path.parent().unwrap(), dir);
}

#[test]
fn get_recording_path_in_dir_custom_dir() {
    let dir = std::path::Path::new("/tmp/my_recordings");
    let path = get_recording_path_in_dir(dir);
    assert_eq!(path.parent().unwrap(), dir);
    assert_eq!(path.extension().unwrap(), "mp4");
}

#[test]
fn get_recording_path_in_dir_stem_starts_with_recording() {
    let dir = std::path::Path::new("/tmp");
    let path = get_recording_path_in_dir(dir);
    let stem = path.file_stem().unwrap().to_str().unwrap();
    assert!(
        stem.starts_with("recording_"),
        "Expected stem starting with 'recording_', got '{stem}'"
    );
}

#[test]
fn get_recording_dir_last_component_is_recordings() {
    let dir = get_recording_dir();
    let last = dir.file_name().unwrap().to_str().unwrap();
    assert_eq!(last, "recordings");
}

// -------------------------------------------------------------------------
// get_recording_path / get_recording_path_in_dir consistency
// -------------------------------------------------------------------------

#[test]
fn get_recording_path_uses_get_recording_dir() {
    let path = get_recording_path();
    let dir = get_recording_dir();
    assert_eq!(
        path.parent().unwrap(),
        dir.as_path(),
        "get_recording_path should use get_recording_dir"
    );
}

#[test]
fn get_recording_path_in_dir_extension_is_mp4() {
    let path = get_recording_path_in_dir(Path::new("/tmp"));
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
}
