//! Format-resolution policy tests: precedence, extension mapping, and settings fallback.

use super::*;

#[test]
fn effective_cli_format_keeps_settings_format_for_hyphenated_output_values() {
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let format_explicit = cli_format_flag_present_in(["rustshot", "--output", "-foo.png"]);

    assert_eq!(
        effective_cli_format(CliFormat::Png, format_explicit, None, &settings),
        capture::Format::Jpg
    );
}

#[test]
fn settings_capture_format_maps_every_supported_format() {
    for (configured, expected) in [
        (Format::Png, capture::Format::Png),
        (Format::Jpg, capture::Format::Jpg),
        (Format::Bmp, capture::Format::Bmp),
    ] {
        let settings = settings::Settings {
            default_format: configured,
            ..settings::Settings::default()
        };
        assert_eq!(settings_capture_format(&settings), expected);
    }
}

#[test]
fn format_from_extension_covers_supported_case_variants_and_unknowns() {
    for (extension, expected) in [
        ("png", Some(capture::Format::Png)),
        ("PNG", Some(capture::Format::Png)),
        ("Png", Some(capture::Format::Png)),
        ("jpg", Some(capture::Format::Jpg)),
        ("jpeg", Some(capture::Format::Jpg)),
        ("JPEG", Some(capture::Format::Jpg)),
        ("bmp", Some(capture::Format::Bmp)),
        ("BMP", Some(capture::Format::Bmp)),
        ("gif", None),
        ("webp", None),
        ("tiff", None),
        ("", None),
    ] {
        assert_eq!(format_from_extension(extension), expected);
    }
}

#[test]
fn output_extension_covers_paths_and_missing_extensions() {
    for (path, expected) in [
        (None, None),
        (Some(Path::new("/tmp/screenshot")), None),
        (Some(Path::new(".hidden")), None),
        (Some(Path::new("/tmp/shot.png")), Some("png")),
        (Some(Path::new("file.jpg")), Some("jpg")),
    ] {
        assert_eq!(output_extension(path), expected);
    }
}

#[test]
fn format_from_output_extension_covers_supported_and_missing_formats() {
    for (path, expected) in [
        (None, None),
        (Some(Path::new("shot.png")), Some(capture::Format::Png)),
        (Some(Path::new("shot.JPG")), Some(capture::Format::Jpg)),
        (Some(Path::new("shot.JPEG")), Some(capture::Format::Jpg)),
        (Some(Path::new("/tmp/file")), None),
        (Some(Path::new("file.gif")), None),
    ] {
        assert_eq!(format_from_output_extension(path), expected);
    }
}

#[test]
fn unsupported_output_extension_covers_known_unknown_and_missing_formats() {
    for (path, expected) in [
        (None, None),
        (Some(Path::new("shot.png")), None),
        (Some(Path::new("shot.jpg")), None),
        (Some(Path::new("shot.bmp")), None),
        (Some(Path::new("/tmp/file")), None),
        (Some(Path::new("shot.gif")), Some("gif".to_string())),
        (Some(Path::new("shot.webp")), Some("webp".to_string())),
    ] {
        assert_eq!(unsupported_output_extension(path), expected);
    }
}

#[test]
fn effective_cli_format_applies_explicit_output_and_settings_precedence() {
    let png_settings = settings::Settings::default();
    let jpg_settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let bmp_settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };

    for (cli_format, explicit, output, settings, expected) in [
        (
            CliFormat::Jpg,
            true,
            None,
            &bmp_settings,
            capture::Format::Jpg,
        ),
        (
            CliFormat::Png,
            true,
            None,
            &jpg_settings,
            capture::Format::Png,
        ),
        (
            CliFormat::Bmp,
            true,
            Some(Path::new("screenshot.jpg")),
            &png_settings,
            capture::Format::Bmp,
        ),
        (
            CliFormat::Png,
            false,
            Some(Path::new("screenshot.jpg")),
            &png_settings,
            capture::Format::Jpg,
        ),
        (
            CliFormat::Png,
            false,
            Some(Path::new("output.bmp")),
            &png_settings,
            capture::Format::Bmp,
        ),
        (
            CliFormat::Png,
            false,
            Some(Path::new("file.webp")),
            &jpg_settings,
            capture::Format::Jpg,
        ),
        (
            CliFormat::Jpg,
            false,
            Some(Path::new("screenshot")),
            &bmp_settings,
            capture::Format::Bmp,
        ),
        (
            CliFormat::Bmp,
            false,
            None,
            &jpg_settings,
            capture::Format::Jpg,
        ),
    ] {
        assert_eq!(
            effective_cli_format(cli_format, explicit, output, settings),
            expected
        );
    }
}
