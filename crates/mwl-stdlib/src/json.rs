//! `Core\Json` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 6, over `serde_json`.
//!
//! That section is authoritative for every signature; what belongs here is the
//! crate choice, the two shapes JSON has that MWL does not, and the three
//! places this module refuses input the C `json_decode` would have accepted.
//!
//! # The crate, and why this one
//!
//! RFC 8259 is an external specification, so [AGENTS.md](../../../../AGENTS.md)
//! § *Implementation invariants* makes it a dependency rather than a
//! hand-written parser. `serde_json` is the pick, for two properties no other
//! Rust JSON crate has both of:
//!
//! * **It can be driven without its own `Value` tree.** [`Decode`] is a
//!   `serde::de::Visitor`, so a document becomes [`mwl_runtime::MwlArray`]s and
//!   [`Value`]s *directly* — nothing is ever materialized twice. That is what
//!   keeps [`ADR 0004`](../../../../docs/adr/0004-memory-for-simplicity.md)'s
//!   priority 3 honest on a member every request path uses.
//! * **The serializer's escaping and number formatting are the crate's.** MWL
//!   writes no JSON grammar of its own at all: [`Encodable`] answers
//!   `serialize_i64`/`serialize_str`/`serialize_map` and the crate decides what
//!   bytes those are.
//!
//! Pure Rust, no build script, no C — ADR 0051 § 4's two questions do not even
//! arise.
//!
//! # `escapeUnicode` is a post-pass, not a formatter
//!
//! `serde_json` never escapes a non-ASCII character, and overriding that means
//! implementing the whole `Formatter` trait twice — once over the compact
//! writer and once over the pretty one. [`escape_non_ascii`] does it in one
//! pass over the finished document instead, and is exactly as correct: **in
//! JSON, a non-ASCII byte can only occur inside a string**, since every
//! structural character, every number and every keyword is ASCII. So a scan
//! that rewrites each non-ASCII `char` as its `\uXXXX` escape cannot touch
//! anything it should not.
//!
//! **What it spends:** one extra `String` the size of the document, on the
//! `{escapeUnicode: true}` path only — an option a call has to ask for.
//!
//! # The three refusals
//!
//! Each is [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R4
//! ("failure throws") applied where PHP's `json_encode`/`json_decode` returned
//! a degraded value instead:
//!
//! 1. **Nesting past `maxDepth` throws**, and the cap is on by default at
//!    [`DEFAULT_MAX_DEPTH`] rather than opt-in — nesting is the one JSON input
//!    that costs unbounded work before any value exists. [`DEPTH_CEILING`] is
//!    the hard bound a call cannot raise past, because the parse recurses.
//! 2. **An integer literal too large for `int` throws**, rather than
//!    degrading to `float`: silent precision loss on a wire format is the bug
//!    `JSON_BIGINT_AS_STRING` exists to work around. Gap 1 below owns how far
//!    that reaches.
//! 3. **A non-finite `float` refuses to encode.** JSON has no `NaN` and no
//!    `Infinity`; PHP's `json_encode` fails too, but only if
//!    `JSON_PARTIAL_OUTPUT_ON_ERROR` was not passed, and there is no such flag
//!    here.
//!
//! # Known gaps
//!
//! 1. **The integer-overflow refusal covers `i64::MAX`..=`u64::MAX` only.**
//!    `serde_json` has already widened a longer integer literal to `f64` by
//!    the time [`Decode::visit_f64`] sees it, and the two are indistinguishable
//!    there — the raw token is not in the visitor's hands. Closing it means
//!    either the `arbitrary_precision` feature, which routes *every* number
//!    through a private map token and is a workspace-wide switch, or a
//!    `RawValue` pre-pass. Neither is worth a whole document's re-scan for a
//!    band that starts at 1.8e19.
//! 2. **`decodeAs<T>`, `Core\Json\Codec` and `#[Json\Derive]` are not built**,
//!    so § 6 is three of its four members and [`Tag::Object`] refuses to
//!    encode. All three are one design —
//!    [ADR 0071](../../../../docs/adr/0071-derived-codecs.md) — and it needs a
//!    language feature nothing else in `Core` does: an *explicit* type argument
//!    at a call site (`decodeAs<User>(…)`).
//! 3. **`isValid` decodes and discards.** It answers exactly what [`mwl_core_json_decode`]
//!    would accept, which is the property that matters, but it allocates the
//!    document to do it. A second `()`-producing visitor would avoid that; it
//!    is a duplicate of [`Decode`] with every body replaced by `Ok(())`, and
//!    not worth carrying until something measures it.

