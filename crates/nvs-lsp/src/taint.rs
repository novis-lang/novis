//! The path a `tainted` or `secret` value took inside its method, as a hover
//! (`rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from`).
//!
//! **The checker records the path, and this module only spells it.** Every
//! analysis this server runs asks the checker to record provenance
//! (`nvs_types::provenance`), so the path of each qualified value is on the
//! expression table by the span of the value. [`at`] picks the innermost
//! value under the cursor that has one. An argument at a sink is a value like
//! any other, so hovering it shows the path that reached the sink.
//!
//! **The innermost value wins over an answer around it.** A cursor on `$q`
//! in `Core\Html::escape($q)` would otherwise get the card of the call, and
//! [`crate::hover::answer`] keeps that card only where the path is on the
//! same node or none is under the cursor.
//!
//! A value whose qualifier was removed keeps its path, with the call or
//! conversion that removed it in bold, so a hover on the escaped text still
//! shows where it came from.

use std::fmt::Write as _;

use nvs_diagnostics::{BytePos, Span};
use nvs_types::provenance::{Step, TaintStep};

use crate::document::Analysed;

/// The longest piece of source text a step quotes before it is cut.
const QUOTE: usize = 60;

/// The path of the innermost value at `offset` that has one, spelled as
/// Markdown, and the value's span.
pub(crate) fn at(analysed: &Analysed, offset: BytePos) -> Option<(String, Span)> {
    let (span, path) = analysed
        .exprs
        .taint_paths()
        .filter(|(span, _)| {
            span.file == analysed.entry && span.start <= offset && offset <= span.end
        })
        .min_by_key(|(span, _)| (span.end - span.start, span.start))?;
    Some((render(analysed, path), span))
}

/// The heading and one numbered line per step.
fn render(analysed: &Analysed, path: &[TaintStep]) -> String {
    let tainted = path.iter().any(|step| step.tainted);
    let secret = path.iter().any(|step| step.secret);
    let names = qualifiers(tainted, secret);
    let mut out = if path.last().is_some_and(TaintStep::qualified) {
        format!("**This value is {names}.** It comes from:\n\n")
    } else {
        format!("**This value was {names}.** It comes from:\n\n")
    };
    let mut before: Option<&TaintStep> = None;
    for (n, step) in path.iter().enumerate() {
        let file = analysed.map.file(step.span.file);
        let line = file.line_col(step.span.start).0 + 1;
        let text = quote(file.text().get(step.span.range()).unwrap_or_default());
        let now = qualifiers(step.tainted, step.secret);
        let said = match step.step {
            Step::Source => format!("`{text}` returns a {now} value."),
            Step::Unchecked => {
                format!("`{text}` reads a value of type `mixed`. Text from `mixed` is `tainted`.")
            }
            Step::Declared => format!("`{text}` is declared {now}."),
            Step::Assigned => format!("the value is assigned to `{text}`."),
            Step::Concatenated => format!("the value is joined with other text in `{text}`."),
            Step::Interpolated => format!("the value is inserted into the string `{text}`."),
            Step::Carried => format!("`{text}` returns a value that is still {now}."),
            Step::Ended => {
                let removed = before.map_or_else(String::new, |prev| {
                    qualifiers(prev.tainted && !step.tainted, prev.secret && !step.secret)
                });
                format!("**`{text}` removes {removed}.**")
            }
        };
        let _ = writeln!(out, "{}. Line {line}: {said}", n + 1);
        before = Some(step);
    }
    out
}

/// `tainted`, `secret`, or both, as Markdown code.
fn qualifiers(tainted: bool, secret: bool) -> String {
    match (tainted, secret) {
        (true, true) => "`tainted` and `secret`".to_owned(),
        (false, true) => "`secret`".to_owned(),
        _ => "`tainted`".to_owned(),
    }
}

/// `text` on one line, cut at [`QUOTE`] characters, and safe inside a
/// Markdown code span.
fn quote(text: &str) -> String {
    let line: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('`', "'");
    if line.chars().count() <= QUOTE {
        return line;
    }
    let cut: String = line.chars().take(QUOTE).collect();
    format!("{cut}…")
}
