//! Deterministic active-window output delivery without a live desktop or clipboard.

use crate::{app, RustShotWorld};
use cucumber::{then, when};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;

#[when("active-window output delivery runs with simulated handlers")]
fn run_active_window_output_delivery(world: &mut RustShotWorld) {
    let expected = image::RgbaImage::from_raw(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 255]).unwrap();
    let capture_calls = Cell::new(0);
    let clipboard_images = RefCell::new(Vec::new());
    let saved_images = RefCell::new(Vec::new());
    let expected_path = PathBuf::from("window.png");

    let result = app::benchmark_capture_active_window_outputs_with_handlers(
        || {
            capture_calls.set(capture_calls.get() + 1);
            Ok(expected.clone())
        },
        |captured: &image::RgbaImage| {
            clipboard_images
                .borrow_mut()
                .push(captured.as_raw().clone());
            Ok(())
        },
        |captured: &image::RgbaImage| {
            saved_images.borrow_mut().push(captured.as_raw().clone());
            Ok(expected_path.clone())
        },
    );

    world.active_window_output_result = Some(result);
    world.active_window_capture_calls = capture_calls.get();
    world.active_window_expected_pixels = expected.as_raw().clone();
    world.active_window_clipboard_images = clipboard_images.into_inner();
    world.active_window_saved_images = saved_images.into_inner();
}

#[then("it captures the active window once")]
fn assert_one_active_window_capture(world: &mut RustShotWorld) {
    assert_eq!(world.active_window_capture_calls, 1);
}

#[then("it copies that same image to the clipboard once")]
fn assert_one_clipboard_copy_of_captured_image(world: &mut RustShotWorld) {
    assert_eq!(world.active_window_clipboard_images.len(), 1);
    assert_eq!(
        world.active_window_clipboard_images[0],
        world.active_window_expected_pixels
    );
}

#[then("it saves that same image once")]
fn assert_one_save_of_captured_image(world: &mut RustShotWorld) {
    assert_eq!(world.active_window_saved_images.len(), 1);
    assert_eq!(
        world.active_window_saved_images[0],
        world.active_window_expected_pixels
    );
    assert_eq!(
        world
            .active_window_output_result
            .as_ref()
            .expect("active-window output delivery did not run")
            .as_ref(),
        Ok(&PathBuf::from("window.png"))
    );
}
