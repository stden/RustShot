//! Windows autorun registry helpers and `sync_autorun` policy tests.

use super::*;

#[test]
fn windows_manifest_declares_per_monitor_v2_dpi_awareness() {
    let manifest = include_str!("../../rustshot.exe.manifest");
    let compact: String = manifest.split_whitespace().collect();

    assert!(compact.contains(">PerMonitorV2</dpiAwareness>"));
    assert!(compact.contains(">true/pm</dpiAware>"));
}

// --- autorun helper tests ---

#[cfg(windows)]
#[test]
fn quoted_autorun_command_wraps_paths_in_quotes() {
    let command = quoted_autorun_command(Path::new(r"C:\Program Files\RustShot\rustshot.exe"));
    assert_eq!(command, r#""C:\Program Files\RustShot\rustshot.exe""#);
}

#[cfg(windows)]
#[test]
fn quoted_autorun_command_simple_path() {
    let command = quoted_autorun_command(Path::new(r"C:\rustshot.exe"));
    assert_eq!(command, r#""C:\rustshot.exe""#);
}

#[cfg(windows)]
#[test]
fn quoted_autorun_command_starts_and_ends_with_quote() {
    let command = quoted_autorun_command(Path::new(r"C:\app\test.exe"));
    assert!(command.starts_with('"'));
    assert!(command.ends_with('"'));
}

#[cfg(windows)]
#[test]
fn autorun_subkey_points_to_run_key() {
    assert!(AUTORUN_SUBKEY.contains("Run"));
    assert!(AUTORUN_SUBKEY.starts_with("Software\\"));
}

#[cfg(windows)]
#[test]
fn autorun_value_is_rustshot() {
    assert_eq!(AUTORUN_VALUE, "RustShot");
}

#[cfg(windows)]
#[test]
fn autorun_constants_produce_valid_wide_strings() {
    let subkey = settings::wide(AUTORUN_SUBKEY);
    let value = settings::wide(AUTORUN_VALUE);
    // Both should be null-terminated
    assert_eq!(*subkey.last().unwrap(), 0);
    assert_eq!(*value.last().unwrap(), 0);
    // Length should be string length + 1 (null)
    assert_eq!(subkey.len(), AUTORUN_SUBKEY.len() + 1);
    assert_eq!(value.len(), AUTORUN_VALUE.len() + 1);
}

#[test]
fn sync_autorun_with_handlers_disables_when_auto_start_is_false() {
    let set_called = Cell::new(false);
    let remove_called = Cell::new(false);
    sync_autorun_with_handlers(false, || set_called.set(true), || remove_called.set(true));
    assert!(!set_called.get());
    assert!(remove_called.get());
}

#[test]
fn sync_autorun_with_handlers_enables_when_auto_start_is_true() {
    let set_called = Cell::new(false);
    let remove_called = Cell::new(false);
    sync_autorun_with_handlers(true, || set_called.set(true), || remove_called.set(true));
    assert!(set_called.get());
    assert!(!remove_called.get());
}

// --- sync_autorun_with_handlers ---

#[test]
fn sync_autorun_with_handlers_calls_enable_when_true() {
    let called = Cell::new(false);
    sync_autorun_with_handlers(true, || called.set(true), || {});
    assert!(called.get());
}

#[test]
fn sync_autorun_with_handlers_calls_disable_when_false() {
    let called = Cell::new(false);
    sync_autorun_with_handlers(false, || {}, || called.set(true));
    assert!(called.get());
}

#[test]
fn sync_autorun_with_handlers_does_not_call_disable_when_true() {
    let called = Cell::new(false);
    sync_autorun_with_handlers(true, || {}, || called.set(true));
    assert!(!called.get());
}

#[test]
fn sync_autorun_with_handlers_does_not_call_enable_when_false() {
    let called = Cell::new(false);
    sync_autorun_with_handlers(false, || called.set(true), || {});
    assert!(!called.get());
}

// -------------------------------------------------------------------------
// sync_autorun_with_handlers — verify mutual exclusivity
// -------------------------------------------------------------------------

#[test]
fn sync_autorun_with_handlers_enable_and_disable_are_mutually_exclusive() {
    let enable_count = Cell::new(0u32);
    let disable_count = Cell::new(0u32);

    sync_autorun_with_handlers(
        true,
        || enable_count.set(enable_count.get() + 1),
        || disable_count.set(disable_count.get() + 1),
    );
    assert_eq!(enable_count.get(), 1);
    assert_eq!(disable_count.get(), 0);

    sync_autorun_with_handlers(
        false,
        || enable_count.set(enable_count.get() + 1),
        || disable_count.set(disable_count.get() + 1),
    );
    assert_eq!(enable_count.get(), 1);
    assert_eq!(disable_count.get(), 1);
}
