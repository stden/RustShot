use crate::record::*;
use cucumber::{given, then, when, World};

#[derive(Default, World)]
struct RecordingWorld {
    session: Option<RecordingSession>,
    finished: std::sync::Arc<std::sync::atomic::AtomicBool>,
    result: Option<Result<PathBuf, String>>,
}

impl std::fmt::Debug for RecordingWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordingWorld")
            .field("result", &self.result)
            .finish()
    }
}

#[given(regex = r#"^a controlled recording worker will "(finalize|fail|panic)" when stopped$"#)]
fn recording_worker(world: &mut RecordingWorld, outcome: String) {
    let (stop_tx, stop_rx) = mpsc::sync_channel(1);
    let finished = std::sync::Arc::clone(&world.finished);
    let path = PathBuf::from("recordings/session.mp4");
    let output = path.clone();
    let join = std::thread::spawn(move || {
        stop_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("stop must reach the worker");
        finished.store(true, std::sync::atomic::Ordering::Release);
        match outcome.as_str() {
            "finalize" => Ok(output),
            "fail" => Err("encoder failed".into()),
            _ => panic!("simulated recording worker panic"),
        }
    });
    world.session = Some(RecordingSession {
        stop_tx,
        join: Some(join),
        output_path: path,
    });
    assert!(!world.session.as_ref().unwrap().is_finished());
}

#[when("the recording session is stopped")]
fn stop_recording(world: &mut RecordingWorld) {
    assert_eq!(
        world.session.as_ref().unwrap().output_path(),
        &PathBuf::from("recordings/session.mp4")
    );
    world.result = Some(world.session.take().unwrap().stop());
}

#[when("the recording session owner is dropped")]
fn drop_recording(world: &mut RecordingWorld) {
    drop(world.session.take());
}

#[then("recording shutdown waits for the worker to finish")]
fn assert_finished(world: &mut RecordingWorld) {
    assert!(world.finished.load(std::sync::atomic::Ordering::Acquire));
}

#[then(regex = r#"^recording completion reports "([^"]+)"$"#)]
fn assert_recording_result(world: &mut RecordingWorld, expected: String) {
    let actual = match world.result.as_ref().unwrap() {
        Ok(path) => path.display().to_string(),
        Err(error) => error.clone(),
    };
    assert_eq!(actual, expected);
}

#[test]
fn bdd_recording_session_lifecycle() {
    futures::executor::block_on(
        RecordingWorld::cucumber()
            .with_default_cli()
            .fail_on_skipped()
            .run_and_exit("tests/features/in_process/recording_lifecycle.feature"),
    );
}
