//! Order class members: the `source.orderMembers.nvs` code action, and the
//! function `nvs code order` calls.
//!
//! [`of`] writes the member order `rule:tooling/class-members-have-one-order`
//! gives to every class and interface in one file. The
//! editor's action and the command are two clients of it, so they write the
//! same bytes. `nvs fmt` never calls it
//! (`rule:tooling/fmt-never-reorders-members`), and no save runs the action:
//! [`crate::actions::Kind`] answers it only when a client names it or names
//! `source`.
//!
//! # What moves
//!
//! A member moves as whole lines. Its unit starts on the line after the
//! member above it ends, or after the body's `{`, so its `///` run, its
//! attributes and every comment above it move with it. It ends at the end of
//! its own last line, so a comment written after it on that line moves too.
//! What follows the last member, above the `}`, stays last. A joined
//! declaration (`public int $a, $b;`) is several members that start at one
//! byte, and it is one unit. A body this cannot cut into whole lines, because two
//! members share a line, is left as written.
//!
//! # The derived codec
//!
//! A class with `Core\Json\Derive` encodes its fields in the order
//! [`nvs_types::derive::field_order`] gives, which is the order the codec
//! check reads. With `Core\Json\KeepKeyOrder`, the instance properties keep
//! their written order, and a class whose field order would still change is
//! left as written, with a [`NoteKind::KeptAsWritten`]. Without it, a class
//! whose key order changes is ordered and gets a
//! [`NoteKind::KeyOrderChanges`]. Both attribute names are resolved through
//! the file's own namespace and `use` lines, as the compiler resolves them.
//!
//! # The output is formatted
//!
//! The moved text is run through [`nvs_fmt::format`], so the file written is
//! one `nvs fmt --check` accepts. A file that does not parse is refused, and
//! so is a moved text that does not parse, which names the class.
//!
//! # What it spends
//!
//! Per call, one parse of the file, one copy of its text with the members
//! moved, and one parse and one format of that copy. All of it is dropped
//! with the answer, and nothing here runs on the request path.

use nvs_diagnostics::{BytePos, Diagnostics, PositionEncoding, SourceFile, SourceMap, Span};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassMember, ClassMemberKind, Modifier, Stmt, StmtKind,
};
use nvs_syntax::{Trivia, TriviaKind};
use nvs_types::derive::{DERIVE, KEEP_KEY_ORDER, field_order};
use rustc_hash::FxHashMap;

use crate::document::Analysed;
use crate::organize::changed;
use crate::position::range_at;
use crate::render::Action;

/// The kind the action is filed under.
pub const KIND: &str = "source.orderMembers.nvs";

/// What the action is called in the editor.
const TITLE: &str = "Order class members";

/// What [`of`] wrote for one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ordered {
    /// The whole new file, formatted, and `None` where no class moves.
    pub text: Option<String>,
    /// Each class whose members moved, in the order the file declares them.
    pub moved: Vec<Class>,
    /// What a reader of the diff should know about a derived class.
    pub notes: Vec<Note>,
}

/// One class or interface of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    /// The full name.
    pub name: String,
    /// Where its declaration starts in the file [`of`] read.
    pub at: BytePos,
}

/// One note about a class with `Core\Json\Derive`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The class.
    pub class: Class,
    /// What happened to it.
    pub kind: NoteKind,
}

/// The two notes a derived class can get.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// The class was ordered, and its JSON key order changes.
    KeyOrderChanges,
    /// The class has `Core\Json\KeepKeyOrder`, and no member order keeps its
    /// keys, so it is left as written.
    KeptAsWritten,
}

/// Why [`of`] wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// The file has a syntax error, or the formatter does not accept it.
    DoesNotParse,
    /// The text with this class's members moved does not parse. The file is
    /// left as written.
    Unsafe(Class),
}

