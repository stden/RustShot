//! Tests for load/save on disk and the open-settings-dialog workflow.

use super::*;

#[test]
fn load_settings_returns_defaults_when_no_file() {
    // Reads SETTINGS_PATH_OVERRIDE, so it must not run while another test is
    // mid-way through pointing that override at a populated temp file.
    let _global = lock_global_overrides();

    // settings file unlikely to exist in test env at the exact expected path
    let s = load_settings();
    assert_eq!(s.default_format, Format::Png);
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

    let temp_root =
        std::env::temp_dir().join(format!("rustshot_settings_io_{}", generate_timestamp()));
    std::fs::create_dir_all(&temp_root).unwrap();
    let settings_path = temp_root.join("RustShot").join("settings.json");
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

    let _ = std::fs::remove_dir_all(&temp_root);
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

    let temp_root =
        std::env::temp_dir().join(format!("rustshot_settings_dialog_{}", generate_timestamp()));
    std::fs::create_dir_all(&temp_root).unwrap();
    let settings_path = temp_root.join("RustShot").join("settings.json");
    let _guard = Guard;
    set_settings_path_for_tests(Some(settings_path.clone()));
    set_open_file_in_editor_for_tests(Some(record_opened_path));
    OPENED_PATHS.lock().unwrap().clear();

    open_settings_dialog();

    let opened_paths = OPENED_PATHS.lock().unwrap().clone();
    assert_eq!(opened_paths, vec![settings_path.clone()]);
    assert!(settings_path.is_file());

    let _ = std::fs::remove_dir_all(&temp_root);
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

    let temp_root =
        std::env::temp_dir().join(format!("rustshot_settings_fail_{}", generate_timestamp()));
    std::fs::create_dir_all(&temp_root).unwrap();
    let blocked_parent = temp_root.join("blocked_parent");
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

    let _ = std::fs::remove_dir_all(&temp_root);
}

// -------------------------------------------------------------------------
// save_settings roundtrip
// -------------------------------------------------------------------------

#[test]
fn save_settings_creates_file_and_roundtrips() {
    let dir = std::env::temp_dir().join("rustshot_test_save_settings");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.json");

    let settings = Settings {
        screenshots_dir: PathBuf::from("/tmp/shots"),
        recordings_dir: PathBuf::from("/tmp/recs"),
        default_format: Format::Jpg,
        recording_fps: Fps::new(60).unwrap(),
        auto_start: false,
    };

    let json = format_settings_json(&settings);
    std::fs::write(&path, &json).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    let loaded = parse_settings_json(&content);
    assert_eq!(loaded.default_format, Format::Jpg);
    assert_eq!(loaded.recording_fps.get(), 60);
    assert!(!loaded.auto_start);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_settings_roundtrip_preserves_all_fields() {
    let settings = Settings {
        screenshots_dir: PathBuf::from("/home/user/Pictures"),
        recordings_dir: PathBuf::from("/home/user/Videos"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(144).unwrap(),
        auto_start: true,
    };
    let json = format_settings_json(&settings);
    let loaded = parse_settings_json(&json);
    assert_eq!(loaded.screenshots_dir, settings.screenshots_dir);
    assert_eq!(loaded.recordings_dir, settings.recordings_dir);
    assert_eq!(loaded.default_format, settings.default_format);
    assert_eq!(loaded.recording_fps, settings.recording_fps);
    assert_eq!(loaded.auto_start, settings.auto_start);
}
