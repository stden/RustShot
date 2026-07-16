//! `run_selection_with_clipboard` and `selection_stderr_message` tests.

use super::*;

#[test]
fn run_selection_with_clipboard_copies_image() {
    let copied = Cell::new(false);
    let image = image::RgbaImage::new(2, 3);
    run_selection_with_clipboard(
        || Ok(image.clone()),
        |img| {
            copied.set(true);
            assert_eq!(img.width(), 2);
            assert_eq!(img.height(), 3);
            Ok(())
        },
    )
    .unwrap();

    assert!(copied.get());
}

#[test]
fn run_selection_with_clipboard_returns_cancelled_error() {
    let result =
        run_selection_with_clipboard(|| Err("Selection cancelled".to_string()), |_| Ok(()));
    assert_eq!(result, Err("Selection cancelled".to_string()));
}

#[test]
fn run_selection_with_clipboard_wraps_non_cancelled_errors() {
    let result = run_selection_with_clipboard(|| Err("boom".to_string()), |_| Ok(()));
    assert_eq!(result, Err("Selection error: boom".to_string()));
}

#[test]
fn run_selection_with_clipboard_wraps_clipboard_errors() {
    let image = image::RgbaImage::new(2, 3);
    let result = run_selection_with_clipboard(
        || Ok(image.clone()),
        |_| Err("clipboard failed".to_string()),
    );
    assert_eq!(result, Err("clipboard failed".to_string()));
}

#[test]
fn selection_stderr_message_omits_cancelled_errors() {
    assert_eq!(selection_stderr_message("Selection cancelled"), None);
}

#[test]
fn selection_stderr_message_returns_non_cancelled_errors() {
    assert_eq!(
        selection_stderr_message("Selection error: boom"),
        Some("Selection error: boom".to_string())
    );
}

#[test]
fn selection_stderr_message_preserves_plain_error_text() {
    assert_eq!(
        selection_stderr_message("clipboard failed"),
        Some("clipboard failed".to_string())
    );
}

#[test]
fn selection_stderr_message_preserves_empty_error_text() {
    assert_eq!(selection_stderr_message(""), Some(String::new()));
}

// --- run_selection_with_clipboard additional tests ---

#[test]
fn run_selection_with_clipboard_propagates_ok_from_clipboard() {
    let image = image::RgbaImage::new(10, 10);
    let result = run_selection_with_clipboard(|| Ok(image.clone()), |_img| Ok(()));
    assert!(result.is_ok());
}

