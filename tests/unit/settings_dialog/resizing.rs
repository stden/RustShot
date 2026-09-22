//! Resizing keeps folder paths usable and actions reachable in client coordinates.

use super::*;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetUpdateRect, PtVisible, RedrawWindow, ReleaseDC, RDW_ALLCHILDREN, RDW_VALIDATE,
};

#[test]
fn parent_background_paint_does_not_cover_cancel_button() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);
    let _close_on_failure = CloseSettingsWindowOnDrop;
    show_settings_dialog();
    let hwnd = wait_for_settings_window().expect("settings dialog must appear");
    let cancel = control_rect(hwnd, IDC_CANCEL_BTN);
    // SAFETY: the test keeps the dialog alive until its DC is released below.
    let dc = unsafe { GetDC(Some(hwnd)) };
    assert!(!dc.is_invalid(), "acquire dialog paint context");
    // SAFETY: dc is live and the point is in the Cancel button's client bounds.
    let paints_over_cancel = unsafe {
        PtVisible(
            dc,
            (cancel.left + cancel.right) / 2,
            (cancel.top + cancel.bottom) / 2,
        )
    }
    .as_bool();
    // SAFETY: release exactly the DC acquired above for this still-live window.
    unsafe { ReleaseDC(Some(hwnd), dc) };
    assert!(
        !paints_over_cancel,
        "parent background must not erase Cancel while resizing"
    );
}

struct ResizePaintWindow(HWND);

impl Drop for ResizePaintWindow {
    fn drop(&mut self) {
        // SAFETY: this test fixture owns the window on this thread. The STATIC
        // class has no settings cleanup, so release the font after its children.
        let font = unsafe { SetWindowLongPtrW(self.0, GWLP_USERDATA, 0) };
        // SAFETY: the fixture is the sole owner and destroys the window once.
        let _ = unsafe { DestroyWindow(self.0) };
        if font != 0 {
            // SAFETY: create_controls allocated this font; all its users are gone.
            let _ = unsafe {
                windows::Win32::Graphics::Gdi::DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(
                    font as *mut core::ffi::c_void,
                ))
            };
        }
    }
}

#[test]
fn resizing_invalidates_old_button_positions_and_all_moved_controls() {
    // A same-thread native host lets us inspect update regions before the
    // message loop paints them, without relying on desktop pixels or timing.
    // SAFETY: STATIC is a system class; the returned window belongs to this thread.
    let window = ResizePaintWindow(
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("RustShot resize paint test"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                800,
                320,
                None,
                None,
                None,
                None,
            )
        }
        .expect("create resize paint host"),
    );
    let hwnd = window.0;
    // SAFETY: the fixture owns a live, empty native parent on this thread.
    unsafe { create_controls(hwnd) };
    // SAFETY: the fixture keeps the window alive through all paint queries.
    unsafe { activate_dialog(hwnd) };
    let old_save = control_rect(hwnd, IDC_SAVE_BTN);
    // SAFETY: resize only this fixture. Suppress incidental native invalidation
    // so the assertions observe exactly the production layout handler's work.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            1040,
            500,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOREDRAW,
        )
    }
    .expect("resize paint host");
    // SAFETY: hwnd and all children are live; validation only clears paint debt.
    assert!(
        unsafe { RedrawWindow(Some(hwnd), None, None, RDW_VALIDATE | RDW_ALLCHILDREN) }.as_bool()
    );
    // SAFETY: called on the owning thread after changing the native client size.
    unsafe { resize_controls(hwnd) };

    let mut dirty = RECT::default();
    // SAFETY: hwnd is live and dirty is writable; false avoids painting here.
    assert!(
        unsafe { GetUpdateRect(hwnd, Some(&mut dirty), false) }.as_bool(),
        "moving buttons must invalidate the exposed parent background"
    );
    assert!(
        dirty.left <= old_save.left
            && dirty.top <= old_save.top
            && dirty.right >= old_save.right
            && dirty.bottom >= old_save.bottom,
        "old Save button would remain painted: {old_save:?}, update: {dirty:?}"
    );
    for id in [
        IDC_SCREENSHOTS_EDIT,
        IDC_SCREENSHOTS_BROWSE,
        IDC_RECORDINGS_EDIT,
        IDC_RECORDINGS_BROWSE,
        IDC_SAVE_BTN,
        IDC_CANCEL_BTN,
    ] {
        // SAFETY: create_controls created this child; the fixture keeps it live.
        let child = unsafe { GetDlgItem(Some(hwnd), id) }.expect("moved control exists");
        // SAFETY: live child, writable output, no erase or paint requested.
        assert!(
            unsafe { GetUpdateRect(child, Some(&mut dirty), false) }.as_bool(),
            "moved control {id} needs repainting at its new position"
        );
    }
}

#[test]
fn client_layout_preserves_spacing_at_narrow_default_and_large_sizes() {
    let fixed = settings_dialog_layout();
    for (width, height) in [(594, 281), (784, 281), (1024, 461)] {
        let layout = resized_controls_layout(width, height);
        assert!(layout.path_edit_width > 0);
        assert_eq!(fixed.field_x + layout.path_edit_width + 10, layout.browse_x);
        assert_eq!(layout.browse_x + fixed.browse_width + 20, width);
        assert_eq!(layout.save_x + 90 + 10, layout.cancel_x);
        assert_eq!(layout.cancel_x + 90 + 20, width);
        assert_eq!(layout.button_y + fixed.button_height + 20, height);
        assert!(
            layout.button_y > 178 + fixed.field_height,
            "actions clear the checkbox"
        );
    }
}

