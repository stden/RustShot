//! Application settings persistence and settings dialog.
//!
//! Settings are stored as a JSON file in the OS app-data directory
//! (`AppData/Roaming/RustShot/settings.json` on Windows).
//!
//! # Design decision: hand-rolled JSON
//!
//! The settings schema is small and stable (5 fields). A hand-rolled
//! JSON parser/formatter avoids pulling in `serde` + `serde_json`
//! (~30 transitive crates), keeping compile times fast and the binary
//! small. The trade-off is acceptable because the format never changes
//! at runtime and round-trip fidelity is covered by unit tests.
//!
//! On Windows the tray menu opens a native settings dialog
//! (`crate::settings_dialog`); on other platforms the settings
//! file is opened in the default editor.

use crate::capture::Format;
use chrono::Local;
use std::fs;
use std::path::PathBuf;

mod json;
pub use json::*;

/// Generate a nanosecond-precision timestamp string for file names.
///
/// Format: `YYYYMMDD_HHMMSS_NNNNNNNNN`
pub fn generate_timestamp() -> String {
    let now = Local::now();
    format!(
        "{}_{:09}",
        now.format("%Y%m%d_%H%M%S"),
        now.timestamp_subsec_nanos()
    )
}

/// Recording frames per second.
///
/// A newtype rather than a bare `u32` so the valid range is carried by the type:
/// once you hold an `Fps`, it is in `1..=Fps::MAX`. That makes an out-of-range
/// FPS unrepresentable in [`Settings`] instead of something every consumer has
/// to re-check.
///
/// Two constructors, for the two different trust levels:
/// - [`Fps::new`] — strict. Rejects `0` and anything above [`Fps::MAX`]. Used at
///   the boundary, e.g. validating a recording request or dialog input.
/// - [`Fps::from_settings`] — forgiving. Falls back to [`Fps::DEFAULT`] for any
///   invalid value, because `settings.json` is hand-editable and a bad number
///   should not brick the app. This mirrors how an unknown image format falls
///   back to PNG.
///
/// # Examples
///
/// ```
/// use rustshot::settings::Fps;
///
/// assert_eq!(Fps::new(60).unwrap().get(), 60);
/// assert!(Fps::new(0).is_err());
/// assert!(Fps::new(300).is_err());
///
/// // Forgiving path: invalid values become the default rather than an error.
/// assert_eq!(Fps::from_settings(0), Fps::DEFAULT);
/// assert_eq!(Fps::from_settings(300), Fps::DEFAULT);
/// assert_eq!(Fps::from_settings(60).get(), 60);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fps(u32);

impl Fps {
    /// Maximum FPS a recording may request.
    pub const MAX: u32 = 240;

    /// The default recording rate, used whenever a value is missing or invalid.
    pub const DEFAULT: Fps = Fps(30);

    /// Strict constructor: the only way to build an `Fps` from untrusted input.
    pub fn new(value: u32) -> Result<Self, String> {
        if value == 0 {
            return Err("Recording FPS must be greater than 0".to_string());
        }
        if value > Self::MAX {
            return Err(format!("Recording FPS must be {} or less", Self::MAX));
        }
        Ok(Self(value))
    }

    /// Forgiving constructor for hand-edited `settings.json`: invalid values
    /// become [`Fps::DEFAULT`] rather than an error.
    pub fn from_settings(value: u32) -> Self {
        Self::new(value).unwrap_or(Self::DEFAULT)
    }

    /// The underlying rate. Guaranteed to be in `1..=Fps::MAX`.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for Fps {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u32> for Fps {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Fps> for u32 {
    fn from(fps: Fps) -> Self {
        fps.get()
    }
}

impl std::fmt::Display for Fps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Convert a Rust `&str` to a null-terminated UTF-16 vector for Win32 APIs.
///
/// # Examples
///
/// ```ignore
/// let v = wide("hi");
/// assert_eq!(v, vec![b'h' as u16, b'i' as u16, 0]);
/// ```
#[cfg(any(test, windows))]
pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
static SETTINGS_PATH_OVERRIDE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

