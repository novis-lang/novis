//! The number styles of an ods, as the format codes `crate::format` renders.
//!
//! An ods cell names a cell style (`table:style-name`), or its row or column names a default
//! one, and the cell style names a number style (`style:data-style-name`), itself or through its
//! parents. A number style is a list of elements, one per part of the text it shows, and each
//! element is written as the format code part that shows the same thing:
//!
//! | ods element | Format code |
//! |---|---|
//! | `number:number` | `0`, `#,##0.00`, and `General` with no `decimal-places` |
//! | `number:scientific-number` | `0.00E+00` |
//! | `number:fraction` | `# ?/?`, or `# ?/8` with a `denominator-value` |
//! | `number:day`, `month`, `year`, `day-of-week` | `d`, `m` or `mmm`, `yy`, `ddd`, and each doubled or longer for `style="long"` |
//! | `number:hours`, `minutes`, `seconds`, `am-pm` | `h`, `m`, `s` with its decimals, `AM/PM` |
//! | `number:text`, `number:currency-symbol` | the text, quoted |
//! | `number:text-content` | `@` |
//!
//! A time style with `number:truncate-on-overflow="false"` shows elapsed time, so its first part
//! is `[h]` or `[mm]`. A `%` in a percentage style is the percent sign, which multiplies by 100,
//! and every other text is quoted. A `style:map` adds a section: `value()>=0` makes the mapped
//! style the first of two sections, `value()>0` and `value()<0` the first two of three, and any
//! other condition is a `[>5]` section. A colour, an era, a quarter and a week number are not
//! shown in a cell's text, and are not read.
//!
//! What it spends, per read: the code of each number style and the name of each cell style,
//! while `content.xml` is read.

use std::collections::HashMap;
use std::sync::Arc;

use super::xml::{self, Token};
use crate::Error;

/// A number style read: its own code, and the conditions and style names of its maps.
#[derive(Default)]
struct Number {
    code: String,
    maps: Vec<(String, String)>,
}

/// The number styles and cell styles of an ods's parts.
#[derive(Default)]
pub struct Styles {
    numbers: HashMap<String, Number>,
    /// Each cell style's number style and parent style.
    cells: HashMap<String, (Option<String>, Option<String>)>,
}

/// A number style being read.
struct Open {
    name: String,
    number: Number,
    percent: bool,
    /// Whether the first time part shows elapsed time, until a part has used it.
    elapsed: bool,
    /// The text of a `number:text` or `number:currency-symbol` being read.
    text: Option<String>,
}

fn count(attrs: &[u8], name: &[u8]) -> Option<usize> {
    xml::attr(attrs, name)
        .and_then(|raw| std::str::from_utf8(raw).ok()?.trim().parse::<usize>().ok())
        .map(|n| n.min(30))
}

fn long(attrs: &[u8]) -> bool {
    xml::attr(attrs, b"style") == Some(b"long")
}

/// Appends the text `text` as a literal: a percent sign of a percentage style and a space as
/// themselves, and the rest in quotes.
fn literal(text: &str, percent: bool, out: &mut String) {
    let mut quoted = false;
    for c in text.chars() {
        let bare = c == ' ' || (c == '%' && percent);
        if bare || c == '"' {
            if quoted {
                out.push('"');
                quoted = false;
            }
            if c == '"' {
                out.push_str("\\\"");
            } else {
                out.push(c);
            }
            continue;
        }
        if !quoted {
            out.push('"');
            quoted = true;
        }
        out.push(c);
    }
    if quoted {
        out.push('"');
    }
}

/// Appends the integer and decimal digits a `number:number` or `number:scientific-number` shows.
fn digits(attrs: &[u8], out: &mut String) {
    let Some(decimals) = count(attrs, b"decimal-places") else {
        out.push_str("General");
        return;
    };
    let integers = count(attrs, b"min-integer-digits").unwrap_or(1);
    if xml::attr(attrs, b"grouping") == Some(b"true") {
        out.push_str("#,");
        out.extend(std::iter::repeat_n('#', 3usize.saturating_sub(integers)));
    } else if integers == 0 {
        out.push('#');
    }
    out.extend(std::iter::repeat_n('0', integers));
    if decimals > 0 {
        let shown = count(attrs, b"min-decimal-places").map_or(decimals, |n| n.min(decimals));
        out.push('.');
        out.extend(std::iter::repeat_n('0', shown));
        out.extend(std::iter::repeat_n('#', decimals - shown));
    }
}

