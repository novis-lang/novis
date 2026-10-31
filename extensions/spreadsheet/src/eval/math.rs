//! The math, statistics, logic and information functions of the table
//! (`rule:core-classes/spreadsheet-evaluation-and-roster`).
//!
//! - **Aggregates** read their arguments as `SUM` does: a reference gives its numbers and skips
//!   text, booleans and blanks, and a value given directly is converted to a number. The first
//!   error met, left first and then by row, is the result.
//! - **Rounding** works on the number written with 15 significant digits, so `ROUND(2.675,2)` is
//!   2.68 where the binary value is just below 2.675. `CEILING`, `FLOOR` and their `.MATH` forms
//!   take the quotient at 15 digits too, so `CEILING(0.3,0.1)` is 0.3.
//! - **Criteria** (`COUNTIF` and the other `*IF` and `*IFS`) parse Excel's criterion: an optional
//!   operator, then a number, a boolean, an error or text with wildcards. An equality with a number
//!   also matches a text that reads as that number; an order compares numbers with numbers and text
//!   with text. `""` matches a blank and an empty text, and `=` a blank alone. Ranges align by
//!   offset from their top-left cell. Only stored cells are walked, and blanks a criterion matches
//!   are counted from the area's size, so a whole column costs what it stores.
//! - **Logic** is lazy where Excel's is: `IFERROR`, `IFNA`, `IFS` and `SWITCH` compute only what
//!   they return. `AND`, `OR` and `XOR` read every argument.
//! - **`SUBTOTAL`** skips cells whose formula calls `SUBTOTAL`, and codes 101 to 111 skip hidden
//!   rows. Codes 1 to 11 over a hidden row are listed: the file does not say whether a filter or a
//!   person hid it, and Excel counts only the second.
//!
//! An input whose Excel result is not certain is listed under the function's name, never given a
//! guessed value.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use super::functions::{arity, error, listed, number, value, wildcard};
use super::parse::{Expr, Rect};
use super::value::{Value, number_of_text};
use super::{Ctx, Miss, Operand};
use crate::read::CellError;

/// The value of a `Result<_, CellError>`, or the error as the function's result.
macro_rules! take {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return error(e),
        }
    };
}

// ---- Reading arguments ----

