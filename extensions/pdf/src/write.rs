//! The writer: laid-out pages as a PDF file, by `krilla` (`rule:core-classes/pdf-output-is-inert`).
//!
//! Each page has the size its `@page` rules give, and each glyph run is drawn in its colour and
//! its face, a default or an `@font-face` font, which `krilla` subsets to the glyphs the document
//! draws. Each path is filled with the even-odd rule or stroked, in a colour, a gradient or an
//! image's tiles, and each image is drawn at its box. A group is a clip path, an opacity or both,
//! pushed at its start and popped at its end.
//!
//! Each [`Item::Link`] is a link annotation with no border: a URI action for an address, and a
//! destination at the page and top of the `id` for an anchor, left out when no element has that
//! `id`. The headings are the outline, each one under the last heading of a lower level before
//! it. [`Info`] is the document's title, authors, description, keywords and language.
//!
//! The file is the same bytes for the same input. Nothing here reads a clock: the creation
//! time is written only when the program passes one, in UTC. The file's `/ID` and its XMP
//! document and instance ids are `krilla`'s defaults, a SipHash of the serialized file with
//! fixed keys, so they come from the content and nothing else. Every collection the writer
//! walks is a `Vec` in document order. `krilla` writes no JavaScript, launch action, open
//! action, form field or attachment unless asked, and this writer asks for none.
//!
//! **Every file is tagged.** The layout brackets an element's content with [`Item::Tag`] and
//! [`Item::Untag`] in its [`Role`], and the writer builds `krilla`'s tag tree from them in page
//! order, so an element that goes on past a page is one group. A heading is `H1` to `H6`, titled
//! by its text, a list `L` of `LI` each holding an `LBody`, a table `Table` of `TR` of `TH` and
//! `TD`, an `<img>` a `Figure` with its `alt`, and the tree carries the document's `lang`. A run
//! of text is a span leaf and an image an `Other` leaf of the innermost element open; text
//! outside every element is a paragraph of its own. A path, a margin box and a table row drawn
//! again on a later page are artifacts. A piece of a row split across pages is a `TR` of plain
//! `TD`s, and a link annotation is not in the tree.
//!
//! **A standard is `krilla`'s validator for it**: PDF/A-2b and PDF/A-3b are its archival
//! validators, and PDF/UA-1 its accessibility one. A file that breaks the standard returns
//! `Invalid`, naming each thing that broke ([`broken`]). PDF/A needs the creation time, because
//! nothing here reads a clock, and PDF/UA-1 needs a `<title>`, an `alt` on every image and an
//! outline, which is empty when the document has no heading. Every link annotation carries its
//! target as its alternate text, under any standard or none.
//!
//! **The invoice XML is the one attachment** (`factur_x`'s module doc): embedded beside the
//! metadata, and its XMP fields written into the finished file. **A password encrypts the
//! finished file** with AES-256 (`encrypt`'s module doc).

use krilla::action::LinkAction;
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::color::rgb;
use krilla::configure::{Accessibility, Archival, ConfigurationBuilder, ValidationError};
use krilla::destination::XyzDestination;
use krilla::geom::{Path, PathBuilder, Point, Rect, Size, Transform};
use krilla::metadata::{DateTime, Metadata};
use krilla::num::NormalizedF32;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::paint::{
    Fill, FillRule, LineCap, LinearGradient, Pattern, RadialGradient, SpreadMethod, Stop, Stroke,
    StrokeDash,
};
use krilla::surface::Surface;
use krilla::tagging::{
    Artifact, ArtifactType, ContentTag, Identifier, ListNumbering, Node, SpanTag, TableHeaderScope,
    Tag, TagGroup, TagTree,
};
use krilla::text::{Font, GlyphId, KrillaGlyph};
use krilla::{Data, Document, SerializeSettings};
use krilla_svg::{SurfaceExt, SvgSettings};
use std::num::NonZeroU16;

