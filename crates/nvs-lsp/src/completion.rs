//! What may be written where the cursor is.
//!
//! `textDocument/completion` answers four things in
//! `rule:ide/the-request-set-is-closed`'s list, and this module holds three of
//! them: the members reachable off a receiver whose class the analysis
//! resolved, the static members and constants reached through a class name,
//! and the cases of an enum written after `Type::` — a user-declared class and
//! a `Core` one alike. The fourth, the keywords and the variables in scope a
//! bare position offers, is the same walk asked at a node that is no access at
//! all, and lands beside this one.
//!
//! **`->` and `::` are one walk and two lookups.** Both are an access whose
//! first child is its receiver, so which of the two the cursor is in decides
//! only which half of the class it reaches: a value reaches what an instance
//! holds, a class name reaches the static members, the class constants and an
//! enum's cases, and no member is on both lists. That half is [`Reach`], read
//! off the access's own node kind rather than off the operator's text.
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
//! # Where a class name's meaning comes from
//!
//! A `::` receiver *names* a class rather than holding one, so there is no
//! type to ask for. The checker's own answer comes first, read back off the
//! whole access — `Mode::Read` is a `nvs_types::ExprInfo::EnumCase`, and a
//! resolved static call carries the `nvs_types::ResolvedCall` it was made
//! against — because that resolution has the namespace and the imports of the
//! site already applied, and it is the only one that answers `self::` and
//! `parent::`, which name no class the source can be read for.
//!
//! A member half still being written is exactly the access that resolved to
//! nothing, though, so the written name is resolved instead:
//! `nvs_hir::resolve_ref`, the one qualified/unqualified/imported lookup every
//! resolver in this codebase shares, given the namespace the cursor sits
//! inside and the imports of the file that wrote it. Restating that rule here
//! would be the second implementation [`crate::definition`] refuses to keep;
//! handing it its two inputs is not.
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
//! **Known gaps.** Two of them are the class the cursor reached rather than
//! this walk. An inherited member is not offered: the members are the ones the
//! resolved class *declares*, and walking `nvs_hir::ClassGraph` for the rest is
//! the same widening `rule:ide/five-features-are-one-reference-index`'s index
//! does properly. Visibility is not applied either, so a `private` member is
//! offered outside its class — which is a name the checker then refuses where
//! it was written, rather than a wrong answer that compiles. The third is the
//! class name that is not written down: `self::`, `static::` and `parent::`
//! reach a class only through the recorded answer above, so an access whose
//! member half is still empty offers nothing after them.

use lsp_types::{CompletionItem, CompletionItemKind};
use nvs_diagnostics::{BytePos, SourceFile, Span};
use nvs_hir::QName;
use nvs_stdlib::registry::{self, CoreClass, CoreConst, CoreEnum, CoreMethod};
use nvs_syntax::IndexNode;
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, EnumCase, MethodMember, Modifier, PropertyMember, StmtKind,
};
use nvs_types::{ExprInfo, Ty, TypeId};
use rustc_hash::FxHashMap;

use crate::definition::{declared_type, text_of};
use crate::document::Analysed;

/// Every access shape a member is written inside, as `nvs_syntax::walk` spells
/// them, with the half of the class each one reaches.
const ACCESS: &[(&str, Reach)] = &[
    ("PropertyAccess", Reach::Instance),
    ("MethodCall", Reach::Instance),
    ("StaticCall", Reach::Static),
    ("StaticPropertyAccess", Reach::Static),
    ("ClassConstAccess", Reach::Static),
];

/// Which half of a class an access reaches.
///
/// `Foo::class` is neither: the member it writes is a keyword rather than a
/// name the class declares, so it is the position arm's answer and not this
/// one's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    /// Written after `->`, off a value — what an instance holds.
    Instance,
    /// Written after `::`, off a class name — the static members, the class
    /// constants, and an enum's cases.
    Static,
}

