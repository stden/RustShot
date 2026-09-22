use super::*;
use std::cell::RefCell;

#[when(regex = r#"^window delivery fails at "(capture|clipboard|file|both outputs|nowhere)"$"#)]
fn deliver_window(world: &mut RustShotWorld, failure: String) {
    let events = RefCell::new(Vec::new());
    let pixels = image::RgbaImage::from_pixel(2, 1, image::Rgba([1, 2, 3, 255]));
    world.workflows.delivery = Some(
        rustshot::app::benchmark_capture_active_window_outputs_with_handlers(
            || {
                events.borrow_mut().push("capture".to_string());
                if failure == "capture" {
                    Err("window closed".into())
                } else {
                    Ok(pixels.clone())
                }
            },
            |image: &image::RgbaImage| {
                events.borrow_mut().push("clipboard".to_string());
                assert_eq!(image, &pixels);
                if matches!(failure.as_str(), "clipboard" | "both outputs") {
                    Err("clipboard busy".into())
                } else {
                    Ok(())
                }
            },
            |image: &image::RgbaImage| {
                events.borrow_mut().push("file".to_string());
                assert_eq!(image, &pixels);
                if matches!(failure.as_str(), "file" | "both outputs") {
                    Err("disk full".into())
                } else {
                    Ok(PathBuf::from("window.png"))
                }
            },
        ),
    );
    world.workflows.events = events.into_inner();
}

#[then(regex = r#"^window delivery attempts "([^"]+)" in order$"#)]
fn assert_delivery_order(world: &mut RustShotWorld, expected: String) {
    assert_eq!(
        world.workflows.events,
        expected.split(',').collect::<Vec<_>>()
    );
}

#[then(regex = r#"^window delivery reports "([^"]+)"$"#)]
fn assert_delivery_result(world: &mut RustShotWorld, expected: String) {
    let actual = match world.workflows.delivery.as_ref().unwrap() {
        Ok(path) => path.display().to_string(),
        Err(error) => error.clone(),
    };
    assert_eq!(actual, expected);
}
