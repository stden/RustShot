//! Gherkin persistence/editor scenarios use the existing private I/O overrides.
use super::*;
use crate::test_support::TestDir;
use cucumber::{given, then, when, World};

static OPENED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

#[derive(Debug, Default, World)]
struct SettingsWorld {
    directory: Option<TestDir>,
    expected: Option<Settings>,
    loaded: Option<Settings>,
    saved: Option<Result<(), String>>,
    original: Option<String>,
}

impl Drop for SettingsWorld {
    fn drop(&mut self) {
        set_settings_path_for_tests(None);
        set_open_file_in_editor_for_tests(None);
    }
}

#[given("an isolated settings file and a recording editor launcher")]
fn isolated_settings(world: &mut SettingsWorld) {
    let directory = TestDir::new("rustshot_bdd_settings");
    set_settings_path_for_tests(Some(directory.file("config/settings.json")));
    OPENED.lock().unwrap().clear();
    set_open_file_in_editor_for_tests(Some(|path| OPENED.lock().unwrap().push(path.to_owned())));
    world.directory = Some(directory);
}

#[given("a previous settings file exists")]
fn previous_settings(_world: &mut SettingsWorld) {
    save_settings(&Settings::default()).unwrap();
}

#[given("the settings parent is blocked by an existing file")]
fn blocked_parent(world: &mut SettingsWorld) {
    std::fs::write(world.directory.as_ref().unwrap().file("config"), b"keep me").unwrap();
}

#[given("a directory occupies the settings filename")]
fn occupied_settings(_world: &mut SettingsWorld) {
    std::fs::create_dir_all(settings_path()).unwrap();
}

#[given("the existing settings file contains malformed hand edits")]
fn malformed_settings(world: &mut SettingsWorld) {
    let content = "{ hand-edited JSON waiting for repair";
    std::fs::create_dir_all(settings_path().parent().unwrap()).unwrap();
    std::fs::write(settings_path(), content).unwrap();
    world.original = Some(content.to_string());
}

#[when("custom folders, JPG, 60 FPS, and disabled auto-start are saved")]
fn save_custom_settings(world: &mut SettingsWorld) {
    let directory = world.directory.as_ref().unwrap();
    let expected = Settings {
        screenshots_dir: directory.file("写真"),
        recordings_dir: directory.file("video clips"),
        default_format: Format::Jpg,
        recording_fps: Fps::new(60).unwrap(),
        auto_start: false,
    };
    world.saved = Some(save_settings(&expected));
    world.expected = Some(expected);
}

#[when("settings are loaded from disk")]
fn load_from_disk(world: &mut SettingsWorld) {
    world.loaded = Some(load_settings());
}

#[when("the settings editor is requested")]
fn request_editor(_world: &mut SettingsWorld) {
    open_settings_dialog();
}

#[then("the default settings are returned without creating a file")]
fn assert_missing_defaults(world: &mut SettingsWorld) {
    assert_eq!(world.loaded.as_ref(), Some(&Settings::default()));
    assert!(!settings_path().exists());
}

#[then("all saved settings are loaded and the file is valid JSON")]
fn assert_disk_roundtrip(world: &mut SettingsWorld) {
    assert_eq!(world.saved.as_ref().unwrap(), &Ok(()));
    assert_eq!(world.loaded, world.expected);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(settings_path()).unwrap()).unwrap();
    assert_eq!(json["default_format"], "jpg");
    assert_eq!(json["recording_fps"], 60);
    assert_eq!(json["auto_start"], false);
}

#[then("no settings scratch file remains")]
fn assert_no_scratch(_world: &mut SettingsWorld) {
    for entry in std::fs::read_dir(settings_path().parent().unwrap()).unwrap() {
        assert!(!entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp"));
    }
}

#[then(regex = r#"^saving settings fails mentioning "([^"]+)"$"#)]
fn assert_save_failure(world: &mut SettingsWorld, expected: String) {
    let error = world.saved.as_ref().unwrap().as_ref().unwrap_err();
    assert!(error.contains(&expected), "{error}");
}

#[then("the blocked parent file is preserved")]
fn assert_blocker(world: &mut SettingsWorld) {
    assert_eq!(
        std::fs::read(world.directory.as_ref().unwrap().file("config")).unwrap(),
        b"keep me"
    );
}

#[then("the editor receives the isolated settings path once")]
fn assert_editor_opened(_world: &mut SettingsWorld) {
    assert_eq!(*OPENED.lock().unwrap(), [settings_path()]);
}

#[then("the editor is not launched")]
fn assert_editor_not_opened(_world: &mut SettingsWorld) {
    assert!(OPENED.lock().unwrap().is_empty());
}

#[then("a default settings file is ready for editing")]
fn assert_editor_defaults(_world: &mut SettingsWorld) {
    assert!(settings_path().is_file());
    assert_eq!(load_settings(), Settings::default());
}

#[then("the hand-edited settings bytes are unchanged")]
fn assert_manual_edits(world: &mut SettingsWorld) {
    assert_eq!(
        std::fs::read_to_string(settings_path()).unwrap(),
        *world.original.as_ref().unwrap()
    );
}

#[test]
fn bdd_settings_file_workflows() {
    let _lock = lock_global_overrides();
    futures::executor::block_on(
        SettingsWorld::cucumber()
            .with_default_cli()
            .max_concurrent_scenarios(1)
            .fail_on_skipped()
            .run_and_exit("tests/features/in_process/settings_files.feature"),
    );
}
