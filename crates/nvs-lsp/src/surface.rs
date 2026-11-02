//! The declaration surface of a file: what another file can see of it, hashed.
//!
//! A keystroke inside a method body changes nothing another file of the graph
//! resolves against, so the entries that read the file need no new analysis.
//! [`of`] hashes the part of a parsed file that another file can see, and
//! [`crate::SymbolIndex::refresh`] queues a file's readers only when the hash
//! of the file it re-indexed differs from the one it had.
//!
//! # What the hash includes
//!
//! The source text of every declaration at file scope, with the bodies below
//! cut out, and with a marker where each cut was:
//!
//! - `namespace`, `use`, `autoload` and `type` lines, and file-scope constants;
//! - every class, interface and enum: its doc comment, attributes, header,
//!   cases with their values, and each member's doc comment, attributes,
//!   modifiers, type, default, constant value and signature, and each property
//!   hook's signature;
//! - every function's signature, top-level ones included;
//! - every other file-scope statement whose text has the word `require`, which
//!   is every file-scope `require`.
//!
//! The hash is of text, so it also changes for a comment or a space between
//! two members. That costs a reader's analysis, and never a wrong answer.
//!
//! # What it leaves out
//!
//! Every function, method and hook body, every span, and every other statement
//! at file scope. A body stays in the hash when its text has the word
//! `require`, because a `require` anywhere loads a file into the graph, and
//! whether a body yields is in the hash, because a generator's return type
//! is not its body's business alone. Where it is unclear whether something is
//! visible, it is in the hash: one that holds too much costs time, and one that
//! misses something gives a wrong answer.
//!
//! # What it costs
//!
//! One pass over a file's declarations and one BLAKE3 hash of their text, per
//! file an analysis indexes, and 32 bytes per indexed file.

use nvs_config::cache::{Digest, content_hash};
use nvs_diagnostics::{Diagnostics, SourceMap, Span};
use nvs_syntax::ast::{
    Block, ClassMember, ClassMemberKind, DocComment, PropertyHookBody, Stmt, StmtKind,
    is_generator_body,
};

/// The declaration-surface hash of `stmts`, parsed from `text`.
#[must_use]
pub fn of(stmts: &[Stmt], text: &str) -> Digest {
    let mut surface = Surface {
        text,
        out: Vec::new(),
    };
    surface.file_scope(stmts);
    content_hash(&surface.out)
}

/// The declaration-surface hash of `text`, parsed on its own.
#[must_use]
pub fn of_source(text: &str) -> Digest {
    let mut map = SourceMap::new();
    let id = map.add("surface.nvs", text);
    let mut diags = Diagnostics::new();
    let parsed = nvs_syntax::parse(map.file(id), &mut diags);
    of(&parsed.stmts, text)
}

/// The text written so far, and the source it is cut from.
struct Surface<'a> {
    text: &'a str,
    out: Vec<u8>,
}

impl<'a> Surface<'a> {
    /// The text of `span`.
    fn slice(&self, span: Span) -> &'a str {
        self.text
            .get(span.start as usize..span.end as usize)
            .unwrap_or_default()
    }

    /// Writes the text of `span` with each of `cuts` replaced by a marker.
    /// `cuts` is in source order and inside `span`.
    fn write_cut(&mut self, span: Span, cuts: &[Span]) {
        let mut at = span.start;
        for cut in cuts {
            let before = Span { end: cut.start, start: at, ..span };
            self.out.extend_from_slice(self.slice(before).as_bytes());
            self.out.push(1);
            at = cut.end;
        }
        let rest = Span { start: at, ..span };
        self.out.extend_from_slice(self.slice(rest).as_bytes());
        self.out.push(0);
    }

    /// Writes a doc comment's text, which a hover in another file shows.
    fn doc(&mut self, doc: Option<&DocComment>) {
        if let Some(doc) = doc {
            self.write_cut(doc.span, &[]);
        }
    }

    fn file_scope(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            let mut cuts = Vec::new();
            match &stmt.kind {
                StmtKind::ClassDecl(class) => {
                    self.doc(class.doc.as_ref());
                    self.members(&class.members, &mut cuts);
                }
                StmtKind::InterfaceDecl(interface) => {
                    self.doc(interface.doc.as_ref());
                    self.members(&interface.members, &mut cuts);
                }
                StmtKind::EnumDecl(decl) => {
                    self.doc(decl.doc.as_ref());
                    for case in &decl.cases {
                        self.doc(case.doc.as_ref());
                    }
                    self.members(&decl.members, &mut cuts);
                }
                StmtKind::TopLevelFunction(function) => {
                    if let Some(body) = &function.body {
                        self.body(body, &mut cuts);
                    }
                }
                StmtKind::NamespaceDecl(namespace) => {
                    if let Some(body) = &namespace.body {
                        let header = Span {
                            end: body.span.start,
                            ..stmt.span
                        };
                        self.write_cut(header, &[]);
                        self.file_scope(&body.stmts);
                        continue;
                    }
                }
                StmtKind::UseDecl(_)
                | StmtKind::AutoloadDecl(_)
                | StmtKind::TypeAliasDecl(_)
                | StmtKind::TopLevelConst(_) => {}
                _ => {
                    if !self.slice(stmt.span).contains("require") {
                        continue;
                    }
                }
            }
            cuts.sort_by_key(|cut| cut.start);
            self.write_cut(stmt.span, &cuts);
        }
    }

    /// Adds the bodies of `members` to `cuts`, and writes their doc comments.
    fn members(&mut self, members: &[ClassMember], cuts: &mut Vec<Span>) {
        for member in members {
            self.doc(member.doc.as_ref());
            match &member.kind {
                ClassMemberKind::Method(method) => {
                    if let Some(body) = &method.body {
                        self.body(body, cuts);
                    }
                }
                ClassMemberKind::Property(property) => {
                    for hook in property.hooks.iter().flatten() {
                        let span = match &hook.body {
                            Some(PropertyHookBody::Expr(expr)) => expr.span,
                            Some(PropertyHookBody::Block(block)) => block.span,
                            None => continue,
                        };
                        if !self.slice(span).contains("require") {
                            cuts.push(span);
                        }
                    }
                }
                ClassMemberKind::Enum(decl) => {
                    self.doc(decl.doc.as_ref());
                    for case in &decl.cases {
                        self.doc(case.doc.as_ref());
                    }
                    self.members(&decl.members, cuts);
                }
                // A constant, a type alias and every member kind added later
                // are written whole.
                _ => {}
            }
        }
    }

    /// Adds a function body to `cuts`, unless it has the word `require`, and
    /// writes whether it yields.
    fn body(&mut self, body: &Block, cuts: &mut Vec<Span>) {
        self.out.push(if is_generator_body(body) { b'g' } else { b'f' });
        if !self.slice(body.span).contains("require") {
            cuts.push(body.span);
        }
    }
}
