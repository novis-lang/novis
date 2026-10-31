//! The parser: A1 formula text into an [`Expr`] tree
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! It reads numbers, strings, booleans, error constants, cells, ranges, whole rows and columns,
//! each with an optional sheet (`'Q1 sales'!A1`), defined names, structured references
//! (`Table[Column]`, `[@x]`, `Table[[#This Row],[x]]`), the operators by Excel's precedence, and
//! calls. The reference lexing is the shifter's ([`crate::template::shift`]): its sheet prefix,
//! its row digits and its test of where a reference ends. A column's letters may be in either
//! case here, because a program's formula text may be.
//!
//! The precedence, from the tightest: `:` (range), a space (intersection), `,` (union, inside
//! parentheses only), `-` and `+` before a value, `%` after one, `^`, `*` and `/`, `+` and `-`,
//! `&`, and the comparisons. Excel reads `-2^2` as `4` and `2^3^2` as `64`, and so does this.
//!
//! A part the evaluator lists and never computes is an [`Expr::Listed`] with its text as the
//! formula wrote it: a 3D reference (`Jan:Mar!A1`), a reference into another workbook
//! (`[1]Shop!A1`, `'[Book.xlsx]Shop'!A1`), a union, an intersection and an array constant.
//!
//! Formula text is [`Unreadable`] past three bounds, so the recursive parser and evaluator have
//! a bounded stack: parentheses and calls more than [`NESTING`] deep, a tree more than
//! [`HEIGHT`] nodes deep, and more than [`NODES`] operators and calls.

use crate::read::CellError;
use crate::template::shift::{self, ROWS};

/// The most columns a sheet has.
pub const COLUMNS: u32 = 16_384;

/// The deepest tree the parser returns.
pub const HEIGHT: usize = 512;

/// The most parentheses and calls one inside another, Excel's own limit.
pub const NESTING: usize = 64;

/// The most operators and calls one formula may have.
pub const NODES: usize = 4_096;

/// A rectangle of cells, rows and columns from 1, `top` <= `bottom` and `left` <= `right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub top: u32,
    pub left: u32,
    pub bottom: u32,
    pub right: u32,
}

impl Rect {
    pub fn cell(row: u32, col: u32) -> Self {
        Self {
            top: row,
            left: col,
            bottom: row,
            right: col,
        }
    }

    pub fn contains(self, row: u32, col: u32) -> bool {
        (self.top..=self.bottom).contains(&row) && (self.left..=self.right).contains(&col)
    }

    /// The rectangle as A1 text: `A1`, `A1:B2`, `A:C` or `1:3`.
    pub fn a1(self) -> String {
        if self.top == 1 && self.bottom == ROWS {
            return format!("{}:{}", letters(self.left), letters(self.right));
        }
        if self.left == 1 && self.right == COLUMNS {
            return format!("{}:{}", self.top, self.bottom);
        }
        let first = format!("{}{}", letters(self.left), self.top);
        if self.top == self.bottom && self.left == self.right {
            first
        } else {
            format!("{first}:{}{}", letters(self.right), self.bottom)
        }
    }
}

