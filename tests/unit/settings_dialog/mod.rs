//! Settings validation, lifecycle guards, and native dialog regressions.

use super::*;

use crate::capture::Format;
use crate::settings::Fps;

mod behavior;
mod keyboard;
mod resizing;

#[test]
fn fos_pickfolders_value_is_0x20() {
    assert_eq!(FOS_PICKFOLDERS, 0x20);
}

#[test]
fn settings_dialog_fields_are_wider_than_the_previous_compact_layout() {
    let layout = settings_dialog_layout();

    assert_eq!(layout.dialog_width, 800);
    assert!(layout.path_edit_width >= 530);
    assert!(layout.format_field_width >= 150);
    assert!(layout.fps_field_width >= 100);
    assert_eq!(
        layout.field_x + layout.path_edit_width + 10,
        layout.browse_x
    );
    assert!(
        layout.dialog_width - (layout.browse_x + layout.browse_width) >= 30,
        "the outer width must leave room for the non-client frame and a right margin"
    );
}

#[test]
fn settings_dialog_uses_a_larger_font_with_room_for_its_text() {
    let layout = settings_dialog_layout();

    assert_eq!(layout.font_point_size, 11);
    assert_eq!(font_pixel_height(layout.font_point_size), -15);
    assert!(layout.field_height >= 27);
    assert!(layout.button_height >= 30);
}

// --- validate_fps_input ---

#[test]
fn validate_fps_input_non_numeric_is_error() {
    assert!(validate_fps_input("abc").is_err());
}

// --- format_index_to_name ---

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
fn validate_fps_input_at_max_is_ok() {
    assert_eq!(validate_fps_input("240").map(Fps::get), Ok(240));
}

#[test]
fn validate_fps_input_overflow_is_error() {
    assert!(validate_fps_input("4294967296").is_err());
}

#[test]
fn validate_fps_input_leading_zeros_accepted() {
    assert_eq!(validate_fps_input("030").map(Fps::get), Ok(30));
}

#[test]
fn validate_fps_input_error_message_contains_positive() {
    let err = validate_fps_input("0").unwrap_err();
    assert!(err.contains("positive"), "error: {err}");
}

// --- format_index_to_name edge cases ---

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
    let settings = build_settings_from_inputs("C:\\shots", "C:\\records", 1, "60", true).unwrap();

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
    let err = build_settings_from_inputs("C:\\shots", "C:\\records", 2, "0", false).unwrap_err();
    assert_eq!(err, "Recording FPS must be a positive number.");
}

#[test]
fn build_settings_from_inputs_preserves_false_auto_start() {
    let settings = build_settings_from_inputs("C:\\shots", "C:\\records", 2, "30", false).unwrap();

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

/// Serializes the tests that mutate the process-global [`DIALOG_OPEN`]
/// flag. `cargo test` runs tests on multiple threads, so without this lock
/// one test's store races another's assertion — an intermittent failure
/// that only shows up under parallelism (exactly how CI runs). Same pattern
/// as `GLOBAL_OVERRIDES_LOCK` in `settings.rs`.
static DIALOG_FLAG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A panicking test poisons the mutex; recover the guard so one failure
/// cannot cascade into failing the whole group.
fn lock_dialog_flag() -> std::sync::MutexGuard<'static, ()> {
    DIALOG_FLAG_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn dialog_guard_resets_flag_when_unwinding() {
    let _serial = lock_dialog_flag();
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
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(true, Ordering::SeqCst);
    show_settings_dialog();
    assert!(DIALOG_OPEN.load(Ordering::SeqCst));
    DIALOG_OPEN.store(false, Ordering::SeqCst);
}

#[test]
fn dialog_slot_is_released_when_the_thread_cannot_start() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);

    // The OS refusing a thread must not strand the flag: `DialogGuard` lives
    // inside the thread that never ran, so nothing else would clear it and the
    // Settings menu item would stay dead for the rest of the process.
    let started = claim_dialog_slot_with(|| Err::<(), _>("no threads available"));

    assert!(!started);
    assert!(
        !DIALOG_OPEN.load(Ordering::SeqCst),
        "a failed spawn left the dialog permanently locked out"
    );

    // Proof the slot is genuinely reusable afterwards.
    let mut spawned = false;
    let started = claim_dialog_slot_with(|| {
        spawned = true;
        Ok::<(), &str>(())
    });
    assert!(started && spawned);
    assert!(DIALOG_OPEN.load(Ordering::SeqCst));
    DIALOG_OPEN.store(false, Ordering::SeqCst);
}

