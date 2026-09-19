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
//! force in the entry document, every type the workspace index holds, and the
//! `Core` registry's classes and enums. Each is labelled with the shortest
//! spelling that resolves at that cursor — an import's own short name, a last
//! segment for a declaration in the namespace in force — which is
//! `nvs_hir::resolve_ref`'s order read backwards, and a type neither reaches is
//! labelled with its last segment and carries the `use` line that makes it
//! resolve
//! (`rule:ide/a-bare-name-reaches-every-type-and-imports-the-one-accepted`).
//!
//! **Which of those lists a cursor gets is read off the tokens before the name
//! being written** ([`Written`]): a `$` takes variables alone, `use` and a
//! written type take types alone, `new` takes the classes it compiles on, the
//! start of a statement takes every word and the inside of an expression the
//! words that open one
//! (`rule:ide/keywords-are-offered-where-the-compiler-accepts-them`). What a
//! client cannot rank by the match it made itself, [`Tier`] ranks.
//!
//! # What accepting a type writes after its name
//!
//! A type's name is the whole of what is written in three of the places one is
//! offered, and in the other two something always follows it
//! (`rule:ide/an-accepted-type-writes-what-follows-it`). **Inside an
//! expression the name is a receiver**, so the item writes `Name::` and asks
//! the client to open the list again, which is the static half of that class.
//! **After `new` it is a call**, so the item writes `Name()` and, where the
//! constructor declares a parameter, leaves the cursor between the parentheses
//! with signature help open.
//!
//! **Everywhere else the name is written alone**, and the start of a statement
//! is one of those places: `User $u = …` and `User::create()` both start
//! there, and nothing before the name tells them apart. [`Written::Other`] is
//! the same answer for every token that says neither, because a `::` nobody
//! wanted is deleted by hand and a missing one is two keystrokes.
//!
//! What follows the cursor is read too: an item writes no `::` in front of one
//! and no `()` in front of a `(`. A command is sent only to a client that named
//! it ([`Client`]), and a client without snippets gets `Name()` as plain text.
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
//! # What markup offers
//!
//! Nothing, except at a half-written open tag: `<?` is not yet one of
//! `nvs_syntax::OPEN_TAGS`, so the lexer still reads it as markup, and it is
//! the one place in a run of markup where the developer is writing Novis. The
//! tags are offered there and [`crate::regions`] cuts the same bytes out of
//! the HTML region, so the editor's HTML service does not answer beside them
//! (`rule:ide/completion-is-asked-where-a-spelling-ends`).
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
//! # What a `::` in type position offers
//!
//! `Owner::Name` is written in type position too, and there it names a `type`
//! alias the owner declares, one of its enum cases or one of its constants —
//! never a method or a property, neither of which may stand in a type. The
//! offered list is exactly those three, which is also the order the checker
//! resolves them in (`rule:types/type-alias`).
//!
//! **This arm is reached off the source, because a type is not a node.** The
//! index answers no question about a written type
//! (`crate::definition::written_type_at`), and a type still being typed is not
//! even a complete one, so there is nothing to ask the tree for beyond
//! [`NOT_CODE`]'s question — is the cursor writing a program at all. What is
//! read is the name run before the `::`, put through the one resolver
//! (`crate::definition::resolved_name`). It is reached only where the two
//! access arms above found no node, so a `::` the parser did build an access
//! for is answered there and never here.
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
//! it is in an access before it asks what half of one. The fifth is a written
//! type the tokens do not give away — a property's, and a type argument
//! between `<` and `>` — which is answered as the position around it. The
//! sixth is the inside of
//! a string literal, answered as the position around it: the variables are
//! what an interpolation slot takes and are right, and the words that open a
//! statement sit beside them as noise no filter here removes. The seventh is
//! how far a namespace reaches: the declarations offered under a prefix are
//! the ones the index holds, so under
//! `rule:ide/check-scope-defaults-to-the-workspace`'s `"open"` a type in a
//! file nobody has opened is not among them. That setting is the answer, and
//! this arm deliberately has no second one — a directory walk of its own is
//! what `rule:ide/completion-offers-only-what-the-compiler-derived` refuses.
//! — owner: M10

use std::collections::BTreeMap;

use lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit,
    Documentation, InsertTextFormat, MarkupContent, MarkupKind, TextEdit,
};
use nvs_diagnostics::{BytePos, Diagnostics, PositionEncoding, SourceFile, Span};
use nvs_hir::QName;
use nvs_stdlib::php_names::{self, Candidate, Item, Kind};
use nvs_stdlib::registry::{self, CoreClass, CoreConst, CoreEnum, CoreMethod};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, EnumCase, MethodMember, Modifier, PropertyMember, Stmt, StmtKind,
};
use nvs_syntax::{IndexNode, Keyword, NodePath, OPEN_TAGS, Token, TokenKind, tokenize};
use nvs_types::{ExprInfo, Ty, TypeId};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::definition::{
    declared_type, imports_of, namespace_at, resolved_name, supertype_names, text_of,
};
use crate::document::Analysed;
use crate::index::{DeclKind, Declaration, SymbolIndex};
use crate::position::range_at;
use crate::regions::half_written_tag;
use crate::settings::{Client, PhpNames};

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
    client: Client,
    encoding: PositionEncoding,
) -> Vec<CompletionItem> {
    let path = analysed.index.at(offset);
    let cursor = Cursor {
        analysed,
        symbols,
        path: &path,
        offset,
        client,
        encoding,
    };
    let mut items = match asked(analysed, &path, offset) {
        Asked::Member(class, reach) => members_of(analysed, &class, reach),
        Asked::TypeMember(owner) => type_members_of(analysed, &owner),
        Asked::Namespace(prefix) => followed(
            &cursor,
            After::of(written(&cursor)),
            under(symbols, &prefix),
        ),
        Asked::OpenTag(written) => open_tags(&cursor, written),
        Asked::Position => position(&cursor, php),
        Asked::Nothing => return Vec::new(),
    };
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

/// Whether a request a trigger character raised at `offset` is one this module
/// answers.
///
/// A trigger character is one keystroke of a longer spelling, and the editor
/// asks on the keystroke rather than on the spelling: `:` is half of `::`, `>`
/// is a comparison far more often than the end of `->`, and `?` is a ternary
/// before it is the second byte of an open tag. Answering those with whatever
/// the position offers opens a list nobody asked for, so a triggered request is
/// answered only where the text before the cursor ends in the whole spelling.
/// A request the developer raised by hand, or by typing a name, is not asked
/// this.
#[must_use]
pub fn continues_a_trigger(analysed: &Analysed, offset: BytePos) -> bool {
    let Some(upto) = analysed
        .map
        .file(analysed.entry)
        .text()
        .get(..offset as usize)
    else {
        return false;
    };
    ["->", "::", "\\", "$"]
        .iter()
        .any(|spelling| upto.ends_with(spelling))
        || open_tag_written(upto).is_some()
}

/// What every arm of a bare position reads: the analysis, the index, and where
/// the cursor is in both.
struct Cursor<'a> {
    analysed: &'a Analysed,
    symbols: &'a SymbolIndex,
    path: &'a NodePath,
    offset: BytePos,
    /// What the client does with an item beyond inserting its text.
    client: Client,
    /// The units a range on the wire is counted in. An item that names the
    /// text it replaces, or a line it adds elsewhere, is the one place this
    /// module writes a range.
    encoding: PositionEncoding,
}

