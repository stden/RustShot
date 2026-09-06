use super::*;

#[test]
fn catalog_preserves_menu_order_and_captions() {
    let actual: Vec<_> = MenuAction::ALL
        .into_iter()
        .map(|action| (action, action.menu_label()))
        .collect();
    assert_eq!(
        actual,
        [
            (MenuAction::Capture, "Capture Screenshot (PrintScreen)"),
            (
                MenuAction::CaptureWindow,
                "Capture Active Window (Alt+PrintScreen)"
            ),
            (MenuAction::OpenFolder, "Open Screenshots Folder"),
            (
                MenuAction::ToggleRecord,
                "Start or Stop Recording (Ctrl+Shift+R)"
            ),
            (MenuAction::Settings, "Settings"),
            (MenuAction::Exit, "Exit"),
        ]
    );
}

#[test]
fn catalog_has_exact_hotkey_bindings() {
    let actual: Vec<_> = MenuAction::ALL
        .into_iter()
        .filter_map(|action| action.hotkey().map(|hotkey| (action, hotkey)))
        .collect();
    assert_eq!(
        actual,
        [
            (
                MenuAction::Capture,
                HotkeySpec {
                    key: HotkeyKey::PrintScreen,
                    modifiers: HotkeyModifiers::None,
                    label: "PrintScreen",
                }
            ),
            (
                MenuAction::CaptureWindow,
                HotkeySpec {
                    key: HotkeyKey::PrintScreen,
                    modifiers: HotkeyModifiers::Alt,
                    label: "Alt+PrintScreen",
                }
            ),
            (
                MenuAction::ToggleRecord,
                HotkeySpec {
                    key: HotkeyKey::R,
                    modifiers: HotkeyModifiers::ControlShift,
                    label: "Ctrl+Shift+R",
                }
            ),
        ]
    );
}

#[test]
fn catalog_shortcuts_are_unique_and_match_menu_labels() {
    let mut seen_chords = Vec::new();
    for action in MenuAction::ALL {
        let Some(hotkey) = action.hotkey() else {
            continue;
        };
        let chord = (hotkey.key, hotkey.modifiers);
        assert!(
            !seen_chords.contains(&chord),
            "duplicate shortcut: {action:?}"
        );
        seen_chords.push(chord);
        assert!(action
            .menu_label()
            .ends_with(&format!(" ({})", hotkey.label)));
    }
}

#[test]
fn catalog_has_one_recording_toggle_and_no_legacy_menu_entry() {
    assert_eq!(
        MenuAction::ALL
            .into_iter()
            .filter(|action| *action == MenuAction::ToggleRecord)
            .count(),
        1
    );
    assert!(MenuAction::ALL
        .into_iter()
        .all(|action| action.id() != MENU_STOP_RECORD));
    assert_eq!(
        MenuAction::from_id(MENU_STOP_RECORD),
        Some(MenuAction::ToggleRecord)
    );
}
