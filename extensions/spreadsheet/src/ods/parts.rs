//! The sheet-end parts as ods writes them: data validation, conditional formats, the page setup,
//! the protection key and the structured references of a table
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! **A validation** is a `table:content-validation`, whose `table:condition` is OpenFormula:
//! `of:cell-content-is-in-list("a";"b")` for a list, `of:cell-content-is-whole-number() and
//! cell-content-is-between(1;10)` for a whole number, `cell-content-text-length()` for a length
//! and `of:is-true-formula(…)` for a formula. A date bound is its serial.
//!
//! **A conditional format** is a `calcext:conditional-format`, the form LibreOffice reads. A rule
//! that formats cells applies a named cell style of `styles.xml`. A colour scale, a data bar and
//! an icon set draw their own colours, red, yellow and green for a scale with none.
//!
//! **A structured reference** has no ods form, so [`resolve`] turns `Shop[qty]` into the column's
//! data rows, such as `$C$2:$C$10`, and `Shop[[#This Row],[qty]]` into the cell of the formula's
//! own row, such as `C5`, before the formula becomes OpenFormula.

use std::fmt::Write as _;

use super::cells::paragraphs;
use super::sheet_ref;
use super::styles::color;
use crate::conditional::{Conditional, Kind};
use crate::table::Placed;
use crate::tail::{
    Area, DEFAULT_MARGINS, Operator, Orientation, Print, RuleKind, Validation, area, bound,
    password_hash,
};
use crate::{cells, openformula, xml};

/// The name LibreOffice gives Excel's 16-bit password hash as a digest algorithm.
pub const LEGACY_HASH: &str = "http://docs.oasis-open.org/office/ns/table/legacy-hash-excel";

/// The A1 text `formula` as OpenFormula with no `of:=`.
pub fn bare(formula: &str) -> String {
    let text = openformula::from_a1(formula);
    match text.strip_prefix("of:=") {
        Some(rest) => rest.to_owned(),
        None => text,
    }
}

/// `at` on the sheet `sheet` as an ods address, such as `Shop.A1:Shop.C10`.
pub fn range(sheet: &str, at: &Area) -> String {
    let sheet = sheet_ref(sheet);
    let from = cells::reference(at.left, at.top);
    if at.left == at.right && at.top == at.bottom {
        return format!("{sheet}.{from}");
    }
    format!(
        "{sheet}.{from}:{sheet}.{}",
        cells::reference(at.right, at.bottom)
    )
}

/// The OpenFormula operator of `operator`, for the operators with one operand.
fn symbol(operator: Operator) -> &'static str {
    match operator {
        Operator::Equal => "=",
        Operator::NotEqual => "!=",
        Operator::Greater => ">",
        Operator::GreaterOrEqual => ">=",
        Operator::Less => "<",
        Operator::LessOrEqual => "<=",
        Operator::Between | Operator::NotBetween => "",
    }
}

/// The test of `subject`, such as `cell-content`, against `values` by `operator`.
fn compare(subject: &str, operator: Operator, values: &[String]) -> String {
    let first = values.first().map_or("", String::as_str);
    let second = values.get(1).map_or("", String::as_str);
    match operator {
        Operator::Between => format!("{subject}-is-between({first};{second})"),
        Operator::NotBetween => format!("{subject}-is-not-between({first};{second})"),
        operator => format!("{subject}(){}{first}", symbol(operator)),
    }
}

/// The `table:condition` of `rule`.
fn condition(rule: &Validation) -> String {
    let values: Vec<String> = rule
        .values
        .iter()
        .map(|value| bare(&bound(rule.kind, value)))
        .collect();
    let operator = rule.operator.unwrap_or(Operator::Between);
    match rule.kind {
        RuleKind::List => match &rule.values[..] {
            [one] if one.starts_with('=') => {
                format!("of:cell-content-is-in-list({})", bare(&one[1..]))
            }
            items => {
                let quoted: Vec<String> = items
                    .iter()
                    .map(|item| format!("\"{}\"", item.replace('"', "\"\"")))
                    .collect();
                format!("of:cell-content-is-in-list({})", quoted.join(";"))
            }
        },
        RuleKind::Formula => format!(
            "of:is-true-formula({})",
            values.first().map_or("", String::as_str)
        ),
        RuleKind::Length => format!(
            "of:{}",
            compare("cell-content-text-length", operator, &values)
        ),
        kind => {
            let test = match kind {
                RuleKind::Whole => "cell-content-is-whole-number()",
                RuleKind::Decimal => "cell-content-is-decimal-number()",
                _ => "cell-content-is-date()",
            };
            format!(
                "of:{test} and {}",
                compare("cell-content", operator, &values)
            )
        }
    }
}

