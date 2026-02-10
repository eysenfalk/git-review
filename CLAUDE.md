# git-review

Rust TUI wrapping `git diff` with per-hunk review tracking and commit gating.
Linear ticket: ENG-2.

## Behavioral Rules

- ALWAYS read a file before editing it
- NEVER create files unless absolutely necessary for the goal
- ALWAYS prefer editing an existing file to creating a new one
- NEVER save working files, tests, or docs to the root folder
- NEVER commit secrets, credentials, or .env files
- NEVER proactively create documentation files unless explicitly requested
- Do what has been asked; nothing more, nothing less

## File Organization

- `/src/parser/` — git diff parsing
- `/src/state/` — SQLite persistence, hunk hashing, staleness
- `/src/tui/` — ratatui interactive review interface
- `/src/gate/` — pre-commit hook + wrapper command
- `/src/cli/` — clap argument parsing, subcommands
- `/tests/` — integration and unit tests
- `/scripts/` — utility scripts

## Architecture (Bounded Contexts)

### Parser
Parses raw `git diff` output into `DiffFile` / `DiffHunk` structures. No side effects, pure transformation. Handles unified diff format, binary files, rename detection.

### State
SQLite persistence via rusqlite. Stores review status per hunk (keyed by SHA-256 content hash). Detects stale hunks when diff content changes. Provides `ReviewDb` as the single entry point.

### TUI
ratatui-based interactive interface. File list on left, hunk view on right. Keyboard-driven: navigate hunks, mark reviewed, view progress. Reads from Parser output, writes to State.

### Gate
Pre-commit hook integration. `check_gate` returns whether all hunks are reviewed. `enable_gate` / `disable_gate` install/remove the git hook. Wrapper command for CI integration.

### CLI
clap derive-based argument parsing. Subcommands: `review` (default TUI), `status` (print progress), `gate` (check/enable/disable), `reset` (clear review state).

## Build & Test

```bash
cargo build                # Build
cargo test                 # Run all tests
cargo clippy               # Lint
cargo fmt --check          # Format check
cargo check                # Type check only
```

- ALWAYS run `cargo test` after code changes
- ALWAYS run `cargo check` before committing
- ALWAYS run `cargo clippy` before opening PRs

## TDD Enforcement

Follow red-green-refactor:
1. Write a failing test first
2. Write minimal code to make it pass
3. Refactor while keeping tests green

Use London School (mock-first) for integration boundaries. Use real implementations for pure logic.

## Error Handling

- `thiserror` for library errors in `src/lib.rs` and modules
- `anyhow` for the binary in `src/main.rs`
- No empty catch blocks or silent error swallowing
- Propagate errors with `?` operator; add context with `.context()`

## Definition of Done

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] No hardcoded paths or credentials
- [ ] Public APIs have doc comments
- [ ] Error cases handled, not ignored

## MCP Server Usage

- **context7**: Query Rust/ratatui/rusqlite docs BEFORE using unfamiliar APIs
- **claude-mem**: Save architectural decisions and debugging insights
- **linear**: Reference Linear ticket ENG-2 for requirements

## Git Workflow

- Feature branches off main
- Never push directly to main
- Atomic commits with descriptive messages
- Run full test suite before committing

## Security Rules

- Sanitize all file paths to prevent directory traversal
- Validate git refs before passing to shell commands
- Never pass unsanitized user input to `std::process::Command`
- Never hardcode API keys or credentials

## Agent Teams

Agent teams are enabled (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`). Teammates spawn as tmux split panes automatically (tmux is detected via `$TMUX`).

### Spawning workflow

1. `TeamCreate` — create a named team (creates shared task list)
2. `Task` with `team_name` and `name` — spawn teammates into the team
3. Teammates appear as tmux split panes and communicate via `SendMessage`
4. `SendMessage` with `type: "shutdown_request"` — gracefully stop teammates
5. `TeamDelete` — clean up team resources (only after all teammates shut down)

### Rules

- Always shut down all teammates before calling `TeamDelete`
- Use `haiku` model for lightweight/test teammates to save tokens
- Each teammate gets its own context window; they do NOT inherit conversation history
- Provide full task context in the spawn prompt
- Avoid assigning multiple teammates to the same file to prevent conflicts
- Use `TaskCreate` / `TaskUpdate` for coordinating work across teammates

## Anti-Patterns

- Do NOT shell out to `git diff` without sanitizing arguments
- Do NOT store absolute paths in the SQLite database (use repo-relative)
- Do NOT assume UTF-8 for all diff content (handle binary gracefully)
- Do NOT block the TUI event loop with synchronous I/O
- Do NOT use `unwrap()` in library code; reserve for tests only
