//! What the declaration under the cursor documents, and the call it is inside.
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
//! **Some string literals are answered as what they name.** An `autoload`
//! prefix's literal answers the namespace it maps and the directories a name
//! under it is looked for in ([`autoload_prefix`]). The operand of `as
//! class<T>`, and an argument at a class-name parameter, answer the class
//! their text names, the way that class's name does ([`written_class_name`]). An
//! argument at a path parameter answers the absolute path it names and
//! whether anything is there ([`path_argument`]). Each is asked before the
//! walk over the nodes, because a literal records no name and the walk would
//! step out to what holds it.
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
//!
//! **`textDocument/signatureHelp` is the same reading asked of the call rather
//! than of the name.** [`help_at`] walks outward to the call the cursor is
//! inside and renders its row through [`signature`] — the line hover already
//! shows above a `Core` member's card — so a registry row and a user class's
//! declared parameter list come out in one shape and a parameter list has no
//! second spelling. What it adds is which parameter the cursor is on, counted
//! off the argument spans the parser produced rather than off commas looked
//! for here.
//!
//! **A string argument equal to a completion file's value shows that value.**
//! [`file_value`] reads the values completion offers at the same argument,
//! from the attachments that apply at the call
//! (`rule:ide/completion-files-offer-values-at-named-parameters`), and the one
//! whose `value` is the literal's text answers with its `title` and its
//! documentation, a relative link resolved against the completion file's
//! folder. A value that gives neither answers nothing here.

use std::borrow::Cow;
use std::fmt::Write as _;

use lsp_types::{
    Documentation, Hover, HoverContents, MarkupContent, MarkupKind, ParameterInformation,
    ParameterLabel, SignatureHelp, SignatureInformation,
};
use nvs_diagnostics::{BytePos, PositionEncoding, Span};
use nvs_hir::autoload::SiteKind;
use nvs_hir::{QName, SymbolKind};
use nvs_stdlib::registry::{self, CoreMethod, MethodDoc};
use nvs_syntax::ast::DocComment;
use nvs_syntax::{DOC_MARKER, IndexNode};
use nvs_types::{ExprInfo, ResolvedCall, TypeInterner};

use crate::card::{core_member_hover, core_type_hover, namespace_card};
use crate::completion_files::CompletionFiles;
use crate::definition::{
    Target, attribute_at, payload_path, site, target_of, written_class_name_at,
};
use crate::document::Analysed;
use crate::position::range_at;

/// What the name at `offset` in the entry document documents.
///
/// `None` when the cursor is inside no node, and when nothing it is inside was
/// documented or typed: no `///` run above the declaration its name reached, no
/// registry row behind it, and no type recorded for the node itself.
#[must_use]
pub fn at(
    analysed: &Analysed,
    files: &CompletionFiles,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<Hover> {
    let nodes: Vec<Span> = analysed
        .index
        .at(offset)
        .nodes()
        .iter()
        .map(|node| node.span)
        .collect();
    let (value, node) = autoload_prefix(analysed, offset)
        .or_else(|| written_class_name(analysed, offset))
        .or_else(|| path_argument(analysed, offset))
        .or_else(|| file_value(analysed, files, offset))
        .or_else(|| answer_in(analysed, &nodes, offset))
        .or_else(|| answer_in(analysed, &payload_path(analysed, offset), offset))
        .or_else(|| attribute(analysed, offset))?;
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

/// The row of the call at `offset`, and which of its parameters the cursor is
/// on.
///
/// **The enclosing call, not the innermost name.** [`at`] stops at the first
/// node that names anything, because the nearest answer is the one a reader is
/// pointing at; this walk keeps going outward past every target that is not a
/// call, so a cursor on `$u->name` inside `take($u->name)` is still on `take`'s
/// first parameter.
///
/// One signature and never a list. A call site either resolved to exactly one
/// declaration or carries no `nvs_types::ResolvedCall` at all, so there is no
/// overload set to choose between and `active_signature` is always the one
/// entry — which is `rule:statements/nothing-gets-a-second-name`'s consequence
/// here rather than a simplification.
#[must_use]
pub fn help_at(analysed: &Analysed, offset: BytePos) -> Option<SignatureHelp> {
    let (call, node) =
        analysed.index.at(offset).nodes().iter().find_map(|node| {
            match target_of(analysed.exprs.lookup(node.span)?)? {
                Target::Method(call) => Some((call, *node)),
                _ => None,
            }
        })?;
    let (label, params) = signature(&call.class.to_string(), call, &analysed.interner);
    let active = active_parameter(analysed, node, offset, call);
    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            documentation: documentation(analysed, call),
            parameters: Some(
                params
                    .into_iter()
                    .map(|at| ParameterInformation {
                        label: ParameterLabel::LabelOffsets(at),
                        // The card's own parameter sentences are in the
                        // signature's documentation below, written as one
                        // block; splitting them per entry would be a second
                        // reading of `MethodDoc::params` that a declared
                        // method has no equivalent of.
                        documentation: None,
                    })
                    .collect(),
            ),
            active_parameter: Some(active),
        }],
        active_signature: Some(0),
        active_parameter: Some(active),
    })
}

