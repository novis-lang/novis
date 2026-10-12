//! The cascade: every element's computed style, from the document's `<style>` blocks, its
//! `<link rel=stylesheet>` sheets and their `@import`s from the asset map, and its `style`
//! attributes (`rule:core-classes/pdf-css-subset`).
//!
//! The engine has its own cascade over `cssparser` and `selectors`. A sheet is read into rules,
//! each a selector list and its declarations. `@media` applies its rules when a query names
//! `print` or `all`, and never `screen`; a query with a media feature does not apply. `@page`
//! and `@font-face` are kept for the stages that lay pages out and load fonts, with `@page`'s
//! margin boxes beside its declarations. Any other at-rule, a selector outside the subset, a
//! property outside it, and a value its property does not take are dropped and reported in
//! [`Styles::dropped`] with the selector and the source position, never guessed at.
//!
//! [`USER_AGENT`] is read before the document's sheets and sorts below every one of them: it gives
//! HTML's elements their `display`, their default margins and their fonts. An element the subset
//! does not render ([`UNRENDERED`]) is `display: none` there and is reported in
//! [`Styles::dropped`] as `<name>` with an empty property, at its start tag.
//!
//! For each element, the declarations that apply are sorted by `!important`, then the document
//! over the user-agent sheet, then a `style` attribute over a sheet, then specificity, then source
//! order, and the last one sets each longhand. `font-weight` computes to a number, so `bolder`
//! counts from the parent's. A shorthand sets all of its longhands (`properties`' module doc). Custom
//! properties inherit, and a `var()` is substituted when the element is computed. A value that is
//! no longer valid after substitution is reported once and leaves its longhands unset. An
//! inherited longhand an element does not set takes its parent's computed value. A longhand absent
//! from [`Style::values`] has its initial value, which the layout knows.
//!
//! A position counts lines and characters from 1. A `<style>` block or a `style` attribute is
//! found in the HTML by its text, in document order. An attribute written with a character
//! reference is not found that way, and its declarations are reported at the position the search
//! reached. A position in a linked or imported sheet counts in that sheet.
//!
//! Cost: every element is tested against every selector, and the pseudo-element passes only
//! against selectors that end in one. Memory: one [`Style`] per element, holding its declared and
//! inherited longhands, freed when the render returns.

pub mod properties;
pub mod select;
pub mod value;

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use cssparser::{Delimiter, ParseError, Parser, SourceLocation, Token};
use selectors::SelectorList;
use selectors::context::SelectorCaches;

use crate::assets::Assets;
use crate::parse::{Dom, Kind, Space};
use select::{Impl, PseudoElement, Tree};
use value::Specified;
pub use value::{Color, Length, Value};

/// The font size an element has when nothing sets one: CSS's `medium`, in points.
pub const DEFAULT_FONT_SIZE: f32 = 12.0;

/// How deep `@import` may nest. An import cycle ends here.
const MAX_IMPORT_DEPTH: usize = 16;

/// The `@font-face` descriptors the subset reads.
pub const FONT_FACE_DESCRIPTORS: &[&str] = &[
    "font-family",
    "src",
    "font-weight",
    "font-style",
    "font-stretch",
    "unicode-range",
];

/// The elements the subset does not render: each is drawn as nothing and reported.
pub const UNRENDERED: &[&str] = &[
    "iframe", "video", "audio", "canvas", "object", "embed", "input", "select", "textarea",
    "button",
];

/// The user-agent sheet: HTML's default display, margins and fonts for print.
pub const USER_AGENT: &str = "
html, body, div, p, ul, ol, dl, dt, dd, pre, blockquote, figure, figcaption, h1, h2, h3, h4,
h5, h6, hr, section, article, header, footer, main, nav, aside, address, form, fieldset,
legend, details, summary, center, table, caption, thead, tbody, tfoot, tr, td, th
    { display: block }
li { display: list-item }
head, script, style, title, meta, link, base, template, noscript, iframe, video, audio, canvas,
object, embed, input, select, textarea, button, math, datalist, source, track, area, map
    { display: none }
