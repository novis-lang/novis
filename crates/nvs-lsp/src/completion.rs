//! What may be written where the cursor is.
//!
//! `textDocument/completion` answers four things in
//! `rule:ide/the-request-set-is-closed`'s list: the members reachable off a
//! receiver whose class the analysis resolved, the static members and
//! constants reached through a class name, the cases of an enum written after
//! `Type::` — a user-declared class and a `Core` one alike — and what a bare
//! position offers, which is the keywords that may be written there, the
//! variables in scope, the types a bare name reaches and the PHP built-ins a
//! half-written one matches, and is the same walk asked at a node that is no
//! access at all.
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
//! **A cursor at the very end of an access is still in its member half.** Where
//! the token after the arrow closes something — a `}`, a `)`, a `]` — or the
//! file simply stops, the parser has no token left to make a member name out
//! of, so the access ends at the arrow and
//! `nvs_syntax::SyntaxIndex::at`'s half-open containment puts the cursor
//! outside every node of it. Such an access is reached by asking the index
//! about the byte *before* the cursor and keeping the answer only where that
//! access ends exactly at the cursor, which is that shape and no other. The
//! containment stays half-open, because a cursor being outside the node it
//! touches is what `selectionRange` is frozen on.
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
//! # What a bare cursor is offered
//!
//! A cursor in no access is a *position*, and a file has three of them because
//! it is made of three kinds of body: one of statements — a method's, a
//! function's, or the file's own script frame — one of members, which is a
//! class or an interface, and an enum's, which takes cases and nothing else
//! (`rule:enums/no-class-machinery`). Each has its own list of words, read out
//! of the grammar's own dispatch over the first token of one, and only the
//! first has a variable in it.
//!
//! The node the index answers with is what tells the three apart, and it has
//! to be something like this, because the script frame's span is the *whole
//! file*: a cursor between two members of a class sits inside that frame too,
//! so "the innermost body covering the offset" would offer a class body the
//! file's own variables, which are names no member can reach
//! (`rule:statements/storage-that-outlives-a-call` — the script body is a
//! function, and its locals are locals). "The innermost node is a body" is not
//! the test either: a half-written `$na` is a `Variable` node of its own, so
//! the question is asked the other way round — which body the node the cursor
//! is *directly* inside belongs to.
//!
//! The variables offered are the innermost body's own and only its own. An
//! enclosing body's are not in scope: a closure captures by value
//! (`rule:types/closure-literal`) and a file-scope local is unreachable from a
//! function, so a name taken from the body outside would be one the checker
//! refuses where it was offered.
//!
//! A statement position also offers the **types a bare name could reach**,
//! beside those words rather than instead of them: the `use` declarations in
//! force in the entry document, and every type the workspace index holds. Each
//! is labelled with the shortest spelling that resolves at that cursor — an
//! import's own short name, a last segment for a declaration in the namespace
//! in force, and the qualified name for everything else — which is
//! `nvs_hir::resolve_ref`'s order read backwards. It is admitted here and not
//! at M4B for one reason, and it is ADR 0099 § 3's: at M4B it would have been
//! a workspace symbol search with no index under it.
//!
//! # What a half-written name reaches in PHP's inventory
//!
//! A statement position also offers the **PHP built-ins** whose spelling starts
//! with the name being written, from
//! `rule:php-migration/every-php-builtin-is-a-completion-candidate`'s two
//! audited documents joined at build time into
//! `nvs_stdlib::php_names::CANDIDATES`. A name with no migration row is an item
//! that says *undecided*, which reads as an audited language that owes an
//! answer where a name that never appears reads as one that cannot do the job.
//!
//! **What may be typed on the developer's behalf is narrower than what is
//! offered**, and the two are not the same question:
//! `nvs_stdlib::php_names::Item::insertion` answers only for a destination the
//! registry holds, and three of its four shapes insert nothing at all
//! (`rule:ide/three-of-four-item-shapes-insert-nothing`). The PHP spelling
//! itself never reaches a file — it is a label and a filter, and
//! [`php_item`]'s `insert_text` is what keeps it one.
//!
//! **The inventory answers a name and not a cursor.** [`PHP_PREFIX`] characters
//! are written first, because a whole language's built-ins arriving beside the
//! keyword list on every keystroke is a list about the alphabet rather than
//! about this program.
//!
//! # What a namespace separator offers
//!
//! A name with a separator in it is read from the root
//! (`rule:statements/a-qualified-name-is-absolute`), so `Core\` is asking what
//! that namespace holds. The answer is the two rosters the member arms already
//! read — `nvs_stdlib::registry`'s classes and its enums — beside the
//! declarations [`crate::index`] holds for the workspace, each filtered to the
//! names that lie under the prefix. **No keyword is among them**, and that is
//! the half of this arm which closes a defect rather than adding a feature:
//! `\` is a declared trigger character, so before the arm existed a cursor
//! after `Core\` was answered the words that open a statement, at the one
//! place the parser refuses every one of them.
//!
//! Each item is labelled with the **rest** of the name and not its last
//! segment. What a client replaces is the word it is completing, so `App\`
//! offering `Models\User` writes a name that resolves where `User` alone would
//! write one that does not.
//!
//! **The prefix is read off the source, because no node spans it.** `Core\St`
//! is one `ConstFetch` covering both segments, while `Core\` with nothing
//! after it is two statements — the name, and an expression covering the lone
//! `\` that `E0240` reports — so there is no single production to ask. What is
//! read is a name's own spelling, whose meaning `nvs_hir::QName` owns; the
//! tree is still what says the cursor is in code at all, which is [`NOT_CODE`].
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
//! member half is still empty offers nothing after them. The fourth is the
//! receiver half of an access, which is a position and is answered as though
//! it were not: `$u<|>->name` is a variable being written, and offering the
//! variables in scope there is right and is not done, because the walk decides
//! it is in an access before it asks what half of one. The fifth is the
//! parameter list and the return type of a method, which are inside its own
//! node and no statement's, so a cursor there is answered as the body it
//! precedes rather than as the type position it is. The sixth is the inside of
//! a string literal, answered as the position around it: the variables are
//! what an interpolation slot takes and are right, and the words that open a
//! statement sit beside them as noise no filter here removes. The seventh is
//! how far a namespace reaches: the declarations offered under a prefix are
//! the ones the index holds, so under
//! `rule:ide/check-scope-defaults-to-open-documents`'s default a type in a
//! file nobody has opened is not among them. That setting is the answer, and
//! this arm deliberately has no second one — a directory walk of its own is
//! what `rule:ide/completion-offers-only-what-the-compiler-derived` refuses.
//! — owner: workspace-index

