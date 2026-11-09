use nvs_diagnostics::{Diagnostics, SourceMap};

/// A document cut in two is still two documents. The input's last byte says
/// where to cut and everything before it is the source, so one entry covers
/// both shapes an editor hands the parser mid-keystroke — a prefix that stops
/// in the middle of a construct, and a suffix that starts in one.
///
/// Nothing is asserted beyond "it returned". What the tree *contains* over a
/// real corpus is `crates/nvs-syntax/tests/prefixes.rs`, which cuts every
/// `examples/*.nvs` at every token boundary; what this adds is the cuts no
/// corpus holds — inside a token, inside a character, and over text no
/// example would ever contain (`rule:ide/the-tree-survives-a-syntax-error`).
///
/// The selector is the *last* byte rather than the first so that any `.nvs`
/// file is a usable seed exactly as it stands: the source is then the file
/// itself and the selector is whatever it ends with.
pub fn prefix(data: &[u8]) {
    let [source @ .., selector] = data else {
        return;
    };
    let Ok(text) = std::str::from_utf8(source) else {
        return;
    };

    // Scaled rather than masked, so 0 and 255 reach the empty prefix and the
    // whole text. A cut landing inside a multi-byte character walks back to the
    // boundary below it: slicing one is this check's own panic rather than the
    // parser's, and it would say nothing about the grammar.
    let mut cut = text.len() * usize::from(*selector) / 255;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }

    let mut map = SourceMap::new();
    let mut diags = Diagnostics::new();
    let head = map.add("head.nvs", text[..cut].to_string());
    let _ = nvs_syntax::parse(map.file(head), &mut diags);
    let tail = map.add("tail.nvs", text[cut..].to_string());
    let _ = nvs_syntax::parse(map.file(tail), &mut diags);
}
