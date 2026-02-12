# Deep Research Team Template - Requirements Specification

## Overview

A self-contained, copy-paste-ready research template that replicates Gemini Deep Research and ChatGPT Deep Research capabilities using Claude Code's Task tool with parallel haiku subagents. The system decomposes a user query into subtopics, dispatches parallel researcher subagents to search and fetch sources, scores credibility, cross-validates findings, and synthesizes results into a coherent cited report.

## Functional Requirements

### FR-1: Query Decomposition

The orchestrator decomposes the user's research query into N non-overlapping subtopics, where N depends on the selected depth level:
- **Quick**: 3 subtopics (broad coverage, minimal overlap)
- **Medium**: 5 subtopics (balanced depth and breadth)
- **Deep**: 10 subtopics (comprehensive, fine-grained analysis)

Each subtopic includes:
- Clear focus area with no overlap with other subtopics
- Relevant keywords and search terms
- Search strategy (academic sources, industry news, technical docs, community forums)
- Coverage rationale explaining why this subtopic matters

### FR-2: Parallel Research Execution

All N researcher subagents spawn simultaneously in a SINGLE orchestrator message to achieve true parallelism. Each researcher subagent:
- Receives a specific subtopic assignment with keywords and search strategy
- Independently searches the web using WebSearch tool
- Fetches and reads sources using WebFetch tool
- Extracts relevant facts, claims, and data points
- Returns structured JSON results to the orchestrator

No sequential waiting between researchers. All work happens concurrently.

### FR-3: Source Credibility Scoring

Every source receives a credibility score from 1 (lowest) to 5 (highest):

| Score | Description | Examples |
|-------|-------------|----------|
| 5 | Academic papers, official documentation, government/standards body publications | IEEE papers, arXiv, official language docs, government reports, W3C specs |
| 4 | Established news outlets, industry reports, major tech company blogs | TechCrunch, Wired, Google/Microsoft engineering blogs, industry whitepapers |
| 3 | Technical blogs by recognized authors, well-known community sites | Medium posts by recognized experts, popular dev blogs, known community forums |
| 2 | Forums, personal blogs, social media posts from non-experts | Stack Overflow, Reddit, GitHub issues, personal blogs |
| 1 | Unverified sources, content farms, pages with no clear authorship | Anonymous blogs, content aggregators, no clear attribution |

Researchers apply scoring rules consistently based on domain authority, author credentials, and publication type.

### FR-4: Cross-Validation and Confidence Assessment

After all researchers return findings, the system applies cross-validation:

- **High confidence** (🟢): Supported by 2+ independent sources with credibility ≥ 3
- **Medium confidence** (🟡): Supported by 1 source with credibility ≥ 3, OR 2+ sources with credibility < 3
- **Low confidence** (🔴): Single source with credibility ≤ 2

Independence is determined by ALL of:
- Different domains (not just different URLs on the same site)
- Different authors OR different organizations
- Original reporting (not republished/copied content)

**Note**: Same publication date does NOT prevent sources from being independent.

### FR-5: Deduplication

The system performs two levels of deduplication:

**Source-level deduplication:**
- Same URL accessed by multiple researchers → merge entries, keep highest credibility score
- Preserve all extracted facts from any access

**Fact-level deduplication:**
- Claims with >80% semantic similarity → merge into single fact
- Preserve ALL source citations (union of all sources that mentioned the fact)
- Similarity calculated via embedding cosine similarity or fuzzy text matching
- Keep the most complete/detailed version of the claim

### FR-6: Synthesis

A single sonnet subagent (deep-synthesizer) aggregates all findings into a structured report:

1. Receives all researcher outputs (deduplicated, scored, validated)
2. Identifies common themes and cross-cutting insights
3. Organizes findings by theme, not by subtopic
4. Writes executive summary highlighting top insights
5. Produces detailed analysis with inline citations
6. Groups sources by credibility tier
7. Identifies research gaps and limitations

The synthesizer prioritizes high-confidence findings but includes lower-confidence findings with appropriate caveats.

### FR-7: Output Format

The final report is a markdown document with these sections:

1. **Executive Summary**: 3-5 paragraph overview of key insights
2. **Key Findings**: Bullet list of top 8-12 findings with confidence indicators
3. **Detailed Analysis**: Thematic sections with full explanations and inline citations
4. **Sources**: Grouped by credibility tier (5→1), with URLs and descriptions
5. **Confidence Statistics**: Summary of high/medium/low confidence finding counts
6. **Research Gaps**: Topics that lacked sufficient sources or had conflicting information

Each claim includes inline citations in format: `[Source Title](URL)` or `[1][2][3]` with footnotes.

Confidence indicators:
- 🟢 High confidence (2+ sources, credibility ≥3)
- 🟡 Medium confidence (1 credible source OR 2+ lower-credibility)
- 🔴 Low confidence (single source, credibility ≤2)

## Non-Functional Requirements

### NFR-1: Performance

- **Deep mode** (10 agents): Completes in under 10 minutes on average internet connection
- **Medium mode** (5 agents): Completes in under 5 minutes
- **Quick mode** (3 agents): Completes in under 3 minutes

Timing measured from query submission to final report delivery. Assumes no rate limiting from search APIs.

### NFR-2: Token Efficiency

Approximate token usage by depth level:

| Depth | Agents | Orchestrator | Researchers (total) | Synthesizer | Total Est. |
|-------|--------|--------------|---------------------|-------------|------------|
| Quick | 3 | 5K | 60K | 25K | ~100K |
| Medium | 5 | 8K | 100K | 35K | ~150K |
| Deep | 10 | 12K | 180K | 60K | ~260K |

Researchers use haiku model for cost efficiency. Synthesizer uses sonnet for quality synthesis.

### NFR-3: Portability

The template must work by copying files into any project's `.claude/` directory:

```bash
cp -r skills/deep-research <target-project>/.claude/skills/
cp agents/deep-*.md <target-project>/.claude/agents/
```

No external dependencies beyond Claude Code's built-in tools (Task, WebSearch, WebFetch). No API keys required. No database or persistent storage needed.

### NFR-4: Configurability

Depth levels selectable via skill argument:

```
/deep-research "query"                    # Default: deep (10 agents)
/deep-research --depth quick "query"      # Quick mode (3 agents)
/deep-research --depth medium "query"     # Medium mode (5 agents)
/deep-research --depth deep "query"       # Explicit deep mode (10 agents)
```

Orchestrator parses argument and adjusts subtopic count and agent spawn count accordingly.

## Acceptance Criteria

### AC-1: File Structure Completeness

All 8 template files exist in correct directory structure:

```
research-team-template/
├── README.md
├── requirements.md
├── skills/
│   └── deep-research/
│       └── SKILL.md
├── agents/
│   ├── deep-researcher.md
│   └── deep-synthesizer.md
└── prompts/
    ├── decompose.md
    ├── research.md
    └── synthesize.md
```

### AC-2: Skill Discoverability

`skills/deep-research/SKILL.md` has valid YAML frontmatter:

```yaml
---
name: deep-research
description: Run deep research using parallel AI agents
---
```

Skill appears in `/help` command output and is invocable via `/deep-research "query"`.

### AC-3: True Parallel Execution

Researcher agents spawn in parallel (single Task message with multiple `<invoke name="Task">` calls). Verify by checking orchestrator message structure:

```xml
<function_calls>
  <invoke name="Task">...</invoke>
  <invoke name="Task">...</invoke>
  <invoke name="Task">...</invoke>