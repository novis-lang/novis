//! The declaration surface of a file: what another file can see of it, one
//! row per declaration, each hashed.
//!
//! A keystroke inside a method body changes nothing another file of the graph
//! resolves against, so the entries that read the file need no new analysis.
//! [`rows`] cuts the part of a parsed file that another file can see into one
//! [`Row`] per declaration, keyed by its symbol. [`of`] is the hash of all the
//! rows, and [`crate::SymbolIndex::refresh`] queues a file's readers only when
//! that hash differs from the one the file had. [`diff`] gives the symbols that
//! were added, removed or changed between two parses, each with its visibility
//! and its [`Kind`], so a caller can reach only the files that can see them
//! (`rule:ide/the-index-is-incremental-and-persistent`).
//!
//! # The rows
//!
//! A symbol is written the way the index writes it (`crate::index`): `App\User`,
//! `App\User::greet`, `App\User::$name`, `App\User::LIMIT`, `App\Status::Draft`,
//! and `Owner::Name` for an enum, alias or attribute a body declares.
//!
//! - **The graph row**, symbol `""`: every `namespace`, `use` and `autoload`
//!   line, every rejected top-level function or constant, every other
//!   file-scope statement whose text has the word `require`, and every function
//!   or hook body whose text has it. A `require` anywhere loads a file into the
//!   graph, so its text is here and not in its member's row.
//! - **A header row** for each class, interface and enum: its doc comment,
//!   modifiers, name, `extends`, `implements` (with `by` delegation) and, for
//!   an enum, its backing type. A file-scope or body `type` alias and an
//!   `attribute` declaration are one header row each, written whole.
//! - **A cases row** for each enum: the names of its cases, in order. The set
//!   and order of the cases is what `cases()` and an enum's codecs read.
//! - **A properties row** for each class and interface: the names of its
//!   properties, promoted ones included, in order. The derived JSON and
//!   database codecs read every property, private ones included, in this
//!   order (`rule:core-classes/derive-property-list`), so an added, removed or
//!   moved property changes this row whatever its visibility.
//! - **A member row** for each method ([`Kind::Method`], or
//!   [`Kind::AbstractMethod`] without a body), property ([`Kind::Property`],
//!   promoted constructor parameters included), constant ([`Kind::Constant`])
//!   and enum case ([`Kind::Case`]): its doc comment, modifiers, type, default,
//!   value and signature, and whether a method yields. A case without a value
//!   takes the last written value plus its distance from it, so its row has
//!   that value's text and the distance: inserting a case changes the rows of
//!   the cases whose value moved.
//!
//! Every row has a second hash, of the attribute groups written on it (on a
//! method, its parameters' too). A change to it is reported as
//! [`Kind::Attributes`], because routes, commands, links and `#[Deprecated]`
//! are read across the whole graph.
//!
//! A member's visibility is its `public`, `protected` or `private`, and
//! `public` where none is written. A header, cases and graph row is public. A
//! properties row is private: its reach is a private property's.
//!
//! # What the text includes
//!
//! The source text of the declaration, with each body, each attribute group
//! and each member below it cut out. A cut is written as a gap, as a skipped
//! comment is, so adding an attribute to a member or a member to a class
//! leaves the row's own text as it was. A header leaves out the `;` and `,`
//! that end the members and cases it cut, for the same reason. Whether a
//! method has a body is in its kind, and whether it yields is one byte of its
//! row. The text is written without its whitespace and its `//`, `#` and `/* */`
//! comments, which the grammar never reads. Each run of them between two
//! tokens is written as one gap byte, so `public function` and
//! `publicfunction` still differ. A blank line, an indent, a space at the end
//! of a line or a comment between two members therefore leaves every row as
//! it was, and queues no reader. A space between two tokens that had none,
//! such as `f (` for `f(`, still changes the row: that costs a reader's
//! analysis, and never a wrong answer. A `///` doc comment is written whole,
//! because the checker reads its tags.
//!
//! Text the walk does not cut out stays in the row that contains it: a member
//! kind added to the grammar later stays in its class's header row, and so
//! does a member the parser recovered from. Where it is unclear whether
//! something is visible, it is in a row, and in the widest row: one that holds
//! too much costs time, and one that misses something gives a wrong answer.
//! Two declarations with one symbol, which does not compile, share one row.
//!
//! # What it leaves out
//!
//! Every function, method and hook body without the word `require`, every
//! span, every whitespace run and comment the grammar skips, and every other
//! statement at file scope.
//!
//! # What it costs
//!
//! One pass over a file's declarations and over the trivia inside them, and
//! one BLAKE3 hash per row and one of all rows, per parse of a file. The parse
//! cache keeps the rows beside the parse, and the index reads the hash of all
//! rows from there. Per declaration: its symbol and two 32-byte hashes.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::mem;

