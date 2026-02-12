# Skills Architecture Deep Research — Wave 1 Raw Findings

## Status
- Wave 1: 10/10 researchers COMPLETE
- Wave 2: 5/10 spawned (11-15), 5 remaining (16-20)
- Background agents running: a22619b (11), a1c92cf (12), a0b85a6 (13), a826fa4 (14), a2aec14 (15)
- Still need to spawn: 16 (MCP Tool Wrapping), 17 (Skills as Capability Metrics), 18 (R1-R6 via Skills), 19 (R7-R12 via Skills), 20 (Portable Skills Framework)

## Interview Summary
- **Goal**: Implementation plan (actionable tickets + file specs)
- **Skills model**: Layered hierarchy — atomic skills wrap tools, composite skills compose workflows
- **Architecture**: Research both unified (skills subsume agent specs) and separate (skills as glue)
- **Core deliverable**: CLAUDE.md → skills decomposition
- **Key features**: Progressive context loading, dual invocation, skills-as-evals, hooks→skills migration
- **R1-R12**: Let research determine which recommendations skills address
- **Portability**: Project-agnostic core, git-review specifics on top
- **Depth**: 20 researchers in 2 waves of 10

---

## Researcher 1: Claude Code Skills System Internals

### Key Claims (13 total)

1. **Three-level progressive disclosure**: Only metadata pre-loaded at startup, full SKILL.md loads on relevance, additional files load on-demand. No practical limit on bundled content.
   - Source: code.claude.com/docs/en/skills (credibility: 5)

2. **YAML frontmatter fields**: Required: `name`, `description`. Optional: `disable-model-invocation`, `user-invocable`, `allowed-tools`, `context`, `agent`.
   - Source: code.claude.com/docs/en/skills (credibility: 5)

3. **Nested discovery with hierarchy**: enterprise > personal > project priority. Claude Code scans user settings, project settings, plugin-provided, and built-in skills.
   - Source: code.claude.com/docs/en/skills (credibility: 5)

4. **Meta-tool architecture**: Skills are injected instructions, not separate processes. A "Skill" tool container dispatches to individual skills based on LLM reasoning.
   - Source: mikhail.io/2025/10/claude-code-skills/ (credibility: 3)

5. **String substitution**: $ARGUMENTS, $ARGUMENTS[N], $N, ${CLAUDE_SESSION_ID} for dynamic context.
   - Source: code.claude.com/docs/en/skills (credibility: 5)

6. **Dynamic context injection**: `!`command`` syntax executes shell commands before sending skill content, replacing placeholders with live data (git diffs, API data, DB queries).
   - Source: code.claude.com/docs/en/skills (credibility: 5)

7. **Subagent execution**: `context: fork` creates isolated context where skill content becomes task prompt. `agent` field specifies execution environment.
   - Source: code.claude.com/docs/en/skills (credibility: 5)

8. **Tool access control**: `allowed-tools` field limits which tools Claude can use when skill is active.
   - Source: code.claude.com/docs/en/skills (credibility: 5)

9. **Dual invocation control**: `disable-model-invocation: true` = user-only. `user-invocable: false` = Claude-only. Default: both can invoke.
   - Source: code.claude.com/docs/en/skills (credibility: 5)

10. **Context loading distinction**: Descriptions always loaded for discovery, full content only on invocation. Subagents with preloaded skills get full content at startup.
    - Source: code.claude.com/docs/en/skills (credibility: 5)

11. **Bundled files**: SKILL.md is required entry point. Supporting files (templates, examples, scripts, reference docs) load only when referenced. Keep SKILL.md under 500 lines.
    - Source: code.claude.com/docs/en/skills (credibility: 5)

12. **No system prompt modification**: Skills inject into conversation history, not system prompt.
    - Source: mikhail.io/2025/10/claude-code-skills/ (credibility: 3)

13. **Character budget**: 2% of context window (fallback 16,000 chars). Override with SLASH_COMMAND_TOOL_CHAR_BUDGET env var.
    - Source: code.claude.com/docs/en/skills (credibility: 5)

---

## Researcher 2: Claude Code Agent Specs System

### Key Claims (15 total)

