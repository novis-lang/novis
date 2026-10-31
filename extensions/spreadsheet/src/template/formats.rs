//! The template's cell styles, read once so a date, a time or a duration filled into a cell
//! whose style shows none gets the write table's format (`rule:core-classes/spreadsheet-template-fill`).
//!
//! [`Formats::read`] keeps `styles.xml` as text, with the byte range of each `cellXfs` entry and
//! whether its number format shows a date or a time. [`Formats::style`] gives such a value a new
//! entry: a copy of the cell's own with the format [`crate::styles::Implicit`] names, one for each style and
//! format pair. [`Formats::rewritten`] writes the part with those entries and their formats added,
//! and every other byte as it was. A template with no `styles.xml` writes the value as its serial
//! number in the cell's own style.
//!
//! What it spends, per filling request: the text of `styles.xml` and one entry per `cellXfs`
//! entry, from `Template::open` to `finish`.

use std::fmt::Write as _;
use std::ops::Range;

use crate::Error;
use crate::Value;
use crate::format;
use crate::read::xml::{self, Token};

/// One `cellXfs` entry: its element's bytes and whether its format shows a date or a time.
#[derive(Debug, Default, Clone)]
struct Xf {
    at: Range<usize>,
    dated: bool,
}

/// The styles part of a template, and the entries the fill adds to it.
#[derive(Debug, Default)]
pub struct Formats {
    text: String,
    /// The root's prefix, such as `x:`, for an element the fill adds.
    prefix: String,
    /// Where the root's start tag ends, the `cellXfs` start tag and its end tag, and the
    /// `numFmts` start tag and its end tag, if the part has one.
    root_end: usize,
    xfs_open: Range<usize>,
    xfs_close: usize,
    fmts_open: Option<Range<usize>>,
    fmts_close: Option<usize>,
    xfs: Vec<Xf>,
    codes: Vec<(u32, String)>,
    /// Each added entry: the entry it copies and its format's id.
    added: Vec<(usize, u32)>,
    /// Each added format: its id and its code.
    new_codes: Vec<(u32, &'static str)>,
}

fn number(bytes: Option<&[u8]>) -> Option<u32> {
    std::str::from_utf8(bytes?).ok()?.parse().ok()
}

/// Whether the format code shows a date or a time in any of its sections.
fn shows_date(code: &str) -> bool {
    format::split(code)
        .into_iter()
        .any(|section| format::lex(section).is_date())
}

/// `tag`, a start tag, with the attribute `name` removed.
fn without_attr(tag: &str, name: &str) -> String {
    for quote in ['"', '\''] {
        let needle = format!("{name}={quote}");
        let mut from = 0;
        while let Some(found) = tag[from..].find(&needle) {
            let at = from + found;
            if at > 0 && tag.as_bytes()[at - 1].is_ascii_whitespace() {
                let value = at + needle.len();
                let end = tag[value..]
                    .find(quote)
                    .map_or(tag.len(), |end| value + end + 1);
                return format!("{}{}", &tag[..at - 1], &tag[end..]);
            }
            from = at + needle.len();
        }
    }
    tag.to_owned()
}

/// `text` with the `count` attribute of the start tag `open` set to `count`.
fn counted(text: &str, open: &Range<usize>, count: usize) -> String {
    let tag = &text[open.clone()];
    let end = tag.len() - if tag.ends_with("/>") { 2 } else { 1 };
    let bare = without_attr(&tag[..end], "count");
    format!("{bare} count=\"{count}\"{}", &tag[end..])
}

impl Formats {
    /// Reads the styles part `text`.
    pub fn read(text: String) -> Result<Self, Error> {
        let mut formats = Self::default();
        let bytes = text.as_bytes();
        let (mut at, mut in_xfs, mut root) = (0, false, true);
        let mut open_xf: Option<(usize, u32)> = None;
        let mut ids = Vec::new();
        while let Some((token, length)) = xml::next(&bytes[at..])? {
            let start = at;
            at += length;
            match token {
                Token::Open { name, .. } if root => {
                    root = false;
                    let name = String::from_utf8_lossy(name);
                    formats.prefix = name.rsplit_once(':').map_or("", |(p, _)| p).to_owned();
                    if !formats.prefix.is_empty() {
                        formats.prefix.push(':');
                    }
                    formats.root_end = at;
                }
                Token::Open { name, attrs, empty } => match xml::local(name) {
                    b"numFmts" => {
                        formats.fmts_open = Some(start..at);
                        if empty {
                            formats.fmts_close = Some(at);
                        }
                    }
                    b"numFmt" => {
                        if let (Some(id), Some(code)) = (
                            number(xml::attr(attrs, b"numFmtId")),
                            xml::attr_text(attrs, b"formatCode")?,
                        ) {
                            formats.codes.push((id, code));
                        }
                    }
                    b"cellXfs" => {
                        formats.xfs_open = start..at;
                        formats.xfs_close = at;
                        in_xfs = !empty;
                    }
                    b"xf" if in_xfs => {
                        let id = number(xml::attr(attrs, b"numFmtId")).unwrap_or(0);
                        if empty {
                            formats.xfs.push(Xf {
                                at: start..at,
                                dated: false,
                            });
                            ids.push(id);
                        } else {
                            open_xf = Some((start, id));
                        }
                    }
                    _ => {}
                },
                Token::Close(name) => match xml::local(name) {
                    b"numFmts" => formats.fmts_close = Some(start),
                    b"cellXfs" => {
                        formats.xfs_close = start;
                        in_xfs = false;
                    }
                    b"xf" => {
                        if let Some((from, id)) = open_xf.take() {
                            formats.xfs.push(Xf {
                                at: from..at,
                                dated: false,
                            });
                            ids.push(id);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        for (xf, id) in formats.xfs.iter_mut().zip(ids) {
            let custom = formats.codes.iter().find(|(code, _)| *code == id);
            xf.dated = match custom {
                Some((_, code)) => shows_date(code),
                None => format::builtin(id).is_some_and(shows_date),
            };
        }
        formats.text = text;
        Ok(formats)
    }

    /// The style a cell with the style `style` is written with when it holds `value`: a new
    /// entry when the value is a date, a time or a duration and the style shows none, or else
    /// `None` for the cell's own.
    pub fn style(&mut self, style: Option<&str>, value: Option<&Value>) -> Option<String> {
        let implicit = crate::cells::stored_number(value?)?.1;
        let code = implicit.code()?;
        let base = style.map_or(Some(0), |style| style.parse::<usize>().ok())?;
        if self.xfs.get(base)?.dated {
            return None;
        }
        let id = match self.codes.iter().find(|(_, existing)| existing == code) {
            Some((id, _)) => *id,
            None => match self.new_codes.iter().find(|(_, added)| *added == code) {
                Some((id, _)) => *id,
                None => {
                    let highest = self
                        .codes
                        .iter()
                        .map(|(id, _)| *id)
                        .chain(self.new_codes.iter().map(|(id, _)| *id))
                        .max()
                        .unwrap_or(0);
                    let id = highest.max(163) + 1;
                    self.new_codes.push((id, code));
                    id
                }
            },
        };
        let index = match self.added.iter().position(|added| *added == (base, id)) {
            Some(at) => at,
            None => {
                self.added.push((base, id));
                self.added.len() - 1
            }
        };
        Some((self.xfs.len() + index).to_string())
    }

    /// The styles part with the added entries and formats, or `None` when the fill added none.
    pub fn rewritten(&self) -> Option<String> {
        if self.added.is_empty() {
            return None;
        }
        let text = &self.text;
        let mut entries = String::new();
        for (base, id) in &self.added {
            let element = &text[self.xfs[*base].at.clone()];
            let end = element.find('>').unwrap_or(element.len());
            let end = if element[..end].ends_with('/') {
                end - 1
            } else {
                end
            };
            let tag = without_attr(
                &without_attr(&element[..end], "numFmtId"),
                "applyNumberFormat",
            );
            let _ = write!(
                entries,
                "{tag} numFmtId=\"{id}\" applyNumberFormat=\"1\"{}",
                &element[end..]
            );
        }
        let mut codes = String::new();
        for (id, code) in &self.new_codes {
            let _ = write!(
                codes,
                "<{}numFmt numFmtId=\"{id}\" formatCode=\"",
                self.prefix
            );
            crate::xml::attr(&mut codes, code);
            codes.push_str("\"/>");
        }
        let xfs = self.xfs.len() + self.added.len();
        let fmts = self.codes.len() + self.new_codes.len();
        let mut out = String::with_capacity(text.len() + entries.len() + codes.len() + 64);
        // The pieces of the part in order: each is a range of `text` and what replaces it.
        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        match (&self.fmts_open, self.fmts_close) {
            (Some(open), Some(close))
                if close == open.end && text[open.clone()].ends_with("/>") =>
            {
                let tag = counted(text, open, fmts);
                let tag = tag.trim_end_matches("/>");
                edits.push((
                    open.clone(),
                    format!("{tag}>{codes}</{}numFmts>", self.prefix),
                ));
            }
            (Some(open), Some(close)) => {
                edits.push((open.clone(), counted(text, open, fmts)));
                edits.push((close..close, codes));
            }
            _ => edits.push((
                self.root_end..self.root_end,
                format!(
                    "<{p}numFmts count=\"{fmts}\">{codes}</{p}numFmts>",
                    p = self.prefix
                ),
            )),
        }
        edits.push((self.xfs_open.clone(), counted(text, &self.xfs_open, xfs)));
        edits.push((self.xfs_close..self.xfs_close, entries));
        edits.sort_by_key(|(range, _)| (range.start, range.end));
        let mut copied = 0;
        for (range, replacement) in edits {
            out.push_str(&text[copied..range.start]);
            out.push_str(&replacement);
            copied = range.end;
        }
        out.push_str(&text[copied..]);
        Some(out)
    }
}
