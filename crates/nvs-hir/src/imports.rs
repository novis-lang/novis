//! Where a `use` line goes in a file, and which declarations a short name
//! could be importing.
//!
//! Three readers write a `use` line into a file somebody else is editing: the
//! fix [`crate::hierarchy::undeclared_name`] attaches to an undeclared class
//! name, the completion item an editor accepts for a type no short name
//! reaches, and the paste that carries a copied name's import along with it
//! (`rule:ide/a-pasted-type-carries-its-use-line`). All three have to agree on
//! where the line lands and how it is spelled, or a file that took one from
//! each ends up with three groups of imports — so the answer is written once,
//! here, in the crate every one of them already depends on.
//!
//! **After the last `use` the namespace already has**, which keeps the group
//! together; failing that after the `namespace Name;` line in force, with a
//! blank line between; failing that right after the open tag. Every one of
//! those is above the position asked about, so an edit made here never touches
//! the text the asker is itself writing. A bracketed namespace with no `use` in
//! it and a file with no open tag — a shebang script — have no such line, and
//! there the answer is `None`: a reader then writes the qualified name instead,
//! and never invents a place.
//!
//! The open tag is found by `nvs_syntax::first_open_tag`, the lexer's own test
//! over the text, rather than by tokenizing the file: this is asked on the
//! checker's error path and at every class declaration the hierarchy collects,
//! and a tokenize per class would be a compile-time cost paid for a fix nobody
//! may ever accept.

use nvs_diagnostics::{BytePos, SourceFile, SourceId, Span};
use nvs_syntax::ast::{Stmt, StmtKind};
use nvs_syntax::first_open_tag;

use crate::qname::QName;
use crate::symbol::{SymbolKind, SymbolTable};

/// Where a `use` line is added to a file, and what separates it from the line
/// it follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportSite {
    /// The offset the line is inserted at: the end of the line it follows.
    pub at: BytePos,
    /// What is written before the declaration: the line break that ends the
    /// line it follows, and a blank line where it starts a group of its own.
    pub lead: &'static str,
}

impl ImportSite {
    /// The empty span at [`Self::at`] in `file`, which is what an edit that
    /// inserts here replaces.
    #[must_use]
    pub const fn span(&self, file: SourceId) -> Span {
        Span {
            file,
            start: self.at,
            end: self.at,
        }
    }

    /// The text that imports `symbol` here: the lead, then `use Name;`.
    #[must_use]
    pub fn use_line(&self, symbol: &QName) -> String {
        format!("{}use {symbol};", self.lead)
    }

    /// The text that imports every one of `symbols` here, one `use` line each
    /// in the order given, or an empty string for none.
    #[must_use]
    pub fn use_lines<'a>(&self, symbols: impl IntoIterator<Item = &'a QName>) -> String {
        let lines: Vec<String> = symbols
            .into_iter()
            .map(|symbol| format!("use {symbol};"))
            .collect();
        if lines.is_empty() {
            return String::new();
        }
        format!("{}{}", self.lead, lines.join("\n"))
    }
}

/// Where a `use` line goes for a name written at `at` in the file whose
/// top-level statements are `stmts`, or `None` where this module will not
/// choose. The module doc is the order.
#[must_use]
pub fn import_site(stmts: &[Stmt], src: &SourceFile, at: BytePos) -> Option<ImportSite> {
    // The statements the position's namespace is made of: a bracketed block's
    // own where the position is inside one, and otherwise the file's, from
    // the `namespace Name;` line in force onward.
    let block = stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::NamespaceDecl(decl) => decl
            .body
            .as_ref()
            .filter(|block| block.span.start <= at && at < block.span.end),
        _ => None,
    });
    match block {
        Some(block) => site_in(&block.stmts, src, at, true),
        None => site_in(stmts, src, at, false),
    }
}

/// [`import_site`] over one statement sequence, which is the whole file or the
/// block of a bracketed namespace: `bracketed` says which, because a block
/// with no `use` in it has no line to write after and a file falls back to
/// its open tag.
#[must_use]
pub fn site_in(
    stmts: &[Stmt],
    src: &SourceFile,
    at: BytePos,
    bracketed: bool,
) -> Option<ImportSite> {
    let mut above = SitesAbove::default();
    for stmt in stmts.iter().filter(|stmt| stmt.span.end <= at) {
        above.pass(stmt);
    }
    above.site(src, at, bracketed)
}

/// What [`site_in`] reads off the statements above a position: where the
/// `namespace Name;` line in force ends, and where the last `use` after it
/// ends.
///
/// A walk over one statement sequence passes each statement to this as it goes,
/// so a walk that asks at every declaration reads each statement once. Asking
/// [`site_in`] at each one would read every statement above it again, and a
/// file of n classes would cost O(n²).
#[derive(Debug, Clone, Copy, Default)]
pub struct SitesAbove {
    governing: Option<BytePos>,
    last_use: Option<BytePos>,
}

