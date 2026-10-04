//! Where a document may be collapsed: every construct that spans lines.
//!
//! `textDocument/foldingRange` is a projection like the outline is
//! (`rule:ide/the-request-set-is-closed`): a statement carries the span of
//! everything it covers, so a construct written over several lines is already
//! foldable and finding one needs neither a cursor nor a type. This module is
//! that walk over [`crate::analyse`]'s statements, and
//! [`crate::render`] freezes what it answers as one `L-L kind` line each.
//!
//! **The entry file only**, on [`crate::symbols::for_document`]'s terms: one
//! analysis reads a whole `require` graph and the request names one URI.
//!
//! **A fold ends one line short of its last, so the closing brace stays on
//! screen.** A client hides `start_line + 1` through `end_line`, so a range
//! reaching the closer collapses `class User {` and its `}` into one visible
//! line and hides the shape a reader folds *to*. It follows that a construct
//! covering fewer than three lines has nothing to fold — one line to keep, one
//! to hide, one to leave — and contributes no range rather than an empty one.
//!
//! **A construct with no closer includes its last line.** A run of `use`
//! declarations and a `switch` arm both end at content rather than at a brace,
//! and every line of one but the first is what folding it means to hide.
//!
//! **Two of LSP's three kinds are answered, and the third is a convention
//! rather than a construct.** An import run is `imports` and a comment block is
//! `comment`; a body has no kind in the protocol at all, which is what `-` is
//! in a rendering. `region` is a `#region`/`#endregion` marker pair written
//! inside comments, and nothing in this language has decided that spelling, so
//! a fold for one would be this module inventing a convention rather than
//! projecting the tree.
//!
//! **A comment block is a run of comment lines, and it comes out of the
//! trivia** the same parse produced (`rule:ide/one-grammar-one-tree`) — the
//! one fold in this module that is not a statement. Two rules make a run:
//!
//! - **A comment sharing its line with code is not part of one.** A fold hides
//!   every line but its first, so merging the trailing `// why` on two
//!   consecutive statements would hide the second statement with it. Only a
//!   comment with nothing but whitespace before it on its line starts or
//!   extends a run.
//! - **A run is comment lines with no gap.** One blank line ends it, because a
//!   reader who left one wrote two blocks. A block comment joins the run it
//!   touches rather than standing apart: `/* … */` above `// …` is one thing
//!   on screen and folds as one.
//!
//! **A body written inside an expression is not reached.** A closure's
//! `=> { ... }` and an anonymous class body are both foldable and both sit in
//! an `Expr`, which this walk does not descend into. Adding one here would be
//! a second expression walk beside the offset index
//! `rule:ide/the-index-answers-the-cursor` builds per analysis, so it waits for
//! that index rather than duplicating it.
//!
//! **A declaration the parser refuses is foldable anyway**, for
//! [`crate::symbols`]' reason: a top-level `function`
//! (`rule:classes/no-free-functions-or-constants`) is in the document whether
//! or not it is legal, and the diagnostic is what says it is refused.

use lsp_types::{FoldingRange, FoldingRangeKind};
use nvs_diagnostics::{BytePos, PositionEncoding, SourceFile, Span};
use nvs_syntax::ast::{
    Block, ClassMember, ClassMemberKind, MethodMember, PropertyHookBody, Stmt, StmtKind,
};
use nvs_syntax::{Trivia, TriviaKind};

use crate::document::Analysed;
use crate::position::position_at;

/// Every range the entry document of `analysed` folds at, in `encoding`.
///
/// Sorted by where a range starts and then where it ends, so what a `.lspt`
/// case freezes is the document's shape rather than the order this walk
/// happened to reach two constructs in.
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<FoldingRange> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let mut walk = Walk {
        file: analysed.map.file(analysed.entry),
        encoding,
        found: Vec::new(),
    };
    walk.stmts(&loaded.stmts);
    walk.comments(&analysed.trivia);
    walk.found
        .sort_by_key(|range| (range.start_line, range.end_line));
    walk.found
}

