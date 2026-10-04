//! The 16-byte tagged value every helper takes and returns.
//!
//! `docs/implementation-plan.md`'s § *Value representation* owns the decision;
//! this module is its implementation. Two things about it are load-bearing and
//! easy to mistake for accidents:
//!
//! * **It is not NaN-boxed.** PHP semantics need the full `i64` range, which
//!   does not fit alongside a tag in 64 bits. Sixteen bytes instead of eight
//!   is memory spent to buy correct semantics — priority 5 spent on priority 2
//!   in [AGENTS.md](/AGENTS.md)'s ordering, not an oversight.
//! * **`uint` is a tag, not a wider slot.** `rule:types/arithmetic`'s separate unsigned type therefore costs nothing here.
//!
//! # Where a `Value` actually appears
//!
//! Less often than it looks. Because `rule:types/declaration` makes operand types known by
//! construction, `nvs-codegen`'s baseline tier keeps a statically-typed local
//! in a native register — an `int` is an `i64`, a `string` is a bare
//! [`StrHeader`] pointer — and only *materializes* a `Value` where the ABI
//! demands one: at a call boundary, and eventually as the representation of a
//! `mixed`-typed slot. A tagged value is the interchange format, not the
//! working format.

use std::fmt;

use crate::array::{ArrayHeader, NvsArray};
use crate::decimal::Decimal;
use crate::object::{ClassDesc, NvsObj, ObjHeader, ShapeCodec};
use crate::string::{NvsStr, StrHeader};

