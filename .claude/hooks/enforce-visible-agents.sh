#!/bin/bash
# Hook: Log agent spawning mode (team vs subagent)
# Matcher: ^Task$
# Updated 2026-02-12: subagents allowed without team_name per new delegation framework
# Teams required only when peer communication is needed (see CLAUDE.md)

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

# Allow both subagents (no team_name) and team agents (with team_name)
# Per decision framework: subagents for independent tasks, teams for peer communication
exit 0
