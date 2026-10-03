//! Which of an analysis's diagnostics an editor is shown, and what one looks
//! like once it is on the wire.
//!
//! `rule:ide/diagnostics-are-phase-gated` is [`phase_gated`] and nothing else:
//! a file that has produced a lexer (`E00xx`) or parser (`E01xx`) error
//! publishes those and its declaration diagnostics, and its own resolution and
//! type diagnostics are held back — because resolution walking an
//! `ExprKind::Error` reports "assigned to but never declared" *above* the
//! `E0102` that caused it, and a wall of red whose topmost entry is wrong on
//! every keystroke mid-statement teaches a developer to stop reading squiggles.
//!
//! **The band is the phase.** Codes are allocated per compiler phase
//! (`crates/nvs-diagnostics/src/lib.rs` is the legend), so the two digits after
//! the `E0` say which phase reported a diagnostic without anything having to
//! carry a phase tag through the front end. The type phase spans three bands
//! rather than one — it filled `E04xx` at `E0499` and `E07xx` at `E0799` and
//! continues in `E08xx` — and all three are gated together, because a code's
//! band is an allocation detail and its phase is what the rule is about.
//!
//! **An error opens the gate, a warning does not.** The rule's reason is a
//! parse that produced a broken tree, and a lexer or parser *warning* produced
//! a whole one: there is no `ExprKind::Error` under it for the phases below to
//! cascade from, so a documentation warning would otherwise silence the type
//! check of a file that parses perfectly.
//!
//! [`to_wire`] is the other half: one `nvs_diagnostics::Diagnostic` as one
//! `lsp_types::Diagnostic`. It is here rather than in [`crate::render`] because
//! the two answer different questions — this builds the value the client is
//! sent, and that module freezes the text a `.lspt` case compares against, over
//! the same value.
//!
//! **This is presentation, not analysis.** [`crate::analyse`] runs every phase
//! and keeps every diagnostic, so nothing is lost and `nvs check` is untouched.
//! The gate is a filter over a finished walk rather than a bail-out inside one,
//! which is what lets [`Phases::All`] hand back exactly what it held — a `.lspt`
//! case asking `phase=all` is the same walk read with the filter switched off.
//!
//! [`for_document`] is what the two halves compose into, and the one function
//! the server and the suite both call: the gate, narrowed to the entry file,
//! crossed to the wire.
//!
//! **A completion file's `strict` and `deprecated` are read here and nowhere
//! else.** [`from_completion_files`] compares each string literal argument with
//! the values a completion file lists for its parameter: a strict parameter
//! warns on a literal that is none of them, and a literal equal to a
//! deprecated value gets a hint with the `Deprecated` tag. Both are the
//! language server's alone, and `nvs check` never reads a completion file
//! (`rule:ide/completion-files-offer-values-at-named-parameters`). Their cost
//! is one table lookup per string literal argument per analysis.

use std::collections::BTreeSet;
use std::sync::Arc;

use lsp_types::{DiagnosticSeverity, DiagnosticTag, NumberOrString, Range};
use nvs_diagnostics::{
    Code, Diagnostic, Diagnostics, PositionEncoding, Severity, SourceFile, SourceId, Span, code,
};
use nvs_syntax::string_lit::cook_string_literal;

use crate::arguments;
use crate::completion_files::{CompletionFiles, Value};
use crate::document::Analysed;
use crate::index::{CheckScope, Declaration, Import};
use crate::position::position_at;

/// The bands whose error means this file's tree is broken, so what the phases
/// below it made of that tree is not worth showing: the lexer's and the
/// parser's.
const A_BROKEN_TREE: &[&str] = &["00", "01"];

/// The bands reported by the phases that read the tree the two above build:
/// name resolution, and the type check across all three bands it occupies.
const READS_THE_TREE: &[&str] = &["03", "04", "07", "08"];

/// The two digits that name a code's phase — `E0102` is the parser's `01`.
///
/// Empty for a code too short to have one, which no `Code::new` in the registry
/// is; a code that somehow lacked a band belongs to no phase and so is gated by
/// nothing, which is the right way for this to fail.
fn band(code: Code) -> &'static str {
    code.as_str().get(1..3).unwrap_or_default()
}

/// The file a diagnostic is about, or `None` for one that points nowhere.
fn file_of(diagnostic: &Diagnostic) -> Option<SourceId> {
    diagnostic.primary_span().map(|span| span.file)
}

/// Whether `diagnostic`'s code is in one of `bands`.
fn in_bands(diagnostic: &Diagnostic, bands: &[&str]) -> bool {
    diagnostic
        .code
        .is_some_and(|code| bands.contains(&band(code)))
}

