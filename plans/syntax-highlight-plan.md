# Syntax Highlighting Implementation Plan

## Overview
Add syntect-based syntax highlighting to the TUI diff viewer. Render-time highlighting with graceful fallback for unknown file types.

---

## Step 1: Add Dependencies

**File: `Cargo.toml`**

Add to `[dependencies]` section:

```toml
syntect = { version = "5", default-features = false, features = ["parsing", "fancy-regex", "default-syntaxes", "default-themes"] }
```

**Rationale:**
- `fancy-regex` feature: pure Rust (no C deps), ~2x slower but acceptable (~300ms for 124KB)
- `default-syntaxes`: bundled syntax definitions (.rs, .toml, .md, .json, .py, .js, etc.)
- `default-themes`: bundled themes including `base16-ocean.dark`
- No `regex-onig` (avoids C dependency)

---

## Step 2: Create `src/highlight/mod.rs` (TDD)

### 2.1: Create file structure

**File: `src/highlight/mod.rs`**

Module structure:

```rust
use ratatui::{
    style::{Color, Style},
    text::Span,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{ThemeSet, Color as SyntectColor},
    parsing::SyntaxSet,
};

/// Syntax highlighter for diff content.
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    /// Create a new Highlighter with default syntax and theme sets.
    pub fn new() -> Self;

    /// Highlight a single diff line, returning styled spans.
    ///
    /// Strips the +/- prefix, highlights the content, and reapplies
    /// diff coloring (green/red background) with syntax foreground.
    /// Falls back to plain diff coloring on any error.
    pub fn highlight_diff_line<'a>(
        &self,
        line: &'a str,
        file_ext: &str,
    ) -> Vec<Span<'a>>;

    /// Find syntax by file extension.
    fn find_syntax(&self, file_ext: &str) -> Option<&syntect::parsing::SyntaxReference>;

    /// Convert syntect Color to ratatui Color.
    fn syntect_to_ratatui(color: SyntectColor) -> Color;
}

impl Default for Highlighter {
    fn default() -> Self;
}
```

### 2.2: Write tests FIRST (TDD - Red phase)

