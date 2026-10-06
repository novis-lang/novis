//! An extension class in the editor: its methods complete and hover from the manifest of the
//! extension set the document's `nvs.toml` pins, and the server never instantiates the component
//! (`rule:packaging/extension-calls-are-statically-typed`).
//!
//! The `.nvsx` is the `ledger` fixture's manifest and source over a component that imports a
//! function no host provides, so an instantiation could not succeed. An answer that names the
//! manifest's methods is an answer read from the manifest alone.

use lsp_types::{Hover, HoverContents};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{
    Analysed, CheckScope, Client, Documents, Response, SymbolIndex, analyse, completion, hover,
    uri_of,
};
use nvs_repo::Scratch;

/// A directory whose `nvs.toml` pins the ledger extension over a component that cannot be
/// instantiated.
fn tree(name: &str) -> Scratch {
    let dir = nvs_repo::scratch(name);
    let fixture = nvs_repo::path("tests/conformance/ext/fixtures/ledger");
    let manifest = std::fs::read(fixture.join("manifest.json")).expect("the fixture's manifest");
    let receipt = std::fs::read_to_string(fixture.join("source/Ledger/Receipt.nvs.src"))
        .expect("the fixture's source file");
    let source = format!(
        "{{\"source\": 1, \"files\": [{{\"path\": \"Ledger/Receipt.nvs\", \"text\": {receipt:?}}}]}}"
    );
    let component =
        wat::parse_str("(component (import \"missing\" (func)))").expect("the component parses");
    let nvsx = nvs_ext::pack::append_section(component, nvs_ext::section::MANIFEST, &manifest);
    let nvsx = nvs_ext::pack::append_section(nvsx, nvs_ext::section::SOURCE, source.as_bytes());
    std::fs::write(dir.join("shop.nvsx"), &nvsx).expect("the scratch directory is writable");
    std::fs::write(
        dir.join("nvs.toml"),
        format!(
            "[[extension]]\npath = 'shop.nvsx'\nsha256 = \"{}\"\n",
            nvs_ext::load::pin(&nvsx)
        ),
    )
    .expect("the scratch directory is writable");
    dir
}

/// `source` opened as `shop.nvs` in `dir`, and its analysis.
fn opened(dir: &Scratch, source: &str) -> (Documents, Analysed) {
    let uri = uri_of(&dir.join("shop.nvs")).expect("a test path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    (documents, analysis)
}

/// The offset just past the first `written` in `source`.
fn after(source: &str, written: &str) -> u32 {
    let at = source.find(written).expect("the document writes it") + written.len();
    u32::try_from(at).expect("a test document is short")
}

/// What is offered at `at`, rendered as a `.lspt` case freezes it.
fn offered(documents: &Documents, analysis: &Analysed, at: u32) -> String {
    let index = SymbolIndex::build(documents, CheckScope::Open, None);
    Response::Completion(completion::at(
        analysis,
        &index,
        &CompletionFiles::default(),
        at,
        Client::default(),
        PositionEncoding::Utf8,
    ))
    .render()
}

/// The text of the hover at `at`.
fn hovered(analysis: &Analysed, at: u32) -> String {
    let hover = hover::at(
        analysis,
        &CompletionFiles::default(),
        at,
        PositionEncoding::Utf8,
    );
    match hover {
        Some(Hover {
            contents: HoverContents::Markup(markup),
            ..
        }) => markup.value,
        other => panic!("a markup hover: {other:?}"),
    }
}

const CALL: &str = "<?nvs\nShop\\Ledger::echoInt(7);\n";

#[test]
fn completion_after_an_extension_class_offers_its_manifest_methods() {
    let dir = tree("lsp-ext-completion");
    let source = "<?nvs\nShop\\Ledger::\n";
    let (documents, analysis) = opened(&dir, source);
    let answer = offered(&documents, &analysis, after(source, "Ledger::"));
    for method in ["echoInt", "writeLine", "fetch", "CURRENCY"] {
        assert!(answer.contains(method), "{method}: {answer}");
    }
}

#[test]
fn hover_on_an_extension_method_shows_its_manifest_signature_and_help() {
    let dir = tree("lsp-ext-hover");
    let (_, analysis) = opened(&dir, CALL);
    let text = hovered(&analysis, after(CALL, "echoI"));
    assert!(text.contains("echoInt"), "{text}");
    assert!(text.contains("int $number"), "the signature: {text}");
    assert!(
        text.contains("Returns `number` unchanged."),
        "the help: {text}"
    );
}

/// A call into the extension types and publishes nothing, though the component behind it cannot
/// be instantiated, and a wrong argument is the `E0401` `nvs check` reports.
#[test]
fn the_language_server_never_instantiates_an_extension() {
    let dir = tree("lsp-ext-never-instantiated");
    let (_, good) = opened(&dir, CALL);
    assert!(!good.diags.has_errors(), "{:?}", good.diags);

    let (_, bad) = opened(&dir, "<?nvs\nShop\\Ledger::echoInt(\"seven\");\n");
    let codes: Vec<String> = bad
        .diags
        .iter()
        .filter_map(|diag| diag.code.map(|code| code.to_string()))
        .collect();
    assert!(codes.iter().any(|code| code == "E0401"), "{codes:?}");
}
