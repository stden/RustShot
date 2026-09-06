//! Child-control creation, data binding, and input validation for the settings dialog.

use std::path::PathBuf;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, GetStockObject, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_GUI_FONT, DEFAULT_PITCH, FF_DONTCARE, FW_NORMAL, HGDIOBJ,
    OUT_DEFAULT_PRECIS,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::domain::Format;
use crate::settings::{wide, Fps, Settings};

use super::{
    font_pixel_height, get_control_text, set_control_text, settings_dialog_layout,
    IDC_AUTOSTART_CHECK, IDC_CANCEL_BTN, IDC_FORMAT_COMBO, IDC_FPS_EDIT, IDC_RECORDINGS_BROWSE,
    IDC_RECORDINGS_EDIT, IDC_SAVE_BTN, IDC_SCREENSHOTS_BROWSE, IDC_SCREENSHOTS_EDIT,
};

// Win32 control-specific style constants.
const ES_READONLY: u32 = 0x0800;
const ES_NUMBER: u32 = 0x2000;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const BS_AUTOCHECKBOX: u32 = 0x0003;

// Win32 messages for controls.
const WM_SETFONT_MSG: u32 = 0x0030;
const CB_ADDSTRINGW: u32 = 0x0143;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETCURSEL: u32 = 0x0147;
const BM_SETCHECK: u32 = 0x00F1;
const BM_GETCHECK: u32 = 0x00F0;
const BST_CHECKED: usize = 1;

/// Positions that follow the client edges; the remaining controls stay fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ResizedControlsLayout {
    pub(super) path_edit_width: i32,
    pub(super) browse_x: i32,
    pub(super) save_x: i32,
    pub(super) cancel_x: i32,
    pub(super) button_y: i32,
}

/// Derive anchors from usable client space, not outer window dimensions whose
/// non-client frame changes with the window style and desktop configuration.
pub(super) fn resized_controls_layout(
    client_width: i32,
    client_height: i32,
) -> ResizedControlsLayout {
    let layout = settings_dialog_layout();
    let margin = 20;
    let gap = 10;
    let button_width = 90;
    // WM_GETMINMAXINFO normally prevents undersized windows, but defensive
    // clamps also keep programmatic or transient sizing from creating negative
    // child dimensions. The folder field retains at least one pixel.
    let browse_x = (client_width - margin - layout.browse_width).max(layout.field_x + gap + 1);
    let cancel_x = (client_width - margin - button_width).max(0);
    ResizedControlsLayout {
        path_edit_width: browse_x - layout.field_x - gap,
        browse_x,
        save_x: (cancel_x - gap - button_width).max(0),
        cancel_x,
        button_y: (client_height - margin - layout.button_height).max(0),
    }
}

/// Create the dialog's larger font, falling back to the stock GUI font if GDI
/// cannot allocate it. Only a custom font is stored on the parent for cleanup;
/// stock objects remain system-owned.
///
/// # Safety
///
/// `hwnd` must be a valid settings-dialog window whose `GWLP_USERDATA` slot is
/// not used for another value.
unsafe fn create_dialog_font(hwnd: HWND, point_size: i32) -> HGDIOBJ {
    // SAFETY: every scalar is a documented font attribute and the face name is
    // a static NUL-terminated UTF-16 literal. The returned font is owned by this
    // dialog until `release_dialog_font` handles WM_NCDESTROY.
    let custom_font = unsafe {
        CreateFontW(
            font_pixel_height(point_size),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            u32::from(DEFAULT_PITCH.0 | FF_DONTCARE.0),
            w!("Segoe UI"),
        )
    };

    if custom_font.is_invalid() {
        // SAFETY: `DEFAULT_GUI_FONT` is a predefined stock-object index. The
        // returned object remains system-owned and therefore is not recorded in
        // `GWLP_USERDATA` for deletion.
        unsafe { GetStockObject(DEFAULT_GUI_FONT) }
    } else {
        // SAFETY: `hwnd` is live by contract and this module exclusively owns
        // its user-data slot. Storing the raw handle keeps the HFONT reachable
        // until the window's final destruction message.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, custom_font.0 as isize) };
        custom_font.into()
    }
}

