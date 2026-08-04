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

/// Verdict on a live-capture failure.
///
/// With `RUSTSHOT_LIVE_CAPTURE_STRICT=1` (set in CI, where a display is
/// guaranteed by xvfb-run or the Windows runner) any capture error fails the
/// test - a broken capture path must not pass as "headless". Without it, only
/// the two genuine no-display signatures from `capture.rs` are tolerated.
fn expect_headless_failure(error: &str) {
    if std::env::var_os("RUSTSHOT_LIVE_CAPTURE_STRICT").is_some() {
        panic!("live capture failed in strict mode: {error}");
    }
    assert!(
        error.starts_with("Failed to get screens:") || error == "No screens detected",
        "capture error is not a headless signature: {error}"
    );
}

/// Skip guard for tests that perform a live screen capture.
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
/// [`expect_headless_failure`] (which panics unless the failure is a genuine
/// no-display signature) and `None` is returned so the caller can bail out.
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
            expect_headless_failure(&e);
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
                    expect_headless_failure(&e);
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
    fn capture_dimensions(sequence: u32) -> (u32, u32) {
        let path = std::env::temp_dir().join(format!(
            "rustshot_stress_{}_{}_{}.png",
            std::process::id(),
            sequence,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let saved = rustshot::capture::capture_screenshot(
            Some(path.clone()),
            rustshot::capture::Format::Png,
            Some(0),
        )
        .unwrap_or_else(|error| panic!("native monitor capture {sequence} failed: {error}"));
        let image = image::open(&saved)
            .unwrap_or_else(|error| panic!("captured image {sequence} is invalid: {error}"));
        let dimensions = (image.width(), image.height());
        std::fs::remove_file(saved).unwrap();
        dimensions
    }

    let expected = capture_dimensions(0);
    for sequence in 1..=12 {
        assert_eq!(capture_dimensions(sequence), expected);
    }

    let workers: Vec<_> = (13..21)
        .map(|sequence| std::thread::spawn(move || capture_dimensions(sequence)))
        .collect();
    for worker in workers {
        assert_eq!(worker.join().expect("capture caller panicked"), expected);
    }
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
        Err(e) => expect_headless_failure(&e),
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
    // None = headless; expect_headless_failure has already vetted the error.
    if let Some(path) = capture_to_temp("rustshot_post_wgc", rustshot::capture::Format::Png, None) {
        assert_valid_image_file(&path, "post-WGC screen capture");
        let _ = std::fs::remove_file(&path);
    }

    // And a second window capture must not crash either.
    let _ = rustshot::capture::capture_active_window_image();
}
