//! Values: a declaration's text read into [`Value`]s by its property's grammar, `var()`
//! substituted, `calc()` evaluated, shorthands expanded into their longhands, and relative
//! lengths computed against the element's font size.
//!
//! A length is held as a sum of units ([`Sum`]) until it is computed, so `calc(100% - 2em)`
//! stays exact: the absolute units fold into points, `em`, `ex` and `ch` resolve against the
//! element's font size and `rem` against the root's, and a percentage stays a percentage for the
//! layout to resolve against its box. `ex` and `ch` are half an `em`, as CSS allows when the font
//! does not say. A computed [`Length`] is points and a percentage.
//!
//! A colour is `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`, `hsla()`, a
//! CSS named colour or `transparent`, and `currentcolor` computes to the element's `color`.

use cssparser::{Parser, Token};

use super::properties::{self, Grammar};

/// A colour as straight-alpha RGBA.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

/// A computed length: points plus a percentage of the box the layout resolves it against.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Length {
    pub pt: f32,
    pub percent: f32,
}

/// A computed value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Keywords(Vec<String>),
    Length(Length),
    Number(f32),
    Integer(i64),
    Color(Color),
    Families(Vec<String>),
    /// A function the grammar names, with its argument text: `running(header)`.
    Function(String, String),
    /// A [`Grammar::Free`] or [`Grammar::Transform`] value's text after substitution.
    Text(String),
}

/// A length before it is computed: points, `em`s, `rem`s and a percentage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sum {
    pub pt: f32,
    pub em: f32,
    pub rem: f32,
    pub percent: f32,
}

impl Sum {
    fn scale(self, by: f32) -> Sum {
        Sum {
            pt: self.pt * by,
            em: self.em * by,
            rem: self.rem * by,
            percent: self.percent * by,
        }
    }

    fn add(self, other: Sum) -> Sum {
        Sum {
            pt: self.pt + other.pt,
            em: self.em + other.em,
            rem: self.rem + other.rem,
            percent: self.percent + other.percent,
        }
    }

    fn is_negative(self) -> bool {
        self.pt < 0.0 || self.em < 0.0 || self.rem < 0.0 || self.percent < 0.0
    }

    /// The computed length against the element's font size and the root's, in points.
    pub fn compute(self, font_size: f32, root_font_size: f32) -> Length {
        Length {
            pt: self.pt + self.em * font_size + self.rem * root_font_size,
            percent: self.percent,
        }
    }
}

/// One top-level component of a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Comp {
    Ident(String),
    Dimension(f32, String),
    Percentage(f32),
    Number(f32, bool),
    Hash(String),
    Str(String),
    Url(String),
    /// A function's lower-case name and its argument text.
    Function(String, String),
    Slash,
    Comma,
    Delim(char),
}

/// The value's top-level components, or `None` when it holds a token no grammar takes: a bare
/// block or a bad string or URL. Each identifier is lower-cased.
pub fn components(text: &str) -> Option<Vec<Comp>> {
    parse_components(text, true)
}

/// [`components`] with each identifier as written, for a custom identifier such as a named
/// string's name, which CSS compares with its case.
pub fn components_as_written(text: &str) -> Option<Vec<Comp>> {
    parse_components(text, false)
}

fn parse_components(text: &str, lower: bool) -> Option<Vec<Comp>> {
    let mut parser = Parser::new(text);
    let mut out = Vec::new();
    loop {
        let token = match parser.next() {
            Ok(token) => token.clone(),
            Err(_) => break,
        };
        let comp = match token {
            Token::Ident(s) if lower => Comp::Ident(s.to_ascii_lowercase()),
            Token::Ident(s) => Comp::Ident(s.to_string()),
            Token::Dimension { value, unit, .. } => {
                Comp::Dimension(value, unit.to_ascii_lowercase())
            }
            Token::Percentage { unit_value, .. } => Comp::Percentage(unit_value * 100.0),
            Token::Number {
                value, int_value, ..
            } => Comp::Number(value, int_value.is_some()),
            Token::Hash(s) | Token::IDHash(s) => Comp::Hash(s.to_string()),
            Token::QuotedString(s) => Comp::Str(s.to_string()),
            Token::UnquotedUrl(s) => Comp::Url(s.to_string()),
            Token::Function(name) => {
                let name = name.to_ascii_lowercase();
                let start = parser.position();
                let args = parser
                    .parse_nested_block(|p| {
                        while p.next_including_whitespace_and_comments().is_ok() {}
                        Ok::<_, cssparser::ParseError<()>>(p.slice_from(start).to_owned())
                    })
                    .ok()?;
                if name == "url" {
                    Comp::Url(unquote(args.trim()))
                } else {
                    Comp::Function(name, args.trim().to_owned())
                }
            }
            Token::Delim('/') => Comp::Slash,
            Token::Comma => Comp::Comma,
            Token::Delim(c) => Comp::Delim(c),
            _ => return None,
        };
        out.push(comp);
    }
    Some(out)
}

fn unquote(text: &str) -> String {
    let bytes = text.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
    {
        text[1..text.len() - 1].to_owned()
    } else {
        text.to_owned()
    }
}

