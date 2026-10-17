# Security hardening implementation plan

**Goal:** Close all four reviewed security findings with focused regressions.

**Architecture:** Isolate writable CI state, pin a public build image, use the
Windows shell API, and make recording lifetime follow the parent's control pipe.
Use a small process owner where needed to test shutdown without a tray session.

**Tech stack:** Rust standard process/IO primitives, windows 0.51, GitLab CI,
shell-based CI policy validation.

## Tasks

- [x] CI worker: add a failing policy check covering writable shared Cargo state,
  mutable build-image consumption, and untrusted image publication.
- [x] CI worker: remove shared Cargo/sccache/target state; use fresh job storage,
  verify and pin the public Rust image digest, preserve checks and coverage, and
  restrict optional custom-image publication to the protected default branch.
- [x] CI worker: verify policy checks and YAML and document runner assumptions.
- [x] Folder worker: add shell result and path regressions, then implement the
  native Windows directory-opening helper without executable search.
- [x] Lifecycle worker: add failing real-subprocess tests for EOF stop, owner
  cleanup, and bounded fallback termination. Implement parent-owned recording
  control pipes and selection/recording cleanup; remove stop-file polling.
- [x] Integrator: connect the folder helper to the Windows tray action and review
  every shutdown path against the specification.
- [x] Independent reviewers: check specification coverage, then code quality and
  security boundaries. Resolve material findings before completion.
- [x] Integrator: run focused regressions, cargo fmt --check, cargo check,
  cargo clippy, and RUST_TEST_THREADS=1 cargo test. Attempt a Windows compile check
  and report the exact evidence and any environment limits.

Changes remain in the shared working tree for user review; no publication or
live GitLab configuration changes are part of this implementation.