/// One walk's state: what turns a span into a line, and what it has found.
struct Walk<'a> {
    /// The entry file, whose text says where its lines begin.
    file: &'a SourceFile,
    /// The units a column is counted in — carried because
    /// [`position_at`] answers a whole [`lsp_types::Position`] and
    /// `rule:ide/positions-have-one-home` owns the arithmetic, not this module.
    encoding: PositionEncoding,
    /// The ranges so far, in walk order until [`for_document`] sorts them.
    found: Vec<FoldingRange>,
}

impl Walk<'_> {
    /// Every fold in one statement list, its import runs included.
    fn stmts(&mut self, stmts: &[Stmt]) {
        self.imports(stmts);
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    /// One range per maximal run of `use` declarations.
    ///
    /// The only place a range covers more than one statement, and the reason
    /// this walk sees a list rather than a statement at a time: an editor folds
    /// an import block as a block, and each `use` on its own is one line.
    fn imports(&mut self, stmts: &[Stmt]) {
        let mut run: Option<(Span, Span)> = None;
        for stmt in stmts {
            if matches!(stmt.kind, StmtKind::UseDecl(_)) {
                let first = run.map_or(stmt.span, |(first, _)| first);
                run = Some((first, stmt.span));
            } else if let Some((first, last)) = run.take() {
                self.import_run(first, last);
            }
        }
        if let Some((first, last)) = run {
            self.import_run(first, last);
        }
    }

    /// One `comment` range per run of comment lines.
    ///
    /// The trivia is the entry document's and is already in source order, so
    /// this is one pass: a comment alone on its line either continues the run
    /// below it or starts a new one, and anything else — a trailing comment, a
    /// whitespace run, a blank line — closes whatever was open.
    fn comments(&mut self, trivia: &[Trivia]) {
        let mut run: Option<(Span, u32)> = None;
        for trivium in trivia {
            if !matches!(
                trivium.kind,
                TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::DocComment
            ) || !self.alone_on_its_line(trivium.span)
            {
                continue;
            }
            let first_line = self.line(trivium.span.start);
            let last_line = self.line(trivium.span.end.saturating_sub(1));
            run = match run {
                Some((first, previous)) if first_line <= previous + 1 => Some((first, last_line)),
                Some((first, previous)) => {
                    self.comment_run(first, previous);
                    Some((trivium.span, last_line))
                }
                None => Some((trivium.span, last_line)),
            };
        }
        if let Some((first, last)) = run {
            self.comment_run(first, last);
        }
    }

    /// One run of comment lines, folded through its last.
    ///
    /// Through rather than one short, because a comment block has no closing
    /// line to leave on screen — even a `*/` is the end of the text rather than
    /// the shape a reader folds back to.
    fn comment_run(&mut self, first: Span, last_line: u32) {
        let start = self.line(first.start);
        if last_line > start {
            self.push(start, last_line, Some(FoldingRangeKind::Comment));
        }
    }

    /// Whether nothing but whitespace precedes `span` on its own line.
    fn alone_on_its_line(&self, span: Span) -> bool {
        let text = self.file.text();
        let before = &text[..span.start as usize];
        let line = before.rsplit_once('\n').map_or(before, |(_, tail)| tail);
        line.chars().all(char::is_whitespace)
    }

    /// One run of imports, as the one span it folds over.
    fn import_run(&mut self, first: Span, last: Span) {
        self.through(
            Span::new(first.file, first.start, last.end),
            Some(FoldingRangeKind::Imports),
        );
    }

    /// Every fold one statement holds, its own and its body's.
    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Block(block) => self.block(block),
            // A branch is a statement, so a braced one is folded by the arm
            // above and an unbraced one covers a line and folds at nothing.
            StmtKind::If { arms, else_ } => {
                for arm in arms {
                    self.stmt(&arm.then);
                }
                if let Some(otherwise) = else_ {
                    self.stmt(otherwise);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Foreach { body, .. } => self.stmt(body),
            // The one statement whose own span is what folds: a `switch` holds
            // its arms directly rather than a block, so there is no node for
            // the braces and the statement is them. An arm in turn reaches to
            // its last statement and not to a brace, so it folds through that
            // line.
            StmtKind::Switch { cases, .. } => {
                self.braced(stmt.span);
                for case in cases {
                    self.through(case.span, None);
                    self.stmts(&case.body);
                }
            }
            StmtKind::Try {
                body,
                catches,
                finally,
            } => {
                self.block(body);
                for catch in catches {
                    self.block(&catch.body);
                }
                if let Some(block) = finally {
                    self.block(block);
                }
            }
            // A declaration's own span is what folds, because the tree holds no
            // span for the body alone: it runs from the keyword through the
            // closing brace, and the header line it starts on is the line an
            // editor leaves on screen anyway.
            StmtKind::ClassDecl(decl) => {
                self.braced(decl.span);
                self.members(&decl.members);
            }
            StmtKind::InterfaceDecl(decl) => {
                self.braced(decl.span);
                self.members(&decl.members);
            }
            StmtKind::EnumDecl(decl) => {
                self.braced(decl.span);
                self.members(&decl.members);
            }
            // The bracketed form holds a block and folds; the statement form
            // has no body at all, and the rest of the file is not a region.
            StmtKind::NamespaceDecl(decl) => {
                if let Some(body) = &decl.body {
                    self.block(body);
                }
            }
            StmtKind::TopLevelFunction(method) => self.method(method),
            // A statement covering one line, or one whose body is an
            // expression — either way, nothing to fold.
            _ => {}
        }
    }

    /// Every fold a class, interface or enum body's members hold.
    fn members(&mut self, members: &[ClassMember]) {
        for member in members {
            match &member.kind {
                ClassMemberKind::Method(method) => self.method(method),
                // A hook's body is a block on the same terms a method's is;
                // the `{ get ... set ... }` around the pair is not a span the
                // tree keeps, so the property itself does not fold.
                ClassMemberKind::Property(property) => {
                    for hook in property.hooks.iter().flatten() {
                        if let Some(PropertyHookBody::Block(block)) = &hook.body {
                            self.block(block);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// A method's body, which an abstract one and an interface's signature
    /// do not have.
    fn method(&mut self, method: &MethodMember) {
        if let Some(body) = &method.body {
            self.block(body);
        }
    }

    /// A `{ ... }` block, and everything written inside it.
    fn block(&mut self, block: &Block) {
        self.braced(block.span);
        self.stmts(&block.stmts);
    }

    /// A range over a span whose last line closes it, which folds to
    /// everything between its first line and that one.
    fn braced(&mut self, span: Span) {
        let start = self.line(span.start);
        let last = self.line(span.end.saturating_sub(1));
        if last > start + 1 {
            self.push(start, last - 1, None);
        }
    }

    /// A range over a span whose last line is content, which folds through it.
    fn through(&mut self, span: Span, kind: Option<FoldingRangeKind>) {
        let start = self.line(span.start);
        let last = self.line(span.end.saturating_sub(1));
        if last > start {
            self.push(start, last, kind);
        }
    }

    /// One range, on the wire's 0-based lines.
    fn push(&mut self, start_line: u32, end_line: u32, kind: Option<FoldingRangeKind>) {
        self.found.push(FoldingRange {
            start_line,
            end_line,
            // No columns: a client that folds whole lines ignores them, and a
            // fold is a line-granular answer here by construction — which is
            // exactly why `rule:ide/redaction-covers-bytes-only` cannot be
            // built on one.
            start_character: None,
            end_character: None,
            kind,
            // The client's own placeholder, which is the one a reader of that
            // editor already recognises.
            collapsed_text: None,
        });
    }

    /// Which line `offset` is on, 0-based as the wire counts them.
    fn line(&self, offset: BytePos) -> u32 {
        position_at(self.file, offset, self.encoding).line
    }
}