impl Cursor<'_> {
    /// The entry document's text up to the cursor.
    fn upto(&self) -> &str {
        self.analysed
            .map
            .file(self.analysed.entry)
            .text()
            .get(..self.offset as usize)
            .unwrap_or_default()
    }

    /// The entry document's text after the name the cursor is inside.
    fn after_the_name(&self) -> &str {
        self.analysed
            .map
            .file(self.analysed.entry)
            .text()
            .get(self.offset as usize..)
            .unwrap_or_default()
            .trim_start_matches(is_name)
    }

    /// An edit that replaces the `len` bytes before the cursor with `text`.
    fn replacing(&self, len: usize, text: String) -> CompletionTextEdit {
        let len = BytePos::try_from(len).expect("a name is shorter than its file");
        let file = self.analysed.map.file(self.analysed.entry);
        let span = Span {
            file: self.analysed.entry,
            start: self.offset - len,
            end: self.offset,
        };
        CompletionTextEdit::Edit(TextEdit {
            range: range_at(file, span, self.encoding),
            new_text: text,
        })
    }
}

/// What the cursor is asking for, which is decided before anything is looked
/// up.
enum Asked {
    /// The member half of an access, off the class it resolved to and reaching
    /// the half of it the access shape names.
    Member(QName, Reach),
    /// The member half of an `Owner::` written in **type** position, off the
    /// owner the name before the `::` resolved to.
    TypeMember(QName),
    /// A name reaching into the namespace these segments spell — what that
    /// namespace holds, and no word.
    Namespace(Vec<String>),
    /// A half-written open tag in a run of markup, this many bytes of it
    /// written so far — the tags that open code, and nothing else.
    OpenTag(usize),
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
    //
    // The one thing Novis answers in markup is the tag that leaves it: `<?`
    // is not yet a tag, so the lexer still reads it as markup, and what the
    // developer is writing there is one of `nvs_syntax::OPEN_TAGS`.
    if in_markup(analysed, path, offset) {
        let upto = analysed
            .map
            .file(analysed.entry)
            .text()
            .get(..offset as usize);
        return upto
            .and_then(open_tag_written)
            .map_or(Asked::Nothing, Asked::OpenTag);
    }
    // `Name:` is half of `Name::`, and nothing may be written between the two
    // colons, so the position's own list would be offered at the one place no
    // entry of it parses.
    if after_a_lone_colon(analysed, offset) {
        return Asked::Nothing;
    }
    if let Some(prefix) = namespace_written(analysed, offset) {
        return Asked::Namespace(prefix);
    }
    let Some((access, reach)) = access_in(path).or_else(|| ended_at(analysed, offset)) else {
        return owner_written(analysed, offset).map_or(Asked::Position, Asked::TypeMember);
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

/// Whether the cursor at `offset` is writing markup.
///
/// Inside a run of it, or at the end of one: a span is half-open, so the
/// cursor after the last byte of a run is outside that node, and what is typed
/// there still extends the run — the file stops there, or an open tag follows.
fn in_markup(analysed: &Analysed, path: &NodePath, offset: BytePos) -> bool {
    let markup = |node: Option<IndexNode>| node.is_some_and(|node| node.kind == MARKUP);
    markup(path.innermost())
        || offset
            .checked_sub(1)
            .is_some_and(|before| markup(analysed.index.at(before).innermost()))
}

/// How many bytes of a half-written open tag `upto` ends in, or `None` where
/// it ends in none.
///
/// [`half_written_tag`] is the one reading of "half-written", shared with
/// [`crate::regions`] so the server offers the tags exactly where it stops
/// reporting the bytes as HTML.
fn open_tag_written(upto: &str) -> Option<usize> {
    let at = upto.rfind("<?")?;
    half_written_tag(&upto[at..]).filter(|len| at + len == upto.len())
}

/// The tags that open code, each replacing the `written` bytes of one already
/// typed.
///
/// The item names the text it replaces because `<?` is no word: a client left
/// to choose would insert the whole tag after the two bytes already there.
fn open_tags(cursor: &Cursor<'_>, written: usize) -> Vec<CompletionItem> {
    let details = ["starts a block of Novis code", "prints one expression"];
    OPEN_TAGS
        .iter()
        .zip(details)
        .enumerate()
        .map(|(rank, (tag, detail))| CompletionItem {
            text_edit: Some(cursor.replacing(written, (*tag).to_owned())),
            filter_text: Some((*tag).to_owned()),
            // The code tag first, which is the order the lexer lists them in.
            sort_text: Some(rank.to_string()),
            ..item(
                (*tag).to_owned(),
                CompletionItemKind::KEYWORD,
                detail.to_owned(),
            )
        })
        .collect()
}

/// Whether the name being written at `offset` follows a single `:` that itself
/// follows a bare name, as `Core\Str:` does.
///
/// A variable before the colon is a ternary's middle operand and is left
/// alone, and so is a colon with a space after it, which is where a `case`
/// arm and a ternary's last operand are written.
fn after_a_lone_colon(analysed: &Analysed, offset: BytePos) -> bool {
    let Some(before) = offset.checked_sub(1) else {
        return false;
    };
    if analysed
        .index
        .at(before)
        .innermost()
        .is_some_and(|node| NOT_CODE.contains(&node.kind))
    {
        return false;
    }
    let Some(upto) = analysed
        .map
        .file(analysed.entry)
        .text()
        .get(..offset as usize)
    else {
        return false;
    };
    let stem = &upto[..upto.len() - name_run(upto).len()];
    let Some(named) = stem.strip_suffix(':') else {
        return false;
    };
    let owner = name_run(named);
    !owner.is_empty() && !named[..named.len() - owner.len()].ends_with('$')
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
    let (prefix, _) = name_run(upto).rsplit_once('\\')?;
    if prefix.is_empty() {
        return None;
    }
    Some(QName::parse(prefix).segments().to_vec())
}

/// The run of name characters ending at the end of `text`, which is the name
/// the cursor there is writing.
fn name_run(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, ch)| is_name(*ch))
        .last()
        .map_or(text.len(), |(at, _)| at);
    &text[start..]
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
///
/// Each word carries the two things that decide whether it is offered. Its
/// [`Slot`] is which of the two dispatches it came from: a word that opens a
/// statement form is written only where a statement starts, and one that
/// opens a primary expression is written there and inside an expression alike.
/// Its [`Within`] is what has to be around the cursor for the compiler to
/// accept it: `break` leaves a loop, `self` names the class it is written in,
/// and a type declaration nested in a body is `E0233`.
const STATEMENT_WORDS: &[(&str, Slot, Within)] = &[
    ("abstract", Slot::Statement, Within::NoBody),
    ("autoload", Slot::Statement, Within::NoBody),
    ("break", Slot::Statement, Within::LoopOrSwitch),
    ("class", Slot::Statement, Within::NoBody),
    ("clone", Slot::Expression, Within::Anything),
    ("continue", Slot::Statement, Within::Loop),
    ("do", Slot::Statement, Within::Anything),
    ("echo", Slot::Statement, Within::Anything),
    ("empty", Slot::Expression, Within::Anything),
    ("enum", Slot::Statement, Within::NoBody),
    ("false", Slot::Expression, Within::Anything),
    ("final", Slot::Statement, Within::NoBody),
    ("fn", Slot::Expression, Within::Anything),
    ("for", Slot::Statement, Within::Anything),
    ("foreach", Slot::Statement, Within::Anything),
    ("if", Slot::Statement, Within::Anything),
    ("interface", Slot::Statement, Within::NoBody),
    ("isset", Slot::Expression, Within::Anything),
    ("match", Slot::Expression, Within::Anything),
    ("namespace", Slot::Statement, Within::NoBody),
    ("new", Slot::Expression, Within::Anything),
    ("null", Slot::Expression, Within::Anything),
    ("parent", Slot::Expression, Within::Class),
    ("print", Slot::Expression, Within::Anything),
    ("require", Slot::Expression, Within::Anything),
    ("return", Slot::Statement, Within::Anything),
    ("self", Slot::Expression, Within::Class),
    ("static", Slot::Expression, Within::Class),
    ("switch", Slot::Statement, Within::Anything),
    ("throw", Slot::Expression, Within::Anything),
    ("true", Slot::Expression, Within::Anything),
    ("try", Slot::Statement, Within::Anything),
    ("unset", Slot::Statement, Within::Anything),
    ("use", Slot::Statement, Within::NoBody),
    ("var", Slot::Statement, Within::Anything),
    ("while", Slot::Statement, Within::Anything),
    ("yield", Slot::Expression, Within::Body),
];

/// Which of the grammar's two dispatches a word of [`STATEMENT_WORDS`] opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    /// A statement form of its own, written only where a statement starts.
    Statement,
    /// A primary expression, written wherever an expression is.
    Expression,
}