/// Appends ` table:title="…"` when `title` is set.
fn title(out: &mut String, title: &Option<String>) {
    if let Some(title) = title {
        out.push_str(" table:title=\"");
        xml::odf(out, title, true);
        out.push('"');
    }
}

/// Appends `rule`, over `at` on the sheet `sheet`, as the `table:content-validation` `name`.
pub fn validation(out: &mut String, name: &str, sheet: &str, at: &Area, rule: &Validation) {
    let _ = write!(
        out,
        "<table:content-validation table:name=\"{name}\" table:condition=\""
    );
    xml::odf(out, &condition(rule), true);
    let _ = write!(
        out,
        "\" table:allow-empty-cell=\"{}\" table:base-cell-address=\"",
        rule.allow_empty
    );
    let corner = Area {
        right: at.left,
        bottom: at.top,
        ..*at
    };
    xml::odf(out, &range(sheet, &corner), true);
    out.push('"');
    if rule.kind == RuleKind::List {
        out.push_str(" table:display-list=\"unsorted\"");
    }
    out.push('>');
    if rule.input_title.is_some() || rule.input_message.is_some() {
        out.push_str("<table:help-message");
        title(out, &rule.input_title);
        out.push_str(" table:display=\"true\">");
        if let Some(message) = &rule.input_message {
            paragraphs(out, message);
        }
        out.push_str("</table:help-message>");
    }
    out.push_str("<table:error-message");
    title(out, &rule.error_title);
    out.push_str(" table:display=\"true\" table:message-type=\"stop\">");
    if let Some(message) = &rule.error_message {
        paragraphs(out, message);
    }
    out.push_str("</table:error-message></table:content-validation>");
}

/// The colour `color` as ods writes it, or `fallback` for a colour `conditional::check` did not
/// allow.
fn ink(color_text: &str, fallback: &str) -> String {
    color(color_text).unwrap_or_else(|_| fallback.to_owned())
}

