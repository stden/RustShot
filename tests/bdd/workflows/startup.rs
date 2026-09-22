use super::*;
use std::cell::RefCell;
use std::io;
use std::path::Path;

#[when(
    regex = r#"^first-run auto-start is "(enabled|disabled)" with "(none|inspect|create|copy|register|launch|rollback|remove)" failing$"#
)]
fn prepare_startup(world: &mut RustShotWorld, enabled: String, failure: String) {
    let events = RefCell::new(Vec::new());
    let operation = |name: &str| {
        events.borrow_mut().push(name.to_string());
        if failure == name || (failure == "rollback" && name == "launch") {
            Err(io::Error::other(format!("{name} denied")))
        } else {
            Ok(())
        }
    };
    world.startup_should_exit = Some(rustshot::app::benchmark_sync_autorun_at_with_handlers(
        enabled == "enabled",
        Path::new("Downloads/RustShot.exe"),
        Path::new("LocalData"),
        |_| operation("inspect").map(|()| false),
        |_| operation("create"),
        |_, _| operation("copy").map(|()| 1),
        |_| operation("register").map_err(|error| error.to_string()),
        || {
            operation(if enabled == "enabled" {
                "rollback"
            } else {
                "remove"
            })
            .map_err(|error| error.to_string())
        },
        |_| operation("launch"),
    ));
    world.workflows.events = events.into_inner();
}

#[then(regex = r#"^first-run operations are "([^"]+)"$"#)]
fn assert_startup_operations(world: &mut RustShotWorld, expected: String) {
    assert_eq!(
        world.workflows.events,
        expected.split(',').collect::<Vec<_>>()
    );
}

#[then(regex = r#"^first-run startup reports "([^"]+)"$"#)]
fn assert_startup_result(world: &mut RustShotWorld, expected: String) {
    let result = world.startup_should_exit.as_ref().unwrap();
    match expected.as_str() {
        "continue" => assert_eq!(result, &Ok(false)),
        "handoff" => assert_eq!(result, &Ok(true)),
        _ => assert!(
            result.as_ref().unwrap_err().contains(&expected),
            "{result:?}"
        ),
    }
}