/// Whether the text calls `var()` anywhere, nested or not.
pub fn has_var(text: &str) -> bool {
    let mut parser = Parser::new(text);
    fn walk(p: &mut Parser<'_>) -> bool {
        while let Ok(token) = p.next_including_whitespace_and_comments() {
            let is_var = matches!(token, Token::Function(name) if name.eq_ignore_ascii_case("var"));
            let nests = matches!(
                token,
                Token::Function(_)
                    | Token::ParenthesisBlock
                    | Token::SquareBracketBlock
                    | Token::CurlyBracketBlock
            );
            if is_var {
                return true;
            }
            // A nested block must be read to its end, or the parser returns an error for it.
            if nests
                && p.parse_nested_block(|p| {
                    let found = walk(p);
                    while p.next_including_whitespace_and_comments().is_ok() {}
                    Ok::<_, cssparser::ParseError<()>>(found)
                })
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }
    walk(&mut parser)
}

/// `text` with every `var(--name, fallback)` replaced by the custom property's value, or its
/// fallback when the property is not set. `None` when a name is missing and has no fallback, or
/// the substitution nests deeper than [`MAX_VAR_DEPTH`], which is how a cycle ends.
pub fn substitute(text: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    substitute_at(text, lookup, 0)
}

/// How deep one `var()` may reach through the custom properties it names.
pub const MAX_VAR_DEPTH: usize = 32;

fn substitute_at(
    text: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    depth: usize,
) -> Option<String> {
    if depth > MAX_VAR_DEPTH {
        return None;
    }
    let mut parser = Parser::new(text);
    let mut out = String::new();
    let mut ok = true;
    walk_substitute(&mut parser, lookup, depth, &mut out, &mut ok);
    ok.then_some(out)
}

fn walk_substitute(
    p: &mut Parser<'_>,
    lookup: &dyn Fn(&str) -> Option<String>,
    depth: usize,
    out: &mut String,
    ok: &mut bool,
) {
    loop {
        let start = p.position();
        let token = match p.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => return,
        };
        match token {
            Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                let args_start = p.position();
                let args = p
                    .parse_nested_block(|p| {
                        while p.next_including_whitespace_and_comments().is_ok() {}
                        Ok::<_, cssparser::ParseError<()>>(p.slice_from(args_start).to_owned())
                    })
                    .unwrap_or_default();
                let (name, fallback) = match args.split_once(',') {
                    Some((name, fallback)) => (name.trim(), Some(fallback.trim())),
                    None => (args.trim(), None),
                };
                let got = lookup(name).or_else(|| fallback.map(str::to_owned));
                match got.and_then(|value| substitute_at(&value, lookup, depth + 1)) {
                    Some(value) => out.push_str(&value),
                    None => *ok = false,
                }
            }
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                out.push_str(p.slice_from(start));
                let close = match token {
                    Token::SquareBracketBlock => "]",
                    Token::CurlyBracketBlock => "}",
                    _ => ")",
                };
                let _ = p.parse_nested_block(|p| {
                    walk_substitute(p, lookup, depth, out, ok);
                    Ok::<_, cssparser::ParseError<()>>(())
                });
                out.push_str(close);
            }
            _ => out.push_str(p.slice_from(start)),
        }
    }
}

