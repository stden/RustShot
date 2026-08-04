//! Unit tests for the selection module, split into thematic chunks.

use crate::selection::*;

mod adjacency_and_layouts;
mod app_state_and_scaling;
mod domain_types;
mod edge_cases_wave2;
mod geometry_edges;
mod layout_fallbacks;
mod layout_inputs;
mod neighbor_inference;
mod overlay_scaling;
mod recording_resolution;
mod region_basics;
mod requirements_and_props;
mod selection_app;
mod task_requirements;

fn make_screen(lx: i32, ly: i32, lw: u32, lh: u32, pw: u32, ph: u32) -> ScreenLayoutInput {
    ScreenLayoutInput {
        logical_x: lx,
        logical_y: ly,
        logical_width: lw,
        logical_height: lh,
        physical_width: pw,
        physical_height: ph,
        is_primary: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn make_layout(
    lx: i32,
    ly: i32,
    lw: u32,
    lh: u32,
    px: i32,
    py: i32,
    pw: u32,
    ph: u32,
) -> PhysicalScreenLayout {
    PhysicalScreenLayout {
        logical_x: lx,
        logical_y: ly,
        logical_width: lw,
        logical_height: lh,
        physical_x: px,
        physical_y: py,
        physical_width: pw,
        physical_height: ph,
    }
}

/// The canonical two-monitor arrangement used across the resolution tests:
/// a 1920x1080 primary at the origin and a 2560x1440 secondary immediately
/// to its right, both at 1:1 DPI (physical == logical).
fn two_monitor_layouts() -> Vec<PhysicalScreenLayout> {
    vec![
        make_layout(0, 0, 1920, 1080, 0, 0, 1920, 1080),
        make_layout(1920, 0, 2560, 1440, 1920, 0, 2560, 1440),
    ]
}