impl Open {
    /// Appends the code of the number style element `tag`.
    fn part(&mut self, tag: &[u8], attrs: &[u8]) {
        let out = &mut self.number.code;
        let pick = |long_form: &'static str, short: &'static str| {
            if long(attrs) { long_form } else { short }
        };
        match tag {
            b"number" => digits(attrs, out),
            b"scientific-number" => {
                digits(attrs, out);
                out.push_str("E+");
                let exponent = count(attrs, b"min-exponent-digits").unwrap_or(2).max(1);
                out.extend(std::iter::repeat_n('0', exponent));
            }
            b"fraction" => {
                if let Some(integers) = count(attrs, b"min-integer-digits") {
                    if integers == 0 {
                        out.push('#');
                    }
                    out.extend(std::iter::repeat_n('0', integers));
                    out.push(' ');
                }
                let numerator = count(attrs, b"min-numerator-digits").unwrap_or(1).max(1);
                out.extend(std::iter::repeat_n('?', numerator));
                out.push('/');
                match count(attrs, b"denominator-value") {
                    Some(value) if value > 0 => out.push_str(&value.to_string()),
                    _ => {
                        let denominator =
                            count(attrs, b"min-denominator-digits").unwrap_or(1).max(1);
                        out.extend(std::iter::repeat_n('?', denominator));
                    }
                }
            }
            b"day" => out.push_str(pick("dd", "d")),
            b"month" if xml::attr(attrs, b"textual") == Some(b"true") => {
                out.push_str(pick("mmmm", "mmm"));
            }
            b"month" => out.push_str(pick("mm", "m")),
            b"year" => out.push_str(pick("yyyy", "yy")),
            b"day-of-week" => out.push_str(pick("dddd", "ddd")),
            b"hours" | b"minutes" => {
                let part = match (tag, long(attrs)) {
                    (b"hours", true) => "hh",
                    (b"hours", false) => "h",
                    (_, true) => "mm",
                    (_, false) => "m",
                };
                if std::mem::take(&mut self.elapsed) {
                    out.push('[');
                    out.push_str(part);
                    out.push(']');
                } else {
                    out.push_str(part);
                }
            }
            b"seconds" => {
                out.push_str(pick("ss", "s"));
                let decimals = count(attrs, b"decimal-places").unwrap_or(0).min(3);
                if decimals > 0 {
                    out.push('.');
                    out.extend(std::iter::repeat_n('0', decimals));
                }
            }
            b"am-pm" => out.push_str("AM/PM"),
            b"text-content" => out.push('@'),
            _ => {}
        }
    }
}

/// Whether `tag` is the local name of a number style.
fn is_number_style(tag: &[u8]) -> bool {
    matches!(
        tag,
        b"number-style"
            | b"currency-style"
            | b"percentage-style"
            | b"date-style"
            | b"time-style"
            | b"boolean-style"
            | b"text-style"
    )
}

impl Styles {
    /// Adds the number styles and cell styles of the part `bytes`, up to its body.
    pub fn add(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let mut open: Option<Open> = None;
        let mut at = 0;
        while let Some((token, size)) = xml::next(&bytes[at..])? {
            at += size;
            match token {
                Token::Open { name, attrs, empty } => {
                    let tag = xml::local(name);
                    if let Some(style) = open.as_mut() {
                        match tag {
                            b"text" | b"currency-symbol" if !empty => {
                                style.text = Some(String::new());
                            }
                            b"map" => {
                                let condition = xml::attr_text(attrs, b"condition")?;
                                let apply = xml::attr_text(attrs, b"apply-style-name")?;
                                if let (Some(condition), Some(apply)) = (condition, apply) {
                                    style.number.maps.push((condition, apply));
                                }
                            }
                            _ => style.part(tag, attrs),
                        }
                        continue;
                    }
                    match tag {
                        b"body" => return Ok(()),
                        _ if is_number_style(tag) => {
                            let Some(name) = xml::attr_text(attrs, b"name")? else {
                                continue;
                            };
                            let style = Open {
                                name,
                                number: Number::default(),
                                percent: tag == b"percentage-style",
                                elapsed: tag == b"time-style"
                                    && xml::attr(attrs, b"truncate-on-overflow") == Some(b"false"),
                                text: None,
                            };
                            if empty {
                                self.numbers.insert(style.name, style.number);
                            } else {
                                open = Some(style);
                            }
                        }
                        b"style" if xml::attr(attrs, b"family") == Some(b"table-cell") => {
                            if let Some(name) = xml::attr_text(attrs, b"name")? {
                                let data = xml::attr_text(attrs, b"data-style-name")?;
                                let parent = xml::attr_text(attrs, b"parent-style-name")?;
                                self.cells.insert(name, (data, parent));
                            }
                        }
                        _ => {}
                    }
                }
                Token::Text(raw) => {
                    if let Some(text) = open.as_mut().and_then(|style| style.text.as_mut()) {
                        xml::decode(raw, text)?;
                    }
                }
                Token::Close(name) => {
                    let tag = xml::local(name);
                    if is_number_style(tag) {
                        if let Some(style) = open.take() {
                            self.numbers.insert(style.name, style.number);
                        }
                    } else if matches!(tag, b"text" | b"currency-symbol")
                        && let Some(style) = open.as_mut()
                        && let Some(text) = style.text.take()
                    {
                        literal(&text, style.percent, &mut style.number.code);
                    }
                }
                Token::Raw(_) | Token::Skip => {}
            }
        }
        Ok(())
    }

