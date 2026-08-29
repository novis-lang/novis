//! `Core\Uuid` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 11's second table, which replaces `uniqid`, `com_create_guid` and every
//! userland UUID library with one type.
//!
//! All four of that table's members are here. A `Core\Uuid` is a **value with a
//! type**, not a 36-character string a program passes around and re-validates
//! at every boundary: that is what makes ADR 0067 § 4's native `UUID` column
//! binding and ADR 0077's `Core\Uuid` route segment able to state what they
//! take.
//!
//! # Reading text: the canonical form, and nothing else
//!
//! `parse` accepts RFC 9562 § 4's `8-4-4-4-12` hyphenated form in either case,
//! and refuses the three shapes the `uuid` crate would otherwise also take: the
//! unhyphenated 32 hex digits, `{…}` braces, and a `urn:uuid:` prefix. One
//! length check does it, so nothing is re-implemented to be strict.
//!
//! The reason is that each of those is a **second spelling of one value**, and
//! a program that keys a cache, a rate limiter or an audit log on the text
//! while authorizing on the UUID then has two strings that are one identity.
//! ADR 0007 § 2's `string → int` row takes exactly this line — the whole string
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
//! construction, which is AGENTS.md's priority 5 spent to buy priority 3 at
//! the commoner of the two sites. A single `bytes` slot would be the natural
//! third answer and is gap 1: `nvs_runtime::Tag` has no `Bytes` variant yet.
//!
//! # The dependency, and why `uuid`
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 4's first
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
//! it. Nothing is retained between calls, and no table grows with traffic.
//!
//! # Known gaps
//!
//! 1. **There is no `bytes` round trip**, which is what an ADR 0067 driver
//!    binding a native `UUID` column will want. It waits on the same
//!    `nvs_runtime::Tag::Bytes` variant [`crate::random`]'s gap 1 does.
//! 2. **`==` on two `Uuid` values is object identity**, so two instances
//!    holding the same 128 bits are not equal
//!    ([ADR 0090](../../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
//!    § 3's non-scalar row). Comparing `toString()` is the spelling that works
//!    today. This is every `Core`-owned instance's gap, not this class's, and
//!    it is why no member here answers `bool` about another UUID.
//! 3. **`v7` has no intra-millisecond counter.** RFC 9562's optional
//!    monotonic counter is not built, so two UUIDs drawn inside the same
//!    millisecond order randomly against each other. Across milliseconds the
//!    ordering is exact, which is the property a database key is chosen for;
//!    a program that needs a total order inside one millisecond needs a
//!    sequence, not a UUID.

use rand::Rng;
use uuid::{Builder, Uuid};

