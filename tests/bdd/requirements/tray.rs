use crate::{tray, RustShotWorld};
use cucumber::{then, when};

fn action_name(action: tray::MenuAction) -> &'static str {
    match action {
        tray::MenuAction::Capture => "Capture",
        tray::MenuAction::CaptureWindow => "CaptureWindow",
        tray::MenuAction::OpenFolder => "OpenFolder",
        tray::MenuAction::ToggleRecord => "ToggleRecord",
        tray::MenuAction::Settings => "Settings",
        tray::MenuAction::Exit => "Exit",
    }
}

#[when(regex = r#"^tray menu id \"([^\"]+)\" is resolved$"#)]
fn resolve_tray_id(world: &mut RustShotWorld, id: String) {
    world.tray_action = Some(tray::MenuAction::from_id(&id));
}

#[then(regex = r#"^the tray action is \"([A-Za-z]+)\"$"#)]
fn assert_tray_action(world: &mut RustShotWorld, expected: String) {
    let action = world
        .tray_action
        .expect("tray id was not resolved")
        .expect("expected a tray action");
    assert_eq!(action_name(action), expected);
}

#[then("no tray action is resolved")]
fn assert_no_tray_action(world: &mut RustShotWorld) {
    assert_eq!(world.tray_action, Some(None));
}

#[when("the default tray icon pixels are generated")]
fn generate_tray_pixels(world: &mut RustShotWorld) {
    world.tray_pixels = Some(tray::benchmark_build_default_icon_pixels());
}

fn tray_pixels(world: &RustShotWorld) -> &[u8] {
    world
        .tray_pixels
        .as_deref()
        .expect("tray pixels were not generated")
}

#[then(regex = r"^the tray icon is a (\d+) x (\d+) RGBA buffer$")]
fn assert_tray_dimensions(world: &mut RustShotWorld, width: usize, height: usize) {
    assert_eq!(tray_pixels(world).len(), width * height * 4);
}

#[then("every tray icon alpha value is binary")]
fn assert_binary_alpha(world: &mut RustShotWorld) {
    assert!(tray_pixels(world)
        .chunks_exact(4)
        .all(|pixel| matches!(pixel[3], 0 | 255)));
}

#[then("the tray icon opacity is horizontally symmetric")]
fn assert_horizontal_symmetry(world: &mut RustShotWorld) {
    let pixels = tray_pixels(world);
    let alpha = |x: usize, y: usize| pixels[(y * 32 + x) * 4 + 3];
    for y in 0..32 {
        for distance in 1..=15 {
            assert_eq!(
                alpha(16 - distance, y),
                alpha(16 + distance, y),
                "opacity differs at y={y}, distance={distance}"
            );
        }
    }
}

#[then("every opaque tray icon pixel is camera blue or lens white")]
fn assert_tray_colors(world: &mut RustShotWorld) {
    for pixel in tray_pixels(world).chunks_exact(4) {
        if pixel[3] == 255 {
            assert!(
                pixel[..3] == [41, 128, 185] || pixel[..3] == [255, 255, 255],
                "unexpected opaque color: {:?}",
                &pixel[..3]
            );
        }
    }
}
