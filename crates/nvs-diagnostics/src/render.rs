//! Terminal rendering for diagnostics.
//!
//! Every line it prints comes from the diagnostic's record, which
//! [`record::record`] builds, so the text says nothing the record does not.
//!
//! Hand-written rather than delegated to a crate: the output format is part of
//! a language's user interface and should not shift with a dependency bump;
//! Novis needs the same layout engine to emit LSP-shaped data; and it keeps the
//! dependency tree of the compiler front end minimal.

use std::io::{self, Write};

use anstyle::{AnsiColor, Style};
use nvs_render::{Node, Record, Rendered, Scalar};

use crate::diagnostic::{Diagnostic, Severity};
use crate::record;
use crate::source::SourceMap;

/// The line a terminal rendering closes with when any diagnostic in it carried
/// a code: where the reader finds that code's card.
///
/// It does not start with a severity label, so a reader that picks the heads
/// out of a rendering by their labels passes over it as it passes over the
/// summary.
pub const AGENT_SHOW_LINE: &str =
    "To read what a code means and how to fix it, run `nvs agent show <code>`.";

/// Renders diagnostics as rustc-style annotated source snippets.
#[derive(Debug, Clone)]
pub struct Renderer {
    color: bool,
    tab_width: usize,
    /// Lines of context shown around a labelled line. Kept at 0 for now: Novis's
    /// diagnostics point at small spans and context mostly adds noise.
    context_lines: usize,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            color: false,
            tab_width: 4,
            context_lines: 0,
        }
    }
}