#[cfg(test)]
static OPEN_FILE_EDITOR_OVERRIDE: std::sync::Mutex<Option<fn(&std::path::Path)>> =
    std::sync::Mutex::new(None);

/// Application configuration persisted across sessions.
///
/// The format and FPS fields are typed rather than `String`/`u32`: parsing
/// happens once, at the JSON boundary, and every consumer downstream gets a
/// value that is valid by construction. There is no "normalize before use" step
/// for callers to forget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Directory where screenshots are saved.
    pub screenshots_dir: PathBuf,
    /// Directory where recordings are saved.
    pub recordings_dir: PathBuf,
    /// Default image format. Serialized as its extension (`"png"`/`"jpg"`/`"bmp"`).
    pub default_format: Format,
    /// Recording frames per second. Always in `1..=Fps::MAX`.
    pub recording_fps: Fps,
    /// Whether to auto-start on Windows login.
    pub auto_start: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let pictures = dirs::picture_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            screenshots_dir: pictures.join("RustShot"),
            recordings_dir: pictures.join("RustShot").join("recordings"),
            default_format: Format::Png,
            recording_fps: Fps::DEFAULT,
            auto_start: true,
        }
    }
}

/// Return the path to the settings JSON file.
pub fn settings_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = SETTINGS_PATH_OVERRIDE.lock().unwrap().clone() {
        return path;
    }

    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustShot")
        .join("settings.json")
}

/// Return the path to the log file (`%APPDATA%/RustShot/rustshot.log` on Windows).
#[allow(dead_code)]
pub fn log_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustShot")
        .join("rustshot.log")
}

/// Load settings from disk, falling back to defaults.
pub fn load_settings() -> Settings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(content) => parse_settings_json(&content),
        Err(_) => Settings::default(),
    }
}

/// Save settings to disk as JSON, atomically.
///
/// Writes a scratch file beside the target and renames it into place rather than
/// truncating the target and writing into it. RustShot runs several processes
/// that read this file — the `--select` and `--select-record` children and the
/// MCP server — while the settings dialog may be writing it. A truncate-then-
/// write leaves a window where a reader sees an empty or partial file, and
/// [`parse_settings_json`] turns that into [`Settings::default`] without an
/// error, so screenshots would silently start landing in the wrong directory.
///
/// `fs::rename` replaces the destination atomically on Windows and POSIX alike,
/// so a concurrent reader observes either the old file or the new one. The
/// scratch name carries the process id so two processes saving at once cannot
/// clobber each other's temporary file.
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let json = format_settings_json(settings);

    let mut scratch = path.clone().into_os_string();
    scratch.push(format!(".{}.tmp", std::process::id()));
    let scratch = PathBuf::from(scratch);

    fs::write(&scratch, json).map_err(|e| format!("Failed to write settings: {}", e))?;
    fs::rename(&scratch, &path).map_err(|e| {
        // Do not leave the scratch file behind if the rename failed.
        let _ = fs::remove_file(&scratch);
        format!("Failed to write settings: {}", e)
    })
}

/// Open the settings file in the default editor (or create it with defaults first).
#[cfg_attr(windows, allow(dead_code))]
pub fn open_settings_dialog() {
    let settings = load_settings();
    if save_settings(&settings).is_err() {
        return;
    }
    let path = settings_path();
    open_file_in_editor(&path);
}

#[cfg_attr(windows, allow(dead_code))]
fn open_file_in_editor(path: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = launch_editor_command("notepad", path);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = launch_editor_command("open", path);
    }
    #[cfg(target_os = "linux")]
    {
        let _ = launch_editor_command("xdg-open", path);
    }
}

#[cfg(any(test, windows, target_os = "macos", target_os = "linux"))]
fn launch_editor_command(program: &str, path: &std::path::Path) -> Result<(), std::io::Error> {
    #[cfg(test)]
    if let Some(callback) = OPEN_FILE_EDITOR_OVERRIDE.lock().unwrap().as_ref().copied() {
        callback(path);
        return Ok(());
    }

    std::process::Command::new(program)
        .arg(path)
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests;