use nvs_config::cache::{Digest, content_hash};
use nvs_diagnostics::{Diagnostics, SourceMap, Span};
use nvs_syntax::ast::{
    AttributeGroup, ClassMember, ClassMemberKind, DocComment, EnumDecl, Modifier, Param,
    PropertyHookBody, Stmt, StmtKind, Visibility, is_generator_body,
};
use nvs_syntax::{Trivia, TriviaKind};

/// The byte written for each run of skipped whitespace and comments between
/// two tokens. The text of a token never has it outside a literal, and a
/// literal is never trivia.
const GAP: u8 = 2;

/// What a row is the surface of, and what part of a symbol a change touched.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Kind {
    /// The file's `namespace`, `use`, `autoload` and `require` text. Its
    /// symbol is `""`.
    Graph,
    /// A class, interface or enum's header, or a `type` alias or `attribute`
    /// declaration whole.
    Header,
    /// The names of an enum's cases, in order.
    Cases,
    /// The names of a class's properties, in order.
    Properties,
    /// A method with a body.
    Method,
    /// A method without a body: abstract, or an interface's.
    AbstractMethod,
    /// A property, a promoted constructor parameter included.
    Property,
    /// A class constant.
    Constant,
    /// An enum case.
    Case,
    /// The attribute groups written on a symbol. Only a [`Change`] has this
    /// kind: a row keeps them as its second hash.
    Attributes,
}

/// One declaration's surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The symbol, as the index writes it.
    pub symbol: String,
    /// What the row is the surface of. Never [`Kind::Attributes`].
    pub kind: Kind,
    /// Who may see it.
    pub visibility: Visibility,
    /// The hash of its text.
    pub text: Digest,
    /// The hash of the attribute groups written on it.
    pub attributes: Digest,
}

impl Row {
    fn key(&self) -> (&str, Kind) {
        (&self.symbol, self.kind)
    }
}

/// Every row of one parse, sorted by symbol and kind, and the hash of them all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rows {
    rows: Vec<Row>,
    all: Digest,
}

impl Rows {
    /// The declaration-surface hash: the hash of every row.
    #[must_use]
    pub const fn digest(&self) -> Digest {
        self.all
    }

    /// Every row, sorted by symbol and kind.
    pub fn iter(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }
}

/// Whether a symbol is new, gone or different.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum How {
    /// Only the new parse has it.
    Added,
    /// Only the old parse has it.
    Removed,
    /// Both have it, with different text.
    Changed,
}

/// One symbol that differs between two parses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    /// The symbol, as the index writes it.
    pub symbol: String,
    /// The part of it that differs.
    pub kind: Kind,
    /// The wider of its old and new visibility.
    pub visibility: Visibility,
    /// Whether it was added, removed or changed.
    pub how: How,
}

/// The rows of `stmts`, parsed from `text` with `trivia`, the whitespace runs
/// and comments of that parse in source order.
#[must_use]
pub fn rows(stmts: &[Stmt], trivia: &[Trivia], text: &str) -> Rows {
    let mut surface = Surface {
        text,
        trivia,
        out: Vec::new(),
        rows: BTreeMap::new(),
        graph: Vec::new(),
    };
    surface.file_scope(stmts, &mut String::new());
    surface.out = mem::take(&mut surface.graph);
    surface.row(String::new(), Kind::Graph, Visibility::Public, Vec::new());
    let rows: Vec<Row> = surface
        .rows
        .into_iter()
        .map(|((symbol, kind), buf)| Row {
            symbol,
            kind,
            visibility: buf.visibility,
            text: content_hash(&buf.text),
            attributes: content_hash(&buf.attributes),
        })
        .collect();
    let mut all = Vec::with_capacity(rows.len() * 80);
    for row in &rows {
        all.extend_from_slice(row.symbol.as_bytes());
        all.extend([0, row.kind as u8, rank(row.visibility)]);
        all.extend_from_slice(row.text.as_bytes());
        all.extend_from_slice(row.attributes.as_bytes());
    }
    Rows {
        rows,
        all: content_hash(&all),
    }
}

/// The declaration-surface hash of `stmts`: the hash of all its [`rows`].
#[must_use]
pub fn of(stmts: &[Stmt], trivia: &[Trivia], text: &str) -> Digest {
    rows(stmts, trivia, text).digest()
}

