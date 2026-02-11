# ENG-31: Branch Review Dashboard — Implementation Plan

## Overview

Add an interactive branch dashboard to git-review's TUI. When the user is on `main` (or runs `git-review` with no args on main), they see a table of all feature branches with review progress, diff stats, and one-keypress navigation into the existing hunk-review view. Merge and live-refresh are also supported.

**Sub-tickets:** ENG-32 (dashboard view), ENG-33 (navigation), ENG-34 (merge), ENG-35 (live refresh)
**Execution order:** ENG-32 → ENG-33 → ENG-34 + ENG-35 in parallel

---

## Phase 1: Foundation Types & Git Module (ENG-32, Part A)

### Step 1.1 — Scaffold `src/git/mod.rs` types (junior-coder, Haiku)

**Files to create:** `src/git/mod.rs`
**Files to modify:** `src/lib.rs` (add `pub mod git;`)
**Read first:** `src/lib.rs`, `src/main.rs` (lines 230-295 for existing git helpers)

Create `src/git/mod.rs` with these exact type definitions and empty function stubs:

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("not in a git repository")]
    NotARepo,
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("invalid git ref: {0}")]
    InvalidRef(String),
    #[error("merge failed: {0}")]
    MergeFailed(String),
    #[error("utf-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GitError>;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_local: bool,
    pub ahead: u32,
    pub behind: u32,
    pub last_commit_sha: String,
    pub last_commit_author: String,
    pub last_commit_age: String,
    pub last_commit_timestamp: i64,
}

#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub file_count: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoState {
    pub branches_hash: String,
    pub head_sha: String,
}

#[derive(Debug, Clone)]
pub struct MergeOptions {
    pub branch: String,
    pub delete_after: bool,
    pub push_remote: bool,
}

#[derive(Debug, Clone)]
pub enum WorktreeStatus {
    Clean,
    Dirty { modified: usize, untracked: usize },
}
```

Function stubs (all `todo!()`): `find_repo_root`, `validate_git_ref`, `get_diff`, `list_branches`, `get_diff_stats`, `get_repo_state`, `check_worktree_status`, `merge_branch`

**Expected output:** `cargo check` passes
**Why junior-coder:** Pure type definitions from spec, zero logic

### Step 1.2 — Implement git operations (coder, Sonnet)

**Files to modify:** `src/git/mod.rs`
**Read first:** `src/main.rs` (lines 230-295), `src/git/mod.rs` (from 1.1)

Implement all function stubs:
- **`find_repo_root`**: Move logic from main.rs:252-269 (`git rev-parse --show-toplevel`)
- **`validate_git_ref`**: Move logic from main.rs:274-287 (character whitelist)
- **`get_diff`**: Move logic from main.rs:233-249 (validate then `git diff`)
- **`list_branches`**: `git branch --format '%(refname:short)|%(objectname:short)|%(authorname)|%(committerdate:relative)|%(committerdate:unix)'`, parse output. For ahead/behind: `git rev-list --count --left-right main...{branch}`
- **`get_diff_stats`**: `git diff --numstat {range}`, sum columns
- **`get_repo_state`**: `git rev-parse HEAD` + hash of branch list
- **`check_worktree_status`**: `git status --porcelain`, count entries
- **`merge_branch`**: `git merge --no-ff {branch}`, optionally `git branch -d`

**Tests:** validate_git_ref (port existing), get_diff_stats parsing
**Why coder:** Logic (parsing git output), but each function is self-contained

### Step 1.3 — Scaffold `src/dashboard/mod.rs` types (junior-coder, Haiku)

**Files to create:** `src/dashboard/mod.rs`
**Files to modify:** `src/lib.rs` (add `pub mod dashboard;`)
**Read first:** `src/lib.rs`, `src/git/mod.rs`

Create with: `DashboardItem` (branch_info + diff_stats + review_progress), `Dashboard` struct (items, selected, last_repo_state, db_path), trivial `select_next`/`select_prev`/`selected_branch` methods, `todo!()` stubs for `load`/`is_stale`/`refresh_if_stale`.

**Why junior-coder:** Struct definitions, trivial navigation methods

### Step 1.4 — Implement dashboard loading logic (coder, Sonnet)

**Files to modify:** `src/dashboard/mod.rs`
**Read first:** `src/dashboard/mod.rs`, `src/git/mod.rs`, `src/state/mod.rs`

Implement `load()` (list branches, get diff stats, load review progress per branch, sort by timestamp desc), `is_stale()` (compare repo state), `refresh_if_stale()`.

**Tests:** select_next/select_prev boundary, selected_branch on empty
**Why coder:** Orchestrating multiple git calls, DB access, sorting

---

## Phase 2: TUI Integration (ENG-32, Part B + ENG-33)

### Step 2.1 — Add ViewMode to TUI and split rendering (senior-coder, Opus)

**Files to modify:** `src/tui/mod.rs`, `src/main.rs`

This is the highest-risk step — refactors the core TUI loop:

1. Add `ViewMode` enum (`Dashboard` | `HunkReview { branch, base_ref }`)
2. Add `view_mode` and `dashboard` fields to `App`
3. Split `handle_input` → `handle_dashboard_input` + `handle_hunk_review_input`
4. Split `render` → `render_dashboard` + `render_hunk_review`
5. Update `run_tui` signature to accept `ViewMode`
6. Add auto-detection in main.rs: on main with no args → Dashboard, otherwise → HunkReview
7. Replace main.rs git helpers with `git::*` calls (removes ~60 lines duplication)

Dashboard layout: table with branch name, +/- stats, file count, review %, last commit age. Keys: j/k nav, Enter drill-in, m merge, r refresh, q quit.

**Why senior-coder:** Cross-module refactor touching the two largest files. Requires understanding entire event loop, careful method extraction, backward compatibility. Mistakes break everything.

---

## Phase 3: Navigation (ENG-33)

### Step 3.1 — Implement view transitions (coder, Sonnet)

**Files to modify:** `src/tui/mod.rs`

Implement `enter_hunk_review(branch)` (compute diff, parse, open DB, set ViewMode) and `return_to_dashboard()` (set ViewMode::Dashboard, refresh). Wire Enter in dashboard, Esc/Backspace in hunk review.

**Why coder:** Logic-heavy but single-file

---

## Phase 4A: Merge (ENG-34)

### Step 4.1 — Implement merge flow (coder, Sonnet)

**Files to modify:** `src/tui/mod.rs`

Add `MergeBranch` to `ConfirmAction`, `handle_merge_request()` (check worktree clean + review 100%), confirmation dialog, execute merge on confirm, status messages. Wire `m` key.

**Why coder:** Uses existing confirmation modal pattern

---

## Phase 4B: Live Refresh (ENG-35)

### Step 4.2 — Implement auto-refresh (coder, Sonnet)

**Files to modify:** `src/tui/mod.rs`

Add `last_refresh: Instant` to App. In event loop: if Dashboard and 2s elapsed, call `refresh_if_stale()`. Add `r` key for manual refresh. Show "Last updated: Xs ago" in status.

**Why coder:** Simple polling logic

---

## Phase 5: Cleanup

### Step 5.1 — Remove duplicated helpers from main.rs (coder, Sonnet)

**Files to modify:** `src/main.rs`

Remove `find_repo_root`, `validate_git_ref`, `get_git_diff` from main.rs, replace with `git::*` calls. ~60 lines removed.

---

## Dependency Graph

```
1.1 + 1.3 (parallel, junior-coder)
  → 1.2 (coder)
    → 1.4 (coder)
      → 2.1 (senior-coder, CRITICAL PATH)
        → 3.1 + 5.1 (parallel, coder)
          → 4.1 (coder)
          → 4.2 (coder)
