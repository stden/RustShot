---
id: entity.screenshots-crate
kind: entity
title: screenshots Crate
summary: Defines where RustShot uses the screenshots crate for monitor enumeration and one-shot full-screen capture.
tags:
  - capture
  - dependency
  - monitors
  - screenshots
source_paths:
  - Cargo.toml
  - src/capture.rs
  - src/selection/desktop.rs
  - src/platform/monitor/gdi.rs
related:
  - concept.dpi-coordinate-systems
  - concept.screen-recording
---
# screenshots Crate

## Role in RustShot

The `screenshots` crate supplies `Screen::all()`, display metadata, and one-shot `Screen::capture()`. RustShot uses
it to list monitors, choose the capture target, save static full-screen images, and obtain monitor geometry for the
selection overlay.

The Windows recording hot loop does not call full-screen `Screen::capture()`. Its primary backend is a persistent
Windows Graphics Capture session, with region-scoped GDI capture as the fallback.

## Contract and constraints

- Display metadata includes desktop origin, size, scale, and primary-monitor status.
- Static captures cross an `image` crate version boundary: `screenshots` 0.8 uses `image` 0.24 while RustShot uses
  0.25, so `src/capture.rs` transfers the raw RGBA buffer.
- Selection deliberately captures only the monitor under the cursor.
- The GDI fallback uses the crate's region capture API rather than capturing and cropping the whole screen.

## Grounding

- [Dependency declaration and version rationale](../../Cargo.toml)
- [Monitor enumeration and static capture](../../src/capture.rs)
- [Cursor-monitor selection](../../src/selection/desktop.rs)
- [Region-scoped GDI fallback](../../src/platform/monitor/gdi.rs)

## Related

- [DPI and Coordinate Systems](dpi-coordinate-systems.md)
- [Screen Recording](screen-recording.md)
- [screenshots on crates.io](https://crates.io/crates/screenshots)