/// The rows of `text`, parsed on its own.
#[must_use]
pub fn rows_of_source(text: &str) -> Rows {
    let mut map = SourceMap::new();
    let id = map.add("surface.nvs", text);
    let mut diags = Diagnostics::new();
    let parsed = nvs_syntax::parse(map.file(id), &mut diags);
    rows(&parsed.stmts, &parsed.trivia, text)
}

/// The declaration-surface hash of `text`, parsed on its own.
#[must_use]
pub fn of_source(text: &str) -> Digest {
    rows_of_source(text).digest()
}

/// Every symbol that `new` adds, removes or changes against `old`, sorted by
/// symbol and kind. A symbol whose text and attributes both changed gives two
/// changes, one of its own kind and one of [`Kind::Attributes`].
#[must_use]
pub fn diff(old: &Rows, new: &Rows) -> Vec<Change> {
    let mut changes = Vec::new();
    let mut before = old.rows.iter().peekable();
    let mut after = new.rows.iter().peekable();
    loop {
        let order = match (before.peek(), after.peek()) {
            (None, None) => break,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(old), Some(new)) => old.key().cmp(&new.key()),
        };
        match order {
            Ordering::Less => {
                if let Some(row) = before.next() {
                    changes.push(change(row, row.kind, row.visibility, How::Removed));
                }
            }
            Ordering::Greater => {
                if let Some(row) = after.next() {
                    changes.push(change(row, row.kind, row.visibility, How::Added));
                }
            }
            Ordering::Equal => {
                if let (Some(old), Some(new)) = (before.next(), after.next()) {
                    let visibility = wider(old.visibility, new.visibility);
                    if old.text != new.text {
                        changes.push(change(new, new.kind, visibility, How::Changed));
                    }
                    if old.attributes != new.attributes {
                        changes.push(change(new, Kind::Attributes, visibility, How::Changed));
                    }
                }
            }
        }
    }
    changes
}

fn change(row: &Row, kind: Kind, visibility: Visibility, how: How) -> Change {
    Change {
        symbol: row.symbol.clone(),
        kind,
        visibility,
        how,
    }
}

/// How far a visibility reaches: private, then protected, then public.
const fn rank(visibility: Visibility) -> u8 {
    match visibility {
        Visibility::Private => 0,
        Visibility::Protected => 1,
        Visibility::Public => 2,
    }
}

/// The one of `a` and `b` that reaches further.
const fn wider(a: Visibility, b: Visibility) -> Visibility {
    if rank(a) >= rank(b) { a } else { b }
}

/// The visibility `modifiers` declare, `public` where they declare none.
fn visibility(modifiers: &[Modifier]) -> Visibility {
    modifiers
        .iter()
        .find_map(|modifier| match modifier {
            Modifier::Public => Some(Visibility::Public),
            Modifier::Protected => Some(Visibility::Protected),
            Modifier::Private => Some(Visibility::Private),
            _ => None,
        })
        .unwrap_or(Visibility::Public)
}

/// The text from a declaration's doc comment, if it has one, to its end.
fn extent(doc: Option<&DocComment>, span: Span) -> Span {
    Span {
        start: doc.map_or(span.start, |doc| doc.span.start.min(span.start)),
        ..span
    }
}

/// Whether the walk writes a row for a member of this kind, and so cuts it
/// out of its owner's header.
const fn has_row(kind: &ClassMemberKind) -> bool {
    matches!(
        kind,
        ClassMemberKind::Method(_)
            | ClassMemberKind::Property(_)
            | ClassMemberKind::Const(_)
            | ClassMemberKind::TypeAlias(_)
            | ClassMemberKind::Enum(_)
            | ClassMemberKind::Attribute(_)
    )
}

/// One row's text while the walk writes it.
struct Buf {
    visibility: Visibility,
    text: Vec<u8>,
    attributes: Vec<u8>,
}

/// The rows written so far, and the source they are cut from.
struct Surface<'a> {
    text: &'a str,
    /// Every whitespace run and comment of the parse, in source order.
    trivia: &'a [Trivia],
    /// The text of the row being written. Empty between two rows.
    out: Vec<u8>,
    rows: BTreeMap<(String, Kind), Buf>,
    /// The graph row's text.
    graph: Vec<u8>,
}

