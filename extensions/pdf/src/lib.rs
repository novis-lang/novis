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
//! **A render with header or footer functions is two calls**
//! (`rule:core-classes/pdf-header-is-a-page-function`). [`first`] lays the body out as
//! [`render`] does, with the top or bottom margin a function's height gives, and returns a
//! [`Kept`] beside each page's facts. [`second`] lays each page's header and footer markup out
//! on its own, as a document whose page is the size of the margin's area, and draws it over that
//! margin with [`layout::overlay`]: the body does not move, a margin box along that side is not
//! drawn, and markup taller than the area is clipped and listed in [`Rendered::overflows`]. The
//! area is the page's content width and the whole margin's height, and the markup is set at its
//! top. The markup is styled by its own `<style>` and not the document's, and it uses the
//! document's assets and fonts. Each slot's fill is drawn the same way over every area of the
//! page whose `<slot>` has its name: its cell's content box, which does not grow, so the fill
//! moves nothing either. A slot's overflow names the slot. [`draw`] is that half of
//! [`second`], before the file is written. The guest keeps the [`Kept`] in the instance between
//! the two calls, and the request's end drops it.
//!
//! Known gaps, each owned by a later stage of the goal that builds this component: `merge`,
//! `read` and `read-text` return `runtime` until their stages land. The output is untagged, and
//! the standards, the password and the invoice XML are checked against each other and otherwise
//! not applied (Stage 12).

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

/// The facts of one page: its number from 1, its page name, its named strings and its
/// `data-mark` marks (`layout`'s `margin` module doc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageFacts {
    pub number: u64,
    pub name: Option<String>,
    pub strings: Vec<(String, String)>,
    pub marks: Vec<(String, String)>,
}

/// Markup taller than its area: the page from 1, `header`, `footer` or a slot's name, and the
/// markup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overflow {
    pub page: u64,
    pub area: String,
    pub text: String,
}

/// What a render returns: the file, which is empty after [`first`], one [`PageFacts`] per page
/// in order, every declaration, rule and element the render dropped, the text no font had, and
/// the markup taller than its area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub data: Vec<u8>,
    pub pages: Vec<PageFacts>,
    pub dropped: Vec<css::Dropped>,
    pub missing_glyphs: Vec<layout::Missing>,
    pub overflows: Vec<Overflow>,
}

/// The WIT `first-pass`'s heights: each a CSS length for the top or bottom margin of every
/// page, or `None` to keep `@page`'s.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Heights {
    pub header: Option<String>,
    pub footer: Option<String>,
}

/// The WIT `page-markup`: one page's header and footer HTML, and each slot's name and fill.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Markup {
    pub header: Option<String>,
    pub footer: Option<String>,
    pub slots: Vec<(String, String)>,
}

/// What [`first`] keeps for [`second`]: the laid-out body, the fonts it was laid out in, the
/// assets, the metadata, the creation time and the dropped list.
pub struct Kept {
    laid: layout::Laid,
    fonts: fonts::Fonts,
    assets: Vec<Asset>,
    info: write::Info,
    created: Option<Instant>,
    dropped: Vec<css::Dropped>,
}

impl Kept {
    /// The laid-out pages.
    pub fn laid(&self) -> &layout::Laid {
        &self.laid
    }
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
    let (kept, _) = first(document, &Heights::default())?;
    finish(kept, Vec::new())
}

/// The `first` pass: `document` laid out as [`render`] lays it out, each height in `heights`
/// replacing its margin, and each page's facts with no file.
///
/// A height that is not one CSS length of 0 or more returns `Invalid`, and so does everything
/// [`render`] returns `Invalid` for.
pub fn first(document: &Document, heights: &Heights) -> Result<(Kept, Rendered), Error> {
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
    let mut styles = css::cascade(&dom, &document.html, &assets);
    for (side, height, what) in [
        ("margin-top", &heights.header, "header"),
        ("margin-bottom", &heights.footer, "footer"),
    ] {
        if let Some(height) = height {
            let rule = paginate::margin_rule(side, height).ok_or_else(|| {
                Error::Invalid(format!(
                    "the {what} height `{height}` is not a CSS length of 0 or more"
                ))
            })?;
            styles.pages.push(rule);
        }
    }
    let fonts = fonts::Fonts::new(&styles.font_faces, &assets)?;
    let (laid, fonts) = layout::layout_keeping(&dom, &styles, &assets, fonts)?;
    let rendered = Rendered {
        data: Vec::new(),
        pages: facts(&laid),
        dropped: styles.dropped.clone(),
        missing_glyphs: laid.missing.clone(),
        overflows: Vec::new(),
    };
    let kept = Kept {
        laid,
        fonts,
        assets: document.assets.clone(),
        info: write::Info::of(&dom),
        created: options.created,
        dropped: styles.dropped,
    };
    Ok((kept, rendered))
}

/// The `second` pass: each page's markup in `pages` drawn over the body `kept` holds, as
/// [`draw`] draws it, and the file written.
pub fn second(kept: Kept, pages: &[Markup]) -> Result<Rendered, Error> {
    let (kept, overflows) = draw(kept, pages)?;
    finish(kept, overflows)
}