/// Which of the runtime's representations a [`Value`]'s payload is.
///
/// The roster is the plan's § *Value representation*, and every entry on it
/// has a representation behind it. A discriminant never moves:
/// `nvs_ir::lower::param_tag_nibble` writes these numbers down in a crate that
/// cannot name this type, and compiled code embeds them, so a discriminant
/// that moves moves in two crates at once and in every artifact already built
/// against the old one. Every discriminant is also below 15, the nibble
/// `nvs_ir::lower::FN_PARAM_TAG_ANY` keeps for "any tag".
/// [`Self::Unset`] is not on that roster at all: it is a storage
/// state rather than a value, and its own doc comment says why it lives here.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tag {
    /// `null`.
    Null = 0,
    /// `bool`; the payload is `0` or `1`.
    Bool = 1,
    /// `int`; the payload is an `i64`'s bit pattern.
    Int = 2,
    /// `uint`; the payload is a `u64` (`rule:types/arithmetic`).
    Uint = 3,
    /// `float`; the payload is an `f64`'s bit pattern.
    Float = 4,
    /// `string`; the payload is a [`StrHeader`] pointer and the value owns
    /// one reference to it. [`Self::Bytes`] points at the same heap shape and
    /// is a tag of its own anyway — the crate docs' § *`bytes` is a tag, not a
    /// second heap shape* says why.
    Str = 5,
    /// `array<T>`; the payload is an [`ArrayHeader`] pointer and the value owns
    /// one reference to it.
    Array = 6,
    /// A class instance; the payload is an [`ObjHeader`] pointer and the value
    /// owns one reference to it.
    ///
    /// A `rule:types/callable-values` callable is one of these — one field
    /// per capture, one `invoke` method — so it needs no tag of its own;
    /// `nvs_ir::lower::lower_anon_fn` owns that decision and says why it reuses
    /// the object machinery rather than adding a second heap shape, and
    /// [`crate::callable`] is what reads a callable back out of an object value.
    /// An engine-owned handle is a `Core` class holding a key into its own
    /// context's table, for the reason `nvs_stdlib::instance`'s module doc
    /// gives, so neither shape is a row of its own here.
    Object = 7,
    /// A case of an `int`-backed enum; the payload is the case's `i64`, exactly
    /// as [`Self::Int`]'s is (`rule:enums/representation`).
    ///
    /// Only a reader that asks *which type* a value is sees this tag:
    /// [`Value::exact_tag`]. [`Value::tag`] answers [`Self::Int`] for it, so
    /// every reader that decodes a payload reads a case as its backing integer.
    EnumInt = 8,
    /// A case of a `uint`-backed enum; the payload is the case's `u64`, read
    /// as [`Self::Uint`] by [`Value::tag`] for [`Self::EnumInt`]'s reason.
    EnumUint = 9,
    /// `decimal` —`rule:types/decimal`'s
    /// scalar, and the one tag whose value does **not** fit in the payload
    /// alone: its 96-bit mantissa spans the padding bytes too, so a `decimal`
    /// is the whole sixteen bytes rather than a tag plus eight. See
    /// [`crate::decimal`]'s own module docs for the bit positions and why one
    /// `Value` shape carries it rather than a representation of its own.
    Decimal = 10,
    /// `bytes` — `rule:types/bytes`'s
    /// binary scalar. The payload is a [`StrHeader`] pointer and the value
    /// owns one reference to it, exactly as [`Self::Str`] does: the two types
    /// differ only in the UTF-8 promise, which is a checker property rather
    /// than a layout one. What this tag buys is telling them apart once the
    /// static type is gone; the crate docs' § *`bytes` is a tag, not a second
    /// heap shape* is the one home for that decision and for what it spends.
    Bytes = 11,
    /// **Not a value**: the "never written" storage state
    /// `rule:classes/an-unwritten-property-read-throws` requires of a property slot, distinct from every legal value
    /// including [`Self::Null`]. The payload is zero.
    ///
    /// It is a tag rather than a flag beside the slot for that section's own
    /// reason — one more discriminant on a representation that already
    /// carries one costs **zero additional bytes per property** — and it is
    /// deliberately outside the type system: nothing in `nvs_types` produces
    /// it, no expression evaluates to it, and every read that could hand one
    /// to user code turns it into a checked throw first
    /// ([`crate::nvs_object_slot_get`], and `nvs_ir::lower`'s guard on the
    /// compiled read). It is not refcounted, so a slot still holding one
    /// sweeps like a `null` when the object is freed.
    ///
    /// Only a `lateinit` property (`rule:classes/lateinit`) can currently reach the state:
    /// `rule:classes/definite-property-initialization` discharges every other non-nullable property at its
    /// constructor. `Core\Reflect`'s constructor-bypassing instantiation
    /// (`rule:tooling/reflection-and-source-parsing-are-core-features`, M6) is the other one § 3 names, and it will need no new
    /// state — only the same stamp on every slot it does not fill.
    Unset = 12,
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
            8 => Self::EnumInt,
            9 => Self::EnumUint,
            10 => Self::Decimal,
            11 => Self::Bytes,
            12 => Self::Unset,
            _ => return None,
        })
    }

    /// How a diagnostic spells this tag: the Novis type name a program would
    /// have written, not the variant's own.
    ///
    /// Deliberately coarser than a type: [`Self::Str`] answers `string` for
    /// every `string` there is, and [`Self::Object`] answers `object` for
    /// every class — a tag is all a message raised from compiled code has,
    /// and claiming more than that would be claiming the concrete type is
    /// known when it is not.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Float => "float",
            Self::Str => "string",
            Self::Array => "array",
            Self::Object => "object",
            Self::EnumInt | Self::EnumUint => "enum",
            Self::Decimal => "decimal",
            Self::Bytes => "bytes",
            // The one entry that is not a Novis type name, because the
            // state is not a value: nothing user code can hold has this
            // tag, so a message that reaches it is reporting a slot, not an
            // operand.
            Self::Unset => "an unset property",
        }
    }

    /// Whether a payload with this tag owns a reference that must be released.
    #[must_use]
    pub const fn is_refcounted(self) -> bool {
        matches!(self, Self::Str | Self::Bytes | Self::Array | Self::Object)
    }

    /// The tag whose payload this one's payload is: an enum case's backing
    /// integer, and every other tag itself.
    #[must_use]
    pub const fn payload_tag(self) -> Self {
        match self {
            Self::EnumInt => Self::Int,
            Self::EnumUint => Self::Uint,
            other => other,
        }
    }
}

