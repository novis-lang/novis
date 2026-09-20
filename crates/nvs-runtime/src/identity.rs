//! Strict identity over two [`Value`]s, and the hash that agrees with it.
//!
//! One comparison, defined once, because `Core\Arr`'s members and every
//! `ObjectSet`/`ObjectMap` operation ask the same question —
//! `docs/spec/01-core-library.md` § 2 says `contains`, `keyOf`, `unique`,
//! `diff` and `intersect` "compare by **strict identity**", and a second
//! answer living in `nvs-stdlib` would be a second set of PHP-divergence
//! decisions nothing keeps in step. It is also what `==` means:
//! `rule:expressions/equality-semantics`'s table is this module's rows, reached by whichever door the
//! operands' static types already settled. A scalar pair is one machine
//! comparison and never arrives here. An array pair arrives through
//! [`nvs_array_eq`], a string pair through [`crate::nvs_str_eq`] and an object
//! pair through an inline pointer comparison, because in each of those the row
//! is known before the program runs. Only a `mixed` or union operand — § 5's
//! case, where the row is a runtime tag — arrives through `nvs_value_identical`
//! on [`crate::abi`]'s general helper convention, and § 5's "a mismatched
//! runtime type is `false` rather than a throw" is the fall-through arm of
//! [`shallow_identical`] rather than a rule stated twice. The numeric row is
//! answered by [`numeric_identical`], which the statically typed
//! `nvs_ir::Helper::NumericEq` reaches directly and [`shallow_identical`]
//! delegates to, so the row has one answer however it is reached.
//!
//! # What identity means, one row per representation
//!
//! PHP's `===` is the starting point, and each row below either matches it or
//! says why it cannot:
//!
//! * **`null`** — one value, so two of them are identical.
//! * **`bool`** — equal payloads.
//! * **`int`, `uint`, `float` and `decimal` are one numeric domain**, and
//!   [`numeric_identical`] is the whole of it: `3` and `3 as uint` are
//!   identical, `1` and `1.0` are identical, and so is `1.00` written as a
//!   `decimal`. `rule:expressions/equality-semantics` gives that row as "mathematically equal across
//!   the whole domain", and this comparison takes it **including** where
//!   `Core\Arr` reads it — so `Arr::contains([1.0], 1)` is `true`, diverging
//!   from PHP's `1 === 1.0`. The alternative was the worse trap: one
//!   comparison answering `==` one way and `contains` the other, with nothing
//!   in the language to tell a reader which they had. Along the edges of that
//!   one row:
//!     * `int` against `uint` goes through `i128`, so no large `uint` is ever
//!       reinterpreted as a negative `int`.
//!       `rule:types/arithmetic` makes
//!       `uint` a *range* restriction over the same integers rather than a
//!       different value space, so there was never a second value here.
//!     * Two floats compare with `==`, not by bits: `0.0` and `-0.0` are
//!       identical and `NaN` is identical to nothing, both PHP's answers, and
//!       both the opposite of what `total_cmp` (which `Core\Arr::sort` needs,
//!       and which is a different question) would say.
//!     * Two decimals compare by value and never by scale.
//!       `rule:types/conversion` puts it
//!       outright — "scale is carried for rendering, and does not affect
//!       equality or hashing" — so `1.10` and `1.1000` are one value even
//!       though they hold different bits.
//!     * A `float` against an integer is read at the float's **exact** binary
//!       value; a `float` against a `decimal` is read at the decimal it
//!       **prints** as, which is `rule:types/conversion`'s conversion row and is what
//!       makes `(0.1 as decimal) == 0.1` true. The two readings differ only
//!       for an integral float past 2^53, where they disagree about a third
//!       value rather than about each other — [`hash_numeric`] carries the
//!       consequence.
//! * **`string`/`bytes`** — by content, never by pointer, so a computed
//!   string matches a literal. `rule:types/bytes`
//!   makes a `string` valid UTF-8 but does *not* normalize it, so this is a
//!   byte comparison and `"é"` written two ways is two values.
//! * **An `array` is compared entry by entry, in order** — the same count,
//!   the same keys in the same positions, and identical values — which is
//!   PHP's `===` on arrays exactly. Two handles on one allocation short-
//!   circuit to `true` before anything is walked.
//! * **An object is identical only to itself.** Pointer identity, PHP's
//!   answer, and the one that stays coherent under
//!   `rule:classes/comparable`: comparing
//!   two instances *by their contents* is a `Comparable::compareTo` call the
//!   class opts into, so a member that walked properties here would be the
//!   property-walk fallback that rule refuses. A closure is an object
//!   ([`crate::closure`]), so two `fn` literals are never identical and one
//!   closure value is identical to a copy of itself.
//!
//! # Why this terminates, and what it costs
//!
//! The array walk uses an explicit worklist rather than recursion, so nesting
//! depth costs heap rather than stack — an adversarially deep literal is a
//! slow comparison, never a stack overflow (AGENTS.md's priority 1). It
//! cannot loop: an array is a copy-on-write *value*, so storing one into
//! itself separates the copy first and no array ever reaches itself, and an
//! object — the one shape that can form a cycle (crate docs, known gap 7) —
//! is compared by pointer without being walked.
//!
//! The cost is one worklist `Vec` per comparison that reaches a nested array,
//! and nothing at all for the scalar and string cases, which is every
//! comparison the members above actually make in practice.
//!
//! # The hash is a second answer to the same question
//!
//! [`value_hash`] exists so `unique`/`diff`/`intersect` can index a set
//! instead of scanning it — a linear scan makes each of those quadratic, and
//! a quadratic member over request-shaped input is a denial of service, not a
//! slow path. Its one obligation is the standard one: **identical values hash
//! equally.** It is deliberately allowed to be coarse in the other direction,
//! and is — a `NaN` hashes like any other float though it is identical to
//! nothing, an array is hashed only [`HASH_DEPTH`] levels deep, and the
//! numeric domain merges the one trio [`hash_numeric`] describes — because a
//! collision costs one extra [`value_identical`] call and bounded work is
//! what keeps the hash itself immune to a deep input.
//!
//! The numeric family is the one row where the hash costs more than a write:
//! an `int` pays two casts, a `float` pays nothing, and a `decimal` pays the
//! rendering `crate::Decimal::to_f64` and `compare_f64` each do — the same
//! allocation the *comparison* already makes for a `decimal`/`float` pair, so
//! it buys agreement rather than adding a new class of cost.
//!
//! It writes into a caller-supplied [`Hasher`] rather than returning a `u64`
//! so the caller's own [`std::collections::HashSet`] supplies the randomly
//! keyed [`std::collections::hash_map::RandomState`]. A fixed-key hash here
//! would hand an attacker collision-crafting against a `Core` member, which
//! is the exact failure the set index exists to avoid.

