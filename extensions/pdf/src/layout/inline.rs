//! The walk over the document that collects its blocks: each element's text style and frame,
//! its counters and markers, its generated content, and the text of each block with its spans.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Range;

use cssparser::{Parser, Token};
use hypher::Lang;
use parley::{
    Alignment, AlignmentBaseline, BaseDirection, BaselineShift, LineHeight, OverflowWrap,
    TextWrapMode, VerticalAlign, WordBreak,
};

use super::block::{BoxGeometry, Break, Insets, geometry, insets, sides, z_index};
use super::flex::{self, Flex};
use super::table::{self, Cell, Repeat, Row, Table};
use super::{
    Anchor, Atom, BLACK, Block, Frame, Href, Images, Inline, MARGIN, Para, Role, Spaces, Span,
    color, keyword, length,
};
use crate::Error;
use crate::assets::Assets;
use crate::css::{Style, Styles, Value};
use crate::fonts::Fonts;
use crate::paginate;
use crate::parse::{Dom, Kind, Node, Space};

pub(super) fn span_of(style: &Style, fonts: &Fonts, parent: Option<&Span>) -> Span {
    let size = style.font_size();
    let families = match style.get("font-family") {
        Some(Value::Families(f)) => f.clone(),
        _ => Vec::new(),
    };
    let decoration = match style.get("text-decoration-line") {
        Some(Value::Keywords(words)) => words.clone(),
        _ => Vec::new(),
    };
    let color_of_text = color(style, "color").unwrap_or(BLACK);
    let line = color(style, "text-decoration-color").unwrap_or(color_of_text);
    let has = |w: &str| decoration.iter().any(|d| d == w).then_some(line);
    let spacing = |name: &str| length(style, name, size).unwrap_or(0.0);
    Span {
        chain: fonts.chain(&families),
        size,
        weight: match style.get("font-weight") {
            Some(Value::Number(n)) => *n,
            _ => 400.0,
        },
        italic: matches!(keyword(style, "font-style"), Some("italic" | "oblique")),
        color: color_of_text,
        letter_spacing: spacing("letter-spacing"),
        word_spacing: spacing("word-spacing"),
        features: match style.get("font-feature-settings") {
            Some(Value::Text(t)) if t != "normal" => Some(t.clone()),
            _ => None,
        },
        underline: has("underline").or(parent.and_then(|p| p.underline)),
        strike: has("line-through").or(parent.and_then(|p| p.strike)),
        align: vertical_align(style, parent),
        link: parent.and_then(|p| p.link.clone()),
    }
}

/// `style`'s `vertical-align` inside the span `parent`. `sub`, `super` and a length or
/// percentage add to the parent's shift: `sub` lowers by a fifth of the parent's font size,
/// `super` raises by a third, and a percentage is of the element's own line height. `top` and
/// `bottom` align with the line box, and `baseline` keeps the parent's alignment.
fn vertical_align(style: &Style, parent: Option<&Span>) -> VerticalAlign {
    let inherited = parent.map_or(VerticalAlign::BASELINE, |p| p.align);
    let rise = match inherited.shift {
        BaselineShift::Length(rise) => rise,
        _ => 0.0,
    };
    let parent_size = parent.map_or(style.font_size(), |p| p.size);
    let raised = |by: f32| VerticalAlign {
        alignment: AlignmentBaseline::Baseline,
        shift: BaselineShift::Length(rise + by),
    };
    let aligned = |alignment| VerticalAlign {
        alignment,
        shift: BaselineShift::Length(rise),
    };
    match style.get("vertical-align") {
        Some(Value::Keyword(word)) => match word.as_str() {
            "sub" => raised(-parent_size / 5.0),
            "super" => raised(parent_size / 3.0),
            "top" => VerticalAlign::TOP,
            "bottom" => VerticalAlign::BOTTOM,
            "middle" => aligned(AlignmentBaseline::Middle),
            "text-top" => aligned(AlignmentBaseline::TextTop),
            "text-bottom" => aligned(AlignmentBaseline::TextBottom),
            _ => inherited,
        },
        Some(Value::Length(l)) => {
            let size = style.font_size();
            let line_height = match style.get("line-height") {
                Some(Value::Number(n)) => n * size,
                Some(Value::Length(h)) => h.pt + h.percent / 100.0 * size,
                _ => 1.2 * size,
            };
            raised(l.pt + l.percent / 100.0 * line_height)
        }
        _ => inherited,
    }
}

/// `style`'s paragraph, in a content box `width` wide, whose own text style is the span at
/// `span`.
pub(super) fn para_of(style: &Style, width: f32, span: usize) -> Para {
    let size = style.font_size();
    let white = keyword(style, "white-space");
    Para {
        direction: match (keyword(style, "unicode-bidi"), keyword(style, "direction")) {
            (Some("plaintext"), _) => BaseDirection::Auto,
            (_, Some("rtl")) => BaseDirection::Rtl,
            _ => BaseDirection::Ltr,
        },
        align: match keyword(style, "text-align") {
            Some("left") => Alignment::Left,
            Some("right") => Alignment::Right,
            Some("center") => Alignment::Center,
            Some("justify") => Alignment::Justify,
            Some("end") => Alignment::End,
            _ => Alignment::Start,
        },
        line_height: match style.get("line-height") {
            Some(Value::Number(n)) => LineHeight::FontSizeRelative(*n),
            Some(Value::Length(l)) => LineHeight::Absolute(l.pt + l.percent / 100.0 * size),
            _ => LineHeight::MetricsRelative(1.0),
        },
        wrap: if matches!(white, Some("nowrap" | "pre")) {
            TextWrapMode::NoWrap
        } else {
            TextWrapMode::Wrap
        },
        word_break: match keyword(style, "word-break") {
            Some("break-all") => WordBreak::BreakAll,
            Some("keep-all") => WordBreak::KeepAll,
            _ => WordBreak::Normal,
        },
        overflow_wrap: match keyword(style, "overflow-wrap") {
            Some("anywhere") => OverflowWrap::Anywhere,
            Some("break-word") => OverflowWrap::BreakWord,
            _ if keyword(style, "word-break") == Some("break-word") => OverflowWrap::BreakWord,
            _ => OverflowWrap::Normal,
        },
        indent: length(style, "text-indent", width).unwrap_or(0.0),
        ellipsis: keyword(style, "text-overflow") == Some("ellipsis")
            && !matches!(keyword(style, "overflow-x"), None | Some("visible")),
        orphans: lines(style, "orphans"),
        widows: lines(style, "widows"),
        span,
    }
}

/// A count of lines from `orphans` or `widows`, which is 2 when it is not set.
fn lines(style: &Style, name: &str) -> usize {
    match style.get(name) {
        Some(Value::Integer(n)) => usize::try_from(*n).unwrap_or(1).max(1),
        _ => 2,
    }
}

/// The bidi control characters that open and close a box's `unicode-bidi` in its paragraph's
/// text, by its `direction`. A block is its own paragraph, so only an override changes it.
fn bidi_controls(style: &Style, block: bool) -> Option<(&'static str, &'static str)> {
    let rtl = keyword(style, "direction") == Some("rtl");
    let pick = |ltr: &'static str, rtl_: &'static str| if rtl { rtl_ } else { ltr };
    match keyword(style, "unicode-bidi") {
        Some("embed") if !block => Some((pick("\u{202a}", "\u{202b}"), "\u{202c}")),
        Some("isolate") if !block => Some((pick("\u{2066}", "\u{2067}"), "\u{2069}")),
        Some("plaintext") if !block => Some(("\u{2068}", "\u{2069}")),
        Some("isolate-override") if !block => Some((
            pick("\u{2066}\u{202d}", "\u{2067}\u{202e}"),
            "\u{202c}\u{2069}",
        )),
        Some("bidi-override" | "isolate-override") => {
            Some((pick("\u{202d}", "\u{202e}"), "\u{202c}"))
        }
        _ => None,
    }
}