use std::collections::BTreeMap;

use lsp_types::{CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind};
use nvs_diagnostics::{BytePos, SourceFile, Span};
use nvs_hir::QName;
use nvs_stdlib::php_names::{self, Candidate, Item, Kind};
use nvs_stdlib::registry::{self, CoreClass, CoreConst, CoreEnum, CoreMethod};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, EnumCase, MethodMember, Modifier, PropertyMember, StmtKind,
};
use nvs_syntax::{IndexNode, NodePath};
use nvs_types::{ExprInfo, Ty, TypeId};
use rustc_hash::FxHashMap;

use crate::definition::{declared_type, text_of};
use crate::document::Analysed;
use crate::index::{DeclKind, SymbolIndex};
use crate::settings::PhpNames;

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
/// Empty for a cursor in an access this cannot resolve a receiver for, or in
/// the receiver half of one rather than the member half — which are one answer
/// for a client, since LSP has no shape for "ask me again somewhere else".
#[must_use]
pub fn at(
    analysed: &Analysed,
    symbols: &SymbolIndex,
    offset: BytePos,
    php: PhpNames,
) -> Vec<CompletionItem> {
    let path = analysed.index.at(offset);
    let mut items = match asked(analysed, &path, offset) {
        Asked::Member(class, reach) => members_of(analysed, &class, reach),
        Asked::Namespace(prefix) => under(symbols, &prefix),
        Asked::Position => position(analysed, symbols, &path, offset, php),
        Asked::Nothing => return Vec::new(),
    };
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

/// What the cursor is asking for, which is decided before anything is looked
/// up.
enum Asked {
    /// The member half of an access, off the class it resolved to and reaching
    /// the half of it the access shape names.
    Member(QName, Reach),
    /// A name reaching into the namespace these segments spell — what that
    /// namespace holds, and no word.
    Namespace(Vec<String>),
    /// No access at all — what may be written where a statement or an
    /// expression goes.
    Position,
    /// Nothing this module answers: a cursor in a run of markup, the receiver
    /// half of an access, or one whose receiver resolved to no class.
    Nothing,
}

/// Every member of `class` that `reach` reaches, whoever declared it.
fn members_of(analysed: &Analysed, class: &QName, reach: Reach) -> Vec<CompletionItem> {
    let name = class.to_string();
    if let Some(core) = registry::class(&name) {
        core_members(core, reach)
    } else if let Some(core) = registry::core_enum(&name) {
        core_cases(core, reach)
    } else {
        declared_members(analysed, class, reach)
    }
}

/// Which of the four questions the cursor at `offset` is asking.
///
/// The namespace question is asked first and answers on its own terms: a
/// separator has been written in the name the cursor is inside, which is true
/// in no access — a member name carries none — so the order costs the arms
/// below nothing and buys the receiver half of `Core\Str::` an answer it would
/// otherwise be refused for standing before the receiver's end.
fn asked(analysed: &Analysed, path: &NodePath, offset: BytePos) -> Asked {
    // A cursor inside a run of markup is not writing a program, and a keyword
    // list offered there inserts text the page would render rather than run.
    // The editor's own HTML service answers here instead, inside the region
    // `crate::regions` reports
    // (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`) —
    // which is what makes this an answer rather than a gap, and what separates
    // markup from the other three spellings in `NOT_CODE`: a cursor in a string
    // has nothing else to ask.
    if path.innermost().is_some_and(|node| node.kind == MARKUP) {
        return Asked::Nothing;
    }
    if let Some(prefix) = namespace_written(analysed, offset) {
        return Asked::Namespace(prefix);
    }
    let Some((access, reach)) = access_in(path).or_else(|| ended_at(analysed, offset)) else {
        return Asked::Position;
    };
    let Some(receiver) = analysed.index.children_of(access).into_iter().next() else {
        return Asked::Nothing;
    };
    // A cursor still inside the receiver is writing the receiver, and the
    // members of its own class are not what it is asking for.
    if offset < receiver.span.end {
        return Asked::Nothing;
    }
    let class = match reach {
        Reach::Instance => held_class(analysed, receiver.span, offset),
        Reach::Static => named_class(analysed, access, receiver.span),
    };
    class.map_or(Asked::Nothing, |class| Asked::Member(class, reach))
}

/// The innermost access `path` runs through, and the half of a class it
/// reaches.
fn access_in(path: &NodePath) -> Option<(IndexNode, Reach)> {
    path.nodes().iter().find_map(|node| {
        ACCESS
            .iter()
            .find(|(kind, _)| *kind == node.kind)
            .map(|(_, reach)| (*node, *reach))
    })
}

/// The access the cursor is standing at the end of, which is the one the
/// member half of is still empty because nothing followed the arrow.
///
/// The byte before the cursor is inside that access whenever the access ends
/// at the cursor, so one lookup finds it; the `end` test is what keeps a
/// cursor that has walked well past an access from reaching this.
fn ended_at(analysed: &Analysed, offset: BytePos) -> Option<(IndexNode, Reach)> {
    let before = analysed.index.at(offset.checked_sub(1)?);
    access_in(&before).filter(|(access, _)| access.span.end == offset)
}

/// The productions whose text is not code, so a separator written inside one
/// belongs to a string, a comment or a run of markup rather than to a name.
///
/// The list is what the module doc's *What a namespace separator offers* leans
/// on: the prefix is read off the source, and this is the tree's half of that —
/// the one question only the tree can answer is whether the cursor is writing
/// a program at all. `Whitespace` is `nvs_syntax::walk`'s node for a comment
/// too, which is why a comment needs no entry of its own.
const NOT_CODE: &[&str] = &["Str", "Interpolated", MARKUP, "Whitespace"];

/// `nvs_syntax::walk`'s name for a run of inline markup, which is the one
/// entry of [`NOT_CODE`] that another language service answers inside.
const MARKUP: &str = "InlineHtml";

/// The namespace the name being written at `offset` reaches into, as its
/// segments, or `None` where that name carries no separator.
///
/// The name is the run of name characters ending at the cursor. Reading it out
/// of the source rather than off a node is the module doc's decision and its
/// reasoning: `Core\` with nothing after it is two statements, so no one
/// production covers the prefix in both of the shapes a half-written qualified
/// name takes.
fn namespace_written(analysed: &Analysed, offset: BytePos) -> Option<Vec<String>> {
    let node = analysed.index.at(offset.checked_sub(1)?).innermost()?;
    if NOT_CODE.contains(&node.kind) {
        return None;
    }
    let upto = analysed
        .map
        .file(analysed.entry)
        .text()
        .get(..offset as usize)?;
    let start = upto
        .char_indices()
        .rev()
        .take_while(|(_, ch)| is_name(*ch))
        .last()
        .map_or(upto.len(), |(at, _)| at);
    let (prefix, _) = upto[start..].rsplit_once('\\')?;
    if prefix.is_empty() {
        return None;
    }
    Some(QName::parse(prefix).segments().to_vec())
}

/// Whether `ch` may appear in a qualified name.
///
/// The separator is one of them: what this bounds is the whole name the cursor
/// is writing, and `Core\St` is one name and not two.
fn is_name(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '\\'
}

/// Every type declared under `prefix`, labelled by the rest of its name.
///
/// Three rosters and no fourth: `nvs_stdlib::registry`'s classes and enums,
/// which are the same two [`members_of`] reads a member list off, and the
/// declarations the one workspace index holds — reached through
/// [`SymbolIndex::files`] and [`SymbolIndex::declarations_in`], which are the
/// queries it already answers `textDocument/codeLens` with. Nothing here walks
/// a directory or asks anything off this machine
/// (`rule:ide/completion-offers-only-what-the-compiler-derived`).
///
/// Keyed by label, so a name two files both declare is offered once rather
/// than twice.
fn under(symbols: &SymbolIndex, prefix: &[String]) -> Vec<CompletionItem> {
    let mut found: BTreeMap<String, CompletionItem> = BTreeMap::new();
    let core = registry::CLASSES
        .iter()
        .map(|class| (class.name, CompletionItemKind::CLASS))
        .chain(
            registry::ENUMS
                .iter()
                .map(|core| (core.name, CompletionItemKind::ENUM)),
        );
    for (name, kind) in core {
        if let Some(rest) = under_prefix(name, prefix) {
            found.insert(rest.clone(), item(rest, kind, name.to_owned()));
        }
    }
    for path in symbols.files() {
        for declared in symbols.declarations_in(path) {
            let Some(kind) = namespace_kind(declared.kind) else {
                continue;
            };
            if let Some(rest) = under_prefix(&declared.symbol, prefix) {
                found.insert(rest.clone(), item(rest, kind, declared.symbol.clone()));
            }
        }
    }
    found.into_values().collect()
}

/// The rest of `name` after `prefix`, or `None` for a name outside it.
///
/// Segment by segment rather than one string comparison, because `Core` is not
/// a prefix of `Coroutine` and a text one would say it is.
fn under_prefix(name: &str, prefix: &[String]) -> Option<String> {
    let mut rest = name;
    for segment in prefix {
        rest = rest.strip_prefix(segment.as_str())?.strip_prefix('\\')?;
    }
    (!rest.is_empty()).then(|| rest.to_owned())
}

/// The kind a declaration is offered under after a separator, and `None` for
/// one a separator does not reach.
///
/// A member is written after `->` or `::` and never after `\`, so the index's
/// member rows are not a narrower answer here — they are no answer at all. The
/// alias's spelling is `crate::server`'s, so the outline and this list call one
/// thing by one name.
fn namespace_kind(kind: DeclKind) -> Option<CompletionItemKind> {
    match kind {
        DeclKind::Class => Some(CompletionItemKind::CLASS),
        DeclKind::Interface => Some(CompletionItemKind::INTERFACE),
        DeclKind::Enum => Some(CompletionItemKind::ENUM),
        DeclKind::TypeAlias => Some(CompletionItemKind::TYPE_PARAMETER),
        DeclKind::Method | DeclKind::Property | DeclKind::Const | DeclKind::EnumCase => None,
    }
}

/// The words that may open a statement, whether as a statement form of their
/// own or as the expression one.
///
/// Read out of `nvs_syntax::parser`'s two dispatches — the one over a
/// statement's first token and the one over a primary expression's — minus
/// every arm either of them routes to a rejection, so no word here is one the
/// compiler answers with an `E02xx`. `else`, `elseif`, `case`, `catch` and
/// `finally` are absent for the other reason: they continue a construct rather
/// than open one, and the construct that takes them writes them itself.
const STATEMENT_WORDS: &[&str] = &[
    "abstract",
    "autoload",
    "break",
    "class",
    "clone",
    "continue",
    "do",
    "echo",
    "empty",
    "enum",
    "false",
    "final",
    "fn",
    "for",
    "foreach",
    "if",
    "interface",
    "isset",
    "match",
    "namespace",
    "new",
    "null",
    "parent",
    "print",
    "require",
    "return",
    "self",
    "static",
    "switch",
    "throw",
    "true",
    "try",
    "unset",
    "use",
    "var",
    "while",
    "yield",
];

/// The words that may open a member of a class or an interface — the modifiers
/// `nvs_syntax::parser`'s `parse_modifiers` takes, and the two introducers
/// after them.
///
/// `use` is not among them: it opens a trait member, and
/// `rule:classes/no-traits` is why there are no traits.
const MEMBER_WORDS: &[&str] = &[
    "abstract",
    "const",
    "final",
    "function",
    "lateinit",
    "private",
    "protected",
    "public",
    "readonly",
    "static",
];

/// The one word an enum body takes.
///
/// An enum declares its cases and nothing else
/// (`rule:enums/no-class-machinery`), so this is not the member list narrowed —
/// it is a different list, the same way `nvs_stdlib::registry::ENUMS` is a
/// roster beside `CLASSES` rather than a filter over it.
const CASE_WORDS: &[&str] = &["case"];

/// What may be written at a cursor that is in no access.
///
/// The node the cursor is *directly* inside decides, which the module doc's
/// *What a bare cursor is offered* is the reasoning for: the body that
/// **covers** the offset is the whole file for a class body too, so covering
/// cannot be the question. A member's own node — a property, a class constant,
/// an enum case — is a value position with no local in scope and no word of
/// its own, and answers nothing.
fn position(
    analysed: &Analysed,
    symbols: &SymbolIndex,
    path: &NodePath,
    offset: BytePos,
    php: PhpNames,
) -> Vec<CompletionItem> {
    match path.innermost().map(|node| node.kind) {
        Some("ClassDecl" | "InterfaceDecl") => words(MEMBER_WORDS),
        Some("EnumDecl") => words(CASE_WORDS),
        Some("Property" | "Const" | "EnumCase") => Vec::new(),
        _ => {
            let mut items = words(STATEMENT_WORDS);
            items.extend(in_scope(analysed, offset));
            items.extend(in_reach(analysed, symbols, offset));
            items.extend(php_builtins(analysed, offset, php));
            items
        }
    }
}

/// One list of reserved words, as items a client can insert.
///
/// No detail: a keyword has neither a type nor a signature, and the column
/// `crate::render` writes it into is empty rather than filled with a category
/// the label already is.
fn words(offered: &[&str]) -> Vec<CompletionItem> {
    offered
        .iter()
        .map(|word| {
            item(
                (*word).to_owned(),
                CompletionItemKind::KEYWORD,
                String::new(),
            )
        })
        .collect()
}

/// The variables the body the cursor is in declared, with the type each was
/// declared at.
fn in_scope(analysed: &Analysed, offset: BytePos) -> Vec<CompletionItem> {
    analysed
        .bodies_at(offset)
        .into_iter()
        .next()
        .unwrap_or_default()
        .iter()
        .map(|local| {
            item(
                format!("${}", local.name),
                CompletionItemKind::VARIABLE,
                analysed.interner.describe(local.ty),
            )
        })
        .collect()
}

/// Every type a bare name at `offset` could be, spelled the way it may be
/// written there.
///
/// One rule and three spellings of it: the shortest name that **resolves at
/// this cursor**. An import in force answers with its own short name, a
/// declaration in the namespace in force with its last segment, and everything
/// else with the qualified name it is reached by. That is
/// `nvs_hir::resolve_ref`'s own order read backwards, so no name offered here
/// is one the resolver would refuse where it was offered — the same bar
/// [`position`]'s keyword lists meet.
///
/// The two tables are the ones the compiler already keeps: the entry
/// document's `use` declarations, and the workspace index's declaration side.
/// A name the index does not hold is not searched for anywhere else.
fn in_reach(analysed: &Analysed, symbols: &SymbolIndex, offset: BytePos) -> Vec<CompletionItem> {
    let imports = imports_of(analysed);
    let short: FxHashMap<String, String> = imports
        .iter()
        .map(|(name, target)| (target.to_string(), name.clone()))
        .collect();
    let here = namespace_at(analysed, offset);
    let mut found: BTreeMap<String, CompletionItem> = BTreeMap::new();
    for path in symbols.files() {
        for declared in symbols.declarations_in(path) {
            let Some(kind) = namespace_kind(declared.kind) else {
                continue;
            };
            let label = written_as(&declared.symbol, &here, &short);
            found
                .entry(label.clone())
                .or_insert_with(|| item(label, kind, declared.symbol.clone()));
        }
    }
    // An import whose target the index does not hold is still a name in force
    // — a `Core` class, or one in a file the current scope does not reach
    // (`rule:ide/check-scope-defaults-to-open-documents`). What it declares is
    // read off the registry, which is the one table that can say.
    for (name, target) in &imports {
        let spelled = target.to_string();
        found.entry(name.clone()).or_insert_with(|| {
            let kind = if registry::core_enum(&spelled).is_some() {
                CompletionItemKind::ENUM
            } else {
                CompletionItemKind::CLASS
            };
            item(name.clone(), kind, spelled)
        });
    }
    found.into_values().collect()
}

/// How `symbol` is written at a cursor whose namespace is `here` and whose
/// imports are `short`, keyed the way they are looked up — target to the name
/// it was imported under.
fn written_as(symbol: &str, here: &[String], short: &FxHashMap<String, String>) -> String {
    if let Some(name) = short.get(symbol) {
        return name.clone();
    }
    let segments = QName::parse(symbol).segments().to_vec();
    if segments.len() == here.len() + 1 && segments.starts_with(here) {
        return segments[here.len()].clone();
    }
    symbol.to_owned()
}

/// How many characters of a name are written before the PHP inventory answers.
///
/// The inventory is every built-in the differential oracle's own PHP build
/// lists, and a one- or two-character prefix reaches hundreds of them — a list
/// that arrives on every keystroke and buries the words, the variables and the
/// types this program is actually made of. Three characters is where the answer
/// is about the name being written rather than about the alphabet, and a
/// developer reaching for a PHP built-in is by definition writing one. No name
/// is dropped from the inventory by this: every one of them is still a
/// candidate, which is what
/// `rule:php-migration/every-php-builtin-is-a-completion-candidate` requires,
/// and the prefix is only how much of it has to be written first.
const PHP_PREFIX: usize = 3;

/// Every PHP built-in whose spelling starts with the name being written at
/// `offset`, as the item shape the migration table's row for it takes.
///
/// The table is `nvs_stdlib::php_names::CANDIDATES`, joined at build time out
/// of the oracle inventory and `docs/spec/02-php-migration.md` and sorted by
/// PHP spelling, so what is read here is one contiguous run of it. Two audited
/// documents and the `Core` registry, and no fourth source — the name comes
/// from a table this repository maintains, exactly as
/// `rule:ide/completion-offers-only-what-the-compiler-derived` admits it.
///
/// `nvs.completion.phpNames` is read here and not at the call site because what
/// it selects is a *shape*: `Resolved` keeps the items that insert and drops the
/// three that do not, which is the lever
/// `rule:php-migration/an-item-inserts-only-a-registered-member` names for a
/// developer who wants only the names that go somewhere.
fn php_builtins(analysed: &Analysed, offset: BytePos, php: PhpNames) -> Vec<CompletionItem> {
    let typed = typed_name(analysed, offset);
    if php == PhpNames::Off || typed.len() < PHP_PREFIX || typed.contains('\\') {
        return Vec::new();
    }
    let mut found = Vec::new();
    for candidate in php_names::starting_with(typed) {
        for shape in candidate.items() {
            // `Resolved` is the developer who wants only the names that go
            // somewhere: the other three shapes are the audit, and an audit is
            // not what everyone opened the editor for.
            if php == PhpNames::Resolved && shape.insertion().is_none() {
                continue;
            }
            found.push(php_item(candidate, shape, typed));
        }
    }
    found
}

/// The name being written at `offset` — the run of name characters ending
/// there — and the empty string where the cursor is in no name or in something
/// that is not code.
///
/// Read off the source for [`namespace_written`]'s reason: a half-written name
/// is a node in some shapes and two in others, and the run of characters before
/// the cursor is the same thing in all of them. The tree is still what says the
/// cursor is in code at all, which is [`NOT_CODE`] — a PHP built-in offered
/// inside a string literal would be a name completed where no name is being
/// written.
fn typed_name(analysed: &Analysed, offset: BytePos) -> &str {
    let Some(before) = offset.checked_sub(1) else {
        return "";
    };
    let Some(node) = analysed.index.at(before).innermost() else {
        return "";
    };
    if NOT_CODE.contains(&node.kind) {
        return "";
    }
    let Some(upto) = analysed
        .map
        .file(analysed.entry)
        .text()
        .get(..offset as usize)
    else {
        return "";
    };
    let start = upto
        .char_indices()
        .rev()
        .take_while(|(_, ch)| is_name(*ch))
        .last()
        .map_or(upto.len(), |(at, _)| at);
    &upto[start..]
}

/// One PHP built-in, as the shape its row and the registry gave it.
///
/// **`insert_text` is the whole of what this may put in a buffer**, and it is
/// `php_names::Item::insertion` where there is one. The three shapes that
/// insert nothing carry the characters the developer has already typed instead:
/// a client replaces the word being completed with this field, so leaving it
/// unset would type the *label* — the PHP spelling — into the file, which is
/// the second name `rule:statements/nothing-gets-a-second-name` removes and the
/// unresolvable call `rule:php-migration/an-item-inserts-only-a-registered-member`
/// refuses. Accepting one of them therefore leaves the buffer as it was.
///
/// The row's own cell is the documentation, verbatim as the migration table
/// writes it, because that is where a `dropped` row's reason and its rewrite
/// are and neither fits a detail column. The detail says which of the four
/// shapes this is in one line, and only the first of them names a signature —
/// the registry's, which is the same row [`core_member`] spells a `Core` member
/// from.
fn php_item(candidate: &Candidate, shape: php_names::Item, typed: &str) -> CompletionItem {
    let detail = match shape {
        Item::Inserts(destination) => match destination.method() {
            Some(method) => format!("{}{}", destination.spelling(), core_signature(method)),
            None => destination.spelling(),
        },
        Item::NotRegistered(Some(destination)) => {
            format!("{} (not in Core yet)", destination.spelling())
        }
        Item::NotRegistered(None) => "no Core member to insert".to_owned(),
        Item::Dropped => "dropped from Novis".to_owned(),
        Item::Undecided => "undecided".to_owned(),
    };
    let kind = match (shape, candidate.kind) {
        (Item::Inserts(_), Kind::Function) => CompletionItemKind::FUNCTION,
        (Item::Inserts(_), Kind::Type) => CompletionItemKind::CLASS,
        // Three shapes insert nothing, and a symbol icon beside one would say
        // the language has a name it does not have.
        _ => CompletionItemKind::TEXT,
    };
    CompletionItem {
        label: candidate.php.to_owned(),
        kind: Some(kind),
        detail: Some(detail),
        documentation: (!candidate.cell.is_empty()).then(|| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: candidate.cell.to_owned(),
            })
        }),
        insert_text: Some(shape.insertion().unwrap_or_else(|| typed.to_owned())),
        ..CompletionItem::default()
    }
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
    analysed.local_ty(span, offset)
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
    item(
        method.name.to_owned(),
        CompletionItemKind::METHOD,
        core_signature(method),
    )
}

