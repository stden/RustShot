[← Back to Index](../index.md)

# Entity: screenshots Crate

The `screenshots` crate is the core library used by RustShot for cross-platform screen capture and monitor enumeration.

## Role in Project

* **Monitor Enumeration**: Uses `Screen::all()` to detect all connected monitors and their properties (resolution,
  position, primary status).
* **Static Capture**: Used in `src/capture.rs` to take full-screen snapshots.
* **Recording Source**: Used in `src/record.rs` as the source of frames for the video pipeline.
* **Selection Geometry**: Provides the monitor rects used by `src/selection/desktop.rs` to build the overlay over the
  **cursor's monitor only**. The spanning virtual-desktop overlay was abandoned — a single window across mixed-DPI
  monitors gets stretched — and `capture_virtual_desktop` survives only as dead code.

## Key Types & Methods

* **`Screen`**: Represents a physical monitor.
    * `Screen::all()`: Returns a list of all detected screens.
    * `screen.display_info`: Contains `x`, `y`, `width`, `height`, and `is_primary` flags.
    * `screen.capture()`: Captures the full screen. Returns a `screenshots::Image`.
* **`Image`**: A wrapper around raw pixel data. RustShot typically converts this to `image::RgbaImage` for further
  processing.

## Constraints & Notes

* **Coordinate Systems**: Returns coordinates in the system's global virtual desktop space.
* **Pixel Format**: Returns raw RGBA bytes.
* **Performance**: The recording path no longer routes frames through this crate's full-screen `capture()`. Windows
  Graphics Capture is the primary backend (`src/capture/platform/wgc.rs`), with a GDI fallback that calls
  `capture_area_ignore_area_check` — so capture is region-scoped, and the "future optimization" has landed. This crate
  is still used for monitor enumeration (`Screen::all()`) and for one-shot screenshots.
* **Platform Support**: Handles the underlying platform-specific capture APIs (X11 on Linux, DXGI/GDI on Windows).

## Links

* [Crates.io: screenshots](https://crates.io/crates/screenshots)
* [DPI and Coordinate Systems](dpi-coordinate-systems.md)
* [Architecture Overview](architecture-overview.md)
