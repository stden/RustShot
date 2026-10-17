#!/bin/sh
# Source this file before Cargo commands. Never load state from runner caches.
CARGO_HOME="$(mktemp -d /tmp/rustshot-cargo.XXXXXX)" || return 1
CARGO_TARGET_DIR="$(mktemp -d /tmp/rustshot-target.XXXXXX)" || return 1
export CARGO_HOME CARGO_TARGET_DIR
export RUSTC_WRAPPER=""
export PATH="$CARGO_HOME/bin:$PATH"
