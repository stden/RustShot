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
fn autorun_registry_location_and_value_are_stable() {
    assert_eq!(
        AUTORUN_SUBKEY,
        r"Software\Microsoft\Windows\CurrentVersion\Run"
    );
    assert_eq!(AUTORUN_VALUE, "RustShot");
}

#[cfg(windows)]
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
