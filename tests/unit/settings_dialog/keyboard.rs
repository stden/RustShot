//! Native keyboard navigation and cancellation must work without saving settings.

use super::*;

#[test]
fn settings_keyboard_tab_order_reaches_every_interactive_control() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);
    let _close_on_failure = CloseSettingsWindowOnDrop;
    show_settings_dialog();
    let hwnd = wait_for_settings_window().expect("settings window must appear");
    let mut previous = None;

    for id in [
        IDC_SCREENSHOTS_EDIT,
        IDC_SCREENSHOTS_BROWSE,
        IDC_RECORDINGS_EDIT,
        IDC_RECORDINGS_BROWSE,
        IDC_FORMAT_COMBO,
        IDC_FPS_EDIT,
        IDC_AUTOSTART_CHECK,
        IDC_SAVE_BTN,
        IDC_CANCEL_BTN,
    ] {
        // SAFETY: the fixture keeps the dialog and each previously returned
        // child alive; this query does not change focus or dispatch commands.
        let next = unsafe { GetNextDlgTabItem(hwnd, previous, false) }
            .expect("every settings control must be reachable with Tab");
        // SAFETY: next is a child of the live dialog returned by Windows.
        assert_eq!(unsafe { GetDlgCtrlID(next) }, id);
        previous = Some(next);
    }
}

#[test]
fn settings_keyboard_escape_closes_the_dialog() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);
    let _close_on_failure = CloseSettingsWindowOnDrop;
    show_settings_dialog();
    let hwnd = wait_for_settings_window().expect("settings window must appear");
    // SAFETY: post a scalar Escape key message to this test's live window.
    // Its owning message loop must translate it into Cancel, never Save.
    unsafe {
        PostMessageW(
            Some(hwnd),
            WM_KEYDOWN,
            WPARAM(0x1b), // VK_ESCAPE
            LPARAM(1),
        )
    }
    .expect("post Escape to the settings loop");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while DIALOG_OPEN.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        !DIALOG_OPEN.load(Ordering::SeqCst),
        "Escape must dismiss the settings window through its Cancel action"
    );
}
