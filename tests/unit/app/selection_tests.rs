//! `run_selection_with_clipboard` and `selection_stderr_message` tests.

use super::*;

#[test]
fn run_selection_with_clipboard_forwards_the_selected_image_once() {
    let call_count = Cell::new(0);
    let mut image = image::RgbaImage::new(2, 3);
    image.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));

    run_selection_with_clipboard(
        |source| {
            assert!(matches!(
                source,
                crate::application::capture::CaptureSource::Selection
            ));
            Ok(image.clone())
        },
        |received| {
            call_count.set(call_count.get() + 1);
            assert_eq!(received.dimensions(), (2, 3));
            assert_eq!(received.get_pixel(0, 0), &image::Rgba([1, 2, 3, 255]));
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(call_count.get(), 1);
}

#[test]
fn run_selection_with_clipboard_preserves_clipboard_errors() {
    let image = image::RgbaImage::new(2, 3);
    let result = run_selection_with_clipboard(
        |_| Ok(image.clone()),
        |_| Err("clipboard failed".to_string()),
    );
    assert_eq!(result, Err("clipboard failed".to_string()));
}

#[test]
fn run_selection_with_clipboard_cancelled_does_not_call_clipboard() {
    let clipboard_called = Cell::new(false);
    let result = run_selection_with_clipboard(
        |_| Err("Selection cancelled".to_string()),
        |_| {
            clipboard_called.set(true);
            Ok(())
        },
    );
    assert_eq!(result, Err("Selection cancelled".to_string()));
    assert!(!clipboard_called.get());
}

#[test]
fn run_selection_with_clipboard_wraps_selection_error_and_skips_clipboard() {
    let clipboard_called = Cell::new(false);
    let result = run_selection_with_clipboard(
        |_| Err("some error".to_string()),
        |_| {
            clipboard_called.set(true);
            Ok(())
        },
    );
    assert_eq!(result, Err("Selection error: some error".to_string()));
    assert!(!clipboard_called.get());
}

#[test]
fn selection_stderr_message_only_suppresses_exact_cancellation() {
    assert_eq!(selection_stderr_message("Selection cancelled"), None);

    for message in [
        "selection cancelled",
        "Selection cancelled!",
        "Selection cancelled ",
        "Cancelled",
        "",
        "  ",
        "Something went wrong",
    ] {
        assert_eq!(
            selection_stderr_message(message),
            Some(message.to_string()),
            "message should be preserved: {message:?}"
        );
    }
}
