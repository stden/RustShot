//! Tests for the injected `capture_screenshot_with` pipeline, `bgra_to_rgba`, and active-window capture stubs.

use crate::capture::*;
use std::cell::{Cell, RefCell};

#[test]
fn capture_screenshot_with_saves_to_explicit_path() {
    let captured = Cell::new(false);
    let ensured = Cell::new(false);
    let saved = Cell::new(false);
    let output = PathBuf::from("custom.jpg");

    let result = capture_screenshot_with(
        2,
        Some(1),
        || {
            captured.set(true);
            Ok("fake-image")
        },
        Some(output.clone()),
        Format::Jpg,
        &PathBuf::from("."),
        |path| {
            ensured.set(true);
            assert_eq!(path, output.as_path());
            Ok(())
        },
        |image, path, image_format| {
            saved.set(true);
            assert_eq!(*image, "fake-image");
            assert_eq!(path, output.as_path());
            assert_eq!(image_format, ImageFormat::Jpeg);
            Ok(())
        },
    )
    .unwrap();

    assert!(captured.get());
    assert!(ensured.get());
    assert!(saved.get());
    assert_eq!(result, output);
}

#[test]
fn capture_screenshot_with_reports_save_error() {
    let result = capture_screenshot_with(
        1,
        None,
        || Ok(()),
        Some(PathBuf::from("out.png")),
        Format::Png,
        &PathBuf::from("."),
        |_| Ok(()),
        |_, _, _| Err("save failed".to_string()),
    );

    assert_eq!(result, Err("save failed".to_string()));
}

#[test]
fn capture_screenshot_with_reports_invalid_screen_index() {
    let capture_called = Cell::new(false);
    let result = capture_screenshot_with(
        1,
        Some(5),
        || {
            capture_called.set(true);
            Ok(())
        },
        None,
        Format::Bmp,
        &PathBuf::from("."),
        |_| Ok(()),
        |_, _, _| Ok(()),
    );

    assert_eq!(
        result,
        Err("Screen index 5 is out of range (0-0)".to_string())
    );
    assert!(!capture_called.get());
}

#[test]
fn capture_screenshot_with_reports_no_screens_detected() {
    let capture_called = Cell::new(false);
    let result = capture_screenshot_with(
        0,
        None,
        || {
            capture_called.set(true);
            Ok(())
        },
        None,
        Format::Png,
        &PathBuf::from("."),
        |_| Ok(()),
        |_, _, _| Ok(()),
    );

    assert_eq!(result, Err("No screens detected".to_string()));
    assert!(!capture_called.get());
}

#[test]
fn capture_screenshot_with_uses_default_path_when_output_is_none() {
    let ensured_path = RefCell::new(None::<PathBuf>);
    let saved_path = RefCell::new(None::<PathBuf>);
    let screenshots_dir = PathBuf::from("custom-default-shots");

    let result = capture_screenshot_with(
        1,
        None,
        || Ok(()),
        None,
        Format::Bmp,
        &screenshots_dir,
        |path| {
            ensured_path.replace(Some(path.to_path_buf()));
            Ok(())
        },
        |_, path, image_format| {
            saved_path.replace(Some(path.to_path_buf()));
            assert_eq!(image_format, ImageFormat::Bmp);
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(ensured_path.borrow().as_ref(), Some(&result));
    assert_eq!(saved_path.borrow().as_ref(), Some(&result));
    assert_eq!(result.extension().and_then(|ext| ext.to_str()), Some("bmp"));
    assert_eq!(result.parent(), Some(screenshots_dir.as_path()));
    assert!(result
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .starts_with("screenshot_"));
}

#[test]
fn bgra_to_rgba_double_swap_is_identity() {
    let original = vec![10, 20, 30, 40, 50, 60, 70, 80];
    let mut pixels = original.clone();
    bgra_to_rgba(&mut pixels);
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, original);
}

// --- capture_active_window_platform tests ---

#[cfg(not(windows))]
#[test]
fn capture_active_window_returns_unsupported_on_non_windows() {
    let result = capture_active_window_image();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "Active window capture is only supported on Windows"
    );
}

#[cfg(not(windows))]
#[test]
fn capture_active_window_save_returns_unsupported_on_non_windows() {
    let result = capture_active_window(None, Format::Png);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}
