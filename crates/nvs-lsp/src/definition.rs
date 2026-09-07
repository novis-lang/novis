//! Where the name under the cursor was declared.
//!
//! `textDocument/definition` is the first request in this crate that needs two
//! things at once: [`nvs_syntax::SyntaxIndex`] to say which node the cursor is
//! in, and the type phase's own table to say what the name in that node
//! resolved to (`rule:ide/the-index-answers-the-cursor`). The index alone
//! cannot answer it — a type name is a *property* of the node that writes it
//! and never a node of its own, which is `nvs_syntax::walk`'s decision and the
//! reason nothing here looks for a node called `TypeName`.
//!
//! **The resolution is the checker's, read back rather than redone.**
//! `nvs_types::ExprInfo` already carries the fully-qualified name each `new`,
//! `instanceof` and enum-case access resolved to, with the namespace and the
//! imports of the site that wrote it applied
//! (`rule:classes/names-resolve-case-sensitively`), and
//! [`crate::document::Analysed`] now keeps that table. A server that resolved a
//! written name for itself would be a second implementation of the rule the
//! checker already applies, and it would disagree the first time an import was
//! involved.
//!
//! **A member is found inside its class's own declaration.**
//! `nvs_hir::MemberTable` records which names a class declares and never where
//! they were written, so the span an editor puts a caret on comes from the tree
//! instead: the declaring class is `nvs_hir::SymbolTable`'s, and the member is
//! the one the parser hung under that declaration. [`site`] is that walk, and
//! it is where [`crate::hover`] reads a `///` run off as well — the two
//! requests ask the same question about *where* and differ only in which field
//! of the answer they use.
//!
//! **The access is the node, so the receiver is inside it.** A member name is a
//! property of the expression that writes it, exactly as a type name is, so a
//! cursor anywhere in `$u->name` — on the `$u` included — answers where `name`
//! is declared. Narrowing that needs a node per member name, which is
//! `nvs_syntax::walk`'s decision rather than this module's.
//!
//! **An enum case answers its enum.** `Mode::Read` under the cursor jumps to
//! `enum Mode`, because a case is a member and the symbol table holds
//! declarations. That is a true answer to a narrower question than was asked,
//! which is the same trade the member slice above will close.
//!
//! **The whole graph, not the entry alone.** The cursor is always in the open
//! document ([`crate::selection`]'s reasoning), but what it names may be
//! declared in a required file — so the span answered here carries its own
//! `SourceId` and the caller spells whichever file that is, exactly as
//! [`crate::links`] spells a `require`'s target.

use std::path::PathBuf;

use lsp_types::Range;
use nvs_diagnostics::{BytePos, PositionEncoding, SourceFile, Span};
use nvs_hir::QName;
use nvs_syntax::ast::{ClassMember, ClassMemberKind, DocComment, Stmt, StmtKind};
use nvs_types::ExprInfo;

use crate::document::Analysed;
use crate::position::range_at;

/// One declaration, and the file it was written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The file, as the analysis loaded it: absolute and canonical, on
    /// [`crate::links::RequireLink`]'s terms.
    pub path: PathBuf,
    /// The declared name's own range — not the whole declaration's, because
    /// that is the span `nvs_hir::Symbol` records and it is what an editor
    /// wants to put a caret on.
    pub range: Range,
}

/// Where the name at `offset` in the entry document is declared.
///
/// `None` when the cursor is inside no node, when the node it is in resolved to
/// no name this can follow, when the name is declared nowhere the analysis
/// reached, or when it is declared in a file with no path — a `Core` class is
/// the last of those, and it has no Novis declaration to open.
#[must_use]
pub fn at(analysed: &Analysed, offset: BytePos, encoding: PositionEncoding) -> Option<Declared> {
    let (target, _) = named_at(analysed, offset)?;
    let declared = site(analysed, &target)?;
    let file = analysed.map.file(declared.span.file);
    Some(Declared {
        path: file.path()?.to_path_buf(),
        range: range_at(file, declared.span, encoding),
    })
}

