[← Back to Index](../index.md)

# Screen Recording

Screen recording is a complex pipeline that coordinates time, memory, and specialized codecs.

## The Recording Pipeline

Unlike static screenshots, recording requires continuous frame acquisition and synchronization.

1. **Frame Capture**: Regular polling of screen data.
2. **Buffer Conversion**: Transforming raw pixels to codec-compatible formats (e.g., RGB32 to YUV).
3. **Encoding**: Feeding frames into an encoder (e.g., H.264 via Media Foundation).
4. **Muxing**: Writing encoded data into a container (MP4).
5. **Finalization**: Gracefully closing the writer to ensure the file is valid.

## Common Pitfalls

* **Odd Dimensions**: H.264 encoders often fail or produce artifacts with non-even widths/heights.
* **Timestamp Drifts**: Inaccurate timing leads to desynchronized video.
* **Resource Leaks**: Improper COM/MF initialization or failed finalization.
* **Race Conditions**: Signals to stop recording must be coordinated with the final frame processing.

## Windows Implementation

Uses **Windows Media Foundation** (`IMFSinkWriter`) for H.264 encoding. Requires careful management of 100-ns unit
timestamps.

## 📋 Project Specifications (Codev)

* [Spec 0002: Test Plan Screenshots & Recording](../../codev/specs/0002-test-plan-screenshots-recording.md)

## See Also

* [Source: Screen Recording Pitfalls](../pages/source-recording-pitfalls.md)
