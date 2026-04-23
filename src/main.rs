//! RustShot — screenshot and screen recording utility.
//!
//! # Application modes
//!
//! The binary runs in one of three modes depending on command-line flags:
//!
//! | Mode | Trigger | Description |
//! |------|---------|-------------|
//! | **Tray** | no flags | System tray icon with hotkeys (PrintScreen, Ctrl+Shift+R). |
//! | **CLI** | `--cli`, `--list`, `--output`, `--screen`, or `--format` | Single capture, then exit. |
//! | **Selection** | `--select` / `--select-record` (internal) | Child process spawned by tray mode for the overlay UI. |
//!
//! # Architecture overview
//!
//! ```text
//! main.rs          ──  CLI parsing, app mode routing, tray event loop
//!   ├─ capture.rs  ──  screenshot capture & file saving
//!   ├─ selection.rs ── interactive region selection overlay (winit + softbuffer)
//!   ├─ tray.rs     ──  tray icon & context menu construction
//!   └─ record.rs   ──  screen recording via Windows Media Foundation
//! ```
//!
//! On Windows the binary is built as a GUI subsystem app (no console window).
//! The `--select` child process performs the overlay selection, copies the
//! cropped image to the clipboard, and exits.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod capture;
mod cli;
mod record;
mod selection;
mod settings;
#[cfg(windows)]
mod settings_dialog;
mod tray;

/// Windows registry subkey for auto-start entries.
#[cfg(windows)]
const AUTORUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Registry value name used by RustShot.
#[cfg(windows)]
const AUTORUN_VALUE: &str = "RustShot";

/// Wrap an exe path in quotes for the Windows Run registry value.
#[cfg(windows)]
fn quoted_autorun_command(path: &std::path::Path) -> String {
    format!("\"{}\"", path.to_string_lossy())
}

/// Register RustShot in Windows startup (HKCU\...\Run) using the native Registry API.
/// Uses RegSetKeyValueW which creates the key and sets the value in one call.
#[cfg(windows)]
fn setup_autorun() {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::*;

    let exe_path = match std::env::current_exe() {
        Ok(path) => quoted_autorun_command(&path),
        Err(_) => return,
    };

    let subkey = settings::wide(AUTORUN_SUBKEY);
    let value_name = settings::wide(AUTORUN_VALUE);
    let exe_wide = settings::wide(&exe_path);

    // SAFETY: RegSetKeyValueW is a stable Win32 API. The `subkey`, `value_name`,
    // and `exe_wide` buffers are null-terminated UTF-16 vectors that remain valid
    // for the duration of the call. `byte_len` is computed from the Vec length.
    unsafe {
        let byte_len = (exe_wide.len() * 2) as u32;
        let _ = RegSetKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            REG_SZ.0,
            Some(exe_wide.as_ptr() as *const core::ffi::c_void),
            byte_len,
        );
    }
}

#[cfg(windows)]
fn remove_autorun() {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegDeleteKeyValueW, HKEY_CURRENT_USER};

    let subkey = settings::wide(AUTORUN_SUBKEY);
    let value_name = settings::wide(AUTORUN_VALUE);

    // SAFETY: RegDeleteKeyValueW is a stable Win32 API. The `subkey` and
    // `value_name` buffers are null-terminated UTF-16 vectors that outlive the call.
    unsafe {
        let _ = RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
        );
    }
}

use arboard::Clipboard;
use clap::{Parser, ValueEnum};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use log::{debug, error, info};
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

#[derive(Parser, Debug)]
#[command(name = "rustshot")]
#[command(author = "RustShot Contributors")]
#[command(version = "0.1.0")]
#[command(about = "A screenshot and screen recording utility written in Rust", long_about = None)]
struct Args {
    /// Run in CLI mode (single capture, no tray)
    #[arg(long)]
    cli: bool,

    /// Interactive selection mode (internal use)
    #[arg(long, hide = true)]
    select: bool,

    /// Capture the active window
    #[arg(short, long)]
    window: bool,

    /// Start recording mode (internal use)
    #[arg(long, hide = true)]
    select_record: bool,

    /// Output file path (CLI mode only)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Image format to save
    #[arg(short, long, value_enum, default_value_t = CliFormat::Png)]
    format: CliFormat,

    /// Screen index to capture (0-based, default: primary screen)
    #[arg(short, long)]
    screen: Option<usize>,

    /// List available screens
    #[arg(short, long)]
    list: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)]
enum CliFormat {
    Png,
    Jpg,
    Bmp,
}

impl From<CliFormat> for capture::Format {
    fn from(f: CliFormat) -> Self {
        match f {
            CliFormat::Png => capture::Format::Png,
            CliFormat::Jpg => capture::Format::Jpg,
            CliFormat::Bmp => capture::Format::Bmp,
        }
    }
}

/// Main application state for the tray/event-loop mode.
///
/// Owns the tray icon, global hotkey registrations, and handles to any
/// spawned child processes (selection overlay, recording session).
struct RustShotApp {
    _tray: Option<tray_icon::TrayIcon>,
    _hotkey_manager: GlobalHotKeyManager,
    hotkey_id: u32,
    window_hotkey_id: u32,
    record_hotkey_id: u32,
    #[cfg(target_os = "macos")]
    macos_hotkey_id: u32,
    #[cfg(target_os = "macos")]
    macos_undo_hotkey_id: u32,
    selection_process: Option<std::process::Child>,
    recording: Option<(std::process::Child, std::path::PathBuf)>,
}

fn register_hotkey_with<F, E>(register: F, hotkey: HotKey, description: &str) -> Result<u32, String>
where
    F: FnOnce(HotKey) -> Result<(), E>,
    E: std::fmt::Display,
{
    let hotkey_id = hotkey.id();
    register(hotkey).map_err(|e| format!("Failed to register {description} hotkey: {e}"))?;
    Ok(hotkey_id)
}

impl RustShotApp {
    fn new() -> Result<Self, String> {
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

        // Register Cmd+Shift+P for macOS as requested (Cmd+P is usually for Print)
        #[cfg(target_os = "macos")]
        let macos_hotkey = HotKey::new(
            Some(Modifiers::SUPER | Modifiers::SHIFT),
            global_hotkey::hotkey::Code::KeyP,
        );
        #[cfg(target_os = "macos")]
        let macos_hotkey_id = register_hotkey_with(
            |hotkey| manager.register(hotkey),
            macos_hotkey,
            "Cmd+Shift+P",
        )?;

        // Register Cmd+Shift+Z for macOS as requested (Cmd+Z is usually for Undo)
        #[cfg(target_os = "macos")]
        let macos_undo_hotkey = HotKey::new(
            Some(Modifiers::SUPER | Modifiers::SHIFT),
            global_hotkey::hotkey::Code::KeyZ,
        );
        #[cfg(target_os = "macos")]
        let macos_undo_hotkey_id = register_hotkey_with(
            |hotkey| manager.register(hotkey),
            macos_undo_hotkey,
            "Cmd+Shift+Z",
        )?;
        #[cfg(target_os = "macos")]
        info!(
            "macOS hotkey Cmd+Shift+Z registered with ID: {}",
            macos_undo_hotkey_id
        );

        info!("Hotkeys registered");

        Ok(Self {
            _tray: None,
            _hotkey_manager: manager,
            hotkey_id,
            window_hotkey_id,
            record_hotkey_id,
            #[cfg(target_os = "macos")]
            macos_hotkey_id,
            #[cfg(target_os = "macos")]
            macos_undo_hotkey_id,
            selection_process: None,
            recording: None,
        })
    }

    fn handle_window_capture(&self) {
        let settings = settings::load_settings();
        let format = settings_capture_format(&settings);
        match capture::capture_active_window_in_dir(None, format, &settings.screenshots_dir) {
            Ok(path) => info!("Window screenshot saved to: {}", path.display()),
            Err(e) => error!("Window capture error: {}", e),
        }
    }

    fn handle_capture(&mut self) {
        info!("handle_capture called");
        // Check if a selection process is already running
        if let Some(ref mut child) = self.selection_process {
            match child.try_wait() {
                Ok(None) => {
                    info!("Selection process is already running");
                    return;
                }
                Ok(Some(status)) => {
                    info!("Previous selection process exited with status: {}", status);
                    self.selection_process = None;
                }
                Err(e) => {
                    error!("Error checking selection process status: {}", e);
                    self.selection_process = None;
                }
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
        info!("Spawning selection process: {} --select", exe.display());
        match std::process::Command::new(exe).arg("--select").spawn() {
            Ok(child) => {
                info!("Selection process spawned with PID: {}", child.id());
                self.selection_process = Some(child);
            }
            Err(e) => {
                error!("Failed to spawn selection process: {}", e);
            }
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
                // Stop: create stop-flag file, wait for child
                let _ = std::fs::write(&stop_file, b"stop");
                let _ = child.wait();
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
            let stop_file = std::env::temp_dir().join(format!("rustshot_stop_{}.flag", pid));
            self.recording = Some((child, stop_file));
        }
    }
}

fn child_process_is_running<T, E>(try_wait_result: Result<Option<T>, E>) -> bool {
    cli::child_process_is_running(try_wait_result)
}

fn settings_capture_format(settings: &settings::Settings) -> capture::Format {
    cli::settings_capture_format(&settings.default_format)
}

/// Detect whether the raw CLI arguments contain an explicit format flag.
///
/// Algorithm:
/// 1. Skip `argv[0]`.
/// 2. Track whether the previous token was an option that consumes the next
///    token as its value.
/// 3. Recognize explicit format forms:
///    `--format`, `-f`, `--format=value`, and attached short forms such as
///    `-fpng`.
/// 4. Ignore hyphen-prefixed values that belong to `--output/-o` or
///    `--screen/-s`.
///
/// Possible improvements:
/// - Replace the manual scan with a dedicated clap signal if the library later
///   exposes "was this argument explicitly present?" directly.
/// - Model options as tokens instead of raw strings if more attached-value
///   short flags are introduced.
fn cli_format_flag_present_in<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    cli::cli_format_flag_present_in(args)
}

/// Resolve the capture format for CLI mode using the project's precedence rules.
///
/// Algorithm:
/// 1. If the user passed an explicit format flag, use it.
/// 2. Otherwise, inspect the output filename extension and use it when it maps
///    to a supported format.
/// 3. If neither source is explicit enough, fall back to the saved setting.
///
/// Possible improvements:
/// - Return the winning source alongside the format for better diagnostics and
///   user messaging.
/// - Consolidate CLI and settings precedence into a shared policy type if more
///   output-related options are added.
fn effective_cli_format(
    cli_format: CliFormat,
    format_explicit: bool,
    output: Option<&Path>,
    settings: &settings::Settings,
) -> capture::Format {
    cli::effective_cli_format(
        cli_format.into(),
        format_explicit,
        output,
        settings_capture_format(settings),
    )
}

#[cfg(test)]
fn output_extension(output: Option<&Path>) -> Option<&str> {
    cli::output_extension(output)
}

#[cfg(test)]
fn format_from_extension(extension: &str) -> Option<capture::Format> {
    cli::format_from_extension(extension)
}

#[cfg(test)]
fn format_from_output_extension(output: Option<&Path>) -> Option<capture::Format> {
    cli::format_from_output_extension(output)
}

fn unsupported_output_extension(output: Option<&Path>) -> Option<String> {
    cli::unsupported_output_extension(output)
}

#[cfg(any(test, windows))]
fn sync_autorun_with_handlers<Enable, Disable>(
    enabled: bool,
    enable_autorun: Enable,
    disable_autorun: Disable,
) where
    Enable: FnOnce(),
    Disable: FnOnce(),
{
    if enabled {
        enable_autorun();
    } else {
        disable_autorun();
    }
}

/// Synchronise the Windows auto-start registry entry with the current setting.
///
/// When `enabled` is `true`, registers the current exe path under
/// `HKCU\...\Run`; when `false`, removes it.
#[cfg(windows)]
pub(crate) fn sync_autorun(enabled: bool) {
    sync_autorun_with_handlers(enabled, setup_autorun, remove_autorun);
}

fn should_run_cli_mode_with_format_explicit(args: &Args, format_explicit: bool) -> bool {
    cli::should_run_cli_mode(
        args.cli,
        args.list,
        args.window,
        args.output.is_some(),
        args.screen.is_some(),
        format_explicit,
    )
}

/// Execute one CLI capture flow using persisted settings and injected backends.
///
/// Algorithm:
/// 1. Handle `--list` before any capture-specific validation.
/// 2. If the format was not explicit, reject unsupported output extensions so
///    the filename cannot disagree with the actual encoder.
/// 3. Resolve the effective format from CLI input, output path, and settings.
/// 4. Dispatch to either the active-window backend or the normal screenshot
///    backend.
/// 5. Wrap the returned path in the stable human-readable CLI success message.
///
/// Possible improvements:
/// - Return a structured CLI result type and render the final message in one
///   place to reduce formatting duplication.
/// - Split validation from execution if future commands need dry-run or preview
///   support.
fn run_cli_with_capture_using_settings<F, W>(
    args: Args,
    settings: &settings::Settings,
    format_explicit: bool,
    capture_fn: F,
    window_fn: W,
) -> Result<String, String>
where
    F: Fn(Option<PathBuf>, capture::Format, Option<usize>) -> Result<PathBuf, String>,
    W: Fn(Option<PathBuf>, capture::Format) -> Result<PathBuf, String>,
{
    // Resolve CLI intent in phases:
    // 1. short-circuit the `--list` path
    // 2. reject unsupported output extensions when format was not explicit
    // 3. compute the effective capture format
    // 4. dispatch to the requested capture backend
    if args.list {
        return Ok(format_screen_list_output(capture::list_screens()));
    }

    if !format_explicit {
        if let Some(extension) = unsupported_output_extension(args.output.as_deref()) {
            return Err(format!(
                "Unsupported output extension '.{extension}'. Use png, jpg, or bmp, or pass --format explicitly."
            ));
        }
    }

    let format = effective_cli_format(
        args.format,
        format_explicit,
        args.output.as_deref(),
        settings,
    );

    if args.window {
        return window_fn(args.output, format)
            .map(|path| format!("Screenshot saved to: {}", path.display()));
    }

    capture_fn(args.output, format, args.screen)
        .map(|path| format!("Screenshot saved to: {}", path.display()))
}

fn open_screenshots_folder_at_with_handlers<CreateDir, OpenDir>(
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
            debug!("Hotkey event: ID={}, state={:?}", event.id, event.state);
            if event.id == self.hotkey_id && event.state == HotKeyState::Pressed {
                self.handle_capture();
            }
            if event.id == self.window_hotkey_id && event.state == HotKeyState::Pressed {
                self.handle_window_capture();
            }
            if event.id == self.record_hotkey_id && event.state == HotKeyState::Pressed {
                self.handle_recording_toggle();
            }
            #[cfg(target_os = "macos")]
            if event.id == self.macos_hotkey_id && event.state == HotKeyState::Pressed {
                info!("macOS hotkey Cmd+Shift+P pressed");
                self.handle_capture();
            }
            #[cfg(target_os = "macos")]
            if event.id == self.macos_undo_hotkey_id && event.state == HotKeyState::Pressed {
                info!("macOS undo hotkey Cmd+Shift+Z pressed");
                self.handle_capture();
            }
        }

