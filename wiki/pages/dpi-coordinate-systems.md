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
  - src/domain/layout.rs
  - src/selection/desktop.rs
  - tests/features/multi_monitor_layout.feature
  - tests/features/overlay_targeting.feature
  - tests/features/recording_selection.feature
related:
  - concept.screen-recording
  - entity.screenshots-crate
  - source.pixels-lie
---
# DPI and Coordinate Systems

## Answer

Desktop APIs can report monitor positions in logical, DPI-scaled coordinates while captured frames use physical pixels.
RustShot builds an explicit physical layout instead of applying one global scale factor. The primary monitor anchors
physical origin `(0, 0)`; neighbouring monitors are placed iteratively and logical gaps are scaled using the relevant
monitor geometry.

Selection covers only the monitor under the cursor. A single overlay spanning mixed-DPI monitors was abandoned because
the compositor can stretch and displace it. A recording selection must fit inside one physical monitor because RustShot
has no cross-monitor video compositing path.

## Grounding

- [Physical layout reconstruction](../../src/domain/layout.rs)
- [Cursor-monitor capture and overlay placement](../../src/selection/desktop.rs)
- [Executable multi-monitor examples](../../tests/features/multi_monitor_layout.feature)
- [Executable overlay matching examples](../../tests/features/overlay_targeting.feature)
- [Executable single-monitor recording constraint](../../tests/features/recording_selection.feature)

## Constraints

- Monitor bounds are half-open when locating a point.
- Negative desktop coordinates are valid.
- Overlay targeting prefers an exact origin-and-size match, then an origin match when DPI virtualization changes the
  reported size.
- Recording coordinates are translated from desktop space into the selected monitor's local physical space.

## Related

- [Screen Recording](screen-recording.md)
- [screenshots Crate](entity-screenshots-crate.md)
- [Why Pixels Lie](source-pixels-lie.md)