/// The diagnostics of `diags` an editor publishes, with the phase gate applied.
///
/// Order is preserved, and every diagnostic that survives is the one the walk
/// reported — this filters and never rewrites, so the codes, spans and messages
/// a `.lspt` case freezes are the compiler's own.
#[must_use]
pub fn phase_gated(diags: &Diagnostics) -> Vec<&Diagnostic> {
    let broken: BTreeSet<SourceId> = diags
        .iter()
        .filter(|diagnostic| diagnostic.is_error() && in_bands(diagnostic, A_BROKEN_TREE))
        .filter_map(file_of)
        .collect();

    diags
        .iter()
        .filter(|diagnostic| {
            !in_bands(diagnostic, READS_THE_TREE)
                || file_of(diagnostic).is_none_or(|file| !broken.contains(&file))
        })
        .collect()
}

/// Whether the phase gate is applied to what a document publishes.
///
/// An editor is always sent [`Gated`](Phases::Gated): the rule is what a person
/// typing sees and there is no setting that turns it off.
/// [`All`](Phases::All) is the `.lspt` case that pins the gate from the other
/// side, because a filter is only shown to be filtering by what it holds back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phases {
    /// [`phase_gated`] applied, which is `rule:ide/diagnostics-are-phase-gated`.
    #[default]
    Gated,
    /// Every phase's, gate and all — `diagnostics phase=all` and nothing a
    /// client ever asks for.
    All,
}

/// The diagnostics an editor is sent for one analysed document, positioned in
/// `encoding`.
///
/// Three things happen here: the phase gate is applied unless `phases` defeats
/// it, what survives is narrowed to the entry file, and that crosses to the
/// wire. The second is the one that is easy to miss. The diagnostics of
/// [`from_completion_files`], which read `files` and no compiler phase, join
/// them.
///
/// **The entry file only.** One analysis reads a whole `require` graph and
/// reports over all of it, but a `publishDiagnostics` notification is about one
/// URI. A diagnostic in a file the entry required belongs to *that* file's own
/// notification, which it gets when it is open and analysed as its own entry
/// point — and which is nothing at all when nobody opened it, because
/// publishing for an unopened file is workspace scope and
/// `rule:ide/an-open-document-is-its-own-entry-point` puts that at M10.
///
/// A diagnostic with no span at all is kept rather than dropped: it belongs to
/// no file, so filtering by file is how it would be lost everywhere at once,
/// and [`to_wire`] lands it at the start of the entry where it can be reported.
#[must_use]
pub fn for_document(
    analysed: &Analysed,
    files: &CompletionFiles,
    phases: Phases,
    encoding: PositionEncoding,
) -> Vec<lsp_types::Diagnostic> {
    let surviving = match phases {
        Phases::Gated => phase_gated(&analysed.diags),
        Phases::All => analysed.diags.iter().collect(),
    };
    let entry = analysed.map.file(analysed.entry);
    let listed = from_completion_files(analysed, files);
    surviving
        .into_iter()
        .filter(|diagnostic| file_of(diagnostic).is_none_or(|file| file == analysed.entry))
        .chain(&listed)
        .map(|diagnostic| to_wire(diagnostic, entry, encoding))
        .collect()
}

/// What the completion files say about the string literal arguments in the
/// entry document (`rule:ide/completion-files-offer-values-at-named-parameters`
/// § *What else reads it*): the warning of [`unknown_value`] and the hint of
/// [`deprecated_value`], in the order the literals are written.
///
/// [`for_document`] publishes these and [`crate::actions::at`] offers their
/// fixes, so both read this one walk and a quick fix is offered exactly where
/// its hint is shown.
#[must_use]
pub(crate) fn from_completion_files(
    analysed: &Analysed,
    files: &CompletionFiles,
) -> Vec<Diagnostic> {
    let entry = analysed.map.file(analysed.entry);
    let checked = entry
        .path()
        .is_some_and(|path| path.extension().is_some_and(|extension| extension == "nvs"));
    arguments::named_in_document(analysed)
        .into_iter()
        .flat_map(|named| {
            let text = cook_string_literal(entry, named.span);
            let values =
                files.values_at(&named.class, &named.method, &named.parameter, &named.others);
            let unknown = checked
                .then(|| unknown_value(files, &named, &text, &values))
                .flatten();
            unknown
                .into_iter()
                .chain(deprecated_value(entry, &named, &text, &values))
        })
        .collect()
}