use crate::css::Color;
use crate::encrypt;
use crate::factur_x::{self, Level};
use crate::layout::{self, Bookmark, Href, Item, Laid, Paint, Picture, Role, Seg};
use crate::paginate::PageBox;
use crate::parse::{Dom, Space};
use crate::{Error, Instant, Options, Standard};

/// Draws `picture` with its top-left corner at the surface's origin, stretched to `size`. An
/// SVG is drawn as vector paths, and a PDF page as a form XObject of its content.
fn draw_picture(surface: &mut Surface<'_>, picture: &Picture, size: Size) {
    match picture {
        Picture::Raster(image) => surface.draw_image(image.clone(), size),
        Picture::Vector(tree) => {
            surface.draw_svg(tree, size, SvgSettings::default());
        }
        Picture::Page(page) => surface.draw_pdf_page(&page.document, size, page.index),
    }
}

fn fill(surface: &mut Surface<'_>, color: Color) {
    surface.set_fill(Some(Fill {
        paint: rgb_of(color).into(),
        opacity: normal(color.a),
        ..Fill::default()
    }));
}

fn rgb_of(color: Color) -> krilla::color::Color {
    rgb::Color::new(color.r, color.g, color.b).into()
}

fn normal(value: f32) -> NormalizedF32 {
    NormalizedF32::new(value.clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE)
}

/// `path` as a `krilla` path, or `None` when it has no area or no length.
fn shape(path: &[Seg]) -> Option<Path> {
    let mut builder = PathBuilder::new();
    for seg in path {
        match *seg {
            Seg::Move(x, y) => builder.move_to(x, y),
            Seg::Line(x, y) => builder.line_to(x, y),
            Seg::Cubic(x1, y1, x2, y2, x, y) => builder.cubic_to(x1, y1, x2, y2, x, y),
            Seg::Close => builder.close(),
        }
    }
    builder.finish()
}

/// `paint` as a `krilla` paint and the opacity it is drawn at. A gradient's stops carry their
/// own opacity, and an image's tile is drawn once into a pattern.
fn paint_of(
    surface: &mut Surface<'_>,
    paint: &Paint,
    laid: &Laid,
) -> Option<(krilla::paint::Paint, NormalizedF32)> {
    let stops = |stops: &[layout::Stop]| -> Vec<Stop> {
        stops
            .iter()
            .map(|s| Stop {
                offset: normal(s.offset),
                color: rgb_of(s.color),
                opacity: normal(s.color.a),
            })
            .collect()
    };
    Some(match paint {
        Paint::Color(color) => (rgb_of(*color).into(), normal(color.a)),
        Paint::Linear {
            x1,
            y1,
            x2,
            y2,
            stops: each,
        } => (
            LinearGradient {
                x1: *x1,
                y1: *y1,
                x2: *x2,
                y2: *y2,
                transform: Transform::identity(),
                spread_method: SpreadMethod::Pad,
                stops: stops(each),
                anti_alias: false,
            }
            .into(),
            NormalizedF32::ONE,
        ),
        Paint::Radial {
            cx,
            cy,
            r,
            scale_y,
            stops: each,
        } => (
            RadialGradient {
                fx: *cx,
                fy: *cy,
                fr: 0.0,
                cx: *cx,
                cy: *cy,
                cr: *r,
                // Stretches the circle down about its centre into the ellipse.
                transform: Transform::from_row(1.0, 0.0, 0.0, *scale_y, 0.0, cy * (1.0 - scale_y)),
                spread_method: SpreadMethod::Pad,
                stops: stops(each),
                anti_alias: false,
            }
            .into(),
            NormalizedF32::ONE,
        ),
        Paint::Tiles {
            image,
            x,
            y,
            width,
            height,
        } => {
            let size = Size::from_wh(*width, *height)?;
            let mut builder = surface.stream_builder();
            let mut tile = builder.surface();
            draw_picture(&mut tile, &laid.images[*image].0, size);
            tile.finish();
            let stream = builder.finish();
            (
                Pattern {
                    stream,
                    transform: Transform::from_translate(*x, *y),
                    width: *width,
                    height: *height,
                }
                .into(),
                NormalizedF32::ONE,
            )
        }
    })
}

