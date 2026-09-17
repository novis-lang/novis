//! The document outline: every declaration the entry file writes, as a tree.
//!
//! `textDocument/documentSymbol` is the request with the least behind it — it
//! takes no cursor and asks nothing of the type phase — and this module is why:
//! the outline is a walk of the statements [`crate::analyse`] already parsed,
//! and nothing more. What the walk builds is `lsp_types::DocumentSymbol`, which
//! [`crate::render`] freezes as one indented line per symbol.
//!
//! **The entry file only**, on [`crate::diagnostics::for_document`]'s terms:
//! one analysis reads a whole `require` graph and the request names one URI, so
//! a required file's declarations belong to *its* outline rather than to this
//! one.
//!
//! **A declaration the parser refuses is in the outline anyway.**
//! `rule:classes/no-free-functions-or-constants` rejects a top-level `function`
//! or `const` and the parser still produces the whole shape, which is exactly
//! the document a `.lspt` case is written about: an outline says what the file
//! looks like, and a reader hunting the function they have just typed is not
//! helped by its absence. The diagnostic is what says it is refused; the
//! outline is what says it is there.
//!
//! **A declaration with no name contributes nothing.** A `class {` half-typed
//! at the cursor parses to a name whose span covers no bytes, and there is no
//! text to show for it — so the entry appears the moment the name does, rather
//! than flickering as a blank line under whatever the editor renders an empty
//! label as. Nothing here invents a placeholder: a spelling would be
//! `rule:ide/the-rendering-has-one-home`'s to own, and it has no name to spell.
//!
//! `detail` is left unset for the same reason. LSP shows it beside the label
//! and a signature would belong there, but the rendering that freezes an answer
//! has no column for one, so filling it would put a field no `.lspt` case can
//! see into every symbol the server sends.

use lsp_types::{DocumentSymbol, SymbolKind};
use nvs_diagnostics::{PositionEncoding, SourceFile, Span};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, EnumDecl, NamespaceDecl, Stmt, StmtKind};

use crate::document::Analysed;
use crate::position::range_at;

/// The outline of one analysed document, positioned in `encoding`.
///
/// Empty for a document whose analysis reached no statements at all, which is
/// what an unreadable entry file leaves behind.
#[must_use]
pub fn for_document(analysed: &Analysed, encoding: PositionEncoding) -> Vec<DocumentSymbol> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    declarations(&loaded.stmts, analysed.map.file(analysed.entry), encoding)
}

/// The symbols `stmts` declares, in source order.
fn declarations(
    stmts: &[Stmt],
    file: &SourceFile,
    encoding: PositionEncoding,
) -> Vec<DocumentSymbol> {
    stmts
        .iter()
        .flat_map(|stmt| declared(stmt, file, encoding))
        .collect()
}

/// The symbols one statement declares — none for a statement that declares
/// nothing, and several for one declarator list.
fn declared(stmt: &Stmt, file: &SourceFile, encoding: PositionEncoding) -> Vec<DocumentSymbol> {
    let whole = stmt.span;
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => one(symbol(
            file,
            encoding,
            decl.name.span,
            whole,
            SymbolKind::CLASS,
            members(&decl.members, file, encoding),
        )),
        StmtKind::InterfaceDecl(decl) => one(symbol(
            file,
            encoding,
            decl.name.span,
            whole,
            SymbolKind::INTERFACE,
            members(&decl.members, file, encoding),
        )),
        StmtKind::EnumDecl(decl) => one(symbol(
            file,
            encoding,
            decl.name.span,
            whole,
            SymbolKind::ENUM,
            enum_body(decl, file, encoding),
        )),
        // LSP names no kind for a type alias. `TypeParameter` is the only one
        // left that names a type rather than a value, and the alternative is
        // leaving `type Id = uint;` out of the outline of the file that
        // declares it.
        StmtKind::TypeAliasDecl(decl) => one(symbol(
            file,
            encoding,
            decl.name.span,
            whole,
            SymbolKind::TYPE_PARAMETER,
            Vec::new(),
        )),
        StmtKind::TopLevelFunction(method) => one(symbol(
            file,
            encoding,
            method.name,
            whole,
            SymbolKind::FUNCTION,
            Vec::new(),
        )),
        // One statement, several constants: `const A = 1, B = 2;` is flattened
        // at parse time, and each declarator is its own entry under the one
        // range the statement covers.
        StmtKind::TopLevelConst(consts) => consts
            .iter()
            .filter_map(|declared| {
                symbol(
                    file,
                    encoding,
                    declared.name,
                    whole,
                    SymbolKind::CONSTANT,
                    Vec::new(),
                )
            })
            .collect(),
        StmtKind::NamespaceDecl(decl) => namespace(decl, file, encoding),
        _ => Vec::new(),
    }
}

