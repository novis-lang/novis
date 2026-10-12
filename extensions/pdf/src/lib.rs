//! The PDF component: `nvs:pdf/engine` from `wit/pdf.wit`, built into every `nvs` binary
//! (`rule:packaging/the-first-party-components-are-built-in`, `rule:core-classes/pdf-one-engine`).
//!
//! The crate has two halves. The engine core is plain Rust that builds for the host too, so its
//! tests run under an ordinary `cargo test` in this directory. The WIT glue in `guest` is compiled
//! only for a wasm target, and maps each export onto the core and the core's types onto WIT's.
//!
//! A render is one call that takes the whole document and returns the whole file: [`render`]
//! parses the HTML (`parse`'s module doc), resolves every resource it references against the
//! asset map before anything is laid out (`assets`'s module doc), computes every element's
//! style and the list of dropped CSS (`css`'s module doc), lays the body out in pages
//! in the twelve default faces (`layout`'s and `fonts`' module docs), and writes the file
//! (`write`'s module doc). The component performs no I/O and reads no clock
//! (`rule:core-classes/pdf-render-has-no-io`): every byte it draws is in the document or the
//! component, so two renders of one document are the same bytes.
//!
//! Known gaps, each owned by a later stage of the goal that builds this component: the layout
//! is the block and inline flow `layout`'s module doc describes, with no tables, floats, flex or
//! grid, no `@page`, no break control, no hyphenation and no inline SVG drawn. The `first` and
//! `second` passes, `merge`, `read` and `read-text` return `runtime` until their stages land,
//! and so `rendered` lists no overflows, and a page has no name, strings or marks. The output is untagged, and the standards, the password and the
//! invoice XML are checked against each other and otherwise not applied (Stage 12 and
//! Stage 10).

pub mod assets;
pub mod css;
pub mod fonts;
pub mod layout;
pub mod paginate;
pub mod parse;
pub mod write;

/// The WIT `standard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standard {
    PdfA2b,
    PdfA3b,
    PdfUa1,
}

/// The WIT `instant`: seconds since the Unix epoch, and nanoseconds, which a PDF date does not
/// carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instant {
    pub seconds: i64,
    pub nanos: i64,
}

/// The WIT `asset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub data: Vec<u8>,
}

/// The WIT `render-options`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    pub standard: Option<Standard>,
    pub password: Option<String>,
    pub invoice_xml: Option<String>,
    pub created: Option<Instant>,
}

/// The WIT `document`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    pub html: String,
    pub assets: Vec<Asset>,
    pub options: Options,
}

/// The facts of one page that this stage has: its number, from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFacts {
    pub number: u64,
}

/// What [`render`] returns: the file, one [`PageFacts`] per page in order, every declaration,
/// rule and element the render dropped, and the text no font had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub data: Vec<u8>,
    pub pages: Vec<PageFacts>,
    pub dropped: Vec<css::Dropped>,
    pub missing_glyphs: Vec<layout::Missing>,
}

/// The WIT `error`: `Invalid` becomes `LogicError`, `Parse` becomes `ParseError` and `Runtime`
/// becomes `RuntimeError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(String),
    Parse(String),
    Runtime(String),
}

/// The `whole` pass: `document` laid out and written in one call.
///
/// An `invoice_xml` without [`Standard::PdfA3b`], or a `password` with either PDF/A standard,
/// returns `Invalid`, as `wit/pdf.wit` says. Otherwise `standard`, `password` and
/// `invoice_xml` are accepted and not yet applied (Stage 12). A reference the asset map does
/// not resolve returns `Invalid` naming it, and an image whose bytes do not decode returns
/// `Parse` naming it.
pub fn render(document: &Document) -> Result<Rendered, Error> {
    let options = &document.options;
    if options.invoice_xml.is_some() && options.standard != Some(Standard::PdfA3b) {
        return Err(Error::Invalid(
            "an invoice XML is embedded only under the PDF/A-3b standard".to_owned(),
        ));
    }
    if options.password.is_some()
        && matches!(options.standard, Some(Standard::PdfA2b | Standard::PdfA3b))
    {
        return Err(Error::Invalid(
            "a PDF/A document cannot have a password".to_owned(),
        ));
    }
    let dom = parse::parse(&document.html);
    let assets = assets::Assets::new(&document.assets);
    assets::check(&dom, &assets)?;
    let styles = css::cascade(&dom, &document.html, &assets);
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets)?;
    let laid = layout::layout(&dom, &styles, &assets, fonts)?;
    let data = write::write(&laid, options.created)?;
    let pages = (1..=laid.pages.len() as u64)
        .map(|number| PageFacts { number })
        .collect();
    Ok(Rendered {
        data,
        pages,
        dropped: styles.dropped,
        missing_glyphs: laid.missing,
    })
}

