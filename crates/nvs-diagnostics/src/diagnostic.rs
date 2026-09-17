//! Diagnostic values: what the compiler wants to say about a span of source.
//!
//! Construction is a builder chain so that a call site reads as a sentence:
//!
//! ```
//! # use nvs_diagnostics::{Diagnostic, SourceMap, Span, code};
//! # let mut map = SourceMap::new();
//! # let f = map.add("t.nvs", "eval('x');");
//! # let span = Span::new(f, 0, 4);
//! let d = Diagnostic::error(code::E_EVAL_UNSUPPORTED, "`eval` is not supported in Novis")
//!     .with_primary(span, "remove this call")
//!     .with_note("Novis compiles ahead of execution, so runtime code generation \
//!                 cannot be type-checked, cached, or sandboxed")
//!     .with_help("restructure as a closure, a match, or a lookup table");
//! assert!(d.is_error());
//! ```

use std::collections::HashSet;
use std::fmt;

use crate::span::Span;

/// How serious a diagnostic is.
///
/// Ordering runs least to most severe, so `max` over a set of severities gives
/// the overall outcome.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Severity {
    /// Additional context attached to another diagnostic.
    Note,
    /// A suggested change.
    Help,
    /// Suspicious but compilable.
    Warning,
    /// Compilation cannot produce a result.
    Error,
    /// The compiler itself is in a broken state. Always a bug.
    Bug,
}

impl Severity {
    /// Lower-case name, as it appears at the head of a rendered diagnostic.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Help => "help",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Bug => "internal compiler error",
        }
    }

    /// Whether a diagnostic at this severity prevents producing output.
    #[must_use]
    pub const fn is_fatal(self) -> bool {
        matches!(self, Self::Error | Self::Bug)
    }
}

/// A stable diagnostic code.
///
/// Codes are part of Novis's public interface: they appear in `#[allow]`-style
/// suppressions, in documentation, and in tooling that filters diagnostics. Once
/// published, a code's meaning never changes — retire it instead of reusing it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Code(&'static str);

impl Code {
    /// Defines a code. Intended for the constants in [`crate::code`].
    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(s)
    }

    /// The code as it is displayed, e.g. `"E0104"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Whether a label marks the cause or merely relates to it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum LabelStyle {
    /// The span the diagnostic is really about. Rendered with `^^^`.
    Primary,
    /// Supporting context — a prior declaration, the opening brace of an
    /// unclosed block. Rendered with `---`.
    Secondary,
}

/// A span with something to say about it.
#[derive(Clone, Debug)]
pub struct Label {
    /// Where.
    pub span: Span,
    /// What about it. May be empty, when the main message says enough.
    pub message: String,
    /// Cause or context.
    pub style: LabelStyle,
}

/// A machine-applicable edit.
///
/// Consumed by the LSP as a code action, by `nvs convert` when rewriting PHP,
/// and printed as a suggestion on the terminal.
#[derive(Clone, Debug)]
pub struct Suggestion {
    /// The range to replace.
    pub span: Span,
    /// The text to put there. Empty means deletion.
    pub replacement: String,
    /// How to describe the edit to a human.
    pub message: String,
    /// Whether applying this is known to preserve behaviour.
    ///
    /// `false` means a human must review it — `nvs convert` uses this to decide
    /// between rewriting silently and leaving a `TODO`.
    pub safe: bool,
}

