//! `Core\Crypto` — `rule:core-api/tier-roster`'s "AEAD only, no ECB, no unauthenticated CBC, no cipher-name-as-string",
//! as three members that take a key and a message and nothing else.
//!
//! § 3 places the class and states the roster's one rule; what belongs here is
//! which construction that rule picked, what the sealed bytes are, and why a
//! `secret` key crosses this surface without any of it being a laundering.
//!
//! # No cipher argument, for `Core\Password`'s reason one class over
//!
//! `openssl_encrypt($data, "aes-256-cbc", …)` names its primitive in a string,
//! which is how a program ends up with `aes-256-ecb` in one file and a
//! typo-silent fallback in another — the cipher is chosen by whichever call
//! site was copied last, and "encrypted" and "authenticated" become two
//! decisions a caller can get half right. Here the primitive is the library's:
//! [`seal`](nvs_core_crypto_seal) and [`open`](nvs_core_crypto_open) take a
//! message and a key, there is no mode, no padding and no IV parameter, and
//! there is no unauthenticated spelling to reach for. [`crate::password`]'s
//! module doc makes the same argument about cost parameters; this is that
//! argument applied to the primitive itself.
//!
//! # XChaCha20-Poly1305, and why the extended nonce
//!
//! The construction is XChaCha20-Poly1305: ChaCha20 keyed by HChaCha20 over a
//! **192-bit** nonce, with Poly1305 authenticating the ciphertext. RFC 8439's
//! original takes a 96-bit nonce, and 96 bits is the reason the extended
//! variant is here rather than it. A member with no nonce parameter must draw
//! one itself, and a 96-bit nonce drawn at random has a birthday bound around
//! 2^32 messages under one key — reachable by a busy application in a year,
//! and a repeat there loses confidentiality outright rather than degrading.
//! At 192 bits the same collision is not reachable by anything, which is what
//! makes "the caller never sees a nonce" a safe surface rather than a hidden
//! counter the caller has to keep. It is the same reasoning libsodium's
//! `crypto_secretbox` is built on, and the cost is 12 extra bytes per message
//! and one extra HChaCha20 block per call.
//!
//! AES-256-GCM is in this module too, and it is not a second default: it is the
//! **interop** cipher, and `rule:core-classes/crypto-interop-tier` is the whole
//! of why it is here. `A256GCM` is what a browser's WebCrypto encrypts with and
//! the one content encryption JWE has, so a runtime that cannot produce those
//! bytes cannot read what the other end wrote. Where the choice is free it
//! loses to XChaCha20-Poly1305 on two counts: a 96-bit nonce with the bound
//! above and no extended variant, and a software fallback that is a
//! constant-time bitslice several times slower than ChaCha20 on a machine
//! without AES-NI — of which a container host scheduling this runtime is still
//! one often enough to matter. So a program with Novis at both ends is *told*
//! to prefer the extended-nonce construction, as advice in a member's doc and
//! never as a default: nothing here picks a cipher for its caller.
//!
//! # What a sealed message is
//!
//! `nonce ‖ ciphertext ‖ tag` — [`NONCE_LEN`] octets, then as many as the
//! plaintext had, then [`TAG_LEN`]. The overhead is [`OVERHEAD`] octets flat,
//! and the layout is not a format anything else parses: nothing outside this
//! module reads a field of it, and a program that wants an interchange format
//! wants a protocol from `rule:security/protocol-roster`'s
//! roster rather than this member's output. Stating it here is so that the
//! *size* is predictable, not so that it is depended on.
//!
//! **The interop cipher's layout is the same shape and is a format on purpose.**
//! Its nonce is [`GCM_NONCE_LEN`] octets rather than [`NONCE_LEN`], so a sealed
//! message is `nonce ‖ ciphertext ‖ tag` with 12 in front and [`TAG_LEN`] at the
//! back — exactly what WebCrypto's `encrypt` answers with the IV it was handed
//! put before it, which is what lets the other end split at octet 12 and do
//! nothing else. [`GCM_OVERHEAD`] is that promise as a number, and
//! `rule:core-classes/crypto-interop-tier` is where it is made.
//!
//! **What this spends:** one buffer the size of the message plus 16 octets per
//! `seal`, one the size of the plaintext per `open`, both allocated inside the
//! call and charged to the request by `nvs_runtime::budget` like every other
//! `Core` buffer, and both passed through `nvs_runtime::affordable` first. The
//! cipher state itself is a few hundred bytes on the stack. Nothing is held
//! between calls, so it is O(in-flight messages).
//!
//! # A forgery throws, and every way of not being authentic throws the same
//!
//! `open` answers the plaintext or it throws; there is no `false` and no
//! `?bytes`, which is `rule:core-api/shape-rules`'s
//! rule and, here, the whole point of the class — an unauthenticated mode
//! would have handed back plausible rubbish for `examples/crypto.nvs`'s
//! one-byte truncation, and the last line of that fixture is what an AEAD is
//! for.
//!
//! **One message for every failure of authenticity.** A tag that does not
//! verify, a buffer too short to hold a nonce and a tag, and a key that is
//! simply the wrong one are one `RuntimeError` with one sentence, because
//! telling them apart is telling an attacker which half of a forgery attempt
//! landed. The exception is a key of the wrong *length*, which is a program
//! bug rather than a message — a `bytes` that was never a key — and is a
//! `LogicError` that names the length it wanted and not the bytes it got.
//!
//! # The key is a `secret bytes` in the signature, and nothing is laundered
//!
//! [`generateKey`](nvs_core_crypto_generate_key) answers
//! [`CoreTy::SecretBytes`] and both other members declare
//! [`CoreTy::SecretBlob`], which is a **qualifier written into the row** rather
//! than one of `rule:security/unclassified-parameter-refuses-tainted`'s classifications — that variant's own docs are the
//! home of the difference. Two things follow, and they are the reason this
//! class was worth waiting for the spelling. A program cannot put a generated
//! key in a plain `bytes` variable: the assignment narrows a qualifier and the
//! checker refuses it, so a key stays out of `echo`, out of a log and out of a
//! `Throwable` message by construction rather than by review. And a `secret`
//! key reaches `seal` without any member removing the mark, so `Core\Crypto`
//! writes no [`Qual::Reveal`] and `rule:core-classes/secret-reveal`'s launderer roster stays the two
//! classes `nvs_types`'
//! `reveal_and_the_password_helpers_are_the_only_launderers_of_secret` closes
//! it at.
//!
//! **Sealing a `secret` message is a written call, not a free pass.** The
//! `$message` parameter is an ordinary classified `bytes`, so a `secret bytes`
//! plaintext is refused there exactly as it is at every other `Core` member,
//! and a program that means to encrypt one writes
//! `Core\Secret::revealBytes($plaintext, "sealed under a key the store cannot
//! read")`. That reads like friction and is the mechanism: turning a secret
//! into bytes that leave this process is the one event `rule:security/secret-qualifier` exists to make
//! greppable, and encryption is not an exemption from it — it is the case it
//! was written for.
//!
//! # This construction has one home, and two classes are on the near side of it
//!
//! [`cipher`], [`seal_under`] and [`open_under`] are `pub(crate)`, and
//! [`crate::signed_cookie`] — `rule:security/protocol-roster`
//! 's first roster entry — is their second caller. That is what makes a
//! signed cookie *this* AEAD with a key ring over it rather than a second
//! construction with its own nonce policy and its own opinion about tags: there
//! is exactly one `XChaCha20Poly1305::new_from_slice` in `nvs-stdlib`, and
//! everything above reaches it through those three functions. Nothing outside
//! this module reads a *field* of an XChaCha-sealed message, which is the
//! sentence above and is still true — a caller gets the whole buffer or
//! nothing.
//!
//! The interop cipher has the same three — [`gcm_cipher`], [`gcm_seal_under`]
//! and [`gcm_open_under`] — and the nonce is where they differ: this one is an
//! argument rather than a draw. A member draws one through
//! [`crate::random::draw`] before it calls, `Core\Jwe` supplies the one it has
//! already written into a token's header, and a test replays WebCrypto's own
//! input so the two implementations can be compared octet for octet. A nonce is
//! never reused under one key, which is the caller's obligation here and the
//! reason the argument is a fixed-width array rather than a slice.
//!
//! # The nonce is drawn through `Core\Random`'s seam
//!
//! `chacha20poly1305`'s `getrandom` feature is off in `Cargo.toml`, for
//! [`crate::password`]'s reason and the same one again: every draw in `Core`
//! goes through [`crate::random::draw`], whose own doc comment is the home of
//! why, so this tree has one CSPRNG and a `#[Test(seed: …)]` reproduces a
//! sealed message byte for byte along with every other draw the test made.
//!
//! # Two derivations, and PBKDF2's bounds are checked before the first HMAC
//!
//! [`derive_key`] is PBKDF2-HMAC-SHA256 and [`expand_key`] is HKDF-SHA256, both
//! answering [`DERIVED_LEN`] octets, which is [`KEY_LEN`]: a derivation here
//! answers a key for one of the ciphers above and there is nothing longer to
//! key. They are not interchangeable. PBKDF2 stretches something a person chose
//! and is slow on purpose; HKDF spreads something already uniform — the raw
//! output of a key agreement, which is a coordinate rather than a key — and is
//! two HMAC runs.
//!
//! The iteration count is the caller's, because the other end chose it, and it
//! is refused outside [`MIN_ITERATIONS`]`..=`[`MAX_ITERATIONS`] with a salt
//! shorter than [`MIN_SALT_LEN`] refused beside it, all of it before a single
//! HMAC runs. The floor is the ordinary one. **The ceiling is the load-bearing
//! half**: JWE's PBES2 takes its `p2c` from a token an attacker wrote, so
//! without a ceiling one token buys unbounded CPU.
//! `rule:security/jwe-compact-subset` is the rule, and these four constants are
//! its one implementation — the bound is written here rather than twice, and
//! `Core\Jwe` reads them rather than repeating the numbers.
//!
//! # Two curves agree, and what a peer sent is checked before it is multiplied
//!
//! [`agree_x25519`] and [`agree_p256`] are the roster's key agreements, and each
//! answers a **coordinate rather than a key**: the low bits of an x-coordinate
//! are not uniform, so either answer goes to [`expand_key`] and to nothing else,
//! which is the section above's split between the two derivations doing its
//! work. X25519 is the one to prefer where both ends are being written now;
//! P-256 is here for `rule:core-classes/crypto-interop-tier`'s reason and only
//! that one — it is the curve every browser's WebCrypto ships and the one JWE's
//! ECDH-ES meets in the field.
//!
//! The two curves check a peer's point in different places, because the attack
//! is in a different place. Every 32-octet string is a well-formed X25519
//! u-coordinate, so there is nothing to read and the check is on the way *out*:
//! a point of small order drives the shared secret to zero whatever the private
//! scalar is — a peer choosing the key for both ends — and [`agree_x25519`]
//! refuses that answer rather than expanding it. P-256 has the opposite shape:
//! a point that is not on the curve is the invalid-curve attack, which recovers
//! a private scalar a few bits per exchange, so [`read_p256_point`] is where an
//! encoding becomes a point at all and nothing here multiplies by anything it
//! did not read.
//!
//! # Two pieces JWE needs, and neither is ever a member
//!
//! [`wrap_key`]/[`unwrap_key`] are RFC 3394's key wrap and [`concat_kdf`] is
//! RFC 7518 § 4.6.2's derivation, and `rule:security/jwe-compact-subset` is the
//! whole of why either is in this tree: PBES2 wraps a content key under what a
//! password derived, and ECDH-ES runs an agreement's coordinate through that
//! derivation and not through [`expand_key`], because the other end is a browser
//! and the browser does what the RFC says. Neither is reachable from source —
//! there is no member and no row — for the reason the roster is closed at all: a
//! program with a key wrap in reach writes the protocol that goes around it, and
//! that protocol is the thing `Core\Jwe` exists to have written once.
//!
//! The derivation is **not** HKDF wearing a different name. It is SHA-256 over a
//! counter, the secret and a length-prefixed context, with no extract step,
//! which is weaker where the secret is not uniform and exactly right where it is
//! a curve coordinate. [`expand_key`] stays the one a program reaches.

