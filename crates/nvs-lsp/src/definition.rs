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
//! `nvs_types::ExprInfo` already carries what each `new`, `is` test and
//! enum-case access resolved to, with the namespace and the
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
//! **A written type is not in the tree, so it is walked for.** A type name is a
//! property of the node that writes it, which is why nothing above looks for a
//! node called `TypeName` — and it is why [`written_type_at`] enumerates the
//! positions a type is *written* at rather than asking the index anything.
//! Those positions are a declaration's own: a `type` alias's right-hand side at
//! either declaration site, a property's and a class constant's declared type,
//! a method's parameter and return types, a property hook's parameter, a
//! `foreach` binding's and a `catch` clause's, and a typed local's wherever a
//! statement sequence holds one. A type written inside an *expression* — `as`,
//! `is`, a closure literal's signature — is reached through none of those and
//! is answered by nothing here.
//!
//! What that walk exists for is [`type_member_at`]. `Owner::Name` in type
//! position is the one type atom that names a **member**, and which member it
//! names is what the owner declares: a `type` alias of that owner, one of its
//! enum cases, or one of its constants, in that order — an order that decides
//! nothing a compiling program can observe, because a body declaring two of
//! them under one name is refused where the second is written
//! (`rule:types/type-alias`).
//!
//! **A `use` line names a type too.** An import is a statement and no
//! expression, so nothing above reaches it, and what it resolved to is already
//! `nvs_hir::Import`'s: [`import_at`] answers the target of the import whose
//! path the cursor is on. One that resolved to nothing answers nothing, and
//! already carries its diagnostic.
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
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, DocComment, EnumCase, MethodMember, Param, PropertyHook,
    PropertyHookBody, Stmt, StmtKind, Type, TypeAtom, TypeKind,
};
use nvs_types::{ExprInfo, ResolvedCall, Ty, TypeId, TypeInterner};
use rustc_hash::FxHashMap;

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
        ExprInfo::New { class, .. } => Some(class.clone()),
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
        // `$x is Shape` names a type and allocates nothing, so the cursor on
        // it opens that declaration. A test the checker settled carries a
        // constant instead and has no type to open, which is the same silence
        // every unresolved node here gets.
        ExprInfo::TypeTest { tested } => *tested,
        ExprInfo::ClassRefTest { base } => *base,
        ExprInfo::Call(call) | ExprInfo::ClassRefCall(call) => call.return_ty,
        _ => return None,
    })
}

