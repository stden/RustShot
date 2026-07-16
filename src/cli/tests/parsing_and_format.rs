//! Tests for argument parsing and format-selection helpers.

#[allow(unused_imports)]
use super::*;
use crate::capture;
use crate::cli::*;
use std::path::Path;

// =========================================================================
// child_process_is_running
// =========================================================================

#[test]
fn child_process_running_when_ok_none() {
    assert!(child_process_is_running(Ok::<Option<i32>, &str>(None)));
}

#[test]
fn child_process_not_running_when_ok_some() {
    assert!(!child_process_is_running(Ok::<Option<i32>, &str>(Some(0))));
}

#[test]
fn child_process_not_running_when_err() {
    assert!(!child_process_is_running(Err::<Option<i32>, &str>("error")));
}

#[test]
fn child_process_not_running_when_ok_some_nonzero_exit() {
    assert!(!child_process_is_running(Ok::<Option<i32>, &str>(Some(1))));
}

// =========================================================================
// Format::from_settings_str
//
// These cases used to test `settings_capture_format(&str)`. That function is
// gone; parsing a settings format string is now the enum's own constructor.
// The behaviour it must preserve is unchanged.
// =========================================================================

#[test]
fn settings_format_jpg() {
    assert_eq!(
        capture::Format::from_settings_str("jpg"),
        capture::Format::Jpg
    );
}

#[test]
fn settings_format_bmp() {
    assert_eq!(
        capture::Format::from_settings_str("bmp"),
        capture::Format::Bmp
    );
}

#[test]
fn settings_format_png() {
    assert_eq!(
        capture::Format::from_settings_str("png"),
        capture::Format::Png
    );
}

#[test]
fn settings_format_unknown_defaults_to_png() {
    assert_eq!(
        capture::Format::from_settings_str("webp"),
        capture::Format::Png
    );
}

#[test]
fn settings_format_empty_defaults_to_png() {
    assert_eq!(capture::Format::from_settings_str(""), capture::Format::Png);
}

#[test]
fn settings_format_case_sensitive_jpg() {
    // "JPG" is not "jpg", so falls back to Png.
    assert_eq!(
        capture::Format::from_settings_str("JPG"),
        capture::Format::Png
    );
}

// =========================================================================
// cli_format_flag_present_in
// =========================================================================

#[test]
fn format_flag_long_form() {
    let args = vec!["rustshot", "--format", "png"];
    assert!(cli_format_flag_present_in(args));
}

#[test]
fn format_flag_short_form() {
    let args = vec!["rustshot", "-f", "jpg"];
    assert!(cli_format_flag_present_in(args));
}

#[test]
fn format_flag_equals_form() {
    let args = vec!["rustshot", "--format=png"];
    assert!(cli_format_flag_present_in(args));
}

#[test]
fn format_flag_attached_short_form() {
    let args = vec!["rustshot", "-fpng"];
    assert!(cli_format_flag_present_in(args));
}

#[test]
fn format_flag_not_present() {
    let args = vec!["rustshot", "--cli"];
    assert!(!cli_format_flag_present_in(args));
}

#[test]
fn format_flag_empty_args() {
    let args: Vec<&str> = vec!["rustshot"];
    assert!(!cli_format_flag_present_in(args));
}

#[test]
fn format_flag_skips_output_value() {
    // --output takes a value; the value should not be confused with --format
    let args = vec!["rustshot", "--output", "--format"];
    assert!(!cli_format_flag_present_in(args));
}

#[test]
fn format_flag_skips_short_output_value() {
    let args = vec!["rustshot", "-o", "-f"];
    assert!(!cli_format_flag_present_in(args));
}

#[test]
fn format_flag_skips_screen_value() {
    let args = vec!["rustshot", "--screen", "--format"];
    assert!(!cli_format_flag_present_in(args));
}

#[test]
fn format_flag_skips_short_screen_value() {
    let args = vec!["rustshot", "-s", "-f"];
    assert!(!cli_format_flag_present_in(args));
}

#[test]
fn format_flag_after_screen_value_detected() {
    // -s takes a numeric value; --format png should still be detected after it
    let args = vec!["rustshot", "-s", "0", "--format", "png"];
    assert!(cli_format_flag_present_in(args));
}

#[test]
fn format_flag_after_other_flags() {
    let args = vec![
        "rustshot", "--cli", "--output", "out.png", "--format", "jpg",
    ];
    assert!(cli_format_flag_present_in(args));
}

#[test]
fn format_flag_argv0_ignored() {
    // argv[0] is skipped, so even if it matches it shouldn't count
    let args = vec!["--format", "--cli"];
    assert!(!cli_format_flag_present_in(args));
}

// =========================================================================
// effective_cli_format
// =========================================================================

