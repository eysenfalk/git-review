# ENG-31: Branch Review Dashboard — Implementation Plan (v2)

## Overview

Add an interactive branch dashboard to git-review's TUI. When the user is on the default branch (or runs `git-review` with no args on it), they see a table of all feature branches with review progress, diff stats, and one-keypress navigation into the existing hunk-review view. Merge and live-refresh are also supported.

**Sub-tickets:** ENG-32 (dashboard view), ENG-33 (navigation), ENG-34 (merge), ENG-35 (live refresh)
**Execution order:** ENG-32 → ENG-33 → ENG-34 + ENG-35 in parallel

**Red-team review:** v1 had 3 critical issues (performance, hard-coded main, merge safety), 4 high issues, and 3 medium issues. All addressed in this revision.

---

## Key Design Decisions (from red-team)

### D1. No hard-coded `main` — detect default branch
All references to `main` replaced with `detect_default_branch()` which checks `refs/remotes/origin/HEAD`, then falls back to `main`/`master`. Cached once per session.

### D2. Batch git operations — no O(N) process spawning
Branch listing uses ONE `git for-each-ref` call. Ahead/behind and diff stats are loaded LAZILY (only for visible/selected branches). Staleness check uses ONE `git rev-parse HEAD`.

### D3. Merge drops TUI, pre-checks conflicts, aborts on failure
Merge flow: check worktree clean → pre-check conflicts with `git merge-tree` → drop raw mode → execute merge → re-enter TUI. On failure: `git merge --abort` automatically.

### D4. ReviewDb stays at App level, never inside view modes
Both Dashboard and HunkReview access `self.db`. View transitions do NOT move or clone the DB. "Free dashboard memory" means freeing the items list, not the DB.

### D5. validate_git_ref only for user-supplied refs
Refs from `git for-each-ref` output are safe (we use `.arg()`, not shell). Only validate CLI arguments from the user.

### D6. Push to remote deferred to v2
`MergeOptions.push_remote` removed from v1 scope. Too risky without separate confirmation UX.

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
    pub last_commit_sha: String,
    pub last_commit_author: String,
    pub last_commit_age: String,
    pub last_commit_timestamp: i64,
}

/// Loaded lazily — only for visible/selected branches
#[derive(Debug, Clone, Default)]
pub struct BranchDetail {
    pub ahead: u32,
    pub behind: u32,
    pub diff_stats: DiffStats,
}

#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub file_count: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct MergeOptions {
    pub branch: String,
    pub delete_after: bool,
}

#[derive(Debug, Clone)]
pub enum WorktreeStatus {
    Clean,
    Dirty { modified: usize, untracked: usize },
}

