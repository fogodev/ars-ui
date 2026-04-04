#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  exit 0
fi

mapfile -t files < <(
  {
    git diff --name-only -- '*.rs'
    git diff --cached --name-only -- '*.rs'
    git ls-files --others --exclude-standard -- '*.rs'
  } | awk '!seen[$0]++' | while IFS= read -r file; do
    [[ -n "$file" && -f "$file" ]] && printf '%s\n' "$file"
  done
)

if [[ ${#files[@]} -eq 0 ]]; then
  exit 0
fi

if cargo +nightly fmt --help >/dev/null 2>&1; then
  cargo +nightly fmt -- "${files[@]}"
else
  cargo fmt -- "${files[@]}"
fi
