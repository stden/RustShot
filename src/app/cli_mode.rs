//! CLI argument model, capture-format policy, app-mode routing, and the
//! one-shot CLI capture flow.

use clap::{Parser, ValueEnum};
use log::{error, info};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::{capture, cli, settings, window};

#[derive(Parser, Debug)]
#[command(name = "rustshot")]
#[command(author = "RustShot Contributors")]
#[command(version = "0.1.0")]
#[command(about = "A screenshot and screen recording utility written in Rust", long_about = None)]
pub(crate) struct Args {
    /// Run in CLI mode (single capture, no tray)
    #[arg(long)]
    pub(crate) cli: bool,

    /// Run as an MCP server over stdio (exposes capture tools to an MCP client)
    #[arg(long)]
    pub(crate) mcp: bool,

    /// Interactive selection mode (internal use)
    #[arg(long, hide = true)]
    pub(crate) select: bool,

    /// Capture the active window
    #[arg(short, long)]
    pub(crate) window: bool,

    /// List open windows (id and title), one per line
    #[arg(long)]
    pub(crate) list_windows: bool,

    /// Capture a specific window by its id (from --list-windows)
    #[arg(long, value_name = "ID")]
    pub(crate) window_id: Option<u64>,

    /// Start recording mode (internal use)
    #[arg(long, hide = true)]
    pub(crate) select_record: bool,

    /// Output file path (CLI mode only)
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,

    /// Image format to save
    #[arg(short, long, value_enum, default_value_t = CliFormat::Png)]
    pub(crate) format: CliFormat,

    /// Screen index to capture (0-based, default: primary screen)
    #[arg(short, long)]
    pub(crate) screen: Option<usize>,

    /// List available screens
    #[arg(short, long)]
    pub(crate) list: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)]
pub(crate) enum CliFormat {
    Png,
    Jpg,
    Bmp,
}

impl From<CliFormat> for capture::Format {
    fn from(f: CliFormat) -> Self {
        match f {
            CliFormat::Png => capture::Format::Png,
            CliFormat::Jpg => capture::Format::Jpg,
            CliFormat::Bmp => capture::Format::Bmp,
        }
    }
}

/// The capture format configured in settings.
///
/// This used to map a `String` onto [`capture::Format`]; now that `Settings`
/// stores the enum, there is nothing to map — the conversion happened once, at
/// the JSON boundary.
pub(crate) fn settings_capture_format(settings: &settings::Settings) -> capture::Format {
    settings.default_format
}

/// Detect whether the raw CLI arguments contain an explicit format flag.
///
/// Algorithm:
/// 1. Skip `argv[0]`.
/// 2. Track whether the previous token was an option that consumes the next
///    token as its value.
/// 3. Recognize explicit format forms:
///    `--format`, `-f`, `--format=value`, and attached short forms such as
///    `-fpng`.
/// 4. Ignore hyphen-prefixed values that belong to `--output/-o` or
///    `--screen/-s`.
///
/// Possible improvements:
/// - Replace the manual scan with a dedicated clap signal if the library later
///   exposes "was this argument explicitly present?" directly.
/// - Model options as tokens instead of raw strings if more attached-value
///   short flags are introduced.
pub(crate) fn cli_format_flag_present_in<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    cli::cli_format_flag_present_in(args)
}

/// Resolve the capture format for CLI mode using the project's precedence rules.
///
/// Algorithm:
/// 1. If the user passed an explicit format flag, use it.
/// 2. Otherwise, inspect the output filename extension and use it when it maps
///    to a supported format.
/// 3. If neither source is explicit enough, fall back to the saved setting.
///
/// Possible improvements:
/// - Return the winning source alongside the format for better diagnostics and
///   user messaging.
/// - Consolidate CLI and settings precedence into a shared policy type if more
///   output-related options are added.
pub(crate) fn effective_cli_format(
    cli_format: CliFormat,
    format_explicit: bool,
    output: Option<&Path>,
    settings: &settings::Settings,
) -> capture::Format {
    cli::effective_cli_format(
        cli_format.into(),
        format_explicit,
        output,
        settings_capture_format(settings),
    )
}

pub(crate) fn unsupported_output_extension(output: Option<&Path>) -> Option<String> {
    cli::unsupported_output_extension(output)
}

