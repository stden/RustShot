// ---------------------------------------------------------------------------
// Cross-module: resolve_output_path + Format interaction
// ---------------------------------------------------------------------------

#[test]
fn resolve_output_path_all_formats_produce_matching_extension() {
    use rustshot::capture::Format;
    for fmt in [Format::Png, Format::Jpg, Format::Bmp] {
        let path = rustshot::capture::resolve_output_path(None, fmt);
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some(fmt.extension()),
            "extension mismatch for format {:?}",
            fmt
        );
    }
}

#[test]
fn resolve_output_path_default_filename_starts_with_screenshot() {
    let path = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with("screenshot_"));
}

#[test]
fn resolve_output_path_default_filename_only_valid_chars() {
    let path = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.'));
}

#[test]
fn resolve_output_path_default_timestamp_segment_has_expected_length() {
    let path = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let inner = &name["screenshot_".len()..name.len() - ".png".len()];
    assert_eq!(inner.len(), 25);
}

// ---------------------------------------------------------------------------
// Cross-module default path consistency
// ---------------------------------------------------------------------------

#[test]
fn settings_default_screenshots_dir_matches_capture_dir() {
    let settings = rustshot::settings::Settings::default();
    assert_eq!(
        settings.screenshots_dir,
        rustshot::capture::get_screenshots_dir()
    );
}

#[test]
fn settings_default_recordings_dir_matches_recording_dir() {
    let settings = rustshot::settings::Settings::default();
    assert_eq!(
        settings.recordings_dir,
        rustshot::record::get_recording_dir()
    );
}

#[test]
fn settings_default_recordings_dir_is_child_of_screenshots_dir() {
    let settings = rustshot::settings::Settings::default();
    assert_eq!(
        settings.recordings_dir.parent(),
        Some(settings.screenshots_dir.as_path())
    );
}

// ---------------------------------------------------------------------------
// Cross-module: bgra_to_rgba ↔ rgba_to_bgra_bottom_up composability
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_then_rgba_to_bgra_bottom_up_1x1_roundtrip() {
    // Start with BGRA data, convert to RGBA, then to BGRA bottom-up (1x1 = no flip)
    let bgra = vec![50u8, 100, 150, 255]; // B=50, G=100, R=150, A=255
    let mut rgba = bgra.clone();
    rustshot::capture::bgra_to_rgba(&mut rgba);
    // Now rgba = [150, 100, 50, 255] (R=150, G=100, B=50, A=255)
    let bgra_bu = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 1).unwrap();
    // BGRA bottom-up with 1 row = just channel swap: [50, 100, 150, 255]
    assert_eq!(bgra_bu, bgra);
}

// ---------------------------------------------------------------------------
// get_screenshots_dir / get_recording_dir relationship
// ---------------------------------------------------------------------------

#[test]
fn recording_dir_is_child_of_screenshots_dir() {
    let screenshots = rustshot::capture::get_screenshots_dir();
    let recordings = rustshot::record::get_recording_dir();
    assert_eq!(recordings.parent(), Some(screenshots.as_path()));
}

#[test]
fn get_screenshots_dir_is_absolute_or_dot_relative() {
    let dir = rustshot::capture::get_screenshots_dir();
    let display = dir.display().to_string();
    assert!(
        dir.is_absolute() || display.starts_with('.'),
        "should be absolute or dot-relative: {display}"
    );
}

#[test]
fn get_recording_dir_has_at_least_three_components() {
    let dir = rustshot::record::get_recording_dir();
    assert!(
        dir.components().count() >= 3,
        "recording dir should have at least Pictures/RustShot/recordings"
    );
}