/// Whether `c` is one of the characters [`bidi_controls`] writes.
pub(super) fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn spaces_of(style: &Style) -> Spaces {
    match keyword(style, "white-space") {
        Some("pre" | "pre-wrap" | "break-spaces") => Spaces::Keep,
        Some("pre-line") => Spaces::KeepBreaks,
        _ => Spaces::Collapse,
    }
}

/// A list item's marker: `n` as `list-style-type` writes it, a number followed by a full stop.
fn marker(kind: &str, n: i64) -> Option<String> {
    let text = counter_text(kind, n)?;
    Some(if matches!(kind, "disc" | "circle" | "square") {
        text
    } else {
        format!("{text}.")
    })
}

/// `n` as the counter style `kind` writes it, without a suffix. A style with no form for `n`
/// writes it in decimal.
pub(super) fn counter_text(kind: &str, n: i64) -> Option<String> {
    let roman = |n: i64| {
        let mut n = n;
        let mut out = String::new();
        for (value, digits) in [
            (1000, "m"),
            (900, "cm"),
            (500, "d"),
            (400, "cd"),
            (100, "c"),
            (90, "xc"),
            (50, "l"),
            (40, "xl"),
            (10, "x"),
            (9, "ix"),
            (5, "v"),
            (4, "iv"),
            (1, "i"),
        ] {
            while n >= value {
                out.push_str(digits);
                n -= value;
            }
        }
        out
    };
    let alpha = |n: i64, first: u8, count: i64| {
        let mut n = n;
        let mut out = Vec::new();
        while n > 0 {
            n -= 1;
            out.push(first + (n % count) as u8);
            n /= count;
        }
        out.reverse();
        String::from_utf8(out).unwrap_or_default()
    };
    Some(match kind {
        "none" => return None,
        "disc" => "\u{2022}".to_owned(),
        "circle" => "\u{25e6}".to_owned(),
        "square" => "\u{25aa}".to_owned(),
        "decimal-leading-zero" => format!("{n:02}"),
        "lower-roman" if (1..4000).contains(&n) => roman(n),
        "upper-roman" if (1..4000).contains(&n) => roman(n).to_uppercase(),
        "lower-alpha" | "lower-latin" if n > 0 => alpha(n, b'a', 26),
        "upper-alpha" | "upper-latin" if n > 0 => alpha(n, b'A', 26),
        "lower-greek" if (1..=24).contains(&n) => {
            char::from_u32(0x3b0 + n as u32 + u32::from(n > 17))
                .unwrap_or('?')
                .to_string()
        }
        _ => n.to_string(),
    })
}

/// A counter of CSS's `counter-reset` and `counter-increment`. `scope` is the depth of the
/// frame whose element's children see it: the parent of the element that created it.
struct Counter {
    name: String,
    value: i64,
    scope: usize,
}

/// The counters in scope, outermost first.
#[derive(Default)]
struct Counters(Vec<Counter>);

impl Counters {
    /// `counter-reset`: a new counter, in place of one a sibling made.
    fn reset(&mut self, name: &str, value: i64, scope: usize) {
        match self
            .0
            .iter_mut()
            .rfind(|c| c.name == name && c.scope == scope)
        {
            Some(c) => c.value = value,
            None => self.0.push(Counter {
                name: name.to_owned(),
                value,
                scope,
            }),
        }
    }

    /// `counter-increment` by `by`, or `counter-set` to `value` when `set`. A name no counter has
    /// starts one at 0 first.
    fn change(&mut self, name: &str, by: i64, set: bool, scope: usize) {
        match self.0.iter_mut().rfind(|c| c.name == name) {
            Some(c) if set => c.value = by,
            Some(c) => c.value = c.value.saturating_add(by),
            None => self.reset(name, by, scope),
        }
    }

    /// The innermost counter's value, and every counter's from the outermost.
    fn get(&self, name: &str) -> i64 {
        self.0
            .iter()
            .rfind(|c| c.name == name)
            .map_or(0, |c| c.value)
    }

    fn all(&self, name: &str) -> Vec<i64> {
        let all: Vec<i64> = self
            .0
            .iter()
            .filter(|c| c.name == name)
            .map(|c| c.value)
            .collect();
        if all.is_empty() { vec![0] } else { all }
    }

    /// Ends the counters of a frame deeper than `depth`.
    fn leave(&mut self, depth: usize) {
        self.0.retain(|c| c.scope <= depth);
    }

    /// Applies `style`'s `counter-reset` and `counter-increment` for an element whose parent is
    /// the frame at `scope`.
    fn apply(&mut self, style: &Style, scope: usize) {
        for (name, value) in counter_list(style, "counter-reset", 0) {
            self.reset(&name, value, scope);
        }
        for (name, by) in counter_list(style, "counter-increment", 1) {
            self.change(&name, by, false, scope);
        }
    }
}

/// The names and numbers of `counter-reset` or `counter-increment`, `default` where a name has
/// no number.
fn counter_list(style: &Style, property: &str, default: i64) -> Vec<(String, i64)> {
    let Some(Value::Text(text)) = style.get(property) else {
        return Vec::new();
    };
    let mut parser = Parser::new(text);
    let mut out: Vec<(String, i64)> = Vec::new();
    while let Ok(token) = parser.next() {
        match token {
            Token::Ident(name) if !name.eq_ignore_ascii_case("none") => {
                out.push((name.to_string(), default));
            }
            Token::Number {
                int_value: Some(n), ..
            } => {
                if let Some(last) = out.last_mut() {
                    last.1 = i64::from(*n);
                }
            }
            _ => {}
        }
    }
    out
}

/// The page number, counting from 1, of the first element with each `id`, as the last layout
/// pass placed it.
pub(super) type Targets = BTreeMap<String, usize>;

/// A `::before` or `::after` box's generated content: its text, each `leader()` in it as its
/// byte offset in the text and the character it repeats, and whether a `target-counter()` read
/// a page number.
struct Content {
    text: String,
    leaders: Vec<(usize, String)>,
    targeted: bool,
}

