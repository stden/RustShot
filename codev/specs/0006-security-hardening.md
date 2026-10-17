# Security hardening

## Scope

Implement the four findings from the security review: shared writable Cargo
configuration across CI trust levels, mutable CI image poisoning, Windows
folder-opener executable planting, and recording after tray shutdown.

## Required behavior

- Rust CI jobs use a fresh Cargo home and do not consume writable shared build
  state from other jobs. Their base image is an independently verified public
  image pinned by digest, rather than a tag writable by branch pipelines.
- Optional custom-image publishing runs only for the protected default branch;
  ordinary test jobs do not trust its mutable tag. Document the remaining host
  runner and registry administration requirements.
- Opening the screenshots folder on Windows uses a native shell API without
  searching for an executable next to RustShot. Preserve non-ASCII paths and
  report failures through the existing logging convention.
- Exiting tray mode stops and reaps recording and selection children. A recording
  also stops when its parent disappears. A selection completed after parent
  termination must not start a new recording.
- Graceful recording stop allows the encoder to finalize its output. Cleanup has
  a bounded wait followed by forced termination if a child does not cooperate.

## Design

Use job-local Cargo storage and an immutable upstream image for CI isolation.
Use Windows ShellExecuteW for opening a directory. Replace recording stop files
with an inherited stdin pipe: closing the parent writer signals normal stop,
and EOF also covers parent death. Keep child ownership and shutdown logic small
and independently testable without constructing tray windows.

## Validation

Add regressions before implementation for CI isolation policies, shell API
result handling, and real child-process shutdown. Run focused tests followed by
formatting, compilation, Clippy, and the complete test suite. The baseline has a
parallel settings-test override race; a serial full run distinguishes that
pre-existing issue. Check Windows compilation when a target toolchain is
available; disclose any Windows GUI and live CI validation limits.

Dependency advisory matches without an established exploit through current
application paths are outside these four fixes.