/// Delete the custom dialog font, if one was allocated.
///
/// # Safety
///
/// `hwnd` must be the settings-dialog window passed to [`create_dialog_font`],
/// and this function must run only once its child controls are being destroyed.
pub(super) unsafe fn release_dialog_font(hwnd: HWND) {
    // SAFETY: `hwnd` is live for WM_NCDESTROY and this atomically clears the
    // module-owned slot while retrieving the font handle stored there.
    let raw_font = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
    if raw_font != 0 {
        // SAFETY: a non-zero slot value came only from `CreateFontW` above and
        // is deleted exactly once after clearing the slot. Stock objects are
        // never stored there and so cannot reach this branch.
        let _ = unsafe { DeleteObject(HGDIOBJ(raw_font as *mut core::ffi::c_void)) };
    }
}

/// Create all child controls (labels, edits, combo, checkbox, buttons).
///
/// # Safety
///
/// `hwnd` must be a valid parent window.
///
/// The child handles are deliberately **not** checked: `make` maps a failed
/// `CreateWindowExW` to `HWND::default()`, and every later `SendMessageW` and
/// `GetDlgItem` uses whatever it got. That is sound rather than lucky — Win32
/// answers a message sent to an invalid window with 0 instead of misbehaving — so
/// a control that fails to create is silently absent rather than fatal. Do not
/// read this as "the handles are validated"; an earlier version of this doc said
/// so and was wrong.
pub(super) unsafe fn create_controls(hwnd: HWND) {
    let layout = settings_dialog_layout();
    // SAFETY: `hwnd` is the live, newly created parent from this function's
    // contract, and its user-data slot has not been assigned yet.
    let font = unsafe { create_dialog_font(hwnd, layout.font_point_size) };

    let make =
        |class: PCWSTR, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: i32| -> HWND {
            let text_w = wide(text);
            // SAFETY: `hwnd` is the live parent window per this function's
            // contract, and every `class` passed below is a `w!` literal naming a
            // predefined control class. `wide` NUL-terminates `text_w`, which
            // outlives the synchronous call (the new window copies its title).
            // Reinterpreting `id` as the HMENU argument is the documented meaning
            // of that parameter for a WS_CHILD window: it is the control ID.
            let ctrl = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    class,
                    PCWSTR(text_w.as_ptr()),
                    WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | style),
                    x,
                    y,
                    w,
                    h,
                    Some(hwnd),
                    Some(HMENU(id as usize as *mut core::ffi::c_void)),
                    None,
                    None,
                )
            };
            match ctrl {
                Ok(ctrl) => {
                    // SAFETY: `ctrl` is the handle `CreateWindowExW` just
                    // returned, so the control exists. `font` is the stock font
                    // obtained above, still alive — or null if that lookup
                    // failed, which WM_SETFONT reads as "use the system font"
                    // rather than dereferencing. LPARAM(1) only asks for a
                    // redraw; no pointer crosses the boundary.
                    unsafe {
                        SendMessageW(
                            ctrl,
                            WM_SETFONT_MSG,
                            Some(WPARAM(font.0 as usize)),
                            Some(LPARAM(1)),
                        )
                    };
                    ctrl
                }
                Err(_) => HWND::default(),
            }
        };

    // Row 1 — Screenshots folder
    make(w!("STATIC"), "Screenshots folder:", 0, 15, 18, 130, 22, 0);
    make(
        w!("EDIT"),
        "",
        ES_READONLY | WS_BORDER.0,
        layout.field_x,
        14,
        layout.path_edit_width,
        layout.field_height,
        IDC_SCREENSHOTS_EDIT,
    );
    make(
        w!("BUTTON"),
        "Browse",
        0,
        layout.browse_x,
        14,
        layout.browse_width,
        layout.field_height,
        IDC_SCREENSHOTS_BROWSE,
    );

    // Row 2 — Recordings folder
    make(w!("STATIC"), "Recordings folder:", 0, 15, 56, 130, 22, 0);
    make(
        w!("EDIT"),
        "",
        ES_READONLY | WS_BORDER.0,
        layout.field_x,
        52,
        layout.path_edit_width,
        layout.field_height,
        IDC_RECORDINGS_EDIT,
    );
    make(
        w!("BUTTON"),
        "Browse",
        0,
        layout.browse_x,
        52,
        layout.browse_width,
        layout.field_height,
        IDC_RECORDINGS_BROWSE,
    );

    // Row 3 — Default format
    make(w!("STATIC"), "Default format:", 0, 15, 98, 130, 22, 0);
    let combo = make(
        w!("COMBOBOX"),
        "",
        CBS_DROPDOWNLIST | WS_VSCROLL.0,
        layout.field_x,
        94,
        layout.format_field_width,
        140,
        IDC_FORMAT_COMBO,
    );
    // The combo box is populated from the enum, so its row order and
    // `Format::ALL` cannot drift apart.
    for item in Format::ALL.iter().map(Format::extension) {
        let item_w = wide(item);
        // SAFETY: `combo` is the COMBOBOX created just above — or the null handle
        // `make` falls back to, which the OS rejects as an invalid window rather
        // than dispatching anywhere. `item_w` is NUL-terminated by `wide` and
        // outlives the call, which is synchronous and makes the control copy the
        // string, so passing its pointer as an LPARAM is sound.
        unsafe {
            SendMessageW(
                combo,
                CB_ADDSTRINGW,
                Some(WPARAM(0)),
                Some(LPARAM(item_w.as_ptr() as isize)),
            )
        };
    }

    // Row 4 — Recording FPS
    make(w!("STATIC"), "Recording FPS:", 0, 15, 140, 130, 22, 0);
    make(
        w!("EDIT"),
        "",
        ES_NUMBER | WS_BORDER.0,
        layout.field_x,
        136,
        layout.fps_field_width,
        layout.field_height,
        IDC_FPS_EDIT,
    );

    // Row 5 — Auto-start checkbox
    make(
        w!("BUTTON"),
        "Start with Windows",
        BS_AUTOCHECKBOX,
        15,
        178,
        220,
        layout.field_height,
        IDC_AUTOSTART_CHECK,
    );

    // Row 6 — Save / Cancel
    make(
        w!("BUTTON"),
        "Save",
        0,
        layout.dialog_width - 230,
        230,
        90,
        layout.button_height,
        IDC_SAVE_BTN,
    );
    make(
        w!("BUTTON"),
        "Cancel",
        0,
        layout.dialog_width - 130,
        230,
        90,
        layout.button_height,
        IDC_CANCEL_BTN,
    );

    // SAFETY: all child-creation attempts have completed for this live parent.
    // Initial placement must use the same client-edge anchors as later WM_SIZE.
    unsafe { resize_controls(hwnd) };
}

