//! Tests for `extract_json_string` and `find_json_value`.

use super::*;

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

#[test]
fn find_json_value_key_at_end_no_value() {
    assert!(find_json_value(r#"{"key"}"#, "key").is_none());
}

// -------------------------------------------------------------------------
// find_json_value — edge cases
// -------------------------------------------------------------------------

// -------------------------------------------------------------------------
// extract_json_string — escaped quotes in value
// -------------------------------------------------------------------------

#[test]
fn extract_json_string_with_escaped_quotes() {
    let json = r#"{"path": "C:\\Users\\\"test\""}"#;
    let val = extract_json_string(json, "path").unwrap();
    assert_eq!(val, r#"C:\Users\"test""#);
}
