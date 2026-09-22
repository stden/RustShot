//! Pure CLI helper logic shared by the binary, tests, and benchmarks.

use crate::domain::Format;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::Path;

#[expect(
    clippy::needless_pass_by_value,
    reason = "generic process status is consumed by the predicate"
)]
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

        if arg == OsStr::new("--") {
            break;
        }

        if arg == OsStr::new("--format") || arg == OsStr::new("-f") {
            return true;
        }

        if arg_str.starts_with("--format=") {
            return true;
        }

        if let Some((flag, value)) = short_value_option(&arg_str) {
            if flag == 'f' {
                return true;
            }
            skip_next_value = value.is_empty();
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

fn short_value_option(arg: &str) -> Option<(char, &str)> {
    // Clap accepts bundles such as -wfjpg. Stop at a value-taking option so
    // a filename containing 'f' cannot masquerade as -f.
    let flags = arg
        .strip_prefix('-')
        .filter(|flags| !flags.starts_with('-'))?;
    let (index, flag) = flags
        .char_indices()
        .find(|(_, flag)| matches!(flag, 'f' | 'o' | 's'))?;
    Some((flag, &flags[index + 1..]))
}

pub fn effective_cli_format(
    cli_format: Format,
    format_explicit: bool,
    output: Option<&Path>,
    settings_format: Format,
) -> Format {
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

pub fn format_from_extension(extension: &str) -> Option<Format> {
    if extension.eq_ignore_ascii_case("png") {
        Some(Format::Png)
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        Some(Format::Jpg)
    } else if extension.eq_ignore_ascii_case("bmp") {
        Some(Format::Bmp)
    } else {
        None
    }
}

pub fn format_from_output_extension(output: Option<&Path>) -> Option<Format> {
    output_extension(output).and_then(format_from_extension)
}

pub fn unsupported_output_extension(output: Option<&Path>) -> Option<String> {
    let extension = output_extension(output)?;
    format_from_extension(extension)
        .is_none()
        .then(|| extension.to_string())
}

/// Compatibility query for existing benchmark and library callers. Application
/// routing uses the typed [`AppModeRequest`] through [`decide_app_mode`].
#[expect(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    reason = "Legacy public flag signature; the implementation delegates to typed AppModeRequest routing"
)]
pub fn should_run_cli_mode(
    cli: bool,
    list: bool,
    window: bool,
    has_output: bool,
    has_screen: bool,
    format_explicit: bool,
) -> bool {
    decide_app_mode(AppModeRequest {
        select_record: false,
        select: false,
        cli,
        list,
        window,
        has_output,
        has_screen,
        format_explicit,
    }) == AppMode::Cli
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
#[derive(Clone, Copy, Debug)]
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
/// Priority: `select_record` > select > CLI indicators > tray (default).
pub fn decide_app_mode(req: AppModeRequest) -> AppMode {
    if req.select_record {
        return AppMode::SelectRecord;
    }
    if req.select {
        return AppMode::Selection;
    }
    if req.cli || req.list || req.window || req.has_output || req.has_screen || req.format_explicit
    {
        return AppMode::Cli;
    }
    AppMode::Tray
}

#[cfg(test)]
#[path = "../tests/unit/cli/mod.rs"]
mod tests;
