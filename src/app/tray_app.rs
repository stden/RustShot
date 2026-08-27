//! Tray-mode application: global hotkeys, tray menu dispatch, and the winit
//! event-loop handler.

use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use log::{error, info};
use std::path::Path;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use super::cli_mode::settings_capture_format;
use super::runtime::stop_flag_path;
#[cfg(windows)]
use crate::settings_dialog;
use crate::{capture, cli, settings, tray};

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
    /// `about_to_wait` froze the tray icon, its menu, and hotkey dispatch for as
    /// long as it took — the rest of this type already avoids that, spawning a
    /// child process for selection and a thread to reap the recorder.
    fn handle_window_capture(&mut self) {
        if !window_capture_slot_is_free(self.window_capture.as_ref().map(|h| h.is_finished())) {
            return;
        }
        self.window_capture = None;

        let settings = settings::load_settings();
        let worker = std::thread::Builder::new()
            .name("rustshot-window-capture".to_string())
            .spawn(move || {
                let format = settings_capture_format(&settings);
                match capture::capture_active_window_in_dir(None, format, &settings.screenshots_dir)
                {
                    Ok(path) => info!("Window screenshot saved to: {}", path.display()),
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

impl ApplicationHandler for RustShotApp {
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Handle hotkey events
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.hotkey_id && event.state == HotKeyState::Pressed {
                self.handle_capture();
            }
            if event.id == self.window_hotkey_id && event.state == HotKeyState::Pressed {
                self.handle_window_capture();
            }
            if event.id == self.record_hotkey_id && event.state == HotKeyState::Pressed {
                self.handle_recording_toggle();
            }
        }

        // Handle menu events. Dispatch on the `MenuAction` enum rather than raw
        // id strings: the match is exhaustive, so a new menu entry fails to
        // compile until it is handled here. Unknown ids resolve to `None` and
        // are ignored.
        while let Ok(event) = tray::menu_receiver().try_recv() {
            let Some(action) = tray::MenuAction::from_id(event.id.0.as_str()) else {
                continue;
            };
            match action {
                tray::MenuAction::Capture => self.handle_capture(),
                tray::MenuAction::CaptureWindow => self.handle_window_capture(),
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