/// The member order of every class in `file`.
///
/// # Errors
///
/// [`Refused`] for a file that does not parse, and for one whose moved text
/// would not parse or format.
pub fn of(file: &SourceFile) -> Result<Ordered, Refused> {
    let mut diagnostics = Diagnostics::new();
    let parsed = nvs_syntax::parse(file, &mut diagnostics);
    if diagnostics.has_errors() {
        return Err(Refused::DoesNotParse);
    }
    let mut bodies = Vec::new();
    collect(&parsed.stmts, file, &mut Scope::default(), &mut bodies);

    let text = file.text();
    let mut plans = Vec::new();
    let mut moved = Vec::new();
    let mut notes = Vec::new();
    for body in &bodies {
        let Some(plan) = plan(body, text, &parsed.trivia, file) else {
            continue;
        };
        match plan {
            Planned::Kept => notes.push(Note {
                class: body.class.clone(),
                kind: NoteKind::KeptAsWritten,
            }),
            Planned::Moves(moves, key_order_changes) => {
                if key_order_changes {
                    notes.push(Note {
                        class: body.class.clone(),
                        kind: NoteKind::KeyOrderChanges,
                    });
                }
                moved.push(body.class.clone());
                plans.push(moves);
            }
        }
    }
    if plans.is_empty() {
        return Ok(Ordered {
            text: None,
            moved,
            notes,
        });
    }
    let rewritten = rewrite(text, &plans);

    let mut map = SourceMap::new();
    let id = map.add(file.name(), rewritten);
    // The formatter parses the moved text, and refuses it where it does not.
    let formatted =
        nvs_fmt::format(map.file(id)).map_err(|_| Refused::Unsafe(moved[0].clone()))?;
    Ok(Ordered {
        text: Some(formatted),
        moved,
        notes,
    })
}

/// The order-members action for the entry document of `analysed`, or `None`
/// where every class is already in order or the document does not parse.
///
/// The edit covers the whole changed region, for the reason
/// [`crate::organize`] gives.
#[must_use]
pub fn at(analysed: &Analysed, encoding: PositionEncoding) -> Option<Action> {
    let file = analysed.map.file(analysed.entry);
    let text = file.text();
    let ordered = of(file).ok()?.text?;
    if ordered == text {
        return None;
    }
    let (start, end) = changed(text, &ordered);
    let to = |at: usize| BytePos::try_from(at).unwrap_or(BytePos::MAX);
    Some(Action {
        title: TITLE.to_owned(),
        kind: KIND.to_owned(),
        range: range_at(
            file,
            Span::new(analysed.entry, to(start), to(end)),
            encoding,
        ),
        replacement: ordered[start..ordered.len() - (text.len() - end)].to_owned(),
        also: Vec::new(),
    })
}

/// One body whose members may move.
struct Body<'a> {
    class: Class,
    members: &'a [ClassMember],
    /// Where the declaration ends, its `}` included.
    end: BytePos,
    /// Whether it carries `Core\Json\Derive`.
    derive: bool,
    /// Whether it carries `Core\Json\KeepKeyOrder`.
    keep: bool,
}

/// The namespace and the imports in force where a declaration is written.
#[derive(Default, Clone)]
struct Scope {
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
}

impl Scope {
    /// Whether one of `groups` is the attribute `want`.
    fn carries(&self, groups: &[AttributeGroup], file: &SourceFile, want: &str) -> bool {
        let want = QName::parse(want);
        groups
            .iter()
            .flat_map(|group| &group.attributes)
            .any(|attr: &Attribute| {
                attr.member.is_none()
                    && attr.name.as_ref().is_some_and(|name| {
                        nvs_hir::resolve_ref(text_of(file, name.span), &self.namespace, &self.imports)
                            == want
                    })
            })
    }
}

