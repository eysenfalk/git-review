# git-review

Rust TUI wrapping `git diff` with per-hunk review tracking and commit gating.
Linear ticket: ENG-2.

## Behavioral Rules

- ALWAYS read a file before editing it
- NEVER create files unless absolutely necessary for the goal
- ALWAYS prefer editing an existing file to creating a new one
- NEVER save working files or tests to the root folder
- NEVER commit secrets, credentials, or .env files
- Keep README.md updated when adding features or changing behavior
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
- **claude-mem**: Cross-session memory only. Save decisions/patterns (brief). NEVER duplicate Linear content.
- **Linear**: Source of truth for requirements, status, and acceptance criteria. ALL tickets tracked there.
- **Serena**: Semantic code intelligence via LSP. Use `find_symbol` and `get_symbols_overview` INSTEAD of reading full source files. Use `find_referencing_symbols` for impact analysis. Use `replace_symbol_body` for precise edits. Fall back to Read/Edit for non-code files.

## Hybrid Claude-Flow Integration (ENFORCED)

This project uses a **hybrid architecture** combining Claude Code's native agent teams with Claude-Flow V3 features. Both systems MUST work together — neither is optional.

### Architecture Philosophy

- **Priority:** Stability and integrity over speed optimization
- **Context:** Production-quality work, not just internal tooling
- **Enforcement:** 100% accuracy required for all workflow rules

### System Roles

#### Our Custom System (Blocking Enforcement)

- **Agent Teams:** TeamCreate + native experimental teams (visible tmux panes, mandatory)
- **Hooks:** `.claude/hooks/*.sh` scripts enforce workflow rules (blocking, fail-fast)
- **Memory:** `claude-mem` stores human decisions, architectural choices, preferences
- **Agent Specs:** `.claude/agents/*.md` define agent capabilities (authoritative for model routing)
- **Linear Integration:** Single source of truth for requirements, status, tickets

#### Claude-Flow V3 (Learning & Advisory)

- **Hooks:** Claude-Flow hooks run AFTER our enforcement hooks (non-blocking, continueOnError: true)
- **Background Workers:** 10 daemon workers (map, audit, optimize, consolidate, testgaps, ultralearn, deepdive, document, refactor, benchmark)
- **Memory:** Claude-Flow HNSW memory stores agent patterns, coding learnings, optimization history
- **Model Routing:** Claude-Flow routing provides suggestions; agent specs are the final authority
- **Task Management:** Claude-Flow task orchestration supplements native TaskCreate/TaskUpdate
- **Production Features:** DDD domain tracking, ADR generation, security scanning (CVE, threat modeling)

#### Serena MCP (Semantic Code Intelligence)

- **Symbol Navigation:** `find_symbol`, `get_symbols_overview` for token-efficient code reading (50-75% savings vs reading full files)
- **Impact Analysis:** `find_referencing_symbols` to find all callers/users of any symbol
- **Precise Edits:** `replace_symbol_body`, `insert_before/after_symbol` for symbol-level code modifications
- **Rename Refactoring:** `rename_symbol` with automatic reference updates across codebase
- **Pattern Search:** `search_for_pattern` for regex search across project files
- **Project Memory:** `read_memory`, `write_memory` for project-specific code structure and conventions

### Hook Execution Order (CRITICAL)

Hooks run in this exact order to ensure enforcement before learning:

**Write/Edit/MultiEdit:**
1. `.claude/hooks/protect-hooks.sh` (ask — prompts user before hook modifications)
2. `.claude/hooks/enforce-orchestrator-delegation-v2.sh` (blocking — ensures orchestrator delegates to agents; auto-allows subagents)
3. `.claude/hooks/enforce-plan-files.sh` (blocking — protects plan/ directory)
4. `.claude/hooks/check-unwrap.sh` (advisory — warns about `.unwrap()` in library code)
5. `.claude/hooks/scan-secrets.sh` (advisory — detects potential secrets/credentials in code)
6. Claude-Flow `pre-edit` hook (non-blocking — learns patterns, advisory only)

**Bash commands:**
1. `.claude/hooks/protect-hooks.sh` (ask — prompts user before hook modifications)
2. `.claude/hooks/enforce-orchestrator-delegation-v2.sh` (blocking — catches sed/awk/perl bypasses; auto-allows subagents)
3. `.claude/hooks/protect-main.sh` (blocking — prevents direct main commits)
4. `.claude/hooks/enforce-branch-naming.sh` (blocking — validates branch format)
5. `.claude/hooks/enforce-review-gate.sh` (blocking — requires git-review completion)
6. Claude-Flow `pre-command` hook (non-blocking — learns patterns, advisory only)

