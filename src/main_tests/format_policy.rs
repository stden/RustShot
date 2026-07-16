//! Format-resolution policy tests: `effective_cli_format`, extension mapping, and settings fallback.

use super::*;

#[test]
fn effective_cli_format_keeps_settings_format_for_hyphenated_output_values() {
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let format_explicit = cli_format_flag_present_in(["rustshot", "--output", "-foo.png"]);

    assert!(matches!(
        effective_cli_format(CliFormat::Png, format_explicit, None, &settings),
        capture::Format::Jpg
    ));
}

#[test]
fn settings_capture_format_defaults_to_png_for_unknown_value() {
    let settings = settings::Settings {
        default_format: Format::Png,
        ..settings::Settings::default()
    };

    assert!(matches!(
        settings_capture_format(&settings),
        capture::Format::Png
    ));
}

#[test]
fn settings_capture_format_maps_jpg_value() {
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };

    assert!(matches!(
        settings_capture_format(&settings),
        capture::Format::Jpg
    ));
}

#[test]
fn settings_capture_format_maps_bmp_value() {
    let settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };

    assert!(matches!(
        settings_capture_format(&settings),
        capture::Format::Bmp
    ));
}

#[test]
fn effective_cli_format_uses_settings_when_flag_is_absent() {
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };

    assert!(matches!(
        effective_cli_format(CliFormat::Bmp, false, None, &settings),
        capture::Format::Jpg
    ));
}

#[test]
fn effective_cli_format_prefers_explicit_cli_format_when_flag_is_present() {
    let settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };

    assert!(matches!(
        effective_cli_format(CliFormat::Jpg, true, None, &settings),
        capture::Format::Jpg
    ));
}

// --- effective_cli_format edge cases ---

#[test]
fn effective_cli_format_explicit_png_overrides_jpg_settings() {
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let result = effective_cli_format(CliFormat::Png, true, None, &settings);
    assert!(matches!(result, capture::Format::Png));
}

#[test]
fn effective_cli_format_settings_bmp_used_when_not_explicit() {
    let settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };
    let result = effective_cli_format(CliFormat::Png, false, None, &settings);
    assert!(matches!(result, capture::Format::Bmp));
}

// -------------------------------------------------------------------------
// format_from_extension tests
// -------------------------------------------------------------------------

#[test]
fn format_from_extension_png_lowercase() {
    assert!(matches!(
        format_from_extension("png"),
        Some(capture::Format::Png)
    ));
}

#[test]
fn format_from_extension_png_uppercase() {
    assert!(matches!(
        format_from_extension("PNG"),
        Some(capture::Format::Png)
    ));
}

#[test]
fn format_from_extension_png_mixed_case() {
    assert!(matches!(
        format_from_extension("Png"),
        Some(capture::Format::Png)
    ));
}

