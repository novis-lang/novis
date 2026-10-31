//! The evaluator: a workbook's formulas computed in the guest, once, in dependency order
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! A [`Book`] is a sparse grid per sheet: one [`Value`] per non-empty cell in a `BTreeMap` by
//! row and column, so a whole-column reference (`A:A`) walks only the cells that exist. Each
//! formula cell also has its text, and the value it had in the file until it is computed. The
//! workbook's defined names and tables are kept as given.
//!
//! [`Book::evaluate`] parses every formula ([`parse`]), collects the formula cells each one
//! reads, and orders them by Tarjan's strongly connected components, walked with an explicit
//! stack, so a long chain of cells cannot overflow the guest's stack. A component of more than
//! one cell, or a cell that reads itself, is circular: none of its cells is computed. Every
//! other cell is computed after the cells it reads, and its value replaces the stored one.
//!
//! What was not computed is returned as a [`Missed`] for each cell, sorted by sheet, row and
//! column, and that cell keeps its stored value, which the cells that read it then read:
//! - `function`: a function outside the table ([`functions`]), with its name.
//! - `reference`: a reference to a sheet the workbook does not have, to another workbook, a 3D
//!   reference, a union, an intersection, an array constant, or a result of more than one cell,
//!   which Excel spills. The text is the reference.
//! - `circular`: a cell in a cycle, with its formula.
//! - `token`: formula text the parser cannot read, or a formula the reader could not decode,
//!   with its text.
//!
//! Nothing here does I/O or follows a reference out of the workbook. What it spends, per
//! evaluation: the grid, one parsed tree per formula and per name, the list of cells each
//! formula reads, and the order. The work grows with the cells and the references.

mod date;
pub mod functions;
mod lookup;
mod math;
pub mod moved;
pub mod parse;
mod text;
pub mod value;

use std::collections::BTreeMap;

use crate::read::CellError;
use parse::{Area, Expr, Item, Rect, Structured, Unreadable};
pub use value::Value;

/// Why a cell was not computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Function,
    Reference,
    Circular,
    Token,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Reference => "reference",
            Self::Circular => "circular",
            Self::Token => "token",
        }
    }
}

/// One cell not computed: `cell` as `Sheet!A1`, why, and the text that names what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missed {
    pub cell: String,
    pub kind: Kind,
    pub text: String,
}

/// Why one evaluation stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Miss {
    pub kind: Kind,
    pub text: String,
}

/// What an expression gives: a value, or a reference to cells of a sheet.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Value(Value),
    Area(usize, Rect),
}

/// A defined name: its name, the sheet it is local to, and its formula text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub name: String,
    pub sheet: Option<String>,
    pub text: String,
}

/// A table: its name, its sheet, every cell it covers, whether it has a header and a totals
/// row, and its columns' names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub sheet: String,
    pub rect: Rect,
    pub header: bool,
    pub totals: bool,
    pub columns: Vec<String>,
}

#[derive(Debug, Default)]
struct Sheet {
    name: String,
    cells: BTreeMap<(u32, u32), Value>,
    formulas: BTreeMap<(u32, u32), usize>,
    hidden: std::collections::BTreeSet<u32>,
}

#[derive(Debug)]
struct Formula {
    sheet: usize,
    row: u32,
    col: u32,
    text: Option<String>,
    /// Whether [`Book::evaluate`] computed it.
    computed: bool,
}

/// A name ready to evaluate: the sheet it is local to, its name in lower case, and its tree.
struct Parsed {
    scope: Option<usize>,
    name: String,
    expr: Result<Expr, Unreadable>,
}

/// The deepest chain of names one formula may follow.
const NAME_DEPTH: usize = 64;

static BLANK: Value = Value::Blank;

/// A workbook to evaluate.
#[derive(Debug, Default)]
pub struct Book {
    sheets: Vec<Sheet>,
    formulas: Vec<Formula>,
    names: Vec<Name>,
    tables: Vec<Table>,
    date1904: bool,
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts date serials from 1904-01-01, as a workbook with that setting does.
    pub fn use_1904(&mut self) {
        self.date1904 = true;
    }

