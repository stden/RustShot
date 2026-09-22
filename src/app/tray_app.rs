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
use super::runtime::{
    capture_requested_source, copy_image_to_clipboard, log_capture_outcome, save_capture_file,
    stop_flag_path,
};
use crate::application::capture::{execute_capture_with, CaptureFile, CaptureRequest};
#[cfg(windows)]
use crate::settings_dialog;
use crate::timing::{time_result, ActionTimer};
use crate::{capture, cli, settings, tray};

#[cfg(test)]
#[path = "../../tests/unit/app/hotkeys.rs"]
mod hotkey_tests;

#[cfg(test)]
#[path = "../../tests/unit/app/shutdown.rs"]
mod shutdown_tests;

fn prepare_tray_exit_with<Stop, CloseSelection>(
    recording_stop_file: Option<&Path>,
    stop_recording: Stop,
    close_selection: CloseSelection,
) -> Result<(), String>
where
    Stop: FnOnce(&Path) -> Result<(), String>,
    CloseSelection: FnOnce() -> Result<(), String>,
{
    if let Some(path) = recording_stop_file {
        stop_recording(path)?;
    }
    close_selection()
}

fn spawn_selection_process() -> std::io::Result<std::process::Child> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe).arg("--select").spawn()
}

fn close_selection_child(selection: &mut Option<std::process::Child>) -> Result<(), String> {
    let Some(child) = selection.as_mut() else {
        return Ok(());
    };
    if child
        .try_wait()
        .map_err(|error| format!("Failed to inspect selection process: {error}"))?
        .is_none()
    {
        child
            .kill()
            .map_err(|error| format!("Failed to close selection process: {error}"))?;
    }
    child
        .wait()
        .map_err(|error| format!("Failed to reap selection process: {error}"))?;
    *selection = None;
    Ok(())
}

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
    pub(crate) startup_timer: Option<ActionTimer>,
    pub(crate) exit_timer: Option<ActionTimer>,
    _tray: Option<tray_icon::TrayIcon>,
    _hotkey_manager: GlobalHotKeyManager,
    hotkeys: RegisteredHotkeys,
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

/// Native IDs are assigned at registration; the catalogue remains OS-independent.
struct RegisteredHotkeys {
    actions: Vec<(u32, tray::MenuAction)>,
}

impl RegisteredHotkeys {
    fn register_with<F, E>(mut register: F) -> Result<Self, String>
    where
        F: FnMut(HotKey) -> Result<(), E>,
        E: std::fmt::Display,
    {
        let mut actions = Vec::new();
        for action in tray::MenuAction::ALL {
            let Some(shortcut) = action.hotkey() else {
                continue;
            };
            let id = register_hotkey_with(&mut register, native_hotkey(shortcut), shortcut.label)?;
            actions.push((id, action));
        }
        Ok(Self { actions })
    }

    fn action_for(&self, event: GlobalHotKeyEvent) -> Option<tray::MenuAction> {
        if event.state != HotKeyState::Pressed {
            return None;
        }
        self.actions
            .iter()
            .find_map(|&(id, action)| (event.id == id).then_some(action))
    }
}

fn native_hotkey(shortcut: tray::HotkeySpec) -> HotKey {
    use global_hotkey::hotkey::{Code, Modifiers};
    let key = match shortcut.key {
        tray::HotkeyKey::PrintScreen => Code::PrintScreen,
        tray::HotkeyKey::R => Code::KeyR,
    };
    let modifiers = match shortcut.modifiers {
        tray::HotkeyModifiers::None => None,
        tray::HotkeyModifiers::Alt => Some(Modifiers::ALT),
        tray::HotkeyModifiers::ControlShift => Some(Modifiers::CONTROL | Modifiers::SHIFT),
    };
    HotKey::new(modifiers, key)
}

fn run_window_capture(request: CaptureRequest, timer: ActionTimer) {
    timer.mark("worker_started");
    match execute_capture_with(
        request,
        |source| {
            time_result("window_capture.capture", || {
                capture_requested_source(source)
            })
        },
        |image| {
            time_result("window_capture.clipboard", || {
                copy_image_to_clipboard(image)
            })
        },
        |image, file| time_result("window_capture.save", || save_capture_file(image, file)),
    ) {
        Ok(outcome) => {
            log_capture_outcome(outcome);
            timer.finish("success");
        }
        Err(e) => {
            error!("Window capture error: {}", e);
            timer.finish("error");
        }
    }
}