#[test]
fn dialog_slot_admits_only_one_holder() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);

    assert!(claim_dialog_slot_with(|| Ok::<(), &str>(())));

    // A second request while the first still holds the slot must not spawn.
    let mut spawned_again = false;
    let started = claim_dialog_slot_with(|| {
        spawned_again = true;
        Ok::<(), &str>(())
    });
    assert!(!started);
    assert!(!spawned_again, "a second dialog thread was started");

    DIALOG_OPEN.store(false, Ordering::SeqCst);
}

fn current_process_settings_window() -> Result<Option<HWND>, windows::core::Error> {
    // FindWindowW returns only its first global match: an installed RustShot
    // dialog can otherwise hide the test process's own window from the test.
    unsafe extern "system" fn find_ours(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let mut process_id = 0;
        // SAFETY: EnumWindows supplies a live handle; the output stays writable.
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
        if process_id != std::process::id() {
            return windows::core::BOOL(1);
        }
        let mut class_name = [0u16; 64];
        let mut title = [0u16; 64];
        // SAFETY: hwnd is from enumeration, and both APIs derive their output
        // capacity from these live slices without retaining them.
        let class_len = unsafe { GetClassNameW(hwnd, &mut class_name) } as usize;
        // SAFETY: as above, `title` is live and its slice bounds the write.
        let title_len = unsafe { GetWindowTextW(hwnd, &mut title) } as usize;
        if class_name[..class_len]
            .iter()
            .copied()
            .eq("RustShotSettingsWnd".encode_utf16())
            && title[..title_len]
                .iter()
                .copied()
                .eq("RustShot Settings".encode_utf16())
        {
            // SAFETY: the only caller passes a live Option<HWND> through
            // lparam, exclusively borrowed for this synchronous enumeration.
            unsafe { *(lparam.0 as *mut Option<HWND>) = Some(hwnd) };
        }
        windows::core::BOOL(1)
    }

    let mut found = None;
    // SAFETY: find_ours has the native callback signature, and found remains
    // alive and exclusively borrowed until EnumWindows returns.
    unsafe {
        EnumWindows(
            Some(find_ours),
            LPARAM(std::ptr::from_mut(&mut found) as isize),
        )
    }?;
    Ok(found)
}

