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
//! Those positions are a declaration's own — a `type` alias's right-hand side
//! at either declaration site, a property's and a class constant's declared
//! type, a method's parameter and return types, a property hook's parameter,
//! a `foreach` binding's and a `catch` clause's, and a typed local's wherever
//! a statement sequence holds one — and an expression's: an `as` conversion's
//! target, the right side of an `is` test, an anonymous function's signature, the
//! class a `catch` arm names, and the `<Type>` arguments a call or a `new`
//! writes. An expression is reached through the statement holding it and an
//! anonymous function's body through the function, so a type written anywhere in the entry
//! document is on the walk.
//!
//! **A written type is asked before the index is.** To the index a cursor in
//! the `<User>` of `decodeAs<User>()` is inside the call, and the call's method
//! is the wrong answer for a caret on the class it names. So [`named_at`]
//! consults the walk first, and the index only for a cursor no written type
//! covers.
//!
//! The walk answers two kinds of name. [`type_name_at`] is the plain class
//! name — `User $u`, `array<User>`, `implementing<User>()` — resolved the way
//! the checker resolves one
//! (`rule:statements/one-function-resolves-every-name`) and answered as the
//! declaration it names, or as the stub a `Core` class has instead.
//! [`type_member_at`] is `Owner::Name`, the one type atom that names a
//! **member**, and which member it names is what the owner declares: a `type`
//! alias of that owner, one of its enum cases, or one of its constants, in
//! that order — an order that decides nothing a compiling program can observe,
//! because a body declaring two of them under one name is refused where the
//! second is written (`rule:types/type-alias`).
//!
//! **A string can name a class too**, in two places: the operand of `as
//! class<T>` (`rule:types/class-reference`), and an argument at a parameter
//! the registry marks as a class name, such as `Core\Reflect::forClass`'s
//! (`crate::arguments`). [`written_class_name_at`] answers the class whose whole
//! name the literal's text is, compared the way the run-time conversion
//! compares it, so `'App\User' as class<Model>` jumps to `class User`. No
//! other string is read as a name.
//!
//! **A `use` line names a type too.** An import is a statement and no
//! expression, so nothing above reaches it, and what it resolved to is already
//! `nvs_hir::Import`'s: [`import_at`] answers the target of the import whose
//! path the cursor is on. One that resolved to nothing answers nothing, and
//! already carries its diagnostic.
//!
//! **A string argument equal to a completion file's value goes to its
//! `location`.** [`file_value`] reads the values completion offers at the
//! argument (`rule:ide/completion-files-offer-values-at-named-parameters`),
//! and the one whose `value` is the literal's text answers the start of its
//! 1-based line in its file. A value with no `location` answers nothing, and
//! not the method the call resolved to either.
//!
//! **The whole graph, not the entry alone.** The cursor is always in the open
//! document ([`crate::selection`]'s reasoning), but what it names may be
//! declared in a required file — so the span answered here carries its own
//! `SourceId` and the caller spells whichever file that is, exactly as
//! [`crate::links`] spells a `require`'s target.

use std::borrow::Cow;
use std::cell::OnceCell;
use std::path::PathBuf;

use lsp_types::{Position, Range};
use nvs_diagnostics::{BytePos, PositionEncoding, SourceFile, SourceId, Span};
use nvs_hir::QName;
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, DocComment, EnumCase, EnumDecl, Stmt, StmtKind, Type, TypeAtom,
    TypeKind,
};
pub(crate) use nvs_syntax::written::written_types_in;
use nvs_types::{ExprInfo, ResolvedCall, Ty, TypeId, TypeInterner};
use rustc_hash::FxHashMap;

use crate::completion_files::CompletionFiles;
use crate::document::Analysed;
use crate::position::range_at;

