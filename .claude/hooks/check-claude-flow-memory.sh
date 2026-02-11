#!/bin/bash
# Hook: Remind to save patterns to Claude-Flow memory
# Matcher: Stop

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# Extract transcript_path
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')

# If no transcript, allow
if [[ -z "$TRANSCRIPT_PATH" ]] || [[ ! -f "$TRANSCRIPT_PATH" ]]; then
  exit 0
fi

# Check if transcript contains edits to src/tests/ files
if grep -q '"file_path".*\(src/\|tests/\)' "$TRANSCRIPT_PATH"; then
  # Check if Claude-Flow memory_store was called
  if ! grep -q 'mcp__claude-flow__memory_store' "$TRANSCRIPT_PATH"; then
    # Suggest saving patterns
    echo "{
  \"stopReason\": \"💡 Code was modified but no patterns saved to Claude-Flow memory. Consider: mcp__claude-flow__memory_store for agent patterns.\"
}" >&1
    exit 0
  fi
fi

# Allow operation
exit 0
