//! `Core\Json` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 6, over `serde_json`.
//!
//! That section is authoritative for every signature; what belongs here is the
//! crate choice, the two shapes JSON has that Novis does not, and the places
//! this module refuses a value the C `json_encode`/`json_decode` would have
//! degraded instead.
//!
//! # The crate, and why this one
//!
//! RFC 8259 is an external specification, so [AGENTS.md](/AGENTS.md)
//! § *Implementation invariants* makes it a dependency rather than a
//! hand-written parser. `serde_json` is the pick, for two properties no other
//! Rust JSON crate has both of:
//!
//! * **It can be driven without its own `Value` tree.** [`Decode`] is a
//!   `serde::de::Visitor`, so a document becomes [`nvs_runtime::NvsArray`]s and
//!   [`Value`]s *directly* — nothing is ever materialized twice. That is what
//!   keeps [``rule:programs/memory-priority``](/docs/decisions/0004.md)'s
//!   priority 3 honest on a member every request path uses.
//! * **The serializer's escaping and number formatting are the crate's.** Novis
//!   writes no JSON grammar of its own at all: [`Encodable`] answers
//!   `serialize_i64`/`serialize_str`/`serialize_map` and the crate decides what
//!   bytes those are.
//!
//! Pure Rust, no build script, no C — `rule:packaging/a-c-dependency-answers-two-questions`'s two questions do not even
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
//! # The refusals
//!
//! Each is `rule:core-api/shape-rules` R4
//! ("failure throws") applied where PHP's `json_encode`/`json_decode` returned
//! a degraded value instead:
//!
//! * **Nesting past `maxDepth` throws**, and the cap is on by default at
//!   [`DEFAULT_MAX_DEPTH`] rather than opt-in — nesting is the one JSON input
//!   that costs unbounded work before any value exists. [`DEPTH_CEILING`] is
//!   the hard bound a call cannot raise past, because the parse recurses. What
//!   it bounds is structure that is legal and merely deep: a value that
//!   encloses itself is refused by the entry below before this one can be
//!   reached, so the two failures name themselves rather than both reading as
//!   nesting.
//! * **A value the walk is already inside throws**, naming the dotted chain of
//!   keys that closed the cycle —
//!   `rule:classes/an-encoder-ends-a-cycle-by-identity`, carried by
//!   [`Encodable`]'s ancestor frames. The chain is what encloses the value and
//!   not everything already written, so an object two properties both hold is
//!   written out twice: JSON can express repetition and not sharing. Nothing
//!   is substituted into the document to stand for the cycle, because this
//!   document is somebody else's contract rather than our own diagnostic
//!   format — [ADR 0164](/docs/decisions/0164.md).
//! * **An integer literal too large for `int` throws**, rather than
//!   degrading to `float`: silent precision loss on a wire format is the bug
//!   `JSON_BIGINT_AS_STRING` exists to work around. The refusal reaches
//!   `i64::MAX`..=`u64::MAX` and no further: `serde_json` has already widened a
//!   longer literal to `f64` by the time [`Decode::visit_f64`] sees it, and the
//!   two are indistinguishable there, since the raw token is not in the
//!   visitor's hands. Reaching past that band means either the
//!   `arbitrary_precision` feature, which routes *every* number through a
//!   private map token and is a workspace-wide switch, or a `RawValue`
//!   pre-pass — neither worth a whole document's re-scan for a band that starts
//!   at 1.8e19.
//! * **A non-finite `float` refuses to encode.** JSON has no `NaN` and no
//!   `Infinity`; PHP's `json_encode` fails too, but only if
//!   `JSON_PARTIAL_OUTPUT_ON_ERROR` was not passed, and there is no such flag
//!   here.
//!
//! # A shape is a second contract, not a second walk
//!
//! `rule:types/shape-type`'s `{n: int}` may be written where `decodeAs<T>`
//! takes its type argument, and what reaches this module is two things: the
//! class the compiler synthesized for the shape's *field names*, and a
//! [`nvs_runtime::ShapeCodec`] carrying the field *types*. They are separate
//! because `{n: int}` and `{n: string}` share one class, so a shape's wire
//! types have nowhere on a descriptor to live — `nvs-ir`'s module docs own that
//! fork and what it costs.
//!
//! [`Contract`] is the join. It answers the field list either way, so
//! [`decode_fields`] and everything under it is one walk over one
//! [`nvs_runtime::CodecField`] list rather than a decoder per door — and a
//! program catching a decode failure catches one shape of report whichever door
//! it wrote.
//!
//! Two answers differ, and each is the shape's own type saying so rather than a
//! second convention: a shape is built slot by slot ([`build_shape`]), because
//! it declares no constructor to run, and an absent optional key of one is
//! `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`'s marker
//! rather than the constructor default there is no constructor to have
//! declared — [`decode_field`]'s own doc comment owns which and why.
//!
//! # An issue's `path` is the wire key, under every nesting that encloses it
//!
//! A `decodeAs<array<C>>` reports `2.name`, a nested class's field
//! `address.city` and a list field's bad element `tags.3` — § 5's own spelling,
//! and `rule:core-classes/derive-reports-every-field`'s dotted path, built by
//! [`path_of`] out of a prefix each nesting extends by one segment.
//! [`decode_nested`] runs the nested class's own field list under an `address.`
//! prefix and its issues join the enclosing object's rather than throwing where
//! they were found; [`decode_list`] does the same under `tags.3.`. An enum
//! costs no nesting at all: a case is its backing integer, so [`scalar`]
//! answers it as a membership test against the roster
//! [`nvs_runtime::CodecField::cases`] carries.
//!
//! # `isValid` decodes and discards
//!
//! It answers exactly what [`nvs_core_json_decode`] would accept, which is the
//! property that matters, and it allocates the document to do it. A second
//! `()`-producing visitor would avoid that, and it is a duplicate of [`Decode`]
//! with every body replaced by `Ok(())` — a second walk to keep in step with
//! the first, against a cost nothing has measured.
//!
//! # A value type crosses as text, not as its slots
//!
//! `rule:core-classes/derive-field-list` admits the named `Core` value types as
//! fields and JSON has a type for none of them. Two of them carry a wire form
//! here, and both of them are a **string** — written by [`Encodable`] and read
//! by [`scalar`], so a document this crate writes is one it accepts back.
//!
//! A `decimal` is `"19.99"`. JSON has one number type and every consumer reads
//! it as an `f64`, including [`Decode`], so a `decimal` written as a number
//! comes back rounded — the degradation the integer band above already refuses.
//! The cost is that a client sees a string where it may have expected a number,
//! spent under [ADR 0004](/docs/decisions/0004.md) to buy exactness, which is
//! the whole of what `rule:types/decimal` makes the type for.
//!
//! An `Instant` is RFC 3339: `"2024-03-01T12:00:00Z"`, the spelling
//! `$i->toIso()` renders and `Core\Time::fromIso` reads, so the wire carries
//! what a reader of the document already agrees a timestamp looks like. It is
//! **not** the two-slot object its storage is — an epoch second and a subsecond
//! nanosecond are this runtime's representation of the type and no part of what
//! a document promises. `crate::db` answers the same question at the other door
//! and answers it differently, because a column has its own components to build
//! one out of.
//!
//! `rule:core-api/required-optional-and-nullable`'s three shipped rows are
//! answered here, and on the field itself: `nvs_runtime::CodecField::default`
//! carries the constructor parameter's own constant beside `required`, so an
//! absent optional key is filled rather than reported missing. That table's
//! fourth row is another door's — a written `= null` parameter default is
//! refused while checking, which is `nvs_types::defaults`' own gap. A **shape**
//! is in neither and never will be: it declares no constructor, so there is no
//! default to be missing, and [`decode_field`] answers an absent optional key
//! with the never-written marker instead.
//!
//! A constructor position **no field names** is not that question at all: a
//! property `#[Json\Field(skip: true)]` took off the contract while its
//! parameter stayed, so there is no field to read a constant off.
//! `nvs_types::derive`'s `check_json_sites` refuses the call that asks for an
//! instance out of a document over such a class — `E0820`, at every member that
//! writes one, and over every deriving class the written one reaches, since a
//! document is a tree. [`decode_fields`]'s engine fault for the same shape
//! stays under it as the backstop for a class built by hand, as the row door's
//! does.
//!
//! # Known gaps
//!
//! 1. **A hand-written `Core\Json\Codec` is not consulted.** `rule:core-classes/derive-generates-what-is-missing` lets
//!    a class write its own `toJson()` and keep the generated decoder; today
//!    only the derived field list is read, so a class with a hand-written
//!    encoder and no attribute still refuses. Closing it is a
//!    `ClassDesc::method("toJson")` lookup and a call back into compiled code
//!    from the native walk, or it is nothing to write at all once that walk is
//!    the emitted code gap 2 asks about.
//!    Decided: Keep the descriptor and widen it (default constants on CodecField, a ClassDesc method
//!    lookup for toJson); amend the rule — One native walker and small, local changes; costs one loop
//!    and a string compare per field.
//!    — owner: unowned-closures
//! 2. **Both halves walk a per-class field list rather than straight-line
//!    code.** `rule:core-classes/derive-generates-what-is-missing` asks for IR emitted per derived class; what is built
//!    is one compile-time-built descriptor per class, read by native Rust. No
//!    reflection and nothing per object either way — the difference is one
//!    bounded loop and one `String` compare per field, against a table that is
//!    O(derived classes) in the artifact. What has to be decided is which of
//!    the two the machinery stays, and gap 1 waits on that one answer: a
//!    `toJson` lookup is cheap in emitted code and a widening of the descriptor
//!    otherwise.
//!    Decided: Keep the descriptor and widen it (default constants on CodecField, a ClassDesc method
//!    lookup for toJson); amend the rule — One native walker and small, local changes; costs one loop
//!    and a string compare per field.
//!    — owner: unowned-closures
//! 3. **The encoder's real bound is the native stack, not [`DEPTH_CEILING`].**
//!    [`Encodable`] recurses through `serde_json`'s serializer, and a document
//!    nested deeply enough runs the thread's stack out well before the ceiling
//!    is reached — an abort, not a throw. What the refusal above took away is
//!    the half of that a program reaches by accident: a value holding itself
//!    ends at its first repeat instead of descending until something stops it.
//!    What is left is a document that is legal and merely very deep, and what
//!    has to be decided is whether the walk carries an explicit stack — which
//!    makes the bound an allocation the request is charged for — or the ceiling
//!    is read from the space
//!    `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled` reserves.
//!    Goal `resource-ceilings` names the stack ceiling out of its own scope, so
//!    it is not that goal's.
//!    Decided: Walk with an explicit heap stack charged to the request — Always a catchable throw at
//!    the ceiling; the stack is a small allocation billed to the request.
//!    — owner: unowned-closures

use std::fmt;

use nvs_runtime::{CodecTy, EnumCases, Fault, NvsArray, NvsObj, NvsStr, Tag, ThrownClass, Value};
use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Error as _, Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Json`'s registry rows, in the spec's own order — all four of § 6's
/// members.
///
/// `decodeAs` is one of the rows whose helper takes arguments its `params` does
/// not declare: `registry::WRITTEN_CLASS_MEMBERS` puts the class its call site
/// wrote in slot 0, and that roster owns why. `Core\Request::jsonAs` is the
/// same decode over a request body and reaches it through [`check_codec`] and
/// [`hydrate`] rather than through [`decode_as`].
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Json",
    methods: &[
        CoreMethod {
            name: "encode",
            names: &["value"],
            params: &[CoreTy::Mixed, CoreTy::Options(ENCODE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_json_encode",
            doc: Some(&ENCODE_DOC),
        },
        CoreMethod {
            name: "decode",
            names: &["json"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(DECODE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_json_decode",
            doc: Some(&DECODE_DOC),
        },
        CoreMethod {
            name: "decodeAs",
            names: &["json"],
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Options(DECODE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Written("T"),
            symbol: "nvs_core_json_decode_as",
            doc: Some(&DECODE_AS_DOC),
        },
        CoreMethod {
            name: "isValid",
            names: &["json"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_json_is_valid",
            doc: Some(&IS_VALID_DOC),
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
/// are gone rather than moved here — `rule:core-api/shape-rules` R20 leaves no room for a second
/// spelling of an escaping rule that is already the crate's.
/// `Core\Json::encode`'s reference card (`rule:core-api/reference-card`): each option is its own
/// [`ParamDoc`] under the option's name, which is how [`MethodDoc::params`]
/// says a bag is documented. What is stated here is what
/// [`nvs_core_json_encode`] and [`Encodable`] do, and nothing the spec's § 6
/// promises beyond them.
const ENCODE_DOC: MethodDoc = MethodDoc {
    short: "Serializes `$value` as JSON text — scalars, arrays, shape literals and instances of \
            classes carrying `#[Json\\Derive]` — on one line unless `pretty` is set.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The value to encode: `null`, `bool`, `int`, `uint`, `float`, `decimal`, \
                   `string`, an array, a `{name: value}` shape, or an instance of a class \
                   carrying `#[Json\\Derive]`.",
            shape: &[],
        },
        ParamDoc {
            name: "pretty",
            desc: "Indent the output across lines, as `JSON_PRETTY_PRINT` does; the default is \
                   one line.",
            shape: &[],
        },
        ParamDoc {
            name: "escapeUnicode",
            desc: "Write every non-ASCII character as a `\\uXXXX` escape, as `json_encode` does \
                   by default; the default here keeps UTF-8 as it is.",
            shape: &[],
        },
    ],
    ret: "The JSON text.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$value` holds something JSON cannot spell: a `NaN` or infinite `float`, a value \
               of a type with no JSON encoding, an instance of a class without \
               `#[Json\\Derive]`, or nesting past 1024 levels.",
    }],
};

/// `Core\Json::decode`'s reference card — `rule:core-api/reference-card`.
const DECODE_DOC: MethodDoc = MethodDoc {
    short: "Parses the JSON text `$json` into a value, as `json_decode` does with `$associative` \
            set: an object becomes a string-keyed array, an array a list, and a scalar itself. A \
            malformed document throws rather than answering `null`, so there is no \
            `json_last_error`.",
    params: &[
        ParamDoc {
            name: "json",
            desc: "The JSON text to parse.",
            shape: &[],
        },
        ParamDoc {
            name: "maxDepth",
            desc: "The deepest nesting accepted, counted as PHP's `$depth` is — a scalar document \
                   is depth 1, `[1]` is depth 2; `512` by default and at most `1024`.",
            shape: &[],
        },
    ],
    ret: "The decoded value: `null`, `bool`, `int`, `float`, `string`, or an array; a JSON object \
          is always a string-keyed array, never an object.",
    errors: &[
        ErrorDoc {
            error: "ParseError",
            desc: "`$json` is not a valid JSON document, nests deeper than `maxDepth`, or holds \
                   an integer literal too large for `int`; the one issue it carries has an empty \
                   path.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`maxDepth` is `0` or above `1024`.",
        },
    ],
};