/// Result of a merge conflict pre-check
pub enum MergeCheck {
    Clean,
    Conflicts,
    Error(String),
}
```

Function stubs (all `todo!()`):
- `find_repo_root() -> Result<PathBuf>`
- `validate_git_ref(ref_str: &str) -> Result<()>` — only for user-supplied refs
- `detect_default_branch() -> Result<String>` — NEW: checks origin/HEAD, falls back to main/master
- `get_diff(range: &str) -> Result<String>`
- `list_branches() -> Result<Vec<BranchInfo>>` — single `git for-each-ref` call
- `get_branch_detail(base: &str, branch: &str) -> Result<BranchDetail>` — lazy, per-branch
- `get_head_sha() -> Result<String>` — lightweight staleness check
- `check_worktree_status() -> Result<WorktreeStatus>`
- `check_merge_conflicts(base: &str, branch: &str) -> Result<MergeCheck>` — pre-check with merge-tree
- `merge_branch(options: &MergeOptions) -> Result<()>` — includes --abort on failure
- `delete_branch(name: &str) -> Result<()>`
- `get_current_branch() -> Result<Option<String>>` — returns None for detached HEAD

**Expected output:** `cargo check` passes
**Why junior-coder:** Pure type definitions from spec, zero logic

### Step 1.2 — Implement git operations (coder, Sonnet)

**Files to modify:** `src/git/mod.rs`
**Read first:** `src/main.rs` (lines 230-295), `src/git/mod.rs` (from 1.1)

Implement all function stubs:

- **`find_repo_root`**: Move logic from main.rs:252-269 (`git rev-parse --show-toplevel`)
- **`validate_git_ref`**: Move logic from main.rs:274-287 (character whitelist). ONLY called on user-supplied CLI refs, NOT on refs from git output.
- **`detect_default_branch`**: Check `git symbolic-ref refs/remotes/origin/HEAD` → strip prefix. Fallback: check if `main` or `master` exists via `git rev-parse --verify`. Cache-friendly (caller caches result).
- **`get_diff`**: Move logic from main.rs:233-249 (validate then `git diff`)
- **`list_branches`**: ONE call: `git for-each-ref --format='%(refname:short)|%(objectname:short)|%(authorname)|%(committerdate:relative)|%(committerdate:unix)' --sort=-committerdate refs/heads/`. Parse output into `Vec<BranchInfo>`. NO per-branch git calls.
- **`get_branch_detail`**: For a single branch, run `git rev-list --count --left-right {base}...{branch}` for ahead/behind and `git diff --numstat {base}..{branch}` for diff stats. Returns `BranchDetail`. Called lazily.
- **`get_head_sha`**: `git rev-parse HEAD`. Lightweight — used for polling.
- **`check_worktree_status`**: `git status --porcelain`, count entries
- **`check_merge_conflicts`**: `git merge-tree --write-tree $(git merge-base {base} {branch}) HEAD {branch}`. Returns `MergeCheck::Clean` or `MergeCheck::Conflicts`.
- **`merge_branch`**: `git merge --no-ff {branch}`. On failure: `git merge --abort` automatically. Does NOT checkout — caller handles that.
- **`delete_branch`**: `git branch -d {name}` (safe delete, not force)
- **`get_current_branch`**: `git branch --show-current`. Returns `None` if empty (detached HEAD).

All functions use `Command::new("git").arg(...)` — never string interpolation. All user-facing refs validated; git-internal refs trusted.

**Tests:**
- `validate_git_ref`: valid refs pass, injection attempts fail
- `detect_default_branch`: mock test or skip (requires git repo)
- `list_branches` output parsing
- `get_branch_detail` output parsing
- `get_head_sha` returns valid SHA

**Why coder:** Logic (parsing git output), but each function is self-contained

### Step 1.3 — Scaffold `src/dashboard/mod.rs` types (junior-coder, Haiku)

**Files to create:** `src/dashboard/mod.rs`
**Files to modify:** `src/lib.rs` (add `pub mod dashboard;`)
**Read first:** `src/lib.rs`, `src/git/mod.rs`

```rust
use crate::git::{BranchInfo, BranchDetail};
use crate::state::ReviewDb;

/// Review progress for a branch
#[derive(Debug, Clone, Default)]
pub struct ReviewProgress {
    pub reviewed: usize,
    pub total: usize,
}

/// A single row in the dashboard
pub struct DashboardItem {
    pub branch: BranchInfo,
    pub detail: Option<BranchDetail>,      // None = not yet loaded (lazy)
    pub progress: Option<ReviewProgress>,   // None = not yet computed
}

