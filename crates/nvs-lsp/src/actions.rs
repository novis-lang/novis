//! What may be fixed at the cursor, and which diagnostic each fix came from.
//!
//! `textDocument/codeAction` is a translation and not an analysis. Every fix
//! this server offers is made of [`nvs_diagnostics::Suggestion`]s a diagnostic
//! is already carrying — the casing rename (`rule:core-api/identifier-casing`),
//! the legacy cast's `expr as T` (`rule:types/no-legacy-cast`), an undeclared
//! name's import, and a deprecated use's rewrite with the `use` line it needs
//! (`rule:attributes/a-deprecation-names-its-replacement-as-code`) come from the
//! compiler, and a deprecated completion-file value's `replacement` is the one
//! the server adds — so this module reads the diagnostics `publishDiagnostics`
//! sends anyway and rewrites what it finds into [`crate::render::Action`]. A
//! `W1003`'s suggestions are one action; every other suggestion is an action of
//! its own. That is
//! `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows` in
//! full: a fix the checker would have to compute has no `Suggestion` behind it,
//! so there is nothing here for it to be translated *from*, and it is offered
//! by nothing rather than refused by a list.
//!
//! Two actions are not fixes. [`crate::html_template`]'s refactor rewrites a
//! string as an html template, and [`crate::split_join`]'s puts a list on
//! separate lines or joins it onto one. Each is appended to the quick fixes
//! under its own kind and never to a fix-all: the first changes what the line
//! prints, and the second changes a layout that is the author's choice.
//!
//! # Decision: the gate an editor sees is the gate an action is offered behind
//!
//! [`crate::diagnostics::phase_gated`] is applied here too, so an action is
//! offered exactly where a squiggle is. A fix on a diagnostic
//! `rule:ide/diagnostics-are-phase-gated` is holding back would be a light bulb
//! with nothing under it — the developer would be offered a rename for a name
//! the editor is deliberately not complaining about, mid-statement, on a tree
//! the parser has already said is broken.
//!
//! # Decision: the range asked over is the diagnostic's, not the fix's
//!
//! An action is offered when the *diagnostic's* primary span meets the range
//! the client asked over, and the edit it carries may be anywhere: that is
//! what a person means by clicking on the squiggle. An import edits the `use`
//! lines at the top of the file, and it is reached from the name it is for.
//!
//! # Decision: a fix-all writes one `use` line once
//!
//! Two deprecated uses that need the same import each carry the same `use`
//! edit. Under [`Kind::FixAll`] an `also` edit an earlier action carries is
//! dropped from every later one, because an editor applies all of them on save.
//! A light bulb applies one action, so under [`Kind::QuickFix`] each keeps its
//! own.
//!
//! # Decision: `safe` does not gate the light bulb
//!
//! [`nvs_diagnostics::Suggestion::safe`] says whether applying an edit is known
//! to preserve behaviour, and the legacy cast's is not: `(int)$x` truncates
//! where `$x as int` throws. An editor offers the edit anyway, because a code
//! action is applied by a person who was shown the title first
//! (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`). Suppressing the
//! offer would leave the one spelling Novis refuses with no way to reach the
//! one it wants.
//!
//! # Decision: an alternative is a light bulb, never a fix-all
//!
//! [`nvs_diagnostics::Suggestion::alternative`] marks one of several edits a
//! person chooses between, such as the two groupings `W1022` offers
//! (`rule:expressions/misread-grouping-warns`). [`Kind::FixAll`] leaves it out
//! and [`Kind::QuickFix`] keeps it, because a save that applied both would
//! write one over the other, and the choice between them is the reader's.
//!
//! # What it spends
//!
//! One pass over the diagnostics the analysis already holds, and one
//! [`Action`] per fix that survives — both O(diagnostics in the entry
//! document) and both dropped with the answer. A fix-all also compares each
//! `also` edit with the ones before it, which is quadratic in the imports one
//! save writes. No tree is walked and nothing is
//! parsed a second time.

use lsp_types::CodeActionKind;
use nvs_diagnostics::{BytePos, Diagnostic, PositionEncoding, SourceId, Suggestion, code};

use crate::completion_files::CompletionFiles;
use crate::diagnostics::{from_completion_files, phase_gated};
use crate::document::Analysed;
use crate::position::range_at;
use crate::render::{Action, Edit};

/// Which kind the actions of one answer carry.
///
/// LSP matches a kind by prefix, so `quickfix` does not answer a client asking
/// for `source.fixAll.nvs` and the same fix has to arrive under whichever kind
/// was asked for. Both are declared in [`crate::CODE_ACTION_KINDS`], and the
/// pair is the whole of what
/// `rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion` means by "composable
/// with format-on-save": `editor.codeActionsOnSave` asks for the second, a
/// light bulb asks for the first, and one translation answers both.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Kind {
    /// A fix a person is being offered, one at a time.
    #[default]
    QuickFix,
    /// The same fixes, as the batch `editor.codeActionsOnSave` runs.
    FixAll,
}