/// What the node under the cursor named, and so what is looked up to answer it.
///
/// A type and a member are two lookups and not one: a type is declared at file
/// scope and a member inside a declaration, so they are found in different
/// places even though both start at the same [`nvs_hir::SymbolTable`] entry. A
/// method and a property are separate arms rather than one name and a flag
/// because `class C { public int $x; public function x(): int … }` is legal —
/// one name, two declarations, and nothing but the kind tells them apart.
pub(crate) enum Target<'a> {
    /// A class, an interface or an enum, by its fully-qualified name.
    Type(&'a QName),
    /// A method, named on the class that **declares** it rather than on the
    /// receiver's — which is what `nvs_types::ResolvedCall::class` already is,
    /// so an inherited method answers where its body was written.
    Method {
        /// The declaring class.
        class: &'a QName,
        /// The method's own name.
        name: &'a str,
    },
    /// A property, on [`Target::Method`]'s terms, with the `$` sigil not
    /// included — `nvs_types::ExprInfo` records it that way and the source
    /// writes it the other, so the comparison strips rather than the table
    /// growing a second spelling.
    Property {
        /// The declaring class.
        class: &'a QName,
        /// The property's own name.
        name: &'a str,
    },
}

/// One declaration, as the two cursor requests read it.
pub(crate) struct Site<'a> {
    /// The declared name's own span — an editor's caret, and never the whole
    /// declaration's.
    pub span: Span,
    /// The `///` run above it, if the parser attached one.
    pub doc: Option<&'a DocComment>,
}

/// Where `target` was declared, and what was written above it.
///
/// A type answers its `nvs_hir::Symbol`'s span whether or not the walk into the
/// tree finds the node again, because that span is what a jump has always been
/// and a declaration shape this walk does not descend into must not silently
/// stop answering. A member has no such fallback: the tree is the only place
/// its span exists.
pub(crate) fn site<'a>(analysed: &'a Analysed, target: &Target<'_>) -> Option<Site<'a>> {
    let (class, member) = match target {
        Target::Type(qname) => (*qname, None),
        Target::Method { class, name } => (*class, Some((*name, true))),
        Target::Property { class, name } => (*class, Some((*name, false))),
    };
    let symbol = analysed.module.symbols.get(class)?;
    let file = analysed.map.file(symbol.decl_span.file);
    let stmt = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == symbol.decl_span.file)
        .and_then(|loaded| declaration(&loaded.stmts, symbol.decl_span));
    let Some((name, is_method)) = member else {
        return Some(Site {
            span: symbol.decl_span,
            doc: stmt.and_then(doc_of),
        });
    };
    let member = declared_member(stmt?, file, name, is_method)?;
    Some(Site {
        span: member_name(member)?,
        doc: member.doc.as_ref(),
    })
}

/// The statement declaring the name written at `name`.
///
/// Matched on the name's own span rather than by spelling it again: that span
/// is what the parser gave the name and what `nvs_hir::Symbol` recorded, so an
/// equality test cannot pick a different declaration than the symbol table
/// chose. A bracketed namespace body is descended into because its declarations
/// are the file's, written one level in; the statement form holds none of its
/// own.
fn declaration(stmts: &[Stmt], name: Span) -> Option<&Stmt> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::ClassDecl(decl) if decl.name.span == name => Some(stmt),
        StmtKind::InterfaceDecl(decl) if decl.name.span == name => Some(stmt),
        StmtKind::EnumDecl(decl) if decl.name.span == name => Some(stmt),
        StmtKind::TypeAliasDecl(decl) if decl.name.span == name => Some(stmt),
        StmtKind::NamespaceDecl(decl) => declaration(&decl.body.as_ref()?.stmts, name),
        _ => None,
    })
}

