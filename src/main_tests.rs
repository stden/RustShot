//! Unit tests for the binary crate root, split into thematic child modules.
//!
//! This module replaces the former in-file `tests` module of `main.rs`.
//! Shared test helpers live here; the child modules pull them (and the
//! application items re-exported from [`crate::app`]) in via `use super::*;`.

use crate::app::*;
use crate::capture::{self, Format};
use crate::cli::{self, AppMode};
use crate::settings;
use clap::Parser;
use global_hotkey::hotkey::HotKey;
use std::cell::Cell;
use std::io;
use std::path::{Path, PathBuf};

mod capture_flow;
mod capture_window;
mod cli_parsing;
mod format_policy;
mod format_traits;
mod infra;
mod mode_decision;
mod mode_flags;
mod platform;
mod screen_listing;
mod selection_tests;
mod settings_flow;

/// Test helper: wraps run_cli_with_capture with a dummy window capture fn.
fn run_cli_with_capture_test<F>(args: Args, capture_fn: F) -> Result<String, String>
where
    F: Fn(Option<PathBuf>, capture::Format, Option<usize>) -> Result<PathBuf, String>,
{
    run_cli_with_capture(args, capture_fn, |_, _| {
        Err("window capture not used in test".to_string())
    })
}

fn decide_app_mode(args: &Args) -> AppMode {
    decide_app_mode_with_format_explicit(args, args.format != CliFormat::Png)
}

fn output_extension(output: Option<&Path>) -> Option<&str> {
    cli::output_extension(output)
}

fn format_from_extension(extension: &str) -> Option<capture::Format> {
    cli::format_from_extension(extension)
}

fn format_from_output_extension(output: Option<&Path>) -> Option<capture::Format> {
    cli::format_from_output_extension(output)
}

fn should_run_cli_mode_with_format_explicit(args: &Args, format_explicit: bool) -> bool {
    cli::should_run_cli_mode(
        args.cli,
        args.list,
        args.window,
        args.output.is_some(),
        args.screen.is_some(),
        format_explicit,
    )
}

/// Core CLI logic with injected capture/window functions (for testing without real screen).
fn run_cli_with_capture<F, W>(args: Args, capture_fn: F, window_fn: W) -> Result<String, String>
where
    F: Fn(Option<PathBuf>, capture::Format, Option<usize>) -> Result<PathBuf, String>,
    W: Fn(Option<PathBuf>, capture::Format) -> Result<PathBuf, String>,
{
    run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        true,
        capture_fn,
        window_fn,
    )
}

/// Determine whether CLI mode should be used based on the provided arguments.
fn should_run_cli_mode(args: &Args) -> bool {
    should_run_cli_mode_with_format_explicit(args, args.format != CliFormat::Png)
}

/// Open the screenshots folder with injected create/open handlers (for testing).
fn open_screenshots_folder_with_handlers<CreateDir, OpenDir>(
    create_dir: CreateDir,
    open_dir: OpenDir,
) where
    CreateDir: Fn(&Path) -> Result<(), String>,
    OpenDir: Fn(&Path),
{
    let dir = capture::get_screenshots_dir();
    open_screenshots_folder_at_with_handlers(&dir, create_dir, open_dir);
}