/// Dashboard state — owns the item list but NOT the ReviewDb
pub struct Dashboard {
    pub items: Vec<DashboardItem>,
    pub selected: usize,
    pub base_branch: String,        // detected, not hard-coded
    pub last_head_sha: String,      // for staleness check
}
```

Methods (trivial, implement directly):
- `select_next(&mut self)` — clamp to items.len()-1
- `select_prev(&mut self)` — clamp to 0
- `selected_branch(&self) -> Option<&str>` — return items[selected].branch.name
- `selected_item(&self) -> Option<&DashboardItem>`

Stubs (`todo!()`):
- `load(db: &ReviewDb, base_branch: &str) -> Result<Self>`
- `refresh(&mut self, db: &ReviewDb) -> Result<bool>` — returns true if changed
- `load_detail_for_selected(&mut self, db: &ReviewDb) -> Result<()>` — lazy load

**Why junior-coder:** Struct definitions, trivial navigation methods

### Step 1.4 — Implement dashboard loading logic (coder, Sonnet)

**Files to modify:** `src/dashboard/mod.rs`
**Read first:** `src/dashboard/mod.rs`, `src/git/mod.rs`, `src/state/mod.rs`

Implement:
- **`load()`**: Call `git::list_branches()` (single git call). For each branch, set `detail: None`, `progress: None` (lazy). Sort by `last_commit_timestamp` desc. Call `git::get_head_sha()` to set `last_head_sha`.
- **`refresh()`**: Call `git::get_head_sha()`. If same as `last_head_sha`, return `false` (no change). Otherwise, re-run `load()` and return `true`.
- **`load_detail_for_selected()`**: For the selected item, if `detail.is_none()`, call `git::get_branch_detail(base, branch)` and `db.progress(range)` to populate both `detail` and `progress`. This is called on navigation (when selected index changes) so the user sees stats for the branch they're looking at.
- **`can_merge_selected()`**: Returns true if selected item has `progress` with `reviewed == total && total > 0`.

**Tests:**
- `select_next`/`select_prev` boundary (empty list, single item, wrapping)
- `selected_branch` on empty dashboard returns None
- `can_merge_selected` with various progress states

**Why coder:** Orchestrating git calls, DB access, lazy loading logic

---

## Phase 2: TUI Integration (ENG-32, Part B + ENG-33)

### Step 2.1 — Add ViewMode to TUI and split rendering (senior-coder, Opus)

**Files to modify:** `src/tui/mod.rs`, `src/main.rs`

This is the highest-risk step — refactors the core TUI loop.

#### TUI Changes:

1. Add `ViewMode` enum:
   ```rust
   pub enum ViewMode {
       Dashboard,
       HunkReview { branch: String, base_ref: String },
   }
   ```

2. Add fields to `App`:
   ```rust
   pub view_mode: ViewMode,
   pub dashboard: Option<Dashboard>,
   pub status_message: Option<(String, Instant)>,  // temporary status messages
   pub last_refresh: Instant,
   ```
   **IMPORTANT:** `db: ReviewDb` stays at App level. NOT inside Dashboard or ViewMode.

3. Split `handle_input` → dispatch to `handle_dashboard_input` or `handle_hunk_review_input`
4. Split `render` → dispatch to `render_dashboard` or `render_hunk_review`
5. Add new constructors:
   - `App::new_dashboard(db, base_branch) -> Result<Self>` — loads dashboard
   - `App::new_hunk_review(files, db, base_ref) -> Self` — existing behavior

6. Dashboard keybindings: j/k nav, Enter drill-in, Shift+M merge, r refresh, q quit, ? help
7. Help screen: update to show different keys per ViewMode

#### main.rs Changes:

1. Auto-detection in `main()` when no args and no subcommand:
   ```rust
   let current = git::get_current_branch()?;
   let default = git::detect_default_branch()?;
   match current {
       Some(branch) if branch == default => handle_dashboard(),
       Some(_) => handle_review(&format!("{}..HEAD", default), false),
       None => handle_review("HEAD", false),  // detached HEAD
   }
   ```
2. Replace `find_repo_root`, `validate_git_ref`, `get_git_diff` calls with `git::*`
3. Add `handle_dashboard()` function

#### Event Loop:

```rust
pub fn run_tui(mut app: App) -> Result<()> {
    let mut terminal = setup_terminal()?;
    loop {
        terminal.draw(|f| app.render(f))?;
        if app.should_quit { break; }

        // Poll with 200ms timeout (balance between responsiveness and CPU)
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_input(key)?;
                }
            }
        }

        // Auto-refresh in dashboard mode (every 5 seconds)
        if matches!(app.view_mode, ViewMode::Dashboard) {
            if app.last_refresh.elapsed() >= Duration::from_secs(5) {
                app.try_refresh_dashboard();
                app.last_refresh = Instant::now();
            }
        }
    }
    restore_terminal(&mut terminal)?;
    Ok(())
}
```

**Why senior-coder:** Cross-module refactor touching the two largest files. Must preserve backward compatibility. Mistakes break everything.

---

## Phase 3: Navigation (ENG-33)

### Step 3.1 — Implement view transitions (coder, Sonnet)

**Files to modify:** `src/tui/mod.rs`

#### Dashboard → HunkReview:
```rust
fn enter_hunk_review(&mut self, branch: &str) -> Result<()> {
    let base = self.dashboard.as_ref().unwrap().base_branch.clone();
    let range = format!("{}..{}", base, branch);
    let diff_output = git::get_diff(&range)?;
    let files = crate::parser::parse_diff(&diff_output);
    self.db.sync_with_diff(&range, &files)?;
    self.files = files;
    self.base_ref = range;
    self.selected_file = 0;
    self.selected_hunk = 0;
    self.scroll_offset = 0;
    self.view_mode = ViewMode::HunkReview {
        branch: branch.to_string(),
        base_ref: base,
    };
    // Free dashboard items (NOT the db)
    self.dashboard = None;
    Ok(())
}
```

#### HunkReview → Dashboard:
```rust
fn return_to_dashboard(&mut self) -> Result<()> {
    let base = match &self.view_mode {
        ViewMode::HunkReview { base_ref, .. } => base_ref.clone(),
        _ => return Ok(()),
    };
    // Rebuild dashboard (reload branches, recompute progress)
    match Dashboard::load(&self.db, &base) {
        Ok(dashboard) => {
            self.dashboard = Some(dashboard);
            self.view_mode = ViewMode::Dashboard;
            self.files = vec![];  // Free hunk review memory
        }
        Err(e) => {
            // Stay in hunk review, show error
            self.status_message = Some((
                format!("Failed to return to dashboard: {}", e),
                Instant::now(),
            ));
        }
    }
    Ok(())
}
```

Wire: Enter in dashboard → `enter_hunk_review`, Esc/b in HunkReview (when entered from dashboard) → `return_to_dashboard`. If entered via CLI args (not dashboard), Esc quits as before.

**Tests:**
- Transition dashboard → hunk review sets correct ViewMode
- Transition hunk review → dashboard refreshes items
- Failed transition shows error, stays in current view

**Why coder:** Logic-heavy but single-file, clear spec

---

## Phase 4A: Merge (ENG-34) — ESCALATED to senior-coder

### Step 4.1 — Implement merge flow (senior-coder, Opus)

**Files to modify:** `src/tui/mod.rs`, `src/git/mod.rs`

**ESCALATED from coder to senior-coder** per red-team recommendation — merge is a destructive operation with complex failure modes.

#### Merge Flow:

```rust
fn handle_merge_request(&mut self) -> Result<()> {
    let dashboard = self.dashboard.as_ref().ok_or(anyhow!("not in dashboard"))?;
    let branch = dashboard.selected_branch()
        .ok_or(anyhow!("no branch selected"))?.to_string();

    // 1. Gate: all hunks reviewed?
    if !dashboard.can_merge_selected() {
        self.status_message = Some(("Not all hunks reviewed".into(), Instant::now()));
        return Ok(());
    }

    // 2. Gate: worktree clean?
    let status = git::check_worktree_status()?;
    if !matches!(status, WorktreeStatus::Clean) {
        self.status_message = Some((
            "Cannot merge: uncommitted changes. Commit or stash first.".into(),
            Instant::now(),
        ));
        return Ok(());
    }

    // 3. Gate: no merge conflicts?
    let base = &dashboard.base_branch;
    match git::check_merge_conflicts(base, &branch)? {
        MergeCheck::Conflicts => {
            self.status_message = Some((
                format!("Merge would conflict. Run 'git merge {}' manually.", branch),
                Instant::now(),
            ));
            return Ok(());
        }
        MergeCheck::Error(e) => {
            self.status_message = Some((format!("Conflict check failed: {}", e), Instant::now()));
            return Ok(());
        }
        MergeCheck::Clean => {}
    }

    // 4. Check if we need to checkout base first
    let current = git::get_current_branch()?;
    let needs_checkout = current.as_deref() != Some(base.as_str());

    // 5. Show confirmation modal
    self.confirm_action = Some(ConfirmAction::MergeBranch {
        branch: branch.clone(),
        delete_after: false,
        needs_checkout,
    });
    Ok(())
}
```

#### On Confirmation:

```rust
fn execute_merge(&mut self, branch: &str, delete_after: bool, needs_checkout: bool) -> Result<()> {
    let base = self.dashboard.as_ref().unwrap().base_branch.clone();

    // Drop TUI for merge operation
    restore_terminal(&mut self.terminal)?;

    let result = (|| -> Result<()> {
        if needs_checkout {
            // checkout base branch
            let output = Command::new("git").args(["checkout", &base]).output()?;
            if !output.status.success() {
                return Err(anyhow!("Failed to checkout {}", base));
            }
        }

        // Execute merge
        match git::merge_branch(&MergeOptions { branch: branch.into(), delete_after }) {
            Ok(()) => {
                println!("Successfully merged {} into {}", branch, base);
                if delete_after {
                    println!("Deleted branch {}", branch);
                }
                Ok(())
            }
            Err(e) => {
                // Abort failed merge
                let _ = Command::new("git").args(["merge", "--abort"]).output();
                Err(anyhow!("Merge failed: {}. Merge aborted.", e))
            }
        }
    })();

    // Re-enter TUI
    self.terminal = setup_terminal()?;

    match result {
        Ok(()) => {
            // Refresh dashboard
            if let Some(ref mut dashboard) = self.dashboard {
                dashboard.refresh(&self.db)?;
            }
            self.status_message = Some((
                format!("Merged {} into {}", branch, base),
                Instant::now(),
            ));
        }
        Err(e) => {
            self.status_message = Some((e.to_string(), Instant::now()));
        }
    }
    self.confirm_action = None;
    Ok(())
}
```

#### Confirmation Modal:

Add `MergeBranch` variant to `ConfirmAction`:
```rust
MergeBranch {
    branch: String,
    delete_after: bool,
    needs_checkout: bool,
}
```

Modal shows: branch name, review status, merge target. Checkbox: "Delete branch after merge" (toggle with Space). y to confirm, n to cancel. If `needs_checkout`, show warning: "Will checkout {base} first."

#### Merge Integration Tests:

Create `tests/merge_integration.rs`:
- Set up temp git repo with main + feature branch
- Test: merge on clean worktree succeeds
- Test: merge on dirty worktree is rejected
- Test: branch deletion after merge succeeds
- Test: merge --abort on conflict (create conflicting file)
- Test: needs_checkout detection

**Why senior-coder:** Destructive operation. Complex failure modes. Must handle: dirty worktree, conflicts, checkout, abort, TUI drop/restore. Mistakes leave repo in broken state.

---

## Phase 4B: Live Refresh (ENG-35)

### Step 4.2 — Implement auto-refresh (coder, Sonnet)

**Files to modify:** `src/tui/mod.rs`

Already wired into the event loop in Step 2.1. This step implements the actual refresh logic:

```rust
fn try_refresh_dashboard(&mut self) {
    if let Some(ref mut dashboard) = self.dashboard {
        match dashboard.refresh(&self.db) {
            Ok(true) => {
                // State changed, also re-load detail for selected
                let _ = dashboard.load_detail_for_selected(&self.db);
            }
            Ok(false) => {} // No change
            Err(e) => {
                self.status_message = Some((
                    format!("Refresh failed: {}", e),
                    Instant::now(),
                ));
            }
        }
    }
}
```

- Polling interval: **5 seconds** (not 2 — red-team identified this as too aggressive)
- Staleness check: only `git rev-parse HEAD` (one process, <5ms)
- Full reload only when HEAD SHA changes
- `r` key: call `try_refresh_dashboard()` immediately, reset `last_refresh`
- Status bar: "Last updated: Xs ago" in dashboard mode
- On navigation (j/k changes selected): call `load_detail_for_selected()` for lazy loading

**Stale diff detection in HunkReview mode:**
- When entering hunk review, store the HEAD SHA
- On `r` press in hunk review, check if HEAD changed
- If changed: show banner "Diff may have changed. Press 'r' to re-sync."
- Re-sync: re-run `git diff`, `parse_diff`, `sync_with_diff` (preserves reviewed status for unchanged hunks)

**Why coder:** Simple polling logic, clear spec

---

## Phase 5: Cleanup

### Step 5.1 — Remove duplicated helpers from main.rs (coder, Sonnet)

**Files to modify:** `src/main.rs`

Remove `find_repo_root`, `validate_git_ref`, `get_git_diff` from main.rs. Replace all call sites with `git::find_repo_root()`, `git::validate_git_ref()`, `git::get_diff()`. ~60 lines removed.

### Step 5.2 — Refactor `handle_watch` to use git module (coder, Sonnet)

**Files to modify:** `src/main.rs`

`handle_watch` (main.rs:325-377) duplicates branch listing and progress logic. Refactor to use `git::list_branches()`, `git::get_branch_detail()`, and `Dashboard::load()`. Reduces ~50 lines of duplicated git command construction.

### Step 5.3 — Update help screens and README (documentation, Haiku)

**Files to modify:** `src/tui/mod.rs` (help text), `README.md`

Update `render_help()` to show different keybindings per ViewMode. Update README with dashboard usage:
- `git-review` on default branch → dashboard
- `git-review` on feature branch → review vs default branch
- `git-review <range>` → existing behavior

---

## Dependency Graph

```
1.1 + 1.3 (parallel, junior-coder)
  → 1.2 (coder, git operations)
    → 1.4 (coder, dashboard loading)
      → 2.1 (senior-coder, TUI refactor — CRITICAL PATH)
        → 3.1 + 5.1 + 5.2 (parallel, coder)
          → 4.1 (senior-coder, merge — DESTRUCTIVE)
          → 4.2 (coder, live refresh)
            → 5.3 (documentation, help/README)
