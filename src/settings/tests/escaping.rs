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