/// `Core\Json::decodeAs`'s reference card — `rule:core-api/reference-card`.
const DECODE_AS_DOC: MethodDoc = MethodDoc {
    short: "Parses the JSON object `$json` into an instance of `T`, a class carrying \
            `#[Json\\Derive]`, reading every declared field and running the constructor only \
            when all of them matched; write `array<T>` to read a JSON array as one instance \
            per element instead. It replaces hand-written hydration.",
    params: &[
        ParamDoc {
            name: "json",
            desc: "The JSON text to parse, whose top level must be an object — or an array, \
                   where `T` is written `array<C>`.",
            shape: &[],
        },
        ParamDoc {
            name: "maxDepth",
            desc: "The deepest nesting accepted, counted as PHP's `$depth` is — a scalar document \
                   is depth 1, `[1]` is depth 2; `512` by default and at most `1024`.",
            shape: &[],
        },
    ],
    ret: "A new `T` built from the document's fields, or — for an `array<C>` — one new `C` per \
          element, in the document's own order.",
    errors: &[
        ErrorDoc {
            error: "ParseError",
            desc: "`$json` is not a valid JSON document, nests deeper than `maxDepth`, holds an \
                   integer literal too large for `int`, is not an object at the top level (an \
                   array, for an `array<C>`), or has fields that are missing or of the wrong \
                   type — every failed field is one issue on the error, at its own path, and the \
                   message counts them. A list stops at its first bad element, and each of its \
                   paths carries that element's position.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`T` has no JSON codec because it does not carry `#[Json\\Derive]`, or \
                   `maxDepth` is `0` or above `1024`.",
        },
    ],
};

/// `Core\Json::isValid`'s reference card — `rule:core-api/reference-card`.
const IS_VALID_DOC: MethodDoc = MethodDoc {
    short: "Tells whether `$json` is a document `decode` would accept at the default depth of \
            `512`, as `json_validate` does, by parsing it.",
    params: &[ParamDoc {
        name: "json",
        desc: "The JSON text to check.",
        shape: &[],
    }],
    ret: "`true` when `$json` parses; `false` for malformed text, nesting past `512`, or an \
          integer literal too large for `int`.",
    errors: &[],
};

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
///
/// `pub(crate)` because `Core\Request::json` reads a document out of a request
/// body and carries the same bag: one default for the depth of a JSON document,
/// wherever the octets came from.
pub(crate) const DECODE_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "maxDepth",
    ty: CoreTy::Uint,
    default: Const::Uint(DEFAULT_MAX_DEPTH),
}];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_json_encode" => (nvs_core_json_encode as *const ()).cast(),
        "nvs_core_json_decode" => (nvs_core_json_decode as *const ()).cast(),
        "nvs_core_json_decode_as" => (nvs_core_json_decode_as as *const ()).cast(),
        "nvs_core_json_is_valid" => (nvs_core_json_is_valid as *const ()).cast(),
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
/// ([AGENTS.md](/AGENTS.md)'s priority 1). A request past this
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

/// The `maxDepth` option, checked, for the member `who` names.
///
/// The member is a parameter because [`DECODE_OPTIONS`] is carried by every
/// reader of a JSON document, including `Core\Request::json`, and a refusal
/// that named this class regardless would send a program looking at a call it
/// did not write.
pub(crate) fn max_depth(value: &Value, who: &str) -> Result<u32, Fault> {
    // Unreachable from source: `maxDepth` is a `CoreTy::Uint` option in
    // `DECODE_OPTIONS` below, so `{maxDepth: $m}` over a `mixed` is `E0401:
    // expected uint, found mixed` at the checker and the bag a call that
    // omits it passes carries the `Const::Uint` default. The `1..=1024`
    // refusal underneath is the reachable half, and it throws.
    let asked = value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "{who} expected {:?} for the `maxDepth` option, got tag {}",
            Tag::Uint,
            value.tag_byte()
        ))
    })?;
    if asked == 0 || asked > DEPTH_CEILING {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{who}(): a `maxDepth` of {asked} is outside 1..={DEPTH_CEILING}"),
        ));
    }
    // The `1..=DEPTH_CEILING` refusal above is the boundary and it throws; this
    // is its post-condition and is unreachable from source, since a value that
    // got past it is at most 1024 and every `u64` that small is a `u32`.
    u32::try_from(asked)
        .map_err(|_| Fault::fatal(format!("{who}(): a checked `maxDepth` always fits a `u32`")))
}

// ============================================================================
// Encoding
// ============================================================================

/// One Novis value being written as JSON, at a known nesting level and inside
/// a known chain of ancestors.
///
/// `Copy`, and holding the [`Value`] by value rather than by reference: a
/// `Value` is sixteen bytes the caller owns for the length of the call, and
/// every array this walks into is reached through a *borrowed* handle
/// ([`crate::arr::borrowed`]) that takes no reference of its own.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Encodable<'a> {
    value: Value,
    /// This value's own nesting level, counted as [`DEFAULT_MAX_DEPTH`]
    /// counts: the document is 1.
    depth: u32,
    /// The step that reached this value, and `None` for the document: the
    /// segment it contributes to [`Encodable::path`].
    step: Option<Step<'a>>,
    /// The objects and arrays this value is inside, innermost first, which is
    /// what `rule:classes/an-encoder-ends-a-cycle-by-identity` decides a cycle
    /// by.
    ancestors: Option<&'a Ancestor<'a>>,
}

/// One object or array the walk is currently inside.
///
/// A borrowed cons list rather than a set or a `Vec`: a frame lives in the
/// stack frame of the arm that walked into it, which lasts exactly as long as
/// that value's elements are being written, so the encoder allocates nothing
/// for this. The chain is the path from the document down rather than
/// everything seen, so the membership test is linear in the nesting level —
/// bounded by [`DEPTH_CEILING`], which the arms check first.
#[derive(Clone, Copy, Debug)]
struct Ancestor<'a> {
    /// The allocation's address, which is a live value's identity. An object
    /// and an array are distinct allocations, so one `usize` answers for both.
    id: usize,
    /// The step that reached this frame, and `None` for the document.
    step: Option<Step<'a>>,
    /// The frame one level further out.
    outer: Option<&'a Ancestor<'a>>,
}

/// One segment of the chain from the document to a value.
///
/// A key borrowed from the descriptor or the array rather than an owned
/// `String`, and a position held as the number it is, so walking into an
/// element costs no allocation on a path no message ever asks for.
#[derive(Clone, Copy, Debug)]
enum Step<'a> {
    /// An object's wire key, a shape's field name, or a non-list array's key.
    Key(&'a str),
    /// A list element's position.
    Index(usize),
}

impl<'a> Encodable<'a> {
    /// A whole document — the value at the top level, which is where
    /// [`DEFAULT_MAX_DEPTH`] counts from.
    ///
    /// A constructor rather than public fields because the depth is the
    /// invariant: a caller outside this module has no business choosing a
    /// nesting level, and [`crate::log`] — the second writer of a JSON value
    /// in this crate, and the reason this type is `pub(crate)` at all — writes
    /// a `fields` bag that is a document exactly as `Core\Json::encode`'s
    /// argument is. One encoder, so a `float` or a nested array cannot be
    /// spelled two ways depending on which member wrote it.
    pub(crate) fn document(value: Value) -> Self {
        Self {
            value,
            depth: 1,
            step: None,
            ancestors: None,
        }
    }

    /// One of this value's elements, one level deeper and one frame further in.
    fn child(self, inside: &'a Ancestor<'a>, step: Step<'a>, value: Value) -> Self {
        Self {
            value,
            depth: self.depth + 1,
            step: Some(step),
            ancestors: Some(inside),
        }
    }

    /// This value as the frame its own elements are inside, identified by the
    /// address of the allocation the arm is about to walk.
    fn frame(self, id: usize) -> Ancestor<'a> {
        Ancestor {
            id,
            step: self.step,
            outer: self.ancestors,
        }
    }

    /// The refusal `rule:classes/an-encoder-ends-a-cycle-by-identity` asks for
    /// when the allocation at `id` is one this walk is already inside, and
    /// `None` when it is not.
    ///
    /// The test is against the ancestor chain and never against everything
    /// already written: an object two properties both hold is shared rather
    /// than cyclic, and a format that cannot express sharing has no answer but
    /// to write it twice. Only a repeat on the current path is a walk that
    /// would not end, and it ends here — named by the chain that closed it,
    /// with no marker invented in a document somebody else's reader parses.
    fn cycle<E: serde::ser::Error>(self, id: usize) -> Option<E> {
        let mut outer = self.ancestors;
        while let Some(frame) = outer {
            if frame.id == id {
                return Some(E::custom(format!(
                    "a value that holds itself has no JSON encoding — `{}` is a value \
                     it is already inside",
                    self.path()
                )));
            }
            outer = frame.outer;
        }
        None
    }

    /// The dotted chain of keys and positions from the document to this value,
    /// spelled as a decode's issue path is ([`path_of`]).
    fn path(self) -> String {
        use std::fmt::Write as _;

        let mut steps = vec![self.step];
        let mut outer = self.ancestors;
        while let Some(frame) = outer {
            steps.push(frame.step);
            outer = frame.outer;
        }
        let mut path = String::new();
        for step in steps.into_iter().rev().flatten() {
            if !path.is_empty() {
                path.push('.');
            }
            match step {
                Step::Key(key) => path.push_str(key),
                Step::Index(at) => {
                    write!(path, "{at}").expect("writing a usize into a String never fails");
                }
            }
        }
        path
    }
}

impl Serialize for Encodable<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self.value.tag() {
            Some(Tag::Null) => ser.serialize_unit(),
            Some(Tag::Bool) => ser.serialize_bool(self.value.as_bool().unwrap_or(false)),
            Some(Tag::Int) => ser.serialize_i64(self.value.as_int().unwrap_or(0)),
            Some(Tag::Uint) => ser.serialize_u64(self.value.as_uint().unwrap_or(0)),
            Some(Tag::Float) => {
                let number = self.value.as_float().unwrap_or(0.0);
                if !number.is_finite() {
                    // Spelled the way `echo` would spell it — `INF`, not
                    // Rust's `inf` — so the value the message quotes back is
                    // the one the program can see for itself.
                    return Err(S::Error::custom(format!(
                        "`{}` has no JSON spelling — JSON has no `NaN` and no `Infinity`",
                        nvs_runtime::php_float_to_string(number)
                    )));
                }
                ser.serialize_f64(number)
            }
            // `rule:types/decimal`'s scalar is exact and JSON's one number type is
            // an `f64` in every consumer that reads it — this module's own, in
            // [`Decode`], as much as a browser's — so a `decimal` is written as
            // a **string**: `"19.99"`, digits and scale intact. That is the
            // spelling [`scalar`] reads back, so `Core\Json::encode` and
            // `decodeAs<T>` are one round trip rather than a pair that loses
            // digits at the seam, which is the same refusal to degrade the
            // integer band above is written for.
            Some(Tag::Decimal) => {
                let text = self
                    .value
                    .as_decimal()
                    .ok_or_else(|| S::Error::custom("a `Tag::Decimal` value is always a decimal"))?
                    .to_string();
                ser.serialize_str(&text)
            }
            Some(Tag::Str) => ser.serialize_str(self.text()?),
            Some(Tag::Array) => self.serialize_array(ser),
            Some(Tag::Object) => self.serialize_object(ser),
            // The tag is named rather than numbered: `bytes` is the arm a
            // program actually reaches (see [`Self::text`]), and a caller who
            // handed a buffer to a text format needs to be told *that* rather
            // than told a number only this crate can read.
            Some(tag) => Err(S::Error::custom(format!(
                "a `{}` value has no JSON encoding",
                tag.describe()
            ))),
            None => Err(S::Error::custom(format!(
                "tag {} has no JSON encoding",
                self.value.tag_byte()
            ))),
        }
    }
}

