#!/bin/bash
# PostToolUse (Write|Edit): rustfmt the edited .rs file so `cargo fmt -- --check`
# in CI never fails. Never blocks: a file mid-edit may not parse yet, and the
# next edit or `cargo fmt` will catch up.
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
file=$(jq -r '.tool_response.filePath // .tool_input.file_path // empty')
case "$file" in *.rs) ;; *) exit 0 ;; esac
case "$file" in /*) ;; *) file="$root/$file" ;; esac
[ -f "$file" ] || exit 0

# Root crate is edition 2024; the separate scripts/crates workspace is 2021.
edition=2024
case "$file" in "$root"/scripts/crates/*) edition=2021 ;; esac

rustfmt --edition "$edition" "$file" >/dev/null 2>&1 || true
exit 0