use aes_gcm::Aes256Gcm;
use aes_kw::{KwAes128, KwAes256};
use chacha20poly1305::aead::{Aead, Nonce};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use p256::PublicKey as P256PublicKey;
use p256::SecretKey as P256SecretKey;
use p256::ecdh::diffie_hellman;
use rand::Rng;
// The 0.11 line of `sha2`, because `hkdf` and `pbkdf2` are generic over the
// digest family's 0.11 traits and `Core\Hash`'s `sha2` is the 0.10 one. The
// root manifest's row is where both majors being in this tree is argued.
use sha2_v11::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Crypto";

/// A key's length in octets — XChaCha20-Poly1305's only key size and AES-256's,
/// so this is the constructions' number rather than a choice of ours, and one
/// generated key keys either of them.
///
/// It is also the length [`crate::keyring`] holds every entry of a key ring
/// to, including at a door whose primitive would take any length; that module
/// doc is where the reason for a ring being one ring is written.
pub(crate) const KEY_LEN: usize = 32;

/// The nonce's length in octets, and the module doc's *why the extended nonce*
/// section is the whole of why it is 24 and not 12.
const NONCE_LEN: usize = 24;

/// An authentication tag's length in octets — Poly1305's and AES-GCM's alike,
/// which is what lets one constant stand at the back of both layouts.
const TAG_LEN: usize = 16;

/// What a sealed message costs over its plaintext, and the shortest buffer
/// [`nvs_core_crypto_open`] could authenticate — a nonce and a tag with an
/// empty message between them.
const OVERHEAD: usize = NONCE_LEN + TAG_LEN;

// Everything from here to `expand_key` is reached by its own tests and by
// nothing else yet: the `Core` rows that call it are stage 4's, and the
// `#[expect]` is what will fail the day they land and this stops being true.
// It is `cfg_attr(not(test), …)` because under `cfg(test)` the tests are the
// caller, so the lint does not fire and an unconditional expectation would be
// the unfulfilled one.

/// AES-GCM's nonce length in octets, and an interop number rather than a
/// choice: it is what WebCrypto emits as an IV and what a JWE header carries.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const GCM_NONCE_LEN: usize = 12;

/// What an AES-GCM sealed message costs over its plaintext, and the shortest
/// buffer [`gcm_open_under`] could authenticate.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const GCM_OVERHEAD: usize = GCM_NONCE_LEN + TAG_LEN;

/// What a derivation answers, in octets — [`KEY_LEN`], for the module doc's
/// *two derivations* reason.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const DERIVED_LEN: usize = KEY_LEN;

/// The fewest PBKDF2 iterations this module will run.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const MIN_ITERATIONS: u32 = 100_000;

/// The most PBKDF2 iterations this module will run, and the bound that makes
/// PBES2 safe rather than the one that makes a password hard: the module doc's
/// *two derivations* section is why a ceiling exists at all.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const MAX_ITERATIONS: u32 = 2_000_000;

/// The shortest salt PBKDF2 will accept, in octets — enough that a table built
/// against one derivation is worthless against the next.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const MIN_SALT_LEN: usize = 16;

