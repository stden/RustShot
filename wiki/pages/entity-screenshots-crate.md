[← Back to Index](../index.md)

# Entity: screenshots Crate

The `screenshots` crate is the core library used by RustShot for cross-platform screen capture and monitor enumeration.

## Role in Project
* **Monitor Enumeration**: Uses `Screen::all()` to detect all connected monitors and their properties (resolution, position, primary status).
* **Static Capture**: Used in `src/capture.rs` to take full-screen snapshots.
* **Recording Source**: Used in `src/record.rs` as the source of frames for the video pipeline.
* **Selection Geometry**: Provides the monitor rects used by `src/selection.rs` to span the selection overlay across the entire virtual desktop.

## Key Types & Methods
* **`Screen`**: Represents a physical monitor.
    * `Screen::all()`: Returns a list of all detected screens.
    * `screen.display_info`: Contains `x`, `y`, `width`, `height`, and `is_primary` flags.
    * `screen.capture()`: Captures the full screen. Returns a `screenshots::Image`.
* **`Image`**: A wrapper around raw pixel data. RustShot typically converts this to `image::RgbaImage` for further processing.

## Constraints & Notes
* **Coordinate Systems**: Returns coordinates in the system's global virtual desktop space.
* **Pixel Format**: Returns raw RGBA bytes.
* **Performance**: Capturing the full screen and then cropping (current recording behavior) is expensive. Future optimizations plan to use `capture_area` to reduce memory pressure.
* **Platform Support**: Handles the underlying platform-specific capture APIs (X11 on Linux, DXGI/GDI on Windows).

## Links
* [Crates.io: screenshots](https://crates.io/crates/screenshots)
* [DPI and Coordinate Systems](dpi-coordinate-systems.md)
* [Architecture Overview](architecture-overview.md)
