//! `Core\Encoding` — docs/spec/01-core-library.md § 7, the members that sit
//! exactly on the `bytes`↔`string` boundary
//! (`rule:types/bytes`).
//!
//! Everything this class does is a *representation* change: the same
//! information, spelled as text a person or a protocol can carry, or spelled
//! back as the octets it came from. That is why it is one class rather than a
//! member on each of `Core\Str` and `Core\Bytes` — the operation belongs to
//! neither type, it belongs to the seam between them.
//!
//! # What is registered here so far
//!
//! The whole of § 7's `Core\Encoding` table: `toHex`/`fromHex`,
//! `toBase64`/`fromBase64`, `toBase64Url`/`fromBase64Url`,
//! `toBase32`/`fromBase32`, and the `encodeText`/`decodeText`/`isValidText`
//! trio over [`CHARSET`]. § 7's other class, `Core\Bytes`, is elsewhere.
//!
//! `rule:classes/an-encoder-ends-a-cycle-by-identity` has nothing to carry
//! here, because no member of this class walks a value graph at all: every one
//! of them reads a single `bytes` or `string` out of slot 0 ([`bytes_of`],
//! [`text_of`]) and at most a `Core\Charset` case out of slot 1
//! ([`charset_of`]), so there is no container to descend into and no value an
//! identity could be taken of. What these members encode is a byte sequence,
//! not a structure.
//!
//! # `Charset` is the WHATWG index, and Novis keeps three labels apart
//!
//! § 7 says the roster is the **WHATWG Encoding Standard's index**, not a list
//! the spec curates, so adding an encoding is a dependency update rather than
//! a design decision. [`CHARSET`] is that index, and
//! `the_delegated_roster_is_the_standards_own` checks each case name against
//! `encoding_rs`'s own `name()` rather than trusting this file's spelling.
//!
//! Two deliberate departures, both forced by what a `Charset` *is* here:
//!
//! - **`Ascii` and `Latin1` are their own cases.** The standard folds the
//!   labels `ascii`, `us-ascii`, `iso-8859-1` and `latin1` into
//!   `windows-1252`, because it is describing how a browser should read a
//!   document that *claims* one of them — a guess about mislabelled content.
//!   A `Charset` argument is not a claim, it is an instruction, and the two
//!   differ over `0x80`-`0x9f`, where windows-1252 has typographic characters
//!   and ISO-8859-1 has the C1 controls. Reading `Charset::Latin1` as
//!   windows-1252 would answer a *different* string, which is the substitution
//!   `rule:types/conversion` removes
//!   from the language. `Ascii` has a second reason: § 7's own table gives
//!   `isValidText` as the replacement for `mb_check_encoding`, and
//!   `mb_check_encoding($s, "ASCII")` is the commonest call of it — folded
//!   into windows-1252 that question answers `true` for every byte string.
//! - **`replacement` is absent.** That entry of the index exists so a browser
//!   cannot be tricked into decoding an attack string in a confusable
//!   encoding: it maps *every* input to a single error. Under R4 a
//!   `Charset::Replacement` would therefore be a case that throws on
//!   everything — surface with no meaning behind it, which is the rule
//!   [`crate::registry::ENUMS`] already states for a case a program can write
//!   and pass nowhere.
//!
//! Five cases are decoded and encoded here rather than by `encoding_rs`:
//! `Utf8` (`str::from_utf8` and the buffer itself, with no copy on the way
//! out), `Ascii` and `Latin1` for the reason above, and `Utf16Le`/`Utf16Be`
//! because the standard makes UTF-16 **decode-only** — `Encoding::encode` on
//! one silently answers UTF-8, which
//! `every_delegated_encoding_answers_in_its_own_encoding` is here to catch if
//! a case is ever moved onto the delegated side.
//!
//! # A conversion is exact or it throws
//!
//! `decodeText` refuses a malformed sequence rather than emitting U+FFFD, and
//! `encodeText` refuses a character the charset cannot spell rather than
//! emitting `&#NNNN;`. That is R4 and `rule:types/conversion` — there is no `//IGNORE`
//! and no `//TRANSLIT`, which is the whole reason `iconv`'s suffixes have no
//! equivalent — and it is *not* `encoding_rs`'s default: its `decode` replaces
//! and its `encode` reports substitution in a flag most callers drop. The
//! members below take `decode_to_string_without_replacement` and check that
//! flag. `isValidText` is the same question asked without the throw, for a
//! caller who wants to choose.
//!
//! # The four base64 members are two alphabets and two padding rules
//!
//! RFC 4648 defines one algorithm and two alphabets, and the wild has settled
//! on a different padding convention for each. Novis writes what each side
//! actually produces rather than a switch:
//!
//! | member | alphabet | padding |
//! |---|---|---|
//! | `toBase64` / `fromBase64` | `A-Za-z0-9+/` (§ 4) | **required** — `base64_encode`'s own output |
//! | `toBase64Url` / `fromBase64Url` | `A-Za-z0-9-_` (§ 5) | **absent** — JWT, and the `rtrim(strtr(…))` idiom the spec row names |
//!
//! Neither decoder accepts the other's spelling, and neither accepts
//! whitespace, a newline or a stray `=`. That is the same rule `fromHex` reads
//! one paragraph down and the same reason: PHP's `base64_decode` in its
//! default mode discards every character outside the alphabet, so a corrupted
//! transfer decodes to a *shorter* value instead of an error, and one truncated
//! signature compares unequal rather than reporting why.
//!
//! **Canonicality is checked too**, which is the part a hand-written sextet
//! loop gets wrong: the final character of a 2- or 3-character group carries
//! bits that no octet reads, so `"aa=="` and `"ab=="` would otherwise decode to
//! the same octet and only one of them round-trips. Both are refused unless the
//! unread bits are zero (RFC 4648 § 3.5).
//!
//! # base32 is one pair, so its decoder is lenient where base64's is not
//!
//! `toBase32` writes RFC 4648 § 6's alphabet **upper case and unpadded**,
//! which is how an `otpauth:` secret is written — TOTP
//! (`rule:security/protocol-roster`)
//! being the consumer § 7's row names, since PHP has nothing here to replace.
//!
//! `fromBase32` then reads **either case, and padding that is either canonical
//! or absent**. That looks like the tolerance the two base64 decoders refuse
//! one section up, and it is a different thing:
//!
//! - The base64 refusal is a *disambiguation*. Two members exist, they differ
//!   only in alphabet and padding, and a decoder taking both spellings could
//!   not tell a caller which member they meant. There is one base32 pair, so
//!   there is nothing to be ambiguous with.
//! - Neither variance can change the octets. § 6's alphabet has no lower-case
//!   member, so folding case cannot collide with a symbol; padding carries no
//!   bits. Every rule that *could* change the answer is still enforced —
//!   a symbol outside the alphabet (a space grouping a secret for a reader is
//!   one, and so are the digits `0`, `1` and `8`), a truncated final group,
//!   non-canonical trailing bits, and padding present but wrong.
//!
//! Which is the same distinction the whole module runs on: refuse anything
//! that would silently answer a *different* value, and accept a second
//! spelling of the same one — exactly as `fromHex` reads either case.
//!
//! # Why base64 takes a dependency and hex does not
//!
//! Everything above is why: hex has no alphabet question, no padding, no
//! variant and no non-canonical spelling, so there is nothing for a crate to
//! know. base64 has four such rules, `fromBase64` is a member request bodies
//! reach, and every one of those rules is a documented CVE somewhere. The
//! `base64` crate is the pick under
//! `rule:packaging/a-c-dependency-answers-two-questions` —
//! pure Rust, no build script, no C, and already in this tree's lock file
//! under `wasmtime-internal-cache`, so it adds no crate at all.
//! `Cargo.toml`'s `[workspace.dependencies]` comment states the pick; this
//! module owns which engine each member is, above.
//!
//! # Why hex takes no dependency
//!
//! `rule:packaging/a-c-dependency-answers-two-questions` asks
//! two questions of an outside crate, and base-16 answers both the wrong way:
//! the whole algorithm is a nibble table, there is no specification drift to
//! track and no security-relevant parsing to get wrong, so a dependency would
//! buy nothing and add a supply-chain edge to the runtime every request links.
//! **The base64 and base32 rows are a different answer** — their alphabets,
//! padding rules and URL-safe variants are exactly the kind of detail a
//! well-used crate has already gotten wrong once and fixed. base64's is
//! `base64` and base32's is `data-encoding`, both above. This comment is here
//! so nobody reads hex's answer as a rule for the section.
//!
//! # Encoding is total, decoding throws
//!
//! `toHex` cannot fail: every octet has a spelling. `fromHex` is the checked
//! direction (`rule:core-api/shape-rules`
//! R4) — it throws on an odd length or a non-hexadecimal character rather than
//! substituting, dropping or truncating, which is the same reason `rule:types/conversion`
//! refuses `iconv`'s `//IGNORE`. A caller who wants the question without the
//! throw asks it of the text before converting.

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use data_encoding::{BASE32, BASE32_NOPAD};
use encoding_rs::{DecoderResult, Encoding};
use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{
    CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Encoding`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Encoding";

/// `Core\Charset`'s fully-qualified name, written once for [`CHARSET`], for
/// the registry rows that take one, and for every message quoting it.
pub(crate) const CHARSET_NAME: &str = r"Core\Charset";

/// Spec § 7's `Charset` — the WHATWG Encoding Standard's index, in that
/// document's own table order, with the departures the module doc argues.
///
/// The integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`]. They are **ABI**: [`SCHEMES`] is
/// indexed by them, so reordering this list is a behaviour change rather than
/// a cosmetic one, and `the_case_table_and_the_scheme_table_are_one_roster`
/// fails if the two ever disagree in length or order.
pub(crate) const CHARSET: CoreEnum = CoreEnum {
    name: CHARSET_NAME,
    cases: &[
        // The five this module decodes itself — see the module doc.
        ("Utf8", 0),
        ("Utf16Le", 1),
        ("Utf16Be", 2),
        ("Ascii", 3),
        ("Latin1", 4),
        // The index's legacy single-byte table, in its order.
        ("Ibm866", 5),
        ("Iso88592", 6),
        ("Iso88593", 7),
        ("Iso88594", 8),
        ("Iso88595", 9),
        ("Iso88596", 10),
        ("Iso88597", 11),
        ("Iso88598", 12),
        ("Iso88598I", 13),
        ("Iso885910", 14),
        ("Iso885913", 15),
        ("Iso885914", 16),
        ("Iso885915", 17),
        ("Iso885916", 18),
        ("Koi8R", 19),
        ("Koi8U", 20),
        ("Macintosh", 21),
        ("Windows874", 22),
        ("Windows1250", 23),
        ("Windows1251", 24),
        ("Windows1252", 25),
        ("Windows1253", 26),
        ("Windows1254", 27),
        ("Windows1255", 28),
        ("Windows1256", 29),
        ("Windows1257", 30),
        ("Windows1258", 31),
        ("XMacCyrillic", 32),
        // The index's legacy multi-byte tables, in their order.
        ("Gbk", 33),
        ("Gb18030", 34),
        ("Big5", 35),
        ("EucJp", 36),
        ("Iso2022Jp", 37),
        ("ShiftJis", 38),
        ("EucKr", 39),
        // The index's legacy miscellaneous table, less `replacement`.
        ("XUserDefined", 40),
    ],
    doc: Some(&CHARSET_DOC),
};

