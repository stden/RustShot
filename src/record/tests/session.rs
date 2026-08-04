//! Tests for [`RecordingSession`], `record_loop`, and `Region` behaviour.

use crate::record::*;

#[test]
fn recording_session_start_in_dir_accepts_validated_fps() {
    let result = RecordingSession::start_in_dir(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 0,
        },
        Fps::new(30).unwrap(),
        PathBuf::from("/tmp/custom-recordings"),
    );

    assert!(result.is_err());
}

// -------------------------------------------------------------------------
// RecordingSession::output_path / stop tests
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

#[test]
fn recording_stop_is_requested_by_message_or_sender_disconnect() {
    let (stop_tx, stop_rx) = mpsc::sync_channel(1);
    assert!(!recording_stop_requested(&stop_rx));

    stop_tx.send(()).unwrap();
    assert!(recording_stop_requested(&stop_rx));

    let (stop_tx, stop_rx) = mpsc::sync_channel(1);
    drop(stop_tx);
    assert!(recording_stop_requested(&stop_rx));
}

#[test]
fn dropping_recording_session_waits_for_worker_shutdown() {
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_completed = completed.clone();
    let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
    let join = thread::spawn(move || {
        thread::sleep(std::time::Duration::from_millis(25));
        worker_completed.store(true, std::sync::atomic::Ordering::Release);
        Ok(PathBuf::from("recordings/drop.mp4"))
    });
    let session = RecordingSession {
        stop_tx,
        join: Some(join),
        output_path: PathBuf::from("recordings/drop.mp4"),
    };

    drop(session);

    assert!(completed.load(std::sync::atomic::Ordering::Acquire));
}

// -------------------------------------------------------------------------
// Non-windows behaviour
// -------------------------------------------------------------------------

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
        Fps::new(30).unwrap(),
    );
    assert!(result.is_err());
    // Can't use unwrap_err (no Debug on RecordingSession), so use match
    match result {
        Err(e) => assert!(e.contains("only supported on Windows")),
        Ok(_) => panic!("expected error on non-windows"),
    }
}

#[cfg(not(windows))]
#[test]
fn record_loop_returns_error_on_non_windows() {
    let (_, rx) = mpsc::sync_channel(1);
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    let dimensions = RecordingDimensions::try_from(region).unwrap();
    let result = record_loop(
        PathBuf::from("/tmp/test.mp4"),
        0,
        0,
        region,
        dimensions,
        Fps::new(30).unwrap(),
        rx,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}
