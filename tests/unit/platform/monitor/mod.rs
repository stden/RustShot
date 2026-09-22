//! Tests for the persistent monitor-capture sources.
//!
//! The GDI path is the fallback for machines where Windows Graphics Capture is
//! unavailable, and for the case where a WGC session stops delivering frames.
//! It therefore has to be verified on its own: a fallback that only runs once
//! the primary path is already broken is the last place a bug should hide, and
//! nothing else in the suite reaches it while WGC is healthy.

use super::*;

/// A live monitor handle, or `None` on a machine without a usable display.
fn any_monitor() -> Option<isize> {
    Some(
        screenshots::Screen::all()
            .ok()?
            .first()?
            .display_info
            .raw_handle
            .0,
    )
}

fn region(width: u32, height: u32) -> Region {
    Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: width,
        physical_height: height,
    }
}

#[test]
fn gdi_capture_rejects_a_monitor_handle_that_is_not_live() {
    // A monitor can be unplugged between enumeration and capture. The fallback
    // must name the handle it could not find rather than silently capturing a
    // different screen.
    // `GdiRegionCapture` has no `Debug`, so the error is taken rather than
    // unwrapped through `expect_err`.
    let error = GdiRegionCapture::new(isize::MAX, region(8, 8))
        .err()
        .expect("isize::MAX cannot be a live monitor handle");
    assert!(
        error.contains("was not found"),
        "unexpected message: {error}"
    );

    let error = GdiRegionCapture::new_full(isize::MAX)
        .err()
        .expect("isize::MAX cannot be a live monitor handle");
    assert!(
        error.contains("was not found"),
        "unexpected message: {error}"
    );
}

#[test]
fn gdi_region_capture_writes_one_frame_per_request() {
    let Some(monitor) = any_monitor() else {
        eprintln!("skipping GDI region capture: no display available");
        return;
    };

    let requested = region(32, 16);
    let mut capture =
        GdiRegionCapture::new(monitor, requested).expect("a live monitor must be capturable");
    assert_eq!(capture.region(), requested);

    // The region source re-captures on every request rather than caching, so a
    // second call must succeed exactly like the first — a recording asks for one
    // frame per interval for as long as it runs.
    let mut output = vec![0u8; 32 * 16 * 4];
    capture
        .capture_into(32, 16, &mut output)
        .expect("the first GDI region frame must be captured");
    capture
        .capture_into(32, 16, &mut output)
        .expect("the second GDI region frame must be captured");
}

#[test]
fn gdi_full_monitor_capture_reuses_its_first_frame_then_recaptures() {
    let Some(monitor) = any_monitor() else {
        eprintln!("skipping GDI monitor capture: no display available");
        return;
    };

    // `new_full` captures immediately so the caller can learn the monitor's real
    // pixel size, and hands that first frame to the first request instead of
    // capturing the screen twice.
    let mut capture =
        GdiRegionCapture::new_full(monitor).expect("a live monitor must be capturable");
    let size = capture.region();
    assert!(size.physical_width > 0 && size.physical_height > 0);
    assert_eq!(size.physical_x, 0);
    assert_eq!(size.physical_y, 0);

    let mut output =
        vec![0u8; (size.physical_width as usize) * (size.physical_height as usize) * 4];
    capture
        .capture_into(size.physical_width, size.physical_height, &mut output)
        .expect("the pending first frame must satisfy the first request");
    // The pending frame is consumed, so this one goes back to the screen.
    capture
        .capture_into(size.physical_width, size.physical_height, &mut output)
        .expect("later requests must re-capture the monitor");
}

#[test]
fn gdi_capture_rejects_an_output_buffer_smaller_than_the_frame() {
    let Some(monitor) = any_monitor() else {
        eprintln!("skipping GDI buffer check: no display available");
        return;
    };

    let mut capture =
        GdiRegionCapture::new(monitor, region(32, 16)).expect("a live monitor must be capturable");

    // The encoder sizes this buffer from the validated recording dimensions, so
    // a mismatch means the two disagree; copying anyway would write past the end
    // of the encoder's input.
    let mut too_small = vec![0u8; 32 * 16 * 4 - 1];
    assert!(capture.capture_into(32, 16, &mut too_small).is_err());
}

// ---------------------------------------------------------------------------
// Backend selection and Graphics Capture recovery
//
// This is the policy that decides what a recording does when its capture
// session dies: restart Graphics Capture a bounded number of times, then fall
// back to GDI for the rest of the recording rather than aborting. Waiting for a
// real device loss to exercise it is not practical, so the recovery entry point
// is driven directly.
// ---------------------------------------------------------------------------

