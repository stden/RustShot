# Security hardening review

## Result

All four reviewed findings are addressed in the repository. Independent
specification and code-quality review approved the final changes.

## Changes

- GitLab Rust jobs use a verified, immutable Docker Official Image digest and
  fresh Cargo/target directories. Shared sccache, Cargo, and target caches are
  disabled. Branch-writable custom image tags are no longer consumed by those
  jobs. Optional custom-image publication is restricted to protected default-
  branch jobs, with registry authorization limits documented.
- Windows folder opening uses ShellExecuteW without executable-name lookup.
  UTF-16 paths retain their original representation; NULs and native errors are
  handled explicitly.
- A parent-owned stdin pipe replaces recording stop files. EOF stops recording,
  including after parent death, and already-signaled cancellation is checked
  before capture starts. Shutdown cleans up selection children and lets recording
  finalize, with a five-second deadline followed by forced termination.
- Child status inspection errors preserve ownership instead of permitting a
  live child to be overwritten. Regression subprocess readiness works with
  libtest's serial output format.

## Evidence

- Regression tests were observed failing before the fixes: CI trust-boundary
  policies, folder helper boundaries, child ownership/cleanup, and serial helper
  readiness. The final regressions pass.
- Full serial suite with standard Rust 1.95.0: 1,242 library tests, 1,541 binary
  tests, 226 integration tests, and 12 documentation tests pass. All 48 BDD
  scenarios and 114 steps pass.
- cargo fmt --all --check, cargo check --locked, and
  cargo clippy --locked --tests -- -D warnings pass.
- Windows GNU cross-compilation with cargo check --locked --target
  x86_64-pc-windows-gnu --tests passes.
- CI policy tests, shell syntax, YAML parsing, and git diff --check pass.
- The pinned CI image was pulled and tested in a disposable container. GTK/X11
  and OpenSSL development package installation, locked Tarpaulin 0.37.5
  installation, tool discovery, and formatting succeeded. A disposable crate
  passed LLVM coverage collection and produced a nonempty Cobertura report.

## Limits

The baseline has a parallel settings-test override race; final full-suite
validation uses RUST_TEST_THREADS=1. The default custom native-pgo toolchain
lacks rustdoc, so validation uses the installed standard 1.95.0 toolchain and
separate build directories. Windows GUI behavior was not exercised on a live
Windows desktop, and no live GitLab pipeline or runner/registry settings were
modified. Runner cleanup and isolation requirements are in docs/CI_SECURITY.md.

The earlier dependency advisory matches without an established application
exploit path were not part of these four fixes.