use std::cmp::Ordering;
use std::hash::Hasher;
use std::mem::ManuallyDrop;

use crate::array::{ArrayHeader, NvsArray};
use crate::string::NvsStr;
use crate::value::{Tag, Value};

/// How many levels of nested array [`value_hash`] descends before it stops.
///
/// Four rather than one because the shapes that collide in practice —
/// `array<array<string>>` rows out of `groupBy`, a decoded JSON object — differ
/// below the first level; and bounded rather than complete because this
/// module's docs say why a hash may be coarse and a comparison may not.
const HASH_DEPTH: u32 = 4;

/// Whether two values are the same value — the one strict-identity comparison
/// this crate defines, whose rules are this module's own docs.
///
/// **The first pair is compared before the worklist is touched**, and
/// [`Vec::new`] holds no heap block until something is pushed into it. Only an array
/// pair pushes, so every comparison over two scalars — `==` on the request
/// path, and each entry `Core\Arr::contains` walks — allocates nothing. Seeding
/// the vector with the first pair instead cost one allocation per call, which
/// `benches/members/core/Arr/contains.nvs` counted once per entry.
#[must_use]
pub fn value_identical(left: Value, right: Value) -> bool {
    let mut worklist = Vec::new();
    if !shallow_identical(left, right, &mut worklist) {
        return false;
    }
    while let Some((left, right)) = worklist.pop() {
        if !shallow_identical(left, right, &mut worklist) {
            return false;
        }
    }
    true
}

/// Whether two arrays hold the same entries in the same order —
/// `nvs_ir::ir::BinOp::Eq` over a `Ty::Array` operand pair, and
/// `rule:expressions/equality-semantics`'s array row.
///
/// Compiled code reaches [`value_identical`] through this entry point rather
/// than through [`crate::abi`]'s helper convention because the operands' static
/// types already named the row: two raw pointers travel in registers, where the
/// general convention would tag each into a 16-byte stack slot and then check a
/// status a total comparison can never raise. [`crate::nvs_str_eq`] is the same
/// trade for the string row, and the object row needs no call at all — pointer
/// identity is one machine comparison, which `nvs-codegen` emits inline.
///
/// Neither operand is retained or released: the read-only treatment
/// [`crate::nvs_str_eq`] gives its two, so the [`Value`]s built here are
/// borrowed views that are deliberately never dropped.
///
/// # Safety
///
/// `lhs` and `rhs` must each refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw array pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_eq(lhs: *mut ArrayHeader, rhs: *mut ArrayHeader) -> bool {
    value_identical(Value::from_array_ptr(lhs), Value::from_array_ptr(rhs))
}

/// One pair's verdict, pushing an array's entries onto `worklist` rather than
/// descending into them.
fn shallow_identical(left: Value, right: Value, worklist: &mut Vec<(Value, Value)>) -> bool {
    match (left.tag(), right.tag()) {
        (Some(Tag::Null), Some(Tag::Null)) => true,
        (Some(Tag::Bool), Some(Tag::Bool)) => left.bits() == right.bits(),
        // One arm, not one per tag: `rule:expressions/equality-semantics`'s numeric row is a single
        // row, so every representation delegates to the one comparison that
        // spans them rather than to several that disagree across their edges.
        (
            Some(Tag::Int | Tag::Uint | Tag::Float | Tag::Decimal),
            Some(Tag::Int | Tag::Uint | Tag::Float | Tag::Decimal),
        ) => numeric_identical(left, right),
        (Some(Tag::Str), Some(Tag::Str)) => left.as_str_bytes() == right.as_str_bytes(),
        // A `bytes` is a value rather than a handle, so it compares by
        // content — and only against another `bytes`. `rule:expressions/equality-semantics` gives
        // `string` and `bytes` the strict reading two disjoint types get, so a
        // `bytes` whose octets happen to spell UTF-8 is still not the `string`
        // that spells the same thing: that mixed pair falls to the arm below
        // and answers `false` on the tag bytes alone.
        (Some(Tag::Bytes), Some(Tag::Bytes)) => left.as_bytes() == right.as_bytes(),
        (Some(Tag::Array), Some(Tag::Array)) => {
            let (Some(a), Some(b)) = (left.array_ptr(), right.array_ptr()) else {
                return false;
            };
            std::ptr::eq(a, b) || entries_identical(a, b, worklist)
        }
        // Every remaining representation is an opaque handle: identical to
        // itself and to nothing else. That covers an object (this module's
        // docs say why pointer identity is the answer `rule:classes/comparable` leaves room
        // for), the tags nothing constructs yet, and the tag byte only a
        // miscompile can produce.
        _ => left.tag_byte() == right.tag_byte() && left.bits() == right.bits(),
    }
}