/// The one class a type names, borrowed out of the interner rather than
/// cloned — [`class_of`]'s question asked where the answer is a *target* and
/// not a fact about a value, so it must live as long as the analysis.
///
/// A nullable is one class beside `null` and is deliberately not unwrapped
/// here: `is ?Foo` is a union in the interner and a union names no single
/// declaration, which is the same silence [`class_of`] gives one.
fn class_named_by(interner: &TypeInterner, ty: TypeId) -> Option<&QName> {
    match interner.get(ty) {
        Ty::Class(qname, _) | Ty::Enum(qname, _) | Ty::EnumCase(qname, _, _) => Some(qname),
        Ty::ClassRef(inner) => class_named_by(interner, *inner),
        _ => None,
    }
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
    /// A `type` alias a body owns, on [`Target::Constant`]'s terms and told
    /// apart from one by what its owner declares rather than by how the source
    /// spells it: `Owner::Name` is one spelling for all three member kinds
    /// reachable in type position, and it is the only position an alias is
    /// reachable from at all (`rule:types/type-alias`).
    TypeAlias {
        /// The declaring class, interface or enum.
        class: &'a QName,
        /// The alias's own name.
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
    /// A `type` alias a body owns, whose name the source writes bare and in
    /// type position alone.
    TypeAlias,
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
        Target::TypeAlias { class, name } => (*class, Some((*name, MemberKind::TypeAlias))),
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
        (ClassMemberKind::TypeAlias(alias), MemberKind::TypeAlias) => {
            text_of(file, alias.name.span) == name
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
        ClassMemberKind::TypeAlias(alias) => Some(alias.name.span),
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
///
/// **The enum written in front of a case is answered from the node inside.**
/// `Status::Draft` is recorded once, on the whole production, and the case is
/// what it resolved to — so a cursor on `Status` would be answered with the
/// case, while `crate::index` records that same span as a use of the *enum*,
/// which it may because no enum extends another. The production writes its own
/// name without a node of its own and holds exactly one child, the qualifier,
/// so a cursor that is inside a node inside this one is on the qualifier and
/// asks about the enum. `Cart::LIMIT` is not this case and still answers the
/// constant wherever the cursor is: its entry names the class that *declares*
/// the constant, which need not be the one the source wrote.
/// A name no recorded expression covers is answered last, by the three places
/// a name is written outside an expression: an `extends` or `implements`
/// clause off the hierarchy graph ([`clause_at`]), an `Owner::Name` in type
/// position ([`type_member_at`]), and a `use` line ([`import_at`]).
pub(crate) fn named_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    let path = analysed.index.at(offset);
    let nodes = path.nodes();
    nodes
        .iter()
        .enumerate()
        .find_map(|(depth, node)| {
            let info = analysed.exprs.lookup(node.span)?;
            let inside = depth.checked_sub(1).map(|inner| nodes[inner].span);
            match (info, inside) {
                (ExprInfo::EnumCase { enum_, .. }, Some(qualifier)) => {
                    Some((Target::Type(enum_), qualifier))
                }
                // `$x is Shape` writes a type where no other expression does,
                // so the name is reached through the interner rather than off
                // the record: the entry carries the type it lowered, and the
                // class inside it is the declaration to open. A test against
                // anything but one class — a scalar, a union — names no single
                // declaration and falls through to the node above.
                (ExprInfo::TypeTest { tested }, _) => Some((
                    Target::Type(class_named_by(&analysed.interner, *tested)?),
                    node.span,
                )),
                _ => Some((target_of(info)?, node.span)),
            }
        })
        .or_else(|| clause_at(analysed, offset))
        .or_else(|| type_member_at(analysed, offset))
        .or_else(|| import_at(analysed, offset))
}

/// The type a `use` line of the entry document imports at `offset`, and the
/// span the source wrote its path at.
///
/// Read off `nvs_hir::Module::imports`, which holds what each import resolved
/// to, so nothing is resolved again here. The whole path is the name: a cursor
/// anywhere in `App\Models\User` answers `User`, because a namespace segment
/// has no declaration to open. An import that resolved to nothing is passed
/// over, and one naming a `Core` class answers a target [`site`] finds no file
/// for.
pub(crate) fn import_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    analysed
        .module
        .imports
        .iter()
        .filter(|import| import.span.file == analysed.entry && import.resolved)
        .find(|import| covers(import.span, offset))
        .map(|import| (Target::Type(&import.target), import.span))
}

/// The supertype an `extends` or `implements` clause of the entry document
/// names at `offset`, and the span the source wrote it at.
///
/// A clause name is a use, and the one kind the walk above cannot see: a clause
/// is not an expression, so no entry of [`Analysed::exprs`](crate::Analysed)
/// covers one. What it resolved to is `nvs_hir::ClassLinks`, off the graph the
/// checker already built, paired against the written names by position
/// ([`paired`]) — the same pairing `crate::index` records the occurrence side
/// of a clause from, so a jump out of `extends Base` and the reference list
/// that names that clause are answered with one name.
///
/// The entry document alone, because that is the file a cursor is ever in, and
/// the name the *clause resolved to* rather than the text in front of it: under
/// `use App\Base;`, `extends Base` answers `App\Base`.
pub(crate) fn clause_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    analysed.module.symbols.iter().find_map(|symbol| {
        if symbol.decl_span.file != analysed.entry {
            return None;
        }
        let links = analysed.module.graph.get(&symbol.qname)?;
        let (stmt, _) = declared_type(analysed, &symbol.qname)?;
        let (extends, implements) = supertype_names(stmt);
        let (bases, resolved_bases) = paired(&extends, &links.extends);
        let (ifaces, resolved_ifaces) = paired(&implements, &links.implements);
        bases
            .iter()
            .zip(resolved_bases)
            .chain(ifaces.iter().zip(resolved_ifaces))
            .find(|(name, _)| covers(**name, offset))
            .map(|(name, qname)| (Target::Type(qname), *name))
    })
}

