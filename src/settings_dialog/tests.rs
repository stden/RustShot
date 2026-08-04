//! Unit tests for the settings dialog (pure-logic parts only, no GUI).

use super::*;

use crate::capture::Format;
use crate::settings::Fps;

#[test]
fn fos_pickfolders_value_is_0x20() {
    assert_eq!(FOS_PICKFOLDERS, 0x20);
}

// --- validate_fps_input ---

#[test]
fn validate_fps_input_non_numeric_is_error() {
    assert!(validate_fps_input("abc").is_err());
}

// --- format_index_to_name ---

#[test]
fn format_index_to_name_out_of_range_defaults_to_png() {
    assert_eq!(format_index_to_name(99), "png");
    assert_eq!(format_index_to_name(-1), "png");
}

// --- validate_fps_input edge cases ---

#[test]
fn validate_fps_input_one_is_ok() {
    assert_eq!(validate_fps_input("1").map(Fps::get), Ok(1));
}

#[test]
fn validate_fps_input_at_max_is_ok() {
    assert_eq!(validate_fps_input("240").map(Fps::get), Ok(240));
}

#[test]
fn validate_fps_input_overflow_is_error() {
    assert!(validate_fps_input("4294967296").is_err());
}

#[test]
fn validate_fps_input_leading_zeros_accepted() {
    assert_eq!(validate_fps_input("030").map(Fps::get), Ok(30));
}

#[test]
fn validate_fps_input_error_message_contains_positive() {
    let err = validate_fps_input("0").unwrap_err();
    assert!(err.contains("positive"), "error: {err}");
}

// --- format_index_to_name edge cases ---

#[test]
fn format_index_to_name_matches_supported_formats() {
    for (i, &fmt) in Format::ALL.iter().enumerate() {
        assert_eq!(format_index_to_format(i as i32), fmt);
        assert_eq!(format_index_to_name(i as i32), fmt.extension());
    }
}

// --- build_settings_from_inputs ---

#[test]
fn build_settings_from_inputs_accepts_valid_values() {
    let settings = build_settings_from_inputs("C:\\shots", "C:\\records", 1, "60", true).unwrap();

    assert_eq!(settings.screenshots_dir, PathBuf::from("C:\\shots"));
    assert_eq!(settings.recordings_dir, PathBuf::from("C:\\records"));
    assert_eq!(settings.default_format, Format::Jpg);
    assert_eq!(settings.recording_fps.get(), 60);
    assert!(settings.auto_start);
}

#[test]
fn build_settings_from_inputs_rejects_empty_screenshots_dir() {
    let err = build_settings_from_inputs("", "C:\\records", 0, "30", false).unwrap_err();
    assert_eq!(err, "Screenshots folder cannot be empty.");
}

#[test]
fn build_settings_from_inputs_rejects_empty_recordings_dir() {
    let err = build_settings_from_inputs("C:\\shots", "", 0, "30", false).unwrap_err();
    assert_eq!(err, "Recordings folder cannot be empty.");
}

#[test]
fn build_settings_from_inputs_defaults_invalid_format_index_to_png() {
    let settings =
        build_settings_from_inputs("C:\\shots", "C:\\records", i32::MAX, "30", false).unwrap();

    assert_eq!(settings.default_format, Format::Png);
}

#[test]
fn build_settings_from_inputs_propagates_fps_validation_error() {
    let err = build_settings_from_inputs("C:\\shots", "C:\\records", 2, "0", false).unwrap_err();
    assert_eq!(err, "Recording FPS must be a positive number.");
}

#[test]
fn build_settings_from_inputs_preserves_false_auto_start() {
    let settings = build_settings_from_inputs("C:\\shots", "C:\\records", 2, "30", false).unwrap();

    assert_eq!(settings.default_format, Format::Bmp);
    assert!(!settings.auto_start);
}

#[test]
fn build_settings_from_inputs_accepts_max_recording_fps() {
    let max_fps = Fps::MAX.to_string();
    let settings =
        build_settings_from_inputs("C:\\shots", "C:\\records", 0, &max_fps, false).unwrap();

    assert_eq!(settings.default_format, Format::Png);
    assert_eq!(settings.recording_fps.get(), Fps::MAX);
}

#[test]
fn build_settings_from_inputs_preserves_unicode_paths() {
    let settings =
        build_settings_from_inputs("C:\\スクリーン", "C:\\Записи", 1, "30", true).unwrap();

    assert_eq!(settings.screenshots_dir, PathBuf::from("C:\\スクリーン"));
    assert_eq!(settings.recordings_dir, PathBuf::from("C:\\Записи"));
    assert_eq!(settings.default_format, Format::Jpg);
}

#[test]
fn validate_fps_input_above_max_reports_limit() {
    let err = validate_fps_input("241").unwrap_err();
    assert_eq!(err, format!("Recording FPS must not exceed {}.", Fps::MAX));
}

// --- DialogGuard ---

/// Serializes the tests that mutate the process-global [`DIALOG_OPEN`]
/// flag. `cargo test` runs tests on multiple threads, so without this lock
/// one test's store races another's assertion — an intermittent failure
/// that only shows up under parallelism (exactly how CI runs). Same pattern
/// as `GLOBAL_OVERRIDES_LOCK` in `settings.rs`.
static DIALOG_FLAG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A panicking test poisons the mutex; recover the guard so one failure
/// cannot cascade into failing the whole group.
fn lock_dialog_flag() -> std::sync::MutexGuard<'static, ()> {
    DIALOG_FLAG_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn dialog_guard_resets_flag_when_unwinding() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(true, Ordering::SeqCst);
    let result = std::panic::catch_unwind(|| {
        let _guard = DialogGuard;
        panic!("test panic");
    });
    assert!(result.is_err());
    assert!(!DIALOG_OPEN.load(Ordering::SeqCst));
}

#[test]
fn show_settings_dialog_returns_early_when_dialog_is_already_open() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(true, Ordering::SeqCst);
    show_settings_dialog();
    assert!(DIALOG_OPEN.load(Ordering::SeqCst));
    DIALOG_OPEN.store(false, Ordering::SeqCst);
}