fn wait_for_settings_window() -> Option<HWND> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if let Some(hwnd) =
            current_process_settings_window().expect("enumerate settings windows in this process")
        {
            // SAFETY: these handle queries fail safely if the window closes.
            if unsafe { IsWindowVisible(hwnd) }.as_bool() && !unsafe { IsIconic(hwnd) }.as_bool() {
                return Some(hwnd);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    None
}

struct CloseSettingsWindowOnDrop;

impl Drop for CloseSettingsWindowOnDrop {
    fn drop(&mut self) {
        match current_process_settings_window() {
            Ok(Some(hwnd)) => {
                // A hidden-window assertion must not leave its worker and dialog
                // slot alive, poisoning subsequent tests in the same process.
                let _ = close_settings_window(hwnd);
            }
            Ok(None) => {}
            // Cleanup must not panic again while an assertion is unwinding.
            Err(error) => eprintln!("could not find settings dialog for cleanup: {error}"),
        }
    }
}

fn close_settings_window(hwnd: HWND) -> bool {
    // Posting rather than calling `DestroyWindow` is required because the
    // dialog belongs to its worker thread, not this test thread.
    // SAFETY: `hwnd` is the live handle returned by `wait_for_settings_window`;
    // WM_CLOSE carries no pointer arguments.
    if unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_err() {
        return false;
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if !DIALOG_OPEN.load(Ordering::SeqCst) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    false
}

fn control_width(hwnd: HWND, id: i32) -> Option<i32> {
    // SAFETY: callers keep `hwnd` alive for this synchronous lookup. Both APIs
    // fail cleanly if the requested child was not created.
    let control = unsafe { GetDlgItem(Some(hwnd), id) }.ok()?;
    let mut rect = windows::Win32::Foundation::RECT::default();
    // SAFETY: `control` belongs to the live dialog and `rect` stays writable.
    unsafe { GetWindowRect(control, &mut rect) }.ok()?;
    Some(rect.right - rect.left)
}

fn control_font(hwnd: HWND, id: i32) -> Option<usize> {
    // SAFETY: callers keep the dialog alive, `GetDlgItem` establishes a live
    // child, and WM_GETFONT returns a handle value without crossing pointers.
    let control = unsafe { GetDlgItem(Some(hwnd), id) }.ok()?;
    // SAFETY: the live child handles WM_GETFONT without pointer arguments.
    let font = unsafe { SendMessageW(control, WM_GETFONT, None, None) }.0 as usize;
    (font != 0).then_some(font)
}

#[test]
fn settings_dialog_opens_visible_controls_and_can_reopen() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);

    for attempt in 1..=2 {
        let _close_on_failure = CloseSettingsWindowOnDrop;
        show_settings_dialog();
        let hwnd = wait_for_settings_window()
            .unwrap_or_else(|| panic!("attempt {attempt}: settings window did not appear"));

        // SAFETY: the window is live until WM_CLOSE is posted below. A missing
        // Save control is reported as `Err`, not as an invalid usable handle.
        let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
        // SAFETY: `hwnd` remains live until the close request below; the lookup
        // reports a missing child as `Err` without creating a usable handle.
        let has_save_button = unsafe { GetDlgItem(Some(hwnd), IDC_SAVE_BTN) }.is_ok();
        let folder_width = control_width(hwnd, IDC_SCREENSHOTS_EDIT);
        let mut client = windows::Win32::Foundation::RECT::default();
        // SAFETY: the dialog is live until the close request below, and client
        // remains writable for this synchronous geometry query.
        unsafe { GetClientRect(hwnd, &mut client) }.expect("settings client bounds");
        let format_width = control_width(hwnd, IDC_FORMAT_COMBO);
        let fps_width = control_width(hwnd, IDC_FPS_EDIT);
        let fonts = [
            IDC_SCREENSHOTS_EDIT,
            IDC_SCREENSHOTS_BROWSE,
            IDC_RECORDINGS_EDIT,
            IDC_RECORDINGS_BROWSE,
            IDC_FORMAT_COMBO,
            IDC_FPS_EDIT,
            IDC_AUTOSTART_CHECK,
            IDC_SAVE_BTN,
            IDC_CANCEL_BTN,
        ]
        .map(|id| control_font(hwnd, id));
        let closed = close_settings_window(hwnd);

        assert!(visible, "attempt {attempt}: settings window was hidden");
        assert!(
            has_save_button,
            "attempt {attempt}: WM_CREATE did not build the dialog controls"
        );
        let layout = settings_dialog_layout();
        assert_eq!(
            folder_width,
            Some(client.right - client.left - layout.field_x - layout.browse_width - 10 - 20),
            "folder field must fill the client area while preserving the browse gap and margin"
        );
        assert_eq!(format_width, Some(layout.format_field_width));
        assert_eq!(fps_width, Some(layout.fps_field_width));
        assert!(
            fonts.iter().all(Option::is_some),
            "attempt {attempt}: at least one control did not receive the dialog font"
        );
        assert!(
            fonts.windows(2).all(|pair| pair[0] == pair[1]),
            "attempt {attempt}: controls did not share one consistent dialog font"
        );
        assert!(closed, "attempt {attempt}: settings dialog did not close");
    }
}

#[test]
fn settings_request_restores_a_hidden_or_minimized_existing_dialog() {
    let _serial = lock_dialog_flag();
    DIALOG_OPEN.store(false, Ordering::SeqCst);
    let _close_on_failure = CloseSettingsWindowOnDrop;
    show_settings_dialog();
    let hwnd = wait_for_settings_window().expect("initial settings dialog must appear");

    for state in [SW_HIDE, SW_MINIMIZE] {
        // SAFETY: the dialog belongs to this process and remains alive until
        // the cleanup guard posts WM_CLOSE on its owning thread.
        let _ = unsafe { ShowWindow(hwnd, state) };
        // SAFETY: both queries use the same live, test-owned dialog handle.
        let was_hidden_or_minimized =
            !unsafe { IsWindowVisible(hwnd) }.as_bool() || unsafe { IsIconic(hwnd) }.as_bool();
        assert!(
            was_hidden_or_minimized,
            "fixture must hide or minimize the dialog"
        );

        show_settings_dialog();

        assert_eq!(
            wait_for_settings_window(),
            Some(hwnd),
            "requesting Settings must restore the same existing dialog from {state:?}"
        );
    }
}