/// Appends `rule`, checked by `conditional::check`, as a `calcext:conditional-format` on the
/// sheet `sheet`, applying the cell style `style` when it formats cells.
pub fn conditional(out: &mut String, sheet: &str, rule: &Conditional, style: Option<&str>) {
    let ranges: Vec<Area> = rule.formatted.split(' ').filter_map(area).collect();
    let targets: Vec<String> = ranges.iter().map(|at| range(sheet, at)).collect();
    out.push_str("<calcext:conditional-format calcext:target-range-address=\"");
    xml::odf(out, &targets.join(" "), true);
    out.push_str("\">");
    let colors: Vec<String> = rule
        .colors
        .iter()
        .map(|color| ink(color, "#000000"))
        .collect();
    let value = match rule.kind {
        Kind::Cell => {
            let values: Vec<String> = rule.values.iter().map(|value| bare(value)).collect();
            let first = values.first().map_or("", String::as_str);
            Some(match rule.operator.unwrap_or(Operator::Equal) {
                Operator::Between => format!("between({first},{})", values[1]),
                Operator::NotBetween => format!("not-between({first},{})", values[1]),
                operator => format!("{}{first}", symbol(operator)),
            })
        }
        Kind::Formula => Some(format!("formula-is({})", bare(&rule.values[0]))),
        Kind::Top | Kind::Bottom => {
            let side = if rule.kind == Kind::Top {
                "top"
            } else {
                "bottom"
            };
            let unit = if rule.percent { "percent" } else { "elements" };
            Some(format!("{side}-{unit}({})", rule.rank.unwrap_or(10)))
        }
        Kind::Duplicates => Some("duplicate".to_owned()),
        Kind::Unique => Some("unique".to_owned()),
        Kind::ColorScale | Kind::DataBar | Kind::IconSet => None,
    };
    if let Some(value) = value {
        out.push_str("<calcext:condition calcext:apply-style-name=\"");
        xml::odf(out, style.unwrap_or("Default"), true);
        out.push_str("\" calcext:value=\"");
        xml::odf(out, &value, true);
        out.push_str("\" calcext:base-cell-address=\"");
        if let Some(first) = ranges.first() {
            let corner = Area {
                right: first.left,
                bottom: first.top,
                ..*first
            };
            xml::odf(out, &range(sheet, &corner), true);
        }
        out.push_str("\"/>");
    }
    match rule.kind {
        Kind::ColorScale => {
            let colors = if colors.is_empty() {
                vec![
                    "#F8696B".to_owned(),
                    "#FFEB84".to_owned(),
                    "#63BE7B".to_owned(),
                ]
            } else {
                colors
            };
            out.push_str("<calcext:color-scale>");
            let last = colors.len() - 1;
            for (at, color) in colors.iter().enumerate() {
                let (kind, value) = match at {
                    0 => ("minimum", 0),
                    at if at == last => ("maximum", 0),
                    _ => ("percentile", 50),
                };
                let _ = write!(
                    out,
                    "<calcext:color-scale-entry calcext:value=\"{value}\" calcext:type=\"{kind}\" calcext:color=\"{color}\"/>"
                );
            }
            out.push_str("</calcext:color-scale>");
        }
        Kind::DataBar => {
            let color = colors.first().map_or("#638EC6", String::as_str);
            let _ = write!(
                out,
                "<calcext:data-bar calcext:positive-color=\"{color}\">\
                 <calcext:formatting-entry calcext:value=\"0\" calcext:type=\"auto-minimum\"/>\
                 <calcext:formatting-entry calcext:value=\"0\" calcext:type=\"auto-maximum\"/>\
                 </calcext:data-bar>"
            );
        }
        Kind::IconSet => {
            let icons = rule.icons.as_deref().unwrap_or("3TrafficLights1");
            let count: u32 = icons[..1].parse().unwrap_or(3);
            let _ = write!(out, "<calcext:icon-set calcext:icon-set-type=\"{icons}\">");
            for band in 0..count {
                let _ = write!(
                    out,
                    "<calcext:formatting-entry calcext:value=\"{}\" calcext:type=\"percent\"/>",
                    (band * 100 + count / 2) / count
                );
            }
            out.push_str("</calcext:icon-set>");
        }
        _ => {}
    }
    out.push_str("</calcext:conditional-format>");
}

/// The properties of the page layout `print` sets: a Letter page, turned for landscape, its
/// margins in inches and the pages it fits on.
pub fn page_layout(print: &Print) -> String {
    let (width, height, orientation) = match print.orientation {
        Some(Orientation::Landscape) => ("11in", "8.5in", "landscape"),
        _ => ("8.5in", "11in", "portrait"),
    };
    let margins = print.margins.unwrap_or(DEFAULT_MARGINS);
    let mut out = format!(
        " fo:page-width=\"{width}\" fo:page-height=\"{height}\" style:print-orientation=\"{orientation}\""
    );
    for (side, value) in [
        ("top", margins.top),
        ("bottom", margins.bottom),
        ("left", margins.left),
        ("right", margins.right),
    ] {
        let _ = write!(out, " fo:margin-{side}=\"");
        cells::number(&mut out, value);
        out.push_str("in\"");
    }
    if print.fits() {
        let _ = write!(
            out,
            " style:scale-to-X=\"{}\" style:scale-to-Y=\"{}\"",
            print.fit_width.unwrap_or(0),
            print.fit_height.unwrap_or(0)
        );
    }
    out
}