/// What may be written at `offset` in the entry document, sorted by label.
///
/// Empty for a cursor that is in no node, in one this cannot resolve a receiver
/// for, or in the receiver half of an access rather than the member half —
/// which are one answer for a client, since LSP has no shape for "ask me
/// again somewhere else".
#[must_use]
pub fn at(analysed: &Analysed, offset: BytePos) -> Vec<CompletionItem> {
    let Some((class, reach)) = receiver_class(analysed, offset) else {
        return Vec::new();
    };
    let name = class.to_string();
    let mut items = if let Some(core) = registry::class(&name) {
        core_members(core, reach)
    } else if let Some(core) = registry::core_enum(&name) {
        core_cases(core, reach)
    } else {
        declared_members(analysed, &class, reach)
    };
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

/// The class the cursor is writing a member of, and the half of it the access
/// it is in reaches.
fn receiver_class(analysed: &Analysed, offset: BytePos) -> Option<(QName, Reach)> {
    let path = analysed.index.at(offset);
    let (access, reach) = path.nodes().iter().find_map(|node| {
        ACCESS
            .iter()
            .find(|(kind, _)| *kind == node.kind)
            .map(|(_, reach)| (*node, *reach))
    })?;
    let receiver = analysed.index.children_of(access).into_iter().next()?;
    // A cursor still inside the receiver is writing the receiver, and the
    // members of its own class are not what it is asking for.
    if offset < receiver.span.end {
        return None;
    }
    let class = match reach {
        Reach::Instance => held_class(analysed, receiver.span, offset)?,
        Reach::Static => named_class(analysed, access, receiver.span)?,
    };
    Some((class, reach))
}

/// The class the value at `span` holds.
fn held_class(analysed: &Analysed, span: Span, offset: BytePos) -> Option<QName> {
    let ty = receiver_ty(analysed, span, offset)?;
    match analysed.interner.get(ty) {
        Ty::Class(class, _) => Some(class.clone()),
        _ => None,
    }
}

/// The class the `::` access at `access` names, whose name is written at
/// `receiver`.
///
/// The two answers the module doc's *Where a class name's meaning comes from*
/// weighs, in that order: what the checker recorded for the whole access, and
/// failing that the written name put through the resolver's own lookup.
fn named_class(analysed: &Analysed, access: IndexNode, receiver: Span) -> Option<QName> {
    if let Some(class) = analysed.exprs.lookup(access.span).and_then(resolved_class) {
        return Some(class.clone());
    }
    let text = analysed
        .map
        .file(analysed.entry)
        .text()
        .get(receiver.range())?;
    if text.is_empty() {
        return None;
    }
    Some(nvs_hir::resolve_ref(
        text,
        &namespace_at(analysed, receiver.start),
        &imports_of(analysed),
    ))
}

/// The class one recorded `::` access resolved against.
///
/// A call answers the class that *declares* the member, which is
/// `nvs_types::ResolvedCall::class` and what [`crate::definition`] jumps to as
/// well: an inherited member is written on the receiver's class and declared on
/// another, and the declaration is the one this module can read members off.
fn resolved_class(info: &ExprInfo) -> Option<&QName> {
    Some(match info {
        ExprInfo::EnumCase { enum_, .. } => enum_,
        ExprInfo::Call(call) | ExprInfo::ClassRefCall(call) => &call.class,
        ExprInfo::StaticProperty { class, .. } => class,
        _ => return None,
    })
}

/// The namespace `offset` is written inside, as its segments.
///
/// Walked off the entry document's own statements because `nvs-hir` applies a
/// namespace as it collects declarations and keeps no map from a position back
/// to one. The two forms are the ones its resolver reads: `namespace Name;`
/// governs the rest of the file, and `namespace Name { … }` governs its block.
fn namespace_at(analysed: &Analysed, offset: BytePos) -> Vec<String> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let file = analysed.map.file(analysed.entry);
    let mut current = Vec::new();
    for stmt in &loaded.stmts {
        let StmtKind::NamespaceDecl(decl) = &stmt.kind else {
            continue;
        };
        let segments = decl.name.as_ref().map_or_else(Vec::new, |name| {
            QName::parse(text_of(file, name.span)).segments().to_vec()
        });
        match &decl.body {
            Some(block) if block.span.start <= offset && offset < block.span.end => {
                return segments;
            }
            None if stmt.span.end <= offset => current = segments,
            _ => {}
        }
    }
    current
}