#[test]
fn backend_selection_reports_both_failures_when_neither_backend_starts() {
    // A handle that is not a live monitor fails Graphics Capture *and* the GDI
    // fallback. The user has to see both reasons: "WGC failed" alone would send
    // them looking at the wrong subsystem.
    let error = RecordingFrameSource::new_for_monitor(
        SendableHmonitor::from_raw(isize::MAX),
        region(16, 16),
        false,
    )
    .err()
    .expect("isize::MAX cannot be a live monitor handle");

    assert!(
        error.starts_with("Windows Graphics Capture failed ("),
        "unexpected message: {error}"
    );
    assert!(
        error.contains("GDI fallback also failed:"),
        "unexpected message: {error}"
    );
}

#[test]
fn a_gdi_backend_reports_every_frame_as_fresh() {
    let Some(monitor) = any_monitor() else {
        eprintln!("skipping GDI backend dispatch: no display available");
        return;
    };

    // Once a recording has fallen back to GDI there is no compositor to report
    // "no new frame", so every request must answer `true`. Answering `false`
    // would make the encoder reuse one frame for the whole recording.
    let requested = region(32, 16);
    let mut source = RecordingFrameSource {
        backend: RecordingFrameBackend::Gdi(
            GdiRegionCapture::new(monitor, requested).expect("a live monitor must be capturable"),
        ),
        raw_monitor: SendableHmonitor::from_raw(monitor),
        region: requested,
        full_monitor: false,
        wgc_restarts: 0,
    };

    let mut output = vec![0u8; 32 * 16 * 4];
    assert_eq!(
        source.capture_into(32, 16, &mut output, Duration::ZERO),
        Ok(true)
    );
    assert_eq!(source.dimensions(), (32, 16));
}

#[test]
fn graphics_capture_recovery_restarts_the_session_within_its_budget() {
    let Some(monitor) = any_monitor() else {
        eprintln!("skipping WGC recovery: no display available");
        return;
    };

    let raw_monitor = SendableHmonitor::from_raw(monitor);
    let Ok(mut source) = RecordingFrameSource::new_for_monitor(raw_monitor, region(32, 16), false)
    else {
        eprintln!("skipping WGC recovery: Graphics Capture is unavailable here");
        return;
    };

    // Recovery reports "no new frame this tick" rather than an error, so the
    // encoder keeps its timeline and reuses the previous frame.
    assert_eq!(
        source.recover_wgc("simulated device loss".to_string()),
        Ok(false)
    );
    assert_eq!(source.wgc_restarts, 1);

    // The restart budget is spent per session, not per recording: once it is
    // exhausted the source switches to GDI for good instead of retrying forever.
    source.wgc_restarts = MAX_WGC_RESTARTS;
    assert_eq!(source.recover_wgc("still broken".to_string()), Ok(false));
    assert!(
        matches!(source.backend, RecordingFrameBackend::Gdi(_)),
        "an exhausted restart budget must leave a persistent GDI backend"
    );

    // And the GDI backend then serves frames, so the recording continues.
    let mut output = vec![0u8; 32 * 16 * 4];
    let (width, height) = source.dimensions();
    assert_eq!(
        source.capture_into(width, height, &mut output, Duration::ZERO),
        Ok(true)
    );
}

#[test]
fn graphics_capture_recovery_reports_both_failures_for_a_dead_monitor() {
    let Some(monitor) = any_monitor() else {
        eprintln!("skipping WGC recovery failure: no display available");
        return;
    };

    let requested = region(32, 16);
    let mut source = RecordingFrameSource {
        backend: RecordingFrameBackend::Gdi(
            GdiRegionCapture::new(monitor, requested).expect("a live monitor must be capturable"),
        ),
        // The monitor the source will try to recover on no longer exists, so
        // both the restart and the fallback have to fail.
        raw_monitor: SendableHmonitor::from_raw(isize::MAX),
        region: requested,
        full_monitor: false,
        wgc_restarts: 0,
    };

    let error = source
        .recover_wgc("device removed".to_string())
        .expect_err("neither backend can start on a dead monitor");
    assert!(
        error.starts_with("Windows Graphics Capture could not recover (device removed)"),
        "unexpected message: {error}"
    );
    assert!(
        error.contains("GDI fallback failed:"),
        "unexpected message: {error}"
    );
}

#[test]
fn a_disconnected_monitor_capture_worker_is_dropped_from_the_cache() {
    // The cached worker handle must not outlive its thread: without the reset,
    // one dead worker would make every later screenshot in the process fail
    // against the same closed channel.
    let (sender, receiver) = mpsc::sync_channel(1);
    drop(receiver);
    *MONITOR_CAPTURE_WORKER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(sender);

    let error = capture_monitor_image(0).expect_err("a closed worker channel cannot serve");
    assert_eq!(error, "Monitor capture worker stopped unexpectedly");
    assert!(
        MONITOR_CAPTURE_WORKER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_none(),
        "the dead worker handle must be forgotten so the next call starts a fresh one"
    );
}
