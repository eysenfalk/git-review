#!/bin/bash
# Hook: Enforce all agents are visible via TeamCreate + team_name
# Matcher: ^Task$

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# Extract subagent_type and team_name from tool_input
SUBAGENT_TYPE=$(echo "$INPUT" | jq -r '.tool_input.subagent_type // empty')
TEAM_NAME=$(echo "$INPUT" | jq -r '.tool_input.team_name // empty')

# If no subagent_type, this isn't spawning an agent, allow
if [[ -z "$SUBAGENT_TYPE" ]]; then
  exit 0
fi

# If subagent_type is present but team_name is missing, DENY
if [[ -z "$TEAM_NAME" ]]; then
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"All agents must be visible in tmux. Use TeamCreate first, then Task with team_name parameter.\"
  }
}"
  exit 0
fi

# Allow operation (agent spawn with team_name)
exit 0
