use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_syntax::tokenize;

/// The lexer must not panic on any input, valid UTF-8 or not.
pub fn lex(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut map = SourceMap::new();
    let id = map.add("fuzz.nvs", text.to_string());
    let mut diags = Diagnostics::new();
    let _ = tokenize(map.file(id), &mut diags);
}