/// A declaration's value read for its longhands: each longhand the property sets, with its
/// specified value, or `None` when the value is not one the property takes. A CSS-wide keyword
/// (`inherit`, `initial`, `unset`, `revert`) is returned as itself for every longhand.
pub fn expand(property: &str, text: &str) -> Option<Vec<(&'static str, Specified)>> {
    let comps = components(text)?;
    if comps.is_empty() {
        return None;
    }
    if let [Comp::Ident(word)] = comps.as_slice()
        && matches!(word.as_str(), "inherit" | "initial" | "unset" | "revert")
    {
        let wide = Specified::Wide(word.clone());
        return Some(
            properties::longhands(property)?
                .into_iter()
                .map(|name| (name, wide.clone()))
                .collect(),
        );
    }
    if let Some(p) = properties::property(property) {
        return Some(vec![(p.name, parse(p.grammar, &comps, text)?)]);
    }
    let longhands = properties::shorthand(property)?;
    let set = |values: Vec<Specified>| -> Option<Vec<(&'static str, Specified)>> {
        Some(longhands.iter().copied().zip(values).collect())
    };
    match property {
        "margin" | "padding" | "inset" | "border-width" | "border-style" | "border-color" => {
            let grammar = properties::property(longhands[0])?.grammar;
            set(four(&comps, |c| {
                parse(grammar, std::slice::from_ref(c), text)
            })?)
        }
        "border-radius" => {
            // A corner is one radius, so the elliptical syntax with a slash is dropped.
            set(four(&comps, |c| {
                parse(len(false), std::slice::from_ref(c), text)
            })?)
        }
        "border" | "border-top" | "border-right" | "border-bottom" | "border-left" | "outline" => {
            let [width, style, color] = border(&comps)?;
            if longhands.len() == 3 {
                set(vec![width, style, color])
            } else {
                let mut all = vec![width; 4];
                all.extend(vec![style; 4]);
                all.extend(vec![color; 4]);
                set(all)
            }
        }
        "font" => set(font(&comps, text)?),
        "text-decoration" => {
            let mut line = Vec::new();
            let mut style = None;
            let mut color = None;
            for c in &comps {
                if let Comp::Ident(w) = c
                    && matches!(
                        w.as_str(),
                        "underline" | "overline" | "line-through" | "none"
                    )
                {
                    line.push(c.clone());
                } else if style.is_none()
                    && let Some(s) = keyword_of(c, &["solid", "double", "dotted", "dashed", "wavy"])
                {
                    style = Some(s);
                } else if color.is_none()
                    && let Some(c) = color_of(c)
                {
                    color = Some(c);
                } else {
                    return None;
                }
            }
            let line = if line.is_empty() {
                Specified::Wide("initial".to_owned())
            } else {
                parse(
                    Grammar::Keywords(&["underline", "overline", "line-through"]),
                    &line,
                    text,
                )?
            };
            set(vec![
                line,
                style.unwrap_or(Specified::Wide("initial".to_owned())),
                color.unwrap_or(Specified::Wide("initial".to_owned())),
            ])
        }
        "background" => {
            let mut color = None;
            let mut image = None;
            let mut repeat = Vec::new();
            let mut rest = Vec::new();
            for c in &comps {
                match c {
                    Comp::Url(_) => image = Some(Specified::Text(serialize(c))),
                    Comp::Function(name, _) if name.ends_with("gradient") => {
                        image = Some(Specified::Text(serialize(c)))
                    }
                    Comp::Ident(w)
                        if matches!(
                            w.as_str(),
                            "repeat" | "no-repeat" | "repeat-x" | "repeat-y" | "space" | "round"
                        ) =>
                    {
                        repeat.push(serialize(c))
                    }
                    Comp::Ident(w) if w == "none" => image = Some(Specified::Text("none".into())),
                    _ => match color_of(c) {
                        Some(found) if color.is_none() => color = Some(found),
                        _ => rest.push(c.clone()),
                    },
                }
            }
            let (position, size) = match rest.iter().position(|c| *c == Comp::Slash) {
                Some(at) => (&rest[..at], Some(&rest[at + 1..])),
                None => (&rest[..], None),
            };
            let text_of = |part: &[Comp]| -> Option<Specified> {
                part.iter()
                    .all(|c| {
                        matches!(
                            c,
                            Comp::Ident(_)
                                | Comp::Dimension(..)
                                | Comp::Percentage(_)
                                | Comp::Number(0.0, _)
                        )
                    })
                    .then(|| {
                        Specified::Text(part.iter().map(serialize).collect::<Vec<_>>().join(" "))
                    })
            };
            let initial = || Specified::Wide("initial".to_owned());
            set(vec![
                color.unwrap_or_else(initial),
                image.unwrap_or_else(initial),
                if repeat.is_empty() {
                    initial()
                } else {
                    Specified::Text(repeat.join(" "))
                },
                if position.is_empty() {
                    initial()
                } else {
                    text_of(position)?
                },
                match size {
                    Some(size) if !size.is_empty() => text_of(size)?,
                    Some(_) => return None,
                    None => initial(),
                },
            ])
        }
        "list-style" => {
            let mut kind = None;
            let mut position = None;
            let mut image = None;
            for c in &comps {
                if position.is_none()
                    && let Some(p) = keyword_of(c, &["inside", "outside"])
                {
                    position = Some(p);
                } else if kind.is_none()
                    && let Some(k) = keyword_of(
                        c,
                        match properties::property("list-style-type")?.grammar {
                            Grammar::Keyword(words) => words,
                            _ => &[],
                        },
                    )
                {
                    kind = Some(k);
                } else if image.is_none() && matches!(c, Comp::Url(_)) {
                    image = Some(Specified::Text(serialize(c)));
                } else {
                    return None;
                }
            }
            let initial = || Specified::Wide("initial".to_owned());
            set(vec![
                kind.unwrap_or_else(initial),
                position.unwrap_or_else(initial),
                image.unwrap_or_else(initial),
            ])
        }
        "overflow" | "gap" => {
            let grammar = properties::property(longhands[0])?.grammar;
            let values = comps
                .iter()
                .map(|c| parse(grammar, std::slice::from_ref(c), text))
                .collect::<Option<Vec<_>>>()?;
            match values.as_slice() {
                [one] => set(vec![one.clone(), one.clone()]),
                [a, b] => set(vec![a.clone(), b.clone()]),
                _ => None,
            }
        }
        "flex" => {
            let number = |c: &Comp| match c {
                Comp::Number(n, _) if *n >= 0.0 => Some(Specified::Number(*n)),
                _ => None,
            };
            let basis = |c: &Comp| {
                parse(len(false), std::slice::from_ref(c), text)
                    .or_else(|| keyword_of(c, &["auto", "content"]))
            };
            match comps.as_slice() {
                [Comp::Ident(w)] if w == "none" => set(vec![
                    Specified::Number(0.0),
                    Specified::Number(0.0),
                    Specified::Keyword("auto".into()),
                ]),
                [Comp::Ident(w)] if w == "auto" => set(vec![
                    Specified::Number(1.0),
                    Specified::Number(1.0),
                    Specified::Keyword("auto".into()),
                ]),
                [g] if number(g).is_some() => set(vec![
                    number(g)?,
                    Specified::Number(1.0),
                    Specified::Length(Sum::default()),
                ]),
                [b] => set(vec![
                    Specified::Number(1.0),
                    Specified::Number(1.0),
                    basis(b)?,
                ]),
                [g, s] if number(s).is_some() => set(vec![
                    number(g)?,
                    number(s)?,
                    Specified::Length(Sum::default()),
                ]),
                [g, b] => set(vec![number(g)?, Specified::Number(1.0), basis(b)?]),
                [g, s, b] => set(vec![number(g)?, number(s)?, basis(b)?]),
                _ => None,
            }
        }
        "flex-flow" => {
            let mut direction = None;
            let mut wrap = None;
            for c in &comps {
                if direction.is_none()
                    && let Some(d) =
                        keyword_of(c, &["row", "row-reverse", "column", "column-reverse"])
                {
                    direction = Some(d);
                } else if wrap.is_none()
                    && let Some(w) = keyword_of(c, &["nowrap", "wrap", "wrap-reverse"])
                {
                    wrap = Some(w);
                } else {
                    return None;
                }
            }
            let initial = || Specified::Wide("initial".to_owned());
            set(vec![
                direction.unwrap_or_else(initial),
                wrap.unwrap_or_else(initial),
            ])
        }
        "page-break-before" | "page-break-after" | "page-break-inside" => {
            let allowed: &[&str] = if property == "page-break-inside" {
                &["auto", "avoid"]
            } else {
                &["auto", "always", "avoid", "left", "right"]
            };
            match keyword_of(comps.first()?, allowed)? {
                Specified::Keyword(w) if comps.len() == 1 => {
                    set(vec![Specified::Keyword(if w == "always" {
                        "page".to_owned()
                    } else {
                        w
                    })])
                }
                _ => None,
            }
        }
        _ => None,
    }
}