/// One declaration, and the file it was written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The file, as the analysis loaded it: absolute and canonical, on
    /// [`crate::links::PathLink`]'s terms.
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
/// the last of those, and it has no Novis declaration to open. A string
/// argument equal to a value of `files` answers that value's `location`.
#[must_use]
pub fn at(
    analysed: &Analysed,
    files: &CompletionFiles,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Option<Declared> {
    if let Some(located) = file_value(analysed, files, offset) {
        return located;
    }
    let (target, _) = named_at(analysed, offset)?;
    declared(analysed, &target, encoding)
}

/// The `location` of the completion file's value whose `value` is the text of
/// the string argument the cursor at `offset` is inside, as the start of its
/// line. The values are the ones completion offers at that argument. `None`
/// where the text is no value, and `Some(None)` for a value with no
/// `location`, which then has no definition.
fn file_value(
    analysed: &Analysed,
    files: &CompletionFiles,
    offset: BytePos,
) -> Option<Option<Declared>> {
    let named = crate::arguments::named_at(analysed, offset)?;
    let text =
        nvs_syntax::string_lit::cook_string_literal(analysed.map.file(analysed.entry), named.span);
    let value = files
        .values_at(&named.class, &named.method, &named.parameter, &named.others)
        .into_iter()
        .find(|value| value.value == text)?;
    Some(value.location.clone().map(|(path, line)| {
        let start = Position::new(line.get() - 1, 0);
        Declared {
            path,
            range: Range::new(start, start),
        }
    }))
}

/// Where `target` is declared: the declaration the graph holds, or the stub
/// the `Core` tree holds for a name no source file declares.
///
/// The stub is asked second and only for a name the walk found nothing for,
/// so a program's own class is never answered with a stub — and a `Core`
/// name is exactly one no symbol table holds, which is what
/// `rule:ide/the-stub-tree-is-where-core-is-declared` makes the tree for.
fn declared(
    analysed: &Analysed,
    target: &Target<'_>,
    encoding: PositionEncoding,
) -> Option<Declared> {
    if let Some(declared) = site(analysed, target) {
        let file = analysed.map.file(declared.span.file);
        return Some(Declared {
            path: file.path()?.to_path_buf(),
            range: range_at(file, declared.span, encoding),
        });
    }
    let (class, member) = match target {
        Target::Type(class) => (class.as_ref(), None),
        Target::Method(call) => (&call.class, Some(call.method.as_str())),
        Target::Constant { class, name } => (*class, Some(*name)),
        Target::Property { .. } | Target::TypeAlias { .. } => return None,
    };
    let (path, line) = analysed
        .stubs
        .as_ref()?
        .locate(&class.to_string(), member)?;
    Some(Declared {
        path,
        range: Range::new(
            Position::new(line.line, line.character),
            Position::new(line.line, line.character + line.length),
        ),
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
    declared(analysed, &Target::Type(Cow::Owned(class)), encoding)
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
    ///
    /// Borrowed from the checker's record wherever there is one, and owned
    /// for the one name nothing records: a class written in type position
    /// that no source file declares ([`type_name_at`]), which is a `Core`
    /// class on its way to the stub tree.
    Type(Cow<'a, QName>),
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
pub(crate) enum MemberKind {
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
        Target::Type(qname) => (qname.as_ref(), None),
        Target::Method(call) => (
            &call.class,
            Some((call.method.as_str(), MemberKind::Method)),
        ),
        Target::Property { class, name } => (*class, Some((*name, MemberKind::Property))),
        Target::Constant { class, name } => (*class, Some((*name, MemberKind::Constant))),
        Target::TypeAlias { class, name } => (*class, Some((*name, MemberKind::TypeAlias))),
    };
    site_of(analysed, class, member)
}

/// [`site`] by names alone: the type `class` declares, or the member of it
/// `member` names by its bare name and kind — what a caller holding a name it
/// read back off the wire, and no resolved call, asks.
pub(crate) fn site_of<'a>(
    analysed: &'a Analysed,
    class: &QName,
    member: Option<(&str, MemberKind)>,
) -> Option<Site<'a>> {
    let symbol = analysed.module.symbols.get(class)?;
    let file = analysed.map.file(symbol.decl_span.file);
    let stmt = declared_type(analysed, class).map(|(stmt, _)| stmt);
    let owned = match stmt {
        Some(_) => None,
        None => owned_enum_decl(analysed, class),
    };
    let Some((name, kind)) = member else {
        let doc = match (stmt, owned) {
            (Some(stmt), _) => doc_of(stmt),
            (None, Some((member, _))) => member.doc.as_ref(),
            (None, None) => None,
        };
        return Some(Site {
            span: symbol.decl_span,
            doc,
        });
    };
    let (cases, members) = match (stmt, owned) {
        (Some(stmt), _) => body_of(stmt)?,
        (None, Some((_, decl))) => (decl.cases.as_slice(), decl.members.as_slice()),
        (None, None) => return None,
    };
    // A case is declared in its own list and not among the members, so it is
    // the one member name reached without [`declared_member`].
    if let Some(case) = declared_case(cases, file, name, kind) {
        return Some(Site {
            span: case.name.span,
            doc: case.doc.as_ref(),
        });
    }
    let member = declared_member(members, file, name, kind)?;
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

/// The enum a class or interface body declares under the key `class`, and the
/// member it is written as, or nothing for a key no body declares.
///
/// An owned enum is no statement, so [`declared_type`] does not find it, and
/// its `///` run is on the member and not on the [`EnumDecl`]
/// (`rule:enums/class-scoped-enum`). The owner's declaration is found first,
/// and the enum among its members by the name span the symbol recorded.
pub(crate) fn owned_enum_decl<'a>(
    analysed: &'a Analysed,
    class: &QName,
) -> Option<(&'a ClassMember, &'a EnumDecl)> {
    let (owner, _) = class.as_member()?;
    let symbol = analysed.module.symbols.get(class)?;
    let (stmt, _) = declared_type(analysed, &owner)?;
    let (_, members) = body_of(stmt)?;
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Enum(decl) if decl.name.span == symbol.decl_span => Some((member, decl)),
        _ => None,
    })
}

/// The cases and the members a declaration's body writes, and nothing for a
/// declaration with no body. Only an enum has cases.
pub(crate) fn body_of(stmt: &Stmt) -> Option<(&[EnumCase], &[ClassMember])> {
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => Some((&[], &decl.members)),
        StmtKind::InterfaceDecl(decl) => Some((&[], &decl.members)),
        StmtKind::EnumDecl(decl) => Some((&decl.cases, &decl.members)),
        _ => None,
    }
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
        StmtKind::NamespaceDecl(decl) => declaration(&decl.body.as_ref()?.stmts, name),
        _ => (declared_span(stmt) == Some(name)).then_some(stmt),
    })
}

/// The name span a type declaration writes, and nothing for any other
/// statement.
fn declared_span(stmt: &Stmt) -> Option<Span> {
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => Some(decl.name.span),
        StmtKind::InterfaceDecl(decl) => Some(decl.name.span),
        StmtKind::EnumDecl(decl) => Some(decl.name.span),
        StmtKind::TypeAliasDecl(decl) => Some(decl.name.span),
        _ => None,
    }
}

/// Every type declaration in `stmts` by its name span, read the way
/// [`declaration`] reads them: a bracketed namespace's body is descended into,
/// and the first of two declarations at one span is kept.
fn declarations_into<'a>(stmts: &'a [Stmt], found: &mut FxHashMap<Span, &'a Stmt>) {
    for stmt in stmts {
        if let StmtKind::NamespaceDecl(decl) = &stmt.kind {
            if let Some(body) = &decl.body {
                declarations_into(&body.stmts, found);
            }
        } else if let Some(name) = declared_span(stmt) {
            found.entry(name).or_insert(stmt);
        }
    }
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

