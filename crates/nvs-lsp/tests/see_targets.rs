//! The editor reads every `@see` target, `@example` path and `@throws` name
//! the checker resolved: links, go-to-definition, hover, completion and
//! semantic tokens.
//!
//! Each answer here names a file by its absolute path, or a `file:` URI of
//! one, so it names the machine the test ran on. A `.lspt` case cannot freeze
//! that, so the claims are written here.

use std::path::{Path, PathBuf};

use lsp_types::{HoverContents, SemanticTokenType};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{
    Analysed, CURSOR, CheckScope, Client, Documents, SymbolIndex, TOKEN_TYPES, analyse, completion,
    definition, hover, links, semantic, uri_of,
};
use nvs_test::temp::TempDir;

/// The Markdown file the document's `@see` file target points into. The
/// heading `## Paying by card` is on 0-based line 4.
const GUIDE: &str = "# Guide\n\nHow the shop works.\n\n## Paying by card\n\n\
                     Cards are charged once.\n\n## Refunds\n\nRefunds take a week.\n";

/// The document every test but the completion ones analyses.
const SHOP: &str = "<?nvs\n\
                    /// A shop that sells things.\n\
                    ///\n\
                    /// @see Shop::total\n\
                    /// @see docs/guide.md#paying-by-card\n\
                    /// @example examples/run.nvs\n\
                    class Shop\n\
                    {\n\
                    \x20   /// The total, in cents.\n\
                    \x20   ///\n\
                    \x20   /// @throws Core\\Exception when the cart is empty\n\
                    \x20   public function total(): int { return 1; }\n\
                    }\n\
                    var $shop = new Shop();\n";

/// A project folder holding the guide, an example and `case.nvs`.
fn project(name: &str) -> TempDir {
    let dir = TempDir::new(nvs_repo::scratch(&format!("lsp-see-{name}")));
    dir.write("docs/guide.md", GUIDE);
    dir.write("examples/run.nvs", "<?nvs\necho 1;\n");
    dir
}