/// One Novis value: a tag byte, seven bytes of padding, and an eight-byte
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
/// `nvs_ir::InstKind::Retain`/`Release` make explicit in the IR, so the
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

    /// The "never written" storage state — see [`Tag::Unset`], which owns the
    /// decision. Not a value: this is what a slot holds, never what an
    /// expression produces.
    ///
    /// The payload is zero deliberately, and that is what the *compiled* read
    /// tests. A slot in this state belongs to a `lateinit` property, whose
    /// declared type `rule:classes/lateinit-restrictions` restricts to a non-nullable class or
    /// interface — one pointer, which is null in this state and in no other —
    /// so `nvs_ir::lower`'s guard is one compare against the payload it had
    /// already loaded rather than a second load of the tag byte. The tag is
    /// what a reader holding the *whole* slot goes by
    /// ([`crate::nvs_object_slot_get`]), where there is no declared type to
    /// make that argument from.
    #[must_use]
    pub const fn unset() -> Self {
        Self::new(Tag::Unset, 0)
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

    /// A `decimal` — the one representation that is **not** a tag plus a
    /// payload: its sign, scale and 96-bit mantissa fill all sixteen bytes,
    /// so this writes the whole image rather than going through
    /// [`Self::new`]. [`crate::decimal`]'s own docs own the bit positions.
    #[must_use]
    pub fn decimal(value: Decimal) -> Self {
        let bytes = value.to_bits().to_le_bytes();
        let mut pad = [0u8; 7];
        pad.copy_from_slice(&bytes[1..8]);
        let mut bits = [0u8; 8];
        bits.copy_from_slice(&bytes[8..16]);
        Self {
            tag: bytes[0],
            pad,
            bits: u64::from_le_bytes(bits),
        }
    }

    /// The payload as a `decimal`, if this value is one — the inverse of
    /// [`Self::decimal`].
    #[must_use]
    pub fn as_decimal(self) -> Option<Decimal> {
        let mut bytes = [0u8; 16];
        bytes[0] = self.tag;
        bytes[1..8].copy_from_slice(&self.pad);
        bytes[8..16].copy_from_slice(&self.bits.to_le_bytes());
        Decimal::from_bits(u128::from_le_bytes(bytes))
    }

    /// A `string`, taking over the handle's reference.
    #[must_use]
    pub fn str(value: NvsStr) -> Self {
        Self::new(Tag::Str, value.into_raw() as usize as u64)
    }

    /// A `bytes`, taking over the handle's reference.
    ///
    /// The handle is an [`NvsStr`] because a `bytes` *is* one, minus the UTF-8
    /// promise — see [`Tag::Bytes`]. `string as bytes` is therefore this
    /// constructor over a retained payload rather than a copy, which is what
    /// makes `rule:types/conversion`'s
    /// "total, free" row literally free.
    #[must_use]
    pub fn bytes(value: NvsStr) -> Self {
        Self::new(Tag::Bytes, value.into_raw() as usize as u64)
    }

    /// A class instance, taking over the handle's reference.
    #[must_use]
    pub fn object(value: NvsObj) -> Self {
        Self::new(Tag::Object, value.into_raw() as usize as u64)
    }

    /// An array, taking over the handle's reference.
    #[must_use]
    pub fn array(value: NvsArray) -> Self {
        Self::new(Tag::Array, value.into_raw() as usize as u64)
    }

    /// Wraps a raw object pointer whose reference the new value takes over —
    /// what a primitive that was handed a bare pointer uses to reach the one
    /// release path in [`crate::release`].
    pub(crate) fn from_obj_ptr(ptr: *mut ObjHeader) -> Self {
        Self::new(Tag::Object, ptr as usize as u64)
    }

    /// Wraps a raw array pointer whose reference the new value takes over —
    /// see [`Value::from_obj_ptr`].
    pub(crate) fn from_array_ptr(ptr: *mut ArrayHeader) -> Self {
        Self::new(Tag::Array, ptr as usize as u64)
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

    /// How this value's payload is read, or `None` if the tag byte denotes no
    /// representation. An enum case answers its backing integer's tag
    /// ([`Tag::payload_tag`]), so a reader that decodes a payload needs no arm
    /// for an enum.
    #[must_use]
    pub const fn tag(self) -> Option<Tag> {
        match Tag::from_byte(self.tag) {
            Some(tag) => Some(tag.payload_tag()),
            None => None,
        }
    }

    /// This value's own tag, or `None` if the tag byte denotes no
    /// representation. It differs from [`Self::tag`] only for an enum case,
    /// and it is what a reader uses that asks which type a value is: a
    /// condition (`rule:enums/truthiness`), a declared slot's write check, a
    /// callable parameter's check.
    #[must_use]
    pub const fn exact_tag(self) -> Option<Tag> {
        Tag::from_byte(self.tag)
    }

    /// An enum case: `EnumInt` for an `int`-backed enum, `EnumUint` for a
    /// `uint`-backed one. A test builds one with this; compiled code writes
    /// the tag itself.
    ///
    /// # Panics
    ///
    /// If `tag` is not one of the two enum tags.
    #[must_use]
    pub const fn enum_case(tag: Tag, bits: u64) -> Self {
        assert!(
            matches!(tag, Tag::EnumInt | Tag::EnumUint),
            "not an enum tag"
        );
        Self::new(tag, bits)
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
        Some(unsafe { NvsStr::bytes_of(ptr) })
    }

    /// The string payload as text, if this value is a string.
    ///
    /// This is the reader a caller that means text wants, and it costs the tag
    /// check alone: no `from_utf8` pass, because the tag it checks **is** the
    /// UTF-8 guarantee — see [`NvsStr::text_of`] and `string.rs`'s
    /// § *Reading the payload as text*. A `Tag::Bytes` value answers `None`
    /// here for the reason [`Self::as_bytes`] gives below.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        let ptr = self.str_ptr()?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Str value owns a reference to a live allocation \
                      (see this type's Ownership section), so it is live for \
                      at least this borrow — and the tag is what says the \
                      payload is text rather than a `bytes`'s octets"
        )]
        Some(unsafe { NvsStr::text_of(ptr) })
    }

    /// How many extended grapheme clusters this value holds, if it is a
    /// string — `rule:types/string-is-utf8`'s unit, answered from the header rather than
    /// rescanned once anything has asked before.
    ///
    /// The safe seam `nvs_stdlib::granularity` reads: the tag check is what
    /// discharges [`NvsStr::grapheme_count_of`]'s obligation, exactly as it
    /// does for [`Self::as_text`], and a `Tag::Bytes` value answers `None`
    /// here because bytes have no clusters to count — `string.rs`'s
    /// § *The cached grapheme count* is where that split is stated.
    #[must_use]
    pub fn grapheme_count(&self) -> Option<usize> {
        let ptr = self.str_ptr()?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Str value owns a reference to a live allocation \
                      (see this type's Ownership section), so it is live for \
                      at least this call — and the tag is what says the \
                      payload is text rather than a `bytes`'s octets"
        )]
        Some(unsafe { NvsStr::grapheme_count_of(ptr) })
    }

    /// The `bytes` payload's octets, if this value is a `bytes`.
    ///
    /// Deliberately **not** the same reader as [`Self::as_str_bytes`], even
    /// though both hand back a `&[u8]` from the same heap shape: a caller that
    /// means "text" must not silently accept a `bytes` it would then treat as
    /// UTF-8. Where either is genuinely meant, [`Self::buffer_ptr`] is the one
    /// that spans them.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        let Some(Tag::Bytes) = self.tag() else {
            return None;
        };
        let ptr = self.buffer_ptr()?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Bytes value owns a reference to a live allocation \
                      (see this type's Ownership section), so it is live for \
                      at least this borrow"
        )]
        Some(unsafe { NvsStr::bytes_of(ptr) })
    }

    /// The string payload's header pointer, if this value is a string.
    #[must_use]
    pub const fn str_ptr(self) -> Option<*mut StrHeader> {
        match self.tag() {
            Some(Tag::Str) => self.buffer_ptr(),
            _ => None,
        }
    }

    /// The [`StrHeader`] pointer behind a `string` **or** a `bytes`, the two
    /// tags that share one heap representation ([`Tag::Bytes`]).
    ///
    /// This is what a caller that owns the *allocation* rather than its
    /// meaning uses — [`crate::release`]'s one arm for both, and codegen's
    /// retain/release pair, which are the same two symbols either way.
    /// [`Self::str_ptr`] and [`Self::as_bytes`] stay narrow so that a caller
    /// who means one of the two types says which.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload of a Tag::Str or Tag::Bytes value is a pointer that was widened to u64 by `Value::str`/`Value::bytes`, so narrowing it back is exact on every target Novis compiles for, a 32-bit pointer included"
    )]
    pub const fn buffer_ptr(self) -> Option<*mut StrHeader> {
        match self.tag() {
            Some(Tag::Str | Tag::Bytes) => Some(self.bits as usize as *mut StrHeader),
            _ => None,
        }
    }

    /// The object payload's header pointer, if this value is a class instance.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload of a Tag::Object value is a pointer that was widened to u64 by `Value::object`, so narrowing it back is exact on every target Novis compiles for, a 32-bit pointer included"
    )]
    pub const fn obj_ptr(self) -> Option<*mut ObjHeader> {
        match self.tag() {
            Some(Tag::Object) => Some(self.bits as usize as *mut ObjHeader),
            _ => None,
        }
    }

    /// A slot carrying `desc`, for the receiver position of a `static` method:
    /// the encoding side of [`Self::as_class_desc`], and its whole convention
    /// — the descriptor rides in the payload half of an otherwise-`null` slot,
    /// so nothing sweeping a [`Value`] mistakes it for a heap reference.
    ///
    /// A `static` method's slot 0 is the **called** class (`rule:statements/static-is-a-member-modifier`'s late
    /// static binding, `nvs_ir::lower`'s own docs), so a native caller of one —
    /// `rule:testing/fixtures`'s fixture runner is the only one — has to fill it exactly
    /// as a compiled call site does rather than leave it `null`.
    #[must_use]
    pub fn class_desc(desc: *const ClassDesc) -> Self {
        Self::new(Tag::Null, desc as usize as u64)
    }

    /// The [`ClassDesc`] an argument slot carries, if it carries one.
    ///
    /// `nvs_ir::ty::Ty::ClassDesc` is not an Novis value: a descriptor rides in
    /// the payload half of an otherwise-`null` slot, so nothing sweeping a
    /// [`Value`] can mistake it for a heap reference (`nvs_codegen::ty::tag_of`
    /// is the encoding side). This is the one read of that convention from
    /// native code, and it is named for its single purpose rather than exposing
    /// the raw payload: a by-reference parameter's staged slot address rides in
    /// the same place, and a generic accessor would let one be read as the
    /// other.
    ///
    /// `None` for a genuine `null` — a descriptor is never at address zero —
    /// and for every other tag.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload is a pointer `nvs-codegen` widened to u64 when it stored the slot, so narrowing it back is exact on every target"
    )]
    pub const fn as_class_desc(self) -> Option<*const ClassDesc> {
        match self.tag() {
            Some(Tag::Null) if self.bits != 0 => Some(self.bits as usize as *const ClassDesc),
            _ => None,
        }
    }

    /// A slot carrying `codec`, for the argument a call site writing an inline
    /// shape as its type argument hands over beside the descriptor — the
    /// encoding side of [`Self::as_shape_codec`], on
    /// [`Self::class_desc`]'s convention exactly.
    ///
    /// A second named pair rather than a generic payload read, for the reason
    /// [`Self::as_class_desc`] gives: two engine-owned addresses share one slot
    /// spelling, and a generic accessor would let either be read as the other.
    #[must_use]
    pub fn shape_codec(codec: *const ShapeCodec) -> Self {
        Self::new(Tag::Null, codec as usize as u64)
    }

    /// The [`ShapeCodec`] an argument slot carries, if it carries one —
    /// [`Self::as_class_desc`]'s twin, `None` on the same two answers.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload is a pointer `nvs-codegen` widened to u64 when it stored the slot, so narrowing it back is exact on every target"
    )]
    pub const fn as_shape_codec(self) -> Option<*const ShapeCodec> {
        match self.tag() {
            Some(Tag::Null) if self.bits != 0 => Some(self.bits as usize as *const ShapeCodec),
            _ => None,
        }
    }

    /// A `rule:errors/a-record-names-where-it-was-produced` carrier in a slot,
    /// as [`crate::source`] lays one out — the encoding side of
    /// [`Self::as_source_const`], on [`Self::shape_codec`]'s convention
    /// exactly.
    #[must_use]
    pub fn source_const(blob: *const u8) -> Self {
        Self::new(Tag::Null, blob as usize as u64)
    }

    /// The `rule:errors/a-record-names-where-it-was-produced` carrier a
    /// producer's argument 0 holds — [`Self::as_shape_codec`]'s twin, and a
    /// third named accessor rather than a generic payload read for that
    /// method's reason.
    ///
    /// The zero word answers `None` here as it does there: a producer reached
    /// with no call site to name is handed one, and [`crate::source::decode`]
    /// is what turns the rest into a source.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload is a pointer `nvs-codegen` widened to u64 when it stored the slot, so narrowing it back is exact on every target"
    )]
    pub const fn as_source_const(self) -> Option<*const u8> {
        match self.tag() {
            Some(Tag::Null) if self.bits != 0 => Some(self.bits as usize as *const u8),
            _ => None,
        }
    }

    /// The array payload's header pointer, if this value is an array.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the payload of a Tag::Array value is a pointer that was widened to u64 by `Value::array`, so narrowing it back is exact on every target Novis compiles for, a 32-bit pointer included"
    )]
    pub const fn array_ptr(self) -> Option<*mut ArrayHeader> {
        match self.tag() {
            Some(Tag::Array) => Some(self.bits as usize as *mut ArrayHeader),
            _ => None,
        }
    }

    /// Adds a reference to a refcounted payload — `nvs_ir::InstKind::Retain`.
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
        // `buffer_ptr`, not `str_ptr`: a `bytes` is refcounted through the
        // very same primitive, so the pair takes one branch here exactly as it
        // takes one arm in `crate::release`.
        if let Some(ptr) = self.buffer_ptr() {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees the payload is live; \
                          `nvs_str_retain` only increments in place"
            )]
            unsafe {
                crate::string::nvs_str_retain(ptr);
            }
        } else if let Some(ptr) = self.obj_ptr() {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees the payload is live; \
                          `nvs_object_retain` only increments in place"
            )]
            unsafe {
                crate::object::nvs_object_retain(ptr);
            }
        } else if let Some(ptr) = self.array_ptr() {
            #[expect(
                unsafe_code,
                reason = "the caller guarantees the payload is live; \
                          `nvs_array_retain` only increments in place"
            )]
            unsafe {
                crate::array::nvs_array_retain(ptr);
            }
        }
    }

    /// Drops the reference a refcounted payload owns —
    /// `nvs_ir::InstKind::Release`.
    ///
    /// A non-refcounted value is left alone. Everything else goes through
    /// [`crate::release`]'s one worklist, which is why an array of objects of
    /// arrays frees without recursing.
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
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this value owns exactly the \
                      reference being dropped"
        )]
        unsafe {
            crate::release::release_value(self);
        }
    }
}

