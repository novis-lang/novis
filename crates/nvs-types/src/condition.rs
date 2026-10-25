//! A breakpoint's condition and a logpoint's `{…}` holes: an expression over the stopped frame's
//! variables, checked against the frame's slot table when the breakpoint is set
//! (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`).
//!
//! [`check_condition`] and [`check_log_message`] are this crate's entries beside
//! [`crate::check_program`]. The text is parsed by the language's own parser, and the result is a
//! [`Probe`]: a tree that reads variables, properties and array elements, and combines them with
//! literals, operators and comparisons. Anything else is an error with a message the editor
//! shows: a call, an assignment, an `await`, an increment, a `new`, a class constant. So a
//! breakpoint never runs code of the program and never changes what it does.
//!
//! A slot's type is the representation the frame's slot table gives it ([`SlotType`]). Where both
//! sides of an operator have a known scalar type, the pair is checked here, by the rows of
//! `rule:types/arithmetic` and the comparisons a program may write. A side whose type the table
//! does not know (an object, an array, a union, a property) is checked when the probe runs, and
//! a mismatch there is an evaluation error. A condition whose type is known must be a `bool`.
//!
//! A `secret` variable cannot be read by a condition, because the program would stop or run on
//! by its value. A logpoint's hole may name one, and the hole shows it redacted.

use nvs_diagnostics::{Diagnostics, SourceFile, SourceMap, Span};
use nvs_syntax::ast::{BinaryOp, Expr, ExprKind, MemberName, UnaryOp};

/// The type a frame's slot table gives a variable, as far as a probe's check needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotType {
    /// `bool`.
    Bool,
    /// `int`.
    Int,
    /// `uint`.
    Uint,
    /// `float`.
    Float,
    /// `string`.
    Str,
    /// Anything else: an object, an array, a union, a `decimal`. Checked when the probe runs.
    Other,
}

/// One slot of the stopped frame's slot table, in slot order.
#[derive(Clone, Copy, Debug)]
pub struct ProbeSlot<'a> {
    /// The variable's name, without its `$`.
    pub name: &'a str,
    /// Its representation.
    pub ty: SlotType,
    /// Whether its type is `secret`.
    pub secret: bool,
}

/// A literal a probe compares against.
#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer literal.
    Int(i64),
    /// A float literal.
    Float(f64),
    /// A string literal, its escapes decoded.
    Str(String),
}

/// An operator a probe may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    missing_docs,
    reason = "each variant is exactly its operator, as `nvs_syntax::ast::BinaryOp` names it"
)]
pub enum ProbeOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Concat,
    And,
    Or,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Coalesce,
}

/// A checked condition or hole.
#[derive(Clone, Debug, PartialEq)]
pub enum Probe {
    /// A literal.
    Literal(Literal),
    /// The frame's slot at this index.
    Slot(usize),
    /// A property of an object, `?->` when `nullsafe`.
    Property {
        /// The object.
        object: Box<Probe>,
        /// The property's name.
        name: String,
        /// Whether a `null` object gives `null`.
        nullsafe: bool,
    },
    /// An element of an array.
    Index {
        /// The array.
        base: Box<Probe>,
        /// The key.
        key: Box<Probe>,
    },
    /// `!operand`.
    Not(Box<Probe>),
    /// `-operand`.
    Neg(Box<Probe>),
    /// `lhs op rhs`.
    Binary {
        /// The operator.
        op: ProbeOp,
        /// The left side.
        lhs: Box<Probe>,
        /// The right side.
        rhs: Box<Probe>,
    },
}

/// One run of a logpoint's message: text written as it is, or a hole whose value is written.
#[derive(Clone, Debug, PartialEq)]
pub enum LogPart {
    /// Text outside any `{…}`.
    Text(String),
    /// A `{…}` hole.
    Hole(Probe),
}

/// What a probe's value is known to be when it is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Known {
    Null,
    Bool,
    Int,
    Uint,
    Float,
    Str,
    /// Checked when the probe runs.
    Any,
}

impl Known {
    fn of(ty: SlotType) -> Self {
        match ty {
            SlotType::Bool => Self::Bool,
            SlotType::Int => Self::Int,
            SlotType::Uint => Self::Uint,
            SlotType::Float => Self::Float,
            SlotType::Str => Self::Str,
            SlotType::Other => Self::Any,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Float => "float",
            Self::Str => "string",
            Self::Any => "mixed",
        }
    }

    fn is_number(self) -> bool {
        matches!(self, Self::Int | Self::Uint | Self::Float)
    }
}

