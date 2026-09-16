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
//! **An enum case answers the case.** `Mode::Read` under the cursor jumps to
//! `case Read`, because `rule:enums/no-class-machinery` makes a case a constant
//! declared on its enum and a constant is a member like any other. `enum Mode`
//! is what [`type_at`] answers there, which is the request that asks what
//! something *is* rather than where the name came from.
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
use nvs_syntax::ast::{ClassMember, ClassMemberKind, DocComment, EnumCase, Stmt, StmtKind};
use nvs_types::{ExprInfo, ResolvedCall, Ty, TypeId, TypeInterner};

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

/// Where the **type** of the expression at `offset` is declared.
///
/// [`at`]'s answer one step further along: that one follows the name the cursor
/// is on, and this one follows the type the checker recorded for it, so a
/// cursor on `$c->engine` reaches `class Engine` rather than the property that
/// holds one. The walk from a class name to its declaration is [`site`]'s in
/// both, which is what keeps the `require`/`autoload` graph resolved once and
/// what makes a type declared in a required file reachable here at all.
///
/// `None` for an expression the checker recorded no type for — a plain local
/// read is the common one, for [`crate::hover`]'s reason — and for a type with
/// no declaration to open: a scalar, an array, a union of more than one class,
/// and a `Core` class, whose declaration is Rust.
#[must_use]
pub fn type_at(
    analysed: &Analysed,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<Declared> {
    let class = analysed
        .index
        .at(offset)
        .nodes()
        .iter()
        .find_map(|node| instance_of(analysed, analysed.exprs.lookup(node.span)?))?;
    let declared = site(analysed, &Target::Type(&class))?;
    let file = analysed.map.file(declared.span.file);
    Some(Declared {
        path: file.path()?.to_path_buf(),
        range: range_at(file, declared.span, encoding),
    })
}

/// The class one recorded expression is an instance of, by name.
///
/// Read off the type the checker gave the expression rather than off the name
/// it wrote, because only some of them wrote one: `$c->engine` names a property
/// and is an `Engine`, and the type table is the only place that second fact
/// is. The three that *are* their own type answer it directly — asking for the
/// type of `new User()` is asking about `User`, which is also what [`at`]
/// answers there and is the right answer rather than a duplicated one.
fn instance_of(analysed: &Analysed, info: &ExprInfo) -> Option<QName> {
    match info {
        ExprInfo::New { class, .. } | ExprInfo::InstanceOf { class } => Some(class.clone()),
        ExprInfo::EnumCase { enum_, .. } => Some(enum_.clone()),
        _ => class_of(&analysed.interner, recorded_ty(info)?),
    }
}

/// The type the checker recorded on one expression, and `None` for an entry
/// that carries none.
///
/// The same entries [`crate::hover`]'s type arm reads, plus a call's return
/// type: hovering a call shows its whole row, so the return type is already on
/// screen there and is what this request is asking for.
const fn recorded_ty(info: &ExprInfo) -> Option<TypeId> {
    Some(match info {
        ExprInfo::Property { ty, .. }
        | ExprInfo::StaticProperty { ty, .. }
        | ExprInfo::HookedProperty { ty, .. } => *ty,
        ExprInfo::Index { elem_ty, .. } => *elem_ty,
        ExprInfo::NarrowedRead { to } => *to,
        ExprInfo::Call(call) | ExprInfo::ClassRefCall(call) => call.return_ty,
        _ => return None,
    })
}

/// The class a type is an instance of, through the two wrappers that do not
/// change the answer.
///
/// A `ClassRef` is a class written as a value, and a nullable type is one class
/// beside `null` — neither is a different declaration to open. A union of two
/// classes is: there are two answers and this request's response holds one, so
/// it answers none rather than picking one of them.
fn class_of(interner: &TypeInterner, ty: TypeId) -> Option<QName> {
    match interner.get(ty) {
        Ty::Class(qname, _) | Ty::Enum(qname, _) | Ty::EnumCase(qname, _, _) => Some(qname.clone()),
        Ty::ClassRef(inner) => class_of(interner, *inner),
        Ty::Union(members) => {
            let mut classes = members
                .iter()
                .filter(|id| !matches!(interner.get(**id), Ty::Null));
            let one = *classes.next()?;
            if classes.next().is_some() {
                return None;
            }
            class_of(interner, one)
        }
        _ => None,
    }
}

/// What the node under the cursor named, and so what is looked up to answer it.
///
/// A type and a member are two lookups and not one: a type is declared at file
/// scope and a member inside a declaration, so they are found in different
/// places even though both start at the same [`nvs_hir::SymbolTable`] entry. A
/// method, a property and a constant are separate arms rather than one name
/// and a kind flag because `class C { public int $x; public function x(): int
/// … }` is legal — one name, more than one declaration, and nothing but which
/// list the checker resolved it in tells them apart.
pub(crate) enum Target<'a> {
    /// A class, an interface or an enum, by its fully-qualified name.
    Type(&'a QName),
    /// A method, as the checker resolved the call to it — named on the class
    /// that **declares** it rather than on the receiver's, which is what
    /// `nvs_types::ResolvedCall::class` already is, so an inherited method
    /// answers where its body was written.
    ///
    /// The whole resolution travels rather than the two names off it, because
    /// [`crate::hover`] spells a `Core` member's signature out of the parameter
    /// types and names recorded here. That member has no declaration for the
    /// walk below to reach, so the call site is the only place its signature
    /// survives in a form this crate can read.
    Method(&'a ResolvedCall),
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
    /// A class constant, on [`Target::Property`]'s terms and told apart from
    /// one by the sigil neither carries here: the source writes `C::$x` for the
    /// property and `C::NAME` for the constant, so the two spellings differ
    /// where they are compared even though both names arrive bare.
    ///
    /// An **enum case** is one of these and not a type: `rule:enums/no-class-machinery`
    /// makes the case its own integer constant, declared on the enum the way a
    /// constant is declared on a class, so `Suit::Hearts` resolves here and the
    /// enum is what [`type_at`] answers instead.
    Constant {
        /// The declaring class, or the enum a case belongs to.
        class: &'a QName,
        /// The constant's own name.
        name: &'a str,
    },
}

/// Which of a declaration's member lists a name is looked for in.
///
/// A class writes three lists a resolved name can land in and one name may be
/// in all three — `class C { public const int X = 1; public int $x; public
/// function x(): int … }` — so what resolved is carried alongside the name
/// rather than guessed from it.
#[derive(Clone, Copy)]
enum MemberKind {
    /// A method, whose name the source writes bare.
    Method,
    /// A property, whose name the source writes with its `$`.
    Property,
    /// A class constant, whose name the source writes bare.
    Constant,
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
        Target::Method(call) => (
            &call.class,
            Some((call.method.as_str(), MemberKind::Method)),
        ),
        Target::Property { class, name } => (*class, Some((*name, MemberKind::Property))),
        Target::Constant { class, name } => (*class, Some((*name, MemberKind::Constant))),
    };
    let symbol = analysed.module.symbols.get(class)?;
    let file = analysed.map.file(symbol.decl_span.file);
    let stmt = declared_type(analysed, class).map(|(stmt, _)| stmt);
    let Some((name, kind)) = member else {
        return Some(Site {
            span: symbol.decl_span,
            doc: stmt.and_then(doc_of),
        });
    };
    let stmt = stmt?;
    // A case is declared in its own list and not among the members, so it is
    // the one member name reached without [`declared_member`].
    if let Some(case) = declared_case(stmt, file, name, kind) {
        return Some(Site {
            span: case.name.span,
            doc: case.doc.as_ref(),
        });
    }
    let member = declared_member(stmt, file, name, kind)?;
    Some(Site {
        span: member_name(member)?,
        doc: member.doc.as_ref(),
    })
}

/// The declaration `class` was written as, and the file it is in.
///
/// The half of [`site`] that stops at the type: [`crate::completion`] needs the
/// members a class declares rather than one of them, and re-finding the
/// declaration for itself would be a second answer to the question
/// [`nvs_hir::SymbolTable`] already settled.
pub(crate) fn declared_type<'a>(
    analysed: &'a Analysed,
    class: &QName,
) -> Option<(&'a Stmt, &'a SourceFile)> {
    let symbol = analysed.module.symbols.get(class)?;
    let stmt = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == symbol.decl_span.file)
        .and_then(|loaded| declaration(&loaded.stmts, symbol.decl_span))?;
    Some((stmt, analysed.map.file(symbol.decl_span.file)))
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

