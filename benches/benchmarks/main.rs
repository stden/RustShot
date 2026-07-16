use criterion::{criterion_group, criterion_main};

pub(crate) fn patterned_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 256) as u8).collect()
}

pub(crate) fn benchmark_screens() -> Result<Vec<screenshots::Screen>, String> {
    screenshots::Screen::all().map_err(|e| format!("Failed to get screens: {e}"))
}

pub(crate) fn benchmark_skip(group_name: &str, reason: &str) {
    eprintln!("Skipping benchmark group '{group_name}': {reason}");
}

mod capture;
mod capture_live;
mod cli;
mod recording;
mod selection;
mod selection_layout;
mod settings_json;
mod tray;

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    // capture
    capture::bench_bgra_to_rgba,
    capture::bench_resolve_screen_index,
    capture::bench_default_screen_index,
    capture::bench_resolve_output_path,
    capture::bench_resolve_output_path_in_dir,
    capture::bench_format_extension,
    capture::bench_format_to_image_format,
    capture::bench_get_screenshots_dir,
    capture::bench_list_screens,
    capture::bench_save_image_optimized,
    capture::bench_ensure_parent_directory,
    capture_live::bench_capture_screenshot,
    capture_live::bench_capture_screenshot_stages,
    capture_live::bench_capture_active_window,
    // record
    recording::bench_frame_duration_hns,
    recording::bench_round_to_even,
    recording::bench_rgba_to_bgra_bottom_up,
    recording::bench_pack_u32_pair,
    recording::bench_validate_recording_start,
    recording::bench_get_recording_dir,
    recording::bench_get_recording_path,
    recording::bench_get_recording_path_in_dir,
    recording::bench_recording_stages,
    // settings
    settings_json::bench_format_settings_json,
    settings_json::bench_parse_settings_json,
    settings_json::bench_settings_path,
    settings_json::bench_log_file_path,
    settings_json::bench_load_settings,
    settings_json::bench_normalize_default_format,
    settings_json::bench_normalize_recording_fps,
    settings_json::bench_extract_json_string,
    settings_json::bench_extract_json_number,
    settings_json::bench_extract_json_bool,
    // cli
    cli::bench_settings_capture_format,
    cli::bench_cli_format_flag_present_in,
    cli::bench_cli_path_helpers,
    cli::bench_effective_cli_format,
    cli::bench_should_run_cli_mode,
    cli::bench_format_screen_list_output,
    cli::bench_selection_stderr_message,
    cli::bench_write_error_line,
    cli::bench_child_process_is_running,
    // selection
    selection::bench_build_region_from_drag,
    selection::bench_virtual_desktop_scale_factor,
    selection::bench_logical_to_physical,
    selection::bench_find_screen_index_at_position,
    selection::bench_preferred_screen_index,
    selection::bench_get_cursor_position,
    selection::bench_scaled_physical_dimension,
    selection::bench_build_screen_layout_inputs_physical_sizes,
    selection_layout::bench_build_physical_screen_layouts,
    selection_layout::bench_normalize_physical_screen_layouts,
    selection_layout::bench_resolve_recording_selection,
    selection_layout::bench_ranges_overlap,
    selection_layout::bench_monitor_scale_ratio,
    selection_layout::bench_scaled_gap,
    // tray
    tray::bench_build_default_icon_pixels,
);

criterion_main!(benches);
