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

## Resource Constraints

- **12GB RAM available.** Each teammate (new Claude Code session in tmux) costs ~4GB. Subagents (same session) cost minimal RAM.
- Teammate limit: **3 concurrent teammates** + orchestrator (12GB / 4GB)
- Subagent limit: **5 concurrent subagents** (shared session, much lighter)
- **Prefer subagents** for parallelism — 5 subagents > 3 teammates for the same RAM
- When resuming after a crash, always check for stale teams/worktrees before spawning new agents.
- If a session feels sluggish or memory-constrained, reduce agent count before it becomes an OOM.

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

### Hook Testing

- After modifying any hook script in `.claude/hooks/`, run `bash tests/hooks/run_hook_tests.sh`
- When creating test files, be aware that `protect-hooks.sh` may block writes to hook-adjacent paths — if blocked, report to team lead immediately
- Ensure git test environments have `user.email` and `user.name` configured

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

**UserPromptSubmit (every prompt):**
1. `.claude/hooks/enforce-ticket.sh` (blocking — requires branch with ticket ID)
2. Claude-Flow routing hook (non-blocking)
3. `.claude/hooks/workflow-reminder.sh` (advisory)

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
- **Model tiers:** Opus (3: planner, red-teamer, senior-coder), Sonnet (7: requirements-interviewer, explorer, architect, coder, tech-lead, reviewer, qa), Haiku (4: junior-coder, documentation, explainer, optimizer)
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

## Memory & Observation

- Memory observer agents are DEPRECATED — use `/checkpoint` skill instead
- If observers must be used: only record what was actually observed. Never fabricate details.
- If a session has minimal activity, record that limitation explicitly rather than padding with assumptions

## Data Workflow (ENFORCED)

- **Linear** = single source of truth for requirements, status, and acceptance criteria
- **claude-mem** = cross-session decision memory (supplements Linear, never duplicates)
- **Local plan files** = ONLY the planner agent writes these (`plans/<feature>-plan.md`), for code-level implementation detail too granular for Linear. All other agents write to Linear comments.
- Requirements, specs, critiques, and review results go to **Linear comments**, not local files

## Git Workflow

### No Ticket, No Work (ENFORCED by hook)

Every piece of work MUST have a Linear ticket before starting:
- Create the ticket in Linear FIRST
- Create a branch referencing the ticket ID
- The `enforce-ticket.sh` hook blocks prompts on branches without ticket IDs
- This applies to ALL work: features, fixes, docs, chores

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

## Agent Delegation: Teams vs Subagents