/// What a key agreement answers, in octets — X25519's u-coordinate and P-256's
/// x-coordinate are both this long, which is what lets one constant stand for
/// both and either answer reach [`expand_key`] unchanged.
///
/// It is [`KEY_LEN`] as a number and not as a meaning: a shared secret is a
/// coordinate, and the module doc's *two curves agree* section is why it is
/// never used as a key.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const SHARED_LEN: usize = 32;

/// The shorter key-encryption key [`wrap_key`] takes, in octets.
///
/// It is 128 bits because `PBES2-HS256+A128KW` names that width and the other
/// end implements the name, not because anything in this module encrypts under
/// AES-128: a wrap is not a cipher a program can reach, and the roster's own
/// ciphers are both 256-bit.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const KW_128_KEY_LEN: usize = 16;

/// What a wrapped [`KEY_LEN`]-octet key is, in octets — RFC 3394 adds one
/// 64-bit semiblock, which is the integrity check an unwrap verifies.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) const WRAPPED_LEN: usize = KEY_LEN + 8;

/// `rule:core-api/tier-roster`'s AEAD-only surface, as three rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "generateKey",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::SecretBytes,
            symbol: "nvs_core_crypto_generate_key",
            doc: Some(&GENERATE_KEY_DOC),
        },
        CoreMethod {
            name: "seal",
            names: &["message", "key"],
            // The message is contagious on the `tainted` axis — the ciphertext
            // is made of it — and the key is neutral, because not one octet of
            // a key reaches the answer and its provenance says nothing about
            // the message's.
            params: &[
                CoreTy::Blob(Qual::Contagious),
                CoreTy::SecretBlob(Qual::Neutral),
            ],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_crypto_seal",
            doc: Some(&SEAL_DOC),
        },
        CoreMethod {
            name: "open",
            names: &["sealed", "key"],
            params: &[
                CoreTy::Blob(Qual::Contagious),
                CoreTy::SecretBlob(Qual::Neutral),
            ],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_crypto_open",
            doc: Some(&OPEN_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Crypto::generateKey`'s reference card — `rule:core-api/reference-card`.
const GENERATE_KEY_DOC: MethodDoc = MethodDoc {
    short: "Draws a fresh key for `seal` and `open` from the same CSPRNG `Core\\Random` uses. \
            There is no key size argument: the construction has one key size.",
    params: &[],
    ret: "32 octets as a `secret bytes`. The qualifier is part of the type, so the key cannot \
          be assigned to a plain `bytes`, echoed, logged or put in a `Throwable` message.",
    errors: &[],
};

/// `Core\Crypto::seal`'s reference card — `rule:core-api/reference-card`.
const SEAL_DOC: MethodDoc = MethodDoc {
    short: "Encrypts and authenticates `$message` under `$key` with XChaCha20-Poly1305, drawing \
            a fresh nonce per call. There is no cipher, mode, padding or IV argument — the \
            primitive is this library's, and every message it produces is authenticated.",
    params: &[
        ParamDoc {
            name: "message",
            desc: "The plaintext. A `secret` is refused here: sealing one is a written \
                   `Core\\Secret::revealBytes` call, which is what makes it greppable.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "A 32-octet key, as `generateKey` answers one.",
            shape: &[],
        },
    ],
    ret: "The sealed message — 40 octets longer than `$message`, and different on every call \
          for the same inputs, because each draws its own nonce.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` is not 32 octets long — a `bytes` that was never a key.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "This process cannot spare a buffer the size of the sealed message.",
        },
    ],
};

/// `Core\Crypto::open`'s reference card — `rule:core-api/reference-card`.
const OPEN_DOC: MethodDoc = MethodDoc {
    short: "Authenticates `$sealed` under `$key` and answers the plaintext, or throws. A \
            message altered by one octet is refused rather than decrypted into whatever is \
            left of it, which is the whole reason the roster is AEAD only.",
    params: &[
        ParamDoc {
            name: "sealed",
            desc: "A message `seal` produced.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The 32-octet key `$sealed` was sealed under.",
            shape: &[],
        },
    ],
    ret: "The original plaintext, byte for byte.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` is not 32 octets long — a `bytes` that was never a key.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$sealed` is not an authentic message under `$key` — it was altered, it is \
                   too short to be one at all, or the key is the wrong one. The three are one \
                   message on purpose: telling them apart tells a forger which half landed.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_crypto_generate_key" => (nvs_core_crypto_generate_key as *const ()).cast(),
        "nvs_core_crypto_seal" => (nvs_core_crypto_seal as *const ()).cast(),
        "nvs_core_crypto_open" => (nvs_core_crypto_open as *const ()).cast(),
        _ => return None,
    })
}

/// The `bytes` in slot `index`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: every parameter of this class is a
/// `bytes`, so a value of another tag is a compiled-code bug rather than
/// anything a program can write.
fn bytes_of<'a>(args: &'a [Value], index: usize, member: &str) -> Result<&'a [u8], Fault> {
    args[index].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `bytes`, got tag {}",
            args[index].tag_byte()
        ))
    })
}

/// The construction keyed by `key`, or `None` for a `bytes` that is not a key.
///
/// The one place in this tree a key becomes a cipher. [`crate::signedcookie`]
/// reads it too, which is what makes `rule:security/protocol-roster`'s cookie entry *this*
/// construction with a key ring over it rather than a second one: there is one
/// `XChaCha20Poly1305::new_from_slice` in `nvs-stdlib` and both classes are on
/// the near side of it.
pub(crate) fn cipher(key: &[u8]) -> Option<XChaCha20Poly1305> {
    XChaCha20Poly1305::new_from_slice(key).ok()
}

/// The `LogicError` a key of the wrong length earns, in one sentence for every
/// member that takes a key — whether it keys [`cipher`] or keys a MAC.
///
/// `who` is the member, spelled `Core\Class::member`, and `param` the argument
/// as the caller wrote it — `$key` here and `$keys[0]` wherever
/// [`crate::keyring`] is walking a ring, whose entries are what get keyed.
pub(crate) fn wrong_key_length(who: &str, param: &str, got: usize) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "{who}(): {param} is {got} octets, and a key is {KEY_LEN} — \
             Core\\Crypto::generateKey() answers one of the right length. The value is \
             not quoted here, because a key does not belong in a log."
        ),
    )
}

/// Seals `message` under `cipher`, drawing its own nonce and prefixing it.
///
/// Factored out of [`nvs_core_crypto_seal`] for [`cipher`]'s reason: the answer
/// `Core\SignedCookie` puts in a cookie has to be the same bytes this member
/// produces, and the only way to guarantee that is for it to be produced here.
/// `who` names the member for the two throws.
///
/// # Errors
///
/// A `RuntimeError` when the sealed message is larger than this construction
/// can produce or than this process can hold — both unreachable from source,
/// and both explained where they are raised.
pub(crate) fn seal_under(
    ctx: &mut nvs_runtime::Ctx,
    cipher: &XChaCha20Poly1305,
    message: &[u8],
    who: &str,
) -> Result<Vec<u8>, Fault> {
    // The answer's size is known exactly here, so the policy seam is asked
    // once with the real number rather than twice with halves of it.
    let sealed_len = message.len().saturating_add(OVERHEAD);
    nvs_runtime::affordable(Some(sealed_len), who)?;

    let mut nonce = [0_u8; NONCE_LEN];
    crate::random::draw(ctx, |rng| rng.fill_bytes(&mut nonce));

    // Unreachable from source with no diagnostic to name: `aead::Error` is
    // deliberately opaque and carries no reason, and on this path there is
    // exactly one — a plaintext past ChaCha20's 256 GiB-per-nonce bound,
    // which no `bytes` a request can hold comes near under any memory cap.
    // A throw rather than an `expect` because what it reports is the world
    // saying no, which a request can catch.
    let body = cipher.encrypt(&XNonce::from(nonce), message).map_err(|_| {
        Fault::thrown(format!(
            "{who}(): the message could not be sealed — it is larger than this \
             construction can encrypt under one key"
        ))
    })?;

    let mut sealed = Vec::new();
    // Unreachable from source with no diagnostic to name, for the reason
    // above one layer down: the allocator refusing a buffer `affordable`
    // has already admitted, over a length `saturating_add` cannot have
    // wrapped.
    sealed.try_reserve_exact(sealed_len).map_err(|_| {
        Fault::thrown(format!(
            "{who}(): the sealed message is larger than any buffer this process could hold"
        ))
    })?;
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&body);
    Ok(sealed)
}

