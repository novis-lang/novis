//! `Core\Uuid` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 11's second table, which replaces `uniqid`, `com_create_guid` and every
//! userland UUID library with one type.
//!
//! Every one of that table's `Uuid` rows is here. A `Core\Uuid` is a **value
//! with a type**, not a 36-character string a program passes around and
//! re-validates
//! at every boundary: that is what makes `rule:core-classes/db-statement-members`'s native `UUID` column
//! binding and `rule:routing/routes-are-compiled-not-registered`'s `Core\Uuid` route segment able to state what they
//! take.
//!
//! # Reading text: the canonical form, and nothing else
//!
//! `parse` accepts RFC 9562 § 4's `8-4-4-4-12` hyphenated form in either case,
//! and refuses the three shapes the `uuid` crate would otherwise also take: the
//! unhyphenated 32 hex digits, `{…}` braces, and a `urn:uuid:` prefix. One
//! length check does it, so nothing is re-implemented to be strict — and that
//! check is [`nvs_runtime::uuid::read`], because `rule:routing/a-capture-narrows-to-a-closed-set`'s `Core\Uuid`
//! route capture is accepted or refused one crate *below* this one, and a
//! segment a route admits had better not be read by a second grammar. The rule
//! and its reasons are still this module's; the sixteen bytes are down there.
//!
//! The reason is that each of those is a **second spelling of one value**, and
//! a program that keys a cache, a rate limiter or an audit log on the text
//! while authorizing on the UUID then has two strings that are one identity.
//! `rule:types/conversion`'s `string → int` row takes exactly this line — the whole string
//! must be the literal, with no leading-garbage rule — and this is the same
//! decision at the same kind of boundary. A program holding one of the other
//! three forms writes the trim it means.
//!
//! **Any 128 bits in that shape parse**, including RFC 9562 § 5.9's nil UUID
//! and § 5.10's max. The version and variant nibbles are a statement about how
//! a UUID was *generated*, not a checksum, so refusing an unrecognised version
//! would make `parse` reject identifiers a future RFC defines and ones this
//! very module will emit if § 11 ever gains a `v8`.
//!
//! # The layout: two `uint` slots, not one `string`
//!
//! A UUID is 128 bits, and [`crate::instance`]'s decision is that a `Core`
//! instance holds nothing Novis cannot already hold — so the choice was two
//! integer slots (the big-endian halves) or one `string` slot holding the
//! canonical text.
//!
//! Two integers, because **the text is the derived form**. A UUID is generated
//! far more often than it is printed: it goes into a database column that
//! stores 16 bytes, is compared against another one, or is carried through a
//! request and dropped. Holding the text would allocate 36 bytes on every
//! construction to pay for a rendering that most of them never ask for, and it
//! would make `parse` the only member that ever checks the shape — the slots
//! could then hold text that is not a UUID at all. Integers cannot be
//! malformed, so every member below reads its receiver without a validity
//! question.
//!
//! What it costs is one allocation per `toString()` rather than one per
//! construction, which is memory footprint spent to buy latency at
//! the commoner of the two sites. A single `bytes` slot is the natural third
//! answer and waits on nothing — `nvs_runtime::Tag::Bytes` is a live tag — but
//! it is refcounted where a `uint` slot is immediate, so the two halves are
//! held, copied and compared without touching a heap allocation at all.
//!
//! # The dependency, and why `uuid`
//!
//! `rule:packaging/a-c-dependency-answers-two-questions`'s first
//! question — does attacker-controlled data reach it — is **yes** for this
//! class, at `Uuid::parse`, which is exactly why the parser should not
//! be hand-written here. `uuid` is the crate every Rust program already uses
//! for this, so its fixed-length hex reader has more readers than anything
//! written beside it would; it is pure Rust with no build script and no C, and
//! it is `Apache-2.0 OR MIT`. It also adds **no crate to this tree**: `uuid`
//! 1.x is already in `Cargo.lock` under `debugid`, which `wasmtime` pulls.
//!
//! **`default-features = false, features = ["std"]`, and deliberately not
//! `v4`/`v7`.** Those two features exist to give `uuid` its own generator, and
//! taking them would put a second CSPRNG (`getrandom`) in `Core` beside the one
//! spec § 11's first table already binds. So this module draws its bits from
//! [`crate::random`]'s generator and hands them to `uuid::Builder`, which lays
//! out the version and variant nibbles: one entropy source, one layout
//! implementation, and neither hand-written. The timestamp half of `v7` comes
//! from `jiff`, which spec § 4 already binds, for the same reason.
//!
//! # What it spends
//!
//! One [`crate::instance`] object per UUID — an `NvsObj` header plus two
//! 16-byte slots — charged to the request that produced it and released with
//! it, plus one `NvsStr` per *rendering*: the 36 characters `toString` writes
//! or the sixteen octets `toBytes` does, one allocation each call and nothing
//! cached between them (`rule:programs/memory-priority`). Nothing is retained
//! between calls, and no table grows with traffic.
//!
//! # Two UUIDs holding the same bits are not `==`
//!
//! `rule:expressions/equality-semantics`'s non-scalar row makes `==` object
//! identity, so two instances parsed from one string are two values, and
//! comparing `toString()` is the spelling that works today. That is every
//! `Core`-owned instance's answer rather than this class's, and it is why no
//! member here answers `bool` about another UUID.
//!
//! # `v7` orders across milliseconds, not inside one
//!
//! RFC 9562's optional monotonic counter is not built, so two UUIDs drawn
//! inside the same millisecond order randomly against each other. Across
//! milliseconds the ordering is exact, which is the property a database key is
//! chosen for; a program that needs a total order inside one millisecond needs
//! a sequence, not a UUID.
//!
//! # The octets are the second door, and the text is not on the way
//!
//! `fromBytes` and `toBytes` carry the sixteen octets themselves, most
//! significant first — the order the canonical text spells them in — so an
//! `rule:core-classes/db-one-api` driver binding a native `UUID` column, a
//! binary protocol and a hash input read and write a UUID with no
//! 36-character detour in either direction.
//!
//! `fromBytes` has one thing to refuse and it is a **length**: every 128-bit
//! pattern is a UUID, including the nil and the max, which is the rule `parse`
//! states above arriving here without a grammar in front of it. So the pair is
//! a round trip by construction rather than by agreement — `toBytes` writes
//! the slots and `fromBytes` reads them back, and neither consults the
//! rendering.