/// The `///` run above a declaration.
///
/// The four shapes are exactly the four `nvs_hir::SymbolKind` names, which is
/// why nothing checks which kind the symbol was: a span that reached the symbol
/// table came from one of these, and a fifth declaration shape has to be added
/// to both places at once.
fn doc_of(stmt: &Stmt) -> Option<&DocComment> {
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => decl.doc.as_ref(),
        StmtKind::InterfaceDecl(decl) => decl.doc.as_ref(),
        StmtKind::EnumDecl(decl) => decl.doc.as_ref(),
        StmtKind::TypeAliasDecl(decl) => decl.doc.as_ref(),
        _ => None,
    }
}

/// The member of `stmt` written as `name`, of the kind `is_method` selects.
///
/// A type alias has no body and an enum's cases are not what a call or a
/// property access resolves to, so both answer nothing here rather than being
/// searched.
fn declared_member<'a>(
    stmt: &'a Stmt,
    file: &SourceFile,
    name: &str,
    is_method: bool,
) -> Option<&'a ClassMember> {
    let members = match &stmt.kind {
        StmtKind::ClassDecl(decl) => &decl.members,
        StmtKind::InterfaceDecl(decl) => &decl.members,
        StmtKind::EnumDecl(decl) => &decl.members,
        _ => return None,
    };
    members.iter().find(|member| match &member.kind {
        ClassMemberKind::Method(method) if is_method => text_of(file, method.name) == name,
        ClassMemberKind::Property(property) if !is_method => {
            text_of(file, property.name).trim_start_matches('$') == name
        }
        _ => false,
    })
}

/// A member's own name span, or nothing for a shape that has none.
fn member_name(member: &ClassMember) -> Option<Span> {
    match &member.kind {
        ClassMemberKind::Method(method) => Some(method.name),
        ClassMemberKind::Property(property) => Some(property.name),
        _ => None,
    }
}

/// The source text `span` covers, or nothing for a span outside the file.
///
/// A parser's span is always inside the file it parsed, so the fallback is
/// unreachable rather than a policy — and it is a fallback rather than an index
/// because a panic here would take the server down over one malformed name.
fn text_of(file: &SourceFile, span: Span) -> &str {
    file.text().get(span.range()).unwrap_or_default()
}

/// What the cursor is on and the node that wrote it, innermost node first.
///
/// Innermost first because the nodes nest: a cursor on the class name of
/// `new User()` inside `echo (new User())->name;` is inside both, and the
/// nearer answer is the one it is pointing at.
///
/// The node's span comes back with the target because a cursor answer that is
/// not a jump wants to say what it was answered *about* — [`crate::hover`] puts
/// it in `Hover::range` so the editor underlines the expression rather than the
/// word it would otherwise guess at.
///
/// A call and a first-class callable reference are the same answer here: both
/// carry the `nvs_types::ResolvedCall` the checker made, and which of the two
/// the site wrote decides what happens to the closure, not where the method is.
pub(crate) fn named_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    analysed.index.at(offset).nodes().iter().find_map(|node| {
        let target = match analysed.exprs.lookup(node.span)? {
            ExprInfo::New { class, .. } | ExprInfo::InstanceOf { class } => Target::Type(class),
            // The bound, which is the only class this site named — the one
            // allocated is whatever descriptor is in hand, and no compile
            // knows it (`nvs_types::ExprInfo::NewDynamic`).
            ExprInfo::NewDynamic { bound, .. } => Target::Type(bound),
            ExprInfo::EnumCase { enum_, .. } => Target::Type(enum_),
            ExprInfo::Call(call) | ExprInfo::CallableRef(call) | ExprInfo::ClassRefCall(call) => {
                Target::Method {
                    class: &call.class,
                    name: &call.method,
                }
            }
            // A hooked property is a call to an accessor and still a property
            // where it was written, so it jumps to the declaration that carries
            // the hooks rather than into one of their bodies.
            ExprInfo::Property { class, name, .. }
            | ExprInfo::StaticProperty { class, name, .. }
            | ExprInfo::HookedProperty { class, name, .. } => Target::Property { class, name },
            _ => return None,
        };
        Some((target, node.span))
    })
}
