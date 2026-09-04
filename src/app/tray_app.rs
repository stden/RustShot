//! Tray-mode application: global hotkeys, tray menu dispatch, and the winit
//! event-loop handler.

use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use log::{debug, error, info};
use std::path::Path;
use tray_icon::{menu::MenuEvent, MouseButton, MouseButtonState, TrayIconEvent};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use super::cli_mode::settings_capture_format;
use super::runtime::{copy_image_to_clipboard, stop_flag_path};
#[cfg(windows)]
use crate::settings_dialog;
use crate::{capture, cli, settings, tray};

/// External tray-mode events forwarded into winit so `ControlFlow::Wait`
/// wakes as soon as a menu item or global hotkey is activated.
pub(crate) enum RustShotEvent {
    HotKey {
        event: GlobalHotKeyEvent,
        /// Foreground target sampled in the hotkey callback, before focus can move.
        foreground_window: Option<Result<capture::WindowTarget, String>>,
    },
    Menu {
        event: MenuEvent,
        /// Foreground target sampled before tray-icon foregrounds its popup owner.
        foreground_window: Result<capture::WindowTarget, String>,
    },
}

/// Attach the foreground window observed at hotkey-delivery time to the event.
pub(crate) fn hotkey_event_with_foreground<GetForeground>(
    event: GlobalHotKeyEvent,
    get_foreground: GetForeground,
) -> RustShotEvent
where
    GetForeground: FnOnce() -> Result<capture::WindowTarget, String>,
{
    let foreground_window = (event.state == HotKeyState::Pressed).then(get_foreground);
    RustShotEvent::HotKey {
        event,
        foreground_window,
    }
}

pub(crate) type PendingTrayWindowTarget =
    std::sync::Mutex<Option<Result<capture::WindowTarget, String>>>;

/// Sample the foreground before tray-icon gives its hidden popup owner focus.
pub(crate) fn remember_tray_window_target_with<GetForeground>(
    pending: &PendingTrayWindowTarget,
    get_foreground: GetForeground,
) where
    GetForeground: FnOnce() -> Result<capture::WindowTarget, String>,
{
    let target = get_foreground();
    *pending
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(target);
}

fn clear_tray_window_target(pending: &PendingTrayWindowTarget) {
    *pending
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// Pair one menu selection with the pre-popup foreground target exactly once.
pub(crate) fn take_tray_window_target(
    pending: &PendingTrayWindowTarget,
) -> Result<capture::WindowTarget, String> {
    pending
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
        .unwrap_or_else(|| {
            Err("No foreground window was recorded before the tray menu opened".to_string())
        })
}

/// Route process-wide menu and hotkey callbacks into the winit event loop.
pub(crate) fn install_event_handlers(proxy: EventLoopProxy<RustShotEvent>) {
    let hotkey_proxy = proxy.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event| {
        let event = hotkey_event_with_foreground(event, capture::foreground_window_target);
        if let Err(error) = hotkey_proxy.send_event(event) {
            debug!("Hotkey arrived after the event loop closed: {error}");
        }
    }));

    let pending_tray_target = std::sync::Arc::new(PendingTrayWindowTarget::new(None));
    let click_target = std::sync::Arc::clone(&pending_tray_target);
    TrayIconEvent::set_event_handler(Some(move |event| {
        let TrayIconEvent::Click {
            button,
            button_state,
            ..
        } = event
        else {
            return;
        };
        if !matches!(button, MouseButton::Left | MouseButton::Right) {
            return;
        }

        match button_state {
            MouseButtonState::Down => clear_tray_window_target(&click_target),
            // tray-icon 0.24 emits this synchronously before calling
            // SetForegroundWindow for its hidden TrackPopupMenu owner.
            MouseButtonState::Up => {
                remember_tray_window_target_with(&click_target, capture::foreground_window_target)
            }
        }
    }));

    let menu_target = pending_tray_target;
    MenuEvent::set_event_handler(Some(move |event| {
        let foreground_window = take_tray_window_target(&menu_target);
        if let Err(error) = proxy.send_event(RustShotEvent::Menu {
            event,
            foreground_window,
        }) {
            debug!("Tray menu action arrived after the event loop closed: {error}");
        }
    }));
}

