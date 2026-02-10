#!/bin/bash
# Hook: Enforce quality checks before teammate goes idle
# Event: TeammateIdle

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# Extract fields
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

# Use cwd if available, otherwise current directory
if [[ -n "$CWD" ]]; then
  cd "$CWD" || exit 0
fi

# Check if this is a Rust project
if [[ ! -f "Cargo.toml" ]]; then
  # Not a Rust project, skip quality checks
  exit 0
fi

# Check if any source files were modified
# Look for changes in src/, tests/, or Cargo.toml
if ! git diff --name-only HEAD 2>/dev/null | grep -qE '(^src/|^tests/|^Cargo\.toml$)'; then
  # No source file changes, skip quality checks
  exit 0
fi

# Run cargo test
echo "Running cargo test..." >&2
if ! cargo test 2>&1; then
  echo "" >&2
  echo "❌ Tests failed. Fix failing tests before going idle." >&2
  exit 2
fi

# Run cargo clippy
echo "Running cargo clippy..." >&2
if ! cargo clippy -- -D warnings 2>&1; then
  echo "" >&2
  echo "❌ Clippy warnings detected. Fix all warnings before going idle." >&2
  exit 2
fi

echo "✅ All quality checks passed" >&2
exit 0
