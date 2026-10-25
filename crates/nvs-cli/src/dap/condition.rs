//! A breakpoint's condition, hit condition and log message, and the test the controller runs at
//! each of its armed sites (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`).
//!
//! [`Rule`] is what `setBreakpoints` makes of one source breakpoint. The controller binds it to
//! the slot table of each function with a site on its line, and binding checks the condition and
//! each `{…}` hole of the log message with [`nvs_types::condition`]. A condition or hole that does
//! not check leaves the breakpoint unverified, with the checker's message. A hit condition that
//! does not parse does the same.
//!
//! At an armed site the bound test runs in this order: the condition, then the hit count, then
//! the log message. A hit is counted only where the condition is true, and the count is the
//! breakpoint's, across every site and every unit. A logpoint writes its message as an `output`
//! event and never stops. A `secret` value in a hole is shown redacted.
//!
//! **A probe is evaluated over the frame's values, and is not compiled.** It reads slots,
//! properties and elements through [`nvs_runtime::record`]'s readers, which run no program code,
//! and applies `rule:types/arithmetic`'s rows itself. An evaluation that fails stops the program:
//! a property of `null`, a missing key, an overflow, a division by zero, a `secret` value used by
//! an operator, or a result that is not a `bool`. The failure is written as an `output` event, so
//! a wrong condition is seen at once.
//!
//! **What it spends:** one checked tree per breakpoint per function on its line, per loaded unit,
//! and one counter per breakpoint. Evaluating copies the strings it reads.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use nvs_host::debug::{BreakpointTest, DebugFunction, SiteTest, VariableType};
use nvs_render::Node;
use nvs_runtime::record::{self, Key, Read};
use nvs_runtime::{Tag, Value};
use nvs_types::condition::{
    self, Literal, LogPart, Probe, ProbeOp, ProbeSlot, SlotType, check_condition,
};

/// Writes one line of the session's debug console.
pub(super) type Say = Arc<dyn Fn(String) + Send + Sync>;

/// When a hit counts as a stop: `N` and `>=N`, `>N`, `==N` or `%N`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HitCondition {
    AtLeast(u64),
    Above(u64),
    Exactly(u64),
    Every(u64),
}

impl HitCondition {
    fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let (make, digits): (fn(u64) -> Self, &str) = if let Some(rest) = text.strip_prefix(">=") {
            (Self::AtLeast, rest)
        } else if let Some(rest) = text.strip_prefix("==") {
            (Self::Exactly, rest)
        } else if let Some(rest) = text.strip_prefix('>') {
            (Self::Above, rest)
        } else if let Some(rest) = text.strip_prefix('%') {
            (Self::Every, rest)
        } else {
            (Self::AtLeast, text)
        };
        let count = digits.trim().parse::<u64>().map_err(|_| {
            format!("The hit condition `{text}` is not `N`, `>N`, `>=N`, `==N` or `%N`.")
        })?;
        if matches!(make(count), Self::Every(0)) {
            return Err("The hit condition `%0` never stops.".to_owned());
        }
        Ok(make(count))
    }

    fn stops(self, hits: u64) -> bool {
        match self {
            Self::AtLeast(count) => hits >= count,
            Self::Above(count) => hits > count,
            Self::Exactly(count) => hits == count,
            Self::Every(count) => hits.is_multiple_of(count),
        }
    }
}

/// One source breakpoint's condition, hit condition and log message.
pub(super) struct Rule {
    condition: Option<String>,
    hit: Result<Option<HitCondition>, String>,
    log: Option<String>,
    /// Where the breakpoint is, for the messages it writes.
    at: String,
    hits: Arc<AtomicU64>,
    say: Say,
}

impl Rule {
    /// The rule of a breakpoint at `at`, or `None` where it has no condition, hit condition or
    /// log message and so always stops. An empty text counts as none, as editors send one.
    pub(super) fn new(
        condition: Option<&str>,
        hit: Option<&str>,
        log: Option<&str>,
        at: String,
        say: Say,
    ) -> Option<Self> {
        fn given(text: Option<&str>) -> Option<&str> {
            text.filter(|text| !text.trim().is_empty())
        }
        let (condition, hit, log) = (given(condition), given(hit), given(log));
        if condition.is_none() && hit.is_none() && log.is_none() {
            return None;
        }
        Some(Self {
            condition: condition.map(str::to_owned),
            hit: hit.map(HitCondition::parse).transpose(),
            log: log.map(str::to_owned),
            at,
            hits: Arc::new(AtomicU64::new(0)),
            say,
        })
    }
}

