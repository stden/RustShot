//! Tests for menu id constants, menu labels, `MenuAction` dispatch, and
//! tray/icon creation.

use crate::tray::*;

#[test]
fn menu_receiver_does_not_panic() {
    // menu_receiver() must be callable without panicking
    let _recv = menu_receiver();
}

#[test]
fn create_default_icon_succeeds() {
    assert!(create_default_icon().is_ok());
}

#[cfg(windows)]
#[test]
fn create_tray_smoke_test() {
    let _ = create_tray();
}

#[test]
fn menu_constants_are_stable() {
    assert_eq!(MENU_CAPTURE, "capture");
    assert_eq!(MENU_CAPTURE_WINDOW, "capture_window");
    assert_eq!(MENU_OPEN_FOLDER, "open_folder");
    assert_eq!(MENU_EXIT, "exit");
}

#[test]
fn menu_capture_window_constant_value() {
    assert_eq!(MENU_CAPTURE_WINDOW, "capture_window");
}

#[test]
fn menu_capture_constant_value() {
    assert_eq!(MENU_CAPTURE, "capture");
}

#[test]
fn menu_open_folder_constant_value() {
    assert_eq!(MENU_OPEN_FOLDER, "open_folder");
}

#[test]
fn menu_exit_constant_value() {
    assert_eq!(MENU_EXIT, "exit");
}

#[test]
fn menu_start_record_constant_value() {
    assert_eq!(MENU_START_RECORD, "start_record");
}

#[test]
fn menu_stop_record_constant_value() {
    assert_eq!(MENU_STOP_RECORD, "stop_record");
}

#[test]
fn toggle_record_label_mentions_start_and_stop() {
    assert!(TOGGLE_RECORD_LABEL.contains("Start"));
    assert!(TOGGLE_RECORD_LABEL.contains("Stop"));
}

#[test]
fn menu_constants_are_non_empty() {
    assert!(!MENU_CAPTURE.is_empty());
    assert!(!MENU_CAPTURE_WINDOW.is_empty());
    assert!(!MENU_OPEN_FOLDER.is_empty());
    assert!(!MENU_EXIT.is_empty());
    assert!(!MENU_START_RECORD.is_empty());
    assert!(!MENU_STOP_RECORD.is_empty());
}

#[test]
fn menu_constants_contain_no_whitespace() {
    for id in [
        MENU_CAPTURE,
        MENU_CAPTURE_WINDOW,
        MENU_OPEN_FOLDER,
        MENU_EXIT,
        MENU_START_RECORD,
        MENU_STOP_RECORD,
    ] {
        assert!(
            !id.contains(' '),
            "menu ID should not contain spaces: '{id}'"
        );
    }
}

// --- MENU_SETTINGS constant ---

#[test]
fn menu_settings_constant_value() {
    assert_eq!(MENU_SETTINGS, "settings");
}

#[test]
fn menu_settings_contains_no_whitespace() {
    assert!(
        !MENU_SETTINGS.contains(' '),
        "settings menu ID should not contain spaces"
    );
}

#[test]
fn menu_settings_is_non_empty() {
    assert!(!MENU_SETTINGS.is_empty());
}

// --- toggle_record_label ---

#[test]
fn toggle_record_label_mentions_hotkey() {
    assert!(
        TOGGLE_RECORD_LABEL.contains("Ctrl+Shift+R"),
        "label should mention the hotkey"
    );
}

#[test]
fn toggle_record_label_is_non_empty() {
    assert!(!TOGGLE_RECORD_LABEL.is_empty());
}

#[test]
fn toggle_record_label_contains_start_or_stop_keyword() {
    let lower = TOGGLE_RECORD_LABEL.to_lowercase();
    assert!(
        lower.contains("start") || lower.contains("stop"),
        "label should mention start or stop"
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