impl Encodable<'_> {
    /// This value's string payload as UTF-8.
    ///
    /// `rule:types/bytes` makes a `string` guaranteed-valid UTF-8 and [`Tag::Bytes`] is
    /// its own tag over the shared allocation, so "the caller passed binary
    /// data into a text format" is caught one level up — a `bytes` value never
    /// reaches here, it falls into the `_` arm of the match above. What is left
    /// is the tag check itself, which is exactly what [`Value::as_text`] is.
    fn text<E: serde::ser::Error>(&self) -> Result<&str, E> {
        self.value
            .as_text()
            .ok_or_else(|| E::custom("a `Tag::Str` value always has text"))
    }

    /// An object, as the document its class's
    /// `rule:core-classes/derive-attribute` derived codec
    /// declares: one entry per field, in declaration order, under the field's
    /// own wire key.
    ///
    /// The field list is compiled in — `nvs_runtime::ClassDesc::codec`, filled
    /// by `nvs-codegen` from what `nvs_types::derive` read off the declaration
    /// — so nothing here asks the program a question at run time. An empty
    /// list means the class carries no `#[Json\Derive]`, which is the refusal
    /// [ADR 0063](/docs/decisions/0063.md) § 4 asks
    /// for: participation in a wire format is written, never inferred.
    ///
    /// Two instances are not declared classes and answer before that list is
    /// read. An `rule:types/object-literal` shape encodes as a JSON object
    /// keyed by its own field names, and a `Core\Time\Instant` as the RFC 3339
    /// string this module's § *A value type crosses as text* fixes it at;
    /// neither is an exception to the rule above, because neither has a
    /// declaration to carry an attribute and both are wire forms the language
    /// names rather than a program does —
    /// `rule:core-classes/derive-generates-what-is-missing` owns the first and
    /// that section the second.
    fn serialize_object<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        if self.depth >= DEPTH_CEILING_U32 {
            return Err(S::Error::custom(format!(
                "a value nested past {DEPTH_CEILING} levels has no JSON encoding"
            )));
        }
        let ptr = self
            .value
            .obj_ptr()
            .ok_or_else(|| S::Error::custom("a `Tag::Object` value is always an object"))?;
        // Before a field is read, so the walk turns back at the first repeat
        // rather than at the ceiling above —
        // `rule:classes/an-encoder-ends-a-cycle-by-identity`. An instance is
        // the half of the graph that can close a cycle at all: a property
        // holds a reference, where an array entry holds a copy.
        let id = ptr as usize;
        if let Some(cycle) = self.cycle(id) {
            return Err(cycle);
        }
        let inside = self.frame(id);
        #[expect(
            unsafe_code,
            reason = "the value owns a reference to a live allocation, so it is live \
                      for this borrow; the handle is never dropped, so the reference \
                      is not released twice"
        )]
        let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
        #[expect(
            unsafe_code,
            reason = "a live object's descriptor is owned by the compiled unit that \
                      defined its class, which outlives every instance of it"
        )]
        let desc = unsafe { &*object.class() };
        // `rule:types/object-literal`'s shape, before the codec is read: a shape is a bag of
        // named fields with no declaration to hang `#[Json\Derive]` on, so the
        // refusal below has nothing to ask it for — `rule:core-classes/derive-generates-what-is-missing`. Its slots are
        // walked the way an array's entries are, each value spelled by its own
        // tag, because the class is keyed on field *names* alone
        // (`nvs_ir::lower::shape_class_label`) and so has no per-field wire
        // type a `CodecField` could honestly carry: `{n: 1}` and `{n: "s"}` are
        // one class. Key order is that label's, which is sorted, so a shape's
        // document is byte-deterministic on `rule:core-classes/derive-field-list`'s terms.
        if desc.is_shape() {
            let mut map = ser.serialize_map(Some(desc.field_count()))?;
            for slot in 0..desc.field_count() {
                let held = object.field(slot);
                // An optional field the document a hydration read did not carry
                // (`rule:types/shape-type`) is the never-written storage state,
                // so the key that was absent on the way in is absent on the way
                // out. Nothing else can put a slot in that state — a shape
                // literal writes every one of its fields — and reading it from
                // Novis is that rule's own catchable throw, which is not this
                // encoder's answer to give.
                if held.tag() == Some(Tag::Unset) {
                    continue;
                }
                let name = desc
                    .field_name(slot)
                    .ok_or_else(|| S::Error::custom("a slot below the field count is named"))?;
                map.serialize_entry(name, &self.child(&inside, Step::Key(name), held))?;
            }
            return map.end();
        }
        // `Core\Time\Instant`, whose wire form is RFC 3339 text rather than the
        // object its two slots would spell — this module's § *A value type
        // crosses as text* owns the decision and
        // [`crate::time::instant_from_iso`] is the half that reads it back. A `Core` value type declares no member a codec could be
        // generated from, so the refusal below would otherwise be the only
        // answer a wire type of its own already has.
        if crate::instance::is_instance(self.value, &crate::time::INSTANT) {
            let text = crate::time::instant_iso(self.value).ok_or_else(|| {
                S::Error::custom(
                    "a `Core\\Time\\Instant` holds a second and a nanosecond that are \
                     no point on the timeline",
                )
            })?;
            return ser.serialize_str(&text);
        }
        let fields = desc.codec();
        if fields.is_empty() {
            return Err(S::Error::custom(format!(
                "an instance of `{}` has no JSON encoding — a class participates by \
                 carrying `#[Json\\Derive]`",
                desc.name()
            )));
        }
        let mut map = ser.serialize_map(Some(fields.len()))?;
        for field in fields {
            // A borrowed read, exactly as `nvs_ir::InstKind::FieldGet` is: the
            // object holds the reference for the length of this call and
            // nothing here hands the value on to Novis code.
            map.serialize_entry(
                &field.key,
                &self.child(&inside, Step::Key(&field.key), object.field(field.slot)),
            )?;
        }
        map.end()
    }

    /// An array, as a JSON array if its keys are `0, 1, …, n-1` and a JSON
    /// object otherwise — `Core\Arr::isList`'s rule, reached from here rather
    /// than through a helper call.
    ///
    /// One rule, not an option: an Novis array is PHP's one ordered-map type, so
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
        // An array is copied where an object is referenced, so this arm alone
        // cannot close a cycle — but the allocation copy-on-write shares is
        // reachable from an object that is inside it, and then the walk does
        // not end. What the ancestor chain answers is that walk, not the
        // language's value semantics: an address repeating on the current path
        // is a descent with no bottom whichever tag it wears.
        let id = ptr as usize;
        if let Some(cycle) = self.cycle(id) {
            return Err(cycle);
        }
        let inside = self.frame(id);
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
            let mut at = 0usize;
            while let Some(slot) = array.next_slot(from) {
                let value = array
                    .value_at(slot)
                    .expect("next_slot only names live entries");
                seq.serialize_element(&self.child(&inside, Step::Index(at), value))?;
                from = slot + 1;
                at += 1;
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
            map.serialize_entry(name, &self.child(&inside, Step::Key(name), value))?;
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

/// `value` as one compact JSON document, refused under `member` where it holds
/// something JSON cannot write.
///
/// [`Encodable`] and not a second walk, for that type's own reason: a `float`,
/// a nested array and a cycle are spelled the same however the document was
/// asked for. The caller outside this module is `Core\Test::answerHttp`, whose
/// `json` option is a document exactly as [`nvs_core_json_encode`]'s argument
/// is — a faked reply that spelled a number differently from a real one would
/// be a fixture that passes against a client the wire would fail.
///
/// # Errors
///
/// A `LogicError`, named after `member`, for a value this encoder refuses —
/// the program built it, so an unencodable one is a bug in the program.
pub(crate) fn written(value: Value, member: &str) -> Result<String, Fault> {
    serde_json::to_string(&Encodable::document(value))
        .map_err(|why| Fault::thrown_as(ThrownClass::Logic, format!("{member}(): {why}")))
}

nvs_runtime::nvs_helper! {
    /// `Core\Json::encode(mixed $value, {pretty?: bool, escapeUnicode?: bool}): string`
    /// — replacing `json_encode` and its fifteen `JSON_*` flags.
    ///
    /// Anything it cannot write throws a `LogicError`: the value was built by
    /// the program, so an unencodable one is a bug in it rather than something
    /// the world did. This module's own docs list what those are.
    fn nvs_core_json_encode(_ctx, args: [3]) {
        let pretty = flag(&args[1], "pretty")?;
        let escape = flag(&args[2], "escapeUnicode")?;
        let subject = Encodable::document(args[0]);
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
        Ok(Value::str(NvsStr::new(written.as_bytes())))
    }
}

/// One `bool` option.
fn flag(value: &Value, option: &str) -> Result<bool, Fault> {
    // Unreachable from source: both callers pass an `ENCODE_OPTIONS` slot and
    // both of those are `CoreTy::Bool`, so `{pretty: $m}` over a `mixed` is
    // `E0401: expected bool, found mixed` at the checker.
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

/// The seed that turns one JSON value into one Novis [`Value`], at a known
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
    /// `i64::MAX` is spec § 6's "too large for `int`", and this module's
    /// § *The refusals* owns how much further that band reaches.
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
        i64::try_from(value)
            .map(Value::int)
            .map_err(|_| E::custom(format!("the integer {value} is too large for `int`")))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        Ok(Value::float(value))
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::str(NvsStr::new(value.as_bytes())))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        if self.depth >= self.max {
            return Err(self.too_deep());
        }
        // The handle owns every element it is given, and releases them all if
        // an element further along refuses — so a failed decode leaks nothing
        // even though it stops half way.
        let mut array = NvsArray::new();
        while let Some(value) = seq.next_element_seed(self.child())? {
            array.append(value);
        }
        Ok(Value::array(array))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        if self.depth >= self.max {
            return Err(self.too_deep());
        }
        let mut array = NvsArray::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(self.child())?;
            // A repeated key overwrites, exactly as `json_decode` into an
            // associative array does.
            array.set(NvsStr::new(key.as_bytes()), value);
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
pub(crate) fn read(text: &str, max: u32) -> Result<Value, serde_json::Error> {
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

nvs_runtime::nvs_helper! {
    /// `Core\Json::decode(string $json, {maxDepth?: uint}): mixed` — replacing
    /// `json_decode`, `json_last_error`, `json_last_error_msg` and `$depth`.
    ///
    /// Always the associative shape: there is no `$associative` flag, because
    /// `rule:types/object-top`'s anonymous object is not what a JSON object decodes to —
    /// `decodeAs<T>` is.
    fn nvs_core_json_decode(_ctx, args: [2]) {
        let text = text_of(&args[0], "decode")?;
        let max = max_depth(&args[1], "Core\\Json::decode")?;
        read(text, max).map_err(|why| {
            // `rule:core-classes/derive-reports-every-field`'s last sentence: a malformed document records one
            // issue, so a `catch (ParseError $e)` reads the same shape whether
            // the failure was the syntax or the fields. Its `path` is empty —
            // there is no field to point at when the document did not parse.
            let message = format!("Core\\Json::decode(): {why}");
            let issues = crate::issue::list([("", message.as_str())]);
            Fault::thrown_with_issues(ThrownClass::Parse, message, issues)
        })
    }
}

// ============================================================================
// Decoding into a class — `rule:core-classes/derive-attribute`'s generated decoder
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Json::decodeAs<T>(string $json, {maxDepth?: uint}): T` — replacing
    /// hand-written hydration.
    ///
    /// **Arguments 0 to 2 are what the call site wrote as its type argument**,
    /// not values: `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` puts this
    /// member on the roster whose helper is handed a `nvs_runtime::ClassDesc`,
    /// the `array<...>` flag and an inline shape's wire contract ahead of its
    /// declared parameters, and that roster's docs own why. So the arity here
    /// is three more than the registry row's.
    fn nvs_core_json_decode_as(ctx, args: [5]) {
        // Unreachable from source, because arguments 0 to 2 are not a
        // program's values: `crate::registry::WRITTEN_CLASS_MEMBERS` is what
        // puts the resolved `ClassDesc` in slot 0 and the list flag in slot 1,
        // and `nvs_ir::lower` writes both out of the type argument at the call
        // site. A call naming none is `E0442` — `takes 1 type argument(s)` —
        // before any of this runs.
        let class = args[0].as_class_desc().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Json::decodeAs` was called with no class in argument 0",
        ))?;
        // Unreachable from source for the same reason and refused by the same
        // `E0442`: slot 1 is the `ConstBool` the lowering emits beside the
        // descriptor, so a call that has one has the other.
        let list = args[1].as_bool().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Json::decodeAs` was called with no list flag in argument 1",
        ))?;
        // Slot 2 is an inline shape's wire contract, and a written *class* gets
        // the zero word there — which is the whole of what tells the two apart,
        // since `Value::as_shape_codec` answers `None` for a zero payload. A
        // shape class carries no codec of its own, so this is the only thing
        // that says what `{n: int}`'s `n` is on the wire.
        let shape = args[2].as_shape_codec();
        let text = text_of(&args[3], "decodeAs")?;
        let max = max_depth(&args[4], "Core\\Json::decodeAs")?;
        #[expect(
            unsafe_code,
            reason = "the descriptor and the contract came out of the constants a \
                      compiled unit owns, so both outlive this call and every \
                      object made from it"
        )]
        unsafe {
            decode_as(ctx, class, shape, text, max, list, "Core\\Json::decodeAs")
        }
    }
}

/// `rule:core-classes/derive-field-list`'s decode: the
/// contract checked once, the document read once, and then one instance —
/// or, for `list`, one per element of a JSON array.
///
/// The two shapes share this frame because the codec questions are the
/// contract's and not the document's: an `Opaque` field is a decoder this crate
/// has not written, whether it is asked for once or a thousand times.
///
/// `shape` is the wire contract where the call site wrote an inline shape
/// rather than a class name, and `None` where it wrote one — [`Contract`] owns
/// what the two answer differently.
///
/// `Core\Jwt::verifyIssued` is the one caller outside this module: a token's
/// payload is a JSON document by the time its signature has held, so what that
/// member owes its `T` is exactly this decode and not a second one written
/// beside it.
///
/// # Safety
///
/// `class` must refer to a live descriptor whose method table `nvs-codegen`
/// has filled, and `shape` to a live contract where it is `Some`.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub(crate) unsafe fn decode_as(
    ctx: &mut nvs_runtime::Ctx,
    class: *const nvs_runtime::ClassDesc,
    shape: Option<*const nvs_runtime::ShapeCodec>,
    text: &str,
    max: u32,
    list: bool,
    member: &str,
) -> Result<Value, Fault> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    unsafe {
        check_codec(class, shape, member)?;
    }
    let document = read(text, max).map_err(|why| {
        let message = format!("{member}(): {why}");
        let issues = crate::issue::list([("", message.as_str())]);
        Fault::thrown_with_issues(ThrownClass::Parse, message, issues)
    })?;
    #[expect(
        unsafe_code,
        reason = "the same live descriptor the caller vouched for"
    )]
    unsafe {
        hydrate(ctx, class, shape, document, list, Reading::Wire, member)
    }
}

/// The field list one decode walks and the class it fills: a class's own
/// derived codec, or the wire contract an inline shape was written as at the
/// call site.
///
/// One type rather than two decoders, because the two agree everywhere the
/// document is concerned — `rule:types/shape-type`'s structural type is the
/// same field list a `#[Json\Derive]` class carries, read off the type instead
/// of off a declaration. Three questions have two answers, and every one of
/// them is a caller of this:
///
/// * **Where the fields come from.** A shape class is keyed on its field names
///   alone, so `{n: int}` and `{n: string}` are one
///   [`nvs_runtime::ClassDesc`] and the per-field wire types live beside it in
///   a [`nvs_runtime::ShapeCodec`].
/// * **How the object is built** — [`build_shape`] against
///   [`nvs_runtime::construct`].
/// * **What an absent optional key and a `?T` mean** — [`decode_field`].
#[derive(Clone, Copy)]
struct Contract<'a> {
    /// The class an instance is made of, kept as the pointer the two builders
    /// take rather than only as the reference below.
    class: *const nvs_runtime::ClassDesc,
    /// That same descriptor, dereferenced once for every message and every
    /// field-count question under this walk.
    desc: &'a nvs_runtime::ClassDesc,
    /// The wire contract, where a call site wrote an inline shape — and `None`
    /// where it wrote a class name, which is the one bit everything above
    /// branches on.
    shape: Option<&'a nvs_runtime::ShapeCodec>,
    /// What the values under this walk are, which is [`scalar`]'s one
    /// question and no other caller's.
    reading: Reading,
}

