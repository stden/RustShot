# Reviewed runtime and capture bugs

## Scope and approved behavior

Fix the six confirmed findings from the code review, preserving other behavior:

1. Linux tray mode initializes GTK before constructing its menu and services GTK
   events alongside winit without a busy loop. Hotkey/menu channels remain
   responsive while the tray is idle.
2. Recording accepts odd-sized selections. Convert actual source pixels into an
   even-sized, bottom-up BGRA encoder frame with zero padding instead of reading
   beyond the captured image.
3. Screenshot saving reports errors from the final buffered flush for PNG, JPEG,
   and BMP, and reports success only after pending bytes have been written.
4. Linux region captures retain clipboard ownership after selection finishes.
   Transfer the selected RGBA image to the long-lived tray through its child's
   stdout pipe; retain the clipboard in the tray. Repeated captures must remain
   possible. Direct internal selection mode retains ownership itself when no
   parent receives the image. Do not write screenshot data to temporary files.
5. X11 overlays retain the native desktop origin when the primary monitor is not
   at zero. Preserve existing Windows mixed-DPI reconstruction.
6. Explicit formats in valid combined short options, such as -wfjpg, take
   precedence over output extensions and settings. Output/screen argument values
   must never be misclassified as flags.

## Design

Use a small Linux desktop helper for GTK initialization and nonblocking event
servicing. Schedule a short future winit deadline on Linux to service GTK and
existing receiver channels. Keep Windows and macOS event-loop behavior unchanged.

Linux selection subprocesses send one bounded image message (dimensions followed
by RGBA bytes) when launched by the tray. A reader thread forwards the result to
the tray; its main thread updates and retains the clipboard. Empty output means
selection cancellation. Reject partial or unreasonable messages before allocating
large buffers. Keep existing child shutdown ownership intact.

Recording, screenshot writing, layout placement, and CLI changes remain focused
on their respective functions. Do not implement unrelated cursor-detection or
JSON-parser enhancements that were excluded from the reviewed findings.

## Validation

Each fix gets a failing regression before implementation and a passing run after.
Check exact odd-frame pixel placement and padding; exercise final-write errors;
test clustered-option parsing and nonzero X11 origins; test image transfer
roundtrips, truncated input, and cancellation. Use Xvfb to verify real Linux tray
startup and clipboard persistence without a clipboard manager. Run formatting,
compilation, Clippy, the full suite, and Windows GNU compilation.

No publication, Git history changes, or live project settings are requested.