/// Whether two distinct array allocations hold the same entries in the same
/// order, queueing each value pair rather than comparing it here.
fn entries_identical(
    left: *mut ArrayHeader,
    right: *mut ArrayHeader,
    worklist: &mut Vec<(Value, Value)>,
) -> bool {
    let left = borrowed(left);
    let right = borrowed(right);
    if left.count() != right.count() {
        return false;
    }

    let (mut left_from, mut right_from) = (0usize, 0usize);
    loop {
        match (left.next_slot(left_from), right.next_slot(right_from)) {
            (None, None) => return true,
            (Some(left_slot), Some(right_slot)) => {
                if left.key_at(left_slot).as_ref().map(NvsStr::as_bytes)
                    != right.key_at(right_slot).as_ref().map(NvsStr::as_bytes)
                {
                    return false;
                }
                let (Some(left_value), Some(right_value)) =
                    (left.value_at(left_slot), right.value_at(right_slot))
                else {
                    return false;
                };
                worklist.push((left_value, right_value));
                left_from = left_slot + 1;
                right_from = right_slot + 1;
            }
            // Unreachable while the counts agree, and cheaper to answer than
            // to argue about.
            _ => return false,
        }
    }
}

/// Feeds `value` to `state` so that two [`value_identical`] values feed it the
/// same bytes — see this module's docs for where it is deliberately coarser
/// than the comparison.
pub fn value_hash<H: Hasher>(value: Value, state: &mut H) {
    hash_to_depth(value, state, HASH_DEPTH);
}

/// [`value_hash`], carrying how many more levels of array it may descend.
fn hash_to_depth<H: Hasher>(value: Value, state: &mut H, depth: u32) {
    // One discriminant per *family*, not per tag: the numeric representations
    // share one because they are one domain, and every opaque handle shares
    // one because it is hashed as its bits either way.
    match value.tag() {
        Some(Tag::Null) => state.write_u8(0),
        Some(Tag::Bool) => {
            state.write_u8(1);
            state.write_u64(value.bits());
        }
        Some(Tag::Int | Tag::Uint | Tag::Float | Tag::Decimal) => hash_numeric(value, state),
        Some(Tag::Str) => {
            state.write_u8(4);
            state.write(value.as_str_bytes().unwrap_or_default());
        }
        // Its own discriminant, not `string`'s: the two never compare equal,
        // so keying them apart keeps a `bytes` out of the bucket its UTF-8
        // twin already occupies rather than merging two domains that would
        // then have to be told apart on every probe.
        Some(Tag::Bytes) => {
            state.write_u8(7);
            state.write(value.as_bytes().unwrap_or_default());
        }
        Some(Tag::Array) => {
            state.write_u8(5);
            let Some(ptr) = value.array_ptr() else { return };
            let array = borrowed(ptr);
            state.write_usize(array.count());
            if depth == 0 {
                return;
            }
            let mut from = 0usize;
            while let Some(slot) = array.next_slot(from) {
                if let Some(key) = array.key_at(slot) {
                    state.write(key.as_bytes());
                }
                if let Some(entry) = array.value_at(slot) {
                    hash_to_depth(entry, state, depth - 1);
                }
                from = slot + 1;
            }
        }
        _ => {
            state.write_u8(6);
            state.write_u8(value.tag_byte());
            state.write_u64(value.bits());
        }
    }
}

/// The bytes a numeric contributes, chosen so that any pair
/// [`numeric_identical`] answers `true` for feeds the same ones.
///
/// **A numeric hashes as the `f64` it coincides with, when there is one, and
/// as its own exact decimal otherwise.** The `f64` is the canonical form
/// because it is the only thing the domain's two readings of a float agree on:
/// [`integer_eq_float`] reads a float at its exact binary value and
/// [`decimal_eq_float`] reads it at the decimal it prints as, so past 2^53 one
/// float can be identical to two different exact values — `2^60` is identical
/// both to the `int` it holds and to the `decimal` `1152921504606847000` it
/// prints as, while those two are not identical to each other. Keying all
/// three on the float merges exactly that trio, and [`coincident_float`]
/// refuses every value that merely *rounds* to a float, so a neighbouring
/// large `int` keeps its own bucket rather than sharing one with the 1023
/// beside it — a bucket an attacker could fill is the denial of service this
/// hash exists to prevent.
fn hash_numeric<H: Hasher>(value: Value, state: &mut H) {
    state.write_u8(2);
    if let Some(float) = coincident_float(value) {
        state.write_u8(0);
        state.write_u64(float.to_bits());
        return;
    }
    state.write_u8(1);
    // Trailing zeros are stripped, because they are exactly the difference
    // identity ignores: `1.10` and `1.1000` are one value and one hash.
    let exact = match value.tag() {
        Some(Tag::Decimal) => value.as_decimal().map(crate::Decimal::reduced),
        _ => decimal_of_integer(integer(value)),
    };
    let Some(exact) = exact else { return };
    state.write_u8(u8::from(exact.is_negative()));
    state.write_u128(exact.mantissa());
    state.write_u8(exact.scale());
}