1. **Agent specs**: Markdown files with YAML frontmatter in .claude/agents/ (project) or ~/.claude/agents/ (user). Body becomes system prompt.

2. **Model routing**: `model` field with options: sonnet, opus, haiku, inherit (default).

3. **Tool restrictions**: `tools` (allowlist) or `disallowedTools` (denylist). Special `Task(agent_type)` syntax for spawnable subagent control.

4. **Permission modes**: default, acceptEdits, dontAsk, delegate, bypassPermissions, plan.

5. **Skill preloading**: `skills` field injects full skill content into subagent context at startup. Example: `skills: [api-conventions, error-handling-patterns]`.

6. **Persistent memory**: `memory` field with scopes: user (~/.claude/agent-memory/), project (.claude/agent-memory/), local (.claude/agent-memory-local/).

7. **Lifecycle hooks**: `hooks` field with PreToolUse, PostToolUse, Stop events.

8. **MCP server config**: `mcpServers` field for referencing pre-configured servers or inline definitions.

9. **Creation**: Via /agents command (interactive) or manual Markdown files. Reload with /agents.

10. **Priority resolution**: CLI flags > project .claude/agents/ > user ~/.claude/agents/ > plugin directory.

11. **Built-in types**: Explore (Haiku, read-only), Plan (read-only), general-purpose (all tools), Bash, statusline-setup, Claude Code Guide.

12. **Task tool spawning**: Supports background and foreground execution, up to 10 concurrent tasks with queuing.

13. **Minimal required fields**: Only `name` and `description` required. All others optional with defaults.

14. **LLM-driven delegation**: Claude uses agent descriptions to decide delegation. No explicit algorithmic routing.

15. **No nested spawning**: Subagents cannot spawn other subagents. Use Skills or chain from main conversation.

---

## Researcher 3: Claude Code Hooks Architecture

### Key Claims (22 total)

1. **14 hook event types**: SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, Notification, SubagentStart, SubagentStop, Stop, TeammateIdle, TaskCompleted, PreCompact, SessionEnd.

2. **Three handler types**: command (shell), prompt (LLM single-turn), agent (multi-turn with tools up to 50 turns).

3. **Exit codes**: 0 = success (JSON processed), 2 = blocking error (action blocked), other = non-blocking advisory.

4. **Configuration**: settings.json with three levels: event name → matcher group → hook handlers. Stored at user/project/local/managed scopes.

5. **PreToolUse control**: permissionDecision (allow/deny/ask), updatedInput to modify tool params, additionalContext injection.

6. **PostToolUse**: Cannot undo but can provide feedback with decision: 'block'. updatedMCPToolOutput can replace MCP tool output.

7. **Stop hooks**: Can prevent stopping with decision: 'block'. Must check stop_hook_active to prevent infinite loops.

8. **SessionStart**: CLAUDE_ENV_FILE for persisting environment variables. additionalContext for context injection.

9. **Async hooks**: async: true runs in background without blocking. Results delivered on next turn.

10. **Timeouts**: 600s command, 30s prompt, 60s agent (all configurable).

11. **Cannot trigger tools**: Hooks communicate only through stdout/stderr/exit codes. Cannot trigger slash commands or tool calls.

12. **Parallel execution**: All matching hooks run in parallel. Identical commands deduplicated.

13. **Enterprise control**: allowManagedHooksOnly setting blocks user/project hooks.

14. **Skills/agents can define hooks**: Scoped to component lifetime, auto-cleaned on finish.

15. **MCP tool matching**: mcp__<server>__<tool> naming pattern with regex matchers.

16. **once: true**: Skills support single-execution hooks that auto-remove.

17. **PermissionRequest vs PreToolUse**: PermissionRequest only fires when permission dialog would show.

18. **TeammateIdle/TaskCompleted**: Exit-code-only control (no JSON decision).

19. **Blocking vs Advisory**: PreToolUse, PermissionRequest, UserPromptSubmit, Stop = blocking. PostToolUse, Notification = advisory.

---

## Researcher 4: OpenAI AGENTS.md & Scoping

### Key Claims (9 total)

