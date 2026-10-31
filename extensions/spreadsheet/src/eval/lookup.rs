//! The lookup functions: `CHOOSE`, `INDEX`, `MATCH`, `VLOOKUP`, `HLOOKUP` and `XLOOKUP`
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! `CHOOSE`, `INDEX` and `XLOOKUP` return a reference where Excel does, so `SUM(INDEX(A:B,0,2))`
//! adds a column. An exact match walks only the stored cells of the searched line, so a whole
//! column costs what it holds. Text matches without case, and a number never matches text.
//! `VLOOKUP`, `HLOOKUP` and `MATCH` read wildcards in an exact match, and `XLOOKUP` in match mode 2.
//! A sought value that is blank is listed, since whether Excel matches it against an empty cell
//! is not certain, and so is a table given as a value and not a reference.
//!
//! An approximate match is Excel's binary search, whose answer on unsorted data depends on the
//! cells it probes. It is computed only where that answer is certain: the stored cells start the
//! line with no gap, all have the sought value's type, they are sorted, and they fill the line.
//! Blanks after them are allowed for an ascending `VLOOKUP`, `HLOOKUP` or `MATCH`, where a run of
//! equal values gives its last. A tie at the answer is listed everywhere else. `XLOOKUP`'s linear
//! next-smaller and next-larger search is computed on the same terms without the sort. Any other
//! input is listed.

use std::cmp::Ordering;

use super::functions::{arity, error, listed, number, value, wildcard};
use super::parse::{Expr, Rect};
use super::value::Value;
use super::{Ctx, Miss, Operand};
use crate::read::CellError;

/// The value of a `Result<_, CellError>`, or its error as the function's result.
macro_rules! take {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return error(e),
        }
    };
}

/// One row or one column of cells that a lookup searches.
#[derive(Clone, Copy)]
struct Line {
    sheet: usize,
    rect: Rect,
    down: bool,
}

impl Line {
    fn len(self) -> u32 {
        if self.down {
            self.rect.bottom - self.rect.top + 1
        } else {
            self.rect.right - self.rect.left + 1
        }
    }

    /// The position along the line of the cell at `row`, `col`, from 0.
    fn index(self, row: u32, col: u32) -> u32 {
        if self.down {
            row - self.rect.top
        } else {
            col - self.rect.left
        }
    }
}

/// What an approximate search takes: the equal value, the largest not above, or the smallest
/// not below.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Near {
    Exact,
    Below,
    Above,
}

/// The value to search for: an error is the result, and a blank is listed.
fn sought(ctx: &mut Ctx<'_>, name: &str, expr: &Expr) -> Result<Result<Value, CellError>, Miss> {
    match ctx.scalar(expr)? {
        Value::Blank => Err(listed(name)),
        Value::Error(e) => Ok(Err(e)),
        v => Ok(Ok(v)),
    }
}

/// A reference argument: an error is the result, and any other value is listed.
fn area(
    ctx: &mut Ctx<'_>,
    name: &str,
    expr: &Expr,
) -> Result<Result<(usize, Rect), CellError>, Miss> {
    match ctx.eval(expr)? {
        Operand::Area(sheet, rect) => Ok(Ok((sheet, rect))),
        Operand::Value(Value::Error(e)) => Ok(Err(e)),
        Operand::Value(_) => Err(listed(name)),
    }
}

/// A number argument, truncated.
fn whole(ctx: &mut Ctx<'_>, expr: &Expr) -> Result<Result<f64, CellError>, Miss> {
    Ok(ctx.scalar(expr)?.number().map(f64::trunc))
}

fn kind(v: &Value) -> Option<u8> {
    match v {
        Value::Number(_) => Some(0),
        Value::Text(_) => Some(1),
        Value::Bool(_) => Some(2),
        _ => None,
    }
}