/// What the values one walk is handed already are — the second axis
/// [`Contract`] carries, and the only one [`scalar`] reads.
///
/// A JSON document arrives *typed*: `1` is a number and `"1"` is a string,
/// because the wire spelled them apart, so a `"1"` reaching an `int` field is
/// a document that disagrees with the class it claims to be. A form, a query
/// string and an array a program built arrive as whatever is in them — every
/// value of a query string is text, and a field declaring `int` is asking for
/// `rule:types/conversion`'s `string → int` row rather than for a wire type
/// that was never there.
///
/// So the two doors differ in one place and share everything else: the field
/// list, the presence column, the nesting, and — the reason this is one walk
/// rather than two — the issue list a failure arrives as, which is what a
/// program catches. Nothing new enters the conversion table for either;
/// [`Values`] applies the rows `nvs_runtime::to_int` and its two siblings
/// already answer for `$mixed as int`.
///
/// [`Values`]: Reading::Values
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reading {
    /// A JSON document's own types, matched exactly — `Core\Json::decodeAs`
    /// and `Core\Request::jsonAs`.
    Wire,
    /// Loose values converted per field by `rule:types/conversion`'s table —
    /// `Core\Arr::shapeAs` and the request wrappers over it.
    Values,
}

impl<'a> Contract<'a> {
    /// The contract for `class`, read against `shape` where the call site wrote
    /// an inline shape rather than a class name.
    ///
    /// # Safety
    ///
    /// As [`decode_as`]'s: `class` must be live, and `shape` where it is
    /// `Some`. `nvs-codegen` defines the two beside each other and the compiled
    /// unit owns both for its life.
    #[expect(
        unsafe_code,
        reason = "the caller owes the liveness of two addresses no signature can express"
    )]
    unsafe fn new(
        class: *const nvs_runtime::ClassDesc,
        shape: Option<*const nvs_runtime::ShapeCodec>,
    ) -> Self {
        #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
        let desc = unsafe { &*class };
        #[expect(unsafe_code, reason = "the caller guarantees the contract is live")]
        let shape = shape.map(|codec| unsafe { &*codec });
        Self {
            class,
            desc,
            shape,
            reading: Reading::Wire,
        }
    }

    /// The same contract over values of `reading`.
    ///
    /// Set here rather than in [`Contract::new`] because [`check_codec`] asks
    /// only what the *declaration* says, and a parameter it never reads on the
    /// call every door already makes would be one more thing to get right for
    /// nothing.
    const fn over(self, reading: Reading) -> Self {
        Self {
            class: self.class,
            desc: self.desc,
            shape: self.shape,
            reading,
        }
    }

    /// Every field this decode walks, in the order the class lays its slots
    /// out.
    fn fields(&self) -> &'a [nvs_runtime::CodecField] {
        match self.shape {
            Some(codec) => codec.fields(),
            None => self.desc.codec(),
        }
    }

    /// The descriptor the `index`th field decodes into, or `None` where that
    /// field names no class — [`nvs_runtime::ClassDesc::codec_class`] and
    /// [`nvs_runtime::ShapeCodec::class`] being one question asked of two
    /// tables.
    fn class_at(&self, index: usize) -> Option<*const nvs_runtime::ClassDesc> {
        match self.shape {
            Some(codec) => codec.class(index),
            None => self.desc.codec_class(index),
        }
    }

    /// The contract the `index`th field decodes against, or `None` where that
    /// field names no inline shape — [`Self::class_at`]'s question asked of the
    /// same two tables for the second pointer a `CodecTy::Shape` needs, which
    /// `nvs_runtime::CodecTy::Shape`'s own docs say why it needs.
    fn shape_at(&self, index: usize) -> Option<*const nvs_runtime::ShapeCodec> {
        match self.shape {
            Some(codec) => codec.shape(index),
            None => self.desc.codec_shape(index),
        }
    }

    /// How many positions the walk fills: a shape's own field count, since it
    /// writes slots, and a derived class's declared constructor arity.
    fn arity(&self) -> usize {
        match self.shape {
            Some(codec) => codec.fields().len(),
            None => self.desc.ctor_arity(),
        }
    }

    /// The class's rendered name, which is what a message names — `$shape{n}`
    /// for a shape, since that label is the only name it has.
    fn name(&self) -> &'a str {
        self.desc.name()
    }

    /// The descriptor an instance is made of.
    const fn class(&self) -> *const nvs_runtime::ClassDesc {
        self.class
    }

    /// Whether an inline shape's contract is what this walks.
    const fn is_shape(&self) -> bool {
        self.shape.is_some()
    }
}

/// The questions `class` answers about itself before a document is read, for
/// `member`.
///
/// Split out of [`decode_as`] because a caller that reads the document itself
/// still owes them, and owes them in this order: `Core\Request::jsonAs` asks
/// here before it claims the request body, so a class that could never have
/// been built refuses without spending a reading on it.
///
/// # Safety
///
/// As [`decode_as`]'s.
///
/// # Errors
///
/// `LogicError` where `class` carries no derived codec at all, which is a
/// defect in the program.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub(crate) unsafe fn check_codec(
    class: *const nvs_runtime::ClassDesc,
    shape: Option<*const nvs_runtime::ShapeCodec>,
    member: &str,
) -> Result<(), Fault> {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the descriptor and the contract are live"
    )]
    let contract = unsafe { Contract::new(class, shape) };
    let fields = contract.fields();
    // Asked of a class only. An inline shape *is* a wire contract — there is no
    // declaration it could have opted in on and nothing for this to send the
    // reader to write, which is why the message names the attribute.
    if fields.is_empty() && !contract.is_shape() {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{member}(): `{}` has no JSON codec — a class participates by \
                 carrying `#[Json\\Derive]`",
                contract.name()
            ),
        ));
    }
    // Checked before the document is even read: an undecoded field is a
    // declared type `rule:core-classes/derive-field-list`'s reachable test
    // already refused, not something the input did, so it is an engine fault
    // rather than an issue in a list a program shows a user.
    if let Some(field) = fields.iter().find(|field| {
        undecoded(field.ty)
            || field
                .element
                .as_ref()
                .is_some_and(|element| element.any(undecoded))
    }) {
        return Err(Fault::fatal(format!(
            "{member}(): `{}`'s `{}` field has a declared type no wire type describes — \
             `rule:core-classes/derive-field-list`'s reachable test refuses that declaration, \
             so reaching it here is the compiler disagreeing with itself",
            contract.name(),
            field.key
        )));
    }
    Ok(())
}

/// Whether a wire type is one no JSON document can carry — written once so
/// [`decode_as`]'s pre-check and [`decode_field`]'s own arm cannot come to hold
/// different rosters.
///
/// Neither is reachable from a program `nvs_types::derive` accepted: an
/// `Opaque` is a declared type its reachable test refuses outright, and a
/// `bytes` is on the roster for completeness, `rule:types/bytes` giving it no
/// JSON spelling at all. So both are a backstop against a descriptor built by
/// hand rather than a decoder this crate still owes.
const fn undecoded(ty: CodecTy) -> bool {
    matches!(ty, CodecTy::Opaque | CodecTy::Bytes)
}

/// One instance of `class` out of a document already read — or, for `list`, one
/// per element of it.
///
/// Split out of [`decode_as`] for a borrow: `Core\Request::jsonAs` reads its
/// document out of the octets the request holds, and the `&mut Ctx` this half
/// needs cannot be taken while that borrow is live. Reading is the half that
/// needs no context, so the two halves are two calls. The caller owes
/// [`check_codec`] first.
///
/// `document` is transferred: this releases it whichever branch ran and whether
/// or not it failed.
///
/// `reading` is what the values in it already are — [`Reading`] owns the one
/// place the two answers differ, and every door but `Core\Arr::shapeAs` and
/// its wrappers is a [`Reading::Wire`] one.
///
/// # Safety
///
/// As [`decode_as`]'s.
///
/// # Errors
///
/// `ParseError` where the document is not the shape `class` decodes from, or
/// its fields are not what the class declared.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub(crate) unsafe fn hydrate(
    ctx: &mut nvs_runtime::Ctx,
    class: *const nvs_runtime::ClassDesc,
    shape: Option<*const nvs_runtime::ShapeCodec>,
    document: Value,
    list: bool,
    reading: Reading,
    member: &str,
) -> Result<Value, Fault> {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the descriptor and the contract are live"
    )]
    let contract = unsafe { Contract::new(class, shape) }.over(reading);
    #[expect(
        unsafe_code,
        reason = "the same live descriptor the caller vouched for"
    )]
    let decoded = unsafe {
        if list {
            decode_each(ctx, contract, document, member)
        } else {
            decode_object(ctx, contract, document, None, member)
        }
    };
    // Released here whichever branch ran and whether or not it failed: every
    // value either half kept out of the document was retained on its way past,
    // so this frees exactly what nothing else holds.
    #[expect(
        unsafe_code,
        reason = "this frame holds the only reference the caller handed over"
    )]
    unsafe {
        document.release();
    }
    decoded
}

/// `rule:core-classes/derive-field-list`'s decode run once per element: a JSON array in, one instance of
/// `class` per element out, in the document's own order.
///
/// An element that is not an object, or a field that does not match, refuses
/// the **whole** list rather than the element — a partial `array<T>` would be
/// a shorter list than the document held, which is a lie no caller can see.
///
/// **The refusal stops at the first bad element**, so the issue list is one
/// element's fields with that element's position on each path. `rule:core-classes/derive-reports-every-field`
/// accumulates *within* an object because a class's field count is a bound the
/// program wrote; a list's length is a bound the document wrote, and a decode
/// of untrusted input that reported an issue per element would do work in
/// proportion to what an attacker sent.
///
/// # Safety
///
/// As [`decode_as`]'s, and `document` must be a reference this frame's caller
/// keeps alive across the call.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_each(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    document: Value,
    member: &str,
) -> Result<Value, Fault> {
    let refusal = || {
        let message = format!(
            "{member}(): an `array<{}>` decodes from a JSON array",
            contract.name()
        );
        let issues = crate::issue::list([("", message.as_str())]);
        Fault::thrown_with_issues(ThrownClass::Parse, message, issues)
    };
    let Some(ptr) = document.array_ptr() else {
        return Err(refusal());
    };
    let source = crate::arr::borrowed(ptr);
    let mut decoded = NvsArray::new();
    for index in 0..source.count() {
        // A JSON array reads back as a *packed* array, so a position that is
        // not there is a JSON object arriving at a list decode: `{"a": 1}` has
        // a count of one and no index 0. This is the only place the two are
        // told apart, since both are one `NvsArray`.
        let element = i64::try_from(index)
            .ok()
            .and_then(|position| source.get_index(position))
            .ok_or_else(refusal)?;
        // Dropping `decoded` on the way out releases every instance already
        // built, which is what the early return owes.
        #[expect(
            unsafe_code,
            reason = "the same live descriptor, and the document's own element"
        )]
        let value = unsafe { decode_object(ctx, contract, element, Some(index), member)? };
        decoded.append(value);
    }
    Ok(Value::array(decoded))
}

/// `rule:core-classes/derive-reports-every-field`'s decode of one
/// object: every field read into a local, **every** failure accumulated, and
/// the constructor run only if none was.
///
/// `at` is the element's position when this object came out of a list, and is
/// the whole of what a list adds to § 5's issue paths — `2.name` rather than
/// `name`.
///
/// # Safety
///
/// As [`decode_as`]'s, and `document` must be a reference this frame's caller
/// keeps alive across the call.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_object(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    document: Value,
    at: Option<usize>,
    member: &str,
) -> Result<Value, Fault> {
    let prefix = match at {
        None => String::new(),
        Some(index) => format!("{index}."),
    };
    let Some(ptr) = document.array_ptr() else {
        let message = match at {
            None => format!(
                "{member}(): a `{}` decodes from a JSON object",
                contract.name()
            ),
            Some(index) => format!(
                "{member}(): element {index} is not a JSON object, and a `{}` \
                 decodes from one",
                contract.name()
            ),
        };
        let issues = crate::issue::list([(path_of(&prefix, None).as_str(), message.as_str())]);
        return Err(Fault::thrown_with_issues(
            ThrownClass::Parse,
            message,
            issues,
        ));
    };

    let source = crate::arr::borrowed(ptr);
    #[expect(
        unsafe_code,
        reason = "the same live descriptor, and a borrow of the document's own object"
    )]
    match unsafe { decode_fields(ctx, contract, &source, &prefix) } {
        Ok(value) => Ok(value),
        Err(DecodeFailure::Fault(fault)) => Err(fault),
        Err(DecodeFailure::Issues(issues)) => Err(Fault::thrown_with_issues(
            ThrownClass::Parse,
            format!(
                "{member}(): {} field(s) of `{}` did not match",
                issues.len(),
                contract.name()
            ),
            crate::issue::list(
                issues
                    .iter()
                    .map(|(path, message)| (path.as_str(), message.as_str())),
            ),
        )),
    }
}

/// How one field, or one whole nested object, failed.
///
/// The two are not the same failure: `rule:core-classes/derive-reports-every-field` **accumulates** issues
/// across an object's fields, so a nested object's issues have to travel back
/// up and join the enclosing object's list rather than throwing where they were
/// found — while a [`Fault`] is the decoder admitting a gap of its own and ends
/// the decode wherever it happens.
enum DecodeFailure {
    /// `rule:core-classes/derive-reports-every-field` issues, each an already-rooted path and its message.
    Issues(Vec<(String, String)>),
    /// A fault that ends the whole decode — an engine gap, never the
    /// document's doing.
    Fault(Fault),
}

