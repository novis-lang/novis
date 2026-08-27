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
//! seam ADR 0009 draws everywhere else, and the reason `Core\Encoding` exists
//! as a class rather than as members on `Core\Str`.
//!
//! A `crc32` is four octets, big-endian, so `toHex` of it reads as PHP's
//! `hash("crc32b", …)` does. PHP's own `crc32()` answers an `int` instead, and
//! that difference is deliberate: a checksum that is sometimes an integer and
//! sometimes a string is exactly the per-algorithm special case this member
//! exists to remove.
//!
//! # `secret`, and what a registry row cannot yet say
//!
//! § 11 writes `hmac`'s key parameter as `secret bytes $key`
//! ([ADR 0024](../../../../docs/adr/0024-secret-values.md)), and
//! [`crate::registry::CoreTy`] has no qualifier to carry that with — a
//! registry row states an atom, not a qualified type. The row below is
//! therefore a plain [`CoreTy::Bytes`], which is *narrower protection than the
//! spec promises*, not different behaviour: nothing about how the key is read
//! or held changes, only whether the checker refuses to let it reach a sink.
//! When qualifiers become expressible in a row, this parameter is the one that
//! wants it first. Recorded here rather than filed away because the row itself
//! looks complete.
//!
//! # Why these dependencies
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 4 asks
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
//! **What this spends:** one digest state on the stack per call — 200-odd
//! bytes at the largest, released before the member returns — plus one
//! `bytes` allocation of the digest's own width (4 to 64 octets), charged to
//! the request that asked for it. Nothing is held between calls.

use nvs_runtime::{Fault, NvsArray, NvsStr, ObjHeader, Value};
use sha2::Digest as _;
use subtle::ConstantTimeEq as _;

use crate::registry::{CoreClass, CoreEnum, CoreMethod, CoreTy};

// ============================================================================
// Registration — this class's rows, its enum, and where its symbols live
// ============================================================================

/// `Core\Hash`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Hash";

/// `Core\Digest`'s fully-qualified name, for [`DIGEST`] and for the case types
/// [`STRONG`] is built out of.
pub(crate) const DIGEST_NAME: &str = r"Core\Digest";

/// Spec § 11's `Digest` — **every** algorithm, the two broken ones included,
/// ordered weakest first so that [`STRONG`] is a contiguous tail.
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
    ],
};

