use super::*;
use crate::test_support::TestDir;
use cucumber::{then, when, World};

#[derive(Debug, Default, World)]
struct TrayWorld {
    events: Vec<String>,
    exit: Option<Result<(), String>>,
    action: Option<Option<tray::MenuAction>>,
    registration_error: Option<String>,
}

#[when(
    regex = r#"^tray exit is requested with recording "(active|absent)" and "(no failure|stop failure|selection failure)"$"#
)]
fn request_exit(world: &mut TrayWorld, recording: String, failure: String) {
    let events = RefCell::new(Vec::new());
    world.exit = Some(prepare_tray_exit_with(
        (recording == "active").then_some(Path::new("recording.stop")),
        |_| {
            events.borrow_mut().push("stop recording".to_string());
            if failure == "stop failure" {
                Err("stop marker denied".into())
            } else {
                Ok(())
            }
        },
        || {
            events.borrow_mut().push("close selection".to_string());
            if failure == "selection failure" {
                Err("selection could not close".into())
            } else {
                Ok(())
            }
        },
    ));
    world.events = events.into_inner();
}

#[then(regex = r#"^tray operations are "([^"]+)"$"#)]
fn assert_operations(world: &mut TrayWorld, expected: String) {
    assert_eq!(world.events, expected.split(',').collect::<Vec<_>>());
}

#[then(regex = r#"^tray exit reports "([^"]+)"$"#)]
fn assert_exit(world: &mut TrayWorld, expected: String) {
    let actual = world
        .exit
        .as_ref()
        .unwrap()
        .as_ref()
        .err()
        .map_or("ready", String::as_str);
    assert_eq!(actual, expected);
}

#[when(regex = r#"^the screenshots folder is opened after directory creation "(succeeds|fails)"$"#)]
fn open_folder(world: &mut TrayWorld, outcome: String) {
    let directory = TestDir::new("rustshot_bdd_folder");
    let target = directory.file("screenshots");
    let events = RefCell::new(Vec::new());
    open_screenshots_folder_at_with_handlers(
        &target,
        |path| {
            events.borrow_mut().push("prepare folder".to_string());
            if outcome == "fails" {
                return Err("permission denied".into());
            }
            std::fs::create_dir_all(path).map_err(|error| error.to_string())
        },
        |path| {
            assert_eq!(path, target);
            assert!(
                path.is_dir(),
                "the file manager must receive an existing directory"
            );
            events.borrow_mut().push("open folder".to_string());
        },
    );
    world.events = events.into_inner();
}

#[when(
    regex = r#"^registered shortcut "(PrintScreen|Alt\+PrintScreen|Ctrl\+Shift\+R)" is "(pressed|released)"$"#
)]
fn shortcut_event(world: &mut TrayWorld, label: String, state: String) {
    let registered = RegisteredHotkeys::register_with(|_| Ok::<(), String>(())).unwrap();
    let shortcut = tray::MenuAction::ALL
        .iter()
        .filter_map(|action| action.hotkey())
        .find(|shortcut| shortcut.label == label)
        .unwrap();
    world.action = Some(registered.action_for(GlobalHotKeyEvent {
        id: native_hotkey(shortcut).id(),
        state: if state == "pressed" {
            HotKeyState::Pressed
        } else {
            HotKeyState::Released
        },
    }));
}

#[then(regex = r#"^the shortcut dispatches "(capture|capture_window|start_record|nothing)"$"#)]
fn assert_shortcut(world: &mut TrayWorld, expected: String) {
    assert_eq!(
        world
            .action
            .unwrap()
            .map_or("nothing", tray::MenuAction::id),
        expected
    );
}

#[when("Alt+PrintScreen registration fails because it is already in use")]
fn conflicting_shortcut(world: &mut TrayWorld) {
    let mut attempts = 0;
    world.registration_error = RegisteredHotkeys::register_with(|_| {
        attempts += 1;
        if attempts == 2 {
            Err("already in use")
        } else {
            Ok(())
        }
    })
    .err();
    assert_eq!(attempts, 2, "registration must stop at the failed shortcut");
}

#[then("the registration error identifies the conflicting shortcut")]
fn assert_shortcut_error(world: &mut TrayWorld) {
    assert_eq!(
        world.registration_error.as_deref(),
        Some("Failed to register Alt+PrintScreen hotkey: already in use")
    );
}

#[test]
fn bdd_tray_commands_and_shutdown() {
    futures::executor::block_on(
        TrayWorld::cucumber()
            .with_default_cli()
            .fail_on_skipped()
            .run_and_exit("tests/features/in_process/tray_commands.feature"),
    );
}
