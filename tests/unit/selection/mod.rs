//! Unit tests for the selection module, split into thematic chunks.

use crate::selection::*;

mod adjacency_and_layouts;
mod app_state_and_scaling;
mod behavior;
mod domain_types;
mod edge_cases_wave2;
mod geometry_edges;
mod layout_fallbacks;
mod layout_inputs;
#[cfg(windows)]
mod native_cursor;
mod neighbor_inference;
mod overlay_scaling;
#[cfg(windows)]
mod presentation;
mod recording_resolution;
mod region_basics;
mod requirements_and_props;
mod selection_app;
mod task_requirements;

fn make_screen(
    logical_origin: (i32, i32),
    logical_size: (u32, u32),
    physical_size: (u32, u32),
) -> ScreenLayoutInput {
    ScreenLayoutInput {
        logical_x: logical_origin.0,
        logical_y: logical_origin.1,
        logical_width: logical_size.0,
        logical_height: logical_size.1,
        physical_width: physical_size.0,
        physical_height: physical_size.1,
        is_primary: false,
    }
}

fn make_layout(
    logical_origin: (i32, i32),
    logical_size: (u32, u32),
    physical_origin: (i32, i32),
    physical_size: (u32, u32),
) -> PhysicalScreenLayout {
    PhysicalScreenLayout {
        logical_x: logical_origin.0,
        logical_y: logical_origin.1,
        logical_width: logical_size.0,
        logical_height: logical_size.1,
        physical_x: physical_origin.0,
        physical_y: physical_origin.1,
        physical_width: physical_size.0,
        physical_height: physical_size.1,
    }
}

/// The canonical two-monitor arrangement used across the resolution tests:
/// a 1920x1080 primary at the origin and a 2560x1440 secondary immediately
/// to its right, both at 1:1 DPI (physical == logical).
fn two_monitor_layouts() -> Vec<PhysicalScreenLayout> {
    vec![
        make_layout((0, 0), (1920, 1080), (0, 0), (1920, 1080)),
        make_layout((1920, 0), (2560, 1440), (1920, 0), (2560, 1440)),
    ]
}
