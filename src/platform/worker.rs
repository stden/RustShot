//! Reusable capture-worker lifecycle and cache management.

use std::sync::mpsc;

/// Fetch a cached worker handle, starting one on first use and again after a
/// failed start.
///
/// The slot is a `Mutex<Option<_>>` rather than a `OnceLock<Result<_, _>>`
/// precisely so that a failure is *not* remembered: with a `OnceLock`, one
/// transient "failed to start thread" would be cached and every later call would
/// replay that same error, disabling the worker for the rest of the process
/// lifetime. Here the next caller simply tries again.
///
/// A poisoned lock is recovered rather than propagated. The guarded value is
/// just a channel handle, so a panic elsewhere leaves nothing inconsistent, and
/// refusing to hand it out would turn one unrelated panic into a permanently
/// broken worker — the very failure mode this function exists to avoid.
pub(crate) fn cached_worker<T, Start>(
    slot: &std::sync::Mutex<Option<T>>,
    start: Start,
) -> Result<T, String>
where
    T: Clone,
    Start: FnOnce() -> Result<T, String>,
{
    let mut cached = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(handle) = cached.as_ref() {
        return Ok(handle.clone());
    }

    let handle = start()?;
    *cached = Some(handle.clone());
    Ok(handle)
}

/// Drop the cached worker handle so the next [`cached_worker`] call starts a new
/// one. Called when the worker is found to have died.
pub(crate) fn forget_cached_worker<T>(slot: &std::sync::Mutex<Option<T>>) {
    *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

/// What a capture worker should do next.
pub(crate) enum WorkerStep<T> {
    /// Serve this request.
    Request(T),
    /// Nothing arrived within the idle timeout — a good moment to release
    /// whatever the worker was holding on to between requests.
    Idle,
    /// Every sender is gone; the worker has nothing left to serve.
    Stop,
}

/// Wait for the next worker request, reporting a quiet spell as [`WorkerStep::Idle`]
/// rather than as shutdown.
///
/// Separated from the worker loop so the distinction is unit-tested: a worker
/// that treated the timeout as "stop" would exit after the first idle period,
/// and the handle cached by [`cached_worker`] would then hand every later caller
/// a dead channel.
pub(crate) fn next_worker_step<T>(
    receiver: &mpsc::Receiver<T>,
    idle_timeout: std::time::Duration,
) -> WorkerStep<T> {
    match receiver.recv_timeout(idle_timeout) {
        Ok(request) => WorkerStep::Request(request),
        Err(mpsc::RecvTimeoutError::Timeout) => WorkerStep::Idle,
        Err(mpsc::RecvTimeoutError::Disconnected) => WorkerStep::Stop,
    }
}
