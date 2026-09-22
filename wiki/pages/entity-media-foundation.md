---
id: entity.media-foundation
kind: entity
title: Windows Media Foundation
summary: Defines the native Windows encoder boundary that turns validated BGRA frames into finalized H.264 MP4 files.
tags:
  - h264
  - media-foundation
  - mp4
  - windows
source_paths:
  - src/record.rs
  - src/record/domain.rs
  - src/record/encoder.rs
related:
  - concept.screen-recording
  - source.screen-recording-pitfalls
---
# Windows Media Foundation

## Role in RustShot

RustShot uses Windows Media Foundation only for screen recording. `IMFSinkWriter` accepts timestamped RGB32 samples,
encodes H.264, and muxes the output into MP4. RustShot's RGB32 input is a bottom-up BGRA buffer prepared by the recording
pipeline.

## Contract and constraints

- The implementation is gated by `cfg(windows)`.
- Width and height entering the encoder are validated, non-zero, and even.
- Sample timestamps and durations use 100-nanosecond units.
- `CoGuard` balances COM initialization on the recording thread; `MfGuard` balances Media Foundation startup.
- Every native object must be released before its owning guard shuts down COM or Media Foundation.
- The sink writer must finalize before the recording result is considered successful.

## Grounding

- [Recording format and lifecycle](../../src/record.rs)
- [Validated recording values](../../src/record/domain.rs)
- [Native guards and sink-writer implementation](../../src/record/encoder.rs)

## Related

- [Screen Recording](screen-recording.md)
- [Why Screen Recording Breaks](source-recording-pitfalls.md)
- [Microsoft Media Foundation SDK](https://learn.microsoft.com/en-us/windows/win32/medfound/microsoft-media-foundation-sdk)