/// The `f64` a numeric coincides with under either of the domain's readings,
/// or `None` where the nearest float holds a different number.
fn coincident_float(value: Value) -> Option<f64> {
    match value.tag() {
        // A float coincides with itself, and `-0.0` is identical to `0.0`, so
        // both answer `0.0`.
        Some(Tag::Float) => value
            .as_float()
            .map(|float| if float == 0.0 { 0.0 } else { float }),
        Some(Tag::Decimal) => {
            let decimal = value.as_decimal()?;
            let float = decimal.to_f64();
            // Both readings, because both rows reach a `decimal`: an equal
            // `float` takes the printed one, an equal `int` the exact one.
            let printed = decimal.compare_f64(float).is_some_and(Ordering::is_eq);
            let exact = || integer_of(decimal).is_some_and(|n| exactly_a_float(n).is_some());
            (printed || exact()).then_some(float)
        }
        _ => exactly_a_float(integer(value)),
    }
}

/// The `f64` an integer *is*, or `None` where the nearest one holds a
/// different number.
fn exactly_a_float(integer: i128) -> Option<f64> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the round trip below is precisely the test for whether that \
                  loss happened"
    )]
    let float = integer as f64;
    (float.is_finite() && exact_i128(float) == Some(integer)).then_some(float)
}

/// A `decimal` as the `int`/`uint`-range integer it is, or `None` when it has
/// a fractional part or lies outside both ranges — where no integer can be
/// identical to it anyway.
fn integer_of(decimal: crate::Decimal) -> Option<i128> {
    decimal
        .to_i64()
        .map(i128::from)
        .or_else(|| decimal.to_u64().map(i128::from))
}

/// Whether two numeric values are mathematically equal with `int`, `uint`,
/// `float` and `decimal` read as **one domain** —
/// `rule:expressions/equality-semantics`'s numeric row, which its § 2 admits as a compiling pairing and its § 5
/// resolves a `mixed` operand to.
///
/// It is reached from either side, and answers the same question for both:
/// `nvs_ir::Helper::NumericEq` when two statically typed operands crossed two
/// representations, and [`shallow_identical`] when a `mixed` operand's runtime
/// tags did — which is also `Core\Arr`'s strict identity, so
/// `Arr::contains([1.0], 1)` is `true`. `1 == 1.0` is `true` here and
/// deliberately not PHP's `1 === 1.0`: one comparison with two answers is the
/// worse trap, and this module's docs' rows state the whole of it.
///
/// A pair of the same representation still arrives — from the `mixed` side,
/// where the row is a runtime tag rather than a static type. From the other
/// side it never does: that is one machine comparison `nvs-codegen` emits
/// inline.
#[must_use]
pub fn numeric_identical(left: Value, right: Value) -> bool {
    match (left.tag(), right.tag()) {
        (Some(Tag::Int | Tag::Uint), Some(Tag::Int | Tag::Uint)) => integer(left) == integer(right),
        (Some(Tag::Int | Tag::Uint), Some(Tag::Float)) => right
            .as_float()
            .is_some_and(|float| integer_eq_float(integer(left), float)),
        (Some(Tag::Float), Some(Tag::Int | Tag::Uint)) => left
            .as_float()
            .is_some_and(|float| integer_eq_float(integer(right), float)),
        (Some(Tag::Float), Some(Tag::Float)) => match (left.as_float(), right.as_float()) {
            // `==` rather than the bit pattern: `-0.0 == 0.0` and
            // `NaN != NaN` are both PHP's answers, and both differ from
            // comparing `bits()`.
            (Some(a), Some(b)) => a == b,
            _ => false,
        },
        (Some(Tag::Decimal), Some(Tag::Decimal)) => match (left.as_decimal(), right.as_decimal()) {
            (Some(a), Some(b)) => a.compare(b).is_eq(),
            _ => false,
        },
        (Some(Tag::Decimal), Some(Tag::Int | Tag::Uint)) => {
            decimal_eq_integer(left, integer(right))
        }
        (Some(Tag::Int | Tag::Uint), Some(Tag::Decimal)) => {
            decimal_eq_integer(right, integer(left))
        }
        (Some(Tag::Decimal), Some(Tag::Float)) => decimal_eq_float(left, right),
        (Some(Tag::Float), Some(Tag::Decimal)) => decimal_eq_float(right, left),
        // Unreachable from a compiled `==`, whose two operands were both
        // numeric before this was chosen — and cheaper to answer than to
        // argue about, exactly like [`entries_identical`]'s last arm.
        _ => false,
    }
}

/// Whether a `decimal` and an integer denote the same number, **exactly**.
///
/// Every `int` and every `uint` payload is a `decimal` without loss —
/// `u64::MAX` needs 64 of the mantissa's 96 bits and `i64::MIN`'s magnitude
/// needs 63 — so there is no reading to choose here, only
/// `crate::Decimal::compare`'s scale-independent ordering.
fn decimal_eq_integer(decimal: Value, integer: i128) -> bool {
    match (decimal.as_decimal(), decimal_of_integer(integer)) {
        (Some(decimal), Some(other)) => decimal.compare(other).is_eq(),
        _ => false,
    }
}

/// Whether a `decimal` and a `float` denote the same number, with the float
/// read at the decimal it **prints** as.
///
/// That is `crate::Decimal::compare_f64`'s reading and
/// `rule:types/conversion`'s `float →
/// decimal` row, so `(0.1 as decimal) == 0.1` holds — the answer this pairing
/// exists to give, and the one the statically typed `nvs_ir::Helper::DecimalEq`
/// gives. It differs from [`integer_eq_float`]'s exact reading only for
/// an integral float past 2^53, which [`hash_numeric`] is coarse enough to
/// cover.
fn decimal_eq_float(decimal: Value, float: Value) -> bool {
    match (decimal.as_decimal(), float.as_float()) {
        (Some(decimal), Some(float)) => decimal.compare_f64(float).is_some_and(Ordering::is_eq),
        _ => false,
    }
}