/// The member of `members` written as `name`, of the kind `kind` selects.
///
/// An enum's cases are not what a call, a property access or a constant read
/// resolves to, so they are not searched here.
fn declared_member<'a>(
    members: &'a [ClassMember],
    file: &SourceFile,
    name: &str,
    kind: MemberKind,
) -> Option<&'a ClassMember> {
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

/// The case of `cases` written as `name`, and nothing for any other kind of
/// name.
///
/// `rule:enums/no-class-machinery` makes a case an integer constant declared on
/// its enum, so a read of one arrives here as a [`Target::Constant`] like a
/// class constant does — and is then looked for in the list an enum keeps its
/// cases in, which is not the member list every other declaration uses.
fn declared_case<'a>(
    cases: &'a [EnumCase],
    file: &SourceFile,
    name: &str,
    kind: MemberKind,
) -> Option<&'a EnumCase> {
    if !matches!(kind, MemberKind::Constant) {
        return None;
    }
    cases
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
/// A call and a method reference are the same answer here: both carry the
/// `nvs_types::ResolvedCall` the checker made, and which of the two the site
/// wrote decides what happens to the callable, not where the method is.
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
/// **A written type is asked before any node is**, for the module doc's
/// reason: the index cannot see a type, so a cursor on one is inside whatever
/// wrote it — a call, for its `<Type>` argument — and that node is the wrong
/// answer. [`type_member_at`] and [`type_name_at`] answer nothing for a cursor
/// outside a written type, and the walk over the nodes then runs as before.
///
/// **A class-name literal is asked next**, for the same reason: the literal
/// records no name, so the walk over the nodes would step out to whatever
/// holds it. [`written_class_name_at`] answers the class the literal's text names.
///
/// A name no recorded expression covers is answered last, by the places a
/// name is written outside an expression and outside a type: an `extends` or
/// `implements` clause off the hierarchy graph ([`clause_at`]), an attribute's
/// name ([`attribute_at`]) and a `use` line ([`import_at`]).
pub(crate) fn named_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    let nodes: Vec<Span> = analysed
        .index
        .at(offset)
        .nodes()
        .iter()
        .map(|node| node.span)
        .collect();
    type_member_at(analysed, offset)
        .or_else(|| type_name_at(analysed, offset))
        .or_else(|| written_class_name_at(analysed, offset))
        .or_else(|| property_segment_at(analysed, offset).map(|found| (found.target, found.span)))
        .or_else(|| resolved_in(analysed, &nodes))
        .or_else(|| clause_at(analysed, offset))
        .or_else(|| attribute_at(analysed, offset))
        .or_else(|| import_at(analysed, offset))
}

/// The nearest of `nodes` — innermost first — that the checker recorded a
/// name for, and the span to underline.
///
/// The one walk [`named_at`] makes over the nodes, the index's own path. An
/// attribute's payload is on that path too, because `nvs_syntax::walk` makes
/// each attribute a node and its values that node's children.
fn resolved_in<'a>(analysed: &'a Analysed, nodes: &[Span]) -> Option<(Target<'a>, Span)> {
    nodes.iter().enumerate().find_map(|(depth, node)| {
        let info = analysed.exprs.lookup(*node)?;
        let inside = depth.checked_sub(1).map(|inner| nodes[inner]);
        match (info, inside) {
            (ExprInfo::EnumCase { enum_, .. }, Some(qualifier)) => {
                Some((Target::Type(Cow::Borrowed(enum_)), qualifier))
            }
            // `$x is Shape` writes a type where no other expression does,
            // so the name is reached through the interner rather than off
            // the record: the entry carries the type it lowered, and the
            // class inside it is the declaration to open. A test against
            // anything but one class — a scalar, a union — names no single
            // declaration and falls through to the node above.
            (ExprInfo::TypeTest { tested }, _) => Some((
                Target::Type(Cow::Borrowed(class_named_by(&analysed.interner, *tested)?)),
                *node,
            )),
            _ => Some((target_of(info)?, *node)),
        }
    })
}

/// The type the name of an attribute at `offset` in the entry document
/// resolved to, and the span the source wrote that name at.
///
/// Read off the checker's own record: an attribute's name is no expression, so
/// the index's `Attribute` node knows where it is written and not what it
/// names, and `nvs_types::attributes` recorded what it resolved to at the one
/// place it resolved it. A compiler attribute is a
/// `Core` name with no declaration, which is what the stub tree answers for; a
/// userland one is the `type` alias it names.
pub(crate) fn attribute_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    analysed
        .exprs
        .attribute_names()
        .filter(|(span, _)| span.file == analysed.entry)
        .find(|(span, _)| covers(*span, offset))
        .map(|(span, name)| (Target::Type(Cow::Borrowed(name)), span))
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
        .map(|import| (Target::Type(Cow::Borrowed(&import.target)), import.span))
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
    let scope = FileScope::of(analysed, analysed.entry);
    analysed.module.symbols.iter().find_map(|symbol| {
        if symbol.decl_span.file != analysed.entry {
            return None;
        }
        let links = analysed.module.graph.get(&symbol.qname)?;
        let (stmt, _) = scope.declared_type(&symbol.qname)?;
        let (extends, implements) = supertype_names(stmt);
        let (bases, resolved_bases) = paired(&extends, &links.extends);
        let (ifaces, resolved_ifaces) = paired(&implements, &links.implements);
        bases
            .iter()
            .zip(resolved_bases)
            .chain(ifaces.iter().zip(resolved_ifaces))
            .find(|(name, _)| covers(**name, offset))
            .map(|(name, qname)| (Target::Type(Cow::Borrowed(qname)), *name))
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
        ExprInfo::New { class, .. } => Target::Type(Cow::Borrowed(class)),
        // The bound, which is the only class this site named — the one
        // allocated is whatever descriptor is in hand, and no compile
        // knows it (`nvs_types::ExprInfo::NewDynamic`).
        ExprInfo::NewDynamic { bound, .. } => Target::Type(Cow::Borrowed(bound)),
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
        // A `Core\Router` link to a declared route is recorded over its call,
        // and still names the method it calls.
        ExprInfo::RouteLink { call, .. } | ExprInfo::RouteIs { call, .. } => Target::Method(call),
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
        ExprInfo::ClassConst { class, name, .. } | ExprInfo::ClassConstLate { class, name, .. } => {
            Target::Constant { class, name }
        }
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
        return Some((Target::Type(Cow::Borrowed(owner)), written.span));
    }
    let name = text_of(file, *member);
    let target = if analysed.module.aliases.get_member(owner, name).is_some() {
        Target::TypeAlias { class: owner, name }
    } else if let Some(owned) = nvs_hir::owned_enum(owner, name, &analysed.module.symbols) {
        Target::Type(Cow::Owned(owned))
    } else {
        Target::Constant { class: owner, name }
    };
    Some((target, *member))
}

