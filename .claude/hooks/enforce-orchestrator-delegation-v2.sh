#!/bin/bash
# Hook: Enforce orchestrator delegates implementation to agents
# Matcher: ^(Write|Edit|MultiEdit|Bash)$

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# If running as a subagent, allow — agents ARE the delegation
TRANSCRIPT_PATH_CHECK=$(echo "$INPUT" | jq -r '.transcript_path // empty')
if [[ "$TRANSCRIPT_PATH_CHECK" =~ /subagents/ ]]; then
  exit 0
fi

# If running in a worktree (.trees/), allow — worktree agents are delegated
# Team agents spawned via TeamCreate use worktrees, which creates a different
# project path that doesn't contain /subagents/
if [[ "$TRANSCRIPT_PATH_CHECK" =~ \.trees[/-] ]] || [[ "$TRANSCRIPT_PATH_CHECK" =~ -\.trees- ]]; then
  exit 0
fi

# Get tool name
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Handle Write/Edit/MultiEdit tools
if [[ "$TOOL_NAME" =~ ^(Write|Edit|MultiEdit)$ ]]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

  # If no file_path, allow
  if [[ -z "$FILE_PATH" ]]; then
    exit 0
  fi

  # Check if this is a source/test/config file (handles both relative and absolute paths)
  if [[ "$FILE_PATH" =~ (^|/)(src|tests)/ ]] || [[ "$FILE_PATH" =~ (^|/)Cargo\.toml$ ]]; then
    # Whitelist: .claude/, plans/, docs/, README.md, project-template/, .trees/ (worktree agents)
    if [[ "$FILE_PATH" =~ (\.claude/|plans/|docs/|README\.md|project-template/|\.trees/) ]]; then
      exit 0
    fi

    # Extract transcript_path
    TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')

    # If transcript exists, check for agent spawn
    if [[ -n "$TRANSCRIPT_PATH" ]] && [[ -f "$TRANSCRIPT_PATH" ]]; then
      # Look for Task or TeamCreate in transcript (signs of agent spawning)
      if grep -q '"name".*"Task"' "$TRANSCRIPT_PATH" || grep -q '"name".*"TeamCreate"' "$TRANSCRIPT_PATH"; then
        # Agent spawned, allow
        exit 0
      fi

      # No agent spawn found, this is orchestrator writing directly
      echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Orchestrator must delegate implementation. Spawn coder/senior-coder agents via TeamCreate + Task.\"
  }
}"
      exit 0
    else
      # Transcript unavailable — deny by default
      echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Cannot verify agent delegation (transcript unavailable). Spawn agents first.\"
  }
}"
      exit 0
    fi
  fi
fi

# Handle Bash tool (check for sed/awk/perl editing source files)
if [[ "$TOOL_NAME" == "Bash" ]]; then
  COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

  # If targeting worktree files, allow — worktree agents are delegated
  if [[ "$COMMAND" =~ \.trees/ ]]; then
    exit 0
  fi

  # Check for sed/awk/perl editing src/tests/Cargo.toml
  if [[ "$COMMAND" =~ (sed|awk|perl).*(src/|tests/|Cargo\.toml) ]]; then
    # Extract transcript_path
    TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')

    # If transcript exists, check for agent spawn
    if [[ -n "$TRANSCRIPT_PATH" ]] && [[ -f "$TRANSCRIPT_PATH" ]]; then
      if grep -q '"name".*"Task"' "$TRANSCRIPT_PATH" || grep -q '"name".*"TeamCreate"' "$TRANSCRIPT_PATH"; then
        exit 0
      fi

      # No agent spawn found
      echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Orchestrator must delegate implementation. Spawn coder/senior-coder agents via TeamCreate + Task.\"
  }
}"
      exit 0
    else
      # Transcript unavailable — deny by default
      echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Cannot verify agent delegation (transcript unavailable). Spawn agents first.\"
  }
}"
      exit 0
    fi
  fi
fi

# Allow operation
exit 0
