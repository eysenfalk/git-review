# Syntax Highlighting Plan Critique

## Executive Summary

**OVERALL: FAIL (3 critical issues, 2 warnings, 3 passes)**

The plan has a **CRITICAL** architectural flaw in how `HighlightLines` state is managed that will break multi-line syntax highlighting (strings, comments, etc.). Several other issues need addressing before implementation.

---

## 1. Correctness: API Lifetime Design

**Rating: PASS (with verification needed)**

**Analysis:**

The proposed signature works:
```rust
pub fn highlight_diff_line<'a>(&self, line: &'a str, file_ext: &str) -> Vec<Span<'a>>
```

Lifetime reasoning:
- Input `line` has lifetime `'a`
- Stripping prefix: `&line[1..]` produces a slice borrowing from `line` with lifetime `'a`
- syntect's `highlight_line(&'a str, ...) -> Vec<(Style, &'a str)>` returns slices borrowing from input
- Prefix span: `&line[0..1]` or `"+"` (static) both work with lifetime `'a`
- Result: All spans transitively borrow from `line` with lifetime `'a` ✅

**Verification Required:**

Confirm syntect 5.x returns `Vec<(Style, &'a str)>` and not `Vec<(Style, String)>`. If syntect allocates new strings, the lifetime signature breaks.

**Action:** Document this assumption in code comments.

---

## 2. Performance: Startup Cost

**Rating: WARN**

**Analysis:**

- **250ms startup**: Acceptable for TUI (users wait for git diff anyway)
- **Problem**: Loading ALL syntaxes upfront even if user reviews a single .txt file
- **Memory**: ~10MB for SyntaxSet/ThemeSet

**Concerns:**

1. **No lazy loading**: Why load Python syntax if user never reviews .py files?
2. **Binary bloat**: Default syntaxes add ~5-10MB to binary size
3. **Opportunity cost**: Could lazy-load on first use of each extension

**Recommendation:**

Document this tradeoff in a comment:
```rust
// Note: 250ms startup cost acceptable for TUI use case.
// Alternative: lazy-load per extension, but adds complexity.
```

Consider lazy loading in a future iteration if users complain.

**Action:** Add TODO comment. Ship as-is for v1.

---

## 3. Lifetime Issues: Deep Dive

**Rating: PASS (assuming verification passes)**

**Detailed Analysis:**

The lifetime chain:
```
line: &'a str
 └─> stripped: &'a str = &line[1..]     // slice of line
      └─> syntect spans: Vec<(Style, &'a str)>  // slices of stripped
           └─> ratatui spans: Vec<Span<'a>>      // converted to Span<'a>
```

All spans ultimately borrow from `line`, so `'a` propagates correctly.

**Edge Case to Test:**

Empty line prefix handling:
```rust
if line.is_empty() {
    return vec![Span::raw("")];  // 'static str, OK
}
```

**Action:** Add integration test with empty lines.

---

## 4. HighlightLines State: CRITICAL FAILURE

**Rating: FAIL 🔴**

**Problem:**

The plan says:
> "Use `HighlightLines::new(syntax, theme)` to highlight content"

This creates a **new** `HighlightLines` instance **per line**. This is **architecturally wrong**.

**Why This Breaks:**

`HighlightLines` is **stateful**. It maintains parse state across lines:
- **Multi-line strings**: `"foo\nbar"` requires state from first line to highlight second
- **Multi-line comments**: `/* comment\n continues */`
- **Nested blocks**: Indentation-sensitive languages

Creating fresh `HighlightLines` per line resets state → **broken highlighting for multi-line constructs**.

**Example Failure:**

```rust
// Input:
+    let s = "multi
+             line";

// Current plan: each line gets fresh HighlightLines
// Result: Second line not recognized as string continuation → WRONG COLORS
```

**Correct Architecture:**

```rust
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_highlighter: Option<HighlightLines<'static>>,  // NEW: stateful per file
    current_file_ext: Option<String>,  // NEW: track current file
}

impl Highlighter {
    /// Call when starting a new file
    pub fn start_file(&mut self, file_ext: &str) {
        if let Some(syntax) = self.find_syntax(file_ext) {
            let theme = &self.theme_set.themes["base16-ocean.dark"];
            self.current_highlighter = Some(HighlightLines::new(syntax, theme));
            self.current_file_ext = Some(file_ext.to_string());
        } else {
            self.current_highlighter = None;
        }
    }

    /// Call when done with a file
    pub fn end_file(&mut self) {
        self.current_highlighter = None;
        self.current_file_ext = None;
    }

    /// Highlight a line (uses stored state)
    pub fn highlight_diff_line<'a>(&mut self, line: &'a str) -> Vec<Span<'a>> {
        // Now uses self.current_highlighter, maintaining state
    }
}
```

**TUI Integration Changes:**

In `render_hunk_detail()`, before the line loop:
```rust
// Before rendering first hunk line
self.highlighter.start_file(file_ext);

// Render loop
for line in hunk.content.lines() {
    let spans = self.highlighter.highlight_diff_line(line);
    lines.push(Line::from(spans));
}

// After rendering last hunk line (or when switching files)
self.highlighter.end_file();
```

**Action:** **BLOCK IMPLEMENTATION** until this is fixed in the plan.

---

## 5. Theme Hardcoding

**Rating: WARN**

**Problem:**

The theme is hardcoded:
```rust
theme_set.themes["base16-ocean.dark"]
```

This is a **dark theme** (dark blue background). Users with **light terminals** will get:
- Poor contrast (dark text on light background)
- Unreadable output

**Evidence from TUI Code:**

Current TUI uses terminal's default colors (no hardcoded background). Adding a dark theme breaks this.

**Mitigation Options:**

1. **Short-term**: Add TODO comment
   ```rust
   // TODO: Support light themes (env var GITREVIEW_THEME or auto-detect)
   ```

2. **Medium-term**: Add CLI flag `--theme dark|light`

3. **Long-term**: Auto-detect terminal background (non-trivial, requires terminal queries)

**Recommendation:**

For v1: Ship with dark theme + TODO comment. Document in README that users should set `TERM=xterm-256color` or similar.

**Action:** Add TODO comment. File issue for future CLI flag.

---

## 6. Test Quality

**Rating: FAIL (critical gap)**

**Missing Tests:**

1. **Multi-line syntax** (would catch #4 bug!)
   ```rust
   #[test]
   fn test_multiline_string() {
       let highlighter = Highlighter::new();
       highlighter.start_file("rs");

       let line1 = r#+let s = "foo"#;
       let line2 = r#bar";#;

       let spans1 = highlighter.highlight_diff_line(line1);
       let spans2 = highlighter.highlight_diff_line(line2);

       // Both should be highlighted as string (green or similar)
       // With current plan: line2 would be plain text (WRONG)
   }
   ```

2. **Very long lines** (performance)
   ```rust
   #[test]
   fn test_long_line_performance() {
       let highlighter = Highlighter::new();
       let long_line = "+".to_string() + &"x".repeat(100_000);

       let start = Instant::now();
       let spans = highlighter.highlight_diff_line(&long_line, "rs");
       let elapsed = start.elapsed();

       assert!(elapsed < Duration::from_millis(100)); // Should be fast
       assert!(!spans.is_empty());
   }
   ```

3. **Edge cases:**
   - Line with only `+` (no content)
   - Line with only `-` (no content)
   - Non-UTF8 content (use `b"+"` byte strings)

**Existing Test Weakness:**

`test_highlight_added_line` only checks:
```rust
assert_eq!(spans[0].content, "+");
assert_eq!(spans[0].style.fg, Some(Color::Green));
```

It doesn't verify that `fn`, `main`, `println!` are highlighted differently (keywords vs macros).

**Action:** Add multi-line test BEFORE implementing (TDD). Add long-line test.

---

## 7. Over-engineering

**Rating: PASS (acceptable)**

**Analysis:**

The plan is reasonably minimal. Only concern:

**Binary Size:**

Including `default-syntaxes` adds ~5-10MB for all languages. For a Rust-focused tool, could be slimmer:

```toml
syntect = { version = "5", default-features = false, features = ["parsing", "fancy-regex"] }
```

Then manually bundle only `.rs`, `.toml`, `.md`, `.json` syntaxes.

**Tradeoff:**

- **Pro (current plan)**: Works for any language out-of-box
- **Con**: Larger binary
- **Pro (minimal)**: Smaller binary
- **Con**: Users reviewing .py files get no highlighting

**Recommendation:**

Ship with `default-syntaxes` for v1. Users reviewing Rust code in a Rust project likely review other languages too (Python scripts, JSON configs, etc.).

**Action:** Accept current approach. No changes needed.

---

## 8. Missing Concerns

**Rating: FAIL (2 critical, 3 minor)**

### Critical:

1. **Multi-line state** (covered in #4) — ARCHITECTURAL BUG

2. **Non-UTF8 content**
   - Git diffs can contain binary data
   - Rust strings require UTF-8
   - If `line` contains invalid UTF-8, conversion to `&str` would panic
   - **Mitigation**: Parser should already filter binary files, but verify this

### Minor but Important:

3. **`\ No newline at end of file` markers**
   - Git diffs include `\ No newline at end of file` lines
   - These don't start with `+/-/ `
   - Plan's `if line.starts_with('+')` check won't match
   - **Mitigation**: Fall through to plain text span (OK, but test it)

4. **Very long lines**
   - Minified .js files can have 100KB lines
   - Syntect may be slow on these
   - **Mitigation**: Add timeout or length limit (e.g., skip highlighting if `line.len() > 10_000`)

5. **ANSI escape codes in diff**
   - If user has `git config color.diff always`, output contains `\x1b[31m` codes
   - Parser should strip these, but verify
   - **Mitigation**: Document that git-review assumes `color.diff=never` or parse ANSI

### Not Critical:

6. **Contextual highlighting**
   - Context lines (no `+/-`) are highlighted as if at file start
   - Lose state from before the hunk
   - **Acceptable**: Hunks are self-contained for review purposes

7. **Line-only prefix handling**
   - What if a line is exactly `"+"` with no content?
   - `&line[1..]` would be `""` (empty string) — OK
   - Test this edge case

**Action:**

- **MUST FIX**: #1 (multi-line state)
- **VERIFY**: #2 (parser filters binary)
- **TEST**: #3, #7 (edge cases)
- **DOCUMENT**: #4 (long line behavior), #5 (ANSI assumption)

---

## Summary of Required Changes

### CRITICAL (must fix before implementation):

1. **Change Highlighter API to be stateful:**
   - Add `current_highlighter: Option<HighlightLines<'static>>` field
   - Add `start_file(file_ext)` and `end_file()` methods
   - Change `highlight_diff_line` to `&mut self` (needs state)
   - Update TUI integration to call `start_file` before rendering each file

2. **Add multi-line syntax test:**
   - Test that multi-line strings/comments work correctly
   - This would have caught the bug during TDD

### RECOMMENDED (should add):

3. **Add TODO comments for:**
   - Theme selection (env var or CLI flag)
   - Long line handling (performance concern)

4. **Add tests for:**
   - Very long lines (performance regression check)
   - Edge cases: empty content, `\ No newline`, line-only `+`

5. **Verify assumptions:**
   - Parser filters binary files (non-UTF8 won't reach highlighter)
   - syntect 5.x API matches expected signature

### OPTIONAL (nice to have):

6. **Consider for future:**
   - Lazy-load syntaxes per extension (reduce startup time)
   - Bundle only common syntaxes (reduce binary size)
   - Auto-detect terminal theme

---

## Revised Implementation Order

1. **Fix plan:**
   - Rewrite Highlighter API to be stateful (add `start_file`/`end_file`)
   - Update Step 2.2 tests to include multi-line test
   - Update Step 3.2 TUI integration to call `start_file` before line loop

2. **Add tests (TDD Red phase):**
   - All tests from plan
   - **NEW**: Multi-line string test
   - **NEW**: Long line performance test
   - **NEW**: Edge case tests

3. **Implement (TDD Green phase):**
   - Implement with stateful API
   - Make all tests pass

4. **Verify:**
   - Run full test suite
   - Manual TUI test with Rust files containing multi-line strings
   - Check that comments spanning hunks highlight correctly

---

## Risk Assessment

**If implemented as-is (without fixes):**

- **HIGH RISK**: Multi-line syntax will be broken (strings, comments rendered incorrectly)
- **MEDIUM RISK**: Performance issues with long lines (no mitigation)
- **LOW RISK**: Theme incompatibility with light terminals (usability issue)

**With fixes:**

- **LOW RISK**: Well-tested, incremental addition
- **ACCEPTABLE**: Minor theme limitation documented

---

## Recommendation

**DO NOT PROCEED** with current plan. Revise plan to fix critical state management issue (#4), then restart TDD cycle with updated tests.