/// One object's fields decoded into the positions that fill them, and the
/// instance built — `rule:core-classes/derive-reports-every-field`'s
/// accumulate-then-construct, over an object whose JSON shape a caller has
/// already checked.
///
/// A position is a constructor parameter under a class contract and a field
/// slot under a shape's, which is the whole of what [`build_shape`] is for: the
/// accumulation is one vector either way, and only the last statement knows
/// which door it goes out of.
///
/// `prefix` is what § 5's issue paths are rooted at: `""` at the top, `2.`
/// inside a list's third element, `2.address.` inside that element's nested
/// `address`. It is a string rather than an index because nesting is
/// unbounded and a list position is only its first level.
///
/// # Safety
///
/// As [`decode_as`]'s, and `source` must be a borrow the caller keeps alive
/// across the call.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_fields(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    source: &NvsArray,
    prefix: &str,
) -> Result<Value, DecodeFailure> {
    let fields = contract.fields();
    let arity = contract.arity();
    let mut ctor_args = vec![Value::null(); arity];
    let mut filled = vec![false; arity];
    let mut issues: Vec<(String, String)> = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        #[expect(
            unsafe_code,
            reason = "the descriptor is the caller's, and a nested field's is one \
                      `nvs-codegen` resolved out of the same class table"
        )]
        let outcome = unsafe { decode_field(ctx, contract, index, source, prefix) };
        match outcome {
            Ok(value) => match ctor_args.get_mut(field.param) {
                Some(slot) => {
                    *slot = value;
                    filled[field.param] = true;
                }
                None => {
                    release_all(&ctor_args);
                    #[expect(
                        unsafe_code,
                        reason = "this frame holds this value's only reference, and \
                                  the argument vector it was destined for has no room"
                    )]
                    unsafe {
                        value.release();
                    }
                    // Unreachable from source with no diagnostic to name:
                    // `field.param` and the arity are two readings of one
                    // contract, both written while compiling the call site or
                    // the class, so a field naming a position it does not have
                    // is a generated table disagreeing with itself rather than
                    // anything a program can write.
                    return Err(DecodeFailure::Fault(Fault::fatal(format!(
                        "internal error: `{}`'s `{}` field names position {} of {arity}",
                        contract.name(),
                        field.key,
                        field.param
                    ))));
                }
            },
            // § 5's accumulation, and the one place a nested object's issues
            // join the enclosing object's: they already carry their own rooted
            // paths, so this is an append rather than a re-rooting.
            Err(DecodeFailure::Issues(mut nested)) => issues.append(&mut nested),
            Err(DecodeFailure::Fault(fault)) => {
                release_all(&ctor_args);
                return Err(DecodeFailure::Fault(fault));
            }
        }
    }

    if !issues.is_empty() {
        release_all(&ctor_args);
        return Err(DecodeFailure::Issues(issues));
    }
    // `rule:core-classes/derive-field-list`'s skipped field is the one shape
    // that leaves a position unfilled: a field the contract carries took its
    // own default above, and a property `skip: true` removed from the contract
    // leaves a parameter no field names, so there is no field here to read a
    // constant off. `nvs_types::derive`'s `check_json_sites` refuses a call
    // naming such a class while compiling; this is the backstop for a class
    // built by hand, and it is loud rather than passing `null`, which would be
    // right for `?T $x = null` and silently wrong for everything else. A shape
    // reaches this with every position filled, an absent optional key included:
    // what fills that one is the never-written marker.
    if let Some(index) = filled.iter().position(|done| !done) {
        release_all(&ctor_args);
        return Err(DecodeFailure::Fault(Fault::fatal(format!(
            "Core\\Json::decodeAs(): `{}`'s constructor parameter {index} is not a codec \
             field, which `E0820` refuses at the call that names such a class",
            contract.name()
        ))));
    }
    if contract.is_shape() {
        #[expect(
            unsafe_code,
            reason = "the same live descriptor, and every value is one this frame \
                      owns and hands over"
        )]
        return unsafe { build_shape(contract, ctor_args) };
    }
    #[expect(
        unsafe_code,
        reason = "the same live descriptor, and every argument is one this frame \
                  owns and hands over"
    )]
    unsafe { nvs_runtime::construct(ctx, contract.class(), &ctor_args) }
        .map_err(DecodeFailure::Fault)
}

/// One shape's decoded fields written into a fresh instance of its class — the
/// half a derived class gets by running its own constructor.
///
/// A shape class declares none: `rule:types/object-literal` gives a shape
/// literal no constructor to write, so the class `nvs-ir` synthesizes lays its
/// slots out in the shape's sorted field-name order and every writer fills them
/// one at a time. That order is why a shape's
/// [`nvs_runtime::CodecField::param`] and `slot` are the same number, and why
/// [`decode_fields`] can accumulate into one vector for both doors.
///
/// `values` is transferred: every reference in it is written into the object or
/// released.
///
/// # Safety
///
/// As [`decode_as`]'s.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn build_shape(
    contract: Contract<'_>,
    mut values: Vec<Value>,
) -> Result<Value, DecodeFailure> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let object = unsafe { NvsObj::new(contract.class()) };
    for field in contract.fields() {
        // Unreachable from source with no diagnostic to name: a shape class's
        // slots are the sorted field names its contract was built from, so a
        // slot past the end is `nvs-ir`'s two halves disagreeing about one
        // written shape. Checked here rather than left to
        // `nvs_runtime::NvsObj::set_field`, whose answer is a panic and so
        // costs the process rather than the request.
        if field.param >= values.len() || field.slot >= object.field_count() {
            release_all(&values);
            return Err(DecodeFailure::Fault(Fault::fatal(format!(
                "internal error: `{}`'s `{}` field names position {} and slot {} of {}",
                contract.name(),
                field.key,
                field.param,
                field.slot,
                object.field_count()
            ))));
        }
        // Left `null` behind so the release above frees each value exactly
        // once, whichever field a later failure stops at.
        let value = std::mem::replace(&mut values[field.param], Value::null());
        object.set_field(field.slot, value);
    }
    Ok(Value::object(object))
}

/// The `index`th field of `contract`, taken over, or how it failed.
///
/// `rule:core-api/required-optional-and-nullable`'s table, and which of its
/// rows are answered here is the one place the two contracts part.
///
/// A **class** gets every row a program can write. An absent key is that rule's
/// first column alone — `nvs_runtime::CodecField::required` carries it down
/// from the declaration — and an absent *optional* key is filled from
/// `nvs_runtime::CodecField::default`, the constant beside it, which is the
/// constructor parameter's own default carried here because this decoder is not
/// the call site that would otherwise emit it. The row left out is
/// `?T $x = null`, which no program can write: a written `= null` parameter
/// default is refused while checking (`nvs_types::defaults`).
///
/// A **shape** has no constructor and therefore no default to be missing, so
/// both columns are answered rather than deferred:
///
/// * **An absent optional key** answers the never-written marker
///   ([`nvs_runtime::Value::unset`]), which is the storage state
///   `rule:types/shape-type`'s "an absent key is that same rule's checked,
///   catchable throw" is read back out of. Writing `null` would instead make
///   `{a?: int}` and `{a: ?int}` — two types that intern apart — hold the same
///   thing.
/// * **A `?T` field is `expr as ?T`**, which answers `null` exactly where
///   `as T` would throw (`rule:expressions/nullable-conversion`), so a value
///   the wire type refuses is that `null` rather than an issue. Only the
///   *conversion* is answered that way: presence is the other column, kept
///   independent of the type, so a required `?T` whose key is missing still
///   fails.
///
/// Takes the contract and a position rather than the
/// [`nvs_runtime::CodecField`] alone because a nested field's class is
/// [`Contract::class_at`]'s answer, indexed the same way — the field itself
/// carries the label and never the descriptor.
///
/// # Safety
///
/// As [`decode_as`]'s, for the contract's own descriptor and for the nested one
/// it hands out.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_field(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    index: usize,
    source: &NvsArray,
    prefix: &str,
) -> Result<Value, DecodeFailure> {
    let field = &contract.fields()[index];
    let issue = |why: String| DecodeFailure::Issues(vec![(path_of(prefix, Some(&field.key)), why)]);
    let Some(found) = source.get(field.key.as_bytes()) else {
        // `nvs_runtime::CodecField::required` is what tells the two absences
        // apart, and this is the presence column alone: a required field fails
        // whatever its type admits.
        if field.required {
            return Err(issue("required field missing".to_owned()));
        }
        // An optional key is filled by the declaration that made it optional —
        // the constructor parameter's default, evaluated while compiling and
        // carried onto the field because this decoder is not a call site.
        if let Some(default) = &field.default {
            return Ok(default.materialize());
        }
        if contract.is_shape() {
            return Ok(Value::unset());
        }
        // A class field is optional exactly when its parameter declares a
        // default, so a descriptor reaching this is one built by hand rather
        // than by `nvs_types::derive` — or the module's own gap 1, a position a
        // `skip: true` left with no field to carry a constant on.
        return Err(issue(
            "optional field missing, and its constructor parameter carries no constant \
             to fill it with"
                .to_owned(),
        ));
    };
    #[expect(
        unsafe_code,
        reason = "the contract's descriptor is the caller's, and a nested field's \
                  is one `nvs-codegen` resolved out of the same class table"
    )]
    let converted = unsafe { convert_field(ctx, contract, index, found, prefix) };
    match converted {
        // The `as ?T` above, in the one place every conversion under this field
        // comes back through — a nested object's issue list included, since
        // what failed is the whole conversion and `as ?T` has one answer for
        // that. A `Fault` is the decoder admitting a gap of its own and is
        // never a conversion's answer, so it passes through untouched.
        Err(DecodeFailure::Issues(_)) if contract.is_shape() && field.nullable => Ok(Value::null()),
        outcome => outcome,
    }
}

/// The `index`th field's value converted, for a key the document carries —
/// [`decode_field`]'s second half.
///
/// Split from it so that the presence column is answered once, above, and every
/// path a *conversion* can fail on comes back through one place — which is what
/// lets a shape's `?T` field answer `null` without also swallowing an absent
/// key.
///
/// # Safety
///
/// As [`decode_field`]'s.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn convert_field(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    index: usize,
    found: Value,
    prefix: &str,
) -> Result<Value, DecodeFailure> {
    let field = &contract.fields()[index];
    let issue = |why: String| DecodeFailure::Issues(vec![(path_of(prefix, Some(&field.key)), why)]);
    if found.tag() == Some(Tag::Null) {
        if field.nullable {
            return Ok(Value::null());
        }
        return Err(issue("null is not permitted".to_owned()));
    }
    let converted = match field.ty {
        // `rule:core-classes/derive-field-list`'s nested class and its inline
        // shape, both decoded by running a field list over this key's object —
        // which is why § 5's paths are dotted in the first place. What differs
        // is only where that list is read from, which [`Contract`] answers.
        CodecTy::Class | CodecTy::Shape => {
            #[expect(
                unsafe_code,
                reason = "`nvs-codegen` resolved this out of the same class table \
                          the contract came from, so it lives exactly as long"
            )]
            return unsafe { decode_nested(ctx, contract, index, found, prefix) };
        }
        // `rule:core-classes/derive-field-list`'s list field, decoded one element at a time under a
        // path this key extends — the second nesting § 5's dotted path covers.
        CodecTy::List => {
            #[expect(
                unsafe_code,
                reason = "the contract is the caller's, and an element class is one \
                          `nvs-codegen` resolved out of the same class table"
            )]
            return unsafe { decode_list(ctx, contract, index, found, prefix) };
        }
        CodecTy::Mixed
        | CodecTy::Bool
        | CodecTy::Int
        | CodecTy::Uint
        | CodecTy::Float
        | CodecTy::Str
        | CodecTy::Decimal
        | CodecTy::Instant => {
            #[expect(
                unsafe_code,
                reason = "the document owns this value for the length of this call"
            )]
            unsafe {
                scalar(field.ty, None, found, contract.reading)
            }
        }
        // `rule:core-classes/derive-field-list`'s enum field: the roster travels with the field and the
        // decode is a membership test over it, so this is a scalar with one
        // more thing in hand rather than a nesting of its own.
        CodecTy::Enum => {
            let cases = cases_of(contract, field)?;
            #[expect(
                unsafe_code,
                reason = "the document owns this value for the length of this call"
            )]
            unsafe {
                scalar(field.ty, Some(cases), found, contract.reading)
            }
        }
        // Reachable only through a *nested* class, whose own fields
        // [`decode_as`]'s pre-check never saw: an [`undecoded`] wire type is a
        // declaration `rule:core-classes/derive-field-list`'s reachable test
        // refuses, so it is an engine fault wherever it is met and never an
        // issue in a list a program shows a user.
        CodecTy::Opaque | CodecTy::Bytes => {
            return Err(DecodeFailure::Fault(Fault::fatal(format!(
                "Core\\Json::decodeAs(): `{}`'s `{}` field has a declared type no wire type \
                 describes — `rule:core-classes/derive-field-list`'s reachable test refuses \
                 that declaration, so reaching it here is the compiler disagreeing with itself",
                contract.name(),
                field.key
            ))));
        }
    };
    let Some(value) = converted else {
        return Err(issue(format!(
            "expected {}, found {}",
            wanted(field.ty),
            describe(found)
        )));
    };
    // [`scalar`] answered with a reference of its own, which is what the
    // object being built needs: the document is released before the
    // constructor runs.
    Ok(value)
}

/// One field of a nested class decoded — `rule:core-classes/derive-field-list`'s "another class that
/// itself has a codec", run over the object this key holds.
///
/// The class is the compiler's answer and never the document's: the descriptor
/// comes out of [`Contract::class_at`], which `nvs-codegen` resolved from the
/// declared type. A decoder that read a class name out of the JSON would let
/// untrusted input choose which constructor runs.
///
/// What it nests into is a class's contract or an inline shape's, which are one
/// walk over two tables: a shape field's descriptor is the class a literal of
/// those same field names builds, and the per-field wire types that label cannot
/// carry come from the [`nvs_runtime::ShapeCodec`] beside it
/// ([`Contract::shape_at`]).
///
/// # Safety
///
/// As [`decode_as`]'s, for the contract's own descriptor and for the one it
/// names at `index`.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_nested(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    index: usize,
    found: Value,
    prefix: &str,
) -> Result<Value, DecodeFailure> {
    let field = &contract.fields()[index];
    let Some(class) = contract.class_at(index) else {
        // Unreachable from source: `nvs_types::derive` refused a field type
        // with no codec at the declaration, so a label with no descriptor is
        // `nvs-codegen`'s join disagreeing with the class table it built.
        return Err(DecodeFailure::Fault(Fault::fatal(format!(
            "internal error: `{}`'s `{}` field decodes into `{}`, which this unit's class \
             table has no descriptor for",
            contract.name(),
            field.key,
            field.class.as_deref().unwrap_or("<unnamed>")
        ))));
    };
    #[expect(unsafe_code, reason = "the descriptor `nvs-codegen` resolved is live")]
    // The reading carries down: what the values are is a property of the door
    // the whole call came through, not of how deep the field sits.
    let nested = unsafe { Contract::new(class, contract.shape_at(index)) }.over(contract.reading);
    // Asked of a class only, on [`check_codec`]'s terms: an inline shape *is* a
    // wire contract, and `{}` is an empty one rather than a missing one.
    if nested.fields().is_empty() && !nested.is_shape() {
        return Err(DecodeFailure::Fault(Fault::fatal(format!(
            "Core\\Json::decodeAs(): `{}`'s `{}` field decodes into `{}`, which carries no \
             derived codec — `rule:core-classes/derive-generates-what-is-missing`'s hand-written half is `nvs_stdlib::json`'s own \
             known gap",
            contract.name(),
            field.key,
            nested.name()
        ))));
    }
    let Some(ptr) = found.array_ptr() else {
        return Err(DecodeFailure::Issues(vec![(
            path_of(prefix, Some(&field.key)),
            format!(
                "expected an object for `{}`, found {}",
                nested.name(),
                describe(found)
            ),
        )]));
    };
    let source = crate::arr::borrowed(ptr);
    #[expect(
        unsafe_code,
        reason = "the resolved descriptor, and a borrow of the document's own object"
    )]
    unsafe {
        decode_fields(ctx, nested, &source, &format!("{prefix}{}.", field.key))
    }
}