/// [`CHARSET`]'s reference card — `rule:core-api/reference-card`; a table rather than a card, so
/// only `short` is written, as [`EnumDoc::cases`] allows.
const CHARSET_DOC: EnumDoc = EnumDoc {
    short: "The encoding a `Core\\Encoding` text conversion reads or writes — one case per \
            encoding in the WHATWG Encoding Standard's index, in that document's order and in \
            Novis's casing, with `Ascii` and `Latin1` kept apart from `Windows1252` and no \
            `Replacement` case, since a conversion is exact or it throws.",
    cases: &[],
};

/// How one [`CHARSET`] case's octets are made.
///
/// A Rust mirror of the source-visible enum rather than a reuse of it, for
/// [`crate::hash`]'s reason: the enum is what the *checker* reads and this is
/// what the conversion dispatches on. Five variants rather than one wrapping
/// `encoding_rs` because five cases are not that crate's to answer — the
/// module doc says which and why.
#[derive(Clone, Copy, Debug)]
enum Scheme {
    /// UTF-8 both ways, which is a validation one way and free the other:
    /// a `string` is already the octets (`rule:types/conversion`).
    Utf8,
    /// UTF-16 in the stated byte order. Encoding is written here because the
    /// standard has no UTF-16 encoder at all.
    Utf16 {
        /// `Utf16Le` when true, `Utf16Be` when false.
        little_endian: bool,
    },
    /// US-ASCII proper: `0x00`-`0x7f` and nothing above it.
    Ascii,
    /// ISO-8859-1 proper: every octet is the code point of the same value,
    /// C1 controls included.
    Latin1,
    /// The standard's own table for this encoding, run without replacement.
    Whatwg(&'static Encoding),
}

/// One [`Scheme`] per [`CHARSET`] case, at that case's own integer.
///
/// A `static` rather than a `const`, and therefore a second table rather than
/// a field on [`CoreEnum::cases`]: `encoding_rs`'s handles are `static`s, and
/// a `const` may not read one. The pairing is checked instead — see
/// [`CHARSET`].
static SCHEMES: [Scheme; CHARSET.cases.len()] = [
    Scheme::Utf8,
    Scheme::Utf16 {
        little_endian: true,
    },
    Scheme::Utf16 {
        little_endian: false,
    },
    Scheme::Ascii,
    Scheme::Latin1,
    Scheme::Whatwg(encoding_rs::IBM866),
    Scheme::Whatwg(encoding_rs::ISO_8859_2),
    Scheme::Whatwg(encoding_rs::ISO_8859_3),
    Scheme::Whatwg(encoding_rs::ISO_8859_4),
    Scheme::Whatwg(encoding_rs::ISO_8859_5),
    Scheme::Whatwg(encoding_rs::ISO_8859_6),
    Scheme::Whatwg(encoding_rs::ISO_8859_7),
    Scheme::Whatwg(encoding_rs::ISO_8859_8),
    Scheme::Whatwg(encoding_rs::ISO_8859_8_I),
    Scheme::Whatwg(encoding_rs::ISO_8859_10),
    Scheme::Whatwg(encoding_rs::ISO_8859_13),
    Scheme::Whatwg(encoding_rs::ISO_8859_14),
    Scheme::Whatwg(encoding_rs::ISO_8859_15),
    Scheme::Whatwg(encoding_rs::ISO_8859_16),
    Scheme::Whatwg(encoding_rs::KOI8_R),
    Scheme::Whatwg(encoding_rs::KOI8_U),
    Scheme::Whatwg(encoding_rs::MACINTOSH),
    Scheme::Whatwg(encoding_rs::WINDOWS_874),
    Scheme::Whatwg(encoding_rs::WINDOWS_1250),
    Scheme::Whatwg(encoding_rs::WINDOWS_1251),
    Scheme::Whatwg(encoding_rs::WINDOWS_1252),
    Scheme::Whatwg(encoding_rs::WINDOWS_1253),
    Scheme::Whatwg(encoding_rs::WINDOWS_1254),
    Scheme::Whatwg(encoding_rs::WINDOWS_1255),
    Scheme::Whatwg(encoding_rs::WINDOWS_1256),
    Scheme::Whatwg(encoding_rs::WINDOWS_1257),
    Scheme::Whatwg(encoding_rs::WINDOWS_1258),
    Scheme::Whatwg(encoding_rs::X_MAC_CYRILLIC),
    Scheme::Whatwg(encoding_rs::GBK),
    Scheme::Whatwg(encoding_rs::GB18030),
    Scheme::Whatwg(encoding_rs::BIG5),
    Scheme::Whatwg(encoding_rs::EUC_JP),
    Scheme::Whatwg(encoding_rs::ISO_2022_JP),
    Scheme::Whatwg(encoding_rs::SHIFT_JIS),
    Scheme::Whatwg(encoding_rs::EUC_KR),
    Scheme::Whatwg(encoding_rs::X_USER_DEFINED),
];

/// `Core\Encoding`'s registry rows — § 7's text trio, its hex pair, its base64
/// family and its base32 pair.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "encodeText",
            names: &["s", "charset"],
            params: &[CoreTy::Text(Qual::Contagious), CoreTy::Enum(CHARSET_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_encoding_encode_text",
            doc: Some(&ENCODE_TEXT_DOC),
        },
        CoreMethod {
            name: "decodeText",
            names: &["b", "charset"],
            params: &[CoreTy::Blob(Qual::Contagious), CoreTy::Enum(CHARSET_NAME)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_encoding_decode_text",
            doc: Some(&DECODE_TEXT_DOC),
        },
        CoreMethod {
            name: "isValidText",
            names: &["b", "charset"],
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Enum(CHARSET_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_encoding_is_valid_text",
            doc: Some(&IS_VALID_TEXT_DOC),
        },
        CoreMethod {
            name: "toBase64",
            names: &["b"],
            params: &[CoreTy::Blob(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_encoding_to_base64",
            doc: Some(&TO_BASE64_DOC),
        },
        CoreMethod {
            name: "fromBase64",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_encoding_from_base64",
            doc: Some(&FROM_BASE64_DOC),
        },
        CoreMethod {
            name: "toBase64Url",
            names: &["b"],
            params: &[CoreTy::Blob(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_encoding_to_base64_url",
            doc: Some(&TO_BASE64_URL_DOC),
        },
        CoreMethod {
            name: "fromBase64Url",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_encoding_from_base64_url",
            doc: Some(&FROM_BASE64_URL_DOC),
        },
        CoreMethod {
            name: "toBase32",
            names: &["b"],
            params: &[CoreTy::Blob(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_encoding_to_base32",
            doc: Some(&TO_BASE32_DOC),
        },
        CoreMethod {
            name: "fromBase32",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_encoding_from_base32",
            doc: Some(&FROM_BASE32_DOC),
        },
        CoreMethod {
            name: "toHex",
            names: &["b"],
            params: &[CoreTy::Blob(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_encoding_to_hex",
            doc: Some(&TO_HEX_DOC),
        },
        CoreMethod {
            name: "fromHex",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_encoding_from_hex",
            doc: Some(&FROM_HEX_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Encoding::encodeText`'s reference card — `rule:core-api/reference-card`.
const ENCODE_TEXT_DOC: MethodDoc = MethodDoc {
    short: "Writes `$s` as `$charset`'s octets, as `iconv`, `mb_convert_encoding` and \
            `utf8_encode` do — exactly, with no `//IGNORE` or `//TRANSLIT` mode: a character \
            the charset cannot spell throws rather than becoming `?` or `&#NNNN;`.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The text to encode.",
            shape: &[],
        },
        ParamDoc {
            name: "charset",
            desc: "The encoding to write.",
            shape: &[],
        },
    ],
    ret: "The encoded octets.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$charset` has no spelling for a character of `$s`; the message names the \
               character and its offset.",
    }],
};

/// `Core\Encoding::decodeText`'s reference card — `rule:core-api/reference-card`.
const DECODE_TEXT_DOC: MethodDoc = MethodDoc {
    short: "Reads the octets `$b` as `$charset` into a string, as `iconv`, \
            `mb_convert_encoding` and `utf8_decode` do — exactly: a sequence the charset \
            cannot read throws rather than becoming U+FFFD or being dropped.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The octets to decode.",
            shape: &[],
        },
        ParamDoc {
            name: "charset",
            desc: "The encoding they are in.",
            shape: &[],
        },
    ],
    ret: "The decoded text; the empty string for the empty buffer.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A byte sequence of `$b` is not valid `$charset`; the message names its offset.",
    }],
};

/// `Core\Encoding::isValidText`'s reference card — `rule:core-api/reference-card`.
const IS_VALID_TEXT_DOC: MethodDoc = MethodDoc {
    short: "Tells whether every byte sequence in `$b` is one `$charset` reads, as \
            `mb_check_encoding` does — `decodeText`'s question without the throw.",
    params: &[
        ParamDoc {
            name: "b",
            desc: "The octets to check.",
            shape: &[],
        },
        ParamDoc {
            name: "charset",
            desc: "The encoding they are checked against.",
            shape: &[],
        },
    ],
    ret: "`true` when `decodeText` would succeed; always `true` under `Charset::Latin1`, \
          which gives all 256 octets a meaning.",
    errors: &[],
};

/// `Core\Encoding::toBase64`'s reference card — `rule:core-api/reference-card`.
const TO_BASE64_DOC: MethodDoc = MethodDoc {
    short: "Spells `$b` in RFC 4648 § 4's base64 alphabet, padded — byte for byte what \
            `base64_encode` answers.",
    params: &[ParamDoc {
        name: "b",
        desc: "The octets to encode.",
        shape: &[],
    }],
    ret: "The base64 text; the empty string for the empty buffer.",
    errors: &[],
};

/// `Core\Encoding::fromBase64`'s reference card — `rule:core-api/reference-card`.
const FROM_BASE64_DOC: MethodDoc = MethodDoc {
    short: "Reads base64 text `$s` back to octets, as `base64_decode` does in strict mode and \
            stricter: § 4's alphabet only, padding required and canonical, and no unread bits \
            in the last symbol. URL-safe text is `fromBase64Url`'s to read.",
    params: &[ParamDoc {
        name: "s",
        desc: "The base64 text.",
        shape: &[],
    }],
    ret: "The decoded octets; the empty buffer for the empty string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$s` is not strict base64 — a symbol outside the alphabet, `-` and `_` \
               included, missing or wrong padding, a truncated final group, or non-canonical \
               trailing bits.",
    }],
};

/// `Core\Encoding::toBase64Url`'s reference card — `rule:core-api/reference-card`.
const TO_BASE64_URL_DOC: MethodDoc = MethodDoc {
    short: "Spells `$b` in RFC 4648 § 5's URL-safe base64 alphabet, unpadded — the \
            `rtrim(strtr(base64_encode($b), \"+/\", \"-_\"), \"=\")` idiom, as a JWT or a \
            query string expects it.",
    params: &[ParamDoc {
        name: "b",
        desc: "The octets to encode.",
        shape: &[],
    }],
    ret: "The URL-safe base64 text; the empty string for the empty buffer.",
    errors: &[],
};

/// `Core\Encoding::fromBase64Url`'s reference card — `rule:core-api/reference-card`.
const FROM_BASE64_URL_DOC: MethodDoc = MethodDoc {
    short: "Reads URL-safe base64 text `$s` back to octets — `toBase64Url`'s other half, as \
            strict as `fromBase64` and refusing padding rather than tolerating it.",
    params: &[ParamDoc {
        name: "s",
        desc: "The URL-safe base64 text, unpadded.",
        shape: &[],
    }],
    ret: "The decoded octets; the empty buffer for the empty string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$s` is not unpadded URL-safe base64 — a symbol outside the alphabet, `+`, \
               `/` and `=` included, a truncated final group, or non-canonical trailing bits.",
    }],
};

/// `Core\Encoding::toBase32`'s reference card — `rule:core-api/reference-card`.
const TO_BASE32_DOC: MethodDoc = MethodDoc {
    short: "Spells `$b` in RFC 4648 § 6's base32 alphabet, upper case and unpadded — the form \
            an `otpauth:` secret is written in; PHP has no counterpart.",
    params: &[ParamDoc {
        name: "b",
        desc: "The octets to encode.",
        shape: &[],
    }],
    ret: "The base32 text; the empty string for the empty buffer.",
    errors: &[],
};

/// `Core\Encoding::fromBase32`'s reference card — `rule:core-api/reference-card`.
const FROM_BASE32_DOC: MethodDoc = MethodDoc {
    short: "Reads base32 text `$s` back to octets — `toBase32`'s other half, taking either \
            case and padding that is canonical or absent, since neither changes which octets \
            come out.",
    params: &[ParamDoc {
        name: "s",
        desc: "The base32 text.",
        shape: &[],
    }],
    ret: "The decoded octets; the empty buffer for the empty string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$s` holds a symbol outside the alphabet — a space between groups included — \
               a truncated final group, non-canonical trailing bits, or padding that is \
               present but wrong.",
    }],
};

