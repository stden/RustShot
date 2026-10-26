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
//! ([`crate::settings_dialog`]); on other platforms the settings
//! file is opened in the default editor.

use chrono::Local;
#[cfg(test)]
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;

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

/// Image formats supported by RustShot.
pub const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "bmp"];

/// Maximum allowed recording FPS.
#[allow(dead_code)]
pub const MAX_RECORDING_FPS: u32 = 240;

/// Convert a Rust `&str` to a null-terminated UTF-16 vector for Win32 APIs.
///
/// # Examples
///
/// ```ignore
/// let v = wide("hi");
/// assert_eq!(v, vec![b'h' as u16, b'i' as u16, 0]);
/// ```
#[cfg(any(test, windows))]
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// Thread-local so that each test thread sees only its own overrides.
#[cfg(test)]
thread_local! {
    static SETTINGS_PATH_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

#[cfg(test)]
fn set_settings_path_for_tests(path: Option<PathBuf>) {
    SETTINGS_PATH_OVERRIDE.with(|cell| *cell.borrow_mut() = path);
}

#[cfg(test)]
thread_local! {
    static OPEN_FILE_EDITOR_OVERRIDE: Cell<Option<fn(&std::path::Path)>> = const { Cell::new(None) };
}

#[cfg(test)]
fn set_open_file_in_editor_for_tests(callback: Option<fn(&std::path::Path)>) {
    OPEN_FILE_EDITOR_OVERRIDE.with(|cell| cell.set(callback));
}

/// Application configuration persisted across sessions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Directory where screenshots are saved.
    pub screenshots_dir: PathBuf,
    /// Directory where recordings are saved.
    pub recordings_dir: PathBuf,
    /// Default image format: "png", "jpg", or "bmp".
    pub default_format: String,
    /// Recording frames per second.
    pub recording_fps: u32,
    /// Whether to auto-start on Windows login.
    pub auto_start: bool,
}

pub(crate) fn dir_or_current(dir: Option<PathBuf>) -> PathBuf {
    dir.unwrap_or_else(|| PathBuf::from("."))
}

impl Default for Settings {
    fn default() -> Self {
        let pictures = dir_or_current(dirs::picture_dir());
        Self {
            screenshots_dir: pictures.join("RustShot"),
            recordings_dir: pictures.join("RustShot").join("recordings"),
            default_format: "png".to_string(),
            recording_fps: 30,
            auto_start: true,
        }
    }
}

/// Return the path to the settings JSON file.
pub fn settings_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = SETTINGS_PATH_OVERRIDE.with(|cell| cell.borrow().clone()) {
        return path;
    }

    dir_or_current(dirs::config_dir())
        .join("RustShot")
        .join("settings.json")
}

/// Return the path to the log file (`%APPDATA%/RustShot/rustshot.log` on Windows).
#[allow(dead_code)]
pub fn log_file_path() -> PathBuf {
    dir_or_current(dirs::config_dir())
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

/// Save settings to disk as JSON.
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let json = format_settings_json(settings);
    fs::write(&path, json).map_err(|e| format!("Failed to write settings: {}", e))
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
    if let Some(callback) = OPEN_FILE_EDITOR_OVERRIDE.with(Cell::get) {
        callback(path);
        return Ok(());
    }

    std::process::Command::new(program)
        .arg(path)
        .spawn()
        .map(|_| ())
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\u{0008}' => escaped.push_str("\\b"),
            '\u{000C}' => escaped.push_str("\\f"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn unescape_json_string(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            unescaped.push(ch);
            continue;
        }

        match chars.next() {
            Some('"') => unescaped.push('"'),
            Some('\\') => unescaped.push('\\'),
            Some('/') => unescaped.push('/'),
            Some('b') => unescaped.push('\u{0008}'),
            Some('f') => unescaped.push('\u{000C}'),
            Some('n') => unescaped.push('\n'),
            Some('r') => unescaped.push('\r'),
            Some('t') => unescaped.push('\t'),
            Some(other) => {
                unescaped.push('\\');
                unescaped.push(other);
            }
            None => unescaped.push('\\'),
        }
    }

    unescaped
}

/// Serialize settings to a JSON string (manual, no serde dependency).
pub fn format_settings_json(settings: &Settings) -> String {
    let screenshots = escape_json_string(&settings.screenshots_dir.display().to_string());
    let recordings = escape_json_string(&settings.recordings_dir.display().to_string());
    let default_format = escape_json_string(&settings.default_format);
    let mut buf = String::with_capacity(256);
    buf.push_str("{\n  \"screenshots_dir\": \"");
    buf.push_str(&screenshots);
    buf.push_str("\",\n  \"recordings_dir\": \"");
    buf.push_str(&recordings);
    buf.push_str("\",\n  \"default_format\": \"");
    buf.push_str(&default_format);
    buf.push_str("\",\n  \"recording_fps\": ");
    buf.push_str(&settings.recording_fps.to_string());
    buf.push_str(",\n  \"auto_start\": ");
    buf.push_str(if settings.auto_start { "true" } else { "false" });
    buf.push_str("\n}");
    buf
}

/// Parse the RustShot settings JSON using the project's minimal handwritten parser.
///
/// Algorithm:
/// 1. Start from [`Settings::default`].
/// 2. Probe each supported key independently with a type-specific extractor.
/// 3. Overwrite only the fields that were parsed successfully.
/// 4. Normalize values whose textual form may still be invalid, such as the
///    default format and zero FPS.
/// 5. Return the partially populated settings object.
///
/// The parser is intentionally forgiving: malformed fields fall back to
/// defaults instead of rejecting the whole file, which is friendlier for a
/// user-editable config.
///
/// Possible improvements:
/// - Emit warnings for ignored fields so bad manual edits are visible.
/// - Move to a real JSON parser if the format grows beyond the current small
///   flat schema.
pub fn parse_settings_json(json: &str) -> Settings {
    const SEEN_SCREENSHOTS_DIR: u8 = 1 << 0;
    const SEEN_RECORDINGS_DIR: u8 = 1 << 1;
    const SEEN_DEFAULT_FORMAT: u8 = 1 << 2;
    const SEEN_RECORDING_FPS: u8 = 1 << 3;
    const SEEN_AUTO_START: u8 = 1 << 4;
    const SEEN_ALL: u8 = SEEN_SCREENSHOTS_DIR
        | SEEN_RECORDINGS_DIR
        | SEEN_DEFAULT_FORMAT
        | SEEN_RECORDING_FPS
        | SEEN_AUTO_START;

    let mut screenshots_dir: Option<PathBuf> = None;
    let mut recordings_dir: Option<PathBuf> = None;
    let mut default_format: Option<String> = None;
    let mut recording_fps: u32 = 30;
    let mut auto_start: bool = true;
    let mut seen_mask: u8 = 0;

    let bytes = json.as_bytes();
    let mut index = 0;
    let mut depth = 0usize;

    while index < bytes.len() && seen_mask != SEEN_ALL {
        match bytes[index] {
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            b'"' => {
                let Some((token_end, _)) = scan_json_string_end(bytes, index + 1) else {
                    break;
                };
                let key = &json[index + 1..token_end];
                index = token_end + 1;

                if depth != 1 {
                    continue;
                }

                let key_bit = match key {
                    "screenshots_dir" => SEEN_SCREENSHOTS_DIR,
                    "recordings_dir" => SEEN_RECORDINGS_DIR,
                    "default_format" => SEEN_DEFAULT_FORMAT,
                    "recording_fps" => SEEN_RECORDING_FPS,
                    "auto_start" => SEEN_AUTO_START,
                    _ => 0,
                };

                while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                    index += 1;
                }
                if bytes.get(index) != Some(&b':') {
                    continue;
                }
                index += 1;
                while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                    index += 1;
                }

                let is_new_known_key = key_bit != 0 && (seen_mask & key_bit) == 0;
                seen_mask |= key_bit;
                let after_colon = &json[index..];

                if is_new_known_key
                    && (key_bit
                        & (SEEN_SCREENSHOTS_DIR | SEEN_RECORDINGS_DIR | SEEN_DEFAULT_FORMAT))
                        != 0
                {
                    if let Some((val, consumed)) = parse_json_string_value(after_colon) {
                        index += consumed;
                        match key_bit {
                            SEEN_SCREENSHOTS_DIR if !val.is_empty() => {
                                screenshots_dir = Some(PathBuf::from(val));
                            }
                            SEEN_RECORDINGS_DIR if !val.is_empty() => {
                                recordings_dir = Some(PathBuf::from(val));
                            }
                            SEEN_DEFAULT_FORMAT => default_format = Some(val),
                            _ => {}
                        }
                    }
                } else if bytes.get(index) == Some(&b'"') {
                    if let Some((val_end, _)) = scan_json_string_end(bytes, index + 1) {
                        index = val_end + 1;
                    }
                } else if is_new_known_key {
                    if key_bit == SEEN_RECORDING_FPS {
                        if let Some(val) = parse_json_number_value(after_colon) {
                            recording_fps = val;
                        }
                    } else if let Some(val) = parse_json_bool_value(after_colon) {
                        auto_start = val;
                    }
                }
            }
            _ => index += 1,
        }
    }

    let (screenshots_dir, recordings_dir) = match (screenshots_dir, recordings_dir) {
        (Some(shots), Some(recs)) => (shots, recs),
        (shots, recs) => {
            let pictures = dir_or_current(dirs::picture_dir());
            let default_shots = pictures.join("RustShot");
            let final_recs = recs.unwrap_or_else(|| default_shots.join("recordings"));
            let final_shots = shots.unwrap_or(default_shots);
            (final_shots, final_recs)
        }
    };

    Settings {
        screenshots_dir,
        recordings_dir,
        default_format: normalize_default_format(default_format.as_deref().unwrap_or("png"))
            .to_string(),
        recording_fps: normalize_recording_fps(recording_fps),
        auto_start,
    }
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_normalize_default_format(value: &str) -> &'static str {
    normalize_default_format(value)
}

