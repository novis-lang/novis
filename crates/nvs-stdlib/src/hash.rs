//! `Core\Hash` and `Core\Digest` — docs/spec/01-core-library.md § 11's second
//! half: one member per *question* a program asks about a digest, with the
//! algorithm as an argument rather than as part of the member's name.
//!
//! PHP spells this surface eight ways — `hash`, `md5`, `sha1`, `crc32`,
//! `openssl_digest`, `hash_hmac`, `hash_equals`, `hash_init`/`update`/`final`
//! — and the first five differ only in which algorithm they hard-code and
//! whether the result comes back as hex, as raw octets, or as an `int`.
//! Collapsing them costs a program one argument and buys it the property that
//! *changing algorithm is an edit to a value, not to a call*.
//!
//! # `Hash\Stream` accumulates, and what that spends
//!
//! § 11's fourth row is the incremental form — `hash_init`/`hash_update`/
//! `hash_final`, and PHP's `HashContext` — and [`STREAM`] is it, with the
//! surface a program writes settled here and the *internals* deliberately not.
//!
//! **It holds the chunks, not a compression state.** A `Core` instance's slots
//! hold values Novis already holds ([`crate::instance`]), so the running state of
//! a `sha2::Sha256` cannot live in one: it is a native object with no Novis
//! spelling, and an instance has no destructor to free it with — the same wall
//! [`crate::identity_store`] met and answered the same way. So `update` retains
//! its argument into an array slot and `finish` hashes the concatenation in one
//! pass. `digest 0.10` cannot serialize a hasher's state into a slot either
//! (`crypto_common::SerializableState` is a later major version), which is
//! the one thing that would make a fixed-size slot possible.
//!
//! **What it spends is a reference to each chunk, held until `finish`** — no
//! copy, since a retained `NvsStr` is the caller's own buffer, but a program
//! that streams a file to keep its memory flat does not get that here: the
//! footprint is the total fed, charged to the request, and released at `finish`
//! or when the stream itself is. That is AGENTS.md's priority 5 spent to buy
//! priority 4, and it is *observably* `hash_init`/`update`/`final` either way —
//! which is what keeps the choice cheap to reverse. When a runtime tag owns a
//! native object with a release hook, this class's three slots become that
//! object and no written program changes.
//!
//! **`finish` closes the stream**, and `update` after it throws rather than
//! restarting. Nothing in the accumulating implementation needs that rule — the
//! chunks are still there — and it is written anyway, because a real context is
//! consumed by its own finalization and a program allowed to depend on the
//! looser behaviour would be the thing that made the swap above expensive.
//!
//! # The result is `bytes`, never hex
//!
//! `of` and `hmac` answer `bytes`, and there is no `raw_output` flag beside
//! them. PHP's default is the hex *string* with the raw octets behind a
//! boolean, which is how `hash_equals(md5($a), md5($b))` became the shape
//! everybody writes: the value that leaves the hash is already text, so it is
//! already the thing that gets compared, logged and concatenated. Here the
//! digest is the 32 octets it actually is, and `Core\Encoding::toHex`
//! ([`crate::encoding`]) is the one place a spelling is chosen — the same
//! seam `rule:types/bytes` draws everywhere else, and the reason `Core\Encoding` exists
//! as a class rather than as members on `Core\Str`.
//!
//! A `crc32` is four octets, big-endian, so `toHex` of it reads as PHP's
//! `hash("crc32b", …)` does. PHP's own `crc32()` answers an `int` instead, and
//! that difference is deliberate: a checksum that is sometimes an integer and
//! sometimes a string is exactly the per-algorithm special case this member
//! exists to remove.
//!
//! # `secret`, and the spelling that arrived for it
//!
//! § 11 writes `hmac`'s key parameter as `secret bytes $key`
//! (`rule:security/secret-qualifier`),
//! and for a long time [`crate::registry::CoreTy`] had no qualifier to carry
//! that with — a row stated an atom, not a qualified type.
//! [`CoreTy::SecretBlob`] is that spelling, added for [`crate::crypto`]'s key
//! parameters, and the row below now writes it: the key is a `secret bytes`
//! the checker keeps out of a sink, and nothing about how it is read or held
//! here changed. A caller handing a plain `bytes` key is unaffected, because
//! `nvs_types`' assignment relation widens onto a qualifier bit and never off
//! one.
//!
//! # Why these dependencies
//!
//! `rule:packaging/a-c-dependency-answers-two-questions` asks
//! two questions, and both land on "take the audited implementation" here —
//! the opposite of the answer [`crate::encoding`]'s hex pair got, and worth
//! reading beside it.
//!
//! Attacker-controlled data reaches every one of these members, so § 4's
//! second question applies and wants a demonstrable verification record. Each
//! algorithm is an external specification with published test vectors and a
//! decade of cryptanalysis attached, so a hand-written one would be a
//! reimplementation that is *only* as good as its own test suite — which is
//! the case where a dependency is strictly the safer trade. The RustCrypto
//! family (`sha2`, `sha1`, `md-5`, `hmac`) is pure Rust with no build script
//! and no C, so § 4's confine-to-wasm branch never opens; it verifies against
//! each algorithm's own NIST or RFC vectors, and one `digest` trait set drives
//! all five hashes *and* the HMAC construction, so [`DIGEST`] dispatches over
//! one shape rather than five hand-fitted APIs. `crc32fast` is not from that
//! family and not a hash at all — it is the checksum § 11 keeps for interop.
//!
//! `subtle` is the one that looks like it should not be a dependency, and is
//! the one that most needs to be. [`nvs_core_hash_equals`] is three lines in
//! any language; the difficulty is that an optimiser is entitled to turn the
//! obvious loop back into an early exit, which is the entire bug a
//! constant-time comparison exists to avoid, and nothing in the source says
//! otherwise. `subtle`'s whole purpose is the optimisation barrier that stops
//! that, maintained by people who track what each compiler version does with
//! it.
//!
//! **What this spends:** one digest state on the stack per call — a couple of
//! kilobytes at the largest, which is BLAKE3's chunk stack; every other
//! algorithm here is 200-odd bytes — released before the member returns, plus
//! one `bytes` allocation of the digest's own width (4 to 64 octets), charged
//! to the request that asked for it. Nothing is held between calls. Binary
//! size is the other side of the roster: each algorithm carries its own round
//! constants and tables, which is AGENTS.md's priority 5 spent on priority 2.

