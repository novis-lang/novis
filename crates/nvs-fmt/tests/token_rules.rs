//! The rules that rewrite a token, as `rule:tooling/fmt-quotes` and
//! `rule:tooling/fmt-trailing-commas` state them.
//!
//! Each case is one file and the canonical text it formats to, so what a
//! failure prints is the whole disagreement rather than a property that went
//! false somewhere. An input is wrong about the rule under test and about
//! nothing else — every other byte of it is what the printer answers today, so
//! a case fails for its own reason and an unlanded rule cannot mask it.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

#[test]
fn a_plain_string_is_rewritten_to_single_quotes() {
    let mangled = "\
<?nvs
var $name = \"novis\";
var $empty = \"\";
var $kept = 'already';
var $greeting = \"hello, $name\";
var $apostrophe = \"it's here\";
var $escaped = \"one\\ttwo\";
echo $greeting;
";
    let canonical = "\
<?nvs
var $name = 'novis';
var $empty = '';
var $kept = 'already';
var $greeting = \"hello, $name\";
var $apostrophe = \"it's here\";
var $escaped = \"one\\ttwo\";
echo $greeting;
";
    assert_eq!(formatted(mangled), canonical);
}

#[test]
fn a_heredoc_body_and_a_comment_are_never_touched() {
    let source = "\
<?nvs
// a \"quoted\" word in a line comment
/* and a \"quoted\" one in a block comment */
var $doc = <<<TEXT
a \"quoted\" line
    kept at its own depth
TEXT;
var $raw = <<<'TEXT'
no \\t escape, and a \"quote\"
TEXT;
echo $doc, $raw;
";
    assert_eq!(formatted(source), source);
}

#[test]
fn a_multi_line_list_gains_a_trailing_comma_and_a_one_line_list_has_none() {
    let mangled = "\
<?nvs
var $one = [1, 2,];
var $empty = [];
var $spread = [
    1,
    2
];
var $call = Str::join(
    $spread
);
var $object = {
    left: 1,
    right: 2
};
var $arm = match ($one) {
    1 => 'one',
    default => 'rest'
};
echo $one, $empty, $spread, $call, $object, $arm;
";
    let canonical = "\
<?nvs
var $one = [1, 2];
var $empty = [];
var $spread = [
    1,
    2,
];
var $call = Str::join(
    $spread,
);
var $object = {
    left: 1,
    right: 2,
};
var $arm = match ($one) {
    1 => 'one',
    default => 'rest',
};
echo $one, $empty, $spread, $call, $object, $arm;
";
    assert_eq!(formatted(mangled), canonical);
}

/// `rule:tooling/fmt-normalizes-only-reserved-spellings`'s two spellings are
/// both errors, and neither of them refuses the file: the tree behind a
/// mis-cased spelling is whole, so the formatter writes the one case the
/// spelling has. Everything else keeps the case it was written in — an
/// identifier's is a workspace-wide rename and a string's is its value.
#[test]
fn a_mis_cased_open_tag_and_duration_unit_are_lower_cased() {
    let mangled = "\
<?NVS
var $ttl = 30S;
var $window = 1H30M;
var $kept = 'Novis';
echo $ttl, $window, $kept;
";
    let canonical = "\
<?nvs
var $ttl = 30s;
var $window = 1h30m;
var $kept = 'Novis';
echo $ttl, $window, $kept;
";
    assert_eq!(formatted(mangled), canonical);
}