impl Kind {
    /// How the wire spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::QuickFix => "quickfix",
            Self::FixAll => "source.fixAll.nvs",
        }
    }

    /// The kind to answer a request whose context named `only`.
    ///
    /// [`FixAll`](Self::FixAll) only when it was asked for by name. A client
    /// that asked for nothing in particular gets
    /// [`QuickFix`](Self::QuickFix), which is what a light bulb shows, and so
    /// does one that asked for a kind this server does not produce — the
    /// filtering LSP specifies for that case is the client's, and answering
    /// under a kind nobody named would be this server deciding it.
    #[must_use]
    pub fn asked_for(only: Option<&[CodeActionKind]>) -> Self {
        let asked = only.is_some_and(|kinds| {
            kinds
                .iter()
                .any(|kind| kind.as_str() == Self::FixAll.name())
        });
        if asked { Self::FixAll } else { Self::QuickFix }
    }
}

/// Every fix offered over `[start, end)` of the entry document of `analysed`,
/// under `kind`, positioned in `encoding` — and, beside the quick fixes, the
/// html-template refactor when the range is on a string ([`crate::html_template`])
/// and the split or join when it is inside a list ([`crate::split_join`]).
///
/// The diagnostics read are the compiler's, phase-gated, and the ones `files`
/// gives the entry document, which is where a deprecated value's replacement
/// comes from.
///
/// Sorted by where the edit lands and then by title, so the order an editor
/// lists them in is the order a `.lspt` case freezes and neither depends on
/// which walk reported the diagnostic first.
///
/// Empty means *nothing to fix here*, and it is the answer for a document that
/// has no diagnostic under the cursor as much as for one whose diagnostic
/// carries no suggestion. The two are the same offer, which is the point of the
/// boundary this module draws.
#[must_use]
pub fn at(
    analysed: &Analysed,
    files: &CompletionFiles,
    start: BytePos,
    end: BytePos,
    kind: Kind,
    encoding: PositionEncoding,
) -> Vec<Action> {
    let file = analysed.map.file(analysed.entry);
    let listed = from_completion_files(analysed, files);
    let mut offered: Vec<Action> = phase_gated(&analysed.diags)
        .into_iter()
        .chain(&listed)
        .filter(|diagnostic| touches(diagnostic, analysed.entry, start, end))
        .flat_map(fixes)
        .filter(|fix| {
            fix.iter()
                .all(|suggestion| suggestion.span.file == analysed.entry)
        })
        .filter(|fix| kind == Kind::QuickFix || !fix[0].alternative)
        .map(|fix| Action {
            title: fix[0].message.clone(),
            kind: kind.name().to_owned(),
            range: range_at(file, fix[0].span, encoding),
            replacement: fix[0].replacement.clone(),
            also: fix[1..]
                .iter()
                .map(|suggestion| Edit {
                    range: range_at(file, suggestion.span, encoding),
                    replacement: suggestion.replacement.clone(),
                })
                .collect(),
        })
        .collect();
    if kind == Kind::QuickFix {
        offered.extend(crate::html_template::at(analysed, start, end, encoding));
        offered.extend(crate::split_join::at(analysed, start, encoding));
    }
    offered.sort_by(|left, right| order(left).cmp(&order(right)));
    if kind == Kind::FixAll {
        once_each(&mut offered);
    }
    offered
}

/// The fixes `diagnostic` offers, each as the suggestions one action applies.
///
/// A `W1003` is one fix. Its first suggestion is the rewrite, and the one after
/// it is the `use` line the rewrite needs, so applying either alone leaves code
/// that does not compile. Every other diagnostic offers each suggestion as a
/// fix of its own, which is how `W1022`'s two groupings stay a choice.
fn fixes(diagnostic: &Diagnostic) -> Vec<&[Suggestion]> {
    if diagnostic.code == Some(code::W_DEPRECATED) {
        return (!diagnostic.suggestions.is_empty())
            .then_some(diagnostic.suggestions.as_slice())
            .into_iter()
            .collect();
    }
    diagnostic.suggestions.chunks(1).collect()
}

/// Drops from each action of a fix-all an `also` edit an earlier action already
/// carries.
///
/// Two uses in one file that need the same import each carry the same `use`
/// line, at the same place. An editor applies every action of a fix-all, so
/// keeping both would write the line twice. The first action keeps it, and a
/// light bulb, which applies one action, keeps it on every one.
fn once_each(offered: &mut [Action]) {
    let mut seen: Vec<Edit> = Vec::new();
    for action in offered {
        action.also.retain(|edit| !seen.contains(edit));
        seen.extend(action.also.iter().cloned());
    }
}

/// What one action sorts by: where its edit lands, then what it is called.
fn order(action: &Action) -> ((u32, u32, u32, u32), &str) {
    (
        (
            action.range.start.line,
            action.range.start.character,
            action.range.end.line,
            action.range.end.character,
        ),
        action.title.as_str(),
    )
}

/// Whether `diagnostic` is about `entry` and meets `[start, end)`.
///
/// Touching counts, in both directions: the range a client sends is usually
/// empty — the cursor — and a cursor resting against either end of a name is
/// still on it, which is where a person expects the light bulb.
///
/// A diagnostic with no span at all is about no range, so it offers nothing.
/// [`crate::diagnostics::to_wire`] lands one at the start of the file so it can
/// be reported; an action there would be a fix attached to a position it says
/// nothing about.
fn touches(diagnostic: &Diagnostic, entry: SourceId, start: BytePos, end: BytePos) -> bool {
    diagnostic
        .primary_span()
        .is_some_and(|span| span.file == entry && span.start <= end && start <= span.end)
}
