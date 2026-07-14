//! Pure CLI helper logic shared by the binary, tests, and benchmarks.

use crate::capture;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::Path;

pub fn child_process_is_running<T, E>(try_wait_result: Result<Option<T>, E>) -> bool {
    matches!(try_wait_result, Ok(None))
}

// `settings_capture_format(&str) -> Format` used to live here, mapping the
// settings format string onto the enum. It is gone: `Settings` now stores a
// `Format` directly, so the conversion happens exactly once, in
// `Format::from_settings_str`, at the JSON boundary.

pub fn cli_format_flag_present_in<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut skip_next_value = false;

    for arg in args.into_iter().skip(1) {
        if skip_next_value {
            skip_next_value = false;
            continue;
        }

        let arg = arg.as_ref();
        let arg_str = arg.to_string_lossy();

        if arg == OsStr::new("--format") || arg == OsStr::new("-f") {
            return true;
        }

        if arg_str.starts_with("--format=")
            || (arg_str.starts_with("-f") && !arg_str.starts_with("--") && arg_str.len() > 2)
        {
            return true;
        }

        if arg == OsStr::new("--output")
            || arg == OsStr::new("-o")
            || arg == OsStr::new("--screen")
            || arg == OsStr::new("-s")
        {
            skip_next_value = true;
        }
    }

    false
}

pub fn effective_cli_format(
    cli_format: capture::Format,
    format_explicit: bool,
    output: Option<&Path>,
    settings_format: capture::Format,
) -> capture::Format {
    if format_explicit {
        return cli_format;
    }

    if let Some(format) = format_from_output_extension(output) {
        return format;
    }

    settings_format
}

pub fn output_extension(output: Option<&Path>) -> Option<&str> {
    output
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
}

pub fn format_from_extension(extension: &str) -> Option<capture::Format> {
    if extension.eq_ignore_ascii_case("png") {
        Some(capture::Format::Png)
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        Some(capture::Format::Jpg)
    } else if extension.eq_ignore_ascii_case("bmp") {
        Some(capture::Format::Bmp)
    } else {
        None
    }
}

pub fn format_from_output_extension(output: Option<&Path>) -> Option<capture::Format> {
    output_extension(output).and_then(format_from_extension)
}

pub fn unsupported_output_extension(output: Option<&Path>) -> Option<String> {
    let extension = output_extension(output)?;
    format_from_extension(extension)
        .is_none()
        .then(|| extension.to_string())
}

pub fn should_run_cli_mode(
    cli: bool,
    list: bool,
    window: bool,
    has_output: bool,
    has_screen: bool,
    format_explicit: bool,
) -> bool {
    cli || list || window || has_output || has_screen || format_explicit
}

pub fn format_screen_list_output<I, S>(screens: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut output = String::from("Available screens:");
    for screen in screens {
        output.push('\n');
        output.push_str("  ");
        output.push_str(screen.as_ref());
    }
    output
}

pub fn selection_stderr_message(error: &str) -> Option<String> {
    if error == "Selection cancelled" {
        None
    } else {
        Some(error.to_string())
    }
}

pub fn write_error_line<W: Write>(mut writer: W, message: &str) -> io::Result<()> {
    writeln!(writer, "{message}")
}

/// Application mode determined from CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Tray,
    Cli,
    Selection,
    SelectRecord,
}

/// Request struct capturing the flags needed to decide the application mode.
#[derive(Debug, Clone)]
pub struct AppModeRequest {
    pub select_record: bool,
    pub select: bool,
    pub cli: bool,
    pub list: bool,
    pub window: bool,
    pub has_output: bool,
    pub has_screen: bool,
    pub format_explicit: bool,
}

/// Determines which application mode to run based on CLI flags.
///
/// Priority: select_record > select > CLI indicators > tray (default).
pub fn decide_app_mode(req: AppModeRequest) -> AppMode {
    if req.select_record {
        return AppMode::SelectRecord;
    }
    if req.select {
        return AppMode::Selection;
    }
    if should_run_cli_mode(
        req.cli,
        req.list,
        req.window,
        req.has_output,
        req.has_screen,
        req.format_explicit,
    ) {
        return AppMode::Cli;
    }
    AppMode::Tray
}