/// The names one declaration's `extends` and `implements` clauses write, in the
/// order they were written.
///
/// That order is `nvs_hir::HierarchyResolver::collect_links`'s own, which is
/// what lets [`paired`] match the written names to the resolved ones by
/// position. An enum writes neither: `rule:enums/no-class-machinery` rejects
/// `implements` on one and there is no `extends` grammar for it at all, so the
/// graph holds no entry for an enum to pair against either.
pub(crate) fn supertype_names(stmt: &Stmt) -> (Vec<Span>, Vec<Span>) {
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => (
            decl.extends.iter().map(|base| base.span).collect(),
            decl.implements
                .iter()
                .map(|entry| entry.name.span)
                .collect(),
        ),
        StmtKind::InterfaceDecl(decl) => (
            decl.extends.iter().map(|parent| parent.span).collect(),
            Vec::new(),
        ),
        _ => (Vec::new(), Vec::new()),
    }
}

/// The written clause names and what they resolved to, to be read off by
/// position — and two empty slices unless the two sides are the same length.
///
/// `nvs_hir::HierarchyResolver::resolve` keeps a name that resolved and drops
/// one that named nothing or named the wrong kind of declaration, so a clause
/// with an unresolved entry in it would otherwise pair every name after that
/// one with its neighbour's symbol. A file whose clause did not fully resolve
/// already carries an `E_UNDEFINED_CLASS`, and answering nothing out of that
/// clause is what cannot be wrong about which name a reader is looking at.
pub(crate) fn paired<'a, 'b>(
    written: &'a [Span],
    resolved: &'b [QName],
) -> (&'a [Span], &'b [QName]) {
    if written.len() == resolved.len() {
        (written, resolved)
    } else {
        (&[], &[])
    }
}

/// Whether `offset` is inside `span`, its last byte included.
///
/// One byte wider than `nvs_diagnostics::Span::contains`, and deliberately: a
/// caret just past the `r` of `User` is on `User` to whoever put it there, and
/// a double-click leaves it exactly there. It is the bound `crate::server`'s
/// hierarchy request has always asked a declaration about.
pub(crate) const fn covers(span: Span, offset: BytePos) -> bool {
    span.start <= offset && offset <= span.end
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
        ExprInfo::New { class, .. } => Target::Type(class),
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

/// What an `Owner::Name` written in type position at `offset` names, and the
/// span the source wrote that half of it at.
///
/// The second kind of name the expression walk cannot see, beside a clause's
/// ([`clause_at`]): a type is not an expression either. Which member the name
/// picks out is the module doc's order — the owner's own `type` alias first,
/// then the enum case or the constant [`site`] already looks a
/// [`Target::Constant`] up as.
///
/// **A cursor in the owner half answers the owner**, exactly as a cursor on the
/// qualifier of `Status::Draft` answers the enum: the two halves of one
/// spelling are two names and the caret says which one is being asked about.
pub(crate) fn type_member_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    let ty = written_type_at(analysed, offset)?;
    let TypeKind::Atom(TypeAtom::Member(written, member)) = &ty.kind else {
        return None;
    };
    let file = analysed.map.file(analysed.entry);
    let resolved = resolved_name(analysed, text_of(file, written.span), written.span.start)?;
    let owner = &analysed.module.symbols.get(&resolved)?.qname;
    if !covers(*member, offset) {
        return Some((Target::Type(owner), written.span));
    }
    let name = text_of(file, *member);
    let target = if analysed.module.aliases.get_member(owner, name).is_some() {
        Target::TypeAlias { class: owner, name }
    } else {
        Target::Constant { class: owner, name }
    };
    Some((target, *member))
}

