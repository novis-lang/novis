//! What the declaration under the cursor documents.
//!
//! `textDocument/hover` answers three things in
//! `rule:ide/the-request-set-is-closed`'s list, and all three are arms here
//! rather than three modules, because they answer the same request at the same
//! cursor: the `///` run attached to the declaration the name under the cursor
//! resolves to, handed to the client as Markdown; a `Core` member's registry
//! row; and, for a cursor that reached neither, the declared type of what it is
//! on. They are tried in that order, so the most specific answer a cursor has
//! is the one it gets and the type is what is left when nothing documented it.
//!
//! **The walk from the cursor to the declaration is
//! [`crate::definition`]'s.** Hover and go-to-definition ask the same question
//! about *where* and differ only in what they read once they are there, so the
//! resolution is shared: `crate::definition::site` finds the declaration the
//! cursor's name resolves to, and this module takes its `///` run while
//! `definition` takes its span. A second walk here would be a second answer to
//! "where", and the two would disagree the first time one of them learned
//! something.
//!
//! **The run is already on the node, and nothing here rescans trivia.** The
//! parser attaches it while it parses the declaration
//! (`rule:tooling/doc-comment-attaches-to-the-next-declaration`), which is what
//! makes this reachable at all: [`crate::document::Analysed`] keeps the trivia
//! of the **entry** document only, and the declaration a cursor points at is
//! often in a required file. What is read instead is that file's statements,
//! which the graph walk parsed once and kept.
//!
//! **A `Core` member is answered before that walk, not after it fails.** It is
//! declared in Rust and has no `nvs_hir::Symbol`, so the walk reaches nothing
//! and there is no `///` run anywhere to find: what it documents is its
//! registry row's reference card (`rule:core-api/reference-card`), which is the
//! one artifact that provably matches shipped behaviour.
//!
//! **The signature line is spelled from the checker's own types, not from
//! `nvs_stdlib::registry::CoreTy`.** The row is what seeded them —
//! `nvs_types::core_lib` turns every registry row into the same
//! `ClassSignature` a user-declared class produces — so the parameter types and
//! names on the `nvs_types::ResolvedCall` the cursor is sitting on *are* the
//! row, already interned, and `nvs_types::TypeInterner::describe` is the one
//! home for spelling one of those. Reading `CoreTy` again here would be a
//! second spelling of the same fact, and `nvs meta --json`'s is the one that
//! already exists.
//!
//! **The type arm answers what the checker recorded, and a plain variable read
//! is not that.** A property access, a static or a hooked one, a subscript and
//! a read a dominating test narrowed each carry their type in
//! `nvs_types::ExprInfo`, so hovering one is a projection of the table
//! (`rule:ide/the-index-answers-the-cursor`). An ordinary `$total` carries no
//! entry at all — a local's type lives in a scope the checker drops rather than
//! on the read — so it answers nothing instead of being guessed at, and closing
//! that is a change to what the type phase keeps rather than an arm here.
//!
//! **A declaration with no run answers nothing**, rather than an empty popup.
//! `None` is what LSP has for "there is nothing to say here", and an editor
//! shows it as no popup at all; a `Hover` carrying an empty string is a box
//! that opens onto nothing every time a name is passed over.
//!
//! **A `@see` line is prose here, not a link.** ADR 0099 § 3 wants its target
//! rendered as one, and a link needs a URI: the target is a member, and the
//! `file:` URI of its declaration is what a client would follow. That is
//! [`crate::definition`]'s answer pointed at a name written in a comment rather
//! than in code, which is a resolution of its own and so a slice of its own.
//! Until then the tag renders as what was written, which is readable and not
//! wrong.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};
use nvs_diagnostics::{BytePos, PositionEncoding};
use nvs_stdlib::registry::{self, CoreMethod, MethodDoc};
use nvs_syntax::DOC_MARKER;
use nvs_syntax::ast::DocComment;
use nvs_types::{ExprInfo, ResolvedCall, TypeInterner};

use crate::definition::{Target, site, target_of};
use crate::document::Analysed;
use crate::position::range_at;

/// What the name at `offset` in the entry document documents.
///
/// `None` when the cursor is inside no node, and when nothing it is inside was
/// documented or typed: no `///` run above the declaration its name reached, no
/// registry row behind it, and no type recorded for the node itself.
#[must_use]
pub fn at(analysed: &Analysed, offset: BytePos, encoding: PositionEncoding) -> Option<Hover> {
    let (value, node) = analysed.index.at(offset).nodes().iter().find_map(|node| {
        let info = analysed.exprs.lookup(node.span)?;
        let value = target_of(info)
            .and_then(|target| core(analysed, &target).or_else(|| run(analysed, &target)))
            .or_else(|| declared(analysed, info))?;
        Some((value, node.span))
    })?;
    // A run of bare `///` markers is a run with nothing in it, and it takes the
    // same answer as no run at all rather than opening a popup on whitespace.
    if value.trim().is_empty() {
        return None;
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        // The node the answer came from, so the editor underlines the
        // expression the reader is on. It is in the entry document, which is
        // the only file a cursor is ever in, and never in the file the run was
        // written in.
        range: Some(range_at(analysed.map.file(analysed.entry), node, encoding)),
    })
}

/// The `///` run above the declaration `target` resolves to, as Markdown.
fn run(analysed: &Analysed, target: &Target<'_>) -> Option<String> {
    let declared = site(analysed, target)?;
    Some(markdown(
        analysed.map.file(declared.span.file).text(),
        declared.doc?,
    ))
}