/// One value read as a scalar wire type, or `None` when it is not one and
/// `reading` gives no row that makes it one.
///
/// Split out of [`decode_field`] because [`decode_list`] asks the same
/// question of every element, and a list whose elements converted by their own
/// rules would be a second answer to "what is an `int` here".
///
/// A [`CodecTy::Class`], a [`CodecTy::List`] and every [`undecoded`] wire type
/// are not scalars this reads and answer `None`; each has a caller that handles
/// it before reaching here.
///
/// `cases` is [`nvs_runtime::CodecField::cases`], and is read only for a
/// [`CodecTy::Enum`] — the one wire type whose accepted values are a property
/// of the field rather than of the type. Both callers resolve it before
/// asking, so a `None` here is an already-diagnosed program and refuses.
///
/// **The answer is an owned reference.** A pass-through arm retains the
/// document's own value on the way out and a converting arm hands over the one
/// it built, so a caller adds the value to what it is assembling and never
/// asks which happened — which it could not answer anyway once
/// [`Reading::Values`]'s `→ string` row builds a string where the strict read
/// would have refused outright.
///
/// # Safety
///
/// `found` must be a reference the caller keeps alive across the call, since a
/// pass-through arm takes a second one to it.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of the value it borrowed from the document"
)]
unsafe fn scalar(
    ty: CodecTy,
    cases: Option<&EnumCases>,
    found: Value,
    reading: Reading,
) -> Option<Value> {
    let coerce = reading == Reading::Values;
    let kept = match ty {
        // A `mixed` field is exactly as checked as `mixed` ever is (`rule:core-classes/derive-field-list`), so whatever the document held is the value.
        CodecTy::Mixed => Some(found),
        // No row of `rule:types/conversion` lands on `bool` — PHP's truthy
        // table is a *condition*'s answer and not a conversion's — so both
        // readings want the value to be one already. A `"1"` from a checkbox
        // is therefore a failed field and not a `true`, which is the whole
        // point of the table being closed.
        CodecTy::Bool => found.as_bool().map(Value::bool),
        CodecTy::Int if coerce => nvs_runtime::to_int(found).map(Value::int),
        CodecTy::Int => found.as_int().map(Value::int),
        CodecTy::Uint if coerce => nvs_runtime::to_uint(found).map(Value::uint),
        CodecTy::Uint => found
            .as_int()
            .and_then(|number| u64::try_from(number).ok())
            .map(Value::uint),
        CodecTy::Float if coerce => nvs_runtime::to_float(found).map(Value::float),
        // A JSON `1` reaching a `float` field widens, which is the one place
        // Novis does that — `rule:types/conversion` has no int-to-float widening in the
        // language, but a wire format has one number type and refusing an
        // unfractional literal would make `1.0` and `1` different documents.
        CodecTy::Float => found
            .as_float()
            .or_else(|| found.as_int().map(|number| number as f64))
            .map(Value::float),
        // `rule:types/conversion`'s "anything → `string`: total for scalars",
        // which is the one converting row that allocates. An array or an
        // object is not a scalar and needs `Stringable`, so both refuse here
        // rather than reaching for a method a value read out of a form has no
        // business having.
        CodecTy::Str if coerce && found.tag() != Some(Tag::Str) => {
            return nvs_runtime::value_to_string(found).ok();
        }
        CodecTy::Str => (found.tag() == Some(Tag::Str)).then_some(found),
        // `rule:enums/representation` reserves an enum tag and nothing writes one, so a case
        // is the integer behind it and there is nothing to construct: what a
        // decode owes is the membership test, and an integer outside the
        // roster is a bad document rather than a case this build forgot.
        CodecTy::Enum => {
            let cases = cases?;
            // The backing value first and the membership test after, which is
            // `rule:types/conversion`'s "backing type / `mixed` → `EnumName`"
            // read in the order it is written. Under `Values` the backing
            // value is `"2"` as often as `2`, so the row that reaches it is
            // the same one an `int` field's is.
            let number = if coerce {
                nvs_runtime::to_int(found)?
            } else {
                found.as_int()?
            };
            if cases.values.binary_search(&i128::from(number)).is_err() {
                return None;
            }
            // A `uint`-backed enum's case is a `Value::uint` everywhere else
            // in the runtime — `nvs_ir::lower::expr` emits one for a written
            // `Role::Admin` — so a decoded case has to be the same value a
            // written one is, or the two would compare unequal.
            return if cases.unsigned {
                u64::try_from(number).ok().map(Value::uint)
            } else {
                Some(Value::int(number))
            };
        }
        // `rule:types/decimal`'s exactness is why the wire form is a **string**:
        // JSON has one number type, and a document's number is an `f64` by the
        // time this sees it, so a `decimal` spelled as a number could not be
        // read back as the value that was written. [`Encodable`] writes the
        // same spelling, so the two halves are one round trip. A `Values`
        // reading wants a string too — a form field is text — and a value that
        // already is a `decimal` passes through, which is the only way
        // `Core\Arr::shapeAs` meets one here at all.
        CodecTy::Decimal => match found.tag() {
            Some(Tag::Decimal) => Some(found),
            Some(Tag::Str) => found
                .as_text()
                .and_then(nvs_runtime::Decimal::parse)
                .map(Value::decimal),
            _ => None,
        },
        // `Core\Time\Instant`'s wire form, fixed at RFC 3339 text by this
        // module's gap 1: the type is a point on the timeline and the string is
        // what every other reader of the document already agrees that is. The
        // instance it builds owns the one reference it was made with, so that
        // arm returns rather than reaching the retain below; an `Instant` the
        // caller already holds is a pass-through, which is the only way
        // `Core\Arr::shapeAs` meets the type.
        CodecTy::Instant => match found.tag() {
            Some(Tag::Object) if crate::instance::is_instance(found, &crate::time::INSTANT) => {
                Some(found)
            }
            Some(Tag::Str) => return found.as_text().and_then(crate::time::instant_from_iso),
            _ => None,
        },
        CodecTy::Class | CodecTy::Shape | CodecTy::List | CodecTy::Opaque | CodecTy::Bytes => None,
    }?;
    // Every arm reaching here either passed the document's own value through
    // or built an unrefcounted scalar, and a retain on the second is the
    // no-op `Value::retain` documents. The two arms that allocate return
    // above, holding the one reference they made.
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the document owns this value for the \
                  length of the call, so a second reference to it is sound"
    )]
    unsafe {
        kept.retain();
    }
    Some(kept)
}

/// The enum roster `field` carries, or the engine fault a missing one is.
///
/// Unreachable from source: `nvs_types::derive` writes the roster beside the
/// [`CodecTy::Enum`] in one expression, so an enum field without one is that
/// erasure disagreeing with itself — the same shape [`decode_list`] gives a
/// list with no element wire type.
fn cases_of<'a>(
    contract: Contract<'_>,
    field: &'a nvs_runtime::CodecField,
) -> Result<&'a EnumCases, DecodeFailure> {
    // Unreachable from source, as the doc comment above says: the roster is
    // written beside the `CodecTy::Enum` in one expression.
    field.cases.as_ref().ok_or_else(|| {
        DecodeFailure::Fault(Fault::fatal(format!(
            "internal error: `{}`'s `{}` field is an enum with no case roster",
            contract.name(),
            field.key
        )))
    })
}

/// One `array<T>` field decoded — `rule:core-classes/derive-field-list`'s list field, every position
/// read through [`nvs_runtime::CodecField::element`] and **every** bad one
/// accumulated, so a list reports like an object rather than at its first
/// failure.
///
/// The element's issue path is the field's own path with the position appended
/// — `tags.3`, which is § 5's own example — and an element that is a class, a
/// shape or a list of either nests once further, `authors.1.name` and
/// `grid.2.0`.
///
/// # Safety
///
/// As [`decode_as`]'s, for `owner` and for the element descriptor it names at
/// `index`.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_list(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    index: usize,
    found: Value,
    prefix: &str,
) -> Result<Value, DecodeFailure> {
    let field = &contract.fields()[index];
    let path = path_of(prefix, Some(&field.key));
    let Some(element) = field.element.as_ref() else {
        // Unreachable from source: `nvs_types::derive` writes the element
        // beside the `List` in one expression, so a list with no element is
        // that erasure disagreeing with itself.
        return Err(DecodeFailure::Fault(Fault::fatal(format!(
            "internal error: `{}`'s `{}` field is a list with no element wire type",
            contract.name(),
            field.key
        ))));
    };
    #[expect(
        unsafe_code,
        reason = "the contract is the caller's, and an element class is one \
                  `nvs-codegen` resolved out of the same class table"
    )]
    unsafe {
        decode_positions(ctx, contract, index, element, found, &path)
    }
}

/// One array's positions decoded against `element` — [`decode_list`]'s body,
/// split out so that a nested `array<array<T>>` is this same walk one link
/// further down the chain rather than a second reading of what a list is.
///
/// The labels stay on the *field* at every depth
/// ([`nvs_runtime::CodecElement`]), so a class element four lists deep resolves
/// through the same [`Contract::class_at`] the outermost one does: what the
/// recursion carries is the chain, and what it does not carry is a second
/// resolution.
///
/// # Safety
///
/// As [`decode_list`]'s.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_positions(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    index: usize,
    element: &nvs_runtime::CodecElement,
    found: Value,
    path: &str,
) -> Result<Value, DecodeFailure> {
    let field = &contract.fields()[index];
    let Some(ptr) = found.array_ptr() else {
        return Err(DecodeFailure::Issues(vec![(
            path.to_owned(),
            format!("expected an array, found {}", describe(found)),
        )]));
    };
    let source = crate::arr::borrowed(ptr);
    // Resolved once rather than per position: an element's roster is the
    // field's own, as its class label is.
    let cases = if element.ty == CodecTy::Enum {
        Some(cases_of(contract, field)?)
    } else {
        None
    };
    // Dropping `decoded` on any early return releases every element already
    // built, which is what an error path owes.
    let mut decoded = NvsArray::new();
    let mut issues: Vec<(String, String)> = Vec::new();
    for at in 0..source.count() {
        let at_path = format!("{path}.{at}");
        // A JSON object reads back as one `NvsArray` too, so a position that
        // is not there is `{"a": 1}` arriving where a list was declared —
        // `decode_each` tells the two apart the same way.
        let Some(item) = i64::try_from(at)
            .ok()
            .and_then(|position| source.get_index(position))
        else {
            return Err(DecodeFailure::Issues(vec![(
                path.to_owned(),
                format!("expected an array, found {}", describe(found)),
            )]));
        };
        // A nested object at a position, whichever of the two kinds it is: a
        // declared class and an inline shape are one walk over two tables, and
        // [`decode_element`] reads both out of the field's own index.
        let nested = match element.ty {
            CodecTy::Class | CodecTy::Shape =>
            {
                #[expect(
                    unsafe_code,
                    reason = "the element descriptor `nvs-codegen` resolved out of the \
                              owner's own class table"
                )]
                Some(unsafe { decode_element(ctx, contract, index, item, &at_path) })
            }
            CodecTy::List => Some(match element.element.as_deref() {
                #[expect(
                    unsafe_code,
                    reason = "the same contract and the same resolved descriptor, one \
                              link further down the element chain"
                )]
                Some(inner) => unsafe {
                    decode_positions(ctx, contract, index, inner, item, &at_path)
                },
                // Unreachable from source, as [`decode_list`]'s own missing
                // element is: `nvs_types::derive` writes a `List` and the link
                // under it in one expression.
                None => Err(DecodeFailure::Fault(Fault::fatal(format!(
                    "internal error: `{}`'s `{}` field nests a list with no element wire type",
                    contract.name(),
                    field.key
                )))),
            }),
            _ => None,
        };
        if let Some(outcome) = nested {
            match outcome {
                Ok(value) => decoded.append(value),
                Err(DecodeFailure::Issues(mut nested)) => issues.append(&mut nested),
                Err(fault @ DecodeFailure::Fault(_)) => return Err(fault),
            }
            continue;
        }
        #[expect(
            unsafe_code,
            reason = "the document owns this element for the length of this call"
        )]
        let converted = unsafe { scalar(element.ty, cases, item, contract.reading) };
        let Some(value) = converted else {
            issues.push((
                at_path,
                format!("expected {}, found {}", wanted(element.ty), describe(item)),
            ));
            continue;
        };
        // As [`decode_field`]'s tail: [`scalar`] answered with a reference of
        // its own, and the document is released before the constructor runs.
        decoded.append(value);
    }
    if !issues.is_empty() {
        return Err(DecodeFailure::Issues(issues));
    }
    Ok(Value::array(decoded))
}