/// Draws each page's header and footer markup in `pages` over that page's margins, and each
/// slot's fill in every area of that page whose slot has its name. Returns the body with the
/// markup drawn, and the markup taller than its area. The body does not move.
///
/// A list whose length is not the page count returns `Invalid`, and so does markup that names
/// an asset the map does not have.
pub fn draw(kept: Kept, pages: &[Markup]) -> Result<(Kept, Vec<Overflow>), Error> {
    let count = kept.laid.pages.len();
    if pages.len() != count {
        return Err(Error::Invalid(format!(
            "the second pass has markup for {} pages, and the document has {count}",
            pages.len()
        )));
    }
    let Kept {
        mut laid,
        mut fonts,
        assets: owned,
        info,
        created,
        dropped,
    } = kept;
    let assets = assets::Assets::new(&owned);
    let mut overflows = Vec::new();
    for (index, markup) in pages.iter().enumerate() {
        let geometry = laid.pages[index].geometry;
        let [top, _, bottom, left] = geometry.margin;
        let width = geometry.content_width();
        let mut areas = vec![
            ("top", "header", markup.header.as_ref(), [left, 0.0, width, top]),
            (
                "bottom",
                "footer",
                markup.footer.as_ref(),
                [left, geometry.height - bottom, width, bottom],
            ),
        ];
        for (name, html) in &markup.slots {
            for (slot, area) in &laid.pages[index].slots {
                if slot == name {
                    areas.push(("slot", name.as_str(), Some(html), *area));
                }
            }
        }
        for (side, what, html, area) in areas {
            let Some(html) = html else { continue };
            let over = if area[2] > 0.0 && area[3] > 0.0 {
                let page = format!(
                    "<style>@page {{ size: {}pt {}pt; margin: 0 }}</style>",
                    area[2], area[3]
                );
                let source = page + html;
                let dom = parse::parse(&source);
                assets::check(&dom, &assets)?;
                let styles = css::cascade(&dom, &source, &assets);
                let (markup, back) = layout::layout_keeping(&dom, &styles, &assets, fonts)?;
                fonts = back;
                layout::overlay(&mut laid, index, side, area, markup)
            } else {
                !html.trim().is_empty()
            };
            if over {
                overflows.push(Overflow {
                    page: index as u64 + 1,
                    area: what.to_owned(),
                    text: html.clone(),
                });
            }
        }
    }
    let kept = Kept {
        laid,
        fonts,
        assets: owned,
        info,
        created,
        dropped,
    };
    Ok((kept, overflows))
}

/// The file of `kept`'s pages, with `overflows`.
fn finish(kept: Kept, overflows: Vec<Overflow>) -> Result<Rendered, Error> {
    let data = write::write(&kept.laid, &kept.info, kept.created)?;
    Ok(Rendered {
        data,
        pages: facts(&kept.laid),
        dropped: kept.dropped,
        missing_glyphs: kept.laid.missing,
        overflows,
    })
}

fn facts(laid: &layout::Laid) -> Vec<PageFacts> {
    laid.pages
        .iter()
        .zip(1..)
        .map(|(page, number)| PageFacts {
            number,
            name: page.name.clone(),
            strings: page.strings.clone(),
            marks: page.marks.clone(),
        })
        .collect()
}

#[cfg(target_family = "wasm")]
mod guest {
    wit_bindgen::generate!({
        path: ["../../wit/nvs-ext", "../../wit/pdf.wit"],
        world: "nvs:pdf/pdf",
        generate_all,
    });

    use std::cell::RefCell;

    use exports::nvs::pdf::engine::{
        Document, DroppedItem, Error, Guest, MissingGlyph, NamedValue, Overflow, PageFacts, Part,
        Pass, PdfInfo, Rendered, Standard,
    };

    struct Component;

    thread_local! {
        /// The body a `first` pass laid out, until the `second` pass writes it. Each request has
        /// an instance of its own, so this is one request's.
        static KEPT: RefCell<Option<crate::Kept>> = const { RefCell::new(None) };
    }

    fn named(pairs: Vec<(String, String)>) -> Vec<NamedValue> {
        pairs
            .into_iter()
            .map(|(name, value)| NamedValue { name, value })
            .collect()
    }

    fn rendered(rendered: crate::Rendered) -> Rendered {
        Rendered {
            page_count: rendered.pages.len() as u64,
            pages: rendered
                .pages
                .into_iter()
                .map(|page| PageFacts {
                    number: page.number,
                    name: page.name,
                    strings: named(page.strings),
                    marks: named(page.marks),
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
            overflows: rendered
                .overflows
                .into_iter()
                .map(|overflow| Overflow {
                    page: overflow.page,
                    area: overflow.area,
                    text: overflow.text,
                })
                .collect(),
        }
    }

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
            // Whatever this call is, the body an earlier `first` kept is used at most once.
            let body = KEPT.with(|kept| kept.borrow_mut().take());
            let out = match pass {
                Pass::Whole(document) => crate::render(&self::document(document)),
                Pass::First(first) => {
                    let heights = crate::Heights {
                        header: first.header_height,
                        footer: first.footer_height,
                    };
                    crate::first(&self::document(first.document), &heights).map(
                        |(body, rendered)| {
                            KEPT.with(|kept| *kept.borrow_mut() = Some(body));
                            rendered
                        },
                    )
                }
                Pass::Second(margins) => {
                    let pages: Vec<crate::Markup> = margins
                        .pages
                        .into_iter()
                        .map(|page| crate::Markup {
                            header: page.header,
                            footer: page.footer,
                            slots: page
                                .slots
                                .into_iter()
                                .map(|slot| (slot.name, slot.value))
                                .collect(),
                        })
                        .collect();
                    match body {
                        Some(body) => crate::second(body, &pages),
                        None => Err(crate::Error::Invalid(
                            "the second pass needs a first pass in the same request".to_owned(),
                        )),
                    }
                }
            };
            out.map(rendered).map_err(error)
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