impl<'a> Surface<'a> {
    /// The text of `span`.
    fn slice(&self, span: Span) -> &'a str {
        self.text
            .get(span.start as usize..span.end as usize)
            .unwrap_or_default()
    }

    /// `name` inside `namespace`.
    fn qualify(&self, namespace: &str, name: Span) -> String {
        let name = self.slice(name);
        if namespace.is_empty() {
            name.to_owned()
        } else {
            format!("{namespace}\\{name}")
        }
    }

    /// Files the text written since the last row as the row of `symbol` and
    /// `kind`, with `attributes` as its attribute text. A second declaration
    /// of one symbol adds to its row.
    fn row(&mut self, symbol: String, kind: Kind, visibility: Visibility, attributes: Vec<u8>) {
        let text = mem::take(&mut self.out);
        match self.rows.entry((symbol, kind)) {
            Entry::Vacant(vacant) => {
                vacant.insert(Buf {
                    visibility,
                    text,
                    attributes,
                });
            }
            Entry::Occupied(occupied) => {
                let buf = occupied.into_mut();
                buf.visibility = wider(buf.visibility, visibility);
                buf.text.extend(text);
                buf.attributes.extend(attributes);
            }
        }
    }

    /// The text of `groups`, for a row's attribute hash.
    fn groups<'g>(&mut self, groups: impl IntoIterator<Item = &'g AttributeGroup>) -> Vec<u8> {
        let row = mem::take(&mut self.out);
        for group in groups {
            self.write_cut(group.span, &[]);
        }
        mem::replace(&mut self.out, row)
    }

    /// Writes `span` with `cuts` cut out into the graph row.
    fn graph(&mut self, span: Span, cuts: &[Span]) {
        let row = mem::take(&mut self.out);
        self.write_cut(span, cuts);
        let text = mem::replace(&mut self.out, row);
        self.graph.extend(text);
    }

    /// Writes the text from `start` to `end` without the whitespace and the
    /// comments the grammar skips. A run of them between two pieces of text
    /// is one [`GAP`], and a run at either end writes nothing: a cut or the
    /// end marker stands there.
    fn write_text(&mut self, start: u32, end: u32) {
        let first = self
            .trivia
            .partition_point(|trivia| trivia.span.end <= start);
        let mut at = start;
        // Whether a piece of this range was written, and so whether a skipped
        // run is a gap before the next piece.
        let mut wrote = false;
        for trivia in &self.trivia[first..] {
            if trivia.span.start >= end {
                break;
            }
            if trivia.kind == TriviaKind::DocComment {
                continue;
            }
            let skip_from = trivia.span.start.max(at);
            if skip_from > at {
                self.write_piece(at, skip_from, wrote);
                wrote = true;
            }
            at = at.max(trivia.span.end.min(end));
        }
        let gap = wrote;
        if end > at {
            self.write_piece(at, end, gap);
        }
    }

    /// Writes the text from `start` to `end`, after a [`GAP`] when `gap`.
    fn write_piece(&mut self, start: u32, end: u32, gap: bool) {
        if gap {
            self.out.push(GAP);
        }
        let text = self
            .text
            .get(start as usize..end as usize)
            .unwrap_or_default();
        self.out.extend_from_slice(text.as_bytes());
    }

    /// Writes the text of `span` with each of `cuts` cut out, and an end
    /// marker. A cut between two pieces of text is written as one [`GAP`],
    /// as a skipped comment is, so a row does not count what was cut: adding
    /// an attribute to a member or a member to a class leaves the row's own
    /// text as it was. A cut outside `span` is ignored.
    fn write_cut(&mut self, span: Span, cuts: &[Span]) {
        self.write_without(span, cuts, true);
    }

    /// [`write_cut`](Self::write_cut), and where `separators` is false, a
    /// piece between two cuts that is only `;` and `,` is left out too: it is
    /// the end of a member or a case that was cut.
    fn write_without(&mut self, span: Span, cuts: &[Span], separators: bool) {
        let mut inside: Vec<Span> = cuts
            .iter()
            .filter(|cut| cut.start >= span.start && cut.end <= span.end && cut.start < cut.end)
            .copied()
            .collect();
        inside.sort_by_key(|cut| (cut.start, cut.end));
        let base = self.out.len();
        let mut at = span.start;
        for cut in inside {
            if cut.end <= at {
                continue;
            }
            self.write_piece_after_cut(at, cut.start, base, !separators);
            at = cut.end;
        }
        self.write_piece_after_cut(at, span.end, base, !separators);
        self.out.push(0);
    }

    /// Writes the text from `start` to `end`, after a [`GAP`] when text was
    /// written since `base`. Where `drop_separators`, a piece that is only
    /// `;` and `,` is not written.
    fn write_piece_after_cut(&mut self, start: u32, end: u32, base: usize, drop_separators: bool) {
        if end <= start {
            return;
        }
        let len = self.out.len();
        self.write_text(start, end);
        let piece = &self.out[len..];
        if drop_separators && piece.iter().all(|&byte| matches!(byte, b';' | b',' | GAP)) {
            self.out.truncate(len);
        } else if len > base && self.out.len() > len {
            self.out.insert(len, GAP);
        }
    }

    /// Writes a doc comment's text, which a hover in another file shows.
    fn doc(&mut self, doc: Option<&DocComment>) {
        if let Some(doc) = doc {
            self.write_cut(doc.span, &[]);
        }
    }

    fn file_scope(&mut self, stmts: &[Stmt], namespace: &mut String) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::ClassDecl(class) => {
                    let name = self.qualify(namespace, class.name.span);
                    self.class_like(
                        &name,
                        class.doc.as_ref(),
                        &class.attributes,
                        class.span,
                        &class.members,
                    );
                }
                StmtKind::InterfaceDecl(interface) => {
                    let name = self.qualify(namespace, interface.name.span);
                    self.class_like(
                        &name,
                        interface.doc.as_ref(),
                        &interface.attributes,
                        interface.span,
                        &interface.members,
                    );
                }
                StmtKind::EnumDecl(decl) => {
                    let name = self.qualify(namespace, decl.name.span);
                    self.enumeration(&name, decl, decl.doc.as_ref());
                }
                StmtKind::TypeAliasDecl(alias) => {
                    let name = self.qualify(namespace, alias.name.span);
                    self.doc(alias.doc.as_ref());
                    self.write_cut(alias.span, &[]);
                    self.row(name, Kind::Header, Visibility::Public, Vec::new());
                }
                // An attribute declaration is written whole: its sites and its
                // fields, with each field's doc comment, are what a use checks.
                StmtKind::AttributeDecl(decl) => {
                    let name = self.qualify(namespace, decl.name.span);
                    self.doc(decl.doc.as_ref());
                    self.write_cut(decl.span, &[]);
                    self.row(name, Kind::Header, Visibility::Public, Vec::new());
                }
                StmtKind::NamespaceDecl(decl) => {
                    let name = decl
                        .name
                        .map(|name| self.slice(name.span).to_owned())
                        .unwrap_or_default();
                    if let Some(body) = &decl.body {
                        let header = Span {
                            end: body.span.start,
                            ..stmt.span
                        };
                        self.graph(header, &[]);
                        self.file_scope(&body.stmts, &mut name.clone());
                    } else {
                        self.graph(stmt.span, &[]);
                        *namespace = name;
                    }
                }
                StmtKind::UseDecl(_)
                | StmtKind::AutoloadDecl(_)
                | StmtKind::TopLevelConst(_)
                | StmtKind::TopLevelFunction(_) => self.graph(stmt.span, &[]),
                _ => {
                    if self.slice(stmt.span).contains("require") {
                        self.graph(stmt.span, &[]);
                    }
                }
            }
        }
    }

    /// The header and properties rows of a class or interface, and its
    /// members' rows.
    fn class_like(
        &mut self,
        name: &str,
        doc: Option<&DocComment>,
        attributes: &[AttributeGroup],
        span: Span,
        members: &[ClassMember],
    ) {
        let attrs = self.groups(attributes);
        let mut cuts: Vec<Span> = attributes.iter().map(|group| group.span).collect();
        cuts.extend(
            members
                .iter()
                .filter(|member| has_row(&member.kind))
                .map(|member| extent(member.doc.as_ref(), member.span)),
        );
        self.doc(doc);
        self.write_without(span, &cuts, false);
        self.row(name.to_owned(), Kind::Header, Visibility::Public, attrs);
        let mut properties = Vec::new();
        self.members(name, members, &mut properties);
        self.out = properties;
        self.row(
            name.to_owned(),
            Kind::Properties,
            Visibility::Private,
            Vec::new(),
        );
    }

    /// The header and cases rows of an enum, a row per case, and its
    /// members' rows.
    fn enumeration(&mut self, name: &str, decl: &EnumDecl, doc: Option<&DocComment>) {
        let attrs = self.groups(&decl.attributes);
        let mut cuts: Vec<Span> = decl.attributes.iter().map(|group| group.span).collect();
        cuts.extend(
            decl.cases
                .iter()
                .map(|case| extent(case.doc.as_ref(), case.span)),
        );
        cuts.extend(
            decl.members
                .iter()
                .filter(|member| has_row(&member.kind))
                .map(|member| extent(member.doc.as_ref(), member.span)),
        );
        self.doc(doc);
        self.write_without(decl.span, &cuts, false);
        self.row(name.to_owned(), Kind::Header, Visibility::Public, attrs);

        for case in &decl.cases {
            self.out
                .extend_from_slice(self.slice(case.name.span).as_bytes());
            self.out.push(0);
        }
        self.row(name.to_owned(), Kind::Cases, Visibility::Public, Vec::new());

        // The last value written, and how many cases after it this one is.
        let mut written: Option<Span> = None;
        let mut distance: u32 = 0;
        for case in &decl.cases {
            let symbol = format!("{name}::{}", self.slice(case.name.span));
            let attrs = self.groups(&case.attributes);
            let cuts: Vec<Span> = case.attributes.iter().map(|group| group.span).collect();
            self.doc(case.doc.as_ref());
            self.write_cut(case.span, &cuts);
            if let Some(value) = &case.value {
                written = Some(value.span);
                distance = 0;
            } else {
                distance += 1;
                self.out.extend(distance.to_le_bytes());
                if let Some(value) = written {
                    self.write_cut(value, &[]);
                }
            }
            self.row(symbol, Kind::Case, Visibility::Public, attrs);
        }
        self.members(name, &decl.members, &mut Vec::new());
    }

    /// The rows of `members`, declared by `owner`. Each property's name is
    /// added to `properties`, in order.
    fn members(&mut self, owner: &str, members: &[ClassMember], properties: &mut Vec<u8>) {
        for member in members {
            let doc = member.doc.as_ref();
            match &member.kind {
                ClassMemberKind::Method(method) => {
                    for param in method.params.iter().filter(|param| param.is_promoted()) {
                        self.promoted(owner, param, properties);
                    }
                    let groups = method
                        .attributes
                        .iter()
                        .chain(method.params.iter().flat_map(|param| &param.attributes));
                    let mut cuts: Vec<Span> = groups.clone().map(|group| group.span).collect();
                    let attrs = self.groups(groups);
                    self.doc(doc);
                    if let Some(body) = &method.body {
                        self.out
                            .push(if is_generator_body(body) { b'g' } else { b'f' });
                        self.cut_body(body.span, &mut cuts);
                    }
                    let kind = if method.body.is_some() {
                        Kind::Method
                    } else {
                        Kind::AbstractMethod
                    };
                    self.write_cut(member.span, &cuts);
                    let symbol = format!("{owner}::{}", self.slice(method.name));
                    self.row(symbol, kind, visibility(&method.modifiers), attrs);
                }
                ClassMemberKind::Property(property) => {
                    let name = self.slice(property.name);
                    properties.extend_from_slice(name.as_bytes());
                    properties.push(0);
                    let hooks = property.hooks.iter().flatten();
                    let groups = property
                        .attributes
                        .iter()
                        .chain(hooks.clone().flat_map(|hook| {
                            hook.attributes
                                .iter()
                                .chain(hook.param.iter().flat_map(|param| &param.attributes))
                        }));
                    let mut cuts: Vec<Span> = groups.clone().map(|group| group.span).collect();
                    let attrs = self.groups(groups);
                    for hook in hooks {
                        match &hook.body {
                            Some(PropertyHookBody::Expr(expr)) => {
                                self.cut_body(expr.span, &mut cuts)
                            }
                            Some(PropertyHookBody::Block(block)) => {
                                self.cut_body(block.span, &mut cuts);
                            }
                            None => {}
                        }
                    }
                    self.doc(doc);
                    self.write_cut(member.span, &cuts);
                    self.row(
                        format!("{owner}::{name}"),
                        Kind::Property,
                        visibility(&property.modifiers),
                        attrs,
                    );
                }
                ClassMemberKind::Const(constant) => {
                    let cuts: Vec<Span> =
                        constant.attributes.iter().map(|group| group.span).collect();
                    let attrs = self.groups(&constant.attributes);
                    self.doc(doc);
                    self.write_cut(member.span, &cuts);
                    let symbol = format!("{owner}::{}", self.slice(constant.name));
                    self.row(
                        symbol,
                        Kind::Constant,
                        visibility(&constant.modifiers),
                        attrs,
                    );
                }
                ClassMemberKind::TypeAlias(alias) => {
                    self.doc(doc);
                    self.write_cut(member.span, &[]);
                    let symbol = format!("{owner}::{}", self.slice(alias.name.span));
                    self.row(symbol, Kind::Header, Visibility::Public, Vec::new());
                }
                ClassMemberKind::Attribute(decl) => {
                    self.doc(doc);
                    self.write_cut(member.span, &[]);
                    let symbol = format!("{owner}::{}", self.slice(decl.name.span));
                    self.row(symbol, Kind::Header, Visibility::Public, Vec::new());
                }
                ClassMemberKind::Enum(decl) => {
                    let symbol = format!("{owner}::{}", self.slice(decl.name.span));
                    self.enumeration(&symbol, decl, doc);
                }
                // A member the parser recovered from, and every member kind
                // added later, stays in its owner's header row.
                _ => {}
            }
        }
    }

    /// The property row of a promoted constructor parameter.
    fn promoted(&mut self, owner: &str, param: &Param, properties: &mut Vec<u8>) {
        let name = self.slice(param.name);
        properties.extend_from_slice(name.as_bytes());
        properties.push(0);
        let cuts: Vec<Span> = param.attributes.iter().map(|group| group.span).collect();
        let attrs = self.groups(&param.attributes);
        self.write_cut(param.span, &cuts);
        self.row(
            format!("{owner}::{name}"),
            Kind::Property,
            visibility(&param.modifiers),
            attrs,
        );
    }

    /// Cuts a function or hook body out of its member's row. A body whose
    /// text has the word `require` is written into the graph row.
    fn cut_body(&mut self, body: Span, cuts: &mut Vec<Span>) {
        if self.slice(body).contains("require") {
            self.graph(body, &[]);
        }
        cuts.push(body);
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, How, Kind, diff, rows_of_source};
    use nvs_syntax::ast::Visibility;

    /// A library file with one of each declaration the reach table names.
    const LIBRARY: &str = "<?nvs