use rand::Rng;
use uuid::{Builder, Uuid};

use nvs_runtime::{Fault, NvsStr, Value};

use crate::ext_record::Part;

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Uuid`'s fully-qualified name, written once so the registry row and
/// every [`CoreTy::Instance`] naming it cannot drift apart.
pub const NAME: &str = r"Core\Uuid";

/// `Core\Uuid`'s registry rows — § 11's second table's static members, and
/// beside them the two members that render a receiver, one into text and one
/// into octets. `isValid` was a static row until `rule:expressions/try-parse`
/// deleted it: `tryParse` is that question asked through `parse` itself, so the
/// two were one predicate and R17 keeps one.
pub const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "v4",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uuid_v4",
            doc: Some(&V4_DOC),
        },
        CoreMethod {
            name: "v7",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uuid_v7",
            doc: Some(&V7_DOC),
        },
        CoreMethod {
            name: "parse",
            names: &["s"],
            // `Qual::Neutral` rather than unclassified, and it is
            // `rule:expressions/try-parse`'s pair being readable as `Parses`
            // that turns on it: the text a program parses arrived from outside
            // the process, so the parameter admits a `tainted` one, and the
            // 128 bits that come back carry none of it — a canonical UUID is
            // what the parse checked, not what it was handed.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uuid_parse",
            doc: Some(&PARSE_DOC),
        },
        CoreMethod {
            name: "tryParse",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(NAME)),
            symbol: "nvs_core_uuid_try_parse",
            doc: Some(&TRY_PARSE_DOC),
        },
        CoreMethod {
            name: "fromBytes",
            names: &["b"],
            // `Qual::Neutral` for `parse`'s reason one representation over:
            // the octets arrived from outside the process, so the parameter
            // admits a `tainted` buffer, and the 128 bits that come back carry
            // none of it — a UUID is a closed shape with nothing an injection
            // could ride into.
            params: &[CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uuid_from_bytes",
            doc: Some(&FROM_BYTES_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "toString",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_uuid_to_string",
            doc: Some(&TO_STRING_DOC),
        },
        CoreMethod {
            name: "toBytes",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_uuid_to_bytes",
            doc: Some(&TO_BYTES_DOC),
        },
    ],
    slots: &["high", "low"],
    constants: &[],
};

/// `Core\Uuid`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "A UUID, which is a 128-bit identifier. `v4` and `v7` create a new one. `parse` reads \
            one from its text, such as `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`, and `fromBytes` \
            reads one from its sixteen bytes. `toString` and `toBytes` give the text and the \
            bytes back.",
};

/// `Core\Uuid::v4`'s reference card — `rule:core-api/reference-card`.
const V4_DOC: MethodDoc = MethodDoc {
    short: "Creates a new random UUID, such as `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`. 122 of \
            its 128 bits come from a secure random number generator. The other six bits say \
            that it is a version 4 UUID.",
    params: &[],
    ret: "A new `Uuid`. Nobody can guess it, and it contains no date or time, so it is safe to \
          show to other people.",
    errors: &[],
};

/// `Core\Uuid::v7`'s reference card — `rule:core-api/reference-card`.
const V7_DOC: MethodDoc = MethodDoc {
    short: "Creates a new UUID that starts with the current time, such as \
            `0190d1e6-8e3c-7c2a-9a4b-1f2e3d4c5b6a`. The first 48 bits are the number of \
            milliseconds since 1970. 74 of the other bits are random. Use it as a database key, \
            because new keys sort after older ones.",
    params: &[],
    ret: "A new `Uuid`. It sorts after every UUID created in an earlier millisecond. Two UUIDs \
          created in the same millisecond sort in a random order. Anybody who has the UUID can \
          read the time it was created, so use `Core\\Uuid::v4` for an ID that other people \
          see. If the computer's clock is before 1970, the time part is zero.",
    errors: &[],
};

/// `Core\Uuid::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads the text `$s` as a UUID. The text must be 32 hexadecimal digits in groups of \
            8, 4, 4, 4 and 12, joined by hyphens, such as \
            `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`. Upper case and lower case letters are both \
            allowed.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to read. It must be exactly 36 characters long.",
        shape: &[],
    }],
    ret: "The `Uuid` that the text gives. The version digit is not checked, so the nil UUID \
          (all zeros) and the max UUID (all `f`) are both valid.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$s` is not in that form. The 32 digits without hyphens, a UUID in `{}` braces and \
               a UUID after `urn:uuid:` all throw this error too. The message quotes up to 48 \
               characters of `$s`.",
    }],
};