/// Opens `sealed` under `cipher`: the plaintext, or `None` for anything that is
/// not authentic under this key.
///
/// **`None` is one answer for every way of failing to be authentic** — a tag
/// that does not verify, a buffer too short to hold a nonce and a tag, and the
/// wrong key are indistinguishable to the caller, which is the module doc's
/// *a forgery throws* section as a return type. It is also what lets
/// `Core\SignedCookie` try a key ring: a caller that could tell "wrong key"
/// from "altered" apart would learn which of a rotated pair a forgery was aimed
/// at.
///
/// # Errors
///
/// A `RuntimeError` when this process cannot spare the plaintext's buffer.
/// `who` names the member for it.
pub(crate) fn open_under(
    cipher: &XChaCha20Poly1305,
    sealed: &[u8],
    who: &str,
) -> Result<Option<Vec<u8>>, Fault> {
    // `split_first_chunk` rather than a length check and a slice, so the
    // nonce arrives as a `[u8; NONCE_LEN]` and there is no second place
    // where it could be the wrong width.
    let Some((nonce, body)) = sealed.split_first_chunk::<NONCE_LEN>() else {
        return Ok(None);
    };
    if body.len() < TAG_LEN {
        return Ok(None);
    }

    nvs_runtime::affordable(Some(body.len() - TAG_LEN), who)?;
    Ok(cipher.decrypt(&XNonce::from(*nonce), body).ok())
}

/// AES-256-GCM keyed by `key`, or `None` for a `bytes` that is not a key.
///
/// [`cipher`]'s counterpart for the interop cipher, and the one place in this
/// tree an `Aes256Gcm` is built: `Core\Crypto`'s AES case and `Core\Jwe`'s
/// `A256GCM` are both on the near side of it, so what a browser reads back was
/// assembled by one piece of code either way.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn gcm_cipher(key: &[u8]) -> Option<Aes256Gcm> {
    Aes256Gcm::new_from_slice(key).ok()
}

/// Seals `message` under `cipher` with the nonce it is handed, prefixing it.
///
/// The nonce is an argument where [`seal_under`]'s is a draw, and the module
/// doc's *one home* section is why: a member draws one through
/// [`crate::random::draw`] and a token's header supplies one that is already
/// written down. It is never reused under one key — AES-GCM's 96 bits put the
/// birthday bound around 2^32 messages, which is the whole of why the other
/// construction exists.
///
/// # Errors
///
/// A `RuntimeError` when the sealed message is larger than this construction
/// can produce or than this process can hold — both unreachable from source,
/// for [`seal_under`]'s reasons.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn gcm_seal_under(
    cipher: &Aes256Gcm,
    nonce: &[u8; GCM_NONCE_LEN],
    message: &[u8],
    who: &str,
) -> Result<Vec<u8>, Fault> {
    let sealed_len = message.len().saturating_add(GCM_OVERHEAD);
    nvs_runtime::affordable(Some(sealed_len), who)?;

    // Unreachable from source, one bound lower than the other construction's:
    // GCM stops at 2^36 - 32 octets under one nonce, and no `bytes` a request
    // can hold comes near it under any memory cap.
    let body = cipher
        .encrypt(&Nonce::<Aes256Gcm>::from(*nonce), message)
        .map_err(|_| {
            Fault::thrown(format!(
                "{who}(): the message could not be sealed — it is larger than this \
                 construction can encrypt under one nonce"
            ))
        })?;

    let mut sealed = Vec::new();
    sealed.try_reserve_exact(sealed_len).map_err(|_| {
        Fault::thrown(format!(
            "{who}(): the sealed message is larger than any buffer this process could hold"
        ))
    })?;
    sealed.extend_from_slice(nonce);
    sealed.extend_from_slice(&body);
    Ok(sealed)
}

/// Opens `sealed` under `cipher`: the plaintext, or `None` for anything that is
/// not authentic under this key.
///
/// [`open_under`]'s counterpart, reading the layout [`gcm_seal_under`] writes,
/// and `None` is one answer for every way of not being authentic for that
/// function's reason — a caller that could tell a wrong key from an altered
/// message apart would learn which half of a forgery attempt landed.
///
/// # Errors
///
/// A `RuntimeError` when this process cannot spare the plaintext's buffer.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn gcm_open_under(
    cipher: &Aes256Gcm,
    sealed: &[u8],
    who: &str,
) -> Result<Option<Vec<u8>>, Fault> {
    let Some((nonce, body)) = sealed.split_first_chunk::<GCM_NONCE_LEN>() else {
        return Ok(None);
    };
    if body.len() < TAG_LEN {
        return Ok(None);
    }

    nvs_runtime::affordable(Some(body.len() - TAG_LEN), who)?;
    Ok(cipher.decrypt(&Nonce::<Aes256Gcm>::from(*nonce), body).ok())
}

/// PBKDF2-HMAC-SHA256 over `password` and `salt`, `iterations` times.
///
/// The derivation with no bounds on it. [`derive_key`] is this plus the two the
/// module doc states, and the split is what lets RFC 7914 § 11's published
/// vectors — one iteration, a four-octet salt — run against the same code a
/// member reaches rather than against a copy of it.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; DERIVED_LEN] {
    pbkdf2::pbkdf2_hmac_array::<Sha256, DERIVED_LEN>(password, salt, iterations)
}

/// A key stretched out of something a person chose, with both bounds checked
/// before the first HMAC.
///
/// The refusal is a `LogicError` because the count and the salt are arguments
/// the program passed, and a program passing 10 iterations has a bug rather
/// than bad luck. `Core\Jwe`'s PBES2 reads a `p2c` off the wire instead, where
/// the same number out of bounds is a verdict on the token: it compares against
/// [`MIN_ITERATIONS`] and [`MAX_ITERATIONS`] itself and reaches
/// [`pbkdf2_sha256`], so the bound has one home and two reports.
///
/// # Errors
///
/// A `LogicError` naming the bound that was missed. `who` is the member, spelled
/// `Core\Class::member`.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn derive_key(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    who: &str,
) -> Result<[u8; DERIVED_LEN], Fault> {
    if !(MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{who}(): $iterations is {iterations}, and this derivation runs \
                 {MIN_ITERATIONS} to {MAX_ITERATIONS} — under the floor the answer is cheap \
                 to attack, and over the ceiling one call is a denial of service against \
                 the process that made it"
            ),
        ));
    }
    if salt.len() < MIN_SALT_LEN {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{who}(): $salt is {} octets, and a salt is at least {MIN_SALT_LEN} — \
                 Core\\Random::bytes({MIN_SALT_LEN}) answers one, and it is stored beside the \
                 derived key rather than kept secret",
                salt.len()
            ),
        ));
    }

    Ok(pbkdf2_sha256(password, salt, iterations))
}

/// HKDF-SHA256 over `material`, answering [`DERIVED_LEN`] octets.
///
/// The other derivation, for material that is already uniform: extract with
/// `salt`, then expand under `info`, which is the context string separating two
/// keys derived from one secret. An empty `salt` is RFC 5869's own default of a
/// zero-filled one, since HMAC pads either to the same block. Handing a password
/// to this rather than to [`derive_key`] is the mistake the module doc names,
/// and no type here can catch it.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn expand_key(material: &[u8], salt: &[u8], info: &[u8]) -> [u8; DERIVED_LEN] {
    let mut key = [0_u8; DERIVED_LEN];
    Hkdf::<Sha256>::new(Some(salt), material)
        .expand(info, &mut key)
        .expect("expand refuses only an output past 255 SHA-256 blocks, and this one is 32 octets");
    key
}