impl Renderer {
    /// A renderer with colour disabled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables or disables ANSI colour.
    ///
    /// Callers should pass the result of a terminal check plus `NO_COLOR`
    /// handling; this type does not probe the environment itself.
    #[must_use]
    pub const fn with_color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }

    /// Sets how many columns a tab advances. Defaults to 4.
    #[must_use]
    pub const fn with_tab_width(mut self, width: usize) -> Self {
        // `Ord::max` is not const yet; a tab width of 0 would divide by zero in
        // the tab-stop arithmetic, so clamp it here.
        self.tab_width = if width == 0 { 1 } else { width };
        self
    }

    /// Sets how many lines of surrounding context to show. Defaults to 0.
    #[must_use]
    pub const fn with_context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    fn paint(&self, style: Style, text: &str, out: &mut impl Write) -> io::Result<()> {
        if self.color {
            write!(out, "{}{text}{}", style.render(), style.render_reset())
        } else {
            out.write_all(text.as_bytes())
        }
    }

    fn severity_style(severity: Severity) -> Style {
        let color = match severity {
            Severity::Error | Severity::Bug => AnsiColor::Red,
            Severity::Warning => AnsiColor::Yellow,
            Severity::Note => AnsiColor::BrightBlue,
            Severity::Help => AnsiColor::Green,
        };
        Style::new().bold().fg_color(Some(color.into()))
    }

    fn gutter_style() -> Style {
        Style::new()
            .bold()
            .fg_color(Some(AnsiColor::BrightBlue.into()))
    }

    /// Renders one diagnostic, including a trailing blank line.
    ///
    /// # Errors
    ///
    /// Propagates any write error from `out`.
    pub fn render(&self, d: &Diagnostic, map: &SourceMap, out: &mut impl Write) -> io::Result<()> {
        self.render_record(&record::record(d, map), out)
    }

    /// Renders one diagnostic's record, as [`record::record`] builds it,
    /// including a trailing blank line.
    ///
    /// # Errors
    ///
    /// Propagates any write error from `out`.
    pub fn render_record(&self, r: &Record, out: &mut impl Write) -> io::Result<()> {
        let severity = record::severity(r).unwrap_or(Severity::Error);
        let sev = Self::severity_style(severity);
        let gutter = Self::gutter_style();
        let bold = Style::new().bold();

        // error[E0401]: message
        let head = match record::field_text(r, record::CODE) {
            Some(code) => format!("{}[{code}]", severity.label()),
            None => severity.label().to_string(),
        };
        self.paint(sev, &head, out)?;
        self.paint(bold, ": ", out)?;
        let message = r.envelope.message.as_ref().map_or("", Rendered::as_str);
        self.paint(bold, message, out)?;
        out.write_all(b"\n")?;

        // Labels, grouped by file so a diagnostic spanning two files reads
        // as two snippets rather than interleaved lines.
        let spans: Vec<SpanView<'_>> = r.nodes.iter().filter_map(SpanView::of).collect();
        let mut files: Vec<&str> = Vec::new();
        for s in &spans {
            if !files.contains(&s.file) {
                files.push(s.file);
            }
        }

        let width = Self::gutter_width(&spans);

        for (i, file) in files.iter().copied().enumerate() {
            let mut labels: Vec<&SpanView<'_>> = spans.iter().filter(|s| s.file == file).collect();
            labels.sort_by_key(|s| (s.line, s.column));

            // --> path:line:col, anchored on this file's first label
            let anchor = labels[0];
            write!(out, "{:width$}", "")?;
            self.paint(gutter, "--> ", out)?;
            writeln!(out, "{file}:{}:{}", anchor.line, anchor.column)?;

            self.write_gutter_only(width, out)?;

            let mut prev_line: Option<usize> = None;
            for label in labels {
                let lo_line = label.line;

                // A gap in line numbers gets an ellipsis so it is obvious that
                // lines were skipped rather than that the file is short.
                if let Some(p) = prev_line
                    && lo_line > p + 1 + self.context_lines
                {
                    self.paint(gutter, "...", out)?;
                    out.write_all(b"\n")?;
                }
                prev_line = Some(lo_line);

                let Some(text) = label.source else {
                    continue;
                };
                let shown = expand_tabs(text, self.tab_width);

                // 12 | let x = ...
                let num = format!("{:>w$}", lo_line, w = width.saturating_sub(1));
                self.paint(gutter, &num, out)?;
                self.paint(gutter, " | ", out)?;
                writeln!(out, "{shown}")?;

                // Underline. A span that runs past this line is clipped to the
                // line end and marked, rather than drawing an enormous caret.
                let start_col = display_col(text, label.column - 1, self.tab_width);
                let multiline = label.end_line > lo_line;
                let end_col = if multiline {
                    display_col(text, text.chars().count(), self.tab_width)
                } else {
                    display_col(text, label.end_column - 1, self.tab_width)
                };
                let len = end_col.saturating_sub(start_col).max(1);

                let (mark, style) = if label.primary {
                    ("^", sev)
                } else {
                    ("-", gutter)
                };

                self.write_gutter_prefix(width, out)?;
                write!(out, "{:start$}", "", start = start_col)?;
                self.paint(style, &mark.repeat(len), out)?;

                if multiline {
                    self.paint(style, "...", out)?;
                }
                if !label.label.is_empty() {
                    out.write_all(b" ")?;
                    self.paint(style, label.label, out)?;
                }
                out.write_all(b"\n")?;

                if multiline {
                    self.write_gutter_prefix(width, out)?;
                    self.paint(
                        gutter,
                        &format!("(continues to line {})", label.end_line),
                        out,
                    )?;
                    out.write_all(b"\n")?;
                }
            }

            if i + 1 < files.len() {
                self.write_gutter_only(width, out)?;
            }
        }

        // = note: ... and = help: ...
        for (kind, note) in sequence(r, record::NOTES).iter().filter_map(|n| match n {
            Node::Map(entries) => entries.first(),
            _ => None,
        }) {
            write!(out, "{:width$}", "")?;
            self.paint(gutter, "= ", out)?;
            let note = scalar_text(note).unwrap_or("");
            if kind.as_str() == "help" {
                self.paint(Self::severity_style(Severity::Help), "help", out)?;
            } else {
                self.paint(bold, "note", out)?;
            }
            write!(out, ": {note}")?;
            out.write_all(b"\n")?;
        }

        // Suggestions, with the replacement shown inline.
        for entries in sequence(r, record::SUGGESTIONS)
            .iter()
            .filter_map(|n| match n {
                Node::Map(entries) => Some(entries),
                _ => None,
            })
        {
            let message = entry(entries, "message")
                .and_then(scalar_text)
                .unwrap_or("");
            let replacement = entry(entries, "replacement")
                .and_then(scalar_text)
                .unwrap_or("");
            let safe = matches!(
                entry(entries, "safe"),
                Some(Node::Scalar(Scalar::Bool(true)))
            );
            write!(out, "{:width$}", "")?;
            self.paint(gutter, "= ", out)?;
            let tag = if safe {
                "suggestion"
            } else {
                "suggestion (review)"
            };
            self.paint(Self::severity_style(Severity::Help), tag, out)?;
            write!(out, ": {message}")?;
            if replacement.is_empty() {
                out.write_all(b" (delete)")?;
            } else if !replacement.contains('\n') {
                write!(out, ": `{replacement}`")?;
            }
            out.write_all(b"\n")?;
        }

        out.write_all(b"\n")
    }

    /// Renders many diagnostics, then a summary line if anything was fatal, then
    /// [`AGENT_SHOW_LINE`] if any of them carried a code.
    ///
    /// The last line is written once per call rather than once per diagnostic,
    /// and it is the same text whatever the codes were, because each head above
    /// it already names its own — `rule:tooling/a-diagnostic-code-carries-its-card`.
    ///
    /// # Errors
    ///
    /// Propagates any write error from `out`.
    pub fn render_all<'a>(
        &self,
        diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
        map: &SourceMap,
        out: &mut impl Write,
    ) -> io::Result<()> {
        let mut errors = 0usize;
        let mut warnings = 0usize;
        let mut coded = false;
        for d in diagnostics {
            let r = record::record(d, map);
            match record::severity(&r) {
                Some(Severity::Error | Severity::Bug) => errors += 1,
                Some(Severity::Warning) => warnings += 1,
                _ => {}
            }
            coded |= record::field_text(&r, record::CODE).is_some();
            self.render_record(&r, out)?;
        }

        if errors > 0 {
            let sev = Self::severity_style(Severity::Error);
            self.paint(sev, "error", out)?;
            let plural = if errors == 1 { "" } else { "s" };
            let mut summary = format!(": aborting due to {errors} error{plural}");
            if warnings > 0 {
                let wp = if warnings == 1 { "" } else { "s" };
                summary.push_str(&format!("; {warnings} warning{wp} emitted"));
            }
            self.paint(Style::new().bold(), &summary, out)?;
            out.write_all(b"\n")?;
        }
        if coded {
            out.write_all(AGENT_SHOW_LINE.as_bytes())?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Width of the line-number gutter, including the trailing space.
    fn gutter_width(spans: &[SpanView<'_>]) -> usize {
        let max_line = spans.iter().map(|s| s.end_line).max().unwrap_or(1);
        // digits + 1 space, minimum 2 so an unlabelled diagnostic still indents.
        decimal_digits(max_line) + 1
    }

    /// A gutter with nothing after it, used as a separator row.
    fn write_gutter_only(&self, width: usize, out: &mut impl Write) -> io::Result<()> {
        write!(out, "{:w$}", "", w = width.saturating_sub(1))?;
        self.paint(Self::gutter_style(), " |", out)?;
        out.write_all(b"\n")
    }

    /// A gutter that content follows on the same line — the underline row and
    /// continuation notes. Deliberately does *not* end the line.
    fn write_gutter_prefix(&self, width: usize, out: &mut impl Write) -> io::Result<()> {
        write!(out, "{:w$}", "", w = width.saturating_sub(1))?;
        self.paint(Self::gutter_style(), " | ", out)
    }
}

/// One [`Node::Span`] of a diagnostic's record, its numbers widened for the
/// column arithmetic.
struct SpanView<'a> {
    file: &'a str,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    primary: bool,
    label: &'a str,
    source: Option<&'a str>,
}

impl<'a> SpanView<'a> {
    /// `node` when it is a span, or `None` when it is any other kind.
    fn of(node: &'a Node) -> Option<Self> {
        let Node::Span {
            file,
            line,
            column,
            end_line,
            end_column,
            primary,
            label,
            source,
            ..
        } = node
        else {
            return None;
        };
        // A record's positions are one-based, so a zero is clamped to the first.
        let widen = |n: u32| usize::try_from(n).unwrap_or(usize::MAX).max(1);
        Some(Self {
            file,
            line: widen(*line),
            column: widen(*column),
            end_line: widen(*end_line),
            end_column: widen(*end_column),
            primary: *primary,
            label: label.as_str(),
            source: source.as_ref().map(Rendered::as_str),
        })
    }
}

/// The entries of the sequence field `name`, empty where the record has none.
fn sequence<'a>(r: &'a Record, name: &str) -> &'a [Node] {
    match record::field(r, name) {
        Some(Node::Sequence(nodes)) => nodes,
        _ => &[],
    }
}