1. **Three-level hierarchy**: global (~/.codex), project (root to cwd), directory-level overrides. Merge precedence: root-down, closer files override.

2. **AGENTS.override.md**: Completely supersedes AGENTS.md at that directory level.

3. **32 KiB merge limit**: project_doc_max_bytes stops merging when combined size hits limit.

4. **Progressive disclosure three layers**: metadata (names/descriptions), instruction (full SKILL.md), resource (supporting files on-demand).

5. **On-demand prevents cognitive degradation**: Agents perform worse with excess irrelevant info upfront.

6. **CLAUDE.md-centric waste**: 2000+ line files consume 20% of context before work begins. Alternative: ~50 lines + contextual docs.

7. **OpenAI's own AGENTS.md**: Domain-specific: Rust conventions, TUI guidelines, testing standards, API rules.

8. **Progressive disclosure scales better than MCP**: Skills navigate directories discovering files as needed. MCP loads everything.

9. **Directory-scoped discovery**: Codex walks directory tree; Claude uses single CLAUDE.md + optional nested instructions.

---

## Researcher 5: Progressive Context Loading Patterns

### Key Claims (5 total)

1. **Three-tier loading**: metadata/index first → full content on-demand → deep details when needed. 2000-line CLAUDE.md = 20% context waste.

2. **Context budgeting**: RCR-Router (arxiv) implements role-aware context routing with token budgets per agent and semantic relevance scoring. Can halve LLM agent deployment costs.

3. **Context engineering**: Anthropic: "find the smallest set of high-signal tokens that maximize desired outcome." Maintain lightweight identifiers, load data at runtime via tools.

4. **On-demand skill activation**: Skills represented as metadata initially. Full content loads only when needed. Context cost grows with usage, not installed count.

5. **Memory type separation**: Episodic, procedural, semantic memory assembled dynamically per task. Relevance tracking monitors which context pieces influence reasoning.

---

## Researcher 6: Skill Composition Design Patterns

### Key Claims (9 total)

1. **Atomic skills**: Single-responsibility, independently reusable. Atomic Agents framework uses atomicity as core principle.

2. **Composite skills with middleware**: Compose atomic skills into multi-tool workflows. Progressive disclosure keeps context efficient.

3. **Hierarchical composition**: Leader agents coordinate subtasks, delegate to subordinate agents. AgentOrchestra framework.

4. **Control Plane as a Tool**: Abstraction layer routing requests to appropriate backends with policies and semantic matching.

5. **Six-layer model**: Autonomous AI Agents, Interoperability Layer, Coordination Module, Knowledge Base, Adaptability Engine, Monitoring & Feedback Loop.

6. **Tool wrapper requirements**: Consistent response shapes (Unix pipe-like), batch support, multiple abstraction levels. Descriptions optimized for LLM, not humans.

7. **Simplicity-first**: Anthropic: most successful implementations use simple composable patterns, not complex frameworks. Augmented LLM → Workflows → Agents progression.

8. **SKILL.md standard**: Agent Skills (inference.sh) defines format with YAML frontmatter. Supported by Claude Code, ChatGPT, Cursor, Copilot.

9. **COMPACT pattern**: Atomic capabilities systematically combine into complex ones with explicit complexity control.

---

## Researcher 7: MCP Protocol Tool Architecture

### Key Claims (10 total)

1. **JSON-RPC 2.0**: tools/list for discovery, tools/call for invocation. Three primitives: Tools (executable), Resources (read-only), Prompts (templates).

2. **Client-server architecture**: One client per server, dedicated connections. Capability negotiation during initialization handshake.

3. **Dynamic discovery**: Agent-agnostic, pagination support, notifications/tools/list_changed for real-time updates.

4. **Four composition patterns**: Client-side selection, virtual server composition, dynamic tool selection (50%+ token savings via RAG-MCP), programmatic code generation.

5. **MCP-Zero**: Active discovery framework — agents identify capability gaps and request specific tools on-demand.

6. **Skills vs MCP efficiency**: Skills ~100 tokens metadata at startup. MCP tens of thousands of tokens per server definition upfront.

7. **MCP connects to data; Skills teach procedures**: Complementary, not competing.