use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Uuid`'s fully-qualified name, written once so the registry row and
/// every [`CoreTy::Instance`] naming it cannot drift apart.
pub const NAME: &str = r"Core\Uuid";

/// `Core\Uuid`'s registry rows — § 11's second table's four static members,
/// plus the rendering member that section's table now writes. `isValid` was a
/// fifth until ADR 0066 § 3b deleted it: `tryParse` is that question asked
/// through `parse` itself, so the two were one predicate and R17 keeps one.
pub const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "v4",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uuid_v4",
            doc: None,
        },
        CoreMethod {
            name: "v7",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uuid_v7",
            doc: None,
        },
        CoreMethod {
            name: "parse",
            names: &["s"],
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uuid_parse",
            doc: None,
        },
        CoreMethod {
            name: "tryParse",
            names: &["s"],
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(NAME)),
            symbol: "nvs_core_uuid_try_parse",
            doc: None,
        },
    ],
    instance: &[CoreMethod {
        name: "toString",
        names: &[],
        params: &[],
        defaults: &[],
        return_ty: CoreTy::Str,
        symbol: "nvs_core_uuid_to_string",
        doc: None,
    }],
    slots: &["high", "low"],
    constants: &[],
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
        "nvs_core_uuid_to_string" => (nvs_core_uuid_to_string as *const ()).cast(),
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

/// The `Uuid` `text` spells, or `None` if it is not the canonical form.
///
/// The length check is what narrows `uuid`'s four accepted spellings to the one
/// this module reads — see the module docs. The unhyphenated form is 32
/// characters, braced is 38 and a `urn:uuid:` is 45, so `36` admits none of
/// them, and the crate's own reader still decides everything about the 36
/// characters that remain.
fn read(text: &str) -> Option<Uuid> {
    (text.len() == uuid::fmt::Hyphenated::LENGTH)
        .then(|| Uuid::try_parse(text).ok())
        .flatten()
}

/// The `string` in argument slot 0, for the two members that read text.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for [`uuid_of`]'s reason — and only
/// that one: a `string` is guaranteed-valid UTF-8 (ADR 0009), and the tag
/// [`Value::as_text`] checks *is* that guarantee, so nothing here re-derives it.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uuid::{member} expected a `string`, got tag {}",
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
/// ([ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
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
fn uuid_of(args: &[Value], at: usize, member: &str) -> Result<Uuid, Fault> {
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
    fn nvs_core_uuid_v4(_ctx, args: [0]) {
        let _ = args;
        let mut bytes = [0_u8; 16];
        rand::rng().fill_bytes(&mut bytes);
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
    /// Ordering inside one millisecond is random — gap 3.
    fn nvs_core_uuid_v7(_ctx, args: [0]) {
        let _ = args;
        let millis = u64::try_from(jiff::Timestamp::now().as_millisecond()).unwrap_or(0);
        let mut bytes = [0_u8; 10];
        rand::rng().fill_bytes(&mut bytes);
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
    /// **Throws on anything else** (ADR 0063 R4), which is what makes the
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
    /// [ADR 0066](../../../../docs/adr/0066-nullable-conversion-operator.md)
    /// § 3a: [`nvs_core_uuid_parse`] exactly, with `null` where it throws.
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
    /// `$uuid->toString(): string` — the canonical
    /// `8-4-4-4-12` lower-case hex form, which is what RFC 9562 § 4 writes and
    /// what every database, log line and HTTP header expects.
    ///
    /// **Lower case, always.** The RFC's own grammar accepts either case on
    /// input and emits lower, and two renderings of one UUID that differ only
    /// in case would compare unequal as strings — which is the comparison gap
    /// 3 leaves programs writing today.
    ///
    /// Named `toString` rather than `format` or `toText` so that it is already
    /// the member `Stringable` declares
    /// ([ADR 0028](../../../../docs/adr/0028-closing-the-remaining-magic-methods.md)
    /// § 1), which is what makes `echo $uuid` render: that name is the whole
    /// of what says a `Core`-owned class is stringifiable, read by
    /// `nvs_types::expr::operators::require_stringable` where the operand's
    /// type names this class and by [`crate::instance`]'s descriptor renderer
    /// where it names none.
    fn nvs_core_uuid_to_string(_ctx, args: [1]) {
        let value = uuid_of(args, 0, "toString")?;
        let mut buffer = [0_u8; uuid::fmt::Hyphenated::LENGTH];
        let text = value.hyphenated().encode_lower(&mut buffer);
        Ok(Value::str(NvsStr::new(text.as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, OutputSink, Value, call};

    use super::{CLASS, HIGH_SLOT, LOW_SLOT};

    /// Runs one member through the ADR 0002 boundary compiled code reaches it
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
        .expect("ADR 0009 guarantees a `string` is UTF-8");
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

    /// Whether `Core\Uuid::tryParse($text)` answers a UUID rather than `null`
    /// — the spelling ADR 0066 § 3a left standing when this class lost its
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
    #[test]
    fn the_nil_and_max_uuids_are_valid() {
        assert!(valid("00000000-0000-0000-0000-000000000000"));
        assert!(valid("ffffffff-ffff-ffff-ffff-ffffffffffff"));
    }

    /// The three spellings `uuid` itself would take and this module refuses,
    /// each written as the same value the canonical form above holds.
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