/// A column's letters, `1` as `A`.
pub fn letters(mut col: u32) -> String {
    let mut out = Vec::new();
    while col > 0 {
        let digit = (col - 1) % 26;
        out.push(b'A' + digit as u8);
        col = (col - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// A plain address such as `B3` as its row and column, or `None`.
pub fn address(text: &str) -> Option<(u32, u32)> {
    let bytes = text.as_bytes();
    let (col, at) = column(bytes, 0)?;
    let at = at + usize::from(bytes.get(at) == Some(&b'$'));
    let (row, end) = shift::row_digits(bytes, at)?;
    (end == bytes.len()).then_some((row, col))
}

/// A reference to cells, on `sheet` or on the formula's own sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub sheet: Option<String>,
    pub rect: Rect,
}

/// One special item of a structured reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    All,
    Data,
    Headers,
    Totals,
    ThisRow,
}

/// A structured reference: its table, or the table the formula is in, its items, and its first
/// and last column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structured {
    pub table: Option<String>,
    pub items: Vec<Item>,
    pub columns: Option<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unary {
    Minus,
    Plus,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {
    Range,
    Power,
    Times,
    Divide,
    Plus,
    Minus,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Text(String),
    Bool(bool),
    Error(CellError),
    /// An argument left empty, as in `IF(A1,,2)`.
    Missing,
    Area(Area),
    /// A defined name, with the sheet that qualifies it.
    Name(Option<String>, String),
    Structured(Structured),
    Unary(Unary, Box<Expr>),
    Binary(Binary, Box<Expr>, Box<Expr>),
    /// A call, its name in upper case without an `_xlfn.` prefix.
    Call(String, Vec<Expr>),
    /// A part the evaluator lists, with its text.
    Listed(String),
}

/// Formula text the parser cannot read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

type Parsed = Result<Expr, Unreadable>;

/// `text`, with or without its `=`, as a tree.
pub fn parse(text: &str) -> Parsed {
    let body = text.strip_prefix('=').unwrap_or(text);
    let mut parser = Parser {
        text: body,
        bytes: body.as_bytes(),
        at: 0,
        depth: 0,
        nodes: 0,
    };
    let expr = parser.expression()?;
    parser.spaces();
    if parser.at != body.len() || height(&expr) > HEIGHT {
        return Err(Unreadable);
    }
    Ok(expr)
}

/// How deep `expr` is, walked with a stack of its own.
fn height(expr: &Expr) -> usize {
    let mut most = 0;
    let mut stack = vec![(expr, 1)];
    while let Some((node, depth)) = stack.pop() {
        most = most.max(depth);
        match node {
            Expr::Unary(_, inner) => stack.push((inner, depth + 1)),
            Expr::Binary(_, left, right) => {
                stack.push((left, depth + 1));
                stack.push((right, depth + 1));
            }
            Expr::Call(_, args) => stack.extend(args.iter().map(|arg| (arg, depth + 1))),
            _ => {}
        }
    }
    most
}

/// A column's letters at `at`, after an optional `$`, in either case: the column and where the
/// letters end.
fn column(text: &[u8], at: usize) -> Option<(u32, usize)> {
    let mut i = at + usize::from(text.get(at) == Some(&b'$'));
    let start = i;
    let mut col: u32 = 0;
    while let Some(b) = text.get(i).filter(|b| b.is_ascii_alphabetic()) {
        col = col * 26 + u32::from(b.to_ascii_uppercase() - b'A' + 1);
        i += 1;
        if i - start > 3 {
            return None;
        }
    }
    (i > start && col <= COLUMNS).then_some((col, i))
}

/// A cell at `at`: `$`, letters, `$`, digits. Its row, column and end.
fn cell(text: &[u8], at: usize) -> Option<(u32, u32, usize)> {
    let (col, i) = column(text, at)?;
    let i = i + usize::from(text.get(i) == Some(&b'$'));
    shift::row_digits(text, i).map(|(row, end)| (row, col, end))
}

fn is_name_byte(b: u8) -> bool {
    shift::ident(b) || b == b'?'
}

/// Whether `expr` can stand for a reference, so a `:` or a space after it is an operator.
fn refers(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Area(_)
            | Expr::Name(..)
            | Expr::Structured(_)
            | Expr::Listed(_)
            | Expr::Call(..)
            | Expr::Binary(Binary::Range, ..)
    )
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
    depth: usize,
    nodes: usize,
}

impl Parser<'_> {
    /// Counts one operator or call, so a hostile formula cannot build a huge tree.
    fn node(&mut self) -> Result<(), Unreadable> {
        self.nodes += 1;
        if self.nodes > NODES {
            Err(Unreadable)
        } else {
            Ok(())
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Skips spaces and line breaks, and returns whether there were any.
    fn spaces(&mut self) -> bool {
        let from = self.at;
        while self
            .peek()
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
        self.at > from
    }

    fn eat(&mut self, b: u8) -> bool {
        let found = self.peek() == Some(b);
        if found {
            self.at += 1;
        }
        found
    }

    fn expect(&mut self, b: u8) -> Result<(), Unreadable> {
        if self.eat(b) { Ok(()) } else { Err(Unreadable) }
    }

    fn expression(&mut self) -> Parsed {
        self.depth += 1;
        if self.depth > NESTING {
            return Err(Unreadable);
        }
        let expr = self.comparison();
        self.depth -= 1;
        expr
    }

    fn comparison(&mut self) -> Parsed {
        let mut left = self.concat()?;
        loop {
            self.spaces();
            let rest = &self.bytes[self.at..];
            let (op, len) = if rest.starts_with(b"<=") {
                (Binary::Le, 2)
            } else if rest.starts_with(b">=") {
                (Binary::Ge, 2)
            } else if rest.starts_with(b"<>") {
                (Binary::Ne, 2)
            } else {
                match rest.first() {
                    Some(b'=') => (Binary::Eq, 1),
                    Some(b'<') => (Binary::Lt, 1),
                    Some(b'>') => (Binary::Gt, 1),
                    _ => return Ok(left),
                }
            };
            self.at += len;
            let right = self.concat()?;
            self.node()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn concat(&mut self) -> Parsed {
        let mut left = self.additive()?;
        loop {
            self.spaces();
            if !self.eat(b'&') {
                return Ok(left);
            }
            let right = self.additive()?;
            self.node()?;
            left = Expr::Binary(Binary::Concat, Box::new(left), Box::new(right));
        }
    }

    fn additive(&mut self) -> Parsed {
        let mut left = self.multiplicative()?;
        loop {
            self.spaces();
            let op = match self.peek() {
                Some(b'+') => Binary::Plus,
                Some(b'-') => Binary::Minus,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.multiplicative()?;
            self.node()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn multiplicative(&mut self) -> Parsed {
        let mut left = self.power()?;
        loop {
            self.spaces();
            let op = match self.peek() {
                Some(b'*') => Binary::Times,
                Some(b'/') => Binary::Divide,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.power()?;
            self.node()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn power(&mut self) -> Parsed {
        let mut left = self.unary()?;
        loop {
            self.spaces();
            if !self.eat(b'^') {
                return Ok(left);
            }
            let right = self.unary()?;
            self.node()?;
            left = Expr::Binary(Binary::Power, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Parsed {
        // Signs are counted first, so a long run of them does not recurse.
        let mut signs = Vec::new();
        loop {
            self.spaces();
            match self.peek() {
                Some(b'-') => signs.push(Unary::Minus),
                Some(b'+') => signs.push(Unary::Plus),
                _ => break,
            }
            self.node()?;
            self.at += 1;
        }
        let mut expr = self.intersection()?;
        loop {
            let save = self.at;
            self.spaces();
            if !self.eat(b'%') {
                self.at = save;
                break;
            }
            self.node()?;
            expr = Expr::Unary(Unary::Percent, Box::new(expr));
        }
        for sign in signs.into_iter().rev() {
            expr = Expr::Unary(sign, Box::new(expr));
        }
        Ok(expr)
    }

    fn intersection(&mut self) -> Parsed {
        let start = self.at;
        let mut expr = self.range()?;
        loop {
            let save = self.at;
            let spaced = self.spaces();
            let next = self.peek();
            let begins = next.is_some_and(|b| {
                matches!(b, b'$' | b'\'' | b'(' | b'[') || b.is_ascii_alphabetic() || b == b'_'
            });
            if !(spaced && begins && refers(&expr)) {
                self.at = save;
                return Ok(expr);
            }
            self.range()?;
            expr = Expr::Listed(self.text[start..self.at].trim().to_owned());
        }
    }

    fn range(&mut self) -> Parsed {
        let mut expr = self.primary()?;
        while self.peek() == Some(b':') && refers(&expr) {
            self.at += 1;
            self.node()?;
            let right = self.primary()?;
            expr = Expr::Binary(Binary::Range, Box::new(expr), Box::new(right));
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Parsed {
        self.spaces();
        let start = self.at;
        let Some(b) = self.peek() else {
            return Err(Unreadable);
        };
        match b {
            b'(' => {
                self.at += 1;
                let inner = self.expression()?;
                self.spaces();
                if self.peek() == Some(b',') {
                    while self.eat(b',') {
                        self.expression()?;
                        self.spaces();
                    }
                    self.expect(b')')?;
                    return Ok(Expr::Listed(self.text[start..self.at].to_owned()));
                }
                self.expect(b')')?;
                Ok(inner)
            }
            b'"' => self.string(),
            b'#' => self.error(),
            b'{' => self.array(start),
            b'[' if self.external() => self.listed_reference(start),
            b'[' => self.structured(None),
            b'\'' => {
                let (name, begin) = shift::sheet(self.text, self.at).ok_or(Unreadable)?;
                if name.contains([':', '[', ']']) {
                    self.at = begin;
                    return self.listed_reference(start);
                }
                self.qualified(name, begin)
            }
            b'0'..=b'9' | b'.' => {
                if let Some(expr) = self.reference(None, self.at) {
                    return Ok(expr);
                }
                self.number()
            }
            _ if b == b'$' || is_name_byte(b) => self.word(start),
            _ => Err(Unreadable),
        }
    }

    fn string(&mut self) -> Parsed {
        self.at += 1;
        let mut out = String::new();
        loop {
            let rest = &self.text[self.at..];
            let quote = rest.find('"').ok_or(Unreadable)?;
            out.push_str(&rest[..quote]);
            self.at += quote + 1;
            if self.peek() == Some(b'"') {
                out.push('"');
                self.at += 1;
            } else {
                return Ok(Expr::Text(out));
            }
        }
    }

    fn error(&mut self) -> Parsed {
        const TEXTS: [&str; 10] = [
            "#NULL!",
            "#DIV/0!",
            "#VALUE!",
            "#REF!",
            "#NAME?",
            "#NUM!",
            "#N/A",
            "#GETTING_DATA",
            "#SPILL!",
            "#CALC!",
        ];
        let rest = &self.text[self.at..];
        for text in TEXTS {
            if rest.len() >= text.len() && rest[..text.len()].eq_ignore_ascii_case(text) {
                self.at += text.len();
                return CellError::of(text).map(Expr::Error).ok_or(Unreadable);
            }
        }
        Err(Unreadable)
    }

    fn array(&mut self, start: usize) -> Parsed {
        let mut quoted = false;
        while let Some(b) = self.peek() {
            self.at += 1;
            match b {
                b'"' => quoted = !quoted,
                b'}' if !quoted => return Ok(Expr::Listed(self.text[start..self.at].to_owned())),
                _ => {}
            }
        }
        Err(Unreadable)
    }

    fn number(&mut self) -> Parsed {
        let start = self.at;
        let digits = |p: &mut Self| {
            while p.peek().is_some_and(|b| b.is_ascii_digit()) {
                p.at += 1;
            }
        };
        digits(self);
        if self.eat(b'.') {
            digits(self);
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let save = self.at;
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.peek().is_some_and(|b| b.is_ascii_digit()) {
                digits(self);
            } else {
                self.at = save;
            }
        }
        self.text[start..self.at]
            .parse::<f64>()
            .map(Expr::Number)
            .map_err(|_| Unreadable)
    }

    /// Whether the `[` here begins another workbook's reference: `[1]Shop!A1` or `[Book.xlsx]Shop!A1`.
    fn external(&self) -> bool {
        let Some(close) = self.text[self.at..].find(']') else {
            return false;
        };
        let after = self.at + close + 1;
        if self.text[self.at + 1..after - 1].contains(['[', '@', '#']) {
            return false;
        }
        let mut i = after;
        while self.bytes.get(i).is_some_and(|&b| is_name_byte(b)) {
            i += 1;
        }
        self.bytes.get(i) == Some(&b'!') || (i > after && self.bytes.get(i) != Some(&b'['))
    }

    /// Reads past a reference the evaluator lists, from `start`, and returns its text.
    fn listed_reference(&mut self, start: usize) -> Parsed {
        while let Some(b) = self.peek() {
            if b == b'\'' {
                let (_, begin) = shift::sheet(self.text, self.at).ok_or(Unreadable)?;
                self.at = begin;
                continue;
            }
            if !(is_name_byte(b) || matches!(b, b'$' | b':' | b'!' | b'[' | b']' | b'#')) {
                break;
            }
            self.at += 1;
        }
        Ok(Expr::Listed(self.text[start..self.at].to_owned()))
    }

    /// A cell, a range, whole columns or whole rows at `at`, on `sheet`. Moves past it.
    fn reference(&mut self, sheet: Option<&str>, at: usize) -> Option<Expr> {
        let bytes = self.bytes;
        let area = |rect| {
            Some(Expr::Area(Area {
                sheet: sheet.map(str::to_owned),
                rect,
            }))
        };
        if let Some((row, col, end)) = cell(bytes, at) {
            if bytes.get(end) == Some(&b':')
                && let Some((row2, col2, end2)) = cell(bytes, end + 1)
                && shift::ends(bytes, end2)
            {
                self.at = end2;
                return area(Rect {
                    top: row.min(row2),
                    left: col.min(col2),
                    bottom: row.max(row2),
                    right: col.max(col2),
                });
            }
            if shift::ends(bytes, end) {
                self.at = end;
                return area(Rect::cell(row, col));
            }
            return None;
        }
        if let Some((col, end)) = column(bytes, at)
            && bytes.get(end) == Some(&b':')
            && let Some((col2, end2)) = column(bytes, end + 1)
            && shift::ends(bytes, end2)
            && !bytes.get(end2).is_some_and(u8::is_ascii_digit)
        {
            self.at = end2;
            return area(Rect {
                top: 1,
                left: col.min(col2),
                bottom: ROWS,
                right: col.max(col2),
            });
        }
        if let Some((row, _, end)) = shift::row_only(bytes, at)
            && bytes.get(end) == Some(&b':')
            && let Some((row2, _, end2)) = shift::row_only(bytes, end + 1)
            && shift::ends(bytes, end2)
        {
            self.at = end2;
            return area(Rect {
                top: row.min(row2),
                left: 1,
                bottom: row.max(row2),
                right: COLUMNS,
            });
        }
        None
    }

    /// What follows `Sheet!`: a reference, a name, or `#REF!`.
    fn qualified(&mut self, sheet: String, begin: usize) -> Parsed {
        if let Some(expr) = self.reference(Some(&sheet), begin) {
            return Ok(expr);
        }
        self.at = begin;
        if self.peek() == Some(b'#') {
            return self.error();
        }
        let name = self.name();
        if name.is_empty() {
            return Err(Unreadable);
        }
        Ok(Expr::Name(Some(sheet), name))
    }

    fn name(&mut self) -> String {
        let start = self.at;
        while self.peek().is_some_and(is_name_byte) {
            self.at += 1;
        }
        self.text[start..self.at].to_owned()
    }

    /// A word at `start`: a sheet prefix, a 3D reference, a reference, a call, a structured
    /// reference, a boolean or a name.
    fn word(&mut self, start: usize) -> Parsed {
        if let Some((name, begin)) = shift::sheet(self.text, start) {
            return self.qualified(name, begin);
        }
        // `Jan:Mar!A1`: a sheet range.
        let first = {
            let mut i = start;
            while self.bytes.get(i).is_some_and(|&b| is_name_byte(b)) {
                i += 1;
            }
            i
        };
        if first > start
            && self.bytes.get(first) == Some(&b':')
            && let Some((_, begin)) = shift::sheet(self.text, first + 1)
        {
            self.at = begin;
            return self.listed_reference(start);
        }
        if let Some(expr) = self.reference(None, start) {
            return Ok(expr);
        }
        if self.peek() == Some(b'$') {
            return Err(Unreadable);
        }
        let name = self.name();
        match self.peek() {
            Some(b'(') => {
                self.at += 1;
                self.call(&name)
            }
            Some(b'[') => self.structured(Some(name)),
            _ if name.eq_ignore_ascii_case("TRUE") => Ok(Expr::Bool(true)),
            _ if name.eq_ignore_ascii_case("FALSE") => Ok(Expr::Bool(false)),
            _ => Ok(Expr::Name(None, name)),
        }
    }

    fn call(&mut self, name: &str) -> Parsed {
        let mut upper = name.to_ascii_uppercase();
        for prefix in ["_XLFN._XLWS.", "_XLFN.", "_XLWS."] {
            if let Some(rest) = upper.strip_prefix(prefix) {
                upper = rest.to_owned();
                break;
            }
        }
        self.node()?;
        let mut args = Vec::new();
        self.spaces();
        if self.eat(b')') {
            return Ok(Expr::Call(upper, args));
        }
        loop {
            self.spaces();
            if matches!(self.peek(), Some(b',' | b')')) {
                args.push(Expr::Missing);
            } else {
                args.push(self.expression()?);
                self.spaces();
            }
            if self.eat(b')') {
                return Ok(Expr::Call(upper, args));
            }
            self.expect(b',')?;
        }
    }

    /// A structured reference's brackets, at `[`.
    fn structured(&mut self, table: Option<String>) -> Parsed {
        self.expect(b'[')?;
        let mut items = Vec::new();
        let mut columns: Option<(String, String)> = None;
        self.spaces();
        match self.peek() {
            Some(b']') => {}
            Some(b'[') => loop {
                self.spaces();
                self.expect(b'[')?;
                let first = self.bracket()?;
                let first = part(first, &mut items)?;
                if let Some(name) = first {
                    let mut last = name.clone();
                    if self.eat(b':') {
                        self.expect(b'[')?;
                        last = self.bracket()?;
                    }
                    columns = Some((name, last));
                }
                self.spaces();
                if self.eat(b',') {
                    continue;
                }
                break;
            },
            Some(b'@') => {
                self.at += 1;
                items.push(Item::ThisRow);
                if self.eat(b'[') {
                    let name = self.bracket()?;
                    columns = Some((name.clone(), name));
                } else if self.peek() != Some(b']') {
                    let name = self.bracket_text()?;
                    columns = Some((name.clone(), name));
                }
            }
            _ => {
                let text = self.bracket_text()?;
                if let Some(name) = part(text, &mut items)? {
                    columns = Some((name.clone(), name));
                }
            }
        }
        self.spaces();
        self.expect(b']')?;
        Ok(Expr::Structured(Structured {
            table,
            items,
            columns,
        }))
    }

    /// A bracket's text up to its `]`, which it moves past.
    fn bracket(&mut self) -> Result<String, Unreadable> {
        let text = self.bracket_text()?;
        self.expect(b']')?;
        Ok(text)
    }

    /// Text up to an unescaped `]`, with each `'` escape read.
    fn bracket_text(&mut self) -> Result<String, Unreadable> {
        let mut out = String::new();
        loop {
            let c = self.text[self.at..].chars().next().ok_or(Unreadable)?;
            match c {
                ']' => return Ok(out),
                '\'' => {
                    self.at += 1;
                    let escaped = self.text[self.at..].chars().next().ok_or(Unreadable)?;
                    out.push(escaped);
                    self.at += escaped.len_utf8();
                }
                _ => {
                    out.push(c);
                    self.at += c.len_utf8();
                }
            }
        }
    }
}

/// One part of a structured reference: an item, pushed to `items`, or a column's name.
fn part(text: String, items: &mut Vec<Item>) -> Result<Option<String>, Unreadable> {
    if !text.starts_with('#') {
        return Ok(Some(text));
    }
    let item = match text.to_ascii_lowercase().as_str() {
        "#all" => Item::All,
        "#data" => Item::Data,
        "#headers" => Item::Headers,
        "#totals" => Item::Totals,
        "#this row" => Item::ThisRow,
        _ => return Err(Unreadable),
    };
    items.push(item);
    Ok(None)
}