/// Checks a breakpoint's condition against the stopped frame's slots. Returns the probe, or the
/// message the editor shows on the breakpoint.
///
/// # Errors
///
/// The message, when the text does not parse, uses anything but a read, a literal or an operator,
/// names a variable the frame does not have or a `secret` one, or is known not to be a `bool`.
pub fn check_condition(text: &str, slots: &[ProbeSlot<'_>]) -> Result<Probe, String> {
    let (probe, known) = check_expression(text, slots, false)?;
    match known {
        Known::Bool | Known::Any => Ok(probe),
        other => Err(format!(
            "A condition must be a `bool`, and this one is `{}`.",
            other.name()
        )),
    }
}

/// Checks a logpoint's message: text with `{…}` holes, each an expression over the stopped
/// frame's slots. Returns its parts in order, or the message the editor shows on the logpoint.
///
/// # Errors
///
/// The message, when a `{` has no `}`, or a hole fails as [`check_condition`] fails. A hole may
/// be of any type, and may name a `secret` variable.
pub fn check_log_message(text: &str, slots: &[ProbeSlot<'_>]) -> Result<Vec<LogPart>, String> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        if open > 0 {
            parts.push(LogPart::Text(rest[..open].to_owned()));
        }
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or_else(|| "A `{` in the log message has no `}` after it.".to_owned())?;
        let (probe, _) = check_expression(&after[..close], slots, true)?;
        parts.push(LogPart::Hole(probe));
        rest = &after[close + 1..];
    }
    if !rest.is_empty() {
        parts.push(LogPart::Text(rest.to_owned()));
    }
    Ok(parts)
}

/// Parses `text` as one expression and checks it. `reads_secret` lets it name a `secret`
/// variable.
fn check_expression(
    text: &str,
    slots: &[ProbeSlot<'_>],
    reads_secret: bool,
) -> Result<(Probe, Known), String> {
    // A shebang line opens code mode, so the expression needs no open tag
    // (`rule:tooling/shebang-opens-code-mode`).
    const LEAD: &str = "#!\n";
    let mut map = SourceMap::new();
    let id = map.add("<condition>", format!("{LEAD}{text}"));
    let file = map.file(id);
    let mut diags = Diagnostics::new();
    let expr = nvs_syntax::parse_expression(file, &mut diags);
    if let Some(diagnostic) = diags.iter().next() {
        return Err(diagnostic.message.clone());
    }
    let end = usize::try_from(expr.span.end).unwrap_or(usize::MAX);
    if !file
        .text()
        .get(end..)
        .is_some_and(|rest| rest.trim().is_empty())
    {
        return Err("The expression has more text after its end.".to_owned());
    }
    let checker = Checker {
        file,
        slots,
        reads_secret,
    };
    checker.expr(&expr)
}

struct Checker<'a> {
    file: &'a SourceFile,
    slots: &'a [ProbeSlot<'a>],
    reads_secret: bool,
}