/// The numbers of `args` as `SUM` reads them.
fn numbers(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Result<Vec<f64>, CellError>, Miss> {
    let mut out = Vec::new();
    for arg in args {
        match ctx.eval(arg)? {
            Operand::Area(sheet, rect) => {
                for cell in ctx.cells(sheet, rect) {
                    match cell {
                        Value::Number(n) => out.push(*n),
                        Value::Error(e) => return Ok(Err(*e)),
                        _ => {}
                    }
                }
            }
            Operand::Value(v) => match v.number() {
                Ok(n) => out.push(n),
                Err(e) => return Ok(Err(e)),
            },
        }
    }
    Ok(Ok(out))
}

/// One argument as a number.
fn arg(ctx: &mut Ctx<'_>, expr: &Expr) -> Result<Result<f64, CellError>, Miss> {
    Ok(ctx.scalar(expr)?.number())
}

/// The argument at `at` as a number, or `default` when it is not given.
fn optional(
    ctx: &mut Ctx<'_>,
    args: &[Expr],
    at: usize,
    default: f64,
) -> Result<Result<f64, CellError>, Miss> {
    match args.get(at) {
        None => Ok(Ok(default)),
        Some(expr) => arg(ctx, expr),
    }
}

/// A function of one number.
fn unary(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
    f: fn(f64) -> Result<f64, CellError>,
) -> Result<Operand, Miss> {
    arity(name, args, 1, 1)?;
    let x = take!(arg(ctx, &args[0])?);
    number(take!(f(x)))
}

/// The number of cells of `rect`.
fn size(rect: Rect) -> u64 {
    u64::from(rect.bottom - rect.top + 1) * u64::from(rect.right - rect.left + 1)
}

fn shape(rect: Rect) -> (u32, u32) {
    (rect.bottom - rect.top, rect.right - rect.left)
}

/// An argument that must be a reference: its sheet and cells, or the error it gives. Any other
/// value is listed, because Excel does not accept the formula.
fn reference(
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

// ---- Math ----

pub(super) fn abs(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    unary(ctx, "ABS", args, |x| Ok(x.abs()))
}

pub(super) fn exp(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    unary(ctx, "EXP", args, |x| Ok(x.exp()))
}

pub(super) fn int(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    unary(ctx, "INT", args, |x| Ok(x.floor()))
}

pub(super) fn ln(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    unary(ctx, "LN", args, |x| {
        if x <= 0.0 {
            Err(CellError::Num)
        } else {
            Ok(x.ln())
        }
    })
}

pub(super) fn log10(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    unary(ctx, "LOG10", args, |x| {
        if x <= 0.0 {
            Err(CellError::Num)
        } else {
            Ok(x.log10())
        }
    })
}

pub(super) fn sqrt(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    unary(ctx, "SQRT", args, |x| {
        if x < 0.0 {
            Err(CellError::Num)
        } else {
            Ok(x.sqrt())
        }
    })
}

pub(super) fn sign(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    unary(ctx, "SIGN", args, |x| {
        Ok(match x.partial_cmp(&0.0) {
            Some(Ordering::Greater) => 1.0,
            Some(Ordering::Less) => -1.0,
            _ => 0.0,
        })
    })
}

pub(super) fn pi(_ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("PI", args, 0, 0)?;
    number(std::f64::consts::PI)
}

/// `LOG`: the logarithm to a base, 10 when it is not given. A base of 1 is `#DIV/0!`.
pub(super) fn log(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("LOG", args, 1, 2)?;
    let x = take!(arg(ctx, &args[0])?);
    let base = take!(optional(ctx, args, 1, 10.0)?);
    if x <= 0.0 || base <= 0.0 {
        return error(CellError::Num);
    }
    if base == 1.0 {
        return error(CellError::DivZero);
    }
    number(if base == 10.0 {
        x.log10()
    } else {
        x.ln() / base.ln()
    })
}

/// `POWER`: `0^0` is `#NUM!`, and 0 to a negative power is `#DIV/0!`.
pub(super) fn power(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("POWER", args, 2, 2)?;
    let x = take!(arg(ctx, &args[0])?);
    let y = take!(arg(ctx, &args[1])?);
    if x == 0.0 && y == 0.0 {
        return error(CellError::Num);
    }
    if x == 0.0 && y < 0.0 {
        return error(CellError::DivZero);
    }
    number(x.powf(y))
}

/// `MOD`: the remainder with the divisor's sign. Excel gives `#NUM!` once the quotient reaches
/// 2^27 in some versions and not others, so that input is listed.
pub(super) fn mod_(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("MOD", args, 2, 2)?;
    let n = take!(arg(ctx, &args[0])?);
    let d = take!(arg(ctx, &args[1])?);
    if d == 0.0 {
        return error(CellError::DivZero);
    }
    let q = n / d;
    if q.abs() >= 134_217_728.0 {
        return Err(listed("MOD"));
    }
    number(n - d * q.floor())
}

pub(super) fn product(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("PRODUCT", args, 1, 255)?;
    let ns = take!(numbers(ctx, args)?);
    if ns.is_empty() {
        return number(0.0);
    }
    number(ns.iter().product())
}

/// `x` rounded to 15 significant digits.
fn sig15(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    format!("{x:.14e}").parse().unwrap_or(x)
}

#[derive(Clone, Copy)]
enum Mode {
    /// Half away from zero.
    Half,
    /// Away from zero.
    Up,
    /// Toward zero.
    Down,
}

/// `x` rounded to `places` decimal places by `mode`, on its 15 significant digits.
fn round_to(x: f64, places: i64, mode: Mode) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let written = format!("{:.14e}", x.abs());
    let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    // How many leading digits stay: the digit at index `i` is worth `10^(exponent - i)`.
    let keep = exponent + places + 1;
    if keep >= digits.len() as i64 {
        return x;
    }
    let (kept, bump) = if keep < 0 {
        (0u64, matches!(mode, Mode::Up))
    } else {
        let k = keep as usize;
        let kept = digits[..k]
            .iter()
            .fold(0u64, |acc, &d| acc * 10 + u64::from(d));
        let bump = match mode {
            Mode::Half => digits[k] >= 5,
            Mode::Up => digits[k..].iter().any(|&d| d != 0),
            Mode::Down => false,
        };
        (kept, bump)
    };
    let units = kept + u64::from(bump);
    let rounded: f64 = format!("{units}e{}", -places).parse().unwrap_or(0.0);
    if x < 0.0 { -rounded } else { rounded }
}

/// A count of decimal places as Excel reads it: truncated toward zero.
fn places(d: f64) -> i64 {
    d.trunc().clamp(-400.0, 400.0) as i64
}

fn rounding(ctx: &mut Ctx<'_>, name: &str, args: &[Expr], mode: Mode) -> Result<Operand, Miss> {
    if name == "TRUNC" {
        arity(name, args, 1, 2)?;
    } else {
        arity(name, args, 2, 2)?;
    }
    let x = take!(arg(ctx, &args[0])?);
    let d = take!(optional(ctx, args, 1, 0.0)?);
    number(round_to(x, places(d), mode))
}

/// `ROUND`: half away from zero, on the decimal value.
pub(super) fn round(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    rounding(ctx, "ROUND", args, Mode::Half)
}

pub(super) fn roundup(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    rounding(ctx, "ROUNDUP", args, Mode::Up)
}

pub(super) fn rounddown(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    rounding(ctx, "ROUNDDOWN", args, Mode::Down)
}

pub(super) fn trunc(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    rounding(ctx, "TRUNC", args, Mode::Down)
}

/// `CEILING`: up to a multiple of the significance. A negative number with a positive
/// significance goes toward zero, and a positive number with a negative one is `#NUM!`.
pub(super) fn ceiling(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("CEILING", args, 2, 2)?;
    let n = take!(arg(ctx, &args[0])?);
    let s = take!(arg(ctx, &args[1])?);
    if n == 0.0 || s == 0.0 {
        return number(0.0);
    }
    if n > 0.0 && s < 0.0 {
        return error(CellError::Num);
    }
    number(sig15(sig15(n / s).ceil() * s))
}

/// `FLOOR`: down to a multiple of the significance, with `CEILING`'s sign rules. A significance
/// of 0 is `#DIV/0!`.
pub(super) fn floor(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("FLOOR", args, 2, 2)?;
    let n = take!(arg(ctx, &args[0])?);
    let s = take!(arg(ctx, &args[1])?);
    if s == 0.0 {
        return if n == 0.0 {
            Err(listed("FLOOR"))
        } else {
            error(CellError::DivZero)
        };
    }
    if n == 0.0 {
        return number(0.0);
    }
    if n > 0.0 && s < 0.0 {
        return error(CellError::Num);
    }
    number(sig15(sig15(n / s).floor() * s))
}

/// `CEILING.MATH` and `FLOOR.MATH`: the significance's sign is ignored, and a nonzero mode
/// turns a negative number's direction around.
fn math_rounding(ctx: &mut Ctx<'_>, name: &str, args: &[Expr], up: bool) -> Result<Operand, Miss> {
    arity(name, args, 1, 3)?;
    let n = take!(arg(ctx, &args[0])?);
    let s = take!(optional(ctx, args, 1, 1.0)?).abs();
    let mode = take!(optional(ctx, args, 2, 0.0)?);
    if s == 0.0 || n == 0.0 {
        return number(0.0);
    }
    let q = sig15(n / s);
    let up = if n < 0.0 && mode != 0.0 { !up } else { up };
    let whole = if up { q.ceil() } else { q.floor() };
    number(sig15(whole * s))
}

pub(super) fn ceiling_math(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    math_rounding(ctx, "CEILING.MATH", args, true)
}

pub(super) fn floor_math(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    math_rounding(ctx, "FLOOR.MATH", args, false)
}

/// `SUMPRODUCT`: the sum of the products of arrays of one shape, where anything but a number
/// is 0. It walks every position, so an area of more than a million cells is listed.
pub(super) fn sumproduct(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("SUMPRODUCT", args, 1, 255)?;
    let mut dims: Option<(u32, u32)> = None;
    let mut products: Vec<f64> = Vec::new();
    for expr in args {
        let (here_dims, here) = match ctx.eval(expr)? {
            Operand::Area(sheet, rect) => {
                if size(rect) > 1_000_000 {
                    return Err(listed("SUMPRODUCT"));
                }
                let (rows, cols) = shape(rect);
                let width = cols as usize + 1;
                let mut here = vec![0.0; size(rect) as usize];
                for (row, col, cell) in ctx.entries(sheet, rect) {
                    match cell {
                        Value::Number(n) => {
                            here[(row - rect.top) as usize * width + (col - rect.left) as usize] =
                                *n;
                        }
                        Value::Error(e) => return error(*e),
                        _ => {}
                    }
                }
                ((rows, cols), here)
            }
            Operand::Value(Value::Number(n)) => ((0, 0), vec![n]),
            Operand::Value(Value::Error(e)) => return error(e),
            Operand::Value(_) => return Err(listed("SUMPRODUCT")),
        };
        match dims {
            None => {
                dims = Some(here_dims);
                products = here;
            }
            Some(d) if d == here_dims => {
                for (p, h) in products.iter_mut().zip(&here) {
                    *p *= h;
                }
            }
            Some(_) => return error(CellError::Value),
        }
    }
    number(products.iter().sum())
}

// ---- Statistics ----

pub(super) fn average(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("AVERAGE", args, 1, 255)?;
    let ns = take!(numbers(ctx, args)?);
    mean(&ns)
}

fn mean(ns: &[f64]) -> Result<Operand, Miss> {
    if ns.is_empty() {
        return error(CellError::DivZero);
    }
    number(ns.iter().sum::<f64>() / ns.len() as f64)
}

/// The largest or smallest of `ns`, and 0 when there is none.
fn extreme(ns: &[f64], most: bool) -> Result<Operand, Miss> {
    let found = if most {
        ns.iter().copied().reduce(f64::max)
    } else {
        ns.iter().copied().reduce(f64::min)
    };
    number(found.unwrap_or(0.0))
}

pub(super) fn max(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("MAX", args, 1, 255)?;
    let ns = take!(numbers(ctx, args)?);
    extreme(&ns, true)
}

pub(super) fn min(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("MIN", args, 1, 255)?;
    let ns = take!(numbers(ctx, args)?);
    extreme(&ns, false)
}

/// `MEDIAN`: the middle number, or the mean of the middle two. No number is `#NUM!`.
pub(super) fn median(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("MEDIAN", args, 1, 255)?;
    let mut ns = take!(numbers(ctx, args)?);
    if ns.is_empty() {
        return error(CellError::Num);
    }
    ns.sort_by(f64::total_cmp);
    let mid = ns.len() / 2;
    number(if !ns.len().is_multiple_of(2) {
        ns[mid]
    } else {
        (ns[mid - 1] + ns[mid]) / 2.0
    })
}

/// `COUNT`: the numbers of a reference, and each value given directly that is or reads as a
/// number. An empty argument is listed.
pub(super) fn count(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("COUNT", args, 1, 255)?;
    let mut n = 0u64;
    for expr in args {
        match ctx.eval(expr)? {
            Operand::Area(sheet, rect) => {
                n += ctx
                    .cells(sheet, rect)
                    .filter(|v| matches!(v, Value::Number(_)))
                    .count() as u64;
            }
            Operand::Value(Value::Blank) => return Err(listed("COUNT")),
            Operand::Value(v) => n += u64::from(v.number().is_ok()),
        }
    }
    number(n as f64)
}

/// `COUNTA`: every stored cell of a reference, an empty text among them, and each value given
/// directly, an error among them. An empty argument is listed.
pub(super) fn counta(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("COUNTA", args, 1, 255)?;
    let mut n = 0u64;
    for expr in args {
        match ctx.eval(expr)? {
            Operand::Area(sheet, rect) => n += ctx.cells(sheet, rect).count() as u64,
            Operand::Value(Value::Blank) => return Err(listed("COUNTA")),
            Operand::Value(_) => n += 1,
        }
    }
    number(n as f64)
}

/// `COUNTBLANK`: the empty cells of a reference and those with an empty text.
pub(super) fn countblank(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("COUNTBLANK", args, 1, 1)?;
    let (sheet, rect) = take!(reference(ctx, "COUNTBLANK", &args[0])?);
    let mut stored = 0u64;
    let mut empty = 0u64;
    for cell in ctx.cells(sheet, rect) {
        stored += 1;
        if matches!(cell, Value::Text(t) if t.is_empty()) {
            empty += 1;
        }
    }
    number((size(rect) - stored + empty) as f64)
}

/// The variance of `ns`: of a sample, or of the whole population.
fn variance(ns: &[f64], sample: bool) -> Option<f64> {
    let n = ns.len();
    if n == 0 || (sample && n < 2) {
        return None;
    }
    let mean = ns.iter().sum::<f64>() / n as f64;
    let squares: f64 = ns.iter().map(|x| (x - mean) * (x - mean)).sum();
    let divisor = if sample { n - 1 } else { n };
    Some(squares / divisor as f64)
}

/// `SUBTOTAL`: one of eleven aggregates over references, by code.
pub(super) fn subtotal(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("SUBTOTAL", args, 2, 255)?;
    let code = take!(arg(ctx, &args[0])?).trunc();
    let skip_hidden = code > 100.0;
    let code = if skip_hidden { code - 100.0 } else { code };
    if !(1.0..=11.0).contains(&code) {
        return error(CellError::Value);
    }
    let code = code as u8;
    let mut ns = Vec::new();
    let mut stored = 0u64;
    for expr in &args[1..] {
        let (sheet, rect) = take!(reference(ctx, "SUBTOTAL", expr)?);
        if !skip_hidden && ctx.any_hidden(sheet, rect.top, rect.bottom) {
            return Err(listed("SUBTOTAL"));
        }
        for (row, col, cell) in ctx.entries(sheet, rect) {
            if (skip_hidden && ctx.hidden(sheet, row)) || ctx.is_subtotal(sheet, row, col) {
                continue;
            }
            stored += 1;
            match cell {
                Value::Number(n) => ns.push(*n),
                Value::Error(e) if !matches!(code, 2 | 3) => return error(*e),
                _ => {}
            }
        }
    }
    let spread = |sample: bool, root: bool| match variance(&ns, sample) {
        Some(v) => number(if root { v.sqrt() } else { v }),
        None => error(CellError::DivZero),
    };
    match code {
        1 => mean(&ns),
        2 => number(ns.len() as f64),
        3 => number(stored as f64),
        4 => extreme(&ns, true),
        5 => extreme(&ns, false),
        6 if ns.is_empty() => number(0.0),
        6 => number(ns.iter().product()),
        7 => spread(true, true),
        8 => spread(false, true),
        9 => number(ns.iter().sum()),
        10 => spread(true, false),
        _ => spread(false, false),
    }
}

// ---- Criteria ----

#[derive(Clone, Copy, PartialEq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

enum Target {
    Number(f64),
    Bool(bool),
    Error(CellError),
    Text(String),
    /// Nothing after the operator.
    Empty,
}

/// One criterion of `COUNTIF` and its kin.
struct Criterion {
    op: Op,
    target: Target,
    /// Whether the criterion had no operator: `""` matches an empty text, and `=` does not.
    bare: bool,
}

impl Criterion {
    /// The criterion a value gives, or `None` for one whose meaning is not certain.
    fn of(v: &Value) -> Option<Self> {
        let eq = |target| {
            Some(Self {
                op: Op::Eq,
                target,
                bare: true,
            })
        };
        let text = match v {
            Value::Number(n) => return eq(Target::Number(*n)),
            Value::Bool(b) => return eq(Target::Bool(*b)),
            Value::Error(e) => return eq(Target::Error(*e)),
            Value::Blank => return None,
            Value::Text(t) => t,
        };
        let (op, rest) = [
            ("<=", Op::Le),
            (">=", Op::Ge),
            ("<>", Op::Ne),
            ("<", Op::Lt),
            (">", Op::Gt),
            ("=", Op::Eq),
        ]
        .into_iter()
        .find_map(|(prefix, op)| text.strip_prefix(prefix).map(|rest| (Some(op), rest)))
        .unwrap_or((None, text.as_str()));
        let target = if rest.is_empty() {
            Target::Empty
        } else if let Some(n) = number_of_text(rest) {
            Target::Number(n)
        } else if rest.eq_ignore_ascii_case("TRUE") {
            Target::Bool(true)
        } else if rest.eq_ignore_ascii_case("FALSE") {
            Target::Bool(false)
        } else if let Some(e) = CellError::of(&rest.to_ascii_uppercase()) {
            Target::Error(e)
        } else {
            Target::Text(rest.to_owned())
        };
        Some(Self {
            op: op.unwrap_or(Op::Eq),
            target,
            bare: op.is_none(),
        })
    }

    /// Whether `cell` matches, or `None` where Excel's answer is not certain.
    fn test(&self, cell: &Value) -> Option<bool> {
        match self.op {
            Op::Eq => self.equal(cell),
            Op::Ne => match (&self.target, cell) {
                (_, Value::Error(_)) => None,
                (Target::Empty, Value::Text(t)) if t.is_empty() => None,
                _ => self.equal(cell).map(|b| !b),
            },
            op => {
                let order = match (&self.target, cell) {
                    (Target::Number(n), Value::Number(c)) => c.partial_cmp(n)?,
                    (Target::Text(t), Value::Text(c)) => c.to_lowercase().cmp(&t.to_lowercase()),
                    (Target::Number(_) | Target::Text(_), _) => return Some(false),
                    _ => return None,
                };
                Some(match op {
                    Op::Lt => order == Ordering::Less,
                    Op::Le => order != Ordering::Greater,
                    Op::Gt => order == Ordering::Greater,
                    _ => order != Ordering::Less,
                })
            }
        }
    }

    fn equal(&self, cell: &Value) -> Option<bool> {
        Some(match (&self.target, cell) {
            (Target::Empty, Value::Blank) => true,
            (Target::Empty, Value::Text(t)) => self.bare && t.is_empty(),
            (Target::Empty, _) => false,
            (Target::Number(n), Value::Number(c)) => c == n,
            (Target::Number(n), Value::Text(t)) => number_of_text(t) == Some(*n),
            (Target::Bool(b), Value::Bool(c)) => b == c,
            (Target::Bool(_), Value::Text(t))
                if t.eq_ignore_ascii_case("TRUE") || t.eq_ignore_ascii_case("FALSE") =>
            {
                return None;
            }
            (Target::Error(e), Value::Error(c)) => e == c,
            (Target::Text(t), Value::Text(c)) => wildcard(t, c),
            _ => false,
        })
    }
}

/// One criteria range of `COUNTIFS` and its kin, with its criterion.
struct Range {
    sheet: usize,
    rect: Rect,
    criterion: Criterion,
}

/// The ranges and criteria of `pairs`, which alternate. Ranges of different shapes are `#VALUE!`.
fn ranges(
    ctx: &mut Ctx<'_>,
    name: &str,
    pairs: &[Expr],
) -> Result<Result<Vec<Range>, CellError>, Miss> {
    let mut out: Vec<Range> = Vec::new();
    for pair in pairs.chunks(2) {
        let (sheet, rect) = match reference(ctx, name, &pair[0])? {
            Ok(at) => at,
            Err(e) => return Ok(Err(e)),
        };
        let criterion = Criterion::of(&ctx.scalar(&pair[1])?).ok_or_else(|| listed(name))?;
        if out
            .first()
            .is_some_and(|first| shape(first.rect) != shape(rect))
        {
            return Ok(Err(CellError::Value));
        }
        out.push(Range {
            sheet,
            rect,
            criterion,
        });
    }
    Ok(Ok(out))
}

/// Whether every range matches its criterion at the offset `(dr, dc)` from its top-left.
fn all_match(ctx: &Ctx<'_>, name: &str, ranges: &[Range], dr: u32, dc: u32) -> Result<bool, Miss> {
    for range in ranges {
        let cell = ctx.at(range.sheet, range.rect.top + dr, range.rect.left + dc);
        if !range.criterion.test(cell).ok_or_else(|| listed(name))? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// How many positions match every criterion. When a criterion cannot match a blank, only its
/// range's stored cells are tried. Otherwise a position fails only at a stored cell, so the
/// stored positions of every range are tried and the failures taken from the area's size.
fn count_matches(ctx: &Ctx<'_>, name: &str, ranges: &[Range]) -> Result<f64, Miss> {
    let mut blank_fails = None;
    for (at, range) in ranges.iter().enumerate() {
        if !range
            .criterion
            .test(&Value::Blank)
            .ok_or_else(|| listed(name))?
        {
            blank_fails = Some(at);
            break;
        }
    }
    if let Some(at) = blank_fails {
        let (sheet, rect) = (ranges[at].sheet, ranges[at].rect);
        let mut n = 0u64;
        for (row, col, _) in ctx.entries(sheet, rect) {
            if all_match(ctx, name, ranges, row - rect.top, col - rect.left)? {
                n += 1;
            }
        }
        return Ok(n as f64);
    }
    let mut stored = BTreeSet::new();
    for range in ranges {
        for (row, col, _) in ctx.entries(range.sheet, range.rect) {
            stored.insert((row - range.rect.top, col - range.rect.left));
        }
    }
    let mut fails = 0u64;
    for (dr, dc) in stored {
        if !all_match(ctx, name, ranges, dr, dc)? {
            fails += 1;
        }
    }
    Ok((size(ranges[0].rect) - fails) as f64)
}

/// The numbers of `values` at positions where every criterion matches. An error there is the
/// result.
fn matched_numbers(
    ctx: &Ctx<'_>,
    name: &str,
    sheet: usize,
    values: Rect,
    ranges: &[Range],
) -> Result<Result<Vec<f64>, CellError>, Miss> {
    let mut out = Vec::new();
    for (row, col, cell) in ctx.entries(sheet, values) {
        let matched = match cell {
            Value::Number(_) | Value::Error(_) => {
                all_match(ctx, name, ranges, row - values.top, col - values.left)?
            }
            _ => false,
        };
        if matched {
            match cell {
                Value::Number(n) => out.push(*n),
                Value::Error(e) => return Ok(Err(*e)),
                _ => {}
            }
        }
    }
    Ok(Ok(out))
}

pub(super) fn countif(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("COUNTIF", args, 2, 2)?;
    let ranges = take!(ranges(ctx, "COUNTIF", args)?);
    number(count_matches(ctx, "COUNTIF", &ranges)?)
}

pub(super) fn countifs(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("COUNTIFS", args, 2, 254)?;
    if !args.len().is_multiple_of(2) {
        return Err(listed("COUNTIFS"));
    }
    let ranges = take!(ranges(ctx, "COUNTIFS", args)?);
    number(count_matches(ctx, "COUNTIFS", &ranges)?)
}

/// The numbers `SUMIF` or `AVERAGEIF` reads. A value range of another size is taken at the
/// criteria range's size from its top-left cell, as Excel does.
fn if_numbers(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
) -> Result<Result<Vec<f64>, CellError>, Miss> {
    arity(name, args, 2, 3)?;
    let ranges = match ranges(ctx, name, &args[..2])? {
        Ok(r) => r,
        Err(e) => return Ok(Err(e)),
    };
    let (sheet, top_left) = match args.get(2) {
        None => (ranges[0].sheet, ranges[0].rect),
        Some(expr) => match reference(ctx, name, expr)? {
            Ok(at) => at,
            Err(e) => return Ok(Err(e)),
        },
    };
    let (rows, cols) = shape(ranges[0].rect);
    let values = Rect {
        top: top_left.top,
        left: top_left.left,
        bottom: top_left.top + rows,
        right: top_left.left + cols,
    };
    matched_numbers(ctx, name, sheet, values, &ranges)
}

/// The numbers `SUMIFS` and its kin read: the first argument's numbers where every criterion
/// matches. A range of another shape is `#VALUE!`.
fn ifs_numbers(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
) -> Result<Result<Vec<f64>, CellError>, Miss> {
    arity(name, args, 3, 255)?;
    if args.len().is_multiple_of(2) {
        return Err(listed(name));
    }
    let (sheet, values) = match reference(ctx, name, &args[0])? {
        Ok(at) => at,
        Err(e) => return Ok(Err(e)),
    };
    let ranges = match ranges(ctx, name, &args[1..])? {
        Ok(r) => r,
        Err(e) => return Ok(Err(e)),
    };
    if shape(ranges[0].rect) != shape(values) {
        return Ok(Err(CellError::Value));
    }
    matched_numbers(ctx, name, sheet, values, &ranges)
}

pub(super) fn sumif(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let ns = take!(if_numbers(ctx, "SUMIF", args)?);
    number(ns.iter().sum())
}

pub(super) fn averageif(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let ns = take!(if_numbers(ctx, "AVERAGEIF", args)?);
    mean(&ns)
}

pub(super) fn sumifs(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let ns = take!(ifs_numbers(ctx, "SUMIFS", args)?);
    number(ns.iter().sum())
}

pub(super) fn averageifs(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let ns = take!(ifs_numbers(ctx, "AVERAGEIFS", args)?);
    mean(&ns)
}

pub(super) fn maxifs(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let ns = take!(ifs_numbers(ctx, "MAXIFS", args)?);
    extreme(&ns, true)
}

pub(super) fn minifs(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let ns = take!(ifs_numbers(ctx, "MINIFS", args)?);
    extreme(&ns, false)
}

// ---- Logic ----

pub(super) fn true_(_ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("TRUE", args, 0, 0)?;
    value(Value::Bool(true))
}

pub(super) fn false_(_ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("FALSE", args, 0, 0)?;
    value(Value::Bool(false))
}

pub(super) fn not(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("NOT", args, 1, 1)?;
    let b = take!(ctx.scalar(&args[0])?.boolean());
    value(Value::Bool(!b))
}

/// How many of the conditions `AND`, `OR` and `XOR` read are true, of how many. A reference
/// gives its booleans and numbers and skips text and blanks. None at all is `#VALUE!`.
fn conditions(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
) -> Result<Result<(u64, u64), CellError>, Miss> {
    arity(name, args, 1, 255)?;
    let (mut trues, mut all) = (0u64, 0u64);
    for expr in args {
        match ctx.eval(expr)? {
            Operand::Area(sheet, rect) => {
                for cell in ctx.cells(sheet, rect) {
                    let b = match cell {
                        Value::Bool(b) => *b,
                        Value::Number(n) => *n != 0.0,
                        Value::Error(e) => return Ok(Err(*e)),
                        _ => continue,
                    };
                    trues += u64::from(b);
                    all += 1;
                }
            }
            Operand::Value(Value::Blank) => return Err(listed(name)),
            Operand::Value(v) => match v.boolean() {
                Ok(b) => {
                    trues += u64::from(b);
                    all += 1;
                }
                Err(e) => return Ok(Err(e)),
            },
        }
    }
    if all == 0 {
        return Ok(Err(CellError::Value));
    }
    Ok(Ok((trues, all)))
}

pub(super) fn and(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let (trues, all) = take!(conditions(ctx, "AND", args)?);
    value(Value::Bool(trues == all))
}

pub(super) fn or(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let (trues, _) = take!(conditions(ctx, "OR", args)?);
    value(Value::Bool(trues > 0))
}

pub(super) fn xor(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    let (trues, _) = take!(conditions(ctx, "XOR", args)?);
    value(Value::Bool(!trues.is_multiple_of(2)))
}

/// `IFERROR` and `IFNA`: the first argument, or the second when the first is an error `catches`
/// takes. A reference of more than one cell would be an array, so it is listed.
fn catch(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
    catches: fn(CellError) -> bool,
) -> Result<Operand, Miss> {
    arity(name, args, 2, 2)?;
    let got = ctx.eval(&args[0])?;
    let caught = match &got {
        Operand::Value(Value::Error(e)) => catches(*e),
        Operand::Value(_) => false,
        Operand::Area(sheet, rect) if rect.top == rect.bottom && rect.left == rect.right => {
            matches!(ctx.at(*sheet, rect.top, rect.left), Value::Error(e) if catches(*e))
        }
        Operand::Area(..) => return Err(listed(name)),
    };
    if caught { ctx.eval(&args[1]) } else { Ok(got) }
}

pub(super) fn iferror(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    catch(ctx, "IFERROR", args, |_| true)
}

pub(super) fn ifna(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    catch(ctx, "IFNA", args, |e| e == CellError::NotAvailable)
}

/// `IFS`: the value after the first true condition, and `#N/A` when none is true.
pub(super) fn ifs(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("IFS", args, 2, 254)?;
    if !args.len().is_multiple_of(2) {
        return Err(listed("IFS"));
    }
    for pair in args.chunks(2) {
        if take!(ctx.scalar(&pair[0])?.boolean()) {
            return ctx.eval(&pair[1]);
        }
    }
    error(CellError::NotAvailable)
}

/// `SWITCH`: the result after the first value equal to the first argument, then the default,
/// and `#N/A` without one. Text compares without case.
pub(super) fn switch(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("SWITCH", args, 3, 254)?;
    let subject = ctx.scalar(&args[0])?;
    if let Value::Error(e) = &subject {
        return error(*e);
    }
    let cases = &args[1..];
    let paired = cases.len() - cases.len() % 2;
    for pair in cases[..paired].chunks(2) {
        let case = ctx.scalar(&pair[0])?;
        if take!(subject.compare(&case)) == Ordering::Equal {
            return ctx.eval(&pair[1]);
        }
    }
    match cases.get(paired) {
        Some(default) => ctx.eval(default),
        None => error(CellError::NotAvailable),
    }
}

// ---- Information ----

/// An `IS` function: a test of its argument's value, an error value among them.
fn is(
    ctx: &mut Ctx<'_>,
    name: &str,
    args: &[Expr],
    test: fn(&Value) -> bool,
) -> Result<Operand, Miss> {
    arity(name, args, 1, 1)?;
    let v = ctx.scalar(&args[0])?;
    value(Value::Bool(test(&v)))
}

pub(super) fn isblank(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    is(ctx, "ISBLANK", args, |v| matches!(v, Value::Blank))
}

pub(super) fn iserr(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    is(
        ctx,
        "ISERR",
        args,
        |v| matches!(v, Value::Error(e) if *e != CellError::NotAvailable),
    )
}

pub(super) fn iserror(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    is(ctx, "ISERROR", args, |v| matches!(v, Value::Error(_)))
}

pub(super) fn isna(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    is(ctx, "ISNA", args, |v| {
        matches!(v, Value::Error(CellError::NotAvailable))
    })
}

pub(super) fn islogical(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    is(ctx, "ISLOGICAL", args, |v| matches!(v, Value::Bool(_)))
}

pub(super) fn isnumber(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    is(ctx, "ISNUMBER", args, |v| matches!(v, Value::Number(_)))
}

pub(super) fn istext(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    is(ctx, "ISTEXT", args, |v| matches!(v, Value::Text(_)))
}

/// `ISEVEN` and `ISODD`: the number truncated to a whole one. A boolean is listed.
fn parity(ctx: &mut Ctx<'_>, name: &str, args: &[Expr], odd: bool) -> Result<Operand, Miss> {
    arity(name, args, 1, 1)?;
    let v = ctx.scalar(&args[0])?;
    if matches!(v, Value::Bool(_)) {
        return Err(listed(name));
    }
    let n = take!(v.number()).trunc();
    value(Value::Bool((n % 2.0 != 0.0) == odd))
}

pub(super) fn iseven(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    parity(ctx, "ISEVEN", args, false)
}

pub(super) fn isodd(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    parity(ctx, "ISODD", args, true)
}

pub(super) fn na(_ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Operand, Miss> {
    arity("NA", args, 0, 0)?;
    error(CellError::NotAvailable)
}
