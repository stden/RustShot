//! Tests for [`validate_recording_start`] (FPS range and platform gating).

use crate::record::*;

#[test]
fn validate_recording_start_rejects_zero_fps() {
    assert_eq!(
        validate_recording_start(0),
        Err("Recording FPS must be greater than 0".to_string())
    );
}

#[cfg(windows)]
#[test]
fn validate_recording_start_accepts_positive_fps_on_windows() {
    assert!(validate_recording_start(30).is_ok());
}

#[cfg(not(windows))]
#[test]
fn validate_recording_start_rejects_recording_on_non_windows() {
    assert_eq!(
        validate_recording_start(30),
        Err("Recording is only supported on Windows".to_string())
    );
}

// --- validate_recording_start edge cases ---

#[test]
fn validate_recording_start_rejects_zero_fps_error_message() {
    let err = validate_recording_start(0).unwrap_err();
    assert!(err.contains("FPS"), "error should mention FPS: {err}");
    assert!(
        err.contains("greater than 0"),
        "error should say > 0: {err}"
    );
}

#[cfg(not(windows))]
#[test]
fn validate_recording_start_rejects_1_fps_on_non_windows() {
    assert!(validate_recording_start(1).is_err());
}

#[cfg(not(windows))]
#[test]
fn validate_recording_start_rejects_high_fps_on_non_windows() {
    assert!(validate_recording_start(120).is_err());
}

// --- validate_recording_start boundary ---

#[cfg(windows)]
#[test]
fn validate_recording_start_accepts_1_fps_on_windows() {
    assert!(validate_recording_start(1).is_ok());
}

#[cfg(windows)]
#[test]
fn validate_recording_start_accepts_240_fps_on_windows() {
    assert!(validate_recording_start(240).is_ok());
}

// =========================================================================
// validate_recording_start — additional coverage
// =========================================================================

#[test]
fn validate_recording_start_zero_fps_error_message() {
    let err = validate_recording_start(0).unwrap_err();
    assert!(err.contains("greater than 0"));
}

// -------------------------------------------------------------------------
// validate_recording_start additional tests
// -------------------------------------------------------------------------

#[cfg(windows)]
#[test]
fn validate_recording_start_1fps_is_ok() {
    assert!(validate_recording_start(1).is_ok());
}

#[cfg(windows)]
#[test]
fn validate_recording_start_240fps_is_ok() {
    assert!(validate_recording_start(240).is_ok());
}

#[cfg(windows)]
#[test]
fn validate_recording_start_rejects_fps_above_max() {
    let err = validate_recording_start(Fps::MAX + 1).unwrap_err();
    assert!(
        err.contains("240"),
        "error should mention the max FPS: {err}"
    );
}

#[cfg(not(windows))]
#[test]
fn validate_recording_start_nonzero_fps_returns_unsupported_on_linux() {
    let result = validate_recording_start(30);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}

#[test]
fn validate_recording_start_zero_error_mentions_fps() {
    let err = validate_recording_start(0).unwrap_err();
    assert!(err.to_lowercase().contains("fps") || err.contains("0"));
}

#[test]
fn validate_recording_start_above_max_error_mentions_limit() {
    let err = validate_recording_start(241).unwrap_err();
    assert!(
        err.contains("240"),
        "error should mention the max FPS: {err}"
    );
}
