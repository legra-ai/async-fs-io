#!/usr/bin/env bash

# Enforce the async-fs-io boundary. Direct filesystem APIs are allowed only
# in explicitly named backend files. Every other source file, including
# private helpers and tests, must use async-fs-io.

root="."
allow_paths=()

while (($# > 0)); do
    case "$1" in
        --root)
            if (($# < 2)); then
                printf '%s\n' 'lint-fs-io: --root requires a path' >&2
                exit 2
            fi
            root=$2
            shift 2
            ;;
        --allow-path)
            if (($# < 2)); then
                printf '%s\n' 'lint-fs-io: --allow-path requires a path' >&2
                exit 2
            fi
            allow_paths+=("$2")
            shift 2
            ;;
        *)
            printf 'lint-fs-io: unknown argument: %s\n' "$1" >&2
            exit 2
            ;;
    esac
done

# Two match classes close the aliasing hole together: fully-qualified
# paths, and the import statements themselves (`use std::fs;`,
# `use std::fs as x;`, `use std::fs::{...}`). Any code touching a
# filesystem API needs one of the two, so `use std::fs; fs::read(...)`
# cannot slip through on spelling.
pattern='(std|tokio|async_std)::fs::|std::os::(unix|windows)::fs(::|;|,| )|^[[:space:]]*(pub[[:space:]]+)?use[[:space:]]+(std|tokio|async_std)::fs([[:space:]]*(;|::|as[[:space:]])|$)|spawn_blocking.*(File|read|write|rename|remove|metadata|read_dir)'
if command -v rg >/dev/null 2>&1; then
    matches=$(rg --line-number --no-heading --color never --glob '*.rs' "$pattern" "$root" 2>/tmp/lint-fs-io.err)
    rc=$?
else
    matches=$(grep -RInE --include='*.rs' "$pattern" "$root" 2>/tmp/lint-fs-io.err)
    rc=$?
fi
if (( rc > 1 )); then
    cat /tmp/lint-fs-io.err >&2
    exit "$rc"
fi

violations=""
while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    file=${line%%:*}
    file=${file#./}
    file=${file#"$root"/}
    allowed=false
    for allow_path in "${allow_paths[@]}"; do
        if [[ "$file" == "$allow_path" || "$file" == "$allow_path"/* ]]; then
            allowed=true
            break
        fi
    done
    if [[ "$allowed" == true ]]; then continue; fi
    violations+="$line\n"
done <<< "$matches"

if [[ -n "$violations" ]]; then
    printf '%s\n' 'lint-fs-io: direct filesystem I/O found outside the audited backend:' >&2
    printf '%b' "$violations" >&2
    exit 1
fi

printf '%s\n' 'lint-fs-io: no direct filesystem I/O outside the allowlist'
