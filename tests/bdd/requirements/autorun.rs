//! First-run installation policy exercised without live filesystem or registry access.

use crate::{app, RustShotWorld};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};

#[given("auto-start is enabled for a RustShot executable outside its install directory")]
fn enabled_downloaded_executable(world: &mut RustShotWorld) {
    world.startup_current_exe = Some(PathBuf::from("Downloads").join("RustShot.exe"));
    world.startup_local_app_data = Some(PathBuf::from("LOCALAPPDATA"));
}

#[given("a stable RustShot installation already exists")]
fn stable_install_exists(world: &mut RustShotWorld) {
    world.startup_installed_exists = true;
}

#[given("a portable RustShot session outside its install directory re-enables auto-start")]
fn portable_session_reenables_auto_start(world: &mut RustShotWorld) {
    world.startup_current_exe = Some(PathBuf::from("Downloads").join("RustShot.exe"));
    world.startup_local_app_data = Some(PathBuf::from("LOCALAPPDATA"));
}

#[when("startup preparation runs through simulated Windows handlers")]
fn prepare_startup(world: &mut RustShotWorld) {
    let current_exe = world
        .startup_current_exe
        .as_ref()
        .expect("the current executable path was not configured");
    let local_app_data = world
        .startup_local_app_data
        .as_ref()
        .expect("the local app data path was not configured");
    let installed_exe = local_app_data
        .join("Programs")
        .join("RustShot")
        .join("RustShot.exe");
    let installed_exists = world.startup_installed_exists;
    let events = RefCell::new(Vec::new());

    let result = app::benchmark_sync_autorun_at_with_handlers(
        true,
        current_exe,
        local_app_data,
        |target: &Path| -> io::Result<bool> {
            assert_eq!(target, installed_exe);
            Ok(installed_exists)
        },
        |directory: &Path| -> io::Result<()> {
            assert_eq!(directory, installed_exe.parent().unwrap());
            events.borrow_mut().push("create install directory".into());
            Ok(())
        },
        |source: &Path, target: &Path| -> io::Result<u64> {
            assert_eq!(source, current_exe);
            assert_eq!(target, installed_exe);
            events.borrow_mut().push("copy executable".into());
            Ok(1)
        },
        |target: &Path| -> Result<(), String> {
            assert_eq!(target, installed_exe);
            events
                .borrow_mut()
                .push("register installed executable".into());
            Ok(())
        },
        || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
        |target: &Path| -> io::Result<()> {
            assert_eq!(target, installed_exe);
            events
                .borrow_mut()
                .push("launch installed executable".into());
            Ok(())
        },
    );

    world.startup_installed_exe = Some(installed_exe);
    world.startup_events = events.into_inner();
    world.startup_should_exit = Some(result);
}

#[when("Settings startup synchronization runs through simulated Windows handlers")]
fn synchronize_settings_startup(world: &mut RustShotWorld) {
    let current_exe = world
        .startup_current_exe
        .as_ref()
        .expect("the current executable path was not configured");
    let local_app_data = world
        .startup_local_app_data
        .as_ref()
        .expect("the local app data path was not configured");
    let installed_exe = local_app_data
        .join("Programs")
        .join("RustShot")
        .join("RustShot.exe");
    let events = RefCell::new(Vec::new());

    let result = app::benchmark_sync_current_exe_autorun_at_with_handlers(
        true,
        current_exe,
        local_app_data,
        |target: &Path| -> io::Result<bool> {
            assert_eq!(target, installed_exe);
            Ok(false)
        },
        |directory: &Path| -> io::Result<()> {
            assert_eq!(directory, installed_exe.parent().unwrap());
            events.borrow_mut().push("create install directory".into());
            Ok(())
        },
        |source: &Path, target: &Path| -> io::Result<u64> {
            assert_eq!(source, current_exe);
            assert_eq!(target, installed_exe);
            events.borrow_mut().push("copy executable".into());
            Ok(1)
        },
        |target: &Path| -> Result<(), String> {
            assert_eq!(target, installed_exe);
            events
                .borrow_mut()
                .push("register installed executable".into());
            Ok(())
        },
        || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
    );

    world.startup_installed_exe = Some(installed_exe);
    world.startup_events = events.into_inner();
    world.startup_settings_result = Some(result);
}

#[then(regex = r#"^the stable startup path is under local app data at "([^"]+)"$"#)]
fn assert_stable_path(world: &mut RustShotWorld, relative_path: String) {
    let local_app_data = world
        .startup_local_app_data
        .as_ref()
        .expect("the local app data path was not configured");
    let expected = relative_path
        .split('/')
        .fold(local_app_data.clone(), |path, component| {
            path.join(component)
        });

    assert_eq!(world.startup_installed_exe.as_ref(), Some(&expected));
}

#[then("startup preparation performs these actions in order:")]
fn assert_startup_actions(world: &mut RustShotWorld, step: &Step) {
    let expected: Vec<String> = step
        .table
        .as_ref()
        .expect("this step needs an action table")
        .rows
        .iter()
        .skip(1)
        .map(|row| row[0].clone())
        .collect();

    assert_eq!(world.startup_events, expected);
}

#[then("the startup caller is told to exit")]
fn assert_startup_exit(world: &mut RustShotWorld) {
    let result = world
        .startup_should_exit
        .as_ref()
        .expect("startup preparation did not run");
    assert_eq!(result.as_ref(), Ok(&true));
}

#[then("the current RustShot session remains running")]
fn assert_current_session_continues(world: &mut RustShotWorld) {
    let result = world
        .startup_settings_result
        .as_ref()
        .expect("Settings startup synchronization did not run");
    assert_eq!(result, &Ok(()));
}