/// One element of a list whose element type is another derived class or an
/// inline shape, decoded under `path` — [`decode_nested`] for a position
/// rather than a key.
///
/// Both pointers are read at the *field's* index, because a list field carries
/// its terminal element's labels rather than its own
/// ([`nvs_runtime::CodecElement`]): the two halves of
/// [`nvs_runtime::CodecField::class`] meet here, and a shape element's
/// contract joins them from [`nvs_runtime::CodecField::shape`] on the same
/// terms.
///
/// # Safety
///
/// As [`decode_as`]'s, for the contract's own descriptor and for the one it
/// names at `index`.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
unsafe fn decode_element(
    ctx: &mut nvs_runtime::Ctx,
    contract: Contract<'_>,
    index: usize,
    item: Value,
    path: &str,
) -> Result<Value, DecodeFailure> {
    let field = &contract.fields()[index];
    let Some(class) = contract.class_at(index) else {
        // Unreachable from source, as [`decode_nested`]'s twin: the element
        // type was refused at the declaration if it had no codec, so a label
        // with no descriptor is `nvs-codegen`'s join disagreeing with the
        // class table it built.
        return Err(DecodeFailure::Fault(Fault::fatal(format!(
            "internal error: `{}`'s `{}` field holds `{}`, which this unit's class table \
             has no descriptor for",
            contract.name(),
            field.key,
            field.class.as_deref().unwrap_or("<unnamed>")
        ))));
    };
    #[expect(unsafe_code, reason = "the descriptor `nvs-codegen` resolved is live")]
    // [`decode_nested`]'s reason, for a list's element class — and its second
    // pointer too, since an element is an inline shape as readily as a field
    // is and the contract holding that shape's field types is resolved beside
    // the class at the same index.
    let element = unsafe { Contract::new(class, contract.shape_at(index)) }.over(contract.reading);
    if element.fields().is_empty() && !element.is_shape() {
        return Err(DecodeFailure::Fault(Fault::fatal(format!(
            "Core\\Json::decodeAs(): `{}`'s `{}` field holds `{}`, which carries no derived \
             codec — `rule:core-classes/derive-generates-what-is-missing`'s hand-written half is `nvs_stdlib::json`'s own known gap",
            contract.name(),
            field.key,
            element.name()
        ))));
    }
    let Some(ptr) = item.array_ptr() else {
        return Err(DecodeFailure::Issues(vec![(
            path.to_owned(),
            format!(
                "expected an object for `{}`, found {}",
                element.name(),
                describe(item)
            ),
        )]));
    };
    let source = crate::arr::borrowed(ptr);
    #[expect(
        unsafe_code,
        reason = "the resolved descriptor, and a borrow of the document's own object"
    )]
    unsafe {
        decode_fields(ctx, element, &source, &format!("{path}."))
    }
}

/// `rule:core-classes/derive-reports-every-field`'s issue path for a field, rooted at whatever encloses it:
/// `name` alone at the top, `2.name` inside a list's third element,
/// `2.address.city` inside that element's nested `address`.
///
/// `prefix` is that rooting, dot-terminated where it is not empty, so a field
/// path is one concatenation. With no field it names the enclosing object
/// itself, which is the prefix with its trailing dot dropped.
///
/// The dotted spelling is the one the § 5 example already uses for a nested
/// field, so a caller walking a list's issues reads the same path grammar it
/// reads for `address.city`.
fn path_of(prefix: &str, field: Option<&str>) -> String {
    match field {
        Some(key) => format!("{prefix}{key}"),
        None => prefix.strip_suffix('.').unwrap_or(prefix).to_owned(),
    }
}

/// What a [`CodecTy`] is called in an issue message — the Novis type name, since
/// that is what the reader has in front of them in the class declaration.
const fn wanted(ty: CodecTy) -> &'static str {
    match ty {
        CodecTy::Bool => "bool",
        CodecTy::Int => "int",
        CodecTy::Uint => "uint",
        CodecTy::Float => "float",
        CodecTy::Str => "string",
        CodecTy::Mixed => "mixed",
        // The two entries that name their wire form rather than only the
        // declared type, because for both of them the document's own spelling is
        // the thing the reader has to change: `rule:types/decimal` is exact and
        // JSON's number is not, and an `Instant` crosses as text JSON has no
        // type for at all. [`scalar`]'s own arms own why each is what it is.
        CodecTy::Decimal => "a decimal as a JSON string",
        CodecTy::Instant => "an instant as an RFC 3339 string",
        // Named for the roster's sake: an [`undecoded`] wire type faults before
        // a document is read, so no issue message reaches for one of these.
        CodecTy::Bytes => "bytes",
        // Never reached through a field: `decode_nested` names the class
        // itself, which is what the reader wrote. Here for the roster.
        CodecTy::Class => "an object",
        // Never reached for [`CodecTy::Class`]'s reason exactly: `decode_nested`
        // names the shape's own label, which is the nearest thing a shape has to
        // what the reader wrote.
        CodecTy::Shape => "an object",
        // Likewise: `decode_list` reports the array itself, and an element is
        // named by its own wire type.
        CodecTy::List => "an array",
        // The enum's own name never reaches the runtime — `CodecTy::Enum`'s
        // docs say why — so the message names the shape rather than the type,
        // and the issue's path names the field the declaration is written on.
        CodecTy::Enum => "a declared enum case",
        CodecTy::Opaque => "a decodable type",
    }
}

/// What the document actually held there, named the way JSON names it.
const fn describe(value: Value) -> &'static str {
    match value.tag() {
        Some(Tag::Null) => "null",
        Some(Tag::Bool) => "a boolean",
        Some(Tag::Int | Tag::Uint | Tag::Float) => "a number",
        Some(Tag::Str) => "a string",
        Some(Tag::Array) => "an object or array",
        _ => "an unrepresentable value",
    }
}

