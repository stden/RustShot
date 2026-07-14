//! Native Win32 settings dialog (Windows-only).
//!
//! Opens a modal window with controls for all [`Settings`] fields.
//! Called from the tray menu "Settings" item instead of opening
//! `settings.json` in notepad.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, COLOR_BTNFACE, DEFAULT_GUI_FONT, HBRUSH};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::capture::Format;
use crate::settings::{load_settings, save_settings, wide, Fps, Settings};

// --- Control IDs ---
const IDC_SCREENSHOTS_EDIT: i32 = 101;
const IDC_SCREENSHOTS_BROWSE: i32 = 102;
const IDC_RECORDINGS_EDIT: i32 = 103;
const IDC_RECORDINGS_BROWSE: i32 = 104;
const IDC_FORMAT_COMBO: i32 = 105;
const IDC_FPS_EDIT: i32 = 106;
const IDC_AUTOSTART_CHECK: i32 = 107;
const IDC_SAVE_BTN: i32 = 108;
const IDC_CANCEL_BTN: i32 = 109;

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

// Win32 file dialog option: pick folders instead of files.
const FOS_PICKFOLDERS: u32 = 0x20;

/// Re-entry guard — only one settings dialog at a time.
static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// RAII guard that resets [`DIALOG_OPEN`] on thread exit (including panics).
///
/// Ensures the singleton flag is cleared even if `run_dialog` panics,
/// preventing the dialog from becoming permanently locked out.
struct DialogGuard;

impl Drop for DialogGuard {
    fn drop(&mut self) {
        DIALOG_OPEN.store(false, Ordering::SeqCst);
    }
}

/// Show the native settings dialog.  Re-entry safe; spawns a dedicated thread.
pub fn show_settings_dialog() {
    if DIALOG_OPEN.swap(true, Ordering::SeqCst) {
        return; // already open
    }

    std::thread::spawn(|| {
        let _guard = DialogGuard;
        // SAFETY: `run_dialog` creates Win32 windows and runs a message loop.
        // It is called on a dedicated thread (spawned above) with COM already
        // initialised inside the function. The `DialogGuard` RAII ensures
        // `DIALOG_OPEN` is reset even if a panic occurs.
        unsafe { run_dialog() };
    });
}

/// Create the dialog window and run its message loop (must be called on its own thread).
///
/// # Safety
///
/// Must be called on a dedicated thread with no prior COM initialisation.
/// All Win32 handles are valid for the lifetime of the message loop.
unsafe fn run_dialog() {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

    let class_name = w!("RustShotSettingsWnd");

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wnd_proc),
        hInstance: HINSTANCE(0),
        lpszClassName: class_name,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH(COLOR_BTNFACE.0 as isize + 1),
        ..Default::default()
    };
    RegisterClassExW(&wc);

    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        class_name,
        w!("RustShot Settings"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        460,
        300,
        None,
        None,
        None,
        None,
    );

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    CoUninitialize();
}

// ---------------------------------------------------------------------------
// Window procedure
// ---------------------------------------------------------------------------

/// Win32 window procedure callback.
///
/// # Safety
///
/// Called by the OS message dispatcher. `hwnd` is guaranteed valid for the
/// lifetime of the window. `wparam`/`lparam` semantics depend on `msg`.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            create_controls(hwnd);
            populate_controls(hwnd, &load_settings());
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            handle_command(hwnd, id);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Open a folder picker and write the selected path into the given edit control.
///
/// # Safety
///
/// `hwnd` must be a valid window handle. `edit_id` must identify an existing
/// child EDIT control.
unsafe fn handle_browse(hwnd: HWND, edit_id: i32) {
    if let Some(path) = browse_folder(hwnd) {
        set_control_text(hwnd, edit_id, &path.display().to_string());
    }
}

/// Dispatch a WM_COMMAND message by control ID.
///
/// # Safety
///
/// `hwnd` must be a valid dialog window containing the expected child controls.
unsafe fn handle_command(hwnd: HWND, id: i32) {
    match id {
        IDC_SCREENSHOTS_BROWSE => handle_browse(hwnd, IDC_SCREENSHOTS_EDIT),
        IDC_RECORDINGS_BROWSE => handle_browse(hwnd, IDC_RECORDINGS_EDIT),
        IDC_SAVE_BTN => match read_controls(hwnd) {
            Ok(settings) => {
                if let Err(e) = save_settings(&settings) {
                    show_error(hwnd, &format!("Failed to save: {e}"));
                } else {
                    crate::sync_autorun(settings.auto_start);
                    let _ = DestroyWindow(hwnd);
                }
            }
            Err(e) => show_error(hwnd, &e),
        },
        IDC_CANCEL_BTN => {
            let _ = DestroyWindow(hwnd);
        }
        _ => {}
    }
}

