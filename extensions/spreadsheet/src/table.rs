//! Excel tables: the part `xl/tables/table<n>.xml` and its totals row
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! [`Placed::new`] checks a table the program adds: its range, its name and its style. Its top
//! row is its header row, and each `tableColumn` is named by the text of its header cell, which
//! Excel needs to equal. The writer keeps only the sheet's header row once it is written, so a
//! table added after its first rows starts at that header row, and a table added before its
//! header row takes the names when that row is written ([`Placed::header`]). Every header cell of
//! the table needs a name, and no two names may be equal, ignoring case.
//!
//! **The totals row is the row after the table** ([`Placed::totals_row`]). A table added after
//! its last row writes it at once, so its last row is the last row written. A table added before
//! its last row writes it right after that row, and the program's next row comes after the
//! totals row. A sheet that ends before a table's last row is an error. A function is
//! `SUBTOTAL` with Excel's code for it, which skips the rows a filter hides, and a formula is
//! written as given. The first column with no total is labelled `Total`.
//!
//! **Structured references** ([`expand`]): a formula in a table's own cells may write `[@qty]`
//! for the row's own `qty` and `[qty]` for the whole column. Excel stores them qualified by the
//! table's name and with `@` in its long form, as `Shop[[#This Row],[qty]]` and `Shop[qty]`
//! (`tests/excel.rs` checks it against a calculated column Excel wrote). The writer rewrites them
//! so in the totals row and in every row written after the table is added, and `Shop[@qty]`
//! becomes `Shop[[#This Row],[qty]]`. A row written before its table is added keeps its formula
//! text as it was given.
//!
//! [`Placed::write`] is the table's part. Its id and its default name `Table<id>` count across
//! the workbook. Memory: per table, its range, its name and its column names until the sheet
//! ends.

use std::fmt::Write as _;

use crate::tail::{Area, area};
use crate::{Cell, Error, Row, Value, cells, xml};

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// The style Excel gives a new table.
const DEFAULT_STYLE: &str = "TableStyleMedium9";

/// Excel's longest name.
const MAX_NAME: usize = 255;

/// The functions a totals row computes for one column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    Sum,
    Average,
    Count,
    Min,
    Max,
}

impl Function {
    /// The name `totalsRowFunction` gives it.
    fn name(self) -> &'static str {
        match self {
            Function::Sum => "sum",
            Function::Average => "average",
            Function::Count => "count",
            Function::Min => "min",
            Function::Max => "max",
        }
    }

    /// The `SUBTOTAL` code that skips the rows a filter hides.
    fn code(self) -> u32 {
        match self {
            Function::Average => 101,
            Function::Count => 103,
            Function::Max => 104,
            Function::Min => 105,
            Function::Sum => 109,
        }
    }
}

/// The total of the column whose header is `column`: a function, or a formula with no leading
/// `=`.
#[derive(Debug, Clone, PartialEq)]
pub struct Total {
    pub column: String,
    pub function: Option<Function>,
    pub formula: Option<String>,
}

/// A table the program adds over `range`, its top row being its header row.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub range: String,
    pub name: Option<String>,
    pub style: Option<String>,
    pub banded: bool,
    pub filter: bool,
    pub totals: Vec<Total>,
}

/// A table checked and numbered, until its sheet ends.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub id: usize,
    pub name: String,
    /// Its header row and its data rows, without the totals row.
    pub area: Area,
    style: String,
    banded: bool,
    filter: bool,
    /// Its column names, empty until its header row is written.
    columns: Vec<String>,
    totals: Vec<Total>,
    /// Whether its totals row is written.
    totaled: bool,
}

/// Whether `name` is a name Excel allows for a table.
fn check_name(name: &str) -> Result<(), Error> {
    let mut chars = name.chars();
    let first = chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '\\');
    if !first
        || !chars.all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        || name.chars().count() > MAX_NAME
    {
        return Err(Error::Invalid(format!(
            "`{name}` is not a table name Excel allows. A name starts with a letter or `_`, has only letters, digits, `_` and `.`, and has at most 255 characters."
        )));
    }
    let upper = name.to_ascii_uppercase();
    let r1c1 = |text: &str| {
        let rest = text.strip_prefix('R').unwrap_or(text);
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        let rest = rest.strip_prefix('C').unwrap_or(rest);
        rest.bytes().all(|b| b.is_ascii_digit())
    };
    if area(&upper).is_some() || r1c1(&upper) {
        return Err(Error::Invalid(format!(
            "`{name}` is a cell reference, so it cannot be a table name."
        )));
    }
    Ok(())
}

