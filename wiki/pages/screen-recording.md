---
id: concept.screen-recording
kind: concept
title: Screen Recording
summary: Describes RustShot's validated recording lifecycle from region capture through Media Foundation finalization.
tags:
  - h264
  - media-foundation
  - recording
  - timing
source_paths:
  - src/record.rs
  - src/record/domain.rs
  - src/record/encoder.rs
  - tests/features/recording_domain.feature
  - tests/features/in_process/recording_lifecycle.feature
  - tests/features/windows_recording.feature
related:
  - concept.dpi-coordinate-systems
  - entity.media-foundation
  - source.screen-recording-pitfalls
---
# Screen Recording

## Answer

A recording session validates its dimensions and frame rate, starts a worker, and returns control to the caller. The
worker keeps one Windows Graphics Capture session with a GDI fallback, converts captured pixels to bottom-up BGRA, and
submits timestamped samples to a Media Foundation sink writer. Stopping or dropping the owner signals the worker and
joins it so MP4 finalization completes before the result is returned.

Frame timestamps use 100-nanosecond units and are derived from the frame index, which avoids accumulated rounding drift.
H.264 dimensions are non-zero and even; an unroundable maximum dimension is rejected instead of silently reduced.

## Grounding

- [Recording API, lifecycle, and pixel format](../../src/record.rs)
- [Validated dimensions, FPS, timing, and bitrate](../../src/record/domain.rs)
- [COM, Media Foundation, and sink-writer boundary](../../src/record/encoder.rs)
- [Executable recording-domain examples](../../tests/features/recording_domain.feature)
- [Executable stop, drop, error, and panic lifecycle](../../tests/features/in_process/recording_lifecycle.feature)
- [Native Windows MP4 scenarios](../../tests/features/windows_recording.feature)

## Constraints

- Native MP4 encoding is available only on Windows.
- The selected desktop rectangle must belong to one monitor.
- COM and Media Foundation guards must outlive every native object they initialize.
- Finalization errors and worker panics are surfaced to the caller.

## Related

- [DPI and Coordinate Systems](dpi-coordinate-systems.md)
- [Windows Media Foundation](entity-media-foundation.md)
- [Why Screen Recording Breaks](source-recording-pitfalls.md)