/// Which parameter of `call` the cursor at `offset` is on.
///
/// Counted from the arguments the call has already closed: an argument that
/// ends before the cursor is one the reader has finished writing, so how many
/// of them there are is the position being written now. A cursor at an
/// argument's last byte is still on that argument — `take($a)` with the caret
/// after `$a` is the first parameter and not the second — which is why the
/// comparison is strict.
///
/// **The arguments are spans the parser produced, not commas found here.** They
/// are the call node's own children after
/// [`crate::arguments::opens_at`]'s parenthesis, so a
/// comma inside a string literal or inside a nested call is inside one of those
/// spans and separates nothing. A variadic tail keeps the last parameter
/// highlighted however many arguments follow it, because that parameter is what
/// every one of them binds to.
fn active_parameter(
    analysed: &Analysed,
    node: IndexNode,
    offset: BytePos,
    call: &ResolvedCall,
) -> u32 {
    let children: Vec<Span> = analysed
        .index
        .children_of(node)
        .into_iter()
        .map(|child| child.span)
        .collect();
    let Some(open) = crate::arguments::opens_at(analysed, node.span, &children) else {
        return 0;
    };
    let closed = children
        .iter()
        .filter(|child| child.start >= open && child.end < offset)
        .count();
    let closed = if call.variadic {
        closed.min(call.param_tys.len().saturating_sub(1))
    } else {
        closed
    };
    u32::try_from(closed).unwrap_or(0)
}

/// What the call's own declaration documents, as the popup beside its row.
///
/// The two readings [`at`] already has, tried in its order: a `Core` member's
/// reference card, and otherwise the `///` run above the declaration the call
/// resolved to. A member that documents neither gets no popup rather than an
/// empty one, which is [`at`]'s answer to the same case.
fn documentation(analysed: &Analysed, call: &ResolvedCall) -> Option<Documentation> {
    let value = registry_row(call)
        .and_then(|member| {
            nvs_footprint::card(&call.class.to_string());
            member.doc.map(|doc| reference_card(member, doc))
        })
        .or_else(|| run(analysed, &Target::Method(call)))?;
    let value = value.trim().to_owned();
    if value.is_empty() {
        return None;
    }
    Some(Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    }))
}

/// The registry row `call` resolved to, and `None` for a call to anything a
/// program declared.
///
/// The registry is asked rather than the symbol table, and a name it does not
/// carry is not a `Core` member: `nvs_hir` resolves any `Core\…` without a
/// declaration, so a member of a not-yet-implemented class is a name the
/// checker refused rather than a row this can render.
fn registry_row(call: &ResolvedCall) -> Option<&'static CoreMethod> {
    let class = registry::class(&call.class.to_string())?;
    class.members().find(|row| row.name == call.method)
}

/// The length of `text` in UTF-16 code units, which is what a parameter's label
/// offsets are counted in.
///
/// Not the encoding `rule:ide/positions-have-one-home` negotiated: that one
/// settles positions in a *document*, and an offset into a signature label is
/// an offset into a string this server has just built. LSP fixes those at
/// UTF-16 and there is nothing about them to negotiate.
fn utf16_len(text: &str) -> u32 {
    u32::try_from(text.chars().map(char::len_utf16).sum::<usize>()).unwrap_or(u32::MAX)
}