/// One thing the compiler has to say.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// How serious.
    pub severity: Severity,
    /// Stable identifier, if this diagnostic has one.
    pub code: Option<Code>,
    /// The headline. One line, lower case, no trailing period.
    pub message: String,
    /// Spans this diagnostic points at. The first primary label is where the
    /// renderer centres its output.
    pub labels: Vec<Label>,
    /// Longer explanation, rendered below the source snippet.
    pub notes: Vec<String>,
    /// Suggested changes.
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    /// A diagnostic with no labels yet.
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// An error with a stable code.
    #[must_use]
    pub fn error(code: Code, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message).with_code(code)
    }

    /// A warning with a stable code.
    #[must_use]
    pub fn warning(code: Code, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message).with_code(code)
    }

    /// An internal compiler error. Reaching one is always a bug in Novis.
    #[must_use]
    pub fn bug(message: impl Into<String>) -> Self {
        Self::new(Severity::Bug, message)
    }

    /// Attaches a stable code.
    #[must_use]
    pub fn with_code(mut self, code: Code) -> Self {
        self.code = Some(code);
        self
    }

    /// Adds the primary label — the span this diagnostic is about.
    #[must_use]
    pub fn with_primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        });
        self
    }

    /// Adds a supporting label.
    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        });
        self
    }

    /// Adds an explanatory note, rendered after the snippet.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Adds a help line. Shorthand for a note prefixed as help.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.notes.push(format!("help: {}", help.into()));
        self
    }

    /// Adds a machine-applicable edit that is known to preserve behaviour.
    #[must_use]
    pub fn with_fix(
        mut self,
        span: Span,
        replacement: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.suggestions.push(Suggestion {
            span,
            replacement: replacement.into(),
            message: message.into(),
            safe: true,
        });
        self
    }

    /// Adds an edit that a human must review before applying.
    #[must_use]
    pub fn with_unsafe_fix(
        mut self,
        span: Span,
        replacement: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.suggestions.push(Suggestion {
            span,
            replacement: replacement.into(),
            message: message.into(),
            safe: false,
        });
        self
    }

    /// Whether this prevents compilation.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        self.severity.is_fatal()
    }

    /// The primary label's span, falling back to the first label of any style.
    #[must_use]
    pub fn primary_span(&self) -> Option<Span> {
        self.labels
            .iter()
            .find(|l| l.style == LabelStyle::Primary)
            .or_else(|| self.labels.first())
            .map(|l| l.span)
    }
}

/// Collects diagnostics for one compilation.
///
/// The compiler reports into this and keeps going wherever it can, so a single
/// run surfaces every problem rather than only the first — but it says each
/// one **once**: a diagnostic identical to one already held is dropped by
/// [`Diagnostics::report`]. Several passes reach the same written type, since
/// a signature's types are lowered for the signature and again for the body
/// that binds them, and the second copy of one mistake at one span tells a
/// reader nothing while doubling every error count.
#[derive(Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
    errors: usize,
    /// One key per diagnostic currently in `items`, kept in step with it by
    /// every method that adds or removes one.
    seen: HashSet<String>,
}

/// What was reported, and how much of it was fatal.
///
/// Written out rather than derived because `seen` is a `HashSet` and its
/// iteration order is not the same twice: two sinks holding the very same
/// diagnostics would print differently, which is a comparison this repository
/// makes — `crates/nvs-syntax/src/parser/tests/mod.rs` holds one entry point's
/// report against the other's. The set is derived from `items` and says nothing
/// `items` does not, so leaving it out costs a reader nothing.
impl fmt::Debug for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Diagnostics")
            .field("items", &self.items)
            .field("errors", &self.errors)
            .finish()
    }
}

/// What makes two diagnostics the same one: every field a reader can see.
/// Identical labels mean identical spans, so this can only collapse two
/// reports of one mistake at one site, never two sites that happen to share a
/// message.
fn identity(d: &Diagnostic) -> String {
    format!("{d:?}")
}

impl Diagnostics {
    /// An empty sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a diagnostic, unless one identical to it is already held.
    pub fn report(&mut self, d: Diagnostic) {
        if !self.seen.insert(identity(&d)) {
            return;
        }
        if d.is_error() {
            self.errors += 1;
        }
        self.items.push(d);
    }

    /// Whether anything fatal was reported.
    #[must_use]
    pub const fn has_errors(&self) -> bool {
        self.errors > 0
    }

    /// How many fatal diagnostics were reported.
    #[must_use]
    pub const fn error_count(&self) -> usize {
        self.errors
    }

    /// Every diagnostic, in the order reported.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Diagnostic> {
        self.items.iter()
    }

    /// Total number of diagnostics at any severity.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether nothing at all was reported.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Sorts diagnostics by source position, so output order matches reading
    /// order regardless of the order compiler passes ran in.
    pub fn sort_by_position(&mut self) {
        // Diagnostics with no span sort last, so an internal error does not
        // displace the located diagnostics a user can act on.
        self.items.sort_by_key(|d| {
            d.primary_span()
                .map_or((u32::MAX, u32::MAX), |s| (s.file.raw(), s.start))
        });
    }

    /// Takes the diagnostics out, leaving the sink empty.
    #[must_use]
    pub fn take(&mut self) -> Vec<Diagnostic> {
        self.errors = 0;
        self.seen.clear();
        std::mem::take(&mut self.items)
    }

    /// Discards every diagnostic reported after the first `len`, undoing a
    /// speculative parse that decided to backtrack. `len` must be `<=
    /// self.len()` — it always is when it came from an earlier call to
    /// [`Self::len`] on this same sink.
    /// A withdrawn diagnostic gives up its identity too, so the same one
    /// reported again down the path the parser backtracked onto is recorded
    /// rather than mistaken for a duplicate.
    pub fn truncate(&mut self, len: usize) {
        for d in self.items.drain(len..) {
            if d.is_error() {
                self.errors -= 1;
            }
            self.seen.remove(&identity(&d));
        }
    }
}

