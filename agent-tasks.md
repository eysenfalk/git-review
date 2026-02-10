# Agent Tasks — git-review build

Linear: ENG-2 | Branch: eysenfalk/eng-2-build-tui-tool-for-hunk-level-git-diff-review-tracking-with

## Status

- [x] Task 1: Project setup (CLAUDE.md, Cargo.toml, types, stubs) — DONE
- [ ] Task 2: Parser + State layer
- [ ] Task 3: TUI interface
- [ ] Task 4: Gate + CLI wiring
- [ ] Task 5: Integration verification (final)

## Shared Types (src/lib.rs) — already created

```rust
HunkStatus { Unreviewed, Reviewed, Stale }
DiffHunk { old_start, old_count, new_start, new_count, content, content_hash, status }
DiffFile { path: PathBuf, hunks: Vec<DiffHunk> }
ReviewProgress { total_hunks, reviewed, unreviewed, stale, files_remaining, total_files }
```

---

## Task 2: Parser + State Layer

**Agent**: parser-state-agent | **Model**: sonnet | **Files owned**: src/parser/mod.rs, src/state/mod.rs

### Parser (src/parser/mod.rs)

Implement `pub fn parse_diff(input: &str) -> Vec<DiffFile>`:
- Parse `diff --git a/path b/path` to extract file paths
- Parse `@@ -old_start,old_count +new_start,new_count @@` hunk headers
- Collect hunk content lines (context, +, -)
- SHA-256 hash each hunk's content (use `sha2` crate)
- Skip binary files (`Binary files ... differ`)
- Handle new files (--- /dev/null), deleted files (+++ /dev/null), renames
- All hunks start as HunkStatus::Unreviewed

Unit tests:
- Empty input → empty vec
- Single file, single hunk
- Single file, multiple hunks
- Multiple files
- Binary file skipped
- New file (--- /dev/null)
- Deleted file (+++ /dev/null)
- Hash is deterministic
- Hunk header edge cases (count=0, omitted count)

### State (src/state/mod.rs)

Implement `ReviewDb` with `conn: rusqlite::Connection`:

Schema:
```sql
CREATE TABLE IF NOT EXISTS hunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    base_ref TEXT NOT NULL,
    file_path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unreviewed',
    reviewed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(base_ref, file_path, content_hash)
);
```

Methods:
- `open(path: &Path) -> Result<Self>` — open DB, create tables
- `get_status(base_ref, file_path, content_hash) -> Result<HunkStatus>`
- `set_status(base_ref, file_path, content_hash, status) -> Result<()>`
- `sync_with_diff(&mut self, base_ref, files: &[DiffFile]) -> Result<()>` — reconcile: new hunks→Unreviewed, missing→Stale, unchanged Reviewed→keep
- `progress(base_ref) -> Result<ReviewProgress>`
- `reset(base_ref) -> Result<()>` — delete all state for base ref

Error type:
```rust
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid hunk status: {0}")]
    InvalidStatus(String),
}
```

Unit tests (use tempfile):
- Open creates tables
- Save and retrieve status
- Toggle Unreviewed↔Reviewed
- Sync marks new hunks Unreviewed
- Sync marks changed hunks Stale
- Sync preserves Reviewed with same hash
- Progress counts accurate
- Reset clears state

### Verification
- `cargo test` passes
- `cargo clippy` clean
- No unwrap() in library code
- Doc comments on public APIs

---

## Task 3: TUI Interface

**Agent**: tui-agent | **Model**: sonnet | **Files owned**: src/tui/mod.rs

**MUST query context7 for ratatui docs before writing code.**

### App State
```rust
pub struct App {
    files: Vec<DiffFile>,
    db: ReviewDb,
    base_ref: String,
    selected_file: usize,
    selected_hunk: usize,
    scroll_offset: u16,
    filter: FilterMode,
    should_quit: bool,
}

pub enum FilterMode { All, Unreviewed, Stale }
```

### Entry point
`pub fn run_tui(files: Vec<DiffFile>, db: ReviewDb, base_ref: String) -> anyhow::Result<()>`

### Layout
- Left panel (30%): file list with review counts, color-coded
- Right panel (70%): hunk content with diff syntax highlighting
- Bottom bar: progress + keybinding hints

### Key bindings
- q/Esc → quit
- j/Down, k/Up → navigate hunks
- Tab/Shift+Tab → navigate files
- Space → toggle reviewed (writes to SQLite immediately)
- u/s/a → filter modes
- ? → help overlay

### Rendering
- `+` lines green, `-` lines red, `@@` lines cyan
- File list: green (all reviewed), yellow (partial), red (none)
- Status bar: "12/37 hunks reviewed (3 stale), 4 files remaining"

### Terminal safety
- Panic hook restores terminal (LeaveAlternateScreen, disable_raw_mode)
- All exit paths restore terminal

### Verification
- `cargo check` passes
- No unwrap() in library code
- Terminal restored on all exit paths

---

## Task 4: Gate + CLI Wiring

**Agent**: gate-cli-agent | **Model**: sonnet | **Files owned**: src/gate/mod.rs, src/cli/mod.rs, src/main.rs

### Gate (src/gate/mod.rs)

Functions:
- `check_gate(db: &ReviewDb, base_ref: &str) -> Result<bool>` — true if all reviewed
- `enable_gate(repo_root: &Path, base_ref: &str) -> Result<()>` — write pre-commit hook
- `disable_gate(repo_root: &Path) -> Result<()>` — remove hook (only if marker present)

Hook content:
```bash
#!/bin/sh
# Installed by git-review
exec git-review gate check
```

Safety: backup existing hooks, only remove hooks with marker, validate repo root.

### CLI (src/cli/mod.rs)

Update clap structs to support:
```
git-review <base>..<head>           # TUI
git-review <base>..<head> --status  # progress summary
git-review gate enable|disable|check
git-review commit [-- <git-args>]
git-review reset <base>..<head>
```

Add `diff_range` argument to Review/Status/Reset. Add `Commit` subcommand.

### main.rs wiring

Wire all subcommands:
1. Review: run git diff → parse → open DB → sync → launch TUI (or --status)
2. Gate: enable/disable/check
3. Commit: gate check then exec `git commit`
4. Reset: clear state

Helper functions:
- `get_git_diff(range: &str) -> Result<String>` — validate range format, run git diff
- `find_repo_root() -> Result<PathBuf>` — git rev-parse --show-toplevel
- DB path: `.git/review-state/review.db`

### Tests (tests/ directory)

- enable_gate creates hook with correct content
- disable_gate removes hook
- disable_gate ignores non-git-review hooks
- check_gate returns true when all reviewed
- check_gate returns false when unreviewed

### Verification
- `cargo test` passes
- `cargo clippy` clean
- Range argument validated (no shell injection)
- File paths sanitized

---

## Task 5: Integration Verification (orchestrator runs this)

After all agents complete:
1. `cargo test` — all tests pass
2. `cargo clippy -- -D warnings` — no warnings
3. `cargo fmt --check` — formatted
4. Manual smoke test if possible
5. Update Linear ENG-2 status
