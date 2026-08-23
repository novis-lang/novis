//! The 16-byte tagged value every helper takes and returns.
//!
//! `docs/implementation-plan.md`'s § *Value representation* owns the decision;
//! this module is its implementation. Two things about it are load-bearing and
//! easy to mistake for accidents:
//!
//! * **It is not NaN-boxed.** PHP semantics need the full `i64` range, which
//!   does not fit alongside a tag in 64 bits. Sixteen bytes instead of eight
//!   is memory spent to buy correct semantics — priority 5 spent on priority 2
//!   in [CLAUDE.md](../../../CLAUDE.md)'s ordering, not an oversight.
//! * **`uint` is a tag, not a wider slot.** [ADR 0007](../../../docs/adr/0007-explicit-type-system.md)
//!   § 4's separate unsigned type therefore costs nothing here.
//!
//! # Where a `Value` actually appears
//!
//! Less often than it looks. Because ADR 0007 makes operand types known by
//! construction, `mwl-codegen`'s baseline tier keeps a statically-typed local
//! in a native register — an `int` is an `i64`, a `string` is a bare
//! [`StrHeader`] pointer — and only *materializes* a `Value` where the ABI
//! demands one: at a call boundary, and eventually as the representation of a
//! `mixed`-typed slot. A tagged value is the interchange format, not the
//! working format.

use std::fmt;

use crate::object::{MwlObj, ObjHeader};
use crate::string::{MwlStr, StrHeader};

/// Which of the runtime's representations a [`Value`]'s payload is.
///
/// The roster is the plan's § *Value representation* verbatim. Four of the ten
/// have no representation behind them yet — see the crate docs' known gap 1 —
/// but they are numbered now so the discriminants never have to move.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tag {
    /// `null`.
    Null = 0,
    /// `bool`; the payload is `0` or `1`.
    Bool = 1,
    /// `int`; the payload is an `i64`'s bit pattern.
    Int = 2,
    /// `uint`; the payload is a `u64` ([ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 4).
    Uint = 3,
    /// `float`; the payload is an `f64`'s bit pattern.
    Float = 4,
    /// `string` or `bytes`; the payload is a [`StrHeader`] pointer and the
    /// value owns one reference to it.
    Str = 5,
    /// `array<T>`; no representation exists yet.
    Array = 6,
    /// A class instance; the payload is an [`ObjHeader`] pointer and the value
    /// owns one reference to it.
    Object = 7,
    /// A closure ([ADR 0031](../../../docs/adr/0031-callable-is-the-only-closure-type.md));
    /// no representation exists yet.
    Closure = 8,
    /// An engine-owned resource handle; no representation exists yet.
    Resource = 9,
}

impl Tag {
    /// The tag a raw byte denotes, or `None` if it denotes none of them.
    ///
    /// Compiled code writes tag bytes, so a [`Value`] read back from the ABI
    /// is not trusted to hold a valid one — that is why [`Value::tag`] is
    /// fallible rather than a field read.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        Some(match byte {
            0 => Self::Null,
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::Uint,
            4 => Self::Float,
            5 => Self::Str,
            6 => Self::Array,
            7 => Self::Object,
            8 => Self::Closure,
            9 => Self::Resource,
            _ => return None,
        })
    }

    /// Whether a payload with this tag owns a reference that must be released.
    #[must_use]
    pub const fn is_refcounted(self) -> bool {
        matches!(self, Self::Str | Self::Array | Self::Object | Self::Closure)
    }
}

/// One MWL value: a tag byte, seven bytes of padding, and an eight-byte
/// payload.
///
/// `#[repr(C)]` with the padding spelled out so compiled code can write the
/// two halves independently and so the layout is identical on every target.
///
/// # Ownership
///
/// A `Value` whose tag [`Tag::is_refcounted`] **owns** one reference to its
/// payload. It is `Copy` because compiled code moves it through registers and
/// stack slots by the word, exactly like every other machine value — copying
/// the bits does not add a reference. Whoever holds the live copy is
/// responsible for exactly one [`Value::release`]; that obligation is what
/// `mwl_ir::InstKind::Retain`/`Release` make explicit in the IR, so the
/// bookkeeping is the compiler's, not this type's.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Value {
    tag: u8,
    pad: [u8; 7],
    bits: u64,
}

const _: () = assert!(std::mem::size_of::<Value>() == 16);
const _: () = assert!(std::mem::align_of::<Value>() == 8);

impl Value {
    /// Byte offset of the tag.
    pub const TAG_OFFSET: usize = std::mem::offset_of!(Self, tag);

    /// Byte offset of the payload.
    pub const BITS_OFFSET: usize = std::mem::offset_of!(Self, bits);

    const fn new(tag: Tag, bits: u64) -> Self {
        Self {
            tag: tag as u8,
            pad: [0; 7],
            bits,
        }
    }

    /// `null`.
    #[must_use]
    pub const fn null() -> Self {
        Self::new(Tag::Null, 0)
    }

    /// A `bool`.
    #[must_use]
    pub const fn bool(value: bool) -> Self {
        Self::new(Tag::Bool, value as u64)
    }

