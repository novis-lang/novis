//! The pages: each laid-out page's size and margins from the document's `@page` rules
//! (`rule:core-classes/pdf-css-subset-table`).
//!
//! A page is A4 portrait with a 2 cm margin on every side unless an `@page` rule says otherwise.
//! The rules that match a page apply in the order of their selector's specificity and then their
//! source order, the last value of a property kept: a page name counts most, then `:first` and
//! `:blank`, then `:left` and `:right`. The first page is `:first` and a right page, and the pages
//! alternate right and left from there. A page is `:blank` when a forced break to a `left` or
//! `right` page left it empty (`layout`'s module doc). A selector with a page name matches a page
//! of that name only. A page's name is the `page` of the first block box placed on it, or of the
//! box it goes on with, and a block box whose `page` is `auto` has its parent's. A block box
//! whose page name differs from the block box before it starts a new page.
//!
//! `size` is `auto`, a named size (`A3`, `A4`, `A5`, `B4`, `B5`, `letter`, `legal`, `ledger`) with
//! `portrait` or `landscape` or neither, an orientation alone, or one or two positive lengths, the
//! width first. A `size` that does not read is A4 portrait. A margin's percentage counts against the
//! page's width for `margin-left` and `margin-right` and against its height for the other two, and
//! `auto` is 0.
//!
//! A margin box whose `content` is `element(name)` draws the running element of that name
//! (`position: running(name)`) that is first on the page, or `element(name, last)` the last one.
//! A page with none draws the last one of an earlier page, and the box is not made before the
//! first. Any other second argument reads as `first`.
//!
//! A margin box's `string(name)` is the value an element's `string-set` gave that name: the first
//! set on the page, or with a second argument `last` the last, `start` the one in force when the
//! page starts, or the one set at its top, and `first-except` nothing on a page that sets one.
//! A page that sets none takes the one in force when it starts, and before the first it is
//! empty. A string is set on the page its block box starts on, or for an inline element on the
//! page its paragraph starts on. Names are compared without case.
//!
//! The content is laid out at the first page's content width, and a block box that changes the
//! page name at the content width the first page would have under the new name. Every page then
//! has its own content height, and its content is moved to its own content box's top-left corner,
//! so a `:left` or `:right` page whose content box is wider or narrower than the first page's
//! keeps the line widths of its name.
//!
//! Cost: each page's rules are matched when the page is made, a few comparisons a rule. Memory: a
//! copy of the `@page` rules for the render.

use std::collections::BTreeMap;

use crate::css::value::{Comp, components, components_as_written, length};
use crate::css::{DEFAULT_FONT_SIZE, PageRule, Value};
use crate::layout::{MARGIN, PAGE_HEIGHT, PAGE_WIDTH};

/// A page's size and its margins, `[top, right, bottom, left]`, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageBox {
    pub width: f32,
    pub height: f32,
    pub margin: [f32; 4],
}

impl Default for PageBox {
    fn default() -> PageBox {
        PageBox {
            width: PAGE_WIDTH,
            height: PAGE_HEIGHT,
            margin: [MARGIN; 4],
        }
    }
}

impl PageBox {
    pub fn content_width(&self) -> f32 {
        (self.width - self.margin[1] - self.margin[3]).max(0.0)
    }

    pub fn content_height(&self) -> f32 {
        (self.height - self.margin[0] - self.margin[2]).max(0.0)
    }
}

/// What an `@page` selector names: a page name, and the pseudo-classes it holds.
#[derive(Debug, Clone, Default)]
struct Selector {
    name: Option<String>,
    first: bool,
    blank: bool,
    left: bool,
    right: bool,
}

impl Selector {
    /// The selector `text`, or `None` for one with a pseudo-class outside the four.
    fn read(text: &str) -> Option<Selector> {
        let mut parts = text.trim().split(':');
        let name = parts.next().unwrap_or("").trim();
        let mut selector = Selector {
            name: (!name.is_empty()).then(|| name.to_owned()),
            ..Selector::default()
        };
        for pseudo in parts {
            match pseudo.trim().to_ascii_lowercase().as_str() {
                "first" => selector.first = true,
                "blank" => selector.blank = true,
                "left" => selector.left = true,
                "right" => selector.right = true,
                _ => return None,
            }
        }
        Some(selector)
    }

