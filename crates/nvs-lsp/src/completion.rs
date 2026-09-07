//! What may be written where the cursor is.
//!
//! `textDocument/completion` answers four things in
//! `rule:ide/the-request-set-is-closed`'s list, and this module holds the first
//! of them: the members reachable off a receiver whose class the analysis
//! resolved, a user-declared class and a `Core` one alike. The other three —
//! enum cases after `Type::`, a class's static members, and the keywords and
//! in-scope variables a bare position offers — are the same walk asked at a
//! different node, and land beside this one.
//!
//! **The document usually does not parse, and the receiver is still found.**
//! `$u->` followed by the next line's `if (true) {` parses as one method call
//! whose member name is `if` and whose argument is `true`: the grammar has no
//! way to know the developer stopped typing, and it is not allowed to guess
//! (`rule:ide/one-grammar-one-tree` — there is one grammar, and the resilient
//! parse is the same one). So nothing here reads the member name. What the
//! cursor's position in the access says is only *which half of it* the cursor
//! is in, and a cursor at or past the receiver's end is in the member half.
//!
//! **The receiver is reached downward, through the index.**
//! `nvs_syntax::SyntaxIndex::at` answers a cursor with the node it is in and
//! that node's ancestors, and the member half of `$u->` is inside no node of
//! its own — so the receiver is the access's own first child
//! (`nvs_syntax::SyntaxIndex::children_of`). Scanning the source backwards over
//! an arrow would be this crate reading a grammar `nvs-syntax` owns.
//!
//! # Where a plain variable's type comes from
//!
//! `$u` is a variable *read*, and until this request existed the checker kept
//! nothing about one: a local's type lives in
//! `nvs_types::locals::LocalScope`, which is built per body and dropped with
//! it, and only a **narrowed** read records an entry of its own. Two ways to
//! close that were open, and this crate takes the second:
//!
//! * **An entry per variable read**, recorded beside the narrowed ones. It is
//!   the same fact keyed the other way, and it is the wrong way: a read is the
//!   most common expression a program writes, so it costs a table row at every
//!   *occurrence* of a name, on every compile, to answer a question only the
//!   server asks.
//! * **The body's own scope, kept rather than dropped** —
//!   `nvs_types::ExprTypeTable::local_scopes`, one entry per *declaration* and
//!   moved out of a frame that was about to be freed anyway. What a compile
//!   that never reads it back pays is holding that map to the end of the check
//!   run instead of to the end of the body, and `AGENTS.md`'s priority
//!   ordering spends memory to buy latency rather than the reverse.
//!
//! It is also the one of the two that answers the *other* three arms: the
//! variables in scope at a bare cursor are a scope, not a set of reads.
//!
//! # What a member's detail column says
//!
//! The declaration is the home, and there are two kinds of declaration. A user
//! class's members are spelled from **the source they were written in** — the
//! same tree [`crate::definition`] reaches for a `///` run — because that text
//! is what the developer wrote and re-deriving it through the interner would
//! turn `?User` into whatever the checker's own normalization spells it as. A
//! `Core` class's are spelled from its registry row by
//! `nvs_stdlib::registry::CoreTy::spelled`, which is the same one home
//! `nvs meta --json` prints. Neither writes a default, on
//! [`crate::hover`]'s reasoning: what a caller may leave out is documentation
//! rather than signature.
//!
//! **A `Core` receiver offers methods and nothing else.** A `Core` instance has
//! no property a program can reach — `nvs_types::core_lib`'s seeding declares
//! none and `nvs_stdlib::registry::CoreTy::Instance` owns why — so `$m->groups`
//! is an unknown member and `$m->groups()` is the member.
//!
//! **Known gaps**, both of which are the class the cursor reached rather than
//! this walk. An inherited member is not offered: the members are the ones the
//! resolved class *declares*, and walking `nvs_hir::ClassGraph` for the rest is
//! the same widening `rule:ide/five-features-are-one-reference-index`'s index
//! does properly. Visibility is not applied either, so a `private` member is
//! offered outside its class — which is a name the checker then refuses where
//! it was written, rather than a wrong answer that compiles.