/// `Core\Encoding::toHex`'s reference card — `rule:core-api/reference-card`.
const TO_HEX_DOC: MethodDoc = MethodDoc {
    short: "Spells `$b` as lowercase hexadecimal, two digits per octet, as `bin2hex` and the \
            `unpack(\"H*\", …)` idiom do.",
    params: &[ParamDoc {
        name: "b",
        desc: "The octets to encode.",
        shape: &[],
    }],
    ret: "The hex text, twice `$b`'s length; the empty string for the empty buffer.",
    errors: &[],
};

/// `Core\Encoding::fromHex`'s reference card — `rule:core-api/reference-card`.
const FROM_HEX_DOC: MethodDoc = MethodDoc {
    short: "Reads hexadecimal text `$s` back to octets, as `hex2bin` does but throwing where \
            it warned and answered `false`: either case, two digits per octet, and nothing \
            between the pairs.",
    params: &[ParamDoc {
        name: "s",
        desc: "The hexadecimal text.",
        shape: &[],
    }],
    ret: "The decoded octets, half as many as `$s` has digits; the empty buffer for the empty \
          string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$s` has an odd number of digits, or a character that is not `0`-`9`, `a`-`f` \
               or `A`-`F` — a space, a colon or a `0x` prefix included.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_encoding_encode_text" => (nvs_core_encoding_encode_text as *const ()).cast(),
        "nvs_core_encoding_decode_text" => (nvs_core_encoding_decode_text as *const ()).cast(),
        "nvs_core_encoding_is_valid_text" => (nvs_core_encoding_is_valid_text as *const ()).cast(),
        "nvs_core_encoding_to_base64" => (nvs_core_encoding_to_base64 as *const ()).cast(),
        "nvs_core_encoding_from_base64" => (nvs_core_encoding_from_base64 as *const ()).cast(),
        "nvs_core_encoding_to_base64_url" => (nvs_core_encoding_to_base64_url as *const ()).cast(),
        "nvs_core_encoding_from_base64_url" => {
            (nvs_core_encoding_from_base64_url as *const ()).cast()
        }
        "nvs_core_encoding_to_base32" => (nvs_core_encoding_to_base32 as *const ()).cast(),
        "nvs_core_encoding_from_base32" => (nvs_core_encoding_from_base32 as *const ()).cast(),
        "nvs_core_encoding_to_hex" => (nvs_core_encoding_to_hex as *const ()).cast(),
        "nvs_core_encoding_from_hex" => (nvs_core_encoding_from_hex as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Reading arguments
// ============================================================================

/// The `bytes` in argument slot 0.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member if the slot carries another tag:
/// `nvs_types` already checked the declared type and compiled code wrote the
/// tag, so a mismatch is a runtime-contract violation rather than anything a
/// program can cause — the same treatment [`crate::uuid`] gives a `string`.
fn bytes_of<'a>(args: &'a [Value], member: &str) -> Result<&'a [u8], Fault> {
    args[0].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Encoding::{member} expected a `bytes`, got tag {}",
            args[0].tag_byte()
        ))
    })
}

/// The `string` in argument slot 0, for [`bytes_of`]'s reason.
///
/// One failure, the wrong tag. A `string` is guaranteed-valid UTF-8 (`rule:types/string-is-utf8`), and the tag [`Value::as_text`] checks *is* that guarantee, so there is
/// nothing left here to re-derive — `crate::str`'s own `text` states the cost
/// of doing it anyway.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Encoding::{member} expected a `string`, got tag {}",
            args[0].tag_byte()
        ))
    })
}

/// One [`CHARSET`] case, as the two things a conversion needs from it.
#[derive(Clone, Copy, Debug)]
struct Charset {
    /// The case exactly as source writes it (`Utf8`), for a throw message —
    /// a caller reads `Core\Charset::Utf8`, not an integer.
    case: &'static str,
    /// How its octets are made.
    scheme: Scheme,
}

/// The `Core\Charset` case in slot 1, or the `FATAL` a value that is no case
/// is.
///
/// `member` is the **fully-qualified** spelling — `Core\Encoding::decodeText`,
/// or `Core\IO::readText` for [`decode_argument`]'s caller in another class,
/// which is why the prefix is not written into the message here.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for [`bytes_of`]'s reason: the
/// checker has already placed the argument and compiled code wrote the
/// integer, so anything else here is a runtime-contract violation rather than
/// something a program can cause.
fn charset_of(args: &[Value], member: &str) -> Result<Charset, Fault> {
    // One index into both tables, which is what makes their agreement worth
    // pinning: a case's integer *is* its row in `SCHEMES`.
    args[1]
        .as_int()
        .and_then(|value| usize::try_from(value).ok())
        .and_then(|index| Some((CHARSET.cases.get(index)?, SCHEMES.get(index)?)))
        .map(|((case, _), scheme)| Charset {
            case,
            scheme: *scheme,
        })
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `Core\\Charset` case, got tag {} value {:?}",
                args[1].tag_byte(),
                args[1].as_int()
            ))
        })
}

/// The `Core\Charset` case in slot 1, applied to `raw` — the conversion whole,
/// for a member of another class.
///
/// The whole of it rather than the lookup alone, because both messages a caller
/// can meet belong beside the tables they name: a member elsewhere that did its
/// own decode would be a second opinion about what "exact" means, which is what
/// [`decode_exact`]'s own doc comment exists to prevent. `Core\IO::readText` is
/// the one caller today.
///
/// # Errors
///
/// [`charset_of`]'s `FATAL` for a slot-1 value that is no case, or the
/// catchable `RuntimeError` naming the offset of the first sequence `charset`
/// cannot read.
pub(crate) fn decode_argument(args: &[Value], member: &str, raw: &[u8]) -> Result<String, Fault> {
    let charset = charset_of(args, member)?;
    decode_exact(charset, raw).map_err(|offset| {
        // The rule first and the member in the tail, so that the stem
        // `conformance_coverage.rs` reads off this site is neither empty nor
        // full of backslashes a Novis literal cannot spell — the playbook's
        // *Writing a test case* bullet owns why both halves of that matter.
        Fault::thrown(format!(
            "a conversion is exact or it throws: {member}() found at offset {offset} a byte \
             sequence that is not `Core\\Charset::{}`, and nothing is replaced with U+FFFD or \
             dropped",
            charset.case
        ))
    })
}

// ============================================================================
// The conversions
// ============================================================================

/// `text` as `charset`'s octets, or the offset and the identity of the first
/// character it cannot spell.
///
/// Total for the four Unicode-complete schemes; checked for every other one,
/// which is R4 rather than `encoding_rs`'s own answer — its `encode` writes an
/// HTML numeric character reference for an unmappable character and reports
/// that in a flag, and a `&#8364;` sitting in what a caller believes is
/// Shift_JIS is exactly the silent substitution `rule:types/conversion` refuses.
fn encode_exact(charset: Charset, text: &str) -> Result<Vec<u8>, (usize, char)> {
    match charset.scheme {
        Scheme::Utf8 => Ok(text.as_bytes().to_vec()),
        Scheme::Utf16 { little_endian } => {
            let mut out = Vec::with_capacity(text.len() * 2);
            for unit in text.encode_utf16() {
                let pair = if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                out.extend_from_slice(&pair);
            }
            Ok(out)
        }
        Scheme::Ascii => narrow(text, 0x7f),
        Scheme::Latin1 => narrow(text, 0xff),
        Scheme::Whatwg(encoding) => {
            let (octets, _, unmappable) = encoding.encode(text);
            if unmappable {
                return Err(first_unmappable(encoding, text));
            }
            Ok(octets.into_owned())
        }
    }
}

/// Every character of `text` as the one octet of the same value, or the first
/// one above `ceiling` — the whole of `Ascii` and `Latin1`, which are each a
/// prefix of the code space rather than a table.
fn narrow(text: &str, ceiling: u8) -> Result<Vec<u8>, (usize, char)> {
    let mut out = Vec::with_capacity(text.len());
    for (offset, ch) in text.char_indices() {
        match u8::try_from(u32::from(ch)) {
            Ok(octet) if octet <= ceiling => out.push(octet),
            _ => return Err((offset, ch)),
        }
    }
    Ok(out)
}

/// The first character of `text` that `encoding` has no spelling for.
///
/// Only ever reached once `encode` has already said one exists, so the linear
/// re-scan is on the throwing path alone: the flag `encoding_rs` answers with
/// says *that* a character was substituted and never *which*, and a message
/// naming neither the character nor its offset is one the caller has to
/// bisect by hand.
fn first_unmappable(encoding: &'static Encoding, text: &str) -> (usize, char) {
    let mut buffer = [0u8; 4];
    for (offset, ch) in text.char_indices() {
        let (_, _, unmappable) = encoding.encode(ch.encode_utf8(&mut buffer));
        if unmappable {
            return (offset, ch);
        }
    }
    // Unreachable: `encode` reported a substitution over the whole string, and
    // mappability is a property of the character rather than of its context.
    (0, text.chars().next().unwrap_or('\u{0}'))
}

/// `raw` as text under `charset`, or the byte offset of the first sequence it
/// cannot read.
///
/// Total only for `Latin1`, where every octet is a code point. Everything else
/// is checked, and `decode_to_string_without_replacement` is what makes that
/// true of the delegated schemes — `Encoding::decode` would answer U+FFFD and
/// a flag most callers drop.
fn decode_exact(charset: Charset, raw: &[u8]) -> Result<String, usize> {
    match charset.scheme {
        Scheme::Utf8 => std::str::from_utf8(raw)
            .map(str::to_owned)
            .map_err(|invalid| invalid.valid_up_to()),
        Scheme::Utf16 { little_endian } => decode_utf16(raw, little_endian),
        Scheme::Ascii => match raw.iter().position(|octet| !octet.is_ascii()) {
            Some(offset) => Err(offset),
            None => Ok(raw.iter().map(|&octet| char::from(octet)).collect()),
        },
        Scheme::Latin1 => Ok(raw.iter().map(|&octet| char::from(octet)).collect()),
        Scheme::Whatwg(encoding) => decode_whatwg(encoding, raw),
    }
}