#[test]
fn effective_format_explicit_wins() {
    let result = effective_cli_format(
        capture::Format::Jpg,
        true,
        Some(Path::new("out.png")),
        capture::Format::Bmp,
    );
    assert!(matches!(result, capture::Format::Jpg));
}

#[test]
fn effective_format_output_extension_wins_when_not_explicit() {
    let result = effective_cli_format(
        capture::Format::Png,
        false,
        Some(Path::new("shot.bmp")),
        capture::Format::Jpg,
    );
    assert!(matches!(result, capture::Format::Bmp));
}

#[test]
fn effective_format_settings_fallback() {
    let result = effective_cli_format(capture::Format::Png, false, None, capture::Format::Jpg);
    assert!(matches!(result, capture::Format::Jpg));
}

#[test]
fn effective_format_unknown_extension_falls_to_settings() {
    let result = effective_cli_format(
        capture::Format::Png,
        false,
        Some(Path::new("out.tiff")),
        capture::Format::Bmp,
    );
    assert!(matches!(result, capture::Format::Bmp));
}

#[test]
fn effective_format_no_extension_falls_to_settings() {
    let result = effective_cli_format(
        capture::Format::Png,
        false,
        Some(Path::new("screenshot")),
        capture::Format::Jpg,
    );
    assert!(matches!(result, capture::Format::Jpg));
}

// =========================================================================
// output_extension
// =========================================================================

#[test]
fn output_extension_png() {
    assert_eq!(output_extension(Some(Path::new("file.png"))), Some("png"));
}

#[test]
fn output_extension_none_path() {
    assert_eq!(output_extension(None), None);
}

#[test]
fn output_extension_no_ext() {
    assert_eq!(output_extension(Some(Path::new("noext"))), None);
}

#[test]
fn output_extension_dot_only() {
    assert_eq!(output_extension(Some(Path::new("file."))), Some(""));
}

#[test]
fn output_extension_nested_path() {
    assert_eq!(output_extension(Some(Path::new("/a/b/c.jpg"))), Some("jpg"));
}

// =========================================================================
// format_from_extension
// =========================================================================

#[test]
fn format_from_ext_png() {
    assert!(matches!(
        format_from_extension("png"),
        Some(capture::Format::Png)
    ));
}

#[test]
fn format_from_ext_jpg() {
    assert!(matches!(
        format_from_extension("jpg"),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_ext_jpeg() {
    assert!(matches!(
        format_from_extension("jpeg"),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_ext_bmp() {
    assert!(matches!(
        format_from_extension("bmp"),
        Some(capture::Format::Bmp)
    ));
}

#[test]
fn format_from_ext_case_insensitive() {
    assert!(matches!(
        format_from_extension("PNG"),
        Some(capture::Format::Png)
    ));
    assert!(matches!(
        format_from_extension("Jpg"),
        Some(capture::Format::Jpg)
    ));
    assert!(matches!(
        format_from_extension("BMP"),
        Some(capture::Format::Bmp)
    ));
}

#[test]
fn format_from_ext_jpeg_case_insensitive() {
    assert!(matches!(
        format_from_extension("JPEG"),
        Some(capture::Format::Jpg)
    ));
    assert!(matches!(
        format_from_extension("Jpeg"),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_ext_unknown() {
    assert!(format_from_extension("tiff").is_none());
    assert!(format_from_extension("webp").is_none());
    assert!(format_from_extension("").is_none());
}

// =========================================================================
// format_from_output_extension
// =========================================================================

#[test]
fn format_from_output_ext_valid() {
    assert!(matches!(
        format_from_output_extension(Some(Path::new("a.png"))),
        Some(capture::Format::Png)
    ));
}

#[test]
fn format_from_output_ext_none_path() {
    assert!(format_from_output_extension(None).is_none());
}

#[test]
fn format_from_output_ext_unsupported() {
    assert!(format_from_output_extension(Some(Path::new("a.gif"))).is_none());
}

// =========================================================================
// unsupported_output_extension
// =========================================================================

#[test]
fn unsupported_ext_returns_extension_for_unknown() {
    assert_eq!(
        unsupported_output_extension(Some(Path::new("f.tiff"))),
        Some("tiff".to_string())
    );
}

#[test]
fn unsupported_ext_returns_none_for_known() {
    assert!(unsupported_output_extension(Some(Path::new("f.png"))).is_none());
    assert!(unsupported_output_extension(Some(Path::new("f.jpg"))).is_none());
    assert!(unsupported_output_extension(Some(Path::new("f.bmp"))).is_none());
}

#[test]
fn unsupported_ext_returns_none_when_no_path() {
    assert!(unsupported_output_extension(None).is_none());
}

#[test]
fn unsupported_ext_returns_none_when_no_extension() {
    assert!(unsupported_output_extension(Some(Path::new("noext"))).is_none());
}