/// A namespace declaration's symbols.
///
/// The bracketed form holds its declarations and so nests them; the statement
/// form applies to the rest of the enclosing scope, so what follows it is a
/// sibling and this is a leaf. `namespace { ... }` names no namespace at all,
/// and its declarations sit at file scope rather than under a nameless entry.
fn namespace(
    decl: &NamespaceDecl,
    file: &SourceFile,
    encoding: PositionEncoding,
) -> Vec<DocumentSymbol> {
    let inner = decl
        .body
        .as_ref()
        .map_or_else(Vec::new, |body| declarations(&body.stmts, file, encoding));
    match decl.name {
        None => inner,
        Some(name) => one(symbol(
            file,
            encoding,
            name.span,
            decl.span,
            SymbolKind::NAMESPACE,
            inner,
        )),
    }
}

/// An enum's cases and whatever else was written in its body, in source order.
///
/// The two are separate vectors on the declaration — a case is legal and any
/// other member is refused (`rule:enums/no-class-machinery`) — so they are
/// merged by where they were written. Sorting by position is what restores
/// document order here rather than imposing one.
fn enum_body(
    decl: &EnumDecl,
    file: &SourceFile,
    encoding: PositionEncoding,
) -> Vec<DocumentSymbol> {
    let mut body: Vec<DocumentSymbol> = decl
        .cases
        .iter()
        .filter_map(|case| {
            symbol(
                file,
                encoding,
                case.name.span,
                case.span,
                SymbolKind::ENUM_MEMBER,
                Vec::new(),
            )
        })
        .chain(members(&decl.members, file, encoding))
        .collect();
    body.sort_by_key(|symbol| symbol.range.start);
    body
}

/// The symbols a class, interface or enum body's members declare.
fn members(
    members: &[ClassMember],
    file: &SourceFile,
    encoding: PositionEncoding,
) -> Vec<DocumentSymbol> {
    members
        .iter()
        .filter_map(|member| {
            let (name, kind) = match &member.kind {
                ClassMemberKind::Property(property) => (property.name, SymbolKind::PROPERTY),
                ClassMemberKind::Const(declared) => (declared.name, SymbolKind::CONSTANT),
                ClassMemberKind::Method(method) => (method.name, SymbolKind::METHOD),
                // A `type` alias a body owns takes the kind the file-scope
                // form takes, for the reason given where that one is built:
                // LSP names none for an alias, and an outline that left the
                // member out would leave `Owner::Name` unreachable from the
                // file's own shape.
                ClassMemberKind::TypeAlias(alias) => (alias.name.span, SymbolKind::TYPE_PARAMETER),
                // `Error` is recovery, and the wildcard is what
                // `ClassMemberKind` being `#[non_exhaustive]` asks for: a
                // member shape this crate has not heard of shows nothing
                // rather than stopping the outline.
                _ => return None,
            };
            symbol(file, encoding, name, member.span, kind, Vec::new())
        })
        .collect()
}

/// One symbol, or nothing where the name covers no source bytes.
///
/// `whole` is the range the editor reveals and `name` the range it selects,
/// which LSP requires to be contained in it — every caller passes a
/// declaration's own span and a name written inside it.
#[expect(
    deprecated,
    reason = "`DocumentSymbol::deprecated` is superseded by `tags` in LSP 3.16, \
              and `lsp_types` still makes it a field every literal has to write"
)]
fn symbol(
    file: &SourceFile,
    encoding: PositionEncoding,
    name: Span,
    whole: Span,
    kind: SymbolKind,
    children: Vec<DocumentSymbol>,
) -> Option<DocumentSymbol> {
    let spelling = text_of(file, name);
    if spelling.is_empty() {
        return None;
    }
    Some(DocumentSymbol {
        name: spelling.to_owned(),
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range: range_at(file, whole, encoding),
        selection_range: range_at(file, name, encoding),
        // `Some(vec![])` and `None` render the same and mean different things
        // to a client: an empty list is a container a person can expand onto
        // nothing.
        children: (!children.is_empty()).then_some(children),
    })
}

