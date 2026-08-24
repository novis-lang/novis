//! Strict identity over two [`Value`]s, and the hash that agrees with it.
//!
//! One comparison, defined once, because seven `Core\Arr` members and every
//! `ObjectSet`/`ObjectMap` operation ask the same question —
//! `docs/spec/01-core-library.md` § 2 says `contains`, `keyOf`, `unique`,
//! `diff` and `intersect` "compare by **strict identity**", and a second
//! answer living in `mwl-stdlib` would be a second set of PHP-divergence
//! decisions nothing keeps in step. It is *not* wired to the `===` operator
//! yet: `mwl_ir::BinOp::Eq` still lowers both equalities to one machine
//! comparison, which is correct exactly while every operand is a scalar of a
//! statically known type.
//!
//! # What identity means, one row per representation
//!
//! PHP's `===` is the starting point, and each row below either matches it or
//! says why it cannot:
//!
//! * **`null`** — one value, so two of them are identical.
//! * **`bool`** — equal payloads.
//! * **`int` and `uint` are one integer domain.** `3` and `3 as uint` are
//!   identical. [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 4
//!   makes `uint` a *range* restriction over the same integers rather than a
//!   different value space, so the alternative — two spellings of three that
//!   `contains` cannot match — would be a trap with nothing to gain. The
//!   comparison goes through `i128` so no large `uint` is ever reinterpreted
//!   as a negative `int`. PHP has no `uint`, so nothing is diverged from.
//! * **`float` never crosses to an integer**, matching PHP's `1 === 1.0`
//!   being `false`. Two floats compare with `==`, not by bits: `0.0` and
//!   `-0.0` are identical and `NaN` is identical to nothing, both PHP's
//!   answers, and both the opposite of what `total_cmp` (which
//!   `Core\Arr::sort` needs, and which is a different question) would say.
//! * **`decimal` compares by value, never by scale, and crosses to neither
//!   `int` nor `float`.** [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md)
//!   § 4 states the first half outright — "scale is carried for rendering, and
//!   does not affect equality or hashing", so `1.10` and `1.1000` are one
//!   value — and the second half is the `float` row's rule applied to a third
//!   numeric type. That is why this needs a row rather than falling into the
//!   opaque-handle case below: two equal decimals at different scales hold
//!   different bits.
//! * **`string`/`bytes`** — by content, never by pointer, so a computed
//!   string matches a literal. [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)
//!   makes a `string` valid UTF-8 but does *not* normalize it, so this is a
//!   byte comparison and `"é"` written two ways is two values.
//! * **An `array` is compared entry by entry, in order** — the same count,
//!   the same keys in the same positions, and identical values — which is
//!   PHP's `===` on arrays exactly. Two handles on one allocation short-
//!   circuit to `true` before anything is walked.
//! * **An object is identical only to itself.** Pointer identity, PHP's
//!   answer, and the one that stays coherent under
//!   [ADR 0013](../../../docs/adr/0013-comparable-interface.md): comparing
//!   two instances *by their contents* is a `Comparable::compareTo` call the
//!   class opts into, so a member that walked properties here would be the
//!   property-walk fallback that ADR removed. A closure is an object
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
//! object — the one shape that can form a cycle (crate docs, known gap 8) —
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
//! and is, in two places — a `NaN` hashes like any other float though it is
//! identical to nothing, and an array is hashed only [`HASH_DEPTH`] levels
//! deep — because a collision costs one extra [`value_identical`] call and
//! bounded work is what keeps the hash itself immune to a deep input.
//!
//! It writes into a caller-supplied [`Hasher`] rather than returning a `u64`
//! so the caller's own [`std::collections::HashSet`] supplies the randomly
//! keyed [`std::collections::hash_map::RandomState`]. A fixed-key hash here
//! would hand an attacker collision-crafting against a `Core` member, which
//! is the exact failure the set index was introduced to avoid.

use std::hash::Hasher;
use std::mem::ManuallyDrop;

use crate::array::{ArrayHeader, MwlArray};
use crate::string::MwlStr;
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
#[must_use]
pub fn value_identical(left: Value, right: Value) -> bool {
    let mut worklist = vec![(left, right)];
    while let Some((left, right)) = worklist.pop() {
        if !shallow_identical(left, right, &mut worklist) {
            return false;
        }
    }
    true
}