        // Handle menu events
        while let Ok(event) = tray::menu_receiver().try_recv() {
            match event.id.0.as_str() {
                tray::MENU_CAPTURE => self.handle_capture(),
                tray::MENU_CAPTURE_WINDOW => self.handle_window_capture(),
                tray::MENU_OPEN_FOLDER => self.open_screenshots_folder(),
                tray::MENU_START_RECORD => self.handle_recording_toggle(),
                tray::MENU_STOP_RECORD => self.handle_recording_toggle(),
                tray::MENU_SETTINGS => {
                    #[cfg(windows)]
                    settings_dialog::show_settings_dialog();
                    #[cfg(not(windows))]
                    settings::open_settings_dialog();
                }
                tray::MENU_EXIT => event_loop.exit(),
                _ => {}
            }
        }
    }
}

/// Run the recording sub-process: show selection overlay, then record until stop-flag file appears.
fn run_select_record() {
    use crate::record::RecordingSession;
    let settings = settings::load_settings();
    if let Err(e) = record::validate_recording_start(settings.recording_fps) {
        report_child_process_error(&format!("Failed to start recording: {}", e));
        return;
    }

    use crate::selection::select_region_for_recording;

    let (screen, region) = match select_region_for_recording() {
        Ok(r) => r,
        Err(e) if e == "Selection cancelled" => return,
        Err(e) => {
            report_child_process_error(&format!("Selection error: {}", e));
            return;
        }
    };

    let screen_x = screen.display_info.x;
    let screen_y = screen.display_info.y;

    info!(
        "Starting recording at ({}, {}), region: {:?}",
        screen_x, screen_y, region
    );
    let session = match RecordingSession::start_in_dir(
        screen_x,
        screen_y,
        region,
        settings.recording_fps,
        settings.recordings_dir.clone(),
    ) {
        Ok(s) => s,
        Err(e) => {
            report_child_process_error(&format!("Failed to start recording: {}", e));
            return;
        }
    };

    // Poll for stop-flag file
    let stop_file = std::env::temp_dir().join(format!("rustshot_stop_{}.flag", std::process::id()));

    loop {
        if stop_file.exists() {
            let _ = std::fs::remove_file(&stop_file);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    match session.stop() {
        Ok(path) => info!("Recording saved to: {}", path.display()),
        Err(e) => report_child_process_error(&format!("Recording error: {}", e)),
    }
}

/// Execute a single CLI capture and print the result path.
fn run_cli(args: Args) {
    let settings = settings::load_settings();
    let format_explicit = cli_format_flag_present_in(std::env::args_os());
    match run_cli_with_capture_using_settings(
        args,
        &settings,
        format_explicit,
        |output, format, screen| {
            capture::capture_screenshot_in_dir(output, format, screen, &settings.screenshots_dir)
        },
        |output, format| {
            capture::capture_active_window_in_dir(output, format, &settings.screenshots_dir)
        },
    ) {
        Ok(output) => {
            info!("{}", output);
            println!("{}", output);
        }
        Err(e) => {
            error!("CLI error: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Core CLI logic with injected capture/window functions (for testing without real screen).
#[cfg(test)]
fn run_cli_with_capture<F, W>(args: Args, capture_fn: F, window_fn: W) -> Result<String, String>
where
    F: Fn(Option<PathBuf>, capture::Format, Option<usize>) -> Result<PathBuf, String>,
    W: Fn(Option<PathBuf>, capture::Format) -> Result<PathBuf, String>,
{
    run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        true,
        capture_fn,
        window_fn,
    )
}

/// Run the selection overlay sub-process and copy the cropped image to clipboard.
fn run_selection() {
    match run_selection_with_clipboard(selection::select_region, |cropped| {
        let mut clipboard =
            Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
        let img_data = arboard::ImageData {
            width: cropped.width() as usize,
            height: cropped.height() as usize,
            bytes: std::borrow::Cow::Owned(cropped.into_raw()),
        };

        clipboard
            .set_image(img_data)
            .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;
        Ok(())
    }) {
        Ok(()) => {}
        Err(error) => {
            if let Some(message) = selection_stderr_message(&error) {
                report_child_process_error(&message);
            }
        }
    }
}

/// Selection + clipboard logic with injected functions (for testing).
fn run_selection_with_clipboard<FSelect, FClipboard>(
    select_region_fn: FSelect,
    copy_to_clipboard_fn: FClipboard,
) -> Result<(), String>
where
    FSelect: Fn() -> Result<image::RgbaImage, String>,
    FClipboard: Fn(image::RgbaImage) -> Result<(), String>,
{
    let cropped = match select_region_fn() {
        Ok(cropped) => cropped,
        Err(e) if e == "Selection cancelled" => return Err(e),
        Err(e) => return Err(format!("Selection error: {}", e)),
    };

    copy_to_clipboard_fn(cropped)?;
    info!("Screenshot copied to clipboard!");
    Ok(())
}

/// Format screen list for CLI `--list` output.
///
/// Algorithm:
/// 1. Start with a stable header.
/// 2. Append each screen on its own indented line in input order.
/// 3. Return the assembled string as a single printable block.
///
/// Possible improvements:
/// - Stream directly into a writer if screen lists ever become large enough
///   that repeated string growth matters.
/// - Add machine-readable output modes such as JSON if automation becomes a
///   first-class use case.
fn format_screen_list_output<I>(screens: I) -> String
where
    I: IntoIterator<Item = String>,
{
    cli::format_screen_list_output(screens)
}

/// Convert a selection error into an optional stderr message (suppresses "Selection cancelled").
fn selection_stderr_message(error: &str) -> Option<String> {
    cli::selection_stderr_message(error)
}

fn write_error_line<W: Write>(mut writer: W, message: &str) -> io::Result<()> {
    cli::write_error_line(&mut writer, message)
}

fn report_child_process_error(message: &str) {
    error!("{message}");
    let _ = write_error_line(io::stderr(), message);
}

/// Determine whether CLI mode should be used based on the provided arguments.
#[cfg(test)]
fn should_run_cli_mode(args: &Args) -> bool {
    should_run_cli_mode_with_format_explicit(args, args.format != CliFormat::Png)
}

/// Open the screenshots folder with injected create/open handlers (for testing).
#[cfg(test)]
fn open_screenshots_folder_with_handlers<CreateDir, OpenDir>(
    create_dir: CreateDir,
    open_dir: OpenDir,
) where
    CreateDir: Fn(&Path) -> Result<(), String>,
    OpenDir: Fn(&Path),
{
    let dir = capture::get_screenshots_dir();
    open_screenshots_folder_at_with_handlers(&dir, create_dir, open_dir);
}

#[cfg(windows)]
fn open_screenshot_directory(path: &Path) {
    let _ = std::process::Command::new("explorer").arg(path).spawn();
}

#[cfg(target_os = "macos")]
fn open_screenshot_directory(path: &Path) {
    let _ = std::process::Command::new("open").arg(path).spawn();
}

#[cfg(target_os = "linux")]
fn open_screenshot_directory(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

/// Initialise file-based logging to `%APPDATA%/RustShot/rustshot.log`.
/// Silently falls back to no logging on any failure.
fn init_logging() {
    use simplelog::{Config, LevelFilter, WriteLogger};
    let path = settings::log_file_path();
    if let Ok(file) = open_log_file(&path) {
        let _ = WriteLogger::init(LevelFilter::Debug, Config::default(), file);
    }
}

fn open_log_file(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    OpenOptions::new().create(true).append(true).open(path)
}

fn main() {
    init_logging();
    info!("RustShot v{} starting", env!("CARGO_PKG_VERSION"));

    // Ensure per-monitor DPI awareness on Windows so coordinates and captures match.
    #[cfg(windows)]
    {
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        // SAFETY: SetProcessDpiAwarenessContext is safe to call once before any
        // window creation. The constant is a well-known sentinel, not a pointer.
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    let args = Args::parse();
    let format_explicit = cli_format_flag_present_in(std::env::args_os());
    debug!("CLI args: {:?}", args);

    // Recording selection mode (spawned by tray app)
    if args.select_record {
        run_select_record();
        return;
    }

    // Selection mode (spawned by tray app)
    if args.select {
        run_selection();
        return;
    }

    // CLI mode for backward compatibility
    if should_run_cli_mode_with_format_explicit(&args, format_explicit) {
        run_cli(args);
        return;
    }

    // Tray mode
    #[cfg(windows)]
    sync_autorun(settings::load_settings().auto_start);

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = match RustShotApp::new() {
        Ok(app) => app,
        Err(e) => {
            error!("Tray startup error: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    info!("Entering event loop");
    event_loop.run_app(&mut app).expect("Event loop error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Test helper: wraps run_cli_with_capture with a dummy window capture fn.
    fn run_cli_with_capture_test<F>(args: Args, capture_fn: F) -> Result<String, String>
    where
        F: Fn(Option<PathBuf>, capture::Format, Option<usize>) -> Result<PathBuf, String>,
    {
        run_cli_with_capture(args, capture_fn, |_, _| {
            Err("window capture not used in test".to_string())
        })
    }

    #[derive(Debug)]
    enum AppMode {
        Selection,
        Cli,
        Tray,
    }

    fn decide_app_mode(args: &Args) -> AppMode {
        if args.select || args.select_record {
            AppMode::Selection
        } else if should_run_cli_mode(args) {
            AppMode::Cli
        } else {
            AppMode::Tray
        }
    }

    #[test]
    fn run_cli_with_capture_lists_screens() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };

        let output =
            run_cli_with_capture_test(args, |_, _, _| unreachable!("capture fn should not run"))
                .unwrap();
        assert!(output.starts_with("Available screens:"));
    }

    #[test]
    fn format_screen_list_output_formats_multiple_entries() {
        let output = format_screen_list_output(vec![
            "Screen 0: 1920x1080".to_string(),
            "Screen 1: 2560x1440".to_string(),
        ]);

        assert_eq!(
            output,
            "Available screens:\n  Screen 0: 1920x1080\n  Screen 1: 2560x1440"
        );
    }

    #[test]
    fn format_screen_list_output_handles_empty_list() {
        assert_eq!(
            format_screen_list_output(Vec::<String>::new()),
            "Available screens:"
        );
    }

    #[test]
    fn should_run_cli_mode_is_true_for_cli_flag() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_true_for_list_flag() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_true_for_output_path() {
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("out.png")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_true_for_screen_selection() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: Some(1),
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_true_for_non_default_format() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Bmp,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_false_for_select_only() {
        let args = Args {
            cli: false,
            select: true,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(!should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_true_when_cli_and_select_are_both_set() {
        let args = Args {
            cli: true,
            select: true,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn format_screen_list_output_handles_single_entry() {
        let output = format_screen_list_output(vec!["Screen 0: 800x600".to_string()]);
        assert_eq!(output, "Available screens:\n  Screen 0: 800x600");
    }

    #[test]
    fn run_cli_with_capture_uses_selected_output_and_format() {
        let output_path = PathBuf::from("custom.png");
        let args = Args {
            cli: false,
            select: false,
            output: Some(output_path.clone()),
            format: CliFormat::Jpg,
            screen: Some(1),
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        let path_for_assert = output_path.clone();

        let result =
            run_cli_with_capture_test(args, |actual_output, actual_format, _actual_screen| {
                called.set(true);
                assert_eq!(actual_output, Some(path_for_assert.clone()));
                assert!(matches!(actual_format, capture::Format::Jpg));
                Ok(output_path.clone())
            })
            .unwrap();

        assert_eq!(
            result,
            format!("Screenshot saved to: {}", output_path.display())
        );
        assert!(called.get());
    }

    #[test]
    fn run_cli_from_args_roundtrip_format() {
        assert!(matches!(
            capture::Format::from(CliFormat::Png),
            capture::Format::Png
        ));
        assert!(matches!(
            capture::Format::from(CliFormat::Bmp),
            capture::Format::Bmp
        ));
    }

    #[test]
    fn run_cli_with_capture_reports_capture_error() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };

        let result = run_cli_with_capture_test(args, |_, _, _| Err("capture failed".to_string()));
        assert_eq!(result, Err("capture failed".to_string()));
    }

    #[test]
    fn register_hotkey_with_returns_id_on_success() {
        use global_hotkey::hotkey::Code;

        let hotkey = HotKey::new(None, Code::PrintScreen);
        let expected_id = hotkey.id();
        let result = register_hotkey_with(|_| Ok::<(), io::Error>(()), hotkey, "PrintScreen");
        assert_eq!(result.unwrap(), expected_id);
    }

    #[test]
    fn register_hotkey_with_wraps_registration_error() {
        use global_hotkey::hotkey::Code;

        let hotkey = HotKey::new(None, Code::PrintScreen);
        let err = register_hotkey_with(
            |_| Err(io::Error::new(io::ErrorKind::AddrInUse, "already in use")),
            hotkey,
            "PrintScreen",
        )
        .unwrap_err();

        assert!(err.contains("PrintScreen"));
        assert!(err.contains("already in use"));
    }

    #[test]
    fn register_hotkey_with_wraps_custom_display_error() {
        use global_hotkey::hotkey::Code;

        struct FakeError;

        impl std::fmt::Display for FakeError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "custom registration failure")
            }
        }

        let hotkey = HotKey::new(None, Code::PrintScreen);
        let err = register_hotkey_with(|_| Err(FakeError), hotkey, "PrintScreen").unwrap_err();

        assert!(err.contains("PrintScreen"));
        assert!(err.contains("custom registration failure"));
    }

    #[test]
    fn run_selection_with_clipboard_copies_image() {
        let copied = Cell::new(false);
        let image = image::RgbaImage::new(2, 3);
        run_selection_with_clipboard(
            || Ok(image.clone()),
            |img| {
                copied.set(true);
                assert_eq!(img.width(), 2);
                assert_eq!(img.height(), 3);
                Ok(())
            },
        )
        .unwrap();

        assert!(copied.get());
    }

    #[test]
    fn run_selection_with_clipboard_returns_cancelled_error() {
        let result =
            run_selection_with_clipboard(|| Err("Selection cancelled".to_string()), |_| Ok(()));
        assert_eq!(result, Err("Selection cancelled".to_string()));
    }

    #[test]
    fn run_selection_with_clipboard_wraps_non_cancelled_errors() {
        let result = run_selection_with_clipboard(|| Err("boom".to_string()), |_| Ok(()));
        assert_eq!(result, Err("Selection error: boom".to_string()));
    }

    #[test]
    fn run_selection_with_clipboard_wraps_clipboard_errors() {
        let image = image::RgbaImage::new(2, 3);
        let result = run_selection_with_clipboard(
            || Ok(image.clone()),
            |_| Err("clipboard failed".to_string()),
        );
        assert_eq!(result, Err("clipboard failed".to_string()));
    }

    #[test]
    fn selection_stderr_message_omits_cancelled_errors() {
        assert_eq!(selection_stderr_message("Selection cancelled"), None);
    }

    #[test]
    fn selection_stderr_message_returns_non_cancelled_errors() {
        assert_eq!(
            selection_stderr_message("Selection error: boom"),
            Some("Selection error: boom".to_string())
        );
    }

    #[test]
    fn selection_stderr_message_preserves_plain_error_text() {
        assert_eq!(
            selection_stderr_message("clipboard failed"),
            Some("clipboard failed".to_string())
        );
    }

    #[test]
    fn selection_stderr_message_preserves_empty_error_text() {
        assert_eq!(selection_stderr_message(""), Some(String::new()));
    }

    #[test]
    fn write_error_line_appends_newline() {
        let mut buf = Vec::new();
        write_error_line(&mut buf, "Selection error: boom").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Selection error: boom\n");
    }

    #[test]
    fn write_error_line_empty_message_writes_single_newline() {
        let mut buf = Vec::new();
        write_error_line(&mut buf, "").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "\n");
    }

    #[test]
    fn write_error_line_preserves_whitespace_message() {
        let mut buf = Vec::new();
        write_error_line(&mut buf, "  ").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "  \n");
    }

    #[test]
    fn open_log_file_appends_without_truncating_existing_content() {
        use std::io::Read;

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("rustshot_log_test_{unique}"))
            .join("rustshot.log");

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"old\n").unwrap();

        {
            let mut file = open_log_file(&path).unwrap();
            file.write_all(b"new\n").unwrap();
        }

        let mut content = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "old\nnew\n");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(path.parent().unwrap());
    }

    #[test]
    fn open_log_file_creates_missing_parent_directories() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("rustshot_log_create_dir_{unique}"));
        let path = base.join("nested").join("rustshot.log");

        {
            let mut file = open_log_file(&path).unwrap();
            file.write_all(b"created\n").unwrap();
        }

        assert!(path.exists(), "log file should be created");
        assert!(path.parent().unwrap().is_dir(), "parent dir should exist");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(path.parent().unwrap());
        let _ = std::fs::remove_dir(&base);
    }

    #[test]
    fn open_log_file_creates_new_empty_file() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("rustshot_log_new_file_{unique}"));
        let path = base.join("rustshot.log");

        let file = open_log_file(&path).unwrap();
        drop(file);

        assert!(path.exists(), "new log file should exist");
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            0,
            "newly created log file should be empty"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&base);
    }

    #[test]
    fn open_log_file_returns_error_when_parent_path_is_blocked_by_file() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("rustshot_log_blocked_parent_{unique}"));
        std::fs::write(&base, b"not a directory").unwrap();

        let path = base.join("nested").join("rustshot.log");
        let err = open_log_file(&path).unwrap_err();
        assert!(
            matches!(
                err.kind(),
                io::ErrorKind::AlreadyExists
                    | io::ErrorKind::NotADirectory
                    | io::ErrorKind::PermissionDenied
                    | io::ErrorKind::Other
            ),
            "unexpected error kind: {:?}",
            err.kind()
        );

        let _ = std::fs::remove_file(&base);
    }

    #[test]
    fn open_log_file_returns_error_when_target_path_is_directory() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rustshot_log_dir_target_{unique}"));
        std::fs::create_dir_all(&path).unwrap();

        let err = open_log_file(&path).unwrap_err();
        assert!(
            matches!(
                err.kind(),
                io::ErrorKind::PermissionDenied
                    | io::ErrorKind::AlreadyExists
                    | io::ErrorKind::IsADirectory
                    | io::ErrorKind::Other
            ),
            "unexpected error kind: {:?}",
            err.kind()
        );

        let _ = std::fs::remove_dir(&path);
    }

    #[test]
    fn open_screenshots_folder_with_handlers_calls_open_on_create_success() {
        let created = Cell::new(false);
        let opened = Cell::new(false);

        open_screenshots_folder_with_handlers(
            |dir| {
                created.set(true);
                assert!(dir.ends_with("RustShot"));
                Ok(())
            },
            |_| {
                opened.set(true);
            },
        );

        assert!(created.get());
        assert!(opened.get());
    }

    #[test]
    fn open_screenshots_folder_with_handlers_skips_open_on_create_failure() {
        let created = Cell::new(false);
        let opened = Cell::new(false);

        open_screenshots_folder_with_handlers(
            |_| {
                created.set(true);
                Err("cannot create".to_string())
            },
            |_| {
                opened.set(true);
            },
        );

        assert!(created.get());
        assert!(!opened.get());
    }

    #[test]
    fn cli_parse_defaults_to_png_and_tray_mode() {
        let args = Args::parse_from(["rustshot"]);
        assert!(matches!(decide_app_mode(&args), AppMode::Tray));
        assert!(!should_run_cli_mode(&args));
        assert!(args.output.is_none());
        assert!(!args.list);
        assert!(matches!(args.format, CliFormat::Png));
    }

    #[test]
    fn cli_parse_flags_force_cli_mode() {
        let args = Args::parse_from(["rustshot", "--cli"]);
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
        assert!(should_run_cli_mode(&args));

        let args = Args::parse_from(["rustshot", "--list"]);
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn cli_parse_select_mode() {
        let args = Args::parse_from(["rustshot", "--select"]);
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn cli_parse_output_and_screen_options() {
        let args = Args::parse_from(["rustshot", "--output", "shot.png", "--screen", "2"]);
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
        assert!(should_run_cli_mode(&args));
        assert_eq!(args.output, Some(PathBuf::from("shot.png")));
        assert_eq!(args.screen, Some(2));
    }

    #[test]
    fn cli_parse_screen_only_forces_cli_mode() {
        let args = Args::parse_from(["rustshot", "--screen", "1"]);
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn cli_parse_non_default_format_forces_cli_mode() {
        let args = Args::parse_from(["rustshot", "--format", "bmp"]);
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
        assert!(should_run_cli_mode(&args));
    }

    // --- CliFormat conversion tests ---

    #[test]
    fn cli_format_from_png_gives_capture_png() {
        assert!(matches!(
            capture::Format::from(CliFormat::Png),
            capture::Format::Png
        ));
    }

    #[test]
    fn cli_format_from_jpg_gives_capture_jpg() {
        assert!(matches!(
            capture::Format::from(CliFormat::Jpg),
            capture::Format::Jpg
        ));
    }

    #[test]
    fn cli_format_from_bmp_gives_capture_bmp() {
        assert!(matches!(
            capture::Format::from(CliFormat::Bmp),
            capture::Format::Bmp
        ));
    }

    // --- autorun helper tests ---

    #[cfg(windows)]
    #[test]
    fn quoted_autorun_command_wraps_paths_in_quotes() {
        let command = quoted_autorun_command(Path::new(r"C:\Program Files\RustShot\rustshot.exe"));
        assert_eq!(command, r#""C:\Program Files\RustShot\rustshot.exe""#);
    }

    #[cfg(windows)]
    #[test]
    fn quoted_autorun_command_simple_path() {
        let command = quoted_autorun_command(Path::new(r"C:\rustshot.exe"));
        assert_eq!(command, r#""C:\rustshot.exe""#);
    }

    #[cfg(windows)]
    #[test]
    fn quoted_autorun_command_starts_and_ends_with_quote() {
        let command = quoted_autorun_command(Path::new(r"C:\app\test.exe"));
        assert!(command.starts_with('"'));
        assert!(command.ends_with('"'));
    }

    #[cfg(windows)]
    #[test]
    fn autorun_subkey_points_to_run_key() {
        assert!(AUTORUN_SUBKEY.contains("Run"));
        assert!(AUTORUN_SUBKEY.starts_with("Software\\"));
    }

    #[cfg(windows)]
    #[test]
    fn autorun_value_is_rustshot() {
        assert_eq!(AUTORUN_VALUE, "RustShot");
    }

    #[cfg(windows)]
    #[test]
    fn autorun_constants_produce_valid_wide_strings() {
        let subkey = settings::wide(AUTORUN_SUBKEY);
        let value = settings::wide(AUTORUN_VALUE);
        // Both should be null-terminated
        assert_eq!(*subkey.last().unwrap(), 0);
        assert_eq!(*value.last().unwrap(), 0);
        // Length should be string length + 1 (null)
        assert_eq!(subkey.len(), AUTORUN_SUBKEY.len() + 1);
        assert_eq!(value.len(), AUTORUN_VALUE.len() + 1);
    }

    #[test]
    fn cli_format_flag_present_detects_short_and_long_forms() {
        assert!(cli_format_flag_present_in(["rustshot", "--format", "png"]));
        assert!(cli_format_flag_present_in(["rustshot", "-f", "jpg"]));
        assert!(cli_format_flag_present_in(["rustshot", "--format=bmp"]));
        assert!(!cli_format_flag_present_in(["rustshot", "--cli"]));
    }

    #[test]
    fn cli_parse_attached_short_format_value() {
        let args = Args::try_parse_from(["rustshot", "-fjpg"]).unwrap();
        assert!(matches!(args.format, CliFormat::Jpg));
    }

    #[test]
    fn cli_format_flag_present_detects_attached_short_form() {
        assert!(cli_format_flag_present_in(["rustshot", "-fjpg"]));
        assert!(cli_format_flag_present_in(["rustshot", "-fpng"]));
    }

    #[test]
    fn cli_format_flag_present_ignores_hyphenated_output_values() {
        assert!(!cli_format_flag_present_in([
            "rustshot", "--output", "-foo.png",
        ]));
        assert!(!cli_format_flag_present_in([
            "rustshot",
            "-o",
            "-frames/shot.png",
        ]));
    }

    #[test]
    fn effective_cli_format_keeps_settings_format_for_hyphenated_output_values() {
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };
        let format_explicit = cli_format_flag_present_in(["rustshot", "--output", "-foo.png"]);

        assert!(matches!(
            effective_cli_format(CliFormat::Png, format_explicit, None, &settings),
            capture::Format::Jpg
        ));
    }

    #[test]
    fn should_run_cli_mode_with_attached_short_png_format() {
        let args = Args::try_parse_from(["rustshot", "-fpng"]).unwrap();
        let format_explicit = cli_format_flag_present_in(["rustshot", "-fpng"]);
        assert!(should_run_cli_mode_with_format_explicit(
            &args,
            format_explicit
        ));
    }

    #[test]
    fn settings_capture_format_defaults_to_png_for_unknown_value() {
        let settings = settings::Settings {
            default_format: "gif".to_string(),
            ..settings::Settings::default()
        };

        assert!(matches!(
            settings_capture_format(&settings),
            capture::Format::Png
        ));
    }

    #[test]
    fn settings_capture_format_maps_jpg_value() {
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };

        assert!(matches!(
            settings_capture_format(&settings),
            capture::Format::Jpg
        ));
    }

    #[test]
    fn settings_capture_format_maps_bmp_value() {
        let settings = settings::Settings {
            default_format: "bmp".to_string(),
            ..settings::Settings::default()
        };

        assert!(matches!(
            settings_capture_format(&settings),
            capture::Format::Bmp
        ));
    }

    #[test]
    fn effective_cli_format_uses_settings_when_flag_is_absent() {
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };

        assert!(matches!(
            effective_cli_format(CliFormat::Bmp, false, None, &settings),
            capture::Format::Jpg
        ));
    }

    #[test]
    fn effective_cli_format_prefers_explicit_cli_format_when_flag_is_present() {
        let settings = settings::Settings {
            default_format: "bmp".to_string(),
            ..settings::Settings::default()
        };

        assert!(matches!(
            effective_cli_format(CliFormat::Jpg, true, None, &settings),
            capture::Format::Jpg
        ));
    }

    #[test]
    fn should_run_cli_mode_with_format_explicit_true_for_explicit_png() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode_with_format_explicit(&args, true));
    }

    #[test]
    fn run_cli_with_capture_using_settings_uses_settings_format_when_flag_absent() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let settings = settings::Settings {
            default_format: "bmp".to_string(),
            ..settings::Settings::default()
        };
        let called = Cell::new(false);
        run_cli_with_capture_using_settings(
            args,
            &settings,
            false,
            |_output, actual_format, _screen| {
                called.set(true);
                assert!(matches!(actual_format, capture::Format::Bmp));
                Ok(PathBuf::from("shot.bmp"))
            },
            |_, _| unreachable!("window capture should not run"),
        )
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_using_settings_prefers_explicit_cli_format() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Jpg,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let settings = settings::Settings {
            default_format: "bmp".to_string(),
            ..settings::Settings::default()
        };
        let called = Cell::new(false);
        run_cli_with_capture_using_settings(
            args,
            &settings,
            true,
            |_output, actual_format, _screen| {
                called.set(true);
                assert!(matches!(actual_format, capture::Format::Jpg));
                Ok(PathBuf::from("shot.jpg"))
            },
            |_, _| unreachable!("window capture should not run"),
        )
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_using_settings_prefers_output_extension_when_flag_is_absent() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("shot.bmp")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };
        let called = Cell::new(false);

        run_cli_with_capture_using_settings(
            args,
            &settings,
            false,
            |actual_output, actual_format, _screen| {
                called.set(true);
                assert_eq!(actual_output, Some(PathBuf::from("shot.bmp")));
                assert!(matches!(actual_format, capture::Format::Bmp));
                Ok(PathBuf::from("shot.bmp"))
            },
            |_, _| unreachable!("window capture should not run"),
        )
        .unwrap();

        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_using_settings_output_without_extension_uses_settings_format() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("shot")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };
        let called = Cell::new(false);

        run_cli_with_capture_using_settings(
            args,
            &settings,
            false,
            |actual_output, actual_format, _screen| {
                called.set(true);
                assert_eq!(actual_output, Some(PathBuf::from("shot")));
                assert!(matches!(actual_format, capture::Format::Jpg));
                Ok(PathBuf::from("shot"))
            },
            |_, _| unreachable!("window capture should not run"),
        )
        .unwrap();

        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_using_settings_rejects_unsupported_output_extension_without_flag() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("shot.gif")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };

        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            false,
            |_, _, _| unreachable!("capture handler should not run"),
            |_, _| unreachable!("window capture should not run"),
        );

        assert_eq!(
            result,
            Err("Unsupported output extension '.gif'. Use png, jpg, or bmp, or pass --format explicitly.".to_string())
        );
    }

    #[test]
    fn run_cli_with_capture_using_settings_explicit_format_allows_unsupported_extension() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("shot.gif")),
            format: CliFormat::Bmp,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);

        run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            true,
            |actual_output, actual_format, _screen| {
                called.set(true);
                assert_eq!(actual_output, Some(PathBuf::from("shot.gif")));
                assert!(matches!(actual_format, capture::Format::Bmp));
                Ok(PathBuf::from("shot.gif"))
            },
            |_, _| unreachable!("window capture should not run"),
        )
        .unwrap();

        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_using_settings_propagates_capture_error() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: Some(2),
            list: false,
            select_record: false,
            window: false,
        };

        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            false,
            |_, _, actual_screen| {
                assert_eq!(actual_screen, Some(2));
                Err("capture boom".to_string())
            },
            |_, _| unreachable!("window capture should not run"),
        );

        assert_eq!(result, Err("capture boom".to_string()));
    }

    #[test]
    fn run_cli_with_capture_using_settings_list_skips_capture_and_window_handlers() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };

        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            false,
            |_, _, _| unreachable!("capture handler should not run when listing screens"),
            |_, _| unreachable!("window handler should not run when listing screens"),
        )
        .unwrap();

        assert!(result.starts_with("Available screens:"));
    }

    #[test]
    fn run_cli_with_capture_using_settings_list_skips_unsupported_output_extension_validation() {
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("listing.gif")),
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };

        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            false,
            |_, _, _| unreachable!("capture handler should not run when listing screens"),
            |_, _| unreachable!("window handler should not run when listing screens"),
        )
        .unwrap();

        assert!(result.starts_with("Available screens:"));
    }

    #[test]
    fn run_cli_with_capture_using_settings_list_takes_priority_over_window_mode() {
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("window.gif")),
            format: CliFormat::Bmp,
            screen: Some(3),
            list: true,
            select_record: false,
            window: true,
        };

        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            true,
            |_, _, _| unreachable!("capture handler should not run when listing screens"),
            |_, _| unreachable!("window handler should not run when listing screens"),
        )
        .unwrap();

        assert!(result.starts_with("Available screens:"));
    }

    #[test]
    fn run_cli_with_capture_using_settings_uses_window_handler_for_window_mode() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("window-shot.bmp")),
            format: CliFormat::Png,
            screen: Some(7),
            list: false,
            select_record: false,
            window: true,
        };
        let called = Cell::new(false);

        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings {
                default_format: "bmp".to_string(),
                ..settings::Settings::default()
            },
            false,
            |_, _, _| unreachable!("screen capture handler should not run in window mode"),
            |actual_output, actual_format| {
                called.set(true);
                assert_eq!(actual_output, Some(PathBuf::from("window-shot.bmp")));
                assert!(matches!(actual_format, capture::Format::Bmp));
                Ok(PathBuf::from("window-shot.bmp"))
            },
        )
        .unwrap();

        assert!(called.get());
        assert_eq!(result, "Screenshot saved to: window-shot.bmp");
    }

    #[test]
    fn run_cli_with_capture_using_settings_propagates_window_error() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("window-shot.png")),
            format: CliFormat::Jpg,
            screen: Some(5),
            list: false,
            select_record: false,
            window: true,
        };

        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            true,
            |_, _, _| unreachable!("screen capture handler should not run in window mode"),
            |actual_output, actual_format| {
                assert_eq!(actual_output, Some(PathBuf::from("window-shot.png")));
                assert!(matches!(actual_format, capture::Format::Jpg));
                Err("window boom".to_string())
            },
        );

        assert_eq!(result, Err("window boom".to_string()));
    }

    #[test]
    fn open_screenshots_folder_at_with_handlers_uses_provided_dir() {
        let dir = PathBuf::from("/tmp/custom-shots");
        let create_called = Cell::new(false);
        let open_called = Cell::new(false);
        open_screenshots_folder_at_with_handlers(
            &dir,
            |actual_dir| {
                create_called.set(true);
                assert_eq!(actual_dir, dir.as_path());
                Ok(())
            },
            |actual_dir| {
                open_called.set(true);
                assert_eq!(actual_dir, dir.as_path());
            },
        );
        assert!(create_called.get());
        assert!(open_called.get());
    }

    #[test]
    fn open_screenshots_folder_at_with_handlers_skips_open_when_create_dir_fails() {
        let dir = PathBuf::from("/tmp/custom-shots");
        let open_called = Cell::new(false);

        open_screenshots_folder_at_with_handlers(
            &dir,
            |_| Err("mkdir failed".to_string()),
            |_| open_called.set(true),
        );

        assert!(!open_called.get());
    }

    #[test]
    fn cli_format_flag_present_ignores_similar_prefixes() {
        assert!(!cli_format_flag_present_in([
            "rustshot",
            "--formatting=png",
            "--formatter",
            "jpg"
        ]));
    }

    #[test]
    fn sync_autorun_with_handlers_disables_when_auto_start_is_false() {
        let set_called = Cell::new(false);
        let remove_called = Cell::new(false);
        sync_autorun_with_handlers(false, || set_called.set(true), || remove_called.set(true));
        assert!(!set_called.get());
        assert!(remove_called.get());
    }

    #[test]
    fn sync_autorun_with_handlers_enables_when_auto_start_is_true() {
        let set_called = Cell::new(false);
        let remove_called = Cell::new(false);
        sync_autorun_with_handlers(true, || set_called.set(true), || remove_called.set(true));
        assert!(set_called.get());
        assert!(!remove_called.get());
    }

    // --- decide_app_mode additional tests ---

    #[test]
    fn decide_app_mode_output_some_gives_cli() {
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("x.png")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    }

    #[test]
    fn decide_app_mode_select_true_gives_selection() {
        let args = Args {
            cli: false,
            select: true,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn decide_app_mode_cli_true_gives_cli() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    }

    #[test]
    fn decide_app_mode_list_true_gives_cli() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    }

    #[test]
    fn decide_app_mode_all_false_gives_tray() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Tray));
    }

    // --- run_cli_with_capture additional tests ---

    #[test]
    fn run_cli_with_capture_passes_screen_index_correctly() {
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("custom.png")),
            format: CliFormat::Jpg,
            screen: Some(1),
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |_output, _format, actual_screen| {
            called.set(true);
            assert_eq!(actual_screen, Some(1));
            Ok(PathBuf::from("custom.png"))
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_screen_none_passes_none() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |_output, _format, actual_screen| {
            called.set(true);
            assert_eq!(actual_screen, None);
            Ok(PathBuf::from("shot.png"))
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_bmp_format_is_forwarded() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Bmp,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |_output, actual_format, _screen| {
            called.set(true);
            assert!(matches!(actual_format, capture::Format::Bmp));
            Ok(PathBuf::from("shot.bmp"))
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_png_format_is_forwarded() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |_output, actual_format, _screen| {
            called.set(true);
            assert!(matches!(actual_format, capture::Format::Png));
            Ok(PathBuf::from("shot.png"))
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_output_path_is_forwarded() {
        let output_path = PathBuf::from("my_screenshot.jpg");
        let args = Args {
            cli: false,
            select: false,
            output: Some(output_path.clone()),
            format: CliFormat::Jpg,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        let expected = output_path.clone();
        run_cli_with_capture_test(args, |actual_output, _format, _screen| {
            called.set(true);
            assert_eq!(actual_output, Some(expected.clone()));
            Ok(output_path.clone())
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_result_message_contains_path() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let result =
            run_cli_with_capture_test(args, |_, _, _| Ok(PathBuf::from("my_output.png"))).unwrap();
        assert!(result.contains("my_output.png"));
        assert!(result.starts_with("Screenshot saved to:"));
    }

    #[test]
    fn run_cli_with_capture_list_does_not_call_capture_fn() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };
        // The capture fn should never be called when list=true
        let result = run_cli_with_capture_test(args, |_, _, _| {
            panic!("capture fn must not be called when listing screens")
        });
        assert!(result.is_ok());
    }

    // --- run_selection_with_clipboard additional tests ---

    #[test]
    fn run_selection_with_clipboard_propagates_ok_from_clipboard() {
        let image = image::RgbaImage::new(10, 10);
        let result = run_selection_with_clipboard(|| Ok(image.clone()), |_img| Ok(()));
        assert!(result.is_ok());
    }

    #[test]
    fn run_selection_with_clipboard_forwards_image_dimensions() {
        let image = image::RgbaImage::new(640, 480);
        run_selection_with_clipboard(
            || Ok(image.clone()),
            |img| {
                assert_eq!(img.width(), 640);
                assert_eq!(img.height(), 480);
                Ok(())
            },
        )
        .unwrap();
    }

    #[test]
    fn run_selection_with_clipboard_selection_error_is_wrapped() {
        let result = run_selection_with_clipboard(|| Err("network error".to_string()), |_| Ok(()));
        assert_eq!(result, Err("Selection error: network error".to_string()));
    }

    // --- open_screenshots_folder_with_handlers additional tests ---

    #[test]
    fn open_screenshots_folder_with_handlers_passes_correct_dir() {
        let opened = Cell::new(false);
        open_screenshots_folder_with_handlers(
            |_| Ok(()),
            |dir| {
                opened.set(true);
                assert_eq!(
                    dir.file_name().and_then(|n| n.to_str()),
                    Some("RustShot"),
                    "should open the RustShot folder"
                );
            },
        );
        assert!(opened.get());
    }

    #[test]
    fn open_screenshots_folder_with_handlers_create_dir_receives_rustshot_path() {
        let called = Cell::new(false);
        open_screenshots_folder_with_handlers(
            |dir| {
                called.set(true);
                assert!(dir.ends_with("RustShot"));
                Ok(())
            },
            |_| {},
        );
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_jpg_format_is_forwarded() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Jpg,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |_output, actual_format, _screen| {
            called.set(true);
            assert!(matches!(actual_format, capture::Format::Jpg));
            Ok(PathBuf::from("shot.jpg"))
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_result_starts_with_screenshot_saved() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let result =
            run_cli_with_capture_test(args, |_, _, _| Ok(PathBuf::from("out.png"))).unwrap();
        assert!(result.starts_with("Screenshot saved to:"));
    }

    #[test]
    fn run_selection_with_clipboard_single_pixel_image() {
        let image = image::RgbaImage::new(1, 1);
        let result = run_selection_with_clipboard(|| Ok(image.clone()), |_| Ok(()));
        assert!(result.is_ok());
    }

    #[test]
    fn cli_parse_format_jpg_is_parsed() {
        let args = Args::parse_from(["rustshot", "--cli", "--format", "jpg"]);
        assert!(matches!(args.format, CliFormat::Jpg));
    }

    #[test]
    fn cli_parse_format_bmp_is_parsed() {
        let args = Args::parse_from(["rustshot", "--cli", "--format", "bmp"]);
        assert!(matches!(args.format, CliFormat::Bmp));
    }

    #[test]
    fn cli_parse_screen_zero_is_parsed() {
        let args = Args::parse_from(["rustshot", "--screen", "0"]);
        assert_eq!(args.screen, Some(0));
    }

    #[test]
    fn run_cli_with_capture_error_passthrough_preserves_message() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let result = run_cli_with_capture_test(args, |_, _, _| Err("disk full".to_string()));
        assert_eq!(result.unwrap_err(), "disk full");
    }

    #[test]
    fn decide_app_mode_select_wins_over_cli() {
        // select=true takes priority even when cli=true
        let args = Args {
            cli: true,
            select: true,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn cli_short_flag_output() {
        let args = Args::parse_from(["rustshot", "-o", "snap.png"]);
        assert_eq!(args.output, Some(PathBuf::from("snap.png")));
    }

    #[test]
    fn cli_short_flag_format() {
        let args = Args::parse_from(["rustshot", "-f", "bmp"]);
        assert!(matches!(args.format, CliFormat::Bmp));
    }

    #[test]
    fn cli_short_flag_screen() {
        let args = Args::parse_from(["rustshot", "-s", "3"]);
        assert_eq!(args.screen, Some(3));
    }

    #[test]
    fn cli_short_flag_list() {
        let args = Args::parse_from(["rustshot", "-l"]);
        assert!(args.list);
    }

    #[test]
    fn run_cli_with_capture_result_message_display_format() {
        // Result should be "Screenshot saved to: <path>" with a space after the colon
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let result =
            run_cli_with_capture_test(args, |_, _, _| Ok(PathBuf::from("test.png"))).unwrap();
        assert_eq!(result, "Screenshot saved to: test.png");
    }

    #[test]
    fn run_selection_with_clipboard_error_contains_selection_error_prefix() {
        let result = run_selection_with_clipboard(|| Err("something bad".to_string()), |_| Ok(()));
        let err = result.unwrap_err();
        assert!(err.starts_with("Selection error:"));
        assert!(err.contains("something bad"));
    }

    #[test]
    fn run_selection_with_clipboard_clipboard_error_message_is_exact() {
        let image = image::RgbaImage::new(5, 5);
        let result =
            run_selection_with_clipboard(|| Ok(image.clone()), |_| Err("no clipboard".to_string()));
        assert_eq!(result.unwrap_err(), "no clipboard");
    }

    #[test]
    fn cli_format_clone_works() {
        let f = CliFormat::Jpg;
        let g = f.clone();
        assert!(matches!(g, CliFormat::Jpg));
    }

    #[test]
    fn cli_format_debug_repr_is_non_empty() {
        assert!(!format!("{:?}", CliFormat::Png).is_empty());
        assert!(!format!("{:?}", CliFormat::Jpg).is_empty());
        assert!(!format!("{:?}", CliFormat::Bmp).is_empty());
    }

    #[test]
    fn cli_parse_short_output_with_format() {
        let args = Args::parse_from(["rustshot", "-o", "out.jpg", "-f", "jpg"]);
        assert_eq!(args.output, Some(PathBuf::from("out.jpg")));
        assert!(matches!(args.format, CliFormat::Jpg));
    }

    #[test]
    fn run_cli_with_capture_capture_fn_called_exactly_once() {
        let call_count = Cell::new(0u32);
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let _ = run_cli_with_capture_test(args, |_, _, _| {
            call_count.set(call_count.get() + 1);
            Ok(PathBuf::from("shot.png"))
        });
        assert_eq!(
            call_count.get(),
            1,
            "capture fn should be called exactly once"
        );
    }

    #[test]
    fn run_selection_with_clipboard_large_image() {
        let image = image::RgbaImage::new(1920, 1080);
        let result = run_selection_with_clipboard(
            || Ok(image.clone()),
            |img| {
                assert_eq!(img.width(), 1920);
                assert_eq!(img.height(), 1080);
                Ok(())
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn cli_parse_screen_defaults_to_none() {
        let args = Args::parse_from(["rustshot"]);
        assert_eq!(args.screen, None);
    }

    #[test]
    fn decide_app_mode_select_overrides_output() {
        // select=true takes priority even when output is also set
        let args = Args {
            cli: false,
            select: true,
            output: Some(PathBuf::from("x.png")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn cli_parse_all_long_flags_combined() {
        let args = Args::parse_from([
            "rustshot", "--output", "out.bmp", "--format", "bmp", "--screen", "2", "--cli",
        ]);
        assert!(args.cli);
        assert_eq!(args.output, Some(PathBuf::from("out.bmp")));
        assert!(matches!(args.format, CliFormat::Bmp));
        assert_eq!(args.screen, Some(2));
    }

    #[test]
    fn run_cli_with_capture_list_fn_call_count_is_zero() {
        let count = Cell::new(0u32);
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };
        let _ = run_cli_with_capture_test(args, |_, _, _| {
            count.set(count.get() + 1);
            Ok(PathBuf::from("x.png"))
        });
        assert_eq!(
            count.get(),
            0,
            "capture fn must not be called when list=true"
        );
    }

    #[test]
    fn run_selection_with_clipboard_image_pixels_pass_through() {
        // Pixels in the image should arrive unchanged at the clipboard fn
        let mut img = image::RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));
        let result = run_selection_with_clipboard(
            {
                let img = img.clone();
                move || Ok(img.clone())
            },
            |received| {
                assert_eq!(received.get_pixel(0, 0), &image::Rgba([1, 2, 3, 255]));
                Ok(())
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn cli_parse_output_nested_path() {
        let args = Args::parse_from(["rustshot", "--output", "a/b/c.png"]);
        assert_eq!(args.output, Some(PathBuf::from("a/b/c.png")));
    }

    #[test]
    fn open_screenshots_folder_with_handlers_can_be_called_twice() {
        let open_count = Cell::new(0u32);
        open_screenshots_folder_with_handlers(
            |_| Ok(()),
            |_| {
                open_count.set(open_count.get() + 1);
            },
        );
        open_screenshots_folder_with_handlers(
            |_| Ok(()),
            |_| {
                open_count.set(open_count.get() + 1);
            },
        );
        assert_eq!(
            open_count.get(),
            2,
            "open_dir should be called once per invocation"
        );
    }

    // --- select_record flag tests ---

    #[test]
    fn decide_app_mode_select_record_true_gives_selection() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn decide_app_mode_select_record_wins_over_cli() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn decide_app_mode_select_record_wins_over_list() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: true,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn decide_app_mode_select_record_and_select_both_true_gives_selection() {
        let args = Args {
            cli: false,
            select: true,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn should_run_cli_mode_is_false_for_select_record_only() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(!should_run_cli_mode(&args));
    }

    #[test]
    fn cli_parse_select_record_flag_is_parsed() {
        let args = Args::parse_from(["rustshot", "--select-record"]);
        assert!(args.select_record);
        assert!(!args.select);
        assert!(!args.cli);
    }

    #[test]
    fn cli_parse_select_record_and_format_combined() {
        let args = Args::parse_from(["rustshot", "--select-record", "--format", "jpg"]);
        assert!(args.select_record);
        assert!(matches!(args.format, CliFormat::Jpg));
    }

    #[test]
    fn decide_app_mode_select_record_with_output_gives_selection() {
        // select_record takes priority even when output path is set
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("out.png")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn args_default_select_record_is_false() {
        let args = Args::parse_from(["rustshot"]);
        assert!(!args.select_record);
    }

    #[test]
    fn format_screen_list_output_three_entries() {
        let output = format_screen_list_output(vec![
            "Screen 0: 1920x1080".to_string(),
            "Screen 1: 2560x1440".to_string(),
            "Screen 2: 3840x2160".to_string(),
        ]);
        assert_eq!(output.lines().count(), 4); // header + 3 screens
        assert!(output.contains("Screen 2: 3840x2160"));
    }

    #[test]
    fn selection_stderr_message_whitespace_only_returns_some() {
        assert_eq!(selection_stderr_message("  "), Some("  ".to_string()));
    }

    #[test]
    fn should_run_cli_mode_false_when_no_flags() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(!should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_true_when_all_cli_flags_set() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("x.png")),
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn run_cli_with_capture_output_none_passes_none() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |output, _, _| {
            called.set(true);
            assert!(output.is_none());
            Ok(PathBuf::from("shot.png"))
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_selection_with_clipboard_cancelled_does_not_call_clipboard() {
        let clipboard_called = Cell::new(false);
        let result = run_selection_with_clipboard(
            || Err("Selection cancelled".to_string()),
            |_| {
                clipboard_called.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!clipboard_called.get());
    }

    #[test]
    fn run_selection_with_clipboard_select_error_does_not_call_clipboard() {
        let clipboard_called = Cell::new(false);
        let result = run_selection_with_clipboard(
            || Err("some error".to_string()),
            |_| {
                clipboard_called.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!clipboard_called.get());
    }

    #[test]
    fn cli_parse_version_flag_exists() {
        // --version should cause clap to exit with version info (we test it doesn't panic)
        let result = Args::try_parse_from(["rustshot", "--version"]);
        // clap returns an error (with kind DisplayVersion) for --version
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_help_flag_exists() {
        let result = Args::try_parse_from(["rustshot", "--help"]);
        assert!(result.is_err()); // clap exits with help
    }

    // --- window capture CLI tests ---

    #[test]
    fn cli_parse_window_flag_is_parsed() {
        let args = Args::parse_from(["rustshot", "--window"]);
        assert!(args.window);
    }

    #[test]
    fn cli_parse_window_short_flag() {
        let args = Args::parse_from(["rustshot", "-w"]);
        assert!(args.window);
    }

    #[test]
    fn args_default_window_is_false() {
        let args = Args::parse_from(["rustshot"]);
        assert!(!args.window);
    }

    #[test]
    fn should_run_cli_mode_is_true_for_window_flag() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn decide_app_mode_window_gives_cli() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    }

    #[test]
    fn run_cli_with_capture_window_flag_calls_window_fn() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let capture_called = Cell::new(false);
        let window_called = Cell::new(false);
        let result = run_cli_with_capture(
            args,
            |_, _, _| {
                capture_called.set(true);
                Ok(PathBuf::from("screen.png"))
            },
            |_, _| {
                window_called.set(true);
                Ok(PathBuf::from("window.png"))
            },
        )
        .unwrap();
        assert!(!capture_called.get(), "capture fn should not be called");
        assert!(window_called.get(), "window fn should be called");
        assert!(result.contains("window.png"));
    }

    #[test]
    fn run_cli_with_capture_no_window_flag_calls_capture_fn() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let capture_called = Cell::new(false);
        let window_called = Cell::new(false);
        let result = run_cli_with_capture(
            args,
            |_, _, _| {
                capture_called.set(true);
                Ok(PathBuf::from("screen.png"))
            },
            |_, _| {
                window_called.set(true);
                Ok(PathBuf::from("window.png"))
            },
        )
        .unwrap();
        assert!(capture_called.get(), "capture fn should be called");
        assert!(!window_called.get(), "window fn should not be called");
        assert!(result.contains("screen.png"));
    }

    #[test]
    fn run_cli_with_capture_window_error_propagates() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| Ok(PathBuf::from("x.png")),
            |_, _| Err("window not found".to_string()),
        );
        assert_eq!(result.unwrap_err(), "window not found");
    }

    #[test]
    fn run_cli_with_capture_window_output_path_forwarded() {
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("custom_window.png")),
            format: CliFormat::Jpg,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| unreachable!("should not call capture"),
            |output, format| {
                assert_eq!(output, Some(PathBuf::from("custom_window.png")));
                assert!(matches!(format, capture::Format::Jpg));
                Ok(PathBuf::from("custom_window.png"))
            },
        )
        .unwrap();
        assert!(result.contains("custom_window.png"));
    }

    // --- should_run_cli_mode: window flag ---

    #[test]
    fn should_run_cli_mode_window_only_is_true() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_false_for_all_defaults() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(!should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_is_false_for_select_record() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(!should_run_cli_mode(&args));
    }

    // --- decide_app_mode: select_record ---

    #[test]
    fn decide_app_mode_select_record_gives_selection() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn decide_app_mode_window_flag_gives_cli() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    }

    // --- CliFormat trait tests ---

    #[test]
    fn cli_format_debug_repr_non_empty() {
        assert!(!format!("{:?}", CliFormat::Png).is_empty());
        assert!(!format!("{:?}", CliFormat::Jpg).is_empty());
        assert!(!format!("{:?}", CliFormat::Bmp).is_empty());
    }

    #[test]
    fn cli_format_clone_produces_equal_value() {
        let f = CliFormat::Jpg;
        assert_eq!(f.clone(), CliFormat::Jpg);
    }

    #[test]
    fn cli_format_partial_eq_works() {
        assert_eq!(CliFormat::Png, CliFormat::Png);
        assert_ne!(CliFormat::Png, CliFormat::Jpg);
        assert_ne!(CliFormat::Jpg, CliFormat::Bmp);
    }

    // --- run_cli_with_capture: window with different formats ---

    #[test]
    fn run_cli_with_capture_window_bmp_format() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Bmp,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| unreachable!("should not call capture"),
            |_output, format| {
                assert!(matches!(format, capture::Format::Bmp));
                Ok(PathBuf::from("window.bmp"))
            },
        )
        .unwrap();
        assert!(result.contains("window.bmp"));
    }

    #[test]
    fn run_cli_with_capture_window_none_output() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| unreachable!("should not call capture"),
            |output, _format| {
                assert!(output.is_none());
                Ok(PathBuf::from("auto_window.png"))
            },
        )
        .unwrap();
        assert!(result.contains("auto_window.png"));
    }

    // --- run_cli_with_capture: list takes priority over window ---

    #[test]
    fn run_cli_with_capture_list_takes_priority_over_window() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: true,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| unreachable!("should not call capture"),
            |_, _| unreachable!("should not call window capture"),
        )
        .unwrap();
        assert!(result.starts_with("Available screens:"));
    }

    // --- format_screen_list_output edge cases ---

    #[test]
    fn format_screen_list_output_three_short_entries() {
        let output =
            format_screen_list_output(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        assert_eq!(output, "Available screens:\n  A\n  B\n  C");
    }

    #[test]
    fn format_screen_list_output_entries_are_indented() {
        let output = format_screen_list_output(vec!["Screen 0: 800x600".to_string()]);
        assert!(output.contains("  Screen 0"));
    }

    // --- selection_stderr_message edge cases ---

    #[test]
    fn selection_stderr_message_exact_cancelled_string() {
        // Only exact match "Selection cancelled" should return None
        assert_eq!(selection_stderr_message("Selection cancelled"), None);
        assert!(selection_stderr_message("selection cancelled").is_some()); // case sensitive
        assert!(selection_stderr_message("Selection cancelled!").is_some()); // extra char
    }

    // --- open_screenshots_folder_with_handlers path ---

    #[test]
    fn open_screenshots_folder_with_handlers_passes_rustshot_dir() {
        let received_path = std::cell::RefCell::new(None::<PathBuf>);
        open_screenshots_folder_with_handlers(
            |_| Ok(()),
            |dir| {
                received_path.replace(Some(dir.to_path_buf()));
            },
        );
        let path = received_path.borrow();
        let path = path.as_ref().expect("open_dir should be called");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("RustShot"));
    }

    // --- CLI parse additional tests ---

    #[test]
    fn cli_parse_window_flag() {
        let args = Args::parse_from(["rustshot", "--window"]);
        assert!(args.window);
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn cli_parse_short_window_flag() {
        let args = Args::parse_from(["rustshot", "-w"]);
        assert!(args.window);
    }

    #[test]
    fn cli_parse_format_jpg() {
        let args = Args::parse_from(["rustshot", "--format", "jpg"]);
        assert_eq!(args.format, CliFormat::Jpg);
    }

    #[test]
    fn cli_parse_short_output() {
        let args = Args::parse_from(["rustshot", "-o", "my_shot.png"]);
        assert_eq!(args.output, Some(PathBuf::from("my_shot.png")));
    }

    #[test]
    fn cli_parse_short_screen() {
        let args = Args::parse_from(["rustshot", "-s", "0"]);
        assert_eq!(args.screen, Some(0));
    }

    #[test]
    fn cli_parse_combined_flags() {
        let args = Args::parse_from([
            "rustshot", "--cli", "--format", "bmp", "--output", "out.bmp", "--screen", "1",
        ]);
        assert!(args.cli);
        assert_eq!(args.format, CliFormat::Bmp);
        assert_eq!(args.output, Some(PathBuf::from("out.bmp")));
        assert_eq!(args.screen, Some(1));
    }

    // --- run_selection_with_clipboard: clipboard error preserves message ---

    #[test]
    fn run_selection_with_clipboard_clipboard_error_message_preserved() {
        let img = image::RgbaImage::new(1, 1);
        let result =
            run_selection_with_clipboard(|| Ok(img.clone()), |_| Err("access denied".to_string()));
        assert_eq!(result.unwrap_err(), "access denied");
    }

    #[test]
    fn run_selection_with_clipboard_selection_error_wraps_message() {
        let result =
            run_selection_with_clipboard(|| Err("X11 not available".to_string()), |_| Ok(()));
        assert_eq!(result.unwrap_err(), "Selection error: X11 not available");
    }

    // -------------------------------------------------------------------------
    // open_screenshot_directory (Linux) — does not panic
    // -------------------------------------------------------------------------

    #[cfg(target_os = "linux")]
    #[test]
    fn open_screenshot_directory_does_not_panic() {
        // xdg-open may not be available in CI but should not panic
        let path = std::path::Path::new("/tmp");
        open_screenshot_directory(path);
    }

    // -------------------------------------------------------------------------
    // list_screens — smoke test
    // -------------------------------------------------------------------------

    #[test]
    fn list_screens_returns_vec() {
        let screens = capture::list_screens();
        // On CI (no display) this may be empty, but should not panic
        assert!(screens.len() <= 100, "unreasonable number of screens");
    }

    #[test]
    fn list_screens_entries_contain_screen_prefix() {
        let screens = capture::list_screens();
        for s in &screens {
            assert!(
                s.starts_with("Screen "),
                "each entry should start with 'Screen '"
            );
        }
    }

    // -------------------------------------------------------------------------
    // selection_stderr_message mutation killers
    // -------------------------------------------------------------------------

    #[test]
    fn selection_stderr_message_cancelled_is_exactly_none() {
        assert!(selection_stderr_message("Selection cancelled").is_none());
    }

    #[test]
    fn selection_stderr_message_similar_but_not_cancelled() {
        // "Selection cancelled" with extra space — should NOT be None
        assert!(selection_stderr_message("Selection cancelled ").is_some());
        assert!(selection_stderr_message("selection cancelled").is_some());
        assert!(selection_stderr_message("Cancelled").is_some());
    }

    // --- run_cli_with_capture: window with jpg and png formats ---

    #[test]
    fn run_cli_with_capture_window_jpg_format() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Jpg,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| unreachable!("should not call capture"),
            |_output, format| {
                assert!(matches!(format, capture::Format::Jpg));
                Ok(PathBuf::from("window.jpg"))
            },
        )
        .unwrap();
        assert!(result.contains("window.jpg"));
    }

    #[test]
    fn run_cli_with_capture_window_png_format() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| unreachable!("should not call capture"),
            |_output, format| {
                assert!(matches!(format, capture::Format::Png));
                Ok(PathBuf::from("window.png"))
            },
        )
        .unwrap();
        assert!(result.contains("window.png"));
    }

    // --- decide_app_mode: window vs select_record precedence ---

    #[test]
    fn decide_app_mode_select_record_wins_over_window() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: true,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    #[test]
    fn decide_app_mode_select_wins_over_window() {
        let args = Args {
            cli: false,
            select: true,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    // --- run_cli_with_capture: screen index forwarding ---

    #[test]
    fn run_cli_with_capture_screen_index_0_forwarded() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: Some(0),
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |_, _, screen| {
            called.set(true);
            assert_eq!(screen, Some(0));
            Ok(PathBuf::from("shot.png"))
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn run_cli_with_capture_screen_index_5_forwarded() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: Some(5),
            list: false,
            select_record: false,
            window: false,
        };
        let called = Cell::new(false);
        run_cli_with_capture_test(args, |_, _, screen| {
            called.set(true);
            assert_eq!(screen, Some(5));
            Ok(PathBuf::from("shot.png"))
        })
        .unwrap();
        assert!(called.get());
    }

    // --- format_screen_list_output edge cases ---

    #[test]
    fn format_screen_list_output_special_characters() {
        let output = format_screen_list_output(vec!["Screen 0: 1920×1080 (Primary)".to_string()]);
        assert!(output.contains("1920×1080"));
        assert!(output.contains("(Primary)"));
    }

    #[test]
    fn format_screen_list_output_many_entries() {
        let entries: Vec<String> = (0..10)
            .map(|i| format!("Screen {}: 1920x1080", i))
            .collect();
        let output = format_screen_list_output(entries);
        assert_eq!(output.lines().count(), 11); // header + 10 screens
    }

    #[test]
    fn format_screen_list_output_preserves_entry_order() {
        let output = format_screen_list_output(vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ]);
        let first_pos = output.find("first").unwrap();
        let second_pos = output.find("second").unwrap();
        let third_pos = output.find("third").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < third_pos);
    }

    // --- selection_stderr_message with long strings ---

    #[test]
    fn selection_stderr_message_long_error_returns_some() {
        let long_msg = "A".repeat(10000);
        let result = selection_stderr_message(&long_msg);
        assert_eq!(result, Some(long_msg));
    }

    #[test]
    fn selection_stderr_message_empty_string_returns_some() {
        assert_eq!(selection_stderr_message(""), Some("".to_string()));
    }

    // --- should_run_cli_mode: each flag in isolation ---

    #[test]
    fn should_run_cli_mode_only_screen_flag() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: Some(0),
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_only_output_flag() {
        let args = Args {
            cli: false,
            select: false,
            output: Some(PathBuf::from("x.png")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_only_jpg_format() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Jpg,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    #[test]
    fn should_run_cli_mode_only_bmp_format() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Bmp,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode(&args));
    }

    // --- CliFormat value_enum parsing ---

    #[test]
    fn cli_parse_invalid_format_is_error() {
        let result = Args::try_parse_from(["rustshot", "--format", "gif"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_format_is_case_sensitive() {
        // clap value_enum is case-sensitive: "PNG" is invalid, only "png" works
        let result = Args::try_parse_from(["rustshot", "--format", "PNG"]);
        assert!(result.is_err());
    }

    // --- CliFormat conversion exhaustiveness ---

    #[test]
    fn cli_format_to_capture_format_all_variants() {
        let conversions: Vec<(CliFormat, &str)> = vec![
            (CliFormat::Png, "png"),
            (CliFormat::Jpg, "jpg"),
            (CliFormat::Bmp, "bmp"),
        ];
        for (cli_fmt, expected_ext) in conversions {
            let capture_fmt: capture::Format = cli_fmt.into();
            assert_eq!(capture_fmt.extension(), expected_ext);
        }
    }

    // --- run_selection_with_clipboard: image dimensions ---

    #[test]
    fn run_selection_with_clipboard_4k_image() {
        let image = image::RgbaImage::new(3840, 2160);
        let result = run_selection_with_clipboard(
            || Ok(image.clone()),
            |img| {
                assert_eq!(img.width(), 3840);
                assert_eq!(img.height(), 2160);
                Ok(())
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn run_selection_with_clipboard_asymmetric_image() {
        let image = image::RgbaImage::new(100, 1);
        let result = run_selection_with_clipboard(
            || Ok(image.clone()),
            |img| {
                assert_eq!(img.width(), 100);
                assert_eq!(img.height(), 1);
                Ok(())
            },
        );
        assert!(result.is_ok());
    }

    // --- open_screenshots_folder_with_handlers: error message ---

    #[test]
    fn open_screenshots_folder_with_handlers_create_error_skips_open() {
        let open_called = Cell::new(false);
        open_screenshots_folder_with_handlers(
            |_| Err("permission denied".to_string()),
            |_| {
                open_called.set(true);
            },
        );
        assert!(
            !open_called.get(),
            "open_dir must not be called when create_dir fails"
        );
    }

    // --- run_cli_with_capture: error from window capture vs screen capture ---

    #[test]
    fn run_cli_with_capture_capture_error_preserves_exact_message() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        let result = run_cli_with_capture_test(args, |_, _, _| {
            Err("Screen index 99 is out of range (0-1)".to_string())
        });
        assert_eq!(result.unwrap_err(), "Screen index 99 is out of range (0-1)");
    }

    #[test]
    fn run_cli_with_capture_window_error_preserves_exact_message() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture(
            args,
            |_, _, _| unreachable!(),
            |_, _| Err("Active window capture is only supported on Windows".to_string()),
        );
        assert_eq!(
            result.unwrap_err(),
            "Active window capture is only supported on Windows"
        );
    }

    // --- CLI arg defaults ---

    #[test]
    fn args_defaults_all_false_and_none() {
        let args = Args::parse_from(["rustshot"]);
        assert!(!args.cli);
        assert!(!args.select);
        assert!(!args.window);
        assert!(!args.list);
        assert!(!args.select_record);
        assert_eq!(args.output, None);
        assert_eq!(args.screen, None);
        assert_eq!(args.format, CliFormat::Png);
    }

    #[test]
    fn child_process_is_running_true_for_ok_none() {
        let running: Result<Option<()>, ()> = Ok(None);
        assert!(child_process_is_running(running));
    }

    #[test]
    fn child_process_is_running_false_for_finished_child() {
        let finished: Result<Option<()>, ()> = Ok(Some(()));
        assert!(!child_process_is_running(finished));
    }

    #[test]
    fn child_process_is_running_false_for_try_wait_error() {
        let errored: Result<Option<()>, ()> = Err(());
        assert!(!child_process_is_running(errored));
    }

    #[test]
    fn cli_parse_screen_large_value() {
        let args = Args::parse_from(["rustshot", "--screen", "999"]);
        assert_eq!(args.screen, Some(999));
    }

    // --- format_screen_list_output edge cases ---

    #[test]
    fn format_screen_list_output_single_entry() {
        let output = format_screen_list_output(vec!["Screen 0: 1920x1080".to_string()]);
        assert_eq!(output, "Available screens:\n  Screen 0: 1920x1080");
    }

    #[test]
    fn format_screen_list_output_three_entries_indented() {
        let output =
            format_screen_list_output(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        assert_eq!(output, "Available screens:\n  A\n  B\n  C");
    }

    // --- CliFormat conversion ---

    #[test]
    fn cli_format_into_capture_format_png() {
        let f: capture::Format = CliFormat::Png.into();
        assert!(matches!(f, capture::Format::Png));
    }

    #[test]
    fn cli_format_into_capture_format_jpg() {
        let f: capture::Format = CliFormat::Jpg.into();
        assert!(matches!(f, capture::Format::Jpg));
    }

    #[test]
    fn cli_format_into_capture_format_bmp() {
        let f: capture::Format = CliFormat::Bmp.into();
        assert!(matches!(f, capture::Format::Bmp));
    }

    // --- CliFormat Debug ---

    #[test]
    fn cli_format_debug_is_non_empty() {
        assert!(!format!("{:?}", CliFormat::Png).is_empty());
        assert!(!format!("{:?}", CliFormat::Jpg).is_empty());
        assert!(!format!("{:?}", CliFormat::Bmp).is_empty());
    }

    // --- CliFormat PartialEq ---

    #[test]
    fn cli_format_equality() {
        assert_eq!(CliFormat::Png, CliFormat::Png);
        assert_eq!(CliFormat::Jpg, CliFormat::Jpg);
        assert_eq!(CliFormat::Bmp, CliFormat::Bmp);
        assert_ne!(CliFormat::Png, CliFormat::Jpg);
        assert_ne!(CliFormat::Png, CliFormat::Bmp);
        assert_ne!(CliFormat::Jpg, CliFormat::Bmp);
    }

    // --- CliFormat Clone ---

    #[test]
    fn cli_format_clone() {
        let f = CliFormat::Jpg;
        let g = f.clone();
        assert_eq!(f, g);
    }

    // --- effective_cli_format edge cases ---

    #[test]
    fn effective_cli_format_explicit_png_overrides_jpg_settings() {
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };
        let result = effective_cli_format(CliFormat::Png, true, None, &settings);
        assert!(matches!(result, capture::Format::Png));
    }

    #[test]
    fn effective_cli_format_settings_bmp_used_when_not_explicit() {
        let settings = settings::Settings {
            default_format: "bmp".to_string(),
            ..settings::Settings::default()
        };
        let result = effective_cli_format(CliFormat::Png, false, None, &settings);
        assert!(matches!(result, capture::Format::Bmp));
    }

    // --- should_run_cli_mode_with_format_explicit ---

    #[test]
    fn should_run_cli_mode_with_format_explicit_false_and_no_flags() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(!should_run_cli_mode_with_format_explicit(&args, false));
    }

    #[test]
    fn should_run_cli_mode_with_format_explicit_true_triggers_cli() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: false,
        };
        assert!(should_run_cli_mode_with_format_explicit(&args, true));
    }

    #[test]
    fn should_run_cli_mode_with_format_explicit_window_flag_triggers_cli() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        assert!(should_run_cli_mode_with_format_explicit(&args, false));
    }

    // --- cli_format_flag_present_in edge cases ---

    #[test]
    fn cli_format_flag_present_in_skips_first_arg() {
        // First argument is the program name, should be skipped
        assert!(!cli_format_flag_present_in(["--format"]));
    }

    #[test]
    fn cli_format_flag_present_in_format_equals_value() {
        assert!(cli_format_flag_present_in(["rustshot", "--format=png"]));
    }

    #[test]
    fn cli_format_flag_present_in_empty_args() {
        assert!(!cli_format_flag_present_in(Vec::<String>::new()));
    }

    // --- write_error_line edge cases ---

    #[test]
    fn write_error_line_with_special_chars() {
        let mut buf = Vec::new();
        write_error_line(&mut buf, "error: \"bad\" path\\n").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "error: \"bad\" path\\n\n");
    }

    // --- sync_autorun_with_handlers ---

    #[test]
    fn sync_autorun_with_handlers_calls_enable_when_true() {
        let called = Cell::new(false);
        sync_autorun_with_handlers(true, || called.set(true), || {});
        assert!(called.get());
    }

    #[test]
    fn sync_autorun_with_handlers_calls_disable_when_false() {
        let called = Cell::new(false);
        sync_autorun_with_handlers(false, || {}, || called.set(true));
        assert!(called.get());
    }

    #[test]
    fn sync_autorun_with_handlers_does_not_call_disable_when_true() {
        let called = Cell::new(false);
        sync_autorun_with_handlers(true, || {}, || called.set(true));
        assert!(!called.get());
    }

    #[test]
    fn sync_autorun_with_handlers_does_not_call_enable_when_false() {
        let called = Cell::new(false);
        sync_autorun_with_handlers(false, || called.set(true), || {});
        assert!(!called.get());
    }

    // -------------------------------------------------------------------------
    // format_from_extension tests
    // -------------------------------------------------------------------------

    #[test]
    fn format_from_extension_png_lowercase() {
        assert!(matches!(
            format_from_extension("png"),
            Some(capture::Format::Png)
        ));
    }

    #[test]
    fn format_from_extension_png_uppercase() {
        assert!(matches!(
            format_from_extension("PNG"),
            Some(capture::Format::Png)
        ));
    }

    #[test]
    fn format_from_extension_png_mixed_case() {
        assert!(matches!(
            format_from_extension("Png"),
            Some(capture::Format::Png)
        ));
    }

    #[test]
    fn format_from_extension_jpg_lowercase() {
        assert!(matches!(
            format_from_extension("jpg"),
            Some(capture::Format::Jpg)
        ));
    }

    #[test]
    fn format_from_extension_jpeg_lowercase() {
        assert!(matches!(
            format_from_extension("jpeg"),
            Some(capture::Format::Jpg)
        ));
    }

    #[test]
    fn format_from_extension_jpeg_uppercase() {
        assert!(matches!(
            format_from_extension("JPEG"),
            Some(capture::Format::Jpg)
        ));
    }

    #[test]
    fn format_from_extension_bmp_lowercase() {
        assert!(matches!(
            format_from_extension("bmp"),
            Some(capture::Format::Bmp)
        ));
    }

    #[test]
    fn format_from_extension_bmp_uppercase() {
        assert!(matches!(
            format_from_extension("BMP"),
            Some(capture::Format::Bmp)
        ));
    }

    #[test]
    fn format_from_extension_unknown_returns_none() {
        assert!(format_from_extension("gif").is_none());
        assert!(format_from_extension("webp").is_none());
        assert!(format_from_extension("tiff").is_none());
        assert!(format_from_extension("").is_none());
    }

    // -------------------------------------------------------------------------
    // output_extension tests
    // -------------------------------------------------------------------------

    #[test]
    fn output_extension_none_path() {
        assert!(output_extension(None).is_none());
    }

    #[test]
    fn output_extension_no_extension() {
        assert!(output_extension(Some(Path::new("/tmp/screenshot"))).is_none());
    }

    #[test]
    fn output_extension_png() {
        assert_eq!(
            output_extension(Some(Path::new("/tmp/shot.png"))),
            Some("png")
        );
    }

    #[test]
    fn output_extension_jpg() {
        assert_eq!(output_extension(Some(Path::new("file.jpg"))), Some("jpg"));
    }

    #[test]
    fn output_extension_dotfile_no_extension() {
        assert!(output_extension(Some(Path::new(".hidden"))).is_none());
    }

    // -------------------------------------------------------------------------
    // format_from_output_extension tests
    // -------------------------------------------------------------------------

    #[test]
    fn format_from_output_extension_none() {
        assert!(format_from_output_extension(None).is_none());
    }

    #[test]
    fn format_from_output_extension_png_path() {
        assert!(matches!(
            format_from_output_extension(Some(Path::new("shot.png"))),
            Some(capture::Format::Png)
        ));
    }

    #[test]
    fn format_from_output_extension_jpg_path() {
        assert!(matches!(
            format_from_output_extension(Some(Path::new("shot.JPG"))),
            Some(capture::Format::Jpg)
        ));
    }

    #[test]
    fn format_from_output_extension_jpeg_path() {
        assert!(matches!(
            format_from_output_extension(Some(Path::new("shot.JPEG"))),
            Some(capture::Format::Jpg)
        ));
    }

    #[test]
    fn format_from_output_extension_no_extension() {
        assert!(format_from_output_extension(Some(Path::new("/tmp/file"))).is_none());
    }

    #[test]
    fn format_from_output_extension_unknown_ext() {
        assert!(format_from_output_extension(Some(Path::new("file.gif"))).is_none());
    }

    // -------------------------------------------------------------------------
    // unsupported_output_extension tests
    // -------------------------------------------------------------------------

    #[test]
    fn unsupported_output_extension_none_returns_none() {
        assert!(unsupported_output_extension(None).is_none());
    }

    #[test]
    fn unsupported_output_extension_png_returns_none() {
        assert!(unsupported_output_extension(Some(Path::new("shot.png"))).is_none());
    }

    #[test]
    fn unsupported_output_extension_jpg_returns_none() {
        assert!(unsupported_output_extension(Some(Path::new("shot.jpg"))).is_none());
    }

    #[test]
    fn unsupported_output_extension_bmp_returns_none() {
        assert!(unsupported_output_extension(Some(Path::new("shot.bmp"))).is_none());
    }

    #[test]
    fn unsupported_output_extension_gif_returns_extension() {
        assert_eq!(
            unsupported_output_extension(Some(Path::new("shot.gif"))),
            Some("gif".to_string())
        );
    }

    #[test]
    fn unsupported_output_extension_webp_returns_extension() {
        assert_eq!(
            unsupported_output_extension(Some(Path::new("shot.webp"))),
            Some("webp".to_string())
        );
    }

    #[test]
    fn unsupported_output_extension_no_ext_returns_none() {
        assert!(unsupported_output_extension(Some(Path::new("/tmp/file"))).is_none());
    }

    // -------------------------------------------------------------------------
    // CliFormat derive trait tests (non-duplicate)
    // -------------------------------------------------------------------------

    #[test]
    fn cli_format_partial_eq_all_pairs_differ() {
        assert_ne!(CliFormat::Png, CliFormat::Jpg);
        assert_ne!(CliFormat::Jpg, CliFormat::Bmp);
        assert_ne!(CliFormat::Png, CliFormat::Bmp);
    }

    #[test]
    fn cli_format_eq_same_variant() {
        assert_eq!(CliFormat::Png, CliFormat::Png);
        assert_eq!(CliFormat::Jpg, CliFormat::Jpg);
        assert_eq!(CliFormat::Bmp, CliFormat::Bmp);
    }

    // -------------------------------------------------------------------------
    // settings_capture_format edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn settings_capture_format_png_fallback_for_empty_string() {
        let settings = settings::Settings {
            default_format: String::new(),
            ..settings::Settings::default()
        };
        assert!(matches!(
            settings_capture_format(&settings),
            capture::Format::Png
        ));
    }

    #[test]
    fn settings_capture_format_png_fallback_for_uppercase() {
        let settings = settings::Settings {
            default_format: "JPG".to_string(),
            ..settings::Settings::default()
        };
        assert!(matches!(
            settings_capture_format(&settings),
            capture::Format::Png
        ));
    }

    // -------------------------------------------------------------------------
    // effective_cli_format edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn effective_cli_format_explicit_bmp_overrides_settings_jpg() {
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };
        let result = effective_cli_format(CliFormat::Bmp, true, None, &settings);
        assert!(matches!(result, capture::Format::Bmp));
    }

    #[test]
    fn effective_cli_format_not_explicit_uses_settings_bmp() {
        let settings = settings::Settings {
            default_format: "bmp".to_string(),
            ..settings::Settings::default()
        };
        let result = effective_cli_format(CliFormat::Jpg, false, None, &settings);
        assert!(matches!(result, capture::Format::Bmp));
    }

    // -------------------------------------------------------------------------
    // selection_stderr_message edge case
    // -------------------------------------------------------------------------

    #[test]
    fn selection_stderr_message_non_cancelled_returns_exact_error() {
        let msg = selection_stderr_message("Something went wrong").unwrap();
        assert_eq!(msg, "Something went wrong");
    }

    // -------------------------------------------------------------------------
    // open_screenshots_folder_at_with_handlers edge case
    // -------------------------------------------------------------------------

    #[test]
    fn open_screenshots_folder_at_with_handlers_calls_both_on_success() {
        let created = Cell::new(false);
        let opened = Cell::new(false);
        let dir = std::env::temp_dir();
        open_screenshots_folder_at_with_handlers(
            &dir,
            |_| {
                created.set(true);
                Ok(())
            },
            |_| opened.set(true),
        );
        assert!(created.get());
        assert!(opened.get());
    }

    // -------------------------------------------------------------------------
    // cli_format_flag_present_in edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn cli_format_flag_present_in_only_program_name_returns_false() {
        assert!(!cli_format_flag_present_in(["rustshot"]));
    }

    #[test]
    fn cli_format_flag_present_in_f_shorthand_with_value() {
        assert!(cli_format_flag_present_in(["rustshot", "-fbmp"]));
    }

    #[test]
    fn cli_format_flag_present_in_skips_value_after_output_flag() {
        // "--output png" should NOT treat "png" as a format flag
        assert!(!cli_format_flag_present_in(
            ["rustshot", "--output", "png",]
        ));
    }

    #[test]
    fn cli_format_flag_present_in_skips_value_after_o_flag() {
        assert!(!cli_format_flag_present_in(["rustshot", "-o", "--format",]));
    }

    #[test]
    fn cli_format_flag_present_in_skips_value_after_screen_flag() {
        assert!(!cli_format_flag_present_in(["rustshot", "--screen", "-f",]));
    }

    #[test]
    fn cli_format_flag_present_in_skips_value_after_s_flag() {
        assert!(!cli_format_flag_present_in(["rustshot", "-s", "--format",]));
    }

    #[test]
    fn cli_format_flag_present_in_format_after_skipped_value() {
        // The value after --output is skipped, then --format is detected
        assert!(cli_format_flag_present_in([
            "rustshot", "--output", "file.png", "--format", "bmp",
        ]));
    }

    #[test]
    fn cli_format_flag_present_in_format_equals_bmp() {
        assert!(cli_format_flag_present_in(["rustshot", "--format=bmp"]));
    }

    // -------------------------------------------------------------------------
    // effective_cli_format — output extension path
    // -------------------------------------------------------------------------

    #[test]
    fn effective_cli_format_output_jpg_overrides_settings_png() {
        let settings = settings::Settings::default(); // default_format = "png"
        let path = Path::new("screenshot.jpg");
        let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
        assert!(matches!(result, capture::Format::Jpg));
    }

    #[test]
    fn effective_cli_format_output_bmp_overrides_settings_png() {
        let settings = settings::Settings::default();
        let path = Path::new("output.bmp");
        let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
        assert!(matches!(result, capture::Format::Bmp));
    }

    #[test]
    fn effective_cli_format_explicit_flag_beats_output_extension() {
        let settings = settings::Settings::default();
        let path = Path::new("screenshot.jpg");
        // Explicit --format bmp should win over output extension .jpg
        let result = effective_cli_format(CliFormat::Bmp, true, Some(path), &settings);
        assert!(matches!(result, capture::Format::Bmp));
    }

    #[test]
    fn effective_cli_format_unknown_output_ext_falls_back_to_settings() {
        let settings = settings::Settings {
            default_format: "jpg".to_string(),
            ..settings::Settings::default()
        };
        let path = Path::new("file.webp"); // unsupported extension
        let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
        // webp is not recognized, so falls back to settings (jpg)
        assert!(matches!(result, capture::Format::Jpg));
    }

    #[test]
    fn effective_cli_format_output_no_ext_falls_back_to_settings() {
        let settings = settings::Settings {
            default_format: "bmp".to_string(),
            ..settings::Settings::default()
        };
        let path = Path::new("screenshot");
        let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
        assert!(matches!(result, capture::Format::Bmp));
    }

    // -------------------------------------------------------------------------
    // run_cli_with_capture_using_settings — unsupported ext with explicit format
    // -------------------------------------------------------------------------

    #[test]
    fn run_cli_with_capture_using_settings_allows_unsupported_ext_when_format_explicit() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("shot.gif")),
            format: CliFormat::Bmp,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let called = Cell::new(false);
        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            true, // format_explicit = true, so unsupported ext check is bypassed
            |_, _, _| unreachable!("screen capture should not run in window mode"),
            |_, actual_format| {
                called.set(true);
                assert!(matches!(actual_format, capture::Format::Bmp));
                Ok(PathBuf::from("shot.gif"))
            },
        );
        assert!(called.get());
        assert!(result.is_ok());
    }

    #[test]
    fn run_cli_with_capture_using_settings_window_error_message_contains_path() {
        let args = Args {
            cli: true,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            false,
            |_, _, _| unreachable!(),
            |_, _| Err("no foreground window".to_string()),
        );
        assert_eq!(result, Err("no foreground window".to_string()));
    }

    // -------------------------------------------------------------------------
    // child_process_is_running — exhaustive pattern coverage
    // -------------------------------------------------------------------------

    #[test]
    fn child_process_is_running_ok_some_means_exited() {
        assert!(!child_process_is_running(Ok::<Option<i32>, String>(Some(
            42
        ))));
    }

    // -------------------------------------------------------------------------
    // write_error_line — unicode content
    // -------------------------------------------------------------------------

    #[test]
    fn write_error_line_handles_unicode() {
        let mut buf = Vec::new();
        write_error_line(&mut buf, "ошибка 日本語").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("ошибка"));
        assert!(output.contains("日本語"));
        assert!(output.ends_with('\n'));
    }

    // -------------------------------------------------------------------------
    // format_screen_list_output — large count
    // -------------------------------------------------------------------------

    #[test]
    fn format_screen_list_output_ten_entries_all_indented() {
        let entries: Vec<String> = (0..10).map(|i| format!("Screen {i}: 1920x1080")).collect();
        let output = format_screen_list_output(entries);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 11); // header + 10 entries
        for line in &lines[1..] {
            assert!(line.starts_with("  "));
        }
    }

    // -------------------------------------------------------------------------
    // should_run_cli_mode_with_format_explicit — window flag
    // -------------------------------------------------------------------------

    #[test]
    fn should_run_cli_mode_with_format_explicit_false_and_window_triggers_cli() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        assert!(should_run_cli_mode_with_format_explicit(&args, false));
    }

    // -------------------------------------------------------------------------
    // decide_app_mode — select_record
    // -------------------------------------------------------------------------

    #[test]
    fn decide_app_mode_select_record_is_selection() {
        let args = Args {
            cli: false,
            select: false,
            output: None,
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: true,
            window: false,
        };
        assert!(matches!(decide_app_mode(&args), AppMode::Selection));
    }

    // -------------------------------------------------------------------------
    // run_cli_with_capture_using_settings — window mode uses output extension
    // -------------------------------------------------------------------------

    #[test]
    fn run_cli_with_capture_using_settings_window_mode_uses_output_extension() {
        let args = Args {
            cli: true,
            select: false,
            output: Some(PathBuf::from("capture.jpg")),
            format: CliFormat::Png,
            screen: None,
            list: false,
            select_record: false,
            window: true,
        };
        let called = Cell::new(false);
        let result = run_cli_with_capture_using_settings(
            args,
            &settings::Settings::default(),
            false, // not explicit — will use output extension
            |_, _, _| unreachable!("screen capture should not run in window mode"),
            |_, actual_format| {
                called.set(true);
                // output is "capture.jpg" → format_from_output_extension → Jpg
                assert!(matches!(actual_format, capture::Format::Jpg));
                Ok(PathBuf::from("capture.jpg"))
            },
        );
        assert!(called.get());
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // cli_format_flag_present_in — double flags
    // -------------------------------------------------------------------------

    #[test]
    fn cli_format_flag_present_in_format_after_output_with_value() {
        // --output takes a value, then --format appears
        assert!(cli_format_flag_present_in([
            "rustshot", "--output", "file.png", "--format", "bmp"
        ]));
    }

    #[test]
    fn cli_format_flag_present_in_no_format_with_multiple_flags() {
        assert!(!cli_format_flag_present_in([
            "rustshot", "--cli", "--list", "--window",
        ]));
    }

    // -------------------------------------------------------------------------
    // Args::parse_from — combined flags
    // -------------------------------------------------------------------------

    #[test]
    fn cli_parse_all_flags_at_once() {
        let args = Args::parse_from([
            "rustshot", "--cli", "--output", "out.bmp", "--format", "bmp", "--screen", "0",
        ]);
        assert!(args.cli);
        assert_eq!(args.output, Some(PathBuf::from("out.bmp")));
        assert!(matches!(args.format, CliFormat::Bmp));
        assert_eq!(args.screen, Some(0));
    }

    // -------------------------------------------------------------------------
    // selection_stderr_message — whitespace-only error
    // -------------------------------------------------------------------------

    #[test]
    fn selection_stderr_message_whitespace_returns_some() {
        assert_eq!(selection_stderr_message("   "), Some("   ".to_string()));
    }

    // -------------------------------------------------------------------------
    // sync_autorun_with_handlers — verify mutual exclusivity
    // -------------------------------------------------------------------------

    #[test]
    fn sync_autorun_with_handlers_enable_and_disable_are_mutually_exclusive() {
        let enable_count = Cell::new(0u32);
        let disable_count = Cell::new(0u32);

        sync_autorun_with_handlers(
            true,
            || enable_count.set(enable_count.get() + 1),
            || disable_count.set(disable_count.get() + 1),
        );
        assert_eq!(enable_count.get(), 1);
        assert_eq!(disable_count.get(), 0);

        sync_autorun_with_handlers(
            false,
            || enable_count.set(enable_count.get() + 1),
            || disable_count.set(disable_count.get() + 1),
        );
        assert_eq!(enable_count.get(), 1);
        assert_eq!(disable_count.get(), 1);
    }
}
