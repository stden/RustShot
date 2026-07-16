//! Unit tests for the selection module, split into thematic chunks.

use crate::selection::*;

mod adjacency_and_layouts;
mod app_state_and_scaling;
mod edge_cases_wave1;
mod edge_cases_wave2;
mod geometry_edges;
mod layout_fallbacks;
mod layout_inputs;
mod neighbor_inference;
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