/// The generated content of a `::before` or `::after` box, `None` for `none`, `normal` or no
/// `content`: its strings, `counter()`, `counters()`, `attr()`, `target-counter()`, `leader()`
/// and quotes, in order. A `target-counter()` whose element is not in `targets` is empty.
fn content_text(
    style: &Style,
    dom: &Dom,
    at: usize,
    counters: &Counters,
    targets: &Targets,
) -> Option<Content> {
    let Some(Value::Text(text)) = style.get("content") else {
        return None;
    };
    let mut parser = Parser::new(text);
    let mut out = String::new();
    let mut leaders = Vec::new();
    let mut targeted = false;
    while let Ok(token) = parser.next() {
        match token.clone() {
            Token::QuotedString(s) => out.push_str(&s),
            Token::Ident(name) => match &*name.to_ascii_lowercase() {
                "none" | "normal" => return None,
                "open-quote" => out.push('\u{201c}'),
                "close-quote" => out.push('\u{201d}'),
                _ => {}
            },
            Token::Function(name) => {
                let name = name.to_ascii_lowercase();
                let _ = parser.parse_nested_block(|p| -> Result<(), cssparser::ParseError<()>> {
                    let mut idents: Vec<String> = Vec::new();
                    let mut separator = String::new();
                    // The element a `target-counter()` names, from a string, a `url()` or
                    // an `attr()`.
                    let mut target: Option<String> = None;
                    while let Ok(token) = p.next() {
                        match token.clone() {
                            Token::Ident(i) => idents.push(i.to_string()),
                            Token::QuotedString(s) => {
                                separator = s.to_string();
                                target.get_or_insert_with(|| s.to_string());
                            }
                            Token::UnquotedUrl(u) => target = Some(u.to_string()),
                            Token::Function(f) if f.eq_ignore_ascii_case("attr") => {
                                let attr = p.parse_nested_block(
                                    |q| -> Result<String, cssparser::ParseError<()>> {
                                        Ok(q.expect_ident()?.to_string())
                                    },
                                );
                                if let Ok(attr) = attr {
                                    target = dom.nodes[at].attr(&attr).map(str::to_owned);
                                }
                            }
                            _ => {}
                        }
                    }
                    match name.as_str() {
                        "leader" => {
                            let fill = match idents.first().map(String::as_str) {
                                Some("solid") => "_",
                                Some("space") => " ",
                                Some(_) => ".",
                                None => separator
                                    .chars()
                                    .next()
                                    .map_or(".", |c| &separator[..c.len_utf8()]),
                            };
                            leaders.push((out.len(), fill.to_owned()));
                            return Ok(());
                        }
                        "target-counter" => {
                            targeted = true;
                            let id = target.as_deref().unwrap_or_default();
                            let page = targets.get(id.strip_prefix('#').unwrap_or(id));
                            if let (Some(&page), Some("page")) =
                                (page, idents.first().map(String::as_str))
                            {
                                let kind = idents.get(1).map_or("decimal", String::as_str);
                                out.push_str(&counter_text(kind, page as i64).unwrap_or_default());
                            }
                            return Ok(());
                        }
                        _ => {}
                    }
                    let Some(first) = idents.first() else {
                        return Ok(());
                    };
                    let kind = idents.get(1).map_or("decimal", String::as_str);
                    match name.as_str() {
                        "counter" => {
                            out.push_str(
                                &counter_text(kind, counters.get(first)).unwrap_or_default(),
                            );
                        }
                        "counters" => {
                            let parts: Vec<String> = counters
                                .all(first)
                                .into_iter()
                                .filter_map(|n| counter_text(kind, n))
                                .collect();
                            out.push_str(&parts.join(&separator));
                        }
                        "attr" => out.push_str(dom.nodes[at].attr(first).unwrap_or_default()),
                        _ => {}
                    }
                    Ok(())
                });
            }
            _ => {}
        }
    }
    Some(Content {
        text: out,
        leaders,
        targeted,
    })
}

/// Collects a document's blocks in order.
pub(super) struct Builder<'a, 'd> {
    pub(super) assets: &'a Assets<'d>,
    pub(super) blocks: Vec<Block>,
    pub(super) images: Images,
    pub(super) spans: Vec<Span>,
    text: String,
    runs: Vec<(Range<usize>, usize)>,
    /// The block the text being collected is laid out in: its paragraph style and content box.
    owner: Option<(Para, f32, f32)>,
    /// Whether the text so far ends in a collapsible space, or is empty.
    after_space: bool,
    /// Whether the innermost open block has laid out text, so its `text-indent` is spent.
    indented: bool,
    /// The root element's font size, which `rem` is relative to.
    pub(super) rem: f32,
    /// The boxes of the inline elements in the text being collected.
    inline: Vec<Inline>,
    /// The index in `inline` of each inline box still open, innermost last.
    open_inline: Vec<usize>,
    /// Each `inline-block` box in the text being collected: its byte offset and its index in
    /// `atoms`.
    text_atoms: Vec<(usize, usize)>,
    /// Every `inline-block` box closed so far, so an inner one comes before the one around it.
    pub(super) atoms: Vec<Atom>,
    /// The text and blocks around each `inline-block` box or flex item still open, innermost
    /// last.
    outside: Vec<Outside>,
    /// Each flex container still open, innermost last.
    containers: Vec<Container>,
    /// Each table still open, innermost last.
    tables: Vec<Building>,
    /// The first page's content height, which an image is scaled down to fit.
    pub(super) page_height: f32,
    /// The page name of the last block box opened.
    page: Option<String>,
    /// Each `leader()` in the text being collected: its byte offset, its text style and the
    /// character it repeats.
    leaders: Vec<(usize, usize, String)>,
    /// The pages `target-counter()` reads, and whether one was read.
    targets: &'a Targets,
    pub(super) targeted: bool,
}

/// A flex container whose element has not closed.
struct Container {
    /// The number of frames on the stack inside it, which its own children see.
    depth: usize,
    flex: Flex,
    /// Each item's `order`, beside `flex.items`.
    order: Vec<i64>,
    /// Whether its main axis is a row.
    row: bool,
    /// Whether an anonymous item is open for the text between its children.
    anonymous: bool,
}

/// A table whose element has not closed.
struct Building {
    table: Table,
    /// The stack depth of the children of the table and of each group and row open in it,
    /// innermost last, and whether it is a row.
    depths: Vec<(usize, bool)>,
    /// The group the rows now opened are in.
    part: table::Part,
    /// How many group elements have opened, and the number of the one now open.
    groups: usize,
    group: Option<usize>,
    /// Whether a row is open for the cells now opened.
    row: bool,
}

/// What an element in a table is.
#[derive(Clone, Copy)]
enum TablePart {
    /// The table itself.
    Table,
    /// A caption, under the rows when `bottom` is set.
    Caption {
        bottom: bool,
    },
    Group(table::Part),
    Row(Repeat),
    Cell {
        span: usize,
        rows: usize,
        align: table::Align,
    },
    /// A table directly in a table, a group or a row, in an anonymous cell of its own.
    Wrapped,
}

/// What an `inline-block` box sets aside while its own blocks are collected.
struct Outside {
    blocks: Vec<Block>,
    text: String,
    runs: Vec<(Range<usize>, usize)>,
    owner: Option<(Para, f32, f32)>,
    indented: bool,
    inline: Vec<Inline>,
    open_inline: Vec<usize>,
    text_atoms: Vec<(usize, usize)>,
    leaders: Vec<(usize, usize, String)>,
}

