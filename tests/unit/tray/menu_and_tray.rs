//! Tests for menu id constants, menu labels, `MenuAction` dispatch, and
//! tray/icon creation.

use crate::tray::*;

#[test]
fn create_default_icon_succeeds() {
    assert!(create_default_icon().is_ok());
}

#[test]
fn menu_constants_are_stable() {
    assert_eq!(MENU_CAPTURE, "capture");
    assert_eq!(MENU_CAPTURE_WINDOW, "capture_window");
    assert_eq!(MENU_OPEN_FOLDER, "open_folder");
    assert_eq!(MENU_EXIT, "exit");
    assert_eq!(MENU_START_RECORD, "start_record");
    assert_eq!(MENU_STOP_RECORD, "stop_record");
    assert_eq!(MENU_SETTINGS, "settings");
}

#[test]
fn toggle_record_label_describes_action_and_hotkey() {
    let label = MenuAction::ToggleRecord.menu_label();
    assert!(label.contains("Start"));
    assert!(label.contains("Stop"));
    assert!(
        label.contains("Ctrl+Shift+R"),
        "label should mention the hotkey"
    );
}

// -------------------------------------------------------------------------
// MenuAction dispatch: id() / from_id() round-trip and uniqueness
// -------------------------------------------------------------------------

#[test]
fn menu_action_from_id_roundtrips_every_variant() {
    for action in MenuAction::ALL {
        assert_eq!(
            MenuAction::from_id(action.id()),
            Some(action),
            "from_id(id()) must return the same variant for {action:?}"
        );
    }
}

/// All ids the dispatcher can ever see — the six `MenuAction::ALL` ids plus
/// the legacy `MENU_STOP_RECORD` — are pairwise unique. The `MENU_*` constants
/// are defined as `MenuAction::X.id()` in production code, so this single test
/// also guarantees uniqueness of every menu constant.
#[test]
fn menu_action_ids_and_legacy_stop_record_are_pairwise_unique() {
    let mut ids: Vec<&str> = MenuAction::ALL.iter().map(|action| action.id()).collect();
    ids.push(MENU_STOP_RECORD);
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j], "menu ids at index {i} and {j} must differ");
        }
    }
}

#[test]
fn menu_action_from_id_maps_legacy_stop_record_to_toggle_record() {
    assert_eq!(
        MenuAction::from_id(MENU_STOP_RECORD),
        Some(MenuAction::ToggleRecord)
    );
    assert_eq!(
        MenuAction::from_id("stop_record"),
        Some(MenuAction::ToggleRecord)
    );
}

#[test]
fn menu_action_from_id_unknown_or_empty_is_none() {
    for bogus in [
        "", "nonsense", "Capture", "CAPTURE", " capture", "capture ", "record",
    ] {
        assert_eq!(
            MenuAction::from_id(bogus),
            None,
            "expected None for {bogus:?}"
        );
    }
}

#[test]
fn menu_receiver_is_the_process_wide_singleton() {
    // Receiver-based callers must always observe the same process-wide channel.
    // If each call handed back a fresh channel, menu clicks would be split
    // across receivers and silently lost.
    let first = menu_receiver();
    let second = menu_receiver();
    assert!(std::ptr::eq(first, second));
    // Nothing has been clicked, so a drain must not block or yield an event.
    assert!(first.try_recv().is_err());
}

/// Whether the tray backend can actually be exercised on this machine.
///
/// Windows and macOS: always. Linux: only with a display, because
/// `create_tray` initialises GTK there and GTK cannot open a display that is not
/// there — a bare container or SSH session would fail this test on its
/// environment rather than on a regression. The GitLab `test` job is exactly
/// that: `Dockerfile.ci` installs no Xvfb and no appindicator.
///
/// `RUSTSHOT_LIVE_CAPTURE_STRICT` DISABLES the skip, matching the live-capture
/// guards in `tests/bdd.rs` and `tests/bench_functions/live_capture.rs`. It is
/// set for every job in `.github/workflows/ci.yml`, where `xvfb-run` guarantees a
/// display, so the one CI job that can cover the Linux tray path cannot quietly
/// stop covering it — this gate can never degrade into an everywhere-green no-op.
/// The name says "live capture" but the meaning is "CI guarantees a display,
/// tolerate no skips", which is precisely the condition needed here.
#[cfg(not(target_os = "macos"))]
fn tray_backend_is_testable() -> bool {
    if !cfg!(target_os = "linux") {
        return true;
    }
    std::env::var_os("RUSTSHOT_LIVE_CAPTURE_STRICT").is_some()
        || std::env::var_os("DISPLAY").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// The tray is built exactly once, at startup, and a failure there leaves the
/// app running with no icon and no menu — reachable only by the hotkeys. On
/// macOS, `muda::Menu` must be created on the process main thread, which the
/// standard Rust test harness does not provide. The runtime creates it from the
/// main event loop, so exercise this direct construction test on other targets.
///
/// On Linux this is also the regression test for `tray::init_platform_gui`:
/// without it `TrayIconBuilder::build` panics on the gtk-rs
/// "GTK has not been initialized" assertion. It deliberately does NOT initialise
/// GTK itself — if it did, deleting the initialisation from `create_tray` would
/// leave this test green while the app crashed on every Linux desktop. For the
/// same reason there is no second GTK-touching test: the harness gives each
/// `#[test]` its own thread, and gtk-rs panics with "Attempted to initialize GTK
/// from two different threads" when a second one initialises it.
#[cfg(not(target_os = "macos"))]
#[test]
fn create_tray_builds_the_icon_and_menu() {
    if !tray_backend_is_testable() {
        eprintln!("skipping tray build: no display server (DISPLAY/WAYLAND_DISPLAY) for GTK");
        return;
    }

    let tray = create_tray().expect("the tray icon and its six menu items must build");
    // Dropping it removes the icon again, so the test leaves no tray entry
    // behind for the rest of the run.
    drop(tray);
}