impl Extend<Diagnostic> for Diagnostics {
    fn extend<T: IntoIterator<Item = Diagnostic>>(&mut self, iter: T) {
        for d in iter {
            self.report(d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::SourceId;

    const F: SourceId = SourceId(0);

    #[test]
    fn severity_orders_least_to_most_serious() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Help);
        assert!(Severity::Bug > Severity::Error);
        assert!(Severity::Error.is_fatal());
        assert!(!Severity::Warning.is_fatal());
    }

    #[test]
    fn sink_counts_only_fatal_diagnostics() {
        let mut d = Diagnostics::new();
        d.report(Diagnostic::warning(Code::new("W0001"), "hmm"));
        assert!(!d.has_errors());
        d.report(Diagnostic::error(Code::new("E0001"), "no"));
        assert!(d.has_errors());
        assert_eq!(d.error_count(), 1);
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn one_mistake_reported_twice_is_held_once() {
        let mut d = Diagnostics::new();
        let twice = || {
            Diagnostic::error(Code::new("E0405"), "`Sub` has no constant named `Id`")
                .with_primary(Span::new(F, 10, 17), "referenced here")
        };
        d.report(twice());
        d.report(twice());
        assert_eq!(d.len(), 1);
        assert_eq!(d.error_count(), 1);
        // A different span is a different site, not a duplicate.
        d.report(
            Diagnostic::error(Code::new("E0405"), "`Sub` has no constant named `Id`")
                .with_primary(Span::new(F, 40, 47), "referenced here"),
        );
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn a_truncated_diagnostic_can_be_reported_again() {
        let mut d = Diagnostics::new();
        let mark = d.len();
        d.report(Diagnostic::error(Code::new("E0101"), "expected `;`"));
        d.truncate(mark);
        assert_eq!(d.len(), 0);
        assert_eq!(d.error_count(), 0);
        d.report(Diagnostic::error(Code::new("E0101"), "expected `;`"));
        assert_eq!(d.len(), 1);
        assert_eq!(d.error_count(), 1);
    }

    #[test]
    fn primary_span_prefers_primary_over_secondary() {
        let d = Diagnostic::error(Code::new("E0001"), "x")
            .with_secondary(Span::new(F, 100, 105), "context")
            .with_primary(Span::new(F, 10, 15), "here");
        assert_eq!(d.primary_span(), Some(Span::new(F, 10, 15)));
    }

    #[test]
    fn primary_span_falls_back_to_any_label() {
        let d = Diagnostic::error(Code::new("E0001"), "x")
            .with_secondary(Span::new(F, 100, 105), "context");
        assert_eq!(d.primary_span(), Some(Span::new(F, 100, 105)));
    }

    #[test]
    fn sorting_puts_diagnostics_in_reading_order() {
        let mut d = Diagnostics::new();
        d.report(Diagnostic::error(Code::new("E1"), "late").with_primary(Span::new(F, 50, 51), ""));
        d.report(Diagnostic::error(Code::new("E2"), "early").with_primary(Span::new(F, 5, 6), ""));
        d.sort_by_position();
        let msgs: Vec<_> = d.iter().map(|x| x.message.as_str()).collect();
        assert_eq!(msgs, ["early", "late"]);
    }

    #[test]
    fn unlabelled_diagnostics_sort_last() {
        let mut d = Diagnostics::new();
        d.report(Diagnostic::bug("no span"));
        d.report(
            Diagnostic::error(Code::new("E2"), "located").with_primary(Span::new(F, 5, 6), ""),
        );
        d.sort_by_position();
        assert_eq!(d.iter().next().unwrap().message, "located");
    }

    #[test]
    fn take_resets_the_error_count() {
        let mut d = Diagnostics::new();
        d.report(Diagnostic::error(Code::new("E1"), "x"));
        let taken = d.take();
        assert_eq!(taken.len(), 1);
        assert!(!d.has_errors());
        assert!(d.is_empty());
    }
}