/// The value of `key` in a map node's entries.
fn entry<'a>(entries: &'a [(Rendered, Node)], key: &str) -> Option<&'a Node> {
    entries
        .iter()
        .find(|(k, _)| k.as_str() == key)
        .map(|(_, node)| node)
}

/// The text of a string scalar.
fn scalar_text(node: &Node) -> Option<&str> {
    match node {
        Node::Scalar(Scalar::Str { text, .. }) => Some(text.as_str()),
        _ => None,
    }
}

fn decimal_digits(mut n: usize) -> usize {
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

/// Replaces tabs with spaces so that a caret line drawn in spaces lines up with
/// the rendered source line.
fn expand_tabs(line: &str, tab_width: usize) -> String {
    if !line.contains('\t') {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + 8);
    let mut col = 0;
    for ch in line.chars() {
        if ch == '\t' {
            let advance = tab_width - (col % tab_width);
            out.extend(std::iter::repeat_n(' ', advance));
            col += advance;
        } else {
            out.push(ch);
            col += 1;
        }
    }
    out
}

/// Display column of the `char_col`-th character of `line`, accounting for tabs.
fn display_col(line: &str, char_col: usize, tab_width: usize) -> usize {
    let mut col = 0;
    for ch in line.chars().take(char_col) {
        if ch == '\t' {
            col += tab_width - (col % tab_width);
        } else {
            col += 1;
        }
    }
    col
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code;
    use crate::span::Span;

    fn render_to_string(d: &Diagnostic, map: &SourceMap) -> String {
        let mut buf = Vec::new();
        Renderer::new().render(d, map, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn single_line_span_underlines_exactly() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "<?nvs\neval('x');\n");
        let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "`eval` is not supported")
            .with_primary(Span::new(f, 6, 10), "remove this");
        let out = render_to_string(&d, &map);

        // Written out in full deliberately: this is the layout contract every
        // other renderer test assumes.
        let expected = concat!(
            "error[E0201]: `eval` is not supported\n",
            "  --> t.nvs:2:1\n",
            "  |\n",
            "2 | eval('x');\n",
            "  | ^^^^ remove this\n",
            "\n",
        );
        assert_eq!(out, expected, "\n--- got ---\n{out}");
    }

    #[test]
    fn caret_aligns_past_a_tab() {
        let mut map = SourceMap::new();
        // A tab then `bad`: the caret must sit at display column 4, not 1.
        let f = map.add("t.nvs", "\tbad\n");
        let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "nope")
            .with_primary(Span::new(f, 1, 4), "");
        let out = render_to_string(&d, &map);
        let caret_line = out.lines().nth(4).unwrap();
        assert_eq!(caret_line, "  |     ^^^", "tab expanded to 4 columns");
    }

    #[test]
    fn empty_span_still_draws_one_caret() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "$x = 1\n");
        // Missing semicolon: an empty span at end of the expression.
        let d = Diagnostic::error(code::E_EXPECTED_TOKEN, "expected `;`")
            .with_primary(Span::at(f, 6), "insert `;` here");
        let out = render_to_string(&d, &map);
        assert!(out.contains("^ insert `;` here"), "got:\n{out}");
    }

    #[test]
    fn gutter_widens_for_large_line_numbers() {
        let mut map = SourceMap::new();
        let mut text = "\n".repeat(1234);
        text.push_str("boom\n");
        let f = map.add("t.nvs", text);
        let start = 1234;
        let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "x")
            .with_primary(Span::new(f, start, start + 4), "");
        let out = render_to_string(&d, &map);
        assert!(out.contains("1235 | boom"), "got:\n{out}");
        // The `-->` line indents to match the widened gutter.
        assert!(out.contains("     --> t.nvs:1235:1"), "got:\n{out}");
    }

    #[test]
    fn multiline_span_is_clipped_and_annotated() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "fn a() {\n  body\n}\n");
        let d = Diagnostic::error(code::E_UNTERMINATED, "unclosed block")
            .with_primary(Span::new(f, 7, 17), "starts here");
        let out = render_to_string(&d, &map);
        assert!(out.contains("..."), "clipped marker missing:\n{out}");
        assert!(out.contains("continues to line 3"), "got:\n{out}");
    }

    #[test]
    fn secondary_labels_use_dashes() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "$a = 1;\n$a = 2;\n");
        let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "redefined")
            .with_secondary(Span::new(f, 0, 2), "first here")
            .with_primary(Span::new(f, 8, 10), "again here");
        let out = render_to_string(&d, &map);
        assert!(out.contains("-- first here"), "got:\n{out}");
        assert!(out.contains("^^ again here"), "got:\n{out}");
    }

    #[test]
    fn notes_and_help_render_distinctly() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "x\n");
        let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "m")
            .with_primary(Span::new(f, 0, 1), "")
            .with_note("because of a reason")
            .with_help("try this instead");
        let out = render_to_string(&d, &map);
        assert!(out.contains("= note: because of a reason"), "got:\n{out}");
        assert!(out.contains("= help: try this instead"), "got:\n{out}");
    }

    #[test]
    fn suggestion_shows_the_replacement() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "$$name\n");
        let d = Diagnostic::error(
            code::E_VARIABLE_VARIABLE,
            "variable variables are not supported",
        )
        .with_primary(Span::new(f, 0, 6), "")
        .with_unsafe_fix(Span::new(f, 0, 6), "$vars[$name]", "use an explicit map");
        let out = render_to_string(&d, &map);
        assert!(
            out.contains("suggestion (review): use an explicit map: `$vars[$name]`"),
            "got:\n{out}"
        );
    }

    #[test]
    fn summary_counts_errors_and_warnings() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "abc\n");
        let items = [
            Diagnostic::error(code::E_EVAL_UNSUPPORTED, "one").with_primary(Span::new(f, 0, 1), ""),
            Diagnostic::error(code::E_EVAL_UNSUPPORTED, "two").with_primary(Span::new(f, 1, 2), ""),
            Diagnostic::warning(code::W_DEPRECATED, "three").with_primary(Span::new(f, 2, 3), ""),
        ];
        let mut buf = Vec::new();
        Renderer::new()
            .render_all(items.iter(), &map, &mut buf)
            .unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.ends_with(&format!(
                "error: aborting due to 2 errors; 1 warning emitted\n{AGENT_SHOW_LINE}\n"
            )),
            "got:\n{out}"
        );
        assert_eq!(out.matches(AGENT_SHOW_LINE).count(), 1, "got:\n{out}");
    }

    #[test]
    fn no_summary_when_only_warnings() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "abc\n");
        let items =
            [Diagnostic::warning(code::W_DEPRECATED, "w").with_primary(Span::new(f, 0, 1), "")];
        let mut buf = Vec::new();
        Renderer::new()
            .render_all(items.iter(), &map, &mut buf)
            .unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(!out.contains("aborting"), "got:\n{out}");
        // A warning has a card as an error does, so the run still names where
        // to read it.
        assert!(
            out.ends_with(&format!("{AGENT_SHOW_LINE}\n")),
            "got:\n{out}"
        );
    }

    #[test]
    fn no_agent_show_line_without_a_code() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "abc\n");
        let mut d =
            Diagnostic::error(code::E_EVAL_UNSUPPORTED, "e").with_primary(Span::new(f, 0, 1), "");
        d.code = None;
        let mut buf = Vec::new();
        Renderer::new().render_all([&d], &map, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("aborting due to 1 error"), "got:\n{out}");
        assert!(!out.contains(AGENT_SHOW_LINE), "got:\n{out}");
    }

    #[test]
    fn colour_output_contains_ansi_and_plain_does_not() {
        let mut map = SourceMap::new();
        let f = map.add("t.nvs", "abc\n");
        let d =
            Diagnostic::error(code::E_EVAL_UNSUPPORTED, "m").with_primary(Span::new(f, 0, 3), "");

        let mut plain = Vec::new();
        Renderer::new().render(&d, &map, &mut plain).unwrap();
        assert!(!String::from_utf8(plain).unwrap().contains('\x1b'));

        let mut colored = Vec::new();
        Renderer::new()
            .with_color(true)
            .render(&d, &map, &mut colored)
            .unwrap();
        assert!(String::from_utf8(colored).unwrap().contains('\x1b'));
    }

    #[test]
    fn tab_expansion_respects_stops() {
        assert_eq!(expand_tabs("\tx", 4), "    x");
        assert_eq!(expand_tabs("ab\tx", 4), "ab  x", "advance to the next stop");
        assert_eq!(expand_tabs("abcd\tx", 4), "abcd    x");
        assert_eq!(expand_tabs("no tabs", 4), "no tabs");
    }

    #[test]
    fn display_col_accounts_for_tabs() {
        assert_eq!(display_col("\tx", 1, 4), 4);
        assert_eq!(display_col("ab\tx", 3, 4), 4);
        assert_eq!(display_col("abc", 2, 4), 2);
    }

    #[test]
    fn digit_count_is_right_at_boundaries() {
        assert_eq!(decimal_digits(1), 1);
        assert_eq!(decimal_digits(9), 1);
        assert_eq!(decimal_digits(10), 2);
        assert_eq!(decimal_digits(999), 3);
        assert_eq!(decimal_digits(1000), 4);
    }
}
