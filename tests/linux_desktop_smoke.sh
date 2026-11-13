#!/bin/sh
# Build first with `cargo build --bin RustShot`, then run this on Linux with
# Xvfb, xauth, GNU timeout, and RustShot's GTK/AppIndicator runtime libraries.
set -eu

cd "$(dirname "$0")/.."
binary=${1:-${CARGO_TARGET_DIR:-target}/debug/RustShot}
case "$binary" in
    /*) ;;
    *) binary="$PWD/$binary" ;;
esac
[ -x "$binary" ] || {
    printf 'FAIL: RustShot binary is not executable: %s\n' "$binary" >&2
    exit 1
}

for command in xvfb-run timeout; do
    command -v "$command" >/dev/null || {
        printf 'FAIL: required command is unavailable: %s\n' "$command" >&2
        exit 1
    }
done

test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT HUP INT TERM

# No arguments must start tray mode and remain alive until the test stops it.
# Isolated XDG directories keep the smoke test away from the user's settings.
status=0
xvfb-run -a env \
    GDK_BACKEND=x11 WINIT_UNIX_BACKEND=x11 \
    XDG_CONFIG_HOME="$test_dir/config" \
    XDG_DATA_HOME="$test_dir/data" \
    XDG_CACHE_HOME="$test_dir/cache" \
    timeout --kill-after=1s 3s "$binary" >"$test_dir/output" 2>&1 || status=$?

if [ "$status" -ne 124 ]; then
    printf 'FAIL: Linux tray exited before the smoke timeout (status %s)\n' "$status" >&2
    cat "$test_dir/output" >&2
    exit 1
fi

printf 'PASS: Linux tray started without a panic and remained alive\n'
