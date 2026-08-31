//! Executable-level startup diagnostics that must finish without opening tray mode.

use crate::RustShotWorld;
use cucumber::{then, when};

fn process_output(world: &RustShotWorld) -> &std::process::Output {
    world
        .executable_process
        .as_ref()
        .expect("the RustShot executable was not launched")
}

fn combined_output(process: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&process.stdout),
        String::from_utf8_lossy(&process.stderr)
    )
}

#[when(regex = r#"^the RustShot executable is launched with "([^"]+)"$"#)]
fn launch_executable(world: &mut RustShotWorld, argument: String) {
    world.executable_process = Some(
        std::process::Command::new(env!("CARGO_BIN_EXE_RustShot"))
            .arg(argument)
            .output()
            .expect("failed to launch the RustShot executable"),
    );
}

#[then("the RustShot executable exits successfully")]
fn assert_executable_succeeds(world: &mut RustShotWorld) {
    let process = process_output(world);
    assert!(
        process.status.success(),
        "RustShot failed with {}: {}",
        process.status,
        combined_output(process)
    );
}

#[then("the RustShot executable reports a command-line error")]
fn assert_executable_fails(world: &mut RustShotWorld) {
    let process = process_output(world);
    assert_eq!(
        process.status.code(),
        Some(2),
        "expected clap's command-line error exit code, got {}: {}",
        process.status,
        combined_output(process)
    );
}

#[then("the RustShot executable reports its package version")]
fn assert_package_version(world: &mut RustShotWorld) {
    let process = process_output(world);
    let output = combined_output(process);
    assert_eq!(
        output.trim(),
        format!("rustshot {}", env!("CARGO_PKG_VERSION"))
    );
}

#[then(regex = r#"^the RustShot process output contains "([^"]+)"$"#)]
fn assert_process_output_contains(world: &mut RustShotWorld, expected: String) {
    let process = process_output(world);
    let output = combined_output(process);
    assert!(
        output.contains(&expected),
        "process output {output:?} does not contain {expected:?}"
    );
}