/// What has to enclose the cursor for a word to be one the compiler accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Within {
    /// Nothing in particular.
    Anything,
    /// A loop, with no function body between it and the cursor.
    Loop,
    /// A loop or a `switch`, on the same terms.
    LoopOrSwitch,
    /// A class, an interface or an enum.
    Class,
    /// A method's, a function's or a closure's body.
    Body,
    /// No such body: a declaration is written at the top of a file or of a
    /// namespace.
    NoBody,
}

/// The node kinds that are a function body's owner, as `nvs_syntax::walk`
/// spells them. A loop outside one of these is not a loop the cursor is in.
const BODIES: &[&str] = &["Method", "Function", "Fn"];

/// The node kinds that are a loop.
const LOOPS: &[&str] = &["For", "Foreach", "While", "DoWhile"];

/// The node kinds that declare a type with members.
const TYPES: &[&str] = &["ClassDecl", "InterfaceDecl", "EnumDecl"];

impl Within {
    /// Whether the nodes around the cursor, innermost first, are what this
    /// asks for.
    fn holds(self, path: &NodePath) -> bool {
        let kinds = || path.nodes().iter().map(|node| node.kind);
        let before_a_body = || kinds().take_while(|kind| !BODIES.contains(kind));
        match self {
            Self::Anything => true,
            Self::Loop => before_a_body().any(|kind| LOOPS.contains(&kind)),
            Self::LoopOrSwitch => {
                before_a_body().any(|kind| LOOPS.contains(&kind) || kind == "Switch")
            }
            Self::Class => kinds().any(|kind| TYPES.contains(&kind)),
            Self::Body => kinds().any(|kind| BODIES.contains(&kind)),
            Self::NoBody => !kinds().any(|kind| BODIES.contains(&kind)),
        }
    }
}

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
///
/// Inside a body of statements, **the tokens before the name being written**
/// say which of its lists the cursor may have ([`Written`]): a `$` is a
/// variable and nothing else, `use` and a written type take a type, `new`
/// takes a class it compiles on, the start of a statement takes every word,
/// and the inside of an expression takes the words that open one. The tree
/// cannot say this, because the name being written is usually what stops the
/// statement around it from parsing.
fn position(cursor: &Cursor<'_>, php: PhpNames) -> Vec<CompletionItem> {
    let Cursor {
        analysed,
        path,
        offset,
        ..
    } = *cursor;
    match path.innermost().map(|node| node.kind) {
        Some("ClassDecl" | "InterfaceDecl") => words(MEMBER_WORDS),
        Some("EnumDecl") => words(CASE_WORDS),
        Some("Property" | "Const" | "EnumCase") => Vec::new(),
        _ => match written(cursor) {
            Written::Variable => in_scope(cursor),
            Written::Import => in_reach(cursor, Spelled::Qualified),
            Written::TypeName => in_reach(cursor, Spelled::Shortest),
            Written::Constructed => {
                let mut items = Vec::new();
                if Within::Class.holds(path) {
                    items.extend(words(CLASS_WORDS));
                }
                items.extend(in_reach(cursor, Spelled::Shortest));
                followed(cursor, After::Arguments, items)
            }
            placed @ (Written::Statement | Written::Expression | Written::Other) => {
                let mut items = statement_words(cursor, placed);
                items.extend(in_scope(cursor));
                items.extend(followed(
                    cursor,
                    After::of(placed),
                    in_reach(cursor, Spelled::Shortest),
                ));
                items.extend(php_builtins(analysed, offset, php));
                items
            }
        },
    }
}

