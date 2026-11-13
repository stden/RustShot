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
  - src/selection.rs
  - src/record.rs
related:
  - concept.dpi-coordinate-systems
  - concept.screen-recording
---
# screenshots Crate

## Role in RustShot

The `screenshots` crate supplies `Screen::all()`, display metadata, and one-shot `Screen::capture()`. RustShot uses it to list monitors, choose the capture target, save static full-screen images, and obtain monitor geometry for the selection overlay and recording target resolution.

On Windows, interactive selection and the recording hot loop use direct Win32 GDI `BitBlt(..., SRCCOPY)` + `GetDIBits` for fast zero-stretch capture, keeping `screenshots::Screen` as a cross-platform fallback.

## Contract and constraints

- Display metadata includes desktop origin, size, scale, and primary-monitor status.
- Static captures cross an `image` crate version boundary: `screenshots` 0.8 uses `image` 0.24 while RustShot uses 0.25, so `src/capture.rs` transfers the raw RGBA buffer.
- Selection deliberately captures only the monitor under the cursor.

## Grounding

- [Dependency declaration](../../Cargo.toml)
- [Monitor enumeration and static capture](../../src/capture.rs)
- [Cursor-monitor selection](../../src/selection.rs)
- [Recording screen lookup and fallback capture](../../src/record.rs)

## Related

- [DPI and Coordinate Systems](dpi-coordinate-systems.md)
- [Screen Recording](screen-recording.md)
- [screenshots on crates.io](https://crates.io/crates/screenshots)