/// A warning on `named` when a completion file marks its parameter `strict`
/// and `text` is none of the `values` that apply at its call.
///
/// Only a `.nvs` file is checked, which [`from_completion_files`] decides. A
/// literal is skipped where an attachment's `when` names an argument the call
/// leaves out or writes as something other than one string literal, because
/// which values apply there is not known. A value built at run time is not a
/// string literal, so it is never checked.
fn unknown_value(
    files: &CompletionFiles,
    named: &arguments::Named,
    text: &str,
    values: &[Arc<Value>],
) -> Option<Diagnostic> {
    let attachments = files.attachments(&named.class, &named.method, &named.parameter);
    if !attachments.iter().any(|attachment| attachment.strict) {
        return None;
    }
    let undecided = attachments
        .iter()
        .filter_map(|attachment| attachment.when.as_ref())
        .any(|when| {
            !named
                .others
                .iter()
                .any(|(name, text)| *name == when.parameter && text.is_some())
        });
    if undecided || values.iter().any(|value| value.value == text) {
        return None;
    }
    Some(
        Diagnostic::warning(
            code::W_COMPLETION_VALUE_UNKNOWN,
            format!(
                "`{text}` is not one of the values the completion files list for `${}` of `{}::{}`",
                named.parameter, named.class, named.method
            ),
        )
        .with_primary(named.span, "not a listed value"),
    )
}

/// A hint on `named` when `text` equals a deprecated one of the `values` that
/// apply at its call. [`to_wire`] gives it the `Deprecated` tag.
///
/// When the value names a `replacement`, the hint carries it as a suggestion
/// whose span is the text between the quotes, so the fix changes only the
/// string. The replacement is escaped for the literal's quote the way a
/// completion item's text is ([`crate::completion::escaped`]). It is not
/// `safe`: the program then passes a different value.
fn deprecated_value(
    entry: &SourceFile,
    named: &arguments::Named,
    text: &str,
    values: &[Arc<Value>],
) -> Option<Diagnostic> {
    let value = values
        .iter()
        .find(|value| value.value == text && value.deprecated)?;
    let message = match &value.replacement {
        Some(replacement) => format!("`{text}` is deprecated, use `{replacement}`"),
        None => format!("`{text}` is deprecated"),
    };
    let hint = Diagnostic::new(Severity::Help, message)
        .with_code(code::W_COMPLETION_VALUE_DEPRECATED)
        .with_primary(named.span, "deprecated");
    let Some(replacement) = &value.replacement else {
        return Some(hint);
    };
    let quote = entry.text()[named.span.start as usize..]
        .chars()
        .next()
        .unwrap_or('\'');
    let inside = Span::new(named.span.file, named.span.start + 1, named.span.end - 1);
    Some(hint.with_unsafe_fix(
        inside,
        crate::completion::escaped(replacement, quote),
        format!("replace with `{replacement}`"),
    ))
}

/// The dimming an editor is sent for one document: one `Unnecessary` tag per
/// private declaration in it that nothing in the index refers to, and one per
/// `use` import that no name in its scope reads.
///
/// **Silent at open scope**, and that is the rule rather than a shortcut.
/// `rule:ide/check-scope-defaults-to-the-workspace`'s `"open"` indexes the open
/// documents and their graph alone, and a member unreferenced across that
/// much of a workspace is not a member that is unreferenced — so where the
/// index does not span the workspace the honest answer is nothing at all,
/// rather than a guess a client renders in grey.
///
/// `unused` and `imports` are the answer to *which* names, which is
/// [`crate::SymbolIndex::unused_private`]'s and
/// [`crate::SymbolIndex::unused_imports`]'s, and this is the crossing to the
/// wire — the same split [`for_document`] makes between the gate and
/// [`to_wire`].
///
/// A tag carries no `code`. Every other diagnostic here is one the compiler
/// produced and a reader can look up; this one is a reading of the index that
/// no phase reports, so a code would name a check that does not exist.
#[must_use]
pub fn dimming(
    unused: &[&Declaration],
    imports: &[&Import],
    scope: CheckScope,
    analysed: &Analysed,
    encoding: PositionEncoding,
) -> Vec<lsp_types::Diagnostic> {
    if scope != CheckScope::Workspace {
        return Vec::new();
    }
    let file = analysed.map.file(analysed.entry);
    let faded = |site: &crate::Site, message: String| lsp_types::Diagnostic {
        range: Range::new(
            position_at(file, site.start, encoding),
            position_at(file, site.end, encoding),
        ),
        // A hint, so an editor fades the name rather than listing it beside
        // the errors: nothing here is wrong, and a private member written
        // before its first caller is an ordinary minute of work.
        severity: Some(DiagnosticSeverity::HINT),
        tags: Some(vec![DiagnosticTag::UNNECESSARY]),
        source: Some(SOURCE.to_owned()),
        message,
        ..lsp_types::Diagnostic::default()
    };
    let members = unused.iter().map(|declared| {
        faded(
            &declared.site,
            format!(
                "{} `{}` is private and nothing in this workspace uses it",
                declared.kind.describe(),
                declared.symbol
            ),
        )
    });
    let imported = imports.iter().map(|import| {
        faded(
            &import.site,
            format!(
                "`{}` is imported and nothing in this file uses it",
                import.symbol
            ),
        )
    });
    members.chain(imported).collect()
}

