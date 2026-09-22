//! Shared fixtures for the settings test suite; the tests live in the theme submodules.

use super::*;

mod behavior;
mod defaults;
mod escaping;
mod extract_scalars;
mod extract_strings;
mod formatting;
mod io;
mod parsing;

fn set_settings_path_for_tests(path: Option<PathBuf>) {
    *SETTINGS_PATH_OVERRIDE.lock().unwrap() = path;
}

fn set_open_file_in_editor_for_tests(callback: Option<fn(&std::path::Path)>) {
    *OPEN_FILE_EDITOR_OVERRIDE.lock().unwrap() = callback;
}

/// Serializes the tests that mutate the process-global settings overrides.
///
/// `SETTINGS_PATH_OVERRIDE` and `OPEN_FILE_EDITOR_OVERRIDE` are process-wide,
/// but `cargo test` runs tests on multiple threads by default. Without this
/// lock, one test's teardown (`set_*_for_tests(None)`) resets the override
/// while another test is still relying on it — an intermittent failure that
/// only shows up under parallelism, which is exactly how CI runs.
///
/// Every test touching those globals must hold this guard for its whole body.
/// Declare it FIRST, so it is dropped LAST: the test's own teardown guard
/// resets the overrides before the lock is released.
static GLOBAL_OVERRIDES_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A panicking test poisons the mutex; that must not cascade into failing
/// every other test in this group, so recover the guard rather than unwrap.
fn lock_global_overrides() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_OVERRIDES_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