/// The `///` run above the **enum** an undocumented case belongs to, as
/// Markdown, and `None` for every other target.
///
/// A case is where `rule:enums/no-class-machinery` puts a name and the enum is
/// where a program documents what the names mean — "`Draft` is written but not
/// visible" is a sentence about a case that has nowhere else to live, since a
/// case is one word and a value. So a case carrying its own run answers with
/// it, and one carrying none answers with its enum's rather than with nothing.
/// A class constant takes no such fallback: its declaration is a place a `///`
/// can be written, and the class's card is about the class.
fn enclosing_run(analysed: &Analysed, target: &Target<'_>) -> Option<String> {
    let Target::Constant { class, .. } = target else {
        return None;
    };
    let symbol = analysed.module.symbols.get(class)?;
    if symbol.kind != SymbolKind::Enum {
        return None;
    }
    run(analysed, &Target::Type(Cow::Borrowed(class)))
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

/// The nearest of `nodes` — innermost first — that documents or types
/// anything, and the span to underline.
///
/// The one walk [`at`] makes, over whichever spans hold the cursor: the
/// index's own path, or the attribute payload path the index does not hold
/// (`crate::definition::payload_path`).
fn answer_in(analysed: &Analysed, nodes: &[Span], offset: BytePos) -> Option<(String, Span)> {
    let text = analysed.map.file(analysed.entry).text();
    // The cursor is on the receiver's own name when the innermost span is a
    // bare name and not the access resolved around it.
    let innermost = nodes.first().copied();
    nodes.iter().find_map(|node| {
        let info = analysed.exprs.lookup(*node)?;
        let receiver = innermost.is_some_and(|inner| {
            inner != *node
                && text
                    .get(inner.range())
                    .is_some_and(|written| written.bytes().all(is_name_byte))
        });
        let value = target_of(info)
            .and_then(|target| {
                written(analysed, &target, *node, offset, receiver)
                    .or_else(|| core(analysed, &target))
                    .or_else(|| run(analysed, &target))
                    .or_else(|| enclosing_run(analysed, &target))
            })
            .or_else(|| declared(analysed, info))?;
        Some((value, *node))
    })
}

/// Whether `byte` is one a qualified name is written with.
fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'\\'
}

/// What the class a written class name names documents, on the terms of
/// a cursor on the class's name, and the literal's span.
///
/// The class is [`crate::definition::written_class_name_at`]'s, and the answer is
/// the one hovering the name gives: a `///` run, and nothing for a class with
/// none.
fn written_class_name(analysed: &Analysed, offset: BytePos) -> Option<(String, Span)> {
    let (target, span) = written_class_name_at(analysed, offset)?;
    Some((run(analysed, &target)?, span))
}

/// The absolute path an argument at a path parameter names, and whether a
/// file or a directory is there, for a cursor inside the literal, and the
/// literal's span.
///
/// The path is [`crate::arguments::target`]'s: the checker's join for a
/// written relative path, and the text itself for an absolute one. A relative
/// literal the checker did not resolve answers nothing here, and the walk
/// over the nodes answers instead.
fn path_argument(analysed: &Analysed, offset: BytePos) -> Option<(String, Span)> {
    let argument = crate::arguments::at(analysed, offset)
        .filter(|argument| argument.text == registry::ParamText::Path)?;
    let target = crate::arguments::target(analysed, &argument)?;
    let there = match std::fs::metadata(&target) {
        Ok(metadata) if metadata.is_dir() => "This directory exists.",
        Ok(_) => "This file exists.",
        Err(_) => "Nothing exists at this path.",
    };
    Some((
        format!("```text\n{}\n```\n\n{there}", target.display()),
        argument.span,
    ))
}

/// The `title` and documentation of the completion file's value whose `value`
/// is the text of the string argument the cursor at `offset` is inside, and the
/// literal's span. The values are the ones completion offers at that argument.
fn file_value(
    analysed: &Analysed,
    files: &CompletionFiles,
    offset: BytePos,
) -> Option<(String, Span)> {
    let named = crate::arguments::named_at(analysed, offset)?;
    let text =
        nvs_syntax::string_lit::cook_string_literal(analysed.map.file(analysed.entry), named.span);
    let value = files
        .values_at(&named.class, &named.method, &named.parameter, &named.others)
        .into_iter()
        .find(|value| value.value == text)?;
    let parts: Vec<String> = [value.title.clone(), value.markdown()]
        .into_iter()
        .flatten()
        .collect();
    (!parts.is_empty()).then(|| (parts.join("\n\n"), named.span))
}