impl Builder<'_, '_> {
    fn push_text(&mut self, text: &str, frame: &Frame) {
        let start = self.text.len();
        let text: Cow<'_, str> = match frame.transform {
            Some("uppercase") => text.to_uppercase().into(),
            Some("lowercase") => text.to_lowercase().into(),
            Some("capitalize") => {
                let mut out = String::with_capacity(text.len());
                let mut word_start = self.after_space || self.text.is_empty();
                for c in text.chars() {
                    if word_start && c.is_alphabetic() {
                        out.extend(c.to_uppercase());
                    } else {
                        out.push(c);
                    }
                    word_start = c.is_whitespace();
                }
                out.into()
            }
            _ => text.into(),
        };
        let text = match frame.hyphenate {
            Some(lang) if frame.para.direction != BaseDirection::Rtl => {
                hyphenated(&text, lang).into()
            }
            _ => text,
        };
        for c in text.chars() {
            let space = matches!(c, ' ' | '\t' | '\n' | '\x0c' | '\r');
            match frame.spaces {
                Spaces::Keep => self.text.push(c),
                Spaces::KeepBreaks if c == '\n' => {
                    while self.text.ends_with(' ') {
                        self.text.pop();
                    }
                    self.text.push('\n');
                    self.after_space = true;
                    continue;
                }
                _ if space => {
                    if !self.after_space {
                        self.text.push(' ');
                        self.after_space = true;
                    }
                    continue;
                }
                _ => self.text.push(c),
            }
            self.after_space = false;
        }
        if self.text.len() > start {
            if self.owner.is_none() {
                self.owner = Some((frame.para, frame.x, frame.width));
            }
            match self.runs.last_mut() {
                Some((range, span)) if *span == frame.span && range.end == start => {
                    range.end = self.text.len();
                }
                _ => self.runs.push((start..self.text.len(), frame.span)),
            }
        }
    }

    /// The index in [`Builder::spans`] of `style`'s text style inside the one at `parent`, linked
    /// to `link` when there is one and to the parent's link otherwise.
    fn span(&mut self, style: &Style, parent: usize, fonts: &Fonts, link: Option<Href>) -> usize {
        let mut span = span_of(style, fonts, Some(&self.spans[parent]));
        if link.is_some() {
            span.link = link;
        }
        if span == self.spans[parent] {
            parent
        } else {
            self.spans.push(span);
            self.spans.len() - 1
        }
    }

    /// Opens the block box of `style` inside the content box of `outer`, and gives `inner` the
    /// box's content box and paragraph.
    fn open(&mut self, style: &Style, outer: &Frame, inner: &mut Frame) -> Result<(), Error> {
        self.flush();
        let mut geometry = geometry(style, outer.x, outer.width);
        geometry.decor.layer = self.layer(style, self.rem)?;
        geometry.decor.transform = super::transform::read(style, self.rem);
        inner.x = geometry.x + geometry.border[3] + geometry.padding[3];
        inner.width = (geometry.width
            - geometry.border[1]
            - geometry.border[3]
            - geometry.padding[1]
            - geometry.padding[3])
            .max(0.0);
        inner.para = para_of(style, inner.width, inner.span);
        self.blocks.push(Block::Open(geometry));
        self.indented = false;
        Ok(())
    }

    /// Gives the block box just opened the page name `page`, and a break before it when the
    /// block box before it had another.
    fn name(&mut self, page: Option<String>) {
        if let Some(Block::Open(geometry)) = self.blocks.last_mut() {
            if self.page != page && geometry.breaks[0].is_none() {
                geometry.breaks[0] = Some(Break::Page);
            }
            geometry.page.clone_from(&page);
        }
        self.page = page;
    }

    /// Opens the box of an inline element with `style`, inside the content box of `frame`, when
    /// it has a margin, a border, padding or a background, and says whether it did.
    fn open_inline(&mut self, style: &Style, frame: &Frame) -> Result<bool, Error> {
        let mut geometry = geometry(style, frame.x, frame.width);
        geometry.decor.layer = self.layer(style, self.rem)?;
        let d = &geometry.decor;
        let drawn = d.background.is_some()
            || d.layer.is_some()
            || d.outline.is_some()
            || geometry
                .border
                .iter()
                .chain(&geometry.padding)
                .any(|w| *w > 0.0)
            || geometry.margin[1] > 0.0
            || geometry.margin[3] > 0.0;
        if drawn {
            self.open_inline.push(self.inline.len());
            self.inline.push(Inline {
                range: self.text.len()..usize::MAX,
                geometry,
            });
        }
        Ok(drawn)
    }

    /// Opens the `inline-block` box of `style` inside the content box of `outer`: sets the
    /// text around it aside and opens its box against a containing block at `x` 0.
    fn open_atom(&mut self, style: &Style, outer: &Frame, inner: &mut Frame) -> Result<(), Error> {
        self.set_aside();
        let origin = Frame {
            x: 0.0,
            ..outer.clone()
        };
        self.open(style, &origin, inner)
    }

    /// Sets the blocks and the text collected so far aside, until [`Builder::close_box`].
    fn set_aside(&mut self) {
        self.outside.push(Outside {
            blocks: std::mem::take(&mut self.blocks),
            text: std::mem::take(&mut self.text),
            runs: std::mem::take(&mut self.runs),
            owner: self.owner.take(),
            indented: self.indented,
            inline: std::mem::take(&mut self.inline),
            open_inline: std::mem::take(&mut self.open_inline),
            text_atoms: std::mem::take(&mut self.text_atoms),
            leaders: std::mem::take(&mut self.leaders),
        });
    }

    /// Closes the innermost box opened after [`Builder::set_aside`], puts back what it set
    /// aside, and returns the box's blocks.
    fn close_box(&mut self) -> Option<Vec<Block>> {
        self.flush();
        self.blocks.push(Block::Close);
        let outside = self.outside.pop()?;
        let blocks = std::mem::replace(&mut self.blocks, outside.blocks);
        self.text = outside.text;
        self.runs = outside.runs;
        self.owner = outside.owner;
        self.indented = outside.indented;
        self.inline = outside.inline;
        self.open_inline = outside.open_inline;
        self.text_atoms = outside.text_atoms;
        self.leaders = outside.leaders;
        Some(blocks)
    }

    /// Closes the innermost open `inline-block` box, whose `vertical-align` is `align`, and
    /// adds it to the text around it at the end of that text, in `frame`'s paragraph.
    fn close_atom(&mut self, align: VerticalAlign, frame: &Frame) {
        let Some(blocks) = self.close_box() else {
            return;
        };
        if self.owner.is_none() {
            self.owner = Some((frame.para, frame.x, frame.width));
        }
        self.text_atoms.push((self.text.len(), self.atoms.len()));
        self.atoms.push(Atom {
            blocks,
            align,
            item: false,
        });
        // Spaces after an inline-block are kept, as after a word.
        self.after_space = false;
    }

    /// Closes the innermost open float, which floats right when `right` is true and clears the
    /// sides `clear` names, inside the content box of `frame`. It is placed before the text
    /// collected around it, at the top of that text's paragraph.
    fn close_float(&mut self, right: bool, clear: [bool; 2], frame: &Frame) {
        let Some(blocks) = self.close_box() else {
            return;
        };
        self.blocks.push(Block::Float {
            atom: self.atoms.len(),
            x: frame.x,
            width: frame.width,
            right,
            clear,
        });
        self.atoms.push(Atom {
            blocks,
            align: VerticalAlign::BASELINE,
            item: false,
        });
    }

    /// Closes the innermost open absolute or `fixed` box, with its `insets` and `z-index`,
    /// inside the content box of `frame`. It is placed before the text collected around it.
    fn close_absolute(&mut self, insets: Insets, z: Option<i64>, fixed: bool, frame: &Frame) {
        let Some(blocks) = self.close_box() else {
            return;
        };
        self.blocks.push(Block::Absolute {
            atom: self.atoms.len(),
            x: frame.x,
            insets,
            z,
            fixed,
        });
        self.atoms.push(Atom {
            blocks,
            align: VerticalAlign::BASELINE,
            item: false,
        });
    }

    /// Closes the innermost open running element, which `position: running(name)` names.
    fn close_running(&mut self, name: String) {
        let Some(blocks) = self.close_box() else {
            return;
        };
        self.blocks.push(Block::Running {
            atom: self.atoms.len(),
            name,
        });
        self.atoms.push(Atom {
            blocks,
            align: VerticalAlign::BASELINE,
            item: false,
        });
    }

    /// Whether an element or text at the stack's `depth` is a child of the innermost open flex
    /// container.
    fn in_container(&self, depth: usize) -> bool {
        self.containers.last().is_some_and(|c| c.depth == depth)
    }

    /// Starts the flex container of `style`, whose block box is the last opened, with its
    /// content box `inner` and its children at the stack's `depth`.
    fn open_container(&mut self, style: &Style, inner: &Frame, depth: usize) {
        let height = match self.blocks.last() {
            Some(Block::Open(g)) => g.min_content,
            _ => 0.0,
        };
        let row = !matches!(
            keyword(style, "flex-direction"),
            Some("column" | "column-reverse")
        );
        self.containers.push(Container {
            depth,
            flex: Flex {
                x: inner.x,
                width: inner.width,
                style: flex::container(style, inner.width, height, self.rem),
                items: Vec::new(),
            },
            order: Vec::new(),
            row,
            anonymous: false,
        });
    }

    /// Ends the innermost flex container: its items become one [`Block::Flex`].
    fn close_container(&mut self) {
        self.close_anonymous();
        if let Some(container) = self.containers.pop() {
            self.flush();
            self.blocks.push(Block::Flex(container.flex));
        }
    }

    /// Starts the table of `style`, whose block box is the last opened, with its content box
    /// `inner` and its children at the stack's `depth`.
    fn open_table(&mut self, style: &Style, inner: &Frame, depth: usize) {
        self.flush();
        self.tables.push(Building {
            table: Table {
                x: inner.x,
                width: inner.width,
                spacing: table::spacing(style, self.rem),
                fixed: table::fixed(style),
                collapse: keyword(style, "border-collapse") == Some("collapse"),
                orphans: lines(style, "orphans"),
                widows: lines(style, "widows"),
                captions: Vec::new(),
                rows: Vec::new(),
                groups: Vec::new(),
            },
            depths: vec![(depth, false)],
            part: table::Part::Body,
            groups: 0,
            group: None,
            row: false,
        });
    }

    /// Ends the innermost table: its rows become one [`Block::Table`].
    fn close_table(&mut self) {
        if let Some(building) = self.tables.pop() {
            self.flush();
            self.blocks.push(Block::Table(building.table));
        }
    }

    /// What the element of `node` with `style` is in the innermost table, when it is a child at
    /// the stack's `depth` of the table, a group or a row. A column is `None` and draws nothing;
    /// any other element is a cell.
    fn table_part(&self, style: &Style, node: &Node, depth: usize) -> Option<Option<TablePart>> {
        if !self.in_table(depth) {
            return None;
        }
        let count = |name: &str, most: usize| {
            node.attr(name)
                .and_then(|v| v.trim().parse::<usize>().ok())
                .filter(|n| *n > 0)
                .map_or(1, |n| n.min(most))
        };
        Some(match keyword(style, "display").unwrap_or("inline") {
            "table-caption" => Some(TablePart::Caption {
                bottom: keyword(style, "caption-side") == Some("bottom"),
            }),
            "table-header-group" => Some(TablePart::Group(table::Part::Head)),
            "table-row-group" => Some(TablePart::Group(table::Part::Body)),
            "table-footer-group" => Some(TablePart::Group(table::Part::Foot)),
            "table-row" => Some(TablePart::Row(Repeat::of(node.attr("data-repeat")))),
            "table-column" | "table-column-group" => None,
            "table" | "inline-table" => Some(TablePart::Wrapped),
            _ => Some(TablePart::Cell {
                span: count("colspan", 1000),
                rows: count("rowspan", 65534),
                align: table::Align::of(style),
            }),
        })
    }

    /// The box a group or a row of the innermost table with `style` draws under its cells:
    /// its background, and its borders under `border-collapse: collapse`. `None` when it
    /// draws nothing.
    fn part_box(&mut self, style: &Style) -> Result<Option<BoxGeometry>, Error> {
        let collapse = self.tables.last().is_some_and(|t| t.table.collapse);
        let mut g = geometry(style, 0.0, 0.0);
        g.decor.layer = self.layer(style, self.rem)?;
        g.margin = [0.0; 4];
        g.padding = [0.0; 4];
        if !collapse {
            g.border = [0.0; 4];
        }
        let drawn = g.decor.background.is_some()
            || g.decor.layer.is_some()
            || g.border.iter().any(|w| *w > 0.0);
        Ok(drawn.then_some(g))
    }

    /// Whether an element or text at the stack's `depth` is a child of the innermost table or
    /// of a group or a row in it.
    fn in_table(&self, depth: usize) -> bool {
        self.tables
            .last()
            .is_some_and(|t| t.depths.last().is_some_and(|(d, _)| *d == depth))
    }

    /// Opens a group or a row of the innermost table, whose children are at the stack's
    /// `depth`, a row with its `data-mark` marks, and each with the box it draws.
    fn open_part(
        &mut self,
        part: TablePart,
        depth: usize,
        marks: Vec<(String, String)>,
        decor: Option<BoxGeometry>,
    ) {
        let Some(t) = self.tables.last_mut() else {
            return;
        };
        match part {
            TablePart::Group(group) => {
                t.part = group;
                t.groups += 1;
                t.group = Some(t.groups);
                t.table.groups.push(decor);
                t.row = false;
                t.depths.push((depth, false));
            }
            TablePart::Row(repeat) => {
                t.table.rows.push(Row {
                    part: t.part,
                    group: t.group,
                    repeat,
                    cells: Vec::new(),
                    slots: Vec::new(),
                    marks,
                    decor,
                });
                t.row = true;
                t.depths.push((depth, true));
            }
            _ => {}
        }
    }

    /// Records a `<slot name>` in the cell now open in the last row of the innermost table. A
    /// slot anywhere else, or with no name, is left out.
    fn slot(&mut self, name: Option<&str>) {
        if let Some(name) = name.map(str::trim).filter(|n| !n.is_empty())
            && let Some(t) = self.tables.last_mut()
            && t.row
            && let Some(row) = t.table.rows.last_mut()
        {
            row.slots.push((row.cells.len(), name.to_owned()));
        }
    }

    /// Closes the innermost group or row of the innermost table.
    fn close_part(&mut self) {
        if let Some(t) = self.tables.last_mut()
            && let Some((_, row)) = t.depths.pop()
        {
            t.row = false;
            if !row {
                t.part = table::Part::Body;
                t.group = None;
            }
        }
    }

    /// Closes the innermost open cell, over `span` columns and `rows` rows and with its
    /// `vertical-align`, into the last row of the innermost table, or a row of its own when
    /// none is open.
    fn close_cell(&mut self, span: usize, rows: usize, align: table::Align) {
        let Some(blocks) = self.close_box() else {
            return;
        };
        let atom = self.atoms.len();
        self.atoms.push(Atom {
            blocks,
            align: VerticalAlign::BASELINE,
            item: true,
        });
        if let Some(t) = self.tables.last_mut() {
            if !t.row || t.table.rows.is_empty() {
                t.table.rows.push(Row {
                    part: t.part,
                    group: t.group,
                    repeat: Repeat::Every,
                    cells: Vec::new(),
                    slots: Vec::new(),
                    marks: Vec::new(),
                    decor: None,
                });
                t.row = true;
            }
            if let Some(row) = t.table.rows.last_mut() {
                row.cells.push(Cell {
                    atom,
                    span,
                    rows,
                    align,
                });
            }
        }
        self.after_space = true;
    }

    /// Closes the innermost open caption, under the rows when `bottom` is set.
    fn close_caption(&mut self, bottom: bool) {
        let Some(blocks) = self.close_box() else {
            return;
        };
        let atom = self.atoms.len();
        self.atoms.push(Atom {
            blocks,
            align: VerticalAlign::BASELINE,
            item: true,
        });
        if let Some(t) = self.tables.last_mut() {
            t.table.captions.push((atom, bottom));
        }
        self.after_space = true;
    }

    /// Whether the innermost flex container's main axis is a row.
    fn row(&self) -> bool {
        self.containers.last().is_none_or(|c| c.row)
    }

    /// Closes the innermost open flex item, with its `order` and `taffy` style, and adds it to
    /// its container after every item of a lower or equal `order`.
    fn close_item(&mut self, order: i64, style: taffy::Style) {
        let Some(blocks) = self.close_box() else {
            return;
        };
        let at = self.atoms.len();
        self.atoms.push(Atom {
            blocks,
            align: VerticalAlign::BASELINE,
            item: true,
        });
        if let Some(c) = self.containers.last_mut() {
            let place = c.order.partition_point(|o| *o <= order);
            c.order.insert(place, order);
            c.flex.items.insert(place, (at, style));
        }
        self.after_space = true;
    }

    /// Opens an anonymous flex item in the innermost container, whose content box is `frame`,
    /// unless one is open, and returns the frame its content is collected in.
    fn open_anonymous(&mut self, frame: &Frame) -> Frame {
        if let Some(c) = self.containers.last_mut()
            && !c.anonymous
        {
            c.anonymous = true;
            self.set_aside();
            self.blocks
                .push(Block::Open(geometry(&Style::default(), 0.0, frame.width)));
            self.indented = false;
            self.after_space = true;
        }
        Frame {
            x: 0.0,
            ..frame.clone()
        }
    }

    /// Closes the innermost container's anonymous item, if one is open.
    fn close_anonymous(&mut self) {
        if let Some(c) = self.containers.last_mut()
            && c.anonymous
        {
            c.anonymous = false;
            self.close_item(0, flex::anonymous());
        }
    }

    /// Closes the innermost open inline box at the end of the text so far.
    fn close_inline(&mut self) {
        if let Some(at) = self.open_inline.pop() {
            self.inline[at].range.end = self.text.len();
        }
    }

    /// The `::before` or `::after` box of the element at `at`, whose own content box is
    /// `frame`: inline text, or a block box of its own when its `display` is not `inline`.
    fn generated(
        &mut self,
        style: &Style,
        dom: &Dom,
        at: usize,
        counters: &Counters,
        frame: &Frame,
        fonts: &Fonts,
    ) -> Result<(), Error> {
        let display = keyword(style, "display").unwrap_or("inline");
        if display == "none" {
            return Ok(());
        }
        let Some(content) = content_text(style, dom, at, counters, self.targets) else {
            return Ok(());
        };
        self.targeted |= content.targeted;
        let mut inner = Frame {
            span: self.span(style, frame.span, fonts, None),
            spaces: spaces_of(style),
            ..frame.clone()
        };
        let block = display != "inline";
        if block {
            self.open(style, frame, &mut inner)?;
        }
        let mut from = 0;
        for (at, fill) in content.leaders {
            self.push_text(&content.text[from..at], &inner);
            if self.owner.is_none() {
                self.owner = Some((inner.para, inner.x, inner.width));
            }
            self.leaders.push((self.text.len(), inner.span, fill));
            from = at;
        }
        self.push_text(&content.text[from..], &inner);
        if block {
            self.flush();
            self.blocks.push(Block::Close);
        }
        Ok(())
    }

    /// Adds bidi control characters in `frame`'s text style, leaving the collapsing of spaces as
    /// it was.
    fn push_control(&mut self, controls: &str, frame: &Frame) {
        let start = self.text.len();
        self.text.push_str(controls);
        if self.owner.is_none() {
            self.owner = Some((frame.para, frame.x, frame.width));
        }
        self.runs.push((start..self.text.len(), frame.span));
    }

    /// Ends the text block being collected, if it has anything to draw. A control character
    /// that closes the paragraph is trimmed with its trailing spaces, because the paragraph's end
    /// closes it too, but nothing before its last `inline-block` box is.
    pub(super) fn flush(&mut self) {
        let floor = self.text_atoms.last().map_or(0, |(at, _)| *at);
        let trimmed = self
            .text
            .trim_end_matches(|c| c == ' ' || c == '\n' || is_bidi_control(c))
            .len()
            .max(floor);
        self.text.truncate(trimmed);
        self.runs.retain_mut(|(range, _)| {
            range.end = range.end.min(trimmed);
            range.start < range.end
        });
        // An inline box still open goes on in the next text block.
        let reopened: Vec<Inline> = self
            .open_inline
            .iter()
            .map(|&at| Inline {
                range: 0..usize::MAX,
                ..self.inline[at].clone()
            })
            .collect();
        self.open_inline = (0..reopened.len()).collect();
        let mut inline = std::mem::replace(&mut self.inline, reopened);
        inline.retain_mut(|i| {
            i.range.end = i.range.end.min(trimmed);
            i.range.start < i.range.end
        });
        let atoms = std::mem::take(&mut self.text_atoms);
        let mut leaders = std::mem::take(&mut self.leaders);
        leaders.retain(|(at, ..)| *at <= trimmed);
        if let Some((mut para, x, width)) = self.owner.take()
            && (!self.runs.is_empty() || !atoms.is_empty())
        {
            if self.indented {
                para.indent = 0.0;
            }
            self.indented = true;
            self.blocks.push(Block::Text {
                text: std::mem::take(&mut self.text),
                spans: std::mem::take(&mut self.runs),
                para,
                x,
                width,
                marker: false,
                inline,
                atoms,
                leaders,
            });
        }
        self.text.clear();
        self.runs.clear();
        self.after_space = true;
    }
}

