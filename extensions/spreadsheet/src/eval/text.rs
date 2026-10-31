//! The text functions of the table (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! A position and a length count UTF-16 code units, as Excel does, so `LEN` of an emoji is 2. A
//! cut that would split a surrogate pair is listed, never written as broken text. A result longer
//! than Excel's longest text, 32,767 units, is `#VALUE!`. `CHAR` and `CODE` read code page 1252,
//! the one Excel on Windows uses. Upper and lower case map one character to one character, and a
//! character whose mapping is longer, such as `ß` in upper case, is listed.
//!
//! A text that `VALUE` or `TEXT` cannot read as a number but which has a digit in it is listed,
//! because Excel reads forms such as `1,000` and `1/15/2024` by the machine's locale. An ISO date
//! in a 1904 workbook is listed too, because the shared coercion counts it from 1900.
//!
//! What it spends: the text each call returns and, while it runs, its arguments as UTF-16 units.
//! `CONCAT` and `TEXTJOIN` stop growing their text once it is past the limit, so a whole column
//! costs a walk of its cells and never more than the limit in text.

use super::functions::{arity, error, listed, number, value};
use super::parse::Expr;
use super::value::{Value, number_of_text};
use super::{Ctx, Miss, Operand};
use crate::format::{self, Locale, Options, Section};
use crate::read::CellError;

/// Excel's longest text, in UTF-16 units.
const LIMIT: usize = 32_767;

/// The most cells `TEXTJOIN` walks one by one when it keeps the blanks.
const WALK: u64 = 1_000_000;

/// Code page 1252 from `0x80` to `0x9F`. The five bytes it leaves undefined stand for the code
/// point of the same number, as Windows maps them.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}', '\u{178}',
];

/// The value of a coercion, or its error returned as the function's result.
macro_rules! take {
    ($coerced:expr) => {
        match $coerced {
            Ok(v) => v,
            Err(e) => return error(e),
        }
    };
}

fn text_of(ctx: &mut Ctx<'_>, arg: &Expr) -> Result<Result<String, CellError>, Miss> {
    Ok(ctx.scalar(arg)?.text())
}

fn number_of(ctx: &mut Ctx<'_>, arg: &Expr) -> Result<Result<f64, CellError>, Miss> {
    Ok(ctx.scalar(arg)?.number())
}

/// The number argument at `at`, or `default` when it is not given.
fn optional(
    ctx: &mut Ctx<'_>,
    args: &[Expr],
    at: usize,
    default: f64,
) -> Result<Result<f64, CellError>, Miss> {
    match args.get(at) {
        None => Ok(Ok(default)),
        Some(arg) => number_of(ctx, arg),
    }
}

/// A count argument cut to a whole number, or `None` when it is negative.
fn whole(n: f64) -> Option<usize> {
    let n = n.trunc();
    (n >= 0.0).then_some(n.min(1e15) as usize)
}

fn units(t: &str) -> Vec<u16> {
    t.encode_utf16().collect()
}

fn len16(t: &str) -> usize {
    t.encode_utf16().count()
}

/// A text result, `#VALUE!` past the limit.
fn checked(t: String) -> Result<Operand, Miss> {
    if len16(&t) > LIMIT {
        error(CellError::Value)
    } else {
        value(Value::Text(t))
    }
}

/// `u` as text, listed when a cut split a surrogate pair.
fn cut(name: &str, u: &[u16]) -> Result<Operand, Miss> {
    match String::from_utf16(u) {
        Ok(t) => checked(t),
        Err(_) => Err(listed(name)),
    }
}

/// The one character a case mapping gives, or `None` when it gives more.
fn single(mut mapped: impl Iterator<Item = char>) -> Option<char> {
    let c = mapped.next()?;
    mapped.next().is_none().then_some(c)
}

/// The number a text reads as for `VALUE` and `TEXT`, `None` for a text with no digit, and
/// listed where Excel may read it otherwise.
fn serial(ctx: &Ctx<'_>, name: &str, t: &str) -> Result<Option<f64>, Miss> {
    match number_of_text(t) {
        Some(_) if ctx.date1904() && iso_day(t) => Err(listed(name)),
        Some(n) => Ok(Some(n)),
        None if t.bytes().any(|b| b.is_ascii_digit()) => Err(listed(name)),
        None => Ok(None),
    }
}

/// Whether a text that reads as a number starts with an ISO date.
fn iso_day(t: &str) -> bool {
    let day = t.trim().split(' ').next().unwrap_or("").as_bytes();
    day.len() >= 5 && day[4] == b'-' && day[..4].iter().all(u8::is_ascii_digit)
}