    fn specificity(&self) -> (bool, u8, u8) {
        (
            self.name.is_some(),
            u8::from(self.first) + u8::from(self.blank),
            u8::from(self.left) + u8::from(self.right),
        )
    }

    /// Whether the selector matches the page at `index`, from 0, named `name`.
    fn matches(&self, index: usize, blank: bool, name: Option<&str>) -> bool {
        let right = index.is_multiple_of(2);
        self.name.as_deref().is_none_or(|own| Some(own) == name)
            && (!self.first || index == 0)
            && (!self.blank || blank)
            && (!self.left || !right)
            && (!self.right || right)
    }
}

/// The document's `@page` rules, in the order they apply.
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    rules: Vec<(Selector, PageRule)>,
}

/// The values the rules matching one page give: the page's own, and each margin box's by its
/// name.
#[derive(Debug, Default)]
pub struct Matched<'s> {
    pub values: BTreeMap<&'static str, &'s Value>,
    pub boxes: BTreeMap<&'s str, BTreeMap<&'static str, &'s Value>>,
}

impl Sheet {
    pub fn new(rules: &[PageRule]) -> Sheet {
        let mut rules: Vec<(Selector, PageRule)> = rules
            .iter()
            .filter_map(|rule| Some((Selector::read(&rule.selector)?, rule.clone())))
            .collect();
        rules.sort_by_key(|(selector, _)| selector.specificity());
        Sheet { rules }
    }

    /// What the rules matching the page at `index` named `name` give, `blank` when it is a
    /// blank page.
    pub fn matched(&self, index: usize, blank: bool, name: Option<&str>) -> Matched<'_> {
        let mut matched = Matched::default();
        for (_, rule) in self
            .rules
            .iter()
            .filter(|(selector, _)| selector.matches(index, blank, name))
        {
            for (name, value) in &rule.values {
                matched.values.insert(name, value);
            }
            for (name, values) in &rule.boxes {
                let one = matched.boxes.entry(name.as_str()).or_default();
                for (property, value) in values {
                    one.insert(property, value);
                }
            }
        }
        matched
    }

    /// The size and margins of the page at `index` named `name`, `blank` when it is a blank
    /// page.
    pub fn page(&self, index: usize, blank: bool, name: Option<&str>) -> PageBox {
        let matched = self.matched(index, blank, name);
        let (width, height) = size(matched.values.get("size").copied());
        let margin = ["margin-top", "margin-right", "margin-bottom", "margin-left"]
            .iter()
            .enumerate()
            .map(|(side, name)| {
                let basis = if side % 2 == 0 { height } else { width };
                match matched.values.get(name) {
                    Some(Value::Length(l)) => l.pt + l.percent / 100.0 * basis,
                    Some(_) => 0.0,
                    None => MARGIN,
                }
            });
        let mut sides = [0.0; 4];
        for (side, value) in sides.iter_mut().zip(margin) {
            *side = value;
        }
        PageBox {
            width,
            height,
            margin: sides,
        }
    }
}

/// The `@page` rule a header or footer function's height gives: the margin `side`, such as
/// `margin-top`, set to `text` on every page. It comes after the document's own rules, so it
/// wins over each rule without a page name or a pseudo-class and loses to the others. `None`
/// when `text` is not one length of 0 or more.
pub fn margin_rule(side: &'static str, text: &str) -> Option<PageRule> {
    let [one] = components(text)?.try_into().ok()?;
    let sum = length(&one)?;
    let computed = sum.compute(DEFAULT_FONT_SIZE, DEFAULT_FONT_SIZE);
    (computed.pt >= 0.0 && computed.percent >= 0.0).then(|| PageRule {
        selector: String::new(),
        values: vec![(side, Value::Length(computed))],
        boxes: Vec::new(),
    })
}

