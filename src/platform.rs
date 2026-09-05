//! Native desktop adapters shared by screenshot, selection, and recording workflows.
//!
//! These adapters depend on domain values and pixel transforms. They do not call
//! application workflows, so monitor and window capture can be reused independently.

#[cfg(windows)]
pub(crate) mod autorun;
mod cursor;
#[cfg(windows)]
pub(crate) mod monitor;
pub(crate) mod window;
#[cfg(any(windows, test))]
pub(crate) mod worker;

pub use cursor::get_cursor_position;

/// Keep Win32 string conversion at the native boundary.
#[cfg(any(test, windows))]
pub(crate) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Pin the process-wide COM multithreaded apartment for the lifetime of the
/// process.
///
/// The WGC backends initialize WinRT on short-lived worker threads and
/// uninitialize it when the thread exits. If that `RoUninitialize` closes the
/// LAST thread in the MTA, the apartment is destroyed - and COM objects still
/// cached elsewhere in the process (DXGI factories, the `screenshots` crate's
/// internals, frame-pool callbacks) become dangling, so the next capture or
/// the process teardown dies with STATUS_ACCESS_VIOLATION. This was observed
/// as: CI test binaries crashing on the capture test AFTER a window-capture
/// test passed, criterion crashing at exit, and the app crashing after
/// --window-id captures.
///
/// `CoIncrementMTAUsage` is the documented fix: it keeps the MTA alive without
/// dedicating a thread. The cookie is deliberately leaked - the apartment must
/// live until process exit, where the OS reclaims everything.
#[cfg(windows)]
pub(crate) fn pin_process_mta() {
    use std::sync::OnceLock;
    static MTA_PIN: OnceLock<()> = OnceLock::new();
    MTA_PIN.get_or_init(|| {
        // Failure (ancient Windows) degrades to the old per-thread behavior.
        // SAFETY: `CoIncrementMTAUsage` takes no arguments and no pointers we
        // own, and is documented as callable from any thread at any time -
        // including before this thread has entered an apartment. The returned
        // cookie is dropped without calling `CoDecrementMTAUsage`; that is the
        // intended leak, and it is a no-op drop because windows-rs models the
        // cookie as a bare `#[repr(transparent)]` handle with no `Drop` impl.
        let _ = unsafe { windows::Win32::System::Com::CoIncrementMTAUsage() };
    });
}