body { margin: 8px }
p, pre, blockquote, figure, ul, ol, dl { margin-top: 1em; margin-bottom: 1em }
blockquote, figure { margin-left: 40px; margin-right: 40px }
dd { margin-left: 40px }
ul, ol { padding-left: 40px }
ul { list-style-type: disc }
ol { list-style-type: decimal }
h1 { font-size: 2em; margin-top: 0.67em; margin-bottom: 0.67em }
h2 { font-size: 1.5em; margin-top: 0.83em; margin-bottom: 0.83em }
h3 { font-size: 1.17em; margin-top: 1em; margin-bottom: 1em }
h4 { margin-top: 1.33em; margin-bottom: 1.33em }
h5 { font-size: 0.83em; margin-top: 1.67em; margin-bottom: 1.67em }
h6 { font-size: 0.67em; margin-top: 2.33em; margin-bottom: 2.33em }
h1, h2, h3, h4, h5, h6, b, strong, th { font-weight: bolder }
i, em, cite, var, dfn, address { font-style: italic }
pre, code, kbd, samp, tt { font-family: monospace }
pre { white-space: pre }
center, th { text-align: center }
u, ins { text-decoration-line: underline }
s, strike, del { text-decoration-line: line-through }
hr { border-top: 1px solid #808080; margin-top: 0.5em; margin-bottom: 0.5em }
[dir], bdi { unicode-bidi: isolate }
bdo { unicode-bidi: bidi-override }
bdo[dir] { unicode-bidi: isolate-override }
[dir=rtl] { direction: rtl }
[dir=ltr] { direction: ltr }
[dir=auto] { unicode-bidi: plaintext }
";

/// The 16 margin boxes an `@page` rule may hold.
pub const MARGIN_BOXES: &[&str] = &[
    "top-left-corner",
    "top-left",
    "top-center",
    "top-right",
    "top-right-corner",
    "right-top",
    "right-middle",
    "right-bottom",
    "bottom-right-corner",
    "bottom-right",
    "bottom-center",
    "bottom-left",
    "bottom-left-corner",
    "left-bottom",
    "left-middle",
    "left-top",
];

/// A declaration or a rule the subset does not take. A dropped rule, at-rule or selector has
/// an empty `property` or names the at-rule as `@name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dropped {
    pub property: String,
    pub value: String,
    pub selector: String,
    pub line: u64,
    pub column: u64,
}

/// One element's computed style.
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub values: BTreeMap<&'static str, Value>,
    custom: Rc<BTreeMap<String, String>>,
}

impl Style {
    /// The computed value of the longhand `name`, or `None` for its initial value.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    /// The computed font size in points.
    pub fn font_size(&self) -> f32 {
        match self.values.get("font-size") {
            Some(Value::Length(length)) => length.pt,
            _ => DEFAULT_FONT_SIZE,
        }
    }

    /// The custom property `name`, `--` included, with its `var()`s substituted.
    pub fn custom(&self, name: &str) -> Option<&str> {
        self.custom.get(name).map(String::as_str)
    }
}