/// Adds a reference to whatever a **tagged** value's payload is —
/// `nvs_ir::InstKind::Retain` for a `nvs_ir::Ty::Tagged` operand.
///
/// The two halves arrive separately because that is how compiled code holds
/// one: `nvs_ir::Ty::Tagged` lives in a register pair whose low half is this
/// [`Value`]'s first eight bytes (the tag byte plus its padding) and whose
/// high half is the payload, which is exactly the little-endian memory image
/// of the struct. Passing the pair rather than the struct keeps the C ABI out
/// of the question of how a 16-byte aggregate travels.
///
/// A tag byte denoting no representation is left alone rather than trapped:
/// only a miscompile can produce one, and a refcount primitive has no status
/// to report it in ([`crate::abi`]).
///
/// # Safety
///
/// A refcounted payload must refer to a live allocation.
#[expect(
    unsafe_code,
    reason = "the payload's liveness is the caller's obligation to state"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the tag is the low byte of the word by construction (`nvs_ir::Ty::Tagged`);               the other seven are its padding and mean nothing"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_value_retain(tag_word: u64, bits: u64) {
    let Some(tag) = Tag::from_byte(tag_word as u8) else {
        return;
    };
    #[expect(
        unsafe_code,
        reason = "the caller guarantees a refcounted payload is live; the \
                  tag/payload agreement is compiled code's own, written by \
                  `nvs_codegen`'s `Tag` instruction"
    )]
    unsafe {
        Value::from_parts(tag, bits).retain();
    }
}