/// What the name `text`, written at `at` in the entry document, means — and
/// `None` for an empty spelling, which is a cursor that has written no name at
/// all rather than one that named the root.
///
/// `nvs_hir::resolve_ref` is the whole of the decision
/// (`rule:statements/one-function-resolves-every-name`); what is added here is
/// the two things it takes and a cursor answer has to find for itself, which
/// are [`namespace_at`] and [`imports_of`].
pub(crate) fn resolved_name(analysed: &Analysed, text: &str, at: BytePos) -> Option<QName> {
    if text.is_empty() {
        return None;
    }
    Some(nvs_hir::resolve_ref(
        text,
        &namespace_at(analysed, at),
        &imports_of(analysed),
    ))
}

/// The namespace `offset` is written inside, as its segments.
///
/// Walked off the entry document's own statements because `nvs-hir` applies a
/// namespace as it collects declarations and keeps no map from a position back
/// to one. The two forms are the ones its resolver reads: `namespace Name;`
/// governs the rest of the file, and the bracketed form governs its block.
pub(crate) fn namespace_at(analysed: &Analysed, offset: BytePos) -> Vec<String> {
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
pub(crate) fn imports_of(analysed: &Analysed) -> FxHashMap<String, QName> {
    analysed
        .module
        .imports
        .iter()
        .filter(|import| import.span.file == analysed.entry)
        .map(|import| (import.short_name.clone(), import.target.clone()))
        .collect()
}

/// The written type the cursor at `offset` is inside, innermost first, and
/// `None` for a cursor inside none.
///
/// The positions walked are the module doc's, and they are a **declaration's**:
/// a type reached only through an expression is not among them. Innermost for
/// [`named_at`]'s reason — `array<Order::Meta>` covers one offset with two
/// written types, and the nearer one is what the caret is pointing at.
pub(crate) fn written_type_at(analysed: &Analysed, offset: BytePos) -> Option<&Type> {
    let entry = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)?;
    stmts_type(&entry.stmts, offset)
}

/// The written type at `offset` in one statement sequence.
fn stmts_type(stmts: &[Stmt], offset: BytePos) -> Option<&Type> {
    stmts.iter().find_map(|stmt| stmt_type(stmt, offset))
}

/// The written type at `offset` in one statement, and nothing for a statement
/// the offset is outside.
///
/// The span test is what makes this one path down the file rather than a walk
/// of all of it: a statement the cursor is not in holds no type the cursor
/// could be inside. A top-level `function` or `const` is deliberately absent
/// for `rule:ide/rejected-syntax-gets-no-colour`'s reason — the construct is
/// refused, so the server answers nothing about the names written inside it.
fn stmt_type(stmt: &Stmt, offset: BytePos) -> Option<&Type> {
    if !covers(stmt.span, offset) {
        return None;
    }
    match &stmt.kind {
        StmtKind::TypeAliasDecl(decl) => type_in(&decl.ty, offset),
        StmtKind::ClassDecl(decl) => members_type(&decl.members, offset),
        StmtKind::InterfaceDecl(decl) => members_type(&decl.members, offset),
        StmtKind::EnumDecl(decl) => members_type(&decl.members, offset),
        StmtKind::NamespaceDecl(decl) => stmts_type(&decl.body.as_ref()?.stmts, offset),
        StmtKind::LocalDecl { ty, .. } => type_in(ty.as_ref()?, offset),
        StmtKind::Block(block) => stmts_type(&block.stmts, offset),
        StmtKind::If { then, else_, .. } => {
            stmt_type(then, offset).or_else(|| stmt_type(else_.as_deref()?, offset))
        }
        StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } => stmt_type(body, offset),
        StmtKind::For { init, body, .. } => init
            .decl()
            .and_then(|decl| stmt_type(decl, offset))
            .or_else(|| stmt_type(body, offset)),
        StmtKind::Foreach {
            key, value, body, ..
        } => key
            .as_ref()
            .and_then(|binding| type_in(binding.ty.as_ref()?, offset))
            .or_else(|| type_in(value.ty.as_ref()?, offset))
            .or_else(|| stmt_type(body, offset)),
        StmtKind::Switch { cases, .. } => {
            cases.iter().find_map(|arm| stmts_type(&arm.body, offset))
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => stmts_type(&body.stmts, offset)
            .or_else(|| {
                catches.iter().find_map(|catch| {
                    type_in(&catch.ty, offset).or_else(|| stmts_type(&catch.body.stmts, offset))
                })
            })
            .or_else(|| stmts_type(&finally.as_ref()?.stmts, offset)),
        _ => None,
    }
}