/// An `int` or `uint` payload as a `decimal`. Total over both ranges, for
/// [`decimal_eq_integer`]'s reason.
fn decimal_of_integer(integer: i128) -> Option<crate::Decimal> {
    crate::Decimal::new(integer.is_negative(), integer.unsigned_abs(), 0)
}

/// Whether an integer and a float denote the same number, **exactly**.
///
/// The float is read at its own binary value rather than at the decimal it
/// prints as, which is what separates this from `crate::Decimal::compare_f64`:
/// an integral `f64` past 2^53 has a shortest printed form that is not the
/// integer it holds, and equating by that form would answer `false` for a pair
/// that really is equal.
fn integer_eq_float(integer: i128, float: f64) -> bool {
    exact_i128(float) == Some(integer)
}

/// How two numeric values order, with `int`, `uint` and `float` read as **one
/// domain** — [`numeric_identical`]'s question for `<`, and exact for the same
/// reason. `nvs_ir::Helper::NumericLt` is the one caller.
///
/// [`None`] only for a `NaN` operand, which is unordered against everything
/// including itself; every `<`/`<=`/`>`/`>=` against one is `false` in PHP,
/// which is what the callers turn a `None` into.
///
/// A `decimal` never arrives — `nvs_ir::Helper::DecimalLt` takes every pairing
/// one side of which is one.
#[must_use]
pub fn numeric_ordering(left: Value, right: Value) -> Option<Ordering> {
    match (left.tag(), right.tag()) {
        (Some(Tag::Int | Tag::Uint), Some(Tag::Int | Tag::Uint)) => {
            Some(integer(left).cmp(&integer(right)))
        }
        (Some(Tag::Int | Tag::Uint), Some(Tag::Float)) => {
            integer_cmp_float(integer(left), right.as_float()?)
        }
        (Some(Tag::Float), Some(Tag::Int | Tag::Uint)) => {
            Some(integer_cmp_float(integer(right), left.as_float()?)?.reverse())
        }
        (Some(Tag::Float), Some(Tag::Float)) => left.as_float()?.partial_cmp(&right.as_float()?),
        // Unreachable from a compiled ordering, whose operands were both
        // numeric before this was chosen — and cheaper to answer than to argue
        // about, exactly like [`numeric_identical`]'s own last arm.
        _ => None,
    }
}

/// An integer against a float, without converting either — the widening that
/// would make this one machine instruction is exactly what loses the answer
/// past 2^53.
///
/// The float's *floor* is the pivot: an integer above it is greater, below it
/// is less, and equal to it decides on whether the float had a fractional part
/// left over. An infinity has no floor and is answered directly.
fn integer_cmp_float(integer: i128, float: f64) -> Option<Ordering> {
    if float.is_nan() {
        return None;
    }
    if float.is_infinite() {
        return Some(if float > 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let floor = float.floor();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "`as` saturates at `i128`'s bounds, which lie far outside the \
                  `[-2^63, 2^64)` an int/uint payload can hold — so a float \
                  that saturates orders correctly against every integer here"
    )]
    let pivot = floor as i128;
    Some(match integer.cmp(&pivot) {
        // Equal to the floor, so the float is the larger unless it *is* the
        // integer: `3 < 3.5` and `3 == 3.0`.
        Ordering::Equal if floor < float => Ordering::Less,
        other => other,
    })
}

/// The integer a float holds exactly, or `None` for a NaN, an infinity or a
/// value with a fractional part.
fn exact_i128(float: f64) -> Option<i128> {
    if !float.is_finite() || float.trunc() != float {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "`as` saturates at `i128`'s bounds, and both of those lie far \
                  outside the `[-2^63, 2^64)` an int/uint payload can hold, so \
                  any float that can equal one converted exactly"
    )]
    let exact = float as i128;
    Some(exact)
}

/// An `int` or a `uint` payload as the integer it denotes.
///
/// Through `i128` because the two ranges only overlap: `u64::MAX` and `-1`
/// share a bit pattern, and reading either as the other is exactly the
/// mistake this returns a wider type to make impossible.
fn integer(value: Value) -> i128 {
    match value.tag() {
        Some(Tag::Uint) => i128::from(value.bits()),
        _ => i128::from(value.bits().cast_signed()),
    }
}

