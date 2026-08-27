[← Back to Index](../index.md)

# Architecture Overview

RustShot keeps its architecture and design rationale in rustdoc, starting at
[`src/lib.rs`](../../src/lib.rs). See [`docs/README.md`](../../docs/README.md) for
the pointer table of non-obvious constraints and the reason there is no separate
architecture document. **This page intentionally holds no module list** — the one
it used to carry had already drifted (it omitted `mcp`, `window`, the split
`app/` modules, `settings_dialog` and `timing`, and still credited `main.rs` with
CLI parsing and the tray event loop, which is now a 33-line launcher).

What remains here is only the shape of the problem, which the code cannot state
for itself:

## Key Principles
* **Narrow Responsibility**: Each function and module has a clear, specific role.
* **Pure Logic vs. Platform Specific**: Separation of cross-platform logic from OS-dependent paths.
* **Explicit Boundary Checks**: Validating coordinates, file paths, and system states at module boundaries.
* **Testable Helpers**: Complex logic is extracted into small, testable helper functions.

## Hidden Complexity
The application must balance multiple states (tray active, selection ongoing, recording in progress) and navigate external factors like display configurations and filesystem permissions.

## See Also
* [Source: Screenshot Architecture](../pages/source-architecture.md)
* [DPI and Coordinate Systems](dpi-coordinate-systems.md)