fn app_mode_request(args: &Args, format_explicit: bool) -> cli::AppModeRequest {
    cli::AppModeRequest {
        select_record: args.select_record,
        select: args.select,
        // --cli and the CLI-only window operations (--list-windows, --window <id>)
        // all mean "run once and exit", never the tray.
        cli: args.cli || args.list_windows || args.window_id.is_some(),
        list: args.list,
        window: args.window,
        has_output: args.output.is_some(),
        has_screen: args.screen.is_some(),
        format_explicit,
    }
}

pub(crate) fn decide_app_mode_with_format_explicit(
    args: &Args,
    format_explicit: bool,
) -> cli::AppMode {
    cli::decide_app_mode(app_mode_request(args, format_explicit))
}

/// Execute one CLI capture flow using persisted settings and injected backends.
///
/// Algorithm:
/// 1. Handle `--list` before any capture-specific validation.
/// 2. If the format was not explicit, reject unsupported output extensions so
///    the filename cannot disagree with the actual encoder.
/// 3. Resolve the effective format from CLI input, output path, and settings.
/// 4. Dispatch to either the active-window backend or the normal screenshot
///    backend.
/// 5. Wrap the returned path in the stable human-readable CLI success message.
///
/// Possible improvements:
/// - Return a structured CLI result type and render the final message in one
///   place to reduce formatting duplication.
/// - Split validation from execution if future commands need dry-run or preview
///   support.
pub(crate) fn run_cli_with_capture_using_settings<F, W>(
    args: Args,
    settings: &settings::Settings,
    format_explicit: bool,
    capture_fn: F,
    window_fn: W,
) -> Result<String, String>
where
    F: Fn(Option<PathBuf>, capture::Format, Option<usize>) -> Result<PathBuf, String>,
    W: Fn(Option<PathBuf>, capture::Format) -> Result<PathBuf, String>,
{
    // Resolve CLI intent in phases:
    // 1. short-circuit the list-only paths (`--list`, `--list-windows`)
    // 2. reject unsupported output extensions when format was not explicit
    // 3. compute the effective capture format
    // 4. dispatch to the requested capture backend
    if args.list {
        return Ok(format_screen_list_output(capture::list_screens()));
    }
    if args.list_windows {
        return Ok(window::format_window_list(&window::list_windows()));
    }

    if !format_explicit {
        if let Some(extension) = unsupported_output_extension(args.output.as_deref()) {
            return Err(format!(
                "Unsupported output extension '.{extension}'. Use png, jpg, or bmp, or pass --format explicitly."
            ));
        }
    }

    let format = effective_cli_format(
        args.format,
        format_explicit,
        args.output.as_deref(),
        settings,
    );

    if let Some(id) = args.window_id {
        return window::capture_window_in_dir(id, args.output, format, &settings.screenshots_dir)
            .map(|path| format!("Screenshot saved to: {}", path.display()));
    }

    if args.window {
        return window_fn(args.output, format)
            .map(|path| format!("Screenshot saved to: {}", path.display()));
    }

    capture_fn(args.output, format, args.screen)
        .map(|path| format!("Screenshot saved to: {}", path.display()))
}

/// Execute a single CLI capture and print the result path.
pub(crate) fn run_cli(args: Args) {
    let settings = settings::load_settings();
    let format_explicit = cli_format_flag_present_in(std::env::args_os());
    match run_cli_with_capture_using_settings(
        args,
        &settings,
        format_explicit,
        |output, format, screen| {
            capture::capture_screenshot_in_dir(output, format, screen, &settings.screenshots_dir)
        },
        |output, format| {
            capture::capture_active_window_in_dir(output, format, &settings.screenshots_dir)
        },
    ) {
        Ok(output) => {
            info!("{}", output);
            println!("{}", output);
        }
        Err(e) => {
            error!("CLI error: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Format screen list for CLI `--list` output.
///
/// Algorithm:
/// 1. Start with a stable header.
/// 2. Append each screen on its own indented line in input order.
/// 3. Return the assembled string as a single printable block.
///
/// Possible improvements:
/// - Stream directly into a writer if screen lists ever become large enough
///   that repeated string growth matters.
/// - Add machine-readable output modes such as JSON if automation becomes a
///   first-class use case.
pub(crate) fn format_screen_list_output<I>(screens: I) -> String
where
    I: IntoIterator<Item = String>,
{
    cli::format_screen_list_output(screens)
}