/// Appends the master page `name` over the page layout `pm<layout>`, with the header and footer
/// of `print`.
pub fn master_page(out: &mut String, name: &str, layout: usize, print: &Print) {
    let _ = write!(
        out,
        "<style:master-page style:name=\"{name}\" style:page-layout-name=\"pm{layout}\">"
    );
    for (tag, text) in [
        ("style:header", &print.header),
        ("style:footer", &print.footer),
    ] {
        if let Some(text) = text {
            let _ = write!(out, "<{tag}>");
            paragraphs(out, text);
            let _ = write!(out, "</{tag}>");
        }
    }
    out.push_str("</style:master-page>");
}

/// `password` as `table:protection-key`: Excel's 16-bit hash, its two bytes high first, in
/// base64.
pub fn protection_key(password: &str) -> String {
    base64(&password_hash(password).to_be_bytes())
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0u32, |word, (at, &byte)| {
            word | u32::from(byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            if at <= chunk.len() {
                out.push(char::from(ALPHABET[(word >> (18 - 6 * at)) as usize & 63]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The index of the `]` that closes the bracket at `from` in `formula`, with `'` escaping one
/// character.
fn group_end(formula: &str, from: usize) -> Option<usize> {
    let mut depth = 0;
    let mut escape = false;
    for (at, c) in formula[from..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\'' => escape = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(from + at);
                }
            }
            _ => {}
        }
    }
    None
}

/// `text` with each `'` escape taken away.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => out.extend(chars.next()),
            c => out.push(c),
        }
    }
    out
}

/// `text` split at each `separator` outside brackets.
fn split_outside(text: &str, separator: char) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start, mut escape) = (0i32, 0, false);
    for (at, c) in text.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\'' => escape = true,
            '[' => depth += 1,
            ']' => depth -= 1,
            c if c == separator && depth == 0 => {
                out.push(&text[start..at]);
                start = at + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&text[start..]);
    out
}

/// The A1 text of the structured reference whose brackets hold `inside`, into the table `table`
/// on the sheet `sheet`, from a formula in the row `row` of the sheet `here`. `None` when it
/// names a column the table does not have.
fn reference(inside: &str, row: u32, here: &str, sheet: &str, table: &Placed) -> Option<String> {
    let at = table.area;
    let items: Vec<String> = if inside.starts_with('[') {
        split_outside(inside, ',')
            .into_iter()
            .map(|item| item.trim().to_owned())
            .collect()
    } else {
        vec![format!("[{inside}]")]
    };
    let mut rows: Option<(u32, u32)> = None;
    let mut columns: Option<(u32, u32)> = None;
    let mut this_row = false;
    let widen = |span: Option<(u32, u32)>, from: u32, to: u32| {
        Some(span.map_or((from, to), |(a, b)| (a.min(from), b.max(to))))
    };
    for item in &items {
        if let Some(special) = item.strip_prefix("[#").and_then(|s| s.strip_suffix(']')) {
            let totals = u32::from(table.is_totaled());
            let (from, to) = match special.to_lowercase().as_str() {
                "this row" => {
                    this_row = true;
                    (row, row)
                }
                "headers" => (at.top, at.top),
                "data" => (at.top + 1, at.bottom),
                "totals" if totals == 1 => (at.bottom + 1, at.bottom + 1),
                "all" => (at.top, at.bottom + totals),
                _ => return None,
            };
            rows = widen(rows, from, to);
            continue;
        }
        for name in split_outside(item, ':') {
            let name = name.trim();
            let name = unescape(name.strip_prefix('[')?.strip_suffix(']')?).to_lowercase();
            let index = table
                .columns()
                .iter()
                .position(|column| column.to_lowercase() == name)?;
            let column = at.left + index as u32;
            columns = widen(columns, column, column);
        }
    }
    let (top, bottom) = rows.unwrap_or((at.top + 1, at.bottom));
    let (left, right) = columns.unwrap_or((at.left, at.right));
    let cell = |column: u32, row: u32| {
        if this_row {
            cells::reference(column, row)
        } else {
            let mut out = String::from("$");
            cells::column_letters(&mut out, column);
            let _ = write!(out, "${row}");
            out
        }
    };
    let mut out = String::new();
    if sheet != here {
        out.push_str(&sheet_ref(sheet));
        out.push('!');
    }
    out.push_str(&cell(left, top));
    if left != right || top != bottom {
        out.push(':');
        out.push_str(&cell(right, bottom));
    }
    Some(out)
}

