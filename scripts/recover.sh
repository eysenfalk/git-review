#!/bin/bash
# Recovery script for cleaning up stale agent teams and worktrees
# Usage: ./scripts/recover.sh [--clean]

set -euo pipefail

CLEAN_MODE=false
if [[ "${1:-}" == "--clean" ]]; then
  CLEAN_MODE=true
fi

TEAMS_DIR="$HOME/.claude/teams"
CHECKPOINTS_DIR="$HOME/.claude/checkpoints"
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo ".")
TREES_DIR="$REPO_ROOT/.trees"

echo "=== Claude Agent Recovery Report ==="
echo

# 1. Check for stale team directories
echo "## Stale Team Directories"
if [[ ! -d "$TEAMS_DIR" ]]; then
  echo "  ✓ No teams directory found"
else
  STALE_TEAMS=$(find "$TEAMS_DIR" -mindepth 1 -maxdepth 1 -type d 2>/dev/null || true)
  if [[ -z "$STALE_TEAMS" ]]; then
    echo "  ✓ No team directories found"
  else
    echo "$STALE_TEAMS" | while read -r team_dir; do
      team_name=$(basename "$team_dir")
      config_file="$team_dir/config.json"
      if [[ -f "$config_file" ]]; then
        member_count=$(jq '.members | length' "$config_file" 2>/dev/null || echo "0")
        echo "  - $team_name: $member_count member(s)"
      else
        echo "  - $team_name: (no config.json)"
      fi

      if [[ "$CLEAN_MODE" == "true" ]]; then
        echo "    → Removing $team_dir"
        rm -rf "$team_dir"
      fi
    done
  fi
fi
echo

# 2. Check for orphaned git worktrees
echo "## Orphaned Git Worktrees"
if ! git worktree list --porcelain > /dev/null 2>&1; then
  echo "  ✗ Not in a git repository"
else
  WORKTREES=$(git worktree list --porcelain | grep -E '^worktree ' | sed 's/^worktree //' || true)
  MAIN_WORKTREE=$(git worktree list --porcelain | grep -E '^worktree ' | sed 's/^worktree //' | head -1)

  if [[ -z "$WORKTREES" ]]; then
    echo "  ✓ No worktrees found"
  else
    echo "$WORKTREES" | while read -r worktree_path; do
      # Skip the main worktree
      if [[ "$worktree_path" == "$MAIN_WORKTREE" ]]; then
        continue
      fi

      if [[ -d "$worktree_path" ]]; then
        branch=$(git -C "$worktree_path" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
        echo "  - $worktree_path (branch: $branch)"

        if [[ "$CLEAN_MODE" == "true" ]]; then
          echo "    → Removing worktree $worktree_path"
          git worktree remove "$worktree_path" --force 2>/dev/null || rm -rf "$worktree_path"
        fi
      else
        echo "  - $worktree_path (missing directory)"
        if [[ "$CLEAN_MODE" == "true" ]]; then
          echo "    → Pruning missing worktree"
          git worktree prune 2>/dev/null || true
        fi
      fi
    done

    if [[ "$CLEAN_MODE" == "true" ]]; then
      # Prune any remaining stale worktree references
      git worktree prune 2>/dev/null || true
    fi
  fi
fi
echo

# 3. Check for stale .trees/ directories
echo "## Stale .trees/ Directories"
if [[ ! -d "$TREES_DIR" ]]; then
  echo "  ✓ No .trees directory found"
else
  TREE_DIRS=$(find "$TREES_DIR" -mindepth 1 -maxdepth 1 -type d 2>/dev/null || true)
  if [[ -z "$TREE_DIRS" ]]; then
    echo "  ✓ No directories in .trees/"
  else
    echo "$TREE_DIRS" | while read -r tree_dir; do
      tree_name=$(basename "$tree_dir")
      echo "  - $tree_name"

      if [[ "$CLEAN_MODE" == "true" ]]; then
        echo "    → Removing $tree_dir"
        rm -rf "$tree_dir"
      fi
    done
  fi
fi
echo

# 4. Check for checkpoints
echo "## Last Checkpoint"
if [[ ! -d "$CHECKPOINTS_DIR" ]]; then
  echo "  ✓ No checkpoints directory found"
else
  LATEST_CHECKPOINT=$(find "$CHECKPOINTS_DIR" -type f -name "*.json" 2>/dev/null | sort -r | head -1 || true)
  if [[ -z "$LATEST_CHECKPOINT" ]]; then
    echo "  ✓ No checkpoints found"
  else
    checkpoint_name=$(basename "$LATEST_CHECKPOINT")
    checkpoint_time=$(stat -c %y "$LATEST_CHECKPOINT" 2>/dev/null || stat -f %Sm "$LATEST_CHECKPOINT" 2>/dev/null || echo "unknown")
    echo "  - $checkpoint_name (modified: $checkpoint_time)"
    echo "    Content preview:"
    jq -r '.summary // .description // "No summary"' "$LATEST_CHECKPOINT" 2>/dev/null | head -3 | sed 's/^/    /'
  fi
fi
echo

# Summary
echo "=== Summary ==="
if [[ "$CLEAN_MODE" == "true" ]]; then
  echo "Cleanup completed."
else
  echo "Dry-run mode. Run with --clean to actually remove stale resources."
fi
echo
echo "To resume work:"
echo "  1. Review the checkpoint content above (if any)"
echo "  2. Create a new team: TeamCreate"
echo "  3. Spawn agents with Task tool"
echo