/// One step of the walk in [`blocks`].
enum Step {
    Enter(usize),
    /// Leaves the element whose frame is on top, writing the bidi controls that close it and
    /// closing its box when it is a block.
    Exit {
        at: usize,
        block: bool,
        /// The `vertical-align` of an `inline-block` box, which closes into the text around it.
        atom: Option<VerticalAlign>,
        /// The `order` and `taffy` style of a flex item, which closes into its container.
        item: Option<(i64, taffy::Style)>,
        /// Whether a float floats right, and the sides its `clear` names.
        float: Option<(bool, [bool; 2])>,
        /// An absolute box's insets, its `z-index`, and whether it is `fixed`.
        absolute: Option<(Insets, Option<i64>, bool)>,
        /// A running element's name.
        running: Option<String>,
        /// Whether the element is a flex container.
        flex: bool,
        /// What the element is in a table, or that it is one.
        table: Option<TablePart>,
        /// Whether the element opened an inline box, which closes after its `::after`.
        inline: bool,
        /// Whether the element's content is tagged with a role, which ends before its box closes.
        tagged: bool,
        close: Option<&'static str>,
    },
}

/// What [`blocks`] collects: the blocks, the `inline-block` boxes, the pictures, the text
/// styles, and whether generated content read a page from `targets`.
type Collected = (Vec<Block>, Vec<Atom>, Images, Vec<Span>, bool);