/// Display a warning message box owned by the dialog.
///
/// # Safety
///
/// `hwnd` must be a valid window handle (used as the message-box owner).
unsafe fn show_error(hwnd: HWND, message: &str) {
    let msg_w = wide(message);
    let title_w = wide("RustShot");
    MessageBoxW(
        hwnd,
        PCWSTR(msg_w.as_ptr()),
        PCWSTR(title_w.as_ptr()),
        MB_OK | MB_ICONWARNING,
    );
}

// ---------------------------------------------------------------------------
// Control creation and data binding
// ---------------------------------------------------------------------------

/// Create all child controls (labels, edits, combo, checkbox, buttons).
///
/// # Safety
///
/// `hwnd` must be a valid parent window. Win32 `CreateWindowExW` and
/// `SendMessageW` calls require valid handles, which are checked
/// immediately after creation.
unsafe fn create_controls(hwnd: HWND) {
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
                hwnd,
                HMENU(id as isize),
                None,
                None,
            );
            if ctrl.0 != 0 {
                SendMessageW(ctrl, WM_SETFONT_MSG, WPARAM(font.0 as usize), LPARAM(1));
                ctrl
            } else {
                HWND::default()
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
            WPARAM(0),
            LPARAM(item_w.as_ptr() as isize),
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
unsafe fn populate_controls(hwnd: HWND, settings: &Settings) {
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
    let combo = GetDlgItem(hwnd, IDC_FORMAT_COMBO);
    if combo.0 != 0 {
        let idx = Format::ALL
            .iter()
            .position(|&f| f == settings.default_format)
            .unwrap_or(0);
        SendMessageW(combo, CB_SETCURSEL, WPARAM(idx), LPARAM(0));
    }

    // FPS
    set_control_text(hwnd, IDC_FPS_EDIT, &settings.recording_fps.to_string());

    // Auto-start checkbox
    let check = GetDlgItem(hwnd, IDC_AUTOSTART_CHECK);
    if check.0 != 0 {
        let state = if settings.auto_start { BST_CHECKED } else { 0 };
        SendMessageW(check, BM_SETCHECK, WPARAM(state), LPARAM(0));
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
fn build_settings_from_inputs(
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
unsafe fn read_controls(hwnd: HWND) -> Result<Settings, String> {
    let screenshots_dir = get_control_text(hwnd, IDC_SCREENSHOTS_EDIT);
    let recordings_dir = get_control_text(hwnd, IDC_RECORDINGS_EDIT);

    let combo = GetDlgItem(hwnd, IDC_FORMAT_COMBO);
    if combo.0 == 0 {
        return Err("Cannot read format control.".to_string());
    }
    let sel = SendMessageW(combo, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as i32;

    let fps_text = get_control_text(hwnd, IDC_FPS_EDIT);

    let check = GetDlgItem(hwnd, IDC_AUTOSTART_CHECK);
    if check.0 == 0 {
        return Err("Cannot read auto-start control.".to_string());
    }
    let checked = SendMessageW(check, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 as usize;
    let auto_start = checked == BST_CHECKED;

    build_settings_from_inputs(
        &screenshots_dir,
        &recordings_dir,
        sel,
        &fps_text,
        auto_start,
    )
}

// ---------------------------------------------------------------------------
// Folder picker (IFileOpenDialog + FOS_PICKFOLDERS)
// ---------------------------------------------------------------------------

/// Show a folder-picker dialog (IFileOpenDialog with FOS_PICKFOLDERS).
///
/// # Safety
///
/// `owner` must be a valid window handle. COM must be initialised on the
/// current thread (done in [`run_dialog`]). The `PWSTR` from
/// `GetDisplayName` is freed via `CoTaskMemFree` before returning.
unsafe fn browse_folder(owner: HWND) -> Option<PathBuf> {
    let dialog: IFileOpenDialog =
        CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;

    let mut options = dialog.GetOptions().ok()?;
    options.0 |= FOS_PICKFOLDERS;
    dialog.SetOptions(options).ok()?;

    // Show returns HRESULT_FROM_WIN32(ERROR_CANCELLED) when user cancels.
    dialog.Show(owner).ok()?;

    let result = dialog.GetResult().ok()?;
    let display_name = result.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
    let path_str = display_name.to_string().ok();
    CoTaskMemFree(Some(display_name.0 as *const core::ffi::c_void));

    Some(PathBuf::from(path_str?))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Set the text of a child control by ID.
///
/// # Safety
///
/// `parent` must be a valid window containing a child with the given `id`.
unsafe fn set_control_text(parent: HWND, id: i32, text: &str) {
    let ctrl = GetDlgItem(parent, id);
    if ctrl.0 != 0 {
        let text_w = wide(text);
        let _ = SetWindowTextW(ctrl, PCWSTR(text_w.as_ptr()));
    }
}

/// Read the text of a child control by ID.
///
/// # Safety
///
/// `parent` must be a valid window. Returns an empty string if the control
/// does not exist or has no text.
unsafe fn get_control_text(parent: HWND, id: i32) -> String {
    let ctrl = GetDlgItem(parent, id);
    if ctrl.0 == 0 {
        return String::new();
    }
    let len = GetWindowTextLengthW(ctrl);
    if len == 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    GetWindowTextW(ctrl, &mut buf);
    String::from_utf16_lossy(&buf[..len as usize])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- wide() tests (function lives in settings.rs, tested here for Win32 context) ---

    #[test]
    fn wide_helper_null_terminated() {
        let v = wide("abc");
        assert_eq!(v, vec![b'a' as u16, b'b' as u16, b'c' as u16, 0]);
    }

    #[test]
    fn wide_helper_ascii() {
        let v = wide("hello");
        assert_eq!(v.len(), 6); // 5 chars + null
        assert_eq!(*v.last().unwrap(), 0);
    }

    #[test]
    fn wide_helper_empty() {
        let v = wide("");
        assert_eq!(v, vec![0u16]);
    }

    #[test]
    fn wide_helper_unicode() {
        let v = wide("\u{00E9}"); // e-acute
        assert_eq!(v.len(), 2); // 1 char + null
        assert_eq!(v[0], 0x00E9);
        assert_eq!(v[1], 0);
    }

    #[test]
    fn wide_helper_surrogate_pair() {
        let v = wide("\u{1F600}"); // grinning face emoji
        assert_eq!(v.len(), 3); // 2 UTF-16 code units + null
        assert_eq!(v[2], 0);
    }

    // --- validate_fps_input ---

    #[test]
    fn validate_fps_input_zero_is_error() {
        assert!(validate_fps_input("0").is_err());
    }

    #[test]
    fn validate_fps_input_valid_30() {
        assert_eq!(validate_fps_input("30").map(Fps::get), Ok(30));
    }

    #[test]
    fn validate_fps_input_non_numeric_is_error() {
        assert!(validate_fps_input("abc").is_err());
    }

    #[test]
    fn validate_fps_input_empty_is_error() {
        assert!(validate_fps_input("").is_err());
    }

    // --- format_index_to_name ---

    #[test]
    fn format_index_to_name_all_valid_indices() {
        assert_eq!(format_index_to_name(0), "png");
        assert_eq!(format_index_to_name(1), "jpg");
        assert_eq!(format_index_to_name(2), "bmp");
    }

    #[test]
    fn format_index_to_name_out_of_range_defaults_to_png() {
        assert_eq!(format_index_to_name(99), "png");
        assert_eq!(format_index_to_name(-1), "png");
    }

    // --- validate_fps_input edge cases ---

    #[test]
    fn validate_fps_input_one_is_ok() {
        assert_eq!(validate_fps_input("1").map(Fps::get), Ok(1));
    }

    #[test]
    fn validate_fps_input_large_value_is_ok() {
        assert_eq!(validate_fps_input("240").map(Fps::get), Ok(240));
    }

    #[test]
    fn validate_fps_input_above_max_is_error() {
        assert!(validate_fps_input("241").is_err());
        assert!(validate_fps_input("4294967295").is_err());
    }

    #[test]
    fn validate_fps_input_at_max_is_ok() {
        assert_eq!(validate_fps_input("240").map(Fps::get), Ok(240));
    }

    #[test]
    fn validate_fps_input_overflow_is_error() {
        assert!(validate_fps_input("4294967296").is_err());
    }

    #[test]
    fn validate_fps_input_negative_is_error() {
        assert!(validate_fps_input("-1").is_err());
    }

    #[test]
    fn validate_fps_input_leading_zeros_accepted() {
        assert_eq!(validate_fps_input("030").map(Fps::get), Ok(30));
    }

    #[test]
    fn validate_fps_input_whitespace_is_error() {
        assert!(validate_fps_input(" 30").is_err());
    }

    #[test]
    fn validate_fps_input_float_is_error() {
        assert!(validate_fps_input("30.5").is_err());
    }

    #[test]
    fn validate_fps_input_error_message_contains_positive() {
        let err = validate_fps_input("0").unwrap_err();
        assert!(err.contains("positive"), "error: {err}");
    }

    // --- format_index_to_name edge cases ---

    #[test]
    fn format_index_to_name_i32_max_defaults_to_png() {
        assert_eq!(format_index_to_name(i32::MAX), "png");
    }

    #[test]
    fn format_index_to_name_i32_min_defaults_to_png() {
        assert_eq!(format_index_to_name(i32::MIN), "png");
    }

    #[test]
    fn format_index_to_name_matches_supported_formats() {
        for (i, &fmt) in Format::ALL.iter().enumerate() {
            assert_eq!(format_index_to_format(i as i32), fmt);
            assert_eq!(format_index_to_name(i as i32), fmt.extension());
        }
    }

    // --- build_settings_from_inputs ---

    #[test]
    fn build_settings_from_inputs_accepts_valid_values() {
        let settings =
            build_settings_from_inputs("C:\\shots", "C:\\records", 1, "60", true).unwrap();

        assert_eq!(settings.screenshots_dir, PathBuf::from("C:\\shots"));
        assert_eq!(settings.recordings_dir, PathBuf::from("C:\\records"));
        assert_eq!(settings.default_format, Format::Jpg);
        assert_eq!(settings.recording_fps.get(), 60);
        assert!(settings.auto_start);
    }

    #[test]
    fn build_settings_from_inputs_rejects_empty_screenshots_dir() {
        let err = build_settings_from_inputs("", "C:\\records", 0, "30", false).unwrap_err();
        assert_eq!(err, "Screenshots folder cannot be empty.");
    }

    #[test]
    fn build_settings_from_inputs_rejects_empty_recordings_dir() {
        let err = build_settings_from_inputs("C:\\shots", "", 0, "30", false).unwrap_err();
        assert_eq!(err, "Recordings folder cannot be empty.");
    }

    #[test]
    fn build_settings_from_inputs_defaults_invalid_format_index_to_png() {
        let settings =
            build_settings_from_inputs("C:\\shots", "C:\\records", i32::MAX, "30", false).unwrap();

        assert_eq!(settings.default_format, Format::Png);
    }

    #[test]
    fn build_settings_from_inputs_propagates_fps_validation_error() {
        let err =
            build_settings_from_inputs("C:\\shots", "C:\\records", 2, "0", false).unwrap_err();
        assert_eq!(err, "Recording FPS must be a positive number.");
    }

    #[test]
    fn build_settings_from_inputs_negative_format_index_defaults_to_png() {
        let settings =
            build_settings_from_inputs("C:\\shots", "C:\\records", -1, "30", false).unwrap();

        assert_eq!(settings.default_format, Format::Png);
    }

    #[test]
    fn build_settings_from_inputs_preserves_false_auto_start() {
        let settings =
            build_settings_from_inputs("C:\\shots", "C:\\records", 2, "30", false).unwrap();

        assert_eq!(settings.default_format, Format::Bmp);
        assert!(!settings.auto_start);
    }

    #[test]
    fn build_settings_from_inputs_accepts_max_recording_fps() {
        let max_fps = Fps::MAX.to_string();
        let settings =
            build_settings_from_inputs("C:\\shots", "C:\\records", 0, &max_fps, false).unwrap();

        assert_eq!(settings.default_format, Format::Png);
        assert_eq!(settings.recording_fps.get(), Fps::MAX);
    }

    #[test]
    fn build_settings_from_inputs_preserves_unicode_paths() {
        let settings =
            build_settings_from_inputs("C:\\スクリーン", "C:\\Записи", 1, "30", true).unwrap();

        assert_eq!(settings.screenshots_dir, PathBuf::from("C:\\スクリーン"));
        assert_eq!(settings.recordings_dir, PathBuf::from("C:\\Записи"));
        assert_eq!(settings.default_format, Format::Jpg);
    }

    #[test]
    fn validate_fps_input_above_max_reports_limit() {
        let err = validate_fps_input("241").unwrap_err();
        assert_eq!(err, format!("Recording FPS must not exceed {}.", Fps::MAX));
    }

    // --- DialogGuard ---

    #[test]
    fn dialog_guard_resets_flag_on_drop() {
        DIALOG_OPEN.store(true, Ordering::SeqCst);
        {
            let _guard = DialogGuard;
        }
        assert!(!DIALOG_OPEN.load(Ordering::SeqCst));
    }

    #[test]
    fn dialog_guard_resets_flag_when_unwinding() {
        DIALOG_OPEN.store(true, Ordering::SeqCst);
        let result = std::panic::catch_unwind(|| {
            let _guard = DialogGuard;
            panic!("test panic");
        });
        assert!(result.is_err());
        assert!(!DIALOG_OPEN.load(Ordering::SeqCst));
    }

    #[test]
    fn show_settings_dialog_returns_early_when_dialog_is_already_open() {
        DIALOG_OPEN.store(true, Ordering::SeqCst);
        show_settings_dialog();
        assert!(DIALOG_OPEN.load(Ordering::SeqCst));
        DIALOG_OPEN.store(false, Ordering::SeqCst);
    }

    // --- FOS_PICKFOLDERS constant ---

    #[test]
    fn fos_pickfolders_value_is_0x20() {
        assert_eq!(FOS_PICKFOLDERS, 0x20);
    }
}
