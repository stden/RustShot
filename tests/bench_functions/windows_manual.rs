// `Path` is only used by the Windows-gated live-capture tests below.
#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
fn primary_screen() -> screenshots::Screen {
    let screens = screenshots::Screen::all().expect("screen enumeration should succeed");
    screens
        .iter()
        .find(|screen| screen.display_info.is_primary)
        .copied()
        .unwrap_or(screens[0])
}

#[cfg(windows)]
fn assert_file_looks_like_mp4(path: &Path) {
    let bytes = std::fs::read(path).expect("recording file bytes should be readable");
    assert!(
        bytes.len() > 16,
        "recording file is too small: {}",
        bytes.len()
    );
    assert_eq!(
        &bytes[4..8],
        b"ftyp",
        "recording file should start with an MP4 ftyp box"
    );

    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("moov"), "recording file should contain moov");
    assert!(text.contains("mdat"), "recording file should contain mdat");
}

// ---------------------------------------------------------------------------
// Manual Windows smoke tests
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_capture_screenshot_saves_valid_png() {
    let output = std::env::temp_dir().join(format!(
        "rustshot_manual_capture_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    let saved = rustshot::capture::capture_screenshot(
        Some(output.clone()),
        rustshot::capture::Format::Png,
        None,
    )
    .expect("live screenshot capture should succeed");

    assert_eq!(saved, output);
    assert!(saved.exists(), "screenshot file should exist");

    let metadata = std::fs::metadata(&saved).expect("screenshot metadata should exist");
    assert!(metadata.len() > 0, "screenshot file should not be empty");

    let decoded = image::open(&saved).expect("saved screenshot should decode as an image");
    assert!(
        decoded.width() > 0 && decoded.height() > 0,
        "decoded screenshot should have non-zero dimensions"
    );

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_capture_screenshot_matches_primary_screen_dimensions() {
    let screens = screenshots::Screen::all().expect("screen enumeration should succeed");
    let screen = screens
        .iter()
        .find(|screen| screen.display_info.is_primary)
        .copied()
        .unwrap_or(screens[0]);
    let reference = screen
        .capture()
        .expect("direct screen capture should succeed for reference dimensions");

    let output = std::env::temp_dir().join(format!(
        "rustshot_manual_capture_dims_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    let saved = rustshot::capture::capture_screenshot(
        Some(output.clone()),
        rustshot::capture::Format::Png,
        None,
    )
    .expect("live screenshot capture should succeed");

    let decoded = image::open(&saved).expect("saved screenshot should decode as an image");
    assert_eq!(decoded.width(), reference.width());
    assert_eq!(decoded.height(), reference.height());

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_capture_active_window_saves_valid_image() {
    let output = std::env::temp_dir().join(format!(
        "rustshot_manual_active_window_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));

    let saved = rustshot::capture::capture_active_window(
        Some(output.clone()),
        rustshot::capture::Format::Png,
    )
    .expect("active-window capture should succeed");

    assert_eq!(saved, output);
    assert!(saved.exists(), "active-window screenshot file should exist");

    let decoded = image::open(&saved).expect("saved active-window screenshot should decode");
    assert!(
        decoded.width() > 0 && decoded.height() > 0,
        "active-window screenshot should have non-zero dimensions"
    );

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_recording_session_creates_non_empty_mp4() {
    use std::time::Duration;

    let screen = primary_screen();

    let width = screen.display_info.width.clamp(2, 64);
    let height = screen.display_info.height.clamp(2, 64);

    let session = rustshot::record::RecordingSession::start(
        screen.display_info.x,
        screen.display_info.y,
        rustshot::selection::Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        rustshot::settings::Fps::new(5).unwrap(),
    )
    .expect("live recording should start");

    std::thread::sleep(Duration::from_millis(700));

    let saved = session.stop().expect("live recording should stop cleanly");
    assert!(saved.exists(), "recording file should exist");

    let metadata = std::fs::metadata(&saved).expect("recording metadata should exist");
    assert!(metadata.len() > 0, "recording file should not be empty");
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("mp4"));
    assert_file_looks_like_mp4(&saved);

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_recording_session_survives_five_seconds() {
    use std::time::Duration;

    let screen = primary_screen();

    let width = screen.display_info.width.clamp(2, 128);
    let height = screen.display_info.height.clamp(2, 72);

    let session = rustshot::record::RecordingSession::start(
        screen.display_info.x,
        screen.display_info.y,
        rustshot::selection::Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        rustshot::settings::Fps::new(5).unwrap(),
    )
    .expect("5-second live recording should start");

    std::thread::sleep(Duration::from_secs(5));

    let saved = session
        .stop()
        .expect("5-second live recording should stop cleanly");
    assert!(saved.exists(), "5-second recording file should exist");

    let metadata = std::fs::metadata(&saved).expect("5-second recording metadata should exist");
    assert!(
        metadata.len() > 0,
        "5-second recording file should not be empty"
    );
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("mp4"));
    assert_file_looks_like_mp4(&saved);

    let _ = std::fs::remove_file(saved);
}

#[cfg(windows)]
#[test]
#[ignore = "requires interactive Windows desktop"]
fn manual_recording_session_with_odd_region_dimensions_creates_valid_mp4() {
    use std::time::Duration;

    let screen = primary_screen();
    let width = screen.display_info.width.clamp(3, 63);
    let height = screen.display_info.height.clamp(3, 47);

    let session = rustshot::record::RecordingSession::start(
        screen.display_info.x,
        screen.display_info.y,
        rustshot::selection::Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        rustshot::settings::Fps::new(5).unwrap(),
    )
    .expect("odd-dimension live recording should start");

    std::thread::sleep(Duration::from_millis(700));

    let saved = session
        .stop()
        .expect("odd-dimension live recording should stop cleanly");
    assert!(saved.exists(), "odd-dimension recording file should exist");

    let metadata =
        std::fs::metadata(&saved).expect("odd-dimension recording metadata should exist");
    assert!(
        metadata.len() > 0,
        "odd-dimension recording file should not be empty"
    );
    assert_eq!(saved.extension().and_then(|e| e.to_str()), Some("mp4"));
    assert_file_looks_like_mp4(&saved);

    let _ = std::fs::remove_file(saved);
}