use nvs_runtime::{Fault, NvsArray, NvsStr, ObjHeader, Value};
use sha2::Digest as _;
use subtle::ConstantTimeEq as _;

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, its enum, and where its symbols live
// ============================================================================

/// `Core\Hash`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Hash";

/// `Core\Digest`'s fully-qualified name, for [`DIGEST`] and for the case types
/// [`STRONG`] is built out of.
pub(crate) const DIGEST_NAME: &str = r"Core\Digest";

/// Spec § 11's `Digest` — **every** algorithm, the two broken ones included.
///
/// The first six were ordered weakest first, which made [`STRONG`] a
/// contiguous tail. That stopped being true when the roster grew: the
/// ordinals below are ABI (see the paragraph after next), so a case is
/// *appended* and never inserted, and `Crc32c` and `Blake3` sit at the end
/// outside [`STRONG`] with ten strong cases in front of them. **The subset
/// is the list in [`STRONG`], never a range**, and nothing may read it as one.
///
/// `Crc32`, `Md5` and `Sha1` are here because interop genuinely needs them: a
/// legacy database column, an ETag, a package manifest and a third party's
/// signature scheme all name one, and a language that refuses to compute them
/// does not stop anyone using them — it makes the program shell out or
/// hand-roll one, which is worse in every direction that matters. What the
/// type system does instead is refuse them *where the choice is security*,
/// which is [`STRONG`].
///
/// The integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`]. They are ABI, in the sense that
/// [`digest_kind`] reads them back out of an argument slot: reordering this
/// list is a behaviour change, not a cosmetic one.
pub(crate) const DIGEST: CoreEnum = CoreEnum {
    name: DIGEST_NAME,
    cases: &[
        ("Crc32", 0),
        ("Md5", 1),
        ("Sha1", 2),
        ("Sha256", 3),
        ("Sha384", 4),
        ("Sha512", 5),
        ("Sha224", 6),
        ("Sha512_224", 7),
        ("Sha512_256", 8),
        ("Sha3_224", 9),
        ("Sha3_256", 10),
        ("Sha3_384", 11),
        ("Sha3_512", 12),
        ("Crc32c", 13),
        ("Blake3", 14),
    ],
    doc: Some(&DIGEST_DOC),
};

/// [`DIGEST`]'s reference card — `rule:core-api/reference-card`. The roster's home is spec § 11's
/// table; the notes here are that table's, condensed.
const DIGEST_DOC: EnumDoc = EnumDoc {
    short: "The algorithm a `Core\\Hash` member computes — every one PHP's `hash()` names that \
            interop needs, checksums and the two broken digests included, plus BLAKE3. \
            `StrongDigest`, the subset `Core\\Hash::hmac` accepts, is the ten SHA-2 and SHA-3 \
            cases.",
    cases: &[
        CaseDoc {
            name: "Crc32",
            desc: "CRC-32/ISO-HDLC, PHP's `crc32b` — a 4-octet checksum against accidental \
                   corruption only.",
        },
        CaseDoc {
            name: "Md5",
            desc: "MD5, 16 octets — collision-broken since 2004, for interop only.",
        },
        CaseDoc {
            name: "Sha1",
            desc: "SHA-1, 20 octets — collision-broken since 2017, for interop only.",
        },
        CaseDoc {
            name: "Sha256",
            desc: "SHA-256, 32 octets — the default to reach for; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha384",
            desc: "SHA-384, 48 octets; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha512",
            desc: "SHA-512, 64 octets — faster than SHA-256 on 64-bit hardware; a \
                   `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha224",
            desc: "SHA-224, 28 octets; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha512_224",
            desc: "SHA-512/224, 28 octets — SHA-512 truncated with its own IV (FIPS 180-4 \
                   § 5.3.6), PHP's `sha512/224`; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha512_256",
            desc: "SHA-512/256, 32 octets — SHA-512's speed at SHA-256's width and \
                   length-extension-proof, PHP's `sha512/256`; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha3_224",
            desc: "SHA3-224, 28 octets — FIPS 202's Keccak sponge, an independent construction \
                   rather than a wider SHA-2; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha3_256",
            desc: "SHA3-256, 32 octets; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha3_384",
            desc: "SHA3-384, 48 octets; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Sha3_512",
            desc: "SHA3-512, 64 octets; a `StrongDigest`.",
        },
        CaseDoc {
            name: "Crc32c",
            desc: "CRC-32C/Castagnoli, 4 octets — the checksum S3 and GCS stamp objects with.",
        },
        CaseDoc {
            name: "Blake3",
            desc: "BLAKE3, 32 octets — the fastest here and the one PHP cannot compute; outside \
                   `StrongDigest` because it is keyed natively rather than through HMAC.",
        },
    ],
};