impl BreakpointTest for Rule {
    fn bind(&self, function: &DebugFunction) -> Result<Arc<dyn SiteTest>, String> {
        let hit = self.hit.clone()?;
        let slots: Vec<ProbeSlot<'_>> = function
            .slots
            .iter()
            .map(|slot| ProbeSlot {
                name: &slot.name,
                ty: slot_type(slot.ty),
                secret: slot.secret,
            })
            .collect();
        let condition = self
            .condition
            .as_deref()
            .map(|text| check_condition(text, &slots))
            .transpose()?;
        let log = self
            .log
            .as_deref()
            .map(|text| condition::check_log_message(text, &slots))
            .transpose()?;
        Ok(Arc::new(Bound {
            condition,
            hit,
            log,
            secret: function.slots.iter().map(|slot| slot.secret).collect(),
            at: self.at.clone(),
            hits: Arc::clone(&self.hits),
            say: Arc::clone(&self.say),
        }))
    }
}

fn slot_type(ty: VariableType) -> SlotType {
    match ty {
        VariableType::Bool => SlotType::Bool,
        VariableType::Int => SlotType::Int,
        VariableType::Uint => SlotType::Uint,
        VariableType::Float => SlotType::Float,
        VariableType::Str => SlotType::Str,
        VariableType::Other => SlotType::Other,
    }
}

/// A [`Rule`] checked against one function's slot table.
struct Bound {
    condition: Option<Probe>,
    hit: Option<HitCondition>,
    log: Option<Vec<LogPart>>,
    /// Whether each slot is `secret`, in slot order.
    secret: Vec<bool>,
    at: String,
    hits: Arc<AtomicU64>,
    say: Say,
}

impl SiteTest for Bound {
    fn stops(&self, slots: &[Value]) -> bool {
        let frame = Frame {
            slots,
            secret: &self.secret,
        };
        if let Some(condition) = &self.condition {
            match frame.eval(condition) {
                Ok(Val::Bool(true)) => {}
                Ok(Val::Bool(false)) => return false,
                Ok(other) => {
                    let message = format!("its result is `{}`, not a `bool`", other.type_name());
                    return self.failed("condition", &message);
                }
                Err(message) => return self.failed("condition", &message),
            }
        }
        let hits = self.hits.fetch_add(1, Ordering::Relaxed) + 1;
        if self.hit.is_some_and(|hit| !hit.stops(hits)) {
            return false;
        }
        let Some(parts) = &self.log else {
            return true;
        };
        let mut line = String::new();
        for part in parts {
            match part {
                LogPart::Text(text) => line.push_str(text),
                LogPart::Hole(probe) => match frame.eval(probe) {
                    Ok(value) => line.push_str(&value.shown()),
                    Err(message) => return self.failed("log message", &message),
                },
            }
        }
        line.push('\n');
        (self.say)(line);
        false
    }
}

impl Bound {
    /// Writes why the breakpoint's `what` failed, and stops.
    fn failed(&self, what: &str, message: &str) -> bool {
        (self.say)(format!(
            "The {what} of the breakpoint at {} failed, so the program stopped: {message}.\n",
            self.at
        ));
        true
    }
}

/// A value a probe gives.
#[derive(Clone, Debug)]
enum Val {
    Null,
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float(f64),
    Str(Vec<u8>),
    /// A `secret` value: shown redacted, and no operator may use it.
    Redacted,
    /// An object, an array or any other value the operators do not read.
    Other(Value),
}

impl Val {
    fn of(value: Value) -> Self {
        match value.tag() {
            None | Some(Tag::Null | Tag::Unset) => Self::Null,
            Some(Tag::Bool) => Self::Bool(value.as_bool() == Some(true)),
            Some(Tag::Int) => value.as_int().map_or(Self::Null, Self::Int),
            Some(Tag::Uint) => value.as_uint().map_or(Self::Null, Self::Uint),
            Some(Tag::Float) => value.as_float().map_or(Self::Null, Self::Float),
            Some(Tag::Str) => Self::Str(value.as_str_bytes().unwrap_or_default().to_vec()),
            Some(_) => Self::Other(value),
        }
    }

