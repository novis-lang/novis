// The code of the extension. Novis calls `to_html` as `Example\Markdown\Parser::toHtml`
// and `headings` as `Example\Markdown\Parser::headings`.

// This macro reads the WIT files in `wit/` and writes the Rust types for them.
// `generate_all` also writes the types of the `nvs:ext` world and of WASI.
wit_bindgen::generate!({
    path: "wit",
    world: "markdown",
    generate_all,
});

use exports::example::markdown::api::{Error, Guest, Heading};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

struct Markdown;

impl Guest for Markdown {
    // `to_html` converts the Markdown to HTML. HTML written inside the Markdown is
    // changed to plain text, so the result contains only the tags the parser writes.
    fn to_html(text: String) -> Result<String, Error> {
        let events = Parser::new_ext(&text, options()).map(|event| match event {
            Event::Html(html) | Event::InlineHtml(html) => Event::Text(html),
            other => other,
        });
        let mut html = String::with_capacity(text.len() * 3 / 2);
        pulldown_cmark::html::push_html(&mut html, events);
        Ok(html)
    }

    // `headings` returns the level and the text of each heading. The text of a
    // heading is the text of every part of it, with the formatting removed.
    // HTML inside a heading is kept as text, as `to_html` keeps it.
    fn headings(text: String) -> Result<Vec<Heading>, Error> {
        let mut found = Vec::new();
        let mut current: Option<Heading> = None;
        for event in Parser::new_ext(&text, options()) {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    current = Some(Heading {
                        level: level as i64,
                        text: String::new(),
                    });
                }
                Event::Text(part) | Event::Code(part) | Event::InlineHtml(part) => {
                    if let Some(heading) = current.as_mut() {
                        heading.text.push_str(&part);
                    }
                }
                Event::End(TagEnd::Heading(_)) => found.extend(current.take()),
                _ => {}
            }
        }
        Ok(found)
    }
}

// The Markdown extensions both functions read: tables, ~~strikethrough~~ and
// task lists, such as `- [x] done`.
fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

export!(Markdown);
