---
id: concept.architecture-overview
kind: concept
title: Architecture Overview
summary: Routes architecture questions to the canonical RustShot module map, boundary rationale, and executable behavior.
tags:
  - architecture
  - boundaries
  - modules
source_paths:
  - src/lib.rs
  - src/main.rs
  - docs/README.md
related:
  - concept.development-and-validation
  - concept.dpi-coordinate-systems
  - source.screenshot-architecture
---
# Architecture Overview

## Answer

[`src/lib.rs`](../../src/lib.rs) and [`src/main.rs`](../../src/main.rs) define the canonical architecture map: CLI parsing and the system-tray event loop live in [`src/main.rs`](../../src/main.rs) and [`src/tray.rs`](../../src/tray.rs), screenshot capture and file saving in [`src/capture.rs`](../../src/capture.rs), interactive multi-monitor selection in [`src/selection.rs`](../../src/selection.rs), Windows Media Foundation recording in [`src/record.rs`](../../src/record.rs), and settings persistence in [`src/settings.rs`](../../src/settings.rs).

This page is a retrieval pointer. It does not duplicate the module map or design rationale because those details must change beside the code they constrain.

## Grounding

- [Library module exports](../../src/lib.rs)
- [Entry point, CLI routing, and tray event loop](../../src/main.rs)
- [Documentation map and rationale locations](../../docs/README.md)

## Constraints

- Architecture rationale stays in rustdoc beside the implementation.
- Complex logic is extracted into pure, testable helper functions returning `Result<_, String>`.
- Platform-specific code is isolated behind `#[cfg(windows)]` / `#[cfg(target_os = "linux")]` boundaries.

## Related

- [Development and Validation](development-and-validation.md)
- [DPI and Coordinate Systems](dpi-coordinate-systems.md)
- [Screenshot Utility Architecture](source-architecture.md)
