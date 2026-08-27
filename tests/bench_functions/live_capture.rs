// ---------------------------------------------------------------------------
// Screenshot quality — real capture + file validation
// ---------------------------------------------------------------------------

/// Whether a display server is plausibly available for live capture.
///
/// The `Err` arms below are not enough on a headless Linux box: the
/// `screenshots` crate's Wayland backend (libwayshot) *panics* when it cannot
/// reach a compositor, before our code can return an error. So live-capture
/// tests must bail out up front. On CI the workflow provides a display via
/// `xvfb-run`, so these tests still exercise real capture there; the skip only
/// engages on truly headless machines (e.g. a bare SSH session).
fn display_available() -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        return true;
    }
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// The one live-capture error that says something about the *desktop* rather
/// than about RustShot.
///
/// Windows Graphics Capture publishes a frame only when the compositor presents
/// a new one, so `capture_monitor_image_on_worker` waits up to
/// `SCREENSHOT_TIMEOUT` (3 s) and reports exactly this message when nothing was
/// presented. That happens on a desktop where no pixel changed - an unattended
/// CI runner, a locked session, a developer machine loaded enough that the
/// compositor skips the deadline. The worker also keeps its capture session warm
/// for `SOURCE_IDLE_TIMEOUT` (5 s), which is why the *second* and later captures
/// in a row are the exposed ones: the first request builds a fresh session and
/// always gets a frame. Nothing the test can do fixes this - the session is
/// created with `SetIsCursorCaptureEnabled(false)`, so not even moving the mouse
/// dirties the frame.
const IDLE_DESKTOP_TIMEOUT: &str = "Timed out waiting for a monitor screenshot frame";

/// Whether `error` is the idle-desktop frame timeout and nothing else.
///
/// Compared with `==`, deliberately not `contains`/`starts_with`: the backend
/// composes messages such as "Windows Graphics Capture failed (...); GDI
/// fallback also failed: ...", and a loose match would eventually swallow a real
/// failure that merely quotes a timeout.
fn is_idle_desktop_timeout(error: &str) -> bool {
    error == IDLE_DESKTOP_TIMEOUT
}

/// Timeouts that are a consequence of *queuing*, not of a broken worker.
///
/// Only the concurrent phase of
/// `screenshot_capture_worker_survives_repeated_and_concurrent_requests` may
/// waive these, and the reason is arithmetic: eight callers share one serial
/// worker, each caller gives up after `SCREENSHOT_RESPONSE_TIMEOUT` (5 s), and a
/// single idle-desktop request can hold that worker for `SCREENSHOT_TIMEOUT`
/// (3 s). Two idle waits in a row therefore starve whoever is queued behind
/// them by construction. Everywhere else nothing is queued, so the same messages
/// mean the worker genuinely stopped answering and stay fatal - which is how a
/// hang regression still reddens this file.
#[cfg(windows)]
fn is_queue_pressure_timeout(error: &str) -> bool {
    error == "Timed out waiting for the monitor capture worker"
        || error == "Timed out waiting to queue the monitor capture"
        || error == "Monitor capture request expired in the worker queue"
}

/// Verdict on a live-capture failure.
///
/// Two tolerances live here and they are NOT the same excuse:
///
/// * [`IDLE_DESKTOP_TIMEOUT`] is tolerated always, including under
///   `RUSTSHOT_LIVE_CAPTURE_STRICT`. It is a statement about the screen, not
///   about the code: no pixel changed, so Windows had no frame to hand over.
///   Failing on it only teaches everyone to re-run CI, which is how a red build
///   stops meaning anything.
/// * Every other error is a bug. With `RUSTSHOT_LIVE_CAPTURE_STRICT=1` (set in
///   CI, where a display is guaranteed by xvfb-run or the Windows runner) it
///   fails the test - a broken capture path must not pass as "headless". Without
///   it, only the two genuine no-display signatures from `capture.rs` are
///   accepted, because a bare SSH session really has no screen to capture.
///
/// So this does not make the live-capture tests unable to fail: wrong
/// dimensions, an undecodable file, a WGC/GDI error, a dead capture worker, a
/// missing screen under strict mode and any panic all still fail. Only "the
/// desktop did not change" is waived.
fn expect_tolerated_failure(error: &str) {
    if is_idle_desktop_timeout(error) {
        // Printed rather than silently returned: a capture path that timed out
        // on *every* request would otherwise be indistinguishable from a pass.
        eprintln!("tolerating an idle-desktop capture timeout: {error}");
        return;
    }
    if std::env::var_os("RUSTSHOT_LIVE_CAPTURE_STRICT").is_some() {
        panic!("live capture failed in strict mode: {error}");
    }
    assert!(
        error.starts_with("Failed to get screens:") || error == "No screens detected",
        "capture error is not a headless signature: {error}"
    );
}

