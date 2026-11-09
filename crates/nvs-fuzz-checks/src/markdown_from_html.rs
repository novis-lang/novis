use nvs_stdlib::fuzz;

/// `Core\Markdown::fromHtml` must not panic on any HTML, and writing its
/// Markdown's own HTML again gives the same Markdown
/// (`rule:core-classes/markdown-from-html`).
///
/// This holds over any input, raw HTML included. That Markdown rendered,
/// written back and rendered again keeps its meaning is checked over fixed
/// documents in `crates/nvs-stdlib/src/markdown/tests.rs`.
pub fn markdown_from_html(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let first = fuzz::from_html(text);
    let second = fuzz::from_html(&fuzz::markdown(&first));
    assert_eq!(second, first, "written from its own HTML");
}
