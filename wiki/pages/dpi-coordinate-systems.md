[← Back to Index](../index.md)

# DPI and Coordinate Systems

Modern desktop environments distinguish between logical coordinates and physical pixels. This is a critical distinction
for screenshot utilities.

## Core Problem

When scaling is not 100% (e.g., 125%, 150%), logical coordinates used for UI layout do not match physical pixels in the
framebuffer.

## Multi-monitor Challenges

* **Mixed DPI**: Different monitors can have different scaling factors.
* **Coordinate Mapping**: Monitors can be positioned anywhere in a virtual desktop, leading to negative coordinates
  (e.g., if a monitor is to the left of the primary one).
* **Overlay vs. Capture**: UI overlays might be drawn in logical coordinates, while screen capture data is often in
  physical pixels.

## Best Practices

* **Explicit Naming**: Use prefixes like `logical_` and `physical_` (or `framebuffer_`) for all geometric variables.
* **API Awareness**: Be careful with APIs that might return pre-scaled or unscaled values.

## See Also

* [Source: Pixels Lie](../pages/source-pixels-lie.md)
