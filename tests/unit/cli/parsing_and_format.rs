//! Tests for argument parsing and format-selection helpers.

use crate::capture;
use crate::cli::*;
use std::path::Path;

#[test]
fn child_process_is_running_covers_every_try_wait_outcome() {
    assert!(child_process_is_running(Ok::<Option<i32>, &str>(None)));
    assert!(!child_process_is_running(Ok::<Option<i32>, &str>(Some(0))));
    assert!(!child_process_is_running(Ok::<Option<i32>, &str>(Some(1))));
    assert!(!child_process_is_running(Err::<Option<i32>, &str>("error")));
}

#[test]
fn settings_format_parser_covers_supported_and_fallback_values() {
    for (value, expected) in [
        ("png", capture::Format::Png),
        ("jpg", capture::Format::Jpg),
        ("bmp", capture::Format::Bmp),
        ("webp", capture::Format::Png),
        ("", capture::Format::Png),
        ("JPG", capture::Format::Png),
    ] {
        assert_eq!(capture::Format::from_settings_str(value), expected);
    }
}

#[test]
fn format_flag_detection_covers_forms_skipped_values_and_absence() {
    let cases = [
        (vec!["rustshot", "--format", "png"], true),
        (vec!["rustshot", "-f", "jpg"], true),
        (vec!["rustshot", "--format=png"], true),
        (vec!["rustshot", "-fpng"], true),
        (vec!["rustshot", "-s", "0", "--format", "png"], true),
        (
            vec![
                "rustshot", "--cli", "--output", "out.png", "--format", "jpg",
            ],
            true,
        ),
        (vec!["rustshot", "--cli"], false),
        (vec!["rustshot"], false),
        (vec!["rustshot", "--output", "--format"], false),
        (vec!["rustshot", "-o", "-f"], false),
        (vec!["rustshot", "--screen", "--format"], false),
        (vec!["rustshot", "-s", "-f"], false),
        (vec!["--format", "--cli"], false),
    ];

    for (args, expected) in cases {
        assert_eq!(cli_format_flag_present_in(args), expected);
    }
}

#[test]
fn effective_format_applies_explicit_output_and_settings_precedence() {
    for (cli, explicit, output, settings, expected) in [
        (
            capture::Format::Jpg,
            true,
            Some(Path::new("out.png")),
            capture::Format::Bmp,
            capture::Format::Jpg,
        ),
        (
            capture::Format::Png,
            false,
            Some(Path::new("shot.bmp")),
            capture::Format::Jpg,
            capture::Format::Bmp,
        ),
        (
            capture::Format::Png,
            false,
            None,
            capture::Format::Jpg,
            capture::Format::Jpg,
        ),
        (
            capture::Format::Png,
            false,
            Some(Path::new("out.tiff")),
            capture::Format::Bmp,
            capture::Format::Bmp,
        ),
        (
            capture::Format::Png,
            false,
            Some(Path::new("screenshot")),
            capture::Format::Jpg,
            capture::Format::Jpg,
        ),
    ] {
        assert_eq!(
            effective_cli_format(cli, explicit, output, settings),
            expected
        );
    }
}

#[test]
fn output_extension_covers_nested_empty_and_missing_cases() {
    for (path, expected) in [
        (Some(Path::new("file.png")), Some("png")),
        (Some(Path::new("/a/b/c.jpg")), Some("jpg")),
        (Some(Path::new("file.")), Some("")),
        (Some(Path::new("noext")), None),
        (None, None),
    ] {
        assert_eq!(output_extension(path), expected);
    }
}

#[test]
fn format_from_extension_covers_supported_case_variants_and_unknowns() {
    for (extension, expected) in [
        ("png", Some(capture::Format::Png)),
        ("PNG", Some(capture::Format::Png)),
        ("jpg", Some(capture::Format::Jpg)),
        ("Jpg", Some(capture::Format::Jpg)),
        ("jpeg", Some(capture::Format::Jpg)),
        ("JPEG", Some(capture::Format::Jpg)),
        ("bmp", Some(capture::Format::Bmp)),
        ("BMP", Some(capture::Format::Bmp)),
        ("tiff", None),
        ("webp", None),
        ("", None),
    ] {
        assert_eq!(format_from_extension(extension), expected);
    }
}

#[test]
fn format_from_output_extension_covers_valid_missing_and_unsupported_paths() {
    for (path, expected) in [
        (Some(Path::new("a.png")), Some(capture::Format::Png)),
        (Some(Path::new("a.gif")), None),
        (None, None),
    ] {
        assert_eq!(format_from_output_extension(path), expected);
    }
}

#[test]
fn unsupported_output_extension_covers_known_unknown_and_missing_paths() {
    for (path, expected) in [
        (Some(Path::new("f.tiff")), Some("tiff".to_string())),
        (Some(Path::new("f.png")), None),
        (Some(Path::new("f.jpg")), None),
        (Some(Path::new("f.bmp")), None),
        (Some(Path::new("noext")), None),
        (None, None),
    ] {
        assert_eq!(unsupported_output_extension(path), expected);
    }
}
