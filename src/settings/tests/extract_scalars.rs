//! Tests for `extract_json_number` and `extract_json_bool`.

use super::*;

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
fn extract_json_bool_falsehood_returns_none() {
    let json = r#"{ "flag": falsehood }"#;
    assert_eq!(extract_json_bool(json, "flag"), None);
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

// -------------------------------------------------------------------------
// extract_json_bool edge cases
// -------------------------------------------------------------------------

#[test]
fn extract_json_bool_string_true_returns_none() {
    let json = r#"{ "key": "true" }"#;
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
// extract_json_number — edge cases
// -------------------------------------------------------------------------

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
