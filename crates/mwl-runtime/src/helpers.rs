//! The closed set of engine-owned runtime helpers compiled code can call.
//!
//! Each one backs exactly one `mwl_ir::Helper` variant. That enum is the
//! authority on *what* each helper means — its own doc comments carry the
//! semantics, including PHP's rules for bool-to-string and for what counts as
//! falsy — so nothing is restated here; this module is the implementation, and
//! [`symbols`] is the table `mwl-codegen` registers with the JIT.
//!
//! `mwl-runtime` deliberately does not depend on `mwl-ir`, so the mapping from
//! a `Helper` tag to a symbol name lives in `mwl-codegen`, which depends on
//! both. What lives here is the name itself, and the guarantee that the name
//! resolves.
//!
//! # Argument tags are checked, not assumed
//!
//! Every helper below re-checks its arguments' tags and returns [`FATAL`] on a
//! mismatch rather than reading the payload anyway. A mismatch cannot happen
//! in a well-typed program — [ADR 0007](../../../docs/adr/0007-explicit-type-system.md)
//! settles every operand type before lowering, and `mwl_ir::lower` picks the
//! helper from that type — so the check is not defending against user code. It
//! is defending against a *miscompile*: reading a `Value`'s payload under the
//! wrong tag would be a memory-safety bug (an `int` payload read as a string
//! pointer), and ADR 0002 § *Consequences* already names a silently wrong call
//! site as the nastier failure mode of the checked-return design. A predicted
//! branch on a tag byte is a cheap price for making that class of bug a
//! `FATAL` with a message instead.
//!
//! Every `mwl_ir::Helper` variant now has an entry point here.

use crate::abi::{Fault, HelperFn};
use crate::decimal::Decimal;
use crate::fmt::php_float_to_string;
use crate::string::MwlStr;
use crate::value::{Tag, Value};

/// The `FATAL` a tag mismatch produces — see this module's docs for why it is
/// checked at all.
fn wrong_tag(helper: &'static str, expected: Tag, actual: Value) -> Fault {
    Fault::fatal(format!(
        "internal error: {helper} expected a {expected:?} argument, got tag {}",
        actual.tag_byte()
    ))
}