/// The written type at `offset` among one body's members.
fn members_type(members: &[ClassMember], offset: BytePos) -> Option<&Type> {
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Property(property) => {
            type_in(&property.ty, offset).or_else(|| hooks_type(property.hooks.as_deref()?, offset))
        }
        ClassMemberKind::Const(constant) => type_in(constant.ty.as_ref()?, offset),
        ClassMemberKind::TypeAlias(alias) => type_in(&alias.ty, offset),
        ClassMemberKind::Method(method) => method_type(method, offset),
        _ => None,
    })
}

/// The written type at `offset` in one method's signature or its body.
fn method_type(method: &MethodMember, offset: BytePos) -> Option<&Type> {
    params_type(&method.params, offset)
        .or_else(|| type_in(method.return_type.as_ref()?, offset))
        .or_else(|| stmts_type(&method.body.as_ref()?.stmts, offset))
}

/// The written type at `offset` in one parameter list.
fn params_type(params: &[Param], offset: BytePos) -> Option<&Type> {
    params
        .iter()
        .find_map(|param| type_in(param.ty.as_ref()?, offset))
}

/// The written type at `offset` in one property's hook block.
///
/// A hook is a method in the two ways that matter here: `set(Order::Meta $m)`
/// writes a parameter type, and a block-bodied hook writes statements. The
/// short `=> expr;` form writes an expression and so writes no type this walk
/// reaches.
fn hooks_type(hooks: &[PropertyHook], offset: BytePos) -> Option<&Type> {
    hooks.iter().find_map(|hook| {
        hook.param
            .as_ref()
            .and_then(|param| type_in(param.ty.as_ref()?, offset))
            .or_else(|| match hook.body.as_ref()? {
                PropertyHookBody::Block(block) => stmts_type(&block.stmts, offset),
                PropertyHookBody::Expr(_) => None,
            })
    })
}

/// The innermost written type covering `offset` inside `ty`, and `None` for an
/// offset outside it.
fn type_in(ty: &Type, offset: BytePos) -> Option<&Type> {
    if !covers(ty.span, offset) {
        return None;
    }
    let inner = match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => type_in(inner, offset),
        TypeKind::Union(members) | TypeKind::Intersection(members) => {
            members.iter().find_map(|member| type_in(member, offset))
        }
        TypeKind::Atom(atom) => atom_type(atom, offset),
        _ => None,
    };
    Some(inner.unwrap_or(ty))
}

