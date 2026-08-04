//! End-to-end regression coverage for validated public API boundaries.

use rustshot::{record, settings};

#[cfg(windows)]
#[test]
fn oversized_recording_region_is_rejected_before_a_worker_starts() {
    let region = rustshot::selection::Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: u32::MAX,
        physical_height: 2,
    };
    let result = record::RecordingSession::start_in_dir(
        0,
        0,
        region,
        settings::Fps::new(30).unwrap(),
        std::env::temp_dir().join("rustshot-e2e-invalid-recording"),
    );

    assert!(result.is_err());
}

#[test]
fn malformed_recording_buffers_return_errors_without_panicking() {
    let result = std::panic::catch_unwind(|| {
        let mut output = [0u8; 4];
        record::rgba_to_bgra_bottom_up_into(&[], &mut output, 1, 1)
    });

    assert!(result.is_ok(), "public conversion API must not panic");
    assert!(result.unwrap().is_err());
}

#[test]
fn settings_with_control_characters_are_valid_json_and_roundtrip() {
    let original = settings::Settings {
        screenshots_dir: "shots\0archive".into(),
        recordings_dir: "records\u{1}archive".into(),
        ..settings::Settings::default()
    };

    let encoded = settings::format_settings_json(&original);
    serde_json::from_str::<serde_json::Value>(&encoded).expect("settings must be valid JSON");
    let decoded = settings::parse_settings_json(&encoded);

    assert_eq!(decoded.screenshots_dir, original.screenshots_dir);
    assert_eq!(decoded.recordings_dir, original.recordings_dir);
}

#[cfg(unix)]
#[test]
fn non_unicode_settings_paths_roundtrip_losslessly() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let mut original = settings::Settings::default();
    original.screenshots_dir = OsString::from_vec(vec![b's', b'h', b'o', b't', 0xff]).into();

    let encoded = settings::format_settings_json(&original);
    let decoded = settings::parse_settings_json(&encoded);
    assert_eq!(decoded.screenshots_dir, original.screenshots_dir);
}