/// Where the margin box `name` is on `page`: its left edge, top, width and height. `present`
/// says whether another box of the page has content, because a box along a side takes the room
/// of the boxes beside it that have none.
pub fn rect(name: &str, page: &PageBox, present: &dyn Fn(&str) -> bool) -> Option<[f32; 4]> {
    let PageBox { width, height, .. } = *page;
    let [top, right, bottom, left] = page.margin;
    let (inner_w, inner_h) = (page.content_width(), page.content_height());
    let (side, at) = name.split_once('-')?;
    // The box's start and length along its side, from the three boxes on that side.
    let along = |names: [&str; 3], start: f32, length: f32| {
        let has = names.map(|n| present(&format!("{side}-{n}")));
        let third = length / 3.0;
        let index = names.iter().position(|n| *n == at)?;
        Some(match (index, has) {
            (1, [false, _, false]) => (start, length),
            (1, _) => (start + third, third),
            (0, [_, false, false]) => (start, length),
            (2, [false, false, _]) => (start, length),
            (0, [_, false, _]) | (2, [_, false, _]) => (
                start + if index == 0 { 0.0 } else { length / 2.0 },
                length / 2.0,
            ),
            _ => (start + third * index as f32, third),
        })
    };
    Some(match (side, at) {
        ("top", "left-corner") => [0.0, 0.0, left, top],
        ("top", "right-corner") => [width - right, 0.0, right, top],
        ("bottom", "left-corner") => [0.0, height - bottom, left, bottom],
        ("bottom", "right-corner") => [width - right, height - bottom, right, bottom],
        ("top" | "bottom", _) => {
            let (x, w) = along(["left", "center", "right"], left, inner_w)?;
            let y = if side == "top" { 0.0 } else { height - bottom };
            [x, y, w, if side == "top" { top } else { bottom }]
        }
        ("left" | "right", _) => {
            let (y, h) = along(["top", "middle", "bottom"], top, inner_h)?;
            let x = if side == "left" { 0.0 } else { width - right };
            [x, y, if side == "left" { left } else { right }, h]
        }
        _ => return None,
    })
}

/// The `text-align` and `vertical-align` the margin box `name` has when it gives none.
pub fn alignment(name: &str) -> (&'static str, &'static str) {
    let horizontal = match name {
        "top-left" | "bottom-left" => "left",
        "top-right" | "bottom-right" => "right",
        _ => "center",
    };
    let vertical = match name {
        "left-top" | "right-top" => "top",
        "left-bottom" | "right-bottom" => "bottom",
        _ => "middle",
    };
    (horizontal, vertical)
}

/// Which value of a named string a margin box takes on a page, from `string(name, which)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    /// The first set on the page, else the one in force when the page starts.
    First,
    /// The one in force when the page starts, unless one is set at the page's top.
    Start,
    /// The last set on the page, else the one in force when the page starts.
    Last,
    /// Nothing when one is set on the page, else the one in force when the page starts.
    FirstExcept,
}

/// The named strings an element's `string-set` sets, each name as written with its value. A
/// name is a CSS custom identifier, so it keeps its case.
/// `content()` and `content(text)` are `text`, the element's text with its spaces collapsed,
/// and `attr(name)` is the attribute `attr` gives, or nothing. Any other part adds nothing.
pub fn string_set(
    value: Option<&Value>,
    text: &str,
    attr: &dyn Fn(&str) -> Option<String>,
) -> Vec<(String, String)> {
    let Some(Value::Text(value)) = value else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = Vec::new();
    let mut open = false;
    for comp in components_as_written(value).unwrap_or_default() {
        match comp {
            Comp::Comma => open = false,
            Comp::Ident(word) if !open => {
                if word.eq_ignore_ascii_case("none") {
                    return Vec::new();
                }
                out.push((word, String::new()));
                open = true;
            }
            _ if !open => {}
            Comp::Str(s) => out.last_mut().expect("open").1.push_str(&s),
            Comp::Function(f, args) if f == "content" => {
                if matches!(args.trim(), "" | "text") {
                    let words: Vec<&str> = text.split_whitespace().collect();
                    out.last_mut().expect("open").1.push_str(&words.join(" "));
                }
            }
            Comp::Function(f, args) if f == "attr" => {
                let found = attr(args.trim()).unwrap_or_default();
                out.last_mut().expect("open").1.push_str(&found);
            }
            _ => {}
        }
    }
    out
}