    /// The format code of the number style `name`, with a section for each of its maps.
    fn code(&self, name: &str) -> Option<String> {
        let own = self.numbers.get(name)?;
        let section = |number: &Number| {
            if number.code.is_empty() {
                "General".to_owned()
            } else {
                number.code.clone()
            }
        };
        let mapped: Vec<(String, String)> = own
            .maps
            .iter()
            .filter_map(|(condition, apply)| {
                let condition: String = condition.chars().filter(|c| !c.is_whitespace()).collect();
                let condition = condition
                    .strip_prefix("value()")
                    .unwrap_or(&condition)
                    .to_owned();
                Some((condition, section(self.numbers.get(apply)?)))
            })
            .collect();
        let own = section(own);
        let conditions: Vec<&str> = mapped.iter().map(|(c, _)| c.as_str()).collect();
        Some(match conditions[..] {
            [] => own,
            [">=0"] => format!("{};{own}", mapped[0].1),
            [">0", "<0"] => format!("{};{};{own}", mapped[0].1, mapped[1].1),
            _ => {
                let mut code = String::new();
                for (condition, section) in &mapped {
                    code.push('[');
                    code.push_str(condition);
                    code.push(']');
                    code.push_str(section);
                    code.push(';');
                }
                code.push_str(&own);
                code
            }
        })
    }

    /// The format code of each cell style that has a number style, itself or through its
    /// parents.
    pub fn codes(&self) -> HashMap<String, Arc<str>> {
        let mut codes = HashMap::new();
        for name in self.cells.keys() {
            let mut at = name;
            // A chain of parents is short in any real file, and a loop in one ends here.
            for _ in 0..16 {
                let Some((data, parent)) = self.cells.get(at) else {
                    break;
                };
                if let Some(data) = data {
                    if let Some(code) = self.code(data) {
                        codes.insert(name.clone(), Arc::from(code));
                    }
                    break;
                }
                match parent {
                    Some(parent) => at = parent,
                    None => break,
                }
            }
        }
        codes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The code the cell style `ce1` of the automatic styles `styles` has.
    fn code(styles: &str) -> Option<String> {
        let part = format!(
            r#"<office:document-content><office:automatic-styles>{styles}</office:automatic-styles><office:body/></office:document-content>"#
        );
        let mut found = Styles::default();
        found.add(part.as_bytes()).unwrap();
        found.codes().get("ce1").map(|code| code.to_string())
    }

    fn cell(data: &str) -> String {
        format!(
            r#"<style:style style:name="ce1" style:family="table-cell" style:data-style-name="{data}"/>"#
        )
    }

    #[test]
    fn each_number_style_element_is_its_format_code_part() {
        let number = r#"<number:number-style style:name="N1"><number:number number:decimal-places="2" number:min-integer-digits="1" number:grouping="true"/></number:number-style>"#;
        assert_eq!(
            code(&(number.to_owned() + &cell("N1"))).as_deref(),
            Some("#,##0.00")
        );
        let date = r#"<number:date-style style:name="D1"><number:day number:style="long"/><number:text>.</number:text><number:month number:textual="true"/><number:text> </number:text><number:year number:style="long"/></number:date-style>"#;
        assert_eq!(
            code(&(date.to_owned() + &cell("D1"))).as_deref(),
            Some(r#"dd"."mmm yyyy"#)
        );
        let elapsed = r#"<number:time-style style:name="T1" number:truncate-on-overflow="false"><number:hours/><number:text>:</number:text><number:minutes number:style="long"/></number:time-style>"#;
        assert_eq!(
            code(&(elapsed.to_owned() + &cell("T1"))).as_deref(),
            Some(r#"[h]":"mm"#)
        );
        let percent = r#"<number:percentage-style style:name="P1"><number:number number:decimal-places="0" number:min-integer-digits="1"/><number:text>%</number:text></number:percentage-style>"#;
        assert_eq!(
            code(&(percent.to_owned() + &cell("P1"))).as_deref(),
            Some("0%")
        );
    }

    #[test]
    fn a_map_is_a_section_and_a_parent_gives_its_number_style() {
        let negative = r#"<number:number-style style:name="N1P0"><number:number number:decimal-places="1" number:min-integer-digits="1"/></number:number-style><number:number-style style:name="N1"><number:text>-</number:text><number:number number:decimal-places="1" number:min-integer-digits="1"/><style:map style:condition="value()&gt;=0" style:apply-style-name="N1P0"/></number:number-style>"#;
        let styles = format!(
            r#"{negative}<style:style style:name="base" style:family="table-cell" style:data-style-name="N1"/><style:style style:name="ce1" style:family="table-cell" style:parent-style-name="base"/>"#
        );
        assert_eq!(code(&styles).as_deref(), Some(r#"0.0;"-"0.0"#));
        // A cell style with no number style anywhere has no code.
        assert_eq!(
            code(r#"<style:style style:name="ce1" style:family="table-cell"/>"#),
            None
        );
    }
}