/// Clamp a format string to one of [`SUPPORTED_FORMATS`], defaulting to `"png"`.
///
/// # Examples
///
/// ```
/// use rustshot::settings::normalize_default_format;
///
/// assert_eq!(normalize_default_format("jpg"), "jpg");
/// assert_eq!(normalize_default_format("webp"), "png"); // unknown → default
/// ```
pub fn normalize_default_format(value: &str) -> &'static str {
    SUPPORTED_FORMATS
        .iter()
        .find(|&&fmt| fmt == value)
        .copied()
        .unwrap_or(SUPPORTED_FORMATS[0])
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_normalize_recording_fps(value: u32) -> u32 {
    normalize_recording_fps(value)
}

/// Normalise a recording FPS value: zero becomes the default (30).
///
/// Range validation against [`MAX_RECORDING_FPS`] is handled separately by
/// [`crate::record::validate_recording_start`].
///
/// # Examples
///
/// ```
/// use rustshot::settings::normalize_recording_fps;
///
/// assert_eq!(normalize_recording_fps(60), 60);
/// assert_eq!(normalize_recording_fps(0), 30); // zero → default
/// ```
pub fn normalize_recording_fps(value: u32) -> u32 {
    if value == 0 {
        30
    } else {
        value
    }
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_extract_json_string(json: &str, key: &str) -> Option<String> {
    extract_json_string(json, key)
}

/// Scan forward from `start` (the byte immediately after an opening `"`) until
/// the matching unescaped closing `"`, returning `(closing_quote_idx, saw_escape)`.
#[inline]
fn scan_json_string_end(bytes: &[u8], start: usize) -> Option<(usize, bool)> {
    let mut index = start;
    let mut escaped = false;
    let mut saw_escape = false;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' if !escaped => {
                escaped = true;
                saw_escape = true;
            }
            b'"' if !escaped => return Some((index, saw_escape)),
            _ => escaped = false,
        }
        index += 1;
    }

    None
}

/// Find a top-level JSON key and return the trimmed text after the colon.
///
/// Algorithm:
/// 1. Scan the JSON byte-by-byte while tracking object/array nesting depth.
/// 2. Parse each quoted string token, honoring backslash escapes.
/// 3. At top-level object depth, treat a matching string followed by `:` as a
///    key.
/// 4. Return the text after that colon with leading whitespace removed.
///
/// Possible improvements:
/// - Replace the repeated string search with a streaming tokenizer if settings
///   files ever become large or nested.
/// - Track byte offsets for better parse diagnostics instead of silently
///   returning `None`.
fn find_json_value<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let bytes = json.as_bytes();
    let mut index = 0;
    let mut depth = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            b'"' => {
                let string_start = index + 1;
                let (string_end, _) = scan_json_string_end(bytes, string_start)?;
                index = string_end + 1;

                if depth != 1 || &json[string_start..string_end] != key {
                    continue;
                }

                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }

                if index >= bytes.len() || bytes[index] != b':' {
                    continue;
                }

                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }

                return Some(&json[index..]);
            }
            _ => index += 1,
        }
    }

    None
}

fn has_json_value_boundary(rest: &str) -> bool {
    rest.as_bytes()
        .first()
        .is_none_or(|&b| b.is_ascii_whitespace() || b == b',' || b == b'}' || b == b']')
}

fn parse_json_string_value(after_colon: &str) -> Option<(String, usize)> {
    let bytes = after_colon.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let (end_idx, saw_escape) = scan_json_string_end(bytes, 1)?;
    let raw = &after_colon[1..end_idx];
    let parsed = if saw_escape {
        unescape_json_string(raw)
    } else {
        raw.to_string()
    };
    Some((parsed, end_idx + 1))
}

fn parse_json_number_value(after_colon: &str) -> Option<u32> {
    let bytes = after_colon.as_bytes();
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 || !has_json_value_boundary(&after_colon[end..]) {
        return None;
    }
    after_colon[..end].parse().ok()
}

fn parse_json_bool_value(after_colon: &str) -> Option<bool> {
    if let Some(rest) = after_colon.strip_prefix("true") {
        if has_json_value_boundary(rest) {
            return Some(true);
        }
    } else if let Some(rest) = after_colon.strip_prefix("false") {
        if has_json_value_boundary(rest) {
            return Some(false);
        }
    }
    None
}

/// Extract a JSON string value for `key` using a minimal escape-aware scan.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let after_colon = find_json_value(json, key)?;
    parse_json_string_value(after_colon).map(|(val, _)| val)
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_extract_json_number(json: &str, key: &str) -> Option<u32> {
    extract_json_number(json, key)
}

