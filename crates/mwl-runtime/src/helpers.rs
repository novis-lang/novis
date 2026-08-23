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

crate::mwl_helper! {
    /// `mwl_ir::Helper::IntToUint`.
    fn mwl_int_to_uint(_ctx, args: [1]) {
        let value = expect_tag!("mwl_int_to_uint", args[0], as_int, Tag::Int);
        u64::try_from(value)
            .map(Value::uint)
            .map_err(|_| does_not_fit(&format!("`int` {value}"), "uint"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::UintToInt`.
    fn mwl_uint_to_int(_ctx, args: [1]) {
        let value = expect_tag!("mwl_uint_to_int", args[0], as_uint, Tag::Uint);
        i64::try_from(value)
            .map(Value::int)
            .map_err(|_| does_not_fit(&format!("`uint` {value}"), "int"))
    }
}

/// The magnitude past which an `f64` no longer represents every integer —
/// ADR 0007 § 2's 2^53 boundary, shared by both integer-to-`float` helpers.
const F64_EXACT_INT_LIMIT: u64 = 1 << 53;

crate::mwl_helper! {
    /// `mwl_ir::Helper::IntToFloat`.
    fn mwl_int_to_float(_ctx, args: [1]) {
        let value = expect_tag!("mwl_int_to_float", args[0], as_int, Tag::Int);
        if value.unsigned_abs() > F64_EXACT_INT_LIMIT {
            return Err(does_not_fit(&format!("`int` {value}"), "float"));
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "the magnitude check above is exactly what makes this exact"
        )]
        Ok(Value::float(value as f64))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::UintToFloat`.
    fn mwl_uint_to_float(_ctx, args: [1]) {
        let value = expect_tag!("mwl_uint_to_float", args[0], as_uint, Tag::Uint);
        if value > F64_EXACT_INT_LIMIT {
            return Err(does_not_fit(&format!("`uint` {value}"), "float"));
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "the magnitude check above is exactly what makes this exact"
        )]
        Ok(Value::float(value as f64))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::FloatToInt`.
    fn mwl_float_to_int(_ctx, args: [1]) {
        let value = expect_tag!("mwl_float_to_int", args[0], as_float, Tag::Float);
        exact_integral(value)
            .and_then(|v| i64::try_from(v).ok())
            .map(Value::int)
            .ok_or_else(|| does_not_fit(&format!("`float` {value}"), "int"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::FloatToUint`.
    fn mwl_float_to_uint(_ctx, args: [1]) {
        let value = expect_tag!("mwl_float_to_uint", args[0], as_float, Tag::Float);
        exact_integral(value)
            .and_then(|v| u64::try_from(v).ok())
            .map(Value::uint)
            .ok_or_else(|| does_not_fit(&format!("`float` {value}"), "uint"))
    }
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
    // Every `f64` with a zero fractional part and a magnitude below 2^127 is
    // exactly an integer, and `i128` holds all of them.
    if value.abs() >= 2.0_f64.powi(127) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the two guards above leave only values `i128` represents exactly"
    )]
    Some(value as i128)
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
        text.parse::<i64>()
            .map(Value::int)
            .map_err(|_| does_not_fit(&format!("string {text:?}"), "int"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::StrToUint`.
    fn mwl_str_to_uint(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("mwl_str_to_uint", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "mwl_str_to_uint", args[0])?;
        text.parse::<u64>()
            .map(Value::uint)
            .map_err(|_| does_not_fit(&format!("string {text:?}"), "uint"))
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::StrToFloat`.
    fn mwl_str_to_float(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("mwl_str_to_float", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "mwl_str_to_float", args[0])?;
        // `f64::from_str` accepts `inf`/`nan`/`infinity` in any case; none is
        // an "exact numeric literal", so each is refused here rather than
        // becoming a value no source literal could have written.
        if text.parse::<f64>().is_ok_and(f64::is_finite) {
            #[expect(clippy::unwrap_used, reason = "the `is_ok_and` above proved it parses")]
            Ok(Value::float(text.parse::<f64>().unwrap()))
        } else {
            Err(does_not_fit(&format!("string {text:?}"), "float"))
        }
    }
}

crate::mwl_helper! {
    /// `mwl_ir::Helper::EchoStr` — raw bytes to the request's own output, with
    /// no escaping. `.claude/loop-goal.md` records that decision and why
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
        ("mwl_int_to_uint", address(mwl_int_to_uint)),
        ("mwl_uint_to_int", address(mwl_uint_to_int)),
        ("mwl_int_to_float", address(mwl_int_to_float)),
        ("mwl_uint_to_float", address(mwl_uint_to_float)),
        ("mwl_float_to_int", address(mwl_float_to_int)),
        ("mwl_float_to_uint", address(mwl_float_to_uint)),
        ("mwl_str_to_int", address(mwl_str_to_int)),
        ("mwl_str_to_uint", address(mwl_str_to_uint)),
        ("mwl_str_to_float", address(mwl_str_to_float)),
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
            "mwl_str_retain",
            (crate::string::mwl_str_retain as *const ()).cast::<u8>(),
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
}