/// An `@page` rule: its selector text (`:first`, `chapter:left` or empty), its declarations and
/// its margin boxes, each computed with the root's custom properties and the default font size.
#[derive(Debug, Clone, Default)]
pub struct PageRule {
    pub selector: String,
    pub values: Vec<(&'static str, Value)>,
    pub boxes: Vec<(String, Vec<(&'static str, Value)>)>,
}

/// The cascade's result for a document.
#[derive(Debug, Default)]
pub struct Styles {
    /// Each node's computed style, `None` for a node that is not an element.
    pub elements: Vec<Option<Style>>,
    /// The `::before` and `::after` styles of the elements a rule gives one.
    pub before: BTreeMap<usize, Style>,
    pub after: BTreeMap<usize, Style>,
    pub pages: Vec<PageRule>,
    /// Each `@font-face` rule's descriptors as written.
    pub font_faces: Vec<Vec<(String, String)>>,
    pub dropped: Vec<Dropped>,
}

impl Styles {
    /// The computed style of the element at `at`.
    pub fn of(&self, at: usize) -> Option<&Style> {
        self.elements.get(at).and_then(Option::as_ref)
    }
}

/// One declaration as written: the property in lower case, its value without `!important`,
/// and its position.
#[derive(Debug, Clone)]
struct Decl {
    property: String,
    value: String,
    important: bool,
    line: u64,
    column: u64,
    has_var: bool,
}

struct Rule {
    selectors: SelectorList<Impl>,
    text: String,
    decls: Vec<Decl>,
}

/// Where a sheet's text starts, so a position inside it can be given in the file it came from.
#[derive(Debug, Clone, Copy)]
struct Origin {
    line: u64,
    column: u64,
}

impl Origin {
    const START: Origin = Origin { line: 1, column: 1 };

    fn place(self, loc: SourceLocation) -> (u64, u64) {
        if loc.line == 0 {
            (self.line, self.column + u64::from(loc.column) - 1)
        } else {
            (self.line + u64::from(loc.line), u64::from(loc.column))
        }
    }
}

/// The HTML with its line breaks normalized as the parser normalizes them, so a block's text is
/// found in it verbatim.
struct Source {
    text: String,
    line_starts: Vec<usize>,
    cursor: usize,
}

impl Source {
    fn new(html: &str) -> Self {
        let text = html.replace("\r\n", "\n").replace('\r', "\n");
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(at, _)| at + 1))
            .collect();
        Source {
            text,
            line_starts,
            cursor: 0,
        }
    }

    /// The origin of the next occurrence of `needle` after the last one found, or the point the
    /// search reached when there is none.
    fn find(&mut self, needle: &str) -> Origin {
        if let Some(found) = self.text[self.cursor..].find(needle) {
            let at = self.cursor + found;
            self.cursor = at + needle.len();
            return self.origin(at);
        }
        self.origin(self.cursor)
    }

    fn origin(&self, offset: usize) -> Origin {
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let column = self.text[self.line_starts[line]..offset].chars().count();
        Origin {
            line: line as u64 + 1,
            column: column as u64 + 1,
        }
    }
}

/// The rules, pages and font faces of every sheet, in cascade order.
struct Collector<'a, 'b> {
    assets: &'a Assets<'b>,
    rules: Vec<Rule>,
    pages: Vec<(String, Vec<Decl>, Vec<(String, Vec<Decl>)>)>,
    font_faces: Vec<Vec<(String, String)>>,
    dropped: Vec<Dropped>,
}

type Error = ParseError<()>;

/// The prelude up to a `{` or a `;`, and whether a block follows it.
fn prelude(p: &mut Parser<'_>) -> (String, bool) {
    let start = p.position();
    let _ = p.parse_until_before(Delimiter::CurlyBracketBlock | Delimiter::Semicolon, |p| {
        while p.next().is_ok() {}
        Ok::<_, Error>(())
    });
    let text = p.slice_from(start).trim().to_owned();
    let block = matches!(p.next(), Ok(Token::CurlyBracketBlock));
    (text, block)
}

/// Whether a media query list applies to print.
fn media_applies(list: &str) -> bool {
    if list.trim().is_empty() {
        return true;
    }
    list.split(',').any(|query| {
        let words: Vec<String> = query
            .split_ascii_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        let mut words = words.as_slice();
        if words.first().is_some_and(|w| w == "only") {
            words = &words[1..];
        }
        let not = words.first().is_some_and(|w| w == "not");
        if not {
            words = &words[1..];
        }
        match words {
            [kind] if !kind.starts_with('(') => (kind == "print" || kind == "all") != not,
            _ => false,
        }
    })
}

impl Collector<'_, '_> {
    fn drop(&mut self, property: &str, value: &str, selector: &str, (line, column): (u64, u64)) {
        self.dropped.push(Dropped {
            property: property.to_owned(),
            value: value.to_owned(),
            selector: selector.to_owned(),
            line,
            column,
        });
    }

    fn sheet(&mut self, css: &str, origin: Origin, depth: usize) {
        let mut parser = Parser::new(css);
        self.rules(&mut parser, origin, depth);
    }

    fn rules(&mut self, p: &mut Parser<'_>, origin: Origin, depth: usize) {
        loop {
            p.skip_whitespace();
            let loc = p.current_source_location();
            let start = p.state();
            let token = match p.next() {
                Ok(token) => token.clone(),
                Err(_) => return,
            };
            match token {
                Token::CDO | Token::CDC => {}
                Token::AtKeyword(name) => self.at_rule(
                    p,
                    &name.to_ascii_lowercase(),
                    origin.place(loc),
                    origin,
                    depth,
                ),
                _ => {
                    p.reset(&start);
                    let (text, block) = prelude(p);
                    if !block {
                        self.drop("", "", &text, origin.place(loc));
                        continue;
                    }
                    let Some(selectors) = select::parse(&text) else {
                        self.drop("", "", &text, origin.place(loc));
                        continue;
                    };
                    let decls = p
                        .parse_nested_block(|p| {
                            Ok::<_, Error>(self.declarations(p, origin, &text, None))
                        })
                        .unwrap_or_default();
                    self.rules.push(Rule {
                        selectors,
                        text,
                        decls,
                    });
                }
            }
        }
    }

