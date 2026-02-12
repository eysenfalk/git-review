#!/bin/bash
# Hook: Block PR creation and merges unless user has reviewed via git-review
# Matcher: ^Bash$

set -euo pipefail

# DISABLED: Review gate temporarily disabled until dashboard has proper merge safety UX
# Tracked by ENG-43. Re-enable once stale hunk detection and merge safety indicators are built.
exit 0

# Read JSON input from stdin
INPUT=$(cat)

# Extract command
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# If no command, allow
if [[ -z "$COMMAND" ]]; then
  exit 0
fi

# Extract only the actual command (first line, before heredoc/pipe content)
FIRST_LINE=$(echo "$COMMAND" | head -1)

# Helper: find the git-review binary
find_git_review() {
  if command -v git-review &>/dev/null; then
    echo "git-review"
  elif [[ -x "./target/release/git-review" ]]; then
    echo "./target/release/git-review"
  elif [[ -x "./target/debug/git-review" ]]; then
    echo "./target/debug/git-review"
  fi
}

# Helper: check if all hunks are reviewed for a branch vs main
is_review_green() {
  local BRANCH="$1"
  local GIT_REVIEW
  GIT_REVIEW=$(find_git_review)

  if [[ -z "$GIT_REVIEW" ]]; then
    return 1
  fi

  local STATUS_OUTPUT
  STATUS_OUTPUT=$($GIT_REVIEW status "main..${BRANCH}" 2>/dev/null || echo "")

  if echo "$STATUS_OUTPUT" | grep -q "All hunks reviewed"; then
    return 0
  fi

  return 1
}

# Helper: resolve the branch to check for a gh pr command
resolve_pr_branch() {
  local CMD_LINE="$1"
  # If gh pr merge <number>, look up the PR's head branch
  local PR_NUM
  PR_NUM=$(echo "$CMD_LINE" | grep -oP 'gh\s+pr\s+(create|merge)\s+\K[0-9]+' || echo "")

  if [[ -n "$PR_NUM" ]]; then
    gh pr view "$PR_NUM" --json headRefName -q '.headRefName' 2>/dev/null || echo ""
    return
  fi

  # Otherwise use current branch
  git rev-parse --abbrev-ref HEAD 2>/dev/null || echo ""
}

# Helper: extract branch name from git merge command, skipping flags
extract_merge_branch() {
  local cmd="$1"

  # Remove everything before and including 'git merge'
  local args="${cmd#*git merge }"

  # Strip -m/-message with their quoted values as a unit, then strip remaining quoted strings
  args=$(echo "$args" | sed -E "s/(-m|--message)[[:space:]]+'[^']*'//g; s/(-m|--message)[[:space:]]+\"[^\"]*\"//g; s/'[^']*'//g; s/\"[^\"]*\"//g")

  # Use awk to parse arguments and extract branch name
  # This handles flags and their values properly
  echo "$args" | awk '
  {
    skip_next = 0
    branch = ""
    for (i = 1; i <= NF; i++) {
      arg = $i

      # Skip if previous arg was a flag that takes a value
      if (skip_next) {
        skip_next = 0
        continue
      }

      # Check if this is a flag that takes a value
      if (arg == "-m" || arg == "--message" || arg == "-s" || arg == "--strategy") {
        skip_next = 1
        continue
      }

      # Skip any other flag (starts with -)
      if (substr(arg, 1, 1) == "-") {
        continue
      }

      # This is the branch name
      branch = arg
      break
    }
    print branch
  }'
}

# Check: gh pr create, gh pr merge
if [[ "$FIRST_LINE" =~ gh[[:space:]]+pr[[:space:]]+(create|merge) ]]; then
  BRANCH=$(resolve_pr_branch "$FIRST_LINE")
  if [[ -n "$BRANCH" && "$BRANCH" != "HEAD" ]] && is_review_green "$BRANCH"; then
    exit 0
  fi
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"PR creation and merges require user review first. The user must: 1) run git-review main..<branch> to review all hunks, 2) mark all hunks as reviewed, 3) confirm git-review status shows all hunks reviewed. Ask the user to review first.\"
  }
}"
  exit 0
fi

# Check: git merge into main
if [[ "$FIRST_LINE" =~ git[[:space:]]+merge ]]; then
  CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
  if [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
    MERGE_BRANCH=$(extract_merge_branch "$FIRST_LINE")
    if [[ -n "$MERGE_BRANCH" ]] && is_review_green "$MERGE_BRANCH"; then
      exit 0
    fi
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Merges to main require user review first. The user must run git-review to review all hunks before merging. Ask the user to review and confirm.\"
  }
}"
    exit 0
  fi
fi

# Allow operation
exit 0
