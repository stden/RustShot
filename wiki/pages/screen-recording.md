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
  - src/selection.rs
related:
  - concept.dpi-coordinate-systems
  - entity.media-foundation
  - source.screen-recording-pitfalls
---
# Screen Recording

## Answer

A recording session validates its dimensions and frame rate, starts a worker thread, and returns a [`RecordingSession`](../../src/record.rs) handle to the caller. The worker maintains a persistent Win32 GDI region capturer (with `screenshots` fallback), writes bottom-up BGRA pixels directly into the locked `IMFMediaBuffer`, and submits timestamped samples to a low-latency Media Foundation `IMFSinkWriter`. Stopping or dropping the session signals the worker and joins it so MP4 finalization completes before returning.

Frame timestamps use 100-nanosecond units and are derived from the frame index, which avoids accumulated rounding drift. H.264 dimensions are non-zero, even, and at least `MIN_H264_DIMENSION` (`64`).

## Grounding

- [Recording API, GDI capturer, Media Foundation sink writer, and pixel format](../../src/record.rs)
- [Single-monitor recording region resolution](../../src/selection.rs)

## Constraints

- Native MP4 encoding is available only on Windows.
- The selected desktop rectangle must belong to one monitor.
- COM and Media Foundation guards must outlive every native object they initialize.
- Finalization errors and worker panics are surfaced to the caller.

## Related

- [DPI and Coordinate Systems](dpi-coordinate-systems.md)
- [Windows Media Foundation](entity-media-foundation.md)
- [Why Screen Recording Breaks](source-recording-pitfalls.md)