/// A string literal whose text the compiler or a `Core` member reads as a
/// class's whole name, which the cursor is inside.
pub(crate) struct WrittenClassName {
    /// The literal, quotes included.
    pub span: Span,
    /// Which classes the name may be.
    pub bound: ClassBound,
}

/// Which classes a [`WrittenClassName`] may name.
pub(crate) enum ClassBound {
    /// The operand of `as class<T>`: a `T`, as the checker resolved it.
    Type(QName),
    /// A class-name parameter, which takes any class.
    Any,
    /// A class-name parameter that names an error to expect: a `Throwable`.
    Thrown,
}

/// The class-name literal the cursor at `offset` is inside, if any.
///
/// Two literals are one. `rule:types/class-reference`'s string row is the
/// first: `'App\User' as class<Model>`. The index has the literal and the
/// conversion as nodes and the target type as neither, so the type is found
/// the way [`written_type_at`] finds one — the type the conversion wrote,
/// which ends where the conversion ends — and its `T` is read off the type the
/// checker recorded for it. The second is an argument at a class-name
/// parameter, `Core\Reflect::forClass('App\User')`, which
/// [`crate::arguments`] finds. The cursor has to be between the quotes.
pub(crate) fn written_class_name(analysed: &Analysed, offset: BytePos) -> Option<WrittenClassName> {
    let path = analysed.index.at(offset);
    let [string, conversion, ..] = path.nodes() else {
        return None;
    };
    if string.kind != "Str" || !(string.span.start < offset && offset < string.span.end) {
        return None;
    }
    if conversion.kind != "Conversion" {
        let argument = crate::arguments::at(analysed, offset)
            .filter(|argument| argument.text == nvs_stdlib::registry::ParamText::ClassName)?;
        return Some(WrittenClassName {
            span: argument.span,
            bound: if argument.thrown {
                ClassBound::Thrown
            } else {
                ClassBound::Any
            },
        });
    }
    let entry = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)?;
    let end = conversion.span.end;
    let ty = written_types_in(&entry.stmts, end, end)
        .into_iter()
        .find(|ty| ty.span.end == end && ty.span.start >= string.span.end)?;
    let base = class_ref_base(&analysed.interner, analysed.exprs.declared_ty(ty.span)?)?;
    Some(WrittenClassName {
        span: string.span,
        bound: ClassBound::Type(base.clone()),
    })
}

/// A string literal whose text is a `property<T>` key or a `propertyPath<T>`
/// path.
pub(crate) struct WrittenKey {
    /// The literal, quotes included.
    pub span: Span,
    /// The class `T` names, where the key or the path starts.
    pub class: QName,
    /// Whether the text is a dotted path rather than one name.
    pub path: bool,
}

/// The key or path literal the cursor at `offset` is inside: the operand of
/// an `as` conversion ([`written_property_key`]), or an argument at a
/// parameter where the literal compiles as one
/// (`crate::arguments::Named::key_classes`). An argument whose parameter
/// takes more than one key type is read as the first one.
pub(crate) fn written_key(analysed: &Analysed, offset: BytePos) -> Option<WrittenKey> {
    if let Some(written) = written_property_key(analysed, offset) {
        return Some(written);
    }
    let named = crate::arguments::named_at(analysed, offset)?;
    let (class, path) = match (named.path_classes.first(), named.key_classes.first()) {
        (Some(class), _) => (class, true),
        (None, Some(class)) => (class, false),
        (None, None) => return None,
    };
    Some(WrittenKey {
        span: named.span,
        class: QName::parse(class),
        path,
    })
}

/// One segment of a key or path literal: the property it names, the span it
/// is written at, and the property's declared type.
pub(crate) struct PathSegment<'a> {
    /// The property, on the class the path reached at this segment.
    pub target: Target<'a>,
    /// The segment's own text in the literal.
    pub span: Span,
    /// The property's declared type.
    pub ty: TypeId,
}

/// The segment of a key or path literal the cursor at `offset` is on, stepped
/// to over the rosters the checker recorded (`rule:types/property-path`).
///
/// Each segment before it is looked up on the class the path has reached, and
/// [`nvs_types::path_step_class`] gives the next class, the step the checker
/// takes. `None` where a segment before the cursor names nothing, and for a
/// literal with an escape in it, whose text does not map to its source byte for
/// byte. The property is named on the class the path reached, so one inherited
/// from a parent has no declaration to jump to.
pub(crate) fn property_segment_at(analysed: &Analysed, offset: BytePos) -> Option<PathSegment<'_>> {
    let written = written_key(analysed, offset)?;
    let file = analysed.map.file(analysed.entry);
    let raw = text_of(file, written.span);
    let quote = raw.chars().next()?;
    let text = raw.get(1..)?;
    let text = text.strip_suffix(quote).unwrap_or(text);
    if text.contains('\\') {
        return None;
    }
    let at = usize::try_from(offset - written.span.start - 1).ok()?;
    let mut class = analysed.module.symbols.get(&written.class)?.qname.clone();
    let mut start = 0;
    let segments: Vec<&str> = if written.path {
        text.split('.').collect()
    } else {
        vec![text]
    };
    for segment in segments {
        let end = start + segment.len();
        let (name, ty) = analysed
            .exprs
            .property_keys(&class.to_string())
            .iter()
            .find(|(name, _)| name == segment)?;
        if at <= end {
            let symbol = analysed.module.symbols.get(&class)?;
            let begin = written.span.start + 1 + BytePos::try_from(start).ok()?;
            return Some(PathSegment {
                target: Target::Property {
                    class: &symbol.qname,
                    name,
                },
                span: Span {
                    file: written.span.file,
                    start: begin,
                    end: begin + BytePos::try_from(segment.len()).ok()?,
                },
                ty: *ty,
            });
        }
        class = nvs_types::path_step_class(*ty, &analysed.interner)?;
        start = end + 1;
    }
    None
}