impl SitesAbove {
    /// Records `stmt`, the next statement of the sequence, as above every
    /// position asked about from now on.
    pub fn pass(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::NamespaceDecl(decl) if decl.body.is_none() => {
                self.governing = Some(stmt.span.end);
                self.last_use = None;
            }
            StmtKind::UseDecl(_) => self.last_use = Some(stmt.span.end),
            _ => {}
        }
    }

    /// [`site_in`]'s answer at `at`, for the statements passed so far.
    #[must_use]
    pub fn site(&self, src: &SourceFile, at: BytePos, bracketed: bool) -> Option<ImportSite> {
        let (after, lead) = match (self.last_use, self.governing) {
            (Some(end), _) => (end, "\n"),
            (None, Some(end)) => (end, "\n\n"),
            (None, None) if bracketed => return None,
            (None, None) => {
                let tag = first_open_tag(src.text())?;
                (BytePos::try_from(tag.end).ok()?, "\n")
            }
        };
        (after <= at).then_some(ImportSite { at: after, lead })
    }
}

/// Every type `short` could be the last segment of: the classes, interfaces
/// and enums `symbols` declares under that name in any namespace, and the
/// `Core` types among `core` — spelled as source writes them, `Core\Request` —
/// in the order found, `Core` first.
///
/// A `type` alias is not among them: an alias is a spelling of another type,
/// and importing one where a class was written would make the reference mean
/// something the writer did not (`rule:types/alias-is-never-a-bare-class`).
#[must_use]
pub fn candidates<'a>(
    short: &str,
    symbols: &SymbolTable,
    core: impl IntoIterator<Item = &'a str>,
) -> Vec<QName> {
    let mut found: Vec<QName> = core
        .into_iter()
        .map(QName::parse)
        .filter(|qname| qname.short_name() == short)
        .collect();
    found.extend(
        symbols
            .iter()
            .filter(|symbol| symbol.kind != SymbolKind::TypeAlias)
            .filter(|symbol| symbol.qname.short_name() == short)
            .map(|symbol| symbol.qname.clone()),
    );
    found.sort_by_key(ToString::to_string);
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_syntax::parse;

    use super::*;

    /// `source` parsed as one file, and the site for a name written just past
    /// `written`, rendered as the offset and the lead.
    fn site(source: &str, written: &str) -> Option<(usize, &'static str)> {
        let mut map = SourceMap::new();
        let id = map.add("case.nvs", source.to_owned());
        let mut diags = Diagnostics::new();
        let parsed = parse(map.file(id), &mut diags);
        let at = source.find(written).expect("the document writes it") + written.len();
        import_site(
            &parsed.stmts,
            map.file(id),
            BytePos::try_from(at).expect("a test document is short"),
        )
        .map(|site| (site.at as usize, site.lead))
    }

    #[test]
    fn after_the_last_use_the_namespace_has() {
        let source = "<?nvs\nnamespace App;\n\nuse Core\\Str;\nuse Core\\Arr;\n\nRequest";
        let after = source.find("use Core\\Arr;").expect("written") + "use Core\\Arr;".len();
        assert_eq!(site(source, "Request"), Some((after, "\n")));
    }

    #[test]
    fn after_the_namespace_line_where_it_has_no_use() {
        let source = "<?nvs\nnamespace App;\n\nRequest";
        let after = source.find("namespace App;").expect("written") + "namespace App;".len();
        assert_eq!(site(source, "Request"), Some((after, "\n\n")));
    }

    #[test]
    fn after_the_open_tag_where_the_file_has_neither() {
        assert_eq!(
            site("<?nvs\nRequest", "Request"),
            Some(("<?nvs".len(), "\n"))
        );
        assert_eq!(
            site("<html>\n<?nvs\nRequest", "Request"),
            Some(("<html>\n<?nvs".len(), "\n"))
        );
    }

    #[test]
    fn a_bracketed_namespace_with_no_use_and_a_file_with_no_tag_have_no_site() {
        assert_eq!(
            site("<?nvs\nnamespace App {\nRequest\n}\n", "Request"),
            None
        );
        assert_eq!(site("Request", "Request"), None);
    }

    #[test]
    fn a_use_below_the_position_is_not_written_after() {
        let source = "<?nvs\nRequest\nuse Core\\Str;\n";
        assert_eq!(site(source, "Request"), Some(("<?nvs".len(), "\n")));
    }

    #[test]
    fn the_lines_for_several_symbols_share_one_lead() {
        let site = ImportSite {
            at: 5,
            lead: "\n\n",
        };
        let symbols = [QName::parse("Core\\Request"), QName::parse("App\\Shop")];
        assert_eq!(
            site.use_lines(symbols.iter()),
            "\n\nuse Core\\Request;\nuse App\\Shop;"
        );
        assert_eq!(site.use_lines(std::iter::empty()), "");
    }
}