    fn at_rule(
        &mut self,
        p: &mut Parser<'_>,
        name: &str,
        at: (u64, u64),
        origin: Origin,
        depth: usize,
    ) {
        let (text, block) = prelude(p);
        match (name, block) {
            ("media", true) => {
                if media_applies(&text) {
                    let _ = p.parse_nested_block(|p| {
                        self.rules(p, origin, depth);
                        Ok::<_, Error>(())
                    });
                }
            }
            ("import", false) => self.import(&text, at, depth),
            ("page", true) => {
                let mut boxes = Vec::new();
                let selector = format!("@page {text}");
                let decls = p
                    .parse_nested_block(|p| {
                        Ok::<_, Error>(self.declarations(
                            p,
                            origin,
                            selector.trim(),
                            Some(&mut boxes),
                        ))
                    })
                    .unwrap_or_default();
                self.pages.push((text, decls, boxes));
            }
            ("font-face", true) => {
                let decls = p
                    .parse_nested_block(|p| Ok::<_, Error>(self.raw_declarations(p, origin)))
                    .unwrap_or_default();
                let mut face = Vec::new();
                for decl in decls {
                    if FONT_FACE_DESCRIPTORS.contains(&decl.property.as_str()) {
                        face.push((decl.property, decl.value));
                    } else {
                        self.drop(
                            &decl.property,
                            &decl.value,
                            "@font-face",
                            (decl.line, decl.column),
                        );
                    }
                }
                self.font_faces.push(face);
            }
            ("charset", false) => {}
            _ => self.drop(&format!("@{name}"), &text, "", at),
        }
    }

    fn import(&mut self, text: &str, at: (u64, u64), depth: usize) {
        let comps = value::components(text).unwrap_or_default();
        let (reference, media) = match comps.split_first() {
            Some((value::Comp::Url(r) | value::Comp::Str(r), rest)) => (r.clone(), rest),
            _ => return self.drop("@import", text, "", at),
        };
        let media_text = media
            .iter()
            .map(|c| match c {
                value::Comp::Ident(w) => w.as_str(),
                value::Comp::Comma => ",",
                _ => "(",
            })
            .collect::<Vec<_>>()
            .join(" ");
        if !media_applies(&media_text) {
            return;
        }
        let css = match self.assets.resolve(&reference) {
            Ok(bytes) if depth < MAX_IMPORT_DEPTH => String::from_utf8(bytes.into_owned()).ok(),
            _ => None,
        };
        match css {
            Some(css) => self.sheet(&css, Origin::START, depth + 1),
            None => self.drop("@import", text, "", at),
        }
    }

    /// The declarations of a block as written, each with `!important` split off.
    fn raw_declarations(&mut self, p: &mut Parser<'_>, origin: Origin) -> Vec<Decl> {
        self.walk_declarations(p, origin, "", &mut None)
    }

    /// The declarations of a rule's block. A declaration outside the subset is reported with
    /// `selector` and left out. `boxes` takes an `@page`'s margin boxes; anywhere else a nested
    /// at-rule is reported.
    fn declarations(
        &mut self,
        p: &mut Parser<'_>,
        origin: Origin,
        selector: &str,
        boxes: Option<&mut Vec<(String, Vec<Decl>)>>,
    ) -> Vec<Decl> {
        let mut boxes = boxes;
        let decls = self.walk_declarations(p, origin, selector, &mut boxes);
        decls
            .into_iter()
            .filter(|decl| self.admit(decl, selector))
            .collect()
    }

    fn admit(&mut self, decl: &Decl, selector: &str) -> bool {
        if decl.property.starts_with("--") {
            return true;
        }
        let ok = properties::longhands(&decl.property).is_some()
            && (decl.has_var || value::expand(&decl.property, &decl.value).is_some());
        if !ok {
            self.drop(
                &decl.property,
                &decl.value,
                selector,
                (decl.line, decl.column),
            );
        }
        ok
    }

