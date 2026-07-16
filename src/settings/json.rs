//! Hand-rolled JSON formatting and parsing for the settings file (no serde).

use super::*;
use std::path::Path;

pub(super) fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                use std::fmt::Write;
                let _ = write!(escaped, "\\u{:04x}", ch as u32);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub(super) fn unescape_json_string(value: &str) -> String {
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
            Some('n') => unescaped.push('\n'),
            Some('r') => unescaped.push('\r'),
            Some('t') => unescaped.push('\t'),
            Some('u') => {
                let digits: String = chars.by_ref().take(4).collect();
                match u32::from_str_radix(&digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                {
                    Some(decoded) if digits.len() == 4 => unescaped.push(decoded),
                    _ => {
                        unescaped.push_str("\\u");
                        unescaped.push_str(&digits);
                    }
                }
            }
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
    let screenshots = escape_json_string(&path_to_settings_string(&settings.screenshots_dir));
    let recordings = escape_json_string(&path_to_settings_string(&settings.recordings_dir));
    let mut buf = String::with_capacity(256);
    buf.push_str("{\n  \"screenshots_dir\": \"");
    buf.push_str(&screenshots);
    buf.push_str("\",\n  \"recordings_dir\": \"");
    buf.push_str(&recordings);
    buf.push_str("\",\n  \"default_format\": \"");
    // The enum's extension IS the on-disk representation, so no escaping is
    // needed: it can only ever be one of png/jpg/bmp.
    buf.push_str(settings.default_format.extension());
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
/// 3. Overwrite only the fields that were parsed successfully, converting into
///    the field's type as we go ([`Format::from_settings_str`],
///    [`Fps::from_settings`]) so an invalid value can never land in [`Settings`].
/// 4. Return the partially populated settings object.
///
/// There is deliberately no separate "normalize afterwards" pass: the types
/// enforce their own invariants at the point of parsing.
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
    let mut settings = Settings::default();

    // Manual field-by-field parse:
    // try each known key independently, keep defaults for anything missing or
    // malformed, then normalize the resulting values once at the end.
    if let Some(val) = extract_json_string(json, "screenshots_dir") {
        settings.screenshots_dir = path_from_settings_string(val);
    }
    if let Some(val) = extract_json_string(json, "recordings_dir") {
        settings.recordings_dir = path_from_settings_string(val);
    }
    if let Some(val) = extract_json_string(json, "default_format") {
        settings.default_format = Format::from_settings_str(&val);
    }
    if let Some(val) = extract_json_number(json, "recording_fps") {
        settings.recording_fps = Fps::from_settings(val);
    }
    if let Some(val) = extract_json_bool(json, "auto_start") {
        settings.auto_start = val;
    }

    settings
}

#[cfg(unix)]
const RAW_PATH_PREFIX: &str = "\u{1e}rustshot-path-hex:";

fn path_to_settings_string(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        if path.as_os_str().to_str().is_none() {
            let mut encoded = String::from(RAW_PATH_PREFIX);
            for byte in path.as_os_str().as_bytes() {
                use std::fmt::Write;
                let _ = write!(encoded, "{byte:02x}");
            }
            return encoded;
        }
    }
    path.to_string_lossy().into_owned()
}

fn path_from_settings_string(value: String) -> PathBuf {
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        if let Some(hex) = value.strip_prefix(RAW_PATH_PREFIX) {
            if hex.len() % 2 == 0 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
                    .collect();
                return OsString::from_vec(bytes).into();
            }
        }
    }
    PathBuf::from(value)
}

// The former `normalize_default_format` / `normalize_recording_fps` helpers are
// gone: their jobs are now the constructors `Format::from_settings_str` and
// `Fps::from_settings`, so an invalid value cannot survive parsing in the first
// place and there is nothing left to normalize after the fact.

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_normalize_default_format(value: &str) -> Format {
    Format::from_settings_str(value)
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_normalize_recording_fps(value: u32) -> Fps {
    Fps::from_settings(value)
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_extract_json_string(json: &str, key: &str) -> Option<String> {
    extract_json_string(json, key)
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
pub(super) fn find_json_value<'a>(json: &'a str, key: &str) -> Option<&'a str> {
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
                index += 1;
                let mut escaped = false;

                while index < bytes.len() {
                    match bytes[index] {
                        b'\\' if !escaped => escaped = true,
                        b'"' if !escaped => break,
                        _ => escaped = false,
                    }
                    index += 1;
                }

                if index >= bytes.len() {
                    return None;
                }

                let string_end = index;
                index += 1;

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

pub(super) fn has_json_value_boundary(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_none_or(|ch| ch.is_whitespace() || ch == ',' || ch == '}' || ch == ']')
}

/// Extract a JSON string value for `key` using a minimal escape-aware scan.
///
/// Algorithm:
/// 1. Locate the candidate value text with [`find_json_value`].
/// 2. Ensure the value starts with a double quote.
/// 3. Scan forward until an unescaped closing quote is found.
/// 4. Unescape the captured slice and return it.
///
/// This parser is intentionally narrow: it understands the subset of JSON that
/// RustShot writes itself and treats anything malformed as "missing".
///
/// Possible improvements:
/// - Support the full JSON escape set, including unicode escape sequences.
/// - Replace the ad-hoc parser with `serde_json` if dependency policy changes.
pub(super) fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let after_colon = find_json_value(json, key)?;
    if !after_colon.starts_with('"') {
        return None;
    }
    let content = &after_colon[1..];
    let mut escaped = false;

    for (idx, ch) in content.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Some(unescape_json_string(&content[..idx])),
            _ => {}
        }
    }

    None
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_extract_json_number(json: &str, key: &str) -> Option<u32> {
    extract_json_number(json, key)
}

/// Extract a positive integer value by consuming consecutive ASCII digits.
///
/// Algorithm:
/// 1. Locate the candidate value text with [`find_json_value`].
/// 2. Read digits until the first non-digit.
/// 3. Require a valid JSON value boundary after the digit sequence.
/// 4. Parse the digit slice as `u32`.
/// 5. Return `None` on empty, invalid, overflowing, or junk-suffixed input.
///
/// Possible improvements:
/// - Support signed or wider numeric types if the settings schema expands.
pub(super) fn extract_json_number(json: &str, key: &str) -> Option<u32> {
    let after_colon = find_json_value(json, key)?;
    let end = after_colon
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_colon.len());
    if end == 0 || !has_json_value_boundary(&after_colon[end..]) {
        return None;
    }
    after_colon[..end].parse().ok()
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_extract_json_bool(json: &str, key: &str) -> Option<bool> {
    extract_json_bool(json, key)
}

/// Extract a JSON boolean while enforcing a token boundary after the literal.
///
/// Algorithm:
/// 1. Locate the candidate value text with [`find_json_value`].
/// 2. Check for the `true` literal followed by a valid JSON boundary.
/// 3. Otherwise check for the `false` literal followed by a valid boundary.
/// 4. Reject lookalikes such as `trueish`, `falsehood`, or numeric values.
///
/// Possible improvements:
/// - Share tokenization with the string/number extractors so all scalar parsing
///   follows one consistent scanning path.
/// - Return a richer parse error in debug tooling to make malformed user edits
///   easier to diagnose.
pub(super) fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let after_colon = find_json_value(json, key)?;

    if after_colon.starts_with("true") && has_json_value_boundary(&after_colon["true".len()..]) {
        Some(true)
    } else if after_colon.starts_with("false")
        && has_json_value_boundary(&after_colon["false".len()..])
    {
        Some(false)
    } else {
        None
    }
}
