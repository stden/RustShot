use super::*;
use cucumber::{given, then, when, World};
use winit::dpi::{PhysicalPosition, PhysicalSize};

#[derive(Default, World)]
struct SelectionWorld {
    app: Option<SelectionApp>,
    completed: Option<bool>,
    output: Option<Result<crate::application::capture::CaptureOutcome, String>>,
    copied: usize,
    directory: Option<crate::test_support::TestDir>,
}

impl std::fmt::Debug for SelectionWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionWorld")
            .field("completed", &self.completed)
            .field("output", &self.output)
            .finish()
    }
}

#[given("a frozen 100 by 100 screenshot in the selection overlay")]
fn frozen_overlay(world: &mut SelectionWorld) {
    world.app = Some(SelectionApp::new(vec![255; 100 * 100 * 4], 100, 100, 0, 0));
}

#[when(regex = r"^the pointer drags from \((\d+), (\d+)\) to \((\d+), (\d+)\)$")]
fn drag_pointer(world: &mut SelectionWorld, x: i32, y: i32, end_x: i32, end_y: i32) {
    let app = world.app.as_mut().unwrap();
    app.on_cursor_moved(PhysicalPosition::new(f64::from(x), f64::from(y)));
    app.on_press();
    app.on_cursor_moved(PhysicalPosition::new(f64::from(end_x), f64::from(end_y)));
    world.completed = Some(app.on_release());
}

#[then(regex = r"^selection completes at \((\d+), (\d+)\) sized (\d+) by (\d+)$")]
fn assert_region(world: &mut SelectionWorld, x: i32, y: i32, width: u32, height: u32) {
    assert_eq!(world.completed, Some(true));
    let region = world.app.as_ref().unwrap().result.unwrap();
    assert_eq!(
        (
            region.physical_x,
            region.physical_y,
            region.physical_width,
            region.physical_height
        ),
        (x, y, width, height)
    );
}

#[then("the overlay remains ready for another drag")]
fn assert_retry(world: &mut SelectionWorld) {
    let app = world.app.as_ref().unwrap();
    assert_eq!(world.completed, Some(false));
    assert!(app.result.is_none() && !app.is_dragging && app.start_pos.is_none());
}

#[when("the overlay is cancelled during a drag")]
fn cancel_drag(world: &mut SelectionWorld) {
    let app = world.app.as_mut().unwrap();
    app.on_cursor_moved(PhysicalPosition::new(10.0, 10.0));
    app.on_press();
    app.on_cursor_moved(PhysicalPosition::new(60.0, 70.0));
    app.on_cancel();
    world.completed = Some(app.on_release());
}

#[then("no selected region is returned after cancellation")]
fn assert_cancelled(world: &mut SelectionWorld) {
    let app = world.app.as_ref().unwrap();
    assert!(app.cancelled);
    assert!(app.result.is_none());
    assert_eq!(world.completed, Some(false));
}

#[when("the overlay is resized during a drag and the button is released")]
fn resize_during_drag(world: &mut SelectionWorld) {
    let app = world.app.as_mut().unwrap();
    app.on_cursor_moved(PhysicalPosition::new(10.0, 10.0));
    app.on_press();
    app.on_resize(PhysicalSize::new(80, 80));
    world.completed = Some(app.on_release());
}

#[when("the recording parent writes a stop marker before a drag completes")]
fn parent_stops_selection(world: &mut SelectionWorld) {
    let directory = crate::test_support::TestDir::new("rustshot_bdd_selection_stop");
    let path = directory.file("recording.stop");
    let app = world.app.as_mut().unwrap();
    app.cancellation_path = Some(path.clone());
    std::fs::write(path, b"stop").unwrap();
    assert!(app.poll_cancellation());
    world.completed = Some(app.on_release());
    world.directory = Some(directory);
}

#[when(regex = r#"^a completed region is delivered with clipboard "(available|unavailable)"$"#)]
fn deliver_selection(world: &mut SelectionWorld, clipboard: String) {
    use crate::application::capture::{execute_capture_with, CaptureRequest};
    world.output = Some(execute_capture_with(
        CaptureRequest::selection(),
        |_| {
            Ok(image::RgbaImage::from_pixel(
                2,
                3,
                image::Rgba([1, 2, 3, 255]),
            ))
        },
        |image| {
            assert_eq!(image.dimensions(), (2, 3));
            assert_eq!(image.get_pixel(0, 0).0, [1, 2, 3, 255]);
            world.copied += 1;
            if clipboard == "available" {
                Ok(())
            } else {
                Err("clipboard busy".into())
            }
        },
        |_, _| panic!("region captures must not save an unsolicited file"),
    ));
}

#[then(regex = r#"^region delivery reports "(copied|clipboard busy)" without writing a file$"#)]
fn assert_delivery(world: &mut SelectionWorld, expected: String) {
    assert_eq!(world.copied, 1);
    let actual = match world.output.as_ref().unwrap() {
        Ok(crate::application::capture::CaptureOutcome::Clipboard) => "copied",
        Err(error) => error.as_str(),
        _ => panic!("unexpected file output"),
    };
    assert_eq!(actual, expected);
}

#[test]
fn bdd_interactive_selection_workflows() {
    futures::executor::block_on(
        SelectionWorld::cucumber()
            .with_default_cli()
            .fail_on_skipped()
            .run_and_exit("tests/features/in_process/selection_interaction.feature"),
    );
}
