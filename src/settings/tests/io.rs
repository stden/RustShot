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

#[test]
fn save_settings_reports_a_config_directory_it_cannot_create() {
    struct PathGuard;
    impl Drop for PathGuard {
        fn drop(&mut self) {
            set_settings_path_for_tests(None);
        }
    }

    let _global = lock_global_overrides();

    // Put a regular file where the settings directory would have to go, so
    // `create_dir_all` cannot succeed and the failure has to surface as a
    // readable message instead of a silent no-op.
    let temp_root = TestDir::new("rustshot_settings_blocked");
    let blocker = temp_root.file("blocker");
    std::fs::write(&blocker, b"not a directory").expect("test fixture file");
    let _guard = PathGuard;
    set_settings_path_for_tests(Some(blocker.join("RustShot").join("settings.json")));

    let error = save_settings(&Settings::default())
        .expect_err("a settings directory under a file cannot be created");
    assert!(
        error.starts_with("Failed to create config dir: "),
        "unexpected message: {error}"
    );
}

#[test]
fn save_settings_removes_its_scratch_file_when_the_replacement_fails() {
    struct PathGuard;
    impl Drop for PathGuard {
        fn drop(&mut self) {
            set_settings_path_for_tests(None);
        }
    }

    let _global = lock_global_overrides();

    // Point the settings path at an existing *directory*: its parent exists, so
    // the scratch file is written successfully, but `fs::rename` cannot replace
    // a directory with a file. Without the cleanup on the rename error path,
    // every failed save would leave one more `settings.json.<pid>.tmp` behind
    // next to the target.
    let temp_root = TestDir::new("rustshot_settings_rename_fail");
    let occupied = temp_root.file("settings.json");
    std::fs::create_dir_all(&occupied).expect("test fixture directory");
    let _guard = PathGuard;
    set_settings_path_for_tests(Some(occupied));

    let error = save_settings(&Settings::default())
        .expect_err("a directory cannot be replaced by the settings file");
    assert!(
        error.starts_with("Failed to write settings: "),
        "unexpected message: {error}"
    );

    let leftovers: Vec<_> = std::fs::read_dir(temp_root.path())
        .expect("temp dir")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name != "settings.json")
        .collect();
    assert!(leftovers.is_empty(), "left temporary files: {leftovers:?}");
}

#[test]
fn launch_editor_command_reports_a_missing_editor() {
    struct EditorGuard;
    impl Drop for EditorGuard {
        fn drop(&mut self) {
            set_open_file_in_editor_for_tests(None);
        }
    }

    let _global = lock_global_overrides();

    // With the injection point cleared, the real spawn runs. A program name that
    // cannot exist proves the error is propagated rather than swallowed.
    set_open_file_in_editor_for_tests(None);
    let _guard = EditorGuard;

    assert!(launch_editor_command(
        "rustshot-editor-that-does-not-exist",
        std::path::Path::new("settings.json")
    )
    .is_err());
}

#[test]
fn save_settings_is_never_observed_half_written() {
    struct PathGuard;
    impl Drop for PathGuard {
        fn drop(&mut self) {
            set_settings_path_for_tests(None);
        }
    }

    let _global = lock_global_overrides();

    // RustShot runs several processes that read settings (the --select and
    // --select-record children, the MCP server) while the settings dialog
    // writes them. A truncate-then-write would let a reader see an empty or
    // partial file, which `parse_settings_json` silently turns into defaults --
    // screenshots would land in the wrong directory with no error anywhere.
    let temp_root = TestDir::new("rustshot_settings_atomic");
    let settings_path = temp_root.file("settings.json");
    let _guard = PathGuard;
    set_settings_path_for_tests(Some(settings_path.clone()));

    let written = Settings {
        screenshots_dir: PathBuf::from("/tmp/atomic-shots"),
        recordings_dir: PathBuf::from("/tmp/atomic-recs"),
        default_format: Format::Bmp,
        recording_fps: Fps::new(24).unwrap(),
        auto_start: false,
    };
    save_settings(&written).expect("first save");

    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            let mut observed_partial = 0usize;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                // Read the raw bytes: any moment where the file exists but is
                // not complete JSON is exactly the failure being guarded.
                if let Ok(content) = std::fs::read_to_string(&settings_path) {
                    if parse_settings_json(&content) != written {
                        observed_partial += 1;
                    }
                }
            }
            observed_partial
        });

        for _ in 0..300 {
            save_settings(&written).expect("concurrent save");
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);

        let observed_partial = reader.join().expect("reader thread");
        assert_eq!(
            observed_partial, 0,
            "a concurrent reader saw {observed_partial} incomplete settings files"
        );
    });

    // The replacement must not leave scratch files behind next to the target.
    let leftovers: Vec<_> = std::fs::read_dir(temp_root.path())
        .expect("temp dir")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name != "settings.json")
        .collect();
    assert!(leftovers.is_empty(), "left temporary files: {leftovers:?}");
}
