//! Integration tests for all public pure functions used in benchmarks.
//! Ensures these functions remain accessible from external crates (benchmarks, tests).

mod benchmark_helpers;
mod capture;
mod cli_tray;
mod cross_module;
mod live_capture;
mod record;
mod record_params;
mod selection;
mod settings;
mod windows_manual;