/// Releases every value in `values` — what a decode that is about to throw
/// owes for the fields it had already decoded.
fn release_all(values: &[Value]) {
    for value in values {
        #[expect(
            unsafe_code,
            reason = "each entry is either `null` or a value this frame took a \
                      reference to in `decode_field`"
        )]
        unsafe {
            value.release();
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Json::isValid(string $json): bool` — replacing `json_validate`.
    ///
    /// Answers exactly what [`nvs_core_json_decode`] would accept at the
    /// default depth, by doing it; this module's § *`isValid` decodes and
    /// discards* owns what that costs.
    fn nvs_core_json_is_valid(_ctx, args: [1]) {
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
pub(crate) const DEFAULT_MAX_DEPTH_U32: u32 = DEFAULT_MAX_DEPTH as u32;

/// One `string` argument, as text.
///
/// One failure, the wrong tag: the tag [`Value::as_text`] checks is itself
/// `rule:types/bytes`'s UTF-8 guarantee, so there is no encoding outcome left to report.
/// `crate::str`'s own `text` states why re-deriving it would be an O(n) pass
/// per argument.
fn text_of<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Json::{member} expected {:?}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{ClassDesc, ClassTable, CodecField};

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

    /// One Novis value as the JSON document `Core\Json::encode($v)` writes,
    /// with the member's option bag and `Fault` wrapping left out: what these
    /// tests ask about is the walk and the message it produces.
    fn encoded(value: Value) -> Result<String, String> {
        serde_json::to_string(&Encodable::document(value)).map_err(|why| why.to_string())
    }

    /// A class whose slots are `fields`, each carrying a `mixed` wire key of
    /// its own name — the smallest thing [`Encodable::serialize_object`] will
    /// walk, since a class with an empty codec is refused before a field is
    /// read.
    ///
    /// The table is leaked because a [`ClassDesc`]'s *address* is its identity
    /// and has to outlive every instance made from it; `crate::request`'s
    /// `reading_class` leaks its own for that reason.
    fn holder_class(name: &str, fields: &[&str]) -> *const ClassDesc {
        let mut table = ClassTable::new();
        let id = table.define(name, fields, &[]);
        let codec: Vec<CodecField> = fields
            .iter()
            .enumerate()
            .map(|(slot, key)| CodecField {
                key: (*key).to_owned(),
                slot,
                param: slot,
                ty: CodecTy::Mixed,
                element: None,
                class: None,
                cases: None,
                shape: None,
                nullable: true,
                required: true,
                default: None,
            })
            .collect();
        // One entry per field, null throughout: the encoder reads a field's key
        // and slot and never its class or its contract, so nothing here
        // resolves either.
        let classes = vec![std::ptr::null(); codec.len()];
        let shapes = vec![std::ptr::null(); codec.len()];
        table.set_codec(id, codec, fields.len(), classes, shapes);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        table.desc(id)
    }

    /// A class of one field of wire type `ty`, carrying that field twice: as
    /// the class's own derived codec, which is what [`Encodable`] reads, and
    /// as the inline-shape contract handed to [`decode_as`].
    ///
    /// The contract is what makes the pair testable in one frame. It sends the
    /// decode through [`build_shape`], which writes slots directly, so nothing
    /// here needs the compiled constructor the class door would call.
    ///
    /// Leaked for [`holder_class`]'s reason: a [`ClassDesc`]'s address is its
    /// identity, and it has to outlive every instance made from it.
    fn codec_class(
        name: &str,
        key: &str,
        ty: CodecTy,
    ) -> (*const ClassDesc, *const nvs_runtime::ShapeCodec) {
        let field = || CodecField {
            key: key.to_owned(),
            slot: 0,
            param: 0,
            ty,
            element: None,
            class: None,
            cases: None,
            shape: None,
            nullable: false,
            required: true,
            default: None,
        };
        let mut table = ClassTable::new();
        let id = table.define(name, &[key], &[]);
        table.set_codec(
            id,
            vec![field()],
            1,
            vec![std::ptr::null()],
            vec![std::ptr::null()],
        );
        // The contract is boxed inside the table, so its address survives the
        // leak below unchanged — which is what lets it be taken first.
        let shape = table.define_shape_codec(
            vec![field()],
            vec![std::ptr::null()],
            vec![std::ptr::null()],
        );
        let table: &'static ClassTable = Box::leak(Box::new(table));
        (table.desc(id), shape)
    }

    /// A shape of one field that is itself a shape: the outer descriptor and
    /// contract, with the inner pair resolved into the outer field the way
    /// `nvs-codegen` resolves a [`CodecTy::Shape`]'s two pointers.
    ///
    /// [`nvs_runtime::CodecField::shape`] is written as the key a compiled unit
    /// would carry, though nothing reads it here: at run time the field is the
    /// resolved address beside it, and the key is what the relocation was
    /// derived from a crate earlier.
    ///
    /// Leaked for [`holder_class`]'s reason, which both contracts inherit: they
    /// are boxed inside the table that owns them.
    fn nested_shape_class() -> (*const ClassDesc, *const nvs_runtime::ShapeCodec) {
        let field = |key: &str, ty, class: Option<&str>, shape: Option<&str>| CodecField {
            key: key.to_owned(),
            slot: 0,
            param: 0,
            ty,
            element: None,
            class: class.map(str::to_owned),
            cases: None,
            shape: shape.map(str::to_owned),
            nullable: false,
            required: true,
            default: None,
        };
        let mut table = ClassTable::new();
        let inner_id = table.define("$shape{n}", &["n"], &[]);
        let inner = table.define_shape_codec(
            vec![field("n", CodecTy::Int, None, None)],
            vec![std::ptr::null()],
            vec![std::ptr::null()],
        );
        let outer_id = table.define("$shape{meta}", &["meta"], &[]);
        let inner_desc = table.desc(inner_id);
        let outer = table.define_shape_codec(
            vec![field(
                "meta",
                CodecTy::Shape,
                Some("$shape{n}"),
                Some("$codec{n:Int}"),
            )],
            vec![inner_desc],
            vec![inner],
        );
        let table: &'static ClassTable = Box::leak(Box::new(table));
        (table.desc(outer_id), outer)
    }

    /// `rule:types/decimal`'s exactness reaches roughly 29 significant digits and
    /// a JSON number is an `f64` in every reader there is, so the wire form is
    /// a string — asserted at a width the `f64` spelling demonstrably cannot
    /// hold, decoded into a `decimal` field and written back byte for byte.
    #[test]
    fn decode_as_fills_a_decimal_field_from_the_numbers_own_digits() {
        const DIGITS: &str = "1234567890123456789.012345";

        // The bound this spelling exists for, on the other side: these digits
        // through a `float` are a different number, so a document holding them
        // as one could not be read back as what was written.
        assert_ne!(
            DIGITS
                .parse::<f64>()
                .expect("the digits are a float too")
                .to_string(),
            DIGITS,
            "an `f64` that held these digits would make the string spelling pointless"
        );

        let (class, shape) = codec_class("Money", "amount", CodecTy::Decimal);
        let document = format!("{{\"amount\":\"{DIGITS}\"}}");
        let mut ctx = nvs_runtime::Ctx::buffered();
        #[expect(
            unsafe_code,
            reason = "`codec_class` leaks its table, so both addresses outlive \
                      every object decoded against them"
        )]
        let decoded = unsafe {
            decode_as(
                &mut ctx,
                class,
                Some(shape),
                &document,
                DEFAULT_MAX_DEPTH_U32,
                false,
                r"Core\Json::decodeAs",
            )
        };
        let Ok(value) = decoded else {
            panic!("a `decimal` field decodes from the string spelling it is written as");
        };

        let ptr = value
            .obj_ptr()
            .expect("a decoded shape is always an object");
        #[expect(
            unsafe_code,
            reason = "the value owns a reference to a live allocation, and the \
                      rebuilt handle is never dropped, so nothing is released twice"
        )]
        let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
        let held = object
            .field(0)
            .as_decimal()
            .expect("a `decimal` field holds a decimal");
        assert_eq!(held.to_string(), DIGITS);

        // Both halves, one spelling: what the encoder writes is the document
        // the decoder was handed, down to the scale.
        assert_eq!(
            encoded(value).expect("a decoded shape re-encodes"),
            document
        );

        #[expect(unsafe_code, reason = "this frame holds the only reference")]
        unsafe {
            value.release();
        }
    }

    /// `rule:core-classes/derive-field-list`'s inline shape reached as a
    /// *field*, which is the one wire type needing **two** resolved pointers: a
    /// shape class is keyed on its field names alone, so the descriptor says
    /// which object to build and says nothing about what goes in it, and the
    /// contract beside it is the half that does.
    ///
    /// Asserted through the nested object's own slot rather than through the
    /// re-encoding alone: an encode walks the value and would print the same
    /// document for a field the decode had filled by luck.
    #[test]
    fn decode_as_fills_an_inline_shape_field() {
        let (class, shape) = nested_shape_class();
        let document = "{\"meta\":{\"n\":7}}";
        let mut ctx = nvs_runtime::Ctx::buffered();
        #[expect(
            unsafe_code,
            reason = "`nested_shape_class` leaks its table, so every address in it \
                      outlives the object decoded against them"
        )]
        let decoded = unsafe {
            decode_as(
                &mut ctx,
                class,
                Some(shape),
                document,
                DEFAULT_MAX_DEPTH_U32,
                false,
                r"Core\Json::decodeAs",
            )
        };
        let Ok(value) = decoded else {
            panic!("a shape field decodes against the contract beside its class label");
        };

        let ptr = value
            .obj_ptr()
            .expect("a decoded shape is always an object");
        #[expect(
            unsafe_code,
            reason = "the value owns a reference to a live allocation, and the \
                      rebuilt handle is never dropped, so nothing is released twice"
        )]
        let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
        let inner = object
            .field(0)
            .obj_ptr()
            .expect("a shape field holds the object its own contract built");
        #[expect(
            unsafe_code,
            reason = "the outer object owns this reference for as long as it lives, \
                      and this handle is never dropped either"
        )]
        let inner = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(inner) });
        assert_eq!(inner.field(0).as_int(), Some(7));

        // Both halves, one spelling: what the encoder writes is the document the
        // decoder was handed, nesting included.
        assert_eq!(
            encoded(value).expect("a decoded shape re-encodes"),
            document
        );

        #[expect(unsafe_code, reason = "this frame holds the only reference")]
        unsafe {
            value.release();
        }
    }

    /// An `Instant` crosses as the RFC 3339 text `$i->toIso()` renders, so the
    /// two halves are one spelling: the document names a point on the timeline,
    /// the field holds an `Instant` of exactly that point, and the encoder
    /// writes the document back. The object its two slots would spell is this
    /// runtime's storage and never the contract — the module's own
    /// *A value type crosses as text* section owns why.
    #[test]
    fn decode_as_fills_an_instant_field_from_an_rfc_3339_string() {
        const AT: &str = "2024-03-01T12:00:00Z";

        let (class, shape) = codec_class("Event", "at", CodecTy::Instant);
        let document = format!("{{\"at\":\"{AT}\"}}");
        let mut ctx = nvs_runtime::Ctx::buffered();
        #[expect(
            unsafe_code,
            reason = "`codec_class` leaks its table, so both addresses outlive \
                      every object decoded against them"
        )]
        let decoded = unsafe {
            decode_as(
                &mut ctx,
                class,
                Some(shape),
                &document,
                DEFAULT_MAX_DEPTH_U32,
                false,
                r"Core\Json::decodeAs",
            )
        };
        let Ok(value) = decoded else {
            panic!("an `Instant` field decodes from the RFC 3339 spelling it is written as");
        };

        let ptr = value
            .obj_ptr()
            .expect("a decoded shape is always an object");
        #[expect(
            unsafe_code,
            reason = "the value owns a reference to a live allocation, and the \
                      rebuilt handle is never dropped, so nothing is released twice"
        )]
        let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
        let held = object.field(0);
        assert!(
            crate::instance::is_instance(held, &crate::time::INSTANT),
            "the field holds an `Instant` rather than the text it was read from"
        );
        assert_eq!(
            crate::time::instant_iso(held).expect("a decoded `Instant` renders"),
            AT
        );

        // The other half, one spelling: what the encoder writes is the document
        // the decoder was handed.
        assert_eq!(
            encoded(value).expect("a decoded shape re-encodes"),
            document
        );

        #[expect(unsafe_code, reason = "this frame holds the only reference")]
        unsafe {
            value.release();
        }
    }

    /// A document that spells the timestamp any other way is a **bad
    /// document** and not an engine gap: a JSON number is what an epoch second
    /// would arrive as, and a civil date is text that is not RFC 3339, so both
    /// come back as the reported `ParseError` the field list accumulates rather
    /// than as the fault an undecodable wire type ends the walk with.
    #[test]
    fn an_instant_field_refuses_every_spelling_but_the_one_it_writes() {
        let (class, shape) = codec_class("Event", "at", CodecTy::Instant);
        let mut ctx = nvs_runtime::Ctx::buffered();
        let mut refused = |text: &str| {
            #[expect(
                unsafe_code,
                reason = "`codec_class` leaks its table, so both addresses outlive \
                          every object decoded against them"
            )]
            let outcome = unsafe {
                decode_as(
                    &mut ctx,
                    class,
                    Some(shape),
                    text,
                    DEFAULT_MAX_DEPTH_U32,
                    false,
                    r"Core\Json::decodeAs",
                )
            };
            match outcome {
                Ok(_) => panic!("`{text}` is not an `Instant`'s spelling"),
                Err(why) => format!("{why:?}"),
            }
        };

        for document in [r#"{"at":1709294400}"#, r#"{"at":"2024-03-01"}"#] {
            let why = refused(document);
            assert!(why.contains("Parse"), "{why}");
            assert!(why.contains("did not match"), "{why}");
        }
    }

    /// An instance of `class` with every slot null, as the value holding the
    /// only reference to it.
    fn instance(class: *const ClassDesc) -> Value {
        #[expect(
            unsafe_code,
            reason = "`holder_class` leaks the table, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { NvsObj::new(class) };
        Value::object(object)
    }

    /// Writes `held` into `object`'s `slot`, taking over its reference.
    ///
    /// A cycle is built value-first and property-second, so the handle has to
    /// be rebuilt from the value's own address. It is never dropped, exactly as
    /// the encoder's is: the value still owns that reference.
    fn set_property(object: Value, slot: usize, held: Value) {
        let ptr = object
            .obj_ptr()
            .expect("a `Tag::Object` value is always an object");
        #[expect(
            unsafe_code,
            reason = "the value owns a reference to a live allocation, and the \
                      rebuilt handle is never dropped, so nothing is released twice"
        )]
        let object = std::mem::ManuallyDrop::new(unsafe { NvsObj::from_raw(ptr) });
        object.set_field(slot, held);
    }

    /// `rule:classes/an-encoder-ends-a-cycle-by-identity`: the walk turns back
    /// at the first value it is already inside, and the refusal names the
    /// property that closed it rather than the depth it would have reached.
    #[test]
    fn an_object_that_holds_itself_is_refused_at_the_property_that_closes_the_cycle() {
        let holder = instance(holder_class("Holder", &["self"]));
        #[expect(unsafe_code, reason = "the property below owns a second reference")]
        unsafe {
            holder.retain();
        }
        set_property(holder, 0, holder);

        let why = encoded(holder).expect_err("a value that holds itself has no JSON encoding");
        assert!(why.contains("`self`"), "{why}");
        assert!(why.contains("already inside"), "{why}");

        // Nothing can drop a cycle, so the property holding the object is
        // cleared before the last reference to it goes.
        set_property(holder, 0, Value::null());
        #[expect(unsafe_code, reason = "this frame now holds the only reference")]
        unsafe {
            holder.release();
        }
    }

    /// The message is the chain of keys from the document to the value that
    /// closed the cycle — the thing a program can go and look at — and never
    /// the level count a depth cap would have reported.
    #[test]
    fn the_refusal_names_the_property_chain_rather_than_a_count_of_levels() {
        let outer = instance(holder_class("Outer", &["child"]));
        let inner = instance(holder_class("Inner", &["back"]));
        #[expect(unsafe_code, reason = "`inner`'s property owns a second reference")]
        unsafe {
            outer.retain();
        }
        set_property(inner, 0, outer);
        set_property(outer, 0, inner);

        let why = encoded(outer).expect_err("a two-object cycle has no JSON encoding");
        assert!(why.contains("`child.back`"), "{why}");
        assert!(!why.contains("levels"), "{why}");

        set_property(inner, 0, Value::null());
        #[expect(unsafe_code, reason = "this frame now holds the only reference")]
        unsafe {
            outer.release();
        }
    }

    /// An array is copied where an object is referenced, so this cycle needs
    /// both: the allocation copy-on-write shares is reachable from an object
    /// inside it, and the walk ends at the array arm's own guard.
    #[test]
    fn a_cycle_that_closes_through_an_array_is_refused_at_the_same_place() {
        let holder = instance(holder_class("Row", &["items"]));
        let mut list = NvsArray::new();
        // The array takes over the reference `instance` handed back.
        list.set(NvsStr::new(b"0"), holder);
        let document = Value::array(list);
        #[expect(unsafe_code, reason = "the property below owns a second reference")]
        unsafe {
            document.retain();
        }
        set_property(holder, 0, document);

        let why = encoded(document).expect_err("an array inside itself has no JSON encoding");
        assert!(why.contains("`0.items`"), "{why}");
        assert!(why.contains("already inside"), "{why}");

        set_property(holder, 0, Value::null());
        #[expect(unsafe_code, reason = "this frame now holds the only reference")]
        unsafe {
            document.release();
        }
    }

    /// The set is the ancestor chain and never everything seen, so one object
    /// two properties hold is shared rather than cyclic — and JSON, which has
    /// no way to express sharing, writes it twice.
    #[test]
    fn one_object_held_by_two_properties_is_written_twice_rather_than_refused() {
        let pair = instance(holder_class("Pair", &["left", "right"]));
        let leaf = instance(holder_class("Leaf", &["n"]));
        set_property(leaf, 0, Value::int(7));
        #[expect(unsafe_code, reason = "the second property owns a second reference")]
        unsafe {
            leaf.retain();
        }
        set_property(pair, 0, leaf);
        set_property(pair, 1, leaf);

        assert_eq!(
            encoded(pair).expect("a shared value is not a cycle"),
            r#"{"left":{"n":7},"right":{"n":7}}"#
        );

        #[expect(
            unsafe_code,
            reason = "this frame holds the only reference to the pair"
        )]
        unsafe {
            pair.release();
        }
    }

    /// The depth cap still bounds what the ancestor chain does not: a document
    /// that is acyclic and merely deeper than any encoder should walk is
    /// refused as nesting, and never reported as a cycle.
    ///
    /// On a thread that sizes its own stack, because what the ceiling bounds is
    /// *nesting* while the frames the walk spends are the serializer's and the
    /// profile's — this module's gap 7 owns that difference, and what this test
    /// pins is the message rather than a frame budget. Every value is built and
    /// released inside that thread: a refcount is a per-thread fact.
    #[test]
    fn a_document_deeper_than_the_ceiling_still_reports_depth_and_not_a_cycle() {
        let walk = std::thread::Builder::new()
            .stack_size(16 << 20)
            .spawn(|| {
                let depth = usize::try_from(DEPTH_CEILING).expect("the ceiling fits a usize");
                let mut value = Value::int(1);
                for _ in 0..depth {
                    let mut level = NvsArray::new();
                    level.set(NvsStr::new(b"0"), value);
                    value = Value::array(level);
                }
                let why =
                    encoded(value).expect_err("a document past the ceiling has no JSON encoding");
                #[expect(unsafe_code, reason = "this frame holds the only reference")]
                unsafe {
                    value.release();
                }
                why
            })
            .expect("a test thread is spawnable");

        let why = walk.join().expect("the walk refuses rather than panicking");
        assert!(
            why.contains(&format!("nested past {DEPTH_CEILING} levels")),
            "{why}"
        );
        assert!(!why.contains("already inside"), "{why}");
    }

    /// The audit's finding, asserted rather than described: one cyclic object
    /// graph is handed to every `Core` member whose walk could meet an object,
    /// and each of them ends where it stands.
    ///
    /// [`nvs_core_json_encode`] is the only one that descends into an object,
    /// so it is the only one a cycle can close inside — and it answers with the
    /// property that closed it. `Core\Csv::format`, `Core\Uri::buildQuery` and
    /// `Core\Encoding::toHex` are handed that same object as a **leaf** and
    /// refuse it by tag where it sits, which is what each of those modules' own
    /// docs records about its walk. `Core\Serialize` is not asked here: it
    /// reaches the same property through `rule:classes/graph-copy`.
    #[test]
    fn every_encoder_that_reaches_an_object_graph_refuses_a_cycle_rather_than_recursing() {
        /// One member the question is put to: the argument list it takes the
        /// object in, and the words its own refusal is written with.
        struct Asked {
            member: &'static str,
            function: nvs_runtime::NvsFn,
            refusal: &'static str,
            args: fn(Value) -> Vec<Value>,
        }

        let holder = instance(holder_class("Holder", &["self"]));
        #[expect(unsafe_code, reason = "the property below owns a second reference")]
        unsafe {
            holder.retain();
        }
        set_property(holder, 0, holder);

        let asked = [
            Asked {
                member: r"Core\Json::encode",
                function: nvs_core_json_encode,
                refusal: "`self`",
                args: |object| vec![object, Value::bool(false), Value::bool(false)],
            },
            Asked {
                member: r"Core\Csv::format",
                function: crate::csv::nvs_core_csv_format,
                refusal: "is not a `string`",
                args: |object| {
                    let mut record = NvsArray::new();
                    record.set(NvsStr::new(b"0"), object);
                    let mut rows = NvsArray::new();
                    rows.set(NvsStr::new(b"0"), Value::array(record));
                    vec![
                        Value::array(rows),
                        Value::str(NvsStr::new(b",")),
                        Value::str(NvsStr::new(b"\"")),
                        Value::null(),
                    ]
                },
            },
            Asked {
                member: r"Core\Uri::buildQuery",
                function: crate::uri::nvs_core_uri_build_query,
                refusal: "neither a scalar nor a nested array",
                args: |object| {
                    let mut parameters = NvsArray::new();
                    parameters.set(NvsStr::new(b"a"), object);
                    vec![Value::array(parameters)]
                },
            },
            Asked {
                member: r"Core\Encoding::toHex",
                function: crate::encoding::nvs_core_encoding_to_hex,
                refusal: "expected a `bytes`",
                args: |object| vec![object],
            },
        ];

        let mut ctx = nvs_runtime::Ctx::buffered();
        let mut answers: Vec<(&str, String)> = Vec::new();
        for asked in asked {
            #[expect(unsafe_code, reason = "the argument list below owns this reference")]
            unsafe {
                holder.retain();
            }
            let args = (asked.args)(holder);
            let refused = nvs_runtime::call(asked.function, &mut ctx, &args).is_err();
            let why = ctx
                .take_pending()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default();
            for argument in args {
                #[expect(
                    unsafe_code,
                    reason = "the argument list holds exactly the references its \
                              builder took, and the cycle keeps the object alive"
                )]
                unsafe {
                    argument.release();
                }
            }

            assert!(refused, "{} encoded a cyclic object", asked.member);
            assert!(why.contains(asked.member), "{why}");
            assert!(why.contains(asked.refusal), "{why}");
            answers.push((asked.member, why));
        }

        // The agreement, counted rather than read off a line: only a walk that
        // is *inside* an object can meet it again, so exactly one of these
        // answers by identity and the rest never entered it at all.
        let by_identity: Vec<&str> = answers
            .iter()
            .filter(|(_, why)| why.contains("already inside"))
            .map(|(member, _)| *member)
            .collect();
        assert_eq!(by_identity, [r"Core\Json::encode"], "{answers:?}");

        set_property(holder, 0, Value::null());
        #[expect(unsafe_code, reason = "this frame now holds the only reference")]
        unsafe {
            holder.release();
        }
    }
}
