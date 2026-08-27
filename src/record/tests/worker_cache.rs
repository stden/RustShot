//! Caching behaviour of the shared capture-worker handle.
//!
//! The point of these tests is the *negative* one: a start-up failure must not
//! be remembered. The `OnceLock<Result<_, _>>` this replaced cached the error,
//! so a single transient "failed to start thread" disabled monitor capture for
//! the rest of the process lifetime.

use crate::record::{cached_worker, forget_cached_worker, next_worker_step, WorkerStep};
use std::cell::Cell;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

#[test]
fn worker_is_started_once_and_then_reused() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);
    let starts = Cell::new(0);
    let start = || {
        starts.set(starts.get() + 1);
        Ok(7u32)
    };

    assert_eq!(cached_worker(&slot, start).unwrap(), 7);
    assert_eq!(cached_worker(&slot, start).unwrap(), 7);
    assert_eq!(cached_worker(&slot, start).unwrap(), 7);

    assert_eq!(starts.get(), 1, "the worker was started more than once");
}

#[test]
fn a_failed_start_is_not_remembered() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);

    let error = cached_worker(&slot, || Err("no threads available".to_string())).unwrap_err();
    assert_eq!(error, "no threads available");

    // The next caller must get a working handle rather than a replay of the
    // earlier failure.
    assert_eq!(cached_worker(&slot, || Ok(3u32)).unwrap(), 3);
    assert_eq!(cached_worker(&slot, || Ok(99u32)).unwrap(), 3, "not cached");
}

#[test]
fn forgetting_the_worker_forces_a_restart() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);

    assert_eq!(cached_worker(&slot, || Ok(1u32)).unwrap(), 1);
    forget_cached_worker(&slot);

    // A worker found dead is forgotten, so the next screenshot starts a fresh
    // one instead of every later capture failing against a dead channel.
    assert_eq!(cached_worker(&slot, || Ok(2u32)).unwrap(), 2);
}

#[test]
fn a_queued_request_is_returned_before_the_idle_timeout() {
    let (sender, receiver) = mpsc::channel();
    sender.send(42u8).unwrap();

    assert!(matches!(
        next_worker_step(&receiver, Duration::from_secs(5)),
        WorkerStep::Request(42)
    ));
}

#[test]
fn an_idle_wait_does_not_stop_the_worker() {
    let (_sender, receiver) = mpsc::channel::<u8>();

    // Reporting idleness rather than shutdown is the whole point: a worker that
    // exited after a quiet spell would leave the cached sender pointing at a
    // dead channel, and every later capture would fail against it.
    assert!(matches!(
        next_worker_step(&receiver, Duration::from_millis(1)),
        WorkerStep::Idle
    ));
}

#[test]
fn a_dropped_sender_stops_the_worker() {
    let (sender, receiver) = mpsc::channel::<u8>();
    drop(sender);

    assert!(matches!(
        next_worker_step(&receiver, Duration::from_secs(5)),
        WorkerStep::Stop
    ));
}

#[test]
fn a_poisoned_slot_still_hands_out_the_worker() {
    let slot: Mutex<Option<u32>> = Mutex::new(None);
    assert_eq!(cached_worker(&slot, || Ok(5u32)).unwrap(), 5);

    // Poison the lock the way a panic in an unrelated caller would.
    let _ = std::panic::catch_unwind(|| {
        let _guard = slot.lock().unwrap();
        panic!("unrelated panic");
    });

    // Propagating the poison would turn one unrelated panic into a permanently
    // broken worker, which is the failure mode this design exists to avoid.
    assert_eq!(cached_worker(&slot, || Ok(6u32)).unwrap(), 5);
    forget_cached_worker(&slot);
    assert_eq!(cached_worker(&slot, || Ok(6u32)).unwrap(), 6);
}