/// The style `style` names, in Excel's case: `TableStyleLight1` to `21`, `TableStyleMedium1` to
/// `28` or `TableStyleDark1` to `11`.
fn style_name(style: &str) -> Result<String, Error> {
    let lower = style.to_ascii_lowercase();
    for (kind, last) in [("Light", 21), ("Medium", 28), ("Dark", 11)] {
        let prefix = format!("tablestyle{}", kind.to_ascii_lowercase());
        if let Some(digits) = lower.strip_prefix(&prefix)
            && !digits.starts_with('0')
            && digits.bytes().all(|b| b.is_ascii_digit())
            && let Ok(number) = digits.parse::<u32>()
            && (1..=last).contains(&number)
        {
            return Ok(format!("TableStyle{kind}{number}"));
        }
    }
    Err(Error::Invalid(format!(
        "`{style}` is not a table style. A style is `TableStyleLight1` to `TableStyleLight21`, `TableStyleMedium1` to `TableStyleMedium28` or `TableStyleDark1` to `TableStyleDark11`."
    )))
}

/// The text a header cell names its column with, or `None` for an empty cell.
fn header_text(cell: Option<&Cell>) -> Option<String> {
    let text = match cell?.value.as_ref()? {
        Value::Text(text) => text.clone(),
        Value::Integer(value) => value.to_string(),
        Value::Decimal(text) => text.clone(),
        Value::Number(value) => {
            let mut out = String::new();
            cells::number(&mut out, *value);
            out
        }
        Value::Rich(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
        _ => return None,
    };
    (!text.trim().is_empty()).then_some(text)
}

/// A column name as a structured reference needs it: `'` before each `[`, `]`, `#` and `'`.
fn escaped(column: &str) -> String {
    let mut out = String::with_capacity(column.len());
    for c in column.chars() {
        if matches!(c, '[' | ']' | '#' | '\'') {
            out.push('\'');
        }
        out.push(c);
    }
    out
}

impl Placed {
    /// Checks `table` as the table numbered `id` in the workbook, on the sheet `sheet`.
    pub fn new(table: Table, id: usize, sheet: &str) -> Result<Self, Error> {
        let at = area(&table.range).ok_or_else(|| {
            Error::Invalid(format!(
                "The table range `{}` is not a range of cells, such as `A1:D10`.",
                table.range
            ))
        })?;
        if at.bottom == at.top {
            return Err(Error::Invalid(format!(
                "The table range `{}` on the sheet `{sheet}` is one row. A table has its header row and at least one row below it.",
                table.range
            )));
        }
        let name = match table.name {
            Some(name) => {
                check_name(&name)?;
                name
            }
            None => format!("Table{id}"),
        };
        let style = match &table.style {
            Some(style) => style_name(style)?,
            None => DEFAULT_STYLE.to_owned(),
        };
        for total in &table.totals {
            if total.function.is_some() == total.formula.is_some() {
                return Err(Error::Invalid(format!(
                    "The total of the column `{}` in the table `{name}` needs a function or a formula, and not both.",
                    total.column
                )));
            }
        }
        Ok(Placed {
            id,
            name,
            area: at,
            style,
            banded: table.banded,
            filter: table.filter,
            columns: Vec::new(),
            totals: table.totals,
            totaled: false,
        })
    }

    /// Whether its header row is written and its columns named.
    pub fn has_header(&self) -> bool {
        !self.columns.is_empty()
    }

    /// Whether it has totals to write.
    pub fn has_totals(&self) -> bool {
        !self.totals.is_empty()
    }

    /// Names its columns from `row`, its header row.
    pub fn header(&mut self, row: &Row) -> Result<(), Error> {
        let mut names: Vec<String> = Vec::new();
        for column in self.area.left..=self.area.right {
            let at = cells::reference(column, self.area.top);
            let Some(name) = header_text(row.cells.get(column as usize)) else {
                return Err(Error::Invalid(format!(
                    "The header cell `{at}` of the table `{}` is empty. Every header cell of a table needs a name.",
                    self.name
                )));
            };
            let lower = name.to_lowercase();
            if names.iter().any(|other| other.to_lowercase() == lower) {
                return Err(Error::Invalid(format!(
                    "The header cell `{at}` of the table `{}` repeats the name `{name}`. The names of a table's columns are different.",
                    self.name
                )));
            }
            names.push(name);
        }
        self.columns = names;
        Ok(())
    }

    /// Its column names, empty until its header row is written.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Whether it shows filter buttons in its header row.
    pub fn filter(&self) -> bool {
        self.filter
    }

    /// Whether its totals row is written.
    pub fn is_totaled(&self) -> bool {
        self.totaled
    }

    /// Whether its totals row is still to write after its last row.
    pub fn owes_totals(&self) -> bool {
        self.has_totals() && !self.totaled
    }

    /// Records that its totals row is written.
    pub fn totaled(&mut self) {
        self.totaled = true;
    }

    /// The totals row, its formulas qualified by the table's name. Its header row is written.
    pub fn totals_row(&self) -> Result<Row, Error> {
        let mut cells = vec![Cell::default(); self.area.right as usize + 1];
        for total in &self.totals {
            let Some(at) = self.columns.iter().position(|name| *name == total.column) else {
                return Err(Error::Invalid(format!(
                    "The table `{}` has no column `{}`. Its columns are {}.",
                    self.name,
                    total.column,
                    self.columns
                        .iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            };
            let formula = match (total.function, &total.formula) {
                (Some(function), _) => format!(
                    "SUBTOTAL({},{}[{}])",
                    function.code(),
                    self.name,
                    escaped(&total.column)
                ),
                (None, Some(formula)) => {
                    expand(formula.strip_prefix('=').unwrap_or(formula), &self.name)
                }
                (None, None) => continue,
            };
            cells[self.area.left as usize + at] = Cell {
                value: Some(Value::Formula {
                    formula,
                    cached_text: None,
                    cached_number: None,
                }),
                style: None,
            };
        }
        if let Some(at) = self.label() {
            cells[self.area.left as usize + at] = Cell {
                value: Some(Value::Text("Total".to_owned())),
                style: None,
            };
        }
        Ok(Row {
            cells,
            ..Row::default()
        })
    }

    /// The column, from the table's first, that the totals row labels `Total`.
    fn label(&self) -> Option<usize> {
        (0..self.columns.len()).find(|&at| {
            !self
                .totals
                .iter()
                .any(|total| total.column == self.columns[at])
        })
    }

    /// Whether the cell at `column` and `row` is one of its data rows or its totals row.
    pub fn covers(&self, column: u32, row: u32) -> bool {
        let bottom = self.area.bottom + u32::from(self.totaled);
        (self.area.left..=self.area.right).contains(&column)
            && (self.area.top + 1..=bottom).contains(&row)
    }

    /// The range it covers with its totals row.
    pub fn whole(&self) -> Area {
        Area {
            bottom: self.area.bottom + u32::from(self.totaled),
            ..self.area
        }
    }

    /// Its part, `xl/tables/table<id>.xml`. A column whose header row was never written is
    /// named `Column<n>`, as Excel names one.
    pub fn write(&self) -> String {
        let range = |at: &Area| {
            format!(
                "{}:{}",
                cells::reference(at.left, at.top),
                cells::reference(at.right, at.bottom)
            )
        };
        let mut out = String::from(crate::writer::XML_HEAD);
        let _ = write!(
            out,
            "<table xmlns=\"{MAIN}\" id=\"{}\" name=\"{}\" displayName=\"{}\" ref=\"{}\"",
            self.id,
            self.name,
            self.name,
            range(&self.whole())
        );
        if self.totaled {
            out.push_str(" totalsRowCount=\"1\"");
        } else {
            out.push_str(" totalsRowShown=\"0\"");
        }
        out.push('>');
        if self.filter {
            let _ = write!(out, "<autoFilter ref=\"{}\"/>", range(&self.area));
        }
        let count = self.area.right - self.area.left + 1;
        let label = if self.totaled { self.label() } else { None };
        let _ = write!(out, "<tableColumns count=\"{count}\">");
        for at in 0..count as usize {
            let name = self
                .columns
                .get(at)
                .cloned()
                .unwrap_or_else(|| format!("Column{}", at + 1));
            let _ = write!(out, "<tableColumn id=\"{}\" name=\"", at + 1);
            xml::attr(&mut out, &name);
            out.push('"');
            let total = self
                .totals
                .iter()
                .find(|total| self.totaled && total.column == name);
            match total {
                Some(Total {
                    function: Some(function),
                    ..
                }) => {
                    let _ = write!(out, " totalsRowFunction=\"{}\"/>", function.name());
                }
                Some(Total {
                    formula: Some(formula),
                    ..
                }) => {
                    // Inside the table's own part, Excel writes a reference to the table's
                    // columns with no table name: `SUM([qty])`. `[@qty]` is written in its
                    // long form, `[[#This Row],[qty]]`, as Excel stores it.
                    out.push_str(" totalsRowFunction=\"custom\"><totalsRowFormula>");
                    xml::text(
                        &mut out,
                        &expand(formula.strip_prefix('=').unwrap_or(formula), ""),
                    );
                    out.push_str("</totalsRowFormula></tableColumn>");
                }
                _ if label == Some(at) => out.push_str(" totalsRowLabel=\"Total\"/>"),
                _ => out.push_str("/>"),
            }
        }
        let _ = write!(
            out,
            "</tableColumns><tableStyleInfo name=\"{}\" showFirstColumn=\"0\" showLastColumn=\"0\" showRowStripes=\"{}\" showColumnStripes=\"0\"/></table>",
            self.style,
            u8::from(self.banded)
        );
        out
    }
}

/// `formula` with each structured reference that names no table qualified by `table`: `[@qty]`
/// and `[@[unit price]]` become `table[[#This Row],[qty]]`, and `[qty]` becomes `table[qty]`.
/// A reference that already names a table keeps its name, and its `@` is expanded too:
/// `Blog[@qty]` becomes `Blog[[#This Row],[qty]]`. Text inside quotes stays as it is. An empty
/// `table` names none, so only `[@qty]` changes, to `[[#This Row],[qty]]`.
pub fn expand(formula: &str, table: &str) -> String {
    let mut out = String::with_capacity(formula.len());
    let mut chars = formula.char_indices().peekable();
    let mut previous: Option<char> = None;
    while let Some((at, c)) = chars.next() {
        if c == '"' {
            out.push(c);
            for (_, inner) in chars.by_ref() {
                out.push(inner);
                if inner == '"' {
                    break;
                }
            }
            previous = Some('"');
            continue;
        }
        let named =
            previous.is_some_and(|p| p.is_alphanumeric() || matches!(p, '_' | '.' | ']' | '\''));
        // A reference that names its table keeps the name, and only its `@` is expanded.
        let this_row = formula[at + c.len_utf8()..].starts_with('@');
        if c != '[' || (named && !this_row) {
            out.push(c);
            previous = Some(c);
            continue;
        }
        // The bracketed group from `at`, its own brackets included, with `'` escaping one
        // character.
        let mut depth = 0;
        let mut end = None;
        let mut escape = false;
        for (index, inner) in formula[at..].char_indices() {
            if escape {
                escape = false;
                continue;
            }
            match inner {
                '\'' => escape = true,
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(at + index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            out.push_str(&formula[at..]);
            return out;
        };
        let inside = &formula[at + 1..end];
        if !named {
            out.push_str(table);
        }
        match inside.strip_prefix('@') {
            Some(column) if column.starts_with('[') => {
                let _ = write!(out, "[[#This Row],{column}]");
            }
            Some(column) => {
                let _ = write!(out, "[[#This Row],[{column}]]");
            }
            None => out.push_str(&formula[at..=end]),
        }
        while chars.peek().is_some_and(|&(index, _)| index <= end) {
            chars.next();
        }
        previous = Some(']');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_references_are_qualified_by_the_table() {
        assert_eq!(
            expand("[@qty]*[@[unit price]]", "Shop"),
            "Shop[[#This Row],[qty]]*Shop[[#This Row],[unit price]]"
        );
        assert_eq!(expand("SUM([qty])/2", "Shop"), "SUM(Shop[qty])/2");
        assert_eq!(expand("Blog[qty]+\"[qty]\"", "Shop"), "Blog[qty]+\"[qty]\"");
        assert_eq!(expand("[#Totals]", "Shop"), "Shop[#Totals]");
        assert_eq!(
            expand("SUM(Blog[@[a]:[c]])", "Shop"),
            "SUM(Blog[[#This Row],[a]:[c]])"
        );
    }

    #[test]
    fn a_style_is_one_of_excels_and_a_name_is_not_a_cell() {
        assert_eq!(style_name("tablestylelight9").unwrap(), "TableStyleLight9");
        assert!(style_name("TableStyleLight22").is_err());
        assert!(style_name("TableStyleMedium09").is_err());
        assert!(check_name("Orders_2024").is_ok());
        for name in ["A1", "R1C1", "R", "C", "1Shop", "Shop Orders"] {
            assert!(check_name(name).is_err(), "{name}");
        }
    }
}
