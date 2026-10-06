//! The closed set of engine-owned runtime helpers compiled code can call.
//!
//! Each one backs exactly one `nvs_ir::Helper` variant. That enum is the
//! authority on *what* each helper means — its own doc comments carry the
//! semantics, including PHP's rules for bool-to-string and for what counts as
//! falsy — so nothing is restated here; this module is the implementation, and
//! [`symbols`] is the table `nvs-codegen` registers with the JIT.
//!
//! `nvs-runtime` deliberately does not depend on `nvs-ir`, so the mapping from
//! a `Helper` tag to a symbol name lives in `nvs-codegen`, which depends on
//! both. What lives here is the name itself, and the guarantee that the name
//! resolves.
//!
//! # Argument tags are checked, not assumed
//!
//! Every helper below re-checks its arguments' tags and returns [`Fault::Fatal`] on a
//! mismatch rather than reading the payload anyway. A mismatch cannot happen
//! in a well-typed program — `rule:types/declaration`
//! settles every operand type before lowering, and `nvs_ir::lower` picks the
//! helper from that type — so the check is not defending against user code. It
//! is defending against a *miscompile*: reading a `Value`'s payload under the
//! wrong tag would be a memory-safety bug (an `int` payload read as a string
//! pointer), and `rule:errors/propagation` already names a silently wrong call
//! site as the nastier failure mode of the checked-return design. A predicted
//! branch on a tag byte is a cheap price for making that class of bug a
//! `FATAL` with a message instead.
//!
//! Every `nvs_ir::Helper` variant has an entry point here. The entry points
//! that back no `Helper` variant at all are [`nvs_array_required_get`] and
//! [`nvs_array_optional_get`], the two halves of `nvs_ir::InstKind::ArrayGet`,
//! which names one of them directly off its own `absent` field. That
//! instruction's doc comment says why a subscript read rather than a conversion
//! needs this signature at all.

use subtle::ConstantTimeEq;

use crate::abi::{Fault, HelperFn};
use crate::decimal::Decimal;
use crate::fmt::php_float_to_string;
use crate::string::NvsStr;
use crate::value::{Tag, Value};

