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
    assert!(TOGGLE_RECORD_LABEL.contains("Start"));
    assert!(TOGGLE_RECORD_LABEL.contains("Stop"));
    assert!(
        TOGGLE_RECORD_LABEL.contains("Ctrl+Shift+R"),
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
    // `about_to_wait` drains this receiver every wake-up. If each call handed
    // back a fresh channel, menu clicks would be split across receivers and
    // silently lost, so the singleton identity is load-bearing.
    let first = menu_receiver();
    let second = menu_receiver();
    assert!(std::ptr::eq(first, second));
    // Nothing has been clicked, so a drain must not block or yield an event.
    assert!(first.try_recv().is_err());
}