/// Stretch folder paths and anchor Browse and action buttons to the client edges.
///
/// # Safety
///
/// `hwnd` must be a live settings window and this must run on its owning thread.
/// Missing children are skipped because native sizing can occur during creation
/// or destruction, and an individual child-creation attempt can also fail.
pub(super) unsafe fn resize_controls(hwnd: HWND) {
    let mut client = RECT::default();
    // SAFETY: hwnd is live by contract and client is writable for this call.
    if let Err(error) = unsafe { GetClientRect(hwnd, &mut client) } {
        log::error!("Failed to read the settings window's client size: {error}");
        return;
    }
    let width = client.right - client.left;
    let height = client.bottom - client.top;
    if width <= 0 || height <= 0 {
        return;
    }

    let layout = settings_dialog_layout();
    let resized = resized_controls_layout(width, height);
    let controls = [
        (
            IDC_SCREENSHOTS_EDIT,
            layout.field_x,
            14,
            resized.path_edit_width,
            layout.field_height,
        ),
        (
            IDC_SCREENSHOTS_BROWSE,
            resized.browse_x,
            14,
            layout.browse_width,
            layout.field_height,
        ),
        (
            IDC_RECORDINGS_EDIT,
            layout.field_x,
            52,
            resized.path_edit_width,
            layout.field_height,
        ),
        (
            IDC_RECORDINGS_BROWSE,
            resized.browse_x,
            52,
            layout.browse_width,
            layout.field_height,
        ),
        (
            IDC_SAVE_BTN,
            resized.save_x,
            resized.button_y,
            90,
            layout.button_height,
        ),
        (
            IDC_CANCEL_BTN,
            resized.cancel_x,
            resized.button_y,
            90,
            layout.button_height,
        ),
    ];
    for (index, (id, x, y, width, height)) in controls.into_iter().enumerate() {
        // SAFETY: hwnd is the live parent and id names one of its controls.
        if let Ok(child) = unsafe { GetDlgItem(Some(hwnd), id) } {
            // SAFETY: child was just resolved on this window's owning thread;
            // dimensions come from the non-negative layout above. Defer repaint
            // until every control has moved so the buttons never render against
            // an intermediate parent background during a resize.
            let redraw = index + 1 == controls.len();
            if let Err(error) = unsafe { MoveWindow(child, x, y, width, height, redraw) } {
                log::error!("Failed to resize settings control {id}: {error}");
            }
        }
    }
}