use std::fmt;

use mwl_runtime::{Fault, MwlArray, MwlStr, Tag, ThrownClass, Value};
use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Error as _, Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Json`'s registry rows, in the spec's own order.
///
/// Three of § 6's four members; gap 2 above owns `decodeAs<T>` and what it
/// waits on.
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Json",
    methods: &[
        CoreMethod {
            name: "encode",
            params: &[CoreTy::Mixed, CoreTy::Options(ENCODE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_json_encode",
        },
        CoreMethod {
            name: "decode",
            params: &[CoreTy::Str, CoreTy::Options(DECODE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "mwl_core_json_decode",
        },
        CoreMethod {
            name: "isValid",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_json_is_valid",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Json::encode`'s `{pretty?: bool, escapeUnicode?: bool}` — the two
/// options that survive `json_encode`'s fifteen `JSON_*` flags.
///
/// Both default to `false`, which is `json_encode`'s own no-flags behaviour:
/// the compact document, and UTF-8 written through. The other thirteen flags
/// are gone rather than moved here — ADR 0063 R20 leaves no room for a second
/// spelling of an escaping rule that is already the crate's.
const ENCODE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "pretty",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "escapeUnicode",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\Json::decode`'s `{maxDepth?: uint}` — see [`DEFAULT_MAX_DEPTH`].
const DECODE_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "maxDepth",
    ty: CoreTy::Uint,
    default: Const::Uint(DEFAULT_MAX_DEPTH),
}];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_json_encode" => (mwl_core_json_encode as *const ()).cast(),
        "mwl_core_json_decode" => (mwl_core_json_decode as *const ()).cast(),
        "mwl_core_json_is_valid" => (mwl_core_json_is_valid as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Depth
// ============================================================================

/// Spec § 6's default `maxDepth`, which is also PHP's `$depth` default.
///
/// Counted PHP's way: a scalar document is depth 1, so `[1]` is depth 2 and a
/// `maxDepth` of 1 rejects it. That is the same arithmetic a program migrating
/// from `json_decode` already has in its head.
pub const DEFAULT_MAX_DEPTH: u64 = 512;

/// The largest `maxDepth` a call may ask for.
///
/// The parse recurses — one Rust frame per JSON nesting level — so `maxDepth`
/// is a bound on *stack*, and a `uint` option is user input
/// ([AGENTS.md](../../../../AGENTS.md)'s priority 1). A request past this
/// throws rather than being silently clamped, because a clamp would make a
/// document's acceptance depend on a number the call site never sees.
///
/// Twice [`DEFAULT_MAX_DEPTH`], which is the deepest anything real nests by
/// four orders of magnitude, and `a_document_at_the_ceiling_decodes` is the
/// test that holds the frame budget honest on the platform with the smallest
/// default stack.
pub const DEPTH_CEILING: u64 = 1024;

/// The default is a depth a call may actually ask for, and both fit the `u32`
/// the counters are — checked here rather than in a test, because a `const`
/// assertion cannot be forgotten to run.
const _: () = assert!(DEFAULT_MAX_DEPTH < DEPTH_CEILING);
const _: () = assert!(DEPTH_CEILING < u32::MAX as u64);

/// The `maxDepth` option, checked.
fn max_depth(value: &Value) -> Result<u32, Fault> {
    let asked = value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Json::decode expected {:?} for the `maxDepth` option, got tag {}",
            Tag::Uint,
            value.tag_byte()
        ))
    })?;
    if asked == 0 || asked > DEPTH_CEILING {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("Core\\Json::decode(): a `maxDepth` of {asked} is outside 1..={DEPTH_CEILING}"),
        ));
    }
    u32::try_from(asked)
        .map_err(|_| Fault::fatal("Core\\Json::decode(): a checked `maxDepth` always fits a `u32`"))
}

// ============================================================================
// Encoding
// ============================================================================

/// One MWL value being written as JSON, at a known nesting level.
///
/// `Copy`, and holding the [`Value`] by value rather than by reference: a
/// `Value` is sixteen bytes the caller owns for the length of the call, and
/// every array this walks into is reached through a *borrowed* handle
/// ([`crate::arr::borrowed`]) that takes no reference of its own.
#[derive(Clone, Copy, Debug)]
struct Encodable {
    value: Value,
    /// This value's own nesting level, counted as [`DEFAULT_MAX_DEPTH`]
    /// counts: the document is 1.
    depth: u32,
}

impl Encodable {
    /// This value's elements, one level deeper.
    fn child(self, value: Value) -> Self {
        Self {
            value,
            depth: self.depth + 1,
        }
    }
}

impl Serialize for Encodable {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self.value.tag() {
            Some(Tag::Null) => ser.serialize_unit(),
            Some(Tag::Bool) => ser.serialize_bool(self.value.as_bool().unwrap_or(false)),
            Some(Tag::Int) => ser.serialize_i64(self.value.as_int().unwrap_or(0)),
            Some(Tag::Uint) => ser.serialize_u64(self.value.as_uint().unwrap_or(0)),
            Some(Tag::Float) => {
                let number = self.value.as_float().unwrap_or(0.0);
                if !number.is_finite() {
                    return Err(S::Error::custom(format!(
                        "`{number}` has no JSON spelling — JSON has no `NaN` and no `Infinity`"
                    )));
                }
                ser.serialize_f64(number)
            }
            // ADR 0054's scalar is exact, so it is written as the exact number
            // it is rather than through an `f64` that would round it. A
            // `RawValue` is the one spelling `serde_json` has for "these bytes
            // are already a JSON number"; it validates them on the way in.
            Some(Tag::Decimal) => {
                let text = self
                    .value
                    .as_decimal()
                    .ok_or_else(|| S::Error::custom("a `Tag::Decimal` value is always a decimal"))?
                    .to_string();
                serde_json::value::RawValue::from_string(text)
                    .map_err(S::Error::custom)?
                    .serialize(ser)
            }
            Some(Tag::Str) => ser.serialize_str(self.text()?),
            Some(Tag::Array) => self.serialize_array(ser),
            Some(Tag::Object) => Err(S::Error::custom(format!(
                "an instance of `{}` has no JSON encoding — a class participates by \
                 implementing `Core\\Json\\Codec`",
                crate::instance::class_name(self.value).unwrap_or_else(|| "?".to_owned())
            ))),
            _ => Err(S::Error::custom(format!(
                "tag {} has no JSON encoding",
                self.value.tag_byte()
            ))),
        }
    }
}

impl Encodable {
    /// This value's string payload as UTF-8.
    ///
    /// ADR 0009 makes a `string` guaranteed-valid UTF-8, but a `bytes` value
    /// carries the same [`Tag::Str`] — so this is where "the caller passed
    /// binary data into a text format" is caught rather than left to produce a
    /// document nothing can read.
    fn text<E: serde::ser::Error>(&self) -> Result<&str, E> {
        let bytes = self
            .value
            .as_str_bytes()
            .ok_or_else(|| E::custom("a `Tag::Str` value always has bytes"))?;
        std::str::from_utf8(bytes).map_err(|_| {
            E::custom(
                "a `bytes` value is not text and has no JSON encoding — encode it with \
                 `Core\\Encoding::toBase64` first",
            )
        })
    }

    /// An array, as a JSON array if its keys are `0, 1, …, n-1` and a JSON
    /// object otherwise — `Core\Arr::isList`'s rule, reached from here rather
    /// than through a helper call.
    ///
    /// One rule, not an option: an MWL array is PHP's one ordered-map type, so
    /// *something* has to decide, and `json_encode`'s own list test is the
    /// answer every program migrating from PHP already expects.
    fn serialize_array<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        use std::fmt::Write as _;

        if self.depth >= DEPTH_CEILING_U32 {
            return Err(S::Error::custom(format!(
                "a value nested past {DEPTH_CEILING} levels has no JSON encoding"
            )));
        }
        let ptr = self
            .value
            .array_ptr()
            .ok_or_else(|| S::Error::custom("a `Tag::Array` value is always an array"))?;
        let array = crate::arr::borrowed(ptr);

        let mut list = true;
        let mut expected = String::new();
        let mut index = 0usize;
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let key = array
                .key_at(slot)
                .expect("next_slot only names live entries");
            expected.clear();
            write!(expected, "{index}").expect("writing a usize into a String never fails");
            if key.as_bytes() != expected.as_bytes() {
                list = false;
                break;
            }
            from = slot + 1;
            index += 1;
        }

        if list {
            let mut seq = ser.serialize_seq(Some(array.count()))?;
            let mut from = 0usize;
            while let Some(slot) = array.next_slot(from) {
                let value = array
                    .value_at(slot)
                    .expect("next_slot only names live entries");
                seq.serialize_element(&self.child(value))?;
                from = slot + 1;
            }
            return seq.end();
        }

        let mut map = ser.serialize_map(Some(array.count()))?;
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let key = array
                .key_at(slot)
                .expect("next_slot only names live entries");
            let value = array
                .value_at(slot)
                .expect("next_slot only names live entries");
            let name = std::str::from_utf8(key.as_bytes()).map_err(|_| {
                S::Error::custom("an array key that is not UTF-8 has no JSON encoding")
            })?;
            map.serialize_entry(name, &self.child(value))?;
            from = slot + 1;
        }
        map.end()
    }
}

/// [`DEPTH_CEILING`] as the counter's own width — encoding has no `maxDepth`
/// option, so the ceiling is the only bound it has.
#[expect(
    clippy::cast_possible_truncation,
    reason = "DEPTH_CEILING is a small constant; `a_document_at_the_ceiling_decodes` \
              pins its value"
)]
const DEPTH_CEILING_U32: u32 = DEPTH_CEILING as u32;

/// Every non-ASCII `char` of `text` rewritten as its `\uXXXX` escape —
/// `{escapeUnicode: true}`. This module's own docs own why a post-pass is
/// sound.
fn escape_non_ascii(text: &str) -> String {
    if text.is_ascii() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut buffer = [0u16; 2];
    for ch in text.chars() {
        if ch.is_ascii() {
            out.push(ch);
            continue;
        }
        for unit in ch.encode_utf16(&mut buffer) {
            out.push_str("\\u");
            for shift in [12, 8, 4, 0] {
                let nibble = u32::from(*unit) >> shift & 0xf;
                out.push(char::from_digit(nibble, 16).expect("a nibble is always a hex digit"));
            }
        }
    }
    out
}

mwl_runtime::mwl_helper! {
    /// `Core\Json::encode(mixed $value, {pretty?: bool, escapeUnicode?: bool}): string`
    /// — replacing `json_encode` and its fifteen `JSON_*` flags.
    ///
    /// Anything it cannot write throws a `LogicError`: the value was built by
    /// the program, so an unencodable one is a bug in it rather than something
    /// the world did. This module's own docs list what those are.
    fn mwl_core_json_encode(_ctx, args: [3]) {
        let pretty = flag(&args[1], "pretty")?;
        let escape = flag(&args[2], "escapeUnicode")?;
        let subject = Encodable { value: args[0], depth: 1 };
        let written = if pretty {
            serde_json::to_string_pretty(&subject)
        } else {
            serde_json::to_string(&subject)
        }
        .map_err(|why| Fault::thrown_as(
            ThrownClass::Logic,
            format!("Core\\Json::encode(): {why}"),
        ))?;
        let written = if escape { escape_non_ascii(&written) } else { written };
        Ok(Value::str(MwlStr::new(written.as_bytes())))
    }
}

/// One `bool` option.
fn flag(value: &Value, option: &str) -> Result<bool, Fault> {
    value.as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Json::encode expected {:?} for the `{option}` option, got tag {}",
            Tag::Bool,
            value.tag_byte()
        ))
    })
}

// ============================================================================
// Decoding
// ============================================================================

/// The seed that turns one JSON value into one MWL [`Value`], at a known
/// nesting level.
///
/// `Copy` and carried by value: a child is `Decode { depth: depth + 1, .. }`,
/// so the recursion needs no borrow of a shared counter and no reset on the
/// way back out.
#[derive(Clone, Copy, Debug)]
struct Decode {
    /// This value's own nesting level; the document is 1.
    depth: u32,
    /// The level past which nothing is read — [`max_depth`]'s answer.
    max: u32,
}

impl Decode {
    /// This value's elements, one level deeper.
    fn child(self) -> Self {
        Self {
            depth: self.depth + 1,
            max: self.max,
        }
    }

    /// The refusal a container at this level makes when its *contents* would
    /// be past the cap.
    fn too_deep<E: serde::de::Error>(self) -> E {
        E::custom(format!(
            "nesting is deeper than the `maxDepth` of {}",
            self.max
        ))
    }
}

impl<'de> DeserializeSeed<'de> for Decode {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Decode {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::null())
    }

    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::bool(value))
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::int(value))
    }

    /// The one number arm that can refuse: a positive literal past
    /// `i64::MAX` is spec § 6's "too large for `int`", and this module's gap 1
    /// owns how much further that reaches.
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
        i64::try_from(value)
            .map(Value::int)
            .map_err(|_| E::custom(format!("the integer {value} is too large for `int`")))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        Ok(Value::float(value))
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::str(MwlStr::new(value.as_bytes())))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        if self.depth >= self.max {
            return Err(self.too_deep());
        }
        // The handle owns every element it is given, and releases them all if
        // an element further along refuses — so a failed decode leaks nothing
        // even though it stops half way.
        let mut array = MwlArray::new();
        while let Some(value) = seq.next_element_seed(self.child())? {
            array.append(value);
        }
        Ok(Value::array(array))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        if self.depth >= self.max {
            return Err(self.too_deep());
        }
        let mut array = MwlArray::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(self.child())?;
            // A repeated key overwrites, exactly as `json_decode` into an
            // associative array does.
            array.set(MwlStr::new(key.as_bytes()), value);
        }
        Ok(Value::array(array))
    }
}

/// Reads one whole document, refusing anything after it.
///
/// `disable_recursion_limit` is deliberate and is what [`DEPTH_CEILING`]
/// exists to make safe: `serde_json`'s own limit is 128, which is below spec
/// § 6's default of [`DEFAULT_MAX_DEPTH`], so leaving it on would make the
/// declared default unreachable. [`Decode`]'s own counter is the bound
/// instead, and it is checked against a ceiling the call cannot raise.
fn read(text: &str, max: u32) -> Result<Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    deserializer.disable_recursion_limit();
    let value = Decode { depth: 1, max }.deserialize(&mut deserializer)?;
    match deserializer.end() {
        Ok(()) => Ok(value),
        Err(why) => {
            #[expect(
                unsafe_code,
                reason = "the decoded value owns exactly the one reference `read` \
                          would have handed back, and nothing else holds it"
            )]
            unsafe {
                value.release();
            }
            Err(why)
        }
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Json::decode(string $json, {maxDepth?: uint}): mixed` — replacing
    /// `json_decode`, `json_last_error`, `json_last_error_msg` and `$depth`.
    ///
    /// Always the associative shape: there is no `$associative` flag, because
    /// ADR 0036's anonymous object is not what a JSON object decodes to —
    /// `decodeAs<T>` is (gap 2).
    fn mwl_core_json_decode(_ctx, args: [2]) {
        let text = text_of(&args[0], "decode")?;
        let max = max_depth(&args[1])?;
        read(text, max).map_err(|why| Fault::thrown_as(
            ThrownClass::Parse,
            format!("Core\\Json::decode(): {why}"),
        ))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Json::isValid(string $json): bool` — replacing `json_validate`.
    ///
    /// Answers exactly what [`mwl_core_json_decode`] would accept at the
    /// default depth, by doing it; this module's gap 3 owns what that costs.
    fn mwl_core_json_is_valid(_ctx, args: [1]) {
        let text = text_of(&args[0], "isValid")?;
        let Ok(value) = read(text, DEFAULT_MAX_DEPTH_U32) else {
            return Ok(Value::bool(false));
        };
        #[expect(
            unsafe_code,
            reason = "the decoded value owns one reference and this frame is its \
                      only holder; nothing is handed back"
        )]
        unsafe {
            value.release();
        }
        Ok(Value::bool(true))
    }
}

/// [`DEFAULT_MAX_DEPTH`] as the counter's own width.
#[expect(
    clippy::cast_possible_truncation,
    reason = "DEFAULT_MAX_DEPTH is a small constant; `the_default_depth_is_below_the_ceiling` \
              pins it under DEPTH_CEILING"
)]
const DEFAULT_MAX_DEPTH_U32: u32 = DEFAULT_MAX_DEPTH as u32;

/// One `string` argument, as text.
fn text_of<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    let bytes = value.as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Json::{member} expected {:?}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })?;
    std::str::from_utf8(bytes).map_err(|_| {
        Fault::fatal(format!(
            "Core\\Json::{member} was given a `string` that is not UTF-8"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A helper that releases what it built, for a test that only wants the
    /// text of a decode.
    fn decoded(text: &str, max: u32) -> Result<Value, String> {
        read(text, max).map_err(|why| why.to_string())
    }

    /// The ceiling is a promise about *stack*, so the deepest document a call
    /// may ask for has to actually decode — on the debug profile, whose frames
    /// are the fattest, and on the platform with the smallest default stack.
    #[test]
    fn a_document_at_the_ceiling_decodes() {
        let depth = usize::try_from(DEPTH_CEILING).expect("the ceiling fits a usize");
        let text = format!("{}1{}", "[".repeat(depth - 1), "]".repeat(depth - 1));
        let value = decoded(&text, DEPTH_CEILING_U32).expect("the ceiling is decodable");
        #[expect(unsafe_code, reason = "this frame holds the only reference")]
        unsafe {
            value.release();
        }
    }

    #[test]
    fn nesting_past_the_cap_is_refused() {
        // Verified against `json_decode($j, true, $d)` case for case: a scalar
        // document is depth 1, so `[]` needs 2 and `[[1]]` needs 3.
        assert!(decoded("1", 1).is_ok());
        assert!(decoded("[]", 1).is_err());
        assert!(decoded("[]", 2).is_ok());
        assert!(decoded("[1]", 2).is_ok());
        assert!(decoded("[[1]]", 3).is_ok());
        let why = decoded("[[1]]", 2).expect_err("two levels is past a cap of two");
        assert!(why.contains("maxDepth"), "{why}");
    }

    #[test]
    fn an_integer_past_int_is_refused_rather_than_widened() {
        let why = decoded("9223372036854775808", 8).expect_err("2^63 is past `int`");
        assert!(why.contains("too large for `int`"), "{why}");
    }

    #[test]
    fn trailing_content_is_refused() {
        assert!(decoded("{} {}", 8).is_err());
        assert!(decoded("{oops}", 8).is_err());
    }

    #[test]
    fn escaping_touches_only_what_is_outside_ascii() {
        assert_eq!(escape_non_ascii(r#"{"a":1}"#), r#"{"a":1}"#);
        assert_eq!(
            escape_non_ascii("{\"a\":\"\u{e9}\"}"),
            "{\"a\":\"\\u00e9\"}"
        );
        // Outside the BMP, so a surrogate pair — which is the only escape a
        // JSON `\u` can spell.
        assert_eq!(escape_non_ascii("\"\u{1f600}\""), "\"\\ud83d\\ude00\"");
    }
}
