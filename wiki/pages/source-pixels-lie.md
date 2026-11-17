---
id: source.pixels-lie
kind: source
title: Why Pixels Lie
summary: Summarizes the project essay about logical coordinates, physical pixels, scaling, and multi-monitor drift.
tags:
  - coordinates
  - dpi
  - multi-monitor
source_paths:
  - articles/001-pixels-lie.ru.md
related:
  - concept.dpi-coordinate-systems
---
# Why Pixels Lie

## Canonical source

- [Why Pixels Lie: DPI, Scaling, and Multi-Monitor Chaos](../../articles/001-pixels-lie.ru.md)

## Retrieval summary

- Logical UI coordinates and physical framebuffer pixels coincide at 100 percent scaling and diverge at other scales.
- Mixed-DPI layouts combine scaling differences with negative monitor origins and API-specific coordinate conventions.
- These defects often produce plausible but shifted captures, so explicit coordinate-system naming and boundary conversion matter more than apparently simple rectangle arithmetic.
- The essay explains the problem shape; the linked concept page records RustShot's current implementation.

## Related

- [DPI and Coordinate Systems](dpi-coordinate-systems.md)