#[test]
fn client_layout_changes_only_the_dimensions_tied_to_the_resized_axis() {
    let original = resized_controls_layout(784, 281);
    let wider = resized_controls_layout(1084, 281);
    assert_eq!(wider.path_edit_width - original.path_edit_width, 300);
    assert_eq!(wider.browse_x - original.browse_x, 300);
    assert_eq!(wider.save_x - original.save_x, 300);
    assert_eq!(wider.cancel_x - original.cancel_x, 300);
    assert_eq!(wider.button_y, original.button_y);

    let taller = resized_controls_layout(784, 481);
    assert_eq!(taller.button_y - original.button_y, 200);
    assert_eq!(taller.path_edit_width, original.path_edit_width);
    assert_eq!(taller.browse_x, original.browse_x);
    assert_eq!(taller.save_x, original.save_x);
    assert_eq!(taller.cancel_x, original.cancel_x);
}

fn control_rect(hwnd: HWND, id: i32) -> RECT {
    // SAFETY: the test keeps its parent open and resolves this existing child.
    let child = unsafe { GetDlgItem(Some(hwnd), id) }.expect("settings control must exist");
    let mut rect = RECT::default();
    // SAFETY: child is live and rect is writable for the synchronous query.
    unsafe { GetWindowRect(child, &mut rect) }.expect("read settings control bounds");
    let mut origin = POINT {
        x: rect.left,
        y: rect.top,
    };
    // SAFETY: hwnd is live and origin is a writable screen-coordinate point.
    assert!(unsafe { windows::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut origin) }.as_bool());
    RECT {
        left: origin.x,
        top: origin.y,
        right: origin.x + rect.right - rect.left,
        bottom: origin.y + rect.bottom - rect.top,
    }
}

fn client_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    // SAFETY: the dialog remains open and rect is writable for this query.
    unsafe { GetClientRect(hwnd, &mut rect) }.expect("read settings client bounds");
    rect
}

#[test]
fn resizing_reflows_folder_fields_and_preserves_fixed_controls() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);
    let _close_on_failure = CloseSettingsWindowOnDrop;
    show_settings_dialog();
    let hwnd = wait_for_settings_window().expect("settings dialog must appear before resizing");
    // SAFETY: hwnd is this test's live window and GWL_STYLE reads its style bits.
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    assert_ne!(
        style & WS_THICKFRAME.0,
        0,
        "users need a draggable resize frame"
    );
    assert_ne!(style & WS_MAXIMIZEBOX.0, 0, "users need a maximize button");
    let mut minimum = MINMAXINFO::default();
    // SAFETY: WM_GETMINMAXINFO synchronously borrows this live, correctly typed
    // output on the window's thread and returns before minimum is inspected.
    unsafe {
        SendMessageW(
            hwnd,
            WM_GETMINMAXINFO,
            Some(WPARAM(0)),
            Some(LPARAM(std::ptr::from_mut(&mut minimum) as isize)),
        )
    };
    assert_eq!(minimum.ptMinTrackSize.x, 610);
    assert_eq!(minimum.ptMinTrackSize.y, 320);
    let original_client = client_rect(hwnd);
    let original_folder = control_rect(hwnd, IDC_SCREENSHOTS_EDIT);
    let fixed_ids = [IDC_FORMAT_COMBO, IDC_FPS_EDIT, IDC_AUTOSTART_CHECK];
    let original_fixed = fixed_ids.map(|id| control_rect(hwnd, id));

    // SAFETY: hwnd is this test's live dialog; the flags preserve position,
    // activation, and Z order while delivering native WM_SIZE synchronously.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            1040,
            500,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    }
    .expect("resize settings dialog");

    let resized_client = client_rect(hwnd);
    let resized_folder = control_rect(hwnd, IDC_SCREENSHOTS_EDIT);
    assert_eq!(
        resized_folder.right - resized_folder.left - (original_folder.right - original_folder.left),
        resized_client.right - original_client.right,
        "folder fields must gain the full increase in client width"
    );
    for (edit_id, browse_id) in [
        (IDC_SCREENSHOTS_EDIT, IDC_SCREENSHOTS_BROWSE),
        (IDC_RECORDINGS_EDIT, IDC_RECORDINGS_BROWSE),
    ] {
        let edit = control_rect(hwnd, edit_id);
        let browse = control_rect(hwnd, browse_id);
        assert_eq!(browse.left - edit.right, 10, "folder and Browse gap");
        assert_eq!(
            resized_client.right - browse.right,
            20,
            "Browse right margin"
        );
    }

    let save = control_rect(hwnd, IDC_SAVE_BTN);
    let cancel = control_rect(hwnd, IDC_CANCEL_BTN);
    assert_eq!(cancel.left - save.right, 10, "action button gap");
    assert_eq!(
        resized_client.right - cancel.right,
        20,
        "Cancel right margin"
    );
    assert_eq!(
        resized_client.bottom - save.bottom,
        20,
        "Save bottom margin"
    );
    assert_eq!(save.bottom, cancel.bottom);
    assert_eq!(
        fixed_ids.map(|id| control_rect(hwnd, id)),
        original_fixed,
        "format, FPS, and checkbox stay in place"
    );

    // SAFETY: as for expansion above, this changes only this test window's size.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            610,
            320,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    }
    .expect("shrink settings dialog to its minimum");
    let minimum_client = client_rect(hwnd);
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
        let rect = control_rect(hwnd, id);
        assert!(
            rect.left >= 0
                && rect.top >= 0
                && rect.right <= minimum_client.right
                && rect.bottom <= minimum_client.bottom,
            "control {id} is clipped at the minimum dialog size: {rect:?} in {minimum_client:?}"
        );
    }
    let checkbox = control_rect(hwnd, IDC_AUTOSTART_CHECK);
    let save = control_rect(hwnd, IDC_SAVE_BTN);
    assert!(
        save.top > checkbox.bottom,
        "minimum-size actions must clear the last row"
    );
}
