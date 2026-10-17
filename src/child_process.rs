use std::io::{self, Read};
use std::process::Child;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

const RECORDING_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Child processes owned by the tray application.
pub struct ChildProcesses {
    pub selection: Option<Child>,
    pub recording: Option<Child>,
    /// How long a recorder may take to exit once its control pipe is closed.
    recording_shutdown_timeout: Duration,
}

impl Default for ChildProcesses {
    fn default() -> Self {
        Self {
            selection: None,
            recording: None,
            recording_shutdown_timeout: RECORDING_SHUTDOWN_TIMEOUT,
        }
    }
}

impl ChildProcesses {
    pub fn stop_recording(&mut self) -> Result<(), String> {
        let Some(mut child) = self.recording.take() else {
            return Ok(());
        };

        // Closing the control pipe also stops the recorder if the tray crashes.
        drop(child.stdin.take());
        let deadline = Instant::now() + self.recording_shutdown_timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                _ => return terminate_child(&mut child),
            }
        }
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        let selection_result = self
            .selection
            .take()
            .map(|mut child| terminate_child(&mut child))
            .unwrap_or(Ok(()));
        let recording_result = self.stop_recording();
        selection_result.and(recording_result)
    }
}

impl Drop for ChildProcesses {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn child_try_wait_exited(child: &mut Child) -> io::Result<bool> {
    child.try_wait().map(|s| s.is_some())
}

fn terminate_child(child: &mut Child) -> Result<(), String> {
    terminate_child_with(child, Child::kill, child_try_wait_exited, |c| {
        c.wait().map(drop)
    })
}

fn terminate_child_with<T>(
    target: &mut T,
    kill: impl FnOnce(&mut T) -> io::Result<()>,
    try_wait_exited: impl FnOnce(&mut T) -> io::Result<bool>,
    wait: impl FnOnce(&mut T) -> io::Result<()>,
) -> Result<(), String> {
    if let Err(e) = kill(target) {
        return match try_wait_exited(target) {
            Ok(true) => Ok(()),
            _ => Err(format!("Failed to terminate child process: {e}")),
        };
    }
    wait(target).map_err(|e| format!("Failed to reap child process: {e}"))
}

fn finish_listen_spawn<T, E: std::fmt::Display>(
    res: Result<T, E>,
    receiver: Receiver<()>,
) -> Result<RecordingStop, String> {
    match res {
        Ok(_) => Ok(RecordingStop { receiver }),
        Err(e) => Err(format!("Failed to monitor recording control pipe: {e}")),
    }
}

/// Watches the tray-owned pipe while the recording selection overlay is open.
pub struct RecordingStop {
    receiver: Receiver<()>,
}

impl RecordingStop {
    pub fn listen<R: Read + Send + 'static>(reader: R) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        finish_listen_spawn(
            std::thread::Builder::new()
                .name("recording-stop".into())
                .spawn(move || {
                    wait_for_stop_signal(reader);
                    // Disconnection is permanent, including after a caller checks it.
                    drop(sender);
                }),
            receiver,
        )
    }

    pub fn is_stopped(&self) -> bool {
        !matches!(self.receiver.try_recv(), Err(TryRecvError::Empty))
    }

    #[allow(dead_code)]
    pub fn wait(&self) {
        let _ = self.receiver.recv();
    }

    /// Wait until a stop signal is received on the control pipe OR `is_finished`
    /// returns `true` (e.g., if the recording worker thread exited early with an error).
    pub fn wait_until_stopped_or(&self, mut is_finished: impl FnMut() -> bool) {
        loop {
            if is_finished() {
                break;
            }
            match self.receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

fn wait_for_stop_signal(mut reader: impl Read) {
    let mut byte = [0];
    loop {
        match reader.read(&mut byte) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            // EOF, a byte, or a failed control pipe all stop recording.
            _ => return,
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Command, Stdio};
    use std::sync::mpsc::{self, Receiver};
    use std::time::{Duration, Instant};

    const HELPER_MODE: &str = "RUSTSHOT_CHILD_PROCESS_TEST_MODE";
    const READY: &str = "RUSTSHOT_CHILD_READY";
    const STOPPED: &str = "RUSTSHOT_CHILD_STOPPED";

    fn spawn_helper(mode: &str) -> (Child, Receiver<String>) {
        let test_name = format!(
            "{}::subprocess_helper",
            module_path!().split_once("::").unwrap().1
        );
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &test_name, "--nocapture"])
            .env(HELPER_MODE, mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let (ready_tx, ready_rx) = mpsc::channel();
        let (exit_tx, exit_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            while stdout.read_line(&mut line).unwrap() != 0 {
                if line.contains(READY) {
                    let _ = ready_tx.send(());
                    break;
                }
                line.clear();
            }
            let mut output = String::new();
            let _ = stdout.read_to_string(&mut output);
            let _ = exit_tx.send(output);
        });
        if let Err(e) = ready_rx.recv_timeout(Duration::from_secs(5)) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Child helper did not become ready: {e}");
        }
        (child, exit_rx)
    }

    #[test]
    fn subprocess_helper() {
        let Ok(mode) = std::env::var(HELPER_MODE) else {
            return;
        };
        // Keep a failed regression test from leaving a helper behind forever.
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(15));
            std::process::exit(99);
        });
        let stop = (mode == "recording").then(|| RecordingStop::listen(std::io::stdin()).unwrap());
        println!("{READY}");
        std::io::stdout().flush().unwrap();
        if let Some(stop) = stop {
            stop.wait();
            std::thread::sleep(Duration::from_millis(100));
            println!("{STOPPED}");
            std::io::stdout().flush().unwrap();
            std::process::exit(0);
        }
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn dropping_owner_stops_recording_through_eof() {
        let (child, exited) = spawn_helper("recording");
        let children = ChildProcesses {
            selection: None,
            recording: Some(child),
            recording_shutdown_timeout: RECORDING_SHUTDOWN_TIMEOUT,
        };

        let start = Instant::now();
        drop(children);

        assert!(start.elapsed() >= Duration::from_millis(100));
        let output = exited.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(output.contains(STOPPED), "Recording did not finalize");
    }

    #[test]
    fn dropping_owner_terminates_selection_child() {
        let (child, exited) = spawn_helper("selection");
        let children = ChildProcesses {
            selection: Some(child),
            recording: None,
            recording_shutdown_timeout: RECORDING_SHUTDOWN_TIMEOUT,
        };

        drop(children);

        exited
            .recv_timeout(Duration::from_secs(1))
            .expect("Selection child survived its owner");
    }

    #[test]
    fn dropping_owner_forces_recording_to_exit_when_eof_is_ignored() {
        let (child, exited) = spawn_helper("ignore-stop");
        let children = ChildProcesses {
            selection: None,
            recording: Some(child),
            recording_shutdown_timeout: Duration::from_millis(50),
        };

        let start = Instant::now();
        drop(children);

        assert!(start.elapsed() < Duration::from_secs(2));
        exited
            .recv_timeout(Duration::from_secs(1))
            .expect("Recording child ignored EOF and survived its owner");
    }

    #[test]
    fn shutdown_stops_both_children_and_is_idempotent() {
        let (selection, selection_exited) = spawn_helper("selection");
        let (recording, recording_exited) = spawn_helper("recording");
        let mut children = ChildProcesses {
            selection: Some(selection),
            recording: Some(recording),
            recording_shutdown_timeout: RECORDING_SHUTDOWN_TIMEOUT,
        };

        children.shutdown().unwrap();
        children.shutdown().unwrap();

        assert!(children.selection.is_none());
        assert!(children.recording.is_none());
        selection_exited
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        assert!(recording_exited
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .contains(STOPPED));
    }

    #[test]
    fn eof_before_selection_finishes_remains_observable() {
        let stop = RecordingStop::listen(io::empty()).unwrap();
        // The selection can finish after the monitor has received parent EOF.
        stop.wait();

        assert!(stop.is_stopped());
        assert!(stop.is_stopped());
    }

    #[test]
    fn a_byte_on_the_control_pipe_requests_stop() {
        let stop = RecordingStop::listen(io::Cursor::new(vec![1])).unwrap();
        stop.wait();

        assert!(stop.is_stopped());
    }

    #[test]
    fn control_pipe_read_error_requests_stop() {
        struct FailedReader;
        impl Read for FailedReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("control pipe failed"))
            }
        }

        let stop = RecordingStop::listen(FailedReader).unwrap();
        stop.wait();

        assert!(stop.is_stopped());
    }

    #[test]
    fn interrupted_control_pipe_read_is_retried() {
        struct InterruptedReader(usize);
        impl Read for InterruptedReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                self.0 += 1;
                if self.0 == 1 {
                    return Err(io::ErrorKind::Interrupted.into());
                }
                Ok(0)
            }
        }

        let mut reader = InterruptedReader(0);
        wait_for_stop_signal(&mut reader);

        assert_eq!(reader.0, 2);
    }

    #[test]
    fn test_wait_until_stopped_or_exits_early_when_worker_finishes() {
        struct PendingReader;
        impl Read for PendingReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                std::thread::sleep(Duration::from_secs(5));
                Ok(0)
            }
        }

        let stop = RecordingStop::listen(PendingReader).unwrap();
        let mut checks = 0usize;
        let mut is_done = || {
            checks += 1;
            checks == 2
        };
        let start = Instant::now();
        stop.wait_until_stopped_or(&mut is_done);
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(!stop.is_stopped());

        let stopped = RecordingStop::listen(std::io::empty()).unwrap();
        stopped.wait();
        stopped.wait_until_stopped_or(&mut is_done);
        assert!(stopped.is_stopped());
    }

    #[test]
    fn test_child_processes_default_and_disconnected_wait_until_stopped_or() {
        let mut children = ChildProcesses::default();
        assert!(children.selection.is_none());
        assert!(children.recording.is_none());
        assert!(children.stop_recording().is_ok());
        assert!(children.shutdown().is_ok());

        let stop = RecordingStop::listen(std::io::empty()).unwrap();
        stop.wait();
        stop.wait_until_stopped_or(|| false);
        assert!(stop.is_stopped());

        let (_tx1, rx1) = mpsc::channel();
        assert!(finish_listen_spawn::<u32, &str>(Ok(5), rx1).is_ok());
        let (_tx2, rx2) = mpsc::channel();
        assert!(
            finish_listen_spawn::<u32, &str>(Err("os thread limit"), rx2)
                .err()
                .unwrap()
                .contains("Failed to monitor recording control pipe")
        );

        for case in 0..4 {
            let res = terminate_child_with(
                &mut case.clone(),
                |c| {
                    if *c <= 1 {
                        Err(io::Error::other("kill failed"))
                    } else {
                        Ok(())
                    }
                },
                |c| Ok(*c == 0),
                |c| {
                    if *c == 2 {
                        Err(io::Error::other("wait failed"))
                    } else {
                        Ok(())
                    }
                },
            );
            assert_eq!(res.is_ok(), case == 0 || case == 3);
        }

        let mut exited_child = Command::new(std::env::current_exe().unwrap())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let _ = exited_child.wait();
        assert!(child_try_wait_exited(&mut exited_child).unwrap());
        assert!(terminate_child(&mut exited_child).is_ok());
    }
}