/// Text joined from the pieces of `CONCAT`, `CONCATENATE` and `TEXTJOIN`. It stops growing
/// once it is past the limit, and still counts that it is.
struct Join {
    out: String,
    units: usize,
    started: bool,
}

impl Join {
    fn new() -> Self {
        Self {
            out: String::new(),
            units: 0,
            started: false,
        }
    }

    fn push(&mut self, piece: &str) {
        if self.units <= LIMIT {
            self.units += len16(piece);
            self.out.push_str(piece);
        }
    }

    /// Adds `v` after `delimiter`, and skips an empty piece when `skip_empty` is set.
    fn add(&mut self, delimiter: &str, skip_empty: bool, v: &Value) -> Result<(), CellError> {
        let piece = v.text()?;
        if skip_empty && piece.is_empty() {
            return Ok(());
        }
        if self.started {
            self.push(delimiter);
        }
        self.started = true;
        self.push(&piece);
        Ok(())
    }

    /// The result of an error met while joining. Past the limit, whether Excel gives the error
    /// or `#VALUE!` is not known, so the call is listed.
    fn fail(&self, name: &str, e: CellError) -> Result<Operand, Miss> {
        if self.units > LIMIT {
            Err(listed(name))
        } else {
            error(e)
        }
    }

    fn finish(self) -> Result<Operand, Miss> {
        if self.units > LIMIT {
            error(CellError::Value)
        } else {
            value(Value::Text(self.out))
        }
    }
}

/// `CHAR`: the character of a code page 1252 byte, 1 to 255.
pub(super) fn char_(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("CHAR", args, 1, 1)?;
    let n = take!(number_of(ctx, &args[0])?).trunc();
    if !(1.0..=255.0).contains(&n) {
        return error(CellError::Value);
    }
    let byte = n as u8;
    let c = match byte {
        0x80..=0x9F => HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    };
    value(Value::Text(c.to_string()))
}

/// `CODE`: the code page 1252 byte of the first character. A character outside that code page
/// is listed.
pub(super) fn code(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("CODE", args, 1, 1)?;
    let t = take!(text_of(ctx, &args[0])?);
    let Some(c) = t.chars().next() else {
        return error(CellError::Value);
    };
    let u = u32::from(c);
    let byte = if (1..0x80).contains(&u) || (0xA0..=0xFF).contains(&u) {
        Some(u)
    } else {
        HIGH.iter().position(|&h| h == c).map(|at| at as u32 + 0x80)
    };
    match byte {
        Some(b) => number(f64::from(b)),
        None => Err(listed("CODE")),
    }
}

/// `CLEAN`: the text without the characters 0 to 31.
pub(super) fn clean(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("CLEAN", args, 1, 1)?;
    let t = take!(text_of(ctx, &args[0])?);
    value(Value::Text(
        t.chars().filter(|&c| u32::from(c) >= 32).collect(),
    ))
}

/// `CONCAT`: every argument and every non-blank cell of a reference, by row and then column.
pub(super) fn concat(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("CONCAT", args, 1, 253)?;
    let mut join = Join::new();
    for arg in args {
        match ctx.eval(arg)? {
            Operand::Area(sheet, rect) => {
                for (_, _, cell) in ctx.entries(sheet, rect) {
                    if let Err(e) = join.add("", false, cell) {
                        return join.fail("CONCAT", e);
                    }
                }
            }
            Operand::Value(v) => {
                if let Err(e) = join.add("", false, &v) {
                    return join.fail("CONCAT", e);
                }
            }
        }
    }
    join.finish()
}

pub(super) fn concatenate(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("CONCATENATE", args, 1, 255)?;
    let mut join = Join::new();
    for arg in args {
        let v = ctx.scalar(arg)?;
        if let Err(e) = join.add("", false, &v) {
            return join.fail("CONCATENATE", e);
        }
    }
    join.finish()
}

