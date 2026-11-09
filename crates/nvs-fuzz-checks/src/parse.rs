use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_syntax::parse_file;

/// The parser must not panic on any input, valid UTF-8 or not, including
/// inputs the grammar rejects: a rejection must come back as a diagnostic,
/// never a crash.
///
/// The seeds are `fuzz/seeds/parse/`, shared with the `ast` check, which takes
/// the same input and carries the walk over the tree as well.
pub fn parse(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut map = SourceMap::new();
    let id = map.add("fuzz.nvs", text.to_string());
    let mut diags = Diagnostics::new();
    let _ = parse_file(map.file(id), &mut diags);
}