    fn read(read: Read, missing: impl FnOnce() -> String) -> Result<Self, String> {
        match read {
            Read::Value(value) => Ok(Self::of(value)),
            Read::Secret => Ok(Self::Redacted),
            Read::Missing => Err(missing()),
        }
    }

    fn type_name(&self) -> String {
        match self {
            Self::Null => "null".to_owned(),
            Self::Bool(_) => "bool".to_owned(),
            Self::Int(_) => "int".to_owned(),
            Self::Uint(_) => "uint".to_owned(),
            Self::Float(_) => "float".to_owned(),
            Self::Str(_) => "string".to_owned(),
            Self::Redacted => "secret".to_owned(),
            Self::Other(value) => super::stopped::summary(&record::node(*value)),
        }
    }

    /// The text a log message shows for the value.
    fn shown(&self) -> String {
        match self {
            Self::Null => "null".to_owned(),
            Self::Bool(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::Uint(value) => value.to_string(),
            Self::Float(value) => format!("{value:?}"),
            Self::Str(bytes) => nvs_render::Rendered::new(&String::from_utf8_lossy(bytes))
                .as_str()
                .to_owned(),
            Self::Redacted => super::stopped::summary(&Node::Redacted),
            Self::Other(value) => super::stopped::summary(&record::node(*value)),
        }
    }

    fn number(&self) -> Option<f64> {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a comparison across `int` and `float` is made in `float`, as the language makes it"
        )]
        match *self {
            Self::Int(value) => Some(value as f64),
            Self::Uint(value) => Some(value as f64),
            Self::Float(value) => Some(value),
            _ => None,
        }
    }
}

/// The stopped frame a probe reads.
struct Frame<'a> {
    slots: &'a [Value],
    secret: &'a [bool],
}

impl Frame<'_> {
    fn eval(&self, probe: &Probe) -> Result<Val, String> {
        match probe {
            Probe::Literal(literal) => Ok(match literal {
                Literal::Null => Val::Null,
                Literal::Bool(value) => Val::Bool(*value),
                Literal::Int(value) => Val::Int(*value),
                Literal::Float(value) => Val::Float(*value),
                Literal::Str(text) => Val::Str(text.as_bytes().to_vec()),
            }),
            Probe::Slot(index) => {
                if self.secret.get(*index).copied().unwrap_or(false) {
                    return Ok(Val::Redacted);
                }
                let value = self.slots.get(*index).copied().unwrap_or_else(Value::unset);
                if value.tag() == Some(Tag::Unset) {
                    return Err("a variable it reads has no value yet".to_owned());
                }
                Ok(Val::of(value))
            }
            Probe::Property {
                object,
                name,
                nullsafe,
            } => match self.eval(object)? {
                Val::Null if *nullsafe => Ok(Val::Null),
                Val::Other(value) => Val::read(record::property(value, name), || {
                    format!("the object has no property `{name}`")
                }),
                other => Err(format!(
                    "`{name}` is read from a `{}`, not an object",
                    other.type_name()
                )),
            },
            Probe::Index { base, key } => {
                let base = match self.eval(base)? {
                    Val::Other(value) => value,
                    other => return Err(format!("a `{}` has no elements", other.type_name())),
                };
                match self.eval(key)? {
                    Val::Int(index) => Val::read(record::element(base, Key::Index(index)), || {
                        format!("the array has no key {index}")
                    }),
                    Val::Str(name) => Val::read(record::element(base, Key::Name(&name)), || {
                        format!("the array has no key {:?}", String::from_utf8_lossy(&name))
                    }),
                    other => Err(format!("a `{}` is not an array key", other.type_name())),
                }
            }
            Probe::Not(operand) => match self.eval(operand)? {
                Val::Bool(value) => Ok(Val::Bool(!value)),
                other => Err(format!("`!` needs a `bool`, not a `{}`", other.type_name())),
            },
            Probe::Neg(operand) => match self.eval(operand)? {
                Val::Int(value) => value.checked_neg().map(Val::Int).ok_or_else(overflow),
                Val::Float(value) => Ok(Val::Float(-value)),
                other => Err(format!("`-` needs a number, not a `{}`", other.type_name())),
            },
            Probe::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs),
        }
    }

    fn binary(&self, op: ProbeOp, lhs: &Probe, rhs: &Probe) -> Result<Val, String> {
        let left = self.eval(lhs)?;
        match (op, &left) {
            (ProbeOp::And, Val::Bool(false)) => return Ok(Val::Bool(false)),
            (ProbeOp::Or, Val::Bool(true)) => return Ok(Val::Bool(true)),
            (ProbeOp::Coalesce, Val::Null) => return self.eval(rhs),
            (ProbeOp::Coalesce, _) => return Ok(left),
            _ => {}
        }
        let right = self.eval(rhs)?;
        if matches!(left, Val::Redacted) || matches!(right, Val::Redacted) {
            return Err("a secret value cannot be used by an operator".to_owned());
        }
        match op {
            ProbeOp::And | ProbeOp::Or => match right {
                Val::Bool(value) if matches!(left, Val::Bool(_)) => Ok(Val::Bool(value)),
                _ => Err(mismatch(&left, &right)),
            },
            ProbeOp::Eq => equal(&left, &right).map(Val::Bool),
            ProbeOp::NotEq => equal(&left, &right).map(|equal| Val::Bool(!equal)),
            ProbeOp::Lt | ProbeOp::LtEq | ProbeOp::Gt | ProbeOp::GtEq => {
                let order = order(&left, &right)?;
                Ok(Val::Bool(match op {
                    ProbeOp::Lt => order.is_lt(),
                    ProbeOp::LtEq => order.is_le(),
                    ProbeOp::Gt => order.is_gt(),
                    _ => order.is_ge(),
                }))
            }
            ProbeOp::Concat => match (left, right) {
                (Val::Str(mut left), Val::Str(right)) => {
                    left.extend_from_slice(&right);
                    Ok(Val::Str(left))
                }
                (left, right) => Err(mismatch(&left, &right)),
            },
            ProbeOp::Add | ProbeOp::Sub | ProbeOp::Mul | ProbeOp::Div | ProbeOp::Mod => {
                arithmetic(op, &left, &right)
            }
            ProbeOp::Coalesce => Ok(left),
        }
    }
}