/// The type one recorded expression was checked at, as the code block a client
/// renders — the answer for a node nothing documented.
///
/// The entries read are the ones that carry a type of their own. A node
/// carrying none answers nothing, and the walk above steps outward to the node
/// containing it, which is how a cursor inside an argument still reaches the
/// call around it when the argument itself was recorded as nothing.
fn declared(analysed: &Analysed, info: &ExprInfo) -> Option<String> {
    let ty = match info {
        ExprInfo::Property { ty, .. }
        | ExprInfo::StaticProperty { ty, .. }
        | ExprInfo::HookedProperty { ty, .. } => *ty,
        ExprInfo::Index { elem_ty, .. } => *elem_ty,
        ExprInfo::NarrowedRead { to } => *to,
        _ => return None,
    };
    Some(format!("```nvs\n{}\n```", analysed.interner.describe(ty)))
}

/// A `Core` member's row and reference card, or `None` for every other target.
///
/// The registry is asked rather than the symbol table, and a name it does not
/// carry falls through to the walk above: `nvs_hir` resolves any `Core\…`
/// without a declaration, so a member of a not-yet-implemented class is a name
/// the checker refused rather than a row this can render.
fn core(analysed: &Analysed, target: &Target<'_>) -> Option<String> {
    let Target::Method(call) = target else {
        return None;
    };
    let class = registry::class(&call.class.to_string())?;
    let member = class.members().find(|row| row.name == call.method)?;
    let mut value = format!(
        "```nvs\n{}\n```",
        signature(class.name, call, &analysed.interner)
    );
    if let Some(doc) = member.doc {
        value.push_str(&reference_card(member, doc));
    }
    Some(value)
}

/// One row as a program reads it — `Core\Str::length(string $s): uint`.
///
/// Spelled with `::` whether the member is static or an instance one, because
/// what this line answers is *which* member the cursor is on and a `Core`
/// member is reachable exactly one way (`rule:core-api/shape-rules` R20): the
/// class that declares it is the name a reader looks it up under, and no
/// receiver is in hand to write the other spelling with.
///
/// A default is not written. The checker records an omitted parameter's value
/// as a `nvs_types::ConstArg` for `nvs-ir` to materialize, and rendering one
/// would be a second spelling of `nvs meta --json`'s — what a caller may leave
/// out is the card's sentence about it, which is right below this line.
fn signature(class: &str, call: &ResolvedCall, interner: &TypeInterner) -> String {
    let last = call.param_tys.len().saturating_sub(1);
    let params: Vec<String> = call
        .param_tys
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let name = call.param_names.get(index).map_or("", String::as_str);
            let ty = interner.describe(*ty);
            // The variadic tail keeps the element type the row wrote, which is
            // what `ResolvedCall::param_tys` carries in that slot.
            if call.variadic && index == last {
                format!("{ty} ...${name}")
            } else {
                format!("{ty} ${name}")
            }
        })
        .collect();
    format!(
        "{class}::{}({}): {}",
        call.method,
        params.join(", "),
        interner.describe(call.return_ty)
    )
}

/// The card under the signature line: what the member does, its parameters,
/// what it answers with, and what it throws — every section that was written,
/// and nothing for one that was not.
///
/// The `$` sigil is on a positional parameter and not on an option, because
/// `MethodDoc::params` runs the positional entries first and then one per
/// option of a trailing bag, and an option is written as a key rather than as
/// an argument.
fn reference_card(member: &CoreMethod, doc: &MethodDoc) -> String {
    let mut out = String::new();
    if !doc.short.is_empty() {
        out.push_str("\n\n");
        out.push_str(doc.short);
    }
    if !doc.params.is_empty() {
        out.push_str("\n\n**Parameters**\n");
        let positional = member.positional().len();
        for (index, param) in doc.params.iter().enumerate() {
            let sigil = if index < positional { "$" } else { "" };
            out.push_str(&format!("\n- `{sigil}{}` — {}", param.name, param.desc));
            for key in param.shape {
                out.push_str(&format!(
                    "\n  - `{}` (`{}`) — {}",
                    key.key, key.ty, key.desc
                ));
            }
        }
    }
    if !doc.ret.is_empty() {
        out.push_str(&format!("\n\n**Returns** {}", doc.ret));
    }
    if !doc.errors.is_empty() {
        out.push_str("\n\n**Throws**\n");
        for error in doc.errors {
            out.push_str(&format!("\n- `{}` — {}", error.error, error.desc));
        }
    }
    out
}

/// One run's prose, as the Markdown a client renders.
///
/// The lines are spans into `text` and carry their markers, which is
/// `nvs_syntax::ast::DocComment`'s decision: the source is the text, and the
/// consumer is what decides where the prose starts.
fn markdown(text: &str, doc: &DocComment) -> String {
    doc.lines
        .iter()
        .map(|line| prose(text.get(line.range()).unwrap_or_default()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One line with its marker off, and the one space after it that a writer
/// leaves and does not mean.
///
/// Exactly one space and never a trim: indentation is Markdown's own syntax, so
/// eating the rest would turn the indented example a doc comment is often
/// mostly made of into a paragraph. The *trailing* whitespace does go, which
/// costs the two-space hard break — a line ending a `.lspt` expectation in
/// invisible spaces is one nobody can maintain, and a blank line is the break
/// that survives being read.
fn prose(line: &str) -> &str {
    let body = line.strip_prefix(DOC_MARKER).unwrap_or(line);
    body.strip_prefix(' ').unwrap_or(body).trim_end()
}