/// Every class and interface `stmts` declares, in the order they are written.
///
/// A class is declared at the top of a file or of a namespace's block, and
/// nowhere else. Novis has no anonymous class (`E0244`), so no expression
/// declares one.
fn collect<'a>(stmts: &'a [Stmt], file: &SourceFile, scope: &mut Scope, out: &mut Vec<Body<'a>>) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(decl) => {
                let namespace = decl.name.as_ref().map_or_else(Vec::new, |name| {
                    QName::parse(text_of(file, name.span)).segments().to_vec()
                });
                match &decl.body {
                    Some(block) => {
                        let mut inner = Scope {
                            namespace,
                            imports: FxHashMap::default(),
                        };
                        collect(&block.stmts, file, &mut inner, out);
                    }
                    None => {
                        scope.namespace = namespace;
                        scope.imports.clear();
                    }
                }
            }
            StmtKind::UseDecl(decl) => {
                let target = QName::parse(text_of(file, decl.path.span));
                scope
                    .imports
                    .insert(target.short_name().to_owned(), target);
            }
            StmtKind::ClassDecl(decl) => {
                out.push(Body {
                    class: Class {
                        name: QName::join(&scope.namespace, text_of(file, decl.name.span))
                            .to_string(),
                        at: decl.span.start,
                    },
                    members: &decl.members,
                    end: decl.span.end,
                    derive: scope.carries(&decl.attributes, file, DERIVE),
                    keep: scope.carries(&decl.attributes, file, KEEP_KEY_ORDER),
                });
            }
            StmtKind::InterfaceDecl(decl) => {
                out.push(Body {
                    class: Class {
                        name: QName::join(&scope.namespace, text_of(file, decl.name.span))
                            .to_string(),
                        at: decl.span.start,
                    },
                    members: &decl.members,
                    end: decl.span.end,
                    derive: false,
                    keep: false,
                });
            }
            _ => {}
        }
    }
}

/// The text `span` covers.
fn text_of(file: &SourceFile, span: Span) -> &str {
    file.span_text(span).unwrap_or_default()
}

/// What one body's order comes to.
enum Planned {
    /// The members move, and `true` where a derived class's key order changes.
    Moves(Plan, bool),
    /// A `KeepKeyOrder` class no member order keeps, left as written.
    Kept,
}

/// The bytes one body rewrites: the region its members' units cover, and
/// those units in their new order.
struct Plan {
    region: (usize, usize),
    units: Vec<(usize, usize)>,
}

/// One member, or one joined declaration, as it moves.
struct Unit<'a> {
    /// Its first member, which its sort key reads.
    first: &'a ClassMember,
    /// Every member it is: more than one for a joined declaration.
    members: &'a [ClassMember],
    /// Its whole lines.
    bytes: (usize, usize),
}

/// How `body` is ordered, or `None` where it is in order already or its
/// members cannot be cut into whole lines.
fn plan(body: &Body<'_>, text: &str, trivia: &[Trivia], file: &SourceFile) -> Option<Planned> {
    let units = units(body, text, trivia)?;
    let keys: Vec<Key> = units
        .iter()
        .map(|unit| key(unit.first, body.keep, file))
        .collect();
    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by_key(|&at| keys[at]);
    if order.iter().enumerate().all(|(at, &unit)| at == unit) {
        return None;
    }
    let names = |members: &mut dyn Iterator<Item = &ClassMember>| -> Vec<String> {
        field_order(members, file)
            .iter()
            .map(|field| text_of(file, field.name).to_owned())
            .collect()
    };
    let key_order_changes = body.derive && {
        let before = names(&mut units.iter().flat_map(|unit| unit.members));
        let after = names(&mut order.iter().flat_map(|&at| units[at].members));
        before != after
    };
    if key_order_changes && body.keep {
        return Some(Planned::Kept);
    }
    let region = (units[0].bytes.0, units[units.len() - 1].bytes.1);
    let units = order.iter().map(|&at| units[at].bytes).collect();
    Some(Planned::Moves(Plan { region, units }, key_order_changes))
}

