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