/// The namespace an `autoload` prefix maps and the directories a name under it
/// is looked for in, for a cursor inside the prefix's literal, and that
/// literal's span.
///
/// Both are read off the declaration's `nvs_hir::autoload::Site`, which
/// resolves them with the functions the autoload map is built with
/// (`rule:programs/autoload`). So a `{..}` segment shows the directory name it
/// is replaced by, and a root that does not exist is listed and says so. A
/// cursor on the `{..}` segment itself is told what that segment is first.
fn autoload_prefix(analysed: &Analysed, offset: BytePos) -> Option<(String, Span)> {
    let (site, literal, written) = analysed
        .autoloads
        .iter()
        .find_map(|site| match &site.kind {
            SiteKind::Prefix {
                prefix, literal, ..
            } if literal.start < offset && offset < literal.end => Some((site, *literal, prefix)),
            _ => None,
        })?;
    let namespace = site.namespace()?;
    let mut value = String::new();
    // The written prefix and the namespace have one segment each, in order, so
    // the segment the cursor is on is found by counting separators.
    let before = analysed
        .map
        .file(analysed.entry)
        .text()
        .get(literal.start as usize + 1..offset as usize)?;
    let segment = before.matches('\\').count();
    if let (Some(braced), Some(name)) = (written.split('\\').nth(segment), namespace.get(segment))
        && braced.starts_with('{')
    {
        let _ = writeln!(
            value,
            "`{braced}` is `{name}`, the name of the directory it reaches.\n"
        );
    }
    let _ = writeln!(value, "```nvs\nnamespace {}\n```\n", namespace.join("\\"));
    value.push_str("A name in this namespace is looked for in these directories, in this order:\n");
    for root in site.roots() {
        let _ = write!(value, "\n- `{}`", root.shown);
        if !root.exists {
            value.push_str(" (does not exist)");
        }
    }
    Some((value, literal))
}

/// The card of the attribute whose name the cursor is on, and that name's
/// span.
///
/// A name is no node of the index, so this is asked after the walk over the
/// nodes found nothing, off the checker's own record of what the name resolved
/// to ([`attribute_at`]). A compiler attribute answers its card; a userland
/// one answers the `///` run above the alias it names, where one is written.
fn attribute(analysed: &Analysed, offset: BytePos) -> Option<(String, Span)> {
    let (target, span) = attribute_at(analysed, offset)?;
    let value = written(analysed, &target, span, offset, false)
        .or_else(|| core(analysed, &target))
        .or_else(|| run(analysed, &target))?;
    Some((value, span))
}

/// The answer for a cursor on the **written** name inside `node` rather than
/// on what the node resolved to: the namespace's card for a segment in front
/// of the last — `Core\Http` in `Core\Http\Method::Get` — and, with `receiver`
/// set, the class's own card for the class half of a member access, which is
/// what a cursor on `Str` in `Core\Str::length` is pointing at. `None` leaves
/// the node to the arms that read what it resolved to.
///
/// The written segments are aligned to the tail of the resolved name, because
/// a program writes the short spelling under a `use` and the checker recorded
/// the whole one: `Http\Method` under `use Core\Http;` is the last two
/// segments of `Core\Http\Method`. A spelling that is not a tail — an aliased
/// import — answers nothing here rather than a namespace it never named.
fn written(
    analysed: &Analysed,
    target: &Target<'_>,
    node: Span,
    offset: BytePos,
    receiver: bool,
) -> Option<String> {
    let class = class_of(target);
    let text = analysed.map.file(analysed.entry).text();
    let (name, start) = name_around(text, node, offset)?;
    let written = name.split('\\').count();
    let resolved = class.segments();
    if written > resolved.len() {
        return None;
    }
    let skip = resolved.len() - written;
    let segment = name
        .get(..usize::try_from(offset - start).ok()?)
        .map_or(0, |before| before.matches('\\').count());
    if segment + 1 < written {
        return namespace_card(&resolved[..=skip + segment].join("\\"));
    }
    if !receiver {
        return None;
    }
    core_type_hover(&class.to_string())
        .or_else(|| run(analysed, &Target::Type(Cow::Borrowed(class))))
}

