//! OpenFormula, the formula text of an ods, to the A1 text an xlsx stores, and back.
//!
//! The two differ in their references and their separators, and agree on the rest: numbers,
//! strings, operators, error values and most function names.
//!
//! | OpenFormula | A1 text |
//! |---|---|
//! | `[.A1]`, `[.$A$1:.B2]` | `A1`, `$A$1:B2` |
//! | `[$Shop.A1]`, `[$'My sheet'.A1:.B2]` | `Shop!A1`, `'My sheet'!A1:B2` |
//! | `[$Shop.A1:$Lists.B2]` | `Shop:Lists!A1:B2` |
//! | `['file:///a.ods'#$Shop.A1]` | `'[file:///a.ods]Shop'!A1` |
//! | `[.A:.A]`, `[.1:.1]` | `A:A`, `1:1` |
//! | `;` between arguments | `,` |
//! | `~` (union), `!` (intersection) | `,` inside parentheses, a space |
//! | `{1;2\|3;4}` | `{1,2;3,4}` |
//! | `TRUE()`, `FALSE()` | `TRUE`, `FALSE` |
//! | `COM.MICROSOFT.IFS(…)` | `IFS(…)` |
//!
//! A reference whose sheet or cell is `#REF!` reads as `#REF!`. A sheet's `$` in OpenFormula
//! has no A1 form and is dropped. [`from_a1`] writes it for every sheet-qualified reference,
//! as LibreOffice does. The functions [`MICROSOFT`] lists carry LibreOffice's
//! `COM.MICROSOFT.` prefix in OpenFormula. Any other prefixed name is kept as it is.
//!
//! [`to_a1`] returns `None` for text it cannot read: a string, a quoted name or a reference
//! with no end. Nothing is evaluated, and no reference is followed.
//!
//! What it spends: the text it returns, per formula.

/// The functions LibreOffice writes with the `COM.MICROSOFT.` prefix.
const MICROSOFT: &[&str] = &[
    "CEILING",
    "CEILING.MATH",
    "CONCAT",
    "FLOOR",
    "FLOOR.MATH",
    "IFS",
    "MAXIFS",
    "MINIFS",
    "SWITCH",
    "TEXTJOIN",
    "XLOOKUP",
    "XMATCH",
];

const PREFIX: &str = "COM.MICROSOFT.";

/// The error values, as both forms write them.
const ERRORS: &[&str] = &[
    "#NULL!", "#DIV/0!", "#VALUE!", "#REF!", "#NAME?", "#NUM!", "#N/A",
];

/// The error value `text` starts with, if any.
fn error_at(text: &str) -> Option<&'static str> {
    ERRORS
        .iter()
        .find(|error| text.len() >= error.len() && text[..error.len()].eq_ignore_ascii_case(error))
        .copied()
}

/// The length of the string literal at the start of `text`, quotes included, with `""` inside
/// as one quote, or `None` when it has no end. `quote` is `"` or `'`.
fn quoted_len(text: &str, quote: char) -> Option<usize> {
    let mut chars = text.char_indices().skip(1).peekable();
    while let Some((at, c)) = chars.next() {
        if c == quote {
            if chars.peek().is_some_and(|&(_, next)| next == quote) {
                chars.next();
            } else {
                return Some(at + 1);
            }
        }
    }
    None
}

/// `name` without its quotes and with `''` as `'`.
fn unquote(name: &str) -> String {
    match name.strip_prefix('\'').and_then(|n| n.strip_suffix('\'')) {
        Some(inner) => inner.replace("''", "'"),
        None => name.to_owned(),
    }
}

/// Whether `c` may be part of a name, a function, a number or a reference.
fn word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | '$' | '\\')
}