/// Spec § 11's `StrongDigest` — the closed subset [`nvs_core_hash_hmac`]
/// declares, so `Hash::hmac($m, $k, Digest::Md5)` does not compile.
///
/// A **union of case types** (`rule:types/enum-case-type`) rather than a second enum, which is the whole reason
/// [`CoreTy::EnumCase`] exists: `Core\StrongDigest::Sha256` would be a
/// different type from `Core\Digest::Sha256`, so no single value could be
/// passed to both `of` and `hmac` and every program holding a configured
/// algorithm would need two of them.
///
/// **`Sha1` is deliberately outside it, and that costs something.** HMAC-SHA1
/// has no practical break — the construction does not need collision
/// resistance, which is the property SHA-1 lost — so this is not the same
/// judgement as excluding `Md5` or `Crc32`, and it does shut out OAuth 1.0a
/// and other legacy signature schemes that name it. It is excluded anyway
/// because the alternative is a type called `StrongDigest` whose cases a
/// reader has to already know the cryptanalysis of, and because AGENTS.md's
/// priority 1 is not traded for reach. If a real integration needs it, the fix
/// is to add the case here: **widening a union is backward compatible**, so
/// this is the direction that can be undone, and the other one is not.
///
/// **`Blake3` is outside it too, and for a different reason than `Sha1`.**
/// HMAC-BLAKE3 is a construction nobody uses: BLAKE3 is keyed natively, so
/// wrapping it in RFC 2104's two padded blocks is slower than the primitive's
/// own keyed mode and interoperates with nothing. Admitting it here would be
/// offering a spelling whose only property is that it type-checks, so it waits
/// for a keyed member designed as one rather than being bent into `hmac`.
/// `Crc32c` is outside for `Crc32`'s reason: it is a checksum.
const STRONG: &[CoreTy] = &[
    CoreTy::EnumCase(DIGEST_NAME, "Sha224"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha256"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha384"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha512"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha512_224"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha512_256"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha3_224"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha3_256"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha3_384"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha3_512"),
];

/// `bytes|string` — what both hashing members take, and the reason neither has
/// a text-flavoured twin.
///
/// Total in one direction and free (`rule:types/conversion`): a `string` is valid UTF-8 and therefore already a valid byte
/// sequence, so [`data_of`] reads the same buffer either tag points at without
/// copying or validating anything.
const DATA: &[CoreTy] = &[CoreTy::Bytes, CoreTy::Str];

/// `Core\Hash`'s registry rows — § 11's `of`/`hmac`/`equals`.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "of",
            names: &["data", "digest"],
            params: &[CoreTy::Union(DATA), CoreTy::Enum(DIGEST_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_hash_of",
            doc: Some(&OF_DOC),
        },
        CoreMethod {
            name: "hmac",
            names: &["data", "key", "digest"],
            params: &[
                CoreTy::Union(DATA),
                CoreTy::SecretBlob(Qual::Neutral),
                CoreTy::Union(STRONG),
            ],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_hash_hmac",
            doc: Some(&HMAC_DOC),
        },
        CoreMethod {
            name: "equals",
            names: &["a", "b"],
            params: &[CoreTy::Bytes, CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_hash_equals",
            doc: Some(&EQUALS_DOC),
        },
        CoreMethod {
            name: "stream",
            names: &["digest"],
            params: &[CoreTy::Enum(DIGEST_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(STREAM_NAME),
            symbol: "nvs_core_hash_stream",
            doc: Some(&STREAM_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Hash::of`'s reference card — `rule:core-api/reference-card`.
const OF_DOC: MethodDoc = MethodDoc {
    short: "Computes the digest of `$data` under `$digest`, replacing `hash`, `md5`, `sha1`, \
            `crc32` and `openssl_digest` at once — raw octets, never hex or an `int`.",
    params: &[
        ParamDoc {
            name: "data",
            desc: "The octets to hash; a `string` is read as its UTF-8 bytes.",
            shape: &[],
        },
        ParamDoc {
            name: "digest",
            desc: "The algorithm, any `Core\\Digest` case — the broken ones included, for \
                   interop.",
            shape: &[],
        },
    ],
    ret: "The digest as `bytes`, as many octets as the case's width; every case accepts every \
          input, the empty one included.",
    errors: &[],
};

/// `Core\Hash::hmac`'s reference card — `rule:core-api/reference-card`.
const HMAC_DOC: MethodDoc = MethodDoc {
    short: "Computes RFC 2104's HMAC of `$data` under `$key` and `$digest`, as `hash_hmac` does \
            without its `raw_output` flag — and only under a `StrongDigest`, so `Digest::Md5` \
            or `Digest::Sha1` here is a compile error.",
    params: &[
        ParamDoc {
            name: "data",
            desc: "The octets to authenticate; a `string` is read as its UTF-8 bytes.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The secret key, of any length — a long one is hashed down and a short one \
                   zero-padded, as RFC 2104 § 2 says.",
            shape: &[],
        },
        ParamDoc {
            name: "digest",
            desc: "The algorithm, one of the ten SHA-2 and SHA-3 `Core\\Digest` cases.",
            shape: &[],
        },
    ],
    ret: "The MAC as `bytes`, as many octets as the case's width.",
    errors: &[],
};

/// `Core\Hash::equals`'s reference card — `rule:core-api/reference-card`.
const EQUALS_DOC: MethodDoc = MethodDoc {
    short: "Compares two digests in constant time, as `hash_equals` does: the running time does \
            not depend on where two equal-length operands first differ.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "One digest.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The other digest.",
            shape: &[],
        },
    ],
    ret: "`true` when the two hold the same octets; `false` at once for two lengths that differ, \
          since a digest's length is the algorithm's and never a secret.",
    errors: &[],
};

/// `Core\Hash::stream`'s reference card — `rule:core-api/reference-card`.
const STREAM_DOC: MethodDoc = MethodDoc {
    short: "Opens an incremental digest under `$digest`, as `hash_init` does — a \
            `Core\\Hash\\Stream` fed by `update` and closed by `finish`.",
    params: &[ParamDoc {
        name: "digest",
        desc: "The algorithm, any `Core\\Digest` case.",
        shape: &[],
    }],
    ret: "A fresh, open stream that has been fed nothing yet.",
    errors: &[],
};

/// [`STREAM`]'s name, written once — see [`NAME`].
pub(crate) const STREAM_NAME: &str = r"Core\Hash\Stream";

/// Spec § 11's `Core\Hash\Stream` — the incremental digest, and the one
/// *mutable* `Core` instance outside § 9's collections.
///
/// Three slots and two members. `digest` is the [`DIGEST`] case the stream was
/// opened with, as the integer the enum already is; `chunks` is every buffer
/// `update` has been handed, in order; `open` is `false` once `finish` has
/// answered. This module's own docs say why the state is the chunks rather
/// than a compression context, and what that spends.
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "update",
            names: &["data"],
            params: &[CoreTy::Union(DATA)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_hash_stream_update",
            doc: Some(&STREAM_UPDATE_DOC),
        },
        CoreMethod {
            name: "finish",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_hash_stream_finish",
            doc: Some(&STREAM_FINISH_DOC),
        },
    ],
    slots: &["digest", "chunks", "open"],
    constants: &[],
};

/// `$stream->update`'s reference card — `rule:core-api/reference-card`.
const STREAM_UPDATE_DOC: MethodDoc = MethodDoc {
    short: "Feeds `$data` to the stream, as `hash_update` does; the chunks are digested in \
            order at `finish`.",
    params: &[ParamDoc {
        name: "data",
        desc: "The next octets; a `string` is read as its UTF-8 bytes.",
        shape: &[],
    }],
    ret: "Nothing; the stream holds a reference to `$data` until `finish`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The stream has already been finished — a digest is final, so open a new \
               stream.",
    }],
};

/// `$stream->finish`'s reference card — `rule:core-api/reference-card`.
const STREAM_FINISH_DOC: MethodDoc = MethodDoc {
    short: "Closes the stream and answers the digest of everything `update` fed it, as \
            `hash_final` does — the same octets `Core\\Hash::of` answers over the \
            concatenation.",
    params: &[],
    ret: "The digest as `bytes`; the stream is finished afterwards and its chunks released.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The stream has already been finished — a second `finish` is refused rather than \
               continuing from the first.",
    }],
};

/// [`STREAM`]'s `digest` slot, by index.
const DIGEST_SLOT: usize = 0;

/// [`STREAM`]'s `chunks` slot, by index.
const CHUNKS_SLOT: usize = 1;

/// [`STREAM`]'s `open` slot, by index.
const OPEN_SLOT: usize = 2;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_hash_of" => (nvs_core_hash_of as *const ()).cast(),
        "nvs_core_hash_hmac" => (nvs_core_hash_hmac as *const ()).cast(),
        "nvs_core_hash_equals" => (nvs_core_hash_equals as *const ()).cast(),
        "nvs_core_hash_stream" => (nvs_core_hash_stream as *const ()).cast(),
        "nvs_core_hash_stream_update" => (nvs_core_hash_stream_update as *const ()).cast(),
        "nvs_core_hash_stream_finish" => (nvs_core_hash_stream_finish as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Reading arguments
// ============================================================================

/// One case of [`DIGEST`], decoded from the integer an argument slot carries.
///
/// A Rust mirror of the source-visible enum rather than a reuse of it, because
/// the two answer different questions: [`DIGEST`] is what the *checker* reads,
/// and this is what dispatch matches on. The `match` in [`digest_kind`] is the
/// one place they are tied together, so a case added to one without the other
/// fails to compile here rather than at a call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DigestKind {
    /// CRC-32/ISO-HDLC, PHP's `crc32b` — a checksum, not a hash. Detects
    /// accidental corruption and nothing an adversary does.
    Crc32,
    /// MD5 — collision-broken since 2004. Interop only.
    Md5,
    /// SHA-1 — collision-broken since 2017 (SHAttered). Interop only.
    Sha1,
    /// SHA-256, the default a program should reach for.
    Sha256,
    /// SHA-384 — SHA-512 truncated, with different initial state.
    Sha384,
    /// SHA-512, faster than SHA-256 on 64-bit hardware.
    Sha512,
    /// SHA-224 — SHA-256 truncated, with different initial state.
    Sha224,
    /// SHA-512/224 — SHA-512 truncated, with its own IV per FIPS 180-4 § 5.3.6.
    Sha512_224,
    /// SHA-512/256 — the same, at 256 bits. Faster than SHA-256 on 64-bit
    /// hardware and structurally immune to length extension, which is why it
    /// is the one addition here worth reaching for on purpose.
    Sha512_256,
    /// SHA3-224 — FIPS 202's Keccak sponge, an independent construction from
    /// SHA-2 rather than a wider one.
    Sha3_224,
    /// SHA3-256.
    Sha3_256,
    /// SHA3-384.
    Sha3_384,
    /// SHA3-512.
    Sha3_512,
    /// CRC-32C, the Castagnoli polynomial — what S3 and GCS stamp objects
    /// with. A checksum, like [`DigestKind::Crc32`], and not a hash.
    Crc32c,
    /// BLAKE3 — the fastest of these by a wide margin, and the one algorithm
    /// on this roster PHP cannot compute at all. Extendable-output; the 32
    /// octets [`digest_of`] answers are its default length.
    Blake3,
}

/// The [`DIGEST`] case an integer names, or `None` for anything that is no
/// case.
///
/// Separate from [`digest_kind`] because a stream reads its algorithm back out
/// of a *slot* rather than out of an argument, and the ordinals are the same
/// ordinals: [`STREAM`] stores what its opening argument decoded from.
fn kind_of(ordinal: Option<i64>) -> Option<DigestKind> {
    Some(match ordinal? {
        0 => DigestKind::Crc32,
        1 => DigestKind::Md5,
        2 => DigestKind::Sha1,
        3 => DigestKind::Sha256,
        4 => DigestKind::Sha384,
        5 => DigestKind::Sha512,
        6 => DigestKind::Sha224,
        7 => DigestKind::Sha512_224,
        8 => DigestKind::Sha512_256,
        9 => DigestKind::Sha3_224,
        10 => DigestKind::Sha3_256,
        11 => DigestKind::Sha3_384,
        12 => DigestKind::Sha3_512,
        13 => DigestKind::Crc32c,
        14 => DigestKind::Blake3,
        _ => return None,
    })
}

/// The `Core\Digest` case in slot `index`, or the `FATAL` a value that is no
/// case is.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: `nvs_types` already checked the
/// declared type and compiled code wrote the integer, so anything else here is
/// a runtime-contract violation rather than something a program can cause —
/// the same treatment [`crate::math`] gives its `RoundMode` option.
fn digest_kind(args: &[Value], index: usize, member: &str) -> Result<DigestKind, Fault> {
    match kind_of(args[index].as_int()) {
        Some(kind) => Ok(kind),
        None => Err(Fault::fatal(format!(
            "Core\\Hash::{member} expected a `Core\\Digest` case, got tag {} value {:?}",
            args[index].tag_byte(),
            args[index].as_int()
        ))),
    }
}

/// The `bytes|string` in slot `index` as the octets to hash — see [`DATA`] for
/// why one reader covers both tags.
///
/// # Errors
///
/// A [`Fault::fatal`], for [`digest_kind`]'s reason.
fn data_of<'a>(args: &'a [Value], index: usize, member: &str) -> Result<&'a [u8], Fault> {
    args[index]
        .as_bytes()
        .or_else(|| args[index].as_str_bytes())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Hash::{member} expected a `bytes` or a `string`, got tag {}",
                args[index].tag_byte()
            ))
        })
}

/// The `bytes` in slot `index`, for [`digest_kind`]'s reason.
fn bytes_of<'a>(args: &'a [Value], index: usize, member: &str) -> Result<&'a [u8], Fault> {
    args[index].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Hash::{member} expected a `bytes`, got tag {}",
            args[index].tag_byte()
        ))
    })
}

// ============================================================================
// The algorithms
// ============================================================================

/// One digest's octets, held inline: the widest of [`DIGEST`]'s fifteen is 64
/// octets, so no digest or MAC needs the heap before the one [`NvsStr`] a
/// member returns it in. That keeps `of` and `hmac` at one allocation per call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Octets {
    buf: [u8; 64],
    len: usize,
}

impl Octets {
    /// `octets`, copied in. Every caller hands over a digest of at most 64
    /// octets, which the slice index enforces.
    fn of(octets: &[u8]) -> Self {
        let mut buf = [0_u8; 64];
        buf[..octets.len()].copy_from_slice(octets);
        Self {
            buf,
            len: octets.len(),
        }
    }
}

impl std::ops::Deref for Octets {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

/// `data` under `kind`, as the digest's own octets.
///
/// Total: there is no input any of these fifteen refuses, and none of them has
/// a size limit short of the address space.
fn digest_of(kind: DigestKind, data: &[u8]) -> Octets {
    match kind {
        // Big-endian, so that `toHex` of the result reads the way PHP's
        // `hash("crc32b", …)` prints it — see this module's own docs.
        DigestKind::Crc32 => {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(data);
            Octets::of(&hasher.finalize().to_be_bytes())
        }
        DigestKind::Md5 => Octets::of(&md5::Md5::digest(data)),
        DigestKind::Sha1 => Octets::of(&sha1::Sha1::digest(data)),
        DigestKind::Sha256 => Octets::of(&sha2::Sha256::digest(data)),
        DigestKind::Sha384 => Octets::of(&sha2::Sha384::digest(data)),
        DigestKind::Sha512 => Octets::of(&sha2::Sha512::digest(data)),
        DigestKind::Sha224 => Octets::of(&sha2::Sha224::digest(data)),
        DigestKind::Sha512_224 => Octets::of(&sha2::Sha512_224::digest(data)),
        DigestKind::Sha512_256 => Octets::of(&sha2::Sha512_256::digest(data)),
        DigestKind::Sha3_224 => Octets::of(&sha3::Sha3_224::digest(data)),
        DigestKind::Sha3_256 => Octets::of(&sha3::Sha3_256::digest(data)),
        DigestKind::Sha3_384 => Octets::of(&sha3::Sha3_384::digest(data)),
        DigestKind::Sha3_512 => Octets::of(&sha3::Sha3_512::digest(data)),
        // Big-endian for `Crc32`'s reason, and the same four octets wide, so
        // the two checksums differ in polynomial and in nothing else a program
        // can see.
        DigestKind::Crc32c => Octets::of(&crc32c::crc32c(data).to_be_bytes()),
        // Not through `digest 0.10`: BLAKE3 implements those traits only under
        // its `traits-preview` feature, and the free function is the whole API
        // this needs. Its extendable output is taken at the default 32 octets,
        // which is what every other implementation calls "the" BLAKE3 hash.
        DigestKind::Blake3 => Octets::of(blake3::hash(data).as_bytes()),
    }
}

/// RFC 2104's HMAC of `data` under `key` and `kind`.
///
/// `None` for a `kind` outside [`STRONG`], which the checker has already
/// refused — the arm exists because [`digest_kind`] answers over the whole
/// enum and this is the one place the subset is enforced twice. Belt and
/// braces on purpose: the type is the guarantee a *program* gets, and this is
/// the guarantee the runtime keeps if a future caller reaches the helper by
/// another route.
fn hmac_of(kind: DigestKind, key: &[u8], data: &[u8]) -> Option<Octets> {
    use hmac::Mac as _;

    /// One algorithm's HMAC, written once. A macro rather than a generic
    /// function because `hmac::Hmac<D>`'s bound set is a dozen `typenum`
    /// obligations about block sizes, none of which says anything a reader of
    /// this module needs to know — and every instantiation is named in the
    /// `match` below anyway, so the generality bought nothing. It takes both
    /// families unchanged: `sha3` is the same RustCrypto `digest 0.10` trait
    /// set `sha2` is, which is the whole reason FIPS 202 cost no hand-fitting.
    ///
    /// `new_from_slice` is infallible for HMAC, whose key may be any length at
    /// all: RFC 2104 § 2 hashes a long one down and zero-pads a short one.
    macro_rules! mac {
        ($digest:ty) => {{
            let mut mac = <hmac::Hmac<$digest>>::new_from_slice(key)
                .expect("HMAC accepts a key of any length (RFC 2104 § 2)");
            mac.update(data);
            Octets::of(&mac.finalize().into_bytes())
        }};
    }

    Some(match kind {
        DigestKind::Sha224 => mac!(sha2::Sha224),
        DigestKind::Sha256 => mac!(sha2::Sha256),
        DigestKind::Sha384 => mac!(sha2::Sha384),
        DigestKind::Sha512 => mac!(sha2::Sha512),
        DigestKind::Sha512_224 => mac!(sha2::Sha512_224),
        DigestKind::Sha512_256 => mac!(sha2::Sha512_256),
        DigestKind::Sha3_224 => mac!(sha3::Sha3_224),
        DigestKind::Sha3_256 => mac!(sha3::Sha3_256),
        DigestKind::Sha3_384 => mac!(sha3::Sha3_384),
        DigestKind::Sha3_512 => mac!(sha3::Sha3_512),
        DigestKind::Crc32
        | DigestKind::Md5
        | DigestKind::Sha1
        | DigestKind::Crc32c
        | DigestKind::Blake3 => return None,
    })
}

/// HMAC-SHA-1 of `data` under `key` — the one algorithm [`hmac_of`] refuses,
/// reachable here by exactly one caller.
///
/// [`crate::totp`] needs it and no `Core\Hash` member offers it: RFC 6238's
/// interoperable algorithm is SHA-1, and an authenticator that quietly ignores
/// a stronger one produces codes that never verify. That module's own doc is
/// the home of why that is the right trade for a one-time code and wrong for a
/// digest a program chooses.
///
/// It lives here rather than there for [`crate::crypto::cipher`]'s reason:
/// there is one HMAC in `nvs-stdlib`, and a second `hmac::Hmac` construction in
/// another module would be a second set of decisions about key handling. What
/// is *not* offered is a `DigestKind::Sha1` route through [`hmac_of`] — the
/// strong set stays closed, so the exception is one named function with one
/// caller rather than a hole in the enum.
pub(crate) fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    use hmac::Mac as _;

    let mut mac = <hmac::Hmac<sha1::Sha1>>::new_from_slice(key)
        .expect("HMAC accepts a key of any length (RFC 2104 § 2)");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

/// HMAC-SHA-256 of `data` under `key`, as the fixed size it always is.
///
/// [`crate::jwt`] needs one algorithm and only one — `rule:security/algorithm-comes-from-the-key`'s "the
/// algorithm comes from the key, never from the token" is a statement about a
/// *closed* choice, and a member that could be handed a [`DigestKind`] would
/// have re-opened it one call site later. So this is the strong set's entry
/// point narrowed to the single row JWS calls `HS256`, and it goes through
/// [`hmac_of`] rather than constructing a second `hmac::Hmac`: unlike
/// [`hmac_sha1`], which exists because the strong set deliberately refuses its
/// algorithm, nothing here is an exception to anything.
///
/// The `expect` is the enum's own guarantee restated at a call that has already
/// chosen: `DigestKind::Sha256` is in [`STRONG`], so the `None` arm belongs to
/// the digests this function does not name.
pub(crate) fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mac = hmac_of(DigestKind::Sha256, key, data)
        .expect("SHA-256 is in the strong set, which is what `hmac_of` answers for");
    let mut tag = [0_u8; 32];
    tag.copy_from_slice(&mac);
    tag
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Hash::of(bytes|string $data, Digest $digest): bytes` — replacing
    /// `hash`, `md5`, `sha1`, `crc32` and `openssl_digest` at once.
    ///
    /// Total. Every one of [`DIGEST`]'s fifteen accepts every input, so there is
    /// nothing here to throw: PHP's `hash()` returning `false` for an
    /// unknown algorithm name has no analogue, because the algorithm is a
    /// closed enum rather than a string the caller might misspell — which is
    /// the single largest reason this member takes one.
    fn nvs_core_hash_of(_ctx, args: [2]) {
        let data = data_of(args, 0, "of")?;
        let kind = digest_kind(args, 1, "of")?;
        Ok(Value::bytes(NvsStr::new(&digest_of(kind, data))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Hash::hmac(bytes|string $data, secret bytes $key, StrongDigest $digest): bytes`
    /// — replacing `hash_hmac`, minus its `raw_output` flag and minus every
    /// algorithm that has no business authenticating anything ([`STRONG`]).
    ///
    /// The key is read and never held: it is borrowed out of the argument
    /// slot, mixed into the two padded blocks RFC 2104 describes, and gone
    /// when the helper returns. See this module's docs for what `secret` on
    /// that parameter does not yet buy.
    fn nvs_core_hash_hmac(_ctx, args: [3]) {
        let data = data_of(args, 0, "hmac")?;
        let key = bytes_of(args, 1, "hmac")?;
        let kind = digest_kind(args, 2, "hmac")?;
        // Unreachable from source: parameter 2 is `CoreTy::Union(STRONG)`, so a
        // weak case is `E0401: expected 'Core\Digest::Sha256|Core\Digest::
        // Sha384|Core\Digest::Sha512', found 'Core\Digest'` — probed with
        // `Core\Digest::Md5` and with a `mixed` binding. `hmac_of`'s own docs
        // own why the subset is then enforced a second time here.
        let mac = hmac_of(kind, key, data).ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Hash::hmac reached with `Core\\Digest::{kind:?}`, which is not a \
                 `StrongDigest`"
            ))
        })?;
        Ok(Value::bytes(NvsStr::new(&mac)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Hash::equals(bytes $a, bytes $b): bool` — replacing
    /// `hash_equals`, and **constant-time in the contents**.
    ///
    /// "Constant-time" here means what it means in `hash_equals`: the running
    /// time does not depend on *where* two equal-length operands first differ,
    /// so an attacker holding one of them cannot recover the other a byte at a
    /// time by timing the comparison. It does not hide the lengths — a length
    /// mismatch answers `false` immediately, exactly as PHP does, because a
    /// digest's length is a property of the algorithm and never a secret.
    ///
    /// `subtle::ConstantTimeEq`, not a hand-written loop: this module's docs
    /// say why three lines are a dependency.
    ///
    /// Takes `bytes` on both sides and no `string`, unlike the two members
    /// above. That is the point of the type: the operands of this comparison
    /// are digests, and a call that reached for it with two `string`s is
    /// comparing something else — probably hex, where a constant-time
    /// comparison of the *spelling* is not the guarantee the caller wanted.
    fn nvs_core_hash_equals(_ctx, args: [2]) {
        let left = bytes_of(args, 0, "equals")?;
        let right = bytes_of(args, 1, "equals")?;
        Ok(Value::bool(bool::from(left.ct_eq(right))))
    }
}

// ============================================================================
// `Core\Hash\Stream` — the incremental form
// ============================================================================

/// The receiver of one of [`STREAM`]'s instance members, with the stream's
/// algorithm read back out of it.
///
/// The two are decoded together because neither member has anything to do
/// without both, and because reading the `digest` slot is where a receiver
/// that is not one of this class's shows up.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is no object or whose `digest` slot
/// holds no [`DIGEST`] ordinal — compiled code can produce neither — and a
/// [`Fault::thrown`] for a stream [`nvs_core_hash_stream_finish`] has already
/// closed, which is the one of the three a program causes.
fn open_stream(value: Value, member: &str) -> Result<(*mut ObjHeader, DigestKind), Fault> {
    let receiver = crate::instance::receiver(value, &STREAM, member)?;
    let kind = kind_of(crate::instance::slot(receiver, DIGEST_SLOT).as_int()).ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Hash\\Stream::{member} received a stream whose `digest` slot is not one \
             `Core\\Hash::stream` wrote"
        ))
    })?;
    if crate::instance::slot(receiver, OPEN_SLOT).as_bool() != Some(true) {
        return Err(Fault::thrown(format!(
            "Core\\Hash\\Stream::{member}(): this stream is finished — a digest is final, so open \
             a new stream with Core\\Hash::stream rather than reusing this one"
        )));
    }
    Ok((receiver, kind))
}

nvs_runtime::nvs_helper! {
    /// `Core\Hash::stream(Digest $digest): Hash\Stream` — replacing
    /// `hash_init`, and the whole of `HashContext`.
    ///
    /// Takes the same `Core\Digest` [`nvs_core_hash_of`] does, including the
    /// three broken ones: an incremental checksum over a file being copied is
    /// exactly the interop case § 11 keeps `Crc32`, `Md5` and `Sha1` for.
    fn nvs_core_hash_stream(_ctx, args: [1]) {
        // The ordinal rather than the decoded kind, since a slot holds values
        // Novis holds — but decoded first, so a bad one is refused here rather
        // than at whichever `update` happens to read it back.
        let _kind = digest_kind(args, 0, "stream")?;
        Ok(crate::instance::build(
            &STREAM,
            [
                args[0],
                Value::array(NvsArray::new()),
                Value::bool(true),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$stream->update(bytes|string $data): void` — replacing `hash_update`.
    ///
    /// **Retains its argument rather than copying it.** The buffer is
    /// immutable-until-copied, so holding a reference to the caller's own is
    /// both the cheap reading and the correct one; this module's docs own what
    /// holding every chunk until `finish` spends and why the alternative is not
    /// available.
    fn nvs_core_hash_stream_update(_ctx, args: [2]) {
        let (receiver, _) = open_stream(args[0], "update")?;
        // Refused before the retain, so a `mixed` that reached here leaves the
        // stream exactly as it found it.
        let _ = data_of(args, 1, "update")?;
        #[expect(
            unsafe_code,
            reason = "the chunk array takes over a reference of its own, and \
                      the argument's belongs to the caller"
        )]
        unsafe {
            args[1].retain();
        }
        crate::identity_store::edit(receiver, CHUNKS_SLOT, &STREAM, "update", |chunks| {
            chunks.append(args[1]);
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$stream->finish(): bytes` — replacing `hash_final`, and answering the
    /// same octets [`nvs_core_hash_of`] would over the concatenation.
    ///
    /// **Closes the stream and releases its chunks.** A second `finish`, and
    /// any `update` after this one, throws — the rule this module's docs
    /// explain is written for the implementation this class will have rather
    /// than for the one it has.
    fn nvs_core_hash_stream_finish(_ctx, args: [1]) {
        let (receiver, kind) = open_stream(args[0], "finish")?;

        let mut data: Vec<u8> = Vec::new();
        {
            let chunks = crate::identity_store::borrow(receiver, CHUNKS_SLOT, &STREAM, "finish")?;
            let mut from = 0_usize;
            while let Some(slot) = chunks.next_slot(from) {
                let held = chunks
                    .value_at(slot)
                    .expect("next_slot only names live entries");
                // `update` is the only writer of `CHUNKS_SLOT` and its one
                // parameter is `CoreTy::Union(DATA)` — `bytes|string` — so the
                // guard below is unreachable from source with no diagnostic to
                // name: a chunk carries `Tag::Bytes` or `Tag::Str`, the pair
                // answers for both, and no spelling puts a third tag here.
                let octets = held
                    .as_bytes()
                    .or_else(|| held.as_str_bytes())
                    .ok_or_else(|| {
                        Fault::fatal(format!(
                            "Core\\Hash\\Stream::finish found tag {} among its chunks, which \
                             only `update` writes",
                            held.tag_byte()
                        ))
                    })?;
                data.extend_from_slice(octets);
                from = slot + 1;
            }
        }

        let out = digest_of(kind, &data);
        crate::instance::set_slot(receiver, OPEN_SLOT, Value::bool(false));
        crate::identity_store::replace(receiver, CHUNKS_SLOT);
        Ok(Value::bytes(NvsStr::new(&out)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each algorithm against a published vector for it, in hex — the check
    /// that dispatch reaches what its case names, which no `.nvst` case can
    /// make for all fifteen without pinning the same constants twice.
    // covers: Core\Hash::of
    #[test]
    fn every_digest_matches_its_published_vector() {
        fn hex(octets: &[u8]) -> String {
            octets.iter().map(|b| format!("{b:02x}")).collect()
        }

        // "abc", the vector every one of these specifications uses.
        let abc = b"abc";
        assert_eq!(hex(&digest_of(DigestKind::Crc32, abc)), "352441c2");
        assert_eq!(
            hex(&digest_of(DigestKind::Md5, abc)),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha1, abc)),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha256, abc)),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha384, abc)),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
             8086072ba1e7cc2358baeca134c825a7"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha512, abc)),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha224, abc)),
            "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha512_224, abc)),
            "4634270f707b6a54daae7530460842e20e37ed265ceee9a43e8924aa"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha512_256, abc)),
            "53048e2681941ef99b2e29b76b4c7dabe4c2d0c634fc6d46e0e2f13107e7af23"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha3_224, abc)),
            "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha3_256, abc)),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha3_384, abc)),
            "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b2\
             98d88cea927ac7f539f1edf228376d25"
        );
        assert_eq!(
            hex(&digest_of(DigestKind::Sha3_512, abc)),
            "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e\
             10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0"
        );
        assert_eq!(hex(&digest_of(DigestKind::Crc32c, abc)), "364b3fb7");
        assert_eq!(
            hex(&digest_of(DigestKind::Blake3, abc)),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    /// The two checksums differ in polynomial and in nothing else observable —
    /// four octets each, big-endian, and never equal for a subject either one
    /// would be reached for.
    #[test]
    fn the_two_checksums_are_the_same_shape_and_different_answers() {
        for subject in [&b""[..], &b"abc"[..], &b"123456789"[..]] {
            let (iso, castagnoli) = (
                digest_of(DigestKind::Crc32, subject),
                digest_of(DigestKind::Crc32c, subject),
            );
            assert_eq!(iso.len(), 4);
            assert_eq!(castagnoli.len(), 4);
            assert_eq!(iso == castagnoli, subject.is_empty());
        }
    }

    /// RFC 4231's test case 2 — the one with an ASCII key short enough to read
    /// — for each of [`STRONG`]'s ten, and `None` for everything else.
    ///
    /// That RFC publishes SHA-224, SHA-256, SHA-384 and SHA-512 only; the
    /// other six are the same message and key through PHP 8.5's `hash_hmac`,
    /// which reproduces all four of the RFC's own on this machine and is
    /// therefore an oracle for the six it extends to.
    // covers: Core\Hash::hmac
    #[test]
    fn hmac_matches_rfc_4231_and_refuses_a_weak_digest() {
        let (key, data) = (&b"Jefe"[..], &b"what do ya want for nothing?"[..]);
        let hex =
            |octets: Octets| -> String { octets.iter().map(|b| format!("{b:02x}")).collect() };

        assert_eq!(
            hex(hmac_of(DigestKind::Sha256, key, data).unwrap()),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha384, key, data).unwrap()),
            "af45d2e376484031617f78d2b58a6b1b9c7ef464f5a01b47e42ec3736322445e\
             8e2240ca5e69e2c78b3239ecfab21649"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha512, key, data).unwrap()),
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea250554\
             9758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );

        assert_eq!(
            hex(hmac_of(DigestKind::Sha224, key, data).unwrap()),
            "a30e01098bc6dbbf45690f3a7e9e6d0f8bbea2a39e6148008fd05e44"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha512_224, key, data).unwrap()),
            "4a530b31a79ebcce36916546317c45f247d83241dfb818fd37254bde"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha512_256, key, data).unwrap()),
            "6df7b24630d5ccb2ee335407081a87188c221489768fa2020513b2d593359456"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha3_224, key, data).unwrap()),
            "7fdb8dd88bd2f60d1b798634ad386811c2cfc85bfaf5d52bbace5e66"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha3_256, key, data).unwrap()),
            "c7d4072e788877ae3596bbb0da73b887c9171f93095b294ae857fbe2645e1ba5"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha3_384, key, data).unwrap()),
            "f1101f8cbf9766fd6764d2ed61903f21ca9b18f57cf3e1a23ca13508a93243ce\
             48c045dc007f26a21b3f5e0e9df4c20a"
        );
        assert_eq!(
            hex(hmac_of(DigestKind::Sha3_512, key, data).unwrap()),
            "5a4bfeab6166427c7a3647b747292b8384537cdb89afb3bf5665e4c5e709350b\
             287baec921fd7ca0ee7a0c31d022a95e1fc92ba9d77df883960275beb4e62024"
        );

        for weak in [
            DigestKind::Crc32,
            DigestKind::Md5,
            DigestKind::Sha1,
            DigestKind::Crc32c,
            DigestKind::Blake3,
        ] {
            assert!(hmac_of(weak, key, data).is_none(), "{weak:?} is not strong");
        }
    }

    /// [`STRONG`] and [`hmac_of`] name the same cases, which is the pair
    /// that would otherwise drift: the union is what a program is refused by
    /// and the `match` is what the runtime is refused by, and a case added to
    /// one alone is either surface with no implementation or an implementation
    /// nothing can reach.
    #[test]
    fn the_strong_subset_and_its_dispatch_agree() {
        let named: Vec<&str> = STRONG
            .iter()
            .map(|ty| match ty {
                CoreTy::EnumCase(enum_name, case) => {
                    assert_eq!(*enum_name, DIGEST_NAME);
                    *case
                }
                other => panic!("`STRONG` holds {other:?}, which is not an enum case type"),
            })
            .collect();

        for (case, value) in DIGEST.cases {
            let kind = digest_kind(&[Value::int(*value)], 0, "test").expect("a declared case");
            assert_eq!(
                hmac_of(kind, b"k", b"d").is_some(),
                named.contains(case),
                "`Core\\Digest::{case}` is in `STRONG` xor its HMAC dispatch"
            );
        }
    }

    /// The member answers by content and length: a length mismatch is `false`
    /// rather than a panic or a partial compare — see [`nvs_core_hash_equals`]
    /// for why the lengths are not hidden — and a difference in the last octet
    /// counts as much as one in the first.
    // covers: Core\Hash::equals
    #[test]
    fn equality_is_by_content_and_length() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let long = [0x5a_u8; 4096];
        let mut last_differs = long;
        last_differs[4095] = 0x5b;
        let table: [(&[u8], &[u8], bool); 7] = [
            (b"abc", b"abc", true),
            (b"abc", b"abd", false),
            (b"abc", b"ab", false),
            (b"", b"", true),
            (b"", b"\x00", false),
            (&long, &long, true),
            (&long, &last_differs, false),
        ];

        let mut agreed = 0_usize;
        for (left, right, want) in table {
            let args = [
                Value::bytes(NvsStr::new(left)),
                Value::bytes(NvsStr::new(right)),
            ];
            let answer = nvs_runtime::call(nvs_core_hash_equals, &mut ctx, &args)
                .expect("`equals` is total over two `bytes`")
                .as_bool();
            assert_eq!(answer, Some(want), "{left:?} against {right:?}");
            agreed += 1;
            for value in args {
                #[expect(
                    unsafe_code,
                    reason = "this test owns the one reference it built for each \
                              operand, and `equals` borrowed rather than consumed it"
                )]
                unsafe {
                    value.release();
                }
            }
        }
        assert_eq!(agreed, table.len());
    }
}