Agent teams are enabled (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`). Use the right tool for the job.

### Decision Framework: When to use teams vs subagents

**Use a SUBAGENT (Task tool, no team_name)** when:
- Single agent, single task, no peer review needed
- Agent reports results back to orchestrator and is done
- Multiple independent tasks in parallel (multiple Task calls)
- Quick fixes, verifications, research, exploration

**Use a TEAM (TeamCreate + team_name)** when:
- Agents need to **communicate with each other** (review, feedback, challenges)
- **Tech-lead review cycle**: coder implements → tech-lead reviews → coder fixes → tech-lead re-approves
- **3+ tasks with dependencies** where agents self-claim from shared task list
- **Long-lived agents** that do multiple sequential tasks
- **Competing hypotheses** where agents debate and challenge each other
- User explicitly wants visible tmux panes to observe/interact

**Default: subagent.** Only escalate to team when peer communication adds value.

### Team Peer Communication (THE WHOLE POINT)

Teams exist so agents can message each other directly. If agents only report to the lead, use subagents instead.

**Required communication patterns when using teams:**
- **Tech-lead + coder**: tech-lead reviews coder's changes via `SendMessage`, sends APPROVED or feedback. Coder fixes issues and notifies tech-lead.
- **Parallel coders on related files**: coordinate interfaces and shared types via `SendMessage`
- **Researcher + implementer**: researcher shares findings directly with coder, not just the lead

**Anti-pattern**: spawning a team where every agent only talks to the lead. That's subagents with extra overhead.

### Team Spawning Workflow

1. `TeamCreate` — create a named team (creates shared task list)
2. `Task` with `team_name` and `name` — spawn teammates into the team
3. Teammates communicate via `SendMessage` (peer-to-peer, not just to lead)
4. `SendMessage` with `type: "shutdown_request"` — gracefully stop teammates
5. `TeamDelete` — clean up team resources (only after all teammates shut down)

### Subagent Spawning Workflow

1. `Task` with `subagent_type` — no team_name, no TeamCreate
2. Agent does its work and returns results
3. No shutdown ceremony needed

### Rules (both modes)

- Use `haiku` model for lightweight/test agents to save tokens
- Each agent gets its own context window; they do NOT inherit conversation history
- Provide full task context in the spawn prompt
- Avoid assigning multiple agents to the same file to prevent conflicts
- **RAM constraints (12GB available):**
  - Teammates: each spawns a NEW Claude Code session (~4GB RAM each) → max **3 teammates** + orchestrator
  - Subagents: share orchestrator's session (minimal RAM) → max **5 subagents** concurrently
  - This is why subagents are the default — you get more parallelism for the same RAM
  - Previous sessions crashed with OOM from excessive teammate spawning

### Enforcement Hooks Awareness

- Enforcement hooks (`enforce-ticket`, `protect-hooks`, `enforce-orchestrator-delegation-v2`) can block Claude's own edits and bash commands. When working on hook-protected paths:
  1. Check which hooks are active in `.claude/settings.json` before editing
  2. If blocked, report back to team lead rather than repeatedly retrying
  3. Never attempt to disable or bypass enforcement hooks

## Agent Routing

### Agent Pipeline (ordered workflow)

1. `requirements-interviewer` (Sonnet) — Gather and clarify requirements from the user
2. `explorer` (Sonnet) — Research libraries, APIs, prior art, technical approaches
3. `architect` (Sonnet) — Design module boundaries, data flow, type definitions
4. `planner` (Opus) — Write step-by-step implementation plan (ONLY agent that writes local plan files)
5. `red-teamer` (Opus) — Critique the plan, find bugs/edge cases/risks before implementation
6. `junior-coder` (Haiku) — Scaffolding, boilerplate, mechanical refactors from fully-specified plans
7. `coder` (Sonnet) — Standard implementation with TDD
8. `senior-coder` (Opus) — Complex/cross-cutting/performance-critical implementation
9. **`tech-lead` (Sonnet) — Cross-cutting review AFTER each implementation step, BEFORE commit** (see `.claude/agents/tech-lead.md`)
10. `reviewer` (Sonnet) — Code review after implementation (reads code, checks quality)
11. `qa` (Sonnet) — QA testing after implementation (runs things, verifies behavior, tests hooks/workflows)
12. `documentation` (Haiku) — Update README, doc comments, guides
13. `explainer` (Haiku) — Explain code at different expertise levels (junior → staff/architect)
14. `optimizer` (Haiku) — Meta-workflow audit (run after every major task completion)

### When to Use junior-coder vs coder vs senior-coder

- **junior-coder (Haiku):** Scaffolding new files from spec, struct/enum definitions, adding imports/mod declarations, moving functions between modules, writing boilerplate (constructors, getters, Display impls). Task MUST be fully specified with zero ambiguity.
- **coder (Sonnet):** Single-module changes, straightforward features, bug fixes with clear cause, test writing, anything requiring logic or design decisions
- **senior-coder (Opus):** Cross-module refactors, performance-critical paths, subtle/intermittent bugs, architecture-sensitive changes, tasks a coder failed at

### Orchestrator Rules

- The orchestrator (main session) MUST NOT write implementation code directly
- The orchestrator coordinates: spawns agents, assigns tasks, reviews results
- ALL code changes go through junior-coder, coder, or senior-coder agents
- The orchestrator MAY edit non-code files: CLAUDE.md, agent specs, hook scripts, plans
- **Use the decision framework**: subagent for independent tasks, team for peer communication
- **After each implementation step**: spawn tech-lead (subagent or team member) to review before committing
- The orchestrator commits ONLY after tech-lead approves (or for trivial/no-code changes)

## Anti-Patterns

- Do NOT shell out to `git diff` without sanitizing arguments
- Do NOT store absolute paths in the SQLite database (use repo-relative)
- Do NOT assume UTF-8 for all diff content (handle binary gracefully)
- Do NOT block the TUI event loop with synchronous I/O
- Do NOT use `unwrap()` in library code; reserve for tests only

### Shell/Regex Patterns

- When writing regex in shell hooks, always test edge cases: dashes as grep options (use `--`), underscores in character classes, special chars in branch names
- Run `cargo build` and `cargo test` after any Rust changes before reporting completion
- Prefer `grep -E` over `egrep` (deprecated) and always use `--` before patterns that might start with `-`