/// Extract a positive integer value by consuming consecutive ASCII digits.
fn extract_json_number(json: &str, key: &str) -> Option<u32> {
    let after_colon = find_json_value(json, key)?;
    parse_json_number_value(after_colon)
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_extract_json_bool(json: &str, key: &str) -> Option<bool> {
    extract_json_bool(json, key)
}

/// Extract a JSON boolean while enforcing a token boundary after the literal.
fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let after_colon = find_json_value(json, key)?;
    parse_json_bool_value(after_colon)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn settings_default_format_is_png() {
        let s = Settings::default();
        assert_eq!(s.default_format, "png");
    }

    #[test]
    fn settings_default_fps_is_30() {
        let s = Settings::default();
        assert_eq!(s.recording_fps, 30);
    }

    #[test]
    fn settings_default_auto_start_is_true() {
        let s = Settings::default();
        assert!(s.auto_start);
    }

    #[test]
    fn settings_default_screenshots_dir_ends_with_rustshot() {
        let s = Settings::default();
        assert_eq!(
            s.screenshots_dir.file_name().and_then(|n| n.to_str()),
            Some("RustShot")
        );
    }

    #[test]
    fn settings_default_recordings_dir_ends_with_recordings() {
        let s = Settings::default();
        assert_eq!(
            s.recordings_dir.file_name().and_then(|n| n.to_str()),
            Some("recordings")
        );
    }

    #[test]
    fn settings_path_ends_with_settings_json() {
        let p = settings_path();
        assert_eq!(
            p.file_name().and_then(|n| n.to_str()),
            Some("settings.json")
        );
    }

    #[test]
    fn settings_path_parent_is_rustshot() {
        let p = settings_path();
        let parent = p.parent().unwrap();
        assert_eq!(
            parent.file_name().and_then(|n| n.to_str()),
            Some("RustShot")
        );
    }

    #[test]
    fn format_settings_json_contains_all_keys() {
        let s = Settings::default();
        let json = format_settings_json(&s);
        assert!(json.contains("screenshots_dir"));
        assert!(json.contains("recordings_dir"));
        assert!(json.contains("default_format"));
        assert!(json.contains("recording_fps"));
        assert!(json.contains("auto_start"));
    }

    #[test]
    fn format_settings_json_contains_png() {
        let s = Settings::default();
        let json = format_settings_json(&s);
        assert!(json.contains("\"png\""));
    }

    #[test]
    fn format_settings_json_contains_fps_30() {
        let s = Settings::default();
        let json = format_settings_json(&s);
        assert!(json.contains("30"));
    }

    #[test]
    fn parse_settings_json_roundtrip() {
        let original = Settings {
            screenshots_dir: PathBuf::from("/tmp/shots"),
            recordings_dir: PathBuf::from("/tmp/recs"),
            default_format: "jpg".to_string(),
            recording_fps: 60,
            auto_start: false,
        };
        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.default_format, "jpg");
        assert_eq!(parsed.recording_fps, 60);
        assert!(!parsed.auto_start);
    }

    #[test]
    fn parse_settings_json_empty_returns_defaults() {
        let parsed = parse_settings_json("{}");
        assert_eq!(parsed.default_format, "png");
        assert_eq!(parsed.recording_fps, 30);
    }

    #[test]
    fn parse_settings_json_partial_keys() {
        let json = r#"{ "default_format": "bmp", "recording_fps": 15 }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.default_format, "bmp");
        assert_eq!(parsed.recording_fps, 15);
        assert!(parsed.auto_start); // default
    }

    #[test]
    fn parse_settings_json_ignores_similar_key_names() {
        let json = r#"{ "default_format_backup": "bmp", "recording_fps_limit": 15, "auto_startup": false }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.default_format, "png");
        assert_eq!(parsed.recording_fps, 30);
        assert!(parsed.auto_start);
    }

    #[test]
    fn extract_json_string_works() {
        let json = r#"{ "key": "value" }"#;
        assert_eq!(extract_json_string(json, "key"), Some("value".to_string()));
    }

    #[test]
    fn extract_json_string_missing_key_returns_none() {
        let json = r#"{ "other": "value" }"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    #[test]
    fn extract_json_string_does_not_match_similar_key_name() {
        let json = r#"{ "key_suffix": "value" }"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    #[test]
    fn extract_json_number_works() {
        let json = r#"{ "fps": 60 }"#;
        assert_eq!(extract_json_number(json, "fps"), Some(60));
    }

    #[test]
    fn extract_json_number_missing_returns_none() {
        let json = r#"{ "other": 10 }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_does_not_match_similar_key_name() {
        let json = r#"{ "fps_limit": 60 }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_bool_true() {
        let json = r#"{ "flag": true }"#;
        assert_eq!(extract_json_bool(json, "flag"), Some(true));
    }

    #[test]
    fn extract_json_bool_false() {
        let json = r#"{ "flag": false }"#;
        assert_eq!(extract_json_bool(json, "flag"), Some(false));
    }

    #[test]
    fn extract_json_bool_missing_returns_none() {
        let json = r#"{ "other": true }"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    #[test]
    fn extract_json_bool_does_not_match_similar_key_name() {
        let json = r#"{ "flag_enabled": true }"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    #[test]
    fn settings_clone_works() {
        let s = Settings::default();
        let s2 = s.clone();
        assert_eq!(s, s2);
    }

    #[test]
    fn settings_debug_is_non_empty() {
        let s = Settings::default();
        assert!(!format!("{:?}", s).is_empty());
    }

    #[test]
    fn load_settings_returns_defaults_when_no_file() {
        // settings file unlikely to exist in test env at the exact expected path
        let s = load_settings();
        assert_eq!(s.default_format, "png");
    }

    #[test]
    fn save_settings_roundtrips_via_disk() {
        struct PathGuard;

        impl Drop for PathGuard {
            fn drop(&mut self) {
                set_settings_path_for_tests(None);
            }
        }

        let temp_root =
            std::env::temp_dir().join(format!("rustshot_settings_io_{}", generate_timestamp()));
        std::fs::create_dir_all(&temp_root).unwrap();
        let settings_path = temp_root.join("RustShot").join("settings.json");
        let _guard = PathGuard;
        set_settings_path_for_tests(Some(settings_path.clone()));

        let settings = Settings {
            screenshots_dir: PathBuf::from("/tmp/shots"),
            recordings_dir: PathBuf::from("/tmp/recs"),
            default_format: "jpg".to_string(),
            recording_fps: 60,
            auto_start: false,
        };

        save_settings(&settings).unwrap();
        let loaded = load_settings();
        assert_eq!(loaded, settings);

        assert!(settings_path.is_file());

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn open_settings_dialog_uses_test_override() {
        struct Guard;

        impl Drop for Guard {
            fn drop(&mut self) {
                set_settings_path_for_tests(None);
                set_open_file_in_editor_for_tests(None);
            }
        }

        static OPENED_PATHS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

        fn record_opened_path(path: &std::path::Path) {
            OPENED_PATHS.lock().unwrap().push(path.to_path_buf());
        }

        let temp_root =
            std::env::temp_dir().join(format!("rustshot_settings_dialog_{}", generate_timestamp()));
        std::fs::create_dir_all(&temp_root).unwrap();
        let settings_path = temp_root.join("RustShot").join("settings.json");
        let _guard = Guard;
        set_settings_path_for_tests(Some(settings_path.clone()));
        set_open_file_in_editor_for_tests(Some(record_opened_path));
        OPENED_PATHS.lock().unwrap().clear();

        open_settings_dialog();

        let opened_paths = OPENED_PATHS.lock().unwrap().clone();
        assert_eq!(opened_paths, vec![settings_path.clone()]);
        assert!(settings_path.is_file());

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_overrides_are_isolated_per_thread() {
        struct Guard;

        impl Drop for Guard {
            fn drop(&mut self) {
                set_settings_path_for_tests(None);
                set_open_file_in_editor_for_tests(None);
            }
        }

        fn accept_path(_path: &std::path::Path) {}

        let _guard = Guard;
        let own_path = PathBuf::from("/tmp/rustshot_isolation/RustShot/settings.json");
        set_settings_path_for_tests(Some(own_path.clone()));
        set_open_file_in_editor_for_tests(Some(accept_path));

        // A test running on another thread clears its own overrides.
        std::thread::spawn(|| {
            set_settings_path_for_tests(None);
            set_open_file_in_editor_for_tests(None);
        })
        .join()
        .unwrap();

        assert_eq!(settings_path(), own_path);
        assert!(launch_editor_command("rustshot-no-such-editor", &own_path).is_ok());
    }

    #[test]
    fn open_settings_dialog_returns_early_when_save_fails() {
        struct Guard;

        impl Drop for Guard {
            fn drop(&mut self) {
                set_settings_path_for_tests(None);
                set_open_file_in_editor_for_tests(None);
            }
        }

        static OPENED_PATHS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

        fn record_opened_path(path: &std::path::Path) {
            OPENED_PATHS.lock().unwrap().push(path.to_path_buf());
        }

        let temp_root =
            std::env::temp_dir().join(format!("rustshot_settings_fail_{}", generate_timestamp()));
        std::fs::create_dir_all(&temp_root).unwrap();
        let blocked_parent = temp_root.join("blocked_parent");
        std::fs::write(&blocked_parent, "not a directory").unwrap();
        let settings_path = blocked_parent.join("settings.json");
        let _guard = Guard;
        set_settings_path_for_tests(Some(settings_path.clone()));
        set_open_file_in_editor_for_tests(Some(record_opened_path));
        OPENED_PATHS.lock().unwrap().clear();

        open_settings_dialog();

        let opened_paths = OPENED_PATHS.lock().unwrap().clone();
        assert!(opened_paths.is_empty());
        assert!(!settings_path.is_file());

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn format_settings_json_auto_start_true() {
        let s = Settings {
            auto_start: true,
            ..Settings::default()
        };
        let json = format_settings_json(&s);
        assert!(json.contains("true"));
    }

    #[test]
    fn format_settings_json_auto_start_false() {
        let s = Settings {
            auto_start: false,
            ..Settings::default()
        };
        let json = format_settings_json(&s);
        assert!(json.contains("false"));
    }

    #[test]
    fn format_settings_json_contains_auto_start_key_once() {
        let json = format_settings_json(&Settings::default());
        assert_eq!(json.matches("\"auto_start\"").count(), 1);
    }

    // --- malformed JSON parsing ---

    #[test]
    fn parse_settings_json_garbage_input_returns_defaults() {
        let parsed = parse_settings_json("not json at all");
        assert_eq!(parsed.default_format, "png");
        assert_eq!(parsed.recording_fps, 30);
        assert!(parsed.auto_start);
    }

    #[test]
    fn parse_settings_json_unterminated_string_returns_defaults() {
        let parsed = parse_settings_json(r#"{ "default_format": "jpg"#);
        assert_eq!(parsed.default_format, "png");
        assert_eq!(parsed.recording_fps, 30);
        assert!(parsed.auto_start);
    }

    #[test]
    fn parse_settings_json_empty_string_returns_defaults() {
        let parsed = parse_settings_json("");
        assert_eq!(parsed, Settings::default());
    }

    #[test]
    fn parse_settings_json_null_value_for_string_field_keeps_default() {
        let json = r#"{ "default_format": null }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.default_format, "png");
    }

    #[test]
    fn parse_settings_json_number_for_string_field_keeps_default() {
        let json = r#"{ "default_format": 42 }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.default_format, "png");
    }

    #[test]
    fn parse_settings_json_string_for_number_field_keeps_default() {
        let json = r#"{ "recording_fps": "sixty" }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.recording_fps, 30);
    }

    #[test]
    fn parse_settings_json_float_recording_fps_keeps_default() {
        let json = r#"{ "recording_fps": 30.5 }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.recording_fps, 30);
    }

    #[test]
    fn parse_settings_json_string_for_bool_field_keeps_default() {
        let json = r#"{ "auto_start": "yes" }"#;
        let parsed = parse_settings_json(json);
        assert!(parsed.auto_start);
    }

    #[test]
    fn parse_settings_json_invalid_default_format_falls_back_to_png() {
        let json = r#"{ "default_format": "gif" }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.default_format, "png");
    }

    #[test]
    fn parse_settings_json_zero_recording_fps_falls_back_to_default() {
        let json = r#"{ "recording_fps": 0 }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.recording_fps, 30);
    }

    #[test]
    fn parse_settings_json_explicit_auto_start_false_overrides_default() {
        let json = r#"{ "auto_start": false, "recording_fps": 60 }"#;
        let parsed = parse_settings_json(json);
        assert!(!parsed.auto_start);
        assert_eq!(parsed.recording_fps, 60);
    }

    #[test]
    fn parse_settings_json_extra_commas_still_parses_valid_keys() {
        let json = r#"{ "default_format": "bmp", "recording_fps": 60, }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.default_format, "bmp");
        assert_eq!(parsed.recording_fps, 60);
    }

    // --- extract_json_string edge cases ---

    #[test]
    fn extract_json_string_no_colon_returns_none() {
        let json = r#"{ "key" "value" }"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    #[test]
    fn extract_json_string_empty_value() {
        let json = r#"{ "key": "" }"#;
        assert_eq!(extract_json_string(json, "key"), Some("".to_string()));
    }

    #[test]
    fn extract_json_string_with_spaces_around_colon() {
        let json = r#"{ "key"  :  "value" }"#;
        assert_eq!(extract_json_string(json, "key"), Some("value".to_string()));
    }

    // --- extract_json_number edge cases ---

    #[test]
    fn extract_json_number_zero() {
        let json = r#"{ "fps": 0 }"#;
        assert_eq!(extract_json_number(json, "fps"), Some(0));
    }

    #[test]
    fn extract_json_number_negative_returns_none() {
        // u32 cannot be negative, so parsing "-1" should fail
        let json = r#"{ "fps": -1 }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_float_returns_none() {
        let json = r#"{ "fps": 30.5 }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_with_alpha_suffix_returns_none() {
        let json = r#"{ "fps": 30fps }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    // --- extract_json_bool edge cases ---

    #[test]
    fn extract_json_bool_null_returns_none() {
        let json = r#"{ "flag": null }"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    #[test]
    fn extract_json_bool_one_returns_none() {
        let json = r#"{ "flag": 1 }"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    // --- Windows backslash roundtrip ---

    #[test]
    fn format_and_parse_windows_backslash_paths() {
        let settings = Settings {
            screenshots_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot"),
            recordings_dir: PathBuf::from("C:\\Users\\test\\Pictures\\RustShot\\recordings"),
            default_format: "png".to_string(),
            recording_fps: 30,
            auto_start: true,
        };
        let json = format_settings_json(&settings);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.screenshots_dir, settings.screenshots_dir);
        assert_eq!(parsed.recordings_dir, settings.recordings_dir);
    }

    #[test]
    fn format_and_parse_paths_with_embedded_quotes() {
        let settings = Settings {
            screenshots_dir: PathBuf::from("/tmp/screens\"shots"),
            recordings_dir: PathBuf::from("/tmp/reco\"rdings"),
            default_format: "png".to_string(),
            recording_fps: 30,
            auto_start: true,
        };
        let json = format_settings_json(&settings);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.screenshots_dir, settings.screenshots_dir);
        assert_eq!(parsed.recordings_dir, settings.recordings_dir);
    }

    // --- Settings equality ---

    #[test]
    fn settings_with_different_format_are_not_equal() {
        let mut a = Settings::default();
        let mut b = Settings::default();
        a.default_format = "jpg".to_string();
        b.default_format = "bmp".to_string();
        assert_ne!(a, b);
    }

    #[test]
    fn settings_with_different_fps_are_not_equal() {
        let mut a = Settings::default();
        let mut b = Settings::default();
        a.recording_fps = 30;
        b.recording_fps = 60;
        assert_ne!(a, b);
    }

    #[test]
    fn settings_with_different_auto_start_are_not_equal() {
        let mut a = Settings::default();
        let mut b = Settings::default();
        a.auto_start = true;
        b.auto_start = false;
        assert_ne!(a, b);
    }

    // --- format_settings_json output structure ---

    #[test]
    fn format_settings_json_is_valid_json_structure() {
        let s = Settings::default();
        let json = format_settings_json(&s);
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn format_settings_json_custom_values() {
        let s = Settings {
            screenshots_dir: PathBuf::from("/custom/shots"),
            recordings_dir: PathBuf::from("/custom/recs"),
            default_format: "bmp".to_string(),
            recording_fps: 120,
            auto_start: false,
        };
        let json = format_settings_json(&s);
        assert!(json.contains("/custom/shots"));
        assert!(json.contains("/custom/recs"));
        assert!(json.contains("\"bmp\""));
        assert!(json.contains("120"));
        assert!(json.contains("false"));
    }

    #[test]
    fn format_settings_json_escapes_backslashes_in_paths() {
        let s = Settings {
            screenshots_dir: PathBuf::from(r"C:\Users\test\Shots"),
            recordings_dir: PathBuf::from(r"C:\Users\test\Shots\recordings"),
            ..Settings::default()
        };
        let json = format_settings_json(&s);
        assert!(json.contains(r#""screenshots_dir": "C:\\Users\\test\\Shots""#));
        assert!(json.contains(r#""recordings_dir": "C:\\Users\\test\\Shots\\recordings""#));
    }

    // --- parse_settings_json roundtrip with all fields changed ---

    #[test]
    fn parse_settings_json_full_roundtrip_all_fields() {
        let original = Settings {
            screenshots_dir: PathBuf::from("/a/b/c"),
            recordings_dir: PathBuf::from("/d/e/f"),
            default_format: "bmp".to_string(),
            recording_fps: 120,
            auto_start: false,
        };
        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
        assert_eq!(parsed.recordings_dir, original.recordings_dir);
        assert_eq!(parsed.default_format, original.default_format);
        assert_eq!(parsed.recording_fps, original.recording_fps);
        assert_eq!(parsed.auto_start, original.auto_start);
    }

    // --- duplicate keys ---

    #[test]
    fn parse_settings_json_duplicate_key_uses_first_occurrence() {
        // The parser uses find() which returns the first match
        let json = r#"{ "default_format": "jpg", "default_format": "bmp" }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.default_format, "jpg");
    }

    #[test]
    fn parse_settings_json_duplicate_recording_fps_uses_first_occurrence() {
        let json = r#"{ "recording_fps": 15, "recording_fps": 60 }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.recording_fps, 15);
    }

    #[test]
    fn parse_settings_json_duplicate_auto_start_uses_first_occurrence() {
        let json = r#"{ "auto_start": false, "auto_start": true }"#;
        let parsed = parse_settings_json(json);
        assert!(!parsed.auto_start);
    }

    #[test]
    fn parse_settings_json_duplicate_screenshots_dir_uses_first_occurrence() {
        let json = r#"{ "screenshots_dir": "/first", "screenshots_dir": "/second" }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.screenshots_dir, PathBuf::from("/first"));
    }

    #[test]
    fn parse_settings_json_duplicate_recordings_dir_uses_first_occurrence() {
        let json = r#"{ "recordings_dir": "/first-recs", "recordings_dir": "/second-recs" }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(parsed.recordings_dir, PathBuf::from("/first-recs"));
    }

    #[test]
    fn parse_settings_json_unescapes_backslashes_in_paths() {
        let json = r#"{ "screenshots_dir": "C:\\Users\\test\\Shots", "recordings_dir": "C:\\Users\\test\\Shots\\recordings" }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(
            parsed.screenshots_dir,
            PathBuf::from(r"C:\Users\test\Shots")
        );
        assert_eq!(
            parsed.recordings_dir,
            PathBuf::from(r"C:\Users\test\Shots\recordings")
        );
    }

    #[test]
    fn parse_settings_json_preserves_unc_paths() {
        let original = Settings {
            screenshots_dir: PathBuf::from(r"\\server\share\shots"),
            recordings_dir: PathBuf::from(r"\\server\share\shots\recordings"),
            ..Settings::default()
        };

        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);

        assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
        assert_eq!(parsed.recordings_dir, original.recordings_dir);
    }

    // --- extract_json_string with special content ---

    #[test]
    fn extract_json_string_with_colons_in_value() {
        // Value contains a colon — parser should stop at the closing quote
        let json = r#"{ "path": "C:" }"#;
        assert_eq!(extract_json_string(json, "path"), Some("C:".to_string()));
    }

    #[test]
    fn extract_json_string_with_spaces_in_value() {
        let json = r#"{ "dir": "My Pictures" }"#;
        assert_eq!(
            extract_json_string(json, "dir"),
            Some("My Pictures".to_string())
        );
    }

    #[test]
    fn extract_json_string_with_escaped_quote() {
        let json = "{\"path\": \"a\\\"b\"}";
        assert_eq!(extract_json_string(json, "path"), Some("a\"b".to_string()));
    }

    // --- extract_json_number boundary values ---

    #[test]
    fn extract_json_number_large_value() {
        let json = r#"{ "fps": 4294967295 }"#;
        assert_eq!(extract_json_number(json, "fps"), Some(u32::MAX));
    }

    #[test]
    fn extract_json_number_overflow_returns_none() {
        // u32::MAX + 1 = 4294967296, which cannot fit in u32
        let json = r#"{ "fps": 4294967296 }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_empty_after_colon_returns_none() {
        let json = r#"{ "fps": }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    // --- extract_json_bool with whitespace ---

    #[test]
    fn extract_json_bool_with_leading_whitespace() {
        let json = r#"{ "flag":   true }"#;
        assert_eq!(extract_json_bool(json, "flag"), Some(true));
    }

    #[test]
    fn extract_json_bool_false_with_trailing_content() {
        let json = r#"{ "flag": false, "other": 1 }"#;
        assert_eq!(extract_json_bool(json, "flag"), Some(false));
    }

    #[test]
    fn extract_json_bool_true_with_trailing_content() {
        let json = r#"{ "flag": true, "other": 1 }"#;
        assert_eq!(extract_json_bool(json, "flag"), Some(true));
    }

    #[test]
    fn extract_json_bool_trueish_returns_none() {
        let json = r#"{ "flag": trueish }"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    #[test]
    fn extract_json_bool_falsehood_returns_none() {
        let json = r#"{ "flag": falsehood }"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    #[test]
    fn parse_settings_json_invalid_bool_token_keeps_default() {
        let json = r#"{ "auto_start": trueish }"#;
        let parsed = parse_settings_json(json);
        assert!(parsed.auto_start);
    }

    // --- settings_path consistency ---

    #[test]
    fn settings_path_is_consistent_across_calls() {
        let p1 = settings_path();
        let p2 = settings_path();
        assert_eq!(p1, p2);
    }

    #[test]
    fn settings_path_has_at_least_two_components() {
        let p = settings_path();
        assert!(
            p.components().count() >= 2,
            "settings path should have at least parent/settings.json"
        );
    }

    // --- format_settings_json structure ---

    #[test]
    fn format_settings_json_contains_newlines() {
        let s = Settings::default();
        let json = format_settings_json(&s);
        assert!(json.contains('\n'), "JSON should be pretty-printed");
    }

    #[test]
    fn format_settings_json_contains_at_least_five_colons() {
        let s = Settings::default();
        let json = format_settings_json(&s);
        let colon_count = json.chars().filter(|&c| c == ':').count();
        assert!(
            colon_count >= 5,
            "should have at least 5 colons (one per key-value pair), got {colon_count}"
        );
    }

    #[test]
    fn log_file_path_ends_with_rustshot_log() {
        let p = log_file_path();
        assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("rustshot.log"));
    }

    #[test]
    fn log_file_path_parent_is_rustshot() {
        let p = log_file_path();
        let parent = p.parent().unwrap();
        assert_eq!(
            parent.file_name().and_then(|n| n.to_str()),
            Some("RustShot")
        );
    }

    #[test]
    fn log_file_path_is_consistent_across_calls() {
        let p1 = log_file_path();
        let p2 = log_file_path();
        assert_eq!(p1, p2);
    }

    // --- normalize_default_format tests ---

    #[test]
    fn normalize_default_format_png_is_valid() {
        assert_eq!(normalize_default_format("png"), "png");
    }

    #[test]
    fn normalize_default_format_jpg_is_valid() {
        assert_eq!(normalize_default_format("jpg"), "jpg");
    }

    #[test]
    fn normalize_default_format_bmp_is_valid() {
        assert_eq!(normalize_default_format("bmp"), "bmp");
    }

    #[test]
    fn normalize_default_format_unknown_falls_back_to_png() {
        assert_eq!(normalize_default_format("gif"), "png");
    }

    #[test]
    fn normalize_default_format_empty_falls_back_to_png() {
        assert_eq!(normalize_default_format(""), "png");
    }

    #[test]
    fn normalize_default_format_case_sensitive() {
        assert_eq!(normalize_default_format("PNG"), "png");
        assert_eq!(normalize_default_format("Jpg"), "png");
    }

    // --- normalize_recording_fps tests ---

    #[test]
    fn normalize_recording_fps_zero_returns_30() {
        assert_eq!(normalize_recording_fps(0), 30);
    }

    #[test]
    fn normalize_recording_fps_nonzero_returns_same() {
        assert_eq!(normalize_recording_fps(60), 60);
    }

    #[test]
    fn normalize_recording_fps_one_is_valid() {
        assert_eq!(normalize_recording_fps(1), 1);
    }

    #[test]
    fn normalize_recording_fps_large_value_passes_through() {
        assert_eq!(normalize_recording_fps(240), 240);
    }

    #[test]
    fn normalize_recording_fps_u32_max_passes_through() {
        assert_eq!(normalize_recording_fps(u32::MAX), u32::MAX);
    }

    // --- escape_json_string direct tests ---

    #[test]
    fn escape_json_string_empty_input() {
        assert_eq!(escape_json_string(""), "");
    }

    #[test]
    fn escape_json_string_no_special_chars() {
        assert_eq!(escape_json_string("hello world"), "hello world");
    }

    #[test]
    fn escape_json_string_backslash() {
        assert_eq!(escape_json_string(r"a\b"), r"a\\b");
    }

    #[test]
    fn escape_json_string_double_quote() {
        assert_eq!(escape_json_string("a\"b"), "a\\\"b");
    }

    #[test]
    fn escape_json_string_newline() {
        assert_eq!(escape_json_string("a\nb"), "a\\nb");
    }

    #[test]
    fn escape_json_string_carriage_return() {
        assert_eq!(escape_json_string("a\rb"), "a\\rb");
    }

    #[test]
    fn escape_json_string_tab() {
        assert_eq!(escape_json_string("a\tb"), "a\\tb");
    }

    #[test]
    fn escape_json_string_all_special_chars() {
        assert_eq!(escape_json_string("\\\"\n\r\t"), "\\\\\\\"\\n\\r\\t");
    }

    #[test]
    fn escape_json_string_windows_path() {
        assert_eq!(escape_json_string(r"C:\Users\test"), r"C:\\Users\\test");
    }

    // --- unescape_json_string direct tests ---

    #[test]
    fn unescape_json_string_empty_input() {
        assert_eq!(unescape_json_string(""), "");
    }

    #[test]
    fn unescape_json_string_no_escapes() {
        assert_eq!(unescape_json_string("hello"), "hello");
    }

    #[test]
    fn unescape_json_string_escaped_backslash() {
        assert_eq!(unescape_json_string(r"a\\b"), r"a\b");
    }

    #[test]
    fn unescape_json_string_escaped_quote() {
        assert_eq!(unescape_json_string(r#"a\"b"#), "a\"b");
    }

    #[test]
    fn unescape_json_string_escaped_newline() {
        assert_eq!(unescape_json_string(r"a\nb"), "a\nb");
    }

    #[test]
    fn unescape_json_string_escaped_carriage_return() {
        assert_eq!(unescape_json_string(r"a\rb"), "a\rb");
    }

    #[test]
    fn unescape_json_string_escaped_tab() {
        assert_eq!(unescape_json_string(r"a\tb"), "a\tb");
    }

    #[test]
    fn unescape_json_string_unknown_escape_preserved() {
        assert_eq!(unescape_json_string(r"a\xb"), r"a\xb");
    }

    #[test]
    fn unescape_json_string_trailing_backslash() {
        assert_eq!(unescape_json_string("abc\\"), "abc\\");
    }

    #[test]
    fn unescape_json_string_all_known_escapes() {
        assert_eq!(unescape_json_string(r#"\\\"\n\r\t"#), "\\\"\n\r\t");
    }

    #[test]
    fn escape_unescape_roundtrip() {
        let original = "C:\\Users\\test\\\"quotes\"\n\rnewlines\ttab";
        let escaped = escape_json_string(original);
        let unescaped = unescape_json_string(&escaped);
        assert_eq!(unescaped, original);
    }

    // --- find_json_value direct tests ---

    #[test]
    fn find_json_value_returns_trimmed_value_after_colon() {
        let json = r#"{ "key": "value" }"#;
        assert_eq!(find_json_value(json, "key"), Some("\"value\" }"));
    }

    #[test]
    fn find_json_value_missing_key_returns_none() {
        let json = r#"{ "other": "value" }"#;
        assert_eq!(find_json_value(json, "key"), None);
    }

    #[test]
    fn find_json_value_no_colon_returns_none() {
        let json = r#"{ "key" "value" }"#;
        assert_eq!(find_json_value(json, "key"), None);
    }

    #[test]
    fn find_json_value_number_value() {
        let json = r#"{ "fps": 60 }"#;
        let result = find_json_value(json, "fps").unwrap();
        assert!(result.starts_with("60"));
    }

    #[test]
    fn find_json_value_bool_value() {
        let json = r#"{ "flag": true }"#;
        let result = find_json_value(json, "flag").unwrap();
        assert!(result.starts_with("true"));
    }

    #[test]
    fn find_json_value_whitespace_around_colon() {
        let json = r#"{ "key"  :   "val" }"#;
        let result = find_json_value(json, "key").unwrap();
        assert!(result.starts_with("\"val\""));
    }

    #[test]
    fn find_json_value_empty_json() {
        assert_eq!(find_json_value("", "key"), None);
    }

    // -------------------------------------------------------------------------
    // save_settings roundtrip
    // -------------------------------------------------------------------------

    #[test]
    fn save_settings_creates_file_and_roundtrips() {
        let dir = std::env::temp_dir().join("rustshot_test_save_settings");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let settings = Settings {
            screenshots_dir: PathBuf::from("/tmp/shots"),
            recordings_dir: PathBuf::from("/tmp/recs"),
            default_format: "jpg".to_string(),
            recording_fps: 60,
            auto_start: false,
        };

        let json = format_settings_json(&settings);
        std::fs::write(&path, &json).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded = parse_settings_json(&content);
        assert_eq!(loaded.default_format, "jpg");
        assert_eq!(loaded.recording_fps, 60);
        assert!(!loaded.auto_start);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_settings_roundtrip_preserves_all_fields() {
        let settings = Settings {
            screenshots_dir: PathBuf::from("/home/user/Pictures"),
            recordings_dir: PathBuf::from("/home/user/Videos"),
            default_format: "bmp".to_string(),
            recording_fps: 144,
            auto_start: true,
        };
        let json = format_settings_json(&settings);
        let loaded = parse_settings_json(&json);
        assert_eq!(loaded.screenshots_dir, settings.screenshots_dir);
        assert_eq!(loaded.recordings_dir, settings.recordings_dir);
        assert_eq!(loaded.default_format, settings.default_format);
        assert_eq!(loaded.recording_fps, settings.recording_fps);
        assert_eq!(loaded.auto_start, settings.auto_start);
    }

    // -------------------------------------------------------------------------
    // extract_json_string edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_string_unterminated_string_returns_none() {
        let json = r#"{ "key": "no closing quote }"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    #[test]
    fn extract_json_string_value_with_backslash_n() {
        let json = r#"{ "path": "C:\\Users\\test" }"#;
        let result = extract_json_string(json, "path").unwrap();
        assert_eq!(result, "C:\\Users\\test");
    }

    #[test]
    fn extract_json_string_non_string_value_returns_none() {
        let json = r#"{ "key": 42 }"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    #[test]
    fn extract_json_string_null_value_returns_none() {
        let json = r#"{ "key": null }"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    #[test]
    fn extract_json_string_bool_value_returns_none() {
        let json = r#"{ "key": true }"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    // -------------------------------------------------------------------------
    // extract_json_number edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_number_string_value_returns_none() {
        let json = r#"{ "fps": "thirty" }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_bool_value_returns_none() {
        let json = r#"{ "fps": true }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_null_value_returns_none() {
        let json = r#"{ "fps": null }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_max_u32() {
        let json = r#"{ "n": 4294967295 }"#;
        assert_eq!(extract_json_number(json, "n"), Some(u32::MAX));
    }

    // -------------------------------------------------------------------------
    // extract_json_bool edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_bool_string_true_returns_none() {
        let json = r#"{ "key": "true" }"#;
        assert_eq!(extract_json_bool(json, "key"), None);
    }

    #[test]
    fn extract_json_bool_number_returns_none() {
        let json = r#"{ "key": 1 }"#;
        assert_eq!(extract_json_bool(json, "key"), None);
    }

    #[test]
    fn extract_json_bool_true_at_end_of_json() {
        let json = r#"{ "key": true}"#;
        assert_eq!(extract_json_bool(json, "key"), Some(true));
    }

    #[test]
    fn extract_json_bool_false_at_end_of_json() {
        let json = r#"{ "key": false}"#;
        assert_eq!(extract_json_bool(json, "key"), Some(false));
    }

    // -------------------------------------------------------------------------
    // normalize_default_format edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn normalize_default_format_jpeg_falls_back_to_png() {
        assert_eq!(normalize_default_format("jpeg"), "png");
    }

    #[test]
    fn normalize_default_format_whitespace_falls_back_to_png() {
        assert_eq!(normalize_default_format(" png"), "png");
    }

    #[test]
    fn normalize_default_format_tiff_falls_back_to_png() {
        assert_eq!(normalize_default_format("tiff"), "png");
    }

    // -------------------------------------------------------------------------
    // normalize_recording_fps edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn normalize_recording_fps_60_is_valid() {
        assert_eq!(normalize_recording_fps(60), 60);
    }

    #[test]
    fn normalize_recording_fps_144_is_valid() {
        assert_eq!(normalize_recording_fps(144), 144);
    }

    // -------------------------------------------------------------------------
    // find_json_value edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn find_json_value_duplicate_keys_returns_first() {
        let json = r#"{ "key": "first", "key": "second" }"#;
        let result = find_json_value(json, "key").unwrap();
        assert!(result.starts_with("\"first\""));
    }

    #[test]
    fn find_json_value_nested_json_returns_inner() {
        let json = r#"{ "outer": { "inner": 42 } }"#;
        let result = find_json_value(json, "outer").unwrap();
        assert!(result.starts_with("{"));
    }

    // -------------------------------------------------------------------------
    // parse_settings_json edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn parse_settings_json_normalizes_unknown_format_to_png() {
        let json = r#"{ "default_format": "webp" }"#;
        let s = parse_settings_json(json);
        assert_eq!(s.default_format, "png");
    }

    #[test]
    fn parse_settings_json_normalizes_zero_fps_to_30() {
        let json = r#"{ "recording_fps": 0 }"#;
        let s = parse_settings_json(json);
        assert_eq!(s.recording_fps, 30);
    }

    #[test]
    fn parse_settings_json_preserves_valid_fps() {
        let json = r#"{ "recording_fps": 120 }"#;
        let s = parse_settings_json(json);
        assert_eq!(s.recording_fps, 120);
    }

    #[test]
    fn parse_settings_json_auto_start_defaults_to_true() {
        let json = r#"{}"#;
        let s = parse_settings_json(json);
        assert!(s.auto_start);
    }

    #[test]
    fn parse_settings_json_auto_start_false() {
        let json = r#"{ "auto_start": false }"#;
        let s = parse_settings_json(json);
        assert!(!s.auto_start);
    }

    // -------------------------------------------------------------------------
    // format_settings_json edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn format_settings_json_includes_every_key() {
        let s = Settings::default();
        let json = format_settings_json(&s);
        assert!(json.contains("screenshots_dir"));
        assert!(json.contains("recordings_dir"));
        assert!(json.contains("default_format"));
        assert!(json.contains("recording_fps"));
        assert!(json.contains("auto_start"));
    }

    #[test]
    fn format_settings_json_escapes_backslashes() {
        let s = Settings {
            screenshots_dir: PathBuf::from("C:\\Users\\test"),
            ..Settings::default()
        };
        let json = format_settings_json(&s);
        assert!(json.contains("C:\\\\Users\\\\test"));
    }

    #[test]
    fn format_settings_json_output_is_valid_roundtrip() {
        let original = Settings {
            screenshots_dir: PathBuf::from("/tmp/a"),
            recordings_dir: PathBuf::from("/tmp/b"),
            default_format: "bmp".to_string(),
            recording_fps: 240,
            auto_start: false,
        };
        let json = format_settings_json(&original);
        let loaded = parse_settings_json(&json);
        assert_eq!(loaded.default_format, original.default_format);
        assert_eq!(loaded.recording_fps, original.recording_fps);
        assert_eq!(loaded.auto_start, original.auto_start);
    }

    // -------------------------------------------------------------------------
    // MAX_RECORDING_FPS constant
    // -------------------------------------------------------------------------

    #[test]
    fn max_recording_fps_is_240() {
        assert_eq!(MAX_RECORDING_FPS, 240);
    }

    // -------------------------------------------------------------------------
    // wide() helper tests
    // -------------------------------------------------------------------------

    #[test]
    fn wide_returns_null_terminated_vec() {
        let v = wide("hi");
        assert_eq!(*v.last().unwrap(), 0u16);
        assert_eq!(v.len(), 3); // 'h', 'i', '\0'
    }

    #[test]
    fn wide_empty_string_is_single_null() {
        assert_eq!(wide(""), vec![0u16]);
    }

    #[test]
    fn wide_ascii_chars() {
        let v = wide("AB");
        assert_eq!(v, vec![0x41, 0x42, 0]);
    }

    #[test]
    fn wide_unicode_char() {
        let v = wide("\u{00E9}"); // e-acute
        assert_eq!(v.len(), 2); // one u16 char + null
        assert_eq!(v[0], 0x00E9);
        assert_eq!(v[1], 0);
    }

    // -------------------------------------------------------------------------
    // Additional edge-case tests
    // -------------------------------------------------------------------------

    #[test]
    fn escape_json_string_multiple_backslashes() {
        assert_eq!(escape_json_string(r"\\"), r"\\\\");
    }

    #[test]
    fn unescape_json_string_multiple_consecutive_escapes() {
        assert_eq!(unescape_json_string(r"\\\\"), r"\\");
    }

    #[test]
    fn escape_unescape_roundtrip_windows_paths() {
        let paths = [
            r"C:\Users\test\Documents",
            r"\\server\share\path",
            r"D:\Program Files (x86)\RustShot",
        ];
        for path in paths {
            let escaped = escape_json_string(path);
            let unescaped = unescape_json_string(&escaped);
            assert_eq!(unescaped, path, "roundtrip failed for {path:?}");
        }
    }

    #[test]
    fn find_json_value_key_at_end_no_value() {
        assert!(find_json_value(r#"{"key"}"#, "key").is_none());
    }

    #[test]
    fn extract_json_string_only_opening_quote_returns_none() {
        let json = r#"{ "key": " }"#;
        // The string never closes (space then } but no closing quote for value)
        assert!(extract_json_string(json, "key").is_none());
    }

    #[test]
    fn extract_json_number_boolean_returns_none() {
        let json = r#"{ "fps": true }"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_bool_number_one_returns_none() {
        let json = r#"{ "flag": 1 }"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    #[test]
    fn parse_settings_json_unicode_in_path() {
        let json = r#"{ "screenshots_dir": "/home/пользователь/снимки" }"#;
        let parsed = parse_settings_json(json);
        assert_eq!(
            parsed.screenshots_dir,
            PathBuf::from("/home/пользователь/снимки")
        );
    }

    #[test]
    fn parse_settings_json_ignores_key_like_text_inside_string_values() {
        let json = concat!(
            "{",
            r#""screenshots_dir": "/tmp/\"recording_fps\": 999","#,
            r#""recording_fps": 30"#,
            "}"
        );

        let parsed = parse_settings_json(json);

        assert_eq!(
            parsed.screenshots_dir,
            PathBuf::from(r#"/tmp/"recording_fps": 999"#)
        );
        assert_eq!(parsed.recording_fps, 30);
    }

    #[test]
    fn parse_settings_json_ignores_nested_keys_when_top_level_key_exists() {
        let json = concat!(
            "{",
            r#""extra": { "recording_fps": 999 },"#,
            r#""recording_fps": 30"#,
            "}"
        );

        let parsed = parse_settings_json(json);

        assert_eq!(parsed.recording_fps, 30);
    }

    #[test]
    fn parse_settings_json_fps_boundary_values() {
        for fps in [1, 30, 60, 120, 240] {
            let json = format!(r#"{{ "recording_fps": {} }}"#, fps);
            let parsed = parse_settings_json(&json);
            assert_eq!(parsed.recording_fps, fps);
        }
    }

    #[test]
    fn format_parse_roundtrip_with_unicode_paths() {
        let original = Settings {
            screenshots_dir: PathBuf::from("/home/用户/截图"),
            recordings_dir: PathBuf::from("/home/用户/录像"),
            default_format: "jpg".to_string(),
            recording_fps: 24,
            auto_start: true,
        };
        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
        assert_eq!(parsed.recordings_dir, original.recordings_dir);
        assert_eq!(parsed.default_format, "jpg");
        assert_eq!(parsed.recording_fps, 24);
    }

    #[test]
    fn wide_surrogate_pair_emoji() {
        let v = wide("🦀");
        // Crab emoji U+1F980 needs a surrogate pair in UTF-16
        assert_eq!(v.len(), 3); // high surrogate + low surrogate + null
        assert_eq!(v[2], 0);
    }

    #[test]
    fn wide_mixed_ascii_and_unicode() {
        let v = wide("Rüst");
        assert_eq!(v.len(), 5); // R, ü, s, t, null
        assert_eq!(v[0], b'R' as u16);
        assert_eq!(v[1], 0x00FC); // ü
        assert_eq!(v[4], 0);
    }

    #[test]
    fn wide_long_string() {
        let s = "a".repeat(1000);
        let v = wide(&s);
        assert_eq!(v.len(), 1001); // 1000 chars + null
        assert_eq!(*v.last().unwrap(), 0);
    }

    // -------------------------------------------------------------------------
    // generate_timestamp tests
    // -------------------------------------------------------------------------

    #[test]
    fn generate_timestamp_has_correct_length() {
        let ts = generate_timestamp();
        assert_eq!(ts.len(), 25, "timestamp should be 25 chars: {ts}");
    }

    #[test]
    fn generate_timestamp_has_two_underscores() {
        let ts = generate_timestamp();
        let underscores = ts.chars().filter(|&c| c == '_').count();
        assert_eq!(underscores, 2, "timestamp should have 2 underscores: {ts}");
    }

    #[test]
    fn generate_timestamp_date_part_is_all_digits() {
        let ts = generate_timestamp();
        let date_part = &ts[..8];
        assert!(
            date_part.chars().all(|c| c.is_ascii_digit()),
            "date part should be all digits: {date_part}"
        );
    }

    #[test]
    fn generate_timestamp_time_part_is_all_digits() {
        let ts = generate_timestamp();
        let time_part = &ts[9..15];
        assert!(
            time_part.chars().all(|c| c.is_ascii_digit()),
            "time part should be all digits: {time_part}"
        );
    }

    #[test]
    fn generate_timestamp_nanos_part_is_9_digits() {
        let ts = generate_timestamp();
        let nanos_part = &ts[16..];
        assert_eq!(nanos_part.len(), 9);
        assert!(
            nanos_part.chars().all(|c| c.is_ascii_digit()),
            "nanos part should be all digits: {nanos_part}"
        );
    }

    #[test]
    fn generate_timestamp_starts_with_20() {
        let ts = generate_timestamp();
        assert!(
            ts.starts_with("20"),
            "timestamp year should start with 20: {ts}"
        );
    }

    #[test]
    fn generate_timestamp_two_calls_differ() {
        let ts1 = generate_timestamp();
        let ts2 = generate_timestamp();
        assert_eq!(ts1.len(), ts2.len());
    }

    #[test]
    fn generate_timestamp_no_spaces_or_colons() {
        let ts = generate_timestamp();
        assert!(!ts.contains(' '), "timestamp should have no spaces");
        assert!(!ts.contains(':'), "timestamp should have no colons");
        assert!(!ts.contains('/'), "timestamp should have no slashes");
    }

    // -------------------------------------------------------------------------
    // escape/unescape roundtrip tests
    // -------------------------------------------------------------------------

    #[test]
    fn escape_unescape_roundtrip_plain_text() {
        let original = "hello world";
        assert_eq!(
            unescape_json_string(&escape_json_string(original)),
            original
        );
    }

    #[test]
    fn escape_unescape_roundtrip_windows_path() {
        let original = r"C:\Users\test\Documents\RustShot";
        assert_eq!(
            unescape_json_string(&escape_json_string(original)),
            original
        );
    }

    #[test]
    fn escape_unescape_roundtrip_all_special_chars() {
        let original = "line1\nline2\r\ttab\t\"quoted\"\\end";
        assert_eq!(
            unescape_json_string(&escape_json_string(original)),
            original
        );
    }

    #[test]
    fn escape_unescape_roundtrip_empty_string() {
        assert_eq!(unescape_json_string(&escape_json_string("")), "");
    }

    #[test]
    fn escape_unescape_roundtrip_unicode() {
        let original = "日本語パス/用户/截图";
        assert_eq!(
            unescape_json_string(&escape_json_string(original)),
            original
        );
    }

    // -------------------------------------------------------------------------
    // extract_json_bool edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_bool_uppercase_true_returns_none() {
        let json = r#"{ "auto_start": TRUE }"#;
        assert_eq!(extract_json_bool(json, "auto_start"), None);
    }

    #[test]
    fn extract_json_bool_mixed_case_false_returns_none() {
        let json = r#"{ "auto_start": False }"#;
        assert_eq!(extract_json_bool(json, "auto_start"), None);
    }

    // -------------------------------------------------------------------------
    // extract_json_number edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_number_leading_whitespace_parses() {
        let json = r#"{ "recording_fps":  60 }"#;
        assert_eq!(extract_json_number(json, "recording_fps"), Some(60));
    }

    #[test]
    fn extract_json_number_with_trailing_comma() {
        let json = r#"{ "recording_fps": 120, "auto_start": true }"#;
        assert_eq!(extract_json_number(json, "recording_fps"), Some(120));
    }

    // -------------------------------------------------------------------------
    // Settings derive trait tests
    // -------------------------------------------------------------------------

    #[test]
    fn settings_clone_produces_equal_instance() {
        let s = Settings {
            screenshots_dir: PathBuf::from("/tmp/shots"),
            recordings_dir: PathBuf::from("/tmp/recs"),
            default_format: "bmp".to_string(),
            recording_fps: 60,
            auto_start: false,
        };
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }

    #[test]
    fn settings_partial_eq_detects_difference() {
        let s1 = Settings::default();
        let s2 = Settings {
            recording_fps: 999,
            ..Settings::default()
        };
        assert_ne!(s1, s2);
    }

    #[test]
    fn settings_debug_repr_is_non_empty() {
        let s = Settings::default();
        let debug = format!("{:?}", s);
        assert!(!debug.is_empty());
        assert!(debug.contains("Settings"));
    }

    // -------------------------------------------------------------------------
    // format_settings_json + parse_settings_json roundtrip with edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn format_parse_roundtrip_with_backslash_heavy_path() {
        let original = Settings {
            screenshots_dir: PathBuf::from(r"C:\Users\test\Documents\My Pictures"),
            recordings_dir: PathBuf::from(r"D:\Videos\Screen Recordings"),
            default_format: "png".to_string(),
            recording_fps: 30,
            auto_start: true,
        };
        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
        assert_eq!(parsed.recordings_dir, original.recordings_dir);
        assert_eq!(parsed.recording_fps, original.recording_fps);
        assert_eq!(parsed.auto_start, original.auto_start);
    }

    #[test]
    fn format_parse_roundtrip_preserves_auto_start_false() {
        let original = Settings {
            auto_start: false,
            ..Settings::default()
        };
        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);
        assert!(!parsed.auto_start);
    }

    // -------------------------------------------------------------------------
    // unescape_json_string — edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn unescape_json_string_trailing_backslash_preserved() {
        // A lone backslash at end of string with no following char
        let result = unescape_json_string("hello\\");
        assert_eq!(result, "hello\\");
    }

    #[test]
    fn unescape_json_string_unknown_escape_preserved_verbatim() {
        // \x is not a recognized escape → preserved as \x
        let result = unescape_json_string("hello\\xworld");
        assert_eq!(result, "hello\\xworld");
    }

    #[test]
    fn unescape_json_string_all_standard_escapes() {
        let result = unescape_json_string("\\\"\\\\\\n\\r\\t");
        assert_eq!(result, "\"\\\n\r\t");
    }

    // -------------------------------------------------------------------------
    // escape_json_string — additional characters
    // -------------------------------------------------------------------------

    #[test]
    fn escape_json_string_nul_and_control_chars_pass_through() {
        // NUL and other control chars are not escaped by our minimal escaper
        let input = "a\x00b\x01c";
        let escaped = escape_json_string(input);
        assert_eq!(
            escaped, input,
            "non-standard control chars pass through unchanged"
        );
    }

    #[test]
    fn escape_json_string_unicode_unchanged() {
        let input = "日本語 Ω α";
        let escaped = escape_json_string(input);
        assert_eq!(escaped, input);
    }

    // -------------------------------------------------------------------------
    // extract_json_string — unterminated string
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_string_unterminated_returns_none() {
        let json = r#"{"key": "no closing quote}"#;
        assert_eq!(extract_json_string(json, "key"), None);
    }

    #[test]
    fn extract_json_string_empty_value_returns_empty_string() {
        let json = r#"{"key": ""}"#;
        assert_eq!(extract_json_string(json, "key"), Some(String::new()));
    }

    // -------------------------------------------------------------------------
    // find_json_value — edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn find_json_value_key_not_present() {
        assert_eq!(find_json_value("{}", "missing"), None);
    }

    #[test]
    fn find_json_value_empty_string_input() {
        assert_eq!(find_json_value("", "key"), None);
    }

    // -------------------------------------------------------------------------
    // extract_json_number — edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_number_negative_value_returns_none() {
        // Negative numbers have a '-' which is not a digit → parse returns None
        let json = r#"{"fps": -5}"#;
        assert_eq!(extract_json_number(json, "fps"), None);
    }

    #[test]
    fn extract_json_number_float_returns_none_in_compact_json() {
        let json = r#"{"val": 3.14}"#;
        assert_eq!(extract_json_number(json, "val"), None);
    }

    // -------------------------------------------------------------------------
    // extract_json_bool — boundary parsing
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_bool_true_at_end_of_string() {
        // No closing brace — "true" is at the very end of the string.
        // has_json_value_boundary("") → None.is_none_or(...) → true → Some(true)
        let json = r#"{"flag": true"#;
        assert_eq!(extract_json_bool(json, "flag"), Some(true));
    }

    #[test]
    fn extract_json_bool_truex_is_not_true() {
        // "truex" starts with "true" but the boundary check rejects 'x'
        let json = r#"{"flag": truex}"#;
        assert_eq!(extract_json_bool(json, "flag"), None);
    }

    #[test]
    fn extract_json_bool_false_followed_by_comma() {
        let json = r#"{"a": false, "b": 1}"#;
        assert_eq!(extract_json_bool(json, "a"), Some(false));
    }

    #[test]
    fn extract_json_bool_false_followed_by_closing_brace() {
        let json = r#"{"a": false}"#;
        assert_eq!(extract_json_bool(json, "a"), Some(false));
    }

    // -------------------------------------------------------------------------
    // parse_settings_json — extra/unknown keys ignored
    // -------------------------------------------------------------------------

    #[test]
    fn parse_settings_json_ignores_unknown_keys() {
        let json = r#"{"unknown_key": "value", "default_format": "bmp"}"#;
        let settings = parse_settings_json(json);
        assert_eq!(settings.default_format, "bmp");
    }

    #[test]
    fn parse_settings_json_empty_object_returns_defaults() {
        let settings = parse_settings_json("{}");
        let defaults = Settings::default();
        assert_eq!(settings, defaults);
    }

    // -------------------------------------------------------------------------
    // format/parse roundtrip with Windows backslash paths
    // -------------------------------------------------------------------------

    #[test]
    fn format_parse_roundtrip_preserves_backslash_paths() {
        let original = Settings {
            screenshots_dir: PathBuf::from(r"C:\Users\test\Pictures\RustShot"),
            recordings_dir: PathBuf::from(r"C:\Users\test\Videos\recordings"),
            ..Settings::default()
        };
        let json = format_settings_json(&original);
        let parsed = parse_settings_json(&json);
        assert_eq!(parsed.screenshots_dir, original.screenshots_dir);
        assert_eq!(parsed.recordings_dir, original.recordings_dir);
    }

    #[test]
    fn format_settings_json_backslash_is_escaped() {
        let s = Settings {
            screenshots_dir: PathBuf::from(r"C:\path"),
            ..Settings::default()
        };
        let json = format_settings_json(&s);
        // Backslash should be escaped as \\
        assert!(
            json.contains(r"C:\\path"),
            "backslash should be escaped: {json}"
        );
    }

    // -------------------------------------------------------------------------
    // parse_settings_json — malformed JSON
    // -------------------------------------------------------------------------

    #[test]
    fn parse_settings_json_completely_invalid_returns_defaults() {
        let settings = parse_settings_json("not json at all");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn parse_settings_json_null_returns_defaults() {
        let settings = parse_settings_json("null");
        assert_eq!(settings, Settings::default());
    }

    // -------------------------------------------------------------------------
    // extract_json_string — escaped quotes in value
    // -------------------------------------------------------------------------

    #[test]
    fn extract_json_string_with_escaped_quotes() {
        let json = r#"{"path": "C:\\Users\\\"test\""}"#;
        let val = extract_json_string(json, "path").unwrap();
        assert_eq!(val, r#"C:\Users\"test""#);
    }

    // -------------------------------------------------------------------------
    // normalize_default_format — all supported formats
    // -------------------------------------------------------------------------

    #[test]
    fn normalize_default_format_all_supported() {
        assert_eq!(normalize_default_format("png"), "png");
        assert_eq!(normalize_default_format("jpg"), "jpg");
        assert_eq!(normalize_default_format("bmp"), "bmp");
    }

    // -------------------------------------------------------------------------
    // Requirement 12.7: format_settings_json each key appears exactly once
    // -------------------------------------------------------------------------

    #[test]
    fn format_settings_json_each_key_appears_exactly_once() {
        let json = format_settings_json(&Settings::default());
        let keys = [
            "\"screenshots_dir\"",
            "\"recordings_dir\"",
            "\"default_format\"",
            "\"recording_fps\"",
            "\"auto_start\"",
        ];
        for key in keys {
            assert_eq!(
                json.matches(key).count(),
                1,
                "key {key} should appear exactly once in: {json}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Property-based test: Settings JSON roundtrip
    // -------------------------------------------------------------------------

    use proptest::prelude::*;

    proptest! {
        /// **Validates: Requirements 17.5, 12.1**
        ///
        /// For all valid Settings with supported format strings and positive FPS,
        /// `parse_settings_json(format_settings_json(s))` produces equivalent Settings.
        #[test]
        fn prop_settings_json_roundtrip(
            screenshots_dir in "[a-zA-Z0-9/_]{1,30}",
            recordings_dir in "[a-zA-Z0-9/_]{1,30}",
            default_format in prop_oneof!["png", "jpg", "bmp"],
            recording_fps in 1u32..=1000,
            auto_start in any::<bool>(),
        ) {
            let original = Settings {
                screenshots_dir: PathBuf::from(&screenshots_dir),
                recordings_dir: PathBuf::from(&recordings_dir),
                default_format: default_format.to_string(),
                recording_fps,
                auto_start,
            };

            let json = format_settings_json(&original);
            let parsed = parse_settings_json(&json);

            prop_assert_eq!(
                &parsed.screenshots_dir, &original.screenshots_dir,
                "screenshots_dir mismatch after roundtrip"
            );
            prop_assert_eq!(
                &parsed.recordings_dir, &original.recordings_dir,
                "recordings_dir mismatch after roundtrip"
            );
            prop_assert_eq!(
                &parsed.default_format, &original.default_format,
                "default_format mismatch after roundtrip"
            );
            prop_assert_eq!(
                parsed.recording_fps, original.recording_fps,
                "recording_fps mismatch after roundtrip"
            );
            prop_assert_eq!(
                parsed.auto_start, original.auto_start,
                "auto_start mismatch after roundtrip"
            );
        }
    }

    #[test]
    fn launch_editor_command_returns_error_for_nonexistent_program() {
        OPEN_FILE_EDITOR_OVERRIDE.with(|cell| cell.set(None));
        let res = launch_editor_command(
            "__rustshot_nonexistent_editor_binary__",
            std::path::Path::new("settings.json"),
        );
        assert!(res.is_err());
    }

    #[test]
    fn dir_or_current_and_save_settings_write_error_branches() {
        assert_eq!(dir_or_current(None), PathBuf::from("."));
        assert_eq!(
            dir_or_current(Some(PathBuf::from("custom_dir"))),
            PathBuf::from("custom_dir")
        );
        let spaced = parse_settings_json(r#"{ "screenshots_dir" , "default_format" : "bmp" }"#);
        assert_eq!(spaced.default_format, "bmp");
        assert_eq!(find_json_value("{\"unterminated", "k"), None);

        let temp_dir = std::env::temp_dir().join(format!(
            "rustshot_settings_write_err_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Target path is an existing directory -> parent creation succeeds, fs::write fails
        set_settings_path_for_tests(Some(temp_dir.clone()));
        let err = save_settings(&Settings::default()).unwrap_err();
        assert!(err.starts_with("Failed to write settings:"));

        // 2. Target path has no parent (`PathBuf::from("")`) -> skips parent creation, fs::write fails
        set_settings_path_for_tests(Some(PathBuf::new()));
        let err2 = save_settings(&Settings::default()).unwrap_err();
        assert!(err2.starts_with("Failed to write settings:"));

        set_settings_path_for_tests(None);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn json_escapes_and_duplicate_or_value_keys_are_handled_correctly() {
        let escaped = escape_json_string("a\u{0008}b\u{000C}c");
        assert_eq!(escaped, "a\\bb\\fc");
        assert_eq!(
            unescape_json_string("a\\/b\\bc\\fd"),
            "a/b\u{0008}c\u{000C}d"
        );

        let default_shots = Settings::default().screenshots_dir;
        let default_recs = Settings::default().recordings_dir;
        let parsed_empty_dirs = parse_settings_json(
            r#"{ "screenshots_dir": "", "screenshots_dir": "/override", "recordings_dir": "", "recordings_dir": "/override_rec" }"#,
        );
        assert_eq!(parsed_empty_dirs.screenshots_dir, default_shots);
        assert_eq!(parsed_empty_dirs.recordings_dir, default_recs);

        // String values that look like keys must be skipped rather than treated as keys
        let malformed = r#"{ "other": "default_format": "bmp", "recording_fps": "default_format": "bmp", "auto_start": "default_format": "bmp", "default_format": "jpg", "default_format": "bmp" }"#;
        assert_eq!(parse_settings_json(malformed).default_format, "jpg");

        // Unterminated value string after unknown key
        let unterminated_val = r#"{ "other": "unterminated"#;
        assert_eq!(parse_settings_json(unterminated_val).default_format, "png");
        assert_eq!(find_json_value(unterminated_val, "default_format"), None);
    }
}
