# Deep Research Report: Harness Engineering & Agent-First Development

**Research date**: 2026-02-12
**Methodology**: 10 parallel haiku researchers, 1 opus synthesizer, web search across ~50 sources
**Scope**: 110+ raw claims from 10 researchers, deduplicated to 78 unique claims, 38 unique sources

---

## Executive Summary

OpenAI's "Harness Engineering" article, published in late 2025/early 2026, introduced a paradigm shift in how engineering teams interact with AI coding agents. The core thesis is that **the harness -- not the model -- is the bottleneck**: the architectural system surrounding an LLM (context management, tool orchestration, feedback loops, verification) determines agent effectiveness far more than model intelligence alone. OpenAI's internal Codex team reported building their product with "no manually-written code," achieving 3.5 PRs per engineer per day across ~1,500 PRs and ~1M lines of code over 5 months, with engineering effort shifting to 80% planning/review and 20% code generation [1][2][3].

This research synthesizes findings from 38 sources across the agent engineering landscape, comparing OpenAI's Codex approach with Anthropic's Claude Code, Aider, and other CLI-based agent tools. Three major themes emerge: (1) **context management is the critical discipline** -- progressive disclosure, scoped instructions, and aggressive context budgeting outperform large monolithic prompts; (2) **multi-agent orchestration patterns are converging** toward a Planner-Worker-Judge architecture with isolated execution environments; and (3) **quality gates and feedback loops are non-negotiable** for production use, with pre-commit hooks, agent-to-agent review, and human oversight forming a layered defense. The user's existing Claude Code setup (orchestrator + tiered subagents, enforcement hooks, Serena LSP, claude-mem) already implements many best practices, but specific optimizations around CLAUDE.md size, context budgeting, and structured output schemas can yield measurable improvements.

The landscape is maturing rapidly: 57% of enterprises have agents in production, but fewer than 25% have scaled successfully [32]. The gap is not model capability but harness engineering -- exactly the discipline this report examines.

---

## Key Findings

- **[HIGH CONFIDENCE]** The harness, not the model, is the primary bottleneck for agent effectiveness. Changing only the harness improved performance across 15 different LLMs; patch-based editing fails 46-50% for unfamiliar models without harness adaptation [1][6][26].

- **[HIGH CONFIDENCE]** AGENTS.md/CLAUDE.md should be a "table of contents, not an encyclopedia." Oversized instruction files cause "context rot" where irrelevant information degrades agent intelligence. Target ~60 lines for the primary file, with detailed guidance in linked docs/ files loaded on demand [1][10][16].

- **[HIGH CONFIDENCE]** The Planner-Worker-Judge pattern is the dominant 2026 orchestration architecture: Opus/strong models plan, Sonnet/capable models execute, evaluation agents judge output at cycle boundaries [8][12][14].

- **[HIGH CONFIDENCE]** Quality gates must exist at multiple stages (pre-commit, CI, agent-review, human-review). Agentic coding does not bypass engineering practices -- it requires more rigorous enforcement [14][15][22].

- **[HIGH CONFIDENCE]** Context isolation with scoped agent windows + strategic message passing reduces token duplication (which runs 53-86% in naive multi-agent systems) and prevents coordination overhead from growing quadratically [33][34].

- **[MEDIUM CONFIDENCE]** Prompt caching hit rate matters more than raw model cost for CLI agent economics. Changing tools, models, or sandbox configuration mid-conversation invalidates the cache [10][24].

- **[MEDIUM CONFIDENCE]** Agent-to-agent code review can reduce human review effort by 40-70% while maintaining or improving defect rates, but requires structured feedback formats and escalation thresholds [15][19].

- **[MEDIUM CONFIDENCE]** Time allocation inverts under agent-first development: 80% planning/review, 20% code generation. Planning becomes the bottleneck, not execution [1][3].

- **[MEDIUM CONFIDENCE]** Infrastructure complexity dwarfs agent logic: 70-80% of effort goes to infrastructure (sandboxing, context management, tool orchestration) vs 20-30% on agent behavior itself [26].

- **[LOW CONFIDENCE]** Architectural assumptions become obsolete every ~6 months with new model releases, suggesting harness designs should prioritize adaptability over optimization for current capabilities [26].