/// The OpenFormula text `formula`, with or without its `of:=` or `=`, as A1 text with no `=`,
/// or `None` for text it cannot read.
pub fn to_a1(formula: &str) -> Option<String> {
    let body = formula.strip_prefix("of:").unwrap_or(formula);
    let body = body.strip_prefix('=').unwrap_or(body);
    let mut out = String::with_capacity(body.len());
    // Whether each open `{` is an inline array.
    let mut arrays = 0usize;
    let mut at = 0;
    while at < body.len() {
        let rest = &body[at..];
        let c = rest.chars().next()?;
        match c {
            '"' => {
                let len = quoted_len(rest, '"')?;
                out.push_str(&rest[..len]);
                at += len;
                continue;
            }
            '[' => {
                let len = reference_len(rest)?;
                out.push_str(&reference_to_a1(&rest[1..len - 1]));
                at += len;
                continue;
            }
            '#' => {
                if let Some(error) = error_at(rest) {
                    out.push_str(error);
                    at += error.len();
                    continue;
                }
                out.push('#');
            }
            '{' => {
                arrays += 1;
                out.push('{');
            }
            '}' => {
                arrays = arrays.saturating_sub(1);
                out.push('}');
            }
            ';' => out.push(','),
            '|' if arrays > 0 => out.push(';'),
            '~' => out.push(','),
            '!' => out.push(' '),
            c if word_char(c) => {
                let len = rest.find(|c: char| !word_char(c)).unwrap_or(rest.len());
                let word = &rest[..len];
                let call = rest[len..].starts_with('(');
                if call
                    && (word.eq_ignore_ascii_case("TRUE") || word.eq_ignore_ascii_case("FALSE"))
                    && rest[len..].starts_with("()")
                {
                    out.push_str(&word.to_ascii_uppercase());
                    at += len + 2;
                    continue;
                }
                let name = if call {
                    match word.get(..PREFIX.len()) {
                        Some(head) if head.eq_ignore_ascii_case(PREFIX) => &word[PREFIX.len()..],
                        _ => word,
                    }
                } else {
                    word
                };
                out.push_str(name);
                at += len;
                continue;
            }
            c => out.push(c),
        }
        at += c.len_utf8();
    }
    Some(out)
}

/// The length of the `[…]` reference at the start of `text`, brackets included, or `None` when
/// it has no end. A quoted name inside may hold a `]`.
fn reference_len(text: &str) -> Option<usize> {
    let mut at = 1;
    while at < text.len() {
        let rest = &text[at..];
        match rest.chars().next()? {
            '\'' => at += quoted_len(rest, '\'')?,
            ']' => return Some(at + 1),
            c => at += c.len_utf8(),
        }
    }
    None
}

/// One end of an OpenFormula reference: its source document, its sheet and its cell, column or
/// row.
struct End {
    source: Option<String>,
    sheet: Option<String>,
    address: String,
}

/// The parts of `text`, such as `$'My sheet'.$A$1` or `.B2`, split at the last `.` outside
/// quotes.
fn end(text: &str) -> End {
    let (mut source, mut rest) = (None, text);
    if rest.starts_with('\'') {
        if let Some(len) = quoted_len(rest, '\'') {
            if rest[len..].starts_with('#') {
                source = Some(unquote(&rest[..len]));
                rest = &rest[len + 1..];
            }
        }
    }
    let mut dot = None;
    let mut at = 0;
    while at < rest.len() {
        let tail = &rest[at..];
        let Some(c) = tail.chars().next() else { break };
        if c == '\'' {
            at += quoted_len(tail, '\'').unwrap_or(tail.len());
            continue;
        }
        if c == '.' {
            dot = Some(at);
        }
        at += c.len_utf8();
    }
    let Some(dot) = dot else {
        return End {
            source,
            sheet: None,
            address: rest.to_owned(),
        };
    };
    let sheet = rest[..dot].trim_start_matches('$');
    End {
        source,
        sheet: (!sheet.is_empty()).then(|| unquote(sheet)),
        address: rest[dot + 1..].to_owned(),
    }
}

