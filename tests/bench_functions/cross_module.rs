#[test]
fn resolve_output_path_all_formats_produce_matching_extension() {
    use rustshot::capture::Format;

    for format in [Format::Png, Format::Jpg, Format::Bmp] {
        let path = rustshot::capture::resolve_output_path(None, format);
        assert_eq!(
            path.extension().and_then(|extension| extension.to_str()),
            Some(format.extension())
        );
    }
}

#[test]
fn settings_default_screenshots_dir_matches_capture_dir() {
    assert_eq!(
        rustshot::settings::Settings::default().screenshots_dir,
        rustshot::capture::get_screenshots_dir()
    );
}

#[test]
fn settings_default_recordings_dir_matches_recording_dir() {
    assert_eq!(
        rustshot::settings::Settings::default().recordings_dir,
        rustshot::record::get_recording_dir()
    );
}

#[test]
fn bgra_and_rgba_converters_compose_for_one_pixel() {
    let bgra = vec![50, 100, 150, 255];
    let mut rgba = bgra.clone();
    rustshot::capture::bgra_to_rgba(&mut rgba);
    assert_eq!(
        rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 1).unwrap(),
        bgra
    );
}

#[test]
fn recording_dir_is_child_of_screenshots_dir() {
    let screenshots = rustshot::capture::get_screenshots_dir();
    let recordings = rustshot::record::get_recording_dir();
    assert_eq!(recordings.parent(), Some(screenshots.as_path()));
}
