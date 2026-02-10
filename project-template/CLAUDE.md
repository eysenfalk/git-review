# [PROJECT_NAME]

<!-- TODO: Add project description and Linear ticket reference -->
<!-- Example: Rust TUI wrapping `git diff` with per-hunk review tracking and commit gating. -->
<!-- Example: Linear ticket: ENG-2. -->

## Behavioral Rules

- ALWAYS read a file before editing it
- NEVER create files unless absolutely necessary for the goal
- ALWAYS prefer editing an existing file to creating a new one
- NEVER save working files or tests to the root folder
- NEVER commit secrets, credentials, or .env files
- Keep README.md updated when adding features or changing behavior
- Do what has been asked; nothing more, nothing less

## File Organization

<!-- TODO: Document your project's file structure -->
<!-- Example:
- `/src/parser/` — git diff parsing
- `/src/state/` — SQLite persistence, hunk hashing, staleness
- `/src/tui/` — ratatui interactive review interface
- `/tests/` — integration and unit tests
- `/scripts/` — utility scripts
-->

## Architecture (Bounded Contexts)

<!-- TODO: Document your bounded contexts and module responsibilities -->
<!-- Example:
### Parser
Parses raw `git diff` output into `DiffFile` / `DiffHunk` structures. No side effects, pure transformation.

### State
SQLite persistence via rusqlite. Stores review status per hunk (keyed by SHA-256 content hash).
-->

## Build & Test

<!-- TODO: Document your build and test commands -->
<!-- Example for Rust:
```bash
cargo build                # Build
cargo test                 # Run all tests
cargo clippy               # Lint
cargo fmt --check          # Format check
```
-->

<!-- Example for Node.js:
```bash
npm install                # Install dependencies
npm test                   # Run tests
npm run lint               # Lint
npm run format:check       # Format check
```
-->

- ALWAYS run tests after code changes
- ALWAYS run linter before committing
- ALWAYS run formatter before opening PRs

## TDD Enforcement

Follow red-green-refactor:
1. Write a failing test first
2. Write minimal code to make it pass
3. Refactor while keeping tests green

Use London School (mock-first) for integration boundaries. Use real implementations for pure logic.

## Error Handling

<!-- TODO: Document your error handling strategy -->
<!-- Example for Rust:
- `thiserror` for library errors in `src/lib.rs` and modules
- `anyhow` for the binary in `src/main.rs`
-->

- No empty catch blocks or silent error swallowing
- Propagate errors appropriately; add context where useful

## Definition of Done

- [ ] All tests pass
- [ ] No linter warnings
- [ ] Code formatted
- [ ] No hardcoded paths or credentials
- [ ] Public APIs have doc comments
- [ ] Error cases handled, not ignored

## MCP Server Usage

- **context7**: Query library/framework docs BEFORE using unfamiliar APIs
- **claude-mem**: Cross-session memory only. Save decisions/patterns (brief). NEVER duplicate Linear content.
- **Linear**: Source of truth for requirements, status, and acceptance criteria. ALL tickets tracked there.

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
- `feat/eng-4-add-feature`
- `docs/eng-5-developer-tooling`
- `fix/eng-6-critical-bug`

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
- NEVER merge without a completed review from the user (enforced by hook)
- Merge to main via PR after tests pass AND user has reviewed all changes
- Merge BEFORE starting dependent work (if ENG-4 needs ENG-3, merge ENG-3 first)
- One branch per Linear ticket — no stacking features on the same branch
- Short-lived branches (1-2 days max)

### Review Gate (ENFORCED by hook)

Every PR requires the user to review changes first:
1. Agent finishes work, runs tests, pushes branch
2. User reviews all changes
3. User confirms review is complete
4. Only then can a PR be created and merged

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
- Validate refs before passing to shell commands
- Never pass unsanitized user input to system commands
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

<!-- TODO: Document project-specific anti-patterns -->
<!-- Example for Rust:
- Do NOT use `unwrap()` in library code; reserve for tests only
-->
<!-- Example for Node.js:
- Do NOT use synchronous I/O in request handlers
-->