/// Fill all controls with the current settings values.
///
/// # Safety
///
/// `hwnd` must be a valid dialog with child controls already created by
/// [`create_controls`]. Control IDs must match the constants defined above.
pub(super) unsafe fn populate_controls(hwnd: HWND, settings: &Settings) {
    // Formatted outside the unsafe blocks below: a `Display` impl and a `String`
    // allocation have no business inside an unsafe scope, and keeping them out is
    // what makes each block one FFI call wide.
    let screenshots = settings.screenshots_dir.display().to_string();
    let recordings = settings.recordings_dir.display().to_string();

    // SAFETY: `hwnd` is a live window whose children were created by
    // `create_controls` per this function's contract, which is exactly what
    // `set_control_text` needs — it resolves the ID itself and does nothing when
    // the child is missing.
    unsafe { set_control_text(hwnd, IDC_SCREENSHOTS_EDIT, &screenshots) };
    // SAFETY: as above; same window, a different child ID.
    unsafe { set_control_text(hwnd, IDC_RECORDINGS_EDIT, &recordings) };

    // Format combo
    // SAFETY: `hwnd` is a live window per the contract; `GetDlgItem` reports
    // `Err` for a missing child, so the `if let` establishes that `combo` exists.
    if let Ok(combo) = unsafe { GetDlgItem(Some(hwnd), IDC_FORMAT_COMBO) } {
        let idx = Format::ALL
            .iter()
            .position(|&f| f == settings.default_format)
            .unwrap_or(0);
        // SAFETY: `combo` is the handle just returned. `idx` is an index into
        // `Format::ALL`, and `create_controls` filled the combo from that same
        // array, so it addresses a row that exists; the message carries no
        // pointers, only integers.
        unsafe { SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(idx)), Some(LPARAM(0))) };
    }

    // FPS
    // SAFETY: as for the two edits above — live parent, child created by
    // `create_controls`.
    unsafe { set_control_text(hwnd, IDC_FPS_EDIT, &settings.recording_fps.to_string()) };

    // Auto-start checkbox
    // SAFETY: live parent per the contract, and a missing child surfaces as
    // `Err` rather than an unusable handle.
    if let Ok(check) = unsafe { GetDlgItem(Some(hwnd), IDC_AUTOSTART_CHECK) } {
        let state = if settings.auto_start { BST_CHECKED } else { 0 };
        // SAFETY: `check` is the handle just returned, and BM_SETCHECK takes only
        // an integer state, no pointer.
        unsafe { SendMessageW(check, BM_SETCHECK, Some(WPARAM(state)), Some(LPARAM(0))) };
    }
}

/// Map a combo-box selection index to a [`Format`].
///
/// Out-of-range selections fall back to the first variant rather than failing —
/// the combo box is populated from `Format::ALL`, so an out-of-range index means
/// the OS handed us something impossible.
pub(crate) fn format_index_to_format(sel: i32) -> Format {
    usize::try_from(sel)
        .ok()
        .and_then(|i| Format::ALL.get(i).copied())
        .unwrap_or(Format::Png)
}

