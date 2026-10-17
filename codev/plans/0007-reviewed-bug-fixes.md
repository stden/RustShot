# Reviewed bug fixes implementation plan

**Goal:** Fix all six reported bugs with focused regressions.

**Architecture:** Keep format, image, and layout fixes local. Add a Linux GTK
helper and transfer child selection images to a persistent tray clipboard owner.

**Tech stack:** Rust, existing image/Clap/arboard/winit APIs, Linux GTK 0.18,
standard-library pipes and channels, Xvfb runtime checks.

## Tasks

- [x] Image worker: reproduce odd recording dimensions and final buffered write
  errors in tests; implement padded conversion and checked flushes in record.rs
  and capture.rs; rerun focused tests.
- [x] Selection/CLI worker: add nonzero X11 origin and clustered-format
  regressions; implement platform-appropriate placement and short-option scanning;
  retain Windows DPI behavior and existing CLI precedence tests.
- [x] GTK worker: add the Linux-only GTK dependency, implement initialization and
  event servicing, and provide a headless startup regression.
- [x] Integrator: add image-pipe tests first, implement bounded selection transfer,
  and retain clipboard ownership in the tray. Handle cancellation and failures
  without blocking the tray or losing child ownership.
- [x] Integrator: wire Linux GTK servicing and the persistent clipboard into the
  existing main event loop; preserve non-Linux behavior.
- [x] Independent review: check specification coverage, then quality and error
  handling; resolve substantive findings.
- [x] Verify focused tests, Xvfb tray startup and clipboard retention, formatting,
  cargo check, Clippy, full cargo test, and Windows GNU test compilation. Record
  exact evidence and any runtime limitations in the Codev review.