**Task (agent spawning):**
1. `.claude/hooks/enforce-visible-agents.sh` (blocking — requires team_name for agent visibility)
2. Claude-Flow task hook (non-blocking — advisory only)

**Read/Grep/Bash (source files):**
1. `.claude/hooks/enforce-serena-usage.sh` (advisory — suggests Serena for code navigation)

**Stop (session end):**
1. `.claude/hooks/enforce-memory.sh` (advisory — reminds to save to claude-mem)
2. `.claude/hooks/check-claude-flow-memory.sh` (advisory — reminds to save patterns)

**TaskCompleted (task marked done):**
1. `.claude/hooks/enforce-task-quality.sh` (blocking — runs `cargo test` + `cargo clippy` before task completion; skips non-code tasks)

**TeammateIdle (agent going idle):**
1. `.claude/hooks/enforce-idle-quality.sh` (blocking — runs `cargo test` + `cargo clippy` before idle; skips if no source changes)

### Memory Separation

- **claude-mem:** User preferences, architectural decisions, debugging insights, cross-session learnings (human-driven)
- **Claude-Flow memory:** Agent coordination patterns, code optimization history, model performance data (agent-driven)
- **Serena memory:** Project code structure, conventions, build commands (code-driven, auto-discovered via onboarding)
- **Linear:** Ticket requirements, acceptance criteria, status tracking (project management)
- **No overlap:** Each memory system serves a distinct purpose; never duplicate content

### Tool Routing (Serena vs Native Tools)

- **Reading code structure**: Use Serena `get_symbols_overview` → only read bodies you need (saves 50-75% tokens)
- **Finding definitions**: Use Serena `find_symbol` instead of Grep for symbol definitions
- **Finding callers**: Use Serena `find_referencing_symbols` instead of Grep for who-calls-what
- **Replacing functions**: Use Serena `replace_symbol_body` for entire function replacement (vs Edit for line-level)
- **Non-code files**: Use Read/Edit/Grep (Serena only works with LSP-supported languages)
- **Config/markdown/TOML**: Use Read/Edit (Serena has limited support for non-code)

### Model Routing

- **Agent specs** (`.claude/agents/*.md`) define the authoritative model for each agent type
- **Claude-Flow routing** provides suggestions based on task complexity and past performance
- **In case of conflict:** Agent spec always wins (manual configuration > automated suggestion)
- **Example:** `coder` uses Sonnet (per spec), even if Claude-Flow suggests Opus for a task

### Background Workers (Always Enabled)

All 10 Claude-Flow daemon workers run continuously:
- **map**: Codebase structure analysis
- **audit**: Code quality and security auditing
- **optimize**: Performance optimization suggestions
- **consolidate**: Memory and pattern consolidation
- **testgaps**: Test coverage gap detection
- **ultralearn**: Advanced pattern learning from agent interactions
- **deepdive**: Deep analysis of complex code changes
- **document**: Automatic documentation generation (ADR, DDD docs)
- **refactor**: Refactoring opportunity detection
- **benchmark**: Performance benchmarking and regression detection

### Production Features (Always Enabled)

- **DDD (Domain-Driven Design):** Track bounded contexts, validate domain boundaries, maintain `/docs/ddd`
- **ADR (Architecture Decision Records):** Auto-generate decision records, maintain `/docs/adr`, use MADR template
- **Security Scanning:** Auto-scan on edit, CVE vulnerability checking, threat modeling, security pattern detection

### Integration Rules

1. **Both systems required:** Claude-Flow features supplement native teams; never replace them
2. **Hook order matters:** Our blocking hooks run first; Claude-Flow hooks learn second
3. **Memory separation:** Never duplicate data across claude-mem, Claude-Flow memory, and Linear
4. **Model authority:** Agent specs define models; Claude-Flow routing is advisory only
5. **Stability first:** All Claude-Flow hooks have `continueOnError: true` to prevent workflow blockage
6. **100% enforcement:** Our custom hooks must maintain perfect accuracy; no false positives

### Why This Architecture?

- **Complementary systems:** Native teams handle coordination; Claude-Flow handles learning
- **Fail-safe design:** Blocking enforcement prevents mistakes; non-blocking learning improves over time
- **Production-ready:** DDD, ADR, security features support large-scale professional work
- **Clear boundaries:** Each system owns specific responsibilities with no overlap
- **Observable behavior:** tmux panes show agent activity; background workers run invisibly
- **Token efficiency:** Serena's LSP-powered navigation reads only what's needed (50-75% savings over full-file reads)

## Data Workflow (ENFORCED)

- **Linear** = single source of truth for requirements, status, and acceptance criteria
- **claude-mem** = cross-session decision memory (supplements Linear, never duplicates)
- **Local plan files** = ONLY the planner agent writes these (`plans/<feature>-plan.md`), for code-level implementation detail too granular for Linear. All other agents write to Linear comments.
- Requirements, specs, critiques, and review results go to **Linear comments**, not local files

