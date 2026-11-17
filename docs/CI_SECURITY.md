# GitLab CI trust boundaries

Rust jobs use the Docker Official Image
`rust:1.95.0-slim-trixie@sha256:e14e87345b4d5964ddcc3491d27ee046a0f23820f340c3c1e24da6880141f7c0`.
The digest is an upstream multi-platform manifest, verified against the Docker
Registry API on 2026-10-16: the `Docker-Content-Digest` response header and the
SHA-256 of the downloaded manifest matched. Updating Rust requires reviewing and
updating this digest in `.gitlab-ci.yml` and `Dockerfile.ci`.

Before Cargo runs, `ci/rust-job-env.sh` creates new directories under `/tmp` for
`CARGO_HOME` and `CARGO_TARGET_DIR`, and clears `RUSTC_WRAPPER`. This overrides the
project's local sccache configuration. Cargo configuration, extracted dependency
sources, installed tools, and compiler outputs therefore come from this job,
rather than a previous branch or merge request. If creating either directory
fails, job setup fails. GitLab caches are disabled, including target caches and
fallbacks to the default branch.

The jobs install GTK/X11 development libraries from Debian's signed repositories.
Formatting and Clippy install their components with rustup. Coverage adds OpenSSL
development headers and installs the exact `cargo-tarpaulin` 0.37.5 crate release
with `--locked` into its private Cargo home. These steps cost download and
compilation time; Rust and coverage timeouts account for the removal of shared
caches. No new CI variables or registry protection settings are needed for the
ordinary Rust jobs.

The optional `build:ci-image` job can publish convenience tags only on a protected
default branch, either after `Dockerfile.ci` changes or through its manual job.
It does not read or write Kaniko layer caches. Ordinary Rust jobs do not use that
image, so replacing a mutable convenience tag cannot change their environment.
Rules in a branch's pipeline are editable by that branch; the rules are not a
registry authorization mechanism. Restricting who can write registry tags still
belongs in GitLab settings if those tags are used outside this pipeline.

`sh tests/ci_security.sh` checks the image, cache, inheritance, and publication
policies, exercises fresh Cargo state with inherited poisoned paths, and verifies
that directory-creation failure stops setup. The formatting job runs this check.

## Runner administration

Repository configuration cannot isolate jobs that share writable host paths.
Do not bind-mount Cargo homes, Rust toolchains, executable directories, or build
directories between trusted and untrusted jobs. The former shared Cargo and
sccache mount is no longer used by this pipeline; remove it from the runner and
clear its old contents before any other workload reuses them. Merely using
different trust-named subdirectories within one writable mount is insufficient.

Do not expose the Docker socket or privileged containers to untrusted jobs.
Protected jobs and secrets require an appropriately isolated or protected runner;
these changes cannot repair a compromised runner host or revoke existing registry
credentials. No live runner or registry settings are changed by this repository
update. See GitLab's [runner security guidance](https://docs.gitlab.com/runner/security/)
and [Docker executor storage documentation](https://docs.gitlab.com/runner/executors/docker/).