impl RustShotApp {
    pub(crate) fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Failed to create hotkey manager: {e}"))?;
        let hotkeys = RegisteredHotkeys::register_with(|hotkey| manager.register(hotkey))?;

        info!("Hotkeys registered");

        Ok(Self {
            startup_timer: None,
            exit_timer: None,
            _tray: None,
            _hotkey_manager: manager,
            hotkeys,
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
        let timer = ActionTimer::new("window_capture");
        if !window_capture_slot_is_free(self.window_capture.as_ref().map(|h| h.is_finished())) {
            timer.finish("ignored_busy");
            return;
        }
        self.window_capture = None;

        let settings = settings::load_settings();
        let request = CaptureRequest::window(
            foreground_window,
            CaptureFile {
                format: settings_capture_format(&settings),
                directory: settings.screenshots_dir,
            },
        );
        let worker = time_result("window_capture.worker_launch", || {
            std::thread::Builder::new()
                .name("rustshot-window-capture".to_string())
                .spawn(move || run_window_capture(request, timer))
        });

        match worker {
            Ok(handle) => self.window_capture = Some(handle),
            Err(error) => error!("Failed to start the window capture worker: {error}"),
        }
    }

    fn handle_capture(&mut self) {
        let timer = ActionTimer::new("selection.launch");
        if self.is_selection_busy() {
            timer.finish("ignored_busy");
            return;
        }

        match spawn_selection_process() {
            Ok(child) => {
                info!("Selection child started with pid={}", child.id());
                self.selection_process = Some(child);
                timer.finish("child_started");
            }
            Err(error) => {
                error!("Failed to start the selection process: {error}");
                timer.finish("error");
            }
        }
    }

    fn is_selection_busy(&mut self) -> bool {
        self.selection_process
            .as_mut()
            .is_some_and(|child| child_process_is_running(child.try_wait()))
    }

    fn open_screenshots_folder(&self) {
        let settings = settings::load_settings();
        open_screenshots_folder_at_with_handlers(
            &settings.screenshots_dir,
            |dir| {
                time_result("folder.prepare", || std::fs::create_dir_all(dir))
                    .map_err(|e| e.to_string())
            },
            open_screenshot_directory,
        );
    }

    fn handle_recording_toggle(&mut self) {
        if let Some((mut child, stop_file)) = self.recording.take() {
            if child_process_is_running(child.try_wait()) {
                self.handle_recording_stop(child, stop_file);
                return;
            }
        }

        // Start: spawn --select-record child
        let timer = ActionTimer::new("recording.launch");
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to get current exe: {e}");
                timer.finish("error");
                return;
            }
        };
        match std::process::Command::new(&exe)
            .arg("--select-record")
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                // Stopping a recording is a filesystem IPC contract: the parent
                // creates this flag file, and the child (`run_select_record`) polls
                // for the same path derived from its OWN pid. Both sides go through
                // the shared `stop_flag_path` helper so the paths cannot drift.
                let stop_file = stop_flag_path(pid);
                self.recording = Some((child, stop_file));
                info!("Recording child started with pid={pid}");
                timer.finish("child_started");
            }
            Err(error) => {
                error!("Failed to start the recording process: {error}");
                timer.finish("error");
            }
        }
    }

    fn handle_recording_stop(&mut self, child: std::process::Child, stop_file: std::path::PathBuf) {
        let timer = ActionTimer::new("recording.stop");
        // Stop without blocking the winit event loop. If signalling fails,
        // retain the child so the user can retry.
        if let Err(error) =
            request_recording_stop_with(&stop_file, |path, data| std::fs::write(path, data))
        {
            error!("Failed to request recording stop: {error}");
            self.recording = Some((child, stop_file));
            timer.finish("error");
            return;
        }
        timer.mark("stop_requested");
        std::thread::spawn(move || wait_for_recording_child(child, timer));
    }

    fn handle_hotkey(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: GlobalHotKeyEvent,
        foreground_window: Option<Result<capture::WindowTarget, String>>,
    ) {
        let Some(action) = self.hotkeys.action_for(event) else {
            return;
        };
        let target = foreground_window.unwrap_or_else(|| {
            Err("No foreground window snapshot accompanied Alt+PrintScreen".to_string())
        });
        self.dispatch_action(event_loop, action, target);
    }

    fn prepare_exit(&mut self) -> Result<(), String> {
        let stop_file = match self.recording.as_mut() {
            Some((child, path)) => child
                .try_wait()
                .map_err(|error| format!("Failed to inspect recording process: {error}"))?
                .is_none()
                .then_some(path.as_path()),
            None => None,
        };
        prepare_tray_exit_with(
            stop_file,
            |path| request_recording_stop_with(path, |path, data| std::fs::write(path, data)),
            || close_selection_child(&mut self.selection_process),
        )?;

        if let Some((child, _)) = self.recording.take() {
            // The signalled child owns finalization, so exiting the tray cannot
            // truncate its MP4. Reaping must not hold the event loop open.
            let timer = ActionTimer::new("recording.stop_on_exit");
            if let Err(error) = std::thread::Builder::new()
                .name("rustshot-recording-exit".to_string())
                .spawn(move || wait_for_recording_child(child, timer))
            {
                // The stop marker is already written; the child still saves
                // independently even if a reaper thread cannot be allocated.
                error!("Failed to start recording exit waiter: {error}");
            }
        }
        Ok(())
    }

    /// Both OS event sources use this exhaustive dispatcher, so actions cannot drift.
    fn dispatch_action(
        &mut self,
        event_loop: &ActiveEventLoop,
        action: tray::MenuAction,
        foreground_window: Result<capture::WindowTarget, String>,
    ) {
        match action {
            tray::MenuAction::Capture => self.handle_capture(),
            tray::MenuAction::CaptureWindow => self.handle_window_capture(foreground_window),
            tray::MenuAction::OpenFolder => self.open_screenshots_folder(),
            tray::MenuAction::ToggleRecord => self.handle_recording_toggle(),
            tray::MenuAction::Settings => {
                #[cfg(windows)]
                settings_dialog::show_settings_dialog();
                #[cfg(not(windows))]
                settings::open_settings_dialog();
            }
            tray::MenuAction::Exit => {
                if let Err(error) = self.prepare_exit() {
                    error!("Cannot exit safely: {error}");
                    return;
                }
                self.exit_timer = Some(ActionTimer::new("exit"));
                event_loop.exit();
            }
        }
    }
}