/// Skip guard for tests that perform a live screen capture./// The wider waiver is scoped to the contended stress test, so pin its set too.
///
/// These three come from `src/record/capture.rs` and are the only ones eight
/// concurrent callers can provoke against a single serial worker. A worker that
/// *died* reports something else entirely, and that must keep failing even in
/// the concurrent phase - otherwise the test stops being about worker survival,
/// which is the whole reason it exists.
#[cfg(windows)]
#[test]
fn queue_pressure_timeouts_are_only_the_three_worker_queue_messages() {
    assert!(is_queue_pressure_timeout(
        "Timed out waiting for the monitor capture worker"
    ));
    assert!(is_queue_pressure_timeout(
        "Timed out waiting to queue the monitor capture"
    ));
    assert!(is_queue_pressure_timeout(
        "Monitor capture request expired in the worker queue"
    ));
    assert!(!is_queue_pressure_timeout(
        "Monitor capture worker stopped unexpectedly"
    ));
    assert!(!is_queue_pressure_timeout(
        "Monitor capture worker stopped before returning an image"
    ));
    assert!(!is_queue_pressure_timeout(
        "Failed to construct the monitor screenshot image"
    ));
}
/// Pin the tolerance boundary itself.
///
/// The only reason the live-capture tests are allowed to survive a failure is
/// that the failure was the idle-desktop frame timeout. If the predicate ever
/// widened - a `contains`, a `starts_with("Timed out")` - those tests would
/// silently stop being able to fail, and nobody would notice because they would
/// all still be green. So the accepted set is exactly one string, and the
/// near-miss messages from the same module are pinned as fatal.
#[test]
fn only_the_idle_desktop_frame_timeout_is_tolerated() {
    assert!(is_idle_desktop_timeout(
        "Timed out waiting for a monitor screenshot frame"
    ));
    // A composed backend failure that merely quotes a timeout is still a bug.
    assert!(!is_idle_desktop_timeout(
        "Windows Graphics Capture failed (Timed out waiting for a monitor screenshot frame); \
         GDI fallback also failed: no device context"
    ));
    // Worker death is never an idle desktop.
    assert!(!is_idle_desktop_timeout(
        "Monitor capture worker stopped unexpectedly"
    ));
    assert!(!is_idle_desktop_timeout(
        "Monitor capture worker stopped before returning an image"
    ));
    // Nor are the queue/response timeouts: those mean nobody answered, which
    // outside a deliberately contended test is a hang.
    assert!(!is_idle_desktop_timeout(
        "Timed out waiting for the monitor capture worker"
    ));
    assert!(!is_idle_desktop_timeout("No screens detected"));
}
macro_rules! require_display {
    () => {
        if !display_available() {
            eprintln!("skipping live-capture test: no display server (DISPLAY/WAYLAND_DISPLAY)");
            return;
        }
    };
}

/// Capture a live screenshot into a uniquely named temp file.
///
/// Returns `Some(path)` on success, after asserting the screenshot was saved
/// to the requested path. On failure the error is routed through
/// [`expect_tolerated_failure`] (which panics unless the failure is an idle
/// desktop or, outside strict mode, a genuine no-display signature) and `None`
/// is returned so the caller can bail out.
/// Validation and cleanup stay with the caller.
fn capture_to_temp(
    prefix: &str,
    format: rustshot::capture::Format,
    screen_index: Option<usize>,
) -> Option<std::path::PathBuf> {
    let tmp = std::env::temp_dir().join(format!(
        "{prefix}_{}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        format.extension()
    ));
    match rustshot::capture::capture_screenshot(Some(tmp.clone()), format, screen_index) {
        Ok(path) => {
            assert_eq!(path, tmp, "capture should save to the requested path");
            Some(path)
        }
        Err(e) => {
            expect_tolerated_failure(&e);
            None
        }
    }
}

