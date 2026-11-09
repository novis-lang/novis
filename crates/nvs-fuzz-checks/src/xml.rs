/// `Core\Xml::parse`'s strict reader must not panic, overflow its stack or hang
/// on any text. It either builds a tree or refuses the whole document
/// (`rule:errors/ambiguous-input-refused`).
pub fn xml(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = nvs_stdlib::fuzz::xml(text);
    }
}