/// The class a target is on: the declaring class of a member, or the type
/// itself.
///
/// Borrowed from the target and not from the analysis behind it, because a
/// [`Target::Type`] may own its name (`crate::definition::type_name_at`).
fn class_of<'t>(target: &'t Target<'_>) -> &'t QName {
    match target {
        Target::Type(class) => class.as_ref(),
        Target::Property { class, .. }
        | Target::Constant { class, .. }
        | Target::TypeAlias { class, .. } => class,
        Target::Method(call) => &call.class,
    }
}

/// The run of name characters around `offset` inside `node` — a qualified
/// name, a member name, a keyword — and where it starts. `None` for a cursor
/// on anything else: an operator, a string, whitespace.
fn name_around(text: &str, node: Span, offset: BytePos) -> Option<(&str, BytePos)> {
    let inside = text.get(node.range())?;
    let bytes = inside.as_bytes();
    let at = usize::try_from(offset.checked_sub(node.start)?).ok()?;
    let is_name = |index: usize| bytes.get(index).is_some_and(|byte| is_name_byte(*byte));
    let mut start = at;
    while start > 0 && is_name(start - 1) {
        start -= 1;
    }
    let mut end = at;
    while is_name(end) {
        end += 1;
    }
    if start == end {
        return None;
    }
    let name = inside.get(start..end)?;
    Some((name, node.start + u32::try_from(start).ok()?))
}

/// A `Core` target's card, or `None` for every other target: a member's row
/// and reference card, a class's, enum's or attribute's card
/// ([`core_type_hover`]), a constant's sentence or a case's line
/// ([`core_member_hover`]).
///
/// The row is [`signature`]'s, which is also what signature help shows for a
/// method a program declared: one renderer, so the two answers cannot come to
/// disagree about how a parameter list is spelled.
fn core(analysed: &Analysed, target: &Target<'_>) -> Option<String> {
    let call = match target {
        Target::Method(call) => call,
        Target::Type(class) => return core_type_hover(&class.to_string()),
        Target::Constant { class, name } => return core_member_hover(&class.to_string(), name),
        Target::Property { .. } | Target::TypeAlias { .. } => return None,
    };
    let member = registry_row(call)?;
    nvs_footprint::card(&call.class.to_string());
    let mut value = format!(
        "```nvs\n{}\n```",
        signature(&call.class.to_string(), call, &analysed.interner).0
    );
    if let Some(doc) = member.doc {
        value.push_str(&reference_card(member, doc));
    }
    Some(value)
}

/// One row as a program reads it — `Core\Str::length(string $s): uint` — and
/// where in it each parameter sits.
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
///
/// **The offsets leave with the row rather than being searched for in it.**
/// [`help_at`] hands each one to a client as the span to highlight, and a
/// parameter located by looking its own text up again would land on the wrong
/// one the first time `(string $search, string $s)` was declared.
fn signature(class: &str, call: &ResolvedCall, interner: &TypeInterner) -> (String, Vec<[u32; 2]>) {
    let last = call.param_tys.len().saturating_sub(1);
    let mut row = format!("{class}::{}(", call.method);
    let mut params = Vec::with_capacity(call.param_tys.len());
    for (index, ty) in call.param_tys.iter().enumerate() {
        if index > 0 {
            row.push_str(", ");
        }
        let name = call.param_names.get(index).map_or("", String::as_str);
        let ty = interner.describe(*ty);
        let start = utf16_len(&row);
        // The variadic tail keeps the element type the row wrote, which is
        // what `ResolvedCall::param_tys` carries in that slot.
        if call.variadic && index == last {
            row.push_str(&format!("{ty} ...${name}"));
        } else {
            row.push_str(&format!("{ty} ${name}"));
        }
        params.push([start, utf16_len(&row)]);
    }
    row.push_str(&format!("): {}", interner.describe(call.return_ty)));
    (row, params)
}

/// The card under the signature line: what the member does, its parameters,
/// what it answers with, and what it throws — every section that was written,
/// and nothing for one that was not.
///
/// The `$` sigil is on a positional parameter and not on an option, because
/// `MethodDoc::params` runs the positional entries first and then one per
/// option of a trailing bag, and an option is written as a key rather than as
/// an argument.
pub(crate) fn reference_card(member: &CoreMethod, doc: &MethodDoc) -> String {
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
pub(crate) fn markdown(text: &str, doc: &DocComment) -> String {
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