/// One pair's verdict, pushing an array's entries onto `worklist` rather than
/// descending into them.
fn shallow_identical(left: Value, right: Value, worklist: &mut Vec<(Value, Value)>) -> bool {
    match (left.tag(), right.tag()) {
        (Some(Tag::Null), Some(Tag::Null)) => true,
        (Some(Tag::Bool), Some(Tag::Bool)) => left.bits() == right.bits(),
        (Some(Tag::Int | Tag::Uint), Some(Tag::Int | Tag::Uint)) => integer(left) == integer(right),
        (Some(Tag::Float), Some(Tag::Float)) => {
            // `==`, not the bit pattern: `-0.0 == 0.0` and `NaN != NaN` are
            // both PHP's answers, and both differ from comparing `bits()`.
            f64::from_bits(left.bits()) == f64::from_bits(right.bits())
        }
        (Some(Tag::Decimal), Some(Tag::Decimal)) => match (left.as_decimal(), right.as_decimal()) {
            (Some(a), Some(b)) => a.compare(b).is_eq(),
            _ => false,
        },
        (Some(Tag::Str), Some(Tag::Str)) => left.as_str_bytes() == right.as_str_bytes(),
        (Some(Tag::Array), Some(Tag::Array)) => {
            let (Some(a), Some(b)) = (left.array_ptr(), right.array_ptr()) else {
                return false;
            };
            std::ptr::eq(a, b) || entries_identical(a, b, worklist)
        }
        // Every remaining representation is an opaque handle: identical to
        // itself and to nothing else. That covers an object (this module's
        // docs say why pointer identity is the answer ADR 0013 leaves room
        // for), the two tags nothing constructs yet, and the tag byte only a
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
                if left.key_at(left_slot).as_ref().map(MwlStr::as_bytes)
                    != right.key_at(right_slot).as_ref().map(MwlStr::as_bytes)
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
/// same bytes — see this module's docs for the two places it is deliberately
/// coarser than the comparison.
pub fn value_hash<H: Hasher>(value: Value, state: &mut H) {
    hash_to_depth(value, state, HASH_DEPTH);
}

/// [`value_hash`], carrying how many more levels of array it may descend.
fn hash_to_depth<H: Hasher>(value: Value, state: &mut H, depth: u32) {
    // One discriminant per *family*, not per tag: `int` and `uint` share one
    // because they are one integer domain, and every opaque handle shares one
    // because it is hashed as its bits either way.
    match value.tag() {
        Some(Tag::Null) => state.write_u8(0),
        Some(Tag::Bool) => {
            state.write_u8(1);
            state.write_u64(value.bits());
        }
        Some(Tag::Int | Tag::Uint) => {
            state.write_u8(2);
            state.write_i128(integer(value));
        }
        Some(Tag::Float) => {
            state.write_u8(3);
            // `-0.0` is identical to `0.0`, so both hash as `0.0`.
            let float = f64::from_bits(value.bits());
            state.write_u64(if float == 0.0 { 0.0f64 } else { float }.to_bits());
        }
        Some(Tag::Decimal) => {
            state.write_u8(7);
            // Trailing zeros are stripped first, because they are exactly the
            // difference identity ignores: `1.10` and `1.1000` are one value
            // and must be one hash.
            let Some(value) = value.as_decimal().map(crate::Decimal::reduced) else {
                return;
            };
            state.write_u8(u8::from(value.is_negative()));
            state.write_u128(value.mantissa());
            state.write_u8(value.scale());
        }
        Some(Tag::Str) => {
            state.write_u8(4);
            state.write(value.as_str_bytes().unwrap_or_default());
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
/// `mwl_stdlib::arr`'s own `borrowed` states, for the same reason.
fn borrowed(ptr: *mut ArrayHeader) -> ManuallyDrop<MwlArray> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array value owns a reference to a live allocation \
                  (see `Value`'s Ownership section), so it is live for this \
                  comparison, and the handle is never dropped"
    )]
    ManuallyDrop::new(unsafe { MwlArray::from_raw(ptr) })
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

    #[test]
    fn a_float_never_crosses_to_an_integer() {
        assert!(!identical(Value::float(1.0), Value::int(1)));
        assert!(!identical(Value::int(1), Value::float(1.0)));
        assert!(identical(Value::float(1.5), Value::float(1.5)));
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
        let left = Value::str(MwlStr::new(b"abc"));
        let right = Value::str(MwlStr::new(b"abc"));
        let other = Value::str(MwlStr::new(b"abd"));
        assert!(identical(left, right));
        assert!(!identical(left, other));
        assert!(!identical(left, Value::int(0)));
        release_all(&[left, right, other]);
    }

    #[test]
    fn an_array_is_identical_entry_by_entry_in_order() {
        let mut left = MwlArray::new();
        left.set(MwlStr::new(b"0"), Value::int(1));
        left.set(MwlStr::new(b"1"), Value::str(MwlStr::new(b"two")));

        let mut same = MwlArray::new();
        same.set(MwlStr::new(b"0"), Value::int(1));
        same.set(MwlStr::new(b"1"), Value::str(MwlStr::new(b"two")));

        // The same entries, inserted the other way round.
        let mut reordered = MwlArray::new();
        reordered.set(MwlStr::new(b"1"), Value::str(MwlStr::new(b"two")));
        reordered.set(MwlStr::new(b"0"), Value::int(1));

        let mut shorter = MwlArray::new();
        shorter.set(MwlStr::new(b"0"), Value::int(1));

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
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"0"), Value::float(f64::NAN));
        let value = Value::array(array);
        // A `NaN` entry is identical to nothing, so a walk would say `false`
        // — the pointer fast path is what makes an array identical to itself.
        assert!(value_identical(value, value));
        release_all(&[value]);
    }

    #[test]
    fn a_nested_array_is_compared_all_the_way_down() {
        fn nested(leaf: i64) -> Value {
            let mut inner = MwlArray::new();
            inner.set(MwlStr::new(b"0"), Value::int(leaf));
            let mut outer = MwlArray::new();
            outer.set(MwlStr::new(b"0"), Value::array(inner));
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
                let mut level = MwlArray::new();
                level.set(MwlStr::new(b"0"), value);
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
        use crate::object::{ClassTable, MwlObj};

        let mut table = ClassTable::new();
        let point = table.define("Point", 0, &[]);
        #[expect(unsafe_code, reason = "the table outlives both objects")]
        let (one, two) = unsafe {
            (
                MwlObj::new(table.desc(point)),
                MwlObj::new(table.desc(point)),
            )
        };

        let first = Value::object(one.clone());
        let copy = Value::object(one);
        let second = Value::object(two);

        assert!(identical(first, copy));
        assert!(!identical(first, second));
        release_all(&[first, copy, second]);
    }
}