    /// The sheet named `name`, added after the others when the book has none of that name.
    pub fn sheet(&mut self, name: &str) -> usize {
        if let Some(at) = self.find(name) {
            return at;
        }
        self.sheets.push(Sheet {
            name: name.to_owned(),
            ..Sheet::default()
        });
        self.sheets.len() - 1
    }

    /// The sheet named `name`, in any case.
    pub fn find(&self, name: &str) -> Option<usize> {
        let name = name.to_lowercase();
        self.sheets
            .iter()
            .position(|s| s.name.to_lowercase() == name)
    }

    /// Sets a cell's value. A blank empties the cell.
    pub fn set(&mut self, sheet: usize, row: u32, col: u32, value: Value) {
        let cells = &mut self.sheets[sheet].cells;
        if value == Value::Blank {
            cells.remove(&(row, col));
        } else {
            cells.insert((row, col), value);
        }
    }

    /// Sets a formula cell: its text, `None` when the reader could not decode it, and the value
    /// the file stored for it.
    pub fn formula(&mut self, sheet: usize, row: u32, col: u32, text: Option<&str>, stored: Value) {
        self.set(sheet, row, col, stored);
        let formula = Formula {
            sheet,
            row,
            col,
            text: text.map(str::to_owned),
            computed: false,
        };
        match self.sheets[sheet].formulas.get(&(row, col)) {
            Some(&at) => self.formulas[at] = formula,
            None => {
                self.sheets[sheet]
                    .formulas
                    .insert((row, col), self.formulas.len());
                self.formulas.push(formula);
            }
        }
    }

    /// Marks a row the file hides, which `SUBTOTAL` reads.
    pub fn hide_row(&mut self, sheet: usize, row: u32) {
        self.sheets[sheet].hidden.insert(row);
    }

    pub fn define(&mut self, name: Name) {
        self.names.push(name);
    }

    pub fn table(&mut self, table: Table) {
        self.tables.push(table);
    }

    /// A cell's value: computed when it was, and stored otherwise.
    pub fn value(&self, sheet: usize, row: u32, col: u32) -> &Value {
        self.sheets
            .get(sheet)
            .and_then(|s| s.cells.get(&(row, col)))
            .unwrap_or(&BLANK)
    }

    /// The computed value of a formula cell, or `None` for a cell that is not a formula or
    /// was not computed.
    pub fn computed(&self, sheet: usize, row: u32, col: u32) -> Option<&Value> {
        let s = self.sheets.get(sheet)?;
        let &at = s.formulas.get(&(row, col))?;
        self.formulas[at]
            .computed
            .then(|| s.cells.get(&(row, col)).unwrap_or(&BLANK))
    }

    /// The value at `reference`, `Sheet!A1` or `A1` on the first sheet.
    pub fn get(&self, reference: &str) -> Option<&Value> {
        let (sheet, cell) = match reference.rsplit_once('!') {
            Some((sheet, cell)) => (self.find(sheet.trim_matches('\''))?, cell),
            None => (0, reference),
        };
        let (row, col) = parse::address(cell)?;
        Some(self.value(sheet, row, col))
    }

    /// Computes every formula cell, and returns what it could not compute.
    pub fn evaluate(&mut self) -> Vec<Missed> {
        let names: Vec<Parsed> = self
            .names
            .iter()
            .map(|n| Parsed {
                scope: n.sheet.as_ref().map(|s| self.find(s).unwrap_or(usize::MAX)),
                name: n.name.to_lowercase(),
                expr: parse::parse(&n.text),
            })
            .collect();
        let exprs: Vec<Option<Result<Expr, Unreadable>>> = self
            .formulas
            .iter()
            .map(|f| f.text.as_deref().map(parse::parse))
            .collect();
        let deps: Vec<Vec<usize>> = self
            .formulas
            .iter()
            .zip(&exprs)
            .map(|(f, expr)| match expr {
                Some(Ok(expr)) => self.precedents(expr, f, &names),
                _ => Vec::new(),
            })
            .collect();
        let mut missed = Vec::new();
        for component in components(&deps) {
            let circular = component.len() > 1 || deps[component[0]].contains(&component[0]);
            for at in component {
                let (sheet, row, col) = {
                    let f = &self.formulas[at];
                    (f.sheet, f.row, f.col)
                };
                let text = || self.formulas[at].text.clone().unwrap_or_default();
                let miss = match &exprs[at] {
                    None | Some(Err(_)) => Some(Miss {
                        kind: Kind::Token,
                        text: text(),
                    }),
                    Some(Ok(_)) if circular => Some(Miss {
                        kind: Kind::Circular,
                        text: text(),
                    }),
                    Some(Ok(expr)) => {
                        let result = Ctx {
                            book: self,
                            names: &names,
                            sheet,
                            row,
                            col,
                            depth: 0,
                        }
                        .result(expr);
                        match result {
                            Ok(v) => {
                                self.sheets[sheet].cells.insert((row, col), v);
                                self.formulas[at].computed = true;
                                None
                            }
                            Err(miss) => Some(miss),
                        }
                    }
                };
                if let Some(miss) = miss {
                    missed.push(((sheet, row, col), miss));
                }
            }
        }
        missed.sort_by_key(|(at, _)| *at);
        missed
            .into_iter()
            .map(|((sheet, row, col), miss)| Missed {
                cell: format!(
                    "{}!{}",
                    quoted(&self.sheets[sheet].name),
                    Rect::cell(row, col).a1()
                ),
                kind: miss.kind,
                text: miss.text,
            })
            .collect()
    }

