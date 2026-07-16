//! Tests for menu id constants, menu labels, and tray/icon creation.

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
fn menu_capture_and_capture_window_are_distinct() {
    assert_ne!(MENU_CAPTURE, MENU_CAPTURE_WINDOW);
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
fn menu_start_record_and_stop_record_are_distinct() {
    assert_ne!(MENU_START_RECORD, MENU_STOP_RECORD);
}

#[test]
fn toggle_record_label_mentions_start_and_stop() {
    assert!(TOGGLE_RECORD_LABEL.contains("Start"));
    assert!(TOGGLE_RECORD_LABEL.contains("Stop"));
}

// --- menu constant uniqueness ---

#[test]
fn all_menu_constants_are_unique() {
    let ids = [
        MENU_CAPTURE,
        MENU_CAPTURE_WINDOW,
        MENU_OPEN_FOLDER,
        MENU_EXIT,
        MENU_START_RECORD,
        MENU_STOP_RECORD,
    ];
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert!(ids[i] != ids[j]);
        }
    }
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
fn menu_settings_is_distinct_from_all_others() {
    assert_ne!(MENU_SETTINGS, MENU_CAPTURE);
    assert_ne!(MENU_SETTINGS, MENU_CAPTURE_WINDOW);
    assert_ne!(MENU_SETTINGS, MENU_OPEN_FOLDER);
    assert_ne!(MENU_SETTINGS, MENU_EXIT);
    assert_ne!(MENU_SETTINGS, MENU_START_RECORD);
    assert_ne!(MENU_SETTINGS, MENU_STOP_RECORD);
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

// --- all_menu_constants_including_settings ---

#[test]
fn all_seven_menu_constants_are_unique() {
    let ids = [
        MENU_CAPTURE,
        MENU_CAPTURE_WINDOW,
        MENU_OPEN_FOLDER,
        MENU_SETTINGS,
        MENU_EXIT,
        MENU_START_RECORD,
        MENU_STOP_RECORD,
    ];
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert!(ids[i] != ids[j]);
        }
    }
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

// -------------------------------------------------------------------------
// All 7 menu constants are distinct from each other
// -------------------------------------------------------------------------

#[test]
fn all_seven_menu_constants_are_pairwise_distinct() {
    let ids = [
        MENU_CAPTURE,
        MENU_OPEN_FOLDER,
        MENU_EXIT,
        MENU_CAPTURE_WINDOW,
        MENU_SETTINGS,
        MENU_START_RECORD,
        MENU_STOP_RECORD,
    ];
    for (i, a) in ids.iter().enumerate() {
        for (j, b) in ids.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "menu constants at index {i} and {j} must differ");
            }
        }
    }
}

#[test]
fn toggle_record_label_contains_start_or_stop_keyword() {
    let lower = TOGGLE_RECORD_LABEL.to_lowercase();
    assert!(
        lower.contains("start") || lower.contains("stop"),
        "label should mention start or stop"
    );
}