/// The operand of an `as` conversion to `property<T>` or `propertyPath<T>`,
/// either nullable, the cursor at `offset` is inside, quotes included, and the
/// class `T` names.
///
/// `rule:types/property-key`'s conversion, found the way
/// [`written_class_name`] finds `as class<T>`'s: the type the conversion wrote
/// ends where the conversion ends, and its `T` is read off the type the
/// checker recorded for it. The cursor has to be between the quotes.
pub(crate) fn written_property_key(analysed: &Analysed, offset: BytePos) -> Option<WrittenKey> {
    let (span, declared) = conversion_operand(analysed, offset)?;
    let interner = &analysed.interner;
    let mut keys = match interner.get(declared) {
        Ty::Union(members) => members.clone(),
        _ => vec![declared],
    };
    keys.retain(|member| !matches!(interner.get(*member), Ty::Null));
    let [key] = keys.as_slice() else {
        return None;
    };
    let (argument, path) = match interner.get(*key) {
        Ty::PropertyKey(argument) => (argument, false),
        Ty::PropertyPath(argument) => (argument, true),
        _ => return None,
    };
    match interner.get(*argument) {
        Ty::Class(qname, _) => Some(WrittenKey {
            span,
            class: qname.clone(),
            path,
        }),
        _ => None,
    }
}

/// The operand of an `as string<K>` conversion, either nullable, the cursor at
/// `offset` is inside, quotes included, and what each of its holes may hold
/// (`rule:types/string-holes`). Found the way [`written_property_key`] finds
/// its operand.
pub(crate) fn written_string_holes(
    analysed: &Analysed,
    offset: BytePos,
) -> Option<(Span, nvs_types::HoleKind)> {
    let (span, declared) = conversion_operand(analysed, offset)?;
    Some((span, crate::arguments::hole_kind(analysed, declared)?))
}

/// The string literal operand of the `as` conversion the cursor at `offset` is
/// inside, quotes included, and the type the checker recorded for the type the
/// conversion wrote. The cursor has to be between the quotes.
fn conversion_operand(analysed: &Analysed, offset: BytePos) -> Option<(Span, TypeId)> {
    let path = analysed.index.at(offset);
    let [string, conversion, ..] = path.nodes() else {
        return None;
    };
    if string.kind != "Str"
        || conversion.kind != "Conversion"
        || !(string.span.start < offset && offset < string.span.end)
    {
        return None;
    }
    let entry = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)?;
    let end = conversion.span.end;
    let ty = written_types_in(&entry.stmts, end, end)
        .into_iter()
        .find(|ty| ty.span.end == end && ty.span.start >= string.span.end)?;
    Some((string.span, analysed.exprs.declared_ty(ty.span)?))
}

/// The `T` of a `class<T>` or a `?class<T>`, and `None` for any other type.
fn class_ref_base(interner: &TypeInterner, ty: TypeId) -> Option<&QName> {
    let ty = match interner.get(ty) {
        Ty::Union(members) => {
            let mut named = members
                .iter()
                .filter(|member| !matches!(interner.get(**member), Ty::Null));
            let only = *named.next()?;
            named.next().is_none().then_some(only)?
        }
        _ => ty,
    };
    let Ty::ClassRef(argument) = interner.get(ty) else {
        return None;
    };
    match interner.get(*argument) {
        Ty::Class(qname, _) => Some(qname),
        _ => None,
    }
}

/// The class a written class name at `offset` names, and the string's
/// span.
///
/// The text is compared with each class's whole name, exactly: the run-time
/// conversion compares the string with the names of the classes the program
/// declares, and resolves no `use` and no namespace, so `'User'` under `use
/// App\User;` and `'\App\User'` name no class there and none here. A literal
/// naming a class outside `T` still names that class, so the jump reaches it
/// and the reader sees why the conversion fails.
pub(crate) fn written_class_name_at(
    analysed: &Analysed,
    offset: BytePos,
) -> Option<(Target<'_>, Span)> {
    let literal = written_class_name(analysed, offset)?;
    let file = analysed.map.file(analysed.entry);
    let raw = text_of(file, literal.span);
    if !(raw.starts_with('\'') || raw.starts_with('"')) {
        return None;
    }
    let text = nvs_syntax::string_lit::cook_string_literal(file, literal.span);
    if text.is_empty() || text.starts_with('\\') {
        return None;
    }
    let symbol = analysed.module.symbols.get(&QName::parse(&text))?;
    Some((Target::Type(Cow::Borrowed(&symbol.qname)), literal.span))
}