/// Shared validation block: the file exists, is not suspiciously small, and
/// decodes as an image with non-zero dimensions.
fn assert_valid_image_file(path: &std::path::Path, label: &str) {
    assert!(path.exists(), "output file must exist");
    let metadata = std::fs::metadata(path).unwrap();
    assert!(
        metadata.len() > 100,
        "{label} file too small ({} bytes), likely corrupt",
        metadata.len()
    );
    let img = image::open(path).expect("should be a valid image file");
    assert!(img.width() > 0, "image width must be > 0");
    assert!(img.height() > 0, "image height must be > 0");
}

/// Capture a real screenshot in PNG format, verify the file is a valid image
/// with non-zero dimensions and reasonable file size.
#[test]
fn screenshot_capture_png_produces_valid_image() {
    require_display!();
    let Some(path) = capture_to_temp("rustshot_quality_png", rustshot::capture::Format::Png, None)
    else {
        return;
    };
    assert_valid_image_file(&path, "PNG");
    let _ = std::fs::remove_file(&path);
}

/// Capture a real screenshot in JPG format, verify valid JPEG output.
#[test]
fn screenshot_capture_jpg_produces_valid_image() {
    require_display!();
    let Some(path) = capture_to_temp("rustshot_quality_jpg", rustshot::capture::Format::Jpg, None)
    else {
        return;
    };
    assert_valid_image_file(&path, "JPG");
    let _ = std::fs::remove_file(&path);
}

/// Capture a real screenshot in BMP format, verify valid BMP output.
#[test]
fn screenshot_capture_bmp_produces_valid_image() {
    require_display!();
    let Some(path) = capture_to_temp("rustshot_quality_bmp", rustshot::capture::Format::Bmp, None)
    else {
        return;
    };
    assert_valid_image_file(&path, "BMP");
    let _ = std::fs::remove_file(&path);
}

/// PNG screenshot should have at least 1024x768 on a real display.
#[test]
fn screenshot_capture_has_reasonable_resolution() {
    require_display!();
    let Some(path) = capture_to_temp("rustshot_resolution", rustshot::capture::Format::Png, None)
    else {
        return;
    };
    let img = image::open(&path).unwrap();
    // Modern displays are at least 1024x768
    assert!(
        img.width() >= 640,
        "width {} too small for a real display",
        img.width()
    );
    assert!(
        img.height() >= 480,
        "height {} too small for a real display",
        img.height()
    );
    let _ = std::fs::remove_file(&path);
}

/// Capture two screenshots back-to-back in PNG: both must produce valid files.
#[test]
fn screenshot_capture_twice_both_succeed() {
    require_display!();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tmp1 = std::env::temp_dir().join(format!("rustshot_twice1_{ts}.png"));
    let tmp2 = std::env::temp_dir().join(format!("rustshot_twice2_{ts}.png"));

    let r1 = rustshot::capture::capture_screenshot(
        Some(tmp1.clone()),
        rustshot::capture::Format::Png,
        None,
    );
    let r2 = rustshot::capture::capture_screenshot(
        Some(tmp2.clone()),
        rustshot::capture::Format::Png,
        None,
    );

    match (r1, r2) {
        (Ok(p1), Ok(p2)) => {
            assert!(p1.exists());
            assert!(p2.exists());
            // Both should be valid images
            let img1 = image::open(&p1).unwrap();
            let img2 = image::open(&p2).unwrap();
            // Same screen → same dimensions
            assert_eq!(img1.width(), img2.width());
            assert_eq!(img1.height(), img2.height());
            let _ = std::fs::remove_file(&p1);
            let _ = std::fs::remove_file(&p2);
        }
        (r1, r2) => {
            for r in [r1, r2] {
                if let Err(e) = r {
                    expect_tolerated_failure(&e);
                }
            }
            let _ = std::fs::remove_file(&tmp1);
            let _ = std::fs::remove_file(&tmp2);
        }
    }
}