/// A read-only handle on an array a live [`Value`] points at.
///
/// Never dropped: the value the pointer came out of owns the reference, and
/// this handle is only borrowing it for the comparison — the same rule
/// `nvs_stdlib::arr`'s own `borrowed` states, for the same reason.
fn borrowed(ptr: *mut ArrayHeader) -> ManuallyDrop<NvsArray> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array value owns a reference to a live allocation \
                  (see `Value`'s Ownership section), so it is live for this \
                  comparison, and the handle is never dropped"
    )]
    ManuallyDrop::new(unsafe { NvsArray::from_raw(ptr) })
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher as _;

    use super::*;

    /// Both halves of the contract in one place: what the comparison answers,
    /// and that the hash never disagrees with a `true`.
    fn identical(left: Value, right: Value) -> bool {
        let verdict = value_identical(left, right);
        if verdict {
            assert_eq!(
                hashed(left),
                hashed(right),
                "identical values must hash alike"
            );
        }
        verdict
    }

    fn hashed(value: Value) -> u64 {
        let mut hasher = DefaultHasher::new();
        value_hash(value, &mut hasher);
        hasher.finish()
    }

    /// Releases every value a test built, in one place.
    fn release_all(values: &[Value]) {
        for value in values {
            #[expect(unsafe_code, reason = "the test owns exactly these references")]
            unsafe {
                value.release();
            }
        }
    }

    #[test]
    fn a_scalar_is_identical_to_its_own_value() {
        assert!(identical(Value::null(), Value::null()));
        assert!(identical(Value::bool(true), Value::bool(true)));
        assert!(!identical(Value::bool(true), Value::bool(false)));
        assert!(identical(Value::int(-7), Value::int(-7)));
        assert!(!identical(Value::int(-7), Value::int(7)));
        assert!(!identical(Value::null(), Value::bool(false)));
        assert!(!identical(Value::null(), Value::int(0)));
    }

    #[test]
    fn an_int_and_a_uint_are_one_integer_domain() {
        assert!(identical(Value::int(3), Value::uint(3)));
        assert!(identical(Value::uint(3), Value::int(3)));
        // The two payloads that share a bit pattern are still two values.
        assert!(!identical(Value::int(-1), Value::uint(u64::MAX)));
        assert!(!identical(Value::uint(u64::MAX), Value::int(-1)));
    }

    /// `rule:expressions/equality-semantics`'s numeric row reached through [`value_identical`] — the
    /// same row [`numeric_identical`] answers for a statically typed pair, and
    /// the one `Core\Arr`'s strict identity therefore takes too. Every `true`
    /// here also asserts the hash agrees, through [`identical`].
    #[test]
    fn the_four_numeric_representations_are_one_domain() {
        let one = crate::Decimal::parse("1.00").expect("a decimal literal");
        let tenth = crate::Decimal::parse("0.1").expect("a decimal literal");

        assert!(identical(Value::float(1.0), Value::int(1)));
        assert!(identical(Value::int(1), Value::float(1.0)));
        assert!(identical(Value::uint(1), Value::decimal(one)));
        assert!(identical(Value::decimal(one), Value::float(1.0)));
        // A `decimal` reads a `float` at the value it prints as, which is
        // exactly what makes `(0.1 as decimal) == 0.1` true.
        assert!(identical(Value::decimal(tenth), Value::float(0.1)));

        assert!(identical(Value::float(1.5), Value::float(1.5)));
        assert!(!identical(Value::float(1.5), Value::int(1)));
        assert!(!identical(Value::decimal(one), Value::int(2)));
        assert!(!identical(Value::decimal(tenth), Value::float(0.2)));
    }

    /// The trio the two readings of a float disagree over: `2^60` is an `f64`
    /// exactly *and* prints as a different integer, so it is identical to the
    /// `int` it holds and to the `decimal` it prints as while those two are
    /// not identical to each other. All three must still hash alike, and the
    /// integer beside them must not join them.
    #[test]
    fn an_integral_float_past_two_to_the_53_hashes_with_both_of_its_readings() {
        let exact = 1_152_921_504_606_846_976_i64;
        let printed = crate::Decimal::parse("1152921504606847000").expect("a decimal literal");
        let held = crate::Decimal::parse("1152921504606846976").expect("a decimal literal");

        assert!(identical(Value::int(exact), Value::float(exact as f64)));
        assert!(identical(
            Value::decimal(printed),
            Value::float(exact as f64)
        ));
        assert!(identical(Value::decimal(held), Value::int(exact)));
        assert!(!identical(Value::decimal(printed), Value::int(exact)));
        assert_eq!(hashed(Value::decimal(printed)), hashed(Value::int(exact)));

        // The neighbour keeps its own bucket: it merely *rounds* to that
        // float, which is what `coincident_float` refuses.
        assert!(!identical(
            Value::int(exact + 1),
            Value::float(exact as f64)
        ));
        assert_ne!(hashed(Value::int(exact + 1)), hashed(Value::int(exact)));
    }

    #[test]
    fn zero_signs_agree_and_nan_is_identical_to_nothing() {
        assert!(identical(Value::float(0.0), Value::float(-0.0)));
        assert!(!identical(Value::float(f64::NAN), Value::float(f64::NAN)));
        assert!(identical(
            Value::float(f64::INFINITY),
            Value::float(f64::INFINITY)
        ));
        assert!(!identical(
            Value::float(f64::INFINITY),
            Value::float(f64::NEG_INFINITY)
        ));
    }

    #[test]
    fn a_string_compares_by_content_not_by_pointer() {
        let left = Value::str(NvsStr::new(b"abc"));
        let right = Value::str(NvsStr::new(b"abc"));
        let other = Value::str(NvsStr::new(b"abd"));
        assert!(identical(left, right));
        assert!(!identical(left, other));
        assert!(!identical(left, Value::int(0)));
        release_all(&[left, right, other]);
    }

    /// `bytes` takes `string`'s content comparison and none of its domain:
    /// `rule:expressions/equality-semantics` makes the two types disjoint, so a digest is never the
    /// text that spells it, and the hash keys them apart so that pair does not
    /// share a bucket either.
    #[test]
    fn a_bytes_compares_by_content_and_never_against_a_string() {
        let left = Value::bytes(NvsStr::new(b"\xff\x00"));
        let right = Value::bytes(NvsStr::new(b"\xff\x00"));
        let other = Value::bytes(NvsStr::new(b"\xff\x01"));
        assert!(identical(left, right));
        assert!(!identical(left, other));

        let text = Value::str(NvsStr::new(b"abc"));
        let raw = Value::bytes(NvsStr::new(b"abc"));
        assert!(!identical(text, raw));
        assert_ne!(hashed(text), hashed(raw));

        release_all(&[left, right, other, text, raw]);
    }

    #[test]
    fn an_array_is_identical_entry_by_entry_in_order() {
        let mut left = NvsArray::new();
        left.set(NvsStr::new(b"0"), Value::int(1));
        left.set(NvsStr::new(b"1"), Value::str(NvsStr::new(b"two")));

        let mut same = NvsArray::new();
        same.set(NvsStr::new(b"0"), Value::int(1));
        same.set(NvsStr::new(b"1"), Value::str(NvsStr::new(b"two")));

        // The same entries, inserted the other way round.
        let mut reordered = NvsArray::new();
        reordered.set(NvsStr::new(b"1"), Value::str(NvsStr::new(b"two")));
        reordered.set(NvsStr::new(b"0"), Value::int(1));

        let mut shorter = NvsArray::new();
        shorter.set(NvsStr::new(b"0"), Value::int(1));

        let left = Value::array(left);
        let same = Value::array(same);
        let reordered = Value::array(reordered);
        let shorter = Value::array(shorter);

        assert!(identical(left, same));
        assert!(!identical(left, reordered));
        assert!(!identical(left, shorter));
        release_all(&[left, same, reordered, shorter]);
    }

    #[test]
    fn two_handles_on_one_allocation_answer_without_walking() {
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"0"), Value::float(f64::NAN));
        let value = Value::array(array);
        // A `NaN` entry is identical to nothing, so a walk would say `false`
        // — the pointer fast path is what makes an array identical to itself.
        assert!(value_identical(value, value));
        release_all(&[value]);
    }

    #[test]
    fn a_nested_array_is_compared_all_the_way_down() {
        fn nested(leaf: i64) -> Value {
            let mut inner = NvsArray::new();
            inner.set(NvsStr::new(b"0"), Value::int(leaf));
            let mut outer = NvsArray::new();
            outer.set(NvsStr::new(b"0"), Value::array(inner));
            Value::array(outer)
        }

        let left = nested(1);
        let same = nested(1);
        let other = nested(2);
        assert!(identical(left, same));
        assert!(!identical(left, other));
        release_all(&[left, same, other]);
    }

    #[test]
    fn a_deeply_nested_array_costs_heap_rather_than_stack() {
        fn tower(depth: usize, leaf: i64) -> Value {
            let mut value = Value::int(leaf);
            for _ in 0..depth {
                let mut level = NvsArray::new();
                level.set(NvsStr::new(b"0"), value);
                value = Value::array(level);
            }
            value
        }

        // Deep enough that a recursive comparison would have overflowed.
        let left = tower(50_000, 1);
        let same = tower(50_000, 1);
        let other = tower(50_000, 2);
        assert!(value_identical(left, same));
        assert!(!value_identical(left, other));
        // The hash stops at `HASH_DEPTH`, so it agrees on the pair that is
        // identical and is free to agree on the pair that is not.
        assert_eq!(hashed(left), hashed(same));
        release_all(&[left, same, other]);
    }

    #[test]
    fn an_object_is_identical_only_to_itself() {
        use crate::object::{ClassTable, NvsObj};

        let mut table = ClassTable::new();
        let point = table.define("Point", &[] as &[&str], &[]);
        #[expect(unsafe_code, reason = "the table outlives both objects")]
        let (one, two) = unsafe {
            (
                NvsObj::new(table.desc(point)),
                NvsObj::new(table.desc(point)),
            )
        };

        let first = Value::object(one.clone());
        let copy = Value::object(one);
        let second = Value::object(two);

        assert!(identical(first, copy));
        assert!(!identical(first, second));
        release_all(&[first, copy, second]);
    }

    /// `rule:expressions/equality-semantics`'s string row, at the door compiled code actually uses.
    /// The tests above ask [`value_identical`] what a string is; this
    /// one asks [`crate::nvs_str_eq`], which is what `$s == $t` calls, and
    /// pins the divergence that row decides: PHP's `==` read two numeric-
    /// looking strings as numbers, so it answered `true` to every pair here.
    #[test]
    fn equal_strings_compare_as_text_and_never_as_numbers() {
        fn compiled_eq(left: &[u8], right: &[u8]) -> bool {
            let (left, right) = (
                Value::str(NvsStr::new(left)),
                Value::str(NvsStr::new(right)),
            );
            #[expect(
                unsafe_code,
                reason = "both values own a live allocation for this call"
            )]
            let verdict = unsafe {
                crate::nvs_str_eq(
                    left.str_ptr().expect("a Tag::Str value"),
                    right.str_ptr().expect("a Tag::Str value"),
                )
            };
            // The operator's door and the definition must never disagree.
            assert_eq!(verdict, value_identical(left, right));
            release_all(&[left, right]);
            verdict
        }

        assert!(compiled_eq(b"abc", b"abc"));
        assert!(!compiled_eq(b"1", b"01"));
        assert!(!compiled_eq(b"1e3", b"1000"));
        assert!(!compiled_eq(b"0", b"0.0"));
        assert!(!compiled_eq(b" 1", b"1"));
        // No normalization either: "e" plus a combining acute and the
        // precomposed "é" render alike and are two values.
        assert!(!compiled_eq("e\u{301}".as_bytes(), "é".as_bytes()));
    }

    /// `rule:expressions/equality-semantics`'s array row at that same door — [`nvs_array_eq`], which
    /// is what `$a == $b` calls. Same length, same keys in the same order,
    /// every value equal by the table, recursively.
    #[test]
    fn equal_arrays_compare_ordered_and_element_wise() {
        fn compiled_eq(left: Value, right: Value) -> bool {
            #[expect(
                unsafe_code,
                reason = "both values own a live allocation for this call"
            )]
            let verdict = unsafe {
                nvs_array_eq(
                    left.array_ptr().expect("a Tag::Array value"),
                    right.array_ptr().expect("a Tag::Array value"),
                )
            };
            assert_eq!(verdict, value_identical(left, right));
            verdict
        }

        fn list(values: &[i64]) -> Value {
            let mut array = NvsArray::new();
            for (index, value) in values.iter().enumerate() {
                array.set(
                    NvsStr::new(index.to_string().as_bytes()),
                    Value::int(*value),
                );
            }
            Value::array(array)
        }

        let a = list(&[1, 2, 3]);
        let same = list(&[1, 2, 3]);
        let reordered = list(&[3, 2, 1]);
        let shorter = list(&[1, 2]);
        assert!(compiled_eq(a, same));
        assert!(!compiled_eq(a, reordered));
        assert!(!compiled_eq(a, shorter));
        assert!(compiled_eq(a, a));
        release_all(&[a, same, reordered, shorter]);

        // Recursively, and by this table rather than by a second one: a
        // nested list is walked, and its `"1"` entry is a string that never
        // reads as the integer beside it.
        let mut outer = NvsArray::new();
        outer.set(NvsStr::new(b"0"), list(&[1]));
        let mut textual = NvsArray::new();
        textual.set(NvsStr::new(b"0"), {
            let mut inner = NvsArray::new();
            inner.set(NvsStr::new(b"0"), Value::str(NvsStr::new(b"1")));
            Value::array(inner)
        });
        let outer = Value::array(outer);
        let textual = Value::array(textual);
        assert!(!compiled_eq(outer, textual));
        release_all(&[outer, textual]);
    }

    /// `rule:expressions/equality-semantics`'s object row: two instances are equal only when they are
    /// **the same instance**. `nvs-codegen` emits that as one pointer
    /// comparison for a statically typed pair, so the row has no runtime door
    /// of its own — what it does have is a reach into the array row, which is
    /// what bounds that walk, and what this pins.
    #[test]
    fn equal_objects_compare_by_identity() {
        use crate::object::{ClassTable, NvsObj};

        let mut table = ClassTable::new();
        let point = table.define("Point", &[] as &[&str], &[]);
        #[expect(unsafe_code, reason = "the table outlives both objects")]
        let (one, two) = unsafe {
            (
                NvsObj::new(table.desc(point)),
                NvsObj::new(table.desc(point)),
            )
        };

        // Two instances of one class, alike in everything a property walk
        // could look at, are still two values — comparing contents is a
        // `Comparable::compareTo` call a class opts into (`rule:classes/comparable`), never
        // what the operator does.
        let first = Value::object(one.clone());
        let copy = Value::object(one.clone());
        let second = Value::object(two.clone());
        assert!(identical(first, copy));
        assert!(!identical(first, second));

        // And an array holding an object inherits that answer, which is what
        // keeps the array walk from ever descending into one.
        let holding = |object: Value| {
            let mut array = NvsArray::new();
            array.set(NvsStr::new(b"0"), object);
            Value::array(array)
        };
        let holds_first = holding(Value::object(one.clone()));
        let holds_same = holding(Value::object(one));
        let holds_second = holding(Value::object(two));
        #[expect(
            unsafe_code,
            reason = "each value owns a live allocation for these calls"
        )]
        unsafe {
            let ptr = |value: Value| value.array_ptr().expect("a Tag::Array value");
            assert!(nvs_array_eq(ptr(holds_first), ptr(holds_same)));
            assert!(!nvs_array_eq(ptr(holds_first), ptr(holds_second)));
        }
        release_all(&[first, copy, second, holds_first, holds_same, holds_second]);
    }

    /// `rule:expressions/disjoint-comparison-refused`'s numeric row at [`numeric_identical`] itself, which is
    /// where `nvs_ir::Helper::NumericEq` enters it — the edges of the domain
    /// rather than the rows [`the_four_numeric_representations_are_one_domain`]
    /// already walks through [`value_identical`].
    #[test]
    fn a_cross_representation_numeric_pair_is_one_domain() {
        assert!(numeric_identical(Value::int(1), Value::float(1.0)));
        assert!(numeric_identical(Value::float(1.0), Value::uint(1)));
        assert!(numeric_identical(Value::int(3), Value::uint(3)));
        // `-0.0 == 0.0`, the float row's own rule reaching across the domain.
        assert!(numeric_identical(Value::float(-0.0), Value::int(0)));

        assert!(!numeric_identical(Value::int(1), Value::float(1.5)));
        assert!(!numeric_identical(Value::float(f64::NAN), Value::int(0)));
        assert!(!numeric_identical(
            Value::float(f64::INFINITY),
            Value::int(0)
        ));
        // The bit pattern `-1` and `u64::MAX` share, which is exactly what
        // reading both through `i128` is for.
        assert!(!numeric_identical(Value::int(-1), Value::uint(u64::MAX)));
        // `u64::MAX as f64` rounds *up* to 2^64, so it equals no `uint`.
        assert!(!numeric_identical(
            Value::uint(u64::MAX),
            Value::float(18_446_744_073_709_551_615_u64 as f64),
        ));

        // A `decimal` reaches every other representation, at any scale.
        let one = crate::Decimal::parse("1.000").expect("a decimal literal");
        assert!(numeric_identical(Value::decimal(one), Value::int(1)));
        assert!(numeric_identical(Value::uint(1), Value::decimal(one)));
        assert!(numeric_identical(Value::decimal(one), Value::float(1.0)));
        assert!(!numeric_identical(
            Value::decimal(one),
            Value::float(f64::NAN)
        ));
        // `u64::MAX` needs 64 of the mantissa's 96 bits, so no integer is out
        // of a `decimal`'s reach in either direction.
        let large = crate::Decimal::parse("18446744073709551615").expect("a decimal literal");
        assert!(numeric_identical(
            Value::decimal(large),
            Value::uint(u64::MAX)
        ));
        assert!(!numeric_identical(Value::decimal(large), Value::int(-1)));
    }
}
