//! Which constructs a case covered, read off the analysis rather than declared.
//!
//! `rule:ide/lspt-coverage-is-inferred`: a case says what document it asks
//! about and what it asks of it, and never what it covers. The construct is
//! taken from the node the question actually landed on, so a case cannot claim
//! coverage it does not have and nobody keeps a list by hand.
//!
//! # The two axes
//!
//! The rows are [`Request::ALL`] — the closed set of
//! `rule:ide/the-request-set-is-closed`, so a request added there arrives here
//! as an empty row rather than as a row nobody notices is missing. The columns
//! are the grammar's own node kinds, taken from
//! [`Analysed::index`](crate::Analysed::index): **no construct is spelled in
//! this module**, so a production the parser grows is a column the matrix grows
//! the first time a case reaches it, and one it drops is a column that
//! disappears.
//!
//! # Where a construct comes from
//!
//! **A request asked at a cursor covers the construct the cursor resolved to**,
//! innermost — one cell per case, with the answer's own content unread. A case
//! that asks `definition` inside an `if` and freezes `none` has covered that
//! cell exactly as much as one that freezes a place: what was asked is the
//! coverage, and what came back is what `--EXPECT--` is for.
//!
//! **A request asked of the whole document covers every construct its answer
//! named.** Each range the answer carries is a position, and the node under
//! that position is the construct. That is what makes one `semanticTokens` case
//! cover a dozen cells while one `documentLink` case covers one — the
//! difference between the two requests, rather than a weighting invented here.
//!
//! The split is [`Request::takes_cursor`] and nothing else, which is the same
//! structural line the case format already draws: a document-wide request has
//! no cursor to be asked at, so there is no third kind of case to decide about.
//!
//! # A question inside no node
//!
//! An offset can be inside no production at all — between two statements, in
//! the run of trivia before the first one, or past the end of the file
//! (`rule:ide/the-index-answers-the-cursor`). Such a case covers nothing, and
//! the matrix reports it by name rather than dropping it: a case the grammar
//! has no word for is one a reader has to see, since it counts toward `N
//! passed` while filling no cell.
//!
//! # Decision: what "no empty cell" can mean, and what it cannot
//!
//! `every_request_answers_every_construct` reads this matrix, and the whole
//! cross product is not what it can ask for: a `documentLink` answer names a
//! `require`'s written path and nothing else, so the cell where that request
//! meets `ClassDecl` is empty in every corpus that could ever exist. A gate
//! demanding it would be red forever, and a gate with a hand-written list of
//! exemptions is the list `rule:ide/lspt-coverage-is-inferred` exists to
//! refuse. So the claim is split along the line the matrix is already built on:
//!
//! - **The vocabulary** is every construct any case reached — the corpus's
//!   own, growing as it grows, never a roster kept here.
//! - **A request asked at a cursor owes the whole vocabulary.** A cursor goes
//!   anywhere, so an empty cell in one of those rows is a case nobody
//!   wrote rather than an answer that cannot exist. `codeAction` is in this
//!   set and is not carved out: a case freezing that nothing is offered at a
//!   construct is what pins
//!   `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`
//!   everywhere the two fixes are not.
//! - **The document-wide requests owe the vocabulary between them**, and each
//!   owes a non-empty row of its own. What one of them can name is decided by
//!   what it answers, so no one of them can be held to the whole vocabulary —
//!   but a construct no whole-document request reaches anywhere is one the
//!   corpus only ever looks at through a cursor, which is the gap this half
//!   closes.
//!
//! Adding a case at a construct nobody had reached therefore obliges every
//! cursor row and one document-wide row, which is the ratchet: the corpus
//! cannot grow deep in one place without growing wide.
//!
//! # Only a passing case covers anything
//!
//! Coverage is credited by [`crate::suite`] for a case whose answer equalled
//! its `--EXPECT--`. A case whose expectation is not what the server says is
//! not a frozen claim about that construct — it is a failure — and crediting it
//! would let the matrix fill up with cells nothing actually pins.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use lsp_types::{DocumentSymbol, Position};
use nvs_diagnostics::{BytePos, PositionEncoding};

use crate::case::Request;
use crate::document::Analysed;
use crate::position::offset_at;
use crate::render::{self, Response};