/// `Core\Uuid::tryParse`'s reference card — `rule:core-api/reference-card`.
const TRY_PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads the text `$s` as a UUID, like `Core\\Uuid::parse`. When the text is not a \
            UUID, it returns `null` and does not throw an error. Use it to check whether a text \
            is a UUID.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to read. It must be 32 hexadecimal digits in groups of 8, 4, 4, 4 and 12, \
               joined by hyphens. Upper case and lower case letters are both allowed.",
        shape: &[],
    }],
    ret: "The `Uuid` that the text gives, or `null` when the text is in any other form. The 32 \
          digits without hyphens, a UUID in `{}` braces and a UUID after `urn:uuid:` all give \
          `null`.",
    errors: &[],
};

/// `Core\Uuid::fromBytes`'s reference card — `rule:core-api/reference-card`.
const FROM_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Reads sixteen bytes as a UUID. Many databases store a UUID in a binary column in \
            this form, and binary protocols send it this way.",
    params: &[ParamDoc {
        name: "b",
        desc: "The sixteen bytes, in the same order as the digits of the UUID's text. \
               `$uuid->toBytes()` gives them in this order.",
        shape: &[],
    }],
    ret: "The `Uuid` that the bytes give. Any sixteen bytes are a valid UUID, so only the length \
          is checked.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$b` is not exactly sixteen bytes long. The message gives the length of `$b`.",
    }],
};

/// `$uuid->toString`'s reference card — `rule:core-api/reference-card`.
const TO_STRING_DOC: MethodDoc = MethodDoc {
    short: "Returns the text of the UUID, such as `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`. The \
            digits are in groups of 8, 4, 4, 4 and 12, joined by hyphens. `echo $uuid` writes \
            the same text.",
    params: &[],
    ret: "A string of 36 characters. The letters are always lower case, even when `parse` read \
          them in upper case.",
    errors: &[],
};

/// `$uuid->toBytes`'s reference card — `rule:core-api/reference-card`.
const TO_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns the sixteen bytes of the UUID. Use it to store a UUID in a binary database \
            column or to send it in a binary message. The text form of the same UUID is 36 \
            bytes long.",
    params: &[],
    ret: "Sixteen bytes, in the same order as the digits of the UUID's text. \
          `Core\\Uuid::fromBytes` reads them back as the same UUID.",
    errors: &[],
};