impl Checker<'_> {
    fn text(&self, span: Span) -> &str {
        self.file.span_text(span).unwrap_or_default()
    }

    fn expr(&self, expr: &Expr) -> Result<(Probe, Known), String> {
        let literal = |literal, known| Ok((Probe::Literal(literal), known));
        match &expr.kind {
            ExprKind::Paren(inner) => self.expr(inner),
            ExprKind::Null => literal(Literal::Null, Known::Null),
            ExprKind::Bool(value) => literal(Literal::Bool(*value), Known::Bool),
            ExprKind::Int(span) => {
                let (radix, digits) = crate::expr::int_literal_digits(self.file, *span);
                let value = i64::from_str_radix(&digits, radix)
                    .map_err(|_| format!("`{}` is too large for an `int`.", self.text(*span)))?;
                literal(Literal::Int(value), Known::Int)
            }
            ExprKind::Float(span) => {
                let written: String = self.text(*span).chars().filter(|&c| c != '_').collect();
                let value = written
                    .parse::<f64>()
                    .map_err(|_| format!("`{written}` is not a float."))?;
                literal(Literal::Float(value), Known::Float)
            }
            ExprKind::Str(span) => literal(
                Literal::Str(nvs_syntax::string_lit::cook_string_literal(
                    self.file, *span,
                )),
                Known::Str,
            ),
            ExprKind::Variable(span) => self.variable(*span),
            ExprKind::PropertyAccess {
                object,
                nullsafe,
                property,
            } => {
                let MemberName::Ident(name) = property else {
                    return Err("A property in a condition must be written by its name.".to_owned());
                };
                let (object, _) = self.expr(object)?;
                let probe = Probe::Property {
                    object: Box::new(object),
                    name: self.text(*name).to_owned(),
                    nullsafe: *nullsafe,
                };
                Ok((probe, Known::Any))
            }
            ExprKind::Index {
                base,
                index: Some(index),
            } => {
                let (base, _) = self.expr(base)?;
                let (key, known) = self.expr(index)?;
                if !matches!(known, Known::Int | Known::Str | Known::Any) {
                    return Err(format!(
                        "An array key is an `int` or a `string`, and this one is `{}`.",
                        known.name()
                    ));
                }
                let probe = Probe::Index {
                    base: Box::new(base),
                    key: Box::new(key),
                };
                Ok((probe, Known::Any))
            }
            ExprKind::Unary { op, expr: operand } => self.unary(*op, operand),
            ExprKind::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs),
            ExprKind::Call { .. }
            | ExprKind::MethodCall { .. }
            | ExprKind::StaticCall { .. }
            | ExprKind::New { .. } => {
                Err("A condition cannot call a function or a method.".to_owned())
            }
            ExprKind::Assign { .. } | ExprKind::PreIncDec { .. } | ExprKind::PostIncDec { .. } => {
                Err("A condition cannot change a variable.".to_owned())
            }
            ExprKind::Await(_) => Err("A condition cannot use `await`.".to_owned()),
            _ => Err(
                "A condition can only read variables, properties and array elements, \
                 and use literals, operators and comparisons."
                    .to_owned(),
            ),
        }
    }

    fn variable(&self, span: Span) -> Result<(Probe, Known), String> {
        let written = self.text(span);
        let name = written.strip_prefix('$').unwrap_or(written);
        let (index, slot) = self
            .slots
            .iter()
            .enumerate()
            .find(|(_, slot)| slot.name == name)
            .ok_or_else(|| format!("`{written}` is not a variable here."))?;
        if slot.secret && !self.reads_secret {
            return Err(format!(
                "`{written}` is secret, so a condition cannot read it."
            ));
        }
        Ok((Probe::Slot(index), Known::of(slot.ty)))
    }

    fn unary(&self, op: UnaryOp, operand: &Expr) -> Result<(Probe, Known), String> {
        let (probe, known) = self.expr(operand)?;
        match op {
            UnaryOp::Not if matches!(known, Known::Bool | Known::Any) => {
                Ok((Probe::Not(Box::new(probe)), Known::Bool))
            }
            UnaryOp::Neg if matches!(known, Known::Int | Known::Float | Known::Any) => {
                Ok((Probe::Neg(Box::new(probe)), known))
            }
            UnaryOp::Plus if matches!(known, Known::Int | Known::Float | Known::Any) => {
                Ok((probe, known))
            }
            UnaryOp::Not | UnaryOp::Neg | UnaryOp::Plus => Err(format!(
                "This operator cannot be used on a `{}`.",
                known.name()
            )),
            _ => Err("A condition cannot use this operator.".to_owned()),
        }
    }

    fn binary(&self, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> Result<(Probe, Known), String> {
        let (left, l) = self.expr(lhs)?;
        let (right, r) = self.expr(rhs)?;
        let mismatch = || {
            Err(format!(
                "`{}` and `{}` cannot be used together with this operator.",
                l.name(),
                r.name()
            ))
        };
        let either_any = l == Known::Any || r == Known::Any;
        let (op, known) = match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let known = match (l, r) {
                    _ if either_any
                        && (l.is_number() || l == Known::Any)
                        && (r.is_number() || r == Known::Any) =>
                    {
                        Known::Any
                    }
                    (Known::Int, Known::Int) | (Known::Uint, Known::Uint)
                        if op == BinaryOp::Div =>
                    {
                        Known::Any
                    }
                    (Known::Int, Known::Int) => Known::Int,
                    (Known::Uint, Known::Uint) => Known::Uint,
                    (Known::Float, Known::Float | Known::Int | Known::Uint)
                    | (Known::Int | Known::Uint, Known::Float) => Known::Float,
                    _ => return mismatch(),
                };
                let op = match op {
                    BinaryOp::Add => ProbeOp::Add,
                    BinaryOp::Sub => ProbeOp::Sub,
                    BinaryOp::Mul => ProbeOp::Mul,
                    _ => ProbeOp::Div,
                };
                (op, known)
            }
            BinaryOp::Mod => match (l, r) {
                (Known::Int, Known::Int) => (ProbeOp::Mod, Known::Int),
                (Known::Uint, Known::Uint) => (ProbeOp::Mod, Known::Uint),
                (Known::Int | Known::Uint | Known::Any, Known::Int | Known::Uint | Known::Any) => {
                    (ProbeOp::Mod, Known::Any)
                }
                _ => return mismatch(),
            },
            BinaryOp::Concat => match (l, r) {
                (Known::Str | Known::Any, Known::Str | Known::Any) => (ProbeOp::Concat, Known::Str),
                _ => return mismatch(),
            },
            BinaryOp::And | BinaryOp::Or => match (l, r) {
                (Known::Bool | Known::Any, Known::Bool | Known::Any) => {
                    let op = if op == BinaryOp::And {
                        ProbeOp::And
                    } else {
                        ProbeOp::Or
                    };
                    (op, Known::Bool)
                }
                _ => return mismatch(),
            },
            BinaryOp::Eq | BinaryOp::NotEq => {
                let comparable = either_any
                    || l == r
                    || l == Known::Null
                    || r == Known::Null
                    || (l.is_number() && r.is_number());
                if !comparable {
                    return mismatch();
                }
                let op = if op == BinaryOp::Eq {
                    ProbeOp::Eq
                } else {
                    ProbeOp::NotEq
                };
                (op, Known::Bool)
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                let ordered = |k: Known| k.is_number() || k == Known::Str || k == Known::Any;
                let comparable = ordered(l)
                    && ordered(r)
                    && (either_any || l == r || (l.is_number() && r.is_number()));
                if !comparable {
                    return mismatch();
                }
                let op = match op {
                    BinaryOp::Lt => ProbeOp::Lt,
                    BinaryOp::LtEq => ProbeOp::LtEq,
                    BinaryOp::Gt => ProbeOp::Gt,
                    _ => ProbeOp::GtEq,
                };
                (op, Known::Bool)
            }
            BinaryOp::Coalesce => {
                let known = if l == r { l } else { Known::Any };
                (ProbeOp::Coalesce, known)
            }
            _ => return Err("A condition cannot use this operator.".to_owned()),
        };
        let probe = Probe::Binary {
            op,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
        Ok((probe, known))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOTS: &[ProbeSlot<'static>] = &[
        ProbeSlot {
            name: "count",
            ty: SlotType::Int,
            secret: false,
        },
        ProbeSlot {
            name: "name",
            ty: SlotType::Str,
            secret: false,
        },
        ProbeSlot {
            name: "user",
            ty: SlotType::Other,
            secret: false,
        },
        ProbeSlot {
            name: "token",
            ty: SlotType::Str,
            secret: true,
        },
    ];

    #[test]
    fn a_comparison_of_a_variable_and_a_literal_checks() {
        let probe = check_condition("$count > 3", SLOTS).expect("the condition checks");
        assert_eq!(
            probe,
            Probe::Binary {
                op: ProbeOp::Gt,
                lhs: Box::new(Probe::Slot(0)),
                rhs: Box::new(Probe::Literal(Literal::Int(3))),
            }
        );
        check_condition("$user->name == 'Ada' && $user->tags[0] != null", SLOTS)
            .expect("a property and an element are read");
    }

    #[test]
    fn a_call_an_assignment_and_an_await_are_errors() {
        for text in [
            "count($items) > 3",
            "$user->isAdmin()",
            "$count = 4",
            "$count++",
        ] {
            assert!(check_condition(text, SLOTS).is_err(), "{text} is refused");
        }
        assert_eq!(
            check_condition("$user->isAdmin()", SLOTS),
            Err("A condition cannot call a function or a method.".to_owned())
        );
    }

    #[test]
    fn a_condition_that_is_not_a_bool_or_mixes_types_is_an_error() {
        assert_eq!(
            check_condition("$count + 1", SLOTS),
            Err("A condition must be a `bool`, and this one is `int`.".to_owned())
        );
        assert!(check_condition("$count == 'three'", SLOTS).is_err());
        assert!(check_condition("$missing > 1", SLOTS).is_err());
        assert!(check_condition("$count > 1 2", SLOTS).is_err());
    }

    #[test]
    fn a_secret_variable_is_read_by_a_hole_and_not_by_a_condition() {
        assert!(check_condition("$token == 'x'", SLOTS).is_err());
        let parts = check_log_message("count is {$count}, token {$token}", SLOTS)
            .expect("the message checks");
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], LogPart::Text("count is ".to_owned()));
        assert_eq!(parts[1], LogPart::Hole(Probe::Slot(0)));
        assert!(check_log_message("count is {$count", SLOTS).is_err());
    }
}
