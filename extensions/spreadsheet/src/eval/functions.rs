//! The function table: every function the evaluator computes, one row each
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! [`FUNCTIONS`] is sorted by name, and [`call`] finds a row by a binary search. A function
//! takes its arguments unevaluated, so `IF` computes only the branch it takes, as Excel does. A
//! function outside the table, and a call with a count of arguments Excel does not accept, is
//! listed as `function` with its name.
//!
//! `ROW`, `COLUMN`, `ROWS` and `COLUMNS` read where a reference is and never its cells, so a
//! formula may name its own cell in them. `ROW` and `COLUMN` of a reference more than one row or
//! column wide would give an array, which Excel spills, so that call is listed.

use super::parse::{Expr, Rect};
use super::value::Value;
use super::{Ctx, Kind, Miss, Operand};
use crate::read::CellError;

/// One function: its context and its arguments as written.
pub type Function = fn(&mut Ctx<'_>, &[Expr]) -> Result<Operand, Miss>;

/// The table, sorted by name.
pub static FUNCTIONS: &[(&str, Function)] = &[
    ("COLUMN", column),
    ("COLUMNS", columns),
    ("IF", if_),
    ("ROW", row),
    ("ROWS", rows),
    ("SUM", sum),
];

/// The function named `name`, in upper case.
pub fn find(name: &str) -> Option<Function> {
    FUNCTIONS
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|at| FUNCTIONS[at].1)
}

/// Calls `name` with `args`.
pub fn call(ctx: &mut Ctx<'_>, name: &str, args: &[Expr]) -> Result<Operand, Miss> {
    match find(name) {
        Some(function) => function(ctx, args),
        None => Err(listed(name)),
    }
}

/// Whether `name` reads where its references are and not their cells.
pub(super) fn reads_no_cells(name: &str) -> bool {
    matches!(name, "ROW" | "COLUMN" | "ROWS" | "COLUMNS")
}

fn listed(name: &str) -> Miss {
    Miss {
        kind: Kind::Function,
        text: name.to_owned(),
    }
}

fn arity(name: &str, args: &[Expr], least: usize, most: usize) -> Result<(), Miss> {
    if (least..=most).contains(&args.len()) {
        Ok(())
    } else {
        Err(listed(name))
    }
}

fn value(v: Value) -> Result<Operand, Miss> {
    Ok(Operand::Value(v))
}

fn error(e: CellError) -> Result<Operand, Miss> {
    value(Value::Error(e))
}

fn number(n: f64) -> Result<Operand, Miss> {
    value(Value::number_or_error(n))
}

/// `SUM`: a reference adds its numbers and skips text, booleans and blanks. A value given
/// directly is converted to a number.
fn sum(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("SUM", args, 1, 255)?;
    let mut total = 0.0;
    for arg in args {
        match ctx.eval(arg)? {
            Operand::Area(sheet, rect) => {
                for cell in ctx.cells(sheet, rect) {
                    match cell {
                        Value::Number(n) => total += n,
                        Value::Error(e) => return error(*e),
                        _ => {}
                    }
                }
            }
            Operand::Value(v) => match v.number() {
                Ok(n) => total += n,
                Err(e) => return error(e),
            },
        }
    }
    number(total)
}

/// `IF`: the second argument when the first is true, and the third otherwise. A branch that is
/// not given is `FALSE`, and one left empty is 0.
fn if_(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("IF", args, 2, 3)?;
    let condition = match ctx.scalar(&args[0])?.boolean() {
        Ok(b) => b,
        Err(e) => return error(e),
    };
    match args.get(if condition { 1 } else { 2 }) {
        None => value(Value::Bool(false)),
        Some(Expr::Missing) => number(0.0),
        Some(branch) => ctx.eval(branch),
    }
}

/// The reference `ROW` or `COLUMN` reads, or `None` for the formula's own cell.
fn place(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
) -> Result<Result<Option<Rect>, CellError>, Miss> {
    arity(name, args, 0, 1)?;
    match args.first() {
        None | Some(Expr::Missing) => Ok(Ok(None)),
        Some(arg) => Ok(match ctx.eval(arg)? {
            Operand::Area(_, rect) => Ok(Some(rect)),
            Operand::Value(Value::Error(e)) => Err(e),
            Operand::Value(_) => Err(CellError::Value),
        }),
    }
}

fn row(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    match place(ctx, "ROW", args)? {
        Ok(None) => number(f64::from(ctx.row)),
        Ok(Some(rect)) if rect.top == rect.bottom => number(f64::from(rect.top)),
        Ok(Some(_)) => Err(listed("ROW")),
        Err(e) => error(e),
    }
}

fn column(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    match place(ctx, "COLUMN", args)? {
        Ok(None) => number(f64::from(ctx.col)),
        Ok(Some(rect)) if rect.left == rect.right => number(f64::from(rect.left)),
        Ok(Some(_)) => Err(listed("COLUMN")),
        Err(e) => error(e),
    }
}

/// The size `ROWS` or `COLUMNS` reads: a reference's, or 1 for a single value.
fn size(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
    of: fn(Rect) -> u32,
) -> Result<Operand, Miss> {
    arity(name, args, 1, 1)?;
    match ctx.eval(&args[0])? {
        Operand::Area(_, rect) => number(f64::from(of(rect))),
        Operand::Value(Value::Error(e)) => error(e),
        Operand::Value(_) => number(1.0),
    }
}

fn rows(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    size(ctx, "ROWS", args, |r| r.bottom - r.top + 1)
}

fn columns(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    size(ctx, "COLUMNS", args, |r| r.right - r.left + 1)
}
