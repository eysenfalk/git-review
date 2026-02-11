#!/bin/bash
# Hook: Remind to save patterns to Claude-Flow memory
# Matcher: Stop

set -uo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin (may be empty or invalid)
INPUT=$(cat 2>/dev/null || true)

# If input is empty or not valid JSON, just exit cleanly
if [[ -z "$INPUT" ]] || ! echo "$INPUT" | jq empty 2>/dev/null; then
  exit 0
fi

# Extract transcript_path
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty' 2>/dev/null || true)

# If no transcript, allow
if [[ -z "$TRANSCRIPT_PATH" ]] || [[ ! -f "$TRANSCRIPT_PATH" ]]; then
  exit 0
fi

# Check if transcript contains edits to src/tests/ files
if grep -q '"file_path".*\(src/\|tests/\)' "$TRANSCRIPT_PATH"; then
  # Check if Claude-Flow memory_store was called
  if ! grep -q 'mcp__claude-flow__memory_store' "$TRANSCRIPT_PATH"; then
    # Suggest saving patterns
    echo "💡 Code was modified but no patterns saved to Claude-Flow memory. Consider: mcp__claude-flow__memory_store for agent patterns." >&2
    exit 0
  fi
fi

# Allow operation
exit 0