/// Main application state for the tray/event-loop mode.
///
/// Owns the tray icon, global hotkey registrations, and handles to any
/// spawned child processes (selection overlay, recording session).
pub(crate) struct RustShotApp {
    _tray: Option<tray_icon::TrayIcon>,
    _hotkey_manager: GlobalHotKeyManager,
    hotkey_id: u32,
    window_hotkey_id: u32,
    record_hotkey_id: u32,
    selection_process: Option<std::process::Child>,
    recording: Option<(std::process::Child, std::path::PathBuf)>,
    /// The in-flight active-window capture, if any. Held only so a second
    /// hotkey press while one is still running does not pile up captures.
    window_capture: Option<std::thread::JoinHandle<()>>,
}

pub(crate) fn register_hotkey_with<F, E>(
    register: F,
    hotkey: HotKey,
    description: &str,
) -> Result<u32, String>
where
    F: FnOnce(HotKey) -> Result<(), E>,
    E: std::fmt::Display,
{
    let hotkey_id = hotkey.id();
    register(hotkey).map_err(|e| format!("Failed to register {description} hotkey: {e}"))?;
    Ok(hotkey_id)
}

/// Capture one active-window image and deliver the same pixels to clipboard and disk.
pub(crate) fn capture_active_window_outputs_with_handlers<Capture, Clipboard, Save>(
    capture: Capture,
    copy_to_clipboard: Clipboard,
    save: Save,
) -> Result<std::path::PathBuf, String>
where
    Capture: FnOnce() -> Result<image::RgbaImage, String>,
    Clipboard: FnOnce(&image::RgbaImage) -> Result<(), String>,
    Save: FnOnce(&image::RgbaImage) -> Result<std::path::PathBuf, String>,
{
    let image = capture().map_err(|error| format!("Failed to capture active window: {error}"))?;
    let clipboard_result = copy_to_clipboard(&image);
    let save_result = save(&image);

    match (clipboard_result, save_result) {
        (Ok(()), Ok(path)) => Ok(path),
        (Err(clipboard_error), Ok(path)) => Err(format!(
            "Window screenshot saved to {}, but clipboard delivery failed: {clipboard_error}",
            path.display()
        )),
        (Ok(()), Err(save_error)) => Err(format!(
            "Window screenshot copied to clipboard, but file save failed: {save_error}"
        )),
        (Err(clipboard_error), Err(save_error)) => Err(format!(
            "Window screenshot clipboard delivery failed: {clipboard_error}; file save also failed: {save_error}"
        )),
    }
}

/// Exercise active-window output delivery without touching the real desktop.
#[doc(hidden)]
pub fn benchmark_capture_active_window_outputs_with_handlers<Capture, Clipboard, Save>(
    capture: Capture,
    copy_to_clipboard: Clipboard,
    save: Save,
) -> Result<std::path::PathBuf, String>
where
    Capture: FnOnce() -> Result<image::RgbaImage, String>,
    Clipboard: FnOnce(&image::RgbaImage) -> Result<(), String>,
    Save: FnOnce(&image::RgbaImage) -> Result<std::path::PathBuf, String>,
{
    capture_active_window_outputs_with_handlers(capture, copy_to_clipboard, save)
}

pub(crate) fn capture_window_target_with<Capture>(
    target: Result<capture::WindowTarget, String>,
    capture_target: Capture,
) -> Result<image::RgbaImage, String>
where
    Capture: FnOnce(capture::WindowTarget) -> Result<image::RgbaImage, String>,
{
    target.and_then(capture_target)
}

impl RustShotApp {
    pub(crate) fn new() -> Result<Self, String> {
        use global_hotkey::hotkey::Modifiers;

        // Register PrintScreen hotkey
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Failed to create hotkey manager: {e}"))?;
        let hotkey = HotKey::new(None, global_hotkey::hotkey::Code::PrintScreen);
        let hotkey_id =
            register_hotkey_with(|hotkey| manager.register(hotkey), hotkey, "PrintScreen")?;

        // Register Alt+PrintScreen for active window capture
        let window_hotkey = HotKey::new(
            Some(Modifiers::ALT),
            global_hotkey::hotkey::Code::PrintScreen,
        );
        let window_hotkey_id = register_hotkey_with(
            |hotkey| manager.register(hotkey),
            window_hotkey,
            "Alt+PrintScreen",
        )?;

        let record_hotkey = HotKey::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            global_hotkey::hotkey::Code::KeyR,
        );
        let record_hotkey_id = register_hotkey_with(
            |hotkey| manager.register(hotkey),
            record_hotkey,
            "Ctrl+Shift+R",
        )?;

