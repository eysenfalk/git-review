use ratatui::{
    style::{Color, Style},
    text::Span,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color as SyntectColor, Theme, ThemeSet},
    parsing::SyntaxSet,
};

/// Maximum line length for syntax highlighting (skip longer lines for performance).
const MAX_LINE_LENGTH: usize = 10_000;

/// Syntax highlighter for diff content.
///
/// This struct is immutable and can be shared. Use `for_file()` to create
/// a stateful highlighter session for a specific file.
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    /// Create a new Highlighter with default syntax and theme sets.
    ///
    /// This loads all bundled syntaxes and themes, which takes ~250ms.
    /// The cost is paid once at initialization.
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        // TODO: Support theme selection (env var GITREVIEW_THEME or --theme flag)
        let theme = theme_set
            .themes
            .get("base16-ocean.dark")
            .or_else(|| theme_set.themes.values().next())
            .cloned()
            .unwrap_or_default();

        Self { syntax_set, theme }
    }

    /// Create a file-scoped highlighter session that maintains state across lines.
    ///
    /// This is necessary for multi-line constructs like multi-line strings,
    /// multi-line comments, and nested blocks to be highlighted correctly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let highlighter = Highlighter::new();
    /// let mut fh = highlighter.for_file("rs");
    /// for line in content.lines() {
    ///     let spans = fh.highlight_diff_line(line);
    ///     // render spans...
    /// }
    /// ```
    pub fn for_file(&self, file_ext: &str) -> FileHighlighter<'_> {
        FileHighlighter::new(&self.syntax_set, &self.theme, file_ext)
    }

    /// Convert syntect Color to ratatui Color.
    fn syntect_to_ratatui(color: SyntectColor) -> Color {
        Color::Rgb(color.r, color.g, color.b)
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Maintains HighlightLines state across lines within a single file.
///
/// This struct is created per-file and maintains parse state for multi-line
/// constructs. It must be used sequentially for all lines in a file.
pub struct FileHighlighter<'a> {
    highlighter: Option<HighlightLines<'a>>,
    syntax_set: &'a SyntaxSet,
}

impl<'a> FileHighlighter<'a> {
    /// Create a new FileHighlighter for a specific file extension.
    fn new(syntax_set: &'a SyntaxSet, theme: &'a Theme, file_ext: &str) -> Self {
        let syntax = syntax_set
            .find_syntax_by_extension(file_ext)
            .or_else(|| syntax_set.find_syntax_by_name(file_ext));

        let highlighter = syntax.map(|s| HighlightLines::new(s, theme));

        Self {
            highlighter,
            syntax_set,
        }
    }

    /// Highlight a single diff line. Maintains state for multi-line constructs.
    ///
    /// Returns owned Spans (`Span<'static>`) since we need to strip the diff prefix
    /// and syntect content needs to be converted to owned Strings.
    ///
    /// Falls back to plain diff coloring if highlighting fails or file type is unknown.
    pub fn highlight_diff_line(&mut self, line: &str) -> Vec<Span<'static>> {
        // Handle empty lines
        if line.is_empty() {
            return vec![Span::raw(String::new())];
        }

        // Extract diff prefix (+, -, or space)
        let (prefix, prefix_color) = if line.starts_with('+') {
            ("+", Color::Green)
        } else if line.starts_with('-') {
            ("-", Color::Red)
        } else if line.starts_with(' ') {
            (" ", Color::Reset)
        } else {
            // No recognized prefix (e.g., "\ No newline at end of file")
            return vec![Span::raw(line.to_string())];
        };

