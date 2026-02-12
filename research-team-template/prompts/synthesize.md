# Synthesis Prompt Template

## Task
Synthesize findings from ${num_researchers} parallel research agents into a single comprehensive report.

## Original Query
"${query}"

## Research Findings
${all_findings}

## Instructions

### Step 1: Build Source Registry
- Collect all URLs across all researcher findings
- Deduplicate: same URL → single entry with highest credibility score
- Assign sequential citation numbers [1], [2], etc.
- Note total unique sources

### Step 2: Deduplicate Claims
- Compare all claims across all subtopics
- Identical claims → merge, combine citation lists
- Similar claims (>80% overlap in meaning) → merge into best wording, keep all citations
- Related but distinct → keep separate

### Step 3: Cross-Validate
For each unique claim, determine confidence:
- **High confidence** (🟢): Supported by 2+ independent sources with credibility ≥ 3
- **Medium confidence** (🟡): Supported by 1 source with credibility ≥ 3, OR 2+ sources with credibility < 3
- **Low confidence** (🔴): Single source with credibility ≤ 2

### Step 4: Organize Themes
- Group claims by topic/theme (NOT by original subtopic assignment)
- Order themes: most well-supported first
- Identify cross-cutting patterns

### Step 5: Write Report
Generate the full report following this structure:

1. **Executive Summary** (2-3 paragraphs): Key takeaways, scope (N sources, M claims)
2. **Key Findings** (5-10 bullets): Most important findings with confidence indicators
3. **Detailed Analysis** (by theme): In-depth discussion with inline citations [N]
4. **Sources** (by credibility tier): Full citation list
5. **Confidence Statistics**: Quantitative summary
6. **Research Gaps**: What couldn't be covered, suggested follow-ups

### Rules
- Every claim must cite at least one source
- Never fabricate information beyond what sources state
- Mark contradictory findings explicitly
- Include researcher-reported gaps in the Gaps section
- Use inline citations [N] that map to the Sources section