---

## Detailed Analysis

### 1. The Harness Engineering Philosophy

OpenAI defines the "agent harness" as the **complete architectural system surrounding an LLM that manages the lifecycle of context**: intent capture, specification, compilation, execution, verification, and persistence [1][2]. This is not just prompt engineering -- it encompasses the entire developer experience infrastructure.

The key insight, validated across multiple sources, is that **changing the harness improved 15 different LLMs**, while model upgrades without harness improvements yielded diminishing returns [6][26]. This finding directly supports the user's investment in enforcement hooks, agent specs, and structured workflows -- these are harness components, and they matter more than which model tier an agent uses.

OpenAI's Codex team adopted a "no manually-written code" philosophy where engineering effort shifted entirely to enabling agents [1][3]. Their reported metrics -- 3.5 PRs per engineer per day, ~1M LOC over 5 months with 3 engineers (later 7) -- represent a 10x throughput claim compared to manual development [1]. While impressive, these numbers should be interpreted cautiously: the codebase was greenfield (started August 2025 from an empty repo), the team had privileged access to frontier models (codex-1, an o3 variant), and the product being built was itself an agent tool [1][24].

The "Delegate, Review, Own" framework provides a practical taxonomy [1]:
- **Delegate**: independent, well-scoped tasks agents handle autonomously
- **Review**: agent output requiring human oversight before merging
- **Own**: high-risk decisions, architecture, security -- humans retain full control

This maps well to the user's existing tiered model routing (Haiku for mechanical tasks, Sonnet for implementation, Opus for planning/critical decisions) and the tech-lead review gate that runs after each implementation step.

**Depth-first decomposition** is the recommended approach to task breakdown: break goals into building blocks (design, code, review, test), prompt the agent to construct them, use completed blocks to unlock increasingly complex tasks [1][2]. When something fails, the diagnostic question is "what capability is missing?" rather than "what prompt fix do I need?" This reframes agent failures as harness gaps rather than model limitations.

### 2. Context Management: Progressive Disclosure vs Monolithic CLAUDE.md

Context management emerged as the single most impactful harness engineering discipline across all 10 research threads. Multiple high-credibility sources converge on the same principle: **show agents only what they need now, with breadcrumbs to more detailed guidance** [1][10][11].

**The "Map Not a Manual" Pattern**: AGENTS.md (OpenAI) and CLAUDE.md (Anthropic) should serve as structural overviews consuming minimal tokens, with selective retrieval of detailed guidance from linked files [1][10]. OpenAI explicitly recommends AGENTS.md as a "table of contents, not an encyclopedia" [1]. Research Claim 3 from Researcher 5 cites a specific target: **CLAUDE.md should stay under ~60 lines**, given that LLMs follow ~150-200 instructions consistently and the system prompt already consumes ~50 of that budget [16].

**Implication for the user's setup**: The current `/home/feysen/dev/personal/repos/git-review/CLAUDE.md` is extensive (300+ lines). While thorough, this likely exceeds the effective instruction-following capacity. The recommended approach:
1. Keep a slim top-level CLAUDE.md (~60 lines) with critical rules and a directory index
2. Move detailed sections (Architecture, Agent Routing, Hook Execution Order, etc.) into `/docs/` files
3. Use Claude Code's Skills system for on-demand loading of specialized context
4. Agent specs (`.claude/agents/*.md`) already implement this pattern well -- they load only when the relevant agent type is spawned

**"Context Rot"**: Irrelevant information actively degrades agent intelligence [10]. This is not merely wasted tokens -- it causes the agent to attend to wrong details, follow outdated instructions, or conflate unrelated rules. The user's CLAUDE.md contains sections relevant to different agent types (orchestrator rules, coder rules, hook details) that create noise for any single agent invocation.

**Progressive disclosure in practice** [10][11][13]:
- Claude Code Skills: metadata-only visibility at startup, supporting files load on-demand
- Continue's layered rules: project hierarchy with nearest-directory overrides
- Codex compaction: auto-summarizes sessions approaching limits, fresh context window
- Selective tool reveal: agents retrieve tools via --help flags, filesystem discovery

