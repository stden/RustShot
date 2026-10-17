# RustShot Retrieval Index

`wiki/pages/` is the compact retrieval layer for RustShot. Canonical facts remain in the linked code, executable
specifications, articles, and project documentation. Use a page summary and tags to find a relevant chunk, then follow
its `source_paths` before changing code or making a time-sensitive claim.

Only `wiki/pages/*.md` belongs in the retrieval corpus. Every page has YAML front matter with a stable `id`, `kind`,
searchable `tags`, canonical `source_paths`, and `related` page IDs. Headings divide the body into useful retrieval
chunks. Run `python wiki/lint.py` after changing the corpus; CI runs the same command.

## Capture, geometry, and recording

| Page | Kind | Retrieve for |
|---|---|---|
| [DPI and Coordinate Systems](pages/dpi-coordinate-systems.md) | Concept | Logical and physical monitor geometry, mixed DPI, overlay placement, recording selection. |
| [Screen Recording](pages/screen-recording.md) | Concept | Recording lifecycle, timing, frame conversion, native encoder boundaries. |
| [screenshots Crate](pages/entity-screenshots-crate.md) | Entity | Monitor enumeration and one-shot screen capture dependency. |
| [Windows Media Foundation](pages/entity-media-foundation.md) | Entity | Windows H.264 and MP4 encoding through `IMFSinkWriter`. |
| [Why Pixels Lie](pages/source-pixels-lie.md) | Source | Narrative source about DPI scaling and multi-monitor coordinates. |
| [Why Screen Recording Breaks](pages/source-recording-pitfalls.md) | Source | Narrative source about frame-to-MP4 failure modes. |

## Architecture and engineering workflow

| Page | Kind | Retrieve for |
|---|---|---|
| [Architecture Overview](pages/architecture-overview.md) | Concept | Canonical architecture entry points and dependency boundaries. |
| [Development and Validation](pages/development-and-validation.md) | Concept | RustShot-specific test, lint, documentation, and benchmark rules. |
| [Screenshot Utility Architecture](pages/source-architecture.md) | Source | Narrative source about the utility's hidden state and boundaries. |
| [GitHub Actions CI/CD](pages/source-github-pipeline.md) | Source | Technical source describing CI and release workflows. |

## Release and distribution

| Page | Kind | Retrieve for |
|---|---|---|
| [Release and Distribution](pages/release-distribution.md) | Concept | Portable binary, installer, versioning, autorun, and release automation. |
| [Release as a Product](pages/source-release-feature.md) | Source | Narrative source about delivery as product behavior. |
| [Inno Setup Pipeline](pages/source-inno-pipeline.md) | Source | Technical source for Windows installer construction. |

## Corpus maintenance

- Start new pages from [`templates/`](templates/).
- Keep each claim scoped to RustShot and list the repository files that ground it.
- Use stable metadata IDs for relationships; use Markdown links for human navigation.
- Update a synthesis page when its canonical source changes, then run `python wiki/lint.py`.