/// [`CLASS`]'s slots, by index — the big-endian halves of the 128 bits.
const HIGH_SLOT: usize = 0;
/// See [`HIGH_SLOT`].
const LOW_SLOT: usize = 1;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_uuid_v4" => (nvs_core_uuid_v4 as *const ()).cast(),
        "nvs_core_uuid_v7" => (nvs_core_uuid_v7 as *const ()).cast(),
        "nvs_core_uuid_parse" => (nvs_core_uuid_parse as *const ()).cast(),
        "nvs_core_uuid_try_parse" => (nvs_core_uuid_try_parse as *const ()).cast(),
        "nvs_core_uuid_from_bytes" => (nvs_core_uuid_from_bytes as *const ()).cast(),
        "nvs_core_uuid_to_string" => (nvs_core_uuid_to_string as *const ()).cast(),
        "nvs_core_uuid_to_bytes" => (nvs_core_uuid_to_bytes as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Reading and building — the two halves of this module's layout decision
// ============================================================================

/// A fresh `Core\Uuid` instance holding `value`.
fn built(value: Uuid) -> Value {
    let (high, low) = value.as_u64_pair();
    crate::instance::build(&CLASS, [Value::uint(high), Value::uint(low)])
}

/// A `Core\Uuid` holding those sixteen octets, in the order the canonical text
/// spells them, for a member **outside this module** holding the bytes.
///
/// `rule:core-classes/db-column-types`'s `UUID` column is the first: `nvs-db` reads the row and hands
/// the octets over, because the instance is this crate's to allocate. There is
/// nothing to refuse — every 128-bit pattern is a UUID, including the nil and
/// the max, and this module's own docs say which of them `parse` accepts.
pub(crate) fn of_octets(octets: [u8; 16]) -> Value {
    built(Uuid::from_bytes(octets))
}

/// The `uuid` record's fields of the `Core\Uuid` `value`, for `crate::ext_record`.
///
/// # Errors
///
/// [`uuid_of`]'s.
pub(crate) fn ext_parts(value: Value) -> Result<Vec<Part>, Fault> {
    let (high, low) = uuid_of(&[value], 0, "ext_parts")?.as_u64_pair();
    Ok(vec![Part::Uint(high), Part::Uint(low)])
}

/// A `Core\Uuid` from a guest's `uuid` record. Every pair of halves is one.
pub(crate) fn ext_built(parts: &[Part]) -> Option<Value> {
    match parts {
        [Part::Uint(high), Part::Uint(low)] => Some(built(Uuid::from_u64_pair(*high, *low))),
        _ => None,
    }
}

/// The buffer [`canonical`] renders into, zeroed. A caller copies it, so the
/// width of the canonical form is written here and nowhere else.
pub(crate) const TEXT: Text = [0; uuid::fmt::Hyphenated::LENGTH];

/// See [`TEXT`].
pub(crate) type Text = [u8; uuid::fmt::Hyphenated::LENGTH];

/// `octets` in the canonical rendering — hyphenated and lower case, § 9's own
/// spelling and [`nvs_core_uuid_to_string`]'s answer.
///
/// A function rather than a second `encode_lower` at each caller, because
/// `nvs_stdlib::router`'s `signedRoute` derives the text of a `Core\Uuid`
/// capture in order to compare it against the text that was signed, and
/// `rule:core-api/signing-is-over-a-payload` is precisely about a canonical
/// form that only the signature depends on: a second spelling here would be one
/// no other test constrains.
pub(crate) fn canonical(octets: [u8; 16], into: &mut Text) -> &str {
    Uuid::from_bytes(octets).hyphenated().encode_lower(into)
}

/// [`of_octets`]'s twin for a `Core\Uuid` that arrives as text — `rule:core-classes/db-column-types`'s
/// SQLite half, where a `uuid` column holds the canonical rendering because
/// SQLite has no type that holds sixteen octets as anything but a `BLOB`.
///
/// `None` for text that is not one, which is § 9's "throws on a value that does
/// not parse" asked here; the caller holds the column's name and this seam does
/// not. It is [`read`] and therefore [`nvs_core_uuid_parse`] exactly, so a cell
/// a database hands back and a string a program parses are one grammar.
pub(crate) fn of_text(text: &str) -> Option<Value> {
    read(text).map(built)
}

/// The `Uuid` `text` spells, or `None` if it is not the canonical form.
///
/// **The grammar itself is [`nvs_runtime::uuid::read`]**, one crate down, and
/// this is that reading with the type put back on it. It lives there because
/// `rule:routing/a-capture-narrows-to-a-closed-set` lets a route capture declare `Core\Uuid` and a capture is
/// accepted or refused in `nvs_runtime::routes`, which cannot depend on this
/// crate — so a segment a route admits and a string a program parses are one
/// grammar with one home rather than two readers that agree today. Everything
/// this module's docs say about *why* the reading is strict is still this
/// module's; what moved is where the length check and the call sit.
fn read(text: &str) -> Option<Uuid> {
    nvs_runtime::uuid::read(text).map(Uuid::from_bytes)
}

/// The `string` in argument slot 0, for the two members that read text.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for [`uuid_of`]'s reason — and only
/// that one: a `string` is guaranteed-valid UTF-8 (`rule:types/bytes`), and the tag
/// [`Value::as_text`] checks *is* that guarantee, so nothing here re-derives it.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uuid::{member} expected a `string`, got tag {}",
            args[0].tag_byte()
        ))
    })
}

/// The `bytes` in argument slot 0, for the member that reads octets.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, [`text_of`]'s twin one tag over and
/// for its reason: the checker placed the argument and compiled code wrote the
/// tag, so anything else in that slot is a runtime-contract violation rather
/// than something a program can cause. **The length is not checked here** —
/// that is the one thing about this argument a program *can* get wrong, so it
/// is a throw at the member and not a fatal at the reader.
fn bytes_of<'a>(args: &'a [Value], member: &str) -> Result<&'a [u8], Fault> {
    args[0].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uuid::{member} expected a `bytes`, got tag {}",
            args[0].tag_byte()
        ))
    })
}

/// `text` as it may be quoted back inside a throw message: the first
/// [`SHOWN_CHARS`] characters, with an ellipsis where anything was dropped.
///
/// The operand of a failed `parse` is by definition text that arrived from
/// somewhere, so it is the one value in this module a caller controls the size
/// of. A message is written to a log, so quoting it whole would let a request
/// choose how many bytes that log gains
/// (`rule:security/sink-predicate`
/// is the wider rule); a bounded quote is still the only thing that makes the
/// diagnostic actionable.
fn shown(text: &str) -> String {
    let mut out: String = text.chars().take(SHOWN_CHARS).collect();
    if text.chars().nth(SHOWN_CHARS).is_some() {
        out.push('…');
    }
    out
}

/// How much of a rejected operand [`shown`] quotes — comfortably past the 36 a
/// canonical UUID takes, so the common near-miss is quoted whole.
const SHOWN_CHARS: usize = 48;

