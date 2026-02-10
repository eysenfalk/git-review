#!/bin/bash
# Hook: Verify claude-mem usage
# Matcher: Stop hook (runs at session end)
#
# CUSTOMIZATION:
# - Adjust the reminder message to match your team's memory usage policy
# - Change from reminder to requirement by using "permissionDecision": "deny"
# - Update the MCP tool name if using a different memory provider

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract transcript path if available
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')

# If we have a transcript path, check for claude-mem usage
if [[ -n "$TRANSCRIPT_PATH" ]] && [[ -f "$TRANSCRIPT_PATH" ]]; then
  # CUSTOMIZATION: Update MCP tool name if using a different memory provider
  # Check if transcript contains any claude-mem tool calls
  if grep -q "mcp__plugin_claude-mem" "$TRANSCRIPT_PATH" 2>/dev/null; then
    # Good! claude-mem was used
    exit 0
  fi

  # CUSTOMIZATION: Adjust reminder message to match your policy
  # No claude-mem usage found - inject reminder (but don't block)
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"Stop\",
    \"additionalContext\": \"💡 Reminder: Consider using claude-mem to save important insights, decisions, or patterns from this session for future reference. Run 'mem-search' to find related context or 'mem-save' to persist learnings.\"
  }
}" >&1
  exit 0
fi

# No transcript available, just allow
exit 0
