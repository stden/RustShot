//! Adapter pins: the catalogue's actions must retain their native hotkey bindings.

use super::*;
use global_hotkey::hotkey::{Code, Modifiers};

#[test]
fn catalogue_registers_the_existing_hotkeys_in_order() {
    let mut observed = Vec::new();
    let registered = RegisteredHotkeys::register_with(|hotkey| {
        observed.push(hotkey);
        Ok::<(), String>(())
    })
    .unwrap();
    let expected = [
        (
            HotKey::new(None, Code::PrintScreen),
            tray::MenuAction::Capture,
        ),
        (
            HotKey::new(Some(Modifiers::ALT), Code::PrintScreen),
            tray::MenuAction::CaptureWindow,
        ),
        (
            HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyR),
            tray::MenuAction::ToggleRecord,
        ),
    ];
    assert_eq!(observed, expected.map(|(hotkey, _)| hotkey));
    for (hotkey, action) in expected {
        assert_eq!(
            registered.action_for(GlobalHotKeyEvent {
                id: hotkey.id(),
                state: HotKeyState::Pressed,
            }),
            Some(action)
        );
        assert_eq!(
            registered.action_for(GlobalHotKeyEvent {
                id: hotkey.id(),
                state: HotKeyState::Released,
            }),
            None
        );
    }
    assert_eq!(
        registered.action_for(GlobalHotKeyEvent {
            id: HotKey::new(None, Code::KeyA).id(),
            state: HotKeyState::Pressed,
        }),
        None
    );
}

#[test]
fn registration_stops_at_failure_and_keeps_the_shortcut_in_the_error() {
    let mut calls = 0;
    let result = RegisteredHotkeys::register_with(|_| {
        calls += 1;
        if calls == 2 {
            Err("already in use")
        } else {
            Ok(())
        }
    });
    assert_eq!(calls, 2);
    assert_eq!(
        result.err().unwrap(),
        "Failed to register Alt+PrintScreen hotkey: already in use"
    );
}