/// What the tokens before the name being written say may be written there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Written {
    /// After a `$`: a variable, and no word or type.
    Variable,
    /// After `use`: a type, by its qualified name, because a declaration's
    /// name is absolute (`rule:statements/a-qualified-name-is-absolute`).
    Import,
    /// After `new`: a class `new` compiles on.
    Constructed,
    /// Where a type is written: after `extends`, `implements`, `is`, `as` or
    /// `#[`, in a parameter list, in a `catch`, and after the `:` of a return
    /// type.
    TypeName,
    /// Where a statement starts, which is also where a typed local does.
    Statement,
    /// Inside an expression, after a token only a value follows.
    Expression,
    /// After a token that says none of the above — a name, a literal, a `<`.
    /// The expression's list, because that is what is usually being written.
    Other,
}

/// Which [`Written`] the cursor is at.
///
/// Read off the lexer's own tokens, so a `;` inside a string or a comment is
/// not a statement's end. The tokens that end at or before the name being
/// written decide, and no token at all is the top of a file.
fn written(cursor: &Cursor<'_>) -> Written {
    let upto = cursor.upto();
    let name = name_run(upto);
    let stem = &upto[..upto.len() - name.len()];
    if stem.ends_with('$') {
        return Written::Variable;
    }
    let file = cursor.analysed.map.file(cursor.analysed.entry);
    let tokens = tokenize(file, &mut Diagnostics::new());
    let before: Vec<TokenKind> = tokens
        .iter()
        .take_while(|token| token.span.end as usize <= stem.len())
        .map(|token: &Token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect();
    placed(&before)
}

/// What a name written straight after `before` is.
///
/// One token decides most of it. Three do not: a `(` and a `,` open a
/// parameter's type in a declaration and a value in a call, and a `:` opens a
/// return type after a declaration's `)` and a statement everywhere else —
/// so each is read with the bracket it belongs to. A `?`, a `|`, a `&` and a
/// parameter's modifier continue a type where one was being written, which is
/// asked of the tokens before that type.
fn placed(before: &[TokenKind]) -> Written {
    use TokenKind as T;
    let Some((last, earlier)) = before.split_last() else {
        return Written::Statement;
    };
    match last {
        T::Keyword(Keyword::Use) => Written::Import,
        T::Keyword(Keyword::New) => Written::Constructed,
        T::Keyword(Keyword::Extends | Keyword::Implements | Keyword::Is | Keyword::As)
        | T::AttributeOpen => Written::TypeName,
        T::LParen if declares(earlier) => Written::TypeName,
        T::Comma if lists_types(earlier) => Written::TypeName,
        T::Colon if returns(earlier) => Written::TypeName,
        T::Question | T::Pipe | T::Amp => match placed(before_the_type(earlier)) {
            found @ (Written::TypeName | Written::Statement) => found,
            _ => Written::Expression,
        },
        T::Keyword(word) if PARAMETER_WORDS.contains(word) => {
            match placed(before_the_type(earlier)) {
                found @ (Written::TypeName | Written::Statement) => found,
                _ => Written::Other,
            }
        }
        T::Semicolon
        | T::LBrace
        | T::RBrace
        | T::RParen
        | T::Colon
        | T::OpenTagNvs
        | T::CloseTag
        | T::InlineHtml
        | T::Keyword(Keyword::Else | Keyword::Do) => Written::Statement,
        T::LParen
        | T::LBracket
        | T::Comma
        | T::FatArrow
        | T::Equals
        | T::PlusEquals
        | T::MinusEquals
        | T::StarEquals
        | T::StarStarEquals
        | T::SlashEquals
        | T::PercentEquals
        | T::DotEquals
        | T::AmpEquals
        | T::PipeEquals
        | T::CaretEquals
        | T::LtLtEquals
        | T::GtGtEquals
        | T::QuestionQuestionEquals
        | T::QuestionQuestion
        | T::Dot
        | T::Plus
        | T::Minus
        | T::Star
        | T::StarStar
        | T::Slash
        | T::Percent
        | T::AmpAmp
        | T::PipePipe
        | T::PipeGreater
        | T::Caret
        | T::Tilde
        | T::Bang
        | T::BangEquals
        | T::EqualsEquals
        | T::LtEquals
        | T::GtEquals
        | T::Spaceship
        | T::LtLt
        | T::Ellipsis
        | T::At
        | T::OpenTagEcho
        | T::Keyword(
            Keyword::And
            | Keyword::Case
            | Keyword::Clone
            | Keyword::Echo
            | Keyword::Or
            | Keyword::Print
            | Keyword::Return
            | Keyword::Throw
            | Keyword::Xor
            | Keyword::Yield,
        ) => Written::Expression,
        _ => Written::Other,
    }
}

/// The words a parameter or a typed local writes in front of its type.
const PARAMETER_WORDS: &[Keyword] = &[
    Keyword::Inout,
    Keyword::Private,
    Keyword::Protected,
    Keyword::Public,
    Keyword::Readonly,
    Keyword::Secret,
    Keyword::Tainted,
];

/// The reserved words that are a type's whole spelling.
const TYPE_WORDS: &[Keyword] = &[
    Keyword::Array,
    Keyword::Bool,
    Keyword::Bytes,
    Keyword::Callable,
    Keyword::Decimal,
    Keyword::False,
    Keyword::Float,
    Keyword::Int,
    Keyword::Iterable,
    Keyword::Mixed,
    Keyword::Never,
    Keyword::Null,
    Keyword::Object,
    Keyword::SelfKw,
    Keyword::Static,
    Keyword::String,
    Keyword::True,
    Keyword::Uint,
    Keyword::Void,
];

/// `before` without the type, or the constant expression, it ends in: names,
/// separators, `?`, `|`, `&`, `::` and the words above.
///
/// The two read the same, which is why the caller asks what came before the
/// run and never what the run is.
fn before_the_type(before: &[TokenKind]) -> &[TokenKind] {
    use TokenKind as T;
    let run = before
        .iter()
        .rev()
        .take_while(|kind| match kind {
            T::Ident | T::Backslash | T::Question | T::Pipe | T::Amp | T::DoubleColon => true,
            T::Keyword(word) => TYPE_WORDS.contains(word) || PARAMETER_WORDS.contains(word),
            _ => false,
        })
        .count();
    &before[..before.len() - run]
}

/// Whether a `(` written straight after `before` opens a list of declared
/// types: a function's, a method's or a closure's parameters, or a `catch`.
///
/// A method's name may be any word, so what is matched is the `function` in
/// front of it. A type parameter list between the name and the `(` is read
/// past.
fn declares(before: &[TokenKind]) -> bool {
    use TokenKind as T;
    let named = |before: &[TokenKind]| matches!(before, [.., T::Keyword(Keyword::Function), _]);
    match before {
        [
            ..,
            T::Keyword(Keyword::Fn | Keyword::Function | Keyword::Catch),
        ] => true,
        [.., T::Gt | T::GtGt] => before
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, kind)| **kind == T::Lt)
            .any(|(at, _)| named(&before[..at])),
        _ => named(before),
    }
}

