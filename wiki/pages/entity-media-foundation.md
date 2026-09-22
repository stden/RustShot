[← Back to Index](../index.md)

# Entity: Windows Media Foundation

Windows Media Foundation (WMF) is the primary multimedia framework in Windows, succeeding DirectShow. In RustShot, it is
used exclusively for screen recording on the Windows platform.

## Role in Project

* **Encoder**: Uses `IMFSinkWriter` to encode raw pixel buffers into H.264 video.
* **Muxer**: Handles the creation and finalization of MP4 containers.
* **Timestamping**: Manages video synchronization using 100-nanosecond unit timestamps.
* **Implementation**: `src/record/encoder.rs` — the `wmf_record` function and the `MfGuard` RAII struct. Of the items
  named on this page, only `checked_round_to_even` still lives in `src/record.rs`.

## Constraints & Notes

* **Windows-Only**: This entity is platform-specific and is gated behind `#[cfg(windows)]`.
* **Even Dimensions**: H.264 requires frame width and height to be even numbers. RustShot rounds through
  `checked_round_to_even`, which rejects `u32::MAX` rather than returning `u32::MAX - 1` the way the infallible
  `round_to_even` does — a dimension that cannot round up is an error, not a silently smaller frame.
* **Coordinate Space**: The input expected is typically `RGB32` (BGRA).
* **Memory Management**: COM and MF APIs require explicit initialization (`MFStartup`) and shutdown (`MFShutdown`),
  managed via RAII guards (`MfGuard`) in RustShot.
* **Stability**: Improper handling of samples or failing to finalize the writer can lead to corrupted MP4 files or
  native crashes.

## Links

* [Official Documentation](https://learn.microsoft.com/en-us/windows/win32/medfound/microsoft-media-foundation-sdk)
* [Screen Recording Concept](screen-recording.md)
* [Screen Recording Pitfalls Source](source-recording-pitfalls.md)
