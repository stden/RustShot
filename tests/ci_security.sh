#!/bin/sh
# Regression checks for CI state and image trust boundaries. No YAML dependencies.
set -eu

cd "$(dirname "$0")/.."
failures=0
check() {
    if "$@"; then
        return
    fi
    printf 'FAIL: %s\n' "$*" >&2
    failures=$((failures + 1))
}

section() {
    awk -v name="$1" '
        /^[^ #]/ { active = ($0 == name ":") }
        active { print }
    ' .gitlab-ci.yml
}

pinned_upstream_image() {
    section default | grep -Eq '^  image: "rust:1\.95\.0-slim-trixie@sha256:[a-f0-9]{64}"$'
}

no_shared_build_state() {
    ! grep -Eq '/cache|SCCACHE_|fallback_keys:' .gitlab-ci.yml
}

no_mutable_rust_image() {
    ! section .rust-base | grep -Eq '^  image:' &&
        ! section fmt | grep -Eq '^  image:' &&
        ! section clippy | grep -Eq '^  image:' &&
        ! section test | grep -Eq '^  image:' &&
        ! section coverage | grep -Eq '^  image:'
}

no_restored_caches() {
    section default | grep -Eq '^  cache: \[\]$' &&
        ! grep -E '^  cache:' .gitlab-ci.yml | grep -Ev '^  cache: \[\]$'
}

protected_image_publishing() {
    section build:ci-image | awk '
        /^  rules:/ { rules = 1; next }
        rules && /^    - / {
            count++
            if ($0 != "    - if: '\''$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH && $CI_COMMIT_REF_PROTECTED == \"true\"'\''") bad = 1
        }
        END { exit !(count == 2 && !bad) }
    '
}

rust_jobs_isolate_state() {
    section .rust-base | grep -Fq '. ci/rust-job-env.sh' &&
        section fmt | grep -Fq '. ci/rust-job-env.sh' &&
        section clippy | grep -Eq '^  extends: \.rust-base$' &&
        section test | grep -Eq '^  extends: \.rust-base$' &&
        section coverage | grep -Eq '^  extends: \.rust-base$' &&
        grep -Eq '^  RUSTC_WRAPPER: ""$' .gitlab-ci.yml
}

fresh_cargo_state() (
    [ -f ci/rust-job-env.sh ] || return 1
    # Simulate inherited poisoned state without executing anything from it.
    CARGO_HOME=/cache/shared/cargo
    CARGO_TARGET_DIR=/cache/shared/target
    RUSTC_WRAPPER=/cache/shared/runner
    . ci/rust-job-env.sh
    trap 'rm -rf "$CARGO_HOME" "$CARGO_TARGET_DIR"' EXIT HUP INT TERM
    case "$CARGO_HOME:$CARGO_TARGET_DIR" in
        /tmp/rustshot-cargo.*:/tmp/rustshot-target.*) ;;
        *) return 1 ;;
    esac
    [ -d "$CARGO_HOME" ] && [ -d "$CARGO_TARGET_DIR" ] &&
        [ -z "$RUSTC_WRAPPER" ] &&
        [ -z "$(ls -A "$CARGO_HOME")" ] &&
        [ -z "$(ls -A "$CARGO_TARGET_DIR")" ]
)

failed_isolation_stops_job() (
    [ -f ci/rust-job-env.sh ] || return 1
    ! sh -c 'mktemp() { return 1; }; . ci/rust-job-env.sh'
)

check pinned_upstream_image
check no_shared_build_state
check no_mutable_rust_image
check no_restored_caches
check protected_image_publishing
check rust_jobs_isolate_state
check fresh_cargo_state
check failed_isolation_stops_job
[ "$failures" -eq 0 ] || exit 1
printf 'PASS: CI image and Cargo state trust boundaries\n'