/// What a diagnostic's `source` field says produced it.
///
/// The compiler, not the server: an editor shows this beside the message to
/// separate one producer's diagnostics from another's in the same file, and
/// `E0401` is the type checker's answer whether it arrived over LSP or out of
/// `nvs check`. Deliberately not [`crate::SERVER_NAME`], which names the
/// process an operator has to start.
pub const SOURCE: &str = "nvs";

/// `severity` as the wire spells it.
///
/// A `Bug` crosses as an error rather than as a category of its own: LSP has
/// four severities and none of them means "the compiler is broken", and the one
/// thing that must not happen to an internal error is that it renders more
/// quietly than the mistakes it is a symptom of.
const fn severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Note => DiagnosticSeverity::INFORMATION,
        Severity::Help => DiagnosticSeverity::HINT,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Error | Severity::Bug => DiagnosticSeverity::ERROR,
    }
}

/// `diagnostic` as the wire carries it, positioned against `file` in
/// `encoding`.
///
/// `file` is the file the diagnostic points into and `encoding` is the one
/// [`crate::negotiate_encoding`] settled on; every column here is
/// [`crate::position_at`]'s, because `rule:ide/positions-have-one-home` puts
/// the arithmetic in `nvs-diagnostics` and this crate does none of it.
///
/// A diagnostic pointing nowhere lands at the start of the file rather than
/// being dropped. It is a compiler bug when one has no span at all, and an
/// editor showing it on line 1 is how that gets reported.
///
/// Two things are deliberately not carried yet, each waiting for the slice that
/// has somewhere to put it: a secondary label wants `relatedInformation`, which
/// needs the `Uri` of a file that is not necessarily this one, and a
/// [`nvs_diagnostics::Suggestion`] is a code action, which is
/// `textDocument/codeAction`'s. The one tag set here is `Deprecated`, on
/// `W1021`, the hint [`from_completion_files`] gives a deprecated value. The
/// `Unnecessary` tag is not set here: no compiler diagnostic carries it, and
/// [`dimming`] is where the index produces one instead.
#[must_use]
pub fn to_wire(
    diagnostic: &Diagnostic,
    file: &SourceFile,
    encoding: PositionEncoding,
) -> lsp_types::Diagnostic {
    let range = diagnostic
        .primary_span()
        .map_or_else(Range::default, |span| Range {
            start: position_at(file, span.start, encoding),
            end: position_at(file, span.end, encoding),
        });

    lsp_types::Diagnostic {
        range,
        severity: Some(severity(diagnostic.severity)),
        code: diagnostic
            .code
            .map(|code| NumberOrString::String(code.as_str().to_owned())),
        // Left unset, and not for want of a value to put in it: ADR 0099 § 3
        // has no documentation site for a code to point at, and a
        // `codeDescription` whose URL 404s is worse than the code alone,
        // which a person can at least search for.
        code_description: None,
        source: Some(SOURCE.to_owned()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: (diagnostic.code == Some(code::W_COMPLETION_VALUE_DEPRECATED))
            .then(|| vec![DiagnosticTag::DEPRECATED]),
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{SourceMap, Span, code};

    use super::*;

    /// A diagnostic of `code` over the whole of `file`, at `severity`.
    fn at(severity: Severity, diagnostic_code: Code, file: SourceId) -> Diagnostic {
        Diagnostic::new(severity, "a fixture")
            .with_code(diagnostic_code)
            .with_primary(Span::new(file, 0, 0), "")
    }

    /// The codes the gate let through, in order.
    fn published(diags: &Diagnostics) -> Vec<&'static str> {
        phase_gated(diags)
            .iter()
            .filter_map(|diagnostic| diagnostic.code.map(Code::as_str))
            .collect()
    }

    /// The type phase occupies `E04xx`, `E07xx` and `E08xx` because the first
    /// two filled up, and a gate that knew only the first would publish an
    /// `E0801` under a parse error — the exact cascade the rule exists to stop,
    /// hidden behind which band a code happened to be allocated from.
    #[test]
    fn the_type_phase_is_gated_in_every_band_it_occupies() {
        let mut map = SourceMap::new();
        let broken = map.add("broken.nvs", "");

        let mut diags = Diagnostics::new();
        diags.report(at(Severity::Error, code::E_EXPECTED_EXPR, broken));
        for continued in [
            code::E_TYPE_MISMATCH,
            code::E_ASSIGN_BY_REFERENCE,
            code::E_ECHO_BESIDE_A_BODY_MEMBER,
        ] {
            diags.report(at(Severity::Error, continued, broken));
        }

        assert_eq!(
            published(&diags),
            vec![code::E_EXPECTED_EXPR.as_str()],
            "a type diagnostic survived a parse error because of the band it \
             was allocated from"
        );
    }

    /// The gate is opened by a broken tree, and a warning from the lexer or the
    /// parser did not break one.
    #[test]
    fn a_parser_warning_does_not_open_the_gate() {
        let mut map = SourceMap::new();
        let warned = map.add("warned.nvs", "");

        let mut diags = Diagnostics::new();
        diags.report(at(Severity::Warning, code::E_EXPECTED_EXPR, warned));
        diags.report(at(Severity::Error, code::E_TYPE_MISMATCH, warned));

        assert_eq!(
            published(&diags),
            vec![
                code::E_EXPECTED_EXPR.as_str(),
                code::E_TYPE_MISMATCH.as_str()
            ],
            "a warning that left the tree whole silenced the type check anyway"
        );
    }

    /// The crossing to the wire, field by field: the stable code travels as a
    /// string, `codeDescription` stays unset because ADR 0099 § 3 has nowhere
    /// to point one, and the range comes out of
    /// `rule:ide/positions-have-one-home`'s arithmetic rather than this
    /// module's — which is why the fixture's line holds a `ß`, whose UTF-16
    /// columns are not its byte offsets.
    #[test]
    fn a_diagnostic_carries_its_code_and_no_code_description() {
        let mut map = SourceMap::new();
        let id = map.add("case.nvs", "<?nvs\nvar $total = \"ß\" + 1;\n");
        let file = map.file(id);
        let text = file.text();
        let start = u32::try_from(text.find('"').expect("the fixture has a literal"))
            .expect("a short fixture");
        let end = start + u32::try_from("\"ß\"".len()).expect("a short literal");

        let reported = Diagnostic::error(code::E_TYPE_MISMATCH, "string is not int")
            .with_primary(Span::new(id, start, end), "here");
        let sent = to_wire(&reported, file, PositionEncoding::Utf16);

        assert_eq!(
            sent.code,
            Some(NumberOrString::String(
                code::E_TYPE_MISMATCH.as_str().to_owned()
            )),
            "the code a person searches for did not survive the crossing"
        );
        assert!(
            sent.code_description.is_none(),
            "a code description was invented for a documentation site that does \
             not exist"
        );
        assert_eq!(sent.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(sent.source.as_deref(), Some(SOURCE));
        assert_eq!(sent.message, "string is not int");
        assert_eq!(
            sent.range,
            Range {
                start: position_at(file, start, PositionEncoding::Utf16),
                end: position_at(file, end, PositionEncoding::Utf16),
            },
            "the range was computed here instead of in the crate that owns the \
             arithmetic"
        );

        let uncoded = Diagnostic::new(Severity::Warning, "no code at all")
            .with_primary(Span::new(id, start, end), "");
        assert!(
            to_wire(&uncoded, file, PositionEncoding::Utf16)
                .code
                .is_none(),
            "a diagnostic with no code was given one"
        );
    }

    /// A diagnostic pointing at no file belongs to no file, so no file's parse
    /// failure can be the reason to hide it.
    #[test]
    fn a_diagnostic_with_no_span_is_never_gated() {
        let mut map = SourceMap::new();
        let broken = map.add("broken.nvs", "");

        let mut diags = Diagnostics::new();
        diags.report(at(Severity::Error, code::E_EXPECTED_EXPR, broken));
        diags.report(Diagnostic::error(code::E_TYPE_MISMATCH, "a fixture"));

        assert_eq!(
            published(&diags),
            vec![
                code::E_EXPECTED_EXPR.as_str(),
                code::E_TYPE_MISMATCH.as_str()
            ],
            "an unplaced diagnostic was attributed to the broken file"
        );
    }
}