#[test]
fn run_selection_with_clipboard_forwards_image_dimensions() {
    let image = image::RgbaImage::new(640, 480);
    run_selection_with_clipboard(
        || Ok(image.clone()),
        |img| {
            assert_eq!(img.width(), 640);
            assert_eq!(img.height(), 480);
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn run_selection_with_clipboard_selection_error_is_wrapped() {
    let result = run_selection_with_clipboard(|| Err("network error".to_string()), |_| Ok(()));
    assert_eq!(result, Err("Selection error: network error".to_string()));
}

#[test]
fn run_selection_with_clipboard_single_pixel_image() {
    let image = image::RgbaImage::new(1, 1);
    let result = run_selection_with_clipboard(|| Ok(image.clone()), |_| Ok(()));
    assert!(result.is_ok());
}

#[test]
fn run_selection_with_clipboard_error_contains_selection_error_prefix() {
    let result = run_selection_with_clipboard(|| Err("something bad".to_string()), |_| Ok(()));
    let err = result.unwrap_err();
    assert!(err.starts_with("Selection error:"));
    assert!(err.contains("something bad"));
}

#[test]
fn run_selection_with_clipboard_clipboard_error_message_is_exact() {
    let image = image::RgbaImage::new(5, 5);
    let result =
        run_selection_with_clipboard(|| Ok(image.clone()), |_| Err("no clipboard".to_string()));
    assert_eq!(result.unwrap_err(), "no clipboard");
}

#[test]
fn run_selection_with_clipboard_large_image() {
    let image = image::RgbaImage::new(1920, 1080);
    let result = run_selection_with_clipboard(
        || Ok(image.clone()),
        |img| {
            assert_eq!(img.width(), 1920);
            assert_eq!(img.height(), 1080);
            Ok(())
        },
    );
    assert!(result.is_ok());
}

#[test]
fn run_selection_with_clipboard_image_pixels_pass_through() {
    // Pixels in the image should arrive unchanged at the clipboard fn
    let mut img = image::RgbaImage::new(2, 2);
    img.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));
    let result = run_selection_with_clipboard(
        {
            let img = img.clone();
            move || Ok(img.clone())
        },
        |received| {
            assert_eq!(received.get_pixel(0, 0), &image::Rgba([1, 2, 3, 255]));
            Ok(())
        },
    );
    assert!(result.is_ok());
}

#[test]
fn selection_stderr_message_whitespace_only_returns_some() {
    assert_eq!(selection_stderr_message("  "), Some("  ".to_string()));
}

#[test]
fn run_selection_with_clipboard_cancelled_does_not_call_clipboard() {
    let clipboard_called = Cell::new(false);
    let result = run_selection_with_clipboard(
        || Err("Selection cancelled".to_string()),
        |_| {
            clipboard_called.set(true);
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(!clipboard_called.get());
}

#[test]
fn run_selection_with_clipboard_select_error_does_not_call_clipboard() {
    let clipboard_called = Cell::new(false);
    let result = run_selection_with_clipboard(
        || Err("some error".to_string()),
        |_| {
            clipboard_called.set(true);
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(!clipboard_called.get());
}

// --- selection_stderr_message edge cases ---

#[test]
fn selection_stderr_message_exact_cancelled_string() {
    // Only exact match "Selection cancelled" should return None
    assert_eq!(selection_stderr_message("Selection cancelled"), None);
    assert!(selection_stderr_message("selection cancelled").is_some()); // case sensitive
    assert!(selection_stderr_message("Selection cancelled!").is_some()); // extra char
}

// --- run_selection_with_clipboard: clipboard error preserves message ---

#[test]
fn run_selection_with_clipboard_clipboard_error_message_preserved() {
    let img = image::RgbaImage::new(1, 1);
    let result =
        run_selection_with_clipboard(|| Ok(img.clone()), |_| Err("access denied".to_string()));
    assert_eq!(result.unwrap_err(), "access denied");
}

#[test]
fn run_selection_with_clipboard_selection_error_wraps_message() {
    let result = run_selection_with_clipboard(|| Err("X11 not available".to_string()), |_| Ok(()));
    assert_eq!(result.unwrap_err(), "Selection error: X11 not available");
}

// -------------------------------------------------------------------------
// selection_stderr_message mutation killers
// -------------------------------------------------------------------------

#[test]
fn selection_stderr_message_cancelled_is_exactly_none() {
    assert!(selection_stderr_message("Selection cancelled").is_none());
}

#[test]
fn selection_stderr_message_similar_but_not_cancelled() {
    // "Selection cancelled" with extra space — should NOT be None
    assert!(selection_stderr_message("Selection cancelled ").is_some());
    assert!(selection_stderr_message("selection cancelled").is_some());
    assert!(selection_stderr_message("Cancelled").is_some());
}

// --- selection_stderr_message with long strings ---

#[test]
fn selection_stderr_message_long_error_returns_some() {
    let long_msg = "A".repeat(10000);
    let result = selection_stderr_message(&long_msg);
    assert_eq!(result, Some(long_msg));
}

#[test]
fn selection_stderr_message_empty_string_returns_some() {
    assert_eq!(selection_stderr_message(""), Some("".to_string()));
}

// --- run_selection_with_clipboard: image dimensions ---

#[test]
fn run_selection_with_clipboard_4k_image() {
    let image = image::RgbaImage::new(3840, 2160);
    let result = run_selection_with_clipboard(
        || Ok(image.clone()),
        |img| {
            assert_eq!(img.width(), 3840);
            assert_eq!(img.height(), 2160);
            Ok(())
        },
    );
    assert!(result.is_ok());
}

#[test]
fn run_selection_with_clipboard_asymmetric_image() {
    let image = image::RgbaImage::new(100, 1);
    let result = run_selection_with_clipboard(
        || Ok(image.clone()),
        |img| {
            assert_eq!(img.width(), 100);
            assert_eq!(img.height(), 1);
            Ok(())
        },
    );
    assert!(result.is_ok());
}

// -------------------------------------------------------------------------
// selection_stderr_message edge case
// -------------------------------------------------------------------------

#[test]
fn selection_stderr_message_non_cancelled_returns_exact_error() {
    let msg = selection_stderr_message("Something went wrong").unwrap();
    assert_eq!(msg, "Something went wrong");
}

// -------------------------------------------------------------------------
// selection_stderr_message — whitespace-only error
// -------------------------------------------------------------------------

#[test]
fn selection_stderr_message_whitespace_returns_some() {
    assert_eq!(selection_stderr_message("   "), Some("   ".to_string()));
}
