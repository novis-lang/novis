//! The PDF component's `nvs:pdf/engine`, called typed: `wit/pdf.wit`'s types as `bindgen!` writes
//! them, and the conversion of each to and from its Novis value. The `kept` body a first pass
//! returns, and the second pass takes, is a [`ResourceAny`] here.

use wasmtime::component::ResourceAny;

use self::bindings::exports::nvs::pdf::engine::{
    Asset, Attachment, Document, DroppedItem, FirstPass, FirstRender, Margins, Metadata,
    MissingGlyph, NamedValue, Overflow, PageFacts, PageMarkup, Part, PdfInfo, RenderOptions,
    Rendered, Standard,
};
use self::bindings::nvs::ext::types::{Error as WitError, Instant};
use crate::call::Error;

/// The types `bindgen!` writes for the world that exports `nvs:pdf/engine` alone. Its calls are
/// made through [`crate::typed::call`], so only the types are used.
#[allow(
    missing_docs,
    unreachable_pub,
    dead_code,
    clippy::all,
    reason = "generated code, of which only the types are named"
)]
mod bindings {
    wasmtime::component::bindgen!({
        path: ["../../wit/nvs-ext", "../../wit/pdf.wit"],
        inline: "
            package nvs:typed;

            world pdf {
                export nvs:pdf/engine@1.0.0;
            }
        ",
        world: "nvs:typed/pdf",
    });
}

impl From<WitError> for Error {
    fn from(err: WitError) -> Self {
        match err {
            WitError::Invalid(message) => Self::Invalid(message),
            WitError::Parse(message) => Self::Parse(message),
            WitError::Runtime(message) => Self::Runtime(message),
        }
    }
}

world! {
    error: WitError;
    render(Document) -> Rendered;
    render_first(FirstPass) -> FirstRender;
    render_second(ResourceAny, Margins) -> Rendered;
    merge(Vec<Part>) -> Vec<u8>;
    read(Vec<u8>) -> PdfInfo;
    read_text(Vec<u8>, Vec<u64>) -> Vec<String>;
}

records! {
    Asset { name, data }
    RenderOptions { standard, password, invoice_xml, created }
    Document { html, assets, options }
    FirstPass { document, header_height, footer_height }
    NamedValue { name, value }
    PageFacts { number, name, strings, marks }
    PageMarkup { header, footer, slots }
    Margins { pages }
    DroppedItem { property, value, selector, line, column }
    MissingGlyph { text, fonts, page }
    Overflow { page, area, text }
    Rendered { data, page_count, pages, dropped, missing_glyphs, overflows }
    FirstRender { body, facts }
    Part { data, pages }
    Metadata { title, author, subject, keywords, creator, producer }
    Attachment { name, data }
    PdfInfo { page_count, metadata, attachments, invoice_xml }
}

enums! {
    Standard [PdfA2b, PdfA3b, PdfUa1]
}

cores! {
    Instant { seconds, nanos }
}
