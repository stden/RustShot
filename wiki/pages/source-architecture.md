---
id: source.screenshot-architecture
kind: source
title: Screenshot Utility Architecture
summary: Summarizes the project essay about hidden state, operating-system boundaries, and modular screenshot design.
tags:
  - architecture
  - boundaries
  - state
source_paths:
  - articles/002-screenshot-architecture.ru.md
related:
  - concept.architecture-overview
---
# Screenshot Utility Architecture

## Canonical source

- [Screenshot Utility Architecture: A Small Program with a Lot of Hidden Complexity](../../articles/002-screenshot-architecture.ru.md)

## Retrieval summary

- A small tray utility still coordinates CLI routing, capture, selection, recording, settings, installation, and automatic startup.
- Much of its complexity is hidden state: tray lifetime, active selection, recording ownership, stop signaling, output location, and monitor identity.
- Narrow responsibilities, testable helpers, pure logic, native adapters, and explicit boundary checks keep that state maintainable.
- The essay is conceptual; use `src/lib.rs` through the architecture concept page for the current module map.

## Related

- [Architecture Overview](architecture-overview.md)