8. **Security**: Human-in-the-loop required. Server-side validation, rate limiting, client-side confirmation.

9. **MCP limitations**: Context bloat beyond 2-3 servers, static discovery per initialization, latency overhead for remote servers.

10. **Tool naming**: `<category>_<operation>` pattern. Names must be unique within server namespace.

---

## Researcher 8: Agent-Tool Binding in Production

### Key Claims (10 total)

1. **Dynamic tool discovery via registries**: Prevents context window saturation. 6-step process starting with discovery as step 0.

2. **Three routing methods**: Rule-based → ML-based → LLM-based (modern standard).

3. **Multi-agent routing patterns**: Single-agent, multi-agent (parallel), hierarchical (progressive refinement across tiers).

4. **AGENTS.md cross-tool standard**: Capability hints across Claude Code, Cursor, Copilot without hardcoding.

5. **MCP adoption**: GitHub Copilot, Cursor, Windsurf all use MCP for standardized tool invocation.

6. **Control mechanisms**: Copilot (security-first, no main branch), Cursor (diff review), Windsurf (human checkpoints).

7. **Stochastic complexity**: Each function call is an LLM invocation with management overhead.

8. **Agent Name Service (ANS)**: PKI-based discovery framework for verifiable agent identity and capabilities.

9. **Developer priorities 2026**: Token efficiency, productivity gains, code quality, repo understanding, privacy > raw capability.

10. **Execution layer underestimated**: Knowing which tool to call is trivial vs infrastructure to call it successfully.

---

## Researcher 9: Capability Evaluation Frameworks

### Key Claims (9 total)

1. **Two-dimensional taxonomy**: What to evaluate (behavior, capabilities, reliability, safety) × How to evaluate (code-based, LLM-judge, human).

2. **pass@k vs pass^k**: pass@k = at least one success in k attempts. pass^k = all k succeed. Informs task routing decisions.

3. **Component + end-to-end metrics**: DeepEval: PlanQualityMetric, ToolCorrectnessMetric, ArgumentCorrectnessMetric, TaskCompletionMetric, StepEfficiencyMetric.

4. **Execution traces for production eval**: Langfuse captures traces for async evaluation without blocking. Edge cases from production feed into test datasets.

5. **Capability-based routing**: Agent config tables queried for capabilities. Failed traces generate prompt adjustments. Weekly retrospectives on failures.

6. **Diagnostic eval design**: Unambiguous tasks with reference solutions. Balanced test sets. Start at low pass rate (capability frontier).

7. **Benchmarks**: AgentBench (8 environments), SWE-bench, WebArena, AgentBoard for realistic multi-step tasks.

8. **Non-determinism as first-class concern**: pass^k for consistency measurement. Repeated execution required for reliable assessment.

9. **Continuous production measurement**: KPIs: intent accuracy, routing latency, task completion, escalation frequency, cost tracking.

---

## Researcher 10: CLAUDE.md Best Practices

### Key Claims (11 total)

1. **Under 300 lines, ideally <60**: HumanLayer's root CLAUDE.md is <60 lines. Frontier LLMs follow ~150-200 instructions reliably.

2. **System prompt already has ~50 instructions**: Limited remaining capacity for CLAUDE.md content.

3. **Instruction degradation is uniform**: Not just new content ignored — ALL instructions degrade as count increases. Smaller models suffer exponentially worse.

4. **Progressive disclosure via separate files**: agent_docs/ directory with file:line references. Claude accesses detail only when needed.

5. **@path/to/import syntax**: Claude Code supports importing external files into CLAUDE.md.

6. **Nested CLAUDE.md in subdirectories**: Closest file takes precedence. Prevents truncation for monorepos.

7. **Skills for occasional knowledge**: Domain-specific knowledge that doesn't apply every session → use skills, not CLAUDE.md.

8. **Ruthless pruning test**: "Would removing this cause Claude to make mistakes?" If no, delete it.

9. **Splitting reduces merge conflicts**: Multi-team codebases benefit from modular configuration.

10. **Context window degrades as it fills**: Performance is not linear. Earlier instructions get "forgotten."

11. **Subagents isolate context bloat**: Investigations in subagents keep main session clean.