/// The member of `stmt` written as `name`, of the kind `kind` selects.
///
/// A type alias has no body and an enum's cases are not what a call, a property
/// access or a constant read resolves to, so both answer nothing here rather
/// than being searched.
fn declared_member<'a>(
    stmt: &'a Stmt,
    file: &SourceFile,
    name: &str,
    kind: MemberKind,
) -> Option<&'a ClassMember> {
    let members = match &stmt.kind {
        StmtKind::ClassDecl(decl) => &decl.members,
        StmtKind::InterfaceDecl(decl) => &decl.members,
        StmtKind::EnumDecl(decl) => &decl.members,
        _ => return None,
    };
    members.iter().find(|member| match (&member.kind, kind) {
        (ClassMemberKind::Method(method), MemberKind::Method) => text_of(file, method.name) == name,
        (ClassMemberKind::Property(property), MemberKind::Property) => {
            text_of(file, property.name).trim_start_matches('$') == name
        }
        (ClassMemberKind::Const(constant), MemberKind::Constant) => {
            text_of(file, constant.name) == name
        }
        _ => false,
    })
}

/// The case of `stmt` written as `name`, and nothing for any other kind of
/// name or any declaration that is not an enum.
///
/// `rule:enums/no-class-machinery` makes a case an integer constant declared on
/// its enum, so a read of one arrives here as a [`Target::Constant`] like a
/// class constant does — and is then looked for in the list an enum keeps its
/// cases in, which is not the member list every other declaration uses.
fn declared_case<'a>(
    stmt: &'a Stmt,
    file: &SourceFile,
    name: &str,
    kind: MemberKind,
) -> Option<&'a EnumCase> {
    let StmtKind::EnumDecl(decl) = &stmt.kind else {
        return None;
    };
    if !matches!(kind, MemberKind::Constant) {
        return None;
    }
    decl.cases
        .iter()
        .find(|case| text_of(file, case.name.span) == name)
}

