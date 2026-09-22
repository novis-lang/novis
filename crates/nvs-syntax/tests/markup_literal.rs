//! The one template habit a markup literal warns about: `{Page::TITLE}`, a
//! brace directly before a class path, which `rule:core-classes/html-literal`
//! keeps as text and which a page then prints as written. The literal's
//! typing is `nvs-types`'s `markup_literal.rs`; what is held here is the
//! lexer's warning — where it points, what it names, and every brace it stays
//! silent on.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap, Span, code};
use nvs_syntax::parse_file;

/// Parses `src` as a whole file, the way every compile path does.
fn parse(src: &str) -> (SourceMap, Diagnostics) {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let _ = parse_file(map.file(id), &mut diags);
    (map, diags)
}

fn text(map: &SourceMap, span: Span) -> &str {
    map.file(span.file)
        .span_text(span)
        .expect("a span from this parse is inside the file it came from")
}

/// Every span this parse warned [`code::W_MARKUP_BRACE_BEFORE_A_CLASS_PATH`] at.
fn braces(diags: &Diagnostics) -> Vec<Span> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::W_MARKUP_BRACE_BEFORE_A_CLASS_PATH))
        .filter_map(Diagnostic::primary_span)
        .collect()
}

/// The warning covers the brace through the `}` that closes it on the same
/// line, and its help rewrites the same text as the `<?= … ?>` hole that
/// takes it. A namespaced path and a static call are the same habit.
#[test]
fn a_brace_before_a_class_path_in_a_markup_literal_warns_at_the_brace() {
    let src = "<?nvs\n\
        echo html`<h1>{Page::TITLE} {Money::format($c)} {App\\Shop::NAME}</h1>`;\n";
    let (map, diags) = parse(src);
    assert!(!diags.has_errors(), "{diags:?}");
    let warned: Vec<&str> = braces(&diags)
        .into_iter()
        .map(|span| text(&map, span))
        .collect();
    assert_eq!(
        warned,
        ["{Page::TITLE}", "{Money::format($c)}", "{App\\Shop::NAME}"]
    );
    let notes = diags
        .iter()
        .find(|d| d.code == Some(code::W_MARKUP_BRACE_BEFORE_A_CLASS_PATH))
        .map(|d| d.notes.join("\n"))
        .unwrap_or_default();
    assert!(
        notes.contains("`<?= Page::TITLE ?>`"),
        "the help names the hole that takes the expression: {notes}"
    );
}

/// A path whose line ends before any `}` is still the habit; the warning then
/// points at the brace alone.
#[test]
fn an_unclosed_brace_before_a_class_path_warns_at_the_brace_alone() {
    let src = "<?nvs\necho html`<p>{Page::TITLE\n</p>`;\n";
    let (map, diags) = parse(src);
    let warned: Vec<&str> = braces(&diags)
        .into_iter()
        .map(|span| text(&map, span))
        .collect();
    assert_eq!(warned, ["{"]);
}

/// Every other brace stays silent: a real hole, a CSS block, a brace before a
/// word with no `::`, an escaped brace, the tag hole that takes the
/// expression, and the same text in a double-quoted string, where the habit
/// is PHP's own and the reader already knows the answer.
#[test]
fn every_other_brace_is_silent() {
    for src in [
        "<?nvs\necho html`<p>{$page->title()}</p>`;\n",
        "<?nvs\necho html`<style>.badge{color:red}</style>`;\n",
        "<?nvs\necho html`<p>{not a hole} {{ }} {Page}</p>`;\n",
        "<?nvs\necho html`<p>\\{Page::TITLE}</p>`;\n",
        "<?nvs\necho html`<p><?= Page::TITLE ?></p>`;\n",
        "<?nvs\necho \"{Page::TITLE}\";\n",
    ] {
        let (_, diags) = parse(src);
        assert!(
            braces(&diags).is_empty(),
            "no warning for {src:?}: {diags:?}"
        );
    }
}
