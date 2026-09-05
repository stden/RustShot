//! Native Win32 settings dialog (Windows-only).
//!
//! Opens a modal window with controls for all
//! [`Settings`](crate::settings::Settings) fields.
//! Called from the tray menu "Settings" item instead of opening
//! `settings.json` in notepad.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::{w, Error, PCWSTR};
use windows::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::{COLOR_BTNFACE, HBRUSH};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
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

/// Geometry shared by top-level window creation and every child-control row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SettingsDialogLayout {
    dialog_width: i32,
    dialog_height: i32,
    field_x: i32,
    path_edit_width: i32,
    format_field_width: i32,
    fps_field_width: i32,
    field_height: i32,
    browse_x: i32,
    browse_width: i32,
    button_height: i32,
    font_point_size: i32,
}

/// Return the dialog dimensions in Win32 logical pixels at the process DPI.
///
/// A function, rather than scattered constants, keeps the wider fields and
/// their surrounding controls on one production-used layout contract.
fn settings_dialog_layout() -> SettingsDialogLayout {
    SettingsDialogLayout {
        dialog_width: 610,
        dialog_height: 320,
        field_x: 150,
        path_edit_width: 350,
        format_field_width: 160,
        fps_field_width: 110,
        field_height: 27,
        browse_x: 510,
        browse_width: 70,
        button_height: 30,
        font_point_size: 11,
    }
}

/// Convert point size to the negative character height expected by
/// `CreateFontW`, rounded to the nearest pixel at the baseline 96 DPI.
fn font_pixel_height(point_size: i32) -> i32 {
    -((point_size * 96 + 36) / 72)
}

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

/// Balances a successful `CoInitializeEx` on the dialog thread.
struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: `ComGuard` is constructed only after `CoInitializeEx`
        // succeeds and never leaves that thread, so this is the matching
        // uninitialisation on the same apartment.
        unsafe { CoUninitialize() };
    }
}

/// Show the native settings dialog.  Re-entry safe; spawns a dedicated thread.
pub(crate) fn show_settings_dialog() {
    claim_dialog_slot_with(|| {
        std::thread::Builder::new()
            .name("rustshot-settings-dialog".to_string())
            .spawn(|| {
                let _guard = DialogGuard;
                // SAFETY: `run_dialog` creates Win32 windows and runs a message
                // loop. It is called on a dedicated thread (spawned here) with
                // COM initialised inside the function. The `DialogGuard` RAII
                // ensures `DIALOG_OPEN` is reset even if a panic occurs.
                if let Err(error) = unsafe { run_dialog() } {
                    log::error!("Failed to open the settings dialog: {error}");
                }
            })
            .map(|_| ())
    });
}

/// Claim the single dialog slot and hand it to `spawn_dialog`.
///
/// Returns whether a dialog was started. The slot is released again when the
/// thread cannot be started, because [`DialogGuard`] only exists *inside* that
/// thread: without this the flag would stay set for the rest of the process and
/// the Settings menu item would be dead until restart. Note also the use of
/// `Builder::spawn` at the call site — plain `thread::spawn` panics when the OS
/// refuses a thread, which would unwind through the tray event loop instead of
/// being reported here.
pub(crate) fn claim_dialog_slot_with<Spawn, E>(spawn_dialog: Spawn) -> bool
where
    Spawn: FnOnce() -> Result<(), E>,
    E: std::fmt::Display,
{
    if DIALOG_OPEN.swap(true, Ordering::SeqCst) {
        return false; // already open
    }

    if let Err(error) = spawn_dialog() {
        log::error!("Failed to open the settings dialog: {error}");
        DIALOG_OPEN.store(false, Ordering::SeqCst);
        return false;
    }

    true
}