/// A member's own name span, or nothing for a shape that has none.
fn member_name(member: &ClassMember) -> Option<Span> {
    match &member.kind {
        ClassMemberKind::Method(method) => Some(method.name),
        ClassMemberKind::Property(property) => Some(property.name),
        ClassMemberKind::Const(constant) => Some(constant.name),
        _ => None,
    }
}

/// The source text `span` covers, or nothing for a span outside the file.
///
/// A parser's span is always inside the file it parsed, so the fallback is
/// unreachable rather than a policy — and it is a fallback rather than an index
/// because a panic here would take the server down over one malformed name.
pub(crate) fn text_of(file: &SourceFile, span: Span) -> &str {
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
    analysed
        .index
        .at(offset)
        .nodes()
        .iter()
        .find_map(|node| Some((target_of(analysed.exprs.lookup(node.span)?)?, node.span)))
}

/// The name one recorded expression carries, or `None` for one that carries
/// none this can follow.
///
/// Split out of the walk above because [`crate::hover`] has a second answer for
/// a node that names nothing — the type it was recorded with — and trying both
/// at each node is what keeps the innermost node the one answered. A walk that
/// found the nearest *named* node first would answer a call's own card for a
/// cursor sitting on one of its arguments.
pub(crate) fn target_of(info: &ExprInfo) -> Option<Target<'_>> {
    Some(match info {
        ExprInfo::New { class, .. } | ExprInfo::InstanceOf { class } => Target::Type(class),
        // The bound, which is the only class this site named — the one
        // allocated is whatever descriptor is in hand, and no compile
        // knows it (`nvs_types::ExprInfo::NewDynamic`).
        ExprInfo::NewDynamic { bound, .. } => Target::Type(bound),
        // The case and not the enum around it, which is the declaration the
        // index already keys under `Status::Draft` — the enum is a fact about
        // the case's *type* and [`type_at`] is the request that asks for one.
        ExprInfo::EnumCase { enum_, case, .. } => Target::Constant {
            class: enum_,
            name: case,
        },
        ExprInfo::Call(call) | ExprInfo::CallableRef(call) | ExprInfo::ClassRefCall(call) => {
            Target::Method(call)
        }
        // A hooked property is a call to an accessor and still a property
        // where it was written, so it jumps to the declaration that carries
        // the hooks rather than into one of their bodies.
        ExprInfo::Property { class, name, .. }
        | ExprInfo::StaticProperty { class, name, .. }
        | ExprInfo::HookedProperty { class, name, .. } => Target::Property { class, name },
        // The class the constant was *declared* on, which is what the checker
        // recorded and what the declaration side of the index is keyed by —
        // reading an inherited constant through a subclass names one
        // declaration, not two (`nvs_types::ExprInfo::ClassConst`).
        ExprInfo::ClassConst { class, name, .. } => Target::Constant { class, name },
        _ => return None,
    })
}
