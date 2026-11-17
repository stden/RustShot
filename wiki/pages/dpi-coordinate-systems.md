---
id: concept.dpi-coordinate-systems
kind: concept
title: DPI and Coordinate Systems
summary: Explains how RustShot converts logical monitor geometry into physical capture coordinates under mixed DPI.
tags:
  - coordinates
  - dpi
  - multi-monitor
  - selection
source_paths:
  - src/selection.rs
  - src/capture.rs
related:
  - concept.screen-recording
  - entity.screenshots-crate
  - source.pixels-lie
---
# DPI and Coordinate Systems

## Answer

Desktop APIs can report monitor positions in logical, DPI-scaled coordinates while captured frames use physical pixels. RustShot builds an explicit physical layout instead of applying one global scale factor. The primary monitor anchors physical origin `(0, 0)`; neighbouring monitors are placed iteratively and logical gaps are scaled using the relevant monitor geometry.

Selection covers only the monitor under the cursor. On Windows, the cursor monitor is captured once via Win32 GDI; the overlay precomputes a cached dimmed `0x00RRGGBB` background buffer and restores only the previous selection's dirty rectangle during drag events, avoiding per-frame full-screen pixel conversions. A recording selection must fit inside one physical monitor because RustShot has no cross-monitor video compositing path.

## Grounding

- [Physical layout reconstruction, cursor-monitor capture, and overlay placement](../../src/selection.rs)
- [Monitor enumeration and screenshot capture](../../src/capture.rs)

## Constraints

- Monitor bounds are half-open when locating a point.
- Negative desktop coordinates are valid.
- Overlay targeting prefers an exact origin-and-size match, then an origin match when DPI virtualization changes the reported size.
- Recording coordinates are translated from desktop space into the selected monitor's local physical space.

## Related

- [Screen Recording](screen-recording.md)
- [screenshots Crate](entity-screenshots-crate.md)
- [Why Pixels Lie](source-pixels-lie.md)