/// The entry document's own imports, in the shape `nvs_hir::resolve_ref` reads.
///
/// The whole graph's imports travel in one list, and a `use` belongs to the
/// file that wrote it — a required file's import must not resolve a name
/// written here.
fn imports_of(analysed: &Analysed) -> FxHashMap<String, QName> {
    analysed
        .module
        .imports
        .iter()
        .filter(|import| import.span.file == analysed.entry)
        .map(|import| (import.short_name.clone(), import.target.clone()))
        .collect()
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

/// Every member a user-declared type writes that `reach` reaches, as it wrote
/// them.
fn declared_members(analysed: &Analysed, class: &QName, reach: Reach) -> Vec<CompletionItem> {
    let Some((stmt, file)) = declared_type(analysed, class) else {
        return Vec::new();
    };
    let members: &[ClassMember] = match &stmt.kind {
        StmtKind::ClassDecl(decl) => &decl.members,
        StmtKind::InterfaceDecl(decl) => &decl.members,
        // An enum declares its cases and nothing else
        // (`rule:enums/no-class-machinery`), and a case is reached off the
        // enum's own name — so one of its values reaches nothing at all.
        StmtKind::EnumDecl(decl) => {
            return match reach {
                Reach::Static => decl
                    .cases
                    .iter()
                    .map(|case| enum_case(file, case))
                    .collect(),
                Reach::Instance => Vec::new(),
            };
        }
        // A type alias has a body of no members.
        _ => return Vec::new(),
    };
    members
        .iter()
        .filter_map(|member| declared_member(file, member, reach))
        .collect()
}

/// One declared member, or `None` for one this access does not reach.
///
/// `static` is what the two halves are told apart by, on a method and a
/// property alike, so no member is offered twice — `rule:core-api/shape-rules`
/// R20's "no operation is reachable two ways" is the shape a user class follows
/// too. A class constant is written after `::` and nowhere else.
fn declared_member(
    file: &SourceFile,
    member: &ClassMember,
    reach: Reach,
) -> Option<CompletionItem> {
    let statics = reach == Reach::Static;
    match &member.kind {
        ClassMemberKind::Method(method)
            if method.modifiers.contains(&Modifier::Static) == statics =>
        {
            Some(item(
                text_of(file, method.name).to_owned(),
                CompletionItemKind::METHOD,
                signature(file, method),
            ))
        }
        ClassMemberKind::Property(property)
            if property.modifiers.contains(&Modifier::Static) == statics =>
        {
            Some(item(
                property_label(file, property, reach),
                CompletionItemKind::PROPERTY,
                text_of(file, property.ty.span).to_owned(),
            ))
        }
        ClassMemberKind::Const(constant) if statics => Some(item(
            text_of(file, constant.name).to_owned(),
            CompletionItemKind::CONSTANT,
            constant.ty.as_ref().map_or_else(
                || text_of(file, constant.value.span).to_owned(),
                |ty| text_of(file, ty.span).to_owned(),
            ),
        )),
        _ => None,
    }
}

/// A property's label, which carries its sigil after `::` and not after `->`.
///
/// The label is what the client replaces the word being completed with, and the
/// two operators are followed by different text: `$user->name` writes no sigil
/// and `User::$count` writes one.
fn property_label(file: &SourceFile, property: &PropertyMember, reach: Reach) -> String {
    let written = text_of(file, property.name);
    match reach {
        Reach::Static => written.to_owned(),
        Reach::Instance => written.trim_start_matches('$').to_owned(),
    }
}

/// One enum case, with the value its declaration wrote.
///
/// A case written without one takes the previous case's plus one
/// (`rule:enums/declaration`), and that arithmetic is `nvs_types::enums`' — this
/// module spells declarations rather than computing them, so an unwritten value
/// contributes no detail rather than a re-derived one.
fn enum_case(file: &SourceFile, case: &EnumCase) -> CompletionItem {
    item(
        text_of(file, case.name.span).to_owned(),
        CompletionItemKind::ENUM_MEMBER,
        case.value
            .as_ref()
            .map(|value| text_of(file, value.span).to_owned())
            .unwrap_or_default(),
    )
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

/// Every member of a `Core` class that `reach` reaches, spelled from its
/// registry rows.
fn core_members(core: &CoreClass, reach: Reach) -> Vec<CompletionItem> {
    match reach {
        Reach::Instance => core.instance.iter().map(core_member).collect(),
        Reach::Static => core
            .methods
            .iter()
            .map(core_member)
            .chain(core.constants.iter().map(core_constant))
            .collect(),
    }
}

/// One `Core` class constant, spelled from its registry row.
///
/// Its declared type and not its value: `nvs_stdlib::registry::CoreConst::value`
/// is an object for the rows that carry one, allocated at the use site, so a
/// value column would print an implementation detail beside a literal.
fn core_constant(constant: &CoreConst) -> CompletionItem {
    item(
        constant.name.to_owned(),
        CompletionItemKind::CONSTANT,
        constant.ty.spelled(),
    )
}

/// Every case a `Core`-owned enum declares, with the constant value the roster
/// states.
///
/// A second roster beside the classes, exactly as `nvs_stdlib::registry::ENUMS`
/// is one beside `CLASSES`: an enum is not a class, and its cases are reached
/// off its name and nowhere else.
fn core_cases(core: &CoreEnum, reach: Reach) -> Vec<CompletionItem> {
    match reach {
        Reach::Instance => Vec::new(),
        Reach::Static => core
            .cases
            .iter()
            .map(|(name, value)| {
                item(
                    (*name).to_owned(),
                    CompletionItemKind::ENUM_MEMBER,
                    value.to_string(),
                )
            })
            .collect(),
    }
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