/// The `Uuid` in argument slot `at` — slot 0 for a receiver, any other slot
/// for a `Uuid` *parameter*.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member if the slot does not hold this class's
/// shape. Compiled code wrote the tag and `nvs_types` already checked the
/// declared type, so that is a runtime-contract violation rather than anything
/// a program can cause — the same treatment `crate::time` gives an `Instant`.
pub(crate) fn uuid_of(args: &[Value], at: usize, member: &str) -> Result<Uuid, Fault> {
    let object = crate::instance::receiver(args[at], &CLASS, member)?;
    let half = |index: usize, which: &str| {
        crate::instance::slot(object, index)
            .as_uint()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Uuid::{member} found a non-`uint` `{which}` slot"
                ))
            })
    };
    Ok(Uuid::from_u64_pair(
        half(HIGH_SLOT, "high")?,
        half(LOW_SLOT, "low")?,
    ))
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Uuid::v4(): Uuid` — 122 random bits, replacing PHP's `uniqid`,
    /// `com_create_guid` and the userland libraries written around them.
    ///
    /// **`uniqid` is not what this replaces the behaviour of.** That function
    /// answers with the clock in hex and is guessable by anyone who knows
    /// roughly when it ran, which is why every PHP program that needed an
    /// unguessable identifier had to reach outside the standard library. Every
    /// bit here that is not the version or variant marker comes from
    /// [`crate::random`]'s CSPRNG.
    ///
    /// Never fails: there is no argument to reject, and the generator is
    /// infallible once seeded.
    fn nvs_core_uuid_v4(ctx, args: [0]) {
        let _ = args;
        let mut bytes = [0_u8; 16];
        crate::random::draw(ctx, |rng| rng.fill_bytes(&mut bytes));
        Ok(built(Builder::from_random_bytes(bytes).into_uuid()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uuid::v7(): Uuid` — a 48-bit millisecond timestamp then 74
    /// random bits, replacing nothing in PHP because PHP has nothing like it.
    ///
    /// **Time-ordered, which is the whole reason to pick it over `v4` for a
    /// database key.** A `v4` primary key arrives uniformly across the index
    /// and dirties a new page on almost every insert; a `v7` appends, because
    /// the most significant bits are the clock. What it gives up is that the
    /// creation time of a row is readable from its key by anyone holding it —
    /// so a `v7` is for a key, and a `v4` is for anything a stranger sees.
    ///
    /// The clock is `jiff`'s, which is spec § 4's, so this member and
    /// `Core\Time::now()` cannot disagree about what time it is. A clock
    /// before 1970 has no representation in this layout at all and is pinned
    /// to the epoch rather than throwing: the answer is still a unique
    /// identifier, and a member whose whole job is "hand me an id" is the
    /// wrong place to surface a misconfigured host clock.
    ///
    /// Ordering inside one millisecond is random, and this module's docs say
    /// what that costs a program that needs a total order.
    fn nvs_core_uuid_v7(ctx, args: [0]) {
        let _ = args;
        // `rule:testing/determinism-declared-on-the-test` names this member beside `Core\Time::now` and
        // `Core\Random::int` as one a test makes deterministic, and it is the
        // only one that needs *both* halves: its timestamp comes from the fixed
        // clock through `crate::time::wall_clock`, and its 74 random bits from
        // the seeded generator through `crate::random::draw`. Outside a test
        // both answer exactly what they answered before.
        let millis = crate::time::wall_clock(ctx)
            .and_then(|at| u64::try_from(at.as_millisecond()).ok())
            .unwrap_or(0);
        let mut bytes = [0_u8; 10];
        crate::random::draw(ctx, |rng| rng.fill_bytes(&mut bytes));
        Ok(built(
            Builder::from_unix_timestamp_millis(millis, &bytes).into_uuid(),
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uuid::parse(string $s): Uuid` — replacing the hand-written
    /// validation every PHP program that carried UUIDs as strings had to
    /// write, and then had to remember to call.
    ///
    /// **The canonical hyphenated form in either case, and nothing else** —
    /// this module's docs own why the three other spellings `uuid` would take
    /// are refused, and why any 128 bits inside that shape are accepted.
    ///
    /// **Throws on anything else** (`rule:core-api/shape-rules` R4), which is what makes the
    /// return type `Uuid` rather than `?Uuid`: a caller asking to *parse* has
    /// asserted that the text is one, and the non-throwing question is
    /// [`nvs_core_uuid_try_parse`] beside it. The message quotes the text,
    /// bounded, so a log line cannot be flooded through it.
    fn nvs_core_uuid_parse(_ctx, args: [1]) {
        let text = text_of(args, "parse")?;

        read(text).map(built).ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Uuid::parse(): \"{}\" is not a UUID — the canonical form is eight \
                 hexadecimal digits, three groups of four and then twelve, hyphen-separated, as \
                 in \"00000000-0000-4000-8000-000000000000\"",
                shown(text)
            ))
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uuid::tryParse(string $s): ?Uuid` —
    /// `rule:expressions/try-parse`: [`nvs_core_uuid_parse`] exactly, with `null` where it throws.
    ///
    /// This replaces the `isValid` that used to sit here, which was already
    /// [`read`] asked without the throw — so there is still one definition of
    /// "is a UUID", now with one spelling instead of two. Both go through
    /// [`read`], which is condition 2 of § 3a and the only one a registry test
    /// cannot check.
    ///
    /// `$text as ?Core\Uuid` does **not** compile: § 3 withdrew the two-class
    /// parse roster that spelling belonged to, and `as` targets no class at
    /// all now. [`crate::uri`]'s module doc argues that withdrawal in full.
    fn nvs_core_uuid_try_parse(_ctx, args: [1]) {
        let text = text_of(args, "tryParse")?;

        Ok(read(text).map_or_else(Value::null, built))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uuid::fromBytes(bytes $b): Uuid` — the octet door this module's
    /// docs open, replacing the `fromBytes`/`getBytes` pair every userland
    /// UUID library carries and the `bin2hex`-then-validate idiom PHP programs
    /// wrote instead.
    ///
    /// **A length is the whole of what this refuses** (`rule:core-api/shape-rules` R4), and
    /// the message names the length it was handed, because that is the fact
    /// the caller does not already have: a buffer of the wrong width came from
    /// a column, a frame or a slice that is off by something, and the count is
    /// what says which. There is nothing else to check — every 128-bit pattern
    /// is a UUID, which is [`nvs_core_uuid_parse`]'s own rule reaching here
    /// without the grammar in front of it.
    ///
    /// The octets are most significant first, the order the canonical text
    /// spells them and the order [`of_octets`] already hands a database row
    /// over in, so the two seams onto the same sixteen bytes agree by
    /// construction rather than by a test that compares them.
    fn nvs_core_uuid_from_bytes(_ctx, args: [1]) {
        let octets = bytes_of(args, "fromBytes")?;

        let sixteen: [u8; 16] = octets.try_into().map_err(|_| {
            Fault::thrown(format!(
                "Core\\Uuid::fromBytes(): a UUID is sixteen bytes, and this buffer is {}",
                octets.len()
            ))
        })?;
        Ok(of_octets(sixteen))
    }
}

nvs_runtime::nvs_helper! {
    /// `$uuid->toString(): string` — the canonical
    /// `8-4-4-4-12` lower-case hex form, which is what RFC 9562 § 4 writes and
    /// what every database, log line and HTTP header expects.
    ///
    /// **Lower case, always.** The RFC's own grammar accepts either case on
    /// input and emits lower, and two renderings of one UUID that differ only
    /// in case would compare unequal as strings — and comparing this text is
    /// the comparison a program writes, `==` on two instances being object
    /// identity as this module's docs say.
    ///
    /// Named `toString` rather than `format` or `toText` so that it is already
    /// the member `Stringable` declares
    /// (`rule:classes/stringable`), which is what makes `echo $uuid` render: that name is the whole
    /// of what says a `Core`-owned class is stringifiable, read by
    /// `nvs_types::expr::operators::require_stringable` where the operand's
    /// type names this class and by [`crate::instance`]'s descriptor renderer
    /// where it names none.
    fn nvs_core_uuid_to_string(_ctx, args: [1]) {
        let value = uuid_of(args, 0, "toString")?;
        let mut buffer = TEXT;
        let text = canonical(value.into_bytes(), &mut buffer);
        Ok(Value::str(NvsStr::new(text.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `$uuid->toBytes(): bytes` — the sixteen octets themselves, most
    /// significant first, which is what a native `UUID` column, a binary
    /// frame and a hash input take.
    ///
    /// [`nvs_core_uuid_to_string`]'s twin, and the same shape of cost: the
    /// slots hold the value and a rendering allocates, so this is sixteen
    /// bytes where that one is thirty-six. A program that has a UUID and wants
    /// its octets had to render the text and unhex it before this existed,
    /// which is two allocations and a grammar to get back the bits it was
    /// already holding.
    ///
    /// Never fails: the receiver is two `uint` slots and every 128-bit pattern
    /// is a UUID, so there is nothing here that can refuse.
    fn nvs_core_uuid_to_bytes(_ctx, args: [1]) {
        let value = uuid_of(args, 0, "toBytes")?;
        Ok(Value::bytes(NvsStr::new(&value.into_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, OutputSink, Value, call};

    use super::{CLASS, HIGH_SLOT, LOW_SLOT};

    /// Runs one member through the `rule:errors/propagation` boundary compiled code reaches it
    /// at — [`crate::random`]'s own test helper, for its reasons.
    fn run(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        args: &[Value],
    ) -> Result<Value, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        call(member, &mut ctx, args)
    }

    /// One built UUID, rendered, releasing both references this test owns.
    fn rendered(value: Value) -> String {
        let text = run(super::nvs_core_uuid_to_string, &[value]).expect("a built UUID renders");
        let out = String::from_utf8(
            text.as_str_bytes()
                .expect("`toString` answers with a `string`")
                .to_vec(),
        )
        .expect("`rule:types/bytes` guarantees a `string` is UTF-8");
        #[expect(
            unsafe_code,
            reason = "this frame owns the reference each helper built, and \
                      `toString` borrowed its receiver rather than consuming it"
        )]
        unsafe {
            text.release();
            value.release();
        }
        out
    }

    /// The slot names this module indexes by are the ones the registry
    /// declares, in that order — the assertion that makes a paste error in
    /// either list a test failure rather than a swapped 64 bits.
    #[test]
    fn the_slot_indices_match_the_declared_layout() {
        assert_eq!(HIGH_SLOT, CLASS.slot("high"));
        assert_eq!(LOW_SLOT, CLASS.slot("low"));
        assert_eq!(CLASS.slots.len(), 2);
    }

    // covers: Core\Uuid::v4
    #[test]
    fn a_v4_is_canonical_lower_case_hex_with_its_version_and_variant() {
        for _ in 0..64 {
            let text = rendered(run(super::nvs_core_uuid_v4, &[]).expect("`v4` never fails"));
            assert_eq!(text.len(), 36);
            assert_eq!(
                text.char_indices()
                    .filter(|(_, ch)| *ch == '-')
                    .map(|(at, _)| at)
                    .collect::<Vec<_>>(),
                [8, 13, 18, 23]
            );
            assert!(
                text.chars()
                    .all(|ch| ch == '-' || ch.is_ascii_digit() || ('a'..='f').contains(&ch)),
                "`{text}` is not lower-case hex"
            );
            assert_eq!(&text[14..15], "4", "`{text}` is not version 4");
            assert!(
                matches!(&text[19..20], "8" | "9" | "a" | "b"),
                "`{text}` is not an RFC 9562 variant"
            );
        }
    }

    // covers: Core\Uuid::v7
    #[test]
    fn a_v7_carries_its_version_and_a_clock_that_moves_forward() {
        let first = rendered(run(super::nvs_core_uuid_v7, &[]).expect("`v7` never fails"));
        assert_eq!(&first[14..15], "7", "`{first}` is not version 7");
        assert!(
            matches!(&first[19..20], "8" | "9" | "a" | "b"),
            "`{first}` is not an RFC 9562 variant"
        );

        // The millisecond timestamp is the leading 12 hex digits, which the
        // canonical form's first hyphen splits 8 and 4 — so the slice that
        // holds all of it is `[..13]`, not `[..12]`. The hyphen sits at a
        // fixed position in both, so comparing the slices as text orders them
        // the way comparing the timestamps would.
        std::thread::sleep(std::time::Duration::from_millis(4));
        let second = rendered(run(super::nvs_core_uuid_v7, &[]).expect("`v7` never fails"));
        assert!(
            second[..13] > first[..13],
            "`{second}` does not sort after `{first}`"
        );
    }

    /// Two draws differing is not a proof of anything on its own, but a member
    /// that answered with a constant would fail it every time.
    #[test]
    fn two_draws_of_each_version_differ() {
        for member in [
            super::nvs_core_uuid_v4 as unsafe extern "C" fn(_, _, _) -> i32,
            super::nvs_core_uuid_v7,
        ] {
            let first = rendered(run(member, &[]).expect("a draw never fails"));
            let second = rendered(run(member, &[]).expect("a draw never fails"));
            assert_ne!(first, second);
        }
    }

    /// A UUID survives the trip through its two slots unchanged — the property
    /// the whole "hold integers, render on demand" decision rests on.
    #[test]
    fn the_slots_round_trip_a_known_value() {
        let known = uuid::Uuid::from_u64_pair(0x0011_2233_4455_6677, 0x8899_aabb_ccdd_eeff);
        assert_eq!(
            rendered(super::built(known)),
            "00112233-4455-6677-8899-aabbccddeeff"
        );
    }

    /// The sixteen octets `$uuid->toBytes()` answers. The receiver is borrowed
    /// rather than released, because every caller below renders it afterwards.
    fn octets(value: &Value) -> [u8; 16] {
        let buffer = run(super::nvs_core_uuid_to_bytes, &[*value]).expect("`toBytes` never fails");
        let out: [u8; 16] = buffer
            .as_bytes()
            .expect("`toBytes` answers with a `bytes`")
            .try_into()
            .expect("a UUID is sixteen octets");
        #[expect(
            unsafe_code,
            reason = "this frame owns the buffer the helper built; the receiver \
                      is the caller's and is borrowed here"
        )]
        unsafe {
            buffer.release();
        }
        out
    }

    /// `Core\Uuid::fromBytes($octets)`, releasing the buffer this test built.
    fn from_octets(octets: &[u8]) -> Result<Value, i32> {
        let subject = Value::bytes(nvs_runtime::NvsStr::new(octets));
        let answer = run(super::nvs_core_uuid_from_bytes, &[subject]);
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built, and the helper \
                      borrowed its argument rather than consuming it"
        )]
        unsafe {
            subject.release();
        }
        answer
    }

    /// The octets are the same 128 bits the text spells, out and back in, over
    /// every shape this class holds: two written values, RFC 9562's nil and
    /// max, and draws from both generators. Counted over the sweep rather than
    /// read off a line, so a pair that agreed on some of them still fails —
    /// and the width is asserted on both sides of sixteen, because a member
    /// that took a short buffer would answer a UUID the caller never had.
    // covers: Core\Uuid::fromBytes, Core\Uuid::toBytes
    #[test]
    fn a_uuid_round_trips_through_its_sixteen_bytes() {
        let known = [
            "00112233-4455-6677-8899-aabbccddeeff",
            "f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4",
            "00000000-0000-0000-0000-000000000000",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
        ];
        let mut agreed = 0;
        for text in known {
            let value = super::built(super::read(text).expect("a canonical UUID"));
            let bytes = octets(&value);
            // The octets are the canonical digits in the canonical order, so
            // their hex is that text with the hyphens taken out — which is
            // what says `toBytes` is not writing the halves the other way up.
            let spelled: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            agreed += usize::from(spelled == text.replace('-', ""));
            agreed += usize::from(rendered(value) == text);
            agreed += usize::from(
                rendered(from_octets(&bytes).expect("sixteen octets are a UUID")) == text,
            );
        }
        assert_eq!(agreed, known.len() * 3);

        // A drawn UUID cannot be written down, so both generators go round the
        // same trip and only the agreement is counted.
        let mut trips = 0;
        for member in [
            super::nvs_core_uuid_v4 as unsafe extern "C" fn(_, _, _) -> i32,
            super::nvs_core_uuid_v7,
        ] {
            for _ in 0..8 {
                let drawn = run(member, &[]).expect("a draw never fails");
                let bytes = octets(&drawn);
                let text = rendered(drawn);
                trips += usize::from(
                    rendered(from_octets(&bytes).expect("sixteen octets are a UUID")) == text,
                );
            }
        }
        assert_eq!(trips, 16);

        // Sixteen is the only width, asserted from both sides: a buffer one
        // octet short and one octet long are refused, and so is the empty one.
        for width in [0_usize, 15, 17, 32] {
            assert!(
                from_octets(&vec![0; width]).is_err(),
                "{width} octets is not a UUID"
            );
        }
        assert_eq!(
            rendered(from_octets(&[0; 16]).expect("sixteen octets are a UUID")),
            "00000000-0000-0000-0000-000000000000"
        );
    }

    /// Whether `Core\Uuid::tryParse($text)` answers a UUID rather than `null`
    /// — the spelling `rule:expressions/try-parse` left standing when this class lost its
    /// `isValid`. Releases the string this test built and whatever came back.
    fn valid(text: &str) -> bool {
        let subject = Value::str(nvs_runtime::NvsStr::new(text.as_bytes()));
        let answer = run(super::nvs_core_uuid_try_parse, &[subject])
            .expect("`tryParse` answers rather than throwing");
        let parsed = answer.tag() != Some(nvs_runtime::Tag::Null);
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built and the one \
                      the conversion answered with; the helper borrowed its \
                      argument rather than consuming it"
        )]
        unsafe {
            answer.release();
            subject.release();
        }
        parsed
    }

    // covers: Core\Uuid::parse, Core\Uuid::toString
    #[test]
    fn the_canonical_form_parses_in_either_case_and_renders_lower() {
        let subject = Value::str(nvs_runtime::NvsStr::new(
            b"F9168C5E-CEB2-4FAA-B6BF-329BF39FA1E4",
        ));
        let parsed = run(super::nvs_core_uuid_parse, &[subject]).expect("that is a UUID");
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
        assert_eq!(rendered(parsed), "f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4");
    }

    /// RFC 9562 §§ 5.9 and 5.10: neither the nil nor the max UUID names a
    /// version, and both are UUIDs — the module docs own why no member here
    /// reads the version nibble as a checksum.
    // covers: Core\Uuid::tryParse
    #[test]
    fn the_nil_and_max_uuids_are_valid() {
        assert!(valid("00000000-0000-0000-0000-000000000000"));
        assert!(valid("ffffffff-ffff-ffff-ffff-ffffffffffff"));
    }

    /// The three spellings `uuid` itself would take and this module refuses,
    /// each written as the same value the canonical form above holds.
    // covers: Core\Uuid::tryParse
    #[test]
    fn the_three_non_canonical_spellings_are_refused() {
        assert!(!valid("f9168c5eceb24faab6bf329bf39fa1e4"));
        assert!(!valid("{f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4}"));
        assert!(!valid("urn:uuid:f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4"));
    }

    #[test]
    fn near_misses_are_refused() {
        // Right length, one character that is not hex.
        assert!(!valid("f9168c5e-ceb2-4faa-b6bf-329bf39fa1eg"));
        // Right characters, hyphens in the wrong places.
        assert!(!valid("f9168c5ec-eb2-4faa-b6bf-329bf39fa1e4"));
        assert!(!valid(""));
        assert!(!valid("   f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4   "));
    }

    /// A rejected operand is quoted back bounded, so a caller cannot choose
    /// how many bytes a log line gains — [`super::shown`]'s whole job.
    #[test]
    fn a_rejected_operand_is_quoted_bounded() {
        let long = "z".repeat(4096);
        let subject = Value::str(nvs_runtime::NvsStr::new(long.as_bytes()));
        let status = run(super::nvs_core_uuid_parse, &[subject]).expect_err("that is not a UUID");
        assert_eq!(status, nvs_runtime::THROWN);
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
        assert_eq!(super::shown(&long).chars().count(), super::SHOWN_CHARS + 1);
        assert_eq!(
            super::shown("00000000-0000-4000-8000-000000000000"),
            "00000000-0000-4000-8000-000000000000"
        );
    }

    #[test]
    fn a_non_string_operand_is_a_contained_fault() {
        let status = run(super::nvs_core_uuid_parse, &[Value::int(7)])
            .expect_err("an `int` is not a `string`");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    #[test]
    fn a_non_object_receiver_is_a_contained_fault() {
        let status = run(super::nvs_core_uuid_to_string, &[Value::int(7)])
            .expect_err("an `int` is not a `Core\\Uuid`");
        assert_eq!(status, nvs_runtime::FATAL);
    }
}