/// `raw` as UTF-16 in the stated order, or the offset of the first code unit
/// pair that is not a character — an unpaired surrogate, or a trailing odd
/// byte.
fn decode_utf16(raw: &[u8], little_endian: bool) -> Result<String, usize> {
    if !raw.len().is_multiple_of(2) {
        return Err(raw.len() - 1);
    }
    let units = raw.chunks_exact(2).map(|pair| {
        let pair = [pair[0], pair[1]];
        if little_endian {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        }
    });
    let mut out = String::with_capacity(raw.len() / 2);
    // Counted in *units consumed*, not in characters produced: a surrogate
    // pair is one character and two units, so enumerating the decoder's own
    // output would report every offset after the first pair too low.
    let mut consumed = 0usize;
    for unit in char::decode_utf16(units) {
        match unit {
            Ok(ch) => {
                out.push(ch);
                consumed += ch.len_utf16();
            }
            Err(_) => return Err(consumed * 2),
        }
    }
    Ok(out)
}

/// `raw` under the standard's own table for `encoding`, or the offset of the
/// first malformed sequence.
fn decode_whatwg(encoding: &'static Encoding, raw: &[u8]) -> Result<String, usize> {
    let mut decoder = encoding.new_decoder_without_bom_handling();
    // The decoder treats the `String`'s *capacity* as its output limit and
    // never reallocates, so this is the size that makes `OutputFull`
    // unreachable. It is `None` only when `raw.len()` is within a factor of
    // three of `usize::MAX`, which no `NvsStr` can be.
    let capacity = decoder
        .max_utf8_buffer_length_without_replacement(raw.len())
        .unwrap_or(0);
    let mut out = String::with_capacity(capacity);
    let (result, read) = decoder.decode_to_string_without_replacement(raw, &mut out, true);
    match result {
        DecoderResult::InputEmpty => Ok(out),
        // `read` counts the bytes consumed up to and past the bad sequence;
        // subtracting both halves of the report leaves where it started.
        DecoderResult::Malformed(length, after) => {
            Err(read.saturating_sub(usize::from(length) + usize::from(after)))
        }
        DecoderResult::OutputFull => Err(read),
    }
}

/// The nibble `digit` spells, in either case, or `None` for anything else.
const fn nibble(digit: u8) -> Option<u8> {
    Some(match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        b'A'..=b'F' => digit - b'A' + 10,
        _ => return None,
    })
}

/// How much of a rejected operand a throw quotes.
///
/// The operand of a failed `fromHex` is text that arrived from somewhere, so
/// it is the one value here whose size a caller chooses. A message reaches a
/// log, and quoting it whole would let a request pick how many bytes that log
/// gains — `rule:security/sink-predicate`
/// is the wider rule.
const SHOWN_CHARS: usize = 32;

/// `text` as it may be quoted back inside a throw message: the first
/// [`SHOWN_CHARS`] characters, with an ellipsis where anything was dropped.
fn shown(text: &str) -> String {
    let mut out: String = text.chars().take(SHOWN_CHARS).collect();
    if text.chars().nth(SHOWN_CHARS).is_some() {
        out.push('…');
    }
    out
}

/// Why `error` refused the text, in the same register [`nvs_core_encoding_from_hex`]
/// uses: what is wrong, and what the member wanted instead.
///
/// `base64`'s own `Display` names a byte value and an offset, which is precise
/// and unreadable; a `Core` throw is read by the person who wrote the call, so
/// it says which rule was broken. `variant` is `"base64"` or `"base64url"` —
/// the two decoders differ only in alphabet and padding, and those are exactly
/// the two things a caller confuses.
fn why_not_base64(error: &base64::DecodeError, variant: &str) -> String {
    match *error {
        base64::DecodeError::InvalidByte(offset, byte) => format!(
            "the byte {byte:#04x} at offset {offset} is not a {variant} character, and \
             whitespace, a newline and a misplaced `=` are each that byte rather than \
             something skipped"
        ),
        base64::DecodeError::InvalidLength(len) => format!(
            "it measures {len} symbol{}, and a base64 group is 2, 3 or 4 of them — this one \
             is truncated",
            if len == 1 { "" } else { "s" }
        ),
        base64::DecodeError::InvalidLastSymbol(offset, byte) => format!(
            "the final symbol {byte:#04x} at offset {offset} carries bits no octet reads, so \
             it is one of several spellings of the same value and not the canonical one \
             (RFC 4648 § 3.5)"
        ),
        base64::DecodeError::InvalidPadding => match variant {
            "base64url" => "it is padded, and the URL-safe form is written without `=`".to_owned(),
            _ => "its `=` padding is missing or malformed, and the standard form is written \
                  with it"
                .to_owned(),
        },
    }
}