/// An optional symbol as the list a caller concatenates.
fn one(symbol: Option<DocumentSymbol>) -> Vec<DocumentSymbol> {
    symbol.into_iter().collect()
}

/// The source text `span` covers, or nothing for a span outside the file.
///
/// A parser's span is always inside the file it parsed, so the fallback is
/// unreachable rather than a policy — and it is a fallback rather than an index
/// because a panic here would take the server down over one malformed name.
fn text_of(file: &SourceFile, span: Span) -> &str {
    file.text().get(span.range()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_syntax::parse_file;

    use super::*;
    use crate::render::Response;

    /// The outline of `source`, rendered as a `.lspt` case freezes it.
    fn outlined(source: &str) -> String {
        let mut map = SourceMap::new();
        let id = map.add("case.nvs", source);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        let symbols = declarations(&stmts, map.file(id), PositionEncoding::Utf8);
        Response::DocumentSymbol(symbols).render()
    }

    /// The outline is the file's shape: a class, its members under it, in the
    /// order they were written rather than sorted.
    #[test]
    fn a_class_carries_its_members_in_document_order() {
        assert_eq!(
            outlined(
                "<?nvs\nclass User {\n  public const int MAX = 10;\n  public string $name;\n  \
                 public function greet(): string { return \"hi\"; }\n}\n"
            ),
            "User class\n  MAX constant\n  $name property\n  greet method\n"
        );
    }

    /// A top-level `function` and `const` are refused by
    /// `rule:classes/no-free-functions-or-constants` and still parse, and the
    /// outline shows what the file holds rather than what the checker allows.
    #[test]
    fn a_refused_top_level_declaration_is_still_in_the_outline() {
        assert_eq!(
            outlined("<?nvs\nfunction helper(): int { return 1; }\nconst A = 1, B = 2;\n"),
            "helper function\nA constant\nB constant\n"
        );
    }

    /// The document a `.lspt` case is written about: the class whose name is
    /// half-typed has nothing to show, and the one above it is unaffected.
    #[test]
    fn a_declaration_with_no_name_yet_contributes_nothing() {
        assert_eq!(
            outlined("<?nvs\nclass Whole {}\nclass {\n"),
            "Whole class\n"
        );
    }

    /// A named `namespace Foo { ... }` nests what it holds; the unnamed
    /// bracketed form is the global namespace, so its declarations are at file
    /// scope.
    #[test]
    fn a_namespace_nests_what_it_brackets_unless_it_names_nothing() {
        assert_eq!(
            outlined("<?nvs\nnamespace App {\n  class User {}\n}\n"),
            "App namespace\n  User class\n"
        );
        assert_eq!(
            outlined("<?nvs\nnamespace {\n  class User {}\n}\n"),
            "User class\n"
        );
    }

    /// An enum's cases are its members, and a document declaring nothing at all
    /// answers `none` rather than an empty rendering.
    #[test]
    fn an_enum_lists_its_cases_and_an_empty_document_answers_none() {
        assert_eq!(
            outlined("<?nvs\nenum Status: int {\n  case Draft = 1;\n  case Live = 2;\n}\n"),
            "Status enum\n  Draft enumMember\n  Live enumMember\n"
        );
        assert_eq!(outlined("<?nvs\nvar $x = 1;\n"), "none\n");
    }

    /// A `type` alias a body owns is in the outline under its owner, in every
    /// body that can declare one — the member is reached as `Owner::Name`, and
    /// the outline is where that owner is read off the file's own shape
    /// (`rule:types/type-alias`). The file-scope form is unmoved beside it.
    #[test]
    fn document_symbols_nest_a_class_scoped_alias_under_its_owner() {
        assert_eq!(
            outlined(
                "<?nvs\ntype Id = uint;\nclass Order {\n  type Meta = {total: int};\n  public \
                 function total(): int { return 1; }\n}\ninterface Shipper {\n  type Label = \
                 {code: string};\n}\nenum Status {\n  type Pair = array<Status>;\n  Open,\n}\n"
            ),
            "Id typeParameter\nOrder class\n  Meta typeParameter\n  total method\nShipper \
             interface\n  Label typeParameter\nStatus enum\n  Pair typeParameter\n  Open \
             enumMember\n"
        );
    }
}
