//! Native Win32 settings dialog (Windows-only).
//!
//! Opens a modal window with controls for all
//! [`Settings`](crate::settings::Settings) fields.
//! Called from the tray menu "Settings" item instead of opening
//! `settings.json` in notepad.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_BTNFACE, HBRUSH};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::settings::{load_settings, save_settings, wide};

mod controls;
pub(crate) use controls::*;

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
mod tests;