fn overflow() -> String {
    "the arithmetic overflows".to_owned()
}

fn mismatch(left: &Val, right: &Val) -> String {
    format!(
        "`{}` and `{}` cannot be used together with this operator",
        left.type_name(),
        right.type_name()
    )
}

fn equal(left: &Val, right: &Val) -> Result<bool, String> {
    Ok(match (left, right) {
        (Val::Null, Val::Null) => true,
        (Val::Null, _) | (_, Val::Null) => false,
        (Val::Bool(a), Val::Bool(b)) => a == b,
        (Val::Int(a), Val::Int(b)) => a == b,
        (Val::Uint(a), Val::Uint(b)) => a == b,
        (Val::Int(a), Val::Uint(b)) | (Val::Uint(b), Val::Int(a)) => {
            i128::from(*a) == i128::from(*b)
        }
        (Val::Str(a), Val::Str(b)) => a == b,
        (Val::Other(a), Val::Other(b)) => match (a.obj_ptr(), b.obj_ptr()) {
            (Some(a), Some(b)) => a == b,
            _ => return Err("a condition can compare objects, but not arrays".to_owned()),
        },
        _ => match (left.number(), right.number()) {
            #[expect(
                clippy::float_cmp,
                reason = "`==` on numbers is exact, as the language makes it"
            )]
            (Some(a), Some(b)) => a == b,
            _ => return Err(mismatch(left, right)),
        },
    })
}

fn order(left: &Val, right: &Val) -> Result<std::cmp::Ordering, String> {
    match (left, right) {
        (Val::Int(a), Val::Int(b)) => Ok(a.cmp(b)),
        (Val::Uint(a), Val::Uint(b)) => Ok(a.cmp(b)),
        (Val::Int(a), Val::Uint(b)) => Ok(i128::from(*a).cmp(&i128::from(*b))),
        (Val::Uint(a), Val::Int(b)) => Ok(i128::from(*a).cmp(&i128::from(*b))),
        (Val::Str(a), Val::Str(b)) => Ok(a.cmp(b)),
        _ => match (left.number(), right.number()) {
            (Some(a), Some(b)) => a
                .partial_cmp(&b)
                .ok_or_else(|| "a comparison with `NAN` has no order".to_owned()),
            _ => Err(mismatch(left, right)),
        },
    }
}