**Prompt caching** is a critical economic factor: cache hit rate matters more than model cost [10][24]. Changing tools, models, or sandbox configuration mid-conversation invalidates the prompt cache [24]. This means the user's practice of spawning specialized subagents (which inherit the orchestrator session but may use different tool subsets) is generally cache-friendly, while switching between team members (separate sessions) resets caching entirely.

**Filesystem offloading** preserves information that would be lost through context compression [10]. Writing intermediate results, plans, and structured data to files -- then reading them back selectively -- is more token-efficient than keeping everything in the conversation. The user's `plans/` directory and `agent-tasks.md` already implement this pattern.

### 3. Orchestration Patterns: Building Blocks & Task Decomposition

The multi-agent orchestration landscape has converged around several well-defined patterns [8][12]:

**Sequential orchestration**: Linear chain with dependencies, progressive refinement. The user's agent pipeline (requirements-interviewer -> explorer -> architect -> planner -> red-teamer -> coder -> tech-lead -> reviewer -> qa) is a textbook sequential chain [8].

**Concurrent orchestration**: Parallel agents on the same task for diverse perspectives. Useful for the user's red-teamer (challenging the planner's output) and reviewer (evaluating the coder's output) stages.

**The 2026 Consensus Pattern -- Planner-Worker-Judge** [8][12]:
- **Planners** (Opus-tier) explore the problem space and create task specifications
- **Workers** (Sonnet/Haiku-tier) execute independently in isolated environments
- **Judges** (evaluation agents) assess output at cycle boundaries

This maps directly to the user's setup:
| Role | User's Agent | Model |
|------|-------------|-------|
| Planner | planner, architect | Opus, Sonnet |
| Worker | coder, junior-coder, senior-coder | Sonnet, Haiku, Opus |
| Judge | tech-lead, reviewer, qa | Sonnet |

**OpenAI's Codex orchestration** follows a similar pattern but with a key difference: Codex uses **agent-to-agent review** where the coding agent requests reviews from specialized review agents in the cloud, responds to feedback, and iterates until all reviewers are satisfied [1][2]. Almost all review effort is pushed to agent-to-agent interaction, with humans providing final oversight only.

**The "depth-first decomposition" approach** [1][2] deserves special attention:
1. Define the goal
2. Break into building blocks (design, code, review, test)
3. Prompt agent to construct each block
4. Use completed blocks to unlock complex tasks
5. When failures occur, ask "what capability is missing?"

This is subtly different from the user's current approach (which is more top-down planning followed by execution). Depth-first decomposition is more iterative and capability-driven -- it builds up from working components rather than decomposing down from requirements.

**Git worktrees** are confirmed as the standard isolation mechanism for parallel agent execution [8][12]. The user already uses `.trees/<ticket-id>/` for this purpose, which is aligned with industry practice.

**Task specification quality** is critical for lower-tier models [16]. The user's distinction between junior-coder (fully specified, zero ambiguity required) and coder (can make design decisions) reflects the research finding that task decomposition quality directly determines agent success rate.

### 4. Feedback Loops & Quality Gates

The research strongly validates the user's enforcement hook architecture while suggesting specific enhancements.

**Layered quality gates** are essential [14][15][22]:
1. **Pre-commit hooks** (local): The user's `.claude/hooks/` directory implements blocking enforcement (protect-hooks, enforce-orchestrator-delegation, protect-main, enforce-branch-naming, enforce-review-gate) with advisory hooks (check-unwrap, scan-secrets) providing additional signal.
2. **Agent review** (automated): The tech-lead review step after each implementation acts as an automated quality gate.
3. **CI/CD gates** (remote): Not yet fully implemented in the user's setup (ENG-41 tracks GitLab CI integration).
4. **Human review** (final): The git-review TUI tool with per-hunk tracking provides the human gate.

**The HULA framework** (Atlassian) implements review checkpoints at plan, code, and PR stages, achieving a 59% merge rate [15]. The user's pipeline already has checkpoints at plan (red-teamer critique), code (tech-lead review), and PR (human git-review) stages.

**Evaluation methodology** should combine three types [14]:
- **Code-based evals**: automated tests, linting, type checking (user has: cargo test, cargo clippy, cargo check)
- **Model-based evals**: LLM-as-judge reviewing code quality (user has: tech-lead and reviewer agents)
- **Human graders**: manual review of output quality (user has: git-review TUI)

**Dual testing strategy** [14]:
- **Capability evals**: measure improvement over time (are agents getting better?)
- **Regression evals**: maintain near-100% pass rate (did we break anything?)

The user's `enforce-task-quality.sh` hook (runs `cargo test` + `cargo clippy` before task completion) implements regression evaluation. Capability evaluation is less formalized and could be enhanced.

**A specific pain point** identified across sources: agents struggle to detect and re-stage files modified by pre-commit hooks [15]. If the user's pre-commit hooks modify files (e.g., auto-formatting), agents may not properly re-stage the changes, leading to commit failures.

**Confidence thresholds with escalation** [15]: agents should be configured to escalate to humans when confidence is below a threshold, providing explainability artifacts. The user's "three-tier boundaries" (Always Do / Ask First / Never Do) in hook configuration already implements a form of this.

### 5. CLI Agent Landscape: Codex vs Claude Code

The CLI agent landscape has three philosophical camps [28]:

**Provider-Native** (Claude Code, Codex CLI): Deep integration with the provider's model, optimized end-to-end, proprietary conventions (CLAUDE.md vs AGENTS.md).

**Model-Agnostic** (Aider): Works with 50+ LLM providers, git-native workflows, auto-commits, oldest and most battle-tested (39K stars, 4.1M installs) [28][29].

**Ecosystem-Specialist** (GitHub Copilot CLI): Native GitHub integration, MCP extensibility, multi-model support [28].

**Head-to-head comparison** [28][29][30]:

| Feature | Claude Code | Codex CLI | Aider |
|---------|------------|-----------|-------|
| Multi-agent | Agent teams + subagents | Single agent + cloud review | No |
| Context window | ~200K tokens | 192K tokens (stateless) | Varies by model |
| Git integration | Manual (user-driven) | Auto-commit | Auto-commit |
| Extensibility | MCP, hooks, skills | MCP | LiteLLM adapters |
| Sandbox | bwrap (user config) | Built-in cloud sandbox | None |
| Model lock-in | Claude only | OpenAI only | Model-agnostic |
| Benchmark (real-world) | 56% | N/A | 67% |

**Key architectural differences** [20][24][28]:
- **Codex**: Fully stateless (entire conversation sent each call), cloud-sandboxed, internet-disabled during execution, same harness powers all surfaces (web, CLI, IDE, macOS)
- **Claude Code**: Session-based with compaction, local execution with optional sandboxing, rich multi-agent coordination, 110+ modular system prompt components conditionally activated
- **Claude Code's harness** is cited as superior to Codex in consistency/reliability, particularly for reproducible refactoring plans across runs [26]

**Codex's strengths**: end-to-end vendor optimization (model + harness + GitHub), 30-minute autonomous execution per task, iterative TDD until tests pass [24].

**Claude Code's strengths**: richer orchestration (subagents, teams, hooks, MCP), deeper customization (CLAUDE.md, agent specs, skills), Serena LSP integration for token-efficient code navigation, more flexible deployment (not GitHub-exclusive) [20][21][28].

### 6. Pain Points & What's Actually Working

**The production gap** is real: 67% of enterprises experiment with coding agents, but fewer than 25% scale to production -- a 3:1 failure ratio [32][34]. The primary barriers are quality/consistency (cited by ~33% of enterprises) and infrastructure complexity, not model capability or cost [26][32].

**Token economics** [33][34]:
- Output tokens cost 3-8x input tokens; every hallucination wastes money
- Multi-agent token duplication: MetaGPT 72%, CAMEL 86%, AgentVerse 53%
- Effective context windows are only 50-60% of stated maximums
- Prompt caching is the primary economic lever

**Context window brittleness** [33]: Hallucinations increase with larger changes, incorrect behavior repeats within threads. The solution is context isolation with scoped agent windows + strategic injection via message passing -- which is exactly what the user's subagent architecture provides (each subagent gets a fresh context with only the relevant task specification).

**Code quality debt** from agent outputs [34]: duplicated functions, excessive comments, agents following instructions literally without refactoring. This validates the user's tech-lead review step and the need for explicit refactoring passes.

**What's actually working in production**:
1. **Enforcement hooks** preventing common mistakes (the user's approach) [14][15]
2. **Tiered model routing** matching task complexity to model capability [8][12][16]
3. **File ownership per agent** preventing merge conflicts [20][21]
4. **Structured output schemas** reducing hallucinations vs verbose prompt examples [33][34]
5. **Progressive disclosure** of context rather than monolithic instruction files [1][10]
6. **Git worktrees** for parallel agent isolation [8][12]

**Security concerns** [26]: Prompt injection attacks are "highly transferable between models." Multi-agent systems create cascading failure risks where a compromised agent can influence others through shared task lists or message passing. The user's enforcement hooks provide some defense, but explicit input sanitization at agent communication boundaries is recommended.

### 7. Actionable Recommendations for the User's Setup

Based on cross-validated findings, here are specific recommendations ordered by expected impact:

#### High Impact, Low Effort

**R1. Slim down CLAUDE.md** [1][10][16]
- Current file is 300+ lines; research suggests ~60 lines as the effective maximum
- Move Architecture, Hook Execution Order, Agent Routing, and Anti-Patterns sections to `/docs/` files
- Keep in CLAUDE.md: behavioral rules, build commands, error handling philosophy, file organization (as an index), and the most critical workflow rules
- Link to detailed docs with clear file paths

**R2. Add structured output schemas to agent task specifications** [33][34]
- When spawning subagents, include expected output format (JSON schema or structured markdown template)
- Reduces hallucination and token waste vs verbose instructions
- Example: coder tasks should specify `{ files_modified: [...], tests_added: [...], summary: "..." }`

**R3. Implement the "what capability is missing?" diagnostic** [1][2]
- When an agent task fails, before retrying with a different prompt, ask: what building block or tool is the agent missing?
- This reframes failures as harness gaps rather than prompt problems
- Track these gaps in claude-mem as harness improvement opportunities

#### High Impact, Medium Effort

**R4. Add agent-to-agent review before human review** [1][15][19]
- Currently: coder -> tech-lead review -> human review
- Enhancement: coder -> automated reviewer agent -> tech-lead -> human review
- The reviewer agent can catch style issues, test coverage gaps, and anti-patterns before the tech-lead spends tokens on deep analysis
- Expected reduction: 40-70% less human review effort [15]

**R5. Implement capability evals alongside regression evals** [14]
- Currently: `enforce-task-quality.sh` runs cargo test + clippy (regression only)
- Add: periodic measurement of agent success rates by task type, model tier, and task complexity
- Track in claude-mem: "coder succeeded/failed on [task type] with [model]"
- Use this data to refine model routing (e.g., if Haiku consistently fails on a task type, route to Sonnet)

**R6. Adopt filesystem offloading for intermediate agent results** [10]
- Write structured intermediate results to `.claude/scratch/` (gitignored)
- Agents read these files selectively rather than passing large payloads in conversation
- Particularly valuable for multi-step pipelines where results feed into subsequent agents

#### Medium Impact, Medium Effort

**R7. Implement confidence-based escalation thresholds** [15]
- Agent task specifications should include: "If uncertain about X, escalate to [higher-tier agent/human] with context Y"
- Reduces wasted tokens on low-confidence agent attempts
- The tech-lead agent should explicitly flag items where it lacks confidence rather than rubber-stamping

**R8. Add pre-commit hook awareness for auto-formatted files** [15]
- If pre-commit hooks modify files (e.g., rustfmt), agents need explicit instructions to re-stage after hook execution
- Add to agent task templates: "After committing, if pre-commit hooks modify files, re-stage and amend"

**R9. Consider context budget tracking** [10][33]
- Track approximate token usage per agent task
- Set per-task token budgets based on task complexity tier
- Alert/escalate when agents approach budget limits rather than letting context degrade silently

#### Lower Impact, Worth Monitoring

**R10. Evaluate Serena for progressive code disclosure** [10][20]
- Serena's `get_symbols_overview` already implements the "map not manual" pattern for code
- Consider extending this pattern: agent first gets symbol overview, then reads only needed function bodies
- The claimed 50-75% token savings align with progressive disclosure research

**R11. Watch the model-agnostic harness space** [26][28][29]
- Aider's model-agnostic approach provides insurance against vendor lock-in
- Open-source harness frameworks (Gambit, Agent Runner, DeepAgents) are emerging
- The user's hook-based architecture is relatively portable -- enforcement hooks are shell scripts, not Claude-specific
- Architectural assumptions may become obsolete every ~6 months with new model releases [26]

**R12. Plan for CI/CD integration** [14][15]
- ENG-41 already tracks GitLab CI integration
- Research strongly validates multi-stage quality gates (local hooks + CI + agent review + human review)
- When implemented, this closes the gap between the user's local enforcement and production-grade agent pipelines

---

## Sources

### Tier 1: High Credibility (4-5)

| # | Source | Credibility | Used By |
|---|--------|-------------|---------|
| [1] | [OpenAI: Harness Engineering](https://openai.com/index/harness-engineering/) | 5 | R1, R2, R3 |
| [2] | [OpenAI Developers: Building Codex](https://developers.openai.com/) | 5 | R1, R2, R3 |
| [3] | [Every.to: OpenAI Agent-First Engineering](https://every.to/) | 3-4 | R1 |
| [4] | [Parallel.ai: Harness Analysis](https://parallel.ai/) | 3 | R1 |
| [6] | [blog.can.ac: Harness Not Model](https://blog.can.ac/) | 5 | R6 |
| [7] | [LangChain Blog: Agent Landscape](https://langchain.com/) | 5 | R6 |
| [8] | [Microsoft Azure Architecture: Multi-Agent Patterns](https://learn.microsoft.com/) | 5 | R3 |
| [10] | [Martin Fowler: Progressive Disclosure](https://martinfowler.com/) | 5 | R2 |
| [11] | [alexop.dev: Context Management](https://alexop.dev/) | 4 | R2 |
| [12] | [solafide.ca: Agent Decomposition](https://solafide.ca/) | 4 | R3 |
| [13] | [mikemason.ca: Orchestration Patterns](https://mikemason.ca/) | 4 | R3 |
| [14] | [Anthropic: Evaluations Guide](https://anthropic.com/) | 5 | R4, R6 |
| [15] | [Atlassian: HULA Framework](https://atlassian.com/) | 5 | R4 |
| [16] | [Prompting Guide: Agent Prompt Engineering](https://promptingguide.ai/) | 5 | R5 |
| [17] | [Addy Osmani: Coding Agent Best Practices](https://addyosmani.com/) | 5 | R5 |
| [18] | [HumanLayer: Agent Boundaries](https://humanlayer.dev/) | 5 | R5 |
| [19] | [HackerOne: AI Code Review](https://hackerone.com/) | 5 | R4 |
| [20] | [Claude Code Docs](https://code.claude.com/) | 5 | R8 |
| [21] | [Anthropic: Agent Teams](https://anthropic.com/) | 5 | R8 |
| [22] | [rlancemartin.github.io: Context Engineering](https://rlancemartin.github.io/) | 3-4 | R2 |
| [24] | [InfoQ: Codex Architecture](https://infoq.com/) | 4 | R7 |
| [25] | [eesel.ai: Codex Deep Dive](https://eesel.ai/) | 4 | R7 |
| [26] | [Builder.io: Agent Landscape](https://builder.io/) | 4 | R6, R9 |
| [27] | [Hacker News: Community Discussion](https://news.ycombinator.com/) | 4 | R6 |
| [28] | [Tembo: CLI Agent Comparison](https://tembo.io/) | 5 | R9 |
| [29] | [Pinggy: Aider vs Claude Code](https://pinggy.io/) | 4 | R9 |
| [30] | [Faros.ai: Agent Benchmarks](https://faros.ai/) | 4 | R9, R10 |
| [31] | [AIMulitple: Agent Market Analysis](https://aimultiple.com/) | 3-4 | R9 |

### Tier 2: Medium Credibility (2-3)

| # | Source | Credibility | Used By |
|---|--------|-------------|---------|
| [32] | [VentureBeat: Enterprise Agent Adoption](https://venturebeat.com/) | 5 | R10 |
| [33] | [Vellum.ai: Token Economics](https://vellum.ai/) | 4 | R10 |
| [34] | [arXiv: Multi-Agent Token Duplication](https://arxiv.org/) | 5 | R10 |
| [35] | [SiliconData: Agent Quality Debt](https://silicondata.com/) | 4 | R10 |
| [36] | [GitHub: Claude Code System Prompt Analysis](https://github.com/) | 4 | R5 |
| [37] | Various community blog posts | 3-4 | R4, R8 |
| [38] | Agent framework documentation (CrewAI, LangGraph) | 3-4 | R10 |

---

## Confidence Statistics

| Metric | Value |
|--------|-------|
| **Total raw claims** | 110+ |
| **Unique claims after deduplication** | 78 |
| **Unique sources** | 38 |
| **High confidence (2+ sources, cred >= 3)** | 42 claims (54%) |
| **Medium confidence (1 source, cred >= 4, or 2+ sources cred 2-3)** | 27 claims (35%) |
| **Low confidence (single source, cred <= 3)** | 9 claims (11%) |
| **Cross-validated (3+ independent sources)** | 18 claims (23%) |

### Highest-Confidence Claims (3+ independent sources)

1. Harness > model for agent effectiveness [1][6][26][14]
2. Progressive disclosure / slim instruction files [1][10][16][20]
3. Planner-Worker-Judge orchestration pattern [8][12][14][1]
4. Multi-stage quality gates required [14][15][19][22]
5. Context isolation reduces token waste [10][33][34]
6. Git worktrees for parallel agent isolation [8][12][20]
7. File ownership per agent prevents conflicts [20][21][8]
8. Production scaling gap (67% experiment, <25% scale) [32][34][26]

---

## Research Gaps

### Inadequately Covered Areas

1. **Long-running agent session management**: How to handle multi-hour agent sessions, session persistence across crashes, context window exhaustion in long tasks. The user's auto-checkpoint hooks (ENG-39) address this partially, but industry best practices are underexplored.

2. **Agent security in multi-agent systems**: Prompt injection across agent boundaries, cascading failures from compromised agents, and defense-in-depth for shared task lists. Only surface-level treatment in the research.

3. **Cost optimization at scale**: Detailed token economics per agent architecture (subagent vs team), prompt caching hit rates in practice, and ROI measurement for different agent configurations.

4. **Serena/LSP integration patterns**: How LSP-powered code navigation compares to tree-sitter AST parsing (mentioned in research as "context engineering") and whether hybrid approaches yield better results.

5. **Agent memory consolidation**: Best practices for merging short-term agent observations into long-term project memory. The user has claude-mem + Claude-Flow memory + Serena memory, but optimal consolidation strategies are not well-studied.

6. **Model routing optimization**: Empirical data on which task types benefit most from model upgrades (Haiku -> Sonnet -> Opus) and where the diminishing returns boundary lies.

### Suggested Follow-Up Queries

1. "Context window management strategies for long-running CLI agent sessions -- compaction vs checkpointing vs session splitting"
2. "Empirical benchmarks: Haiku vs Sonnet vs Opus success rates by task type in real-world coding projects"
3. "Multi-agent security: prompt injection defenses and cascading failure prevention in agent-to-agent communication"
4. "Tree-sitter vs LSP vs hybrid approaches for agent code understanding -- token efficiency and accuracy comparison"
5. "Agent harness portability: extracting model-agnostic harness components from provider-specific agent tools"

---

*Research methodology*: 10 parallel haiku researchers conducted independent web searches on assigned subtopics, producing 110+ raw claims with source attribution and credibility ratings. This opus synthesizer deduplicated claims (>80% textual similarity merged), cross-validated across sources (2+ independent sources with credibility >= 3 for high confidence), and organized findings by theme rather than research thread. All claims cite at least one source. The synthesis prioritized actionable insights for the user's specific Claude Code setup.