/// `TEXTJOIN`: the pieces with the delimiter between them. When empty pieces are kept, a blank
/// cell of a reference is one, so the reference is walked cell by cell, and one of more than a
/// million cells is listed.
pub(super) fn textjoin(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("TEXTJOIN", args, 3, 254)?;
    let delimiter = take!(text_of(ctx, &args[0])?);
    let skip = take!(ctx.scalar(&args[1])?.boolean());
    let mut join = Join::new();
    for arg in &args[2..] {
        match ctx.eval(arg)? {
            Operand::Area(sheet, rect) if skip => {
                for (_, _, cell) in ctx.entries(sheet, rect) {
                    if let Err(e) = join.add(&delimiter, true, cell) {
                        return join.fail("TEXTJOIN", e);
                    }
                }
            }
            Operand::Area(sheet, rect) => {
                let cells =
                    u64::from(rect.bottom - rect.top + 1) * u64::from(rect.right - rect.left + 1);
                if cells > WALK {
                    return Err(listed("TEXTJOIN"));
                }
                for row in rect.top..=rect.bottom {
                    for col in rect.left..=rect.right {
                        if let Err(e) = join.add(&delimiter, false, ctx.at(sheet, row, col)) {
                            return join.fail("TEXTJOIN", e);
                        }
                    }
                }
            }
            Operand::Value(v) => {
                if let Err(e) = join.add(&delimiter, skip, &v) {
                    return join.fail("TEXTJOIN", e);
                }
            }
        }
    }
    join.finish()
}

/// `EXACT`: whether two texts are equal, with case.
pub(super) fn exact(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("EXACT", args, 2, 2)?;
    let a = take!(text_of(ctx, &args[0])?);
    let b = take!(text_of(ctx, &args[1])?);
    value(Value::Bool(a == b))
}

/// One part of a `SEARCH` pattern: a code unit, `?` or `*`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Unit(u16),
    One,
    Star,
}

/// `c` in lower case for `SEARCH`, kept when its lower case is not one character of the same
/// width, so positions do not move.
fn fold(c: char) -> char {
    match single(c.to_lowercase()) {
        Some(l) if l.len_utf16() == c.len_utf16() => l,
        _ => c,
    }
}

/// The parts of a `SEARCH` pattern. `~` before `*`, `?` or `~` makes it plain; a `~` before
/// anything else, or at the end, is listed.
fn pattern(needle: &str) -> Result<Vec<Part>, Miss> {
    let mut out = Vec::new();
    let mut chars = needle.chars().map(fold);
    let mut buf = [0u16; 2];
    while let Some(c) = chars.next() {
        let plain = match c {
            '*' => {
                out.push(Part::Star);
                continue;
            }
            '?' => {
                out.push(Part::One);
                continue;
            }
            '~' => match chars.next() {
                Some(next @ ('*' | '?' | '~')) => next,
                _ => return Err(listed("SEARCH")),
            },
            _ => c,
        };
        out.extend(plain.encode_utf16(&mut buf).iter().map(|&u| Part::Unit(u)));
    }
    Ok(out)
}

/// Whether `run`, which has no `*`, matches `hay` at `at`.
fn matches_at(run: &[Part], hay: &[u16], at: usize) -> bool {
    hay.len() - at >= run.len()
        && run.iter().zip(&hay[at..]).all(|(part, &u)| match part {
            Part::Unit(c) => *c == u,
            Part::One => true,
            Part::Star => false,
        })
}

/// The first position from `from` where `parts` matches a prefix of the rest of `hay`. The
/// first run between stars is placed leftmost, and each later run leftmost after the one before.
/// A later start only moves the later runs right, so the leftmost first run is the answer when
/// any start is.
fn first_match(parts: &[Part], hay: &[u16], from: usize) -> Option<usize> {
    let mut runs = parts.split(|p| *p == Part::Star);
    let first = runs.next().unwrap_or(&[]);
    let begin = (from..=hay.len()).find(|&at| matches_at(first, hay, at))?;
    let mut next = begin + first.len();
    for run in runs.filter(|r| !r.is_empty()) {
        let at = (next..=hay.len()).find(|&at| matches_at(run, hay, at))?;
        next = at + run.len();
    }
    Some(begin)
}

