//! Tests for load/save on disk and the open-settings-dialog workflow.

use super::*;
use crate::test_support::TestDir;

#[test]
fn load_settings_returns_defaults_when_no_file() {
    // Reads SETTINGS_PATH_OVERRIDE, so it must not run while another test is
    // mid-way through pointing that override at a populated temp file.
    let _global = lock_global_overrides();

    struct PathGuard;
    impl Drop for PathGuard {
        fn drop(&mut self) {
            set_settings_path_for_tests(None);
        }
    }
    // Point the override at a path that cannot exist, so this never reads the
    // developer's real settings file (which may hold non-default values).
    // Only the TestDir root exists; the "missing" subdirectory is
    // deliberately never created.
    let temp_root = TestDir::new("rustshot_missing");
    set_settings_path_for_tests(Some(temp_root.file("missing/settings.json")));
    let _guard = PathGuard;

    assert_eq!(load_settings(), Settings::default());
}

#[test]
fn save_settings_roundtrips_via_disk() {
    struct PathGuard;

    impl Drop for PathGuard {
        fn drop(&mut self) {
            set_settings_path_for_tests(None);
        }
    }

    let _global = lock_global_overrides();

    let temp_root = TestDir::new("rustshot_settings_io");
    let settings_path = temp_root.path().join("RustShot").join("settings.json");
    let _guard = PathGuard;
    set_settings_path_for_tests(Some(settings_path.clone()));

    let settings = Settings {
        screenshots_dir: PathBuf::from("/tmp/shots"),
        recordings_dir: PathBuf::from("/tmp/recs"),
        default_format: Format::Jpg,
        recording_fps: Fps::new(60).unwrap(),
        auto_start: false,
    };

    save_settings(&settings).unwrap();
    let loaded = load_settings();
    assert_eq!(loaded, settings);

    assert!(settings_path.is_file());
}

#[test]
fn open_settings_dialog_uses_test_override() {
    struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            set_settings_path_for_tests(None);
            set_open_file_in_editor_for_tests(None);
        }
    }

    static OPENED_PATHS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

    fn record_opened_path(path: &std::path::Path) {
        OPENED_PATHS.lock().unwrap().push(path.to_path_buf());
    }

    let _global = lock_global_overrides();

    let temp_root = TestDir::new("rustshot_settings_dialog");
    let settings_path = temp_root.path().join("RustShot").join("settings.json");
    let _guard = Guard;
    set_settings_path_for_tests(Some(settings_path.clone()));
    set_open_file_in_editor_for_tests(Some(record_opened_path));
    OPENED_PATHS.lock().unwrap().clear();

    open_settings_dialog();

    let opened_paths = OPENED_PATHS.lock().unwrap().clone();
    assert_eq!(opened_paths, vec![settings_path.clone()]);
    assert!(settings_path.is_file());
}

#[test]
fn open_settings_dialog_returns_early_when_save_fails() {
    struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            set_settings_path_for_tests(None);
            set_open_file_in_editor_for_tests(None);
        }
    }

    static OPENED_PATHS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

    fn record_opened_path(path: &std::path::Path) {
        OPENED_PATHS.lock().unwrap().push(path.to_path_buf());
    }

    let _global = lock_global_overrides();

    let temp_root = TestDir::new("rustshot_settings_fail");
    let blocked_parent = temp_root.file("blocked_parent");
    std::fs::write(&blocked_parent, "not a directory").unwrap();
    let settings_path = blocked_parent.join("settings.json");
    let _guard = Guard;
    set_settings_path_for_tests(Some(settings_path.clone()));
    set_open_file_in_editor_for_tests(Some(record_opened_path));
    OPENED_PATHS.lock().unwrap().clear();

    open_settings_dialog();

    let opened_paths = OPENED_PATHS.lock().unwrap().clone();
    assert!(opened_paths.is_empty());
    assert!(!settings_path.is_file());
}