/// Whether a `:` written straight after `before` opens a return type, which
/// is one that follows the `)` of a declaration's parameters.
fn returns(before: &[TokenKind]) -> bool {
    let [inside @ .., TokenKind::RParen] = before else {
        return false;
    };
    enclosing(inside).is_some_and(|open| {
        inside[open] == TokenKind::LParen
            && !matches!(inside[..open], [.., TokenKind::Keyword(Keyword::Catch)])
            && declares(&inside[..open])
    })
}

/// Whether a `,` written straight after `before` is followed by a type: the
/// next name of an `extends` or `implements` list, or the next parameter of a
/// declaration.
fn lists_types(before: &[TokenKind]) -> bool {
    use TokenKind as T;
    let list = before.iter().rev().find(|kind| {
        !matches!(
            kind,
            T::Ident | T::Backslash | T::Comma | T::Variable | T::Lt | T::Gt | T::GtGt
        )
    });
    if matches!(
        list,
        Some(T::Keyword(Keyword::Extends | Keyword::Implements))
    ) {
        return true;
    }
    enclosing(before).is_some_and(|open| before[open] == T::LParen && declares(&before[..open]))
}

/// Where the bracket that is still open at the end of `before` was written.
fn enclosing(before: &[TokenKind]) -> Option<usize> {
    use TokenKind as T;
    let mut closed = 0_usize;
    for (at, kind) in before.iter().enumerate().rev() {
        match kind {
            T::RParen | T::RBracket | T::RBrace => closed += 1,
            T::LParen | T::LBracket | T::LBrace if closed == 0 => return Some(at),
            T::LParen | T::LBracket | T::LBrace => closed -= 1,
            _ => {}
        }
    }
    None
}

/// The three words that name a class from inside it.
const CLASS_WORDS: &[&str] = &["parent", "self", "static"];

/// The words of [`STATEMENT_WORDS`] that may be written at the cursor, which
/// is at the start of a statement or inside an expression.
///
/// A word of [`CLASS_WORDS`] is a receiver the way a type's name is, and
/// writes its `::` on the same terms. `parent` writes it at the start of a
/// statement too, where it opens nothing else.
fn statement_words(cursor: &Cursor<'_>, placed: Written) -> Vec<CompletionItem> {
    let statement = placed == Written::Statement;
    STATEMENT_WORDS
        .iter()
        .filter(|(_, slot, within)| {
            (statement || *slot == Slot::Expression) && within.holds(cursor.path)
        })
        .map(|(word, _, _)| {
            let offered = item(
                (*word).to_owned(),
                CompletionItemKind::KEYWORD,
                String::new(),
            );
            let receiver = CLASS_WORDS.contains(word)
                && (placed == Written::Expression || (statement && *word == "parent"));
            Tier::Keyword.ranks(if receiver {
                scoped(cursor, offered)
            } else {
                offered
            })
        })
        .collect()
}

/// What accepting a type writes after its name — the module doc's *What
/// accepting a type writes after its name*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum After {
    /// Nothing: the name is the whole of it.
    Nothing,
    /// `::`, and the list opened again.
    Scope,
    /// `()`, with the cursor inside where there is an argument to write.
    Arguments,
}

impl After {
    /// What follows a type offered at `placed`.
    fn of(placed: Written) -> Self {
        match placed {
            Written::Expression => Self::Scope,
            Written::Constructed => Self::Arguments,
            _ => Self::Nothing,
        }
    }
}

/// `items`, each writing what `after` says follows it.
///
/// After `new` this is a filter as well: a name `new` does not compile on is
/// not offered. An item's `detail` is the qualified name it was offered for,
/// which is what the two tables that answer are keyed on.
fn followed(cursor: &Cursor<'_>, after: After, items: Vec<CompletionItem>) -> Vec<CompletionItem> {
    match after {
        After::Nothing => items,
        After::Scope => items
            .into_iter()
            .map(|offered| {
                let receiver = matches!(
                    offered.kind,
                    Some(
                        CompletionItemKind::CLASS
                            | CompletionItemKind::INTERFACE
                            | CompletionItemKind::ENUM
                    )
                );
                if receiver {
                    scoped(cursor, offered)
                } else {
                    offered
                }
            })
            .collect(),
        After::Arguments => {
            let classes = Classes::of(cursor.symbols);
            items
                .into_iter()
                .filter_map(|offered| {
                    let takes = if offered.kind == Some(CompletionItemKind::KEYWORD) {
                        Takes::Arguments
                    } else {
                        classes.constructed(offered.detail.as_deref()?)?
                    };
                    Some(called(cursor, offered, takes))
                })
                .collect()
        }
    }
}