/// What a plain class name written in type position at `offset` names, and
/// the span the source wrote it at.
///
/// `User` in `User $u`, in `array<User>`, in `): User`, in `foreach ($rows as
/// User $row)`, in `$v as User` and in `implementing<User>()` — every
/// position [`written_type_at`] walks. The name goes through the one resolver
/// ([`resolved_name`]) and is answered as the declaration it names. A name the
/// symbol table does not hold is answered as its own spelling, owned, so that
/// a `Core` class — declared in no source file — still reaches the stub tree
/// [`declared`] asks second; a name that is neither answers a jump to nowhere
/// there, which is what an unresolved name gets everywhere in this module.
///
/// A scalar is no name: `string` is its own [`TypeAtom`] and never a
/// [`TypeAtom::Name`], so `string $s` answers nothing here. The cursor has to
/// be on the name itself and not on an argument of it — in `Core\ObjectSet<Tag>`
/// the innermost written type covering an offset in `Tag` is `Tag`, and an
/// offset in the owner's segments is covered by the whole, whose own name span
/// is what is tested.
pub(crate) fn type_name_at(analysed: &Analysed, offset: BytePos) -> Option<(Target<'_>, Span)> {
    let ty = written_type_at(analysed, offset)?;
    let TypeKind::Atom(TypeAtom::Name(name, _)) = &ty.kind else {
        return None;
    };
    if !covers(name.span, offset) {
        return None;
    }
    if let Some((class, alias)) = FileScope::of(analysed, analysed.entry).owned_alias(name.span) {
        return Some((Target::TypeAlias { class, name: alias }, name.span));
    }
    let resolved = written_name_in(analysed, analysed.entry, name.span)?;
    let target = match analysed.module.symbols.get(&resolved) {
        Some(symbol) => Target::Type(Cow::Borrowed(&symbol.qname)),
        None => Target::Type(Cow::Owned(resolved)),
    };
    Some((target, name.span))
}

/// What the type name written at `name` in `file` resolves to, by the same
/// reading [`type_name_at`] gives a cursor, and `None` for an empty spelling.
///
/// A bare name inside a class's body is that class's own enum first
/// (`rule:enums/class-scoped-enum`), and every other name goes through
/// `nvs_hir::resolve_ref` with the namespace and the imports of `file`. The
/// index reads every file of a graph with it, so a type annotation is
/// recorded against the name a cursor on it would ask about.
pub(crate) fn written_name_in(analysed: &Analysed, file: SourceId, name: Span) -> Option<QName> {
    FileScope::of(analysed, file).written_name(name)
}

/// What the name `text`, written at `at` in the entry document, means — and
/// `None` for an empty spelling, which is a cursor that has written no name at
/// all rather than one that named the root.
pub(crate) fn resolved_name(analysed: &Analysed, text: &str, at: BytePos) -> Option<QName> {
    resolved_name_in(analysed, analysed.entry, text, at)
}

/// What the name `text`, written at `at` in `file`, means.
///
/// `nvs_hir::resolve_ref` is the whole of the decision
/// (`rule:statements/one-function-resolves-every-name`); what is added here is
/// the two things it takes and a cursor answer has to find for itself, which
/// are [`namespace_in`] and [`imports_in`].
pub(crate) fn resolved_name_in(
    analysed: &Analysed,
    file: SourceId,
    text: &str,
    at: BytePos,
) -> Option<QName> {
    FileScope::of(analysed, file).resolved_name(text, at)
}

/// The namespace `offset` is written inside in the entry document, as its
/// segments.
pub(crate) fn namespace_at(analysed: &Analysed, offset: BytePos) -> Vec<String> {
    namespace_in(analysed, analysed.entry, offset)
}

/// The namespace `offset` is written inside in `file`, as its segments.
///
/// Walked off the file's own statements because `nvs-hir` applies a
/// namespace as it collects declarations and keeps no map from a position back
/// to one. The two forms are the ones its resolver reads: `namespace Name;`
/// governs the rest of the file, and the bracketed form governs its block.
fn namespace_in(analysed: &Analysed, file: SourceId, offset: BytePos) -> Vec<String> {
    FileScope::of(analysed, file).namespace_at(offset).to_vec()
}

/// The class, interface or enum whose declaration `offset` is written inside,
/// by its fully-qualified name, and nothing outside every one of them.
///
/// Walked off the entry document's statements, as [`namespace_at`] is. A
/// bracketed namespace body is descended into, because its declarations are
/// written one level in.
pub(crate) fn class_at(analysed: &Analysed, offset: BytePos) -> Option<QName> {
    class_in(analysed, analysed.entry, offset)
}

/// [`class_at`] in any file of the graph.
fn class_in(analysed: &Analysed, file: SourceId, offset: BytePos) -> Option<QName> {
    FileScope::of(analysed, file).class_at(offset)
}

/// The name span of the class, interface or enum in `stmts` whose span covers
/// `offset`.
///
/// Sibling statements are written one after another and never overlap, so
/// the one that can cover `offset` is the last that starts at or before it,
/// found by a binary search.
fn class_name_at(stmts: &[Stmt], offset: BytePos) -> Option<Span> {
    let before = stmts.partition_point(|stmt| stmt.span.start <= offset);
    let stmt = &stmts[before.checked_sub(1)?];
    if offset >= stmt.span.end {
        return None;
    }
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => Some(decl.name.span),
        StmtKind::InterfaceDecl(decl) => Some(decl.name.span),
        StmtKind::EnumDecl(decl) => Some(decl.name.span),
        StmtKind::NamespaceDecl(decl) => class_name_at(&decl.body.as_ref()?.stmts, offset),
        _ => None,
    }
}

/// The entry document's own imports, in the shape `nvs_hir::resolve_ref` reads.
///
/// The whole graph's imports travel in one list, and a `use` belongs to the
/// file that wrote it — a required file's import must not resolve a name
/// written here.
pub(crate) fn imports_of(analysed: &Analysed) -> FxHashMap<String, QName> {
    imports_in(analysed, analysed.entry)
}

/// [`imports_of`] for any file of the graph.
fn imports_in(analysed: &Analysed, file: SourceId) -> FxHashMap<String, QName> {
    analysed
        .module
        .imports
        .iter()
        .filter(|import| import.span.file == file)
        .map(|import| (import.short_name.clone(), import.target.clone()))
        .collect()
}