/// Spec § 11's `StrongDigest` — the closed subset [`nvs_core_hash_hmac`]
/// declares, so `Hash::hmac($m, $k, Digest::Md5)` does not compile.
///
/// A **union of case types** ([ADR 0047](../../../../docs/adr/0047-literal-and-enum-case-types.md)
/// § 3) rather than a second enum, which is the whole reason
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
const STRONG: &[CoreTy] = &[
    CoreTy::EnumCase(DIGEST_NAME, "Sha256"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha384"),
    CoreTy::EnumCase(DIGEST_NAME, "Sha512"),
];

/// `bytes|string` — what both hashing members take, and the reason neither has
/// a text-flavoured twin.
///
/// Total in one direction and free ([ADR 0009](../../../../docs/adr/0009-string-and-bytes.md)
/// § 3): a `string` is valid UTF-8 and therefore already a valid byte
/// sequence, so [`data_of`] reads the same buffer either tag points at without
/// copying or validating anything.
const DATA: &[CoreTy] = &[CoreTy::Bytes, CoreTy::Str];

/// `Core\Hash`'s registry rows — § 11's `of`/`hmac`/`equals`.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "of",
            params: &[CoreTy::Union(DATA), CoreTy::Enum(DIGEST_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_hash_of",
        },
        CoreMethod {
            name: "hmac",
            params: &[CoreTy::Union(DATA), CoreTy::Bytes, CoreTy::Union(STRONG)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_hash_hmac",
        },
        CoreMethod {
            name: "equals",
            params: &[CoreTy::Bytes, CoreTy::Bytes],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_hash_equals",
        },
        CoreMethod {
            name: "stream",
            params: &[CoreTy::Enum(DIGEST_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(STREAM_NAME),
            symbol: "nvs_core_hash_stream",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
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
    methods: &[],
    instance: &[
        CoreMethod {
            name: "update",
            params: &[CoreTy::Union(DATA)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_hash_stream_update",
        },
        CoreMethod {
            name: "finish",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_hash_stream_finish",
        },
    ],
    slots: &["digest", "chunks", "open"],
    constants: &[],
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

/// `data` under `kind`, as the digest's own octets.
///
/// Total: there is no input any of these six refuses, and none of them has a
/// size limit short of the address space.
fn digest_of(kind: DigestKind, data: &[u8]) -> Vec<u8> {
    match kind {
        // Big-endian, so that `toHex` of the result reads the way PHP's
        // `hash("crc32b", …)` prints it — see this module's own docs.
        DigestKind::Crc32 => {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(data);
            hasher.finalize().to_be_bytes().to_vec()
        }
        DigestKind::Md5 => md5::Md5::digest(data).to_vec(),
        DigestKind::Sha1 => sha1::Sha1::digest(data).to_vec(),
        DigestKind::Sha256 => sha2::Sha256::digest(data).to_vec(),
        DigestKind::Sha384 => sha2::Sha384::digest(data).to_vec(),
        DigestKind::Sha512 => sha2::Sha512::digest(data).to_vec(),
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
fn hmac_of(kind: DigestKind, key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    use hmac::Mac as _;

    /// One algorithm's HMAC, written once. A macro rather than a generic
    /// function because `hmac::Hmac<D>`'s bound set is a dozen `typenum`
    /// obligations about block sizes, none of which says anything a reader of
    /// this module needs to know — and all three instantiations are named on
    /// the next three lines anyway, so the generality bought nothing.
    ///
    /// `new_from_slice` is infallible for HMAC, whose key may be any length at
    /// all: RFC 2104 § 2 hashes a long one down and zero-pads a short one.
    macro_rules! mac {
        ($digest:ty) => {{
            let mut mac = <hmac::Hmac<$digest>>::new_from_slice(key)
                .expect("HMAC accepts a key of any length (RFC 2104 § 2)");
            mac.update(data);
            mac.finalize().into_bytes().to_vec()
        }};
    }

    Some(match kind {
        DigestKind::Sha256 => mac!(sha2::Sha256),
        DigestKind::Sha384 => mac!(sha2::Sha384),
        DigestKind::Sha512 => mac!(sha2::Sha512),
        DigestKind::Crc32 | DigestKind::Md5 | DigestKind::Sha1 => return None,
    })
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Hash::of(bytes|string $data, Digest $digest): bytes` — replacing
    /// `hash`, `md5`, `sha1`, `crc32` and `openssl_digest` at once.
    ///
    /// Total. Every one of [`DIGEST`]'s six accepts every input, so there is
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
    /// make for all six without pinning the same constants twice.
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
    }

    /// RFC 4231's test case 2 — the one with an ASCII key short enough to read
    /// — for each of [`STRONG`]'s three, and `None` for everything else.
    #[test]
    fn hmac_matches_rfc_4231_and_refuses_a_weak_digest() {
        let (key, data) = (&b"Jefe"[..], &b"what do ya want for nothing?"[..]);
        let hex =
            |octets: Vec<u8>| -> String { octets.iter().map(|b| format!("{b:02x}")).collect() };

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

        for weak in [DigestKind::Crc32, DigestKind::Md5, DigestKind::Sha1] {
            assert!(hmac_of(weak, key, data).is_none(), "{weak:?} is not strong");
        }
    }

    /// [`STRONG`] and [`hmac_of`] name the same three cases, which is the pair
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

    /// A length mismatch is `false` rather than a panic or a partial compare —
    /// see [`nvs_core_hash_equals`] for why the lengths are not hidden.
    #[test]
    fn equality_is_by_content_and_length() {
        assert!(bool::from(b"abc".ct_eq(b"abc")));
        assert!(!bool::from(b"abc"[..].ct_eq(&b"abd"[..])));
        assert!(!bool::from(b"abc"[..].ct_eq(&b"ab"[..])));
        assert!(bool::from(b""[..].ct_eq(&b""[..])));
    }
}