#[cfg(target_family = "wasm")]
mod guest {
    wit_bindgen::generate!({
        path: ["../../wit/nvs-ext", "../../wit/pdf.wit"],
        world: "nvs:pdf/pdf",
        generate_all,
    });

    use exports::nvs::pdf::engine::{
        Document, DroppedItem, Error, Guest, MissingGlyph, PageFacts, Part, Pass, PdfInfo,
        Rendered, Standard,
    };

    struct Component;

    fn error(err: crate::Error) -> Error {
        match err {
            crate::Error::Invalid(message) => Error::Invalid(message),
            crate::Error::Parse(message) => Error::Parse(message),
            crate::Error::Runtime(message) => Error::Runtime(message),
        }
    }

    /// What an export this stage does not have yet returns. `runtime` is the case for it: the
    /// program's call is well formed, and `invalid` would call it a bug in the program.
    fn not_yet(what: &str) -> Error {
        Error::Runtime(format!(
            "{what} is not available in this version of the PDF engine"
        ))
    }

    fn document(document: Document) -> crate::Document {
        let options = document.options;
        crate::Document {
            html: document.html,
            assets: document
                .assets
                .into_iter()
                .map(|asset| crate::Asset {
                    name: asset.name,
                    data: asset.data,
                })
                .collect(),
            options: crate::Options {
                standard: options.standard.map(|standard| match standard {
                    Standard::PdfA2b => crate::Standard::PdfA2b,
                    Standard::PdfA3b => crate::Standard::PdfA3b,
                    Standard::PdfUa1 => crate::Standard::PdfUa1,
                }),
                password: options.password,
                invoice_xml: options.invoice_xml,
                created: options.created.map(|created| crate::Instant {
                    seconds: created.seconds,
                    nanos: created.nanos,
                }),
            },
        }
    }

    impl Guest for Component {
        fn render(pass: Pass) -> Result<Rendered, Error> {
            let document = match pass {
                Pass::Whole(document) => self::document(document),
                Pass::First(_) => return Err(not_yet("the `first` pass")),
                Pass::Second(_) => return Err(not_yet("the `second` pass")),
            };
            let rendered = crate::render(&document).map_err(error)?;
            Ok(Rendered {
                page_count: rendered.pages.len() as u64,
                pages: rendered
                    .pages
                    .into_iter()
                    .map(|page| PageFacts {
                        number: page.number,
                        name: None,
                        strings: Vec::new(),
                        marks: Vec::new(),
                    })
                    .collect(),
                data: rendered.data,
                dropped: rendered
                    .dropped
                    .into_iter()
                    .map(|item| DroppedItem {
                        property: item.property,
                        value: item.value,
                        selector: item.selector,
                        line: item.line,
                        column: item.column,
                    })
                    .collect(),
                missing_glyphs: rendered
                    .missing_glyphs
                    .into_iter()
                    .map(|missing| MissingGlyph {
                        text: missing.text,
                        fonts: missing.fonts,
                        page: missing.page,
                    })
                    .collect(),
                overflows: Vec::new(),
            })
        }

        fn merge(_parts: Vec<Part>) -> Result<Vec<u8>, Error> {
            Err(not_yet("`merge`"))
        }

        fn read(_data: Vec<u8>) -> Result<PdfInfo, Error> {
            Err(not_yet("`read`"))
        }

        fn read_text(_data: Vec<u8>, _pages: Vec<u64>) -> Result<Vec<String>, Error> {
            Err(not_yet("`read-text`"))
        }
    }

    export!(Component);
}