/// One file of an analysis, read once, so each name written in it is answered
/// without walking the file again.
///
/// The index resolves every type name and every `X::` qualifier a file writes,
/// and finds the declaration of every type it declares. Done one name at a
/// time, each answer walked the file's statements and filtered the graph's
/// imports again, which made indexing a file quadratic in its size. A scope
/// reads the namespaces and the imports once, and the declarations the first
/// time one is asked for. Each answer is then a binary search or a hash lookup.
///
/// The one-name functions above build a scope for their one answer, so a
/// cursor and the index read a name the same way.
///
/// # What it spends
///
/// One map of the file's imports, one entry per `namespace` statement, and,
/// once a declaration is asked for, one entry per type the file declares. All
/// of it is dropped with the scope.
pub(crate) struct FileScope<'a> {
    analysed: &'a Analysed,
    file: SourceId,
    stmts: &'a [Stmt],
    /// Each bracketed namespace's block and its segments, in source order.
    bracketed: Vec<(Span, Vec<String>)>,
    /// Where each `namespace Name;` statement ends and its segments, in
    /// source order.
    stated: Vec<(BytePos, Vec<String>)>,
    imports: FxHashMap<String, QName>,
    declared: OnceCell<FxHashMap<Span, &'a Stmt>>,
}

impl<'a> FileScope<'a> {
    /// The scope of `file`, which is empty for a file the analysis did not
    /// load.
    pub(crate) fn of(analysed: &'a Analysed, file: SourceId) -> Self {
        let stmts = analysed
            .loaded
            .iter()
            .find(|loaded| loaded.id == file)
            .map_or(&[][..], |loaded| &loaded.stmts[..]);
        let source = analysed.map.file(file);
        let mut bracketed = Vec::new();
        let mut stated = Vec::new();
        for stmt in stmts {
            let StmtKind::NamespaceDecl(decl) = &stmt.kind else {
                continue;
            };
            let segments = decl.name.as_ref().map_or_else(Vec::new, |name| {
                QName::parse(text_of(source, name.span)).segments().to_vec()
            });
            match &decl.body {
                Some(block) => bracketed.push((block.span, segments)),
                None => stated.push((stmt.span.end, segments)),
            }
        }
        Self {
            analysed,
            file,
            stmts,
            bracketed,
            stated,
            imports: imports_in(analysed, file),
            declared: OnceCell::new(),
        }
    }

    /// The namespace `offset` is written inside, as its segments.
    ///
    /// A bracketed namespace governs its block, and a `namespace Name;`
    /// statement governs the rest of the file after it, until the next one.
    pub(crate) fn namespace_at(&self, offset: BytePos) -> &[String] {
        let open = self
            .bracketed
            .partition_point(|(block, _)| block.start <= offset);
        if let Some((block, segments)) = open.checked_sub(1).map(|at| &self.bracketed[at])
            && offset < block.end
        {
            return segments;
        }
        let ended = self.stated.partition_point(|(end, _)| *end <= offset);
        ended
            .checked_sub(1)
            .map_or(&[][..], |at| &self.stated[at].1[..])
    }

    /// The class, interface or enum whose declaration `offset` is written
    /// inside, by its fully-qualified name.
    pub(crate) fn class_at(&self, offset: BytePos) -> Option<QName> {
        let name = class_name_at(self.stmts, offset)?;
        Some(QName::join(
            self.namespace_at(offset),
            text_of(self.analysed.map.file(self.file), name),
        ))
    }

    /// [`written_name_in`] for a name this file writes.
    pub(crate) fn written_name(&self, name: Span) -> Option<QName> {
        let text = text_of(self.analysed.map.file(self.file), name);
        let owned = self.class_at(name.start).and_then(|owner| {
            nvs_hir::owned_enum_in_scope(text, Some(&owner), &self.analysed.module.symbols)
        });
        owned.or_else(|| self.resolved_name(text, name.start))
    }

    /// The owner and the name of the alias a bare name this file writes at
    /// `name` reads, when the class, interface or enum around it declares an
    /// alias of that name (`rule:types/class-scoped-alias`). The owner's alias
    /// comes before every other reading of a bare name inside its body.
    pub(crate) fn owned_alias(&self, name: Span) -> Option<(&'a QName, &'a str)> {
        let text = text_of(self.analysed.map.file(self.file), name);
        if text.is_empty() || text.contains('\\') {
            return None;
        }
        let owner = self.class_at(name.start)?;
        let owner = &self.analysed.module.symbols.get(&owner)?.qname;
        self.analysed
            .module
            .aliases
            .get_member(owner, text)
            .map(|_| (owner, text))
    }

    /// [`resolved_name_in`] for a name this file writes.
    pub(crate) fn resolved_name(&self, text: &str, at: BytePos) -> Option<QName> {
        if text.is_empty() {
            return None;
        }
        Some(nvs_hir::resolve_ref(
            text,
            self.namespace_at(at),
            &self.imports,
        ))
    }

    /// [`declared_type`], found by a hash lookup when this file declares
    /// `class`.
    pub(crate) fn declared_type(&self, class: &QName) -> Option<(&'a Stmt, &'a SourceFile)> {
        let symbol = self.analysed.module.symbols.get(class)?;
        if symbol.decl_span.file != self.file {
            return declared_type(self.analysed, class);
        }
        let declared = self.declared.get_or_init(|| {
            let mut found = FxHashMap::default();
            declarations_into(self.stmts, &mut found);
            found
        });
        let stmt = declared.get(&symbol.decl_span)?;
        Some((stmt, self.analysed.map.file(self.file)))
    }