/// `rule:types/arithmetic`'s rows for `+`, `-`, `*`, `/` and `%`.
fn arithmetic(op: ProbeOp, left: &Val, right: &Val) -> Result<Val, String> {
    let zero = || "the division is by zero".to_owned();
    match (left, right) {
        (Val::Int(a), Val::Int(b)) => {
            let (a, b) = (*a, *b);
            match op {
                ProbeOp::Add => a.checked_add(b).map(Val::Int).ok_or_else(overflow),
                ProbeOp::Sub => a.checked_sub(b).map(Val::Int).ok_or_else(overflow),
                ProbeOp::Mul => a.checked_mul(b).map(Val::Int).ok_or_else(overflow),
                ProbeOp::Mod if b == 0 => Err(zero()),
                ProbeOp::Mod => a.checked_rem(b).map(Val::Int).ok_or_else(overflow),
                _ if b == 0 => Err(zero()),
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "an inexact quotient is a `float`, as `rule:types/arithmetic` says"
                )]
                _ => match a.checked_rem(b) {
                    Some(0) => a.checked_div(b).map(Val::Int).ok_or_else(overflow),
                    Some(_) => Ok(Val::Float(a as f64 / b as f64)),
                    None => Err(overflow()),
                },
            }
        }
        (Val::Uint(a), Val::Uint(b)) => {
            let (a, b) = (*a, *b);
            match op {
                ProbeOp::Add => a.checked_add(b).map(Val::Uint).ok_or_else(overflow),
                ProbeOp::Sub => a.checked_sub(b).map(Val::Uint).ok_or_else(overflow),
                ProbeOp::Mul => a.checked_mul(b).map(Val::Uint).ok_or_else(overflow),
                _ if b == 0 => Err(zero()),
                ProbeOp::Mod => Ok(Val::Uint(a % b)),
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "an inexact quotient is a `float`, as `rule:types/arithmetic` says"
                )]
                _ if a % b != 0 => Ok(Val::Float(a as f64 / b as f64)),
                _ => Ok(Val::Uint(a / b)),
            }
        }
        (Val::Int(_), Val::Uint(_)) | (Val::Uint(_), Val::Int(_)) => Err(mismatch(left, right)),
        _ => match (left.number(), right.number()) {
            (Some(_), Some(_)) if op == ProbeOp::Mod => Err("`%` needs two integers".to_owned()),
            (Some(_), Some(b)) if op == ProbeOp::Div && b == 0.0 => Err(zero()),
            (Some(a), Some(b)) => Ok(Val::Float(match op {
                ProbeOp::Add => a + b,
                ProbeOp::Sub => a - b,
                ProbeOp::Mul => a * b,
                _ => a / b,
            })),
            _ => Err(mismatch(left, right)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_hit_condition_stops_at_the_hits_it_names() {
        let stops = |text: &str| {
            let hit = HitCondition::parse(text).expect("the hit condition parses");
            (1..=6).filter(|&hits| hit.stops(hits)).collect::<Vec<_>>()
        };
        assert_eq!(stops("3"), [3, 4, 5, 6]);
        assert_eq!(stops(">=3"), [3, 4, 5, 6]);
        assert_eq!(stops(">3"), [4, 5, 6]);
        assert_eq!(stops("==3"), [3]);
        assert_eq!(stops("%2"), [2, 4, 6]);
        assert!(HitCondition::parse("%0").is_err());
        assert!(HitCondition::parse("<3").is_err());
    }

    #[test]
    fn arithmetic_follows_the_language_rows() {
        let div = |a, b| arithmetic(ProbeOp::Div, &Val::Int(a), &Val::Int(b));
        assert!(matches!(div(6, 3), Ok(Val::Int(2))));
        assert!(matches!(div(7, 2), Ok(Val::Float(q)) if (q - 3.5).abs() < f64::EPSILON));
        assert!(div(1, 0).is_err());
        assert!(arithmetic(ProbeOp::Add, &Val::Int(i64::MAX), &Val::Int(1)).is_err());
        assert!(arithmetic(ProbeOp::Add, &Val::Int(1), &Val::Uint(1)).is_err());
        assert_eq!(equal(&Val::Int(3), &Val::Float(3.0)), Ok(true));
        assert!(equal(&Val::Int(3), &Val::Str(b"3".to_vec())).is_err());
    }
}
