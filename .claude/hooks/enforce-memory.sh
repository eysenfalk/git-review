#!/bin/bash
# Hook: Verify claude-mem usage
# Matcher: Stop hook (runs at session end)

set -uo pipefail

# Read JSON input from stdin (may be empty or invalid)
INPUT=$(cat 2>/dev/null || true)

# If input is empty or not valid JSON, just exit cleanly
if [[ -z "$INPUT" ]] || ! echo "$INPUT" | jq empty 2>/dev/null; then
  exit 0
fi

# Extract transcript path if available
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty' 2>/dev/null || true)

# If we have a transcript path, check for claude-mem usage
if [[ -n "$TRANSCRIPT_PATH" ]] && [[ -f "$TRANSCRIPT_PATH" ]]; then
  # Check if transcript contains any claude-mem tool calls
  if grep -q "mcp__plugin_claude-mem" "$TRANSCRIPT_PATH" 2>/dev/null; then
    # Good! claude-mem was used
    exit 0
  fi

  # No claude-mem usage found - print reminder to stderr (Stop hooks don't support hookSpecificOutput)
  echo "💡 Reminder: Consider using claude-mem to save important insights, decisions, or patterns from this session." >&2
  exit 0
fi

# No transcript available, just allow
exit 0