    /// [`owned_enum_decl`], with its owner found by [`Self::declared_type`].
    pub(crate) fn owned_enum_decl(&self, class: &QName) -> Option<(&'a ClassMember, &'a EnumDecl)> {
        let (owner, _) = class.as_member()?;
        let symbol = self.analysed.module.symbols.get(class)?;
        let (stmt, _) = self.declared_type(&owner)?;
        let (_, members) = body_of(stmt)?;
        members.iter().find_map(|member| match &member.kind {
            ClassMemberKind::Enum(decl) if decl.name.span == symbol.decl_span => {
                Some((member, decl))
            }
            _ => None,
        })
    }
}

/// The written type the cursor at `offset` is inside, innermost first, and
/// `None` for a cursor inside none.
///
/// The positions walked are the module doc's: a declaration's own and an
/// expression's, anywhere in the entry document. Innermost for [`named_at`]'s
/// reason — `array<Order::Meta>` covers one offset with two written types,
/// and the nearer one is what the caret is pointing at.
pub(crate) fn written_type_at(analysed: &Analysed, offset: BytePos) -> Option<&Type> {
    let entry = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)?;
    written_types_in(&entry.stmts, offset, offset)
        .into_iter()
        .find_map(|root| type_in(root, offset))
}

/// Every name written inside `ty`, as the span each was written at: the leaf
/// a `use` line can make resolve, with the arguments of a generic name and the
/// members of a shape walked for theirs. A reserved word is no name and is
/// skipped, and so is a member of a type written as `Owner::Member`, whose
/// owner is what a cursor on it asks about.
pub(crate) fn type_names(ty: &Type, found: &mut Vec<Span>) {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => type_names(inner, found),
        TypeKind::Union(members) | TypeKind::Intersection(members) => {
            for member in members {
                type_names(member, found);
            }
        }
        TypeKind::Atom(atom) => match atom {
            TypeAtom::Array(Some(arg))
            | TypeAtom::ClassRef(arg)
            | TypeAtom::PropertyKey(arg)
            | TypeAtom::PropertyPath(arg)
            | TypeAtom::StringHoles { key: arg, .. } => {
                type_names(arg, found);
            }
            TypeAtom::Shape(fields) => {
                for field in fields {
                    type_names(&field.ty, found);
                }
            }
            TypeAtom::CallableSig { params, ret } => {
                for param in params {
                    type_names(param, found);
                }
                type_names(ret, found);
            }
            TypeAtom::Name(name, args) => {
                found.push(name.span);
                for arg in args {
                    type_names(arg, found);
                }
            }
            _ => {}
        },
        _ => {}
    }
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
        TypeAtom::Array(Some(arg))
        | TypeAtom::ClassRef(arg)
        | TypeAtom::PropertyKey(arg)
        | TypeAtom::PropertyPath(arg)
        | TypeAtom::StringHoles { key: arg, .. } => type_in(arg, offset),
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
        let dir = nvs_repo::scratch("lsp-definition");
        let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
        let mut documents = Documents::new();
        documents.open(uri.clone(), 1, source.to_owned());
        let analysed = analyse(&documents, &uri).expect("an open document analyses");
        let found = source.find(written).expect("the document writes it") + written.len();
        let offset = u32::try_from(found).expect("a test document is short");
        at(
            &analysed,
            &CompletionFiles::default(),
            offset,
            PositionEncoding::Utf8,
        )
        .map_or_else(
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
    /// (`rule:types/one-type-test`) is also one navigation, and the
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

    /// One interface, written at every type position a program has: a
    /// parameter, a return, a typed local's element, a call's `<Type>`
    /// argument, a `foreach` binding, an `as` conversion and an anonymous
    /// function's signature.
    const WRITTEN: &str = "<?nvs\ninterface Shape { public function area(): int; }\n\
                           class Square implements Shape { public function area(): int { return \
                           1; } }\nclass Box {\n  public function of(Shape $s): Shape { return \
                           $s; }\n}\narray<Shape> $all = Core\\Program::implementing<Shape>();\n\
                           foreach ($all as Shape $one) { echo $one->area(); }\nmixed $m = new \
                           Square();\nvar $cast = $m as Shape;\nvar $f = fn(Shape $p): Shape => \
                           $p;\n";

    /// A class name in type position is a written name like the one after
    /// `is`, so a cursor on it opens the declaration wherever the type was
    /// written — and the one inside a call's `<Type>` argument opens the class,
    /// not the method the index would have answered with.
    #[test]
    fn definition_on_a_class_written_in_type_position_is_its_declaration() {
        assert_eq!(jump(WRITTEN, "of(Sha"), "1:10");
        assert_eq!(jump(WRITTEN, "$s): Sha"), "1:10");
        assert_eq!(jump(WRITTEN, "array<Sha"), "1:10");
        assert_eq!(jump(WRITTEN, "implementing<Sha"), "1:10");
        assert_eq!(jump(WRITTEN, "($all as Sha"), "1:10");
        assert_eq!(jump(WRITTEN, "$m as Sha"), "1:10");
        assert_eq!(jump(WRITTEN, "fn(Sha"), "1:10");
        assert_eq!(jump(WRITTEN, "$p): Sha"), "1:10");
    }

    /// `Core\Program::constructors<T, C>()` writes a class in both of its type
    /// arguments: `T` itself, and the parameter and return of the
    /// `callable(...)` that `C` is. Each opens its own declaration.
    #[test]
    fn a_constructors_type_argument_jumps_to_its_class() {
        const MAKERS: &str = "<?nvs\ninterface Shape { public function area(): int; }\n\
                              class Config { public int $side = 1; }\n\
                              class Square implements Shape {\n  public function \
                              constructor(public Config $config) {}\n  public function area(): \
                              int { return 1; }\n}\n\
                              var $rows = Core\\Program::constructors<Shape, callable(Config): \
                              Shape>();\n";
        assert_eq!(jump(MAKERS, "constructors<Sha"), "1:10");
        assert_eq!(jump(MAKERS, "callable(Con"), "2:6");
        assert_eq!(jump(MAKERS, "callable(Config): Sha"), "1:10");
    }
}