namespace Framework;

use Framework\\Clock;

/// A record that is stored.
abstract class Record
{
    private const int SEED = 1;
    protected const int STEP = 2;
    public const int LIMIT = 3;

    private int $secret = 0;
    protected string $label = 'a';
    public int $count = 0;

    public function __construct(private int $id) {}

    /// Gives the next number.
    public function next(int $n): int
    {
        return $n + 1;
    }

    protected function step(): int
    {
        return 2;
    }

    private function seed(): int
    {
        return 1;
    }

    #[Core\\Deprecated]
    public function old(): int
    {
        return 0;
    }

    abstract public function name(): string;
}

interface Named
{
    public function name(): string;
}

enum Status: int
{
    case Draft = 1;
    case Live;
}
";

    /// The changes one replacement in [`LIBRARY`] makes.
    fn changes(from: &str, to: &str) -> Vec<Change> {
        assert!(LIBRARY.contains(from), "the edit finds `{from}`");
        let old = rows_of_source(LIBRARY);
        let new = rows_of_source(&LIBRARY.replacen(from, to, 1));
        diff(&old, &new)
    }

    fn one(symbol: &str, kind: Kind, visibility: Visibility, how: How) -> Vec<Change> {
        vec![Change {
            symbol: symbol.to_owned(),
            kind,
            visibility,
            how,
        }]
    }

    #[test]
    fn a_whitespace_or_comment_edit_gives_no_change() {
        assert_eq!(
            changes(
                "    public int $count",
                "\n    // A count.\n      public  int $count"
            ),
            []
        );
        assert_eq!(changes("return $n + 1;", "return $n + 2;"), []);
    }

    #[test]
    fn a_private_method_or_constant_is_a_private_change() {
        assert_eq!(
            changes("function seed(): int", "function seed(): ?int"),
            one(
                "Framework\\Record::seed",
                Kind::Method,
                Visibility::Private,
                How::Changed
            )
        );
        assert_eq!(
            changes("SEED = 1", "SEED = 5"),
            one(
                "Framework\\Record::SEED",
                Kind::Constant,
                Visibility::Private,
                How::Changed
            )
        );
    }

    #[test]
    fn a_private_property_type_is_a_private_change() {
        assert_eq!(
            changes("private int $secret", "private ?int $secret"),
            one(
                "Framework\\Record::$secret",
                Kind::Property,
                Visibility::Private,
                How::Changed
            )
        );
        assert_eq!(
            changes("private int $id", "private string $id"),
            [
                one(
                    "Framework\\Record::$id",
                    Kind::Property,
                    Visibility::Private,
                    How::Changed
                ),
                one(
                    "Framework\\Record::__construct",
                    Kind::Method,
                    Visibility::Public,
                    How::Changed
                ),
            ]
            .concat(),
            "a promoted parameter is a property, and in its constructor's signature"
        );
    }

    #[test]
    fn a_protected_member_is_a_protected_change() {
        assert_eq!(
            changes("function step(): int", "function step(int $by): int"),
            one(
                "Framework\\Record::step",
                Kind::Method,
                Visibility::Protected,
                How::Changed
            )
        );
        assert_eq!(
            changes("STEP = 2", "STEP = 4"),
            one(
                "Framework\\Record::STEP",
                Kind::Constant,
                Visibility::Protected,
                How::Changed
            )
        );
    }

    #[test]
    fn a_public_member_or_case_is_a_public_change() {
        assert_eq!(
            changes("next(int $n): int", "next(int $n, int $m = 0): int"),
            one(
                "Framework\\Record::next",
                Kind::Method,
                Visibility::Public,
                How::Changed
            )
        );
        assert_eq!(
            changes("case Draft = 1;", "case Draft = 7;"),
            vec![
                Change {
                    symbol: "Framework\\Status::Draft".to_owned(),
                    kind: Kind::Case,
                    visibility: Visibility::Public,
                    how: How::Changed,
                },
                Change {
                    symbol: "Framework\\Status::Live".to_owned(),
                    kind: Kind::Case,
                    visibility: Visibility::Public,
                    how: How::Changed,
                },
            ],
            "a case without a value moves with the value before it"
        );
    }

    #[test]
    fn a_property_visibility_is_in_its_row() {
        assert_eq!(
            changes("public int $count", "private int $count"),
            one(
                "Framework\\Record::$count",
                Kind::Property,
                Visibility::Public,
                How::Changed
            )
        );
    }

    #[test]
    fn an_added_member_is_added_and_a_property_moves_the_properties_row() {
        assert_eq!(
            changes(
                "    /// Gives the next number.",
                "    public function fresh(): int\n    {\n        return 5;\n    }\n\n    /// Gives the next number."
            ),
            one(
                "Framework\\Record::fresh",
                Kind::Method,
                Visibility::Public,
                How::Added
            )
        );
        assert_eq!(
            changes(
                "    public int $count = 0;",
                "    public int $count = 0;\n    private int $extra = 0;"
            ),
            vec![
                Change {
                    symbol: "Framework\\Record".to_owned(),
                    kind: Kind::Properties,
                    visibility: Visibility::Private,
                    how: How::Changed,
                },
                Change {
                    symbol: "Framework\\Record::$extra".to_owned(),
                    kind: Kind::Property,
                    visibility: Visibility::Private,
                    how: How::Added,
                },
            ]
        );
    }

    #[test]
    fn an_added_case_changes_the_cases_row() {
        assert_eq!(
            changes("    case Live;", "    case Live;\n    case Gone;"),
            vec![
                Change {
                    symbol: "Framework\\Status".to_owned(),
                    kind: Kind::Cases,
                    visibility: Visibility::Public,
                    how: How::Changed,
                },
                Change {
                    symbol: "Framework\\Status::Gone".to_owned(),
                    kind: Kind::Case,
                    visibility: Visibility::Public,
                    how: How::Added,
                },
            ]
        );
    }

    #[test]
    fn an_added_abstract_or_interface_method_is_abstract() {
        assert_eq!(
            changes(
                "    public function name(): string;\n}",
                "    public function name(): string;\n\n    public function tag(): string;\n}"
            ),
            one(
                "Framework\\Named::tag",
                Kind::AbstractMethod,
                Visibility::Public,
                How::Added
            )
        );
        assert_eq!(
            changes(
                "    abstract public function name(): string;",
                "    abstract public function name(): string;\n\n    abstract protected function kind(): int;"
            ),
            one(
                "Framework\\Record::kind",
                Kind::AbstractMethod,
                Visibility::Protected,
                How::Added
            )
        );
    }

    #[test]
    fn a_class_header_edit_is_a_header_change() {
        assert_eq!(
            changes(
                "abstract class Record\n",
                "abstract class Record implements Named\n"
            ),
            one(
                "Framework\\Record",
                Kind::Header,
                Visibility::Public,
                How::Changed
            )
        );
        assert_eq!(
            changes("/// A record that is stored.", "/// A stored record."),
            one(
                "Framework\\Record",
                Kind::Header,
                Visibility::Public,
                How::Changed
            )
        );
    }

    #[test]
    fn a_removed_member_is_removed() {
        assert_eq!(
            changes("    public const int LIMIT = 3;\n", ""),
            one(
                "Framework\\Record::LIMIT",
                Kind::Constant,
                Visibility::Public,
                How::Removed
            )
        );
    }

    #[test]
    fn a_member_attribute_edit_is_an_attributes_change() {
        assert_eq!(
            changes("    #[Core\\Deprecated]\n", ""),
            one(
                "Framework\\Record::old",
                Kind::Attributes,
                Visibility::Public,
                How::Changed
            )
        );
    }

    #[test]
    fn a_graph_edit_is_a_graph_change() {
        assert_eq!(
            changes("use Framework\\Clock;", "use Framework\\Timer;"),
            one("", Kind::Graph, Visibility::Public, How::Changed)
        );
    }
}