use lsp_types::{CompletionItem, CompletionItemKind};
use nvs_diagnostics::{BytePos, SourceFile, Span};
use nvs_hir::QName;
use nvs_stdlib::registry::{self, CoreMethod};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, MethodMember, Modifier, StmtKind};
use nvs_types::{ExprInfo, Ty, TypeId};

use crate::definition::{declared_type, text_of};
use crate::document::Analysed;

/// The two access shapes a member is written after, as `nvs_syntax::walk`
/// spells them.
///
/// `::` is not one of them: a static member and an enum case are reached off a
/// *class name* rather than off a value, which is a different lookup and the
/// next slice's.
const MEMBER_ACCESS: &[&str] = &["PropertyAccess", "MethodCall"];

/// What may be written at `offset` in the entry document, sorted by label.
///
/// Empty for a cursor that is in no node, in one this cannot resolve a receiver
/// for, or in the receiver half of an access rather than the member half —
/// which are one answer for a client, since LSP has no shape for "ask me
/// again somewhere else".
#[must_use]
pub fn at(analysed: &Analysed, offset: BytePos) -> Vec<CompletionItem> {
    let Some(class) = receiver_class(analysed, offset) else {
        return Vec::new();
    };
    let mut items = match registry::class(&class.to_string()) {
        Some(core) => core.instance.iter().map(core_member).collect(),
        None => declared_members(analysed, &class),
    };
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

/// The class of the receiver the cursor is writing a member of.
fn receiver_class(analysed: &Analysed, offset: BytePos) -> Option<QName> {
    let path = analysed.index.at(offset);
    let access = path
        .nodes()
        .iter()
        .find(|node| MEMBER_ACCESS.contains(&node.kind))?;
    let receiver = analysed.index.children_of(*access).into_iter().next()?;
    // A cursor still inside the receiver is writing the receiver, and the
    // members of its own class are not what it is asking for.
    if offset < receiver.span.end {
        return None;
    }
    let ty = receiver_ty(analysed, receiver.span, offset)?;
    match analysed.interner.get(ty) {
        Ty::Class(class, _) => Some(class.clone()),
        _ => None,
    }
}

/// The type the receiver at `span` holds.
///
/// The recorded entry first, because a receiver that is itself a call, a `new`
/// or a property access carries its own type and is the same answer wherever it
/// was written. A plain variable read carries none, and that is what the local
/// scopes below are for.
fn receiver_ty(analysed: &Analysed, span: Span, offset: BytePos) -> Option<TypeId> {
    if let Some(ty) = analysed.exprs.lookup(span).and_then(recorded_ty) {
        return Some(ty);
    }
    local_ty(analysed, span, offset)
}

/// The type one recorded expression answers with.
///
/// [`crate::hover`] reads the same entries for the type it shows, and the two
/// lists differ by the call: hovering a call shows what the *member* is, while
/// standing on its result and writing `->` asks what it returned.
fn recorded_ty(info: &ExprInfo) -> Option<TypeId> {
    Some(match info {
        ExprInfo::New { ty, .. } => *ty,
        ExprInfo::Property { ty, .. }
        | ExprInfo::StaticProperty { ty, .. }
        | ExprInfo::HookedProperty { ty, .. } => *ty,
        ExprInfo::Index { elem_ty, .. } => *elem_ty,
        ExprInfo::NarrowedRead { to } => *to,
        ExprInfo::Call(call) | ExprInfo::ClassRefCall(call) => call.return_ty,
        _ => return None,
    })
}

/// The type the binding named at `span` was declared with.
///
/// Innermost body first, and the first one that declares the name wins: a
/// closure's body is inside the body that wrote it and shares none of its
/// bindings, so the enclosing body's entry is the right answer for a name the
/// closure captured and the wrong one for a name it declared itself.
fn local_ty(analysed: &Analysed, span: Span, offset: BytePos) -> Option<TypeId> {
    let text = analysed.map.file(analysed.entry).text();
    let name = text.get(span.range())?.strip_prefix('$')?;
    let mut bodies: Vec<(Span, &[nvs_types::LocalBinding])> = analysed
        .exprs
        .local_scopes()
        .filter(|(body, _)| body.file == span.file && body.start <= offset && offset < body.end)
        .collect();
    bodies.sort_by_key(|(body, _)| body.end - body.start);
    bodies
        .into_iter()
        .find_map(|(_, locals)| locals.iter().find(|local| local.name == name))
        .map(|local| local.ty)
}

/// Every instance member a user-declared type writes, as it wrote them.
fn declared_members(analysed: &Analysed, class: &QName) -> Vec<CompletionItem> {
    let Some((stmt, file)) = declared_type(analysed, class) else {
        return Vec::new();
    };
    let members: &[ClassMember] = match &stmt.kind {
        StmtKind::ClassDecl(decl) => &decl.members,
        StmtKind::InterfaceDecl(decl) => &decl.members,
        StmtKind::EnumDecl(decl) => &decl.members,
        // A type alias has a body of no members, and an enum's cases are
        // reached off the enum rather than off one of its values.
        _ => return Vec::new(),
    };
    members
        .iter()
        .filter_map(|member| declared_member(file, member))
        .collect()
}

/// One declared member, or `None` for one no receiver reaches.
///
/// A `static` member and a class constant are written after `::`, so neither is
/// offered here — the receiver is a value, and `rule:core-api/shape-rules` R20's
/// "no operation is reachable two ways" is the shape a user class follows too.
fn declared_member(file: &SourceFile, member: &ClassMember) -> Option<CompletionItem> {
    match &member.kind {
        ClassMemberKind::Method(method) if !method.modifiers.contains(&Modifier::Static) => {
            Some(item(
                text_of(file, method.name).to_owned(),
                CompletionItemKind::METHOD,
                signature(file, method),
            ))
        }
        ClassMemberKind::Property(property) if !property.modifiers.contains(&Modifier::Static) => {
            Some(item(
                text_of(file, property.name)
                    .trim_start_matches('$')
                    .to_owned(),
                CompletionItemKind::PROPERTY,
                text_of(file, property.ty.span).to_owned(),
            ))
        }
        _ => None,
    }
}

/// One method's parameters and return type, as its declaration writes them —
/// `(string $name): string`.
///
/// The name is not repeated: it is the label the detail column sits beside.
/// A parameter whose type was left unwritten contributes its name alone, which
/// is a declaration the checker has already reported on.
fn signature(file: &SourceFile, method: &MethodMember) -> String {
    let params: Vec<String> = method
        .params
        .iter()
        .map(|param| {
            let name = text_of(file, param.name);
            let dots = if param.variadic { "..." } else { "" };
            match &param.ty {
                Some(ty) => format!("{} {dots}{name}", text_of(file, ty.span)),
                None => format!("{dots}{name}"),
            }
        })
        .collect();
    match &method.return_type {
        Some(ty) => format!("({}): {}", params.join(", "), text_of(file, ty.span)),
        None => format!("({})", params.join(", ")),
    }
}

/// One `Core` instance member, spelled from its registry row.
fn core_member(method: &CoreMethod) -> CompletionItem {
    let mut params: Vec<String> = method
        .positional()
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let name = method.names.get(index).copied().unwrap_or_default();
            match ty {
                registry::CoreTy::Variadic(elem) => format!("{} ...${name}", elem.spelled()),
                _ => format!("{} ${name}", ty.spelled()),
            }
        })
        .collect();
    // The trailing bag is written as the shape a caller passes rather than as
    // an argument, which is `rule:core-api/reference-card`'s own distinction
    // between a parameter and an option.
    if let Some(options) = method.options() {
        params.push(registry::CoreTy::Options(options).spelled());
    }
    item(
        method.name.to_owned(),
        CompletionItemKind::METHOD,
        format!("({}): {}", params.join(", "), method.return_ty.spelled()),
    )
}

/// One offered name, with the two fields `crate::render` freezes beside it.
///
/// Nothing else is set. `insert_text` would be the label again, and a
/// `text_edit` is a range this server has no reason to narrow: what the client
/// replaces is the word it is already completing.
fn item(label: String, kind: CompletionItemKind, detail: String) -> CompletionItem {
    CompletionItem {
        label,
        kind: Some(kind),
        detail: Some(detail),
        ..CompletionItem::default()
    }
}