/// The OpenFormula reference `body`, the text between its brackets, as A1 text.
fn reference_to_a1(body: &str) -> String {
    let mut ends = Vec::with_capacity(2);
    let mut start = 0;
    let mut at = 0;
    while at < body.len() {
        let rest = &body[at..];
        let Some(c) = rest.chars().next() else { break };
        match c {
            '\'' => {
                at += quoted_len(rest, '\'').unwrap_or(rest.len());
                continue;
            }
            ':' => {
                ends.push(end(&body[start..at]));
                start = at + 1;
            }
            _ => {}
        }
        at += c.len_utf8();
    }
    ends.push(end(&body[start..]));
    if ends
        .iter()
        .any(|e| e.address.contains("#REF!") || e.sheet.as_deref() == Some("#REF!"))
    {
        return "#REF!".to_owned();
    }
    let first = &ends[0];
    let mut out = String::new();
    if let Some(sheet) = &first.sheet {
        let last = ends
            .get(1)
            .and_then(|e| e.sheet.as_ref())
            .filter(|last| *last != sheet);
        let mut name = String::new();
        if let Some(source) = &first.source {
            name.push('[');
            name.push_str(source);
            name.push(']');
        }
        name.push_str(sheet);
        if let Some(last) = last {
            name.push(':');
            name.push_str(last);
        }
        let plain = first.source.is_none()
            && !needs_a1_quotes(sheet)
            && last.is_none_or(|last| !needs_a1_quotes(last));
        if plain {
            out.push_str(&name);
        } else {
            out.push('\'');
            out.push_str(&name.replace('\'', "''"));
            out.push('\'');
        }
        out.push('!');
    }
    for (at, e) in ends.iter().enumerate() {
        if at > 0 {
            out.push(':');
        }
        out.push_str(&e.address);
    }
    out
}

/// Whether the sheet `name` is quoted in A1 text: when it is not a plain word, or when it reads
/// as a cell or a number.
fn needs_a1_quotes(name: &str) -> bool {
    let mut chars = name.chars();
    let plain_start = chars.next().is_some_and(|c| c.is_alphabetic() || c == '_');
    !plain_start
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        || cell(name).is_some()
        || name.eq_ignore_ascii_case("TRUE")
        || name.eq_ignore_ascii_case("FALSE")
}

/// Whether the sheet `name` is quoted in OpenFormula.
fn needs_of_quotes(name: &str) -> bool {
    let plain_start = name
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_');
    !plain_start || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// The parts of an A1 address word: a cell (`$A$1`), a column (`$A`) or a row (`12`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
    Cell,
    Column,
    Row,
}

/// Whether `word` is a cell such as `$A$1` in the grid.
fn cell(word: &str) -> Option<Part> {
    let word = word.strip_prefix('$').unwrap_or(word);
    let letters = word.find(|c: char| !c.is_ascii_alphabetic())?;
    let (column, rest) = word.split_at(letters);
    let row = rest.strip_prefix('$').unwrap_or(rest);
    (column_in_grid(column) && row_in_grid(row)).then_some(Part::Cell)
}

fn column_in_grid(letters: &str) -> bool {
    !letters.is_empty()
        && letters.len() <= 3
        && letters.chars().all(|c| c.is_ascii_alphabetic())
        && letters.chars().fold(0u32, |n, c| {
            n * 26 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1)
        }) <= 16_384
}

fn row_in_grid(digits: &str) -> bool {
    !digits.is_empty()
        && digits.len() <= 7
        && digits.chars().all(|c| c.is_ascii_digit())
        && digits
            .parse::<u32>()
            .is_ok_and(|n| (1..=1_048_576).contains(&n))
}

/// What `word` is as one end of an A1 range.
fn range_end(word: &str) -> Option<Part> {
    if let Some(part) = cell(word) {
        return Some(part);
    }
    let bare = word.strip_prefix('$').unwrap_or(word);
    if column_in_grid(bare) {
        return Some(Part::Column);
    }
    row_in_grid(bare).then_some(Part::Row)
}