const fn len(negative: bool) -> Grammar {
    Grammar::Length {
        negative,
        keywords: &[],
    }
}

/// A value as the grammar reads it, before relative lengths and `currentcolor` are computed.
#[derive(Debug, Clone, PartialEq)]
pub enum Specified {
    /// `inherit`, `initial`, `unset` or `revert`.
    Wide(String),
    Keyword(String),
    Keywords(Vec<String>),
    Length(Sum),
    Number(f32),
    Integer(i64),
    Color(Color),
    CurrentColor,
    Families(Vec<String>),
    Function(String, String),
    Text(String),
}

fn keyword_of(c: &Comp, words: &[&str]) -> Option<Specified> {
    match c {
        Comp::Ident(w) if words.contains(&w.as_str()) => Some(Specified::Keyword(w.clone())),
        _ => None,
    }
}

fn color_of(c: &Comp) -> Option<Specified> {
    parse(Grammar::Color, std::slice::from_ref(c), "")
}

/// One to four values set in the top, right, bottom, left order.
fn four(comps: &[Comp], one: impl Fn(&Comp) -> Option<Specified>) -> Option<Vec<Specified>> {
    let values = comps.iter().map(one).collect::<Option<Vec<_>>>()?;
    let [t, r, b, l] = match values.as_slice() {
        [a] => [a, a, a, a],
        [a, b] => [a, b, a, b],
        [a, b, c] => [a, b, c, b],
        [a, b, c, d] => [a, b, c, d],
        _ => return None,
    };
    Some(vec![t.clone(), r.clone(), b.clone(), l.clone()])
}

/// A border's width, style and colour, in any order and each at most once.
fn border(comps: &[Comp]) -> Option<[Specified; 3]> {
    let width = properties::property("border-top-width")?.grammar;
    let style = properties::property("border-top-style")?.grammar;
    let mut found: [Option<Specified>; 3] = [None, None, None];
    for c in comps {
        let one = std::slice::from_ref(c);
        if found[0].is_none()
            && let Some(w) = parse(width, one, "")
        {
            found[0] = Some(w);
        } else if found[1].is_none()
            && let Some(s) = parse(style, one, "")
        {
            found[1] = Some(s);
        } else if found[2].is_none()
            && let Some(c) = color_of(c)
        {
            found[2] = Some(c);
        } else {
            return None;
        }
    }
    Some(found.map(|v| v.unwrap_or(Specified::Wide("initial".to_owned()))))
}

/// `font`: up to three of style, variant and weight, then the size, an optional `/` and line
/// height, and the families.
fn font(comps: &[Comp], text: &str) -> Option<Vec<Specified>> {
    let initial = || Specified::Wide("initial".to_owned());
    let (mut style, mut variant, mut weight) = (None, None, None);
    let mut at = 0;
    while at < comps.len() {
        let c = &comps[at];
        let one = std::slice::from_ref(c);
        if let Comp::Ident(w) = c
            && w == "normal"
        {
        } else if style.is_none()
            && let Some(s) = keyword_of(c, &["italic", "oblique"])
        {
            style = Some(s);
        } else if variant.is_none()
            && let Some(v) = keyword_of(c, &["small-caps"])
        {
            variant = Some(v);
        } else if weight.is_none()
            && !matches!(c, Comp::Dimension(..) | Comp::Percentage(_))
            && let Some(w) = parse(Grammar::FontWeight, one, text)
        {
            weight = Some(w);
        } else {
            break;
        }
        at += 1;
    }
    let size_grammar = properties::property("font-size")?.grammar;
    let size = parse(size_grammar, std::slice::from_ref(comps.get(at)?), text)?;
    at += 1;
    let mut line_height = initial();
    if comps.get(at) == Some(&Comp::Slash) {
        line_height = parse(
            Grammar::LineHeight,
            std::slice::from_ref(comps.get(at + 1)?),
            text,
        )?;
        at += 2;
    }
    let families = parse(Grammar::Families, &comps[at..], text)?;
    Some(vec![
        style.unwrap_or_else(initial),
        variant.unwrap_or_else(initial),
        weight.unwrap_or_else(initial),
        size,
        line_height,
        families,
    ])
}

