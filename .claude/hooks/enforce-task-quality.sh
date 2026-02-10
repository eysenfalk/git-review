#!/bin/bash
# Hook: Enforce quality checks before task completion
# Event: TaskCompleted

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# Extract fields
TASK_SUBJECT=$(echo "$INPUT" | jq -r '.task_subject // empty')
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

# Skip quality checks for non-code tasks
if [[ "$TASK_SUBJECT" =~ (docs|documentation|plan|research|review|explore) ]]; then
  exit 0
fi

# Run cargo test
echo "Running cargo test..." >&2
if ! cargo test 2>&1; then
  echo "" >&2
  echo "❌ Tests failed. Fix failing tests before completing this task." >&2
  exit 2
fi

# Run cargo clippy
echo "Running cargo clippy..." >&2
if ! cargo clippy -- -D warnings 2>&1; then
  echo "" >&2
  echo "❌ Clippy warnings detected. Fix all warnings before completing this task." >&2
  exit 2
fi

echo "✅ All quality checks passed" >&2
exit 0