/// A `Core` member's parameter list and return type, from its registry row.
///
/// Two readers: the member lists above, where the member's own name is the
/// label, and [`php_item`], where the label is a PHP built-in and this is what
/// says where it went. One spelling for both, so the two lists never disagree
/// about a signature the registry states once.
fn core_signature(method: &CoreMethod) -> String {
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
    format!("({}): {}", params.join(", "), method.return_ty.spelled())
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

#[cfg(test)]
mod tests {
    use super::{CASE_WORDS, MEMBER_WORDS, STATEMENT_WORDS};
    use nvs_syntax::Keyword;

    /// The three lists above are spellings copied out of a grammar that owns
    /// them, and this is what keeps the copy honest: a word that stops being
    /// reserved — or was never spelled the way it is written here — fails the
    /// crate rather than being offered to a developer who then writes it.
    ///
    /// Completeness is not checkable the same way and is not claimed: which
    /// reserved words belong at which position is this module's judgement, and
    /// the grammar has no table of it to compare against.
    #[test]
    fn every_word_offered_is_one_the_lexer_reserves() {
        for word in STATEMENT_WORDS.iter().chain(MEMBER_WORDS).chain(CASE_WORDS) {
            assert!(
                Keyword::from_lowercase(word).is_some(),
                "`{word}` is offered by completion and is not a reserved word"
            );
        }
    }
}