    /// An `int`.
    #[must_use]
    pub const fn int(value: i64) -> Self {
        Self::new(Tag::Int, value.cast_unsigned())
    }

    /// A `uint`.
    #[must_use]
    pub const fn uint(value: u64) -> Self {
        Self::new(Tag::Uint, value)
    }

    /// A `float`.
    #[must_use]
    pub const fn float(value: f64) -> Self {
        Self::new(Tag::Float, value.to_bits())
    }

    /// A `string`, taking over the handle's reference.
    #[must_use]
    pub fn str(value: MwlStr) -> Self {
        Self::new(Tag::Str, value.into_raw() as usize as u64)
    }

    /// A class instance, taking over the handle's reference.
    #[must_use]
    pub fn object(value: MwlObj) -> Self {
        Self::new(Tag::Object, value.into_raw() as usize as u64)
    }

    /// Reassembles a value from bits compiled code produced.
    ///
    /// # Safety
    ///
    /// `bits` must be a payload valid for `tag` — in particular, a
    /// [`Tag::Str`] payload must be a live [`StrHeader`] pointer whose
    /// reference the new value takes over.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the tag/payload agreement is the caller's obligation to state"
    )]
    pub const unsafe fn from_parts(tag: Tag, bits: u64) -> Self {
        Self::new(tag, bits)
    }

    /// This value's tag, or `None` if the tag byte denotes no representation.
    #[must_use]
    pub const fn tag(self) -> Option<Tag> {
        Tag::from_byte(self.tag)
    }

    /// The raw tag byte, whether or not it denotes a representation.
    #[must_use]
    pub const fn tag_byte(self) -> u8 {
        self.tag
    }

    /// The raw payload.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.bits
    }

    /// The payload as a `bool`, if this value is one.
    #[must_use]
    pub const fn as_bool(self) -> Option<bool> {
        match self.tag() {
            Some(Tag::Bool) => Some(self.bits != 0),
            _ => None,
        }
    }

    /// The payload as an `int`, if this value is one.
    #[must_use]
    pub const fn as_int(self) -> Option<i64> {
        match self.tag() {
            Some(Tag::Int) => Some(self.bits.cast_signed()),
            _ => None,
        }
    }

    /// The payload as a `uint`, if this value is one.
    #[must_use]
    pub const fn as_uint(self) -> Option<u64> {
        match self.tag() {
            Some(Tag::Uint) => Some(self.bits),
            _ => None,
        }
    }

    /// The payload as a `float`, if this value is one.
    #[must_use]
    pub const fn as_float(self) -> Option<f64> {
        match self.tag() {
            Some(Tag::Float) => Some(f64::from_bits(self.bits)),
            _ => None,
        }
    }

    /// The string payload's bytes, if this value is a string.
    ///
    /// The returned borrow is tied to this value's own borrow, which is
    /// deliberately conservative: the allocation actually outlives it, and the
    /// reference this value owns is what keeps it alive for at least that
    /// long.
    #[must_use]
    pub fn as_str_bytes(&self) -> Option<&[u8]> {
        let ptr = self.str_ptr()?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Str value owns a reference to a live allocation \
                      (see this type's Ownership section), so it is live for \
                      at least this borrow"
        )]
        Some(unsafe { MwlStr::bytes_of(ptr) })
    }

    /// The string payload's header pointer, if this value is a string.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload of a Tag::Str value is a pointer that was widened to u64 by `Value::str`, so narrowing it back is exact on every target, including the 32-bit wasm32 one of ADR 0025"
    )]
    pub const fn str_ptr(self) -> Option<*mut StrHeader> {
        match self.tag() {
            Some(Tag::Str) => Some(self.bits as usize as *mut StrHeader),
            _ => None,
        }
    }

    /// The object payload's header pointer, if this value is a class instance.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload of a Tag::Object value is a pointer that was widened to u64 by `Value::object`, so narrowing it back is exact on every target, including the 32-bit wasm32 one of ADR 0025"
    )]
    pub const fn obj_ptr(self) -> Option<*mut ObjHeader> {
        match self.tag() {
            Some(Tag::Object) => Some(self.bits as usize as *mut ObjHeader),
            _ => None,
        }
    }

    /// Adds a reference to a refcounted payload — `mwl_ir::InstKind::Retain`.
    ///
    /// A non-refcounted value is left alone, so callers need not branch on the
    /// tag themselves.
    ///
    /// # Safety
    ///
    /// A refcounted payload must refer to a live allocation.
    #[expect(
        unsafe_code,
        reason = "the payload's liveness is the caller's obligation to state"
    )]
    pub unsafe fn retain(self) {
        if let Some(ptr) = self.str_ptr() {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees the payload is live; \
                          `mwl_str_retain` only increments in place"
            )]
            unsafe {
                crate::string::mwl_str_retain(ptr);
            }
        } else if let Some(ptr) = self.obj_ptr() {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees the payload is live; \
                          `mwl_object_retain` only increments in place"
            )]
            unsafe {
                crate::object::mwl_object_retain(ptr);
            }
        }
    }

    /// Drops the reference a refcounted payload owns —
    /// `mwl_ir::InstKind::Release`.
    ///
    /// A non-refcounted value is left alone. An `Array`/`Closure` payload is
    /// *also* left alone today: those representations do not exist yet, so
    /// nothing can construct one to leak (crate docs, known gap 1).
    ///
    /// # Safety
    ///
    /// This value must own the reference being dropped, and must not be
    /// released twice.
    #[expect(
        unsafe_code,
        reason = "owning the reference is the caller's obligation to state"
    )]
    pub unsafe fn release(self) {
        if let Some(ptr) = self.str_ptr() {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees this value owns exactly the \
                          reference `mwl_str_release` drops"
            )]
            unsafe {
                crate::string::mwl_str_release(ptr);
            }
        } else if let Some(ptr) = self.obj_ptr() {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees this value owns exactly the \
                          reference `mwl_object_release` drops"
            )]
            unsafe {
                crate::object::mwl_object_release(ptr);
            }
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::null()
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tag() {
            Some(Tag::Null) => f.write_str("null"),
            Some(Tag::Bool) => write!(f, "bool({})", self.bits != 0),
            Some(Tag::Int) => write!(f, "int({})", self.bits.cast_signed()),
            Some(Tag::Uint) => write!(f, "uint({})", self.bits),
            Some(Tag::Float) => write!(f, "float({})", f64::from_bits(self.bits)),
            Some(Tag::Str) => {
                let bytes = self.as_str_bytes().unwrap_or_default();
                write!(f, "string({:?})", String::from_utf8_lossy(bytes))
            }
            Some(tag) => write!(f, "{tag:?}(0x{:016x})", self.bits),
            None => write!(f, "<invalid tag {}>(0x{:016x})", self.tag, self.bits),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_sixteen_bytes_with_the_tag_first() {
        assert_eq!(std::mem::size_of::<Value>(), 16);
        assert_eq!(Value::TAG_OFFSET, 0);
        assert_eq!(Value::BITS_OFFSET, 8);
    }

    #[test]
    fn every_scalar_round_trips() {
        assert_eq!(Value::null().tag(), Some(Tag::Null));
        assert_eq!(Value::bool(true).as_bool(), Some(true));
        assert_eq!(Value::bool(false).as_bool(), Some(false));
        assert_eq!(Value::int(i64::MIN).as_int(), Some(i64::MIN));
        assert_eq!(Value::int(-1).as_int(), Some(-1));
        assert_eq!(Value::uint(u64::MAX).as_uint(), Some(u64::MAX));
        assert_eq!(Value::float(-0.5).as_float(), Some(-0.5));
    }

    #[test]
    fn an_accessor_refuses_the_wrong_tag() {
        assert_eq!(Value::int(1).as_uint(), None);
        assert_eq!(Value::uint(1).as_int(), None);
        assert_eq!(Value::float(1.0).as_bool(), None);
        assert_eq!(Value::null().as_str_bytes(), None);
    }

    #[test]
    fn an_out_of_range_tag_byte_denotes_nothing() {
        #[expect(unsafe_code, reason = "constructing the shape a miscompile would")]
        let bogus = unsafe { Value::from_parts(Tag::Null, 0) };
        assert_eq!(bogus.tag(), Some(Tag::Null));
        assert_eq!(Tag::from_byte(10), None);
        assert_eq!(Tag::from_byte(u8::MAX), None);
    }

    #[test]
    fn a_string_value_owns_one_reference() {
        let s = MwlStr::new(b"hi");
        let value = Value::str(s.clone());
        assert_eq!(s.refcount(), 2);
        assert_eq!(value.as_str_bytes(), Some(&b"hi"[..]));
        #[expect(
            unsafe_code,
            reason = "the value owns exactly the reference taken above"
        )]
        unsafe {
            value.release();
        }
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn retain_and_release_pair_up_on_a_string_value() {
        let s = MwlStr::new(b"hi");
        let value = Value::str(s.clone());
        #[expect(unsafe_code, reason = "the payload is kept alive by `s`")]
        unsafe {
            value.retain();
            assert_eq!(s.refcount(), 3);
            value.release();
            value.release();
        }
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn releasing_a_scalar_does_nothing() {
        #[expect(unsafe_code, reason = "a scalar owns no reference at all")]
        unsafe {
            Value::int(7).release();
            Value::null().retain();
        }
    }

    #[test]
    fn debug_names_the_representation() {
        assert_eq!(format!("{:?}", Value::null()), "null");
        assert_eq!(format!("{:?}", Value::int(-3)), "int(-3)");
        assert_eq!(format!("{:?}", Value::uint(3)), "uint(3)");
        assert_eq!(format!("{:?}", Value::bool(true)), "bool(true)");
        let value = Value::str(MwlStr::new(b"hi"));
        assert_eq!(format!("{value:?}"), "string(\"hi\")");
        #[expect(unsafe_code, reason = "the value owns the reference it releases")]
        unsafe {
            value.release();
        }
    }
}