/// The `FATAL` a tag mismatch produces — see this module's docs for why it is
/// checked at all.
pub(crate) fn wrong_tag(helper: &'static str, expected: Tag, actual: Value) -> Fault {
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

crate::nvs_helper! {
    /// `nvs_ir::Helper::IntToString`.
    fn nvs_int_to_string(_ctx, args: [1]) {
        let value = expect_tag!("nvs_int_to_string", args[0], as_int, Tag::Int);
        Ok(Value::str(NvsStr::new(value.to_string().as_bytes())))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::UintToString`.
    fn nvs_uint_to_string(_ctx, args: [1]) {
        let value = expect_tag!("nvs_uint_to_string", args[0], as_uint, Tag::Uint);
        Ok(Value::str(NvsStr::new(value.to_string().as_bytes())))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::FloatToString`.
    fn nvs_float_to_string(_ctx, args: [1]) {
        let value = expect_tag!("nvs_float_to_string", args[0], as_float, Tag::Float);
        Ok(Value::str(NvsStr::new(php_float_to_string(value).as_bytes())))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::BoolToString`.
    fn nvs_bool_to_string(_ctx, args: [1]) {
        let value = expect_tag!("nvs_bool_to_string", args[0], as_bool, Tag::Bool);
        Ok(Value::str(NvsStr::new(if value { b"1".as_slice() } else { b"".as_slice() })))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ClassDescName`.
    fn nvs_class_desc_name(_ctx, args: [1]) {
        let desc = expect_tag!("nvs_class_desc_name", args[0], as_class_desc, Tag::Null);
        // A descriptor is owned by the unit's `ClassTable` and outlives every
        // frame that can name one, so this read needs no lifetime beyond the
        // call — the same borrow `nvs_stdlib::reflect` takes of a receiver's
        // own class. `as_class_desc` has already rejected the null payload.
        #[allow(
            unsafe_code,
            reason = "a `Ty::ClassDesc` slot carries a descriptor address by \
                      construction (`nvs_codegen::ty::tag_of`), and the \
                      `ClassTable` that owns it outlives the unit"
        )]
        let name = unsafe { (*desc).name() };
        Ok(Value::str(NvsStr::new(name.as_bytes())))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ClassConstant` — `static::NAME`'s run-time read.
    ///
    /// The descriptor's table is flattened own-first (`ClassDesc::constants`),
    /// so a name a subclass redeclares answers the subclass's value, which is
    /// what late static binding means for a constant. A name the table lacks,
    /// or one whose value is `ConstantValue::Opaque`, is a `FATAL` naming the
    /// class: `nvs_types` refuses both reads before lowering, so reaching
    /// either here is a checker that did not run.
    fn nvs_class_constant(_ctx, args: [2]) {
        let desc = expect_tag!("nvs_class_constant", args[0], as_class_desc, Tag::Null);
        let Some(name) = args[1].as_str_bytes() else {
            return Err(wrong_tag("nvs_class_constant", Tag::Str, args[1]));
        };
        let name = String::from_utf8_lossy(name);
        // The same borrow `nvs_class_desc_name` takes: the descriptor is owned
        // by the unit's `ClassTable` and outlives every frame that can name it.
        #[allow(
            unsafe_code,
            reason = "a `Ty::ClassDesc` slot carries a descriptor address by \
                      construction (`nvs_codegen::ty::tag_of`), and the \
                      `ClassTable` that owns it outlives the unit"
        )]
        let (class, found) = unsafe { ((*desc).name(), (*desc).constant(&name)) };
        match found.map(|constant| &constant.value) {
            Some(crate::object::ConstantValue::Str(text)) => {
                Ok(Value::str(NvsStr::new(text.as_bytes())))
            }
            Some(crate::object::ConstantValue::Int(value)) => Ok(Value::int(*value)),
            Some(crate::object::ConstantValue::Bool(value)) => Ok(Value::bool(*value)),
            Some(crate::object::ConstantValue::Float(value)) => Ok(Value::float(*value)),
            Some(crate::object::ConstantValue::Opaque) => Err(Fault::fatal(format!(
                "`{class}::{name}` has no scalar value to read through `static::`"
            ))),
            None => Err(Fault::fatal(format!(
                "`{class}` declares no constant `{name}` for `static::` to read"
            ))),
        }
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::IntTruthy`.
    fn nvs_int_truthy(_ctx, args: [1]) {
        let value = expect_tag!("nvs_int_truthy", args[0], as_int, Tag::Int);
        Ok(Value::bool(value != 0))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::UintTruthy`.
    fn nvs_uint_truthy(_ctx, args: [1]) {
        let value = expect_tag!("nvs_uint_truthy", args[0], as_uint, Tag::Uint);
        Ok(Value::bool(value != 0))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::FloatTruthy`. `-0.0` is falsy with `0.0`; `NAN` is
    /// truthy, which `!=` gives for free.
    fn nvs_float_truthy(_ctx, args: [1]) {
        let value = expect_tag!("nvs_float_truthy", args[0], as_float, Tag::Float);
        Ok(Value::bool(value != 0.0))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::StrTruthy`.
    fn nvs_str_truthy(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("nvs_str_truthy", Tag::Str, args[0]))?;
        Ok(Value::bool(!(bytes.is_empty() || bytes == b"0")))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::BytesTruthy` — falsy iff the buffer is empty, which
    /// is [`nvs_str_truthy`]'s row **without** its `"0"` case. That case is
    /// PHP's numeric-string rule and
    /// `rule:types/bytes`'s binary scalar
    /// never converts to a number, so a one-octet buffer holding `0x30` is
    /// truthy here where the `string` spelling of the same octet is not.
    /// [`value_truthy`]'s `Tag::Bytes` arm is this rule reached through a
    /// `mixed`, and the two are deliberately one sentence.
    fn nvs_bytes_truthy(_ctx, args: [1]) {
        let bytes = args[0]
            .as_bytes()
            .ok_or_else(|| wrong_tag("nvs_bytes_truthy", Tag::Bytes, args[0]))?;
        Ok(Value::bool(!bytes.is_empty()))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ArrayTruthy` — falsy iff the array holds no entries,
    /// for any element type. [`nvs_str_truthy`]'s structure with a different
    /// emptiness test.
    fn nvs_array_truthy(_ctx, args: [1]) {
        let array = args[0]
            .array_ptr()
            .ok_or_else(|| wrong_tag("nvs_array_truthy", Tag::Array, args[0]))?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live                       allocation, so it is live for this read"
        )]
        let count = unsafe { crate::array::nvs_array_count(array) };
        Ok(Value::bool(count != 0))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueTruthy` —
    /// `rule:expressions/truthy-table`'s
    /// table over a value whose type the compiler erased, which is the table's
    /// own last row. Every other row is reached without this helper, because
    /// the operand's static type already named it:
    /// `nvs_ir::lower::Lowering::truthy_convert`'s arms are that table, one
    /// representation at a time.
    ///
    /// The row itself is [`value_truthy`], which is total — § 2 covers every
    /// type that can reach a condition — so this carries no error edge, the
    /// same reason [`nvs_value_identical`] carries none on the equality side.
    /// The operand is borrowed, the treatment [`nvs_array_truthy`] already
    /// gives its own.
    fn nvs_value_truthy(_ctx, args: [1]) {
        Ok(Value::bool(value_truthy(args[0])))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ArrayRowForWrite` — one level of a nested
    /// `$grid[0][1] = v`'s descent, and the only array entry point whose
    /// answer is **owned** rather than borrowed.
    ///
    /// Two cases, one ownership answer. A row that is there comes back
    /// retained, so the [`crate::array::nvs_array_set`] the lowering emits on
    /// the way back up has a reference to consume — and so the row's count is
    /// at least two, which is exactly what makes that write separate it (ADR
    /// 0007 § 5). A key that is *absent* comes back as the thread's empty
    /// array, which is PHP's auto-vivification: `$g[9][0] = 1` over an empty
    /// `$g` builds the missing row rather than faulting. That row separates on
    /// the write for the same reason the present one does — the singleton's
    /// count is never 1 either — so the two cases reach `make_unique` alike.
    /// The fresh row is not inserted here; the same `nvs_array_set` stores it,
    /// since replacing a row and inserting one are one instruction.
    ///
    /// The declared element type is what rules out the third case PHP has —
    /// a present entry that is not an array — so a non-array row here is an
    /// internal inconsistency and is reported as a wrong tag rather than as a
    /// language-level fault.
    fn nvs_array_row_for_write(_ctx, args: [2]) {
        let array = args[0]
            .array_ptr()
            .ok_or_else(|| wrong_tag("nvs_array_row_for_write", Tag::Array, args[0]))?;
        let mut row = Value::default();
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for this read, and `row` is a \
                      readable 16-byte slot on this frame"
        )]
        unsafe {
            if let Some(key) = args[1].str_ptr() {
                crate::array::nvs_array_get(array, key, &raw mut row);
            } else if let Some(index) = args[1].as_int() {
                crate::array::nvs_array_get_index(array, index, &raw mut row);
            } else {
                return Err(wrong_tag("nvs_array_row_for_write", Tag::Str, args[1]));
            }
        }
        let Some(ptr) = row.array_ptr() else {
            if row.tag().is_none() || row.tag() == Some(Tag::Null) {
                return Ok(Value::from_array_ptr(crate::array::nvs_array_new()));
            }
            return Err(wrong_tag("nvs_array_row_for_write", Tag::Array, row));
        };
        #[expect(
            unsafe_code,
            reason = "the entry just read is live for as long as the array \
                      holding it, which this call's argument owns"
        )]
        unsafe {
            crate::array::nvs_array_retain(ptr);
        }
        Ok(row)
    }
}

/// The message an absent key is reported with, built only on the throwing
/// edge — a read that finds its entry never renders its key at all.
fn undefined_key(key: Value) -> Fault {
    #[expect(
        unsafe_code,
        reason = "a Tag::Str argument owns a reference to a live allocation, \
                  so it is live for this read"
    )]
    let rendered = match key.str_ptr() {
        Some(ptr) => unsafe { String::from_utf8_lossy(NvsStr::bytes_of(ptr)).into_owned() },
        None => match key.as_int() {
            Some(index) => index.to_string(),
            None => "?".to_owned(),
        },
    };
    Fault::thrown(format!("undefined array key `{rendered}`"))
}

crate::nvs_helper! {
    /// `nvs_ir::InstKind::ArrayGet` — the entry `args[1]` names in the array
    /// `args[0]`, **borrowed**, or a throw when the key is absent.
    ///
    /// The one runtime entry point that is not a `nvs_ir::Helper` variant, for
    /// the reason that instruction's own doc comment gives: a subscript read
    /// is an instruction rather than a conversion, and it needs this module's
    /// `rule:errors/propagation` signature only because it can now fail.
    ///
    /// PHP warns and yields `null` here. Novis has no `null` to put in an
    /// `array<string>`, and a null-shaped answer would be read by every
    /// consumer as its declared type — a string pointer, an object pointer —
    /// so the failure would be a null dereference below the language rather
    /// than an error inside it. `rule:types/absent-storage-is-never-a-zero-value` records
    /// the divergence, and it is row 8 (an undefined *variable* is a check-time
    /// error) one storage kind along: absent storage is never a zero value.
    ///
    /// An absent key is told from a stored `null` by
    /// [`crate::array::entry`]'s `Option`, so an `array<?string>` holding a
    /// `null` at `"k"` reads that `null` back rather than throwing.
    /// [`crate::array::nvs_array_get`] is the other read — the vivifying one
    /// the *write* side descends through, whose absent-key answer is a fresh
    /// row (`nvs_ir::Helper::ArrayRowForWrite`).
    fn nvs_array_required_get(_ctx, args: [2]) {
        let array = args[0]
            .array_ptr()
            .ok_or_else(|| wrong_tag("nvs_array_required_get", Tag::Array, args[0]))?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for this read, and so is the \
                      Tag::Str key beside it"
        )]
        let found = unsafe {
            if let Some(key) = args[1].str_ptr() {
                crate::array::entry(array, NvsStr::bytes_of(key))
            } else if let Some(index) = args[1].as_int() {
                crate::array::entry_at_index(array, index)
            } else {
                return Err(wrong_tag("nvs_array_required_get", Tag::Str, args[1]));
            }
        };
        found.ok_or_else(|| undefined_key(args[1]))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::InstKind::ArrayGet` under an
    /// [`nvs_ir::ir::AbsentKey::Null`](../../nvs_ir/ir/enum.AbsentKey.html)
    /// — the same **borrowed** read as [`nvs_array_required_get`], answering an
    /// absent key with `null` instead of throwing.
    ///
    /// This is the read under a `??`, and it is the one place
    /// `rule:types/absent-storage-is-never-a-zero-value`'s throw is carved back
    /// out: `??` means "absent or `null`", so the whole point of the guard is that
    /// the absent case has an answer. The answer is always a
    /// `nvs_ir::Ty::Tagged` value, because "the element, or `null`" is a
    /// nullable however narrow the array's element type is, and `nvs_ir::lower`
    /// hands it straight to `??`'s own null test.
    ///
    /// A stored `null` and an absent key are deliberately *not* told apart
    /// here, unlike in [`nvs_array_required_get`]: `??` yields its right
    /// operand for both, so collapsing them is what PHP does rather than a
    /// simplification of it.
    ///
    /// Infallible — it carries the `rule:errors/propagation` signature every helper does, but
    /// the only status it ever returns is `OK`, so `nvs-ir` emits it with no
    /// error edge.
    fn nvs_array_optional_get(_ctx, args: [2]) {
        // A `null` *array* is the nested chain: `$a["k"]["j"] ?? "d"` marks
        // every level guarded, so this read's own base is the previous level's
        // answer and that answer is `null` when its key was absent. PHP reads
        // the whole chain as one guarded lookup, so the answer here is `null`
        // again rather than the tag mismatch a well-typed program otherwise
        // cannot produce — `nvs_types::Env::coalesce_guarded` owns the marking.
        if args[0].tag() == Some(Tag::Null) {
            return Ok(Value::null());
        }
        let array = args[0]
            .array_ptr()
            .ok_or_else(|| wrong_tag("nvs_array_optional_get", Tag::Array, args[0]))?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for this read, and so is the \
                      Tag::Str key beside it"
        )]
        let found = unsafe {
            if let Some(key) = args[1].str_ptr() {
                crate::array::entry(array, NvsStr::bytes_of(key))
            } else if let Some(index) = args[1].as_int() {
                crate::array::entry_at_index(array, index)
            } else {
                return Err(wrong_tag("nvs_array_optional_get", Tag::Str, args[1]));
            }
        };
        Ok(found.unwrap_or(Value::null()))
    }
}

/// The catchable throw a subscript through a tagged base raises when the tag
/// turns out not to be an array at all.
///
/// It carries `nvs_types::expr::report_unsubscriptable`'s own wording
/// (`E0482`) on purpose: "only an `array<T>` has elements" is one rule, and a
/// base that hid the answer behind a `mixed` should read as the same refusal
/// the site makes wherever the declared type shows it. PHP warns and yields
/// `null` here; `rule:types/absent-storage-is-never-a-zero-value` already records that divergence for the
/// absent key, and this is the same one storage kind up.
fn not_subscriptable(base: Value) -> Fault {
    Fault::thrown(format!(
        "`{}` cannot be subscripted — only an `array<T>` has elements",
        tag_name(base)
    ))
}

/// The two rows of `nvs_ir::Helper::ValueIndexGet` and its
/// `…OptionalGet` twin: an element read whose base is a
/// `nvs_ir::Ty::Tagged`, so that *whether there is an array* is the tag's
/// question rather than the site's.
///
/// One implementation for both, the arrangement
/// [`crate::helpers::nvs_tagged_to_bytes`]'s own twin already uses, because
/// they differ in exactly one axis. `absent_is_null` is `??`'s guard, and it
/// widens **both** failures rather than only the key one: PHP's `$m["k"] ??
/// "d"` yields `"d"` for every `$m` that is not an array as readily as for an
/// array missing the key, so a guarded read has no throw at all. Unguarded,
/// the two failures are distinct and both catchable — [`undefined_key`] for a
/// key the array does not hold, [`not_subscriptable`] for a base that is no
/// array.
///
/// The answer is **borrowed**, exactly as [`nvs_array_required_get`]'s is:
/// the entry lives as long as the array holding it, which the caller owns for
/// the duration of the call.
fn value_index(base: Value, key: Value, absent_is_null: bool) -> Result<Value, Fault> {
    let Some(array) = base.array_ptr() else {
        if absent_is_null {
            return Ok(Value::null());
        }
        return Err(not_subscriptable(base));
    };
    #[expect(
        unsafe_code,
        reason = "a Tag::Array argument owns a reference to a live \
                  allocation, so it is live for this read, and so is the \
                  Tag::Str key beside it"
    )]
    let found = unsafe {
        if let Some(key) = key.str_ptr() {
            crate::array::entry(array, NvsStr::bytes_of(key))
        } else if let Some(index) = key.as_int() {
            crate::array::entry_at_index(array, index)
        } else {
            return Err(wrong_tag("nvs_value_index_get", Tag::Str, key));
        }
    };
    match found {
        Some(value) => Ok(value),
        None if absent_is_null => Ok(Value::null()),
        None => Err(undefined_key(key)),
    }
}

/// `nvs_ir::Helper::ValueToArrayKey` — `rule:types/arrays`'s key
/// normalization, performed on a tag because the key's declared type was a
/// `mixed` and showed nothing.
///
/// The three rows the checker would have accepted are the three rows here: a
/// `string` is already the key, and an `int` or a `uint` is its own decimal,
/// which is the same normalization `$a[8]` gets where the subscript's type is
/// visible. Every other tag is the compile-time refusal `E0434` makes, raised
/// here as a **catchable throw** naming the tag instead, for
/// [`not_subscriptable`]'s reason — a `mixed` deferred the question rather
/// than answering it differently.
///
/// The answer is **owned**: a `string` key is copied rather than passed back,
/// because the caller stages whatever this returns as a temporary it releases,
/// exactly as it does for [`nvs_uint_to_string`]'s freshly rendered buffer.
#[expect(
    unsafe_code,
    reason = "a Tag::Str argument owns a reference to a live allocation, so \
              its bytes are live for the copy taken from them here"
)]
fn value_to_array_key(key: Value) -> Result<Value, Fault> {
    if let Some(text) = key.str_ptr() {
        let bytes = unsafe { NvsStr::bytes_of(text) };
        return Ok(Value::str(NvsStr::new(bytes)));
    }
    if let Some(index) = key.as_int() {
        return Ok(Value::str(NvsStr::new(index.to_string().as_bytes())));
    }
    if let Some(index) = key.as_uint() {
        return Ok(Value::str(NvsStr::new(index.to_string().as_bytes())));
    }
    Err(Fault::thrown(format!(
        "`{}` is not an array key — array keys are `int`, `uint`, or `string`",
        tag_name(key)
    )))
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueToArrayKey` — the key of `$a[$k]` where `$k` is
    /// a `mixed`. See [`value_to_array_key`].
    fn nvs_value_to_array_key(_ctx, args: [1]) {
        value_to_array_key(args[0])
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueIndexGet` — `$m[$k]` where the base's static
    /// type named no element type, so its tag names one instead. See
    /// [`value_index`].
    fn nvs_value_index_get(_ctx, args: [2]) {
        value_index(args[0], args[1], false)
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueIndexOptionalGet` — [`nvs_value_index_get`]
    /// under a `??`, which answers `null` for a missing key *and* for a base
    /// that is no array. See [`value_index`].
    fn nvs_value_index_optional_get(_ctx, args: [2]) {
        value_index(args[0], args[1], true)
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::Identical` — `==` where at least one operand is a
    /// `mixed` or a union, which is
    /// `rule:expressions/mixed-equality`'s case and the only one whose row is a runtime tag rather than a
    /// static type. The row itself is [`crate::value_identical`], so a tagged
    /// operand and a statically typed one answer alike; a pair whose tags name
    /// different rows is `false` there, never a throw, which is why this helper
    /// is infallible and carries no error edge. `!=` is this helper under an
    /// `nvs_ir::UnOp::Not`, the arrangement [`nvs_decimal_eq`] already uses.
    fn nvs_value_identical(_ctx, args: [2]) {
        Ok(Value::bool(crate::value_identical(args[0], args[1])))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::NumericEq` — `==` over two operands whose
    /// representations differ but whose types are
    /// `rule:expressions/disjoint-comparison-refused`'s one numeric domain. The row is [`crate::numeric_identical`],
    /// which is total, so this carries no error edge; `!=` is this helper
    /// under an `nvs_ir::UnOp::Not`, the arrangement [`nvs_decimal_eq`]
    /// already uses.
    fn nvs_numeric_eq(_ctx, args: [2]) {
        Ok(Value::bool(crate::numeric_identical(args[0], args[1])))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::NumericLt` — `<` over two operands whose
    /// representations differ but whose types are `rule:types/arithmetic`'s one numeric
    /// domain. The row is [`crate::numeric_ordering`], and `>` is this helper
    /// with its operands swapped.
    ///
    /// An unordered pair — a `NaN` on either side — is `false`, which is PHP's
    /// answer for all four ordering operators against one. That is why this
    /// takes the ordering rather than a `bool` from the row: `false` here is
    /// "not less", not "greater".
    ///
    /// Total, so it carries no error edge.
    fn nvs_numeric_lt(_ctx, args: [2]) {
        Ok(Value::bool(matches!(
            crate::numeric_ordering(args[0], args[1]),
            Some(core::cmp::Ordering::Less)
        )))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::NumericLtEq` — [`nvs_numeric_lt`]'s row inclusive, and
    /// `>=` is this helper with its operands swapped. A `NaN` operand is
    /// `false` here too, for the reason that one states.
    fn nvs_numeric_lt_eq(_ctx, args: [2]) {
        Ok(Value::bool(matches!(
            crate::numeric_ordering(args[0], args[1]),
            Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
        )))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::NumericCmp` — `<=>` over a mixed numeric pair, which
    /// is [`nvs_numeric_lt`]'s row read whole rather than asked one question.
    ///
    /// Total, so no error edge. See [`spaceship`] for the `NaN` row, which is
    /// the one place a `<=>` helper has a choice to make.
    fn nvs_numeric_cmp(_ctx, args: [2]) {
        Ok(Value::int(spaceship(crate::numeric_ordering(args[0], args[1]))))
    }
}

/// `rule:types/arithmetic`'s ordering table, chosen from two runtime **tags** rather than
/// from two static types — the row a `mixed` or a union operand defers, and the
/// one `nvs_ir::Helper::ValueLt` and its siblings are all reading.
///
/// `Ok(None)` is the unordered answer a `NaN` operand gives, exactly as
/// [`crate::numeric_ordering`] and [`decimal_ordering`] give it, and it makes
/// all four ordering operators false and `<=>` answer `1`.
///
/// `Err` is the other end of the same table, and the reason this helper family
/// carries an error edge where every other comparison helper does not: § 4's
/// ordering row is a **closed** list, so a pair it names none for has no
/// ordering at all rather than a plausible one. The wording is
/// `nvs_types::expr::operators::reject_unordered_operand`'s, because it is the
/// same refusal — made here only because the tags are where it first became
/// answerable.
///
/// An enum case is deliberately *not* one of those pairs: `rule:types/single-value-types` spends
/// no representation on one, so behind a `mixed` it is the `int` or `uint` its
/// cases are, and it orders as one. The static spelling still refuses it
/// (`E0715`), which is where an author is told to say `as int` out loud.
fn value_ordering(left: Value, right: Value) -> Result<Option<std::cmp::Ordering>, Fault> {
    match (left.tag(), right.tag()) {
        // `rule:types/arithmetic`'s `bool` row: the ordering of the one bit it already
        // is, `false < true`.
        (Some(Tag::Bool), Some(Tag::Bool)) => Ok(Some(left.bits().cmp(&right.bits()))),
        // `rule:types/arithmetic`'s comparison row spans every pairing one side of which
        // is a `decimal`, the `decimal`/`float` one included.
        (Some(Tag::Decimal), Some(Tag::Decimal | Tag::Int | Tag::Uint | Tag::Float))
        | (Some(Tag::Int | Tag::Uint | Tag::Float), Some(Tag::Decimal)) => {
            Ok(decimal_ordering(left, right))
        }
        (Some(Tag::Int | Tag::Uint | Tag::Float), Some(Tag::Int | Tag::Uint | Tag::Float)) => {
            Ok(crate::numeric_ordering(left, right))
        }
        _ => Err(no_ordering(left, right)),
    }
}

/// One operand's type as `rule:types/declaration` spells it, for a message a program reads.
///
/// Shared by every refusal a tag-dispatched row makes — [`no_ordering`] and
/// [`no_arithmetic`] — so that "a `string` against an `array<T>`" reads the
/// same whichever operator asked. A callable renders as `object` because it *is*
/// one (`rule:types/callable-values`), and the fallback covers the tags
/// no source value carries.
fn tag_name(value: Value) -> &'static str {
    match value.tag() {
        Some(Tag::Null) => "null",
        Some(Tag::Bool) => "bool",
        Some(Tag::Int) => "int",
        Some(Tag::Uint) => "uint",
        Some(Tag::Float) => "float",
        Some(Tag::Str) => "string",
        Some(Tag::Array) => "array<T>",
        Some(Tag::Object) => "object",
        Some(Tag::Decimal) => "decimal",
        Some(Tag::Bytes) => "bytes",
        _ => "value",
    }
}

/// The catchable throw [`value_ordering`] raises for a pair `rule:types/arithmetic`
/// tabulates no row for, naming the spelling that says what was meant wherever
/// there is one — the same three wordings `E0715` carries.
fn no_ordering(left: Value, right: Value) -> Fault {
    let (left_name, right_name) = (tag_name(left), tag_name(right));
    let hint = match (left.tag(), right.tag()) {
        (Some(Tag::Str), Some(Tag::Str)) => {
            " — `Core\\Str::compare` is the ordering two strings have"
        }
        (Some(Tag::Object), Some(Tag::Object)) => {
            " — ordering two objects needs the `Comparable` class named where the comparison is \
             written (`rule:classes/comparable`)"
        }
        _ => "",
    };
    Fault::thrown(format!(
        "no ordering for a `{left_name}` against a `{right_name}`{hint}"
    ))
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueLt` — `<` where at least one operand's static
    /// type named no row, so the tag names it instead. The row is
    /// [`value_ordering`], and `>` is this helper with its operands swapped.
    ///
    /// An unordered pair — a `NaN` on either side — is `false`, which is PHP's
    /// answer for all four ordering operators against one, and is why this
    /// takes the ordering rather than a `bool` from the row.
    fn nvs_value_lt(_ctx, args: [2]) {
        Ok(Value::bool(matches!(
            value_ordering(args[0], args[1])?,
            Some(core::cmp::Ordering::Less)
        )))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueLtEq` — [`nvs_value_lt`]'s row inclusive, and
    /// `>=` is this helper with its operands swapped.
    fn nvs_value_lt_eq(_ctx, args: [2]) {
        Ok(Value::bool(matches!(
            value_ordering(args[0], args[1])?,
            Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
        )))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueCmp` — `<=>` over a tagged pair, which is
    /// [`nvs_value_lt`]'s row read whole rather than asked one question. See
    /// [`spaceship`] for the `NaN` row.
    fn nvs_value_cmp(_ctx, args: [2]) {
        Ok(Value::int(spaceship(value_ordering(args[0], args[1])?)))
    }
}

/// One row of `rule:types/arithmetic`'s arithmetic and bitwise table, named so that the
/// helpers below share one implementation of it rather than a copy of the tag
/// dispatch apiece.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ArithRow {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl ArithRow {
    /// The operator as a program writes it, for the refusals below.
    fn spelling(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
            Self::Pow => "**",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
        }
    }

    /// The operation as the two overflow messages name it — `nvs-codegen`'s
    /// `emit_checked_int_arith` and `emit_int_pow` word theirs "Integer
    /// {word} overflowed", and [`arithmetic_error`] words `rule:types/arithmetic`'s
    /// "`decimal` {word} is outside the type's range", so one word serves both
    /// and the statically and dynamically typed ends of a row cannot drift
    /// apart in their wording.
    fn word(self) -> &'static str {
        match self {
            Self::Add => "addition",
            Self::Sub => "subtraction",
            Self::Mul => "multiplication",
            Self::Div => "division",
            Self::Mod => "remainder",
            Self::Pow => "exponentiation",
            Self::BitAnd | Self::BitOr | Self::BitXor | Self::Shl | Self::Shr => "bit operation",
        }
    }
}

/// `rule:types/arithmetic`'s **arithmetic** rows, chosen from two runtime **tags** rather
/// than from two static types — the row a `mixed`, a union or the `int|float` a
/// division returns defers, and the one the `nvs_ir::Helper::ValueAdd` family
/// is all reading. It is [`value_ordering`]'s twin, one table over from it.
///
/// The refusals here outnumber the ordering table's, and each is the same
/// refusal `nvs_types` makes wherever the static types show it:
///
/// * **A pair the table names no row for** — a `string`, an `array<T>`, an
///   object, `null`, a `bool` — has no arithmetic at all rather than PHP's
///   converted one, since `rule:types/conversion` has no implicit conversion for that to
///   be. That is [`no_arithmetic`].
/// * **`int ⊕ uint`** is refused outright: § 4 gives the pair no representable
///   common type, so there is nothing to return. `E0407` where it is written,
///   [`mixed_signedness`] where only the tags know.
/// * **Overflow throws**, § 4's least tradeable divergence from PHP, which is
///   what makes every integer row below a `checked_*` and not a `wrapping_*`.
///
/// Certain rows are deliberately narrower than "the operands are numbers", and
/// each is guarded rather than answered:
///
/// * The **`decimal`** rows are `rule:types/arithmetic`'s arithmetic ones and no
///   more — its `**` is `E0455` and its bit operators `E0706` — so a `decimal`
///   under an operator it grants nothing takes the refusal.
/// * The **`float`** rows are § 4's "either operand a `float`" for `+ - * / **`
///   only. `%` is left out on purpose: `nvs-codegen` lowers no `float` `%`
///   either, so refusing here is the answer that *agrees* with the statically
///   typed end, and inventing PHP's integer-modulo reading behind a `mixed`
///   would put a rule in this file that no ADR states.
fn value_arith(op: ArithRow, left: Value, right: Value) -> Result<Value, Fault> {
    match (left.tag(), right.tag()) {
        (Some(Tag::Int), Some(Tag::Int)) => signed_arith(
            op,
            left.as_int()
                .ok_or_else(|| wrong_tag("nvs_value_arith", Tag::Int, left))?,
            right
                .as_int()
                .ok_or_else(|| wrong_tag("nvs_value_arith", Tag::Int, right))?,
        ),
        (Some(Tag::Uint), Some(Tag::Uint)) => unsigned_arith(
            op,
            left.as_uint()
                .ok_or_else(|| wrong_tag("nvs_value_arith", Tag::Uint, left))?,
            right
                .as_uint()
                .ok_or_else(|| wrong_tag("nvs_value_arith", Tag::Uint, right))?,
        ),
        (Some(Tag::Int), Some(Tag::Uint)) | (Some(Tag::Uint), Some(Tag::Int)) => {
            Err(mixed_signedness())
        }
        (Some(Tag::Decimal), Some(Tag::Decimal | Tag::Int | Tag::Uint))
        | (Some(Tag::Int | Tag::Uint), Some(Tag::Decimal))
            if matches!(
                op,
                ArithRow::Add | ArithRow::Sub | ArithRow::Mul | ArithRow::Div | ArithRow::Mod
            ) =>
        {
            decimal_arith(op, left, right)
        }
        (Some(Tag::Int | Tag::Uint | Tag::Float), Some(Tag::Int | Tag::Uint | Tag::Float))
            if matches!(
                op,
                ArithRow::Add | ArithRow::Sub | ArithRow::Mul | ArithRow::Div | ArithRow::Pow
            ) =>
        {
            float_arith(op, left, right)
        }
        _ => Err(no_arithmetic(op, left, right)),
    }
}

/// The catchable throw [`value_arith`] raises for a pair `rule:types/arithmetic`
/// tabulates no row for, naming the spelling that says what was meant wherever
/// there is one — the same shape [`no_ordering`] takes for its own table.
fn no_arithmetic(op: ArithRow, left: Value, right: Value) -> Fault {
    // One hint, for the one operand PHP would have converted silently: ADR
    // 0007 § 2 has no implicit conversion, so a numeric-looking `string` is
    // where an author is told to say `as int` out loud. Every other operand —
    // an `array<T>`, an object, `null` — has no arithmetic to name at all.
    let hint = if matches!(left.tag(), Some(Tag::Str)) || matches!(right.tag(), Some(Tag::Str)) {
        " — convert the operand out loud first: `... as int`/`as float`"
    } else {
        ""
    };
    Fault::thrown(format!(
        "no `{}` for a `{}` and a `{}`{hint}",
        op.spelling(),
        tag_name(left),
        tag_name(right)
    ))
}

/// `rule:types/arithmetic`'s `int ⊕ uint` row, which is a *compile* error wherever the
/// static types show it — [`nvs_types::expr::operators::report_int_uint`]'s
/// `E0407`, whose wording this is, because it is the same refusal made at the
/// first moment two `mixed` operands make it answerable.
fn mixed_signedness() -> Fault {
    Fault::thrown(
        "`int` and `uint` have no representable common type in arithmetic — convert one side \
         explicitly with `as int`/`as uint`"
            .to_owned(),
    )
}

/// `rule:types/arithmetic`'s overflow throw, worded exactly as `nvs-codegen`'s
/// `raise_arithmetic_error` words the statically typed row's, and of the same
/// class: a tagged operand reaching this row and a typed one reaching that
/// one are the same operation, so one `catch (ArithmeticError)` sees both.
fn overflowed(op: ArithRow) -> Fault {
    Fault::thrown_as(
        crate::ThrownClass::Arithmetic,
        format!("Integer {} overflowed", op.word()),
    )
}

/// The `int ⊕ int` rows. Every one that can leave the type is `checked_*`:
/// § 4's "no wrap, no promotion to `float`" is what this table exists to keep.
fn signed_arith(op: ArithRow, left: i64, right: i64) -> Result<Value, Fault> {
    let value = match op {
        ArithRow::Add => left.checked_add(right).ok_or_else(|| overflowed(op))?,
        ArithRow::Sub => left.checked_sub(right).ok_or_else(|| overflowed(op))?,
        ArithRow::Mul => left.checked_mul(right).ok_or_else(|| overflowed(op))?,
        // `int / int` is `int|float`, PHP-exact, so it answers a value rather
        // than an `i64` — see [`signed_div`].
        ArithRow::Div => return signed_div(left, right),
        ArithRow::Mod => {
            if right == 0 {
                return Err(Fault::thrown_as(
                    crate::ThrownClass::Arithmetic,
                    "Modulo by zero",
                ));
            }
            // `i64::MIN % -1` is `0` rather than an overflow, which is PHP 8's
            // answer and the identity `nvs-codegen`'s `emit_int_mod` rewrites
            // the divisor for.
            if right == -1 { 0 } else { left % right }
        }
        ArithRow::Pow => return signed_pow(left, right).map(Value::int),
        ArithRow::BitAnd => left & right,
        ArithRow::BitOr => left | right,
        ArithRow::BitXor => left ^ right,
        ArithRow::Shl | ArithRow::Shr => return signed_shift(op, left, right),
    };
    Ok(Value::int(value))
}

/// `rule:types/arithmetic`'s `int / int` row: `int|float`, "PHP-exact — `6/3` is an
/// integer, `7/2` is a float", which is why the quotient's *type* is a runtime
/// question and this returns a tagged value.
///
/// `i64::MIN / -1` takes the inexact arm rather than a second throw: it is not
/// an integer at all, and PHP answers the `float` this computes.
/// `nvs-codegen`'s `emit_int_div` is the same three guards in Cranelift.
fn signed_div(left: i64, right: i64) -> Result<Value, Fault> {
    if right == 0 {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Arithmetic,
            "Division by zero",
        ));
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "the inexact arm is exactly what `float` means here — `rule:types/arithmetic`'s `int|float`"
    )]
    let inexact = Ok(Value::float(left as f64 / right as f64));
    if left == i64::MIN && right == -1 {
        return inexact;
    }
    if left % right == 0 {
        return Ok(Value::int(left / right));
    }
    inexact
}

/// `rule:types/arithmetic`'s `int ** int` row, square-and-multiply with the overflow throw
/// checked at every step — `nvs-codegen`'s `emit_int_pow` in Rust, down to its
/// details: the square is not taken after the last set bit, and a
/// negative exponent throws except over a base of `1` or `-1`, which do have an
/// integer answer.
fn signed_pow(base: i64, exponent: i64) -> Result<i64, Fault> {
    if exponent < 0 {
        return match base {
            1 => Ok(1),
            -1 => Ok(if exponent % 2 == 0 { 1 } else { -1 }),
            _ => Err(Fault::thrown(
                "Negative exponent has no integer result".to_owned(),
            )),
        };
    }
    let mut accumulator: i64 = 1;
    let mut square = base;
    let mut left = exponent.cast_unsigned();
    loop {
        if left & 1 == 1 {
            accumulator = accumulator
                .checked_mul(square)
                .ok_or_else(|| overflowed(ArithRow::Pow))?;
        }
        left >>= 1;
        if left == 0 {
            return Ok(accumulator);
        }
        square = square
            .checked_mul(square)
            .ok_or_else(|| overflowed(ArithRow::Pow))?;
    }
}

/// `rule:types/arithmetic`'s `<<`/`>>` over an `int`, whose count PHP judges where the
/// machine masks it — `nvs-codegen`'s `emit_shift` in Rust, and its rules
/// unchanged: a negative count throws, a count of 64 or more answers all-zeros
/// or all-sign, and `>>` is arithmetic on an `int`.
fn signed_shift(op: ArithRow, left: i64, count: i64) -> Result<Value, Fault> {
    if count < 0 {
        return Err(Fault::thrown("Bit shift by negative number".to_owned()));
    }
    // `checked_*` is the past-the-width test as well as the shift: a count of
    // 64 or more answers `None`, and the saturated value each row fills with is
    // all-zeros for `<<` and all-sign for `>>`.
    let count = u32::try_from(count).unwrap_or(u32::MAX);
    let value = match op {
        ArithRow::Shl => left.checked_shl(count).unwrap_or(0),
        _ => left.checked_shr(count).unwrap_or(left >> 63),
    };
    Ok(Value::int(value))
}

/// The `uint ⊕ uint` rows — [`signed_arith`]'s, minus the asymmetries an
/// unsigned type does not have: no count can be negative, and `**` has no
/// negative exponent, so neither carries a guard.
fn unsigned_arith(op: ArithRow, left: u64, right: u64) -> Result<Value, Fault> {
    let value = match op {
        ArithRow::Add => left.checked_add(right).ok_or_else(|| overflowed(op))?,
        ArithRow::Sub => left.checked_sub(right).ok_or_else(|| overflowed(op))?,
        ArithRow::Mul => left.checked_mul(right).ok_or_else(|| overflowed(op))?,
        ArithRow::Div => return unsigned_div(left, right),
        ArithRow::Mod => {
            if right == 0 {
                return Err(Fault::thrown_as(
                    crate::ThrownClass::Arithmetic,
                    "Modulo by zero",
                ));
            }
            left % right
        }
        ArithRow::Pow => unsigned_pow(left, right)?,
        ArithRow::BitAnd => left & right,
        ArithRow::BitOr => left | right,
        ArithRow::BitXor => left ^ right,
        // No negative count exists on this row, so unlike [`signed_shift`] it
        // needs no guard — only the past-the-width saturation, which is
        // all-zeros in both directions for an unsigned operand.
        ArithRow::Shl => left
            .checked_shl(u32::try_from(right).unwrap_or(u32::MAX))
            .unwrap_or(0),
        ArithRow::Shr => left
            .checked_shr(u32::try_from(right).unwrap_or(u32::MAX))
            .unwrap_or(0),
    };
    Ok(Value::uint(value))
}

/// `rule:types/arithmetic`'s `uint / uint` row — [`signed_div`]'s `uint|float`, with no
/// overflow arm, an unsigned type having no asymmetric minimum.
fn unsigned_div(left: u64, right: u64) -> Result<Value, Fault> {
    if right == 0 {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Arithmetic,
            "Division by zero",
        ));
    }
    if left.is_multiple_of(right) {
        return Ok(Value::uint(left / right));
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "the inexact arm is exactly what `float` means here — `rule:types/arithmetic`'s `uint|float`"
    )]
    Ok(Value::float(left as f64 / right as f64))
}

/// [`signed_pow`]'s loop over the unsigned row.
fn unsigned_pow(base: u64, exponent: u64) -> Result<u64, Fault> {
    let mut accumulator: u64 = 1;
    let mut square = base;
    let mut left = exponent;
    loop {
        if left & 1 == 1 {
            accumulator = accumulator
                .checked_mul(square)
                .ok_or_else(|| overflowed(ArithRow::Pow))?;
        }
        left >>= 1;
        if left == 0 {
            return Ok(accumulator);
        }
        square = square
            .checked_mul(square)
            .ok_or_else(|| overflowed(ArithRow::Pow))?;
    }
}

/// `rule:types/arithmetic`'s "either operand a `float`" row.
///
/// The integer side widens through [`row::int_to_float`], which is the *checked*
/// widening § 2 names as the language's one implicit conversion — exact, or
/// throwing above 2^53 — because that is the widening `nvs_ir`'s
/// `Lowering::widen_to_float` emits for the statically typed spelling of the
/// same row. A silent `as f64` here would answer a pair one representation
/// down from the one the compiler would have.
///
/// Division by zero **is** a throw on this row, and for that same reason: ADR
/// 0007 § 4 refuses the zero divisor before the operand types are consulted, so
/// `nvs-codegen`'s statically typed `float /` guards it too and the two ends of
/// the row cannot answer differently. `Core\Math::fdiv` is the member that says
/// IEEE's infinity out loud.
fn float_arith(op: ArithRow, left: Value, right: Value) -> Result<Value, Fault> {
    let operand = |value: Value| match value.tag() {
        Some(Tag::Float) => value
            .as_float()
            .ok_or_else(|| wrong_tag("nvs_value_arith", Tag::Float, value)),
        Some(Tag::Int) => {
            let int = value
                .as_int()
                .ok_or_else(|| wrong_tag("nvs_value_arith", Tag::Int, value))?;
            row::int_to_float(int)
                .ok_or_else(|| numeric_does_not_fit(&format!("`int` {int}"), "float"))
        }
        _ => {
            let uint = value
                .as_uint()
                .ok_or_else(|| wrong_tag("nvs_value_arith", Tag::Uint, value))?;
            row::uint_to_float(uint)
                .ok_or_else(|| numeric_does_not_fit(&format!("`uint` {uint}"), "float"))
        }
    };
    let (left, right) = (operand(left)?, operand(right)?);
    // `== 0.0` and not a bit test, so a `-0.0` divisor throws as well: the
    // static guard is an `fcmp Equal` against zero and this is the same
    // question. A `NaN` divisor is not zero and divides to `NAN`.
    if matches!(op, ArithRow::Div) && right == 0.0 {
        return Err(Fault::thrown_as(
            crate::ThrownClass::Arithmetic,
            "Division by zero",
        ));
    }
    Ok(Value::float(match op {
        ArithRow::Add => left + right,
        ArithRow::Sub => left - right,
        ArithRow::Mul => left * right,
        ArithRow::Div => left / right,
        // The one row that leaves the compiled function statically too, and it
        // leaves it for the same symbol: there is no `fpow` instruction, so
        // `nvs-codegen` calls exactly this function for `float ** float`.
        _ => crate::arith::nvs_float_pow(left, right),
    }))
}

/// `rule:types/arithmetic`'s arithmetic rows behind a `mixed`, over
/// [`decimal_operand`]'s one promotion of an `int`/`uint` operand — the same
/// implementation the statically typed `decimal` helpers use, so the two ends
/// of the row cannot answer differently.
fn decimal_arith(op: ArithRow, left: Value, right: Value) -> Result<Value, Fault> {
    let left = decimal_operand("nvs_value_arith", left)?;
    let right = decimal_operand("nvs_value_arith", right)?;
    match op {
        ArithRow::Add => left.checked_add(right),
        ArithRow::Sub => left.checked_sub(right),
        ArithRow::Mul => left.checked_mul(right),
        ArithRow::Div => left.checked_div(right),
        _ => left.checked_rem(right),
    }
    .map(Value::decimal)
    .ok_or_else(|| arithmetic_error(op.word()))
}

macro_rules! value_arith_helper {
    ($(#[$meta:meta])* fn $name:ident = $row:ident) => {
        crate::nvs_helper! {
            $(#[$meta])*
            fn $name(_ctx, args: [2]) {
                value_arith(ArithRow::$row, args[0], args[1])
            }
        }
    };
}

value_arith_helper! {
    /// `nvs_ir::Helper::ValueAdd` — `+` where at least one operand's static
    /// type named no row, so the two tags name it instead. The table is
    /// [`value_arith`], and every helper below is that same table asked a
    /// different row.
    fn nvs_value_add = Add
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueSub` — see [`value_arith`].
    fn nvs_value_sub = Sub
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueMul` — see [`value_arith`].
    fn nvs_value_mul = Mul
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueDiv` — see [`value_arith`]. The one row whose
    /// answer's *tag* is a runtime question even once the operands' are known:
    /// `rule:types/arithmetic` types integer division `int|float`.
    fn nvs_value_div = Div
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueMod` — see [`value_arith`].
    fn nvs_value_mod = Mod
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValuePow` — see [`value_arith`].
    fn nvs_value_pow = Pow
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueBitAnd` — see [`value_arith`]. `rule:types/arithmetic`'s
    /// `& | ^ << >>` row is `int` and `uint` alone, so every other pair of tags
    /// is the refusal rather than PHP's converted answer.
    fn nvs_value_bit_and = BitAnd
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueBitOr` — see [`nvs_value_bit_and`].
    fn nvs_value_bit_or = BitOr
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueBitXor` — see [`nvs_value_bit_and`].
    fn nvs_value_bit_xor = BitXor
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueShl` — see [`nvs_value_bit_and`].
    fn nvs_value_shl = Shl
}
value_arith_helper! {
    /// `nvs_ir::Helper::ValueShr` — see [`nvs_value_bit_and`], and
    /// [`signed_shift`] for the signedness `>>` reads off the tag.
    fn nvs_value_shr = Shr
}

/// `rule:types/arithmetic`'s **unary** rows, chosen from one runtime tag rather than from
/// a static type — [`value_arith`]'s one-operand twin, and the last shape of
/// that table an erased operand had no answer for.
///
/// `-` is over the numeric types and no more. The integer rows are
/// `checked_neg` for the table's own reason: `-i64::MIN` has no `int` and every
/// non-zero `uint` has no negation at all, so § 4's overflow throw reaches the
/// unary row too, worded exactly as `nvs-codegen`'s `emit_unop` words the
/// statically typed spelling's. `float` and `decimal` cannot fail — a float's
/// sign bit is one flip, and `rule:types/decimal`'s mantissa is unsigned, so there is no
/// asymmetric minimum to overflow. Every other tag is the closed table's
/// refusal, which is [`no_unary`].
fn value_neg(value: Value) -> Result<Value, Fault> {
    match value.tag() {
        Some(Tag::Int) => value
            .as_int()
            .ok_or_else(|| wrong_tag("nvs_value_neg", Tag::Int, value))?
            .checked_neg()
            .map(Value::int)
            .ok_or_else(negation_overflowed),
        Some(Tag::Uint) => value
            .as_uint()
            .ok_or_else(|| wrong_tag("nvs_value_neg", Tag::Uint, value))?
            .checked_neg()
            .map(Value::uint)
            .ok_or_else(negation_overflowed),
        Some(Tag::Float) => {
            Ok(Value::float(-value.as_float().ok_or_else(|| {
                wrong_tag("nvs_value_neg", Tag::Float, value)
            })?))
        }
        Some(Tag::Decimal) => Ok(Value::decimal(
            decimal_operand("nvs_value_neg", value)?.negated(),
        )),
        _ => Err(no_unary("-", value)),
    }
}

/// `rule:types/arithmetic`'s `~` row behind a `mixed`, which is narrower than
/// [`value_neg`]'s by exactly the rows `& | ^ << >>` is narrower than the
/// arithmetic ones by: the bit operators are over `int` and `uint` alone, so a
/// `float` or a `decimal` operand is a number with no bit pattern to
/// complement. It is total over the rows it does have — every 64-bit
/// pattern is a value of each — so unlike `-` it cannot overflow.
fn value_bit_not(value: Value) -> Result<Value, Fault> {
    match value.tag() {
        Some(Tag::Int) => {
            Ok(Value::int(!value.as_int().ok_or_else(|| {
                wrong_tag("nvs_value_bit_not", Tag::Int, value)
            })?))
        }
        Some(Tag::Uint) => {
            Ok(Value::uint(!value.as_uint().ok_or_else(|| {
                wrong_tag("nvs_value_bit_not", Tag::Uint, value)
            })?))
        }
        _ => Err(no_unary("~", value)),
    }
}

/// `rule:types/arithmetic`'s negation overflow, worded as `nvs-codegen`'s `emit_unop`
/// words the statically typed row's so the two ends cannot drift apart.
fn negation_overflowed() -> Fault {
    Fault::thrown("Integer negation overflowed".to_owned())
}

/// The catchable throw the unary rows raise for an operand `rule:types/arithmetic`
/// tabulates no row for — [`no_arithmetic`]'s shape with one operand, carrying
/// the reading `nvs_types::expr::operators::reject_unary_arith_operand` gives
/// the same refusal (`E0705`, and `E0706` for the rows `~` leaves out) wherever
/// the static type shows it.
fn no_unary(spelling: &str, value: Value) -> Fault {
    // The one operand PHP would have converted silently, and [`no_arithmetic`]'s
    // reason for naming it: `rule:types/conversion` has no implicit conversion, so a
    // numeric-looking `string` is where an author is told to say `as int` out
    // loud.
    let hint = if matches!(value.tag(), Some(Tag::Str)) {
        " — convert the operand out loud first: `... as int`/`as float`"
    } else {
        ""
    };
    Fault::thrown(format!(
        "no unary `{spelling}` for a `{}`{hint}",
        tag_name(value)
    ))
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueNeg` — `-` where the operand's static type named
    /// no row, so its tag names it instead. See [`value_neg`].
    fn nvs_value_neg(_ctx, args: [1]) {
        value_neg(args[0])
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ValueBitNot` — `~` where the operand's static type
    /// named no row. See [`value_bit_not`].
    fn nvs_value_bit_not(_ctx, args: [1]) {
        value_bit_not(args[0])
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::SecretEq` — `==` where the checker typed at least one
    /// operand `secret`, which
    /// `rule:security/secret-comparison-is-constant-time`
    /// makes a **constant-time** comparison rather than the
    /// short-circuiting one `nvs_str_eq` performs for every other
    /// `string`/`bytes` pair.
    ///
    /// "Constant-time" means what it means in `Core\Hash::equals`: the
    /// running time does not depend on *where* two equal-length operands
    /// first differ, so an attacker holding one of them cannot recover the
    /// other a byte at a time by timing the answer. It does not hide the
    /// lengths — a length mismatch answers `false` at once, because
    /// `subtle::ConstantTimeEq` is defined over equal-length slices and
    /// because a token's length is not the secret.
    ///
    /// `subtle`, not a hand-written loop, for the reason `nvs-stdlib`'s
    /// `hash` module states: a compiler is free to reintroduce the branch a
    /// hand-written loop was written to avoid. This crate cannot call
    /// `nvs-stdlib`, so the dependency is named here too rather than the
    /// comparison being shared.
    ///
    /// Takes a `string` **or** a `bytes` on either side — the two tags share
    /// one allocation, and `rule:security/secret-qualifier` puts the qualifier
    /// on both bases — and a `mixed` on the side the checker did not call
    /// `secret`, which arrives carrying whatever tag it holds. A payload that
    /// is not a buffer of the other side's own tag answers `false`, which is
    /// what [`crate::value_identical`] answers for the same pair: a `bytes` is
    /// a value rather than a handle and compares only against another `bytes`,
    /// and no other tag has a buffer to read at all. Reading a tag leaks
    /// nothing the timing property protects — the tag is not the secret — and
    /// the branch is taken before a byte of either operand is touched.
    ///
    /// The row is total, so this carries no error edge; `!=` is this helper
    /// under an `nvs_ir::UnOp::Not`, the arrangement [`nvs_numeric_eq`] uses.
    fn nvs_secret_eq(_ctx, args: [2]) {
        let (Some(lhs), Some(rhs)) = (args[0].buffer_ptr(), args[1].buffer_ptr()) else {
            return Ok(Value::bool(false));
        };
        if args[0].tag() != args[1].tag() {
            return Ok(Value::bool(false));
        }
        #[expect(
            unsafe_code,
            reason = "a Tag::Str or Tag::Bytes argument owns a reference to a live \
                      allocation, so both pointees are live for this read; both \
                      borrows end with the comparison"
        )]
        let equal = unsafe {
            let left = crate::string::NvsStr::bytes_of(lhs);
            let right = crate::string::NvsStr::bytes_of(rhs);
            left.len() == right.len() && bool::from(left.ct_eq(right))
        };
        Ok(Value::bool(equal))
    }
}

/// The [`Fault::Thrown`] a checked conversion produces when the value does not
/// fit — `rule:types/conversion`'s "`as` ... either produces a value of the target type or
/// throws. It never rounds, truncates, or substitutes a default."
///
/// Spec § 10's `RuntimeError` — "the world said no" — for a string that is not
/// a number and for a `mixed` whose tag is not the target's at all. A
/// *numeric* value the target has no room for is [`numeric_does_not_fit`].
fn does_not_fit(what: &str, target: &str) -> Fault {
    Fault::thrown(format!("cannot convert {what} to `{target}`"))
}

/// [`does_not_fit`] for a number the target type has no room for — an `int`
/// past 2^53 into `float`, a `float` with a fraction into `int`, a `uint` past
/// `int::MAX` — which is the overflow `rule:types/arithmetic` names `ArithmeticError`,
/// and the class `crate::callable`'s identical widening check already raises.
/// Same wording as the non-numeric case, so a diagnostic quoting one quotes
/// both.
fn numeric_does_not_fit(what: &str, target: &str) -> Fault {
    Fault::thrown_as(
        crate::ThrownClass::Arithmetic,
        format!("cannot convert {what} to `{target}`"),
    )
}

/// `rule:types/conversion`'s checked conversion rows, one function each, answering `None`
/// exactly where the row fails.
///
/// Every row has two entry points and never a third: the statically chosen
/// helper below it, which turns a `None` into [`does_not_fit`], and the
/// tag-dispatching [`to_int`]/[`to_uint`]/[`to_float`], which every operand
/// whose representation is `nvs_ir::ty::Ty::Tagged` reaches instead. Each of
/// those is read twice — once throwing, once answering `null` for
/// `rule:expressions/nullable-conversion`'s
/// `expr as ?T`. That ADR's "one implementation now exists because there is one
/// operation" is what this split makes true rather than promised — the throwing
/// and the nullable form cannot drift apart, because there is one row.
mod row {
    /// The magnitude past which an `f64` no longer represents every integer —
    /// `rule:types/conversion`'s 2^53 boundary, shared by both integer-to-`float` rows.
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

    /// `rule:types/conversion`: integral and in range, or the row fails. The
    /// scale is not the question — `7.00` is integral and `7.25` is not — so a
    /// column declared `DECIMAL(10, 2)` converts wherever its value has
    /// nothing after the point.
    pub(super) fn decimal_to_int(value: super::Decimal) -> Option<i64> {
        value.to_i64()
    }

    /// [`decimal_to_int`]'s row, unsigned.
    pub(super) fn decimal_to_uint(value: super::Decimal) -> Option<u64> {
        value.to_u64()
    }

    /// `rule:types/conversion`'s `decimal → float` row: the nearest `f64`,
    /// lossy and total, so this row is the one that never fails.
    pub(super) fn decimal_to_float(value: super::Decimal) -> f64 {
        value.to_f64()
    }

    pub(super) fn str_to_int(text: &str) -> Option<i64> {
        text.parse().ok()
    }

    pub(super) fn str_to_uint(text: &str) -> Option<u64> {
        text.parse().ok()
    }

    /// `f64::from_str` accepts `inf`/`nan`/`infinity` in any case; none is an
    /// "exact numeric literal", so each is refused here rather than becoming a
    /// value no numeric literal in the source could have written.
    pub(super) fn str_to_float(text: &str) -> Option<f64> {
        text.parse::<f64>().ok().filter(|value| value.is_finite())
    }

    /// `value` as an exact integer, or `None` if it is not integral, is not
    /// finite, or is too large for the `i128` both integer targets fit inside.
    ///
    /// `rule:types/conversion`: "integral and in range, or throws. Rounding is
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

crate::nvs_helper! {
    /// `nvs_ir::Helper::IntToUint`.
    fn nvs_int_to_uint(_ctx, args: [1]) {
        let value = expect_tag!("nvs_int_to_uint", args[0], as_int, Tag::Int);
        row::int_to_uint(value)
            .map(Value::uint)
            .ok_or_else(|| numeric_does_not_fit(&format!("`int` {value}"), "uint"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::UintToInt`.
    fn nvs_uint_to_int(_ctx, args: [1]) {
        let value = expect_tag!("nvs_uint_to_int", args[0], as_uint, Tag::Uint);
        row::uint_to_int(value)
            .map(Value::int)
            .ok_or_else(|| numeric_does_not_fit(&format!("`uint` {value}"), "int"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::IntToFloat`.
    fn nvs_int_to_float(_ctx, args: [1]) {
        let value = expect_tag!("nvs_int_to_float", args[0], as_int, Tag::Int);
        row::int_to_float(value)
            .map(Value::float)
            .ok_or_else(|| numeric_does_not_fit(&format!("`int` {value}"), "float"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::UintToFloat`.
    fn nvs_uint_to_float(_ctx, args: [1]) {
        let value = expect_tag!("nvs_uint_to_float", args[0], as_uint, Tag::Uint);
        row::uint_to_float(value)
            .map(Value::float)
            .ok_or_else(|| numeric_does_not_fit(&format!("`uint` {value}"), "float"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::FloatToInt`.
    fn nvs_float_to_int(_ctx, args: [1]) {
        let value = expect_tag!("nvs_float_to_int", args[0], as_float, Tag::Float);
        row::float_to_int(value)
            .map(Value::int)
            .ok_or_else(|| numeric_does_not_fit(&format!("`float` {value}"), "int"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::FloatToUint`.
    fn nvs_float_to_uint(_ctx, args: [1]) {
        let value = expect_tag!("nvs_float_to_uint", args[0], as_float, Tag::Float);
        row::float_to_uint(value)
            .map(Value::uint)
            .ok_or_else(|| numeric_does_not_fit(&format!("`float` {value}"), "uint"))
    }
}

/// The one shape `rule:types/conversion`'s `string` → number row accepts: the *whole*
/// string, with no surrounding whitespace, no leading `+`-and-garbage rule, and
/// no PHP-style prefix parse. Returned as `&str` so each caller can hand it to
/// the standard library's own exact parser.
fn numeric_text<'a>(bytes: &'a [u8], helper: &'static str, value: Value) -> Result<&'a str, Fault> {
    str::from_utf8(bytes).map_err(|_| wrong_tag(helper, Tag::Str, value))
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::StrToInt`.
    fn nvs_str_to_int(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("nvs_str_to_int", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "nvs_str_to_int", args[0])?;
        row::str_to_int(text)
            .map(Value::int)
            .ok_or_else(|| does_not_fit(&format!("string {text:?}"), "int"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::StrToUint`.
    fn nvs_str_to_uint(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("nvs_str_to_uint", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "nvs_str_to_uint", args[0])?;
        row::str_to_uint(text)
            .map(Value::uint)
            .ok_or_else(|| does_not_fit(&format!("string {text:?}"), "uint"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::StrToFloat`.
    fn nvs_str_to_float(_ctx, args: [1]) {
        let bytes = args[0]
            .as_str_bytes()
            .ok_or_else(|| wrong_tag("nvs_str_to_float", Tag::Str, args[0]))?;
        let text = numeric_text(bytes, "nvs_str_to_float", args[0])?;
        row::str_to_float(text)
            .map(Value::float)
            .ok_or_else(|| does_not_fit(&format!("string {text:?}"), "float"))
    }
}

/// The operand's bytes as text, for the two `string` rows — `None` where a
/// `Tag::Str` payload is not UTF-8, which is a failed conversion for
/// `rule:expressions/nullable-conversion`'s form rather than the miscompile [`numeric_text`] reports.
fn str_operand(value: &Value) -> Option<&str> {
    str::from_utf8(value.as_str_bytes()?).ok()
}

/// `rule:types/conversion`'s `→ int` rows, chosen by the operand's **runtime** tag —
/// `None` where the row fails *or* where no row exists at all.
///
/// Dispatching on the tag is why one function covers every source. That is not
/// a shortcut: it is what makes `rule:expressions/nullable-conversion`'s "a `null` operand yields `null`"
/// and § 3's "from `mixed` every target has a checked path" the same code as
/// `"42" as ?int`, with no branch in lowering and no second implementation of
/// any row (see [`row`]). A tag `rule:types/conversion` defines no row from — an array, an
/// object, a `bool` — is a conversion that does not exist, which § 3 makes a
/// compile error for a statically-known operand and a failure for a `mixed`
/// one.
///
/// Two helpers read it inside this crate, the arrangement [`to_decimal`] also
/// uses: `nvs_tagged_to_int` turns a `None` into [`does_not_fit`] for
/// `$mixed as int`, and `nvs_to_int_or_null` turns the same `None` into `null`
/// for `rule:expressions/nullable-conversion`'s `as ?int`. Neither can drift from the other, because there is
/// one row set.
///
/// Public because a third reader is a `Core` member converting a value it was
/// handed rather than one a call site wrote: `nvs_stdlib::json`'s hydration
/// walk applies these rows per field for `Core\Arr::shapeAs`, whose whole
/// conversion rule is `rule:types/conversion`'s table and nothing of its own.
/// It reaches the same row set for the same reason — a member with its own
/// idea of what `"42"` is would be a second conversion table.
#[must_use]
pub fn to_int(value: Value) -> Option<i64> {
    match value.tag() {
        Some(Tag::Int) => value.as_int(),
        Some(Tag::Uint) => value.as_uint().and_then(row::uint_to_int),
        Some(Tag::Float) => value.as_float().and_then(row::float_to_int),
        Some(Tag::Decimal) => value.as_decimal().and_then(row::decimal_to_int),
        Some(Tag::Str) => str_operand(&value).and_then(row::str_to_int),
        _ => None,
    }
}

/// [`to_int`]'s row set, unsigned — public for its reason.
#[must_use]
pub fn to_uint(value: Value) -> Option<u64> {
    match value.tag() {
        Some(Tag::Uint) => value.as_uint(),
        Some(Tag::Int) => value.as_int().and_then(row::int_to_uint),
        Some(Tag::Float) => value.as_float().and_then(row::float_to_uint),
        Some(Tag::Decimal) => value.as_decimal().and_then(row::decimal_to_uint),
        Some(Tag::Str) => str_operand(&value).and_then(row::str_to_uint),
        _ => None,
    }
}

/// [`to_int`]'s row set, landing on `float` — public for its reason.
#[must_use]
pub fn to_float(value: Value) -> Option<f64> {
    match value.tag() {
        Some(Tag::Float) => value.as_float(),
        Some(Tag::Int) => value.as_int().and_then(row::int_to_float),
        Some(Tag::Uint) => value.as_uint().and_then(row::uint_to_float),
        Some(Tag::Decimal) => value.as_decimal().map(row::decimal_to_float),
        Some(Tag::Str) => str_operand(&value).and_then(row::str_to_float),
        _ => None,
    }
}

/// `rule:types/conversion`'s one *implicit* conversion — an `int` or `uint` arriving in a
/// `float` position — as a value-to-value row, for the one caller that cannot
/// reach it through a lowered `as`.
///
/// `crate::callable::check_param_tags` is that caller: a `callable` carries no
/// parameter list (`rule:types/anonymous-function`), so no checker ever saw the call site and
/// nothing inserted the widening conversion the declared `float` earns. It is
/// applied there instead, out of the same [`row`] set every written `as float`
/// goes through, so the 2^53 boundary cannot drift between the two spellings.
///
/// `None` where that row refuses — the magnitude past which an `f64` stops
/// representing every integer — and for any other tag, which the caller has
/// already excluded.
pub(crate) fn widen_to_float(value: Value) -> Option<Value> {
    match value.tag() {
        Some(Tag::Int) => value.as_int().and_then(row::int_to_float).map(Value::float),
        Some(Tag::Uint) => value
            .as_uint()
            .and_then(row::uint_to_float)
            .map(Value::float),
        _ => None,
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::TaggedToInt` — `rule:types/conversion`'s checked `as int` over an
    /// operand whose representation is `nvs_ir::ty::Ty::Tagged`, so [`to_int`]'s
    /// `None` is the throw rather than a `null`.
    fn nvs_tagged_to_int(_ctx, args: [1]) {
        to_int(args[0])
            .map(Value::int)
            .ok_or_else(|| does_not_fit("this value", "int"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::TaggedToUint` — [`nvs_tagged_to_int`]'s row set,
    /// unsigned; see [`to_uint`].
    fn nvs_tagged_to_uint(_ctx, args: [1]) {
        to_uint(args[0])
            .map(Value::uint)
            .ok_or_else(|| does_not_fit("this value", "uint"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::TaggedToFloat` — [`nvs_tagged_to_int`]'s row set,
    /// landing on `float`; see [`to_float`].
    fn nvs_tagged_to_float(_ctx, args: [1]) {
        to_float(args[0])
            .map(Value::float)
            .ok_or_else(|| does_not_fit("this value", "float"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::TaggedWidenToFloat` — a value stored at a union that
    /// names `float` and not the value's integer type. An `int` or `uint` is
    /// converted as [`nvs_int_to_float`] and [`nvs_uint_to_float`] convert it,
    /// and throws above 2^53. Every other value is returned as it is: the
    /// result is the operand's own reference, so a refcounted payload is
    /// neither retained nor released here.
    fn nvs_tagged_widen_to_float(_ctx, args: [1]) {
        let value = args[0];
        match value.tag() {
            Some(Tag::Int) => {
                let n = value.as_int().unwrap_or_default();
                row::int_to_float(n)
                    .map(Value::float)
                    .ok_or_else(|| numeric_does_not_fit(&format!("`int` {n}"), "float"))
            }
            Some(Tag::Uint) => {
                let n = value.as_uint().unwrap_or_default();
                row::uint_to_float(n)
                    .map(Value::float)
                    .ok_or_else(|| numeric_does_not_fit(&format!("`uint` {n}"), "float"))
            }
            _ => Ok(value),
        }
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToIntOrNull` — `rule:expressions/nullable-conversion`'s non-throwing form of
    /// [`nvs_tagged_to_int`], sharing [`to_int`]'s one implementation of every
    /// row.
    fn nvs_to_int_or_null(_ctx, args: [1]) {
        Ok(to_int(args[0]).map_or_else(Value::null, Value::int))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToUintOrNull` — see [`to_uint`].
    fn nvs_to_uint_or_null(_ctx, args: [1]) {
        Ok(to_uint(args[0]).map_or_else(Value::null, Value::uint))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToFloatOrNull` — see [`to_float`].
    fn nvs_to_float_or_null(_ctx, args: [1]) {
        Ok(to_float(args[0]).map_or_else(Value::null, Value::float))
    }
}

/// `rule:types/conversion`'s
/// scalar-to-`string` rows, applied to a value whose representation is
/// `nvs_ir::ty::Ty::Tagged` — a `mixed`, a `?T`, or any other union.
///
/// The counterpart of [`value_truthy`], and for the same reason: compiled code
/// that *knows* its operand is an `int` reaches `nvs_int_to_string` with no tag
/// test at all, so this is only the case where the static type genuinely does
/// not say which row applies. It dispatches on the tag the `Value` already
/// carries, which is what makes `"a" . $mixed` one helper rather than one
/// lowering branch per possible source — the same "one tag per target, not one
/// per (source, target) pair" rule [`to_int`] already follows.
///
/// **The tags that convert to nothing each throw** rather than producing
/// PHP's `"Array"`-plus-warning: `rule:core-api/shape-rules`
/// R4 makes failure a throw, and a silent placeholder is exactly the class of
/// answer `rule:types/conversion` removed from the language. An **object** is among them
/// here, but this is the *tag* table and not the whole rule: `rule:classes/stringable` makes
/// `Stringable` the one way an object renders, and [`fn@stringify`] is where that
/// dispatch happens — every helper reaches this function through that one, so an
/// object only falls to the row below once its runtime class has been asked for
/// a `toString` and answered nothing. The other exception is a **sink carrier**,
/// which renders as the bytes it carries; the row itself says why that is not
/// `Stringable` in disguise.
///
/// A `Tag::Str` operand is returned as itself with one **fresh** reference, so
/// the caller owns the result exactly as it owns a converted one; every other
/// row builds a new string, which carries its one reference already.
pub fn value_to_string(value: Value) -> Result<Value, Fault> {
    let refused = |what: &str| Fault::thrown(format!("cannot convert {what} to `string`"));
    match value.tag() {
        Some(Tag::Null) | None => Ok(Value::str(NvsStr::new(b""))),
        Some(Tag::Bool) => {
            let set = value.as_bool() == Some(true);
            Ok(Value::str(NvsStr::new(if set {
                b"1".as_slice()
            } else {
                b"".as_slice()
            })))
        }
        Some(Tag::Int | Tag::EnumInt) => {
            let n = value.as_int().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(NvsStr::new(n.to_string().as_bytes())))
        }
        Some(Tag::Uint | Tag::EnumUint) => {
            let n = value.as_uint().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(NvsStr::new(n.to_string().as_bytes())))
        }
        Some(Tag::Float) => {
            let n = value.as_float().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(NvsStr::new(php_float_to_string(n).as_bytes())))
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
                crate::string::nvs_str_retain(ptr);
            }
            Ok(value)
        }
        // `rule:types/conversion`'s `decimal → string` row: total, and scale-preserving,
        // so `19.90` renders as `"19.90"` — `crate::decimal`'s `Display` is
        // the one implementation of it.
        Some(Tag::Decimal) => {
            let value = value.as_decimal().ok_or_else(|| refused("this value"))?;
            Ok(Value::str(NvsStr::new(value.to_string().as_bytes())))
        }
        Some(Tag::Array) => Err(refused("an array")),
        // Refused on purpose, and it is the only tag here that is refused for
        // a *semantic* reason rather than a missing one: `rule:types/conversion` makes
        // `bytes as string` a checked conversion that validates UTF-8, so
        // letting an implicit `.` or `echo` do it silently would be exactly
        // the substitution that ADR exists to remove.
        Some(Tag::Bytes) => Err(refused("a `bytes` value")),
        // `rule:security/capture-answers-the-carrier`'s carrier is the one object that renders, and it renders
        // as exactly the bytes it carries: they have *already* been through the
        // sink, so anything else here would put them through it twice. This is
        // not `rule:classes/no-magic-methods`'s `Stringable` and does not re-open it — a carrier is
        // the sink's own value type, `crate::ctx::is_carrier` is the whole
        // roster, and every other object still fails below.
        Some(Tag::Object) => {
            let ptr = value.obj_ptr().ok_or_else(|| refused("this value"))?;
            #[expect(
                unsafe_code,
                reason = "a Tag::Object value's payload is a live allocation the \
                          caller owns a reference to, so its class and its first \
                          slot are readable for the length of this call"
            )]
            let (class, carried) = unsafe {
                let name = (*crate::object::NvsObj::class_of(ptr)).name();
                let carried = if crate::ctx::is_carrier(name) {
                    Some(crate::object::nvs_object_field_get(
                        ptr,
                        crate::ctx::CARRIER_TEXT_SLOT,
                    ))
                } else {
                    None
                };
                (name, carried)
            };
            // The one throw here that names what it was handed, because it is
            // the one a program reaches by writing a class rather than by
            // reaching a tag no row covers: [`stringify`] has already asked
            // this class for its `toString` and been answered nothing, so what
            // the message owes is which class and which member.
            let Some(carried) = carried else {
                return Err(Fault::thrown(format!(
                    "cannot convert an object of class `{class}` to `string`: it does not \
                     implement `Stringable`"
                )));
            };
            let text = carried
                .str_ptr()
                .ok_or_else(|| Fault::fatal("a sink carrier holds no `string` in its text slot"))?;
            #[expect(
                unsafe_code,
                reason = "the carrier's slot owns the reference this borrowed read \
                          returned, so the caller needs one of its own"
            )]
            unsafe {
                crate::string::nvs_str_retain(text);
            }
            Ok(carried)
        }
        // Not a row: `Tag::Unset` is `rule:classes/an-unwritten-property-read-throws`'s storage state and never a
        // value, every read that could hand one out turning it into a throw
        // first (`crate::nvs_object_slot_get`, and `nvs_ir::lower`'s guard on
        // the compiled read). So an arrival is one of those readers having
        // been bypassed, which is a compiler bug rather than a conversion
        // this table refuses.
        Some(Tag::Unset) => Err(Fault::fatal(
            "internal error: a never-written property slot reached a conversion",
        )),
    }
}

/// The method `rule:classes/stringable` fixes as the one way an object renders. The spelling is the interface's,
/// and `nvs_types::expr::operators::require_stringable` resolves the *static*
/// half of the same name.
const TO_STRING: &str = "toString";

/// [`value_to_string`] with `rule:classes/stringable`'s dispatch in front of it: an object
/// whose **runtime** class declares a `toString` renders through that method,
/// and everything else takes the tag row.
///
/// This is the half of the rule no checker can take. `nvs_types` resolves
/// `toString` wherever the operand's static type names a class, and `nvs-ir`
/// then emits an ordinary call — nothing on that path reaches here. What is
/// left is every operand whose static type names *no* class to resolve against:
/// a `mixed`, a `?T` or another union, and the erased `object` of
/// `rule:types/erased-member-access`, whose
/// receiver "cannot be checked at compile time at all" and whose answer is
/// therefore decided by the concrete instance behind the handle. Refusing those
/// instead would diverge from PHP — `function f(object $o) { echo $o; }` calls
/// `__toString` there — and would also make one value render two different ways
/// depending on which binding it was read through, which is the worse half.
///
/// A class that declares no `toString` still throws, and throws the same
/// catchable `Throwable` the tag row already answers with: `rule:types/erased-member-access`'s read
/// through an erased view is "a checked, catchable throw", never a fatal.
///
/// **A `Core`-owned class renders here too, and through the same member.** Its
/// `toString` is native Rust rather than a compiled function, so it is not in
/// the method table this dispatches through — it is on the descriptor as
/// [`crate::ClassDesc::renderer`], put there by `nvs_stdlib::instance` from
/// the very registry row `nvs_types::expr::operators::require_stringable`
/// reads to decide the *static* spelling. That is what makes `echo $m` over a
/// `mixed` holding a `Core\Uri` answer what `echo $uri` answers: one
/// implementation, reached two ways, rather than two rosters free to disagree.
///
/// **Ownership is the callee's convention, and the two differ.** A compiled
/// method owns its parameters, so [`crate::call_method`] retains the receiver
/// on the way in and the callee's own exit sweep releases it; a native
/// renderer is an `rule:errors/propagation` helper and borrows argument 0, so
/// [`crate::dispatch::call_render`] does neither. Either way the result is the
/// `string` that member returned, carrying the one reference every other row
/// here hands back.
///
/// # Errors
///
/// [`Fault::Pending`] when the `toString` body itself throws or faults, so the
/// exception it recorded in `ctx` reaches the request unchanged; otherwise
/// whatever [`value_to_string`] answers for the tag.
pub fn stringify(ctx: &mut crate::Ctx, value: Value) -> Result<Value, Fault> {
    if value.tag() == Some(Tag::Object) {
        const WHAT: &str = "an implicit `toString` conversion";
        // The native half first, because it is one load and a null test, and
        // because the two rosters are disjoint by construction: a `Core` class
        // has no compiled method table and a declared one carries no renderer.
        if let Some(text) = crate::dispatch::call_render(ctx, value, WHAT)? {
            return Ok(text);
        }
        let rendered = crate::dispatch::call_method(ctx, value, TO_STRING, &[], WHAT)?;
        if let Some(text) = rendered {
            return Ok(text);
        }
    }
    value_to_string(value)
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::TaggedToString` — see [`fn@stringify`], and
    /// [`value_to_string`] for the tag table under it.
    fn nvs_tagged_to_string(ctx, args: [1]) {
        stringify(ctx, args[0])
    }
}

/// `rule:types/conversion`'s explicit `as string` over a tagged operand — a
/// `mixed`, a `?T`, any other union.
///
/// [`fn@stringify`]'s rows plus the one row an explicit conversion has and an
/// implicit one does not: a [`Tag::Bytes`] operand is [`bytes_to_string`]'s
/// checked UTF-8 validation, exactly as the statically typed `bytes as string`
/// is. `.` and `echo` stay on [`fn@stringify`], which refuses `bytes` because a
/// conversion nobody wrote must not validate anything.
///
/// # Errors
///
/// [`fn@stringify`]'s and [`bytes_to_string`]'s, unchanged.
pub fn convert_to_string(ctx: &mut crate::Ctx, value: Value) -> Result<Value, Fault> {
    if value.tag() == Some(Tag::Bytes) {
        bytes_to_string(value)
    } else {
        stringify(ctx, value)
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::TaggedAsString` — see [`convert_to_string`].
    fn nvs_tagged_as_string(ctx, args: [1]) {
        convert_to_string(ctx, args[0])
    }
}

/// `rule:expressions/nullable-conversion`'s
/// `as ?string`: [`convert_to_string`]'s answer, with `null` exactly where that
/// one throws.
///
/// One implementation of `rule:types/conversion`'s `→ string` rows and not a second copy
/// of them, which is the whole point of § 1's "yields `null` exactly where
/// `expr as T` would throw" — a twin that decided any row for itself could
/// disagree with the checked spelling on that row.
///
/// **Only the conversion's own failure becomes `null`.** A [`Fault::Thrown`]
/// raised here means this function had no answer; every other fault is
/// something that happened *while* it looked — a `toString()` body that threw
/// ([`Fault::Pending`], the exception already recorded in `ctx`), or an engine
/// fault — and passing those off as `null` would swallow a program's own
/// exception at a conversion that never asked about it.
///
/// # Errors
///
/// Every fault but [`Fault::Thrown`], unchanged.
pub fn stringify_or_null(ctx: &mut crate::Ctx, value: Value) -> Result<Value, Fault> {
    match convert_to_string(ctx, value) {
        Ok(text) => Ok(text),
        Err(Fault::Thrown(..)) => Ok(Value::null()),
        Err(other) => Err(other),
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToStringOrNull` — see [`stringify_or_null`].
    fn nvs_to_string_or_null(ctx, args: [1]) {
        stringify_or_null(ctx, args[0])
    }
}

/// `rule:types/conversion`'s
/// `bytes as string` row: **checked**, and the one direction of that pair that
/// runs any code at all.
///
/// The buffer is validated as well-formed UTF-8 and either produces the
/// `string` or throws. It is never replaced, dropped or truncated — `iconv`'s
/// `//IGNORE` and `//TRANSLIT` are exactly the silent substitution that ADR
/// exists to remove, so the failure names the offset rather than repairing it.
///
/// **Validation is the whole cost.** A `string` and a `bytes` are one heap
/// allocation under two tags ([`Tag::Bytes`]), so a buffer that passes is
/// handed back as the same pointer with a `Tag::Str` and one **fresh**
/// reference — the same ownership [`value_to_string`]'s own `Tag::Str` row
/// gives, so the caller owns the result exactly as it owns a converted one.
/// Nothing is copied and nothing is allocated; the O(n) walk over the octets
/// is all this spends.
///
/// The other direction never reaches a helper: `string as bytes` is total and
/// free, so `nvs-ir` lowers it to an `InstKind::Reinterpret` and no call is
/// emitted.
pub fn bytes_to_string(value: Value) -> Result<Value, Fault> {
    let bytes = value
        .as_bytes()
        .ok_or_else(|| wrong_tag("nvs_bytes_to_string", Tag::Bytes, value))?;
    if let Err(invalid) = str::from_utf8(bytes) {
        return Err(Fault::thrown(format!(
            "cannot convert `bytes` to `string`: not well-formed UTF-8 at byte {}",
            invalid.valid_up_to()
        )));
    }
    let ptr = value
        .buffer_ptr()
        .ok_or_else(|| wrong_tag("nvs_bytes_to_string", Tag::Bytes, value))?;
    #[expect(
        unsafe_code,
        reason = "a Tag::Bytes value's payload is a live allocation the caller \
                  owns a reference to, and the result carries a second one the \
                  caller will release"
    )]
    let retagged = unsafe {
        crate::string::nvs_str_retain(ptr);
        NvsStr::from_raw(ptr)
    };
    Ok(Value::str(retagged))
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::BytesToStr` — see [`bytes_to_string`].
    fn nvs_bytes_to_string(_ctx, args: [1]) {
        bytes_to_string(args[0])
    }
}

/// `rule:types/conversion`'s pair the other
/// way, applied to a value whose representation is `nvs_ir::ty::Ty::Tagged` — a
/// `mixed`, a `?T`, or any other union.
///
/// The *statically* typed half of `string as bytes` never reaches a helper at
/// all: it is total and free, so `nvs-ir` lowers it to an
/// `InstKind::Reinterpret` and emits no call ([`bytes_to_string`] says so from
/// the other side). What is left is the operand whose static type names no row,
/// where the tag the `Value` already carries is the only thing that does — the
/// same "one tag per target" arrangement [`to_int`] and [`value_to_string`]
/// already follow.
///
/// Only the buffer tags have a row, and both are free. A [`Tag::Str`] *is* the
/// buffer a `bytes` is, minus the UTF-8 promise, so it is handed back under the
/// other tag over the same allocation; a [`Tag::Bytes`] is `rule:types/conversion`'s
/// identical-type row, which converts nothing anywhere it is written. Both
/// answer with one **fresh** reference, so the caller owns the result exactly as
/// it owns a converted one, and neither copies an octet.
///
/// `None` for every other tag — `rule:types/conversion`'s table produces a `bytes` from a
/// `string` and from nothing else, so an `int`, an array or an object has no
/// answer here rather than a lossy one. Rendering one through
/// [`value_to_string`] on the way would be exactly the implicit conversion that
/// ADR exists to remove.
fn to_bytes(value: Value) -> Option<Value> {
    let ptr = match value.tag() {
        Some(Tag::Str | Tag::Bytes) => value.buffer_ptr()?,
        _ => return None,
    };
    #[expect(
        unsafe_code,
        reason = "a Tag::Str or Tag::Bytes value's payload is a live allocation \
                  the caller owns a reference to, and the result carries a \
                  second one the caller will release"
    )]
    let retagged = unsafe {
        crate::string::nvs_str_retain(ptr);
        NvsStr::from_raw(ptr)
    };
    Some(Value::bytes(retagged))
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::TaggedToBytes` — `rule:types/conversion`'s checked `as bytes` over
    /// a tagged operand, so [`to_bytes`]'s `None` is the throw rather than a
    /// `null`.
    fn nvs_tagged_to_bytes(_ctx, args: [1]) {
        to_bytes(args[0]).ok_or_else(|| does_not_fit("this value", "bytes"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToBytesOrNull` — `rule:expressions/nullable-conversion`'s non-throwing form of
    /// [`nvs_tagged_to_bytes`], sharing [`to_bytes`]'s one implementation of
    /// both rows. Nothing here can fault on the way, so unlike
    /// [`stringify_or_null`] there is no exception of the program's own to keep
    /// out of the `null`.
    fn nvs_to_bytes_or_null(_ctx, args: [1]) {
        Ok(to_bytes(args[0]).unwrap_or_else(Value::null))
    }
}

/// One element of an `array<U>` target, judged against the level of
/// `nvs_ir::lower::array_element_tags`' word that describes it.
///
/// The nibble *is* a [`Tag`] byte, which is what lets this share
/// [`crate::callable::CALLABLE_PARAM_TAG_ANY`] with the callable-entry check
/// rather than inventing a second encoding: both ask "does this value carry
/// the tag that representation carries", and neither can ask anything narrower
/// in four bits. `Tag::Array` is the one nibble that continues — the rest of
/// the word describes the elements of *this* element, which is how
/// `array<array<int>>` is checked all the way down.
///
/// No widening, unlike the callable-entry check: an `int` element does not
/// have the tag `array<float>` describes. [`convert_element`] is what turns
/// one into a `float`, in the copy [`to_array_of`] makes for it.
fn element_has_tag(value: Value, tags: u64) -> bool {
    let nibble = u8::try_from(tags & 0xf).unwrap_or(u8::MAX);
    if nibble == crate::callable::CALLABLE_PARAM_TAG_ANY {
        return true;
    }
    let (Some(required), Some(given)) = (Tag::from_byte(nibble), value.exact_tag()) else {
        return false;
    };
    if given != required {
        return false;
    }
    if required != Tag::Array {
        return true;
    }
    let Some(inner) = value.array_ptr() else {
        return false;
    };
    #[expect(
        unsafe_code,
        reason = "a Tag::Array value's payload is a live allocation the array \
                  holding it owns a reference to, so it is live for this walk"
    )]
    unsafe {
        every_element_has_tag(inner, tags >> 4)
    }
}

/// [`element_has_tag`] over every entry of one array, in insertion order —
/// `rule:types/conversion`'s O(n) element walk itself.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "the array pointer's liveness is the caller's to guarantee and \
              the signature cannot express it"
)]
unsafe fn every_element_has_tag(array: *mut crate::array::ArrayHeader, tags: u64) -> bool {
    let mut from = 0usize;
    loop {
        #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
        let slot = unsafe { crate::array::nvs_array_next_slot(array, from) };
        let Ok(slot) = usize::try_from(slot) else {
            return true;
        };
        let mut value = Value::null();
        #[expect(
            unsafe_code,
            reason = "the slot is one `nvs_array_next_slot` just returned and \
                      `value` is a writable 16-byte slot"
        )]
        unsafe {
            crate::array::nvs_array_value_at(array, slot, &raw mut value);
        }
        // Borrowed, so there is nothing to release: `nvs_array_value_at`
        // hands back the entry's own `Value` and the array still owns it.
        if !element_has_tag(value, tags) {
            return false;
        }
        from = slot + 1;
    }
}

/// `rule:types/conversion`'s `array<T> as array<U>` row, shared by its throwing and its
/// `null`-answering spelling exactly as [`to_bytes`] shares that pair's.
///
/// `Err(None)` for an operand that is not an array at all —
/// `rule:types/unions-and-mixed`'s `mixed` reaching this row — and for one
/// whose walk found an element `U` does not admit. `Err(Some(fault))` for an
/// `int` or `uint` element that is not exact as a `float`.
///
/// **Where every element already has `U`'s tag, the result is the operand's
/// own allocation under one more reference.** The buffer can stay shared,
/// because a Novis array is copy-on-write and whichever of the two views
/// writes first separates itself ([`crate::array`]'s `make_unique`).
///
/// **Where an `int` or `uint` element meets a `float` in `U`, the result is a
/// new array** ([`converted_array`]): the elements are stored without a
/// conversion of their own, so converting one means rewriting it, and the
/// operand's other owners still read the `int`. That costs one array of the
/// operand's size, per level that holds such an element.
fn to_array_of(value: Value, tags: Value) -> Result<Value, Option<Fault>> {
    let tags = tags.as_uint().ok_or(None)?;
    let array = value.array_ptr().ok_or(None)?;
    #[expect(
        unsafe_code,
        reason = "a Tag::Array value's payload is a live allocation the caller \
                  owns a reference to, and the result carries a reference of its \
                  own the caller will release"
    )]
    unsafe {
        converted_array(array, tags)
    }
}

/// `array`'s elements at the level `tags` describes, as [`to_array_of`]
/// gives them: `array` itself under one more reference where every element
/// has its tag, and otherwise a copy with each element [`convert_element`]
/// converts.
///
/// Where the running request cannot afford the copy, the answer is `array`
/// under one more reference. [`crate::budget::affords`] has recorded the
/// breach by then, so the request is over and allocates nothing more.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "the array pointer's liveness is the caller's to guarantee and \
              the signature cannot express it"
)]
unsafe fn converted_array(
    array: *mut crate::array::ArrayHeader,
    tags: u64,
) -> Result<Value, Option<Fault>> {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let unchanged = unsafe { every_element_has_tag(array, tags) };
    if !unchanged {
        // A borrowed handle: the caller owns the reference, so this one must
        // not release it when it goes out of scope.
        #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
        let source = std::mem::ManuallyDrop::new(unsafe { crate::NvsArray::from_raw(array) });
        if let Some(copy) = source.map_values(|value| convert_element(value, tags))? {
            return Ok(Value::array(copy));
        }
    }
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    unsafe {
        crate::array::nvs_array_retain(array);
    }
    Ok(Value::from_array_ptr(array))
}

/// One element of [`converted_array`]'s walk: `Ok(None)` where it already has
/// the tag the level of `tags` describes, `Ok(Some(v))` with the converted
/// element where it can be converted, and the [`to_array_of`] error where it
/// cannot.
///
/// An `int` or `uint` meets a `float` the way `$n as float` does: exact, or
/// the error above 2^53. An array meets an array one level down.
fn convert_element(value: Value, tags: u64) -> Result<Option<Value>, Option<Fault>> {
    if element_has_tag(value, tags) {
        return Ok(None);
    }
    let target = u8::try_from(tags & 0xf).ok().and_then(Tag::from_byte);
    match (target, value.tag()) {
        (Some(Tag::Float), Some(Tag::Int)) => {
            let n = value.as_int().ok_or(None)?;
            row::int_to_float(n)
                .map(|f| Some(Value::float(f)))
                .ok_or_else(|| Some(numeric_does_not_fit(&format!("`int` {n}"), "float")))
        }
        (Some(Tag::Float), Some(Tag::Uint)) => {
            let n = value.as_uint().ok_or(None)?;
            row::uint_to_float(n)
                .map(|f| Some(Value::float(f)))
                .ok_or_else(|| Some(numeric_does_not_fit(&format!("`uint` {n}"), "float")))
        }
        (Some(Tag::Array), Some(Tag::Array)) => {
            let inner = value.array_ptr().ok_or(None)?;
            #[expect(
                unsafe_code,
                reason = "a Tag::Array element's payload is a live allocation the \
                          array holding it owns a reference to"
            )]
            unsafe {
                converted_array(inner, tags >> 4).map(Some)
            }
        }
        _ => Err(None),
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToArrayOf` — `rule:types/conversion`'s `array<T> as array<U>`
    /// row, so [`to_array_of`]'s error is the throw rather than a `null`.
    ///
    /// The two ways `Err(None)` arises are told apart here rather than inside
    /// the shared walk, because they are two different mistakes: an operand
    /// that is not an array at all is `rule:types/unions-and-mixed`'s `mixed` holding something
    /// else, and it reads as every other row's refusal does; an element the
    /// target's `U` does not admit is the row's *own* failure and says so.
    /// Which element is not named, for the reason
    /// `nvs_ir::lower::Lowering::lower_checked_downcast`'s message does not
    /// name a class: the walk compares tags, and a key would have to be
    /// rendered to be quoted. `Err(Some(fault))` is an `int` element above
    /// 2^53, and throws what `$n as float` throws for it.
    fn nvs_to_array_of(_ctx, args: [2]) {
        if args[0].tag() != Some(Tag::Array) {
            return Err(does_not_fit("this value", "array"));
        }
        to_array_of(args[0], args[1]).map_err(|fault| {
            fault.unwrap_or_else(|| {
                Fault::thrown(
                    "an element of this array is not of the element type it is converted to"
                        .to_owned(),
                )
            })
        })
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToArrayOfOrNull` — `rule:expressions/nullable-conversion`'s non-throwing form of
    /// [`nvs_to_array_of`], over [`to_array_of`]'s one implementation of the
    /// walk. Every error of that walk is the `null`, the `int` above 2^53
    /// included, because `$n as ?float` gives `null` for it too.
    fn nvs_to_array_of_or_null(_ctx, args: [2]) {
        Ok(to_array_of(args[0], args[1]).unwrap_or_else(|_| Value::null()))
    }
}

/// `rule:types/arithmetic`'s
/// `ArithmeticError`: an overflow of either kind, or a zero divisor.
fn arithmetic_error(operation: &str) -> Fault {
    Fault::thrown_as(
        crate::ThrownClass::Arithmetic,
        format!(
            "`decimal` {operation} is outside the type's range (`rule:types/decimal`: a 96-bit \
             mantissa at a scale of 0 to 28), or divides by zero"
        ),
    )
}

/// One arithmetic or comparison operand as a [`Decimal`].
///
/// An `int` or a `uint` operand is promoted here rather than by a conversion
/// instruction in lowering, which is what makes `rule:types/arithmetic`'s
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
        crate::nvs_helper! {
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
    /// `nvs_ir::Helper::DecimalAdd`.
    fn nvs_decimal_add = checked_add, "addition"
}
decimal_arithmetic! {
    /// `nvs_ir::Helper::DecimalSub`.
    fn nvs_decimal_sub = checked_sub, "subtraction"
}
decimal_arithmetic! {
    /// `nvs_ir::Helper::DecimalMul`.
    fn nvs_decimal_mul = checked_mul, "multiplication"
}
decimal_arithmetic! {
    /// `nvs_ir::Helper::DecimalDiv`.
    fn nvs_decimal_div = checked_div, "division"
}
decimal_arithmetic! {
    /// `nvs_ir::Helper::DecimalMod`.
    fn nvs_decimal_mod = checked_rem, "remainder"
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalNeg` — `-$d`, which never fails: the mantissa
    /// is unsigned, so there is no asymmetric minimum to overflow the way
    /// `-i64::MIN` does.
    fn nvs_decimal_neg(_ctx, args: [1]) {
        Ok(Value::decimal(decimal_operand("nvs_decimal_neg", args[0])?.negated()))
    }
}

/// `rule:types/arithmetic`'s comparison row: a `decimal` against a `decimal`, an `int`, a
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

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalEq` — see [`decimal_ordering`]. `!=` is this
    /// helper under an `nvs_ir::UnOp::Not`, which is also what gives a `NaN`
    /// operand PHP's answer at every comparison operator.
    fn nvs_decimal_eq(_ctx, args: [2]) {
        Ok(Value::bool(decimal_ordering(args[0], args[1]).is_some_and(std::cmp::Ordering::is_eq)))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalLt` — `>` is this helper with its operands
    /// swapped.
    fn nvs_decimal_lt(_ctx, args: [2]) {
        Ok(Value::bool(decimal_ordering(args[0], args[1]).is_some_and(std::cmp::Ordering::is_lt)))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalLtEq` — `>=` is this helper with its operands
    /// swapped.
    fn nvs_decimal_lt_eq(_ctx, args: [2]) {
        Ok(Value::bool(decimal_ordering(args[0], args[1]).is_some_and(std::cmp::Ordering::is_le)))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalCmp` — `<=>` with a `decimal` operand, which is
    /// [`nvs_decimal_lt`]'s row read whole. `rule:types/arithmetic` grants it across
    /// every pairing, including the `decimal`/`float` one arithmetic refuses.
    fn nvs_decimal_cmp(_ctx, args: [2]) {
        Ok(Value::int(spaceship(decimal_ordering(args[0], args[1]))))
    }
}

/// The `int` an ordering answers `<=>` with, shared by the two helpers above
/// and matching what `nvs_ir::BinOp::Cmp` emits inline for a matched pair.
///
/// [`None`] — an unordered pair, which means a `NaN` on one side — is **`1`**,
/// not `0`. That is PHP's own answer, and it is the whole reason this is a
/// three-way match rather than the arithmetic `(a > b) - (a < b)`: the latter
/// reads an unordered pair as *equal*, which is the one thing it certainly is
/// not.
fn spaceship(ordering: Option<core::cmp::Ordering>) -> i64 {
    match ordering {
        Some(core::cmp::Ordering::Less) => -1,
        Some(core::cmp::Ordering::Equal) => 0,
        _ => 1,
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalTruthy` — `rule:expressions/truthy-positions`'s numeric row: falsy iff
    /// zero, at any scale.
    fn nvs_decimal_truthy(_ctx, args: [1]) {
        Ok(Value::bool(!decimal_operand("nvs_decimal_truthy", args[0])?.is_zero()))
    }
}

/// `rule:types/conversion`'s `→ decimal` rows, chosen by the operand's **runtime**
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

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToDecimal` — see [`to_decimal`].
    fn nvs_to_decimal(_ctx, args: [1]) {
        to_decimal(args[0])
            .map(Value::decimal)
            .ok_or_else(|| does_not_fit("this value", "decimal"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::ToDecimalOrNull` — `rule:expressions/nullable-conversion`'s non-throwing form
    /// of [`to_decimal`], sharing its one implementation of every row exactly
    /// as `nvs_to_int_or_null` shares [`row`]'s.
    fn nvs_to_decimal_or_null(_ctx, args: [1]) {
        Ok(to_decimal(args[0]).map_or_else(Value::null, Value::decimal))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalToInt` — `rule:types/conversion`: integral and in range,
    /// or throws. Rounding is `Core\Decimal::floor`/`ceil`/`round`, said out
    /// loud, exactly as `float → int` already is.
    fn nvs_decimal_to_int(_ctx, args: [1]) {
        let value = decimal_operand("nvs_decimal_to_int", args[0])?;
        row::decimal_to_int(value)
            .map(Value::int)
            .ok_or_else(|| numeric_does_not_fit(&format!("`decimal` {value}"), "int"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalToUint` — [`nvs_decimal_to_int`]'s row,
    /// unsigned.
    fn nvs_decimal_to_uint(_ctx, args: [1]) {
        let value = decimal_operand("nvs_decimal_to_uint", args[0])?;
        row::decimal_to_uint(value)
            .map(Value::uint)
            .ok_or_else(|| numeric_does_not_fit(&format!("`decimal` {value}"), "uint"))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalToFloat` — the nearest `f64`, lossy and total.
    fn nvs_decimal_to_float(_ctx, args: [1]) {
        Ok(Value::float(row::decimal_to_float(decimal_operand(
            "nvs_decimal_to_float",
            args[0],
        )?)))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::DecimalToString` — total, and scale-preserving.
    fn nvs_decimal_to_string(_ctx, args: [1]) {
        let value = decimal_operand("nvs_decimal_to_string", args[0])?;
        Ok(Value::str(NvsStr::new(value.to_string().as_bytes())))
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::EchoStr` — an operand that cannot be a carrier, so the
    /// sink in force transforms it whatever it holds.
    ///
    /// **Which transform that is belongs to [`write_rendered`]**, which reads
    /// it off `rule:tooling/echo-always-has-a-sink`'s table: inside an HTTP
    /// request a `Tag::Str` is data interpolated into a page and takes
    /// `rule:core-classes/html-auto-escape`'s escape, and everywhere else it
    /// takes the terminal's substitution, which is the rest of this comment.
    ///
    /// `rule:tooling/terminal-output-is-a-sink` is
    /// the rule and [`nvs_render::text::substitute`] is the table, called rather
    /// than restated: `ESC` becomes `␛`, a bare `CR` becomes `␍`, `DEL` becomes
    /// `␡`, a C1 code point and an unterminated bidirectional control both
    /// become `�`, and `LF` and `TAB` pass through. It fires **regardless of
    /// qualifier and regardless of whether the stream is a terminal** — that
    /// section's *uniform, not* paragraphs own why, and the short version is
    /// that a rule whose effect depends on a fact invisible at the `echo` line
    /// is a worse implicit than the uniform one.
    ///
    /// Nothing visible is lost, which is what makes a non-optional default
    /// affordable: the bytes replaced here are commands the terminal consumes
    /// and shows to nobody, so substituting them makes `echo` show *more* of
    /// what arrived rather than less. That asymmetry with
    /// `rule:core-classes/html-auto-escape`'s HTML `&`→`&amp;` is stated in 0086's *Context*.
    ///
    /// The table is idempotent — a Control Picture is not a control byte — so
    /// output that has already passed a sink, `Core\Out::capture`'s buffer most
    /// of all, is unchanged by a second write. A **carrier** does not arrive
    /// here at all: `echo` of one takes [`nvs_echo_value`], which is the one
    /// raw path either sink has and the reason this helper never has to ask
    /// what its operand used to be.
    ///
    /// **Ill-formed UTF-8 goes through `from_utf8_lossy` first.** A `Tag::Str`
    /// is UTF-8 by `rule:types/bytes`, so
    /// this is unreachable from a well-formed value; where a test reaches it
    /// anyway, `�` is the answer the table already gives a byte that is never
    /// legitimate text, and a raw control byte cannot survive the pass. Reading
    /// the operand as bytes rather than as [`Value::as_text`] is what keeps that
    /// a substitution rather than undefined behaviour.
    ///
    /// Ordinary text — nearly every write — costs one scan and no allocation:
    /// both `from_utf8_lossy` and `substitute` answer the borrow.
    fn nvs_echo_str(ctx, args: [1]) {
        write_rendered(ctx, "nvs_echo_str", args[0], Raw::No)?;
        Ok(Value::null())
    }
}

/// Whether a write to the sink carries bytes the sink must leave alone.
///
/// A two-case enum rather than a `bool` because the call sites read
/// `Raw::No` / `Raw::Yes` and a bare `false` at a sink is the kind of argument
/// that gets flipped by a refactor without anyone noticing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Raw {
    /// Ordinary text: the sink's own transform runs over it.
    No,
    /// Bytes that came out of this sink's carrier, written through unchanged.
    Yes,
}

/// Writes one already-rendered operand to the request's output — the single
/// place `echo` turns a `Tag::Str` into bytes on the stream.
///
/// Both [`nvs_echo_str`] and [`nvs_echo_value`] reach the stream through this,
/// so the two spellings of `echo` cannot come to write different bytes for the
/// same text; the only thing that differs between them is which [`Raw`] they
/// pass, and that is decided from the operand's *class* rather than from its
/// bytes. `who` names the caller so a wrong-tag refusal still says which helper
/// was handed what.
///
/// **Which transform runs is the sink's, not the operand's.**
/// `rule:tooling/echo-always-has-a-sink`'s table is a column of carriers and a
/// column of renderings, and [`crate::Ctx::carrier`] is already the first of
/// them — so the second is read off the same answer rather than off a second
/// switch that could come to disagree with it. An HTTP request's sink escapes
/// (`rule:core-classes/html-auto-escape`), every other sink substitutes
/// (`rule:tooling/terminal-output-is-a-sink`), and neither is a rule any call
/// site states.
fn write_rendered(
    ctx: &mut crate::Ctx,
    who: &'static str,
    rendered: Value,
    raw: Raw,
) -> Result<(), Fault> {
    let bytes = rendered
        .as_str_bytes()
        .ok_or_else(|| wrong_tag(who, Tag::Str, rendered))?;
    let carrier = ctx.carrier();
    match raw {
        Raw::Yes => ctx.write_output(bytes),
        Raw::No => {
            let text = String::from_utf8_lossy(bytes);
            let written = if carrier == crate::CARRIER_HTML_MARKUP {
                nvs_render::html::escape(&text)
            } else {
                nvs_render::text::substitute(&text)
            };
            ctx.write_output(written.as_bytes())
        }
    }
    .map_err(|error| Fault::fatal(format!("could not write output: {error}")))
}

/// Whether `value` is the carrier **of the sink it is about to be written
/// to**, which is the one shape that sink must leave alone.
///
/// [`is_carrier_value`] asks whether a value is a carrier at all; this asks
/// whether it is *this* one's, and the difference is the whole of what keeps a
/// sink's raw path its own. A `Cli\Text` echoed inside an HTTP request holds
/// terminal escapes that mean nothing to a browser and everything to whoever
/// reads the log the page ends up in, so it takes
/// `rule:core-classes/html-auto-escape`'s escape like any other non-`Markup`
/// value; a `Markup` echoed from a CLI program is the mirror case and takes
/// the terminal's substitution. Neither is a special case in the table — each
/// is what "the sink decides the carrier" says when the two disagree.
fn writes_raw(ctx: &crate::Ctx, value: Value) -> bool {
    let Some(ptr) = value.obj_ptr() else {
        return false;
    };
    #[expect(
        unsafe_code,
        reason = "a Tag::Object value's payload is a live allocation the caller \
                  owns a reference to, so its class is readable for the length \
                  of this call"
    )]
    unsafe {
        (*crate::object::NvsObj::class_of(ptr)).name() == ctx.carrier()
    }
}

/// Whether `value` is a **sink carrier** at all — [`writes_raw`]'s question
/// asked of the roster instead of of one sink, which is what a member deciding
/// whether it was handed one needs.
///
/// `rule:tooling/terminal-output-is-a-sink` puts
/// exactly one raw path in the language and § 2 makes it a *type*,
/// `Core\Cli\Text`, rather than a member or a bit riding on a string. Both of
/// that type's constructors apply § 1's substitution to their own input, so the
/// only control bytes a carrier can hold are the ones `Cli\Style` put there,
/// and a program cannot be talked into building one that carries an injected
/// sequence. Keying the raw path on the class is what makes that structural: a
/// `raw` flag on a `Tag::Str` would leave the carrier on the first member that
/// answered one, and from then on the sink would be trusting a bit rather than
/// a constructor.
///
/// The roster is [`crate::ctx::is_carrier`]'s and not a second one — this asks
/// the question of a value where that one asks it of a name. What a carrier
/// *renders as* stays [`value_to_string`]'s own `Tag::Object` arm, so there is
/// still exactly one reader of [`crate::ctx::CARRIER_TEXT_SLOT`] in the tree.
#[must_use]
pub fn is_carrier_value(value: Value) -> bool {
    let Some(ptr) = value.obj_ptr() else {
        return false;
    };
    #[expect(
        unsafe_code,
        reason = "a Tag::Object value's payload is a live allocation the caller \
                  owns a reference to, so its class is readable for the length \
                  of this call"
    )]
    unsafe {
        crate::ctx::is_carrier((*crate::object::NvsObj::class_of(ptr)).name())
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::EchoValue` — [`nvs_echo_str`]'s sink reached one
    /// conversion earlier, so that it can recognise its own carrier before
    /// anything has turned the operand into bytes.
    ///
    /// **A sink transforms everything except its own carrier.** That is
    /// `rule:tooling/terminal-output-is-a-sink`'s "exactly one raw path,
    /// `Cli\Text`" and `rule:core-classes/html-auto-escape`'s "`Markup` is the
    /// only raw-write bypass" read literally, and they are one sentence
    /// because the table pairs each sink with one carrier: the raw path is a
    /// *type*, so it has to be recognised while the operand still has one.
    /// `nvs-ir` sends every `Ty::Object` and `Ty::Tagged` operand here for that
    /// reason — those are the static types a carrier can arrive under, and
    /// a scalar or a `Ty::Str` still takes [`nvs_echo_str`] and one helper call
    /// less. [`writes_raw`] owns why the question is asked of the class rather
    /// than of a bit travelling with the bytes, and why it is asked against
    /// *this* sink's carrier rather than against the roster.
    ///
    /// Without this, `echo Cli\Text::styled("…", $warn)` would print `␛` where
    /// the style belongs: the carrier would lower through [`value_to_string`]
    /// to a `Tag::Str` and the sink would neutralize the very bytes `Cli\Style`
    /// had just put there. `Core\Out::capture`'s bytes need no such path: they
    /// have already been through a sink, and the table's idempotence covers
    /// them.
    ///
    /// The render itself is [`fn@stringify`], unchanged and shared with `.`
    /// concatenation: `rule:classes/stringable`'s `toString` dispatch still runs for an
    /// object that declares one, and a class that renders as nothing still
    /// throws with the same sentence. What that answers is a fresh reference
    /// this helper owns and releases, exactly as `nvs-ir` would have.
    fn nvs_echo_value(ctx, args: [1]) {
        let raw = if writes_raw(ctx, args[0]) { Raw::Yes } else { Raw::No };
        let rendered = stringify(ctx, args[0])?;
        let written = write_rendered(ctx, "nvs_echo_value", rendered, raw);
        #[expect(
            unsafe_code,
            reason = "`stringify` answers a fresh reference this helper owns, \
                      and the write above is the last read of it"
        )]
        unsafe {
            rendered.release();
        }
        written?;
        Ok(Value::null())
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::EchoMarkup` — one piece of an html template written at
    /// `echo`, which is the bytes of a `Core\Html\Markup` nobody built.
    ///
    /// [`nvs_echo_value`] writes a `Markup` raw under the HTML sink and
    /// substitutes it under every other one, and this asks the same question of
    /// the sink alone, because the piece no longer has a class to ask. Under
    /// the HTML sink a segment is what the author wrote and a hole was escaped
    /// on the way in, so escaping either here would escape it twice.
    fn nvs_echo_markup(ctx, args: [1]) {
        let raw = if ctx.carrier() == crate::CARRIER_HTML_MARKUP { Raw::Yes } else { Raw::No };
        write_rendered(ctx, "nvs_echo_markup", args[0], raw)?;
        Ok(Value::null())
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::Exit` — records the process status `exit`/`exit(n)`
    /// named, then ends the request.
    ///
    /// **The one helper whose success is a non-`OK` status.** It answers
    /// [`Fault::Pending`] carrying [`crate::EXITED`], so the ordinary `rule:errors/propagation`
    /// status check `nvs-codegen` emits after the call takes the site's error
    /// edge: the frame's live locals are released in its landing block, and
    /// every caller's own check propagates the status the same way. Nothing
    /// catches it and no `finally` runs — `docs/adr/README.md`
    /// § *Decisions taken at project start* owns why that is a fourth status
    /// rather than a `FATAL` carrying a code.
    fn nvs_exit(ctx, args: [1]) {
        let code = expect_tag!("nvs_exit", args[0], as_int, Tag::Int);
        ctx.set_exit_code(code);
        Err(Fault::Pending(crate::EXITED))
    }
}

/// One operand of a failed
/// `rule:types/single-value-types`
/// membership test, rendered the way § 6's compile-time sibling renders it:
/// a `string` double-quoted, an integer bare. Only the representations a
/// set of allowed values can name reach this — `nvs-codegen` boxed the operand
/// from `Ty::Str`, `Ty::Int`, `Ty::Uint` or `Ty::Tagged` — so the last arm is
/// a value that arrived through `mixed` carrying some other tag entirely,
/// which is a miss for the same reason a wrong string is.
fn rendered_operand(value: Value) -> String {
    match value.tag() {
        Some(Tag::Str) => str_operand(&value).map_or_else(
            || "a non-UTF-8 `bytes` value".to_owned(),
            |text| format!("{text:?}"),
        ),
        Some(Tag::Int) => value.as_int().map_or_else(String::new, |n| n.to_string()),
        Some(Tag::Uint) => value.as_uint().map_or_else(String::new, |n| n.to_string()),
        // `rule:types/grammar`'s two `bool` singletons name a value each, so a miss
        // against one has a value to name back — `false`, spelled as the type
        // `true` is spelled, not "a `Bool` value".
        Some(Tag::Bool) => value
            .as_bool()
            .map_or_else(String::new, |b| if b { "true" } else { "false" }.to_owned()),
        Some(Tag::Null) | None => "null".to_owned(),
        Some(other) => format!("a `{other:?}` value"),
    }
}

crate::nvs_helper! {
    /// `nvs_ir::Helper::SingleValueMismatch` — `rule:types/single-value-types`'s membership test
    /// having missed every literal its target names, which § 4 makes a throw.
    ///
    /// **Never returns `Ok`.** The comparison chain that calls it already
    /// decided the answer; this exists to carry the message, whose accepted
    /// half `args[1]` holds already rendered — see that `nvs_ir::ir::Helper`
    /// variant for
    /// why the set is generated at lowering time rather than encoded and
    /// decoded here.
    ///
    /// **It releases `args[1]`**, which is the one place in this file a helper
    /// owns an argument rather than borrowing it. That inversion is forced:
    /// the string is a fresh `InstKind::ConstStr` with exactly one reference,
    /// and a helper that never returns leaves its caller no reachable point to
    /// release one at — an instruction emitted after this call lands in the
    /// block only an `Ok` would reach. `args[0]` keeps the ordinary
    /// convention: the operand is the conversion's own value and its caller
    /// owns it.
    fn nvs_single_value_mismatch(_ctx, args: [2]) {
        let accepted = args[1]
            .as_str_bytes()
            .and_then(|bytes| str::from_utf8(bytes).ok())
            .ok_or_else(|| wrong_tag("nvs_single_value_mismatch", Tag::Str, args[1]))?;
        let message = format!("`{}` is not one of {accepted}", rendered_operand(args[0]));
        #[expect(
            unsafe_code,
            reason = "the rendered set is a fresh `ConstStr` this call is the \
                      last reader of, and its one reference is dropped here \
                      because no reachable instruction follows a helper that \
                      never returns"
        )]
        unsafe {
            args[1].release();
        }
        Err(Fault::thrown(message))
    }
}

/// `rule:expressions/truthy-positions`'s truthy
/// table, applied to a value whose type is known only at runtime.
///
/// The per-type helpers above are what compiled code reaches wherever a static
/// type names the row: the checker already knows an `if`'s operand type, so
/// the branch is picked at compile time and there is no tag test on the hot
/// path. This is the row for everything else, and its callers arrive by
/// different routes at the same question. Native `Core` code holds
/// a [`Value`] a callable just returned, whose static type is `callable`'s
/// opaque result and therefore nothing; and compiled code holding a
/// `nvs_ir::ty::Ty::Tagged` operand — a `mixed`, a union, a `?T` no test
/// narrowed — reaches it through [`nvs_value_truthy`], which is `rule:expressions/truthy-table`'s own last table row rather than a fallback below it.
///
/// The match is over [`Value::exact_tag`], because an enum case carries its
/// own tag (`rule:enums/representation`) and is always truthy, a case backed
/// by `0` included (`rule:enums/truthiness`). [`Value::tag`] would read it as
/// that `0`.
///
/// A `Tag::Object` value is always truthy, which includes an exception and a
/// callable alike; a tag byte denoting nothing at all is falsy,
/// the same "report what can be be sure of" floor every other decoder here
/// takes.
#[must_use]
pub fn value_truthy(value: Value) -> bool {
    match value.exact_tag() {
        None | Some(Tag::Null) => false,
        Some(Tag::EnumInt | Tag::EnumUint) => true,
        Some(Tag::Bool) => value.as_bool() == Some(true),
        Some(Tag::Int) => value.as_int().is_some_and(|n| n != 0),
        Some(Tag::Uint) => value.as_uint().is_some_and(|n| n != 0),
        Some(Tag::Float) => value.as_float().is_some_and(|n| n != 0.0),
        // `rule:expressions/truthy-positions`'s numeric row, at `decimal`'s own precision: falsy iff the
        // value is zero, at any scale.
        Some(Tag::Decimal) => value.as_decimal().is_some_and(|d| !d.is_zero()),
        Some(Tag::Str) => value
            .as_str_bytes()
            .is_some_and(|bytes| !(bytes.is_empty() || bytes == b"0")),
        // Empty is falsy and everything else is truthy — deliberately *not*
        // `string`'s row. `rule:expressions/truthy-positions`'s table names no `bytes` case at all,
        // because PHP has no such type, and the one place the two rows differ
        // is `"0"`, which is PHP's numeric-string rule; a `bytes` never
        // converts to a number, so carrying that quirk over would make a
        // one-octet buffer falsy for a reason that does not apply to it.
        Some(Tag::Bytes) => value.as_bytes().is_some_and(|bytes| !bytes.is_empty()),
        Some(Tag::Array) => value.array_ptr().is_some_and(|array| {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array value's payload is a live allocation \
                          the caller owns a reference to"
            )]
            let count = unsafe { crate::array::nvs_array_count(array) };
            count != 0
        }),
        Some(Tag::Object) => true,
        // `rule:classes/an-unwritten-property-read-throws`'s storage state, which is not a value and cannot be
        // tested for truth — see `value_to_string`'s own arm for why an
        // arrival is a compiler bug. This function has no error channel, so
        // it takes the same floor a tag byte denoting nothing at all takes.
        Some(Tag::Unset) => false,
    }
}

/// Every helper this crate exports, paired with the symbol name compiled code
/// calls it by.
///
/// `nvs-codegen` registers these with `cranelift_jit::JITBuilder::symbol` and
/// maps each `nvs_ir::Helper` variant to one of the names. Returned as a
/// `Vec` of `(name, address)` rather than a `const` table because a function
/// address is not a value a `const` can hold.
#[must_use]
pub fn symbols() -> Vec<(&'static str, *const u8)> {
    fn address(function: HelperFn) -> *const u8 {
        (function as *const ()).cast::<u8>()
    }

    vec![
        ("nvs_int_to_string", address(nvs_int_to_string)),
        ("nvs_uint_to_string", address(nvs_uint_to_string)),
        ("nvs_float_to_string", address(nvs_float_to_string)),
        ("nvs_bool_to_string", address(nvs_bool_to_string)),
        ("nvs_class_desc_name", address(nvs_class_desc_name)),
        ("nvs_class_constant", address(nvs_class_constant)),
        ("nvs_int_truthy", address(nvs_int_truthy)),
        ("nvs_uint_truthy", address(nvs_uint_truthy)),
        ("nvs_float_truthy", address(nvs_float_truthy)),
        ("nvs_str_truthy", address(nvs_str_truthy)),
        ("nvs_bytes_truthy", address(nvs_bytes_truthy)),
        ("nvs_array_truthy", address(nvs_array_truthy)),
        ("nvs_value_truthy", address(nvs_value_truthy)),
        ("nvs_array_row_for_write", address(nvs_array_row_for_write)),
        ("nvs_array_required_get", address(nvs_array_required_get)),
        ("nvs_array_optional_get", address(nvs_array_optional_get)),
        ("nvs_value_to_array_key", address(nvs_value_to_array_key)),
        ("nvs_value_index_get", address(nvs_value_index_get)),
        (
            "nvs_value_index_optional_get",
            address(nvs_value_index_optional_get),
        ),
        ("nvs_value_identical", address(nvs_value_identical)),
        ("nvs_numeric_eq", address(nvs_numeric_eq)),
        ("nvs_numeric_lt", address(nvs_numeric_lt)),
        ("nvs_numeric_cmp", address(nvs_numeric_cmp)),
        ("nvs_value_lt", address(nvs_value_lt)),
        ("nvs_value_lt_eq", address(nvs_value_lt_eq)),
        ("nvs_value_cmp", address(nvs_value_cmp)),
        ("nvs_value_neg", address(nvs_value_neg)),
        ("nvs_value_bit_not", address(nvs_value_bit_not)),
        ("nvs_value_add", address(nvs_value_add)),
        ("nvs_value_sub", address(nvs_value_sub)),
        ("nvs_value_mul", address(nvs_value_mul)),
        ("nvs_value_div", address(nvs_value_div)),
        ("nvs_value_mod", address(nvs_value_mod)),
        ("nvs_value_pow", address(nvs_value_pow)),
        ("nvs_value_bit_and", address(nvs_value_bit_and)),
        ("nvs_value_bit_or", address(nvs_value_bit_or)),
        ("nvs_value_bit_xor", address(nvs_value_bit_xor)),
        ("nvs_value_shl", address(nvs_value_shl)),
        ("nvs_value_shr", address(nvs_value_shr)),
        ("nvs_numeric_lt_eq", address(nvs_numeric_lt_eq)),
        ("nvs_secret_eq", address(nvs_secret_eq)),
        ("nvs_int_to_uint", address(nvs_int_to_uint)),
        ("nvs_uint_to_int", address(nvs_uint_to_int)),
        ("nvs_int_to_float", address(nvs_int_to_float)),
        ("nvs_uint_to_float", address(nvs_uint_to_float)),
        ("nvs_float_to_int", address(nvs_float_to_int)),
        ("nvs_float_to_uint", address(nvs_float_to_uint)),
        ("nvs_str_to_int", address(nvs_str_to_int)),
        ("nvs_str_to_uint", address(nvs_str_to_uint)),
        ("nvs_str_to_float", address(nvs_str_to_float)),
        ("nvs_tagged_to_int", address(nvs_tagged_to_int)),
        ("nvs_tagged_to_uint", address(nvs_tagged_to_uint)),
        ("nvs_tagged_to_float", address(nvs_tagged_to_float)),
        (
            "nvs_tagged_widen_to_float",
            address(nvs_tagged_widen_to_float),
        ),
        ("nvs_to_int_or_null", address(nvs_to_int_or_null)),
        ("nvs_to_uint_or_null", address(nvs_to_uint_or_null)),
        ("nvs_to_float_or_null", address(nvs_to_float_or_null)),
        ("nvs_to_string_or_null", address(nvs_to_string_or_null)),
        ("nvs_tagged_to_string", address(nvs_tagged_to_string)),
        ("nvs_tagged_as_string", address(nvs_tagged_as_string)),
        ("nvs_bytes_to_string", address(nvs_bytes_to_string)),
        ("nvs_tagged_to_bytes", address(nvs_tagged_to_bytes)),
        ("nvs_to_array_of", address(nvs_to_array_of)),
        ("nvs_to_array_of_or_null", address(nvs_to_array_of_or_null)),
        ("nvs_to_bytes_or_null", address(nvs_to_bytes_or_null)),
        ("nvs_decimal_add", address(nvs_decimal_add)),
        ("nvs_decimal_sub", address(nvs_decimal_sub)),
        ("nvs_decimal_mul", address(nvs_decimal_mul)),
        ("nvs_decimal_div", address(nvs_decimal_div)),
        ("nvs_decimal_mod", address(nvs_decimal_mod)),
        ("nvs_decimal_neg", address(nvs_decimal_neg)),
        ("nvs_decimal_eq", address(nvs_decimal_eq)),
        ("nvs_decimal_lt", address(nvs_decimal_lt)),
        ("nvs_decimal_lt_eq", address(nvs_decimal_lt_eq)),
        ("nvs_decimal_cmp", address(nvs_decimal_cmp)),
        ("nvs_decimal_truthy", address(nvs_decimal_truthy)),
        ("nvs_to_decimal", address(nvs_to_decimal)),
        ("nvs_to_decimal_or_null", address(nvs_to_decimal_or_null)),
        ("nvs_decimal_to_int", address(nvs_decimal_to_int)),
        ("nvs_decimal_to_uint", address(nvs_decimal_to_uint)),
        ("nvs_decimal_to_float", address(nvs_decimal_to_float)),
        ("nvs_decimal_to_string", address(nvs_decimal_to_string)),
        ("nvs_echo_str", address(nvs_echo_str)),
        ("nvs_echo_value", address(nvs_echo_value)),
        ("nvs_echo_markup", address(nvs_echo_markup)),
        ("nvs_exit", address(nvs_exit)),
        (
            "nvs_single_value_mismatch",
            address(nvs_single_value_mismatch),
        ),
        (
            "nvs_str_new",
            (crate::string::nvs_str_new as *const ()).cast::<u8>(),
        ),
        (
            "nvs_call_callable",
            (crate::callable::nvs_call_callable as *const ()).cast::<u8>(),
        ),
        (
            "nvs_call_callable_proven",
            (crate::callable::nvs_call_callable_proven as *const ()).cast::<u8>(),
        ),
        (
            "nvs_call_callable_array",
            (crate::callable::nvs_call_callable_array as *const ()).cast::<u8>(),
        ),
        (
            "nvs_extension_call",
            (crate::extension::nvs_extension_call as *const ()).cast::<u8>(),
        ),
        (
            "nvs_callable_bind",
            (crate::callable::nvs_callable_bind as *const ()).cast::<u8>(),
        ),
        (
            "nvs_call_erased_method",
            (crate::dispatch::nvs_call_erased_method as *const ()).cast::<u8>(),
        ),
        (
            "nvs_str_concat",
            (crate::string::nvs_str_concat as *const ()).cast::<u8>(),
        ),
        (
            "nvs_str_concat_n",
            (crate::string::nvs_str_concat_n as *const ()).cast::<u8>(),
        ),
        (
            "nvs_str_append",
            (crate::string::nvs_str_append as *const ()).cast::<u8>(),
        ),
        (
            "nvs_str_eq",
            (crate::string::nvs_str_eq as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_eq",
            (crate::identity::nvs_array_eq as *const ()).cast::<u8>(),
        ),
        (
            "nvs_float_pow",
            (crate::arith::nvs_float_pow as *const ()).cast::<u8>(),
        ),
        (
            "nvs_str_retain",
            (crate::string::nvs_str_retain as *const ()).cast::<u8>(),
        ),
        (
            "nvs_value_retain",
            (crate::value::nvs_value_retain as *const ()).cast::<u8>(),
        ),
        (
            "nvs_value_release",
            (crate::value::nvs_value_release as *const ()).cast::<u8>(),
        ),
        (
            "nvs_str_release",
            (crate::string::nvs_str_release as *const ()).cast::<u8>(),
        ),
        (
            "nvs_raise",
            (crate::throwable::nvs_raise as *const ()).cast::<u8>(),
        ),
        (
            "nvs_raise_new",
            (crate::throwable::nvs_raise_new as *const ()).cast::<u8>(),
        ),
        (
            "nvs_raise_site",
            (crate::throwable::nvs_raise_site as *const ()).cast::<u8>(),
        ),
        (
            "nvs_trace_push",
            (crate::throwable::nvs_trace_push as *const ()).cast::<u8>(),
        ),
        (
            "nvs_take_thrown",
            (crate::throwable::nvs_take_thrown as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_new",
            (crate::object::nvs_object_new as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_clone",
            (crate::object::nvs_object_clone as *const ()).cast::<u8>(),
        ),
        (
            "nvs_clone_not_an_object",
            (crate::object::nvs_clone_not_an_object as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_retain",
            (crate::object::nvs_object_retain as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_release",
            (crate::object::nvs_object_release as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_is_class",
            (crate::object::nvs_object_is_class as *const ()).cast::<u8>(),
        ),
        (
            "nvs_value_is_class",
            (crate::object::nvs_value_is_class as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_slot_get",
            (crate::object::nvs_object_slot_get as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_slot_optional_get",
            (crate::object::nvs_object_slot_optional_get as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_slot_probe",
            (crate::object::nvs_object_slot_probe as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_slot_set",
            (crate::object::nvs_object_slot_set as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_key_get",
            (crate::object::nvs_object_key_get as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_key_set",
            (crate::object::nvs_object_key_set as *const ()).cast::<u8>(),
        ),
        (
            "nvs_abstract_method",
            (crate::object::nvs_abstract_method as *const ()).cast::<u8>(),
        ),
        (
            "nvs_class_method",
            (crate::object::nvs_class_method as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_class_name",
            (crate::object::nvs_object_class_name as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_field_get",
            (crate::object::nvs_object_field_get as *const ()).cast::<u8>(),
        ),
        (
            "nvs_object_field_set",
            (crate::object::nvs_object_field_set as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_new",
            (crate::array::nvs_array_new as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_retain",
            (crate::array::nvs_array_retain as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_release",
            (crate::array::nvs_array_release as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_get",
            (crate::array::nvs_array_get as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_has_key",
            (crate::array::nvs_array_has_key as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_get_index",
            (crate::array::nvs_array_get_index as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_set",
            (crate::array::nvs_array_set as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_set_index",
            (crate::array::nvs_array_set_index as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_append",
            (crate::array::nvs_array_append as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_spread",
            (crate::array::nvs_array_spread as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_unset",
            (crate::array::nvs_array_unset as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_count",
            (crate::array::nvs_array_count as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_next_slot",
            (crate::array::nvs_array_next_slot as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_key_at",
            (crate::array::nvs_array_key_at as *const ()).cast::<u8>(),
        ),
        (
            "nvs_array_value_at",
            (crate::array::nvs_array_value_at as *const ()).cast::<u8>(),
        ),
        (
            "nvs_safepoint",
            (crate::ctx::nvs_safepoint as *const ()).cast::<u8>(),
        ),
        (
            "nvs_stack_check",
            (crate::ctx::nvs_stack_check as *const ()).cast::<u8>(),
        ),
        (
            "nvs_deprecated_use",
            (crate::ctx::nvs_deprecated_use as *const ()).cast::<u8>(),
        ),
        (
            "nvs_probe_stmt",
            (crate::ctx::nvs_probe_stmt as *const ()).cast::<u8>(),
        ),
        (
            "nvs_probe_edge",
            (crate::ctx::nvs_probe_edge as *const ()).cast::<u8>(),
        ),
        (
            "nvs_probe_call_enter",
            (crate::ctx::nvs_probe_call_enter as *const ()).cast::<u8>(),
        ),
        (
            "nvs_probe_call_exit",
            (crate::ctx::nvs_probe_call_exit as *const ()).cast::<u8>(),
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
                      fresh `NvsStr` was built with"
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

    /// Calls a checked conversion helper, expecting `rule:types/conversion`'s throw.
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
        assert_eq!(string_result(nvs_int_to_string, Value::int(0)), "0");
        assert_eq!(string_result(nvs_int_to_string, Value::int(-42)), "-42");
        assert_eq!(
            string_result(nvs_int_to_string, Value::int(i64::MIN)),
            "-9223372036854775808"
        );
        assert_eq!(
            string_result(nvs_int_to_string, Value::int(i64::MAX)),
            "9223372036854775807"
        );
    }

    #[test]
    fn a_uint_converts_past_the_signed_range() {
        assert_eq!(string_result(nvs_uint_to_string, Value::uint(0)), "0");
        assert_eq!(
            string_result(nvs_uint_to_string, Value::uint(u64::MAX)),
            "18446744073709551615"
        );
    }

    /// `rule:types/arrays`'s key normalization performed on a tag, because
    /// the key was a `mixed` and its declared type showed nothing: the three
    /// kinds that are keys, and the three the checker refuses where it can
    /// see them.
    // covers: lang:types/mixed
    #[test]
    fn a_tagged_array_key_normalizes_or_throws() {
        assert_eq!(string_result(nvs_value_to_array_key, Value::int(8)), "8");
        assert_eq!(
            string_result(nvs_value_to_array_key, Value::uint(u64::MAX)),
            "18446744073709551615"
        );
        for refused in [Value::float(1.5), Value::bool(true), Value::null()] {
            let mut ctx = Ctx::buffered();
            assert!(
                call(nvs_value_to_array_key, &mut ctx, &[refused]).is_err(),
                "a key kind `rule:types/arrays` refuses has to throw here too"
            );
        }
    }

    #[test]
    fn a_float_converts_the_way_php_spells_it() {
        assert_eq!(string_result(nvs_float_to_string, Value::float(1.0)), "1");
        assert_eq!(
            string_result(nvs_float_to_string, Value::float(1e20)),
            "1.0E+20"
        );
    }

    #[test]
    fn a_bool_converts_to_one_or_the_empty_string() {
        assert_eq!(string_result(nvs_bool_to_string, Value::bool(true)), "1");
        assert_eq!(string_result(nvs_bool_to_string, Value::bool(false)), "");
    }

    #[test]
    fn scalar_truthiness_follows_phps_table() {
        assert!(!truthy(nvs_int_truthy, Value::int(0)));
        assert!(truthy(nvs_int_truthy, Value::int(-1)));
        assert!(!truthy(nvs_uint_truthy, Value::uint(0)));
        assert!(truthy(nvs_uint_truthy, Value::uint(1)));
        assert!(!truthy(nvs_float_truthy, Value::float(0.0)));
        assert!(!truthy(nvs_float_truthy, Value::float(-0.0)));
        assert!(truthy(nvs_float_truthy, Value::float(f64::NAN)));
        assert!(truthy(nvs_float_truthy, Value::float(0.1)));
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
            let value = Value::str(NvsStr::new(bytes));
            assert_eq!(truthy(nvs_str_truthy, value), expected, "for {bytes:?}");
            #[expect(unsafe_code, reason = "the value owns the reference it releases")]
            unsafe {
                value.release();
            }
        }
    }

    /// A table defining both of `rule:tooling/echo-always-has-a-sink`'s
    /// carriers, so a test can hand either one to either sink. That crossing is
    /// the only way to ask whether the raw path belongs to the **sink** or to
    /// the roster, and the two tests below are that question asked from each
    /// end.
    fn carriers() -> crate::ClassTable {
        let mut classes = crate::ClassTable::new();
        classes.define(crate::CARRIER_CLI_TEXT, &["text"], &[]);
        classes.define(crate::CARRIER_HTML_MARKUP, &["text"], &[]);
        classes
    }

    /// One instance of `class` holding `text` in [`crate::CARRIER_TEXT_SLOT`],
    /// which is where both carriers keep their bytes.
    fn carried(classes: &crate::ClassTable, class: &str, text: &[u8]) -> Value {
        let id = classes
            .id_of(class)
            .expect("the table defines both carriers");
        #[expect(
            unsafe_code,
            reason = "the table outlives every object this builds from it"
        )]
        let object = unsafe { crate::NvsObj::new(classes.desc(id)) };
        object.set_field(crate::CARRIER_TEXT_SLOT, Value::str(NvsStr::new(text)));
        Value::object(object)
    }

    /// Releases a reference this module built and the helper it was handed to
    /// did not take.
    fn dropped(value: Value) {
        #[expect(unsafe_code, reason = "this test owns the reference it built")]
        unsafe {
            value.release();
        }
    }

    /// `rule:core-classes/html-auto-escape` at the sink that applies it: inside
    /// an HTTP request `echo` escapes everything it is handed and writes a
    /// `Core\Html\Markup` as it is.
    ///
    /// The two halves are asked over the **same bytes**, so neither passes on
    /// its own: a sink that escaped the carrier too would corrupt the page it
    /// was built for, and one that wrote the string as it is would be the
    /// missing-escape bug the rule exists to make unwritable. The third line is
    /// the crossing — a `Cli\Text` is a carrier, but not this sink's, so it is
    /// data like any other value and takes the escape.
    #[test]
    fn echo_into_a_body_sink_escapes_a_string_and_writes_a_markup_carrier_as_it_is() {
        let classes = carriers();
        let mut ctx = Ctx::new(crate::OutputSink::Body(Vec::new()));

        let text = Value::str(NvsStr::new(b"<b>ada & co</b>"));
        call(nvs_echo_str, &mut ctx, &[text]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"&lt;b&gt;ada &amp; co&lt;/b&gt;"[..]),
            "a string interpolated into a page is data, so the sink escapes it"
        );

        let markup = carried(&classes, crate::CARRIER_HTML_MARKUP, b"<b>ada & co</b>");
        call(nvs_echo_value, &mut ctx, &[markup]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"<b>ada & co</b>"[..]),
            "the carrier already passed whichever rule made it markup"
        );

        let styled = carried(&classes, crate::CARRIER_CLI_TEXT, b"<b>ada & co</b>");
        call(nvs_echo_value, &mut ctx, &[styled]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"&lt;b&gt;ada &amp; co&lt;/b&gt;"[..]),
            "the raw path is this sink's carrier and not the roster"
        );

        dropped(text);
        dropped(markup);
        dropped(styled);
    }

    /// The other row of the same table: every sink but an HTTP request's keeps
    /// `rule:tooling/terminal-output-is-a-sink`'s substitution, and its raw
    /// path is its own carrier.
    ///
    /// The operands carry an `ESC` **and** the markup characters together, so
    /// the two renderings are told apart on one write rather than assumed: a
    /// terminal sink neutralizes the control byte and leaves `<` and `&`
    /// alone, which is the exact opposite of the sink above. The `Markup`
    /// line is the crossing from the other end — it is a carrier, and the
    /// terminal sink substitutes it anyway, because it is not this sink's.
    #[test]
    fn echo_into_every_other_sink_writes_the_terminals_rendering() {
        let classes = carriers();
        let mut ctx = Ctx::buffered();

        let text = Value::str(NvsStr::new(b"<b>&\x1b"));
        call(nvs_echo_str, &mut ctx, &[text]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some("<b>&␛".as_bytes()),
            "a terminal sink neutralizes the control byte and escapes nothing"
        );

        let styled = carried(&classes, crate::CARRIER_CLI_TEXT, b"<b>&\x1b");
        call(nvs_echo_value, &mut ctx, &[styled]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"<b>&\x1b"[..]),
            "`Cli\\Text` is this sink's carrier, so its escapes are what it is for"
        );

        let markup = carried(&classes, crate::CARRIER_HTML_MARKUP, b"<b>&\x1b");
        call(nvs_echo_value, &mut ctx, &[markup]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some("<b>&␛".as_bytes()),
            "a `Markup` off a request is not this sink's carrier and is substituted"
        );

        dropped(text);
        dropped(styled);
        dropped(markup);
    }

    #[test]
    fn echo_writes_raw_bytes_and_returns_nothing() {
        let mut ctx = Ctx::buffered();
        let value = Value::str(NvsStr::new(b"Hello, World!"));
        let result = call(nvs_echo_str, &mut ctx, &[value]).expect("the helper succeeded");
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

    /// `rule:tooling/terminal-output-is-a-sink` at the sink: control bytes are neutralized, and **nothing
    /// else is**. The markup characters are in the same value on purpose —
    /// this is not an HTML sink, so `<`, `&` and `"` reach the stream as
    /// themselves, and `\x00` does not.
    #[test]
    fn echo_neutralizes_control_bytes_and_nothing_else() {
        let mut ctx = Ctx::buffered();
        let value = Value::str(NvsStr::new(b"<b>&\"\x00"));
        call(nvs_echo_str, &mut ctx, &[value]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some("<b>&\"\u{2400}".as_bytes())
        );
        #[expect(unsafe_code, reason = "the value owns the reference it releases")]
        unsafe {
            value.release();
        }
    }

    /// The lossy read the helper's doc comment names, asserted rather than
    /// assumed: an ill-formed byte cannot reach the stream as itself, and it
    /// cannot make the pass panic either.
    #[test]
    fn echo_replaces_ill_formed_utf8_rather_than_writing_it() {
        let mut ctx = Ctx::buffered();
        let value = Value::str(NvsStr::new(b"a\xffb"));
        call(nvs_echo_str, &mut ctx, &[value]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some("a\u{FFFD}b".as_bytes())
        );
        #[expect(unsafe_code, reason = "the value owns the reference it releases")]
        unsafe {
            value.release();
        }
    }

    /// The substitution is idempotent, which is what makes `echo` of a
    /// `Core\Out::capture` buffer — already neutralized once — correct.
    #[test]
    fn a_second_pass_over_neutralized_output_changes_nothing() {
        let mut ctx = Ctx::buffered();
        let value = Value::str(NvsStr::new("a\u{241B}b".as_bytes()));
        call(nvs_echo_str, &mut ctx, &[value]).expect("the helper succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some("a\u{241B}b".as_bytes())
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
            call(nvs_int_to_string, &mut ctx, &[Value::uint(1)]).unwrap_err(),
            FATAL
        );
        let message = ctx.take_pending().expect("a message was recorded");
        assert!(message.contains("nvs_int_to_string"), "{message}");

        assert_eq!(
            call(nvs_echo_str, &mut ctx, &[Value::int(1)]).unwrap_err(),
            FATAL
        );
        let message = ctx.take_pending().expect("a message was recorded");
        assert!(message.contains("nvs_echo_str"), "{message}");
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

    /// `rule:types/conversion`'s `int` ↔ `uint` row: exact, or throws. The two ends that
    /// have no counterpart on the other side are the whole content of the row.
    #[test]
    fn int_and_uint_convert_where_the_ranges_overlap_and_throw_where_they_do_not() {
        assert_eq!(converted(nvs_int_to_uint, Value::int(0)).as_uint(), Some(0));
        assert_eq!(
            converted(nvs_int_to_uint, Value::int(i64::MAX)).as_uint(),
            Some(i64::MAX.cast_unsigned())
        );
        refused(nvs_int_to_uint, Value::int(-1));

        assert_eq!(
            converted(nvs_uint_to_int, Value::uint(i64::MAX.cast_unsigned())).as_int(),
            Some(i64::MAX)
        );
        refused(nvs_uint_to_int, Value::uint(i64::MAX.cast_unsigned() + 1));
        refused(nvs_uint_to_int, Value::uint(u64::MAX));
    }

    /// `rule:types/conversion`: an integer to `float` is "exact, or throws above 2^53,
    /// where `f64` stops representing every integer."
    #[test]
    fn an_integer_to_float_throws_past_the_point_it_would_stop_being_exact() {
        assert_eq!(
            converted(nvs_int_to_float, Value::int(-9007199254740992)).as_float(),
            Some(-9007199254740992.0)
        );
        assert_eq!(
            converted(nvs_uint_to_float, Value::uint(9007199254740992)).as_float(),
            Some(9007199254740992.0)
        );
        refused(nvs_int_to_float, Value::int(9007199254740993));
        refused(nvs_uint_to_float, Value::uint(u64::MAX));
    }

    /// `rule:types/conversion`: "integral and in range, or throws. Rounding is
    /// `floor`/`ceil`/`round`, said out loud" — so a fractional value is
    /// refused rather than silently picking one of them.
    #[test]
    fn a_float_to_an_integer_refuses_anything_it_would_have_to_round() {
        assert_eq!(
            converted(nvs_float_to_int, Value::float(-3.0)).as_int(),
            Some(-3)
        );
        assert_eq!(
            converted(nvs_float_to_uint, Value::float(3.0)).as_uint(),
            Some(3)
        );
        refused(nvs_float_to_int, Value::float(1.5));
        refused(nvs_float_to_uint, Value::float(-1.0));
        refused(nvs_float_to_int, Value::float(f64::NAN));
        refused(nvs_float_to_int, Value::float(f64::INFINITY));
        refused(nvs_float_to_int, Value::float(1e30));
    }

    /// `rule:types/conversion`: "the whole string must be an exact numeric literal, or
    /// throws. No leading-garbage rule, no `0`" — PHP's `(int)"12abc" === 12`
    /// and `(int)"abc" === 0` are both gone.
    #[test]
    fn a_string_to_a_number_takes_the_whole_string_or_nothing() {
        /// Runs `check` over a fresh string argument and releases it after —
        /// a conversion helper only *reads* its operand (compiled code emits
        /// the release itself, see `nvs_ir::lower::Lowering::convert`), so a
        /// test that dropped the value here would leak it.
        fn with(text: &str, check: impl FnOnce(Value)) {
            let value = Value::str(NvsStr::new(text.as_bytes()));
            check(value);
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference the fresh `NvsStr` \
                          was built with, and the helper borrowed it"
            )]
            unsafe {
                value.release();
            }
        }

        with("-42", |v| {
            assert_eq!(converted(nvs_str_to_int, v).as_int(), Some(-42));
        });
        with("18446744073709551615", |v| {
            assert_eq!(converted(nvs_str_to_uint, v).as_uint(), Some(u64::MAX));
        });
        with("3.5", |v| {
            assert_eq!(converted(nvs_str_to_float, v).as_float(), Some(3.5));
        });
        for bad in ["12abc", "abc", "", " 12", "12 ", "0x10", "1.5"] {
            with(bad, |v| refused(nvs_str_to_int, v));
        }
        with("-1", |v| refused(nvs_str_to_uint, v));
        // Not "an exact numeric literal": no numeric literal in Novis source writes one.
        for bad in ["inf", "NaN", "infinity"] {
            with(bad, |v| refused(nvs_str_to_float, v));
        }
    }

    /// Every row [`value_to_string`] answers, against the statically-typed
    /// helper for the same tag — the two tables are the same table, and a
    /// program rendering a `mixed` must not get a second set of answers.
    #[test]
    fn a_tagged_operand_renders_by_its_tag() {
        assert_eq!(string_result(nvs_tagged_to_string, Value::null()), "");
        assert_eq!(string_result(nvs_tagged_to_string, Value::bool(true)), "1");
        assert_eq!(string_result(nvs_tagged_to_string, Value::bool(false)), "");
        assert_eq!(string_result(nvs_tagged_to_string, Value::int(-42)), "-42");
        assert_eq!(
            string_result(nvs_tagged_to_string, Value::uint(u64::MAX)),
            "18446744073709551615"
        );
        assert_eq!(string_result(nvs_tagged_to_string, Value::float(2.0)), "2");
        assert_eq!(
            string_result(nvs_tagged_to_string, Value::float(1.5)),
            "1.5"
        );
    }

    /// A `Tag::Str` payload is handed back as itself, so the one thing that
    /// could go wrong is the reference count: the result has to carry a
    /// *fresh* reference, because its caller releases it exactly as it
    /// releases a converted one.
    #[test]
    fn a_string_payload_comes_back_with_a_reference_of_its_own() {
        let value = Value::str(NvsStr::new(b"text"));
        let ptr = value.str_ptr().expect("a Tag::Str value carries a pointer");
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference the fresh `NvsStr` was \
                      built with, so the allocation is live for every read below"
        )]
        unsafe {
            assert_eq!(NvsStr::refcount_of(ptr), 1);
            // `string_result` releases what the helper returned, so a helper
            // that answered without retaining would leave zero here — and a
            // use-after-free rather than an assertion failure.
            assert_eq!(string_result(nvs_tagged_to_string, value), "text");
            assert_eq!(NvsStr::refcount_of(ptr), 1);
            value.release();
        }
    }

    /// A tag `rule:types/conversion` writes no row from throws rather than substituting
    /// PHP's `"Array"`-plus-warning. An array stands for every such tag:
    /// they share one arm.
    #[test]
    fn a_tag_with_no_string_row_throws() {
        let value = Value::array(crate::array::NvsArray::new());
        refused(nvs_tagged_to_string, value);
        #[expect(
            unsafe_code,
            reason = "the helper borrowed the array; this test still owns the \
                      one reference it was built with"
        )]
        unsafe {
            value.release();
        }
    }

    /// A tagged `bytes` operand, asked both ways: the explicit `as string`
    /// validates it the way the statically typed `bytes as string` does, and the
    /// implicit rendering `.` and `echo` share refuses it. A `?bytes as string`
    /// that shared the implicit helper threw on well-formed text while
    /// `as ?string` answered it, which is the disagreement this pins.
    #[test]
    fn a_tagged_bytes_converts_only_when_the_conversion_is_written() {
        let text = Value::bytes(NvsStr::new(b"text"));
        let broken = Value::bytes(NvsStr::new(b"\xff"));
        assert_eq!(string_result(nvs_tagged_as_string, text), "text");
        refused(nvs_tagged_as_string, broken);
        refused(nvs_tagged_to_string, text);
        assert_eq!(string_result(nvs_tagged_as_string, Value::int(7)), "7");
        #[expect(
            unsafe_code,
            reason = "every helper above borrowed its operand; this test still \
                      owns the one reference each buffer was built with"
        )]
        unsafe {
            text.release();
            broken.release();
        }
    }

    /// `rule:security/secret-comparison-is-constant-time` reaches a credential
    /// compared against a `mixed`, so this helper is handed a tag rather than
    /// two known buffers. Every tag that is not the other side's answers
    /// `false` — what [`crate::value_identical`] answers for the same pair —
    /// and the `string`/`bytes` row is the one that matters: the two share an
    /// allocation, so a comparison reading the buffer alone would equate a
    /// `secret string` with a `bytes` holding the same octets.
    #[test]
    fn a_secret_comparison_answers_false_for_a_tag_the_other_side_does_not_share() {
        let equal = |left: Value, right: Value| {
            let mut ctx = Ctx::buffered();
            call(nvs_secret_eq, &mut ctx, &[left, right])
                .expect("the row is total")
                .as_bool()
                .expect("a comparison produces a bool")
        };
        let token = Value::str(NvsStr::new(b"s3cr3t"));
        let same = Value::str(NvsStr::new(b"s3cr3t"));
        let octets = Value::bytes(NvsStr::new(b"s3cr3t"));
        let same_octets = Value::bytes(NvsStr::new(b"s3cr3t"));
        assert!(equal(token, same));
        assert!(equal(octets, same_octets));
        assert!(!equal(token, octets));
        assert!(!equal(octets, token));
        // One side is a buffer by declaration — the operand the checker called
        // `secret` — so these are the pairs a `mixed` on the other side makes,
        // and a pair of non-buffers is not one of them.
        assert!(!equal(token, Value::int(0)));
        assert!(!equal(token, Value::null()));
        assert!(!equal(Value::bool(true), octets));
        #[expect(
            unsafe_code,
            reason = "the helper only borrows its operands; this test still \
                      owns the one reference each was built with"
        )]
        unsafe {
            token.release();
            same.release();
            octets.release();
            same_octets.release();
        }
    }
}
