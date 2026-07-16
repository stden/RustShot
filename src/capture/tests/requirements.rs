//! Requirement-tagged tests covering pipeline short-circuits, filename patterns, and exact error messages.

use crate::capture::*;
use std::cell::{Cell, RefCell};

// --- capture_screenshot_with: capture error prevents ensure_dir (Req 1.7) ---

#[test]
fn capture_screenshot_with_capture_error_prevents_ensure_dir() {
    // When capture fails, ensure_dir should never be called
    let ensure_called = Cell::new(false);
    let _ = capture_screenshot_with(
        1,
        None,
        || Err::<(), _>("hardware failure".to_string()),
        Some(PathBuf::from("out.png")),
        Format::Png,
        &PathBuf::from("."),
        |_| {
            ensure_called.set(true);
            Ok(())
        },
        |_: &(), _, _| Ok(()),
    );
    assert!(
        !ensure_called.get(),
        "ensure_dir should not be called when capture fails"
    );
}

// --- capture_screenshot_with: directory error prevents save (Req 2.4) ---

#[test]
fn capture_screenshot_with_dir_error_prevents_save_and_returns_error() {
    let save_called = Cell::new(false);
    let result = capture_screenshot_with(
        2,
        Some(0),
        || Ok("image-data"),
        Some(PathBuf::from("nested/shot.png")),
        Format::Png,
        &PathBuf::from("."),
        |_| Err("permission denied".to_string()),
        |_, _, _| {
            save_called.set(true);
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("permission denied"));
    assert!(
        !save_called.get(),
        "save should not be called when ensure_dir fails"
    );
}

// --- resolve_output_path: generated filename pattern (Req 2.2, 2.6) ---

#[test]
fn resolve_output_path_generated_filename_matches_pattern() {
    // Requirement 2.2: screenshot_YYYYMMDD_HHMMSS_NNNNNNNNN.<ext>
    // Requirement 2.6: only alphanumeric, underscores, and dots
    let path = resolve_output_path(None, Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap();

    // Must start with "screenshot_"
    assert!(name.starts_with("screenshot_"));

    // Strip prefix and extension to get the timestamp part
    let stem = path.file_stem().and_then(|n| n.to_str()).unwrap();
    let timestamp = &stem["screenshot_".len()..];

    // Timestamp format: YYYYMMDD_HHMMSS_NNNNNNNNN (25 chars)
    assert_eq!(
        timestamp.len(),
        25,
        "timestamp should be 25 chars: {timestamp}"
    );

    // Verify only digits and underscores in timestamp
    assert!(
        timestamp.chars().all(|c| c.is_ascii_digit() || c == '_'),
        "timestamp should contain only digits and underscores: {timestamp}"
    );

    // Verify underscore positions: index 8 and 15
    let chars: Vec<char> = timestamp.chars().collect();
    assert_eq!(chars[8], '_', "underscore at position 8");
    assert_eq!(chars[15], '_', "underscore at position 15");
}

// --- resolve_output_path: explicit path verbatim (Req 2.3) ---

#[test]
fn resolve_output_path_explicit_path_with_spaces_returned_verbatim() {
    // Requirement 2.3: explicit path returned verbatim
    let explicit = PathBuf::from("path with spaces/my screenshot.png");
    let result = resolve_output_path(Some(explicit.clone()), Format::Jpg);
    assert_eq!(result, explicit);
}

// --- ensure_parent_directory: idempotency with nested (Req 2.4, 2.5) ---

#[test]
fn ensure_parent_directory_idempotent_three_calls() {
    // Requirement 2.5: idempotent — calling multiple times always succeeds
    let dir = std::env::temp_dir().join(format!(
        "rustshot-idem3-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file_path = dir.join("a").join("b").join("shot.png");

    ensure_parent_directory(&file_path).expect("first call");
    ensure_parent_directory(&file_path).expect("second call");
    ensure_parent_directory(&file_path).expect("third call");
    assert!(file_path.parent().unwrap().exists());

    let _ = std::fs::remove_dir_all(&dir);
}

// --- resolve_screen_index: error message format (Req 1.7) ---

#[test]
fn resolve_screen_index_error_contains_requested_index_and_range() {
    // Requirement 1.7: error contains requested index and valid range
    let err = resolve_screen_index(4, Some(10)).unwrap_err();
    assert!(
        err.contains("10"),
        "error should contain requested index 10: {err}"
    );
    assert!(
        err.contains("0-3"),
        "error should contain valid range 0-3: {err}"
    );
}

// --- resolve_screen_index: "No screens detected" exact message (Req 1.8) ---

#[test]
fn resolve_screen_index_no_screens_exact_message() {
    // Requirement 1.8: exact error message
    assert_eq!(
        resolve_screen_index(0, None).unwrap_err(),
        "No screens detected"
    );
    assert_eq!(
        resolve_screen_index(0, Some(0)).unwrap_err(),
        "No screens detected"
    );
    assert_eq!(
        resolve_screen_index(0, Some(99)).unwrap_err(),
        "No screens detected"
    );
}

// --- capture_screenshot_with: all three formats pass correct ImageFormat ---

#[test]
fn capture_screenshot_with_all_formats_pass_correct_image_format() {
    // Verify each Format variant passes the correct ImageFormat to save closure
    for (fmt, expected_if) in [
        (Format::Png, ImageFormat::Png),
        (Format::Jpg, ImageFormat::Jpeg),
        (Format::Bmp, ImageFormat::Bmp),
    ] {
        let observed = RefCell::new(None::<ImageFormat>);
        let _ = capture_screenshot_with(
            1,
            None,
            || Ok(()),
            Some(PathBuf::from("test.out")),
            fmt,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, image_format| {
                observed.replace(Some(image_format));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            *observed.borrow(),
            Some(expected_if),
            "Format::{fmt:?} should pass {expected_if:?}"
        );
    }
}

// --- resolve_output_path: default directory is Pictures/RustShot (Req 2.1) ---

#[test]
fn resolve_output_path_default_directory_ends_with_rustshot() {
    // Requirement 2.1: default directory is Pictures/RustShot/
    let path = resolve_output_path(None, Format::Png);
    let parent = path.parent().unwrap();
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}
