//! Tests for [`RecordingSession`], `record_loop`, and `Region` behaviour.

use crate::record::*;

#[test]
fn recording_session_start_rejects_zero_fps() {
    let result = RecordingSession::start(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
        0,
    );

    assert!(matches!(
        result,
        Err(ref error) if error == "Recording FPS must be greater than 0"
    ));
}

#[test]
fn recording_session_start_in_dir_rejects_zero_fps() {
    let result = RecordingSession::start_in_dir(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
        0,
        PathBuf::from("/tmp/custom-recordings"),
    );

    assert!(matches!(
        result,
        Err(ref error) if error == "Recording FPS must be greater than 0"
    ));
}

// -------------------------------------------------------------------------
// RecordingSession::output_path tests
// (RecordingSession::start requires real hardware; test pure path logic only)
// -------------------------------------------------------------------------

#[test]
fn recording_session_output_path_accessor_returns_internal_path() {
    let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
    let expected = PathBuf::from("recordings/test_output.mp4");
    let session = RecordingSession {
        stop_tx,
        join: None,
        output_path: expected.clone(),
    };

    assert_eq!(session.output_path(), &expected);
}

#[test]
fn recording_session_output_path_accessor_returns_mp4() {
    // We cannot call RecordingSession::start in unit tests (needs WMF/screen),
    // but we can verify that the path produced by get_recording_path()
    // — which is exactly what start() uses for output_path — has .mp4 extension.
    // This exercises the same code path as output_path() would return.
    let path = get_recording_path();
    assert_eq!(
        path.extension().and_then(|e| e.to_str()),
        Some("mp4"),
        "output_path() value must have .mp4 extension"
    );
}

#[test]
fn recording_session_output_path_is_under_recording_dir() {
    // Mirrors what output_path() returns: it is set to get_recording_path() in start()
    let path = get_recording_path();
    let parent = path.parent().expect("output path must have a parent");
    assert_eq!(
        parent,
        get_recording_dir(),
        "output_path() must be a child of get_recording_dir()"
    );
}

#[test]
fn recording_session_stop_without_join_returns_stored_output_path() {
    let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
    let expected = PathBuf::from("recordings/fallback.mp4");
    let session = RecordingSession {
        stop_tx,
        join: None,
        output_path: expected.clone(),
    };

    assert_eq!(session.stop(), Ok(expected));
}

#[test]
fn recording_session_stop_returns_path_from_recording_thread() {
    let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
    let expected = PathBuf::from("recordings/thread_result.mp4");
    let join = thread::spawn({
        let expected = expected.clone();
        move || Ok(expected)
    });
    let session = RecordingSession {
        stop_tx,
        join: Some(join),
        output_path: PathBuf::from("recordings/original.mp4"),
    };

    assert_eq!(session.stop(), Ok(expected));
}

#[test]
fn recording_session_stop_propagates_recording_thread_error() {
    let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
    let join = thread::spawn(|| Err("encoder failed".to_string()));
    let session = RecordingSession {
        stop_tx,
        join: Some(join),
        output_path: PathBuf::from("recordings/original.mp4"),
    };

    assert_eq!(session.stop(), Err("encoder failed".to_string()));
}

#[test]
fn recording_session_stop_reports_panicked_recording_thread() {
    let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
    let join = thread::spawn(|| -> Result<PathBuf, String> {
        panic!("simulated recorder panic");
    });
    let session = RecordingSession {
        stop_tx,
        join: Some(join),
        output_path: PathBuf::from("recordings/original.mp4"),
    };

    assert_eq!(session.stop(), Err("Recording thread panicked".to_string()));
}

// --- RecordingSession::start with region edge cases ---

#[test]
fn recording_session_start_zero_fps_with_large_region() {
    let result = RecordingSession::start(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
        0,
    );
    assert!(result.is_err());
}

#[cfg(not(windows))]
#[test]
fn recording_session_start_nonzero_fps_fails_on_non_windows() {
    let result = RecordingSession::start(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
        30,
    );
    assert!(result.is_err());
    // Can't use unwrap_err (no Debug on RecordingSession), so use match
    match result {
        Err(e) => assert!(e.contains("only supported on Windows")),
        Ok(_) => panic!("expected error on non-windows"),
    }
}

// -------------------------------------------------------------------------
// record_loop non-windows tests
// -------------------------------------------------------------------------

#[cfg(not(windows))]
#[test]
fn record_loop_returns_error_on_non_windows() {
    let (_, rx) = mpsc::sync_channel(1);
    let result = record_loop(
        PathBuf::from("/tmp/test.mp4"),
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
        30,
        rx,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}

#[cfg(not(windows))]
#[test]
fn record_loop_error_message_is_not_empty() {
    let (_, rx) = mpsc::sync_channel(1);
    let result = record_loop(
        PathBuf::from("/tmp/test.mp4"),
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 640,
            physical_height: 480,
        },
        60,
        rx,
    );
    assert!(!result.unwrap_err().is_empty());
}

// -------------------------------------------------------------------------
// RecordingSession start/stop on non-windows
// -------------------------------------------------------------------------

#[cfg(not(windows))]
#[test]
fn recording_session_stop_returns_error_on_non_windows() {
    let session = RecordingSession::start(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
        30,
    );
    // start itself should fail on non-windows because the thread returns Err
    // but the session is created before the thread finishes
    match session {
        Ok(s) => {
            let result = s.stop();
            assert!(result.is_err());
        }
        Err(e) => assert!(e.contains("only supported on Windows")),
    }
}

#[cfg(not(windows))]
#[test]
fn recording_session_output_path_is_mp4_even_on_non_windows() {
    let session = RecordingSession::start(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
        30,
    );
    if let Ok(s) = session {
        assert_eq!(
            s.output_path().extension().and_then(|e| e.to_str()),
            Some("mp4")
        );
        let _ = s.stop();
    }
}

// -------------------------------------------------------------------------
// RecordingSession::output_path() accessor
// -------------------------------------------------------------------------

#[test]
fn recording_session_start_in_dir_zero_fps_error_message() {
    let dir = PathBuf::from("/tmp/test-recordings");
    let result = RecordingSession::start_in_dir(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
        0,
        dir,
    );
    assert!(matches!(
        result,
        Err(ref e) if e == "Recording FPS must be greater than 0"
    ));
}

// -------------------------------------------------------------------------
// Region derive traits
// -------------------------------------------------------------------------

#[test]
fn region_clone_produces_equal_value() {
    let r = Region {
        physical_x: 10,
        physical_y: 20,
        physical_width: 100,
        physical_height: 200,
    };
    let cloned = r;
    assert_eq!(cloned.physical_x, r.physical_x);
    assert_eq!(cloned.physical_y, r.physical_y);
    assert_eq!(cloned.physical_width, r.physical_width);
    assert_eq!(cloned.physical_height, r.physical_height);
}

#[test]
fn region_debug_repr_is_non_empty() {
    let r = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1,
        physical_height: 1,
    };
    let debug = format!("{:?}", r);
    assert!(!debug.is_empty());
    assert!(debug.contains("Region"));
}