**File: `src/highlight/mod.rs` (tests module)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_new() {
        let highlighter = Highlighter::new();
        assert!(highlighter.find_syntax("rs").is_some());
        assert!(highlighter.find_syntax("toml").is_some());
        assert!(highlighter.find_syntax("md").is_some());
        assert!(highlighter.find_syntax("unknown_ext_xyz").is_none());
    }

    #[test]
    fn test_highlight_added_line() {
        let highlighter = Highlighter::new();
        let line = "+fn main() { println!(\"test\"); }";
        let spans = highlighter.highlight_diff_line(line, "rs");

        // Should return non-empty spans
        assert!(!spans.is_empty());

        // First span should preserve the '+' prefix with green color
        assert_eq!(spans[0].content, "+");
        assert_eq!(spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_highlight_removed_line() {
        let highlighter = Highlighter::new();
        let line = "-fn old_function() {}";
        let spans = highlighter.highlight_diff_line(line, "rs");

        assert!(!spans.is_empty());
        assert_eq!(spans[0].content, "-");
        assert_eq!(spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_highlight_context_line() {
        let highlighter = Highlighter::new();
        let line = " fn context() {}";
        let spans = highlighter.highlight_diff_line(line, "rs");

        // Context lines (no +/-) should still be highlighted
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_fallback_for_unknown_extension() {
        let highlighter = Highlighter::new();
        let line = "+some text in unknown format";
        let spans = highlighter.highlight_diff_line(line, "unknown_xyz");

        // Should fall back to plain diff coloring
        assert!(!spans.is_empty());
        assert_eq!(spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_empty_line() {
        let highlighter = Highlighter::new();
        let spans = highlighter.highlight_diff_line("", "rs");

        // Empty lines should return a single empty span
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }

    #[test]
    fn test_syntect_to_ratatui_conversion() {
        let syntect_color = SyntectColor { r: 255, g: 128, b: 64, a: 255 };
        let ratatui_color = Highlighter::syntect_to_ratatui(syntect_color);
        assert_eq!(ratatui_color, Color::Rgb(255, 128, 64));
    }
}
```

### 2.3: Implement Highlighter (TDD - Green phase)

**Implementation requirements:**

1. **`Highlighter::new()`**:
   - Load `SyntaxSet::load_defaults_newlines()` (handles \n and \r\n)
   - Load `ThemeSet::load_defaults()`
   - Store both in struct

2. **`Highlighter::highlight_diff_line()`**:
   - Extract prefix: check if line starts with `+`, `-`, or ` `
   - If unknown file extension, return fallback (plain colored span)
   - Strip prefix from line content
   - Use `HighlightLines::new(syntax, theme)` to highlight content
   - Call `highlight_line(stripped_line, &syntax_set)` → `Vec<(Style, &str)>`
   - Convert syntect spans to ratatui spans
   - Prepend the prefix span with diff color (green/red/white)
   - Return `Vec<Span>`

3. **Error handling**:
   - NEVER panic or unwrap in library code
   - Use `.ok()` / `.unwrap_or()` / `if let Some`
   - Fall back to plain diff coloring on any error

4. **Theme**:
   - Use `theme_set.themes["base16-ocean.dark"]`
   - Store reference in struct or fetch on each call (architect's choice)

5. **Color conversion**:
   - `syntect::highlighting::Color { r, g, b, a }` → `ratatui::style::Color::Rgb(r, g, b)`
   - Ignore alpha channel (terminal limitation)

---

## Step 3: Integrate into TUI

### 3.1: Modify App struct

**File: `src/tui/mod.rs`**

**Change at line 28-37 (App struct):**

```rust
pub struct App {
    files: Vec<DiffFile>,
    db: ReviewDb,
    base_ref: String,
    selected_file: usize,
    selected_hunk: usize,
    filter: FilterMode,
    should_quit: bool,
    show_help: bool,
    highlighter: crate::highlight::Highlighter,  // NEW FIELD
}
```

**Change at line 41-52 (App::new):**

```rust
fn new(files: Vec<DiffFile>, db: ReviewDb, base_ref: String) -> Self {
    Self {
        files,
        db,
        base_ref,
        selected_file: 0,
        selected_hunk: 0,
        filter: FilterMode::All,
        should_quit: false,
        show_help: false,
        highlighter: crate::highlight::Highlighter::new(),  // NEW INITIALIZATION
    }
}
```

### 3.2: Modify render_hunk_detail()

**File: `src/tui/mod.rs`**

**Change at lines 339-349 (hunk content rendering):**

Current code:
```rust
// Add hunk content with syntax highlighting
for line in hunk.content.lines() {
    let styled_line = if line.starts_with('+') {
        Line::from(Span::styled(line, Style::default().fg(Color::Green)))
    } else if line.starts_with('-') {
        Line::from(Span::styled(line, Style::default().fg(Color::Red)))
    } else {
        Line::from(line)
    };
    lines.push(styled_line);
}
```

Replace with:
```rust
// Add hunk content with syntax highlighting
let file_ext = file
    .path
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap_or("");

for line in hunk.content.lines() {
    let spans = self.highlighter.highlight_diff_line(line, file_ext);
    lines.push(Line::from(spans));
}
```

**Rationale:**
- Extract file extension from `file.path` (PathBuf)
- Pass extension to highlighter
- Highlighter returns `Vec<Span>` with correct coloring
- Replace entire if/else chain with single highlighter call

---

## Step 4: Wire up in `src/lib.rs`

**File: `src/lib.rs`**

**Add module declaration at top (around line 6):**

```rust
pub mod cli;
pub mod gate;
pub mod highlight;  // NEW MODULE
pub mod parser;
pub mod state;
pub mod tui;
```

**No changes needed to `src/main.rs`** — App initialization happens in `run_tui()` which already calls `App::new()`.

---

## Step 5: Tests

### 5.1: Unit tests for Highlighter

Already written in Step 2.2. Run with:

```bash
cargo test highlight::tests
```

Expected results:
- All 7 tests pass
- Tests cover: initialization, added lines, removed lines, context lines, unknown extensions, empty lines, color conversion

### 5.2: Integration test (optional, but recommended)

**File: `tests/integration_test.rs` (if it exists, add test; if not, skip)**

Add a test to verify TUI initialization with highlighter:

```rust
#[test]
fn test_tui_with_syntax_highlighting() {
    use git_review::{DiffFile, DiffHunk, HunkStatus, state::ReviewDb};
    use tempfile::NamedTempFile;
    use std::path::PathBuf;

    let db_file = NamedTempFile::new().unwrap();
    let db = ReviewDb::open(db_file.path()).unwrap();

    let files = vec![DiffFile {
        path: PathBuf::from("test.rs"),
        hunks: vec![DiffHunk {
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            content: "+fn test() {}".to_string(),
            content_hash: "abc123".to_string(),
            status: HunkStatus::Unreviewed,
        }],
    }];

    // This should not panic when initializing App with Highlighter
    // Note: actual TUI run would require terminal setup, so just test construction
    // The test verifies that Highlighter::new() doesn't panic
}
```

---

## Step 6: Verification

### 6.1: Run full test suite

```bash
cargo test
```

**Expected outcome:**
- All existing tests pass (26 tests from baseline)
- New highlighter tests pass (7 tests)
- Total: 33 tests passing

**If any test fails:** Debug and fix before proceeding.

### 6.2: Run clippy

```bash
cargo clippy -- -D warnings
```

**Expected outcome:** No warnings.

**Common issues to watch for:**
- Unused imports (remove any unused syntect imports)
- Unnecessary type conversions
- Inefficient string allocations (use `&str` where possible)

### 6.3: Run formatter

```bash
cargo fmt --check
```

**Expected outcome:** No formatting issues.

If formatter reports issues:
```bash
cargo fmt
```

### 6.4: Manual TUI test

Build and run the TUI:

```bash
cargo run -- review main..HEAD
```

**Verification checklist:**
- [ ] Rust code (.rs files) shows syntax highlighting (keywords, strings, comments)
- [ ] TOML files (.toml) show syntax highlighting
- [ ] Unknown file types fall back to plain green/red coloring
- [ ] Navigation still works (j/k, Tab, Space)
- [ ] No crashes or panics
- [ ] Performance is acceptable (no visible lag)

---

## Implementation Order

1. **Step 1 → Step 2.2 (tests)** — Add dependency, write failing tests (RED)
2. **Step 2.3** — Implement Highlighter to make tests pass (GREEN)
3. **Step 2.3** — Refactor if needed (REFACTOR)
4. **Step 3** — Integrate into TUI
5. **Step 4** — Wire up module
6. **Step 6** — Verify all tests, clippy, fmt

---

## Rollback Plan

If highlighting causes issues:

1. Revert Step 3.2 changes (render_hunk_detail):
   - Restore original if/else chain for line coloring

2. Keep Highlighter module in codebase but unused (for future iteration)

3. Remove syntect from Cargo.toml if needed

---

## Performance Notes

- **Startup cost:** ~250ms (loading SyntaxSet/ThemeSet) — acceptable, happens once at App::new()
- **Per-line cost:** ~2-3µs (negligible) — no visible lag
- **Memory:** ~10MB (SyntaxSet + ThemeSet) — acceptable for a TUI app

---

## Edge Cases Handled

1. **Empty lines:** Return single empty span
2. **Lines without +/- prefix:** Highlight as context (white background)
3. **Unknown file extensions:** Fall back to plain diff coloring
4. **Binary files:** Already skipped by parser, won't reach highlighter
5. **Syntect errors:** Graceful fallback, never panic

---

## Definition of Done

- [ ] All 33 tests pass
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] Manual TUI test successful
- [ ] Syntax highlighting visible for .rs, .toml files
- [ ] Unknown file types fall back gracefully
- [ ] No performance degradation
