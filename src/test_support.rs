//! Shared test-only support utilities.
//!
//! This module is compiled only for `cargo test` (gated by `#[cfg(test)]`
//! at the `mod` declaration in `lib.rs`).

use std::path::{Path, PathBuf};

/// RAII temporary-directory fixture for unit tests.
///
/// `TestDir::new("prefix")` creates a unique directory
/// `env::temp_dir()/<prefix>_<pid>_<nanos>` and removes it (best-effort,
/// recursively) when the value is dropped. Because cleanup happens in
/// `Drop`, it also runs when a test's assertion panics — unlike the manual
/// trailing `remove_dir_all` teardown it replaces, which was skipped on
/// failure and leaked temp directories.
///
/// The `<pid>_<nanos>` suffix makes the path unique across parallel test
/// threads, concurrent `cargo test` processes, and repeated runs.
pub(crate) struct TestDir {
    path: PathBuf,
}

impl TestDir {
    /// Creates `env::temp_dir()/<prefix>_<pid>_<nanos>` (via `create_dir_all`)
    /// and returns the guard owning it.
    pub(crate) fn new(prefix: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is set before the UNIX epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}_{}_{nanos}", std::process::id()));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|e| panic!("failed to create test dir {}: {e}", path.display()));
        Self { path }
    }

    /// The root of the temporary directory.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// A path inside the directory: `path().join(name)`.
    ///
    /// `name` may contain `/` separators for nested paths. Intermediate
    /// components are NOT created — tests that need a deliberately missing
    /// path rely on that.
    pub(crate) fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        // Best-effort cleanup: a removal failure must never mask the test's
        // own outcome.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