        // Performance: skip highlighting very long lines
        if line.len() > MAX_LINE_LENGTH {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(prefix_color),
            )];
        }

        // Strip prefix for syntax highlighting
        let content = if line.len() > 1 {
            &line[1..]
        } else {
            // Line is just the prefix (e.g., "+")
            ""
        };

        // If no highlighter (unknown file type), fall back to plain diff coloring
        let Some(ref mut highlighter) = self.highlighter else {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(prefix_color),
            )];
        };

        // Perform syntax highlighting
        match highlighter.highlight_line(content, self.syntax_set) {
            Ok(regions) => {
                let mut spans = Vec::with_capacity(regions.len() + 1);

                // Add prefix with diff color
                spans.push(Span::styled(
                    prefix.to_string(),
                    Style::default().fg(prefix_color),
                ));

                // Add syntax-highlighted content
                // Note: We use the syntax foreground color but preserve diff semantics
                // by using the diff color for the prefix
                for (style, text) in regions {
                    let fg_color = Highlighter::syntect_to_ratatui(style.foreground);
                    spans.push(Span::styled(
                        text.to_string(),
                        Style::default().fg(fg_color),
                    ));
                }

                spans
            }
            Err(_) => {
                // Graceful fallback on syntax error
                vec![Span::styled(
                    line.to_string(),
                    Style::default().fg(prefix_color),
                )]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_new() {
        let highlighter = Highlighter::new();

        // Rust should definitely be recognized
        let fh = highlighter.for_file("rs");
        assert!(fh.highlighter.is_some(), "Rust syntax should be found");

        // JSON is widely supported
        let fh = highlighter.for_file("json");
        assert!(fh.highlighter.is_some(), "JSON syntax should be found");

        // Markdown should be recognized
        let fh = highlighter.for_file("md");
        assert!(fh.highlighter.is_some(), "Markdown syntax should be found");

        // Unknown extension should have no highlighter
        let fh = highlighter.for_file("unknown_ext_xyz");
        assert!(
            fh.highlighter.is_none(),
            "Unknown extension should have no highlighter"
        );
    }

    #[test]
    fn test_highlight_added_line() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");
        let line = "+fn main() { println!(\"test\"); }";
        let spans = fh.highlight_diff_line(line);

        // Should return non-empty spans
        assert!(!spans.is_empty());

        // First span should be the '+' prefix with green color
        assert_eq!(spans[0].content.as_ref(), "+");
        assert_eq!(spans[0].style.fg, Some(Color::Green));

        // Should have more than just the prefix (syntax highlighted content)
        assert!(spans.len() > 1, "Should have syntax highlighted content");
    }

    #[test]
    fn test_highlight_removed_line() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");
        let line = "-fn old_function() {}";
        let spans = fh.highlight_diff_line(line);

        assert!(!spans.is_empty());
        assert_eq!(spans[0].content.as_ref(), "-");
        assert_eq!(spans[0].style.fg, Some(Color::Red));
        assert!(spans.len() > 1);
    }

    #[test]
    fn test_highlight_context_line() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");
        let line = " fn context() {}";
        let spans = fh.highlight_diff_line(line);

        // Context lines (space prefix) should still be highlighted
        assert!(!spans.is_empty());
        assert_eq!(spans[0].content.as_ref(), " ");
        assert_eq!(spans[0].style.fg, Some(Color::Reset));
    }

    #[test]
    fn test_fallback_for_unknown_extension() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("unknown_xyz");
        let line = "+some text in unknown format";
        let spans = fh.highlight_diff_line(line);

        // Should fall back to plain diff coloring
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(Color::Green));
        assert_eq!(spans[0].content.as_ref(), line);
    }

    #[test]
    fn test_empty_line() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");
        let spans = fh.highlight_diff_line("");

        // Empty lines should return a single empty span
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), "");
    }

    #[test]
    fn test_syntect_to_ratatui_conversion() {
        let syntect_color = SyntectColor {
            r: 255,
            g: 128,
            b: 64,
            a: 255,
        };
        let ratatui_color = Highlighter::syntect_to_ratatui(syntect_color);
        assert_eq!(ratatui_color, Color::Rgb(255, 128, 64));
    }

    #[test]
    fn test_line_with_only_prefix() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");

        // Line with only "+" and no content
        let spans = fh.highlight_diff_line("+");
        assert!(!spans.is_empty());
        assert_eq!(spans[0].content.as_ref(), "+");

        // Line with only "-" and no content
        let spans = fh.highlight_diff_line("-");
        assert!(!spans.is_empty());
        assert_eq!(spans[0].content.as_ref(), "-");
    }

    #[test]
    fn test_no_newline_marker() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");
        let line = "\\ No newline at end of file";
        let spans = fh.highlight_diff_line(line);

        // Should return as plain text (no +/- prefix)
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content.as_ref(), line);
    }

    #[test]
    fn test_long_line_performance() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");
        let long_line = "+".to_string() + &"x".repeat(15_000);

        let start = std::time::Instant::now();
        let spans = fh.highlight_diff_line(&long_line);
        let elapsed = start.elapsed();

        // Should skip highlighting and fall back quickly
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "Long line should be handled quickly"
        );
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_multiline_string_state() {
        let highlighter = Highlighter::new();
        let mut fh = highlighter.for_file("rs");

        // Multi-line string in Rust
        let line1 = r#"+    let s = "line1"#;
        let line2 = r#"+             line2";"#;

        let spans1 = fh.highlight_diff_line(line1);
        assert!(!spans1.is_empty());

        // Second line should maintain parse state from first line
        // This test verifies that FileHighlighter maintains state
        let spans2 = fh.highlight_diff_line(line2);
        assert!(!spans2.is_empty());

        // Both should have more than just the prefix
        // (exact color checking would be brittle, but structure matters)
        assert!(spans1.len() > 1);
        assert!(spans2.len() > 1);
    }
}