/// Why `error` refused the text, in [`why_not_base64`]'s register.
fn why_not_base32(error: &data_encoding::DecodeError) -> String {
    let at = error.position;
    match error.kind {
        data_encoding::DecodeKind::Symbol => format!(
            "the byte at offset {at} is not one of `A`-`Z` or `2`-`7` — the digits `0`, `1` and \
             `8` are not in this alphabet, and a space grouping a secret for a reader is a byte \
             like any other"
        ),
        data_encoding::DecodeKind::Trailing => format!(
            "the symbol at offset {at} carries bits no octet reads, so it is one of several \
             spellings of the same value and not the canonical one (RFC 4648 § 3.5)"
        ),
        data_encoding::DecodeKind::Padding => format!(
            "the `=` padding at offset {at} is not what a base32 group takes — write it in full \
             or leave it off entirely"
        ),
        // `at` is where the short group *starts*, not where the text ends:
        // `data_encoding` reports the first symbol of the block it could not
        // fill, and for the padded engine that block begins at a multiple of
        // eight however long the text is. A sentence saying the text ends
        // there is false of every input whose truncated group is not its
        // first, which is most of them.
        data_encoding::DecodeKind::Length => format!(
            "the group starting at offset {at} has fewer than the 8 symbols a base32 group \
             takes, so the text is truncated"
        ),
    }
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::encodeText(string $s, Charset $charset): bytes` —
    /// replacing `iconv`, `mb_convert_encoding` and `utf8_encode`, with none
    /// of the three's substitution modes.
    ///
    /// Throws naming the first character the charset cannot spell, rather
    /// than writing `?`, `&#NNNN;` or a transliteration for it. `iconv`'s
    /// `//IGNORE` and `//TRANSLIT` have no equivalent here and that is the
    /// point (`rule:types/conversion`):
    /// a caller who genuinely wants a lossy spelling writes the replacement
    /// they want, in their own text, where a reader can see it.
    fn nvs_core_encoding_encode_text(_ctx, args: [2]) {
        let text = text_of(args, "encodeText")?;
        let charset = charset_of(args, "Core\\Encoding::encodeText")?;
        let raw = encode_exact(charset, text).map_err(|(offset, ch)| {
            Fault::thrown(format!(
                "Core\\Encoding::encodeText(): `Core\\Charset::{}` has no spelling for {:?} \
                 (U+{:04X}) at offset {offset} of \"{}\" — it is exact or it throws, and there \
                 is no transliterating or ignoring mode",
                charset.case,
                ch,
                u32::from(ch),
                shown(text)
            ))
        })?;
        Ok(Value::bytes(NvsStr::new(&raw)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::decodeText(bytes $b, Charset $charset): string` —
    /// replacing `iconv`, `mb_convert_encoding` and `utf8_decode`.
    ///
    /// Throws naming the offset of the first sequence the charset cannot
    /// read, rather than answering a string with U+FFFD in it. A replacement
    /// character is not an error a caller can notice later — it compares
    /// unequal, hashes differently and round-trips to a different value — so
    /// [`decode_exact`] runs the decoder without replacement.
    ///
    /// `Core\Encoding::isValidText` is the same question without the throw,
    /// for a caller who has somewhere to put a `false`.
    fn nvs_core_encoding_decode_text(_ctx, args: [2]) {
        let raw = bytes_of(args, "decodeText")?;
        let text = decode_argument(args, "Core\\Encoding::decodeText", raw)?;
        Ok(Value::str(NvsStr::new(text.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::isValidText(bytes $b, Charset $charset): bool` —
    /// replacing `mb_check_encoding`.
    ///
    /// Exactly `decodeText`'s question, answered without the throw: `true`
    /// when every sequence in `$b` is one the charset reads. `Charset::Latin1`
    /// answers `true` for every input, because ISO-8859-1 gives all 256 octets
    /// a meaning — that is a property of the encoding, not a hole here.
    ///
    /// The decode is performed and discarded rather than a separate validator
    /// being written: two implementations of one question is how they come to
    /// disagree, and the cost is one allocation on a member a caller reaches
    /// once per input.
    fn nvs_core_encoding_is_valid_text(_ctx, args: [2]) {
        let raw = bytes_of(args, "isValidText")?;
        let charset = charset_of(args, "Core\\Encoding::isValidText")?;
        Ok(Value::bool(decode_exact(charset, raw).is_ok()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::toBase64(bytes $b): string` — replacing `base64_encode`.
    ///
    /// RFC 4648 § 4's alphabet, **padded**, which is byte-for-byte what
    /// `base64_encode` answers. Total: every octet sequence has a spelling.
    fn nvs_core_encoding_to_base64(_ctx, args: [1]) {
        let raw = bytes_of(args, "toBase64")?;
        Ok(Value::str(NvsStr::new(STANDARD.encode(raw).as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::fromBase64(string $s): bytes` — replacing
    /// `base64_decode`, and throwing where that function's default mode
    /// silently discarded whatever it did not recognise.
    ///
    /// Strict on all three counts the module doc lists: § 4's alphabet only,
    /// padding required and canonical, and no unread bits in the last symbol.
    /// A URL-safe operand is refused here rather than accepted as a courtesy —
    /// [`nvs_core_encoding_from_base64_url`] is the member that reads it, and a
    /// decoder that takes both cannot tell a caller which one they meant.
    fn nvs_core_encoding_from_base64(_ctx, args: [1]) {
        let text = text_of(args, "fromBase64")?;
        let raw = STANDARD.decode(text).map_err(|error| {
            Fault::thrown(format!(
                "Core\\Encoding::fromBase64(): \"{}\" is not base64 — {}",
                shown(text),
                why_not_base64(&error, "base64")
            ))
        })?;
        Ok(Value::bytes(NvsStr::new(&raw)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::toBase64Url(bytes $b): string` — replacing the
    /// `rtrim(strtr(base64_encode($b), "+/", "-_"), "=")` idiom, which is what
    /// the spec row names because it is what every PHP codebase writes.
    ///
    /// RFC 4648 § 5's alphabet, **unpadded**: that idiom's `rtrim` is not an
    /// optional flourish, it is what JWT, `Core\Uri` and every other consumer
    /// of a URL-safe spelling expects, and `=` is percent-encoded in a query
    /// string anyway.
    fn nvs_core_encoding_to_base64_url(_ctx, args: [1]) {
        let raw = bytes_of(args, "toBase64Url")?;
        Ok(Value::str(NvsStr::new(URL_SAFE_NO_PAD.encode(raw).as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::fromBase64Url(string $s): bytes` — the other half of
    /// [`nvs_core_encoding_to_base64_url`], strict in the same three ways and
    /// refusing padding rather than tolerating it.
    fn nvs_core_encoding_from_base64_url(_ctx, args: [1]) {
        let text = text_of(args, "fromBase64Url")?;
        let raw = URL_SAFE_NO_PAD.decode(text).map_err(|error| {
            Fault::thrown(format!(
                "Core\\Encoding::fromBase64Url(): \"{}\" is not URL-safe base64 — {}",
                shown(text),
                why_not_base64(&error, "base64url")
            ))
        })?;
        Ok(Value::bytes(NvsStr::new(&raw)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::toBase32(bytes $b): string` — replacing nothing in PHP,
    /// and needed by TOTP
    /// (`rule:security/protocol-roster`).
    ///
    /// RFC 4648 § 6's alphabet, **upper case and unpadded** — the form an
    /// `otpauth:` secret is written in. Total: every octet sequence has a
    /// spelling.
    fn nvs_core_encoding_to_base32(_ctx, args: [1]) {
        let raw = bytes_of(args, "toBase32")?;
        Ok(Value::str(NvsStr::new(BASE32_NOPAD.encode(raw).as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::fromBase32(string $s): bytes` — the other half of
    /// [`nvs_core_encoding_to_base32`], reading either case and padding that is
    /// either canonical or absent.
    ///
    /// The module doc's *base32 is one pair* section owns why that is not the
    /// leniency the base64 decoders refuse: neither variance can change which
    /// octets come out. What is still refused is a symbol outside the
    /// alphabet — including a space, which is how a user-facing secret is
    /// grouped — a truncated final group, non-canonical trailing bits, and
    /// padding that is present but wrong.
    fn nvs_core_encoding_from_base32(_ctx, args: [1]) {
        let text = text_of(args, "fromBase32")?;
        // Only `a`-`z` move; every other byte, valid or not, reaches the
        // decoder exactly as written, so an offset in the error still indexes
        // the caller's own text.
        let folded = text.to_ascii_uppercase();
        let engine = if folded.ends_with('=') { &BASE32 } else { &BASE32_NOPAD };
        let raw = engine.decode(folded.as_bytes()).map_err(|error| {
            Fault::thrown(format!(
                "Core\\Encoding::fromBase32(): \"{}\" is not base32 — {}",
                shown(text),
                why_not_base32(&error)
            ))
        })?;
        Ok(Value::bytes(NvsStr::new(&raw)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::toHex(bytes $b): string` — replacing `bin2hex` and the
    /// `unpack("H*", …)` idiom written where `bin2hex` was forgotten.
    ///
    /// **Lowercase**, which is `bin2hex`'s own answer and the one every
    /// protocol that names a case names. [`nvs_core_encoding_from_hex`] reads
    /// either case back, so nothing round-trips differently for it.
    ///
    /// Total: every octet has a spelling, so there is nothing to reject.
    fn nvs_core_encoding_to_hex(_ctx, args: [1]) {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";

        let raw = bytes_of(args, "toHex")?;
        let mut out = Vec::with_capacity(raw.len() * 2);
        for octet in raw {
            out.push(DIGITS[usize::from(octet >> 4)]);
            out.push(DIGITS[usize::from(octet & 0x0f)]);
        }
        Ok(Value::str(NvsStr::new(&out)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Encoding::fromHex(string $s): bytes` — replacing `hex2bin`, and
    /// throwing where that function warned and answered `false`.
    ///
    /// **Either case, and no separators.** `"ff"`, `"FF"` and `"fF"` are one
    /// octet; a space, a `0x` prefix or a colon between pairs is a throw, not
    /// a value silently skipped. An odd number of digits is a throw for the
    /// same reason: `hex2bin("abc")` guessing which nibble the caller meant is
    /// exactly the substitution `rule:types/conversion` removes from the language.
    fn nvs_core_encoding_from_hex(_ctx, args: [1]) {
        let text = text_of(args, "fromHex")?;
        let digits = text.as_bytes();
        let refused = |why: &str| {
            Fault::thrown(format!(
                "Core\\Encoding::fromHex(): \"{}\" is not hexadecimal — {why}",
                shown(text)
            ))
        };
        if digits.len() % 2 != 0 {
            return Err(refused("it has an odd number of digits, and an octet takes two"));
        }
        let mut out = Vec::with_capacity(digits.len() / 2);
        for pair in digits.chunks_exact(2) {
            let (Some(high), Some(low)) = (nibble(pair[0]), nibble(pair[1])) else {
                return Err(refused(
                    "only the digits `0`-`9`, `a`-`f` and `A`-`F` spell one, with nothing between \
                     the pairs",
                ));
            };
            out.push((high << 4) | low);
        }
        Ok(Value::bytes(NvsStr::new(&out)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nibble_reads_in_either_case_and_nothing_else() {
        assert_eq!(nibble(b'0'), Some(0));
        assert_eq!(nibble(b'9'), Some(9));
        assert_eq!(nibble(b'a'), Some(10));
        assert_eq!(nibble(b'F'), Some(15));
        assert_eq!(nibble(b'g'), None);
        assert_eq!(nibble(b' '), None);
        assert_eq!(nibble(b'-'), None);
    }

    /// The two engines are constants imported from `base64`, so what this
    /// module promises about padding and canonicality is a property of *which*
    /// constant rather than of any code here. Pin it, so a version bump that
    /// changed a default would fail here rather than in a caller's round trip.
    #[test]
    fn the_two_engines_differ_in_padding_and_refuse_a_non_canonical_tail() {
        assert_eq!(STANDARD.encode([0xff]), "/w==");
        assert_eq!(URL_SAFE_NO_PAD.encode([0xff]), "_w");

        // Each refuses the other's padding rule.
        assert!(STANDARD.decode("/w").is_err());
        assert!(URL_SAFE_NO_PAD.decode("_w==").is_err());

        // `/x` and `/w` name the same octet; only the one with zero unread
        // bits decodes (RFC 4648 § 3.5).
        assert_eq!(STANDARD.decode("/w==").unwrap(), [0xff]);
        assert!(matches!(
            STANDARD.decode("/x=="),
            Err(base64::DecodeError::InvalidLastSymbol(..))
        ));
    }

    /// The same pin as the base64 engines, for the three rules the module doc
    /// says `fromBase32` keeps while it folds case and padding.
    #[test]
    fn base32_is_unpadded_on_the_way_out_and_canonical_on_the_way_in() {
        assert_eq!(BASE32_NOPAD.encode(b"nvs"), "NZ3HG");
        assert_eq!(BASE32.encode(b"nvs"), "NZ3HG===");

        // A trailing bit no octet reads, and a symbol outside the alphabet.
        // Three octets are 24 bits in five symbols, so the last symbol carries
        // four data bits and one that must be zero: `G` is `00110`, and the
        // `H` below is the same group with that pad bit set.
        assert!(BASE32_NOPAD.decode(b"NZ3HH").is_err());
        assert!(BASE32_NOPAD.decode(b"NZ3H1").is_err());

        // A group cut short.
        assert!(BASE32_NOPAD.decode(b"NZ3HGA").is_err());
    }

    /// The `Core\Charset` case that `name` spells, for a test that wants to
    /// name one without depending on its integer.
    fn charset(name: &str) -> Charset {
        let (index, (case, _)) = CHARSET
            .cases
            .iter()
            .enumerate()
            .find(|(_, (case, _))| *case == name)
            .expect("a declared case");
        Charset {
            case,
            scheme: SCHEMES[index],
        }
    }

    /// `Core\Charset::Big5` from `Big5`, `Iso88598I` from `ISO-8859-8-I` — the
    /// mechanical PascalCase of a WHATWG name, which is what "named in Novis
    /// casing" in spec § 7 means and what `nvs_syntax::casing` would demand of
    /// the same name written in source.
    fn pascal(whatwg: &str) -> String {
        whatwg
            .split(['-', '_'])
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        first.to_ascii_uppercase().to_string()
                            + &chars.as_str().to_ascii_lowercase()
                    }
                    None => String::new(),
                }
            })
            .collect()
    }

    /// A case's integer indexes [`SCHEMES`], so the two tables are one roster
    /// written twice — see [`CHARSET`] for why they cannot be one table.
    // covers: Core\Charset
    #[test]
    fn the_case_table_and_the_scheme_table_are_one_roster() {
        assert_eq!(CHARSET.cases.len(), SCHEMES.len());
        for (index, (case, value)) in CHARSET.cases.iter().enumerate() {
            assert_eq!(
                i64::try_from(index).expect("a roster this size"),
                *value,
                "`Core\\Charset::{case}` is at row {index} and carries {value}"
            );
        }
        // One past the end, and a negative, are both the FATAL rather than a
        // panic on the index.
        assert!(charset_of(&[Value::int(0), Value::int(41)], "test").is_err());
        assert!(charset_of(&[Value::int(0), Value::int(-1)], "test").is_err());
        assert_eq!(
            charset_of(&[Value::int(0), Value::int(0)], "test")
                .unwrap()
                .case,
            "Utf8"
        );
    }

    /// Every delegated case is named by `encoding_rs`'s own `name()`, which
    /// is the standard's label — so the roster is the index rather than this
    /// file's transcription of it, which is what spec § 7 requires. A case
    /// added with a mistyped name fails here rather than at a call site.
    #[test]
    fn the_delegated_roster_is_the_standards_own() {
        let mut delegated = 0;
        for ((case, value), scheme) in CHARSET.cases.iter().zip(SCHEMES.iter()) {
            if let Scheme::Whatwg(encoding) = scheme {
                assert_eq!(pascal(encoding.name()), *case, "case {value}");
                delegated += 1;
            }
        }
        // The index, less `replacement` and less the five the module doc says
        // are answered here.
        assert_eq!(delegated, 36);
    }

    /// The standard makes UTF-16 and `replacement` **decode-only**, and
    /// `Encoding::encode` quietly answers UTF-8 for them rather than failing.
    /// Every delegated case must therefore be one whose `output_encoding` is
    /// itself — moving `Utf16Le` onto the delegated side would otherwise
    /// compile, pass a decode test, and encode to UTF-8.
    #[test]
    fn every_delegated_encoding_answers_in_its_own_encoding() {
        for ((case, _), scheme) in CHARSET.cases.iter().zip(SCHEMES.iter()) {
            if let Scheme::Whatwg(encoding) = scheme {
                assert_eq!(encoding.output_encoding().name(), encoding.name(), "{case}");
            }
        }
    }

    /// The three labels the standard folds into `windows-1252` and Novis does
    /// not — the module doc's first departure, and the one a reader is most
    /// likely to think is a bug.
    #[test]
    fn ascii_latin1_and_windows_1252_are_three_different_answers() {
        // `0x80` is U+20AC in windows-1252, U+0080 in ISO-8859-1, and not
        // ASCII at all.
        assert_eq!(
            decode_exact(charset("Windows1252"), &[0x80]).unwrap(),
            "\u{20ac}"
        );
        assert_eq!(decode_exact(charset("Latin1"), &[0x80]).unwrap(), "\u{80}");
        assert_eq!(decode_exact(charset("Ascii"), &[0x80]), Err(0));

        // And the other way: the euro sign has a windows-1252 spelling and no
        // ISO-8859-1 one.
        assert_eq!(
            encode_exact(charset("Windows1252"), "\u{20ac}").unwrap(),
            [0x80]
        );
        assert_eq!(
            encode_exact(charset("Latin1"), "\u{20ac}"),
            Err((0, '\u{20ac}'))
        );
        assert_eq!(
            encode_exact(charset("Ascii"), "e\u{9}\u{7f}").unwrap(),
            b"e\t\x7f"
        );
    }

    /// UTF-16 is encoded here because the standard has no encoder for it, so
    /// both directions and both orders are pinned rather than inherited.
    #[test]
    fn utf16_round_trips_in_both_orders_and_refuses_a_lone_surrogate() {
        // U+1F600, which is a surrogate pair, after one BMP character.
        let text = "a\u{1f600}";
        let little = encode_exact(charset("Utf16Le"), text).unwrap();
        let big = encode_exact(charset("Utf16Be"), text).unwrap();
        assert_eq!(little, [0x61, 0x00, 0x3d, 0xd8, 0x00, 0xde]);
        assert_eq!(big, [0x00, 0x61, 0xd8, 0x3d, 0xde, 0x00]);
        assert_eq!(decode_exact(charset("Utf16Le"), &little).unwrap(), text);
        assert_eq!(decode_exact(charset("Utf16Be"), &big).unwrap(), text);

        // A high surrogate with nothing after it, and an odd trailing byte.
        // The offset counts units *consumed*, so the `a` before it is two.
        assert_eq!(
            decode_exact(charset("Utf16Le"), &[0x61, 0x00, 0x3d, 0xd8]),
            Err(2)
        );
        assert_eq!(
            decode_exact(charset("Utf16Le"), &[0x61, 0x00, 0x00]),
            Err(2)
        );
    }

    /// The delegated half, over one legacy encoding whose tables nothing here
    /// owns: a round trip, an unmappable character on the way out, and a
    /// malformed sequence on the way in.
    #[test]
    fn a_delegated_charset_converts_both_ways_and_refuses_what_it_cannot_say() {
        let jis = charset("ShiftJis");
        let text = "\u{3042}\u{3044}"; // あい
        let raw = encode_exact(jis, text).unwrap();
        assert_eq!(raw, [0x82, 0xa0, 0x82, 0xa2]);
        assert_eq!(decode_exact(jis, &raw).unwrap(), text);
        assert!(decode_exact(jis, &raw).is_ok());

        // Shift_JIS has no euro sign; the message names the character, so the
        // offset is the one in the *text*, past the two-byte あ.
        assert_eq!(encode_exact(jis, "\u{3042}\u{20ac}"), Err((3, '\u{20ac}')));

        // A lead byte with no trail byte after it.
        assert_eq!(decode_exact(jis, &[0x82, 0xa0, 0x82]), Err(2));
    }

    /// `fromBase32` reached the way a program reaches it: the three things a
    /// reader may change about the text without changing the octets, and the
    /// four it may not.
    ///
    /// The refusals are counted rather than read off one line, because what
    /// matters here is that **one value has one spelling** — a decoder that
    /// accepted a second `MZXW7` for `MZXW6`'s octets would let one secret be
    /// held twice under two names, and it would still look right on every row
    /// that decodes. The truncation offset is asserted against the group start
    /// rather than the text length, which is what that message reports.
    // covers: Core\Encoding::fromBase32
    #[test]
    fn one_value_has_one_base32_spelling() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let read = |ctx: &mut nvs_runtime::Ctx, text: &str| {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let answered = nvs_runtime::call(nvs_core_encoding_from_base32, ctx, &[subject]);
            let octets = answered.as_ref().ok().map(|value| {
                let owned = value.as_bytes().expect("a `bytes` answer").to_vec();
                #[expect(
                    unsafe_code,
                    reason = "the answer's one reference is this closure's, \
                              and its octets are copied out before it goes"
                )]
                unsafe {
                    value.release();
                }
                owned
            });
            #[expect(
                unsafe_code,
                reason = "this closure owns the one reference it built, and \
                          the member borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
            octets
        };

        // Case and canonical padding are the reader's to change, and the empty
        // string is the empty buffer.
        for text in ["MZXW6", "mzxw6", "MzXw6", "MZXW6==="] {
            assert_eq!(
                read(&mut ctx, text).as_deref(),
                Some(b"foo".as_slice()),
                "{text}"
            );
        }
        assert_eq!(read(&mut ctx, "").as_deref(), Some(b"".as_slice()));

        // A second spelling of the same octets, a symbol outside the alphabet,
        // padding that is not a group's worth, and a short final group.
        for text in ["MZXW7", "MZXW0YTB", "MZXW6=", "MZXW6YT8"] {
            assert!(read(&mut ctx, text).is_none(), "{text}");
            assert!(ctx.take_pending().is_some(), "{text} refused uncatchably");
        }

        // The truncation message names where the short group *starts*. The
        // text below is 17 symbols, its first two groups are whole, and the
        // third begins at 16 — a message reporting the length would say 17.
        assert!(read(&mut ctx, "MZXW6YTBMZXW6YTBM").is_none());
        let message = ctx.take_pending().expect("a catchable refusal");
        assert!(
            message.contains("group starting at offset 16"),
            "the truncation offset is the group start, not the length: {message}"
        );
    }

    /// `fromBase64` reached the way a program reaches it: the one form it
    /// reads, and the four near misses a caller arrives with.
    ///
    /// Each refusal is asserted for the reason it names rather than only for
    /// happening, because the standard and URL-safe forms differ in exactly
    /// two things — the two symbols outside the letters and digits, and the
    /// padding — and those are the two a caller confuses. A decoder that said
    /// "not base64" to both sends the caller to check the wrong half. The
    /// length reason is reachable here only for a symbol left over after whole
    /// groups, since every other short text is missing its padding first.
    // covers: Core\Encoding::fromBase64
    #[test]
    fn base64_reads_one_alphabet_and_one_padding() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let read = |ctx: &mut nvs_runtime::Ctx, text: &str| {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let answered = nvs_runtime::call(nvs_core_encoding_from_base64, ctx, &[subject]);
            let octets = answered.as_ref().ok().map(|value| {
                let owned = value.as_bytes().expect("a `bytes` answer").to_vec();
                #[expect(
                    unsafe_code,
                    reason = "the answer's one reference is this closure's, \
                              and its octets are copied out before it goes"
                )]
                unsafe {
                    value.release();
                }
                owned
            });
            #[expect(
                unsafe_code,
                reason = "this closure owns the one reference it built, and \
                          the member borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
            octets
        };

        // A whole group, a short one padded to the group, and the empty text.
        assert_eq!(read(&mut ctx, "YWJj").as_deref(), Some(b"abc".as_slice()));
        assert_eq!(
            read(&mut ctx, "YWJjZA==").as_deref(),
            Some(b"abcd".as_slice())
        );
        assert_eq!(read(&mut ctx, "").as_deref(), Some(b"".as_slice()));

        // A second spelling of the same octets, the URL-safe alphabet, padding
        // left off, and the whitespace an older reader skipped.
        for text in ["YWJjZB==", "aGVsbG8-d29ybGQ_", "YWJ", "YWJj ZA=="] {
            assert!(read(&mut ctx, text).is_none(), "{text}");
            assert!(ctx.take_pending().is_some(), "{text} refused uncatchably");
        }

        // The URL-safe text is refused for the byte at offset 7 rather than
        // for its padding, and the unpadded one for its padding rather than
        // its alphabet.
        assert!(read(&mut ctx, "aGVsbG8-d29ybGQ_").is_none());
        let alphabet = ctx.take_pending().expect("a catchable refusal");
        assert!(
            alphabet.contains("0x2d at offset 7"),
            "the URL-safe symbol is named where it sits: {alphabet}"
        );
        assert!(read(&mut ctx, "YWJ").is_none());
        let padding = ctx.take_pending().expect("a catchable refusal");
        assert!(
            padding.contains("`=` padding is missing"),
            "an unpadded group is a padding reason: {padding}"
        );

        // A symbol left over after whole groups is the length reason, and one
        // of them is counted as one symbol.
        assert!(read(&mut ctx, "a").is_none());
        let length = ctx.take_pending().expect("a catchable refusal");
        assert!(
            length.contains("it measures 1 symbol,"),
            "one leftover symbol is counted in the singular: {length}"
        );
    }

    /// `fromBase64Url` reached the way a program reaches it: the alphabet a
    /// web address carries, and the standard form's three characters arriving
    /// where they do not belong.
    ///
    /// The unpadded form is where a decoder is tempted to be helpful, since
    /// `=` is harmless to strip and `+` is one table entry away from `-`. It
    /// is not helpful: a token compared as text and used as bytes must have
    /// one spelling, so each of those is refused and the message names which
    /// of the two differences it was.
    // covers: Core\Encoding::fromBase64Url
    #[test]
    fn base64url_reads_the_web_alphabet_and_refuses_padding() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let read = |ctx: &mut nvs_runtime::Ctx, text: &str| {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let answered = nvs_runtime::call(nvs_core_encoding_from_base64_url, ctx, &[subject]);
            let octets = answered.as_ref().ok().map(|value| {
                let owned = value.as_bytes().expect("a `bytes` answer").to_vec();
                #[expect(
                    unsafe_code,
                    reason = "the answer's one reference is this closure's, \
                              and its octets are copied out before it goes"
                )]
                unsafe {
                    value.release();
                }
                owned
            });
            #[expect(
                unsafe_code,
                reason = "this closure owns the one reference it built, and \
                          the member borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
            octets
        };

        // Its own two symbols, a whole group, a short group that simply stops
        // where it stops, and the empty text.
        assert_eq!(
            read(&mut ctx, "aGVsbG8-d29ybGQ_").as_deref(),
            Some(b"hello>world?".as_slice())
        );
        assert_eq!(read(&mut ctx, "YWJj").as_deref(), Some(b"abc".as_slice()));
        assert_eq!(read(&mut ctx, "YWI").as_deref(), Some(b"ab".as_slice()));
        assert_eq!(read(&mut ctx, "").as_deref(), Some(b"".as_slice()));

        // A second spelling of the same octets, the standard alphabet, the
        // padding that form writes, and whitespace.
        for text in ["YWJjZB", "aGVsbG8+d29ybGQ/", "YWJjZA==", "YWJj ZA"] {
            assert!(read(&mut ctx, text).is_none(), "{text}");
            assert!(ctx.take_pending().is_some(), "{text} refused uncatchably");
        }

        // Which of the two differences: the padded text is refused for its
        // padding rather than for its alphabet, and the `+` for the byte at
        // offset 7 rather than for the length its group then has.
        assert!(read(&mut ctx, "YWJjZA==").is_none());
        let padded = ctx.take_pending().expect("a catchable refusal");
        assert!(
            padded.contains("it is padded"),
            "padding is named as the difference: {padded}"
        );
        assert!(read(&mut ctx, "aGVsbG8+d29ybGQ/").is_none());
        let alphabet = ctx.take_pending().expect("a catchable refusal");
        assert!(
            alphabet.contains("0x2b at offset 7"),
            "the standard symbol is named where it sits: {alphabet}"
        );
    }

    /// `fromHex` reached the way a program reaches it: every octet has exactly
    /// two spellings, and nothing else is one.
    ///
    /// The round trip is swept over all 256 values rather than read off a
    /// handful, because case folding is the one thing this decoder does and a
    /// table that folds all but one entry still prints plausibly on any line
    /// somebody looks at. The two refusals are asserted for which reason they
    /// name: the digit count is checked before the digits are, so a separator
    /// between the pairs is the odd-count reason when it makes the count odd
    /// and the digit reason when it does not, and a caller told the wrong one
    /// goes looking in the wrong half of their text.
    // covers: Core\Encoding::fromHex
    #[test]
    fn hex_reads_two_spellings_of_every_octet_and_nothing_else() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let read = |ctx: &mut nvs_runtime::Ctx, text: &str| {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let answered = nvs_runtime::call(nvs_core_encoding_from_hex, ctx, &[subject]);
            let octets = answered.as_ref().ok().map(|value| {
                let owned = value.as_bytes().expect("a `bytes` answer").to_vec();
                #[expect(
                    unsafe_code,
                    reason = "the answer's one reference is this closure's, \
                              and its octets are copied out before it goes"
                )]
                unsafe {
                    value.release();
                }
                owned
            });
            #[expect(
                unsafe_code,
                reason = "this closure owns the one reference it built, and \
                          the member borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
            octets
        };

        // Every octet, in both of its spellings, and the empty text.
        for octet in 0..=u8::MAX {
            let lower = format!("{octet:02x}");
            let upper = lower.to_ascii_uppercase();
            assert_eq!(read(&mut ctx, &lower).as_deref(), Some([octet].as_slice()));
            assert_eq!(read(&mut ctx, &upper).as_deref(), Some([octet].as_slice()));
        }
        assert_eq!(read(&mut ctx, "").as_deref(), Some(b"".as_slice()));

        // A count no pair can fill, a prefix, a separator, and a trailing
        // newline.
        for text in ["abc", "0xff", "ab:cd", "ff\n"] {
            assert!(read(&mut ctx, text).is_none(), "{text}");
            assert!(ctx.take_pending().is_some(), "{text} refused uncatchably");
        }

        // `ab:cd` is five characters long, so it is the odd-count reason;
        // `0xff` is four, so its `0x` is read as digits and refused as such.
        assert!(read(&mut ctx, "ab:cd").is_none());
        let odd = ctx.take_pending().expect("a catchable refusal");
        assert!(
            odd.contains("odd number of digits"),
            "the count is checked before the digits: {odd}"
        );
        assert!(read(&mut ctx, "0xff").is_none());
        let digits = ctx.take_pending().expect("a catchable refusal");
        assert!(
            digits.contains("only the digits"),
            "an even count is refused on its digits: {digits}"
        );
    }

    /// `toBase64Url` reached the way a program reaches it. What this member
    /// promises is text a URL carries untouched, so the three characters that
    /// would break that promise are swept for over every byte value rather
    /// than read off one sample: `+` and `/` are reached only by the two
    /// highest sextets, and an alphabet left unswapped spells most buffers
    /// identically to [`nvs_core_encoding_to_base64`]. The unpadded length is
    /// pinned beside them, since dropping the padding is the other half of
    /// the difference and a residue left padded is still URL-safe text.
    // covers: Core\Encoding::toBase64Url
    #[test]
    fn base64_url_writes_no_padding_and_nothing_a_url_rereads() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let spell = |ctx: &mut nvs_runtime::Ctx, octets: &[u8]| {
            let subject = Value::bytes(NvsStr::new(octets));
            let answered = nvs_runtime::call(nvs_core_encoding_to_base64_url, ctx, &[subject])
                .expect("every buffer has a spelling");
            let text = answered.as_text().expect("a `string` answer").to_owned();
            #[expect(
                unsafe_code,
                reason = "this closure owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                answered.release();
                subject.release();
            }
            text
        };

        // The empty buffer, one buffer per residue, and § 5's two symbols
        // where § 4 writes `+` and `/`.
        assert_eq!(spell(&mut ctx, b""), "");
        assert_eq!(spell(&mut ctx, b"abc"), "YWJj");
        assert_eq!(spell(&mut ctx, b"ab"), "YWI");
        assert_eq!(spell(&mut ctx, b"a"), "YQ");
        assert_eq!(spell(&mut ctx, &[0xfb, 0xff]), "-_8");
        assert_eq!(spell(&mut ctx, &[0xff, 0xff, 0xff]), "____");

        // Every length to 64: the unpadded length, nothing a URL rereads, and
        // the round trip. A single leftover character is a length no group
        // arithmetic can produce, so it is asserted as impossible rather than
        // as one more arithmetic identity.
        for length in 0..64_usize {
            let octets: Vec<u8> = (0..length)
                .map(|at| u8::try_from(at * 13 % 256).expect("a residue of 256 fits a `u8`"))
                .collect();
            let spelled = spell(&mut ctx, &octets);
            assert_eq!(spelled.len(), length.div_ceil(3) * 4 - (3 - length % 3) % 3);
            assert_ne!(spelled.len() % 4, 1, "{length} octets gave {spelled}");
            assert!(
                !spelled.contains(['+', '/', '=']),
                "{length} octets gave {spelled}"
            );

            let text = Value::str(NvsStr::new(spelled.as_bytes()));
            let read = nvs_runtime::call(nvs_core_encoding_from_base64_url, &mut ctx, &[text])
                .expect("`toBase64Url`'s answer is text `fromBase64Url` reads");
            assert_eq!(read.as_bytes(), Some(octets.as_slice()), "{length} octets");
            #[expect(
                unsafe_code,
                reason = "this test owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                read.release();
                text.release();
            }
        }
    }

    /// `toBase64` reached the way a program reaches it. The three residues a
    /// group can end on are what this member is written around, so all three
    /// are pinned by length as well as by text: an encoder that pads the
    /// wrong one still spells the aligned case correctly, and every sample a
    /// person picks by hand is likely to be aligned. `+` and `/` are asserted
    /// to be present, because § 4's alphabet is the whole difference from
    /// [`nvs_core_encoding_to_base64_url`] and a table swapped for that one
    /// answers text a URL reader accepts and `fromBase64` does not.
    // covers: Core\Encoding::toBase64
    #[test]
    fn base64_pads_every_residue_and_writes_the_standard_alphabet() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let spell = |ctx: &mut nvs_runtime::Ctx, octets: &[u8]| {
            let subject = Value::bytes(NvsStr::new(octets));
            let answered = nvs_runtime::call(nvs_core_encoding_to_base64, ctx, &[subject])
                .expect("every buffer has a spelling");
            let text = answered.as_text().expect("a `string` answer").to_owned();
            #[expect(
                unsafe_code,
                reason = "this closure owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                answered.release();
                subject.release();
            }
            text
        };

        // The empty buffer, and one buffer per residue: none, one `=` and
        // two, spelled out so the padding is read rather than counted.
        assert_eq!(spell(&mut ctx, b""), "");
        assert_eq!(spell(&mut ctx, b"abc"), "YWJj");
        assert_eq!(spell(&mut ctx, b"ab"), "YWI=");
        assert_eq!(spell(&mut ctx, b"a"), "YQ==");

        // § 4's two symbols, which are the whole difference from the URL-safe
        // alphabet, and the high bits that reach them.
        assert_eq!(spell(&mut ctx, &[0xfb, 0xff]), "+/8=");
        assert_eq!(spell(&mut ctx, &[0xff, 0xff, 0xff]), "////");

        // Every length to 64, against the group arithmetic and `fromBase64`.
        // The length is asserted apart from the round trip because a decoder
        // that tolerates its own encoder's mistake hides both halves.
        for length in 0..64_usize {
            let octets: Vec<u8> = (0..length)
                .map(|at| u8::try_from(at * 11 % 256).expect("a residue of 256 fits a `u8`"))
                .collect();
            let spelled = spell(&mut ctx, &octets);
            assert_eq!(spelled.len(), length.div_ceil(3) * 4, "{length} octets");
            assert_eq!(
                spelled.len() - spelled.trim_end_matches('=').len(),
                (3 - length % 3) % 3,
                "{length} octets padded wrongly: {spelled}"
            );

            let text = Value::str(NvsStr::new(spelled.as_bytes()));
            let read = nvs_runtime::call(nvs_core_encoding_from_base64, &mut ctx, &[text])
                .expect("`toBase64`'s answer is text `fromBase64` reads");
            assert_eq!(read.as_bytes(), Some(octets.as_slice()), "{length} octets");
            #[expect(
                unsafe_code,
                reason = "this test owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                read.release();
                text.release();
            }
        }
    }

    /// `toBase32` reached the way a program reaches it. A group is five
    /// octets, so the five residues a buffer can end on are what this member
    /// is written around: each is pinned by text, from RFC 4648 § 10's
    /// vectors, and every length to 64 by the unpadded arithmetic, since an
    /// encoder that pads, or rounds a short group the wrong way, still spells
    /// every aligned sample right. The alphabet is asserted as a *set* over a
    /// sweep of all 256 values, because § 6's alphabet skips `0`, `1`, `8` and
    /// `9` and a table off by one there answers text that reads plausibly and
    /// that `fromBase32` refuses.
    // covers: Core\Encoding::toBase32
    #[test]
    fn base32_spells_every_residue_unpadded_in_the_upper_case_alphabet() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let spell = |ctx: &mut nvs_runtime::Ctx, octets: &[u8]| {
            let subject = Value::bytes(NvsStr::new(octets));
            let answered = nvs_runtime::call(nvs_core_encoding_to_base32, ctx, &[subject])
                .expect("every buffer has a spelling");
            let text = answered.as_text().expect("a `string` answer").to_owned();
            #[expect(
                unsafe_code,
                reason = "this closure owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                answered.release();
                subject.release();
            }
            text
        };

        // § 10's vectors with the padding taken off: one per residue, and the
        // sixth octet that opens a second group.
        for (octets, spelled) in [
            (b"".as_slice(), ""),
            (b"f", "MY"),
            (b"fo", "MZXQ"),
            (b"foo", "MZXW6"),
            (b"foob", "MZXW6YQ"),
            (b"fooba", "MZXW6YTB"),
            (b"foobar", "MZXW6YTBOI"),
        ] {
            assert_eq!(spell(&mut ctx, octets), spelled);
        }

        // Every octet value in one buffer, and the alphabet read off the
        // answer as a set: exactly § 6's thirty-two symbols, and no `=`.
        let every: Vec<u8> = (0..=255_u8).collect();
        let swept = spell(&mut ctx, &every);
        assert_eq!(
            swept.len(),
            410,
            "2048 bits are 409 whole symbols and one short"
        );
        let mut seen: Vec<char> = swept.chars().collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.into_iter().collect::<String>(),
            "234567ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        );

        // Every length to 64, against the unpadded arithmetic and
        // `fromBase32`, asserted apart for the same reason as base64's: a
        // decoder tolerating its own encoder's mistake hides both halves.
        for length in 0..64_usize {
            let octets: Vec<u8> = (0..length)
                .map(|at| u8::try_from(at * 13 % 256).expect("a residue of 256 fits a `u8`"))
                .collect();
            let spelled = spell(&mut ctx, &octets);
            assert_eq!(spelled.len(), (length * 8).div_ceil(5), "{length} octets");

            let text = Value::str(NvsStr::new(spelled.as_bytes()));
            let read = nvs_runtime::call(nvs_core_encoding_from_base32, &mut ctx, &[text])
                .expect("`toBase32`'s answer is text `fromBase32` reads");
            assert_eq!(read.as_bytes(), Some(octets.as_slice()), "{length} octets");
            #[expect(
                unsafe_code,
                reason = "this test owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                read.release();
                text.release();
            }
        }
    }

    /// `toHex` reached the way a program reaches it: the spelling is swept
    /// over all 256 values rather than read off a handful, because a nibble
    /// table with one wrong entry prints plausibly on every line somebody
    /// looks at and only shows up on the value it misspells. The sweep also
    /// pins the two halves in order — a table indexed high nibble first and
    /// then low is what makes `0x1f` read as `f1` — and the length, because a
    /// member that drops a leading zero answers text `fromHex` cannot read.
    // covers: Core\Encoding::toHex
    #[test]
    fn hex_spells_every_octet_as_two_lowercase_digits() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let spell = |ctx: &mut nvs_runtime::Ctx, octets: &[u8]| {
            let subject = Value::bytes(NvsStr::new(octets));
            let answered = nvs_runtime::call(nvs_core_encoding_to_hex, ctx, &[subject])
                .expect("every octet has a spelling");
            let text = answered.as_text().expect("a `string` answer").to_owned();
            #[expect(
                unsafe_code,
                reason = "this closure owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                answered.release();
                subject.release();
            }
            text
        };

        // Every octet alone, against the same two digits written another way.
        for octet in 0..=u8::MAX {
            let spelled = spell(&mut ctx, &[octet]);
            assert_eq!(spelled, format!("{octet:02x}"), "{octet}");
            assert_eq!(spelled.len(), 2, "{octet}");
            assert!(
                spelled
                    .bytes()
                    .all(|digit| digit.is_ascii_digit() || (b'a'..=b'f').contains(&digit)),
                "{octet} spelled {spelled}"
            );
        }

        // The empty buffer, the order of the two nibbles, and a zero octet in
        // the middle: a buffer is octets rather than a C string, so nothing
        // ends at one.
        assert_eq!(spell(&mut ctx, b""), "");
        assert_eq!(spell(&mut ctx, &[0x1f, 0xf1]), "1ff1");
        assert_eq!(spell(&mut ctx, &[0x41, 0x00, 0x42]), "410042");

        // Twice the buffer's length, over a whole sweep rather than one
        // buffer, and readable by `fromHex` — the two members are one round
        // trip and a spelling neither side rejects is the only useful one.
        for length in 0..64_usize {
            let octets: Vec<u8> = (0..length)
                .map(|at| u8::try_from(at * 7 % 256).expect("a residue of 256 fits a `u8`"))
                .collect();
            let spelled = spell(&mut ctx, &octets);
            assert_eq!(spelled.len(), length * 2, "{length} octets");

            let text = Value::str(NvsStr::new(spelled.as_bytes()));
            let read = nvs_runtime::call(nvs_core_encoding_from_hex, &mut ctx, &[text])
                .expect("`toHex`'s answer is text `fromHex` reads");
            assert_eq!(read.as_bytes(), Some(octets.as_slice()), "{length} octets");
            #[expect(
                unsafe_code,
                reason = "this test owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                read.release();
                text.release();
            }
        }
    }

    /// `encodeText` reached the way a program reaches it, for the reason
    /// [`decoding_answers_the_text_and_names_the_offset_it_stopped_at`] gives.
    /// The refusal names the character as well as the offset, and both halves
    /// are asserted: a message quoting the wrong character sends a caller to
    /// fix a field that was never the problem, and the offset is counted in
    /// the *text*, so a character past a multi-byte one is the trap.
    // covers: Core\Encoding::encodeText
    #[test]
    fn encoding_answers_the_octets_and_names_the_character_it_cannot_spell() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let case = |name: &str| {
            let index = CHARSET
                .cases
                .iter()
                .position(|(case, _)| *case == name)
                .expect("a declared case");
            Value::int(i64::try_from(index).expect("a case index fits an `i64`"))
        };

        for (text, charset, octets) in [
            ("", "Utf8", &b""[..]),
            ("nvs", "Ascii", b"nvs".as_slice()),
            ("\u{e9}", "Utf8", &[0xc3, 0xa9][..]),
            ("\u{e9}", "Latin1", &[0xe9][..]),
            ("a", "Utf16Be", &[0x00, 0x61][..]),
            ("\u{20ac}", "Windows1252", &[0x80][..]),
        ] {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let answered = nvs_runtime::call(
                nvs_core_encoding_encode_text,
                &mut ctx,
                &[subject, case(charset)],
            )
            .expect("a text the charset spells encodes");
            assert_eq!(answered.as_bytes(), Some(octets), "{charset} over {text:?}");
            #[expect(
                unsafe_code,
                reason = "this test owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                answered.release();
                subject.release();
            }
        }

        // The euro sign after a two-byte character: its offset is 2, which is
        // where it starts in the UTF-8 the `string` holds.
        for (text, charset, offset, character) in [
            ("\u{20ac}", "Latin1", 0_usize, '\u{20ac}'),
            ("\u{e9}\u{20ac}", "Latin1", 2, '\u{20ac}'),
            ("ok\u{1f600}", "Ascii", 2, '\u{1f600}'),
        ] {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let refused = nvs_runtime::call(
                nvs_core_encoding_encode_text,
                &mut ctx,
                &[subject, case(charset)],
            );
            assert!(refused.is_err(), "{charset} over {text:?}");
            let message = ctx.take_pending().expect("a catchable refusal");
            assert!(
                message.contains(&format!("at offset {offset} ")),
                "{charset} over {text:?} said {message}"
            );
            assert!(
                message.contains(&format!("'{character}'")),
                "{charset} over {text:?} said {message}"
            );
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built, and the \
                          member borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
        }
    }

    /// `decodeText` reached the way a program reaches it rather than through
    /// [`decode_exact`] alone, so the argument walk, the answer's tag and the
    /// refusal's shape are pinned together. The message is asserted to carry
    /// the offset it stopped at, because a member reporting `0` for every
    /// input passes any test that only checks it threw, and that offset is
    /// what tells a caller which field of its input was corrupt.
    // covers: Core\Encoding::decodeText
    #[test]
    fn decoding_answers_the_text_and_names_the_offset_it_stopped_at() {
        let mut ctx = nvs_runtime::Ctx::buffered();

        // A case's position in `CHARSET` is the integer a program passes, per
        // `charset_of` — the same roster identity `CHARSET` documents.
        let case = |name: &str| {
            let index = CHARSET
                .cases
                .iter()
                .position(|(case, _)| *case == name)
                .expect("a declared case");
            Value::int(i64::try_from(index).expect("a case index fits an `i64`"))
        };

        // The empty buffer answers the empty string, and each of the five
        // schemes this module decodes itself answers its own table's text.
        for (octets, charset, text) in [
            (&b""[..], "Utf8", ""),
            (b"nvs".as_slice(), "Ascii", "nvs"),
            (&[0xc3, 0xa9][..], "Utf8", "\u{e9}"),
            (&[0xe9][..], "Latin1", "\u{e9}"),
            (&[0x61, 0x00][..], "Utf16Le", "a"),
            (&[0x80][..], "Windows1252", "\u{20ac}"),
        ] {
            let subject = Value::bytes(NvsStr::new(octets));
            let answered = nvs_runtime::call(
                nvs_core_encoding_decode_text,
                &mut ctx,
                &[subject, case(charset)],
            )
            .expect("a buffer the charset reads decodes");
            assert_eq!(answered.as_text(), Some(text), "{charset} over {octets:?}");
            #[expect(
                unsafe_code,
                reason = "this test owns the reference it built for the \
                          argument and the one the member answered, and the \
                          member borrowed rather than consumed its own"
            )]
            unsafe {
                answered.release();
                subject.release();
            }
        }

        // The refusal, at three different offsets so the number is read off
        // the input rather than off a constant. A lone `0xff` is UTF-8 at no
        // position; the second is valid for two characters and then not; the
        // third counts UTF-16 in the units it consumed.
        for (octets, charset, offset) in [
            (&[0xff][..], "Utf8", 0_usize),
            (&[0x61, 0xc3, 0xa9, 0xff][..], "Utf8", 3),
            (&[0x61, 0x00, 0x3d, 0xd8][..], "Utf16Le", 2),
        ] {
            let subject = Value::bytes(NvsStr::new(octets));
            let refused = nvs_runtime::call(
                nvs_core_encoding_decode_text,
                &mut ctx,
                &[subject, case(charset)],
            );
            assert!(refused.is_err(), "{charset} over {octets:?}");
            let message = ctx.take_pending().expect("a catchable refusal");
            assert!(
                message.contains(&format!("at offset {offset} ")),
                "{charset} over {octets:?} said {message}"
            );
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built, and the \
                          member borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
        }
    }

    /// `isValidText` reached the way a program reaches it rather than through
    /// [`decode_exact`] alone, so the argument walk and the answer's tag are
    /// pinned together with the verdict. Every case also asserts that nothing
    /// is left pending, because a member that threw where it owes a `false`
    /// still satisfies a test that only reads the value it answered. The
    /// sweep counts over the whole octet range rather than naming a byte:
    /// `Latin1` is total, so a member that grew a hole in it fails here while
    /// every hand-picked row still passes.
    // covers: Core\Encoding::isValidText
    #[test]
    fn validity_answers_a_bool_for_every_buffer_and_never_refuses() {
        let mut ctx = nvs_runtime::Ctx::buffered();

        // A case's position in `CHARSET` is the integer a program passes, per
        // `charset_of` — the same roster identity `CHARSET` documents.
        let case = |name: &str| {
            let index = CHARSET
                .cases
                .iter()
                .position(|(case, _)| *case == name)
                .expect("a declared case");
            Value::int(i64::try_from(index).expect("a case index fits an `i64`"))
        };
        let asked = |ctx: &mut nvs_runtime::Ctx, octets: &[u8], charset: &str| {
            let subject = Value::bytes(NvsStr::new(octets));
            let answered = nvs_runtime::call(
                nvs_core_encoding_is_valid_text,
                ctx,
                &[subject, case(charset)],
            )
            .expect("the member answers rather than refusing");
            assert!(
                ctx.take_pending().is_none(),
                "{charset} over {octets:?} left a refusal behind"
            );
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built, and the \
                          member borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
            answered.as_bool().expect("a `bool` answer")
        };

        // The empty buffer is text in every charset, and each refusal
        // `decodeText` has a case for is a `false` here: a byte no sequence
        // starts with, a buffer that ends mid-character, and a UTF-16 unit
        // with no pair.
        for (octets, charset, valid) in [
            (&b""[..], "Utf8", true),
            (b"nvs".as_slice(), "Ascii", true),
            (&[0xc3, 0xa9][..], "Utf8", true),
            (&[0xff][..], "Utf8", false),
            (&[0x61, 0xc3, 0xa9, 0xff][..], "Utf8", false),
            (&[0xc3][..], "Utf8", false),
            (&[0x80][..], "Ascii", false),
            (&[0x61, 0x00][..], "Utf16Le", true),
            (&[0x61, 0x00, 0x3d, 0xd8][..], "Utf16Le", false),
            (&[0x61, 0x00, 0x3d][..], "Utf16Le", false),
        ] {
            assert_eq!(
                asked(&mut ctx, octets, charset),
                valid,
                "{charset} over {octets:?}"
            );
        }

        // ISO-8859-1 gives all 256 octets a meaning, so the member answers
        // `true` for every one of them; UTF-8 reads exactly the 128 below
        // `0x80` on their own.
        let mut latin1 = 0_usize;
        let mut utf8 = 0_usize;
        for octet in 0..=u8::MAX {
            latin1 += usize::from(asked(&mut ctx, &[octet], "Latin1"));
            utf8 += usize::from(asked(&mut ctx, &[octet], "Utf8"));
        }
        assert_eq!(latin1, 256, "ISO-8859-1 reads every octet alone");
        assert_eq!(utf8, 128, "UTF-8 reads the ASCII octets alone");
    }

    #[test]
    fn a_quoted_operand_is_bounded() {
        let long = "a".repeat(SHOWN_CHARS * 2);
        let quoted = shown(&long);
        assert_eq!(quoted.chars().count(), SHOWN_CHARS + 1);
        assert!(quoted.ends_with('…'));
        assert_eq!(shown("ff"), "ff");
    }
}