/// The text of a margin box's `content` on page `page` of `pages`, both from 1, or `None` when
/// it makes no box. `counter(page)` and `counter(pages)` are the two numbers, in the counter
/// style `style` writes; any other counter is 0. `string(name)` is the value `strings` gives
/// for the name as written, or nothing.
pub fn content(
    value: Option<&Value>,
    page: i64,
    pages: i64,
    style: &dyn Fn(&str, i64) -> Option<String>,
    strings: &dyn Fn(&str, Which) -> Option<String>,
) -> Option<String> {
    let Some(Value::Text(text)) = value else {
        return None;
    };
    let mut out = String::new();
    for comp in components(text)? {
        match comp {
            Comp::Str(s) => out.push_str(&s),
            Comp::Ident(word) if word == "none" || word == "normal" => return None,
            Comp::Function(name, args) if name == "counter" => {
                let mut args = args.split(',').map(str::trim);
                let n = match args.next() {
                    Some("page") => page,
                    Some("pages") => pages,
                    _ => 0,
                };
                let kind = args.next().unwrap_or("decimal");
                out.push_str(&style(kind, n).unwrap_or_else(|| n.to_string()));
            }
            Comp::Function(name, args) if name == "string" => {
                let mut args = args.split(',').map(str::trim);
                let name = args.next().unwrap_or("");
                let which = match args.next().map(str::to_ascii_lowercase).as_deref() {
                    Some("start") => Which::Start,
                    Some("last") => Which::Last,
                    Some("first-except") => Which::FirstExcept,
                    _ => Which::First,
                };
                out.push_str(&strings(name, which).unwrap_or_default());
            }
            _ => {}
        }
    }
    Some(out)
}

/// The running element a margin box's `content` places when it is `element(name)` alone: the
/// name, and whether it is the last element of that name on the page (`element(name, last)`)
/// or else the first.
pub fn element(value: Option<&Value>) -> Option<(String, bool)> {
    let Some(Value::Text(text)) = value else {
        return None;
    };
    let [Comp::Function(function, args)] = &components(text)?[..] else {
        return None;
    };
    if function != "element" {
        return None;
    }
    let mut args = args.split(',').map(str::trim);
    let name = args.next().filter(|name| !name.is_empty())?;
    Some((name.to_owned(), args.next() == Some("last")))
}

/// A named page size's width and height in millimetres, portrait.
fn named(name: &str) -> Option<(f32, f32)> {
    Some(match name {
        "a3" => (297.0, 420.0),
        "a4" => (210.0, 297.0),
        "a5" => (148.0, 210.0),
        "b4" => (250.0, 353.0),
        "b5" => (176.0, 250.0),
        "letter" => (215.9, 279.4),
        "legal" => (215.9, 355.6),
        "ledger" => (279.4, 431.8),
        _ => return None,
    })
}

/// The page's width and height in points from its `size`.
fn size(value: Option<&Value>) -> (f32, f32) {
    let a4 = (PAGE_WIDTH, PAGE_HEIGHT);
    let Some(Value::Text(text)) = value else {
        return a4;
    };
    let mut paper = None;
    let mut landscape = None;
    let mut lengths = Vec::new();
    for comp in components(text).unwrap_or_default() {
        match &comp {
            Comp::Ident(word) => match word.as_str() {
                "auto" => {}
                "portrait" => landscape = Some(false),
                "landscape" => landscape = Some(true),
                name => match named(name) {
                    Some((w, h)) => paper = Some((w * 72.0 / 25.4, h * 72.0 / 25.4)),
                    None => return a4,
                },
            },
            other => match length(other) {
                Some(sum) if sum.percent == 0.0 => {
                    let pt = sum.compute(DEFAULT_FONT_SIZE, DEFAULT_FONT_SIZE).pt;
                    if pt <= 0.0 {
                        return a4;
                    }
                    lengths.push(pt);
                }
                _ => return a4,
            },
        }
    }
    let (width, height) = match lengths[..] {
        [] => paper.unwrap_or(a4),
        [side] if paper.is_none() && landscape.is_none() => (side, side),
        [width, height] if paper.is_none() && landscape.is_none() => (width, height),
        _ => return a4,
    };
    match landscape {
        Some(true) => (width.max(height), width.min(height)),
        Some(false) => (width.min(height), width.max(height)),
        None => (width, height),
    }
}
