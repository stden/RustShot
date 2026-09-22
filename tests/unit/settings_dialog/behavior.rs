use super::*;
use cucumber::{given, then, when, World};

#[derive(Debug, Default, World)]
struct DialogWorld {
    hwnd: Option<HWND>,
    input: Option<Result<crate::settings::Settings, String>>,
    previous_width: Option<i32>,
    directory: Option<crate::test_support::TestDir>,
    captured: Option<std::path::PathBuf>,
}

impl Drop for DialogWorld {
    fn drop(&mut self) {
        if let Some(hwnd) = self.hwnd.take() {
            // Close only this scenario's window on its owning thread.
            let _ = close_settings_window(hwnd);
        }
    }
}

#[given("the native Windows settings dialog is open")]
fn open_dialog(world: &mut DialogWorld) {
    show_settings_dialog();
    world.hwnd = Some(wait_for_settings_window().expect("settings window must appear"));
}

#[then("every interactive setting is reachable in the expected Tab order")]
fn assert_tab_order(world: &mut DialogWorld) {
    let mut previous = None;
    for id in [
        IDC_SCREENSHOTS_EDIT,
        IDC_SCREENSHOTS_BROWSE,
        IDC_RECORDINGS_EDIT,
        IDC_RECORDINGS_BROWSE,
        IDC_FORMAT_COMBO,
        IDC_FPS_EDIT,
        IDC_AUTOSTART_CHECK,
        IDC_SAVE_BTN,
        IDC_CANCEL_BTN,
    ] {
        // SAFETY: this scenario keeps the parent and queried children alive.
        let next = unsafe { GetNextDlgTabItem(world.hwnd.unwrap(), previous, false) }.unwrap();
        // SAFETY: Windows returned next as a live child of our dialog.
        assert_eq!(unsafe { GetDlgCtrlID(next) }, id);
        previous = Some(next);
    }
}

#[when("Escape is sent to the settings dialog")]
fn press_escape(world: &mut DialogWorld) {
    // SAFETY: a scalar key message is posted to our window's owning thread.
    unsafe { PostMessageW(world.hwnd, WM_KEYDOWN, WPARAM(0x1b), LPARAM(1)) }.unwrap();
}

#[then("the settings dialog closes and can be opened again")]
fn assert_closed(world: &mut DialogWorld) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while DIALOG_OPEN.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        !DIALOG_OPEN.load(Ordering::SeqCst),
        "Escape did not close settings"
    );
    world.hwnd = None;
    open_dialog(world);
}

#[when("settings is requested a second time")]
fn reopen_dialog(_world: &mut DialogWorld) {
    show_settings_dialog();
}

#[then("the existing settings window is reused")]
fn assert_single_window(world: &mut DialogWorld) {
    assert_eq!(wait_for_settings_window(), world.hwnd);
}

#[when("the settings window is widened")]
fn widen_dialog(world: &mut DialogWorld) {
    let hwnd = world.hwnd.unwrap();
    world.previous_width = control_width(hwnd, IDC_SCREENSHOTS_EDIT);
    // SAFETY: resizing our live window carries no borrowed pointers.
    unsafe { SetWindowPos(hwnd, None, 0, 0, 1100, 420, SWP_NOMOVE | SWP_NOZORDER) }.unwrap();
}

#[then("the screenshot folder field expands")]
fn assert_expanded(world: &mut DialogWorld) {
    assert!(
        control_width(world.hwnd.unwrap(), IDC_SCREENSHOTS_EDIT).unwrap()
            > world.previous_width.unwrap()
    );
}

#[when("the visible settings window is captured through the MCP window tool")]
fn capture_owned_window(world: &mut DialogWorld) {
    let id = world.hwnd.unwrap().0 as u64;
    assert!(crate::window::list_windows()
        .iter()
        .any(|window| window.id == id));
    let directory = crate::test_support::TestDir::new("rustshot_bdd_window_capture");
    let path = directory.file("settings.png");
    let request = serde_json::json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
        "name":"capture_window", "arguments":{"id":id, "format":"png", "output_path":path}
    }});
    let reply = crate::mcp::handle_message(&request.to_string()).unwrap();
    let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(reply["result"]["content"][1]["mimeType"], "image/png");
    world.directory = Some(directory);
    world.captured = Some(path);
}

#[then("the selected window is saved as a non-empty PNG")]
fn assert_window_capture(world: &mut DialogWorld) {
    let bytes = std::fs::read(world.captured.as_ref().unwrap()).unwrap();
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Png
    );
    let captured = image::load_from_memory(&bytes).unwrap();
    assert!(captured.width() > 100 && captured.height() > 100);
}

#[when("the settings form contains")]
fn form_inputs(world: &mut DialogWorld, step: &cucumber::gherkin::Step) {
    let rows = &step.table.as_ref().unwrap().rows;
    assert_eq!(rows.len(), 2);
    let row = &rows[1];
    assert_eq!(row.len(), 5);
    let format = match row[2].as_str() {
        "PNG" => 0,
        "JPG" => 1,
        "BMP" => 2,
        other => panic!("unknown settings format: {other}"),
    };
    world.input = Some(build_settings_from_inputs(
        &row[0],
        &row[1],
        format,
        &row[3],
        row[4].parse().unwrap(),
    ));
}

#[then(regex = r#"^the settings form rejects them with "([^"]+)"$"#)]
fn assert_form_error(world: &mut DialogWorld, expected: String) {
    assert_eq!(
        world.input.as_ref().unwrap().as_ref().unwrap_err(),
        &expected
    );
}

#[then("the form preserves the folders, JPG format, 60 FPS, and disabled auto-start")]
fn assert_form_values(world: &mut DialogWorld) {
    let actual = world.input.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(actual.screenshots_dir, PathBuf::from("screenshots"));
    assert_eq!(actual.recordings_dir, PathBuf::from("recordings"));
    assert_eq!(actual.default_format, Format::Jpg);
    assert_eq!(actual.recording_fps.get(), 60);
    assert!(!actual.auto_start);
}

#[test]
fn bdd_windows_settings_interactions() {
    let _lock = lock_dialog_flag();
    let _close = CloseSettingsWindowOnDrop;
    futures::executor::block_on(
        DialogWorld::cucumber()
            .with_default_cli()
            .max_concurrent_scenarios(1)
            .fail_on_skipped()
            .run_and_exit("tests/features/in_process/windows_settings.feature"),
    );
}