    fn walk_declarations(
        &mut self,
        p: &mut Parser<'_>,
        origin: Origin,
        selector: &str,
        boxes: &mut Option<&mut Vec<(String, Vec<Decl>)>>,
    ) -> Vec<Decl> {
        let mut out = Vec::new();
        loop {
            p.skip_whitespace();
            let loc = p.current_source_location();
            let at = origin.place(loc);
            let token = match p.next() {
                Ok(token) => token.clone(),
                Err(_) => return out,
            };
            match token {
                Token::Semicolon => {}
                Token::Ident(name) => {
                    let property = if name.starts_with("--") {
                        name.to_string()
                    } else {
                        name.to_ascii_lowercase()
                    };
                    let colon = p.expect_colon().is_ok();
                    let start = p.position();
                    let _ = p.parse_until_after(Delimiter::Semicolon, |p| {
                        while p.next().is_ok() {}
                        Ok::<_, Error>(())
                    });
                    let raw = p.slice_from(start).trim_end_matches(';');
                    if !colon {
                        self.drop(&property, raw.trim(), selector, at);
                        continue;
                    }
                    let (value, important) = split_important(raw);
                    out.push(Decl {
                        has_var: value::has_var(&value),
                        property,
                        value,
                        important,
                        line: at.0,
                        column: at.1,
                    });
                }
                Token::AtKeyword(name) => {
                    let name = name.to_ascii_lowercase();
                    let (text, block) = prelude(p);
                    match boxes {
                        Some(boxes) if block && MARGIN_BOXES.contains(&name.as_str()) => {
                            let inner = format!("{selector} @{name}");
                            let decls = p
                                .parse_nested_block(|p| {
                                    Ok::<_, Error>(self.declarations(p, origin, &inner, None))
                                })
                                .unwrap_or_default();
                            boxes.push((name, decls));
                        }
                        _ => self.drop(&format!("@{name}"), &text, selector, at),
                    }
                }
                _ => {
                    let _ = p.parse_until_after(Delimiter::Semicolon, |p| {
                        while p.next().is_ok() {}
                        Ok::<_, Error>(())
                    });
                    self.drop("", "", selector, at);
                }
            }
        }
    }
}

/// The value without a trailing `!important`, and whether it had one.
fn split_important(raw: &str) -> (String, bool) {
    let mut p = Parser::new(raw);
    let mut bang = None;
    loop {
        let before = p.position();
        match p.next() {
            Ok(Token::Delim('!')) => bang = Some(before),
            Ok(Token::Ident(word)) if bang.is_some() && word.eq_ignore_ascii_case("important") => {
                if p.is_exhausted() {
                    let at = bang.map(|b| b.byte_index()).unwrap_or(raw.len());
                    return (raw[..at].trim().to_owned(), true);
                }
                bang = None;
            }
            Ok(_) => bang = None,
            Err(_) => return (raw.trim().to_owned(), false),
        }
    }
}

/// The computed styles of every element in `dom`, from its sheets in document order and its
/// `style` attributes. `html` is the document's text, for the positions in [`Styles::dropped`].
pub fn cascade(dom: &Dom, html: &str, assets: &Assets<'_>) -> Styles {
    let mut source = Source::new(html);
    let mut collector = Collector {
        assets,
        rules: Vec::new(),
        pages: Vec::new(),
        font_faces: Vec::new(),
        dropped: Vec::new(),
    };
    collector.sheet(USER_AGENT, Origin::START, 0);
    let user_agent = collector.rules.len();
    let order = rendered_order(dom);
    let mut attributes: BTreeMap<usize, Vec<Decl>> = BTreeMap::new();
    for &at in &order {
        let node = &dom.nodes[at];
        let Kind::Element { name, space, .. } = &node.kind else {
            continue;
        };
        if *space == Space::Html && UNRENDERED.contains(&name.as_str()) {
            let origin = source.find(&format!("<{name}"));
            collector.dropped.push(Dropped {
                property: String::new(),
                value: String::new(),
                selector: format!("<{name}>"),
                line: origin.line,
                column: origin.column,
            });
        }
        if *space != Space::Other
            && name == "style"
            && media_applies(node.attr("media").unwrap_or(""))
        {
            let text: String = node
                .children
                .iter()
                .filter_map(|&c| match &dom.nodes[c].kind {
                    Kind::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect();
            let origin = source.find(&text);
            collector.sheet(&text, origin, 0);
        } else if *space == Space::Html
            && name == "link"
            && node.attr("rel").is_some_and(|rel| {
                rel.split_ascii_whitespace()
                    .any(|w| w.eq_ignore_ascii_case("stylesheet"))
            })
            && media_applies(node.attr("media").unwrap_or(""))
            && let Some(href) = node.attr("href")
            && let Ok(bytes) = assets.resolve(href)
            && let Ok(css) = std::str::from_utf8(&bytes)
        {
            collector.sheet(css, Origin::START, 0);
        }
        if let Some(style) = node.attr("style") {
            let origin = source.find(style);
            let selector = format!("<{name} style>");
            let mut parser = Parser::new(style);
            let decls = collector.declarations(&mut parser, origin, &selector, None);
            attributes.insert(at, decls);
        }
    }
    let mut cascade = Cascade {
        tree: Tree::new(dom),
        rules: &collector.rules,
        user_agent,
        caches: SelectorCaches::default(),
        root_font_size: DEFAULT_FONT_SIZE,
        reported: BTreeSet::new(),
        dropped: Vec::new(),
    };
    let mut styles = Styles {
        elements: vec![None; dom.nodes.len()],
        ..Styles::default()
    };
    let empty = Style::default();
    for &at in &order {
        if !matches!(dom.nodes[at].kind, Kind::Element { .. }) {
            continue;
        }
        let parent = cascade
            .tree_parent(at)
            .and_then(|p| styles.of(p))
            .unwrap_or(&empty);
        let attribute = attributes.get(&at).map(Vec::as_slice).unwrap_or(&[]);
        let style = cascade.compute(at, None, parent, attribute);
        let is_root = cascade.tree_parent(at).is_none();
        if is_root {
            cascade.root_font_size = style.font_size();
        }
        for (pseudo, map) in [
            (PseudoElement::Before, &mut styles.before),
            (PseudoElement::After, &mut styles.after),
        ] {
            if cascade.has_pseudo(pseudo) {
                let generated = cascade.compute(at, Some(pseudo), &style, &[]);
                if generated.values != style.values || !Rc::ptr_eq(&generated.custom, &style.custom)
                {
                    map.insert(at, generated);
                }
            }
        }
        styles.elements[at] = Some(style);
    }
    let root = order
        .iter()
        .find(|&&at| matches!(dom.nodes[at].kind, Kind::Element { .. }))
        .and_then(|&at| styles.of(at))
        .cloned()
        .unwrap_or_default();
    for (selector, decls, boxes) in std::mem::take(&mut collector.pages) {
        let label = format!("@page {selector}");
        let values = cascade.declared(&decls, &root, label.trim());
        let boxes = boxes
            .iter()
            .map(|(name, decls)| {
                (
                    name.clone(),
                    cascade.declared(decls, &root, &format!("{} @{name}", label.trim())),
                )
            })
            .collect();
        styles.pages.push(PageRule {
            selector,
            values,
            boxes,
        });
    }
    styles.font_faces = std::mem::take(&mut collector.font_faces);
    styles.dropped = collector.dropped;
    styles.dropped.extend(cascade.dropped);
    styles
}

/// The nodes in document order, without what is inside a `<template>`, which is never rendered.
fn rendered_order(dom: &Dom) -> Vec<usize> {
    let mut order = Vec::new();
    let mut stack = vec![0];
    while let Some(at) = stack.pop() {
        order.push(at);
        if dom.nodes[at].element(Space::Html) == Some("template") {
            continue;
        }
        stack.extend(dom.nodes[at].children.iter().rev());
    }
    order
}

struct Cascade<'d, 'r> {
    tree: Tree<'d>,
    rules: &'r [Rule],
    /// How many of `rules`, from the first, are [`USER_AGENT`]'s.
    user_agent: usize,
    caches: SelectorCaches,
    root_font_size: f32,
    reported: BTreeSet<(u64, u64, String)>,
    dropped: Vec<Dropped>,
}

/// Where a declaration sorts: `!important`, then the document over the user-agent sheet, then a
/// `style` attribute, then specificity, then source order.
type Key = (bool, bool, bool, u32, usize, usize);

impl<'r> Cascade<'_, 'r> {
    fn tree_parent(&self, at: usize) -> Option<usize> {
        let parent = self.tree.dom.nodes[at].parent?;
        matches!(self.tree.dom.nodes[parent].kind, Kind::Element { .. }).then_some(parent)
    }

    fn has_pseudo(&self, pseudo: PseudoElement) -> bool {
        self.rules.iter().any(|rule| {
            rule.selectors
                .slice()
                .iter()
                .any(|s| s.pseudo_element() == Some(&pseudo))
        })
    }

    /// The declarations that apply to the element at `at`, or its `pseudo`, in cascade order.
    fn matched<'a>(
        &mut self,
        at: usize,
        pseudo: Option<PseudoElement>,
        attribute: &'a [Decl],
    ) -> Vec<(Key, &'a Decl)>
    where
        'r: 'a,
    {
        let mut out: Vec<(Key, &'a Decl)> = Vec::new();
        let rules: &'r [Rule] = self.rules;
        for (index, rule) in rules.iter().enumerate() {
            let specificity = rule
                .selectors
                .slice()
                .iter()
                .filter(|s| self.tree.matches(s, at, pseudo, &mut self.caches))
                .map(|s| s.specificity())
                .max();
            if let Some(specificity) = specificity {
                let author = index >= self.user_agent;
                for (n, decl) in rule.decls.iter().enumerate() {
                    out.push(((decl.important, author, false, specificity, index, n), decl));
                }
            }
        }
        for (n, decl) in attribute.iter().enumerate() {
            out.push(((decl.important, true, true, 0, usize::MAX, n), decl));
        }
        out.sort_by_key(|(key, _)| *key);
        out
    }

    fn report(&mut self, decl: &Decl, selector: &str) {
        if self
            .reported
            .insert((decl.line, decl.column, decl.property.clone()))
        {
            self.dropped.push(Dropped {
                property: decl.property.clone(),
                value: decl.value.clone(),
                selector: selector.to_owned(),
                line: decl.line,
                column: decl.column,
            });
        }
    }

    fn compute(
        &mut self,
        at: usize,
        pseudo: Option<PseudoElement>,
        parent: &Style,
        attribute: &[Decl],
    ) -> Style {
        let matched = self.matched(at, pseudo, attribute);
        let selector_of = |key: &Key| -> String {
            if key.2 {
                let name = self.tree.dom.nodes[at]
                    .element(Space::Html)
                    .unwrap_or("element");
                format!("<{name} style>")
            } else {
                self.rules[key.4].text.clone()
            }
        };

        // Custom properties first: every `var()` below reads them.
        let mut custom = Rc::clone(&parent.custom);
        let declared: Vec<&Decl> = matched
            .iter()
            .filter(|(_, d)| d.property.starts_with("--"))
            .map(|(_, d)| *d)
            .collect();
        if !declared.is_empty() {
            let mut map = (*custom).clone();
            for decl in &declared {
                map.insert(decl.property.clone(), decl.value.clone());
            }
            let raw = map.clone();
            for decl in &declared {
                let lookup = |name: &str| raw.get(name).cloned();
                match value::substitute(&decl.value, &lookup) {
                    Some(resolved) => map.insert(decl.property.clone(), resolved),
                    None => map.remove(&decl.property),
                };
            }
            custom = Rc::new(map);
        }

        // The winning declaration of each longhand.
        let mut winners: BTreeMap<&'static str, usize> = BTreeMap::new();
        for (index, (_, decl)) in matched.iter().enumerate() {
            for longhand in properties::longhands(&decl.property).unwrap_or_default() {
                winners.insert(longhand, index);
            }
        }
        let mut expanded: BTreeMap<usize, Option<Vec<(&'static str, Specified)>>> = BTreeMap::new();
        let mut specified: BTreeMap<&'static str, Specified> = BTreeMap::new();
        for (&longhand, &index) in &winners {
            let (key, decl) = &matched[index];
            let values = expanded.entry(index).or_insert_with(|| {
                let text = if decl.has_var {
                    let lookup = |name: &str| custom.get(name).cloned();
                    value::substitute(&decl.value, &lookup)
                } else {
                    Some(decl.value.clone())
                };
                text.and_then(|text| value::expand(&decl.property, &text))
            });
            match values {
                Some(values) => {
                    if let Some((_, v)) = values.iter().find(|(name, _)| *name == longhand) {
                        specified.insert(longhand, v.clone());
                    }
                }
                None => {
                    let selector = selector_of(key);
                    let decl: &Decl = decl;
                    self.report(decl, &selector);
                }
            }
        }

        let mut style = Style {
            values: BTreeMap::new(),
            custom,
        };
        for p in properties::PROPERTIES {
            if p.inherited
                && let Some(v) = parent.values.get(p.name)
            {
                style.values.insert(p.name, v.clone());
            }
        }
        let parent_size = parent.font_size();
        if let Some(size) = specified.remove("font-size") {
            self.apply(&mut style, parent, "font-size", size, parent_size);
        }
        let font_size = style.font_size();
        if let Some(color) = specified.remove("color") {
            self.apply(&mut style, parent, "color", color, font_size);
        }
        for (name, v) in specified {
            self.apply(&mut style, parent, name, v, font_size);
        }
        style
    }

    /// Sets the longhand `name` in `style` from its specified value. `size` is the font size
    /// its `em`s count in: the parent's for `font-size`, the element's own for the rest.
    fn apply(
        &self,
        style: &mut Style,
        parent: &Style,
        name: &'static str,
        v: Specified,
        size: f32,
    ) {
        let inherited = properties::property(name).is_some_and(|p| p.inherited);
        let computed = match v {
            Specified::Wide(word) => match word.as_str() {
                "inherit" => parent.values.get(name).cloned(),
                "unset" | "revert" if inherited => parent.values.get(name).cloned(),
                _ => None,
            },
            Specified::Keyword(word) if name == "font-size" => Some(Value::Length(Length {
                pt: match word.as_str() {
                    "xx-small" => 7.0,
                    "x-small" => 7.5,
                    "small" => 10.0,
                    "medium" => 12.0,
                    "large" => 13.5,
                    "x-large" => 18.0,
                    "xx-large" => 24.0,
                    "xxx-large" => 36.0,
                    "smaller" => size / 1.2,
                    _ => size * 1.2,
                },
                percent: 0.0,
            })),
            Specified::Keyword(word) if name == "font-weight" => {
                let parent = match parent.values.get("font-weight") {
                    Some(Value::Number(n)) => *n,
                    _ => 400.0,
                };
                Some(Value::Number(match word.as_str() {
                    "bold" => 700.0,
                    "bolder" if parent < 350.0 => 400.0,
                    "bolder" if parent < 550.0 => 700.0,
                    "bolder" => 900.0,
                    "lighter" if parent < 550.0 => 100.0,
                    "lighter" if parent < 750.0 => 400.0,
                    "lighter" => 700.0,
                    _ => 400.0,
                }))
            }
            Specified::Length(sum) if name == "font-size" => Some(Value::Length(Length {
                pt: sum.pt + (sum.em + sum.percent / 100.0) * size + sum.rem * self.root_font_size,
                percent: 0.0,
            })),
            Specified::Length(sum) => Some(Value::Length(sum.compute(size, self.root_font_size))),
            Specified::CurrentColor if name == "color" => parent.values.get("color").cloned(),
            Specified::CurrentColor => Some(style.values.get("color").cloned().unwrap_or(
                Value::Color(Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 1.0,
                }),
            )),
            Specified::Keyword(word) => Some(Value::Keyword(word)),
            Specified::Keywords(words) => Some(Value::Keywords(words)),
            Specified::Number(n) => Some(Value::Number(n)),
            Specified::Integer(n) => Some(Value::Integer(n)),
            Specified::Color(c) => Some(Value::Color(c)),
            Specified::Families(f) => Some(Value::Families(f)),
            Specified::Function(f, args) => Some(Value::Function(f, args)),
            Specified::Text(t) => Some(Value::Text(t)),
        };
        match computed {
            Some(v) => style.values.insert(name, v),
            None => style.values.remove(name),
        };
    }

    /// An `@page` rule's or a margin box's declarations, computed against the root's custom
    /// properties and the default font size, in source order with the last one of a longhand
    /// kept.
    fn declared(
        &mut self,
        decls: &[Decl],
        root: &Style,
        selector: &str,
    ) -> Vec<(&'static str, Value)> {
        let mut style = Style::default();
        let page = Style::default();
        let mut sorted: Vec<&Decl> = decls
            .iter()
            .filter(|d| !d.property.starts_with("--"))
            .collect();
        sorted.sort_by_key(|d| d.important);
        let mut order: Vec<&'static str> = Vec::new();
        for decl in sorted {
            let text = if decl.has_var {
                let lookup = |name: &str| root.custom.get(name).cloned();
                value::substitute(&decl.value, &lookup)
            } else {
                Some(decl.value.clone())
            };
            match text.and_then(|text| value::expand(&decl.property, &text)) {
                Some(values) => {
                    for (name, v) in values {
                        self.apply(&mut style, &page, name, v, DEFAULT_FONT_SIZE);
                        order.retain(|n| *n != name);
                        order.push(name);
                    }
                }
                None => self.report(decl, selector),
            }
        }
        order
            .into_iter()
            .filter_map(|name| style.values.get(name).map(|v| (name, v.clone())))
            .collect()
    }
}