        info!("Hotkeys registered");

        Ok(Self {
            _tray: None,
            _hotkey_manager: manager,
            hotkey_id,
            window_hotkey_id,
            record_hotkey_id,
            selection_process: None,
            recording: None,
            window_capture: None,
        })
    }

    /// Capture the active window on a worker thread.
    ///
    /// Deliberately not done inline. `capture_active_window_in_dir` waits on the
    /// Windows Graphics Capture worker and then on a GDI fallback, each with a
    /// multi-second timeout, and then encodes a PNG. Running that inside
    /// `user_event` froze the tray icon, its menu, and hotkey dispatch for as
    /// long as it took — the rest of this type already avoids that, spawning a
    /// child process for selection and a thread to reap the recorder.
    fn handle_window_capture(&mut self, foreground_window: Result<capture::WindowTarget, String>) {
        if !window_capture_slot_is_free(self.window_capture.as_ref().map(|h| h.is_finished())) {
            return;
        }
        self.window_capture = None;

        let settings = settings::load_settings();
        let worker = std::thread::Builder::new()
            .name("rustshot-window-capture".to_string())
            .spawn(move || {
                let format = settings_capture_format(&settings);
                match capture_active_window_outputs_with_handlers(
                    || {
                        capture_window_target_with(
                            foreground_window,
                            capture::capture_window_target,
                        )
                    },
                    copy_image_to_clipboard,
                    |image| {
                        capture::save_active_window_image_in_dir(
                            image,
                            None,
                            format,
                            &settings.screenshots_dir,
                        )
                    },
                ) {
                    Ok(path) => info!(
                        "Window screenshot copied to clipboard and saved to: {}",
                        path.display()
                    ),
                    Err(e) => error!("Window capture error: {}", e),
                }
            });

        match worker {
            Ok(handle) => self.window_capture = Some(handle),
            Err(error) => error!("Failed to start the window capture worker: {error}"),
        }
    }

    fn handle_capture(&mut self) {
        // Check if a selection process is already running
        if let Some(ref mut child) = self.selection_process {
            if child_process_is_running(child.try_wait()) {
                return;
            }
        }

        // Spawn a child process for selection
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to get current exe: {e}");
                return;
            }
        };
        if let Ok(child) = std::process::Command::new(exe).arg("--select").spawn() {
            self.selection_process = Some(child);
        }
    }

    fn open_screenshots_folder(&self) {
        let settings = settings::load_settings();
        open_screenshots_folder_at_with_handlers(
            &settings.screenshots_dir,
            |dir| std::fs::create_dir_all(dir).map_err(|e| e.to_string()),
            open_screenshot_directory,
        );
    }

    fn handle_recording_toggle(&mut self) {
        if let Some((mut child, stop_file)) = self.recording.take() {
            if child_process_is_running(child.try_wait()) {
                // Stop without blocking the winit event loop. If signalling
                // fails, retain the child so the user can retry.
                if let Err(error) =
                    request_recording_stop_with(&stop_file, |path, data| std::fs::write(path, data))
                {
                    error!("Failed to request recording stop: {error}");
                    self.recording = Some((child, stop_file));
                    return;
                }
                std::thread::spawn(move || match child.wait() {
                    Ok(status) if status.success() => {}
                    Ok(status) => error!("Recording process exited with {status}"),
                    Err(error) => error!("Failed to wait for recording process: {error}"),
                });
                return;
            }
        }

        // Start: spawn --select-record child
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to get current exe: {e}");
                return;
            }
        };
        if let Ok(child) = std::process::Command::new(&exe)
            .arg("--select-record")
            .spawn()
        {
            let pid = child.id();
            // Stopping a recording is a filesystem IPC contract: the parent
            // creates this flag file, and the child (`run_select_record`) polls
            // for the same path derived from its OWN pid. Both sides go through
            // the shared `stop_flag_path` helper so the paths cannot drift.
            let stop_file = stop_flag_path(pid);
            self.recording = Some((child, stop_file));
        }
    }
}