/// `offered`, writing `::` after its label and opening the list again.
fn scoped(cursor: &Cursor<'_>, offered: CompletionItem) -> CompletionItem {
    if cursor.after_the_name().starts_with("::") {
        return offered;
    }
    CompletionItem {
        insert_text: Some(format!("{}::", offered.label)),
        command: cursor.client.suggest.then(|| Command {
            title: "Suggest".to_owned(),
            command: Client::SUGGEST.to_owned(),
            arguments: None,
        }),
        ..offered
    }
}

/// `offered`, writing the parentheses of a `new` after its label.
fn called(cursor: &Cursor<'_>, offered: CompletionItem, takes: Takes) -> CompletionItem {
    if cursor.after_the_name().trim_start().starts_with('(') {
        return offered;
    }
    if takes == Takes::Nothing || !cursor.client.snippets {
        return CompletionItem {
            insert_text: Some(format!("{}()", offered.label)),
            ..offered
        };
    }
    // A snippet reads `\`, `$` and `}` as its own, and a qualified name is
    // written with the first of them.
    let label = offered.label.replace('\\', r"\\");
    CompletionItem {
        insert_text: Some(format!("{label}($0)")),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        command: cursor.client.parameter_hints.then(|| Command {
            title: "Parameter hints".to_owned(),
            command: Client::PARAMETER_HINTS.to_owned(),
            arguments: None,
        }),
        ..offered
    }
}

/// Whether a `new` has an argument to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Takes {
    /// No constructor anywhere above the class declares a parameter.
    Nothing,
    /// One does, or one is declared where the index cannot read it.
    Arguments,
}

/// Every declaration the index holds, by the name it declares.
struct Classes<'a>(FxHashMap<&'a str, &'a Declaration>);

impl<'a> Classes<'a> {
    /// The index's declarations, keyed once for a list that asks about each.
    fn of(symbols: &'a SymbolIndex) -> Self {
        Self(
            symbols
                .files()
                .flat_map(|path| symbols.declarations_in(path))
                .map(|declared| (declared.symbol.as_str(), declared))
                .collect(),
        )
    }

    /// What `new` on `symbol` takes, or `None` where it does not compile: an
    /// interface, an enum, an alias, an `abstract` class, and a `Core` class
    /// the registry names no constructor for.
    fn constructed(&self, symbol: &str) -> Option<Takes> {
        let Some(declared) = self.0.get(symbol) else {
            return registry::constructor_of(symbol).map(|constructor| {
                if constructor.params.is_empty() {
                    Takes::Nothing
                } else {
                    Takes::Arguments
                }
            });
        };
        let construction = declared.construction.filter(|class| !class.is_abstract)?;
        Some(match construction.parameters {
            Some(0) => Takes::Nothing,
            Some(_) => Takes::Arguments,
            None => self.inherited(declared, 0),
        })
    }

    /// What the constructor `declared` inherits takes. A class with nothing
    /// above it takes nothing, and a superclass the index does not hold is one
    /// whose constructor cannot be read.
    fn inherited(&self, declared: &Declaration, depth: usize) -> Takes {
        // A hierarchy with a cycle in it is a compile error this may still be
        // asked about, and no real one is this deep.
        const DEEPEST: usize = 32;
        let mut takes = Takes::Nothing;
        for above in &declared.supertypes {
            let Some(parent) = self.0.get(above.as_str()) else {
                takes = Takes::Arguments;
                continue;
            };
            let Some(class) = parent.construction else {
                continue;
            };
            return match class.parameters {
                Some(0) => Takes::Nothing,
                Some(_) => Takes::Arguments,
                None if depth < DEEPEST => self.inherited(parent, depth + 1),
                None => Takes::Arguments,
            };
        }
        takes
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
            Tier::Keyword.ranks(item(
                (*word).to_owned(),
                CompletionItemKind::KEYWORD,
                String::new(),
            ))
        })
        .collect()
}

/// Where an item of a bare position ranks among the others, best first.
///
/// A client orders a list by how well each label matches what was typed and
/// breaks a tie on `sortText`, so this decides the order of an empty prefix and
/// of every tie: what this body declared, then the types this file already
/// reaches — imported, in its own namespace, written somewhere in it — then
/// the rest of `Core`, the rest of the workspace, the reserved words, and last
/// the PHP names, which are an audit and not a program's own vocabulary. Every
/// input is a table an arm below already reads. A member list has no tiers:
/// every row of one is as likely as the next, and its order is its labels'.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    Variable,
    Imported,
    SameNamespace,
    Used,
    Core,
    Workspace,
    Keyword,
    Php,
}

impl Tier {
    /// `item`, carrying this tier as the first character of its `sortText` and
    /// its own label after it.
    fn ranks(self, item: CompletionItem) -> CompletionItem {
        CompletionItem {
            sort_text: Some(format!("{}{}", self as u8, item.label)),
            ..item
        }
    }
}

