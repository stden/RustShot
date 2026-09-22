//! Feature-level file, process, listing, and delivery workflows.

use crate::{capture, cli, known_format, record, RustShotWorld};
use cucumber::{given, then, when};
use rustshot::window::{self, WindowInfo};
use std::path::PathBuf;

#[path = "workflows/delivery.rs"]
mod delivery;
#[path = "workflows/startup.rs"]
mod startup;
#[path = "../unit/support/mod.rs"]
mod support;
#[path = "workflows/transport.rs"]
mod transport;

#[derive(Debug, Default)]
pub(crate) struct WorkflowState {
    directory: Option<support::TestDir>,
    image: Option<image::RgbaImage>,
    path: Option<PathBuf>,
    saved: Option<Result<(), String>>,
    format: Option<capture::Format>,
    listed: Option<bool>,
    listing: Option<String>,
    process: Option<std::process::Output>,
    replies: Vec<serde_json::Value>,
    events: Vec<String>,
    delivery: Option<Result<PathBuf, String>>,
    running: Option<bool>,
    diagnostic: Option<Vec<u8>>,
}

#[given("an isolated screenshot output directory")]
fn isolated_directory(world: &mut RustShotWorld) {
    world.workflows.directory = Some(support::TestDir::new("rustshot_bdd_output"));
}

#[when(regex = r#"^a known image is saved as "(png|jpg|bmp)"$"#)]
fn save_image(world: &mut RustShotWorld, format: String) {
    let image = image::RgbaImage::from_pixel(8, 6, image::Rgba([12, 34, 56, 255]));
    let format = known_format(&format);
    let path = world
        .workflows
        .directory
        .as_ref()
        .unwrap()
        .path()
        .join(format!("image.{}", format.extension()));
    world.workflows.saved = Some(capture::save_image_optimized(&image, &path, format));
    world.workflows.image = Some(image);
    world.workflows.path = Some(path);
    world.workflows.format = Some(format);
}

#[then("the saved file has the requested encoding, dimensions, and pixels")]
fn verify_saved_image(world: &mut RustShotWorld) {
    assert_eq!(world.workflows.saved.as_ref().unwrap(), &Ok(()));
    let bytes = std::fs::read(world.workflows.path.as_ref().unwrap()).unwrap();
    let format = world.workflows.format.unwrap();
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        format.to_image_format()
    );
    let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
    let original = world.workflows.image.as_ref().unwrap();
    assert_eq!(decoded.dimensions(), original.dimensions());
    let tolerance = if format == capture::Format::Jpg { 3 } else { 0 };
    for (actual, expected) in decoded.as_raw().iter().zip(original.as_raw()) {
        assert!(actual.abs_diff(*expected) <= tolerance);
    }
}

#[when("an image is saved beneath a file instead of a directory")]
fn save_to_blocked_parent(world: &mut RustShotWorld) {
    let blocker = world.workflows.directory.as_ref().unwrap().file("blocker");
    std::fs::write(&blocker, b"keep this file").unwrap();
    let result = capture::save_image_optimized(
        &image::RgbaImage::new(1, 1),
        &blocker.join("image.png"),
        capture::Format::Png,
    );
    world.workflows.saved = Some(result);
    world.workflows.path = Some(blocker);
}

#[then("saving reports the file error and preserves the existing file")]
fn assert_save_error(world: &mut RustShotWorld) {
    let error = world
        .workflows
        .saved
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_err();
    assert!(error.contains("Failed to create file"), "{error}");
    assert_eq!(
        std::fs::read(world.workflows.path.as_ref().unwrap()).unwrap(),
        b"keep this file"
    );
}

#[when(regex = r#"^a (visible|hidden) window titled "([^"]*)" is considered for listing$"#)]
fn window_eligibility(world: &mut RustShotWorld, visibility: String, title: String) {
    world.workflows.listed = Some(window::is_listable_window(visibility == "visible", &title));
}

#[then(regex = r"^the window is (included|excluded)$")]
fn assert_window_eligibility(world: &mut RustShotWorld, decision: String) {
    assert_eq!(world.workflows.listed, Some(decision == "included"));
}

#[when("these windows are formatted for the user:")]
fn format_windows(world: &mut RustShotWorld, step: &cucumber::gherkin::Step) {
    let windows = step
        .table
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .skip(1)
        .map(|row| WindowInfo {
            id: row[0].parse().unwrap(),
            title: row[1].clone(),
        })
        .collect::<Vec<_>>();
    world.workflows.listing = Some(window::format_window_list(&windows));
}

#[when("an empty window list is formatted for the user")]
fn format_no_windows(world: &mut RustShotWorld) {
    world.workflows.listing = Some(window::format_window_list(&[]));
}

#[then("the listing is:")]
fn assert_listing(world: &mut RustShotWorld, step: &cucumber::gherkin::Step) {
    assert_eq!(
        world.workflows.listing.as_deref(),
        step.docstring
            .as_deref()
            .map(|text| text.trim_matches('\n'))
    );
}

#[when("a recording filename is requested in the isolated directory")]
fn recording_path(world: &mut RustShotWorld) {
    world.workflows.path = Some(record::get_recording_path_in_dir(
        world.workflows.directory.as_ref().unwrap().path(),
    ));
}

#[then("the recording path is a unique timestamped MP4 in that directory")]
fn assert_recording_path(world: &mut RustShotWorld) {
    let directory = world.workflows.directory.as_ref().unwrap().path();
    let path = world.workflows.path.as_ref().unwrap();
    assert_eq!(path.parent(), Some(directory));
    assert_eq!(path.extension().unwrap(), "mp4");
    let name = path.file_name().unwrap().to_str().unwrap();
    let timestamp = name
        .strip_prefix("recording_")
        .unwrap()
        .strip_suffix(".mp4")
        .unwrap();
    let parts = timestamp.split('_').collect::<Vec<_>>();
    assert_eq!(
        parts.iter().map(|part| part.len()).collect::<Vec<_>>(),
        [8, 6, 9]
    );
    assert!(parts
        .iter()
        .all(|part| part.bytes().all(|byte| byte.is_ascii_digit())));
    assert_ne!(*path, record::get_recording_path_in_dir(directory));
    assert!(
        !path.exists(),
        "choosing a path must not create an empty recording"
    );
}

#[when(regex = r#"^a selection child reports "(running|finished|inspection failed)"$"#)]
fn child_state(world: &mut RustShotWorld, state: String) {
    let state = match state.as_str() {
        "running" => Ok(None),
        "finished" => Ok(Some(())),
        _ => Err("inspection failed"),
    };
    world.workflows.running = Some(cli::child_process_is_running(state));
}

#[then(regex = r"^another selection is (blocked|allowed)$")]
fn assert_selection_busy(world: &mut RustShotWorld, state: String) {
    assert_eq!(world.workflows.running, Some(state == "blocked"));
}

#[when(regex = r#"^selection finishes with "([^"]+)"$"#)]
fn selection_diagnostic(world: &mut RustShotWorld, message: String) {
    let mut output = Vec::new();
    if let Some(message) = cli::selection_stderr_message(&message) {
        cli::write_error_line(&mut output, &message).unwrap();
    }
    world.workflows.diagnostic = Some(output);
}

#[then(regex = r#"^the selection diagnostic is "([^"]*)"$"#)]
fn assert_selection_diagnostic(world: &mut RustShotWorld, expected: String) {
    let expected = if expected.is_empty() {
        expected
    } else {
        format!("{expected}\n")
    };
    assert_eq!(
        world.workflows.diagnostic.as_deref(),
        Some(expected.as_bytes())
    );
}