/// `body`'s members as whole-line units, or `None` where two of them share a
/// line, a member shares a line with a brace, or a line ends inside a block
/// comment.
fn units<'a>(body: &Body<'a>, text: &str, trivia: &[Trivia]) -> Option<Vec<Unit<'a>>> {
    let first = body.members.first()?;
    let open = before_trivia(first.span.start as usize, text, trivia);
    if open == 0 || text.as_bytes()[open - 1] != b'{' {
        return None;
    }
    let mut start = line_end(open, text);
    let mut units: Vec<Unit<'a>> = Vec::new();
    let mut at = 0;
    while at < body.members.len() {
        let member = &body.members[at];
        let joined = body.members[at..]
            .iter()
            .take_while(|other| other.span.start == member.span.start)
            .count();
        if (member.span.start as usize) < start {
            return None;
        }
        let last = &body.members[at + joined - 1];
        let end = line_end(last.span.end as usize, text);
        if inside_block_comment(end, trivia) {
            return None;
        }
        units.push(Unit {
            first: member,
            members: &body.members[at..at + joined],
            bytes: (start, end),
        });
        start = end;
        at += joined;
    }
    let close = (body.end as usize).checked_sub(1)?;
    (start <= close && text.as_bytes().get(close) == Some(&b'}')).then_some(units)
}

/// Where the run of whitespace and comments that ends at `at` starts.
fn before_trivia(mut at: usize, text: &str, trivia: &[Trivia]) -> usize {
    loop {
        if at > 0 && text.as_bytes()[at - 1].is_ascii_whitespace() {
            at -= 1;
            continue;
        }
        let ends_here = trivia.partition_point(|run| (run.span.end as usize) < at);
        match trivia.get(ends_here) {
            Some(run) if run.span.end as usize == at && (run.span.start as usize) < at => {
                at = run.span.start as usize;
            }
            _ => return at,
        }
    }
}

/// The first byte of the line after the one `at` is on, or the end of `text`.
fn line_end(at: usize, text: &str) -> usize {
    text[at..].find('\n').map_or(text.len(), |newline| at + newline + 1)
}

/// Whether `at` falls inside a block comment, after its first byte.
fn inside_block_comment(at: usize, trivia: &[Trivia]) -> bool {
    let after = trivia.partition_point(|run| (run.span.start as usize) < at);
    after > 0 && {
        let run = &trivia[after - 1];
        run.kind == TriviaKind::BlockComment && at < run.span.end as usize
    }
}

/// A member's place in the order, smallest first: its group, then static
/// before instance, then the constructor first among instance methods, then
/// its visibility, then `abstract` before `final` before the rest.
type Key = (u8, u8, u8, u8, u8);

/// [`Key`] for `member`. Under `KeepKeyOrder` an instance property is keyed
/// by its group alone, so the stable sort keeps the written order among them.
fn key(member: &ClassMember, keep: bool, file: &SourceFile) -> Key {
    let (group, modifiers, constructor) = match &member.kind {
        ClassMemberKind::Enum(_) | ClassMemberKind::TypeAlias(_) | ClassMemberKind::Error => {
            return (0, 0, 0, 0, 0);
        }
        ClassMemberKind::Const(constant) => (1, constant.modifiers.as_slice(), false),
        ClassMemberKind::Property(property) => (2, property.modifiers.as_slice(), false),
        ClassMemberKind::Method(method) => (
            3,
            method.modifiers.as_slice(),
            text_of(file, method.name) == "constructor",
        ),
        _ => return (4, 0, 0, 0, 0),
    };
    let instance = u8::from(!modifiers.contains(&Modifier::Static));
    if constructor {
        return (group, instance, 0, 0, 0);
    }
    if keep && group == 2 && instance == 1 {
        return (group, instance, 1, 0, 0);
    }
    let visibility = modifiers
        .iter()
        .find_map(|modifier| match modifier {
            Modifier::Public => Some(0),
            Modifier::Protected => Some(1),
            Modifier::Private => Some(2),
            _ => None,
        })
        .unwrap_or(0);
    let shape = if modifiers.contains(&Modifier::Abstract) {
        0
    } else if modifiers.contains(&Modifier::Final) {
        1
    } else {
        2
    };
    (group, instance, 1, visibility, shape)
}

/// `text` with each plan's units written in their new order. The plans are
/// in file order, and no two overlap, because no class is declared inside
/// another.
fn rewrite(text: &str, plans: &[Plan]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for plan in plans {
        out.push_str(&text[at..plan.region.0]);
        for &(start, end) in &plan.units {
            out.push_str(&text[start..end]);
        }
        at = plan.region.1;
    }
    out.push_str(&text[at..]);
    out
}
