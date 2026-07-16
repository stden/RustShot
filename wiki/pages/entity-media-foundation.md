[← Back to Index](../index.md)

# Entity: Windows Media Foundation

Windows Media Foundation (WMF) is the primary multimedia framework in Windows, succeeding DirectShow. In RustShot, it is used exclusively for screen recording on the Windows platform.

## Role in Project
* **Encoder**: Uses `IMFSinkWriter` to encode raw pixel buffers into H.264 video.
* **Muxer**: Handles the creation and finalization of MP4 containers.
* **Timestamping**: Manages video synchronization using 100-nanosecond unit timestamps.
* **Implementation**: Logic is contained in `src/record.rs`, specifically within the `wmf_record` function and `MfGuard` struct.

## Constraints & Notes
* **Windows-Only**: This entity is platform-specific and is gated behind `#[cfg(windows)]`.
* **Even Dimensions**: H.264 requires frame width and height to be even numbers. RustShot handles this with a `round_to_even` helper.
* **Coordinate Space**: The input expected is typically `RGB32` (BGRA).
* **Memory Management**: COM and MF APIs require explicit initialization (`MFStartup`) and shutdown (`MFShutdown`), managed via RAII guards (`MfGuard`) in RustShot.
* **Stability**: Improper handling of samples or failing to finalize the writer can lead to corrupted MP4 files or native crashes.

## Links
* [Official Documentation](https://learn.microsoft.com/en-us/windows/win32/medfound/microsoft-media-foundation-sdk)
* [Screen Recording Concept](screen-recording.md)
* [Screen Recording Pitfalls Source](source-recording-pitfalls.md)