## Git Workflow

### Branch Naming (ENFORCED by hook)

Format: `<type>/<ticket-id>-<short-description>`

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

Examples:
- `feat/eng-4-line-level-review`
- `docs/eng-5-developer-tooling`
- `fix/eng-6-parser-infinite-loop`

Rules:
- Every branch MUST have a Linear ticket
- Branch name MUST start with a type prefix
- Branch name MUST contain the ticket ID (e.g., `eng-4`)
- Use lowercase with hyphens

### Commit Message Format

```
feat(ENG-X): short description of what changed

- Bullet points explaining key changes
- Reference the ticket ID in the prefix
```

Prefixes: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

### Merge Policy

- NEVER commit or push directly to main (enforced by hook)
- NEVER merge without a completed git-review from the user (enforced by hook)
- Merge to main via PR after tests pass AND user has reviewed all hunks via `git-review`
- Merge BEFORE starting dependent work (if ENG-4 needs ENG-3, merge ENG-3 first)
- One branch per Linear ticket — no stacking features on the same branch
- Short-lived branches (1-2 days max)

### Review Gate (ENFORCED by hook)

Every PR requires the user to review changes with git-review first:
1. Agent finishes work, runs tests, pushes branch
2. User runs `git-review main..<branch>` to review all hunks
3. User marks all hunks as reviewed in the TUI
4. `git-review gate check` passes (all hunks reviewed)
5. Only then can a PR be created and merged

Agents MUST NOT create PRs or merge themselves. The user does both after their review.

### Worktrees for Parallel Agents

- Each agent works in its own worktree: `.trees/<ticket-id>/`
- Prevents file conflicts between parallel agents
- Clean up worktrees after merging: `git worktree remove .trees/<ticket-id>`

### What NOT to Do

- NEVER stack features on one branch (one ticket = one branch)
- NEVER branch off an unmerged feature branch (branch from main only)
- NEVER force push without explicit user approval
- NEVER work on main — create a feature branch first

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

- **ALWAYS use TeamCreate + `team_name` when spawning agents** — agents MUST run as visible tmux panes, never as invisible sub-processes. No exceptions.
- Always shut down all teammates before calling `TeamDelete`
- Use `haiku` model for lightweight/test teammates to save tokens
- Each teammate gets its own context window; they do NOT inherit conversation history
- Provide full task context in the spawn prompt
- Avoid assigning multiple teammates to the same file to prevent conflicts
- Use `TaskCreate` / `TaskUpdate` for coordinating work across teammates

## Agent Routing

### Agent Pipeline (ordered workflow)

1. `requirements-interviewer` — Gather and clarify requirements from the user
2. `explorer` — Research libraries, APIs, prior art, technical approaches
3. `architect` — Design module boundaries, data flow, type definitions
4. `planner` (Opus) — Write step-by-step implementation plan (ONLY agent that writes local plan files)
5. `red-teamer` (Opus) — Critique the plan, find bugs/edge cases/risks before implementation
6. `coder` (Sonnet) — Standard implementation with TDD
7. `senior-coder` (Opus) — Complex/cross-cutting/performance-critical implementation
8. `reviewer` (Opus) — Code review after implementation (reads code, checks quality)
9. `qa` (Sonnet) — QA testing after implementation (runs things, verifies behavior, tests hooks/workflows)
10. `documentation` — Update README, doc comments, guides
11. `explainer` — Explain code at different expertise levels (junior → staff/architect)
12. `optimizer` — Meta-workflow audit (run after every major task completion)

### When to Use senior-coder vs coder

- **coder (Sonnet):** Single-module changes, straightforward features, bug fixes with clear cause, test writing
- **senior-coder (Opus):** Cross-module refactors, performance-critical paths, subtle/intermittent bugs, architecture-sensitive changes, tasks a coder failed at

### Orchestrator Rules

- The orchestrator (main session) MUST NOT write implementation code directly
- The orchestrator coordinates: creates teams, spawns agents, assigns tasks, reviews results
- ALL code changes go through coder or senior-coder agents
- The orchestrator MAY edit non-code files: CLAUDE.md, agent specs, hook scripts, plans
- The orchestrator MUST create a team (TeamCreate) before spawning any agents — agents must be visible in tmux panes

## Anti-Patterns

- Do NOT shell out to `git diff` without sanitizing arguments
- Do NOT store absolute paths in the SQLite database (use repo-relative)
- Do NOT assume UTF-8 for all diff content (handle binary gracefully)
- Do NOT block the TUI event loop with synchronous I/O
- Do NOT use `unwrap()` in library code; reserve for tests only