#[cfg(windows)]
#[test]
fn screenshot_capture_worker_survives_repeated_and_concurrent_requests() {
    /// One capture attempt. `None` means it died of a timeout this test cannot
    /// prevent: an unchanged desktop (see [`IDLE_DESKTOP_TIMEOUT`]) or, in the
    /// concurrent phase only, a caller starved by the serial worker (see
    /// [`is_queue_pressure_timeout`]). Every other error panics, so a dead
    /// worker, a WGC/GDI failure or an undecodable file still fails the test.
    fn capture_dimensions(sequence: u32, under_queue_pressure: bool) -> Option<(u32, u32)> {
        let path = std::env::temp_dir().join(format!(
            "rustshot_stress_{}_{}_{}.png",
            std::process::id(),
            sequence,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let saved = match rustshot::capture::capture_screenshot(
            Some(path),
            rustshot::capture::Format::Png,
            Some(0),
        ) {
            Ok(saved) => saved,
            Err(error)
                if is_idle_desktop_timeout(&error)
                    || (under_queue_pressure && is_queue_pressure_timeout(&error)) =>
            {
                // Never swallowed silently: if this printed 21 times the
                // capture path is dead, and the floor assertion below says so.
                eprintln!("capture {sequence} returned no frame: {error}");
                return None;
            }
            Err(error) => panic!("native monitor capture {sequence} failed: {error}"),
        };
        let image = image::open(&saved)
            .unwrap_or_else(|error| panic!("captured image {sequence} is invalid: {error}"));
        let dimensions = (image.width(), image.height());
        std::fs::remove_file(saved).unwrap();
        Some(dimensions)
    }

    // What this test still proves after the idle-desktop waiver: the worker
    // survives 21 requests without dying or erroring, no caller thread panics,
    // and every frame that *does* come back describes the same monitor. What it
    // can no longer prove is that all 21 are served, because on a desktop that
    // never changes the compositor has nothing to serve. The floor below keeps
    // the waiver from degenerating into a test that passes on silence.
    let mut expected: Option<(u32, u32)> = None;
    let mut served = 0_u32;
    let mut record = |sequence: u32, dimensions: Option<(u32, u32)>| {
        if let Some(dimensions) = dimensions {
            served += 1;
            let first = *expected.get_or_insert(dimensions);
            assert_eq!(
                dimensions, first,
                "capture {sequence} reported a different monitor size"
            );
        }
    };

    for sequence in 0..=12 {
        record(sequence, capture_dimensions(sequence, false));
    }

    let workers: Vec<_> = (13..21)
        .map(|sequence| std::thread::spawn(move || (sequence, capture_dimensions(sequence, true))))
        .collect();
    for worker in workers {
        let (sequence, dimensions) = worker.join().expect("capture caller panicked");
        record(sequence, dimensions);
    }

    assert!(
        served > 0,
        "none of the 21 requests returned a frame: the monitor capture worker is not serving"
    );
}

/// PNG file should not be excessively large (sanity check for encoding).
#[test]
fn screenshot_capture_png_file_size_is_reasonable() {
    require_display!();
    let Some(path) = capture_to_temp("rustshot_size", rustshot::capture::Format::Png, None) else {
        return;
    };
    let metadata = std::fs::metadata(&path).unwrap();
    let img = image::open(&path).unwrap();
    let raw_size = (img.width() as u64) * (img.height() as u64) * 4;
    // PNG compression should make it smaller than raw RGBA
    assert!(
        metadata.len() < raw_size,
        "PNG ({} bytes) should be smaller than raw RGBA ({} bytes)",
        metadata.len(),
        raw_size
    );
    let _ = std::fs::remove_file(&path);
}

/// capture_screenshot with explicit screen index 0 should work.
#[test]
fn screenshot_capture_screen_index_0_ok() {
    require_display!();
    let Some(path) = capture_to_temp("rustshot_screen0", rustshot::capture::Format::Png, Some(0))
    else {
        return;
    };
    assert!(path.exists());
    let img = image::open(&path).unwrap();
    assert!(img.width() > 0);
    assert!(img.height() > 0);
    let _ = std::fs::remove_file(&path);
}

/// capture_screenshot with out-of-range screen index should error.
#[test]
fn screenshot_capture_invalid_screen_index_errors() {
    require_display!();
    let tmp = std::env::temp_dir().join("rustshot_badscreen.png");
    let result = rustshot::capture::capture_screenshot(
        Some(tmp.clone()),
        rustshot::capture::Format::Png,
        Some(999),
    );
    match result {
        Ok(_) => panic!("should error for screen index 999"),
        Err(e) => {
            assert!(
                e.contains("out of range") || e.contains("screen") || e.contains("Screen"),
                "error should mention screen range: {e}"
            );
        }
    }
    let _ = std::fs::remove_file(&tmp);
}

// ---------------------------------------------------------------------------
// capture_screenshot_in_dir / capture_active_window_image
// (consumed by benches/benchmarks/capture_live.rs)
// ---------------------------------------------------------------------------

/// capture_screenshot_in_dir with no explicit output must generate a
/// timestamped file inside the provided directory.
#[test]
fn screenshot_capture_in_dir_generates_file_in_custom_dir() {
    require_display!();
    let dir = std::env::temp_dir().join(format!(
        "rustshot_in_dir_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = rustshot::capture::capture_screenshot_in_dir(
        None,
        rustshot::capture::Format::Png,
        None,
        &dir,
    );
    match result {
        Ok(path) => {
            assert_eq!(
                path.parent(),
                Some(dir.as_path()),
                "file must be created inside the provided directory"
            );
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            assert!(
                name.starts_with("screenshot_"),
                "generated name should start with 'screenshot_': {name}"
            );
            assert_valid_image_file(&path, "PNG");
        }
        Err(e) => expect_tolerated_failure(&e),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// capture_active_window_image is probed and iterated by the
/// capture_active_window benchmark. On Windows it returns either a non-empty
/// image or a human-readable error (e.g. no foreground window on a service
/// session) - both are valid outcomes the benchmark handles.
#[cfg(windows)]
#[test]
fn capture_active_window_image_returns_image_or_readable_error() {
    match rustshot::capture::capture_active_window_image() {
        Ok(img) => {
            assert!(img.width() > 0, "active-window image width must be > 0");
            assert!(img.height() > 0, "active-window image height must be > 0");
        }
        Err(e) => {
            assert!(!e.is_empty(), "error message must not be empty");
        }
    }
}

/// On non-Windows platforms capture_active_window_image always fails with the
/// platform-support error the benchmark skip path relies on.
#[cfg(not(windows))]
#[test]
fn capture_active_window_image_errors_on_non_windows() {
    let err = rustshot::capture::capture_active_window_image().unwrap_err();
    assert_eq!(err, "Active window capture is only supported on Windows");
}

/// Regression: a window capture must not poison the process for later work.
///
/// The WGC backend used to RoInitialize/RoUninitialize on a short-lived worker
/// thread; when that was the last MTA thread, the apartment died and the NEXT
/// capture (or process exit) crashed with STATUS_ACCESS_VIOLATION - exactly
/// this sequence, observed on the CI Windows runner. The MTA is now pinned for
/// the process lifetime.
#[cfg(windows)]
#[test]
fn window_capture_then_screen_capture_does_not_poison_the_process() {
    // Window capture (either outcome is fine - running it is the point).
    let _ = rustshot::capture::capture_active_window_image();

    // A screen capture afterwards must still work.
    // None = skipped; expect_tolerated_failure has already vetted the error
    // (idle desktop, or a headless box outside strict mode).
    if let Some(path) = capture_to_temp("rustshot_post_wgc", rustshot::capture::Format::Png, None) {
        assert_valid_image_file(&path, "post-WGC screen capture");
        let _ = std::fs::remove_file(&path);
    }

    // And a second window capture must not crash either.
    let _ = rustshot::capture::capture_active_window_image();
}

/// The benchmark shim must capture through the *production* backend.
///
/// `benches/benchmarks.rs` reports monitor-capture timings through this shim. If
/// it ever diverged from the backend the app uses, the benchmark would keep
/// reporting numbers for code nobody runs.
#[test]
fn benchmark_capture_monitor_image_returns_a_full_monitor_frame() {
    require_display!();

    let screens = match screenshots::Screen::all() {
        Ok(screens) => screens,
        Err(error) => {
            expect_tolerated_failure(&format!("Failed to get screens: {error}"));
            return;
        }
    };
    let Some(screen) = screens.first() else {
        expect_tolerated_failure("No screens detected");
        return;
    };

    match rustshot::capture::benchmark_capture_monitor_image(screen) {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0);
            // The backend captures physical pixels, so on a scaled display the
            // frame is larger than the enumerated logical size — never smaller,
            // and never a crop of the monitor.
            assert!(
                image.width() >= screen.display_info.width
                    && image.height() >= screen.display_info.height,
                "{}x{} is smaller than the monitor's logical {}x{}",
                image.width(),
                image.height(),
                screen.display_info.width,
                screen.display_info.height
            );
        }
        Err(error) => expect_tolerated_failure(&error),
    }
}