/// `source` opened as `case.nvs` in `dir` and analysed, with the documents
/// the analysis read.
fn analysed(dir: &TempDir, source: &str) -> (Documents, Analysed) {
    dir.write("case.nvs", source);
    let uri = uri_of(&dir.at("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    (documents, analysed)
}

/// The byte offset of `needle` in [`SHOP`], plus `into`.
fn offset_of(needle: &str, into: usize) -> u32 {
    let at = SHOP.find(needle).expect("the needle is in the document");
    u32::try_from(at + into).expect("a test document is short")
}

/// The 0-based line `needle` starts on in [`SHOP`].
fn line_of(needle: &str) -> u32 {
    let at = SHOP.find(needle).expect("the needle is in the document");
    u32::try_from(SHOP[..at].matches('\n').count()).expect("a test document is short")
}

/// `path`, canonical.
fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().expect("the fixture exists")
}

/// The Markdown a hover at `offset` answers.
fn hover_at(analysed: &Analysed, offset: u32) -> String {
    let answer = hover::answer(
        analysed,
        &CompletionFiles::default(),
        offset,
        PositionEncoding::Utf8,
        None,
    )
    .expect("the cursor has a hover");
    match answer.contents {
        HoverContents::Markup(markup) => markup.value,
        other => panic!("a hover is Markdown, got {other:?}"),
    }
}

// covers: tools:editor/nvs-lsp
#[test]
fn a_document_link_opens_a_see_file_at_its_heading() {
    let dir = project("link-heading");
    let (_, analysed) = analysed(&dir, SHOP);
    let found = links::for_document(&analysed, PositionEncoding::Utf8);
    let guide = canonical(&dir.at("docs/guide.md"));
    let link = found
        .iter()
        .find(|link| link.target == guide)
        .unwrap_or_else(|| panic!("no link to the guide in {found:?}"));
    assert_eq!(link.line, Some(4));
    assert!(!link.directory);
    assert_eq!(link.range.start.line, line_of("docs/guide.md"));
}

#[test]
fn a_document_link_opens_an_example_file() {
    let dir = project("link-example");
    let (_, analysed) = analysed(&dir, SHOP);
    let found = links::for_document(&analysed, PositionEncoding::Utf8);
    let example = canonical(&dir.at("examples/run.nvs"));
    let link = found
        .iter()
        .find(|link| link.target == example)
        .unwrap_or_else(|| panic!("no link to the example in {found:?}"));
    assert_eq!(link.line, None);
}

#[test]
fn definition_follows_a_see_member_target() {
    let dir = project("definition-member");
    let (_, analysed) = analysed(&dir, SHOP);
    let at = |offset| {
        definition::at(
            &analysed,
            &CompletionFiles::default(),
            offset,
            PositionEncoding::Utf8,
        )
        .expect("the target has a declaration")
    };
    let member = at(offset_of("Shop::total", 8));
    assert_eq!(member.path.file_name(), Some("case.nvs".as_ref()));
    assert_eq!(member.range.start.line, line_of("public function total"));
    let class = at(offset_of("Shop::total", 1));
    assert_eq!(class.range.start.line, line_of("class Shop"));
}

#[test]
fn definition_follows_a_see_file_target_to_its_heading() {
    let dir = project("definition-file");
    let (_, analysed) = analysed(&dir, SHOP);
    let declared = definition::at(
        &analysed,
        &CompletionFiles::default(),
        offset_of("docs/guide.md", 3),
        PositionEncoding::Utf8,
    )
    .expect("the file target has a place");
    assert_eq!(declared.path, canonical(&dir.at("docs/guide.md")));
    assert_eq!(declared.range.start.line, 4);
    let example = definition::at(
        &analysed,
        &CompletionFiles::default(),
        offset_of("examples/run.nvs", 3),
        PositionEncoding::Utf8,
    )
    .expect("the example path has a place");
    assert_eq!(example.path, canonical(&dir.at("examples/run.nvs")));
    assert_eq!(example.range.start.line, 0);
}

#[test]
fn hover_on_a_see_member_target_shows_the_member() {
    let dir = project("hover-member");
    let (_, analysed) = analysed(&dir, SHOP);
    let value = hover_at(&analysed, offset_of("Shop::total", 8));
    assert!(value.contains("Shop::total()"), "{value}");
    assert!(value.contains("The total, in cents."), "{value}");
}

#[test]
fn hover_on_a_see_file_target_shows_the_heading_section() {
    let dir = project("hover-file");
    let (_, analysed) = analysed(&dir, SHOP);
    let value = hover_at(&analysed, offset_of("docs/guide.md", 3));
    assert!(value.contains("guide.md"), "{value}");
    assert!(value.contains("## Paying by card"), "{value}");
    assert!(value.contains("Cards are charged once."), "{value}");
    assert!(
        !value.contains("Refunds"),
        "the next section is shown: {value}"
    );
}

#[test]
fn a_declarations_hover_links_its_see_targets() {
    let dir = project("hover-links");
    let (_, analysed) = analysed(&dir, SHOP);
    let value = hover_at(&analysed, offset_of("new Shop", 5));
    let guide = uri_of(&canonical(&dir.at("docs/guide.md"))).expect("a scratch path is UTF-8");
    assert!(
        value.contains(&format!(
            "[`docs/guide.md#paying-by-card`]({}#L5)",
            guide.as_str()
        )),
        "{value}"
    );
    let member = value
        .split("[`Shop::total`](")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .unwrap_or_else(|| panic!("Shop::total is no link: {value}"));
    assert!(member.starts_with("file:"), "{member}");
    assert!(member.contains("case.nvs"), "{member}");
    let line = line_of("public function total") + 1;
    assert!(member.ends_with(&format!("#L{line}")), "{member}");
    assert!(value.contains("[`examples/run.nvs`](file:"), "{value}");
}

#[test]
fn completion_offers_files_and_headings_after_see() {
    let dir = project("completion");
    let labels = |source: &str| -> Vec<String> {
        let at = source.find(CURSOR).expect("one cursor");
        let source = source.replace(CURSOR, "");
        let (documents, analysed) = analysed(&dir, &source);
        let index = SymbolIndex::build(&documents, CheckScope::Open, None);
        completion::at(
            &analysed,
            &index,
            &CompletionFiles::default(),
            u32::try_from(at).expect("short"),
            Client::default(),
            PositionEncoding::Utf8,
        )
        .into_iter()
        .map(|item| item.label)
        .collect()
    };
    let files = labels("<?nvs\n/// A shop.\n///\n/// @see docs/<|>\nclass Shop {}\n");
    assert!(files.iter().any(|label| label == "guide.md"), "{files:?}");
    let headings = labels("<?nvs\n/// A shop.\n///\n/// @see docs/guide.md#<|>\nclass Shop {}\n");
    for slug in ["guide", "paying-by-card", "refunds"] {
        assert!(headings.iter().any(|label| label == slug), "{headings:?}");
    }
}

#[test]
fn see_and_throws_targets_get_semantic_tokens() {
    let dir = project("semantic");
    let (_, analysed) = analysed(&dir, SHOP);
    let tokens = semantic::for_document(&analysed, PositionEncoding::Utf8, &[]);
    let mut decoded: Vec<(u32, u32, u32, SemanticTokenType)> = Vec::new();
    let (mut line, mut character) = (0, 0);
    for token in tokens {
        if token.delta_line > 0 {
            line += token.delta_line;
            character = token.delta_start;
        } else {
            character += token.delta_start;
        }
        decoded.push((
            line,
            character,
            token.length,
            TOKEN_TYPES[token.token_type as usize].clone(),
        ));
    }
    let token_at = |needle: &str, into: usize| {
        let line = line_of(needle);
        let start = SHOP.find(needle).expect("in the document") + into;
        let line_start = SHOP[..start].rfind('\n').map_or(0, |at| at + 1);
        let character = u32::try_from(start - line_start).expect("short");
        decoded
            .iter()
            .find(|(l, c, _, _)| *l == line && *c == character)
            .map(|(_, _, length, kind)| (*length, kind.clone()))
    };
    assert_eq!(
        token_at("Shop::total", 0),
        Some((4, SemanticTokenType::CLASS)),
        "{decoded:?}"
    );
    assert_eq!(
        token_at("Shop::total", 6),
        Some((5, SemanticTokenType::METHOD)),
        "{decoded:?}"
    );
    assert_eq!(
        token_at("Core\\Exception", 0),
        Some((14, SemanticTokenType::CLASS)),
        "{decoded:?}"
    );
    assert_eq!(token_at("docs/guide.md", 0), None, "{decoded:?}");
}
