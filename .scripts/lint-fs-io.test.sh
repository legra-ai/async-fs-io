#!/usr/bin/env bash
# Self-test for lint-fs-io.sh: the boundary must catch fully-qualified
# calls AND import-site aliasing, honor the allowlist, and pass a clean
# tree. No `set -e` — every command's status is handled explicitly.

script_dir=$(cd "$(dirname "$0")" && pwd)
lint="$script_dir/lint-fs-io.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() { printf 'lint-fs-io.test: %s\n' "$1" >&2; exit 1; }

mkdir -p "$work/clean/src" "$work/dirty/src" "$work/aliased/src" "$work/allowed/backend"

printf 'pub fn ok() {}\n' > "$work/clean/src/lib.rs"
if ! "$lint" --root "$work/clean" >/dev/null 2>&1; then
    fail "a clean tree must pass"
fi

printf 'pub fn bad() { let _ = std::fs::read("x"); }\n' > "$work/dirty/src/lib.rs"
if "$lint" --root "$work/dirty" >/dev/null 2>&1; then
    fail "a fully-qualified std::fs call must be caught"
fi

printf 'use std::fs;\npub fn bad() { let _ = fs::read("x"); }\n' > "$work/aliased/src/lib.rs"
if "$lint" --root "$work/aliased" >/dev/null 2>&1; then
    fail "an aliasing import (use std::fs;) must be caught"
fi

printf 'use tokio::fs as tfs;\n' > "$work/aliased/src/tokio.rs"
if "$lint" --root "$work/aliased" >/dev/null 2>&1; then
    fail "a renamed tokio::fs import must be caught"
fi

printf 'pub fn backend() { let _ = std::fs::read("x"); }\n' > "$work/allowed/backend/impl.rs"
if ! "$lint" --root "$work/allowed" --allow-path backend >/dev/null 2>&1; then
    fail "the audited allow-path must be honored"
fi

printf 'lint-fs-io.test: all cases pass\n'