/// What one answer covered: every construct it reached, sorted and deduplicated.
///
/// Sorted so a matrix built from a run reads the same on any host, and
/// deduplicated because a cell counts *cases*: a `semanticTokens` answer naming
/// forty variables has covered `Variable` once, and a count of forty would say
/// this corpus tests variables forty times when one case would go red.
#[must_use]
pub fn of(
    analysed: &Analysed,
    cursor: Option<BytePos>,
    response: &Response,
    encoding: PositionEncoding,
) -> Vec<&'static str> {
    let mut found = Vec::new();
    if let Some(offset) = cursor {
        found.extend(kind_at(analysed, offset));
    } else {
        let file = analysed.map.file(analysed.entry);
        for position in named(response) {
            found.extend(kind_at(analysed, offset_at(file, position, encoding)));
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// The production `offset` is directly inside, if it is inside one.
fn kind_at(analysed: &Analysed, offset: BytePos) -> Option<&'static str> {
    analysed.index.at(offset).innermost().map(|node| node.kind)
}

/// Every position in the **entry document** that `response` names.
///
/// Whole rather than a lookup table, so a response kind added to [`Response`]
/// is a build error here: a variant that answered ranges and was quietly read
/// as naming none would be a request whose row stopped growing without anything
/// saying so.
///
/// The variants that name nothing here say so rather than being left out. A
/// `definition` and a `references` answer places that are often in another
/// file, a `completion` item carries no range at all, and a `documentHighlight`
/// answers places in this document that are every one of them a use of the name
/// the cursor is already on — each of the four is asked at a cursor, so its
/// coverage comes from the cursor and this is never read for it.
fn named(response: &Response) -> Vec<Position> {
    match response {
        Response::Diagnostics(items) => items.iter().map(|item| item.range.start).collect(),
        Response::Hover(answer) => answer
            .iter()
            .filter_map(|hover| hover.range.map(|range| range.start))
            .collect(),
        Response::Definition(_)
        | Response::Completion(_)
        | Response::References(_)
        | Response::DocumentHighlight(_) => Vec::new(),
        Response::SemanticTokens(tokens) => render::absolute(tokens),
        Response::DocumentSymbol(symbols) => {
            let mut out = Vec::new();
            declared(symbols, &mut out);
            out
        }
        Response::SelectionRange(answer) => {
            let mut out = Vec::new();
            let mut step = answer.as_ref();
            while let Some(range) = step {
                out.push(range.range.start);
                step = range.parent.as_deref();
            }
            out
        }
        Response::FoldingRange(items) => items
            .iter()
            .map(|item| Position {
                line: item.start_line,
                character: item.start_character.unwrap_or(0),
            })
            .collect(),
        Response::DocumentLink(items) => items.iter().map(|item| item.range.start).collect(),
        Response::CodeAction(items) => items.iter().map(|item| item.range.start).collect(),
        Response::CodeLens(items) => items.iter().map(|item| item.range.start).collect(),
        Response::Hints(items) => items.iter().map(|item| item.position).collect(),
        Response::Redactions(items) => items.iter().map(|item| item.range.start).collect(),
        Response::Regions(items) => items.iter().map(|item| item.range.start).collect(),
    }
}

/// Where each symbol in an outline is named, children included.
///
/// The selection range rather than the whole range: a class's range covers its
/// methods, so taking it would land every symbol on the outermost declaration
/// and report one construct for a file full of them.
fn declared(symbols: &[DocumentSymbol], out: &mut Vec<Position>) {
    for symbol in symbols {
        out.push(symbol.selection_range.start);
        if let Some(children) = &symbol.children {
            declared(children, out);
        }
    }
}

/// What a whole corpus covered: one row per request, one column per construct.
///
/// The rows are fixed at [`Request::ALL`] and the columns are whatever the
/// corpus reached, which is the asymmetry the rule asks for: the request set is
/// closed and known in advance, and the construct set is the grammar's, which
/// this module refuses to hold a copy of.
#[derive(Debug, Clone)]
pub struct Matrix {
    /// One entry per request, in [`Request::ALL`] order, holding the number of
    /// cases that covered each construct.
    rows: Vec<BTreeMap<&'static str, usize>>,
    /// Cases whose answer reached no construct at all, in the order they ran.
    reached_nothing: Vec<(PathBuf, Request)>,
}

impl Default for Matrix {
    fn default() -> Self {
        Self::new()
    }
}

impl Matrix {
    /// An empty matrix with a row per request and no columns.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rows: vec![BTreeMap::new(); Request::ALL.len()],
            reached_nothing: Vec::new(),
        }
    }

    /// Credits one case's answer to the cells it reached.
    pub fn record(&mut self, path: &Path, request: Request, constructs: &[&'static str]) {
        if constructs.is_empty() {
            self.reached_nothing.push((path.to_path_buf(), request));
            return;
        }
        let row = &mut self.rows[row_of(request)];
        for construct in constructs {
            *row.entry(construct).or_default() += 1;
        }
    }

    /// Every construct any case reached, sorted — the matrix's columns.
    #[must_use]
    pub fn constructs(&self) -> Vec<&'static str> {
        let mut all: Vec<&'static str> = self
            .rows
            .iter()
            .flat_map(|row| row.keys().copied())
            .collect();
        all.sort_unstable();
        all.dedup();
        all
    }

    /// How many cases covered one cell.
    #[must_use]
    pub fn count(&self, request: Request, construct: &str) -> usize {
        self.rows[row_of(request)]
            .get(construct)
            .copied()
            .unwrap_or(0)
    }

    /// The cases that filled no cell, with the request each asked.
    #[must_use]
    pub fn reached_nothing(&self) -> &[(PathBuf, Request)] {
        &self.reached_nothing
    }

    /// The matrix as a table: a column per request, a row per construct.
    ///
    /// Constructs run down the page because there are far more of them than
    /// there are requests and the list grows, while the request set does not
    /// (`rule:ide/the-request-set-is-closed`). An empty cell is `.` rather than
    /// `0`, so the thing the guard reports is the thing the eye finds.
    #[must_use]
    pub fn render(&self) -> String {
        let constructs = self.constructs();
        let first = constructs
            .iter()
            .map(|construct| construct.len())
            .chain(std::iter::once("construct".len()))
            .max()
            .unwrap_or_default();

        let mut out = format!("{:first$}", "construct");
        for request in Request::ALL {
            let _ = write!(out, "  {}", request.name());
        }
        out.push('\n');

        for construct in &constructs {
            let _ = write!(out, "{construct:first$}");
            for request in Request::ALL {
                let count = self.count(*request, construct);
                let width = request.name().len();
                if count == 0 {
                    let _ = write!(out, "  {:>width$}", ".");
                } else {
                    let _ = write!(out, "  {count:>width$}");
                }
            }
            out.push('\n');
        }

        for (path, request) in &self.reached_nothing {
            let _ = writeln!(
                out,
                "reached no construct: {} ({})",
                path.display(),
                request.name()
            );
        }
        out
    }
}