/// The secret two X25519 keys agree on, or `None` when it is the all-zero one.
///
/// RFC 7748's exchange, with the clamping the specification asks for applied to
/// a copy of `mine` inside the multiplication rather than to the stored scalar,
/// so a pair written back out answers the octets it was read from. `theirs` is
/// a raw u-coordinate, which is what the curve's only encoding is and what a
/// browser exports for this kind, so there is nothing to read and no reading
/// function beside this one.
///
/// The refusal is the module doc's *two curves agree* section: a point of small
/// order sends the shared secret to zero for every private scalar, so a peer
/// sending one picks the key for both ends, and a zero answer is refused rather
/// than expanded. It is an `Option` for [`cipher`]'s reason — whether that is a
/// verdict on what a peer sent or a bug in the program depends on where the
/// point came from, which the member knows and this seam does not, and
/// `rule:security/verification-throws-and-compares-in-constant-time` is what
/// makes the difference worth keeping.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn agree_x25519(
    mine: &[u8; KEY_LEN],
    theirs: &[u8; SHARED_LEN],
) -> Option<[u8; SHARED_LEN]> {
    let shared = StaticSecret::from(*mine).diffie_hellman(&X25519PublicKey::from(*theirs));
    shared.was_contributory().then(|| shared.to_bytes())
}

/// A P-256 public key read out of its SEC 1 encoding, or `None` when the octets
/// are not one.
///
/// **This is where all of P-256's validation happens**, and the module doc's
/// *two curves agree* section is why it happens at all: the length, the leading
/// tag, each coordinate being a field element and the point being *on the
/// curve* are all `from_sec1_bytes`, and the point at infinity is refused beside
/// them because `p256`'s `PublicKey` cannot hold the identity. A point that
/// passes a length check and fails this one is the invalid-curve attack, which
/// is why nothing in this module multiplies by a point it did not read here.
///
/// Both the compressed and the uncompressed encoding are read: a browser
/// exports the uncompressed form, and a JWK's two coordinates assemble into it.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn read_p256_point(encoded: &[u8]) -> Option<P256PublicKey> {
    P256PublicKey::from_sec1_bytes(encoded).ok()
}

/// The secret a P-256 scalar and a point already read agree on, or `None` when
/// `mine` is not a scalar of this curve.
///
/// ECDH over P-256: the answer is the x-coordinate of `mine × theirs`, which is
/// what WebCrypto's `deriveBits` answers over the same two keys and therefore
/// what JWE's ECDH-ES has to feed its derivation. `SecretKey::from_slice` is the
/// only refusal left here — zero and anything at or above the group order are
/// not scalars — because `theirs` came through [`read_p256_point`] and the group
/// has prime order, so no product of the two is the identity and there is no
/// all-zero answer to check for as there is on the other curve.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn agree_p256(mine: &[u8], theirs: &P256PublicKey) -> Option<[u8; SHARED_LEN]> {
    let secret = P256SecretKey::from_slice(mine).ok()?;
    let shared = diffie_hellman(secret.to_nonzero_scalar(), theirs.as_affine());
    let mut agreed = [0_u8; SHARED_LEN];
    agreed.copy_from_slice(shared.raw_secret_bytes().as_slice());
    Some(agreed)
}

/// A content key wrapped under a key-encryption key, or `None` when the wrapping
/// key is neither [`KW_128_KEY_LEN`] nor [`KEY_LEN`] octets.
///
/// RFC 3394, which is a cipher run over the key data with a fixed integrity
/// value woven through six passes, so an unwrap knows whether it unwrapped
/// anything. Only a [`KEY_LEN`]-octet key is wrapped, because the only thing
/// this wraps is a content key and every content key here is that long — a
/// general key wrap would be a second surface with nothing behind it.
///
/// The two key-encryption key widths are JWE's, not a choice: `A128KW` is what
/// `PBES2-HS256+A128KW` names, and the wider one is what the same wrap looks
/// like under a key this module's own derivations answer.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn wrap_key(kek: &[u8], key: &[u8; KEY_LEN]) -> Option<[u8; WRAPPED_LEN]> {
    let mut wrapped = [0_u8; WRAPPED_LEN];
    let wrote = match kek.len() {
        KW_128_KEY_LEN => KwAes128::new_from_slice(kek)
            .ok()?
            .wrap_key(key, &mut wrapped)
            .is_ok(),
        KEY_LEN => KwAes256::new_from_slice(kek)
            .ok()?
            .wrap_key(key, &mut wrapped)
            .is_ok(),
        _ => false,
    };

    wrote.then_some(wrapped)
}

/// The content key inside a wrap, or `None` for every way of it not being one.
///
/// A wrapping key of the wrong width, a wrap of the wrong length and a wrap
/// whose integrity value does not come back are one answer, because they are one
/// event at the door: the token did not open. `Core\Jwe` is where that becomes
/// the single sentence `rule:security/verification-throws-and-compares-in-constant-time`
/// asks for, and telling the three apart there would say which half of a forgery
/// landed.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn unwrap_key(kek: &[u8], wrapped: &[u8]) -> Option<[u8; KEY_LEN]> {
    if wrapped.len() != WRAPPED_LEN {
        return None;
    }

    let mut key = [0_u8; KEY_LEN];
    let unwrapped = match kek.len() {
        KW_128_KEY_LEN => KwAes128::new_from_slice(kek)
            .ok()?
            .unwrap_key(wrapped, &mut key)
            .is_ok(),
        KEY_LEN => KwAes256::new_from_slice(kek)
            .ok()?
            .unwrap_key(wrapped, &mut key)
            .is_ok(),
        _ => false,
    };

    unwrapped.then_some(key)
}

/// RFC 7518 § 4.6.2's derivation, filling `derived` from `shared` and a context.
///
/// SHA-256 over `counter ‖ shared ‖ algorithm ‖ party_u ‖ party_v ‖ keydatalen`,
/// one round per 32 octets wanted, with each of the three context fields carried
/// behind its own 32-bit length so no two contexts can concatenate to the same
/// input. `algorithm` is the JWE header's `enc` where the agreement is direct,
/// and the two party fields are `apu` and `apv`, which
/// `rule:security/jwe-compact-subset` holds empty.
///
/// The length wanted is `derived`'s, which is also what goes into the final
/// field, so a caller cannot ask for one length and label it another. The module
/// doc's *two pieces* section is why this exists beside [`expand_key`] rather
/// than instead of it.
#[cfg_attr(not(test), expect(dead_code, reason = "stage 4 registers the members"))]
pub(crate) fn concat_kdf(
    shared: &[u8],
    algorithm: &str,
    party_u: &[u8],
    party_v: &[u8],
    derived: &mut [u8],
) {
    /// One context field, behind the 32-bit length the derivation counts it by.
    fn prefixed(round: &mut Sha256, field: &[u8]) {
        let len = u32::try_from(field.len()).expect(
            "a context field comes out of a header this module's caller has already capped",
        );
        round.update(len.to_be_bytes());
        round.update(field);
    }

    let bits =
        u32::try_from(derived.len() * 8).expect("a derived key is at most a few hundred bits");
    for (index, block) in derived
        .chunks_mut(<Sha256 as Digest>::output_size())
        .enumerate()
    {
        let counter = u32::try_from(index + 1).expect("one round answers 32 octets");
        let mut round = Sha256::new();
        round.update(counter.to_be_bytes());
        round.update(shared);
        prefixed(&mut round, algorithm.as_bytes());
        prefixed(&mut round, party_u);
        prefixed(&mut round, party_v);
        round.update(bits.to_be_bytes());
        let whole = round.finalize();
        block.copy_from_slice(&whole[..block.len()]);
    }
}

