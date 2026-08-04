//! Tests for [`validate_recording_start`] (FPS range and platform gating).

use crate::record::*;

#[test]
fn validate_recording_start_rejects_zero_fps() {
    assert_eq!(
        validate_recording_start(0),
        Err("Recording FPS must be greater than 0".to_string())
    );
}

#[test]
fn validate_recording_start_above_max_error_mentions_limit() {
    let err = validate_recording_start(241).unwrap_err();
    assert!(
        err.contains("240"),
        "error should mention the max FPS: {err}"
    );
}

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

#[cfg(not(windows))]
#[test]
fn validate_recording_start_rejects_recording_on_non_windows() {
    assert_eq!(
        validate_recording_start(30),
        Err("Recording is only supported on Windows".to_string())
    );
}
