//! Pure regression tests for the platform capture boundary.

use crate::capture::platform::{
    frame_crop_within_window, gdi_bgrx_to_rgba, receive_capture_result, resolve_window_bounds,
    rgba_from_bgra_rows, validate_gdi_scan_lines, WindowBounds,
};
use std::cell::Cell;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// --- frame_crop_within_window: PrintWindow output -> extended-frame crop ---

#[test]
fn frame_crop_offsets_are_relative_to_the_window_rect() {
    // Window rect includes 7px invisible resize borders on the sides/bottom;
    // the DWM extended frame excludes them.
    let window = WindowBounds::new(100, 100, 900, 700);
    let frame = WindowBounds::new(107, 100, 893, 693);

    let (x, y, w, h) = frame_crop_within_window(window, frame).unwrap();

    assert_eq!((x, y), (7, 0));
    assert_eq!((w, h), (786, 593));
}

#[test]
fn frame_crop_of_identical_rects_is_the_full_window() {
    let window = WindowBounds::new(-200, 50, 600, 650);

    let (x, y, w, h) = frame_crop_within_window(window, window).unwrap();

    assert_eq!((x, y), (0, 0));
    assert_eq!((w, h), (800, 600));
}

#[test]
fn frame_crop_clamps_a_frame_that_leaks_outside_the_window() {
    // DWM occasionally reports frame bounds a pixel outside the window rect;
    // the crop must stay within the rendered surface instead of underflowing.
    let window = WindowBounds::new(10, 10, 110, 110);
    let frame = WindowBounds::new(8, 9, 112, 111);

    let (x, y, w, h) = frame_crop_within_window(window, frame).unwrap();

    assert_eq!((x, y), (0, 0));
    assert_eq!((w, h), (100, 100));
}

#[test]
fn frame_crop_rejects_a_frame_fully_outside_the_window() {
    let window = WindowBounds::new(0, 0, 100, 100);
    let frame = WindowBounds::new(200, 200, 300, 300);

    let error = frame_crop_within_window(window, frame).unwrap_err();

    assert!(error.contains("outside"));
}

#[test]
fn gdi_bgrx_conversion_makes_every_pixel_opaque() {
    let mut pixels = vec![
        10, 20, 30, 0, // BGRX with the unused byte cleared
        40, 50, 60, 17, // the unused byte must not leak as alpha
    ];

    gdi_bgrx_to_rgba(&mut pixels);

    assert_eq!(pixels, vec![30, 20, 10, 255, 60, 50, 40, 255]);
}

#[test]
fn gdi_scan_line_validation_requires_the_full_image() {
    assert!(validate_gdi_scan_lines(1080, 1080).is_ok());

    for actual in [0, 1, 1079] {
        let error = validate_gdi_scan_lines(actual, 1080).unwrap_err();
        assert!(error.contains(&actual.to_string()));
        assert!(error.contains("1080"));
    }
}

#[test]
fn window_bounds_prefer_extended_frame_without_fallback() {
    let fallback_called = Cell::new(false);
    let expected = WindowBounds::new(10, 20, 310, 220);

    let actual = resolve_window_bounds(
        || Ok(expected),
        || {
            fallback_called.set(true);
            Err("fallback must not run".to_string())
        },
    )
    .unwrap();

    assert_eq!(actual, expected);
    assert!(!fallback_called.get());
}

#[test]
fn window_bounds_fall_back_when_extended_frame_fails() {
    let expected = WindowBounds::new(-200, 50, 600, 650);

    let actual =
        resolve_window_bounds(|| Err("DWM unavailable".to_string()), || Ok(expected)).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(actual.dimensions().unwrap(), (800, 600));
}

#[test]
fn window_bounds_reject_invalid_fallback_dimensions() {
    let error = resolve_window_bounds(
        || Err("DWM unavailable".to_string()),
        || Ok(WindowBounds::new(10, 10, 10, 20)),
    )
    .unwrap_err();

    assert!(error.contains("zero or negative"));
}

#[test]
fn bgra_row_copy_ignores_gpu_padding_and_preserves_alpha() {
    let mapped = [
        10, 20, 30, 40, 50, 60, 70, 80, 0xAA, 0xAA, 0xAA, 0xAA, // row 1
        90, 100, 110, 120, 130, 140, 150, 160, 0xBB, 0xBB, 0xBB, 0xBB, // row 2
    ];

    let rgba = rgba_from_bgra_rows(&mapped, 12, 2, 2).unwrap();

    assert_eq!(
        rgba,
        vec![30, 20, 10, 40, 70, 60, 50, 80, 110, 100, 90, 120, 150, 140, 130, 160,]
    );
}

#[test]
fn bgra_row_copy_rejects_short_pitch_or_buffer() {
    let short_pitch = rgba_from_bgra_rows(&[0; 16], 7, 2, 2).unwrap_err();
    assert!(short_pitch.contains("row pitch"));

    let short_buffer = rgba_from_bgra_rows(&[0; 15], 8, 2, 2).unwrap_err();
    assert!(short_buffer.contains("buffer"));
}

#[test]
fn bgra_row_copy_rejects_zero_dimensions() {
    assert!(rgba_from_bgra_rows(&[], 0, 0, 1).is_err());
    assert!(rgba_from_bgra_rows(&[], 0, 1, 0).is_err());
}

#[test]
fn capture_worker_result_is_returned_before_the_deadline() {
    let (sender, receiver) = mpsc::channel();
    sender.send(Ok(42)).unwrap();

    assert_eq!(
        receive_capture_result(receiver, Duration::from_millis(50), "GDI").unwrap(),
        42
    );
}

#[test]
fn capture_worker_timeout_does_not_wait_for_the_worker_to_finish() {
    let (sender, receiver) = mpsc::channel::<Result<(), String>>();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        let _ = sender.send(Ok(()));
    });
    let started = Instant::now();

    let error = receive_capture_result(receiver, Duration::from_millis(20), "GDI").unwrap_err();

    assert!(error.contains("timed out"));
    assert!(started.elapsed() < Duration::from_millis(150));
    worker.join().unwrap();
}

#[test]
fn capture_worker_disconnect_is_reported() {
    let (sender, receiver) = mpsc::channel::<Result<(), String>>();
    drop(sender);

    let error = receive_capture_result(receiver, Duration::from_millis(50), "GDI").unwrap_err();

    assert!(error.contains("stopped without returning a result"));
}