pub(crate) fn request_recording_stop_with<WriteStop>(
    stop_file: &Path,
    write_stop: WriteStop,
) -> Result<(), String>
where
    WriteStop: FnOnce(&Path, &[u8]) -> std::io::Result<()>,
{
    write_stop(stop_file, b"stop")
        .map_err(|error| format!("could not write {}: {error}", stop_file.display()))
}

pub(crate) fn child_process_is_running<T, E>(try_wait_result: Result<Option<T>, E>) -> bool {
    cli::child_process_is_running(try_wait_result)
}

/// Whether another active-window capture may start.
///
/// `None` means none has run yet, `Some(true)` that the last one finished. A
/// still-running capture (`Some(false)`) blocks a new one so that holding the
/// hotkey down cannot spawn an unbounded number of capture threads all
/// contending for the same Graphics Capture backend.
pub(crate) fn window_capture_slot_is_free(previous_finished: Option<bool>) -> bool {
    !matches!(previous_finished, Some(false))
}

pub(crate) fn open_screenshots_folder_at_with_handlers<CreateDir, OpenDir>(
    dir: &Path,
    create_dir: CreateDir,
    open_dir: OpenDir,
) where
    CreateDir: Fn(&Path) -> Result<(), String>,
    OpenDir: Fn(&Path),
{
    if let Err(e) = create_dir(dir) {
        error!("Failed to create screenshots directory: {}", e);
        return;
    }
    open_dir(dir);
}

impl ApplicationHandler<RustShotEvent> for RustShotApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Create tray icon when the event loop is ready
        if self._tray.is_none() {
            match tray::create_tray() {
                Ok(tray) => {
                    info!("Tray icon created");
                    self._tray = Some(tray);
                }
                Err(e) => error!("Failed to create tray icon: {}", e),
            }
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RustShotEvent) {
        match event {
            RustShotEvent::HotKey {
                event,
                foreground_window,
            } => {
                if event.id == self.hotkey_id && event.state == HotKeyState::Pressed {
                    self.handle_capture();
                }
                if event.id == self.window_hotkey_id && event.state == HotKeyState::Pressed {
                    let target = foreground_window.unwrap_or_else(|| {
                        Err("No foreground window snapshot accompanied Alt+PrintScreen".to_string())
                    });
                    self.handle_window_capture(target);
                }
                if event.id == self.record_hotkey_id && event.state == HotKeyState::Pressed {
                    self.handle_recording_toggle();
                }
            }
            RustShotEvent::Menu {
                event,
                foreground_window,
            } => {
                // Dispatch on `MenuAction` rather than raw id strings: this is
                // exhaustive, so a new menu entry must be handled here.
                let Some(action) = tray::MenuAction::from_id(event.id.0.as_str()) else {
                    return;
                };
                match action {
                    tray::MenuAction::Capture => self.handle_capture(),
                    tray::MenuAction::CaptureWindow => {
                        self.handle_window_capture(foreground_window)
                    }
                    tray::MenuAction::OpenFolder => self.open_screenshots_folder(),
                    tray::MenuAction::ToggleRecord => self.handle_recording_toggle(),
                    tray::MenuAction::Settings => {
                        #[cfg(windows)]
                        settings_dialog::show_settings_dialog();
                        #[cfg(not(windows))]
                        settings::open_settings_dialog();
                    }
                    tray::MenuAction::Exit => event_loop.exit(),
                }
            }
        }
    }
}

#[cfg(windows)]
pub(crate) fn open_screenshot_directory(path: &Path) {
    let _ = std::process::Command::new("explorer").arg(path).spawn();
}

#[cfg(target_os = "macos")]
pub(crate) fn open_screenshot_directory(path: &Path) {
    let _ = std::process::Command::new("open").arg(path).spawn();
}

#[cfg(target_os = "linux")]
pub(crate) fn open_screenshot_directory(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
