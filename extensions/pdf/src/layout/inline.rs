//! The walk over the document that collects its blocks: each element's text style and frame,
//! its counters and markers, its generated content, and the text of each block with its spans.

use std::borrow::Cow;
use std::ops::Range;

use cssparser::{Parser, Token};
use hypher::Lang;
use parley::{
    Alignment, AlignmentBaseline, BaseDirection, BaselineShift, LineHeight, OverflowWrap,
    TextWrapMode, VerticalAlign, WordBreak,
};

use super::block::geometry;
use super::{
    BLACK, Block, CONTENT_WIDTH, Frame, Images, MARGIN, Para, Spaces, Span, color, keyword, length,
};
use crate::Error;
use crate::assets::Assets;
use crate::css::{Style, Styles, Value};
use crate::fonts::Fonts;
use crate::parse::{Dom, Kind, Space};

fn span_of(style: &Style, fonts: &Fonts, parent: Option<&Span>) -> Span {
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
fn para_of(style: &Style, width: f32, span: usize) -> Para {
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
        span,
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
fn counter_text(kind: &str, n: i64) -> Option<String> {
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

/// The text of a `::before` or `::after` box's `content`, `None` for `none`, `normal` or no
/// value: its strings, `counter()`, `counters()`, `attr()` and quotes, in order.
fn content_text(style: &Style, dom: &Dom, at: usize, counters: &Counters) -> Option<String> {
    let Some(Value::Text(text)) = style.get("content") else {
        return None;
    };
    let mut parser = Parser::new(text);
    let mut out = String::new();
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
                    while let Ok(token) = p.next() {
                        match token {
                            Token::Ident(i) => idents.push(i.to_string()),
                            Token::QuotedString(s) => separator = s.to_string(),
                            _ => {}
                        }
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
    Some(out)
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

    /// The index in [`Builder::spans`] of `style`'s text style inside the one at `parent`.
    fn span(&mut self, style: &Style, parent: usize, fonts: &Fonts) -> usize {
        let span = span_of(style, fonts, Some(&self.spans[parent]));
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
        let Some(text) = content_text(style, dom, at, counters) else {
            return Ok(());
        };
        let mut inner = Frame {
            span: self.span(style, frame.span, fonts),
            spaces: spaces_of(style),
            ..frame.clone()
        };
        let block = display != "inline";
        if block {
            self.open(style, frame, &mut inner)?;
        }
        self.push_text(&text, &inner);
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
    /// closes it too.
    pub(super) fn flush(&mut self) {
        let trimmed = self
            .text
            .trim_end_matches(|c| c == ' ' || c == '\n' || is_bidi_control(c))
            .len();
        self.text.truncate(trimmed);
        self.runs.retain_mut(|(range, _)| {
            range.end = range.end.min(trimmed);
            range.start < range.end
        });
        if let Some((mut para, x, width)) = self.owner.take()
            && !self.runs.is_empty()
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
        close: Option<&'static str>,
    },
}

/// The blocks of `dom`, in document order, and the text styles they name.
pub(super) fn blocks(
    dom: &Dom,
    styles: &Styles,
    fonts: &Fonts,
    assets: &Assets<'_>,
) -> Result<(Vec<Block>, Images, Vec<Span>), Error> {
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
    };
    let root = Frame {
        x: MARGIN,
        width: CONTENT_WIDTH,
        span: 0,
        spaces: Spaces::Collapse,
        para: para_of(&empty, CONTENT_WIDTH, 0),
        transform: None,
        lang: None,
        hyphenate: None,
    };
    let mut counters = Counters::default();
    // An explicit stack, so a deep tree costs heap and never the call stack.
    let mut frames = vec![root];
    let mut work = vec![Step::Enter(0)];
    while let Some(step) = work.pop() {
        let at = match step {
            Step::Enter(at) => at,
            Step::Exit { at, block, close } => {
                let frame = frames.pop().expect("an exit follows its enter");
                let depth = frames.len();
                if let Some(after) = styles.after.get(&at) {
                    counters.apply(after, depth + 1);
                    builder.generated(after, dom, at, &counters, &frame, fonts)?;
                }
                if let Some(close) = close {
                    builder.push_control(close, &frame);
                }
                counters.leave(depth);
                if block {
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
            Kind::Text(text) => builder.push_text(text, &frame),
            Kind::Ignored => {}
            Kind::Element { name, space, .. } => {
                if *space == Space::Svg && name == "svg" {
                    if let Some(style) = styles.of(at)
                        && keyword(style, "display") != Some("none")
                    {
                        builder.inline_svg(dom, at, style, &frame)?;
                    }
                    continue;
                }
                if *space != Space::Html {
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
                    builder.image(dom, at, style, &frame)?;
                    continue;
                }
                if name == "br" {
                    builder.text.push('\n');
                    builder.after_space = true;
                    continue;
                }
                let span = builder.span(style, frame.span, fonts);
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
                if block {
                    builder.open(style, &frame, &mut inner)?;
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
                work.push(Step::Exit {
                    at,
                    block,
                    close: controls.map(|(_, close)| close),
                });
                work.extend(node.children.iter().rev().map(|&c| Step::Enter(c)));
            }
        }
    }
    builder.flush();
    Ok((builder.blocks, builder.images, builder.spans))
}

/// The language of a `lang` attribute, by its primary subtag, if `hypher` has its patterns.
fn lang_of(tag: &str) -> Option<Lang> {
    let primary = tag.trim().split(['-', '_']).next()?.to_ascii_lowercase();
    let code: [u8; 2] = primary.as_bytes().try_into().ok()?;
    Lang::from_iso(code)
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