/// The innermost written type covering `offset` among one atom's own
/// arguments, and `None` for an atom that writes none.
fn atom_type(atom: &TypeAtom, offset: BytePos) -> Option<&Type> {
    match atom {
        TypeAtom::Array(Some(arg)) | TypeAtom::ClassRef(arg) | TypeAtom::PropertyKey(arg) => {
            type_in(arg, offset)
        }
        TypeAtom::Shape(fields) => fields.iter().find_map(|field| type_in(&field.ty, offset)),
        TypeAtom::CallableSig { params, ret } => params
            .iter()
            .find_map(|param| type_in(param, offset))
            .or_else(|| type_in(ret, offset)),
        TypeAtom::Name(_, args) => args.iter().find_map(|arg| type_in(arg, offset)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Documents, analyse, uri_of};

    /// A class and an enum that each own a `type` alias, named from two of the
    /// positions a type is written at: a parameter's and a typed local's.
    const OWNERS: &str = "<?nvs\nclass Order {\n  type Meta = {total: int};\n  public function \
                          of(Order::Meta $m): int { return 1; }\n}\nenum Status {\n  type Pair = \
                          array<Status>;\n  Open,\n}\nStatus::Pair $p = [Status::Open];\n";

    /// Where a cursor just past the first `written` in `source` jumps to, as
    /// `line:character` — the declared name's own range, which is the caret an
    /// editor puts there — and `none` where it jumps nowhere.
    fn jump(source: &str, written: &str) -> String {
        let uri = uri_of(&std::env::temp_dir().join("nvs-definition-case.nvs"))
            .expect("a temp path is UTF-8");
        let mut documents = Documents::new();
        documents.open(uri.clone(), 1, source.to_owned());
        let analysed = analyse(&documents, &uri).expect("an open document analyses");
        let found = source.find(written).expect("the document writes it") + written.len();
        let offset = u32::try_from(found).expect("a test document is short");
        at(&analysed, offset, PositionEncoding::Utf8).map_or_else(
            || "none".to_owned(),
            |declared| {
                format!(
                    "{}:{}",
                    declared.range.start.line, declared.range.start.character
                )
            },
        )
    }

    /// An interface, one implementor, and an `is` test against the interface —
    /// one buffer, because what is under test is which name the request reads
    /// off the node rather than how far it reaches for the declaration.
    ///
    /// The subject is a `mixed` local on purpose: a test the declaration
    /// settles folds to a constant and leaves no node to navigate from, which
    /// [`recorded_ty`] is the other half of.
    const TESTED: &str = "<?nvs\ninterface Shape { public function area(): int; }\n\
                          class Square implements Shape { public function area(): int { return 1; \
                          } }\nmixed $m = new Square();\nif ($m is Shape) { echo 1; }\n";

    /// The type after `is` is a written name like any other, so a cursor on it
    /// opens the declaration the test is against: one type test
    /// (`rule:php-migration/one-type-test`) is also one navigation, and the
    /// interface is what a test against an interface names.
    #[test]
    fn definition_on_the_type_after_is_answers_the_interface_it_tests_against() {
        assert_eq!(jump(TESTED, "is Sha"), "1:10");
    }

    /// A namespaced class and the three imports a `use` line can be: one that
    /// resolved to a declaration, one that resolved to nothing, and one naming
    /// a `Core` class, which has no Novis declaration to open.
    const IMPORTS: &str = "<?nvs\nnamespace App;\n\nuse App\\User;\nuse App\\Missing;\n\
                           use Core\\Str;\n\nclass User {}\n";

    /// A `use` line is a written name like a clause's, so a cursor anywhere in
    /// its path opens the declaration it imports — and an import with nothing
    /// to open answers nothing.
    #[test]
    fn definition_on_a_use_line_is_the_declaration_it_imports() {
        assert_eq!(jump(IMPORTS, "use App\\Us"), "7:6");
        assert_eq!(jump(IMPORTS, "use Ap"), "7:6");
        assert_eq!(jump(IMPORTS, "use App\\Miss"), "none");
        assert_eq!(jump(IMPORTS, "use Core\\St"), "none");
    }

    /// `Owner::Name` in type position lands on the member, and the owner half
    /// of the same spelling lands on the owner — in a class body and an enum
    /// body alike (`rule:types/type-alias`). The last case is the bound: a type
    /// that names no member is not a member reference, so the request answers
    /// nothing rather than the nearest declaration.
    #[test]
    fn definition_of_owner_name_in_type_position_is_the_member() {
        assert_eq!(jump(OWNERS, "of(Order::Me"), "2:7");
        assert_eq!(jump(OWNERS, "of(Ord"), "1:6");
        assert_eq!(jump(OWNERS, "Status::Pa"), "6:7");
        assert_eq!(jump(OWNERS, "Order::Meta $m): i"), "none");
    }
}