/// `FIND` and `SEARCH`: the 1-based position of the first match from the start.
fn locate(ctx: &mut Ctx<'_>, name: &str, args: &[Expr], search: bool) -> Result<Operand, Miss> {
    arity(name, args, 2, 3)?;
    let needle = take!(text_of(ctx, &args[0])?);
    let within = take!(text_of(ctx, &args[1])?);
    let start = take!(optional(ctx, args, 2, 1.0)?).trunc();
    let hay: Vec<u16> = if search {
        within
            .chars()
            .map(fold)
            .collect::<String>()
            .encode_utf16()
            .collect()
    } else {
        units(&within)
    };
    if start < 1.0 || start > hay.len() as f64 + 1.0 {
        return error(CellError::Value);
    }
    let from = start as usize - 1;
    let found = if search {
        let parts = pattern(&needle)?;
        if parts.contains(&Part::One) && hay.iter().any(|u| (0xD800..=0xDFFF).contains(u)) {
            // Whether `?` matches a character or one half of a surrogate pair is not known.
            return Err(listed(name));
        }
        if from == hay.len() && parts.iter().all(|p| *p == Part::Star) {
            return Err(listed(name));
        }
        first_match(&parts, &hay, from)
    } else {
        let needle = units(&needle);
        if needle.is_empty() {
            // Excel's documentation and its behaviour may differ for an empty text at the end.
            if from == hay.len() {
                return Err(listed(name));
            }
            Some(from)
        } else {
            hay[from..]
                .windows(needle.len())
                .position(|w| w == needle.as_slice())
                .map(|at| at + from)
        }
    };
    match found {
        Some(at) => number((at + 1) as f64),
        None => error(CellError::Value),
    }
}

/// `FIND`: with case and without wildcards.
pub(super) fn find(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    locate(ctx, "FIND", args, false)
}

/// `SEARCH`: without case and with the wildcards `*`, `?` and `~`.
pub(super) fn search(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    locate(ctx, "SEARCH", args, true)
}

pub(super) fn left(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("LEFT", args, 1, 2)?;
    let t = take!(text_of(ctx, &args[0])?);
    let Some(n) = whole(take!(optional(ctx, args, 1, 1.0)?)) else {
        return error(CellError::Value);
    };
    let u = units(&t);
    cut("LEFT", &u[..n.min(u.len())])
}

pub(super) fn right(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("RIGHT", args, 1, 2)?;
    let t = take!(text_of(ctx, &args[0])?);
    let Some(n) = whole(take!(optional(ctx, args, 1, 1.0)?)) else {
        return error(CellError::Value);
    };
    let u = units(&t);
    cut("RIGHT", &u[u.len() - n.min(u.len())..])
}

/// `MID`: a start past the end gives `""`.
pub(super) fn mid(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("MID", args, 3, 3)?;
    let t = take!(text_of(ctx, &args[0])?);
    let start = take!(number_of(ctx, &args[1])?).trunc();
    let n = take!(number_of(ctx, &args[2])?);
    if start < 1.0 {
        return error(CellError::Value);
    }
    let Some(n) = whole(n) else {
        return error(CellError::Value);
    };
    let u = units(&t);
    let from = (start.min(1e15) as usize - 1).min(u.len());
    let to = from.saturating_add(n).min(u.len());
    cut("MID", &u[from..to])
}

pub(super) fn len(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("LEN", args, 1, 1)?;
    let t = take!(text_of(ctx, &args[0])?);
    number(len16(&t) as f64)
}

/// Each character of `t` in upper case where `upper` says so and in lower case elsewhere.
fn recase(name: &str, t: &str, mut upper: impl FnMut(char) -> bool) -> Result<Operand, Miss> {
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        let mapped = if upper(c) {
            single(c.to_uppercase())
        } else {
            single(c.to_lowercase())
        };
        out.push(mapped.ok_or_else(|| listed(name))?);
    }
    value(Value::Text(out))
}

pub(super) fn lower(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("LOWER", args, 1, 1)?;
    let t = take!(text_of(ctx, &args[0])?);
    recase("LOWER", &t, |_| false)
}

pub(super) fn upper(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("UPPER", args, 1, 1)?;
    let t = take!(text_of(ctx, &args[0])?);
    recase("UPPER", &t, |_| true)
}

/// `PROPER`: a letter after a non-letter in upper case and every other letter in lower case. A
/// digit is a non-letter, so `2nd` gives `2Nd`.
pub(super) fn proper(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("PROPER", args, 1, 1)?;
    let t = take!(text_of(ctx, &args[0])?);
    let mut after_letter = false;
    recase("PROPER", &t, |c| {
        let up = !after_letter;
        after_letter = c.is_alphabetic();
        up
    })
}

/// `REPLACE`: the units from the start, as many as the count, replaced by the new text. A start
/// past the end appends it.
pub(super) fn replace(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("REPLACE", args, 4, 4)?;
    let old = take!(text_of(ctx, &args[0])?);
    let start = take!(number_of(ctx, &args[1])?).trunc();
    let n = take!(number_of(ctx, &args[2])?);
    let new = take!(text_of(ctx, &args[3])?);
    if start < 1.0 {
        return error(CellError::Value);
    }
    let Some(n) = whole(n) else {
        return error(CellError::Value);
    };
    let u = units(&old);
    let from = (start.min(1e15) as usize - 1).min(u.len());
    let to = from.saturating_add(n).min(u.len());
    let mut out = u[..from].to_vec();
    out.extend(new.encode_utf16());
    out.extend_from_slice(&u[to..]);
    cut("REPLACE", &out)
}