    /// The formula cells `expr` reads, for the formula `at`, walked with a stack.
    fn precedents(&self, expr: &Expr, at: &Formula, names: &[Parsed]) -> Vec<usize> {
        let mut out = Vec::new();
        let mut seen_names = Vec::new();
        let mut stack = vec![expr];
        while let Some(node) = stack.pop() {
            let mut area = |sheet: usize, rect: Rect| {
                if let Some(s) = self.sheets.get(sheet) {
                    out.extend(
                        s.formulas
                            .range((rect.top, rect.left)..=(rect.bottom, rect.right))
                            .filter(|((_, col), _)| (rect.left..=rect.right).contains(col))
                            .map(|(_, &i)| i),
                    );
                }
            };
            match node {
                Expr::Area(Area { sheet, rect }) => {
                    let on = match sheet {
                        Some(name) => self.find(name),
                        None => Some(at.sheet),
                    };
                    if let Some(on) = on {
                        area(on, *rect);
                    }
                }
                Expr::Structured(s) => {
                    if let Ok((on, rect)) = self.structured(s, at.sheet, at.row) {
                        area(on, rect);
                    }
                }
                Expr::Name(sheet, name) => {
                    if let Some(i) = lookup(self, names, sheet.as_deref(), at.sheet, name)
                        && !seen_names.contains(&i)
                    {
                        seen_names.push(i);
                        if let Ok(expr) = &names[i].expr {
                            stack.push(expr);
                        }
                    }
                }
                Expr::Unary(_, inner) => stack.push(inner),
                Expr::Binary(_, left, right) => {
                    stack.push(left);
                    stack.push(right);
                }
                Expr::Call(name, args) => {
                    let skip = functions::reads_no_cells(name);
                    stack.extend(args.iter().filter(|arg| {
                        !(skip
                            && matches!(arg, Expr::Area(_) | Expr::Structured(_) | Expr::Name(..)))
                    }));
                }
                _ => {}
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The sheet and cells a structured reference covers, for a formula on `sheet` in `row`.
    fn structured(
        &self,
        s: &Structured,
        sheet: usize,
        row: u32,
    ) -> Result<(usize, Rect), CellError> {
        let table = match &s.table {
            Some(name) => self
                .tables
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(name)),
            None => self.tables.iter().find(|t| {
                self.find(&t.sheet) == Some(sheet) && (t.rect.top..=t.rect.bottom).contains(&row)
            }),
        }
        .ok_or(CellError::Ref)?;
        let on = self.find(&table.sheet).ok_or(CellError::Ref)?;
        let r = table.rect;
        let first = r.top + u32::from(table.header);
        let last = r.bottom - u32::from(table.totals);
        let span = |item: Item| match item {
            Item::All => Ok((r.top, r.bottom)),
            Item::Data if first <= last => Ok((first, last)),
            Item::Data => Err(CellError::Ref),
            Item::Headers if table.header => Ok((r.top, r.top)),
            Item::Totals if table.totals => Ok((r.bottom, r.bottom)),
            Item::Headers | Item::Totals => Err(CellError::Ref),
            Item::ThisRow if on == sheet && (first..=last).contains(&row) => Ok((row, row)),
            Item::ThisRow => Err(CellError::Value),
        };
        let (mut top, mut bottom) = (u32::MAX, 0);
        let items: &[Item] = if s.items.is_empty() {
            &[Item::Data]
        } else {
            &s.items
        };
        for &item in items {
            let (t, b) = span(item)?;
            top = top.min(t);
            bottom = bottom.max(b);
        }
        let (left, right) = match &s.columns {
            None => (r.left, r.right),
            Some((first, last)) => {
                let at = |name: &str| {
                    table
                        .columns
                        .iter()
                        .position(|c| c.to_lowercase() == name.to_lowercase())
                        .map(|i| r.left + i as u32)
                        .ok_or(CellError::Ref)
                };
                let (a, b) = (at(first)?, at(last)?);
                (a.min(b), a.max(b))
            }
        };
        Ok((
            on,
            Rect {
                top,
                left,
                bottom,
                right,
            },
        ))
    }
}

/// The name `name` a formula on `sheet` reads, qualified by `qualifier` when it is.
fn lookup(
    book: &Book,
    names: &[Parsed],
    qualifier: Option<&str>,
    sheet: usize,
    name: &str,
) -> Option<usize> {
    let name = name.to_lowercase();
    let local = match qualifier {
        Some(q) => Some(book.find(q)?),
        None => Some(sheet),
    };
    names
        .iter()
        .position(|n| n.scope == local && n.name == name)
        .or_else(|| {
            qualifier
                .is_none()
                .then(|| {
                    names
                        .iter()
                        .position(|n| n.scope.is_none() && n.name == name)
                })
                .flatten()
        })
}

/// A sheet's name as a reference writes it, quoted when it has to be.
fn quoted(name: &str) -> String {
    let plain = name
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
        && parse::address(name).is_none();
    if plain {
        name.to_owned()
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}

/// The strongly connected components of `deps`, each after every component it reads, by
/// Tarjan's algorithm with an explicit stack.
fn components(deps: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = deps.len();
    let mut walk = Walk {
        index: vec![Walk::NONE; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        work: Vec::new(),
        next: 0,
    };
    let mut out = Vec::new();
    for root in 0..n {
        if walk.index[root] != Walk::NONE {
            continue;
        }
        walk.visit(root);
        while let Some(&(v, i)) = walk.work.last() {
            if let Some(&w) = deps[v].get(i) {
                if let Some(top) = walk.work.last_mut() {
                    top.1 += 1;
                }
                if walk.index[w] == Walk::NONE {
                    walk.visit(w);
                } else if walk.on_stack[w] {
                    walk.low[v] = walk.low[v].min(walk.index[w]);
                }
                continue;
            }
            walk.work.pop();
            if let Some(&(u, _)) = walk.work.last() {
                walk.low[u] = walk.low[u].min(walk.low[v]);
            }
            if walk.low[v] == walk.index[v] {
                let mut component = Vec::new();
                while let Some(w) = walk.stack.pop() {
                    walk.on_stack[w] = false;
                    component.push(w);
                    if w == v {
                        break;
                    }
                }
                out.push(component);
            }
        }
    }
    out
}

/// Tarjan's state: each cell's visit number and lowest reachable one, the component stack, and
/// the walk's own stack of a cell and its next precedent.
struct Walk {
    index: Vec<usize>,
    low: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    work: Vec<(usize, usize)>,
    next: usize,
}

impl Walk {
    const NONE: usize = usize::MAX;

    fn visit(&mut self, v: usize) {
        self.index[v] = self.next;
        self.low[v] = self.next;
        self.next += 1;
        self.stack.push(v);
        self.on_stack[v] = true;
        self.work.push((v, 0));
    }
}

/// One formula's evaluation: the book, its names, and the cell the formula is in.
pub struct Ctx<'a> {
    book: &'a Book,
    names: &'a [Parsed],
    pub sheet: usize,
    pub row: u32,
    pub col: u32,
    depth: usize,
}

impl<'a> Ctx<'a> {
    /// The formula's result as a cell value. A blank result is 0.
    fn result(&mut self, expr: &Expr) -> Result<Value, Miss> {
        Ok(match self.scalar(expr)? {
            Value::Blank => Value::Number(0.0),
            v => v,
        })
    }

    /// The cells of `rect` on `sheet` that are not blank.
    pub fn cells(&self, sheet: usize, rect: Rect) -> impl Iterator<Item = &'a Value> + use<'a> {
        let book = self.book;
        book.sheets.get(sheet).into_iter().flat_map(move |s| {
            s.cells
                .range((rect.top, rect.left)..=(rect.bottom, rect.right))
                .filter(move |((_, col), _)| (rect.left..=rect.right).contains(col))
                .map(|(_, v)| v)
        })
    }

    /// The non-blank cells of `rect` on `sheet` with their row and column, by row and then
    /// column.
    pub fn entries(
        &self,
        sheet: usize,
        rect: Rect,
    ) -> impl Iterator<Item = (u32, u32, &'a Value)> + use<'a> {
        let book = self.book;
        book.sheets.get(sheet).into_iter().flat_map(move |s| {
            s.cells
                .range((rect.top, rect.left)..=(rect.bottom, rect.right))
                .filter(move |((_, col), _)| (rect.left..=rect.right).contains(col))
                .map(|(&(row, col), v)| (row, col, v))
        })
    }

    /// The value of one cell, blank when it is empty.
    pub fn at(&self, sheet: usize, row: u32, col: u32) -> &'a Value {
        self.book.value(sheet, row, col)
    }

    /// Whether the file hides `row` of `sheet`.
    pub fn hidden(&self, sheet: usize, row: u32) -> bool {
        self.book
            .sheets
            .get(sheet)
            .is_some_and(|s| s.hidden.contains(&row))
    }

    /// Whether any row of `top..=bottom` on `sheet` is hidden.
    pub fn any_hidden(&self, sheet: usize, top: u32, bottom: u32) -> bool {
        self.book
            .sheets
            .get(sheet)
            .is_some_and(|s| s.hidden.range(top..=bottom).next().is_some())
    }

    /// Whether the cell's formula calls `SUBTOTAL`, which another `SUBTOTAL` skips.
    pub fn is_subtotal(&self, sheet: usize, row: u32, col: u32) -> bool {
        self.book
            .sheets
            .get(sheet)
            .and_then(|s| s.formulas.get(&(row, col)))
            .and_then(|&at| self.book.formulas[at].text.as_deref())
            .is_some_and(|text| text.to_ascii_uppercase().contains("SUBTOTAL("))
    }

    /// Whether the formula's workbook counts dates from 1904.
    pub fn date1904(&self) -> bool {
        self.book.date1904
    }

    /// `expr` as one value: a reference of one cell gives that cell's value.
    pub fn scalar(&mut self, expr: &Expr) -> Result<Value, Miss> {
        match self.eval(expr)? {
            Operand::Value(v) => Ok(v),
            Operand::Area(sheet, rect) if rect.top == rect.bottom && rect.left == rect.right => {
                Ok(self.book.value(sheet, rect.top, rect.left).clone())
            }
            Operand::Area(sheet, rect) => Err(self.reference(sheet, rect)),
        }
    }

    fn reference(&self, sheet: usize, rect: Rect) -> Miss {
        let text = if sheet == self.sheet {
            rect.a1()
        } else {
            format!("{}!{}", quoted(&self.book.sheets[sheet].name), rect.a1())
        };
        Miss {
            kind: Kind::Reference,
            text,
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Operand, Miss> {
        let v = |v| Ok(Operand::Value(v));
        match expr {
            Expr::Number(n) => v(Value::Number(*n)),
            Expr::Text(t) => v(Value::Text(t.clone())),
            Expr::Bool(b) => v(Value::Bool(*b)),
            Expr::Error(e) => v(Value::Error(*e)),
            Expr::Missing => v(Value::Blank),
            Expr::Area(Area { sheet, rect }) => match sheet {
                None => Ok(Operand::Area(self.sheet, *rect)),
                Some(name) => match self.book.find(name) {
                    Some(on) => Ok(Operand::Area(on, *rect)),
                    None => Err(Miss {
                        kind: Kind::Reference,
                        text: format!("{}!{}", quoted(name), rect.a1()),
                    }),
                },
            },
            Expr::Name(sheet, name) => self.name(sheet.as_deref(), name),
            Expr::Structured(s) => match self.book.structured(s, self.sheet, self.row) {
                Ok((on, rect)) => Ok(Operand::Area(on, rect)),
                Err(e) => v(Value::Error(e)),
            },
            Expr::Unary(op, inner) => {
                let value = self.scalar(inner)?;
                if *op == parse::Unary::Plus {
                    return v(value);
                }
                v(match value.number() {
                    Ok(n) if *op == parse::Unary::Minus => Value::number_or_error(-n),
                    Ok(n) => Value::number_or_error(n / 100.0),
                    Err(e) => Value::Error(e),
                })
            }
            Expr::Binary(parse::Binary::Range, left, right) => {
                match (self.eval(left)?, self.eval(right)?) {
                    (Operand::Area(s1, a), Operand::Area(s2, b)) if s1 == s2 => Ok(Operand::Area(
                        s1,
                        Rect {
                            top: a.top.min(b.top),
                            left: a.left.min(b.left),
                            bottom: a.bottom.max(b.bottom),
                            right: a.right.max(b.right),
                        },
                    )),
                    _ => v(Value::Error(CellError::Value)),
                }
            }
            Expr::Binary(op, left, right) => {
                let a = self.scalar(left)?;
                let b = self.scalar(right)?;
                v(binary(*op, &a, &b))
            }
            Expr::Call(name, args) => functions::call(self, name, args),
            Expr::Listed(text) => Err(Miss {
                kind: Kind::Reference,
                text: text.clone(),
            }),
        }
    }

    fn name(&mut self, qualifier: Option<&str>, name: &str) -> Result<Operand, Miss> {
        if qualifier.is_some_and(|q| self.book.find(q).is_none()) {
            return Err(Miss {
                kind: Kind::Reference,
                text: format!("{}!{name}", quoted(qualifier.unwrap_or_default())),
            });
        }
        let names = self.names;
        let Some(at) = lookup(self.book, names, qualifier, self.sheet, name) else {
            return Ok(Operand::Value(Value::Error(CellError::Name)));
        };
        let Ok(expr) = &names[at].expr else {
            return Err(Miss {
                kind: Kind::Token,
                text: name.to_owned(),
            });
        };
        if self.depth >= NAME_DEPTH {
            return Err(Miss {
                kind: Kind::Circular,
                text: name.to_owned(),
            });
        }
        self.depth += 1;
        let result = self.eval(expr);
        self.depth -= 1;
        result
    }
}

/// An operator other than `:` over two values.
fn binary(op: parse::Binary, a: &Value, b: &Value) -> Value {
    use parse::Binary as B;
    use std::cmp::Ordering;
    let compare = |test: fn(Ordering) -> bool| match a.compare(b) {
        Ok(order) => Value::Bool(test(order)),
        Err(e) => Value::Error(e),
    };
    match op {
        B::Concat => match (a.text(), b.text()) {
            (Ok(x), Ok(y)) => Value::Text(x + &y),
            (Err(e), _) | (_, Err(e)) => Value::Error(e),
        },
        B::Eq => compare(Ordering::is_eq),
        B::Ne => compare(Ordering::is_ne),
        B::Lt => compare(Ordering::is_lt),
        B::Le => compare(Ordering::is_le),
        B::Gt => compare(Ordering::is_gt),
        B::Ge => compare(Ordering::is_ge),
        _ => {
            let (x, y) = match (a.number(), b.number()) {
                (Ok(x), Ok(y)) => (x, y),
                (Err(e), _) | (_, Err(e)) => return Value::Error(e),
            };
            match op {
                B::Plus => Value::number_or_error(x + y),
                B::Minus => Value::number_or_error(x - y),
                B::Times => Value::number_or_error(x * y),
                B::Divide if y == 0.0 => Value::Error(CellError::DivZero),
                B::Divide => Value::number_or_error(x / y),
                B::Power if x == 0.0 && y == 0.0 => Value::Error(CellError::Num),
                B::Power if x == 0.0 && y < 0.0 => Value::Error(CellError::DivZero),
                _ => Value::number_or_error(x.powf(y)),
            }
        }
    }
}
