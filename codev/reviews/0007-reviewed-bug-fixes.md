# Reviewed bug fixes

## Result

All six reported bugs are fixed. Independent specification and code-quality
review approved the changes; a minor premature clipboard-success log was also
corrected before final verification.

## Implementation

- Linux tray startup initializes GTK on the owning main thread. GTK events and
  hotkey/menu channels are serviced alongside winit, with a new 40 ms future
  deadline on each idle iteration. Other platforms keep their existing flow.
- Recording converts the actual captured dimensions into a zero-padded, even
  BGRA frame. Source and destination strides are independent, buffer sizes are
  validated, and the public same-size conversion APIs remain unchanged.
- PNG, JPEG, and BMP encoders borrow the output writer. The save function checks
  its final flush and propagates errors, including disk-full failures.
- Linux selection children send one image through stdout. A reader thread
  validates dimensions and reads pixels; the tray retains the clipboard owner.
  A persistent result channel supports repeated captures without replacing a
  pending receiver. Clean EOF means cancellation. Transfers are limited to
  512 MiB and never store screenshot data in temporary files. Direct selection
  mode keeps serving its clipboard until another application replaces it.
- Linux overlay placement uses the native desktop origin, reversing the display
  metadata's scale normalization. Existing Windows/macOS layout reconstruction
  is unchanged.
- CLI scanning recognizes combined short options and stops scanning values
  consumed by output/screen options. A main-level Clap regression verifies that
  -wfjpg and -wf jpg override a .png output extension.

## Regression evidence

Tests were observed failing before implementation for odd recording dimensions,
buffered write failures, displaced/scaled X11 origins, combined CLI options,
Linux tray startup, and selection image transfer/clipboard retention. Their
final focused runs pass.

Final full suite with standard Rust 1.95.0, normal parallel execution:

- 1,257 library tests pass.
- 305 binary tests pass; one display-dependent test is run separately below.
- 226 integration tests and 12 documentation tests pass.
- 48 BDD scenarios and 114 steps pass.
- cargo fmt --all --check, cargo check --locked, and
  cargo clippy --locked --tests -- -D warnings pass.
- Windows GNU cargo check --locked --target x86_64-pc-windows-gnu --tests passes.
- CI policy checks, shell syntax, and git diff --check pass.

## Linux runtime evidence

The committed tests/linux_desktop_smoke.sh passes against the rebuilt actual
binary under Xvfb. The display-dependent
selection_clipboard::tests::tray_retains_clipboard_between_captures test also
passes under Xvfb.

A separate actual-app integration run used a dedicated 800x600 Xvfb display and
XTest input, without a clipboard manager:

1. PrintScreen and a drag on a red background produced a 100x70 image. A separate
   arboard process read its red pixels after the overlay exited and again later.
2. A second PrintScreen and drag on green replaced it with a persistent 150x90
   green image.
3. A third PrintScreen followed by Escape preserved that image and left the tray
   alive and responsive.

This checks the hotkey wakeup, actual child image pipe, persistent parent
clipboard ownership, repeated captures, and cancellation together. Temporary
AppIndicator runtime libraries were supplied through LD_LIBRARY_PATH; no system
installation or live project settings changed.

## Limits

Windows code and tests cross-compile; live Windows desktop and Media Foundation
execution were unavailable. Native tray-menu interaction and a live multi-monitor
desktop were not exercised; platform/layout regression tests cover placement.
No Git history changes, commits, or pushes were performed for this task.