/// `formula` with each structured reference into a table of `tables`, each with its sheet,
/// written as the A1 range it covers. `row` is the formula's own row on the sheet `here`. A
/// reference that names no table or no column of its table stays as it is.
pub fn resolve(formula: &str, row: u32, here: &str, tables: &[(&str, &Placed)]) -> String {
    let mut out = String::with_capacity(formula.len());
    let mut at = 0;
    while let Some(c) = formula[at..].chars().next() {
        match c {
            '"' => {
                let end = formula[at + 1..]
                    .find('"')
                    .map_or(formula.len(), |close| at + 1 + close + 1);
                out.push_str(&formula[at..end]);
                at = end;
            }
            '\'' => {
                // A quoted sheet name, where `''` is one quote.
                let mut end = at + 1;
                loop {
                    match formula[end..].find('\'') {
                        Some(close) => {
                            end += close + 1;
                            if formula[end..].starts_with('\'') {
                                end += 1;
                                continue;
                            }
                        }
                        None => end = formula.len(),
                    }
                    break;
                }
                out.push_str(&formula[at..end]);
                at = end;
            }
            c if c.is_alphabetic() || c == '_' || c == '\\' => {
                let length = formula[at..]
                    .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.' | '\\')))
                    .unwrap_or(formula.len() - at);
                let name = &formula[at..at + length];
                let next = at + length;
                let lower = name.to_lowercase();
                let resolved = formula[next..]
                    .starts_with('[')
                    .then(|| group_end(formula, next))
                    .flatten()
                    .and_then(|end| {
                        let (sheet, table) = tables
                            .iter()
                            .find(|(_, table)| table.name.to_lowercase() == lower)?;
                        let text = reference(&formula[next + 1..end], row, here, sheet, table)?;
                        Some((text, end + 1))
                    });
                match resolved {
                    Some((text, end)) => {
                        out.push_str(&text);
                        at = end;
                    }
                    None => {
                        out.push_str(name);
                        at = next;
                    }
                }
            }
            c => {
                out.push(c);
                at += c.len_utf8();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Table;
    use crate::{Cell, Row, Value};

    fn shop() -> Placed {
        let mut placed = Placed::new(
            Table {
                range: "B1:C4".to_owned(),
                name: Some("Shop".to_owned()),
                style: None,
                banded: false,
                filter: true,
                totals: Vec::new(),
            },
            1,
            "Data",
        )
        .unwrap();
        let cells = ["", "item", "qty"]
            .iter()
            .map(|name| Cell {
                value: (!name.is_empty()).then(|| Value::Text((*name).to_owned())),
                style: None,
            })
            .collect();
        placed
            .header(&Row {
                cells,
                ..Row::default()
            })
            .unwrap();
        placed
    }

    #[test]
    fn structured_references_become_a1_ranges() {
        let shop = shop();
        let tables = [("Data", &shop)];
        assert_eq!(
            resolve("Shop[[#This Row],[qty]]*2", 3, "Data", &tables),
            "C3*2"
        );
        assert_eq!(
            resolve("SUBTOTAL(109,Shop[qty])", 5, "Data", &tables),
            "SUBTOTAL(109,$C$2:$C$4)"
        );
        assert_eq!(
            resolve("SUM(Shop[qty])", 1, "Other", &tables),
            "SUM(Data!$C$2:$C$4)"
        );
        assert_eq!(
            resolve("\"Shop[qty]\"&Blog[qty]", 1, "Data", &tables),
            "\"Shop[qty]\"&Blog[qty]"
        );
        assert_eq!(
            resolve("Shop[[#This Row],[item]:[qty]]", 2, "Data", &tables),
            "B2:C2"
        );
    }

    #[test]
    fn the_protection_key_is_the_hash_in_base64() {
        assert_eq!(protection_key("password"), "g68=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"M"), "TQ==");
    }
}