/// The variables the body the cursor is in declared, with the type each was
/// declared at.
///
/// Each item names the text it replaces — the `$` and as much of the name as
/// is written — because a lone `$` is no word to a client, and one left to
/// choose would insert the label after it and write `$$name`. After `::` the
/// `$` opens a static property, which is the access arm's and no local.
fn in_scope(cursor: &Cursor<'_>) -> Vec<CompletionItem> {
    let upto = cursor.upto();
    let name = name_run(upto);
    let stem = &upto[..upto.len() - name.len()];
    if stem.ends_with("::$") {
        return Vec::new();
    }
    let typed = name.len() + usize::from(stem.ends_with('$'));
    cursor
        .analysed
        .bodies_at(cursor.offset)
        .into_iter()
        .next()
        .unwrap_or_default()
        .iter()
        .map(|local| {
            let label = format!("${}", local.name);
            Tier::Variable.ranks(CompletionItem {
                // Only a name that starts with `$` is a variable being
                // written; a bare word keeps the client's own word range.
                text_edit: stem
                    .ends_with('$')
                    .then(|| cursor.replacing(typed, label.clone())),
                filter_text: Some(label.clone()),
                ..item(
                    label.clone(),
                    CompletionItemKind::VARIABLE,
                    cursor.analysed.interner.describe(local.ty),
                )
            })
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
///
/// **A type no short name reaches is offered by its last segment, and
/// accepting it writes the `use` line too.** The item inserts the short name
/// and carries one more edit that adds `use Qualified\Name;` at
/// [`import_site`], so what is left in the buffer is a name that resolves —
/// the same bar, met by an edit. Its `filterText` is both
/// spellings, so `Str` and `Core\Str` both find it, and so do the initials a
/// client's own fuzzy match reads across the separator. Where the short name
/// is already taken in this file, or there is nowhere to write the line, the
/// item is the qualified name and edits nothing else. After `use`
/// ([`Spelled::Qualified`]) every type is its qualified name, because that is
/// the only spelling a `use` declaration takes.
///
/// `nvs_stdlib::registry`'s classes and enums are in the list on the same
/// terms as the workspace's own: they are the two rosters [`under`] reads
/// after a separator, reached here before one is written.
fn in_reach(cursor: &Cursor<'_>, spelled: Spelled) -> Vec<CompletionItem> {
    let Cursor {
        analysed,
        symbols,
        offset,
        ..
    } = *cursor;
    let imports = imports_of(analysed);
    let short: FxHashMap<String, String> = imports
        .iter()
        .map(|(name, target)| (target.to_string(), name.clone()))
        .collect();
    let here = namespace_at(analysed, offset);
    let file = analysed.map.file(analysed.entry);
    let used: FxHashSet<&str> = file
        .path()
        .map(|path| symbols.occurrences_in(path))
        .unwrap_or_default()
        .iter()
        .map(|occurrence| occurrence.symbol.as_str())
        .collect();
    let core = registry::CLASSES
        .iter()
        .map(|class| (class.name.to_owned(), CompletionItemKind::CLASS))
        .chain(
            registry::ENUMS
                .iter()
                .map(|core| (core.name.to_owned(), CompletionItemKind::ENUM)),
        )
        .map(|(symbol, kind)| (symbol, kind, Tier::Core));
    let declared = symbols.files().flat_map(|path| {
        symbols.declarations_in(path).iter().filter_map(|declared| {
            let kind = namespace_kind(declared.kind)?;
            Some((declared.symbol.clone(), kind, Tier::Workspace))
        })
    });
    let candidates: Vec<(String, CompletionItemKind, Tier)> = declared.chain(core).collect();
    // A short name is free to import under only where nothing in force in this
    // file already answers to it.
    let mut taken: FxHashSet<String> = imports.keys().cloned().collect();
    taken.extend(candidates.iter().filter_map(|(symbol, _, _)| {
        let segments = QName::parse(symbol).segments().to_vec();
        (segments.len() == here.len() + 1 && segments.starts_with(&here))
            .then(|| segments[here.len()].clone())
    }));
    let site = import_site(cursor);
    let mut found: BTreeMap<String, CompletionItem> = BTreeMap::new();
    for (symbol, kind, origin) in candidates {
        let reached = written_as(&symbol, &here, &short);
        let tier = if short.contains_key(&symbol) {
            Tier::Imported
        } else if reached != symbol {
            Tier::SameNamespace
        } else if used.contains(symbol.as_str()) {
            Tier::Used
        } else {
            origin
        };
        let last = symbol.rsplit('\\').next().unwrap_or(&symbol).to_owned();
        let offered = if spelled == Spelled::Qualified {
            item(symbol.clone(), kind, symbol.clone())
        } else if reached != symbol || last == symbol {
            item(reached, kind, symbol.clone())
        } else if let Some(site) = site.as_ref().filter(|_| !taken.contains(&last)) {
            CompletionItem {
                filter_text: Some(format!("{last} {symbol}")),
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(symbol.clone()),
                }),
                additional_text_edits: Some(vec![site.importing(&symbol)]),
                ..item(last, kind, symbol.clone())
            }
        } else {
            item(symbol.clone(), kind, symbol.clone())
        };
        found.entry(symbol).or_insert_with(|| tier.ranks(offered));
    }
    if spelled == Spelled::Qualified {
        return found.into_values().collect();
    }
    // An import whose target the index does not hold is still a name in force
    // — a `Core` class, or one in a file the current scope does not reach
    // (`rule:ide/check-scope-defaults-to-the-workspace`). What it declares is
    // read off the registry, which is the one table that can say.
    for (name, target) in &imports {
        let target = target.to_string();
        found.entry(target.clone()).or_insert_with(|| {
            let kind = if registry::core_enum(&target).is_some() {
                CompletionItemKind::ENUM
            } else {
                CompletionItemKind::CLASS
            };
            Tier::Imported.ranks(item(name.clone(), kind, target))
        });
    }
    found.into_values().collect()
}

/// Which spelling of a type [`in_reach`] offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spelled {
    /// The shortest one that resolves at the cursor, importing where it has to.
    Shortest,
    /// The qualified name and nothing else, which is what `use` takes.
    Qualified,
}

/// Where a `use` line is added to the entry document, and what separates it
/// from what is already there.
struct ImportSite {
    /// The empty range the line is inserted at.
    range: lsp_types::Range,
    /// What is written before the declaration: the line break that ends the
    /// line it follows, and a blank line where it starts a group of its own.
    lead: &'static str,
}

impl ImportSite {
    /// The edit that imports `symbol` here.
    fn importing(&self, symbol: &str) -> TextEdit {
        TextEdit {
            range: self.range,
            new_text: format!("{}use {symbol};", self.lead),
        }
    }
}