/// The components read by one longhand's grammar.
pub fn parse(grammar: Grammar, comps: &[Comp], text: &str) -> Option<Specified> {
    match grammar {
        Grammar::Free => (!comps.is_empty()).then(|| Specified::Text(text.trim().to_owned())),
        Grammar::Transform => transform(comps).then(|| Specified::Text(text.trim().to_owned())),
        Grammar::Families => families(comps).map(Specified::Families),
        Grammar::Keywords(words) => {
            if let [Comp::Ident(w)] = comps
                && w == "none"
            {
                return Some(Specified::Keywords(Vec::new()));
            }
            let mut seen = Vec::new();
            for c in comps {
                match c {
                    Comp::Ident(w) if words.contains(&w.as_str()) && !seen.contains(w) => {
                        seen.push(w.clone())
                    }
                    _ => return None,
                }
            }
            (!seen.is_empty()).then_some(Specified::Keywords(seen))
        }
        _ => {
            let [c] = comps else { return None };
            one(grammar, c)
        }
    }
}

fn one(grammar: Grammar, c: &Comp) -> Option<Specified> {
    match grammar {
        Grammar::Length { negative, keywords } => {
            if let Some(k) = keyword_of(c, keywords) {
                return Some(k);
            }
            let sum = length(c)?;
            (negative || is_calc(c) || !sum.is_negative()).then_some(Specified::Length(sum))
        }
        Grammar::Color => match c {
            Comp::Ident(w) if w == "currentcolor" => Some(Specified::CurrentColor),
            _ => color(c).map(Specified::Color),
        },
        Grammar::Keyword(words) => keyword_of(c, words),
        Grammar::Number(words) => keyword_of(c, words).or_else(|| match number(c)? {
            n if n >= 0.0 => Some(Specified::Number(n)),
            _ => None,
        }),
        Grammar::Integer { positive, keywords } => keyword_of(c, keywords).or_else(|| match c {
            Comp::Number(n, true) if !positive || *n >= 1.0 => Some(Specified::Integer(*n as i64)),
            _ => None,
        }),
        Grammar::LineHeight => keyword_of(c, &["normal"])
            .or_else(|| match c {
                Comp::Number(n, _) if *n >= 0.0 => Some(Specified::Number(*n)),
                _ => None,
            })
            .or_else(|| {
                let sum = length(c)?;
                (is_calc(c) || !sum.is_negative()).then_some(Specified::Length(sum))
            }),
        Grammar::FontWeight => {
            keyword_of(c, &["normal", "bold", "bolder", "lighter"]).or_else(|| match number(c)? {
                n if (1.0..=1000.0).contains(&n) => Some(Specified::Number(n)),
                _ => None,
            })
        }
        Grammar::Position => {
            keyword_of(c, &["static", "relative", "absolute", "fixed"]).or_else(|| match c {
                Comp::Function(name, args)
                    if name == "running"
                        && matches!(components(args)?.as_slice(), [Comp::Ident(_)]) =>
                {
                    Some(Specified::Function(name.clone(), args.clone()))
                }
                _ => None,
            })
        }
        Grammar::Free | Grammar::Transform | Grammar::Families | Grammar::Keywords(_) => None,
    }
}

/// Whether the component is a `calc()`. CSS clamps a `calc()` to its property's range where it
/// is used, so its sign never makes a value invalid.
fn is_calc(c: &Comp) -> bool {
    matches!(c, Comp::Function(name, _) if name == "calc")
}

/// A plain number, or a `calc()` that evaluates to one.
fn number(c: &Comp) -> Option<f32> {
    match c {
        Comp::Number(n, _) => Some(*n),
        Comp::Function(name, args) if name == "calc" => match calc(args)? {
            Quantity::Number(n) => Some(n),
            Quantity::Length(_) => None,
        },
        _ => None,
    }
}

/// A length, a percentage, a unitless zero, or a `calc()` that evaluates to a length.
pub fn length(c: &Comp) -> Option<Sum> {
    match c {
        Comp::Dimension(v, unit) => unit_sum(*v, unit),
        Comp::Percentage(p) => Some(Sum {
            percent: *p,
            ..Sum::default()
        }),
        Comp::Number(n, _) if *n == 0.0 => Some(Sum::default()),
        Comp::Function(name, args) if name == "calc" => match calc(args)? {
            Quantity::Length(sum) => Some(sum),
            Quantity::Number(n) if n == 0.0 => Some(Sum::default()),
            Quantity::Number(_) => None,
        },
        _ => None,
    }
}

