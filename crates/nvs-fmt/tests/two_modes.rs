//! The bytes a program prints, across the two modes the lexer has.
//!
//! A `.nvs` file is code and text in turn — an `?> … <?nvs` region, and a
//! markup literal's body — and neither of those is layout. The goal's standing
//! decision and `rule:tooling/fmt-never-reflows` are the same claim from two
//! directions: what a program *prints* is its value, so a formatter that
//! touched one byte of it would be changing the program's output to suit a
//! style. The first two cases are files whose text regions are deliberately
//! ragged, and the assertion is that the whole file comes back as it went in.
//!
//! Where the two modes meet is the other half, and the only byte of a template
//! region this formatter has an opinion about: the `?>` that leaves code mode
//! is code, so a line it opens is indented like the body's other lines, while
//! which line it sits on at all is the author's — moving one would move the
//! boundary between a program's code and its output.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

#[test]
fn an_inline_html_region_is_byte_identical_after_formatting() {
    // Every kind of whitespace a formatter has an opinion about inside code —
    // a run of spaces, a line indented past its neighbours, a tab — is here
    // inside the text region, where it is what the response body says.
    let source = "\
<?nvs
var $title = 'Novis';
if (true) {
    ?>
  <h1   class = \"kept\">   spaced   out   </h1>
\t<p>a tab opens this line</p>
       <br>
<?nvs
}
echo $title;
";
    assert_eq!(formatted(source), source);
}

#[test]
fn a_markup_literal_body_is_byte_identical_after_formatting() {
    // The body of a markup literal is the same bytes in a second spelling
    // (`rule:core-classes/html-literal`), including across the `{$…}` holes
    // that make it more than a string, so the ragged lines inside one survive
    // exactly as the text region's do.
    let source = "\
<?nvs
var $name = 'Novis';
var $page = html`
      <p   class=\"kept\">   {$name}   </p>
  <br>
`;
echo $page;
";
    assert_eq!(formatted(source), source);
}

#[test]
fn a_close_tag_is_never_moved_onto_or_off_a_line() {
    // Which line a close tag sits on is where the code stops and the text
    // begins, so moving one would move a byte of the text region across it.
    let source = "\
<?nvs
if (true) {
    echo 'inline'; ?>
    <p>after a statement</p>
    <?nvs
}
?>
<p>at the end</p>
";
    assert_eq!(formatted(source), source);
}

#[test]
fn a_close_tag_that_begins_its_line_sits_at_the_depth_of_its_block() {
    // A close tag that opens a line is the last thing on it, so the line is
    // the tag's own and is indented like any other line of that block.
    let mangled = "\
<?nvs
if (true) {
?>
<p>text</p>
<?nvs
}
";
    let canonical = "\
<?nvs
if (true) {
    ?>
<p>text</p>
<?nvs
}
";
    assert_eq!(formatted(mangled), canonical);
}