```

## Agent Assignments Summary

| Step | Agent | Model | Files | Risk |
|------|-------|-------|-------|------|
| 1.1 | junior-coder | Haiku | `src/git/mod.rs` (create), `src/lib.rs` | Low |
| 1.2 | coder | Sonnet | `src/git/mod.rs` | Medium |
| 1.3 | junior-coder | Haiku | `src/dashboard/mod.rs` (create), `src/lib.rs` | Low |
| 1.4 | coder | Sonnet | `src/dashboard/mod.rs` | Medium |
| 2.1 | senior-coder | **Opus** | `src/tui/mod.rs`, `src/main.rs` | **High** |
| 3.1 | coder | Sonnet | `src/tui/mod.rs` | Medium |
| 4.1 | senior-coder | **Opus** | `src/tui/mod.rs`, `src/git/mod.rs` | **High** |
| 4.2 | coder | Sonnet | `src/tui/mod.rs` | Low |
| 5.1 | coder | Sonnet | `src/main.rs` | Low |
| 5.2 | coder | Sonnet | `src/main.rs` | Low |
| 5.3 | documentation | Haiku | `src/tui/mod.rs`, `README.md` | Low |

**Max concurrent agents:** 2 (respects the 3-agent limit with orchestrator)

## Risk Areas (updated from red-team)

1. **Step 2.1 (TUI refactor)** — Highest risk. Changing App struct and event loop. Must preserve backward compatibility. Senior-coder (Opus).
2. **Step 4.1 (Merge)** — Destructive operation. Must handle: dirty worktree, conflicts, checkout, abort, TUI drop/restore. Senior-coder (Opus). MUST have integration tests.
3. **Lazy loading UX** — Dashboard shows "—" for stats until branch is selected. Ensure this is clear to the user (not confusing).
4. **ReviewDb ownership** — DB stays at App level. View transitions must not move/clone it. Explicitly documented in design decision D4.
5. **Default branch detection** — `detect_default_branch()` may fail on repos without a remote. Fallback chain: origin/HEAD → main → master → error.
6. **base_ref format mismatch** — Existing reviews keyed by `HEAD` won't appear in dashboard (which uses `{base}..{branch}`). Known limitation for v1 — document in README.

## Test Strategy

Every step: `cargo check` + `cargo clippy` + `cargo test` must pass before marking done.

| Step | Tests |
|------|-------|
| 1.2 | Unit: validate_git_ref, list_branches parsing, get_branch_detail parsing |
| 1.4 | Unit: select_next/prev boundaries, can_merge_selected states |
| 3.1 | Unit: view transition state changes |
| 4.1 | **Integration:** merge on clean/dirty worktree, conflict detection, abort, branch deletion |
| 4.2 | Unit: refresh logic, staleness detection |