/// The cipher keyed by slot 1, or the `LogicError` a wrong-length key earns.
///
/// Reachable from source despite the parameter's `secret bytes`: the qualifier
/// is about confidentiality and says nothing about length, and a plain `bytes`
/// of any size widens onto it. `Core\Random::bytes(8)` is the one-line witness.
fn keyed(args: &[Value], member: &str) -> Result<XChaCha20Poly1305, Fault> {
    let key = bytes_of(args, 1, member)?;
    cipher(key).ok_or_else(|| wrong_key_length(&format!("{NAME}::{member}"), "$key", key.len()))
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::generateKey(): secret bytes` — replacing
    /// `random_bytes(32)` written next to whichever cipher was chosen.
    ///
    /// [`crate::random::draw`] rather than the crate's own `generate`, so the
    /// one CSPRNG rule holds and a seeded test reproduces the key.
    fn nvs_core_crypto_generate_key(ctx, args: [0]) {
        let _ = args;
        let mut key = [0_u8; KEY_LEN];
        crate::random::draw(ctx, |rng| rng.fill_bytes(&mut key));
        Ok(Value::bytes(NvsStr::new(&key)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::seal(bytes $message, secret bytes $key): bytes` —
    /// replacing `openssl_encrypt` and `sodium_crypto_aead_*_encrypt`, with
    /// the cipher, the mode, the padding and the nonce all taken off the call.
    ///
    /// The nonce is drawn before anything is allocated and prefixed to the
    /// answer, which is what lets [`nvs_core_crypto_open`] take one argument
    /// where PHP's pair takes the IV back as a second.
    fn nvs_core_crypto_seal(ctx, args: [2]) {
        let message = bytes_of(args, 0, "seal")?;
        let cipher = keyed(args, "seal")?;
        let sealed = seal_under(ctx, &cipher, message, "Core\\Crypto::seal")?;
        Ok(Value::bytes(NvsStr::new(&sealed)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::open(bytes $sealed, secret bytes $key): bytes` —
    /// replacing `openssl_decrypt`, which answers `false` on a forgery when it
    /// notices one at all.
    ///
    /// Every way `$sealed` can fail to be an authentic message under `$key` is
    /// one throw with one sentence; the module doc's *a forgery throws* section
    /// is why, and it is a security property rather than a simplification.
    fn nvs_core_crypto_open(_ctx, args: [2]) {
        let sealed = bytes_of(args, 0, "open")?;
        let cipher = keyed(args, "open")?;

        // A buffer too short to hold a nonce and a tag, a tag that does not
        // verify and the wrong key are one `None` out of `open_under` and one
        // sentence here, deliberately not three.
        let plain = open_under(&cipher, sealed, "Core\\Crypto::open")?.ok_or_else(|| {
            Fault::thrown(
                "Core\\Crypto::open(): $sealed is not an authentic message under $key — it \
                 has been altered, it is too short to be one, or the key is not the one it \
                 was sealed under"
            )
        })?;

        Ok(Value::bytes(NvsStr::new(&plain)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::vectors as webcrypto;

    /// Every cipher this class can reach is authenticated — stage 4's first
    /// named check, asked of the construction rather than of a roster, because
    /// the roster is one entry and the claim is about what that entry does.
    ///
    /// Asserted by *forging*: a sealed message with one octet changed at every
    /// position in the tag, in the ciphertext and in the nonce has to be
    /// refused. An unauthenticated mode passes a round-trip test and fails
    /// every one of these, which is precisely the failure `rule:core-api/tier-roster`'s
    /// AEAD-only rule exists to make impossible.
    #[test]
    fn every_registered_cipher_is_an_aead() {
        let cipher = XChaCha20Poly1305::new_from_slice(&[7_u8; KEY_LEN])
            .expect("the construction's own key length");
        let nonce = XNonce::from([3_u8; NONCE_LEN]);
        let sealed = cipher
            .encrypt(&nonce, b"attack at dawn".as_ref())
            .expect("a short message seals");

        assert_eq!(sealed.len(), "attack at dawn".len() + TAG_LEN);
        assert_eq!(
            cipher.decrypt(&nonce, sealed.as_ref()).ok().as_deref(),
            Some(b"attack at dawn".as_ref()),
            "the round trip is the floor, not the claim"
        );

        for index in 0..sealed.len() {
            let mut forged = sealed.clone();
            forged[index] ^= 1;
            assert!(
                cipher.decrypt(&nonce, forged.as_ref()).is_err(),
                "one flipped bit at octet {index} of {} is refused",
                sealed.len()
            );
        }

        let truncated = &sealed[..sealed.len() - 1];
        assert!(
            cipher.decrypt(&nonce, truncated).is_err(),
            "examples/crypto.nvs's own tamper — one octet short — is refused"
        );

        let mut other_nonce = [3_u8; NONCE_LEN];
        other_nonce[0] ^= 1;
        assert!(
            cipher
                .decrypt(&XNonce::from(other_nonce), sealed.as_ref())
                .is_err(),
            "the nonce is authenticated too, which is what lets it travel in the clear"
        );
    }

    /// The lengths the module doc states, held against the construction rather
    /// than against each other — a constant that drifted would produce a class
    /// that still round-trips and whose sealed messages are the wrong size,
    /// and nothing a program can observe says which of the two is wrong.
    #[test]
    fn the_sealed_layout_is_the_one_the_docs_state() {
        let cipher = XChaCha20Poly1305::new_from_slice(&[1_u8; KEY_LEN])
            .expect("the construction's own key length");
        let sealed = cipher
            .encrypt(&XNonce::from([0_u8; NONCE_LEN]), b"".as_ref())
            .expect("the empty message seals");

        assert_eq!(sealed.len(), TAG_LEN, "an empty message is a bare tag");
        assert_eq!(OVERHEAD, NONCE_LEN + TAG_LEN);
        assert!(
            XChaCha20Poly1305::new_from_slice(&[0_u8; KEY_LEN - 1]).is_err(),
            "a short key is refused by the construction, which is what `keyed` reports"
        );
    }

    /// The hex a published vector is written in, as the octets it stands for.
    fn hex(text: &str) -> Vec<u8> {
        data_encoding::HEXLOWER
            .decode(text.as_bytes())
            .expect("a vector in this module is written in lower-case hex")
    }

    /// AES-256-GCM against the vectors published with the mode — McGrew and
    /// Viega's specification, Appendix B, test cases 13 to 16, which are the
    /// 256-bit-key half of the set NIST adopted the mode with.
    ///
    /// Run through [`gcm_seal_under`] rather than against the crate, because
    /// what needs pinning is the whole layout
    /// `rule:core-classes/crypto-interop-tier` promises — the nonce in front,
    /// the tag at the back, nothing between — and not that the crate computes
    /// GCM, which is its own test suite's job. Case 16 carries additional data,
    /// which this seam takes none of, so it is asserted against the
    /// construction: that is the path `Core\Jwe` reaches for a protected
    /// header, and the vector is what will pin it.
    #[test]
    fn aes_gcm_matches_the_vectors_published_with_the_mode() {
        let zeros = gcm_cipher(&[0_u8; KEY_LEN]).expect("a 32-octet key keys AES-256");
        let zero_nonce = [0_u8; GCM_NONCE_LEN];
        let who = "Core\\Crypto::seal";

        assert_eq!(
            gcm_seal_under(&zeros, &zero_nonce, b"", who).expect("the empty message seals"),
            [
                zero_nonce.as_slice(),
                &hex("530f8afbc74536b9a963b4f1c4cb738b"),
            ]
            .concat(),
            "case 13 — an empty message is its nonce and a bare tag"
        );
        assert_eq!(
            gcm_seal_under(&zeros, &zero_nonce, &[0_u8; 16], who).expect("one block seals"),
            [
                zero_nonce.as_slice(),
                &hex("cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919"),
            ]
            .concat(),
            "case 14 — one all-zero block under the same key"
        );

        let cipher = gcm_cipher(&hex(
            "feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308",
        ))
        .expect("a 32-octet key keys AES-256");
        let nonce = <[u8; GCM_NONCE_LEN]>::try_from(hex("cafebabefacedbaddecaf888").as_slice())
            .expect("the vector's IV is 12 octets");
        let message = hex(
            "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
             1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255",
        );

        let sealed = gcm_seal_under(&cipher, &nonce, &message, who).expect("the vector seals");
        assert_eq!(
            sealed,
            [
                nonce.as_slice(),
                &hex(
                    "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
                     8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad\
                     b094dac5d93471bdec1a502270e3cc6c"
                ),
            ]
            .concat(),
            "case 15 — 64 octets under a real key, no additional data"
        );
        assert_eq!(
            gcm_open_under(&cipher, &sealed, who)
                .expect("the plaintext is affordable")
                .as_deref(),
            Some(message.as_slice()),
            "and what this module sealed it opens"
        );

        let mut forged = sealed.clone();
        let tag_octet = forged.len() - 1;
        forged[tag_octet] ^= 1;
        assert!(
            gcm_open_under(&cipher, &forged, who)
                .expect("a forgery costs the same buffer")
                .is_none(),
            "one flipped bit in the tag is refused, which is what makes this an AEAD"
        );

        assert_eq!(
            cipher
                .encrypt(
                    &Nonce::<Aes256Gcm>::from(nonce),
                    chacha20poly1305::aead::Payload {
                        msg: &message[..60],
                        aad: &hex("feedfacedeadbeeffeedfacedeadbeefabaddad2"),
                    },
                )
                .expect("the vector seals"),
            hex(
                "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
                 8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662\
                 76fc6ece0f4e1768cddf8853bb2d551b"
            ),
            "case 16 — the same key and nonce with a header authenticated beside the message"
        );
    }

    /// The two derivations against their own specifications' vectors: PBKDF2
    /// against RFC 7914 § 11's SHA-256 pair, HKDF against RFC 5869 Appendix
    /// A.1, A.2 and A.3.
    ///
    /// Each published output is longer than [`DERIVED_LEN`], and comparing
    /// against its first 32 octets is exact rather than partial: both
    /// constructions are a chain of HMAC blocks, so the opening 32 octets of a
    /// 64- or 42-octet answer *are* the 32-octet answer.
    #[test]
    fn the_two_derivations_match_their_published_vectors() {
        assert_eq!(
            pbkdf2_sha256(b"passwd", b"salt", 1).as_slice(),
            hex("55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc"),
            "RFC 7914 § 11, one iteration"
        );
        assert_eq!(
            pbkdf2_sha256(b"Password", b"NaCl", 80_000).as_slice(),
            hex("4ddcd8f60b98be21830cee5ef22701f9641a4418d04c0414aeff08876b34ab56"),
            "RFC 7914 § 11, eighty thousand"
        );

        assert_eq!(
            expand_key(
                &[0x0b; 22],
                &hex("000102030405060708090a0b0c"),
                &hex("f0f1f2f3f4f5f6f7f8f9"),
            )
            .as_slice(),
            hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"),
            "RFC 5869 A.1, the basic SHA-256 case"
        );
        let counted: Vec<u8> = (0..=0x4f_u8).collect();
        let salt: Vec<u8> = (0x60..=0xaf_u8).collect();
        let info: Vec<u8> = (0xb0..=0xff_u8).collect();
        assert_eq!(
            expand_key(&counted, &salt, &info).as_slice(),
            hex("b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c"),
            "RFC 5869 A.2, eighty octets of each input"
        );
        assert_eq!(
            expand_key(&[0x0b; 22], b"", b"").as_slice(),
            hex("8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d"),
            "RFC 5869 A.3, no salt and no info"
        );
    }

    /// PBKDF2's two bounds at their edges, and the ceiling is the one that
    /// matters: PBES2 reads its `p2c` out of a token, so a derivation with no
    /// upper bound turns one token into unbounded CPU. Asserting the edges
    /// rather than the middle is what stops either number being widened
    /// quietly, which `rule:security/jwe-compact-subset` does not allow.
    #[test]
    fn a_derivation_refuses_a_count_or_a_salt_outside_its_bounds() {
        let salt = [7_u8; MIN_SALT_LEN];
        let who = "Core\\Crypto::deriveKey";

        assert!(
            derive_key(b"correct horse", &salt, MIN_ITERATIONS - 1, who).is_err(),
            "one under the floor is refused"
        );
        assert!(
            derive_key(b"correct horse", &salt, MAX_ITERATIONS + 1, who).is_err(),
            "one over the ceiling is refused, and refused without running"
        );
        assert!(
            derive_key(
                b"correct horse",
                &salt[..MIN_SALT_LEN - 1],
                MIN_ITERATIONS,
                who
            )
            .is_err(),
            "one octet short of a salt is refused"
        );
        assert_eq!(
            derive_key(b"correct horse", &salt, MIN_ITERATIONS, who).expect("the floor derives"),
            pbkdf2_sha256(b"correct horse", &salt, MIN_ITERATIONS),
            "inside the bounds it is the derivation above and nothing else"
        );
    }

    /// A 32-octet vector as the array that takes it — a Curve25519 scalar, a
    /// u-coordinate and a content key are all fixed-width here, and all the same
    /// width, while a vector is hex of whatever length it was written at.
    fn thirty_two(octets: &[u8]) -> [u8; KEY_LEN] {
        octets
            .try_into()
            .expect("the field this vector stands in is 32 octets")
    }

    /// Both agreements against the published evidence for their curve: RFC 7748
    /// § 6.1, which the document prints, and the frozen WebCrypto set, which is
    /// where a P-256 exchange another implementation actually performed comes
    /// from — `crate::tests::vectors` is the home of why those octets are read
    /// from the file rather than copied into this one.
    ///
    /// Each direction is asserted, because an exchange that agrees one way round
    /// and not the other is the argument-order slip a single assertion cannot
    /// see. What is pinned is the whole seam a member will reach — the clamping,
    /// the point validation, the coordinate that comes out — and not that either
    /// crate computes its curve, which is its own test suite's job.
    #[test]
    fn the_two_agreements_match_their_published_vectors() {
        let alice = thirty_two(&hex(
            "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
        ));
        let bob = thirty_two(&hex(
            "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb",
        ));
        let alice_public = thirty_two(&hex(
            "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a",
        ));
        let bob_public = thirty_two(&hex(
            "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f",
        ));
        let published = thirty_two(&hex(
            "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742",
        ));
        assert_eq!(
            agree_x25519(&alice, &bob_public),
            Some(published),
            "RFC 7748 § 6.1, Alice's scalar against Bob's public key"
        );
        assert_eq!(
            agree_x25519(&bob, &alice_public),
            Some(published),
            "RFC 7748 § 6.1, the same secret reached from the other end"
        );

        for vector in webcrypto::vectors("ecdh") {
            let name = webcrypto::text(vector, "/name");
            let mine = webcrypto::octets(vector, "/a/scalar");
            let theirs = webcrypto::octets(vector, "/b/scalar");
            let my_point = webcrypto::octets(vector, "/a/raw");
            let their_point = webcrypto::octets(vector, "/b/raw");
            let secret = webcrypto::octets(vector, "/secret");

            match webcrypto::text(vector, "/curve") {
                "X25519" => {
                    assert_eq!(
                        agree_x25519(&thirty_two(&mine), &thirty_two(&their_point))
                            .expect("a WebCrypto public key is contributory")
                            .as_slice(),
                        secret,
                        "{name}"
                    );
                    assert_eq!(
                        agree_x25519(&thirty_two(&theirs), &thirty_two(&my_point))
                            .expect("a WebCrypto public key is contributory")
                            .as_slice(),
                        secret,
                        "{name}, from the other end"
                    );
                }
                "P-256" => {
                    let their_point =
                        read_p256_point(&their_point).expect("a WebCrypto public key reads");
                    let my_point =
                        read_p256_point(&my_point).expect("a WebCrypto public key reads");
                    assert_eq!(
                        agree_p256(&mine, &their_point)
                            .expect("a WebCrypto private key is a scalar of its curve")
                            .as_slice(),
                        secret,
                        "{name}"
                    );
                    assert_eq!(
                        agree_p256(&theirs, &my_point)
                            .expect("a WebCrypto private key is a scalar of its curve")
                            .as_slice(),
                        secret,
                        "{name}, from the other end"
                    );
                }
                curve => panic!("the set carries {curve}, which this module does not agree over"),
            }
        }
    }

    /// Every point WebCrypto itself refused to agree under, refused here — and
    /// on each curve at the place the module doc says it is refused: X25519's
    /// small-order points at the answer, because the encoding is well formed and
    /// the secret is what is wrong, and P-256's off-curve point, point at
    /// infinity and short encoding at the read, before anything is multiplied.
    ///
    /// A refusal is a property of the point rather than of the scalar meeting
    /// it, so each case reuses the vector set's own private key for that curve
    /// rather than carrying one of its own.
    #[test]
    fn an_agreement_refuses_every_point_webcrypto_refuses() {
        let scalar = |curve: &str| {
            webcrypto::vectors("ecdh")
                .iter()
                .find(|vector| webcrypto::text(vector, "/curve") == curve)
                .map(|vector| webcrypto::octets(vector, "/a/scalar"))
                .expect("the set agrees over the curve it refuses points on")
        };

        for refusal in webcrypto::refusals("ecdh") {
            let name = webcrypto::text(refusal, "/name");
            let theirs = webcrypto::octets(refusal, "/theirs");
            match webcrypto::text(refusal, "/curve") {
                "X25519" => assert!(
                    agree_x25519(&thirty_two(&scalar("X25519")), &thirty_two(&theirs)).is_none(),
                    "{name}"
                ),
                "P-256" => assert!(read_p256_point(&theirs).is_none(), "{name}"),
                curve => panic!("the set carries {curve}, which this module does not agree over"),
            }
        }
    }

    /// The key wrap against RFC 3394 § 4.6 — the published case whose key data
    /// is the width every content key here is — and against the frozen
    /// WebCrypto set for both key-encryption key widths. The set is the only
    /// evidence for the narrow one: the RFC publishes no case that wraps 256
    /// bits under a 128-bit key, and `PBES2-HS256+A128KW` is exactly that case.
    ///
    /// The refusals are beside it because an unwrap is the half that meets a
    /// token: a wrap with an octet changed, one a semiblock short and one under
    /// a key of no width this wraps under are each `None`, which is the one
    /// answer the module doc says they share.
    #[test]
    fn the_key_wrap_matches_its_published_vectors_and_refuses_a_changed_wrap() {
        let kek = hex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
        let key = thirty_two(&hex(
            "00112233445566778899aabbccddeeff000102030405060708090a0b0c0d0e0f",
        ));
        let wrapped = wrap_key(&kek, &key).expect("a 256-bit key-encryption key wraps");
        assert_eq!(
            wrapped.as_slice(),
            hex("28c9f404c4b810f4cbccb35cfb87f8263f5786e2d80ed326cbc7f0e71a99f43bfb988b9b7a02dd21"),
            "RFC 3394 § 4.6, 256 bits of key data under a 256-bit key"
        );
        assert_eq!(
            unwrap_key(&kek, &wrapped),
            Some(key),
            "and the same wrap opens back to the key data"
        );

        for vector in webcrypto::vectors("aesKw") {
            let name = webcrypto::text(vector, "/name");
            let kek = webcrypto::octets(vector, "/kek");
            let key = thirty_two(&webcrypto::octets(vector, "/key"));
            assert_eq!(
                wrap_key(&kek, &key)
                    .expect("a WebCrypto key-encryption key is one of the two widths")
                    .as_slice(),
                webcrypto::octets(vector, "/wrapped"),
                "{name}"
            );
            assert_eq!(
                unwrap_key(&kek, &webcrypto::octets(vector, "/wrapped")),
                Some(key),
                "{name}, opened again"
            );
        }

        let mut changed = wrapped;
        changed[0] ^= 1;
        assert!(
            unwrap_key(&kek, &changed).is_none(),
            "one octet of the integrity value changed is refused"
        );
        assert!(
            unwrap_key(&kek, &wrapped[..WRAPPED_LEN - 1]).is_none(),
            "a wrap one octet short is refused before any key is built"
        );
        assert!(
            unwrap_key(&kek[..KEY_LEN - 1], &wrapped).is_none(),
            "a key-encryption key of no width this wraps under is refused"
        );
        assert!(
            wrap_key(&kek[..KW_128_KEY_LEN + 1], &key).is_none(),
            "and refused on the way in as well as on the way out"
        );
    }

    /// The derivation ECDH-ES runs, against RFC 7518 Appendix C, reached the way
    /// the protocol reaches it: the appendix's own two keys through
    /// [`agree_p256`], its published `Z` asserted on the way past, then the
    /// context it names and the key it prints.
    ///
    /// `tools/webcrypto-vectors.mjs` reproduces this same output before it
    /// writes the set, so the two implementations of one derivation are pinned
    /// to one published answer rather than to each other.
    ///
    /// The length is asserted to be *inside* the derivation, not applied after
    /// it: a 32-octet answer is not the opening 32 octets of a 64-octet one,
    /// which is the difference between a length that is bound into the input and
    /// one a caller could relabel.
    #[test]
    fn the_derivation_ecdh_es_runs_matches_rfc_7518_appendix_c() {
        let mine = hex("d3f3716913d4310a0026de741b3f18893afc8114f0c84682ba677e313a13988a");
        let theirs = read_p256_point(&hex(
            "04c1e349cb61ec70248ce801034c3834e1b88ebe1161cb25af38741f785fcfc4c4\
             7bc96708ef80952b53f8d2555fe72b841ed04588628b1d378a594939500ec9c9",
        ))
        .expect("the appendix's public key is a point on the curve");
        let shared = agree_p256(&mine, &theirs).expect("the appendix's private key is a scalar");
        assert_eq!(
            shared.as_slice(),
            hex("9e56d91d817135d372834283bf84269cfb316ea3da806a48f6daa7798cfe90c4"),
            "RFC 7518 Appendix C's Z, the agreement the derivation runs over"
        );

        let mut derived = [0_u8; 16];
        concat_kdf(&shared, "A128GCM", b"Alice", b"Bob", &mut derived);
        assert_eq!(
            derived.as_slice(),
            hex("56aa8deaf8236d205c2228cd71a7101a"),
            "RFC 7518 Appendix C's derived key"
        );

        let mut one_round = [0_u8; DERIVED_LEN];
        let mut two_rounds = [0_u8; DERIVED_LEN * 2];
        concat_kdf(&shared, "A256GCM", b"", b"", &mut one_round);
        concat_kdf(&shared, "A256GCM", b"", b"", &mut two_rounds);
        assert_ne!(
            one_round.as_slice(),
            &two_rounds[..DERIVED_LEN],
            "the length wanted is an input to every round, so a shorter answer is not a prefix"
        );
    }
}