/// Which row a request is, which is its place in the roster it comes from.
fn row_of(request: Request) -> usize {
    Request::ALL
        .iter()
        .position(|it| *it == request)
        .expect("a request is in the roster every request is taken from")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// The constructs one case's document and question come to.
    fn covered(document: &str, request: &str) -> Vec<&'static str> {
        let text = format!(
            "--TEST--\nwhat this question lands on\n--FILE--\n{document}--REQUEST--\n{request}\n\
             --EXPECT--\n"
        );
        let case = crate::Case::parse(Path::new("covered.lspt"), &text).expect("the case parses");
        crate::suite::answer(&case)
            .expect("the case is answered")
            .covered
    }

    /// `rule:ide/lspt-coverage-is-inferred`: the construct is the node the
    /// cursor resolved to, so two cases over one document that differ only in
    /// where the `<|>` sits cover two different cells — which is the whole of
    /// what "a case cannot claim coverage it does not have" means.
    #[test]
    fn a_cursor_covers_the_construct_it_landed_in() {
        let at_the_call = covered(
            "<?nvs\nclass User { public function greet(): string { return \"hi\"; } }\n\
             var $u = new User();\n$u->gre<|>et();\n",
            "hover",
        );
        assert_eq!(at_the_call, vec!["MethodCall"], "at the method call");

        let at_the_class = covered(
            "<?nvs\nclass Us<|>er { public function greet(): string { return \"hi\"; } }\n",
            "hover",
        );
        assert_eq!(at_the_class, vec!["ClassDecl"], "at the declaration");
    }

    /// A document-wide request has no cursor, so its coverage is what its
    /// answer named — and an outline names every declaration, which is why one
    /// case of it fills several cells at once.
    #[test]
    fn a_document_wide_answer_covers_what_it_named() {
        let outline = covered(
            "<?nvs\nclass User { public string $name; public function greet(): string { return \"hi\"; } }\n\
             function free(): int { return 1; }\n",
            "documentSymbol",
        );
        assert_eq!(outline, vec!["ClassDecl", "Function", "Method", "Property"]);

        // The same document asked a request whose answer names one kind of
        // thing covers one cell, and that difference is the requests', not a
        // weighting this module applied.
        let links = covered("<?nvs\nclass User {}\n", "documentLink");
        assert!(links.is_empty(), "{links:?}");
    }

    /// A question inside no production covers nothing rather than covering
    /// whatever happened to be nearest, and the matrix names the case.
    ///
    /// Inline HTML is *not* that case — it is a production of its own, so a
    /// cursor in it lands on `InlineHtml`. What is left over is the trivia
    /// between two nodes, which no span covers.
    #[test]
    fn a_question_inside_no_node_fills_no_cell() {
        let nowhere = covered("<?nvs\nvar $x = 1;\n<|>\nvar $y = 2;\n", "hover");
        assert!(nowhere.is_empty(), "{nowhere:?}");

        let mut matrix = Matrix::new();
        matrix.record(Path::new("gap.lspt"), Request::Hover, &nowhere);
        assert!(matrix.constructs().is_empty());
        assert_eq!(matrix.reached_nothing().len(), 1);
        assert!(
            matrix
                .render()
                .contains("reached no construct: gap.lspt (hover)"),
            "{}",
            matrix.render()
        );
    }

    /// The rows are the closed request set whatever the corpus did, so a
    /// request nobody wrote a case for is an empty column of `.` rather than a
    /// row missing from the table.
    #[test]
    fn every_request_is_a_column_of_the_table() {
        let mut matrix = Matrix::new();
        matrix.record(Path::new("one.lspt"), Request::Hover, &["ClassDecl"]);
        matrix.record(Path::new("two.lspt"), Request::Hover, &["ClassDecl"]);

        assert_eq!(matrix.count(Request::Hover, "ClassDecl"), 2);
        assert_eq!(matrix.count(Request::Definition, "ClassDecl"), 0);

        let rendered = matrix.render();
        for request in Request::ALL {
            assert!(rendered.contains(request.name()), "{rendered}");
        }
        assert!(rendered.contains('.'), "{rendered}");
    }
}
