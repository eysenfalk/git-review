# git-review

Rust TUI wrapping `git diff` with per-hunk review tracking and commit gating.

## Behavioral Rules

- ALWAYS read a file before editing it
- NEVER create files unless absolutely necessary for the goal
- ALWAYS prefer editing an existing file to creating a new one
- NEVER save working files or tests to the root folder
- NEVER commit secrets, credentials, or .env files
- Keep README.md updated when adding features or changing behavior
- Do what has been asked; nothing more, nothing less
- Orchestrator MUST NOT write implementation code — delegate to agents

## Resource Constraints

- **12GB RAM.** Teammates ~4GB each (max 3 + orchestrator). Subagents share session (max 5).
- Default to subagents. Teams only when agents need peer communication.
- Check for stale teams/worktrees after crashes. Reduce agent count if memory-constrained.

## File Organization

- `/src/parser/` — git diff parsing (pure transformation, no side effects)
- `/src/state/` — SQLite persistence, hunk hashing, staleness detection
- `/src/tui/` — ratatui interactive review interface
- `/src/gate/` — pre-commit hook + wrapper command
- `/src/cli/` — clap argument parsing, subcommands
- `/tests/` — integration and unit tests
- `/scripts/` — utility scripts

## Build & Test

```bash
cargo test                          # Run all tests
cargo clippy -- -D warnings         # Lint (treat warnings as errors)
cargo fmt --check                   # Format check
```

ALWAYS run `cargo test` after code changes. ALWAYS run `cargo clippy -- -D warnings` before commits.

## MCP Servers

- **context7**: Query Rust/ratatui/rusqlite docs BEFORE using unfamiliar APIs
- **claude-mem**: Cross-session memory. Save decisions/patterns (brief). NEVER duplicate Linear.
- **Linear**: Source of truth for requirements, status, acceptance criteria. ALL tickets tracked there.
- **Serena**: Semantic code navigation via LSP. Use `find_symbol`/`get_symbols_overview` INSTEAD of reading full files. Use `find_referencing_symbols` for impact analysis. Fall back to Read/Edit for non-code files.

## Skills (Progressive Disclosure)

Load skills on-demand for detailed guidance. Agent specs (`.claude/agents/*.md`) preload relevant skills automatically via the `skills` YAML field.

| Skill | When to Load |
|-------|-------------|
| `/rust-dev` | Writing Rust code: architecture, TDD, error handling, anti-patterns |
| `/git-workflow` | Branch creation, commits, merges, review gate, worktrees |
| `/agent-routing` | Spawning agents: pipeline order, model tiers, orchestrator rules |
| `/code-navigation` | Choosing between Serena LSP tools and native Read/Grep/Edit |
| `/quality-gates` | Definition of Done checklist, review pipeline, hook testing |
| `/orchestration` | Teams vs subagents decision framework, enforcement hooks awareness |
| `/claude-flow-integration` | Hook execution order, system roles, background workers, integration rules |
| `/memory-workflow` | Memory system separation, data workflow rules, observation policy |
| `/context-budget` | Context usage tracking, skill loading limits per agent |
| `/capability-diagnostic` | Diagnosing agent failures, infrastructure issues, escalation triggers |
| `/escalation` | Confidence-based model tier escalation (haiku → sonnet → opus) |