#[cfg(test)]
mod tests {
    use super::*;
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

    // =========================================================================
    // should_run_cli_mode
    // =========================================================================

    #[test]
    fn cli_mode_all_false() {
        assert!(!should_run_cli_mode(
            false, false, false, false, false, false
        ));
    }

    #[test]
    fn cli_mode_cli_flag() {
        assert!(should_run_cli_mode(true, false, false, false, false, false));
    }

    #[test]
    fn cli_mode_list_flag() {
        assert!(should_run_cli_mode(false, true, false, false, false, false));
    }

    #[test]
    fn cli_mode_window_flag() {
        assert!(should_run_cli_mode(false, false, true, false, false, false));
    }

    #[test]
    fn cli_mode_has_output() {
        assert!(should_run_cli_mode(false, false, false, true, false, false));
    }

    #[test]
    fn cli_mode_has_screen() {
        assert!(should_run_cli_mode(false, false, false, false, true, false));
    }

    #[test]
    fn cli_mode_format_explicit() {
        assert!(should_run_cli_mode(false, false, false, false, false, true));
    }

    #[test]
    fn cli_mode_multiple_flags() {
        assert!(should_run_cli_mode(true, true, true, true, true, true));
    }

    // =========================================================================
    // format_screen_list_output
    // =========================================================================

    #[test]
    fn screen_list_empty() {
        let output = format_screen_list_output(Vec::<&str>::new());
        assert_eq!(output, "Available screens:");
    }

    #[test]
    fn screen_list_single() {
        let output = format_screen_list_output(vec!["Screen 0: 1920x1080"]);
        assert_eq!(output, "Available screens:\n  Screen 0: 1920x1080");
    }

    #[test]
    fn screen_list_multiple() {
        let output = format_screen_list_output(vec!["Screen 0: 1920x1080", "Screen 1: 2560x1440"]);
        assert!(output.starts_with("Available screens:"));
        assert!(output.contains("  Screen 0: 1920x1080"));
        assert!(output.contains("  Screen 1: 2560x1440"));
    }

    #[test]
    fn screen_list_with_string_vec() {
        let screens = vec!["A".to_string(), "B".to_string()];
        let output = format_screen_list_output(screens);
        assert!(output.contains("  A"));
        assert!(output.contains("  B"));
    }

    // =========================================================================
    // selection_stderr_message
    // =========================================================================

    #[test]
    fn selection_stderr_cancelled_returns_none() {
        assert!(selection_stderr_message("Selection cancelled").is_none());
    }

    #[test]
    fn selection_stderr_other_error_returns_some() {
        let msg = selection_stderr_message("Something went wrong");
        assert_eq!(msg, Some("Something went wrong".to_string()));
    }

    #[test]
    fn selection_stderr_empty_error_returns_some() {
        assert_eq!(selection_stderr_message(""), Some(String::new()));
    }

    #[test]
    fn selection_stderr_similar_to_cancelled_returns_some() {
        // Must be exact match, not prefix
        assert!(selection_stderr_message("Selection cancelled!").is_some());
    }

    // =========================================================================
    // write_error_line
    // =========================================================================

    #[test]
    fn write_error_line_appends_newline() {
        let mut buf = Vec::new();
        write_error_line(&mut buf, "hello").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "hello\n");
    }

    #[test]
    fn write_error_line_empty_message() {
        let mut buf = Vec::new();
        write_error_line(&mut buf, "").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "\n");
    }

    #[test]
    fn write_error_line_returns_error_on_broken_writer() {
        struct BrokenWriter;
        impl Write for BrokenWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut writer = BrokenWriter;
        assert!(writer.flush().is_ok());
        assert!(write_error_line(writer, "test").is_err());
    }
}
