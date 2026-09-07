//! The parser's unit tests, one module per grammar layer, mirroring
//! [`super`]'s own split — and the helpers all four share.
//!
//! These stay inside the crate rather than moving to `tests/` because they
//! drive the parser below its public surface: a test that wants one
//! expression constructs a [`Parser`] and calls
//! [`Parser::parse_expr`](super::Parser::parse_expr) directly after bumping
//! the open tag, which no integration test can do.

mod decl;
mod expr;
mod inout;
mod stmt;
mod ty;

use nvs_diagnostics::SourceMap;

use super::*;

/// Parses `src` as an expression (wrapped in `<?nvs `) and asserts no
/// diagnostics were reported.
fn parse_ok(src: &str) -> Expr {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", format!("<?nvs {src}"));
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump(); // OpenTagNvs
    let e = p.parse_expr();
    assert!(
        !diags.has_errors(),
        "unexpected diagnostics for {src:?}: {diags:?}"
    );
    e
}

/// Parses `src` as an expression and returns it along with whatever
/// diagnostics were reported, for tests that expect a reported error.
fn parse_with_diags(src: &str) -> (Expr, Diagnostics) {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", format!("<?nvs {src}"));
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump(); // OpenTagNvs
    let e = p.parse_expr();
    (e, diags)
}

fn text(map: &SourceMap, file: nvs_diagnostics::SourceId, span: Span) -> &str {
    map.get(file).and_then(|f| f.span_text(span)).unwrap_or("")
}

/// Parses `src` as one statement (wrapped in `<?nvs `) and asserts no
/// diagnostics were reported.
fn parse_stmt_ok(src: &str) -> Stmt {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", format!("<?nvs {src}"));
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump(); // OpenTagNvs
    let s = p.parse_statement();
    assert!(
        !diags.has_errors(),
        "unexpected diagnostics for {src:?}: {diags:?}"
    );
    s
}

/// Parses `src` as one statement and returns it along with whatever
/// diagnostics were reported, for tests that expect a reported error.
fn parse_stmt_with_diags(src: &str) -> (Stmt, Diagnostics) {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", format!("<?nvs {src}"));
    let mut diags = Diagnostics::new();
    let mut p = Parser::new(map.file(id), &mut diags);
    p.bump();
    let s = p.parse_statement();
    (s, diags)
}

fn parse_file_ok(src: &str) -> Vec<Stmt> {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src.to_string());
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    assert!(
        !diags.has_errors(),
        "unexpected diagnostics for {src:?}: {diags:?}"
    );
    stmts
}

/// The one test this file holds rather than routes to: it is about the pair of
/// entry points themselves, which no grammar module owns.
#[test]
fn the_strict_entry_point_reports_exactly_what_it_reported_before() {
    // `rule:ide/one-grammar-one-tree`: [`parse`] is [`parse_file`] plus what it
    // kept, so the two must agree statement for statement and diagnostic for
    // diagnostic on the same source — a clean file, a refused construct, a file
    // that recovers, and one that never opens a tag. Every compile path calls
    // the strict half, so a refactor that lets the lossless half drift is the
    // one this goal is closest to.
    for src in [
        "<?nvs echo 1 + 2;",
        "<?nvs class C { public int $n = 1; public function m(): int { return $this->n; } }",
        "<?nvs $u->",
        "<?nvs echo (int) $x;",
        "<?nvs // a comment and nothing else\n",
        "plain html, no open tag at all",
    ] {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", src.to_string());

        let mut strict_diags = Diagnostics::new();
        let strict = parse_file(map.file(id), &mut strict_diags);
        let mut lossless_diags = Diagnostics::new();
        let lossless = parse(map.file(id), &mut lossless_diags);

        assert_eq!(
            format!("{strict:?}"),
            format!("{:?}", lossless.stmts),
            "the two entry points parsed {src:?} differently"
        );
        assert_eq!(
            format!("{strict_diags:?}"),
            format!("{lossless_diags:?}"),
            "the two entry points diagnosed {src:?} differently"
        );
    }
}
