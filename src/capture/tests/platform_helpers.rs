//! Pure regression tests for the platform capture boundary.

use crate::capture::platform::{
    gdi_bgrx_to_rgba, resolve_window_bounds, rgba_from_bgra_rows, validate_gdi_scan_lines,
    WindowBounds,
};
use std::cell::Cell;

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