/// `value` in `unit` as a [`Sum`]: the absolute units in points, the relative ones as `em`s and
/// `rem`s.
fn unit_sum(value: f32, unit: &str) -> Option<Sum> {
    let pt = |scale: f32| Sum {
        pt: value * scale,
        ..Sum::default()
    };
    Some(match unit {
        "pt" => pt(1.0),
        "px" => pt(0.75),
        "pc" => pt(12.0),
        "in" => pt(72.0),
        "cm" => pt(72.0 / 2.54),
        "mm" => pt(72.0 / 25.4),
        "q" => pt(72.0 / 101.6),
        "em" => Sum {
            em: value,
            ..Sum::default()
        },
        "ex" | "ch" => Sum {
            em: value / 2.0,
            ..Sum::default()
        },
        "rem" => Sum {
            rem: value,
            ..Sum::default()
        },
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy)]
enum Quantity {
    Number(f32),
    Length(Sum),
}

/// The arguments of a `calc()`: sums and products of numbers, lengths and percentages, with
/// parentheses and nested `calc()`.
fn calc(args: &str) -> Option<Quantity> {
    let mut parser = Parser::new(args);
    let q = calc_sum(&mut parser)?;
    parser.is_exhausted().then_some(q)
}

fn calc_sum(p: &mut Parser<'_>) -> Option<Quantity> {
    let mut left = calc_product(p)?;
    loop {
        let state = p.state();
        let sign = match p.next() {
            Ok(Token::Delim('+')) => 1.0,
            Ok(Token::Delim('-')) => -1.0,
            _ => {
                p.reset(&state);
                return Some(left);
            }
        };
        let right = calc_product(p)?;
        left = match (left, right) {
            (Quantity::Number(a), Quantity::Number(b)) => Quantity::Number(a + sign * b),
            (Quantity::Length(a), Quantity::Length(b)) => Quantity::Length(a.add(b.scale(sign))),
            _ => return None,
        };
    }
}

fn calc_product(p: &mut Parser<'_>) -> Option<Quantity> {
    let mut left = calc_term(p)?;
    loop {
        let state = p.state();
        let divide = match p.next() {
            Ok(Token::Delim('*')) => false,
            Ok(Token::Delim('/')) => true,
            _ => {
                p.reset(&state);
                return Some(left);
            }
        };
        let right = calc_term(p)?;
        left = match (left, right, divide) {
            (Quantity::Number(a), Quantity::Number(b), false) => Quantity::Number(a * b),
            (Quantity::Number(a), Quantity::Number(b), true) if b != 0.0 => Quantity::Number(a / b),
            (Quantity::Length(a), Quantity::Number(b), false)
            | (Quantity::Number(b), Quantity::Length(a), false) => Quantity::Length(a.scale(b)),
            (Quantity::Length(a), Quantity::Number(b), true) if b != 0.0 => {
                Quantity::Length(a.scale(1.0 / b))
            }
            _ => return None,
        };
    }
}

fn calc_term(p: &mut Parser<'_>) -> Option<Quantity> {
    match p.next().ok()?.clone() {
        Token::Number { value, .. } => Some(Quantity::Number(value)),
        Token::Dimension { value, unit, .. } => {
            unit_sum(value, &unit.to_ascii_lowercase()).map(Quantity::Length)
        }
        Token::Percentage { unit_value, .. } => Some(Quantity::Length(Sum {
            percent: unit_value * 100.0,
            ..Sum::default()
        })),
        Token::ParenthesisBlock => p
            .parse_nested_block(|p| {
                let q = calc_sum(p);
                Ok::<_, cssparser::ParseError<()>>(q.filter(|_| p.is_exhausted()))
            })
            .ok()
            .flatten(),
        Token::Function(name) if name.eq_ignore_ascii_case("calc") => p
            .parse_nested_block(|p| {
                let q = calc_sum(p);
                Ok::<_, cssparser::ParseError<()>>(q.filter(|_| p.is_exhausted()))
            })
            .ok()
            .flatten(),
        _ => None,
    }
}

fn families(comps: &[Comp]) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for part in comps.split(|c| *c == Comp::Comma) {
        let name = match part {
            [Comp::Str(s)] => s.clone(),
            words if !words.is_empty() => words
                .iter()
                .map(|c| match c {
                    Comp::Ident(w) => Some(w.as_str()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?
                .join(" "),
            _ => return None,
        };
        out.push(name);
    }
    Some(out)
}

/// `none`, or a list of 2D transform functions only: a 3D one makes the whole value invalid.
fn transform(comps: &[Comp]) -> bool {
    if let [Comp::Ident(w)] = comps {
        return w == "none";
    }
    !comps.is_empty()
        && comps.iter().all(|c| {
            matches!(c, Comp::Function(name, _) if matches!(
                name.as_str(),
                "matrix" | "translate" | "translatex" | "translatey" | "scale" | "scalex"
                    | "scaley" | "rotate" | "skew" | "skewx" | "skewy"
            ))
        })
}

/// The component written back as CSS.
fn serialize(c: &Comp) -> String {
    match c {
        Comp::Ident(w) => w.clone(),
        Comp::Dimension(v, unit) => format!("{v}{unit}"),
        Comp::Percentage(p) => format!("{p}%"),
        Comp::Number(n, _) => format!("{n}"),
        Comp::Hash(h) => format!("#{h}"),
        Comp::Str(s) => format!("{s:?}"),
        Comp::Url(u) => format!("url({u:?})"),
        Comp::Function(name, args) => format!("{name}({args})"),
        Comp::Slash => "/".to_owned(),
        Comp::Comma => ",".to_owned(),
        Comp::Delim(c) => c.to_string(),
    }
}

/// The colour `c` names: a hex colour, a keyword, `rgb()` or `hsl()`.
pub fn color(c: &Comp) -> Option<Color> {
    match c {
        Comp::Hash(hex) => hex_color(hex),
        Comp::Ident(name) if name == "transparent" => Some(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0.0,
        }),
        Comp::Ident(name) => {
            let at = NAMED
                .binary_search_by(|(n, _)| (*n).cmp(name.as_str()))
                .ok()?;
            let rgb = NAMED[at].1;
            Some(Color {
                r: (rgb >> 16) as u8,
                g: (rgb >> 8) as u8,
                b: rgb as u8,
                a: 1.0,
            })
        }
        Comp::Function(name, args) => match name.as_str() {
            "rgb" | "rgba" => rgb_function(args),
            "hsl" | "hsla" => hsl_function(args),
            _ => None,
        },
        _ => None,
    }
}

fn hex_color(hex: &str) -> Option<Color> {
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).ok().map(|d| d * 17);
    let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let (r, g, b, a) = match hex.len() {
        3 => (digit(0)?, digit(1)?, digit(2)?, 255),
        4 => (digit(0)?, digit(1)?, digit(2)?, digit(3)?),
        6 => (pair(0)?, pair(2)?, pair(4)?, 255),
        8 => (pair(0)?, pair(2)?, pair(4)?, pair(6)?),
        _ => return None,
    };
    Some(Color {
        r,
        g,
        b,
        a: f32::from(a) / 255.0,
    })
}

/// A colour function's arguments: three channels and an optional alpha, separated by commas or
/// by spaces with a `/` before the alpha.
fn channels(args: &str) -> Option<(Vec<Comp>, Option<f32>)> {
    let comps = components(args)?;
    let mut parts: Vec<Comp> = comps
        .iter()
        .filter(|c| **c != Comp::Comma)
        .cloned()
        .collect();
    let mut alpha = None;
    if let Some(at) = parts.iter().position(|c| *c == Comp::Slash) {
        if at != 3 || parts.len() != 5 {
            return None;
        }
        alpha = Some(alpha_of(&parts[4])?);
        parts.truncate(3);
    } else if parts.len() == 4 {
        alpha = Some(alpha_of(&parts[3])?);
        parts.truncate(3);
    }
    (parts.len() == 3).then_some((parts, alpha))
}

fn alpha_of(c: &Comp) -> Option<f32> {
    match c {
        Comp::Number(n, _) => Some(n.clamp(0.0, 1.0)),
        Comp::Percentage(p) => Some((p / 100.0).clamp(0.0, 1.0)),
        _ => None,
    }
}

fn rgb_function(args: &str) -> Option<Color> {
    let (parts, alpha) = channels(args)?;
    let channel = |c: &Comp| match c {
        Comp::Number(n, _) => Some(n.clamp(0.0, 255.0).round() as u8),
        Comp::Percentage(p) => Some((p.clamp(0.0, 100.0) * 2.55).round() as u8),
        _ => None,
    };
    Some(Color {
        r: channel(&parts[0])?,
        g: channel(&parts[1])?,
        b: channel(&parts[2])?,
        a: alpha.unwrap_or(1.0),
    })
}

fn hsl_function(args: &str) -> Option<Color> {
    let (parts, alpha) = channels(args)?;
    let hue = match &parts[0] {
        Comp::Number(n, _) => *n,
        Comp::Dimension(n, unit) if unit == "deg" => *n,
        Comp::Dimension(n, unit) if unit == "turn" => n * 360.0,
        _ => return None,
    };
    let percent = |c: &Comp| match c {
        Comp::Percentage(p) => Some(p.clamp(0.0, 100.0) / 100.0),
        Comp::Number(n, _) => Some(n.clamp(0.0, 100.0) / 100.0),
        _ => None,
    };
    let (s, l) = (percent(&parts[1])?, percent(&parts[2])?);
    let h = hue.rem_euclid(360.0) / 30.0;
    let a = s * l.min(1.0 - l);
    let f = |n: f32| {
        let k = (n + h) % 12.0;
        let v = l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (v * 255.0).round() as u8
    };
    Some(Color {
        r: f(0.0),
        g: f(8.0),
        b: f(4.0),
        a: alpha.unwrap_or(1.0),
    })
}

/// The CSS named colours, sorted by name for the binary search.
static NAMED: &[(&str, u32)] = &[
    ("aliceblue", 0xf0f8ff),
    ("antiquewhite", 0xfaebd7),
    ("aqua", 0x00ffff),
    ("aquamarine", 0x7fffd4),
    ("azure", 0xf0ffff),
    ("beige", 0xf5f5dc),
    ("bisque", 0xffe4c4),
    ("black", 0x000000),
    ("blanchedalmond", 0xffebcd),
    ("blue", 0x0000ff),
    ("blueviolet", 0x8a2be2),
    ("brown", 0xa52a2a),
    ("burlywood", 0xdeb887),
    ("cadetblue", 0x5f9ea0),
    ("chartreuse", 0x7fff00),
    ("chocolate", 0xd2691e),
    ("coral", 0xff7f50),
    ("cornflowerblue", 0x6495ed),
    ("cornsilk", 0xfff8dc),
    ("crimson", 0xdc143c),
    ("cyan", 0x00ffff),
    ("darkblue", 0x00008b),
    ("darkcyan", 0x008b8b),
    ("darkgoldenrod", 0xb8860b),
    ("darkgray", 0xa9a9a9),
    ("darkgreen", 0x006400),
    ("darkgrey", 0xa9a9a9),
    ("darkkhaki", 0xbdb76b),
    ("darkmagenta", 0x8b008b),
    ("darkolivegreen", 0x556b2f),
    ("darkorange", 0xff8c00),
    ("darkorchid", 0x9932cc),
    ("darkred", 0x8b0000),
    ("darksalmon", 0xe9967a),
    ("darkseagreen", 0x8fbc8f),
    ("darkslateblue", 0x483d8b),
    ("darkslategray", 0x2f4f4f),
    ("darkslategrey", 0x2f4f4f),
    ("darkturquoise", 0x00ced1),
    ("darkviolet", 0x9400d3),
    ("deeppink", 0xff1493),
    ("deepskyblue", 0x00bfff),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("dodgerblue", 0x1e90ff),
    ("firebrick", 0xb22222),
    ("floralwhite", 0xfffaf0),
    ("forestgreen", 0x228b22),
    ("fuchsia", 0xff00ff),
    ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff),
    ("gold", 0xffd700),
    ("goldenrod", 0xdaa520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xadff2f),
    ("grey", 0x808080),
    ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4),
    ("indianred", 0xcd5c5c),
    ("indigo", 0x4b0082),
    ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c),
    ("lavender", 0xe6e6fa),
    ("lavenderblush", 0xfff0f5),
    ("lawngreen", 0x7cfc00),
    ("lemonchiffon", 0xfffacd),
    ("lightblue", 0xadd8e6),
    ("lightcoral", 0xf08080),
    ("lightcyan", 0xe0ffff),
    ("lightgoldenrodyellow", 0xfafad2),
    ("lightgray", 0xd3d3d3),
    ("lightgreen", 0x90ee90),
    ("lightgrey", 0xd3d3d3),
    ("lightpink", 0xffb6c1),
    ("lightsalmon", 0xffa07a),
    ("lightseagreen", 0x20b2aa),
    ("lightskyblue", 0x87cefa),
    ("lightslategray", 0x778899),
    ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xb0c4de),
    ("lightyellow", 0xffffe0),
    ("lime", 0x00ff00),
    ("limegreen", 0x32cd32),
    ("linen", 0xfaf0e6),
    ("magenta", 0xff00ff),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa),
    ("mediumblue", 0x0000cd),
    ("mediumorchid", 0xba55d3),
    ("mediumpurple", 0x9370db),
    ("mediumseagreen", 0x3cb371),
    ("mediumslateblue", 0x7b68ee),
    ("mediumspringgreen", 0x00fa9a),
    ("mediumturquoise", 0x48d1cc),
    ("mediumvioletred", 0xc71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1),
    ("moccasin", 0xffe4b5),
    ("navajowhite", 0xffdead),
    ("navy", 0x000080),
    ("oldlace", 0xfdf5e6),
    ("olive", 0x808000),
    ("olivedrab", 0x6b8e23),
    ("orange", 0xffa500),
    ("orangered", 0xff4500),
    ("orchid", 0xda70d6),
    ("palegoldenrod", 0xeee8aa),
    ("palegreen", 0x98fb98),
    ("paleturquoise", 0xafeeee),
    ("palevioletred", 0xdb7093),
    ("papayawhip", 0xffefd5),
    ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f),
    ("pink", 0xffc0cb),
    ("plum", 0xdda0dd),
    ("powderblue", 0xb0e0e6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xff0000),
    ("rosybrown", 0xbc8f8f),
    ("royalblue", 0x4169e1),
    ("saddlebrown", 0x8b4513),
    ("salmon", 0xfa8072),
    ("sandybrown", 0xf4a460),
    ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee),
    ("sienna", 0xa0522d),
    ("silver", 0xc0c0c0),
    ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd),
    ("slategray", 0x708090),
    ("slategrey", 0x708090),
    ("snow", 0xfffafa),
    ("springgreen", 0x00ff7f),
    ("steelblue", 0x4682b4),
    ("tan", 0xd2b48c),
    ("teal", 0x008080),
    ("thistle", 0xd8bfd8),
    ("tomato", 0xff6347),
    ("turquoise", 0x40e0d0),
    ("violet", 0xee82ee),
    ("wheat", 0xf5deb3),
    ("white", 0xffffff),
    ("whitesmoke", 0xf5f5f5),
    ("yellow", 0xffff00),
    ("yellowgreen", 0x9acd32),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_named_colours_are_sorted() {
        assert!(NAMED.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn a_var_inside_calc_is_substituted_in_place() {
        assert!(has_var("calc(var(--gap) + 1pt)"));
        let lookup = |name: &str| (name == "--gap").then(|| "4pt".to_owned());
        assert_eq!(
            substitute("calc(var(--gap) + 1pt)", &lookup).as_deref(),
            Some("calc(4pt + 1pt)")
        );
    }

    #[test]
    fn calc_folds_units_and_keeps_a_percentage() {
        let sum = length(&Comp::Function(
            "calc".into(),
            "100% - 2 * 1em + 3pt".into(),
        ))
        .unwrap();
        assert_eq!(sum.percent, 100.0);
        assert_eq!(sum.em, -2.0);
        assert_eq!(sum.pt, 3.0);
    }
}
