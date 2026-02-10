#!/bin/bash
# Hook: Block local plan files from non-planner agents
# Matcher: ^(Write|Edit|MultiEdit)$
#
# CUSTOMIZATION:
# - Update the directory pattern if you use a different plan directory name
# - Modify agent name detection logic if using different identification
# - Adjust error message to reflect your data workflow policy

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract file_path from tool_input
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# If no file_path, allow (nothing to check)
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# CUSTOMIZATION: Update directory pattern if using different plan directory
# Only gate files in the plans/ directory — that's where plan/spec/critique files live
if [[ "$FILE_PATH" =~ plans/ ]]; then
  # CUSTOMIZATION: Update agent name detection if using different identification
  # Extract agent name from session metadata or environment
  AGENT_NAME=$(echo "$INPUT" | jq -r '.session_id // empty' | grep -oP 'planner|Planner' || echo "")

  # Also check environment variable if available
  if [[ -z "$AGENT_NAME" ]] && [[ -n "${CLAUDE_AGENT_NAME:-}" ]]; then
    if [[ "$CLAUDE_AGENT_NAME" =~ planner|Planner ]]; then
      AGENT_NAME="planner"
    fi
  fi

  # If not a planner agent, block the operation
  if [[ -z "$AGENT_NAME" ]]; then
    # CUSTOMIZATION: Update error message to reflect your data workflow
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Only the planner agent can write to plan/spec/requirements files. Save requirements to Linear instead, or delegate planning to the planner agent.\"
  }
}" >&1
    exit 0
  fi
fi

# Allow operation
exit 0