#[test]
fn format_from_extension_jpg_lowercase() {
    assert!(matches!(
        format_from_extension("jpg"),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_extension_jpeg_lowercase() {
    assert!(matches!(
        format_from_extension("jpeg"),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_extension_jpeg_uppercase() {
    assert!(matches!(
        format_from_extension("JPEG"),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_extension_bmp_lowercase() {
    assert!(matches!(
        format_from_extension("bmp"),
        Some(capture::Format::Bmp)
    ));
}

#[test]
fn format_from_extension_bmp_uppercase() {
    assert!(matches!(
        format_from_extension("BMP"),
        Some(capture::Format::Bmp)
    ));
}

#[test]
fn format_from_extension_unknown_returns_none() {
    assert!(format_from_extension("gif").is_none());
    assert!(format_from_extension("webp").is_none());
    assert!(format_from_extension("tiff").is_none());
    assert!(format_from_extension("").is_none());
}

// -------------------------------------------------------------------------
// output_extension tests
// -------------------------------------------------------------------------

#[test]
fn output_extension_none_path() {
    assert!(output_extension(None).is_none());
}

#[test]
fn output_extension_no_extension() {
    assert!(output_extension(Some(Path::new("/tmp/screenshot"))).is_none());
}

#[test]
fn output_extension_png() {
    assert_eq!(
        output_extension(Some(Path::new("/tmp/shot.png"))),
        Some("png")
    );
}

#[test]
fn output_extension_jpg() {
    assert_eq!(output_extension(Some(Path::new("file.jpg"))), Some("jpg"));
}

#[test]
fn output_extension_dotfile_no_extension() {
    assert!(output_extension(Some(Path::new(".hidden"))).is_none());
}

// -------------------------------------------------------------------------
// format_from_output_extension tests
// -------------------------------------------------------------------------

#[test]
fn format_from_output_extension_none() {
    assert!(format_from_output_extension(None).is_none());
}

#[test]
fn format_from_output_extension_png_path() {
    assert!(matches!(
        format_from_output_extension(Some(Path::new("shot.png"))),
        Some(capture::Format::Png)
    ));
}

#[test]
fn format_from_output_extension_jpg_path() {
    assert!(matches!(
        format_from_output_extension(Some(Path::new("shot.JPG"))),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_output_extension_jpeg_path() {
    assert!(matches!(
        format_from_output_extension(Some(Path::new("shot.JPEG"))),
        Some(capture::Format::Jpg)
    ));
}

#[test]
fn format_from_output_extension_no_extension() {
    assert!(format_from_output_extension(Some(Path::new("/tmp/file"))).is_none());
}

#[test]
fn format_from_output_extension_unknown_ext() {
    assert!(format_from_output_extension(Some(Path::new("file.gif"))).is_none());
}

// -------------------------------------------------------------------------
// unsupported_output_extension tests
// -------------------------------------------------------------------------

#[test]
fn unsupported_output_extension_none_returns_none() {
    assert!(unsupported_output_extension(None).is_none());
}

#[test]
fn unsupported_output_extension_png_returns_none() {
    assert!(unsupported_output_extension(Some(Path::new("shot.png"))).is_none());
}

#[test]
fn unsupported_output_extension_jpg_returns_none() {
    assert!(unsupported_output_extension(Some(Path::new("shot.jpg"))).is_none());
}

#[test]
fn unsupported_output_extension_bmp_returns_none() {
    assert!(unsupported_output_extension(Some(Path::new("shot.bmp"))).is_none());
}

#[test]
fn unsupported_output_extension_gif_returns_extension() {
    assert_eq!(
        unsupported_output_extension(Some(Path::new("shot.gif"))),
        Some("gif".to_string())
    );
}

#[test]
fn unsupported_output_extension_webp_returns_extension() {
    assert_eq!(
        unsupported_output_extension(Some(Path::new("shot.webp"))),
        Some("webp".to_string())
    );
}

#[test]
fn unsupported_output_extension_no_ext_returns_none() {
    assert!(unsupported_output_extension(Some(Path::new("/tmp/file"))).is_none());
}

// -------------------------------------------------------------------------
// settings_capture_format edge cases
// -------------------------------------------------------------------------

#[test]
fn settings_capture_format_png_fallback_for_empty_string() {
    let settings = settings::Settings {
        default_format: Format::Png,
        ..settings::Settings::default()
    };
    assert!(matches!(
        settings_capture_format(&settings),
        capture::Format::Png
    ));
}

#[test]
fn settings_capture_format_png_fallback_for_uppercase() {
    let settings = settings::Settings {
        default_format: Format::Png,
        ..settings::Settings::default()
    };
    assert!(matches!(
        settings_capture_format(&settings),
        capture::Format::Png
    ));
}

// -------------------------------------------------------------------------
// effective_cli_format edge cases
// -------------------------------------------------------------------------

#[test]
fn effective_cli_format_explicit_bmp_overrides_settings_jpg() {
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let result = effective_cli_format(CliFormat::Bmp, true, None, &settings);
    assert!(matches!(result, capture::Format::Bmp));
}

#[test]
fn effective_cli_format_not_explicit_uses_settings_bmp() {
    let settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };
    let result = effective_cli_format(CliFormat::Jpg, false, None, &settings);
    assert!(matches!(result, capture::Format::Bmp));
}

// -------------------------------------------------------------------------
// effective_cli_format — output extension path
// -------------------------------------------------------------------------

#[test]
fn effective_cli_format_output_jpg_overrides_settings_png() {
    let settings = settings::Settings::default(); // default_format = "png"
    let path = Path::new("screenshot.jpg");
    let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
    assert!(matches!(result, capture::Format::Jpg));
}

#[test]
fn effective_cli_format_output_bmp_overrides_settings_png() {
    let settings = settings::Settings::default();
    let path = Path::new("output.bmp");
    let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
    assert!(matches!(result, capture::Format::Bmp));
}

#[test]
fn effective_cli_format_explicit_flag_beats_output_extension() {
    let settings = settings::Settings::default();
    let path = Path::new("screenshot.jpg");
    // Explicit --format bmp should win over output extension .jpg
    let result = effective_cli_format(CliFormat::Bmp, true, Some(path), &settings);
    assert!(matches!(result, capture::Format::Bmp));
}

#[test]
fn effective_cli_format_unknown_output_ext_falls_back_to_settings() {
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let path = Path::new("file.webp"); // unsupported extension
    let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
    // webp is not recognized, so falls back to settings (jpg)
    assert!(matches!(result, capture::Format::Jpg));
}

#[test]
fn effective_cli_format_output_no_ext_falls_back_to_settings() {
    let settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };
    let path = Path::new("screenshot");
    let result = effective_cli_format(CliFormat::Png, false, Some(path), &settings);
    assert!(matches!(result, capture::Format::Bmp));
}
