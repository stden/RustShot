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

/// `start` is the default-directory wrapper around `start_in_dir`. It must apply
/// the same region validation, and apply it *before* a capture thread, an output
/// path, or an MP4 file exist — on Windows nothing else stops a zero-sized
/// region, since the platform check passes there.
#[cfg(windows)]
#[test]
fn recording_session_start_rejects_a_zero_sized_region_before_starting_a_worker() {
    let result = RecordingSession::start(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 0,
        },
        Fps::new(30).unwrap(),
    );

    // `RecordingSession` has no Debug, so unwrap_err is unavailable.
    match result {
        Err(error) => assert_eq!(error, "Recording region must have non-zero size"),
        Ok(_) => panic!("a zero-sized region must not start a recording"),
    }
}

#[test]
fn dropping_a_failed_recording_session_logs_instead_of_panicking() {
    // `Drop` joins the worker and therefore sees its `Result`. A failed
    // recording must be logged and swallowed: panicking in a drop that runs
    // while another panic unwinds aborts the process, and the tray parent drops
    // sessions on paths where the error is already being handled elsewhere.
    let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
    let join = thread::spawn(|| Err("encoder failed".to_string()));
    let session = RecordingSession {
        stop_tx,
        join: Some(join),
        output_path: PathBuf::from("recordings/dropped.mp4"),
    };

    drop(session);
}

/// A recordings directory that cannot be created must surface as a readable
/// error from `stop`, not as a silent no-op that leaves the user believing a
/// recording was saved. The worker fails before it touches Media Foundation, so
/// this exercises the real `record_loop` entry without encoding anything.
#[cfg(windows)]
#[test]
fn recording_worker_reports_a_recordings_directory_it_cannot_create() {
    use crate::test_support::TestDir;

    let temp_root = TestDir::new("rustshot_recording_blocked");
    let blocker = temp_root.file("blocker");
    std::fs::write(&blocker, b"not a directory").expect("test fixture file");

    let session = RecordingSession::start_in_dir(
        0,
        0,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 64,
            physical_height: 64,
        },
        Fps::new(30).unwrap(),
        blocker.join("recordings"),
    )
    .unwrap_or_else(|_| panic!("the region is valid, so the worker must start"));

    let error = session
        .stop()
        .expect_err("the output directory cannot exist");
    assert!(
        error.starts_with("Failed to create recording dir: "),
        "unexpected message: {error}"
    );
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