/// The blocks of `dom`, in document order, and the text styles they name. `content` is the first
/// page's content width and height, `named` gives the content width of a named page, and
/// `targets` the pages `target-counter()` prints.
pub(super) fn blocks(
    dom: &Dom,
    styles: &Styles,
    fonts: &Fonts,
    assets: &Assets<'_>,
    content: [f32; 2],
    named: &dyn Fn(&str) -> f32,
    targets: &Targets,
) -> Result<Collected, Error> {
    let [width, height] = content;
    let width_of = |page: Option<&str>| page.map_or(width, named);
    let empty = Style::default();
    // The root element is the first in document order.
    let rem = styles
        .elements
        .iter()
        .flatten()
        .next()
        .map_or(empty.font_size(), Style::font_size);
    let mut builder = Builder {
        assets,
        blocks: Vec::new(),
        images: Vec::new(),
        spans: vec![span_of(&empty, fonts, None)],
        text: String::new(),
        runs: Vec::new(),
        owner: None,
        after_space: true,
        indented: false,
        rem,
        inline: Vec::new(),
        open_inline: Vec::new(),
        text_atoms: Vec::new(),
        atoms: Vec::new(),
        outside: Vec::new(),
        containers: Vec::new(),
        tables: Vec::new(),
        page_height: height,
        page: None,
        leaders: Vec::new(),
        targets,
        targeted: false,
    };
    let root = Frame {
        x: MARGIN,
        width,
        span: 0,
        spaces: Spaces::Collapse,
        para: para_of(&empty, width, 0),
        transform: None,
        lang: None,
        hyphenate: None,
        page: None,
    };
    let mut counters = Counters::default();
    // An explicit stack, so a deep tree costs heap and never the call stack.
    let mut frames = vec![root];
    let mut work = vec![Step::Enter(0)];
    while let Some(step) = work.pop() {
        let at = match step {
            Step::Enter(at) => at,
            Step::Exit {
                at,
                block,
                atom,
                item,
                float,
                absolute,
                running,
                flex,
                table,
                inline,
                tagged,
                close,
            } => {
                let frame = frames.pop().expect("an exit follows its enter");
                let depth = frames.len();
                if let Some(after) = styles.after.get(&at) {
                    counters.apply(after, depth + 1);
                    builder.generated(after, dom, at, &counters, &frame, fonts)?;
                }
                if let Some(close) = close {
                    builder.push_control(close, &frame);
                }
                // A table's tag is around its block, and every other one inside the box.
                let table_tag =
                    tagged && matches!(table, Some(TablePart::Table | TablePart::Wrapped));
                if tagged && !table_tag {
                    builder.flush();
                    builder.blocks.push(Block::Tag(None));
                }
                if inline {
                    builder.close_inline();
                }
                counters.leave(depth);
                if flex {
                    builder.close_container();
                }
                match table {
                    Some(TablePart::Table) => {
                        builder.close_table();
                        if table_tag {
                            builder.blocks.push(Block::Tag(None));
                        }
                    }
                    Some(TablePart::Wrapped) => {
                        builder.close_table();
                        if table_tag {
                            builder.blocks.push(Block::Tag(None));
                        }
                        builder.blocks.push(Block::Close);
                        builder.blocks.push(Block::Tag(None));
                        builder.close_cell(1, 1, table::Align::Top);
                        continue;
                    }
                    Some(TablePart::Cell { span, rows, align }) => {
                        builder.close_cell(span, rows, align);
                        continue;
                    }
                    Some(TablePart::Caption { bottom }) => {
                        builder.close_caption(bottom);
                        continue;
                    }
                    Some(TablePart::Group(_) | TablePart::Row(_)) => {
                        builder.close_part();
                        continue;
                    }
                    None => {}
                }
                if let Some((order, style)) = item {
                    builder.close_item(order, style);
                } else if let Some(name) = running {
                    builder.close_running(name);
                } else if let Some((insets, z, fixed)) = absolute {
                    let outer = frames.last().expect("the root frame stays");
                    builder.close_absolute(insets, z, fixed, outer);
                } else if let Some((right, clear)) = float {
                    let outer = frames.last().expect("the root frame stays");
                    builder.close_float(right, clear, outer);
                } else if let Some(align) = atom {
                    let outer = frames.last().expect("the root frame stays");
                    builder.close_atom(align, outer);
                } else if block {
                    builder.flush();
                    builder.blocks.push(Block::Close);
                }
                continue;
            }
        };
        let node = &dom.nodes[at];
        let frame = frames.last().expect("the root frame stays").clone();
        match &node.kind {
            Kind::Document => {
                work.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
            }
            // Text directly in a flex container is an anonymous item, unless it is only spaces.
            Kind::Text(text) if builder.in_container(frames.len()) => {
                if !text.trim().is_empty() {
                    let inner = builder.open_anonymous(&frame);
                    builder.push_text(text, &inner);
                }
            }
            // Text directly in a table, a group or a row is left out.
            Kind::Text(_) if builder.in_table(frames.len()) => {}
            Kind::Text(text) => builder.push_text(text, &frame),
            Kind::Ignored => {}
            Kind::Element { name, space, .. } => {
                let item = builder.in_container(frames.len());
                if item {
                    builder.close_anonymous();
                }
                if *space == Space::Svg && name == "svg" {
                    if let Some(style) = styles.of(at)
                        && keyword(style, "display") != Some("none")
                    {
                        // A picture in a flex container is an anonymous item of its own.
                        let inner = if item {
                            builder.open_anonymous(&frame)
                        } else {
                            frame.clone()
                        };
                        builder.inline_svg(dom, at, style, &inner)?;
                        if item {
                            builder.close_anonymous();
                        }
                    }
                    continue;
                }
                if *space != Space::Html {
                    continue;
                }
                // A `<slot>` draws nothing here: the second pass draws its fill in its cell.
                if name == "slot" {
                    builder.slot(node.attr("name"));
                    continue;
                }
                let Some(style) = styles.of(at) else {
                    continue;
                };
                let display = keyword(style, "display").unwrap_or("inline");
                if display == "none" {
                    continue;
                }
                let name = name.as_str();
                if name == "img" {
                    let inner = if item {
                        builder.open_anonymous(&frame)
                    } else {
                        frame.clone()
                    };
                    builder.flush();
                    let alt = node.attr("alt").map(str::to_owned);
                    builder.blocks.push(Block::Tag(Some(Role::Figure(alt))));
                    builder.image(dom, at, style, &inner)?;
                    builder.blocks.push(Block::Tag(None));
                    if item {
                        builder.close_anonymous();
                    }
                    continue;
                }
                if name == "br" {
                    builder.text.push('\n');
                    builder.after_space = true;
                    continue;
                }
                let mut part = match builder.table_part(style, node, frames.len()) {
                    // A column draws nothing.
                    Some(None) => continue,
                    Some(part) => part,
                    None => None,
                };
                if part.is_none() && matches!(display, "table" | "inline-table") {
                    part = Some(TablePart::Table);
                }
                let in_part = part.is_some_and(|p| !matches!(p, TablePart::Table));
                let link = node.attr("href").filter(|_| name == "a").and_then(Href::of);
                let span = builder.span(style, frame.span, fonts, link);
                let mut inner = Frame {
                    span,
                    spaces: spaces_of(style),
                    transform: match keyword(style, "text-transform") {
                        Some("uppercase") => Some("uppercase"),
                        Some("lowercase") => Some("lowercase"),
                        Some("capitalize") => Some("capitalize"),
                        _ => None,
                    },
                    ..frame.clone()
                };
                if let Some(lang) = node.attr("lang") {
                    inner.lang = lang_of(lang);
                }
                inner.hyphenate = if keyword(style, "hyphens") == Some("auto") {
                    inner.lang
                } else {
                    None
                };
                let block = display != "inline";
                let flex = matches!(display, "flex" | "inline-flex" | "grid" | "inline-grid");
                // An absolute, `fixed` or running box is neither a flex item nor a float.
                let position = keyword(style, "position");
                // A part of a table is none of these.
                let absolute = (!in_part && matches!(position, Some("absolute" | "fixed")))
                    .then(|| (insets(style), z_index(style), position == Some("fixed")));
                let running = match style.get("position") {
                    Some(Value::Function(f, name)) if f == "running" && !in_part => {
                        Some(name.trim().to_owned())
                    }
                    _ => None,
                };
                let out = absolute.is_some() || running.is_some();
                let item = (item && !out && !in_part)
                    .then(|| flex::item(style, frame.width, builder.row()));
                let side = sides(style, "float");
                let float = (item.is_none() && !out && !in_part && side != [false, false])
                    .then(|| (side[1], sides(style, "clear")));
                let atom = (item.is_none()
                    && float.is_none()
                    && !out
                    && !in_part
                    && display == "inline-block")
                    .then(|| vertical_align(style, None));
                let inline = if matches!(
                    part,
                    Some(TablePart::Cell { .. } | TablePart::Caption { .. })
                ) {
                    builder.open_atom(style, &frame, &mut inner)?;
                    false
                } else if matches!(part, Some(TablePart::Wrapped)) {
                    // The anonymous cell has no style of its own, and the table's box is in it.
                    let mut cell = inner.clone();
                    builder.open_atom(&empty, &frame, &mut cell)?;
                    builder.blocks.push(Block::Tag(Some(Role::Cell)));
                    builder.open(style, &cell, &mut inner)?;
                    false
                } else if in_part {
                    // A group's or a row's own box draws nothing.
                    false
                } else if item.is_some() || atom.is_some() || float.is_some() || out {
                    builder.open_atom(style, &frame, &mut inner)?;
                    false
                } else if block {
                    let page = match style.get("page") {
                        Some(Value::Text(name)) if name.trim() != "auto" => {
                            Some(name.trim().to_owned())
                        }
                        _ => frame.page.clone(),
                    };
                    // A box that changes the page name is laid out at the new page's width.
                    let outer = Frame {
                        width: (frame.width + width_of(page.as_deref())
                            - width_of(frame.page.as_deref()))
                        .max(0.0),
                        ..frame.clone()
                    };
                    builder.open(style, &outer, &mut inner)?;
                    builder.name(page.clone());
                    inner.page = page;
                    false
                } else {
                    builder.open_inline(style, &frame)?
                };
                // An inline element's text is part of the block around it, so only a box is tagged.
                let role = Role::of(name).filter(|_| block || part.is_some());
                let tagged = role.is_some();
                if role.is_some() {
                    builder.blocks.push(Block::Tag(role));
                }
                for (name, value) in
                    paginate::string_set(style.get("string-set"), &dom.text(at), &|attr| {
                        node.attr(attr).map(str::to_owned)
                    })
                {
                    builder.blocks.push(Block::String {
                        name,
                        value,
                        mark: false,
                    });
                }
                // A row's marks are set on the page the row is drawn on, so the row keeps them.
                let mut row_marks = node.attr("data-mark").map(marks).unwrap_or_default();
                if !matches!(part, Some(TablePart::Row(_))) {
                    for (name, value) in std::mem::take(&mut row_marks) {
                        builder.blocks.push(Block::String {
                            name,
                            value,
                            mark: true,
                        });
                    }
                }
                if let Some(id) = node.attr("id") {
                    builder
                        .blocks
                        .push(Block::Target(Anchor::Id(id.to_owned())));
                }
                if let [b'h', level @ b'1'..=b'6'] = name.as_bytes() {
                    let title = dom
                        .text(at)
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !title.is_empty() {
                        builder
                            .blocks
                            .push(Block::Target(Anchor::Heading(level - b'0', title)));
                    }
                }
                let depth = frames.len();
                counters.apply(style, depth);
                if name == "ol"
                    && let Some(start) = node
                        .attr("start")
                        .and_then(|s| s.trim().parse::<i64>().ok())
                {
                    counters.reset("list-item", start.saturating_sub(1), depth);
                }
                if display == "list-item" {
                    if !counter_list(style, "counter-increment", 1)
                        .iter()
                        .any(|(name, _)| name == "list-item")
                    {
                        counters.change("list-item", 1, false, depth);
                    }
                    if let Some(value) = node.attr("value").and_then(|v| v.trim().parse().ok()) {
                        counters.change("list-item", value, true, depth);
                    }
                    let kind = keyword(style, "list-style-type").unwrap_or("disc");
                    if let Some(text) = marker(kind, counters.get("list-item")) {
                        let text = format!("{text} ");
                        if keyword(style, "list-style-position") == Some("inside") {
                            let marked = Frame {
                                spaces: Spaces::Keep,
                                transform: None,
                                ..inner.clone()
                            };
                            builder.push_text(&text, &marked);
                            builder.after_space = true;
                        } else {
                            builder.blocks.push(Block::Text {
                                spans: vec![(0..text.len(), inner.span)],
                                text,
                                para: Para {
                                    align: Alignment::Left,
                                    wrap: TextWrapMode::NoWrap,
                                    indent: 0.0,
                                    ellipsis: false,
                                    ..inner.para
                                },
                                x: inner.x,
                                width: inner.width,
                                marker: true,
                                inline: Vec::new(),
                                atoms: Vec::new(),
                                leaders: Vec::new(),
                            });
                        }
                    }
                }
                let controls = bidi_controls(style, block);
                if let Some((open, _)) = controls {
                    builder.push_control(open, &inner);
                }
                if let Some(before) = styles.before.get(&at) {
                    counters.apply(before, depth + 1);
                    builder.generated(before, dom, at, &counters, &inner, fonts)?;
                }
                frames.push(inner);
                if flex {
                    let inner = frames.last().expect("just pushed");
                    builder.open_container(style, inner, frames.len());
                }
                match part {
                    Some(TablePart::Table | TablePart::Wrapped) => {
                        let inner = frames.last().expect("just pushed");
                        builder.open_table(style, inner, frames.len());
                    }
                    Some(p @ (TablePart::Group(_) | TablePart::Row(_))) => {
                        let decor = builder.part_box(style)?;
                        builder.open_part(p, frames.len(), row_marks, decor);
                    }
                    _ => {}
                }
                work.push(Step::Exit {
                    at,
                    block,
                    atom,
                    item,
                    float,
                    absolute,
                    running,
                    flex,
                    table: part,
                    inline,
                    tagged,
                    close: controls.map(|(_, close)| close),
                });
                work.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
            }
        }
    }
    builder.flush();
    Ok((
        builder.blocks,
        builder.atoms,
        builder.images,
        builder.spans,
        builder.targeted,
    ))
}

/// The language of a `lang` attribute, by its primary subtag, if `hypher` has its patterns.
fn lang_of(tag: &str) -> Option<Lang> {
    let primary = tag.trim().split(['-', '_']).next()?.to_ascii_lowercase();
    let code: [u8; 2] = primary.as_bytes().try_into().ok()?;
    Lang::from_iso(code)
}

/// The marks of a `data-mark` attribute: `name:value` pairs separated by `;`, each name and
/// value trimmed. A pair with no `:` or an empty name is left out.
fn marks(attr: &str) -> Vec<(String, String)> {
    attr.split(';')
        .filter_map(|pair| {
            let (name, value) = pair.split_once(':')?;
            let name = name.trim();
            (!name.is_empty()).then(|| (name.to_owned(), value.trim().to_owned()))
        })
        .collect()
}

/// `text` with a soft hyphen at each point `hypher` breaks its words at in `lang`.
fn hyphenated(text: &str, lang: Lang) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(char::is_alphabetic) {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest
            .find(|c: char| !c.is_alphabetic())
            .unwrap_or(rest.len());
        out.push_str(&hypher::hyphenate(&rest[..end], lang).join("\u{ad}"));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}