/// Where a new `use` line goes for a name written at the cursor, or `None`
/// where this module will not choose.
///
/// After the last `use` declaration the cursor's namespace already has, which
/// keeps the group together; failing that after the `namespace Name;` line in
/// force; failing that after the open tag. Every one of those is above the
/// cursor and ends before the name being written, so the edit never touches
/// the text the item itself replaces. A bracketed namespace with no `use` in
/// it and a file with no open tag — a shebang script — have no such line, and
/// there the type is offered by its qualified name instead.
fn import_site(cursor: &Cursor<'_>) -> Option<ImportSite> {
    let Cursor {
        analysed,
        offset,
        encoding,
        ..
    } = *cursor;
    let file = analysed.map.file(analysed.entry);
    let loaded = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)?;
    // The statements the cursor's namespace is made of: a bracketed block's
    // own where the cursor is inside one, and otherwise the file's, from the
    // `namespace Name;` line in force onward.
    let block = loaded.stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::NamespaceDecl(decl) => decl
            .body
            .as_ref()
            .filter(|block| block.span.start <= offset && offset < block.span.end),
        _ => None,
    });
    let bracketed = block.is_some();
    let stmts: &[Stmt] = block.map_or(&loaded.stmts, |block| &block.stmts);
    let above = || stmts.iter().filter(|stmt| stmt.span.end <= offset);
    let governing = above()
        .rfind(|stmt| matches!(&stmt.kind, StmtKind::NamespaceDecl(decl) if decl.body.is_none()));
    let last_use = above().rfind(|stmt| {
        matches!(stmt.kind, StmtKind::UseDecl(_))
            && governing.is_none_or(|decl| decl.span.end <= stmt.span.start)
    });
    let (after, lead) = match (last_use, governing) {
        (Some(stmt), _) => (stmt.span.end, "\n"),
        (None, Some(decl)) => (decl.span.end, "\n\n"),
        (None, None) if bracketed => return None,
        (None, None) => {
            let tag = tokenize(file, &mut Diagnostics::new())
                .into_iter()
                .find(|token| token.kind == TokenKind::OpenTagNvs)?;
            (tag.span.end, "\n")
        }
    };
    (after <= offset).then(|| ImportSite {
        range: range_at(
            file,
            Span {
                file: analysed.entry,
                start: after,
                end: after,
            },
            encoding,
        ),
        lead,
    })
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
            found.push(Tier::Php.ranks(php_item(candidate, shape, typed)));
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
    name_run(upto)
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
    class_word(analysed, text, receiver.start)
        .or_else(|| resolved_name(analysed, text, receiver.start))
}

/// The class `word` names where it is one of [`CLASS_WORDS`] written at `at`:
/// the class around it, or for `parent` the one that class extends.
fn class_word(analysed: &Analysed, word: &str, at: BytePos) -> Option<QName> {
    if !CLASS_WORDS.contains(&word) {
        return None;
    }
    let path = analysed.index.at(at);
    let around = path
        .nodes()
        .iter()
        .find(|node| TYPES.contains(&node.kind))?
        .span;
    let own = analysed
        .module
        .symbols
        .iter()
        .find(|symbol| {
            let name = symbol.decl_span;
            name.file == around.file && around.start <= name.start && name.end <= around.end
        })?
        .qname
        .clone();
    if word != "parent" {
        return Some(own);
    }
    let (stmt, file) = declared_type(analysed, &own)?;
    let (extends, _) = supertype_names(stmt);
    let base = extends.first()?;
    resolved_name(analysed, text_of(file, *base), base.start)
}

/// The owner an `Owner::` written immediately before `offset` names, where the
/// cursor stands in no access the parser built a node for.
///
/// The module doc's *What a `::` in type position offers* is the reasoning:
/// the name is read off the source, and the tree is asked only whether the
/// cursor is writing a program. A cursor inside **no** node is still writing
/// one — a half-written type in a parameter list is a region the parser may
/// have given up on entirely — so what is refused here is a node whose text is
/// not code, and not the absence of one.
fn owner_written(analysed: &Analysed, offset: BytePos) -> Option<QName> {
    let before = offset.checked_sub(1)?;
    if analysed
        .index
        .at(before)
        .innermost()
        .is_some_and(|node| NOT_CODE.contains(&node.kind))
    {
        return None;
    }
    let upto = analysed
        .map
        .file(analysed.entry)
        .text()
        .get(..offset as usize)?;
    let stem = upto
        .get(..upto.len() - name_run(upto).len())?
        .strip_suffix("::")?;
    let owner = name_run(stem);
    let at = BytePos::try_from(stem.len() - owner.len()).ok()?;
    class_word(analysed, owner, at).or_else(|| resolved_name(analysed, owner, at))
}

/// Every name of `owner` that may be written after `Owner::` in type position:
/// the `type` aliases it declares, its enum cases and its class constants.
///
/// The same three rosters [`members_of`] chooses between, asked for the half of
/// each that stands in a type. A `Core` class contributes its constants and a
/// `Core` enum its cases; neither declares a `type` alias, which is a member of
/// a written body and the registry holds none.
fn type_members_of(analysed: &Analysed, owner: &QName) -> Vec<CompletionItem> {
    let name = owner.to_string();
    if let Some(core) = registry::class(&name) {
        core.constants.iter().map(core_constant).collect()
    } else if let Some(core) = registry::core_enum(&name) {
        core_cases(core, Reach::Static)
    } else {
        declared_type_members(analysed, owner)
    }
}

/// Every name a user-declared body writes that may stand after `Owner::` in a
/// type, as it wrote them.
///
/// The constant arm is [`declared_member`]'s own, so a constant offered here
/// and the same constant offered after `::` in an expression are one row
/// spelled once.
fn declared_type_members(analysed: &Analysed, owner: &QName) -> Vec<CompletionItem> {
    let Some((stmt, file)) = declared_type(analysed, owner) else {
        return Vec::new();
    };
    let (members, cases): (&[ClassMember], &[EnumCase]) = match &stmt.kind {
        StmtKind::ClassDecl(decl) => (&decl.members, &[]),
        StmtKind::InterfaceDecl(decl) => (&decl.members, &[]),
        StmtKind::EnumDecl(decl) => (&decl.members, &decl.cases),
        _ => return Vec::new(),
    };
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::TypeAlias(alias) => Some(item(
                text_of(file, alias.name.span).to_owned(),
                CompletionItemKind::TYPE_PARAMETER,
                text_of(file, alias.ty.span).to_owned(),
            )),
            ClassMemberKind::Const(_) => declared_member(file, member, Reach::Static),
            _ => None,
        })
        .chain(cases.iter().map(|case| enum_case(file, case)))
        .collect()
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
/// Nothing else is set here. `insert_text` would be the label again, and a
/// `text_edit` is a range this server has no reason to narrow: what the client
/// replaces is the word it is already completing. [`followed`] is where a
/// type's item learns what is written after its name.
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
        let statement = STATEMENT_WORDS.iter().map(|(word, _, _)| word);
        for word in statement.chain(MEMBER_WORDS).chain(CASE_WORDS) {
            assert!(
                Keyword::from_lowercase(word).is_some(),
                "`{word}` is offered by completion and is not a reserved word"
            );
        }
    }
}