/// Drops the reference a **tagged** value's payload owns —
/// `nvs_ir::InstKind::Release` for a `nvs_ir::Ty::Tagged` operand, and the
/// counterpart of [`nvs_value_retain`], whose doc comment owns the two-half
/// signature.
///
/// # Safety
///
/// The pair must own the reference being dropped, and must not be released
/// twice.
#[expect(
    unsafe_code,
    reason = "owning the reference is the caller's obligation to state"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "see `nvs_value_retain`: the tag is the word's low byte by construction"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_value_release(tag_word: u64, bits: u64) {
    let Some(tag) = Tag::from_byte(tag_word as u8) else {
        return;
    };
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the pair owns exactly the reference \
                  being dropped"
    )]
    unsafe {
        Value::from_parts(tag, bits).release();
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
            Some(Tag::Bytes) => {
                // Length, not content: a `bytes` payload is by definition not
                // text, so rendering it as one would be the lossy substitution
                // `rule:types/bytes` exists to refuse — in a `Debug` line as much as in
                // a conversion.
                write!(
                    f,
                    "bytes({} byte(s))",
                    self.as_bytes().unwrap_or_default().len()
                )
            }
            Some(Tag::Decimal) => match self.as_decimal() {
                Some(value) => write!(f, "decimal({value})"),
                None => write!(f, "<invalid decimal>"),
            },
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
        assert_eq!(Tag::from_byte(12), Some(Tag::Unset));
        assert_eq!(Tag::from_byte(13), None);
        assert_eq!(Tag::from_byte(u8::MAX), None);
    }

    /// A `bytes` is a `string`'s allocation under a tag of its own — the crate
    /// docs' § *`bytes` is a tag, not a second heap shape*. So the reference
    /// bookkeeping is `string`'s, and the two readers are not: neither type's
    /// accessor answers for the other, which is what stops a `bytes` from
    /// being read as text by a caller that never asked whether it was.
    #[test]
    fn a_bytes_value_is_a_string_allocation_under_a_tag_of_its_own() {
        let s = NvsStr::new(b"\xff\x00hi");
        let value = Value::bytes(s.clone());
        assert_eq!(value.tag(), Some(Tag::Bytes));
        assert_eq!(s.refcount(), 2);
        assert_eq!(value.as_bytes(), Some(&b"\xff\x00hi"[..]));
        assert_eq!(value.as_str_bytes(), None);
        // The unchecked text reader is gated on the same tag, which is what
        // keeps it sound: these octets are not UTF-8 and never reach it.
        assert_eq!(value.as_text(), None);
        assert_eq!(value.str_ptr(), None);
        assert!(value.buffer_ptr().is_some());
        assert!(Tag::Bytes.is_refcounted());

        let text = Value::str(NvsStr::new(b"hi"));
        assert_eq!(text.as_bytes(), None);
        assert_eq!(text.as_text(), Some("hi"));
        assert!(text.buffer_ptr().is_some());

        #[expect(
            unsafe_code,
            reason = "each value owns exactly the reference taken above"
        )]
        unsafe {
            value.release();
            text.release();
        }
        assert_eq!(s.refcount(), 1);
    }

    #[test]
    fn a_string_value_owns_one_reference() {
        let s = NvsStr::new(b"hi");
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
        let s = NvsStr::new(b"hi");
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
        let value = Value::str(NvsStr::new(b"hi"));
        assert_eq!(format!("{value:?}"), "string(\"hi\")");
        let raw = Value::bytes(NvsStr::new(b"\xff\x00hi"));
        assert_eq!(format!("{raw:?}"), "bytes(4 byte(s))");
        #[expect(unsafe_code, reason = "the value owns the reference it releases")]
        unsafe {
            value.release();
            raw.release();
        }
    }
}