/// Create the dialog window and run its message loop (must be called on its own thread).
///
/// # Safety
///
/// Must be called on a dedicated thread with no prior COM initialisation.
/// All Win32 handles are valid for the lifetime of the message loop.
unsafe fn run_dialog() -> Result<(), String> {
    // SAFETY: the contract above requires a thread with no prior COM
    // initialisation, so requesting an STA cannot collide with an existing
    // apartment of a different model — a failed initialisation does not take a
    // reference on the apartment and therefore must not construct `ComGuard`.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|error| format!("Failed to initialise COM: {error}"))?;
    let _com_guard = ComGuard;

    // SAFETY: a null module name asks for the file used to create this process;
    // it does not dereference a caller-provided pointer.
    // Application-local window classes are owned and looked up by module
    // instance. Supplying the real executable module makes the registration
    // and creation pair unambiguous instead of relying on Win32's treatment of
    // a null instance handle.
    let module = unsafe { GetModuleHandleW(None) }
        .map_err(|error| format!("Failed to get the executable module: {error}"))?;
    let instance = HINSTANCE(module.0);

    let class_name = w!("RustShotSettingsWnd");

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wnd_proc),
        hInstance: instance,
        lpszClassName: class_name,
        // SAFETY: `IDC_ARROW` is one of the predefined cursor atoms, which is
        // exactly what `LoadCursorW` expects when no module handle is supplied.
        // The handle it returns is system-owned, so nothing here must free it.
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        hbrBackground: HBRUSH((COLOR_BTNFACE.0 as usize + 1) as *mut core::ffi::c_void),
        ..Default::default()
    };
    // SAFETY: `wc` is fully initialised and `cbSize` is set from the struct's
    // own size, which is how the API decides what it may read. Both `w!`
    // pointers it holds (`lpszClassName`) are static literals, so the class
    // name outlives the registration, and `wnd_proc` has the WNDPROC signature.
    let class_atom = unsafe { RegisterClassExW(&wc) };
    if class_atom == 0 {
        let error = Error::from_thread();
        if WIN32_ERROR::from_error(&error) != Some(ERROR_CLASS_ALREADY_EXISTS) {
            return Err(format!("Failed to register the window class: {error}"));
        }
    }

    // SAFETY: `class_name` was just registered above, so the class exists and
    // its `wnd_proc` will receive this window's messages on this thread. The
    // title is a static `w!` literal. `None` for parent/menu/param is the
    // documented way to create an unowned top-level window, while `instance`
    // is the same executable module that owns the class. The returned handle
    // remains live through the message loop and is destroyed by `wnd_proc`.
    let layout = settings_dialog_layout();
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("RustShot Settings"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            layout.dialog_width,
            layout.dialog_height,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|error| format!("Failed to create the window: {error}"))?;

    // SAFETY: `hwnd` was just created on this thread and remains live through
    // the message loop below. `ShowWindow` reports the previous visibility
    // state rather than success, so its return value is intentionally unused.
    // Foreground activation can
    // legitimately be denied by Windows focus-stealing rules; the window is
    // still visible and available from the taskbar in that case.
    let _ = unsafe { ShowWindow(hwnd, SW_SHOWNORMAL) };
    let _ = unsafe { SetForegroundWindow(hwnd) };

    let mut msg = MSG::default();
    // SAFETY: `msg` is a live local that outlives every iteration, so the OS may
    // write the next message into it. A `None` window filter drains this
    // thread's own queue, and the loop ends on WM_QUIT, which `wnd_proc` posts
    // from WM_DESTROY.
    loop {
        let status = unsafe { GetMessageW(&mut msg, None, 0, 0) }.0;
        if status == -1 {
            let error = Error::from_thread();
            // SAFETY: `hwnd` is still owned by this thread because WM_QUIT has
            // not ended the loop; destroy it before abandoning the thread.
            let _ = unsafe { DestroyWindow(hwnd) };
            return Err(format!("Failed to read a window message: {error}"));
        }
        if status == 0 {
            break;
        }
        // SAFETY: `msg` was just filled in by `GetMessageW` and is only read
        // here, so the shared reference is sound; translation adds any character
        // message to this thread's own queue, ahead of the dispatch below.
        let _ = unsafe { TranslateMessage(&msg) };
        // SAFETY: as above for `msg`. Dispatching re-enters `wnd_proc` on the
        // thread that owns the window, which is the only thread allowed to run a
        // window procedure for it.
        unsafe { DispatchMessageW(&msg) };
    }

    Ok(())
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
            // SAFETY: WM_CREATE is delivered from inside `CreateWindowExW`, so
            // `hwnd` is a live window owned by this thread and has no children
            // yet — `create_controls` is what gives it the ones the rest of this
            // module addresses by ID.
            unsafe { create_controls(hwnd) };
            // SAFETY: as above, and the controls `populate_controls` writes to
            // were created by the call on the previous line.
            unsafe { populate_controls(hwnd, &load_settings()) };
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            // SAFETY: `hwnd` is the live dialog window the OS dispatched this
            // message to, and WM_COMMAND can only arrive after WM_CREATE, so
            // the child controls `handle_command` reaches for already exist.
            unsafe { handle_command(hwnd, id) };
            LRESULT(0)
        }
        WM_CLOSE => {
            // SAFETY: `hwnd` is the live window being closed, and it was created
            // on this thread — `DestroyWindow` may only be called from the
            // window's own thread. Nothing touches `hwnd` after this point.
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: `PostQuitMessage` takes no handle and only queues WM_QUIT
            // for this thread, which is the thread running `run_dialog`'s loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // SAFETY: this is the final destruction message for the dialog;
            // `create_controls` stored only its owned custom HFONT, and the
            // child controls that used it no longer outlive this cleanup.
            unsafe { release_dialog_font(hwnd) };
            // SAFETY: the font bookkeeping above consumes no message data, so
            // the OS still receives its original final-destruction arguments.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        // SAFETY: forwarding the OS's own arguments unchanged to the default
        // handler, which is what a window procedure must do for messages it does
        // not handle; `hwnd` is valid for the duration of the call.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Open a folder picker and write the selected path into the given edit control.
///
/// # Safety
///
/// `hwnd` must be a valid window handle. `edit_id` must identify an existing
/// child EDIT control.
unsafe fn handle_browse(hwnd: HWND, edit_id: i32) {
    // SAFETY: `hwnd` is a live window per this function's contract, so it can
    // own the modal picker. The only caller path is `handle_command` from
    // `wnd_proc`, which runs on the `run_dialog` thread, where COM is already
    // initialised as an STA — `browse_folder`'s own requirement.
    if let Some(path) = unsafe { browse_folder(hwnd) } {
        // SAFETY: as above for `hwnd`; `edit_id` is one of the EDIT control IDs
        // created by `create_controls` on WM_CREATE, which necessarily preceded
        // the WM_COMMAND that got us here.
        unsafe { set_control_text(hwnd, edit_id, &path.display().to_string()) };
    }
}

/// Dispatch a WM_COMMAND message by control ID.
///
/// # Safety
///
/// `hwnd` must be a valid dialog window containing the expected child controls.
unsafe fn handle_command(hwnd: HWND, id: i32) {
    match id {
        // SAFETY: `hwnd` is the live dialog window per this function's contract,
        // and the EDIT control the picker writes into was created in the same
        // WM_CREATE pass as the browse button that raised this command.
        IDC_SCREENSHOTS_BROWSE => unsafe { handle_browse(hwnd, IDC_SCREENSHOTS_EDIT) },
        // SAFETY: as above, for the recordings row.
        IDC_RECORDINGS_BROWSE => unsafe { handle_browse(hwnd, IDC_RECORDINGS_EDIT) },
        IDC_SAVE_BTN => {
            // SAFETY: `hwnd` is the live dialog window, and its child controls
            // still exist: this command comes from the Save button, which is
            // itself one of them, and the window is destroyed only below.
            match unsafe { read_controls(hwnd) } {
                Ok(settings) => {
                    if let Err(e) = save_settings(&settings) {
                        // SAFETY: `hwnd` is still alive — it is only destroyed on
                        // the success path — so it can own the message box.
                        unsafe { show_error(hwnd, &format!("Failed to save: {e}")) };
                    } else {
                        match crate::application::autorun::sync_current_exe_autorun(
                            settings.auto_start,
                        ) {
                            Ok(()) => {
                                // SAFETY: `hwnd` is the live window created on this
                                // thread, the only thread allowed to destroy it, and
                                // nothing uses the handle afterwards.
                                let _ = unsafe { DestroyWindow(hwnd) };
                            }
                            // SAFETY: the settings were saved but the dialog is
                            // still alive, so it can report the separate registry
                            // failure and let the user retry.
                            Err(error) => unsafe {
                                show_error(
                                    hwnd,
                                    &format!("Failed to update Windows startup: {error}"),
                                )
                            },
                        }
                    }
                }
                // SAFETY: validation failed before anything was written, so the
                // dialog window is untouched and still valid as the owner.
                Err(e) => unsafe { show_error(hwnd, &e) },
            }
        }
        IDC_CANCEL_BTN => {
            // SAFETY: as for the Save path — a live window being destroyed from
            // its own thread, with no later use of the handle.
            let _ = unsafe { DestroyWindow(hwnd) };
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
    // SAFETY: `wide` appends a terminating NUL, and both buffers are locals that
    // outlive this call — `MessageBoxW` is modal and synchronous, so it reads
    // them before returning. `hwnd` is a live window per the contract above,
    // which is all the owner parameter requires.
    unsafe {
        MessageBoxW(
            Some(hwnd),
            PCWSTR(msg_w.as_ptr()),
            PCWSTR(title_w.as_ptr()),
            MB_OK | MB_ICONWARNING,
        )
    };
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
    // SAFETY: COM is initialised on this thread as an STA by `run_dialog` before
    // the message loop that leads here starts, which is the apartment the shell
    // picker expects; `FileOpenDialog` is an in-process server, so
    // CLSCTX_INPROC_SERVER matches it. A failed activation is reported as an
    // `Err` and turned into `None` rather than yielding a usable pointer.
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }.ok()?;

    // SAFETY: `dialog` is the live interface pointer just activated, used from
    // the same apartment that created it and never sent to another thread.
    // Setting back the flags the object itself reported, plus FOS_PICKFOLDERS,
    // leaves the rest of its option set as the shell chose it.
    let mut options = unsafe { dialog.GetOptions() }.ok()?;
    options.0 |= FOS_PICKFOLDERS;
    // SAFETY: as above.
    unsafe { dialog.SetOptions(options) }.ok()?;

    // Show returns HRESULT_FROM_WIN32(ERROR_CANCELLED) when user cancels.
    // SAFETY: as above for `dialog`; `owner` is a live window per this function's
    // contract and belongs to this thread, so the modal loop the picker runs
    // nests inside our own message loop rather than blocking a foreign one.
    unsafe { dialog.Show(Some(owner)) }.ok()?;

    // SAFETY: reached only after `Show` returned success, i.e. the user
    // confirmed a selection, which is the precondition for `GetResult`.
    let result = unsafe { dialog.GetResult() }.ok()?;
    // SAFETY: `result` is the live IShellItem returned above. SIGDN_FILESYSPATH
    // is meaningful because FOS_PICKFOLDERS restricts the choice to a real
    // file-system folder, and a failure is again mapped to `None`.
    let display_name = unsafe { result.GetDisplayName(SIGDN_FILESYSPATH) }.ok()?;
    // SAFETY: `GetDisplayName` succeeded, so `display_name` points at a
    // NUL-terminated UTF-16 string the shell allocated with CoTaskMemAlloc; it
    // is still allocated here because nothing has freed it yet.
    let path_str = unsafe { display_name.to_string() }.ok();
    // SAFETY: that same CoTaskMemAlloc'd buffer, freed with its matching
    // allocator exactly once. The line above already copied it into an owned
    // `String`, and `display_name` is not read again — including on the
    // conversion-failure path, where `path_str` is simply `None`.
    unsafe { CoTaskMemFree(Some(display_name.0 as *const core::ffi::c_void)) };

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
    // SAFETY: `parent` is a live window per the contract above; `GetDlgItem`
    // only looks a child up by identifier and reports `Err` when there is none,
    // so the `if let` is what establishes that `ctrl` exists.
    if let Ok(ctrl) = unsafe { GetDlgItem(Some(parent), id) } {
        let text_w = wide(text);
        // SAFETY: `ctrl` is the child handle just returned. `wide` appends a
        // terminating NUL, and the buffer outlives this call because
        // `SetWindowTextW` sends WM_SETTEXT synchronously and the control copies
        // the string into its own storage.
        let _ = unsafe { SetWindowTextW(ctrl, PCWSTR(text_w.as_ptr())) };
    }
}

/// Read the text of a child control by ID.
///
/// # Safety
///
/// `parent` must be a valid window. Returns an empty string if the control
/// does not exist or has no text.
unsafe fn get_control_text(parent: HWND, id: i32) -> String {
    // SAFETY: `parent` is a live window per the contract above, and a missing
    // child is reported as `Err` rather than producing a bogus handle.
    let Ok(ctrl) = (unsafe { GetDlgItem(Some(parent), id) }) else {
        return String::new();
    };
    // SAFETY: `ctrl` is the live child handle from the line above.
    let len = unsafe { GetWindowTextLengthW(ctrl) };
    if len == 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    // SAFETY: as above for `ctrl`. The buffer holds `len + 1` `u16`s, room for
    // the length just reported plus the NUL that `GetWindowTextW` always writes,
    // and the capacity it is given is derived from that same slice, so it cannot
    // write past the allocation even if the control's text grew in between.
    unsafe { GetWindowTextW(ctrl, &mut buf) };
    String::from_utf16_lossy(&buf[..len as usize])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "../tests/unit/settings_dialog/mod.rs"]
mod tests;
