//! Tests for `escape_json_string` / `unescape_json_string` and their roundtrips.

use super::*;

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

// -------------------------------------------------------------------------
// escape/unescape roundtrip tests
// -------------------------------------------------------------------------

#[test]
fn escape_unescape_roundtrip_unicode() {
    let original = "日本語パス/用户/截图";
    assert_eq!(
        unescape_json_string(&escape_json_string(original)),
        original
    );
}

// -------------------------------------------------------------------------
// unescape_json_string — edge cases
// -------------------------------------------------------------------------

// -------------------------------------------------------------------------
// escape_json_string — additional characters
// -------------------------------------------------------------------------

#[test]
fn escape_json_string_escapes_nul_and_control_chars() {
    let input = "a\x00b\x01c";
    let escaped = escape_json_string(input);
    assert_eq!(escaped, "a\\u0000b\\u0001c");
    assert_eq!(unescape_json_string(&escaped), input);
}

#[test]
fn escape_json_string_unicode_unchanged() {
    let input = "日本語 Ω α";
    let escaped = escape_json_string(input);
    assert_eq!(escaped, input);
}

// -------------------------------------------------------------------------
// Property-based test: escape/unescape roundtrip over ALL strings
// -------------------------------------------------------------------------

use proptest::prelude::*;

proptest! {
    /// For every possible string — quotes, backslashes, NUL, control
    /// characters, arbitrary unicode — `unescape(escape(s)) == s`.
    ///
    /// This holds universally because `escape_json_string` only ever emits a
    /// backslash as the start of a known two-character escape, and every
    /// other character (including raw NUL/control characters) passes through
    /// both functions unchanged.
    #[test]
    fn prop_escape_unescape_roundtrip(s in any::<String>()) {
        prop_assert_eq!(unescape_json_string(&escape_json_string(&s)), s);
    }
}