/// Whether a stored cell equals the sought value: the same type, and text without case.
fn equal(sought: &Value, cell: &Value, wild: bool) -> bool {
    match (sought, cell) {
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Text(a), Value::Text(b)) if wild => wildcard(a, b),
        (Value::Text(a), Value::Text(b)) => a.to_lowercase() == b.to_lowercase(),
        _ => false,
    }
}

/// The first stored cell of `line` equal to `sought`, or the last when `last` is set.
fn exact(ctx: &Ctx<'_>, line: Line, sought: &Value, wild: bool, last: bool) -> Option<u32> {
    let mut hits = ctx
        .entries(line.sheet, line.rect)
        .filter(|(_, _, v)| equal(sought, v, wild))
        .map(|(row, col, _)| line.index(row, col));
    if last { hits.last() } else { hits.next() }
}

/// The stored cells of `line` when an approximate search's answer over them is certain, by the
/// terms in the module doc: blanks after them only when `trailing` is set, and in order when
/// `sorted` names a direction (`true` for ascending).
fn certain<'a>(
    ctx: &Ctx<'a>,
    line: Line,
    sought: &Value,
    trailing: bool,
    sorted: Option<bool>,
) -> Option<Vec<(u32, &'a Value)>> {
    let cells: Vec<(u32, &'a Value)> = ctx
        .entries(line.sheet, line.rect)
        .filter(|(_, _, v)| !matches!(v, Value::Blank))
        .map(|(row, col, v)| (line.index(row, col), v))
        .collect();
    if !trailing && u32::try_from(cells.len()).ok() != Some(line.len()) {
        return None;
    }
    let want = kind(sought);
    let gap = cells
        .iter()
        .enumerate()
        .any(|(k, &(at, v))| usize::try_from(at).ok() != Some(k) || kind(v) != want);
    if gap {
        return None;
    }
    if let Some(ascending) = sorted {
        let wrong = if ascending {
            Ordering::Greater
        } else {
            Ordering::Less
        };
        for pair in cells.windows(2) {
            if pair[0].1.compare(pair[1].1).ok()? == wrong {
                return None;
            }
        }
    }
    Some(cells)
}

/// The position of the cell nearest `sought` by `near`, `Ok(None)` when no cell qualifies, and
/// `Err` when the nearest value is tied and `last` is not set. With `last`, a tie gives the last.
fn nearest(
    cells: &[(u32, &Value)],
    sought: &Value,
    near: Near,
    last: bool,
) -> Result<Option<u32>, ()> {
    let mut best: Option<(u32, &Value)> = None;
    let mut tied = false;
    for &(at, v) in cells {
        let order = v.compare(sought).map_err(|_| ())?;
        let fits = match near {
            Near::Exact => order == Ordering::Equal,
            Near::Below => order != Ordering::Greater,
            Near::Above => order != Ordering::Less,
        };
        if !fits {
            continue;
        }
        let Some((_, held)) = best else {
            best = Some((at, v));
            continue;
        };
        let than = v.compare(held).map_err(|_| ())?;
        let better = match near {
            Near::Exact => false,
            Near::Below => than == Ordering::Greater,
            Near::Above => than == Ordering::Less,
        };
        if better {
            best = Some((at, v));
            tied = false;
        } else if than == Ordering::Equal {
            tied = true;
            if last {
                best = Some((at, v));
            }
        }
    }
    if tied && !last {
        return Err(());
    }
    Ok(best.map(|(at, _)| at))
}

/// A binary search over `line`, sorted ascending when `ascending` is set. `family` marks
/// `VLOOKUP`, `HLOOKUP` and `MATCH`, whose ascending search allows trailing blanks and gives the
/// last of a run. An answer that is not certain is listed as `name`.
fn binary(
    ctx: &Ctx<'_>,
    name: &str,
    line: Line,
    sought: &Value,
    near: Near,
    ascending: bool,
    family: bool,
) -> Result<Option<u32>, Miss> {
    let loose = family && ascending;
    let cells = certain(ctx, line, sought, loose, Some(ascending)).ok_or_else(|| listed(name))?;
    nearest(&cells, sought, near, loose && near == Near::Below).map_err(|()| listed(name))
}

pub(super) fn vlookup(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    table_lookup(ctx, "VLOOKUP", args, true)
}

pub(super) fn hlookup(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    table_lookup(ctx, "HLOOKUP", args, false)
}

/// `VLOOKUP` when `down`, and `HLOOKUP` otherwise: searches the table's first column (or row)
/// and returns the value of the cell at the given column (or row) of the match.
fn table_lookup(ctx: &mut Ctx<'_>, name: &str, args: &[Expr], down: bool) -> Result<Operand, Miss> {
    arity(name, args, 3, 4)?;
    let sought = take!(sought(ctx, name, &args[0])?);
    let (sheet, table) = take!(area(ctx, name, &args[1])?);
    let at = take!(whole(ctx, &args[2])?);
    let approximate = match args.get(3) {
        None => true,
        Some(arg) => take!(ctx.scalar(arg)?.boolean()),
    };
    let size = if down {
        table.right - table.left + 1
    } else {
        table.bottom - table.top + 1
    };
    if at < 1.0 {
        return error(CellError::Value);
    }
    if at > f64::from(size) {
        return error(CellError::Ref);
    }
    let at = at as u32 - 1;
    let rect = if down {
        Rect {
            right: table.left,
            ..table
        }
    } else {
        Rect {
            bottom: table.top,
            ..table
        }
    };
    let line = Line { sheet, rect, down };
    let found = if approximate {
        binary(ctx, name, line, &sought, Near::Below, true, true)?
    } else {
        exact(ctx, line, &sought, true, false)
    };
    match found {
        None => error(CellError::NotAvailable),
        Some(i) => {
            let (row, col) = if down {
                (table.top + i, table.left + at)
            } else {
                (table.top + at, table.left + i)
            };
            value(ctx.at(sheet, row, col).clone())
        }
    }
}

/// `MATCH`: the position from 1 of the sought value in a row or column. Match type 0 is exact,
/// above 0 the largest not above in ascending data, and below 0 the smallest not below in
/// descending data.
pub(super) fn match_(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("MATCH", args, 2, 3)?;
    let sought = take!(sought(ctx, "MATCH", &args[0])?);
    let (sheet, rect) = take!(area(ctx, "MATCH", &args[1])?);
    let mode = match args.get(2) {
        None => 1.0,
        Some(arg) => take!(whole(ctx, arg)?),
    };
    let down = rect.left == rect.right;
    if !down && rect.top != rect.bottom {
        return error(CellError::NotAvailable);
    }
    let line = Line { sheet, rect, down };
    let found = if mode == 0.0 {
        exact(ctx, line, &sought, true, false)
    } else if mode > 0.0 {
        binary(ctx, "MATCH", line, &sought, Near::Below, true, true)?
    } else {
        binary(ctx, "MATCH", line, &sought, Near::Above, false, true)?
    };
    match found {
        Some(i) => number(f64::from(i + 1)),
        None => error(CellError::NotAvailable),
    }
}

/// `XLOOKUP`: the row (or column) of the return array at the match, as a reference. Match mode
/// 0 is exact, -1 takes the next smaller, 1 the next larger and 2 reads wildcards. Search mode 1
/// searches from the first, -1 from the last, and 2 and -2 search ascending and descending data
/// by binary search.
pub(super) fn xlookup(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("XLOOKUP", args, 3, 6)?;
    let sought = take!(sought(ctx, "XLOOKUP", &args[0])?);
    let (sheet, rect) = take!(area(ctx, "XLOOKUP", &args[1])?);
    let (back_sheet, back) = take!(area(ctx, "XLOOKUP", &args[2])?);
    let mode = match args.get(4) {
        None => 0.0,
        Some(arg) => take!(whole(ctx, arg)?),
    };
    let search = match args.get(5) {
        None => 1.0,
        Some(arg) => take!(whole(ctx, arg)?),
    };
    if ![-1.0, 0.0, 1.0, 2.0].contains(&mode) || ![-2.0, -1.0, 1.0, 2.0].contains(&search) {
        return Err(listed("XLOOKUP"));
    }
    let down = rect.left == rect.right;
    if !down && rect.top != rect.bottom {
        return error(CellError::Value);
    }
    if rect.top == rect.bottom && rect.left == rect.right && back.top != back.bottom {
        return Err(listed("XLOOKUP"));
    }
    let line = Line { sheet, rect, down };
    let size = if down {
        back.bottom - back.top + 1
    } else {
        back.right - back.left + 1
    };
    if size != line.len() {
        return error(CellError::Value);
    }
    let wild = mode == 2.0;
    let near = if mode < 0.0 {
        Near::Below
    } else if mode == 1.0 {
        Near::Above
    } else {
        Near::Exact
    };
    let found = if search.abs() == 2.0 {
        if wild {
            return Err(listed("XLOOKUP"));
        }
        binary(ctx, "XLOOKUP", line, &sought, near, search > 0.0, false)?
    } else {
        match exact(ctx, line, &sought, wild, search < 0.0) {
            Some(i) => Some(i),
            None if near == Near::Exact => None,
            None => {
                let cells =
                    certain(ctx, line, &sought, false, None).ok_or_else(|| listed("XLOOKUP"))?;
                nearest(&cells, &sought, near, false).map_err(|()| listed("XLOOKUP"))?
            }
        }
    };
    match found {
        Some(i) if down => Ok(Operand::Area(
            back_sheet,
            Rect {
                top: back.top + i,
                bottom: back.top + i,
                ..back
            },
        )),
        Some(i) => Ok(Operand::Area(
            back_sheet,
            Rect {
                left: back.left + i,
                right: back.left + i,
                ..back
            },
        )),
        None => match args.get(3) {
            None => error(CellError::NotAvailable),
            Some(Expr::Missing) => Err(listed("XLOOKUP")),
            Some(arg) => ctx.eval(arg),
        },
    }
}

/// `INDEX`: the cell at a row and column of a reference, as a reference. Row 0 gives every row
/// and column 0 every column. With no column, a one-row reference reads the row number as its
/// column, and any other reference gives the whole row. The fourth argument, an area of a
/// union, is listed.
pub(super) fn index(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("INDEX", args, 2, 4)?;
    if args.len() == 4 {
        return Err(listed("INDEX"));
    }
    let (sheet, rect) = take!(area(ctx, "INDEX", &args[0])?);
    let mut row = take!(whole(ctx, &args[1])?);
    let mut col = match args.get(2) {
        None => 0.0,
        Some(arg) => take!(whole(ctx, arg)?),
    };
    if row < 0.0 || col < 0.0 {
        return error(CellError::Value);
    }
    let height = rect.bottom - rect.top + 1;
    let width = rect.right - rect.left + 1;
    if args.len() == 2 && height == 1 {
        (row, col) = (0.0, row);
    }
    if row > f64::from(height) || col > f64::from(width) {
        return error(CellError::Ref);
    }
    let (top, bottom) = if row == 0.0 {
        (rect.top, rect.bottom)
    } else {
        let at = rect.top + row as u32 - 1;
        (at, at)
    };
    let (left, right) = if col == 0.0 {
        (rect.left, rect.right)
    } else {
        let at = rect.left + col as u32 - 1;
        (at, at)
    };
    Ok(Operand::Area(
        sheet,
        Rect {
            top,
            left,
            bottom,
            right,
        },
    ))
}

/// `CHOOSE`: the argument at the index from 1, as it evaluates, so a reference stays one.
pub(super) fn choose(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("CHOOSE", args, 2, 255)?;
    let at = take!(whole(ctx, &args[0])?);
    if !(1.0..=(args.len() - 1) as f64).contains(&at) {
        return error(CellError::Value);
    }
    ctx.eval(&args[at as usize])
}
