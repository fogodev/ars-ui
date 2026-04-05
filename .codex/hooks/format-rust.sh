#!/bin/sh
set -eu

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
cd "$repo_root"

tmp_files=$(mktemp "${TMPDIR:-/tmp}/format-rust.XXXXXX")
trap 'rm -f "$tmp_files"' EXIT HUP INT TERM

{
  git diff --name-only -- '*.rs'
  git diff --cached --name-only -- '*.rs'
  git ls-files --others --exclude-standard -- '*.rs'
} | awk '!seen[$0]++' > "$tmp_files"

set --
while IFS= read -r file; do
  [ -n "$file" ] || continue
  [ -f "$file" ] || continue
  set -- "$@" "$file"
done < "$tmp_files"

[ "$#" -gt 0 ] || exit 0

if cargo +nightly fmt --help >/dev/null 2>&1; then
  cargo +nightly fmt -- "$@"
else
  cargo fmt -- "$@"
fi
