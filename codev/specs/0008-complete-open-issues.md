# Complete the remaining work in issues 6–10

The user approved implementation of the remaining issue acceptance criteria after
a source audit. Keep the existing screenshot and recording architecture, defaults,
manual JSON settings format, and `Result<_, String>` error handling.

## Design

- **Latency (#6):** add opt-in phase timing and a repeatable capture-request to
  clipboard harness for 1920×1080 and 3840×2160. Report sample count, p50, p95,
  worst, and per-phase timing. Share production capture/clipboard operations where
  practical. Document the measurement boundary, excluding human region selection,
  and the initial 1080p target below 100 ms. Disabled timing must preserve behavior.
- **Overlay (#7):** benchmark the existing renderer at 4K for crosshair, drag, and
  cache rebuild, including dimension changes. Report expected and measured backing
  allocation sizes and actual measurements on the available reference machine.
  State the redraw target below 16.7 ms. Make no speculative memory optimization.
- **Recording (#8):** run a Windows integration test in a bounded child process,
  record a deterministic small region, stop through `RecordingSession`, validate
  dimensions, H.264 codec, duration and MP4 contents, and clean up outputs after
  success and failure. CI must run the test and fail if capture is unavailable.
- **Hotkeys (#9):** persist three typed configurable shortcuts with current defaults;
  expose them in the native settings dialog; validate syntax and duplicate keys;
  apply updates live in the tray. Register additions before removing old bindings,
  retain previous bindings on registration failure, and show useful errors. Preserve
  unknown JSON fields, and add pure/BDD tests for defaults, updates and rollback.
- **Wayland (#10):** keep the existing full-desktop portal backend and X11 capture.
  Make unsupported region, screen, window and screen-list operations explicit;
  test backend/error/cleanup behavior; document prerequisites, GNOME/KDE manual
  verification and compositor differences; correct capability tables. MCP is not
  shipped, so document that dependency rather than inventing a new server.

## Alternatives and decisions

Keep native controls and the existing renderer instead of introducing a GUI
framework or a new pixel representation. Use opt-in measurement instead of adding
unconditional instrumentation. Use a process timeout for live Windows capture
instead of leaving ignored tests or detached threads that can hang the runner.
These choices minimize dependencies and preserve current behavior.

## Validation

Write behavioral tests before implementation and observe a meaningful failure.
Run focused tests, the full Linux suite under Xvfb, formatting, Clippy, duplication
checks and benchmark compilation. Cross-check Windows code by compilation where
the toolchain permits it, and use CI for actual Windows desktop recording.

Never report Windows runtime or GNOME/KDE verification as passed without running
it. Record reference hardware, commands and measurement boundaries. Leave issue
closure dependent on actual platform evidence where that is required.
