//! Child-control creation, data binding, and input validation for the settings dialog.

use std::path::PathBuf;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::capture::Format;
use crate::settings::{wide, Fps, Settings};

use super::{
    get_control_text, set_control_text, IDC_AUTOSTART_CHECK, IDC_CANCEL_BTN, IDC_FORMAT_COMBO,
    IDC_FPS_EDIT, IDC_RECORDINGS_BROWSE, IDC_RECORDINGS_EDIT, IDC_SAVE_BTN, IDC_SCREENSHOTS_BROWSE,
    IDC_SCREENSHOTS_EDIT,
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

/// Create all child controls (labels, edits, combo, checkbox, buttons).
///
/// # Safety
///
/// `hwnd` must be a valid parent window. Win32 `CreateWindowExW` and
/// `SendMessageW` calls require valid handles, which are checked
/// immediately after creation.
pub(super) unsafe fn create_controls(hwnd: HWND) {
    let font = GetStockObject(DEFAULT_GUI_FONT);

    let make =
        |class: PCWSTR, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: i32| -> HWND {
            let text_w = wide(text);
            let ctrl = CreateWindowExW(
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
            );
            match ctrl {
                Ok(ctrl) => {
                    SendMessageW(
                        ctrl,
                        WM_SETFONT_MSG,
                        Some(WPARAM(font.0 as usize)),
                        Some(LPARAM(1)),
                    );
                    ctrl
                }
                Err(_) => HWND::default(),
            }
        };

    // Row 1 — Screenshots folder
    make(w!("STATIC"), "Screenshots folder:", 0, 15, 18, 120, 20, 0);
    make(
        w!("EDIT"),
        "",
        ES_READONLY | WS_BORDER.0,
        140,
        15,
        245,
        23,
        IDC_SCREENSHOTS_EDIT,
    );
    make(
        w!("BUTTON"),
        "Browse",
        0,
        390,
        15,
        55,
        23,
        IDC_SCREENSHOTS_BROWSE,
    );

    // Row 2 — Recordings folder
    make(w!("STATIC"), "Recordings folder:", 0, 15, 53, 120, 20, 0);
    make(
        w!("EDIT"),
        "",
        ES_READONLY | WS_BORDER.0,
        140,
        50,
        245,
        23,
        IDC_RECORDINGS_EDIT,
    );
    make(
        w!("BUTTON"),
        "Browse",
        0,
        390,
        50,
        55,
        23,
        IDC_RECORDINGS_BROWSE,
    );

    // Row 3 — Default format
    make(w!("STATIC"), "Default format:", 0, 15, 93, 120, 20, 0);
    let combo = make(
        w!("COMBOBOX"),
        "",
        CBS_DROPDOWNLIST | WS_VSCROLL.0,
        140,
        90,
        100,
        120,
        IDC_FORMAT_COMBO,
    );
    // The combo box is populated from the enum, so its row order and
    // `Format::ALL` cannot drift apart.
    for item in Format::ALL.iter().map(Format::extension) {
        let item_w = wide(item);
        SendMessageW(
            combo,
            CB_ADDSTRINGW,
            Some(WPARAM(0)),
            Some(LPARAM(item_w.as_ptr() as isize)),
        );
    }

    // Row 4 — Recording FPS
    make(w!("STATIC"), "Recording FPS:", 0, 15, 128, 120, 20, 0);
    make(
        w!("EDIT"),
        "",
        ES_NUMBER | WS_BORDER.0,
        140,
        125,
        60,
        23,
        IDC_FPS_EDIT,
    );

    // Row 5 — Auto-start checkbox
    make(
        w!("BUTTON"),
        "Start with Windows",
        BS_AUTOCHECKBOX,
        15,
        170,
        200,
        23,
        IDC_AUTOSTART_CHECK,
    );

    // Row 6 — Save / Cancel
    make(w!("BUTTON"), "Save", 0, 280, 220, 75, 28, IDC_SAVE_BTN);
    make(w!("BUTTON"), "Cancel", 0, 365, 220, 75, 28, IDC_CANCEL_BTN);
}

/// Fill all controls with the current settings values.
///
/// # Safety
///
/// `hwnd` must be a valid dialog with child controls already created by
/// [`create_controls`]. Control IDs must match the constants defined above.
pub(super) unsafe fn populate_controls(hwnd: HWND, settings: &Settings) {
    set_control_text(
        hwnd,
        IDC_SCREENSHOTS_EDIT,
        &settings.screenshots_dir.display().to_string(),
    );
    set_control_text(
        hwnd,
        IDC_RECORDINGS_EDIT,
        &settings.recordings_dir.display().to_string(),
    );

    // Format combo
    if let Ok(combo) = GetDlgItem(Some(hwnd), IDC_FORMAT_COMBO) {
        let idx = Format::ALL
            .iter()
            .position(|&f| f == settings.default_format)
            .unwrap_or(0);
        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(idx)), Some(LPARAM(0)));
    }

    // FPS
    set_control_text(hwnd, IDC_FPS_EDIT, &settings.recording_fps.to_string());

    // Auto-start checkbox
    if let Ok(check) = GetDlgItem(Some(hwnd), IDC_AUTOSTART_CHECK) {
        let state = if settings.auto_start { BST_CHECKED } else { 0 };
        SendMessageW(check, BM_SETCHECK, Some(WPARAM(state)), Some(LPARAM(0)));
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
    let screenshots_dir = get_control_text(hwnd, IDC_SCREENSHOTS_EDIT);
    let recordings_dir = get_control_text(hwnd, IDC_RECORDINGS_EDIT);

    let combo = GetDlgItem(Some(hwnd), IDC_FORMAT_COMBO)
        .map_err(|_| "Cannot read format control.".to_string())?;
    let sel = SendMessageW(combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as i32;

    let fps_text = get_control_text(hwnd, IDC_FPS_EDIT);

    let check = GetDlgItem(Some(hwnd), IDC_AUTOSTART_CHECK)
        .map_err(|_| "Cannot read auto-start control.".to_string())?;
    let checked = SendMessageW(check, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
    let auto_start = checked == BST_CHECKED;

    build_settings_from_inputs(
        &screenshots_dir,
        &recordings_dir,
        sel,
        &fps_text,
        auto_start,
    )
}