/// The A1 text `formula`, with or without its `=`, as OpenFormula with its `of:=`.
pub fn from_a1(formula: &str) -> String {
    let body = formula.strip_prefix('=').unwrap_or(formula);
    let mut out = String::with_capacity(body.len() + 16);
    out.push_str("of:=");
    // For each open `(`, whether it is a function call; for each `{`, an inline array.
    let mut groups: Vec<char> = Vec::new();
    let mut last_was_reference = false;
    let mut at = 0;
    while at < body.len() {
        let rest = &body[at..];
        let Some(c) = rest.chars().next() else { break };
        let mut reference = false;
        match c {
            '"' => {
                let len = quoted_len(rest, '"').unwrap_or(rest.len());
                out.push_str(&rest[..len]);
                at += len;
                last_was_reference = false;
                continue;
            }
            '#' => {
                if let Some(error) = error_at(rest) {
                    out.push_str(error);
                    at += error.len();
                    last_was_reference = false;
                    continue;
                }
                out.push('#');
            }
            '(' => {
                groups.push('g');
                out.push('(');
            }
            ')' => {
                groups.pop();
                out.push(')');
                reference = last_was_reference;
            }
            '{' => {
                groups.push('{');
                out.push('{');
            }
            '}' => {
                groups.pop();
                out.push('}');
            }
            ',' => out.push(match groups.last() {
                Some('f') => ';',
                Some('{') => ';',
                _ => '~',
            }),
            ';' if groups.last() == Some(&'{') => out.push('|'),
            ' ' => {
                let spaces = rest.len() - rest.trim_start_matches(' ').len();
                let next = rest[spaces..].chars().next();
                let starts_reference =
                    next.is_some_and(|n| n == '\'' || n == '$' || n == '(' || n.is_alphanumeric());
                if last_was_reference && starts_reference {
                    out.push('!');
                } else {
                    out.push_str(&rest[..spaces]);
                }
                at += spaces;
                continue;
            }
            '\'' => {
                if let Some(len) = quoted_len(rest, '\'') {
                    if rest[len..].starts_with('!') {
                        let sheets = unquote(&rest[..len]);
                        let consumed =
                            reference_from_a1(&rest[len + 1..], Some(sheets.as_str()), &mut out);
                        if consumed == 0 {
                            out.push_str(&rest[..=len]);
                        }
                        at += len + 1 + consumed;
                        last_was_reference = true;
                        continue;
                    }
                }
                out.push('\'');
            }
            c if word_char(c) => {
                let len = rest.find(|c: char| !word_char(c)).unwrap_or(rest.len());
                let word = &rest[..len];
                let after = &rest[len..];
                if after.starts_with('(') {
                    let upper = word.to_ascii_uppercase();
                    if MICROSOFT.contains(&upper.as_str()) {
                        out.push_str(PREFIX);
                    }
                    out.push_str(word);
                    out.push('(');
                    groups.push('f');
                    at += len + 1;
                    last_was_reference = false;
                    continue;
                }
                if word.eq_ignore_ascii_case("TRUE") || word.eq_ignore_ascii_case("FALSE") {
                    out.push_str(&word.to_ascii_uppercase());
                    out.push_str("()");
                    at += len;
                    last_was_reference = false;
                    continue;
                }
                if let Some(sheets) = sheet_prefix(rest) {
                    let used = sheets.len() + 1;
                    let consumed = reference_from_a1(&rest[used..], Some(sheets), &mut out);
                    if consumed == 0 {
                        out.push_str(&rest[..used]);
                    }
                    at += used + consumed;
                    last_was_reference = true;
                    continue;
                }
                if after.starts_with('[') {
                    // A structured reference such as `Shop[Total]` is kept as it is.
                    let mut depth = 0;
                    let mut end = len;
                    for (i, ch) in after.char_indices() {
                        match ch {
                            '[' => depth += 1,
                            ']' => {
                                depth -= 1;
                                if depth == 0 {
                                    end = len + i + 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    out.push_str(&rest[..end]);
                    at += end;
                    last_was_reference = true;
                    continue;
                }
                let consumed = reference_from_a1(rest, None, &mut out);
                if consumed > 0 {
                    at += consumed;
                    last_was_reference = true;
                    continue;
                }
                // A number, with its exponent, or a defined name.
                let mut end = len;
                if word.ends_with(['E', 'e'])
                    && word.starts_with(|c: char| c.is_ascii_digit() || c == '.')
                    && after.starts_with(['+', '-'])
                {
                    end += 1 + after[1..]
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(after.len() - 1);
                }
                out.push_str(&rest[..end]);
                at += end;
                last_was_reference = !word.starts_with(|c: char| c.is_ascii_digit() || c == '.');
                continue;
            }
            c => out.push(c),
        }
        last_was_reference = reference;
        at += c.len_utf8();
    }
    out
}

/// The unquoted sheet text before a `!` at the start of `text`, such as `Shop` or `Shop:Lists`,
/// or `None`.
fn sheet_prefix(text: &str) -> Option<&str> {
    let first = text.find(|c: char| !word_char(c)).unwrap_or(text.len());
    let mut end = first;
    if text[end..].starts_with(':') {
        let rest = &text[end + 1..];
        let second = rest.find(|c: char| !word_char(c)).unwrap_or(rest.len());
        if second > 0 && rest[second..].starts_with('!') {
            end += 1 + second;
        }
    }
    text[end..].starts_with('!').then(|| &text[..end])
}

/// Writes the A1 reference at the start of `text`, on the sheets `sheets` (`Shop` or
/// `Shop:Lists`, already unquoted) when it has some, as OpenFormula, and returns its length in
/// `text`. Returns `0`, writing nothing, when `text` does not start with a reference.
fn reference_from_a1(text: &str, sheets: Option<&str>, out: &mut String) -> usize {
    let word_len = |t: &str| t.find(|c: char| !word_char(c)).unwrap_or(t.len());
    let first_len = word_len(text);
    let first = &text[..first_len];
    let Some(first_part) = range_end(first) else {
        return 0;
    };
    let mut second = None;
    if let Some(rest) = text[first_len..].strip_prefix(':') {
        let len = word_len(rest);
        let word = &rest[..len];
        if range_end(word) == Some(first_part) {
            second = Some(word);
        }
    }
    // A lone column or row is a name, not a reference.
    if second.is_none() && first_part != Part::Cell {
        return 0;
    }
    let (source, sheets) = match sheets {
        Some(s) if s.starts_with('[') => match s.find(']') {
            Some(close) => (Some(&s[1..close]), Some(&s[close + 1..])),
            None => (None, Some(s)),
        },
        other => (None, other),
    };
    let (sheet, last) = match sheets {
        Some(s) => match s.split_once(':') {
            Some((a, b)) => (Some(a), Some(b)),
            None => (Some(s), None),
        },
        None => (None, None),
    };
    let sheet_name = |name: &str| {
        if needs_of_quotes(name) {
            format!("'{}'", name.replace('\'', "''"))
        } else {
            name.to_owned()
        }
    };
    out.push('[');
    if let Some(source) = source {
        out.push('\'');
        out.push_str(&source.replace('\'', "''"));
        out.push_str("'#");
    }
    match sheet {
        Some(sheet) => {
            out.push('$');
            out.push_str(&sheet_name(sheet));
            out.push('.');
        }
        None => out.push('.'),
    }
    out.push_str(first);
    if let Some(second) = second {
        out.push(':');
        if let Some(last) = last {
            out.push('$');
            out.push_str(&sheet_name(last));
        }
        out.push('.');
        out.push_str(second);
    }
    out.push(']');
    first_len + second.map_or(0, |s| 1 + s.len())
}