/// What a document says about itself: the text of its first `<title>`, the `content` of its
/// `<meta name>` for `author`, `description` and `keywords`, and the `lang` of its root element.
/// The keywords are split at commas, and an empty value is left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Info {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub lang: Option<String>,
}

impl Info {
    pub fn of(dom: &Dom) -> Info {
        let mut info = Info::default();
        let words = |text: &str| {
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            (!text.is_empty()).then_some(text)
        };
        for (at, node) in dom.nodes.iter().enumerate() {
            match node.element(Space::Html) {
                Some("html") if info.lang.is_none() => {
                    info.lang = node.attr("lang").and_then(words);
                }
                Some("title") if info.title.is_none() => info.title = words(&dom.text(at)),
                Some("meta") => {
                    let content = node.attr("content").unwrap_or_default();
                    match node.attr("name").map(str::to_ascii_lowercase).as_deref() {
                        Some("author") => info.authors.extend(words(content)),
                        Some("description") if info.description.is_none() => {
                            info.description = words(content);
                        }
                        Some("keywords") => {
                            info.keywords.extend(content.split(',').filter_map(words));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        info
    }
}

/// An element whose content is being tagged: its role, its children so far and its text.
struct Open {
    role: Role,
    children: Vec<Node>,
    text: String,
}

/// The tag tree, built while the pages are drawn in order.
#[derive(Default)]
struct Tags {
    /// The elements open, innermost last. An element goes on across a page break.
    open: Vec<Open>,
    /// How many tags deep the content is inside a [`Role::Artifact`], 0 outside one.
    artifact: usize,
    /// The groups closed at the top, and the paragraph collecting text outside every element.
    top: Vec<Node>,
    loose: Vec<Node>,
}

impl Tags {
    fn tag(&mut self, role: &Role) {
        if self.artifact > 0 || *role == Role::Artifact {
            self.artifact += 1;
            return;
        }
        self.flush();
        self.open.push(Open {
            role: role.clone(),
            children: Vec::new(),
            text: String::new(),
        });
    }

    fn untag(&mut self) {
        if self.artifact > 0 {
            self.artifact -= 1;
            return;
        }
        let Some(Open {
            role,
            children,
            text,
        }) = self.open.pop()
        else {
            return;
        };
        let group = match role {
            Role::Heading(level) => {
                let level = NonZeroU16::new(level.into()).unwrap_or(NonZeroU16::MIN);
                let title = text.split_whitespace().collect::<Vec<_>>().join(" ");
                TagGroup::with_children(Tag::Hn(level, Some(title)), children)
            }
            Role::Paragraph => TagGroup::with_children(Tag::P, children),
            Role::List { ordered } => {
                let numbering = if ordered {
                    ListNumbering::Decimal
                } else {
                    ListNumbering::Disc
                };
                TagGroup::with_children(Tag::L(numbering), children)
            }
            Role::ListItem => TagGroup::with_children(
                Tag::LI,
                vec![TagGroup::with_children(Tag::LBody, children).into()],
            ),
            Role::Quote => TagGroup::with_children(Tag::BlockQuote, children),
            Role::Figure(alt) => TagGroup::with_children(Tag::Figure(alt), children),
            Role::Caption => TagGroup::with_children(Tag::Caption, children),
            Role::Table => TagGroup::with_children(Tag::Table, children),
            Role::Row => TagGroup::with_children(Tag::TR, children),
            Role::HeaderCell => {
                TagGroup::with_children(Tag::TH(TableHeaderScope::Column), children)
            }
            Role::Cell => TagGroup::with_children(Tag::TD, children),
            Role::Artifact => return,
        };
        match self.open.last_mut() {
            Some(parent) => {
                parent.text.push(' ');
                parent.text.push_str(&text);
                parent.children.push(group.into());
            }
            None => self.top.push(group.into()),
        }
    }

    /// The content tag of the next leaf, a run of text or not: `artifact` when it is not part
    /// of the document's text.
    fn content(&self, text: bool, artifact: Option<ArtifactType>) -> ContentTag<'static> {
        let artifact = artifact.or((self.artifact > 0).then_some(ArtifactType::Layout));
        match artifact {
            Some(kind) => ContentTag::Artifact(Artifact::with_kind(kind)),
            None if text => ContentTag::Span(SpanTag::empty()),
            None => ContentTag::Other,
        }
    }

    /// Adds the leaf `id` and its `text` to the innermost element open.
    fn leaf(&mut self, id: Identifier, text: &str) {
        if self.artifact > 0 {
            return;
        }
        match self.open.last_mut() {
            Some(open) => {
                open.children.push(id.into());
                open.text.push_str(text);
            }
            None => self.loose.push(id.into()),
        }
    }

    /// Closes the paragraph of text outside every element.
    fn flush(&mut self) {
        if self.open.is_empty() && !self.loose.is_empty() {
            let loose = std::mem::take(&mut self.loose);
            self.top.push(TagGroup::with_children(Tag::P, loose).into());
        }
    }

    fn finish(mut self, lang: Option<String>) -> TagTree {
        while !self.open.is_empty() {
            self.untag();
        }
        self.flush();
        let mut tree = TagTree::new().with_lang(lang);
        for node in self.top {
            tree.push(node);
        }
        tree
    }
}

/// The outline of `bookmarks`: each one under the last before it of a lower level.
fn outline(bookmarks: &[Bookmark]) -> Outline {
    // The open entries, outermost first, each with its level.
    let mut open: Vec<(u8, OutlineNode)> = Vec::new();
    let mut outline = Outline::new();
    let close = |open: &mut Vec<(u8, OutlineNode)>, outline: &mut Outline| {
        let (_, node) = open.pop().expect("an entry is open");
        match open.last_mut() {
            Some((_, parent)) => parent.push_child(node),
            None => outline.push_child(node),
        }
    };
    for bookmark in bookmarks {
        while open
            .last()
            .is_some_and(|(level, _)| *level >= bookmark.level)
        {
            close(&mut open, &mut outline);
        }
        let to = XyzDestination::new(bookmark.page, Point::from_xy(0.0, bookmark.y));
        open.push((bookmark.level, OutlineNode::new(bookmark.title.clone(), to)));
    }
    while !open.is_empty() {
        close(&mut open, &mut outline);
    }
    outline
}

/// The PDF file of `laid` with `info` as its metadata, created at `options.created` when there
/// is one, under `options.standard` when there is one, and carrying `options.invoice_xml` as its
/// Factur-X invoice of `invoice`'s level when both are there.
pub fn write(
    laid: &Laid,
    info: &Info,
    options: &Options,
    invoice: Option<Level>,
) -> Result<Vec<u8>, Error> {
    let configuration = match options.standard {
        None => ConfigurationBuilder::new(),
        Some(Standard::PdfA2b) => {
            ConfigurationBuilder::new().with_archival_validator(Archival::A2_B)
        }
        Some(Standard::PdfA3b) => {
            ConfigurationBuilder::new().with_archival_validator(Archival::A3_B)
        }
        Some(Standard::PdfUa1) => {
            ConfigurationBuilder::new().with_accessibility_validator(Accessibility::UA1)
        }
    }
    .finish()
    .map_err(|err| Error::Runtime(format!("the standard could not be configured: {err:?}")))?;
    let mut document = Document::new_with(SerializeSettings {
        enable_tagging: true,
        configuration,
        ..SerializeSettings::default()
    });
    let mut metadata = Metadata::new()
        .authors(info.authors.clone())
        .keywords(info.keywords.clone());
    if let Some(title) = &info.title {
        metadata = metadata.title(title.clone());
    }
    if let Some(description) = &info.description {
        metadata = metadata.description(description.clone());
    }
    if let Some(lang) = &info.lang {
        metadata = metadata.language(lang.clone());
    }
    let created = options.created.map(date).transpose()?;
    if let Some(created) = created {
        metadata = metadata.creation_date(created);
    }
    document.set_metadata(metadata);
    if let (Some(xml), Some(level)) = (&options.invoice_xml, invoice) {
        factur_x::embed(&mut document, xml, level, created);
    }
    // `krilla` leaves an empty outline out unless the standard needs one.
    document.set_outline(outline(&laid.bookmarks));
    let mut faces: Vec<Option<Font>> = vec![None; laid.faces.len()];
    let mut tags = Tags::default();
    for laid_page in &laid.pages {
        let mut links: Vec<Annotation> = Vec::new();
        let PageBox { width, height, .. } = laid_page.geometry;
        let size = PageSettings::from_wh(width, height).ok_or_else(|| {
            Error::Invalid(format!(
                "a page {width} by {height} points is not a size a PDF page can have"
            ))
        })?;
        let mut page = document.start_page_with(size);
        let mut surface = page.surface();
        // How many states each open group pushed, innermost last.
        let mut groups: Vec<usize> = Vec::new();
        for (index, item) in laid_page.items.iter().enumerate() {
            // A margin box repeats on every page, so it is not part of the document's text.
            let margin = laid_page
                .boxes
                .iter()
                .any(|(_, range)| range.contains(&index))
                .then_some(ArtifactType::PaginationOther);
            match item {
                Item::Text {
                    face,
                    size,
                    x,
                    y,
                    color,
                    glyphs,
                    text,
                    ..
                } => {
                    let font = match &faces[*face] {
                        Some(font) => font.clone(),
                        None => {
                            let font = Font::new(Data::from(laid.faces[*face].clone()), 0)
                                .ok_or_else(|| {
                                    Error::Parse("a font of the document does not parse".to_owned())
                                })?;
                            faces[*face] = Some(font.clone());
                            font
                        }
                    };
                    let glyphs: Vec<KrillaGlyph> = glyphs
                        .iter()
                        .map(|g| KrillaGlyph {
                            glyph_id: GlyphId::new(g.id),
                            text_range: g.text.clone(),
                            x_advance: g.advance / size,
                            x_offset: g.x / size,
                            // `parley`'s offset points down the page and `krilla`'s up it.
                            y_offset: -g.y / size,
                            y_advance: 0.0,
                            location: None,
                        })
                        .collect();
                    fill(&mut surface, *color);
                    let id = surface.start_tagged(tags.content(true, margin));
                    surface.draw_glyphs(Point::from_xy(*x, *y), &glyphs, font, text, *size, false);
                    surface.end_tagged();
                    if margin.is_none() {
                        tags.leaf(id, text);
                    }
                }
                Item::Path {
                    path,
                    paint,
                    stroke,
                } => {
                    let Some(path) = shape(path) else {
                        continue;
                    };
                    let (paint, opacity) = match paint_of(&mut surface, paint, laid) {
                        Some(paint) => paint,
                        None => continue,
                    };
                    match stroke {
                        None => surface.set_fill(Some(Fill {
                            paint,
                            opacity,
                            rule: FillRule::EvenOdd,
                        })),
                        Some(stroke) => {
                            surface.set_fill(None);
                            surface.set_stroke(Some(Stroke {
                                paint,
                                width: stroke.width,
                                line_cap: if stroke.round {
                                    LineCap::Round
                                } else {
                                    LineCap::Butt
                                },
                                opacity,
                                dash: (!stroke.dash.is_empty()).then(|| StrokeDash {
                                    array: stroke.dash.clone(),
                                    offset: 0.0,
                                }),
                                ..Stroke::default()
                            }));
                        }
                    }
                    surface
                        .start_tagged(tags.content(false, margin.or(Some(ArtifactType::Layout))));
                    surface.draw_path(&path);
                    surface.end_tagged();
                    surface.set_stroke(None);
                }
                Item::Begin {
                    opacity,
                    clip,
                    transform,
                } => {
                    let mut pushed = 0;
                    if let Some([a, b, c, d, e, f]) = *transform {
                        surface.push_transform(&Transform::from_row(a, b, c, d, e, f));
                        pushed += 1;
                    }
                    if let Some(clip) = clip.as_deref().and_then(shape) {
                        surface.push_clip_path(&clip, &FillRule::EvenOdd);
                        pushed += 1;
                    }
                    if *opacity < 1.0 {
                        surface.push_opacity(normal(*opacity));
                        pushed += 1;
                    }
                    groups.push(pushed);
                }
                Item::End => {
                    for _ in 0..groups.pop().unwrap_or(0) {
                        surface.pop();
                    }
                }
                Item::Image {
                    image,
                    x,
                    y,
                    width,
                    height,
                } => {
                    let Some(box_size) = Size::from_wh(*width, *height) else {
                        continue;
                    };
                    let id = surface.start_tagged(tags.content(false, margin));
                    surface.push_transform(&Transform::from_translate(*x, *y));
                    draw_picture(&mut surface, &laid.images[*image].0, box_size);
                    surface.pop();
                    surface.end_tagged();
                    if margin.is_none() {
                        tags.leaf(id, "");
                    }
                }
                Item::Tag(role) => {
                    if margin.is_none() {
                        tags.tag(role);
                    }
                }
                Item::Untag => {
                    if margin.is_none() {
                        tags.untag();
                    }
                }
                // The layout takes every one off its page before the writer reads it.
                Item::Here { .. } => {}
                Item::Link {
                    href,
                    x,
                    y,
                    width,
                    height,
                } => {
                    let Some(rect) = Rect::from_xywh(*x, *y, *width, *height) else {
                        continue;
                    };
                    let target = match href {
                        Href::Uri(uri) => Target::Action(LinkAction::new(uri.clone()).into()),
                        Href::Anchor(id) => match laid.anchors.get(id) {
                            Some(&(page, top)) => Target::Destination(
                                XyzDestination::new(page, Point::from_xy(0.0, top)).into(),
                            ),
                            None => continue,
                        },
                    };
                    let alt = match href {
                        Href::Uri(uri) => format!("Link to {uri}"),
                        Href::Anchor(id) => format!("Link to #{id}"),
                    };
                    links.push(Annotation::new_link(
                        LinkAnnotation::new(rect, target),
                        Some(alt),
                    ));
                }
            }
        }
        for _ in 0..groups.into_iter().sum::<usize>() {
            surface.pop();
        }
        surface.finish();
        for link in links {
            page.add_annotation(link);
        }
        page.finish();
    }
    document.set_tag_tree(tags.finish(info.lang.clone()));
    let file = document.finish().map_err(|err| match err {
        krilla::error::KrillaError::Image(image, _, why) => {
            let name = laid
                .images
                .iter()
                .find(|(each, _)| matches!(each, Picture::Raster(each) if *each == image))
                .map_or("an image", |(_, name)| name.as_str());
            Error::Parse(format!("`{name}` does not decode as an image: {why}"))
        }
        krilla::error::KrillaError::Validation(errors) => {
            let mut what: Vec<String> = Vec::new();
            for (error, _) in &errors {
                let one = broken(error);
                if !what.contains(&one) {
                    what.push(one);
                }
            }
            let standard = match options.standard {
                Some(Standard::PdfA2b) => "PDF/A-2b",
                Some(Standard::PdfA3b) => "PDF/A-3b",
                Some(Standard::PdfUa1) | None => "PDF/UA-1",
            };
            Error::Invalid(format!(
                "the document does not meet {standard}: {}",
                what.join("; ")
            ))
        }
        other => Error::Runtime(format!("the PDF could not be written: {other}")),
    })?;
    let file = match invoice {
        Some(level) if options.invoice_xml.is_some() => factur_x::xmp(file, level)?,
        _ => file,
    };
    match &options.password {
        Some(password) => encrypt::encrypt(&file, password),
        None => Ok(file),
    }
}

/// What `error` says broke the standard, in the words of the document's author.
fn broken(error: &ValidationError) -> String {
    match error {
        ValidationError::MissingAltText(_) => "an image has no `alt` text".to_owned(),
        ValidationError::NoDocumentTitle => "the document has no `<title>`".to_owned(),
        ValidationError::NoDocumentLanguage => "the `<html>` element has no `lang`".to_owned(),
        ValidationError::MissingDocumentDate => {
            "the document has no creation time. Set `created` in the options".to_owned()
        }
        ValidationError::MissingHeadingTitle => "a heading has no text".to_owned(),
        ValidationError::MissingDocumentOutline => "the document has no outline".to_owned(),
        ValidationError::MissingAnnotationAltText(_) => "a link has no description".to_owned(),
        ValidationError::MissingTagging => "the file is not tagged".to_owned(),
        ValidationError::ContainsNotDefGlyph(_, _, text) => {
            format!("no font has a glyph for `{text}`")
        }
        ValidationError::NoCodepointMapping(..)
        | ValidationError::InvalidCodepointMapping(..)
        | ValidationError::UnicodePrivateArea(..) => {
            "a glyph has no character a reader can copy".to_owned()
        }
        ValidationError::RestrictedLicense(_) => {
            "a font's licence does not allow it to be embedded".to_owned()
        }
        ValidationError::Transparency(_) => "the document uses transparency".to_owned(),
        ValidationError::ContainsPostScript(_) => "a gradient repeats".to_owned(),
        ValidationError::ImageInterpolation(_) => "an image is smoothed when scaled".to_owned(),
        ValidationError::EmbeddedFile(..) => "the document has an attached file".to_owned(),
        ValidationError::EmbeddedPDF(_) => "the document places a PDF page".to_owned(),
        ValidationError::TooHighQNestingLevel => "clip paths are nested too deeply".to_owned(),
        ValidationError::TooLongString
        | ValidationError::TooLongName
        | ValidationError::TooLongArray
        | ValidationError::TooLongDictionary
        | ValidationError::TooLargeFloat
        | ValidationError::TooManyIndirectObjects => "the file is too large".to_owned(),
        ValidationError::MissingCMYKProfile
        | ValidationError::InconsistentSeparationFallback(_) => {
            "a colour has no profile".to_owned()
        }
        ValidationError::RequiresNewerPdfVersion(..) => {
            "the file needs a newer PDF version".to_owned()
        }
    }
}

/// `instant` as a UTC date and time, to the second. A year before 1 or after 9999 returns
/// `invalid`.
fn date(instant: Instant) -> Result<DateTime, Error> {
    let days = instant.seconds.div_euclid(86_400);
    let seconds = instant.seconds.rem_euclid(86_400);
    let (year, month, day) = civil(days);
    if !(1..=9999).contains(&year) {
        return Err(Error::Invalid(format!(
            "the creation time is in the year {year}, and a PDF date is between 1 and 9999"
        )));
    }
    Ok(DateTime::new(year as u16)
        .month(month)
        .day(day)
        .hour((seconds / 3600) as u8)
        .minute((seconds / 60 % 60) as u8)
        .second((seconds % 60) as u8)
        .utc_offset_hour(0)
        .utc_offset_minute(0))
}

/// The proleptic Gregorian year, month and day `days` after 1970-01-01.
fn civil(days: i64) -> (i64, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::civil;

    #[test]
    fn days_are_civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_723), (2024, 1, 1));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }
}
