//! Typed capture requests pin source identity, output ordering, and exact errors.

use super::*;
use std::cell::{Cell, RefCell};

fn window_request() -> CaptureRequest {
    CaptureRequest::window(
        Ok(WindowTarget::new(17, 101, 201)),
        CaptureFile {
            format: Format::Jpg,
            directory: PathBuf::from("shots"),
        },
    )
}

#[test]
fn selection_delivers_one_owned_capture_by_reference_only_to_clipboard() {
    let image = image::RgbaImage::from_pixel(2, 3, image::Rgba([1, 2, 3, 255]));
    let pixels = image.as_raw().as_ptr();
    let stages = RefCell::new(Vec::new());

    let result = execute_capture_with(
        CaptureRequest::selection(),
        |source| {
            assert!(matches!(source, CaptureSource::Selection));
            stages.borrow_mut().push("capture");
            Ok(image)
        },
        |received| {
            stages.borrow_mut().push("clipboard");
            assert_eq!(received.as_raw().as_ptr(), pixels);
            assert_eq!(received.dimensions(), (2, 3));
            assert_eq!(received.get_pixel(0, 0), &image::Rgba([1, 2, 3, 255]));
            Ok(())
        },
        |_, _| panic!("selection must not invoke file delivery"),
    );

    assert_eq!(result, Ok(CaptureOutcome::Clipboard));
    assert_eq!(*stages.borrow(), ["capture", "clipboard"]);
}

#[test]
fn selection_capture_errors_keep_exact_cancellation_and_skip_delivery() {
    for (raw, expected) in [
        ("Selection cancelled", "Selection cancelled"),
        (
            "selection cancelled",
            "Selection error: selection cancelled",
        ),
        ("overlay failed", "Selection error: overlay failed"),
    ] {
        let result = execute_capture_with(
            CaptureRequest::selection(),
            |_| Err(raw.to_string()),
            |_| panic!("a failed capture must not touch the clipboard"),
            |_, _| panic!("a failed capture must not save a file"),
        );
        assert_eq!(result, Err(expected.to_string()));
    }
}

#[test]
fn selection_preserves_the_clipboard_error_without_a_file_attempt() {
    let result = execute_capture_with(
        CaptureRequest::selection(),
        |_| Ok(image::RgbaImage::new(1, 1)),
        |_| Err("clipboard unavailable".to_string()),
        |_, _| panic!("clipboard-only requests must not save on failure"),
    );
    assert_eq!(result, Err("clipboard unavailable".to_string()));
}

#[test]
fn window_delivers_same_pixels_in_order_with_the_owned_file_policy() {
    let image = image::RgbaImage::from_pixel(2, 1, image::Rgba([10, 20, 30, 255]));
    let pixels = image.as_raw().as_ptr();
    let stages = RefCell::new(Vec::new());
    let target = WindowTarget::new(23, 102, 202);
    let expected_target = target.clone();
    let request = CaptureRequest::window(
        Ok(target),
        CaptureFile {
            format: Format::Jpg,
            directory: PathBuf::from("chosen-shots"),
        },
    );

    let result = execute_capture_with(
        request,
        |source| {
            let CaptureSource::Window(snapshot) = source else {
                panic!("window request lost its callback-time source");
            };
            assert_eq!(snapshot.unwrap(), expected_target);
            stages.borrow_mut().push("capture");
            Ok(image)
        },
        |received| {
            stages.borrow_mut().push("clipboard");
            assert_eq!(received.as_raw().as_ptr(), pixels);
            assert_eq!(received.get_pixel(0, 0), &image::Rgba([10, 20, 30, 255]));
            Ok(())
        },
        |received, file| {
            stages.borrow_mut().push("save");
            assert_eq!(received.as_raw().as_ptr(), pixels);
            assert_eq!(file.format, Format::Jpg);
            assert_eq!(file.directory, PathBuf::from("chosen-shots"));
            Ok(file.directory.join("window.jpg"))
        },
    );

    assert_eq!(
        result,
        Ok(CaptureOutcome::ClipboardAndFile(
            PathBuf::from("chosen-shots").join("window.jpg")
        ))
    );
    assert_eq!(*stages.borrow(), ["capture", "clipboard", "save"]);
}

#[test]
fn window_capture_errors_preserve_context_and_skip_both_outputs() {
    let result = execute_capture_with(
        window_request(),
        |_| Err("window closed".to_string()),
        |_| panic!("a failed capture must not touch the clipboard"),
        |_, _| panic!("a failed capture must not save a file"),
    );
    assert_eq!(
        result,
        Err("Failed to capture active window: window closed".to_string())
    );
}

#[test]
fn window_request_keeps_the_callback_error_instead_of_resampling_a_target() {
    let request = CaptureRequest::window(
        Err("No foreground window when Alt+PrintScreen was pressed".to_string()),
        CaptureFile {
            format: Format::Png,
            directory: PathBuf::from("shots"),
        },
    );
    let native_capture_calls = Cell::new(0);
    let result = execute_capture_with(
        request,
        |source| {
            let CaptureSource::Window(snapshot) = source else {
                panic!("window request lost its callback-time source");
            };
            snapshot.map(|_| {
                native_capture_calls.set(native_capture_calls.get() + 1);
                image::RgbaImage::new(1, 1)
            })
        },
        |_| panic!("a failed snapshot must not touch the clipboard"),
        |_, _| panic!("a failed snapshot must not save a file"),
    );
    assert_eq!(native_capture_calls.get(), 0);
    assert_eq!(
        result,
        Err("Failed to capture active window: No foreground window when Alt+PrintScreen was pressed"
            .to_string())
    );
}

fn assert_window_delivery_outcome(
    clipboard_result: Result<(), String>,
    save_result: Result<PathBuf, String>,
    expected: Result<PathBuf, String>,
) {
    let stages = RefCell::new(Vec::new());
    let result = execute_capture_with(
        window_request(),
        |_| Ok(image::RgbaImage::new(1, 1)),
        |_| {
            stages.borrow_mut().push("clipboard");
            clipboard_result.clone()
        },
        |_, _| {
            stages.borrow_mut().push("save");
            save_result.clone()
        },
    );
    assert_eq!(
        result,
        expected.clone().map(CaptureOutcome::ClipboardAndFile)
    );
    assert_eq!(*stages.borrow(), ["clipboard", "save"]);

    // The public adapter retains its original signature and exact results.
    let legacy = benchmark_capture_active_window_outputs_with_handlers(
        || Ok(image::RgbaImage::new(1, 1)),
        |_| clipboard_result,
        |_| save_result,
    );
    assert_eq!(legacy, expected);
}

#[test]
fn window_delivery_preserves_all_four_exact_partial_success_outcomes() {
    let path = PathBuf::from("window.png");
    assert_window_delivery_outcome(Ok(()), Ok(path.clone()), Ok(path.clone()));
    assert_window_delivery_outcome(
        Err("clipboard failed".to_string()),
        Ok(path),
        Err("Window screenshot saved to window.png, but clipboard delivery failed: clipboard failed"
            .to_string()),
    );
    assert_window_delivery_outcome(
        Ok(()),
        Err("disk full".to_string()),
        Err("Window screenshot copied to clipboard, but file save failed: disk full".to_string()),
    );
    assert_window_delivery_outcome(
        Err("clipboard failed".to_string()),
        Err("disk full".to_string()),
        Err("Window screenshot clipboard delivery failed: clipboard failed; file save also failed: disk full"
            .to_string()),
    );
}