```

## Agent Assignments Summary

| Step | Agent | Model | Files |
|------|-------|-------|-------|
| 1.1 | junior-coder | Haiku | `src/git/mod.rs` (create), `src/lib.rs` (edit) |
| 1.2 | coder | Sonnet | `src/git/mod.rs` (edit) |
| 1.3 | junior-coder | Haiku | `src/dashboard/mod.rs` (create), `src/lib.rs` (edit) |
| 1.4 | coder | Sonnet | `src/dashboard/mod.rs` (edit) |
| 2.1 | senior-coder | Opus | `src/tui/mod.rs`, `src/main.rs` |
| 3.1 | coder | Sonnet | `src/tui/mod.rs` |
| 4.1 | coder | Sonnet | `src/tui/mod.rs` |
| 4.2 | coder | Sonnet | `src/tui/mod.rs` |
| 5.1 | coder | Sonnet | `src/main.rs` |

**Max concurrent agents:** 2 (respects the 3-agent limit with orchestrator)

## Risk Areas

1. **Step 2.1 (TUI refactor)** — Highest risk. Changing App struct and event loop. Must preserve backward compatibility. Needs Opus.
2. **`list_branches` performance** — `git rev-list --count` per branch could be slow. Cap at 50 branches initially.
3. **Dashboard ↔ HunkReview state** — App struct needs careful `Option<...>` handling for fields only used in one mode.
4. **Merge on dirty worktree** — Must check BEFORE attempting. Failed merge leaves messy state.
5. **Review progress per branch** — Computing progress requires parsing diff for each branch. Consider lazy loading (show "—" until computed).

## Test Strategy

Every step: `cargo check` + `cargo clippy` + `cargo test` must pass before marking done.
New unit tests in steps 1.2, 1.4, 3.1. Integration tests deferred (TUI is hard to test programmatically).