fn wait_for_recording_child(mut child: std::process::Child, timer: ActionTimer) {
    match child.wait() {
        // The child logs recording failures before returning, so a zero exit
        // code is not evidence that an MP4 was saved.
        Ok(status) if status.success() => timer.finish("child_exited"),
        Ok(status) => {
            error!("Recording process exited with {status}");
            timer.finish("child_failed");
        }
        Err(error) => {
            error!("Failed to wait for recording process: {error}");
            timer.finish("error");
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
        if self._tray.is_some() {
            return;
        }
        match tray::create_tray() {
            Ok(tray) => {
                info!("Tray icon created");
                self._tray = Some(tray);
                if let Some(timer) = self.startup_timer.take() {
                    timer.finish("tray_ready");
                }
            }
            Err(e) => {
                error!("Failed to create tray icon: {}", e);
                if let Some(timer) = self.startup_timer.take() {
                    timer.finish("error");
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self._tray.is_some() {
            tray::pump_platform_events();
            // GTK uses a different connection from winit, so its input cannot
            // wake ControlFlow::Wait. Poll without spinning on an idle desktop.
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                std::time::Instant::now() + std::time::Duration::from_millis(16),
            ));
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RustShotEvent) {
        match event {
            RustShotEvent::HotKey {
                event,
                foreground_window,
            } => self.handle_hotkey(event_loop, event, foreground_window),
            RustShotEvent::Menu {
                event,
                foreground_window,
            } => {
                let Some(action) = tray::MenuAction::from_id(event.id.0.as_str()) else {
                    return;
                };
                self.dispatch_action(event_loop, action, foreground_window);
            }
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}
}

#[cfg(windows)]
pub(crate) fn open_screenshot_directory(path: &Path) {
    // Explorer can delegate to an existing process; spawn measures only the
    // shell launch request, not when a folder window becomes visible.
    let _ = time_result("folder.launch_request", || {
        std::process::Command::new("explorer").arg(path).spawn()
    });
}

#[cfg(target_os = "macos")]
pub(crate) fn open_screenshot_directory(path: &Path) {
    let _ = time_result("folder.launch_request", || {
        std::process::Command::new("open").arg(path).spawn()
    });
}

#[cfg(target_os = "linux")]
pub(crate) fn open_screenshot_directory(path: &Path) {
    let _ = time_result("folder.launch_request", || {
        std::process::Command::new("xdg-open").arg(path).spawn()
    });
}