pub(super) fn rept(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("REPT", args, 2, 2)?;
    let t = take!(text_of(ctx, &args[0])?);
    let Some(n) = whole(take!(number_of(ctx, &args[1])?)) else {
        return error(CellError::Value);
    };
    if t.is_empty() || n == 0 {
        return value(Value::Text(String::new()));
    }
    if len16(&t).saturating_mul(n) > LIMIT {
        return error(CellError::Value);
    }
    value(Value::Text(t.repeat(n)))
}

/// `SUBSTITUTE`: every match of the old text, or only the one the instance counts, replaced.
/// An empty old text leaves the text as it is.
pub(super) fn substitute(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("SUBSTITUTE", args, 3, 4)?;
    let t = take!(text_of(ctx, &args[0])?);
    let old = take!(text_of(ctx, &args[1])?);
    let new = take!(text_of(ctx, &args[2])?);
    let instance = match args.get(3) {
        None => None,
        Some(arg) => {
            let n = take!(number_of(ctx, arg)?).trunc();
            if n < 1.0 {
                return error(CellError::Value);
            }
            Some(n.min(1e15) as usize)
        }
    };
    if old.is_empty() {
        return value(Value::Text(t));
    }
    let out = match instance {
        None => t.replace(&old, &new),
        Some(n) => {
            let found = t.match_indices(old.as_str()).nth(n - 1).map(|(at, _)| at);
            match found {
                Some(at) => format!("{}{}{}", &t[..at], new, &t[at + old.len()..]),
                None => t,
            }
        }
    };
    checked(out)
}

/// Whether the section `code` takes for `n` shows a date that `n` has none of, where Excel's
/// `TEXT` may give an error.
fn dateless(code: &str, n: f64, options: &Options) -> bool {
    let sections: Vec<Section> = format::split(code).into_iter().map(format::lex).collect();
    if sections.is_empty() {
        return false;
    }
    let (at, _) = format::choose(&sections, n);
    let Some(section) = sections.get(at) else {
        return false;
    };
    let locale = section
        .locale
        .and_then(Locale::of_id)
        .unwrap_or(options.locale);
    section.is_date() && format::date::render(section, n, options.date1904, locale).is_none()
}

/// `TEXT`: the value as its format code shows it in English. A text that reads as a number is
/// shown as that number, and other text by the code's text section.
pub(super) fn text(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("TEXT", args, 2, 2)?;
    let shown = ctx.scalar(&args[0])?;
    if let Value::Error(e) = shown {
        return error(e);
    }
    let code = take!(text_of(ctx, &args[1])?);
    let options = Options {
        date1904: ctx.date1904(),
        locale: Locale::EN,
    };
    let n = match &shown {
        Value::Bool(b) => {
            return checked(format::render(&code, format::Value::Boolean(*b), &options));
        }
        Value::Text(t) => match serial(ctx, "TEXT", t)? {
            Some(n) => n,
            None => return checked(format::render(&code, format::Value::Text(t), &options)),
        },
        Value::Number(n) => *n,
        Value::Blank | Value::Error(_) => 0.0,
    };
    if dateless(&code, n, &options) {
        return Err(listed("TEXT"));
    }
    checked(format::render(&code, format::Value::Number(n), &options))
}

/// `TRIM`: no space at either end, and one space between words. Only the plain space is
/// trimmed.
pub(super) fn trim(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("TRIM", args, 1, 1)?;
    let t = take!(text_of(ctx, &args[0])?);
    let mut out = String::with_capacity(t.len());
    for word in t.split(' ').filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    value(Value::Text(out))
}

/// `VALUE`: a text read as a number, a number as it is and a blank as 0. A boolean is
/// `#VALUE!`.
pub(super) fn value_(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("VALUE", args, 1, 1)?;
    match ctx.scalar(&args[0])? {
        Value::Number(n) => number(n),
        Value::Blank => number(0.0),
        Value::Bool(_) => error(CellError::Value),
        Value::Error(e) => error(e),
        Value::Text(t) => match serial(ctx, "VALUE", &t)? {
            Some(n) => number(n),
            None => error(CellError::Value),
        },
    }
}