macro_rules! expect_tag {
    ($helper:literal, $value:expr, $accessor:ident, $tag:expr) => {
        $value
            .$accessor()
            .ok_or_else(|| wrong_tag($helper, $tag, $value))?
    };
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::IntToString`.
    fn mwl_int_to_string(_ctx, args: [1]) {
        let value = expect_tag!("mwl_int_to_string", args[0], as_int, Tag::Int);
        Ok(Value::str(MwlStr::new(value.to_string().as_bytes())))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::UintToString`.
    fn mwl_uint_to_string(_ctx, args: [1]) {
        let value = expect_tag!("mwl_uint_to_string", args[0], as_uint, Tag::Uint);
        Ok(Value::str(MwlStr::new(value.to_string().as_bytes())))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::FloatToString`.
    fn mwl_float_to_string(_ctx, args: [1]) {
        let value = expect_tag!("mwl_float_to_string", args[0], as_float, Tag::Float);
        Ok(Value::str(MwlStr::new(php_float_to_string(value).as_bytes())))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::BoolToString`.
    fn mwl_bool_to_string(_ctx, args: [1]) {
        let value = expect_tag!("mwl_bool_to_string", args[0], as_bool, Tag::Bool);
        Ok(Value::str(MwlStr::new(if value { b"1".as_slice() } else { b"".as_slice() })))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::IntTruthy`.
    fn mwl_int_truthy(_ctx, args: [1]) {
        let value = expect_tag!("mwl_int_truthy", args[0], as_int, Tag::Int);
        Ok(Value::bool(value != 0))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::UintTruthy`.
    fn mwl_uint_truthy(_ctx, args: [1]) {
        let value = expect_tag!("mwl_uint_truthy", args[0], as_uint, Tag::Uint);
        Ok(Value::bool(value != 0))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::FloatTruthy`. `-0.0` is falsy with `0.0`; `NAN` is
    /// truthy, which `!=` gives for free.
    fn mwl_float_truthy(_ctx, args: [1]) {
        let value = expect_tag!("mwl_float_truthy", args[0], as_float, Tag::Float);
        Ok(Value::bool(value != 0.0))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::StrTruthy`.
    fn mwl_str_truthy(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("mwl_str_truthy", Tag::Str, args[0]))?;
        Ok(Value::bool(!(bytes.is_empty() || bytes == b"0")))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::ArrayTruthy` — falsy iff the array holds no entries,
    /// for any element type. [`mwl_str_truthy`]'s structure with a different
    /// emptiness test, as this module's own docs predicted it would be.
    fn mwl_array_truthy(_ctx, args: [1]) {
        let array = args[0]
            .array_ptr()
            .ok_or_else(|| wrong_tag("mwl_array_truthy", Tag::Array, args[0]))?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live                       allocation, so it is live for this read"
        )]
        let count = unsafe { crate::array::mwl_array_count(array) };
        Ok(Value::bool(count != 0))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::Identical` — `==` where at least one operand is a
    /// `mixed` or a union, which is
    /// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
    /// § 5's case and the only one whose row is a runtime tag rather than a
    /// static type. The row itself is [`crate::value_identical`], so a tagged
    /// operand and a statically typed one answer alike; a pair whose tags name
    /// different rows is `false` there, never a throw, which is why this helper
    /// is infallible and carries no error edge. `!=` is this helper under an
    /// `mwl_ir::UnOp::Not`, the arrangement [`mwl_decimal_eq`] already uses.
    fn mwl_value_identical(_ctx, args: [2]) {
        Ok(Value::bool(crate::value_identical(args[0], args[1])))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::NumericEq` — `==` over two operands whose
    /// representations differ but whose types are
    /// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
    /// § 2's one numeric domain. The row is [`crate::numeric_identical`],
    /// which is total, so this carries no error edge; `!=` is this helper
    /// under an `mwl_ir::UnOp::Not`, the arrangement [`mwl_decimal_eq`]
    /// already uses.
    fn mwl_numeric_eq(_ctx, args: [2]) {
        Ok(Value::bool(crate::numeric_identical(args[0], args[1])))
    }
}

/// The [`Fault::Thrown`] a checked conversion produces when the value does not
/// fit — ADR 0007 § 2's "`as` ... either produces a value of the target type or
/// throws. It never rounds, truncates, or substitutes a default."
///
/// Known gap: the message is all a helper failure can carry, so the driver
/// promotes every one of these to spec § 10's `RuntimeError`. ADR 0007 § 4
/// names `ArithmeticError` for a numeric overflow, which is the closer class —
/// reaching it needs a helper failure to name its own class, which nothing in
/// `crate::abi` expresses yet.
fn does_not_fit(what: &str, target: &str) -> Fault {
    Fault::thrown(format!("cannot convert {what} to `{target}`"))
}

/// ADR 0007 § 2's checked conversion rows, one function each, answering `None`
/// exactly where the row fails.
///
/// Every row has two entry points and never a third: the throwing helper below
/// it, which turns a `None` into [`does_not_fit`], and [`to_int`]/[`to_uint`]/
/// [`to_float`], which turn the same `None` into `null` for
/// [ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md)'s
/// `expr as ?T`. That ADR's "one implementation now exists because there is one
/// operation" is what this split makes true rather than promised — the throwing
/// and the nullable form cannot drift apart, because there is one row.
mod row {
    /// The magnitude past which an `f64` no longer represents every integer —
    /// ADR 0007 § 2's 2^53 boundary, shared by both integer-to-`float` rows.
    const F64_EXACT_INT_LIMIT: u64 = 1 << 53;

    pub(super) fn int_to_uint(value: i64) -> Option<u64> {
        u64::try_from(value).ok()
    }

    pub(super) fn uint_to_int(value: u64) -> Option<i64> {
        i64::try_from(value).ok()
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "the magnitude check is exactly what makes this exact"
    )]
    pub(super) fn int_to_float(value: i64) -> Option<f64> {
        (value.unsigned_abs() <= F64_EXACT_INT_LIMIT).then_some(value as f64)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "the magnitude check is exactly what makes this exact"
    )]
    pub(super) fn uint_to_float(value: u64) -> Option<f64> {
        (value <= F64_EXACT_INT_LIMIT).then_some(value as f64)
    }

    pub(super) fn float_to_int(value: f64) -> Option<i64> {
        exact_integral(value).and_then(|v| i64::try_from(v).ok())
    }

    pub(super) fn float_to_uint(value: f64) -> Option<u64> {
        exact_integral(value).and_then(|v| u64::try_from(v).ok())
    }

    pub(super) fn str_to_int(text: &str) -> Option<i64> {
        text.parse().ok()
    }

    pub(super) fn str_to_uint(text: &str) -> Option<u64> {
        text.parse().ok()
    }

    /// `f64::from_str` accepts `inf`/`nan`/`infinity` in any case; none is an
    /// "exact numeric literal", so each is refused here rather than becoming a
    /// value no source literal could have written.
    pub(super) fn str_to_float(text: &str) -> Option<f64> {
        text.parse::<f64>().ok().filter(|value| value.is_finite())
    }

    /// `value` as an exact integer, or `None` if it is not integral, is not
    /// finite, or is too large for the `i128` both integer targets fit inside.
    ///
    /// ADR 0007 § 2: "integral and in range, or throws. Rounding is
    /// `floor`/`ceil`/`round`, said out loud" — so `1.5` is refused here rather
    /// than silently becoming any of `1`, `2`, or `1`.
    fn exact_integral(value: f64) -> Option<i128> {
        if !value.is_finite() || value.fract() != 0.0 {
            return None;
        }
        // Every `f64` with a zero fractional part and a magnitude below 2^127
        // is exactly an integer, and `i128` holds all of them.
        if value.abs() >= 2.0_f64.powi(127) {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the two guards above leave only values `i128` represents exactly"
        )]
        Some(value as i128)
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::IntToUint`.
    fn mwl_int_to_uint(_ctx, args: [1]) {
        let value = expect_tag!("mwl_int_to_uint", args[0], as_int, Tag::Int);
        row::int_to_uint(value)
            .map(Value::uint)
            .ok_or_else(|| does_not_fit(&format!("`int` {value}"), "uint"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::UintToInt`.
    fn mwl_uint_to_int(_ctx, args: [1]) {
        let value = expect_tag!("mwl_uint_to_int", args[0], as_uint, Tag::Uint);
        row::uint_to_int(value)
            .map(Value::int)
            .ok_or_else(|| does_not_fit(&format!("`uint` {value}"), "int"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::IntToFloat`.
    fn mwl_int_to_float(_ctx, args: [1]) {
        let value = expect_tag!("mwl_int_to_float", args[0], as_int, Tag::Int);
        row::int_to_float(value)
            .map(Value::float)
            .ok_or_else(|| does_not_fit(&format!("`int` {value}"), "float"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::UintToFloat`.
    fn mwl_uint_to_float(_ctx, args: [1]) {
        let value = expect_tag!("mwl_uint_to_float", args[0], as_uint, Tag::Uint);
        row::uint_to_float(value)
            .map(Value::float)
            .ok_or_else(|| does_not_fit(&format!("`uint` {value}"), "float"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::FloatToInt`.
    fn mwl_float_to_int(_ctx, args: [1]) {
        let value = expect_tag!("mwl_float_to_int", args[0], as_float, Tag::Float);
        row::float_to_int(value)
            .map(Value::int)
            .ok_or_else(|| does_not_fit(&format!("`float` {value}"), "int"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::FloatToUint`.
    fn mwl_float_to_uint(_ctx, args: [1]) {
        let value = expect_tag!("mwl_float_to_uint", args[0], as_float, Tag::Float);
        row::float_to_uint(value)
            .map(Value::uint)
            .ok_or_else(|| does_not_fit(&format!("`float` {value}"), "uint"))
    }
}

/// The one shape ADR 0007 § 2's `string` → number row accepts: the *whole*
/// string, with no surrounding whitespace, no leading `+`-and-garbage rule, and
/// no PHP-style prefix parse. Returned as `&str` so each caller can hand it to
/// the standard library's own exact parser.
fn numeric_text<'a>(bytes: &'a [u8], helper: &'static str, value: Value) -> Result<&'a str, Fault> {
    str::from_utf8(bytes).map_err(|_| wrong_tag(helper, Tag::Str, value))
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::StrToInt`.
    fn mwl_str_to_int(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("mwl_str_to_int", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "mwl_str_to_int", args[0])?;
        row::str_to_int(text)
            .map(Value::int)
            .ok_or_else(|| does_not_fit(&format!("string {text:?}"), "int"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::StrToUint`.
    fn mwl_str_to_uint(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("mwl_str_to_uint", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "mwl_str_to_uint", args[0])?;
        row::str_to_uint(text)
            .map(Value::uint)
            .ok_or_else(|| does_not_fit(&format!("string {text:?}"), "uint"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::StrToFloat`.
    fn mwl_str_to_float(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("mwl_str_to_float", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "mwl_str_to_float", args[0])?;
        row::str_to_float(text)
            .map(Value::float)
            .ok_or_else(|| does_not_fit(&format!("string {text:?}"), "float"))
    }
}

/// The operand's bytes as text, for the two `string` rows — `None` where a
/// `Tag::Str` payload is not UTF-8, which is a failed conversion for
/// ADR 0066's form rather than the miscompile [`numeric_text`] reports.
fn str_operand(value: &Value) -> Option<&str> {
    str::from_utf8(value.as_str_bytes()?).ok()
}

/// [ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md) § 1's
/// `expr as ?int`: the value `as int` would produce, or `null` where it would
/// throw.
///
/// Dispatches on the operand's **tag**, which is why one function covers every
/// source. That is not a shortcut: it is what makes § 2's "a `null` operand
/// yields `null`" and § 3's "from `mixed` every target has a checked path" the
/// same code as `"42" as ?int`, with no branch in lowering and no second
/// implementation of any row (see [`row`]). A tag ADR 0007 § 2 defines no row
/// from — an array, an object, a `bool` — is a conversion that does not exist,
/// which § 3 makes a compile error for a statically-known operand and `null`
/// for a `mixed` one; this is the `mixed` answer.
fn to_int(value: Value) -> Value {
    let converted = match value.tag() {
        Some(Tag::Int) => value.as_int(),
        Some(Tag::Uint) => value.as_uint().and_then(row::uint_to_int),
        Some(Tag::Float) => value.as_float().and_then(row::float_to_int),
        Some(Tag::Str) => str_operand(&value).and_then(row::str_to_int),
        _ => None,
    };
    converted.map_or_else(Value::null, Value::int)
}

/// [`to_int`]'s row set, unsigned — ADR 0066 § 1's `expr as ?uint`.
fn to_uint(value: Value) -> Value {
    let converted = match value.tag() {
        Some(Tag::Uint) => value.as_uint(),
        Some(Tag::Int) => value.as_int().and_then(row::int_to_uint),
        Some(Tag::Float) => value.as_float().and_then(row::float_to_uint),
        Some(Tag::Str) => str_operand(&value).and_then(row::str_to_uint),
        _ => None,
    };
    converted.map_or_else(Value::null, Value::uint)
}

/// [`to_int`]'s row set, landing on `float` — ADR 0066 § 1's `expr as ?float`.
fn to_float(value: Value) -> Value {
    let converted = match value.tag() {
        Some(Tag::Float) => value.as_float(),
        Some(Tag::Int) => value.as_int().and_then(row::int_to_float),
        Some(Tag::Uint) => value.as_uint().and_then(row::uint_to_float),
        Some(Tag::Str) => str_operand(&value).and_then(row::str_to_float),
        _ => None,
    };
    converted.map_or_else(Value::null, Value::float)
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::ToIntOrNull` — see [`to_int`].
    fn mwl_to_int_or_null(_ctx, args: [1]) {
        Ok(to_int(args[0]))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::ToUintOrNull` — see [`to_uint`].
    fn mwl_to_uint_or_null(_ctx, args: [1]) {
        Ok(to_uint(args[0]))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::ToFloatOrNull` — see [`to_float`].
    fn mwl_to_float_or_null(_ctx, args: [1]) {
        Ok(to_float(args[0]))
    }
}

/// [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 2's
/// scalar-to-`string` rows, applied to a value whose representation is
/// `mwl_ir::ty::Ty::Tagged` — a `mixed`, a `?T`, or any other union.
///
/// The counterpart of [`value_truthy`], and for the same reason: compiled code
/// that *knows* its operand is an `int` reaches `mwl_int_to_string` with no tag
/// test at all, so this is only the case where the static type genuinely does
/// not say which row applies. It dispatches on the tag the `Value` already
/// carries, which is what makes `"a" . $mixed` one helper rather than one
/// lowering branch per possible source — the same "one tag per target, not one
/// per (source, target) pair" rule [`to_int`] already follows.
///
/// **Three tags convert to nothing, and each throws** rather than producing
/// PHP's `"Array"`-plus-warning: [ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
/// R4 makes failure a throw, and a silent placeholder is exactly the class of
/// answer ADR 0007 § 2 removed from the language. An **object** is among them
/// today for a narrower reason — ADR 0028 makes `Stringable` the one way an
/// object renders, and that interface still carries no member signature for a
/// dispatch to reach, so an object arriving here is a program the checker let
/// through on a union it could not narrow.
///
/// A `Tag::Str` operand is returned as itself with one **fresh** reference, so
/// the caller owns the result exactly as it owns a converted one; every other
/// row builds a new string, which carries its one reference already.
pub fn value_to_string(value: Value) -> Result<Value, Fault> {
    let refused = |what: &str| Fault::thrown(format!("cannot convert {what} to `string`"));
    match value.tag() {
        Some(Tag::Null) | None => Ok(Value::str(MwlStr::new(b""))),
        Some(Tag::Bool) => {
            let set = value.as_bool() == Some(true);
            Ok(Value::str(MwlStr::new(if set {
                b"1".as_slice()
            } else {
                b"".as_slice()
            })))
        }
        Some(Tag::Int) => {
            let n = value.as_int().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(MwlStr::new(n.to_string().as_bytes())))
        }
        Some(Tag::Uint) => {
            let n = value.as_uint().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(MwlStr::new(n.to_string().as_bytes())))
        }
        Some(Tag::Float) => {
            let n = value.as_float().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(MwlStr::new(php_float_to_string(n).as_bytes())))
        }
        Some(Tag::Str) => {
            let ptr = value.str_ptr().ok_or_else(|| refused("this value"))?;
            #[expect(
                unsafe_code,
                reason = "a Tag::Str value's payload is a live allocation the \
                          caller owns a reference to, and the result carries a \
                          second one the caller will release"
            )]
            unsafe {
                crate::string::mwl_str_retain(ptr);
            }
            Ok(value)
        }
        // ADR 0054 § 4's `decimal → string` row: total, and scale-preserving,
        // so `19.90` renders as `"19.90"` — `crate::decimal`'s `Display` is
        // the one implementation of it.
        Some(Tag::Decimal) => {
            let value = value.as_decimal().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(MwlStr::new(value.to_string().as_bytes())))
        }
        Some(Tag::Array) => Err(refused("an array")),
        Some(Tag::Object) => Err(refused("an object")),
        Some(Tag::Closure) => Err(refused("a closure")),
        Some(Tag::Resource) => Err(refused("a resource")),
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::TaggedToString` — see [`value_to_string`].
    fn mwl_tagged_to_string(_ctx, args: [1]) {
        value_to_string(args[0])
    }
}

/// [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md) § 3's
/// `ArithmeticError`: an overflow of either kind, or a zero divisor.
///
/// Known gap, shared with [`does_not_fit`]: a helper failure carries only a
/// message, so the driver promotes this to spec § 10's `RuntimeError` rather
/// than the `ArithmeticError` the ADR names.
fn arithmetic_error(operation: &str) -> Fault {
    Fault::thrown(format!(
        "`decimal` {operation} is outside the type's range (ADR 0054 § 1: a 96-bit \
         mantissa at a scale of 0 to 28), or divides by zero"
    ))
}

/// One arithmetic or comparison operand as a [`Decimal`].
///
/// An `int` or a `uint` operand is promoted here rather than by a conversion
/// instruction in lowering, which is what makes ADR 0054 § 3's
/// `decimal ⊕ int` row cost nothing extra: both are exact in 96 bits, and the
/// promotion is a tag test the helper already performs. A `float` operand
/// **never reaches an arithmetic helper** — § 3 makes `decimal ⊕ float` a
/// compile error — so it is a miscompile here, reported as one; the
/// comparison helpers take the other path ([`decimal_ordering`]) and accept
/// one, because § 3 permits comparison precisely where it forbids arithmetic.
fn decimal_operand(helper: &'static str, value: Value) -> Result<Decimal, Fault> {
    match value.tag() {
        Some(Tag::Decimal) => value
            .as_decimal()
            .ok_or_else(|| wrong_tag(helper, Tag::Decimal, value)),
        Some(Tag::Int) => value
            .as_int()
            .map(Decimal::from_i64)
            .ok_or_else(|| wrong_tag(helper, Tag::Decimal, value)),
        Some(Tag::Uint) => value
            .as_uint()
            .map(Decimal::from_u64)
            .ok_or_else(|| wrong_tag(helper, Tag::Decimal, value)),
        _ => Err(wrong_tag(helper, Tag::Decimal, value)),
    }
}

/// Both operands of one binary `decimal` operator.
fn decimal_pair(helper: &'static str, args: &[Value]) -> Result<(Decimal, Decimal), Fault> {
    Ok((
        decimal_operand(helper, args[0])?,
        decimal_operand(helper, args[1])?,
    ))
}

macro_rules! decimal_arithmetic {
    ($(#[$meta:meta])* fn $name:ident = $method:ident, $operation:literal) => {
        crate::mwl_helper! {
            $(#[$meta])*
            fn $name(_ctx, args: [2]) {
                let (lhs, rhs) = decimal_pair(stringify!($name), args)?;
                lhs.$method(rhs)
                    .map(Value::decimal)
                    .ok_or_else(|| arithmetic_error($operation))
            }
        }
    };
}

decimal_arithmetic! {
    /// `mwl_ir::Helper::DecimalAdd`.
    fn mwl_decimal_add = checked_add, "addition"
}
decimal_arithmetic! {
    /// `mwl_ir::Helper::DecimalSub`.
    fn mwl_decimal_sub = checked_sub, "subtraction"
}
decimal_arithmetic! {
    /// `mwl_ir::Helper::DecimalMul`.
    fn mwl_decimal_mul = checked_mul, "multiplication"
}
decimal_arithmetic! {
    /// `mwl_ir::Helper::DecimalDiv`.
    fn mwl_decimal_div = checked_div, "division"
}
decimal_arithmetic! {
    /// `mwl_ir::Helper::DecimalMod`.
    fn mwl_decimal_mod = checked_rem, "remainder"
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalNeg` — `-$d`, which never fails: the mantissa
    /// is unsigned, so there is no asymmetric minimum to overflow the way
    /// `-i64::MIN` does.
    fn mwl_decimal_neg(_ctx, args: [1]) {
        Ok(Value::decimal(decimal_operand("mwl_decimal_neg", args[0])?.negated()))
    }
}

/// ADR 0054 § 3's comparison row: a `decimal` against a `decimal`, an `int`, a
/// `uint` **or a `float`**, "mathematically exact over the full range of
/// both". `None` is the unordered answer a `NaN` operand gives, which is what
/// makes every comparison against one false and `!=` true.
fn decimal_ordering(left: Value, right: Value) -> Option<std::cmp::Ordering> {
    match (left.tag(), right.tag()) {
        (Some(Tag::Float), _) => Some(right.as_decimal()?.compare_f64(left.as_float()?)?.reverse()),
        (_, Some(Tag::Float)) => left.as_decimal()?.compare_f64(right.as_float()?),
        _ => {
            let decimal = |value: Value| match value.tag() {
                Some(Tag::Decimal) => value.as_decimal(),
                Some(Tag::Int) => value.as_int().map(Decimal::from_i64),
                Some(Tag::Uint) => value.as_uint().map(Decimal::from_u64),
                _ => None,
            };
            Some(decimal(left)?.compare(decimal(right)?))
        }
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalEq` — see [`decimal_ordering`]. `!=` is this
    /// helper under an `mwl_ir::UnOp::Not`, which is also what gives a `NaN`
    /// operand PHP's answer to every one of the six.
    fn mwl_decimal_eq(_ctx, args: [2]) {
        Ok(Value::bool(decimal_ordering(args[0], args[1]).is_some_and(std::cmp::Ordering::is_eq)))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalLt` — `>` is this helper with its operands
    /// swapped.
    fn mwl_decimal_lt(_ctx, args: [2]) {
        Ok(Value::bool(decimal_ordering(args[0], args[1]).is_some_and(std::cmp::Ordering::is_lt)))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalLtEq` — `>=` is this helper with its operands
    /// swapped.
    fn mwl_decimal_lt_eq(_ctx, args: [2]) {
        Ok(Value::bool(decimal_ordering(args[0], args[1]).is_some_and(std::cmp::Ordering::is_le)))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalTruthy` — ADR 0035's numeric row: falsy iff
    /// zero, at any scale.
    fn mwl_decimal_truthy(_ctx, args: [1]) {
        Ok(Value::bool(!decimal_operand("mwl_decimal_truthy", args[0])?.is_zero()))
    }
}

/// ADR 0054 § 4's four `→ decimal` rows, chosen by the operand's **runtime**
/// tag rather than by a static type — the same "one tag per target, not one
/// per (source, target) pair" arrangement [`to_int`] and [`value_to_string`]
/// already follow, which is what makes `$mixed as decimal` the same code as
/// `"19.99" as decimal` with no branch in lowering.
///
/// `None` is a row that fails (a string that is not an exact literal, a value
/// wider than 96 bits) *or* one that does not exist (an array, an object, a
/// `bool`).
fn to_decimal(value: Value) -> Option<Decimal> {
    match value.tag() {
        Some(Tag::Decimal) => value.as_decimal(),
        Some(Tag::Int) => value.as_int().map(Decimal::from_i64),
        Some(Tag::Uint) => value.as_uint().map(Decimal::from_u64),
        Some(Tag::Float) => value.as_float().and_then(Decimal::from_f64),
        Some(Tag::Str) => str_operand(&value).and_then(Decimal::parse),
        _ => None,
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::ToDecimal` — see [`to_decimal`].
    fn mwl_to_decimal(_ctx, args: [1]) {
        to_decimal(args[0])
            .map(Value::decimal)
            .ok_or_else(|| does_not_fit("this value", "decimal"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::ToDecimalOrNull` — ADR 0066 § 1's non-throwing form
    /// of [`to_decimal`], sharing its one implementation of every row exactly
    /// as `mwl_to_int_or_null` shares [`row`]'s.
    fn mwl_to_decimal_or_null(_ctx, args: [1]) {
        Ok(to_decimal(args[0]).map_or_else(Value::null, Value::decimal))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalToInt` — ADR 0054 § 4: integral and in range,
    /// or throws. Rounding is `Core\Decimal::floor`/`ceil`/`round`, said out
    /// loud, exactly as `float → int` already is.
    fn mwl_decimal_to_int(_ctx, args: [1]) {
        let value = decimal_operand("mwl_decimal_to_int", args[0])?;
        value.to_i64()
            .map(Value::int)
            .ok_or_else(|| does_not_fit(&format!("`decimal` {value}"), "int"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalToUint` — [`mwl_decimal_to_int`]'s row,
    /// unsigned.
    fn mwl_decimal_to_uint(_ctx, args: [1]) {
        let value = decimal_operand("mwl_decimal_to_uint", args[0])?;
        value.to_u64()
            .map(Value::uint)
            .ok_or_else(|| does_not_fit(&format!("`decimal` {value}"), "uint"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalToFloat` — the nearest `f64`, lossy and total.
    fn mwl_decimal_to_float(_ctx, args: [1]) {
        Ok(Value::float(decimal_operand("mwl_decimal_to_float", args[0])?.to_f64()))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::DecimalToString` — total, and scale-preserving.
    fn mwl_decimal_to_string(_ctx, args: [1]) {
        let value = decimal_operand("mwl_decimal_to_string", args[0])?;
        Ok(Value::str(MwlStr::new(value.to_string().as_bytes())))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::EchoStr` — raw bytes to the request's own output, with
    /// no escaping. `docs/agent/loop-goal.md` records that decision and why
    /// [ADR 0024](../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
    /// § 5's auto-escaping sink is the HTTP response write rather than this
    /// one.
    fn mwl_echo_str(ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("mwl_echo_str", Tag::Str, args[0]))?;
        ctx.write_output(bytes)
            .map_err(|error| Fault::fatal(format!("could not write output: {error}")))?;
        Ok(Value::null())
    }
}

/// [ADR 0035](../../../docs/adr/0035-truthy-boolean-context.md)'s truthy
/// table, applied to a value whose type is known only at runtime.
///
/// The per-type helpers above are what *compiled* code reaches: the checker
/// already knows an `if`'s operand type, so the branch is picked at compile
/// time and there is no tag test on the hot path. This is the other case —
/// native `Core` code holding a [`Value`] a closure just returned, whose
/// static type is `callable`'s opaque result and therefore nothing. It is
/// deliberately *not* the general `mixed` dispatch `mwl_ir::ty::Ty::Tagged`
/// still defers: this reads a tag a `Value` already carries rather than
/// deciding how a `mixed` binding represents one.
///
/// A `Tag::Object` value is always truthy, which includes an exception and a
/// closure alike (ADR 0035 § 4); a tag byte denoting nothing at all is falsy,
/// the same "report what can be be sure of" floor every other decoder here
/// takes.
#[must_use]
pub fn value_truthy(value: Value) -> bool {
    match value.tag() {
        None | Some(Tag::Null) => false,
        Some(Tag::Bool) => value.as_bool() == Some(true),
        Some(Tag::Int) => value.as_int().is_some_and(|n| n != 0),
        Some(Tag::Uint) => value.as_uint().is_some_and(|n| n != 0),
        Some(Tag::Float) => value.as_float().is_some_and(|n| n != 0.0),
        // ADR 0035's numeric row, at `decimal`'s own precision: falsy iff the
        // value is zero, at any scale.
        Some(Tag::Decimal) => value.as_decimal().is_some_and(|d| !d.is_zero()),
        Some(Tag::Str) => value
            .as_str_bytes()
            .is_some_and(|bytes| !(bytes.is_empty() || bytes == b"0")),
        Some(Tag::Array) => value.array_ptr().is_some_and(|array| {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array value's payload is a live allocation \
                          the caller owns a reference to"
            )]
            let count = unsafe { crate::array::mwl_array_count(array) };
            count != 0
        }),
        Some(Tag::Object | Tag::Closure | Tag::Resource) => true,
    }
}

/// Every helper this crate exports, paired with the symbol name compiled code
/// calls it by.
///
/// `mwl-codegen` registers these with `cranelift_jit::JITBuilder::symbol` and
/// maps each `mwl_ir::Helper` variant to one of the names. Returned as a
/// `Vec` of `(name, address)` rather than a `const` table because a function
/// address is not a value a `const` can hold.
#[must_use]
pub fn symbols() -> Vec<(&'static str, *const u8)> {
    fn address(function: HelperFn) -> *const u8 {
        (function as *const ()).cast::<u8>()
    }

    vec![
        ("mwl_int_to_string", address(mwl_int_to_string)),
        ("mwl_uint_to_string", address(mwl_uint_to_string)),
        ("mwl_float_to_string", address(mwl_float_to_string)),
        ("mwl_bool_to_string", address(mwl_bool_to_string)),
        ("mwl_int_truthy", address(mwl_int_truthy)),
        ("mwl_uint_truthy", address(mwl_uint_truthy)),
        ("mwl_float_truthy", address(mwl_float_truthy)),
        ("mwl_str_truthy", address(mwl_str_truthy)),
        ("mwl_array_truthy", address(mwl_array_truthy)),
        ("mwl_value_identical", address(mwl_value_identical)),
        ("mwl_numeric_eq", address(mwl_numeric_eq)),
        ("mwl_int_to_uint", address(mwl_int_to_uint)),
        ("mwl_uint_to_int", address(mwl_uint_to_int)),
        ("mwl_int_to_float", address(mwl_int_to_float)),
        ("mwl_uint_to_float", address(mwl_uint_to_float)),
        ("mwl_float_to_int", address(mwl_float_to_int)),
        ("mwl_float_to_uint", address(mwl_float_to_uint)),
        ("mwl_str_to_int", address(mwl_str_to_int)),
        ("mwl_str_to_uint", address(mwl_str_to_uint)),
        ("mwl_str_to_float", address(mwl_str_to_float)),
        ("mwl_to_int_or_null", address(mwl_to_int_or_null)),
        ("mwl_to_uint_or_null", address(mwl_to_uint_or_null)),
        ("mwl_to_float_or_null", address(mwl_to_float_or_null)),
        ("mwl_tagged_to_string", address(mwl_tagged_to_string)),
        ("mwl_decimal_add", address(mwl_decimal_add)),
        ("mwl_decimal_sub", address(mwl_decimal_sub)),
        ("mwl_decimal_mul", address(mwl_decimal_mul)),
        ("mwl_decimal_div", address(mwl_decimal_div)),
        ("mwl_decimal_mod", address(mwl_decimal_mod)),
        ("mwl_decimal_neg", address(mwl_decimal_neg)),
        ("mwl_decimal_eq", address(mwl_decimal_eq)),
        ("mwl_decimal_lt", address(mwl_decimal_lt)),
        ("mwl_decimal_lt_eq", address(mwl_decimal_lt_eq)),
        ("mwl_decimal_truthy", address(mwl_decimal_truthy)),
        ("mwl_to_decimal", address(mwl_to_decimal)),
        ("mwl_to_decimal_or_null", address(mwl_to_decimal_or_null)),
        ("mwl_decimal_to_int", address(mwl_decimal_to_int)),
        ("mwl_decimal_to_uint", address(mwl_decimal_to_uint)),
        ("mwl_decimal_to_float", address(mwl_decimal_to_float)),
        ("mwl_decimal_to_string", address(mwl_decimal_to_string)),
        ("mwl_echo_str", address(mwl_echo_str)),
        (
            "mwl_str_new",
            (crate::string::mwl_str_new as *const ()).cast::<u8>(),
        ),
        (
            "mwl_str_concat",
            (crate::string::mwl_str_concat as *const ()).cast::<u8>(),
        ),
        (
            "mwl_str_eq",
            (crate::string::mwl_str_eq as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_eq",
            (crate::identity::mwl_array_eq as *const ()).cast::<u8>(),
        ),
        (
            "mwl_str_retain",
            (crate::string::mwl_str_retain as *const ()).cast::<u8>(),
        ),
        (
            "mwl_value_retain",
            (crate::value::mwl_value_retain as *const ()).cast::<u8>(),
        ),
        (
            "mwl_value_release",
            (crate::value::mwl_value_release as *const ()).cast::<u8>(),
        ),
        (
            "mwl_str_release",
            (crate::string::mwl_str_release as *const ()).cast::<u8>(),
        ),
        (
            "mwl_raise",
            (crate::throwable::mwl_raise as *const ()).cast::<u8>(),
        ),
        (
            "mwl_raise_new",
            (crate::throwable::mwl_raise_new as *const ()).cast::<u8>(),
        ),
        (
            "mwl_trace_push",
            (crate::throwable::mwl_trace_push as *const ()).cast::<u8>(),
        ),
        (
            "mwl_take_thrown",
            (crate::throwable::mwl_take_thrown as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_new",
            (crate::object::mwl_object_new as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_clone",
            (crate::object::mwl_object_clone as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_retain",
            (crate::object::mwl_object_retain as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_release",
            (crate::object::mwl_object_release as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_instanceof",
            (crate::object::mwl_object_instanceof as *const ()).cast::<u8>(),
        ),
        (
            "mwl_abstract_method",
            (crate::object::mwl_abstract_method as *const ()).cast::<u8>(),
        ),
        (
            "mwl_class_method",
            (crate::object::mwl_class_method as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_class_name",
            (crate::object::mwl_object_class_name as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_field_get",
            (crate::object::mwl_object_field_get as *const ()).cast::<u8>(),
        ),
        (
            "mwl_object_field_set",
            (crate::object::mwl_object_field_set as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_new",
            (crate::array::mwl_array_new as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_retain",
            (crate::array::mwl_array_retain as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_release",
            (crate::array::mwl_array_release as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_get",
            (crate::array::mwl_array_get as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_has_key",
            (crate::array::mwl_array_has_key as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_set",
            (crate::array::mwl_array_set as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_append",
            (crate::array::mwl_array_append as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_unset",
            (crate::array::mwl_array_unset as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_count",
            (crate::array::mwl_array_count as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_next_slot",
            (crate::array::mwl_array_next_slot as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_key_at",
            (crate::array::mwl_array_key_at as *const ()).cast::<u8>(),
        ),
        (
            "mwl_array_value_at",
            (crate::array::mwl_array_value_at as *const ()).cast::<u8>(),
        ),
        (
            "mwl_safepoint",
            (crate::ctx::mwl_safepoint as *const ()).cast::<u8>(),
        ),
        (
            "mwl_probe_stmt",
            (crate::ctx::mwl_probe_stmt as *const ()).cast::<u8>(),
        ),
        (
            "mwl_probe_call_enter",
            (crate::ctx::mwl_probe_call_enter as *const ()).cast::<u8>(),
        ),
        (
            "mwl_probe_call_exit",
            (crate::ctx::mwl_probe_call_exit as *const ()).cast::<u8>(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{FATAL, call};
    use crate::ctx::Ctx;

    /// Calls a helper and returns the string it produced, releasing it.
    fn string_result(helper: HelperFn, argument: Value) -> String {
        let mut ctx = Ctx::buffered();
        let value = call(helper, &mut ctx, &[argument]).expect("the helper succeeded");
        let text = String::from_utf8(
            value
                .as_str_bytes()
                .expect("the helper produced a string")
                .to_vec(),
        )
        .expect("the helper produced UTF-8");
        #[expect(
            unsafe_code,
            reason = "the returned value owns the one reference the helper's \
                      fresh `MwlStr` was built with"
        )]
        unsafe {
            value.release();
        }
        text
    }

    /// Calls a checked conversion helper, expecting it to succeed.
    fn converted(helper: HelperFn, argument: Value) -> Value {
        let mut ctx = Ctx::buffered();
        call(helper, &mut ctx, &[argument]).expect("the conversion succeeded")
    }

    /// Calls a checked conversion helper, expecting ADR 0007 § 2's throw.
    fn refused(helper: HelperFn, argument: Value) {
        let mut ctx = Ctx::buffered();
        assert_eq!(
            call(helper, &mut ctx, &[argument]).unwrap_err(),
            crate::abi::THROWN,
            "the conversion should have thrown rather than substituting a value"
        );
    }

    fn truthy(helper: HelperFn, argument: Value) -> bool {
        let mut ctx = Ctx::buffered();
        call(helper, &mut ctx, &[argument])
            .expect("the helper succeeded")
            .as_bool()
            .expect("a truthiness helper produces a bool")
    }

    #[test]
    fn an_int_converts_at_both_extremes() {
        assert_eq!(string_result(mwl_int_to_string, Value::int(0)), "0");
        assert_eq!(string_result(mwl_int_to_string, Value::int(-42)), "-42");
        assert_eq!(
            string_result(mwl_int_to_string, Value::int(i64::MIN)),
            "-9223372036854775808"
        );
        assert_eq!(
            string_result(mwl_int_to_string, Value::int(i64::MAX)),
            "9223372036854775807"
        );
    }

    #[test]
    fn a_uint_converts_past_the_signed_range() {
        assert_eq!(string_result(mwl_uint_to_string, Value::uint(0)), "0");
        assert_eq!(
            string_result(mwl_uint_to_string, Value::uint(u64::MAX)),
            "18446744073709551615"
        );
    }

    #[test]
    fn a_float_converts_the_way_php_spells_it() {
        assert_eq!(string_result(mwl_float_to_string, Value::float(1.0)), "1");
        assert_eq!(
            string_result(mwl_float_to_string, Value::float(1e20)),
            "1.0E+20"
        );
    }

    #[test]
    fn a_bool_converts_to_one_or_the_empty_string() {
        assert_eq!(string_result(mwl_bool_to_string, Value::bool(true)), "1");
        assert_eq!(string_result(mwl_bool_to_string, Value::bool(false)), "");
    }

    #[test]
    fn scalar_truthiness_follows_phps_table() {
        assert!(!truthy(mwl_int_truthy, Value::int(0)));
        assert!(truthy(mwl_int_truthy, Value::int(-1)));
        assert!(!truthy(mwl_uint_truthy, Value::uint(0)));
        assert!(truthy(mwl_uint_truthy, Value::uint(1)));
        assert!(!truthy(mwl_float_truthy, Value::float(0.0)));
        assert!(!truthy(mwl_float_truthy, Value::float(-0.0)));
        assert!(truthy(mwl_float_truthy, Value::float(f64::NAN)));
        assert!(truthy(mwl_float_truthy, Value::float(0.1)));
    }

    #[test]
    fn string_truthiness_treats_only_empty_and_zero_as_falsy() {
        for (bytes, expected) in [
            (&b""[..], false),
            (&b"0"[..], false),
            (&b"0.0"[..], true),
            (&b"false"[..], true),
            (&b"00"[..], true),
            (&b" "[..], true),
        ] {
            let value = Value::str(MwlStr::new(bytes));
            assert_eq!(truthy(mwl_str_truthy, value), expected, "for {bytes:?}");
            #[expect(unsafe_code, reason = "the value owns the reference it releases")]
            unsafe {
                value.release();
            }
        }
    }

    #[test]
    fn echo_writes_raw_bytes_and_returns_nothing() {
        let mut ctx = Ctx::buffered();
        let value = Value::str(MwlStr::new(b"Hello, World!"));
        let result = call(mwl_echo_str, &mut ctx, &[value]).expect("the helper succeeded");
        assert_eq!(result.tag(), Some(Tag::Null));
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"Hello, World!"[..])
        );
        #[expect(unsafe_code, reason = "the value owns the reference it releases")]
        unsafe {
            value.release();
        }
    }

    #[test]
    fn echo_escapes_nothing() {
        let mut ctx = Ctx::buffered();
        let value = Value::str(MwlStr::new(b"<b>&\"\x00\xff"));
        call(mwl_echo_str, &mut ctx, &[value]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"<b>&\"\x00\xff"[..])
        );
        #[expect(unsafe_code, reason = "the value owns the reference it releases")]
        unsafe {
            value.release();
        }
    }

    #[test]
    fn a_wrong_tag_is_a_fatal_naming_the_helper_rather_than_a_bad_read() {
        let mut ctx = Ctx::buffered();
        assert_eq!(
            call(mwl_int_to_string, &mut ctx, &[Value::uint(1)]).unwrap_err(),
            FATAL
        );
        let message = ctx.take_pending().expect("a message was recorded");
        assert!(message.contains("mwl_int_to_string"), "{message}");

        assert_eq!(
            call(mwl_echo_str, &mut ctx, &[Value::int(1)]).unwrap_err(),
            FATAL
        );
        let message = ctx.take_pending().expect("a message was recorded");
        assert!(message.contains("mwl_echo_str"), "{message}");
        assert!(ctx.take_buffered_output().is_some_and(|out| out.is_empty()));
    }

    #[test]
    fn every_exported_symbol_has_a_distinct_non_null_address() {
        let table = symbols();
        assert!(!table.is_empty());
        for (name, address) in &table {
            assert!(!address.is_null(), "{name} resolved to null");
        }
        let mut names: Vec<_> = table.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "a symbol name is registered twice");
    }

    /// ADR 0007 § 2's `int` ↔ `uint` row: exact, or throws. The two ends that
    /// have no counterpart on the other side are the whole content of the row.
    #[test]
    fn int_and_uint_convert_where_the_ranges_overlap_and_throw_where_they_do_not() {
        assert_eq!(converted(mwl_int_to_uint, Value::int(0)).as_uint(), Some(0));
        assert_eq!(
            converted(mwl_int_to_uint, Value::int(i64::MAX)).as_uint(),
            Some(i64::MAX.cast_unsigned())
        );
        refused(mwl_int_to_uint, Value::int(-1));

        assert_eq!(
            converted(mwl_uint_to_int, Value::uint(i64::MAX.cast_unsigned())).as_int(),
            Some(i64::MAX)
        );
        refused(mwl_uint_to_int, Value::uint(i64::MAX.cast_unsigned() + 1));
        refused(mwl_uint_to_int, Value::uint(u64::MAX));
    }

    /// ADR 0007 § 2: an integer to `float` is "exact, or throws above 2^53,
    /// where `f64` stops representing every integer."
    #[test]
    fn an_integer_to_float_throws_past_the_point_it_would_stop_being_exact() {
        assert_eq!(
            converted(mwl_int_to_float, Value::int(-9007199254740992)).as_float(),
            Some(-9007199254740992.0)
        );
        assert_eq!(
            converted(mwl_uint_to_float, Value::uint(9007199254740992)).as_float(),
            Some(9007199254740992.0)
        );
        refused(mwl_int_to_float, Value::int(9007199254740993));
        refused(mwl_uint_to_float, Value::uint(u64::MAX));
    }

    /// ADR 0007 § 2: "integral and in range, or throws. Rounding is
    /// `floor`/`ceil`/`round`, said out loud" — so a fractional value is
    /// refused rather than silently picking one of the three.
    #[test]
    fn a_float_to_an_integer_refuses_anything_it_would_have_to_round() {
        assert_eq!(
            converted(mwl_float_to_int, Value::float(-3.0)).as_int(),
            Some(-3)
        );
        assert_eq!(
            converted(mwl_float_to_uint, Value::float(3.0)).as_uint(),
            Some(3)
        );
        refused(mwl_float_to_int, Value::float(1.5));
        refused(mwl_float_to_uint, Value::float(-1.0));
        refused(mwl_float_to_int, Value::float(f64::NAN));
        refused(mwl_float_to_int, Value::float(f64::INFINITY));
        refused(mwl_float_to_int, Value::float(1e30));
    }

    /// ADR 0007 § 2: "the whole string must be an exact numeric literal, or
    /// throws. No leading-garbage rule, no `0`" — PHP's `(int)"12abc" === 12`
    /// and `(int)"abc" === 0` are both gone.
    #[test]
    fn a_string_to_a_number_takes_the_whole_string_or_nothing() {
        /// Runs `check` over a fresh string argument and releases it after —
        /// a conversion helper only *reads* its operand (compiled code emits
        /// the release itself, see `mwl_ir::lower::Lowering::convert`), so a
        /// test that dropped the value here would leak it.
        fn with(text: &str, check: impl FnOnce(Value)) {
            let value = Value::str(MwlStr::new(text.as_bytes()));
            check(value);
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference the fresh `MwlStr` \
                          was built with, and the helper borrowed it"
            )]
            unsafe {
                value.release();
            }
        }

        with("-42", |v| {
            assert_eq!(converted(mwl_str_to_int, v).as_int(), Some(-42));
        });
        with("18446744073709551615", |v| {
            assert_eq!(converted(mwl_str_to_uint, v).as_uint(), Some(u64::MAX));
        });
        with("3.5", |v| {
            assert_eq!(converted(mwl_str_to_float, v).as_float(), Some(3.5));
        });
        for bad in ["12abc", "abc", "", " 12", "12 ", "0x10", "1.5"] {
            with(bad, |v| refused(mwl_str_to_int, v));
        }
        with("-1", |v| refused(mwl_str_to_uint, v));
        // Not "an exact numeric literal": no MWL source literal writes one.
        for bad in ["inf", "NaN", "infinity"] {
            with(bad, |v| refused(mwl_str_to_float, v));
        }
    }

    /// Every row [`value_to_string`] answers, against the statically-typed
    /// helper for the same tag — the two tables are the same table, and a
    /// program rendering a `mixed` must not get a second set of answers.
    #[test]
    fn a_tagged_operand_renders_by_its_tag() {
        assert_eq!(string_result(mwl_tagged_to_string, Value::null()), "");
        assert_eq!(string_result(mwl_tagged_to_string, Value::bool(true)), "1");
        assert_eq!(string_result(mwl_tagged_to_string, Value::bool(false)), "");
        assert_eq!(string_result(mwl_tagged_to_string, Value::int(-42)), "-42");
        assert_eq!(
            string_result(mwl_tagged_to_string, Value::uint(u64::MAX)),
            "18446744073709551615"
        );
        assert_eq!(string_result(mwl_tagged_to_string, Value::float(2.0)), "2");
        assert_eq!(
            string_result(mwl_tagged_to_string, Value::float(1.5)),
            "1.5"
        );
    }

    /// A `Tag::Str` payload is handed back as itself, so the one thing that
    /// could go wrong is the reference count: the result has to carry a
    /// *fresh* reference, because its caller releases it exactly as it
    /// releases a converted one.
    #[test]
    fn a_string_payload_comes_back_with_a_reference_of_its_own() {
        let value = Value::str(MwlStr::new(b"text"));
        let ptr = value.str_ptr().expect("a Tag::Str value carries a pointer");
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference the fresh `MwlStr` was \
                      built with, so the allocation is live for every read below"
        )]
        unsafe {
            assert_eq!(MwlStr::refcount_of(ptr), 1);
            // `string_result` releases what the helper returned, so a helper
            // that answered without retaining would leave zero here — and a
            // use-after-free rather than an assertion failure.
            assert_eq!(string_result(mwl_tagged_to_string, value), "text");
            assert_eq!(MwlStr::refcount_of(ptr), 1);
            value.release();
        }
    }

    /// A tag ADR 0007 § 2 writes no row from throws rather than substituting
    /// PHP's `"Array"`-plus-warning. An array stands for the four such tags:
    /// they share one arm.
    #[test]
    fn a_tag_with_no_string_row_throws() {
        let value = Value::array(crate::array::MwlArray::new());
        refused(mwl_tagged_to_string, value);
        #[expect(
            unsafe_code,
            reason = "the helper borrowed the array; this test still owns the \
                      one reference it was built with"
        )]
        unsafe {
            value.release();
        }
    }
}
