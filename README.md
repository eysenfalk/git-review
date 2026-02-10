# git-review

A terminal UI for reviewing git diffs hunk-by-hunk, tracking review progress in a local SQLite database, and gating commits until all hunks are reviewed.

## Install

```bash
git clone <repo-url>
cd git-review
cargo install --path .
```

## Quick Start

```bash
# Review changes between main and HEAD
git-review main..HEAD

# Review staged changes
git-review

# Review a specific range
git-review v1.0..v2.0
```

## TUI Controls

| Key | Action |
|-----|--------|
| `j` / `↓` | Next hunk |
| `k` / `↑` | Previous hunk |
| `Tab` | Next file |
| `Shift+Tab` | Previous file |
| `Space` | Toggle hunk reviewed/unreviewed |
| `r` | Mark hunk as reviewed |
| `u` | Mark hunk as unreviewed |
| `s` | Skip hunk (mark as skipped) |
| `a` | Mark all hunks in current file as reviewed |
| `Ctrl+d` | Scroll down 10 lines |
| `Ctrl+u` | Scroll up 10 lines |
| `PageDown` | Scroll down 20 lines |
| `PageUp` | Scroll up 20 lines |
| `f` | Filter: show only unreviewed hunks |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit |

## Layout

```
┌──────────────────────┬─────────────────────────────────┐
│ Files                │ Hunk Detail                     │
│                      │                                 │
│ ✓ src/main.rs   2/2 │ @@ -10,6 +10,8 @@              │
│ ◐ src/lib.rs    1/3 │ -old line                       │
│ ○ src/cli.rs    0/1 │ +new line                       │
│                      │  context line                   │
│                      │                                 │
├──────────────────────┴─────────────────────────────────┤
│ [2/6 reviewed] ██████░░░░░░░░░ 33%                    │
└────────────────────────────────────────────────────────┘
```

- Left panel: file list with per-file progress (`✓` all reviewed, `◐` partial, `○` none)
- Right panel: current hunk with syntax-highlighted diff content
- Bottom bar: overall review progress

## Hunk States

- **Unreviewed** — default state, not yet looked at
- **Reviewed** — you've approved this change
- **Skipped** — intentionally deferred

## Syntax Highlighting

Diff content is syntax-highlighted based on the file extension using [syntect](https://github.com/trishume/syntect). Addition lines (`+`), deletion lines (`-`), and context lines are colored appropriately with language-aware highlighting on top.

## Commands

### `review` (default)

Launch the interactive TUI to review a diff range.

```bash
git-review main..HEAD          # shorthand
git-review review main..HEAD   # explicit subcommand
git-review                     # defaults to HEAD (staged changes)
```

### `status`

Print review progress without launching the TUI.

```bash
git-review status main..HEAD
git-review --status main..HEAD   # top-level flag
```

### `gate`

Manage the pre-commit hook that blocks commits with unreviewed hunks.

```bash
git-review gate check             # exit 0 if all reviewed, exit 1 otherwise
git-review gate enable            # install pre-commit hook
git-review gate disable           # remove pre-commit hook
```

### `reset`

Clear all review state for a given diff range.

```bash
git-review reset main..HEAD
```

## How State Works

Review state is stored in a local SQLite database (`.git-review.db` in the repo root). Each hunk is identified by a SHA-256 hash of its content. If a hunk's content changes (e.g., after amending a commit), it becomes **stale** and reverts to unreviewed — you'll need to re-review it.

This means:
- Rebasing or amending invalidates changed hunks (as expected)
- Unchanged hunks retain their review status across rebases
- The database is local and not committed to the repo

## Pre-commit Gate

When enabled, the gate installs a git pre-commit hook that runs `git-review gate check`. If any hunks in the staged diff are unreviewed, the commit is blocked.

```bash
git-review gate enable    # install hook
git commit                # blocked if unreviewed hunks exist
git-review gate disable   # remove hook
```

## Tech Stack

- [ratatui](https://ratatui.rs/) — terminal UI framework
- [rusqlite](https://docs.rs/rusqlite/) — SQLite bindings
- [clap](https://docs.rs/clap/) — CLI argument parsing
- [sha2](https://docs.rs/sha2/) — content hashing
- [syntect](https://docs.rs/syntect/) — syntax highlighting

## License

MIT