/// The name shown for a combo-box selection index.
///
/// Test-only: production code works with the [`Format`] itself and never needs
/// the string back, since `Settings` stores the enum.
#[cfg(test)]
pub(crate) fn format_index_to_name(sel: i32) -> &'static str {
    format_index_to_format(sel).extension()
}

/// Parse and validate the FPS text typed into the dialog.
///
/// Returns an [`Fps`], not a `u32`: the dialog is a trust boundary, so it parses
/// into the validated type once and everything downstream is spared the check.
/// The range rule itself lives in [`Fps::new`]; this only supplies the
/// dialog-specific wording.
pub(crate) fn validate_fps_input(text: &str) -> Result<Fps, String> {
    let raw: u32 = text
        .parse()
        .map_err(|_| "Recording FPS must be a positive number.".to_string())?;
    if raw == 0 {
        return Err("Recording FPS must be a positive number.".to_string());
    }
    Fps::new(raw).map_err(|_| format!("Recording FPS must not exceed {}.", Fps::MAX))
}

/// Build a validated [`Settings`] value from raw dialog field inputs.
pub(super) fn build_settings_from_inputs(
    screenshots_dir: &str,
    recordings_dir: &str,
    format_selection: i32,
    fps_text: &str,
    auto_start: bool,
) -> Result<Settings, String> {
    if screenshots_dir.is_empty() {
        return Err("Screenshots folder cannot be empty.".to_string());
    }

    if recordings_dir.is_empty() {
        return Err("Recordings folder cannot be empty.".to_string());
    }

    Ok(Settings {
        screenshots_dir: PathBuf::from(screenshots_dir),
        recordings_dir: PathBuf::from(recordings_dir),
        default_format: format_index_to_format(format_selection),
        recording_fps: validate_fps_input(fps_text)?,
        auto_start,
    })
}

/// Read all control values and construct a validated [`Settings`].
///
/// # Safety
///
/// `hwnd` must be a valid dialog whose child controls were created by
/// [`create_controls`] and still exist.
pub(super) unsafe fn read_controls(hwnd: HWND) -> Result<Settings, String> {
    // SAFETY: `hwnd` is a live window whose children were created by
    // `create_controls` and still exist, per this function's contract;
    // `get_control_text` resolves each ID itself and yields an empty string when
    // the child is gone.
    let screenshots_dir = unsafe { get_control_text(hwnd, IDC_SCREENSHOTS_EDIT) };
    // SAFETY: as above.
    let recordings_dir = unsafe { get_control_text(hwnd, IDC_RECORDINGS_EDIT) };

    // SAFETY: live parent per the contract; a missing child becomes the `Err`
    // that is mapped to a user-facing message below.
    let combo = unsafe { GetDlgItem(Some(hwnd), IDC_FORMAT_COMBO) }
        .map_err(|_| "Cannot read format control.".to_string())?;
    // SAFETY: `combo` is the handle just returned, and CB_GETCURSEL passes no
    // pointers — it only reads the selection index out of the control.
    let sel =
        unsafe { SendMessageW(combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))) }.0 as i32;

    // SAFETY: as for the folder edits above.
    let fps_text = unsafe { get_control_text(hwnd, IDC_FPS_EDIT) };

    // SAFETY: live parent per the contract; a missing child becomes an `Err`.
    let check = unsafe { GetDlgItem(Some(hwnd), IDC_AUTOSTART_CHECK) }
        .map_err(|_| "Cannot read auto-start control.".to_string())?;
    // SAFETY: `check` is the handle just returned, and BM_GETCHECK only reads the
    // control's own state; no pointer crosses the boundary.
    let checked =
        unsafe { SendMessageW(check, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))) }.0 as usize;
    let auto_start = checked == BST_CHECKED;

    build_settings_from_inputs(
        &screenshots_dir,
        &recordings_dir,
        sel,
        &fps_text,
        auto_start,
    )
}
