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
  - docs/README.md
  - tests/architecture.rs
related:
  - concept.development-and-validation
  - concept.dpi-coordinate-systems
  - source.screenshot-architecture
---
# Architecture Overview

## Answer

[`src/lib.rs`](../../src/lib.rs) is the canonical architecture map. Its rustdoc describes the dependency layers,
interactive capture sequence, and code-to-test boundaries. The executable is a thin launcher; mode routing lives in
`src/app/`, shared workflows in `src/application/`, operating-system adapters in `src/platform/`, reusable pixel
operations in `src/pixels/`, and validated shared values in `src/domain/`.

This page is a retrieval pointer. It does not duplicate the module map or design rationale because those details must
change beside the code they constrain.

## Grounding

- [Library architecture and Mermaid diagrams](../../src/lib.rs)
- [Documentation map and rationale locations](../../docs/README.md)
- [Dependency-direction checks](../../tests/architecture.rs)
- [Executable behavior specifications](../../tests/features/)

## Constraints

- Architecture rationale stays in rustdoc beside the implementation.
- User-visible behavior is specified in Gherkin under `tests/features/`.
- Shared domain and application layers must not depend on platform adapters; the architecture test enforces this.

## Related

- [Development and Validation](development-and-validation.md)
- [DPI and Coordinate Systems](dpi-coordinate-systems.md)
- [Screenshot Utility Architecture](source-architecture.md)
