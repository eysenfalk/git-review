#!/bin/bash
# Auto-checkpoint: saves session state to .claude/checkpoints/latest.json
# Called from TaskCompleted, TeammateIdle, and Stop hooks
set -euo pipefail

# Ensure checkpoint directory exists
CHECKPOINT_DIR=".claude/checkpoints"
mkdir -p "$CHECKPOINT_DIR"

# Read hook input
INPUT=$(cat)
EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // "unknown"' 2>/dev/null || echo "unknown")
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"' 2>/dev/null || echo "unknown")

# Gather git state
BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
LAST_COMMIT=$(git log -1 --format="%h %s" 2>/dev/null || echo "unknown")
HEAD_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Get uncommitted files as JSON array
UNCOMMITTED_RAW=$(git diff --name-only 2>/dev/null | head -20 || true)
if [ -z "$UNCOMMITTED_RAW" ]; then
  UNCOMMITTED="[]"
else
  UNCOMMITTED=$(echo "$UNCOMMITTED_RAW" | jq -R . | jq -s .)
fi

# Get untracked files as JSON array
UNTRACKED_RAW=$(git ls-files --others --exclude-standard 2>/dev/null | head -20 || true)
if [ -z "$UNTRACKED_RAW" ]; then
  UNTRACKED="[]"
else
  UNTRACKED=$(echo "$UNTRACKED_RAW" | jq -R . | jq -s .)
fi

# Gather task state from team task directories
TASKS_JSON="[]"
for task_dir in "$HOME"/.claude/tasks/*/; do
  [ -d "$task_dir" ] || continue
  for task_file in "$task_dir"*.json; do
    [ -f "$task_file" ] || continue
    TASKS_JSON=$(echo "$TASKS_JSON" | jq --slurpfile t "$task_file" '. + [$t[0] | {id: .id, subject: .subject, status: .status, owner: (.owner // "unassigned")}]' 2>/dev/null || echo "$TASKS_JSON")
  done
done

# Gather team state
TEAM_INFO="null"
for config in "$HOME"/.claude/teams/*/config.json; do
  [ -f "$config" ] || continue
  TEAM_INFO=$(jq '{name: .name, members: [.members[] | {name: .name, type: .agentType, active: .isActive}]}' "$config" 2>/dev/null || echo "null")
  break  # just grab first active team
done

# Extract ticket from branch name
TICKET=$(echo "$BRANCH" | grep -oE 'eng-[0-9]+' | head -1 || echo "none")

# Try to get last test result (check if cargo test was run recently)
TEST_STATUS="unknown"
if [ -f "target/debug/deps/git_review-"*.d ] 2>/dev/null; then
  # There's a built test binary — tests have been compiled at least
  TEST_STATUS="compiled"
fi

# Build checkpoint JSON
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
CHECKPOINT=$(jq -n \
  --arg ts "$TIMESTAMP" \
  --arg event "$EVENT" \
  --arg session "$SESSION_ID" \
  --arg branch "$BRANCH" \
  --arg ticket "$TICKET" \
  --arg last_commit "$LAST_COMMIT" \
  --arg head_sha "$HEAD_SHA" \
  --arg test_status "$TEST_STATUS" \
  --argjson uncommitted "$UNCOMMITTED" \
  --argjson untracked "$UNTRACKED" \
  --argjson tasks "$TASKS_JSON" \
  --argjson team "$TEAM_INFO" \
  '{
    timestamp: $ts,
    event: $event,
    session_id: $session,
    git: {
      branch: $branch,
      ticket: $ticket,
      head_sha: $head_sha,
      last_commit: $last_commit,
      uncommitted_files: $uncommitted,
      untracked_files: $untracked
    },
    tasks: $tasks,
    team: $team,
    build: {
      test_status: $test_status
    }
  }')

# Write checkpoint
echo "$CHECKPOINT" > "$CHECKPOINT_DIR/latest.json"

# Also write timestamped copy (keep last 10)
echo "$CHECKPOINT" > "$CHECKPOINT_DIR/checkpoint-$(date +%s).json"
ls -t "$CHECKPOINT_DIR"/checkpoint-*.json 2>/dev/null | tail -n +11 | xargs rm -f 2>/dev/null || true

# For Stop events, output the reminder
if [ "$EVENT" = "Stop" ]; then
  echo "{
  \"stopReason\": \"Session checkpoint saved to .claude/checkpoints/latest.json\"
}"
fi

exit 0
