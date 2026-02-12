#!/bin/bash
# Hook: Enforce 3-agent concurrent limit to prevent OOM crashes
# Matcher: ^Task$

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract prompt to ensure this is an agent spawn
PROMPT=$(echo "$INPUT" | jq -r '.tool_input.prompt // empty')

# If no prompt, this isn't a Task call, allow it
if [[ -z "$PROMPT" ]]; then
  exit 0
fi

# Count active teammates across all team configs
TEAM_CONFIGS_DIR="$HOME/.claude/teams"
ACTIVE_COUNT=0

# If no team configs directory exists, this is the first spawn, allow it
if [[ ! -d "$TEAM_CONFIGS_DIR" ]]; then
  exit 0
fi

# Count members in each team config
for config_file in "$TEAM_CONFIGS_DIR"/*/config.json; do
  # Skip if glob doesn't match any files
  if [[ ! -f "$config_file" ]]; then
    continue
  fi

  # Count active non-leader members in this team config
  MEMBERS_COUNT=$(jq '[.members[] | select(.isActive == true and .agentType != "team-lead")] | length' "$config_file" 2>/dev/null || echo "0")
  ACTIVE_COUNT=$((ACTIVE_COUNT + MEMBERS_COUNT))
done

# If we already have 3 or more active members, deny the spawn
if [[ $ACTIVE_COUNT -ge 3 ]]; then
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Agent cap: max 3 concurrent agents (currently $ACTIVE_COUNT active). Wait for teammates to complete or shut down idle agents before spawning new ones.\"
  }
}"
  exit 0
fi

# Allow the spawn (under the cap)
exit 0
