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
mod tests;
