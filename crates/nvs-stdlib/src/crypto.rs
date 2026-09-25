//! `Core\Crypto` — `rule:core-api/tier-roster`'s "AEAD only, no ECB, no unauthenticated CBC, no cipher-name-as-string"
//! and `rule:core-classes/crypto-interop-tier`'s primitives in one class: sealing and opening, the two
//! key derivations, key pairs and the half of one that is sent, agreement over the roster's curves, and
//! signatures — each algorithm a case of a closed enum the call names, and not one of them defaulted.
//!
//! § 3 places the class and states the roster's one rule; what belongs here is
//! which constructions that rule picked, what the sealed bytes are, and why a
//! `secret` key crosses this surface without any of it being a laundering.
//!
//! # An algorithm is a case of a closed enum, required, and never a string
//!
//! `openssl_encrypt($data, "aes-256-cbc", …)` names its primitive in a string,
//! which is how a program ends up with `aes-256-ecb` in one file and a
//! typo-silent fallback in another — the cipher is chosen by whichever call
//! site was copied last, and "encrypted" and "authenticated" become two
//! decisions a caller can get half right. Here every algorithm is a case of a
//! closed enum: [`CIPHER`] for a cipher, [`KEY_KIND`] for what a key is, and
//! [`KEY_FORMAT`] for the encoding one travels in. A misspelled case is a
//! compile error rather than a silent fallback, every cipher case
//! authenticates, and no row in this module carries a default. What stays off
//! the call is everything the library is entitled to choose — no mode, no
//! padding, no IV, and no unauthenticated spelling to reach for.
//!
//! **A caller choosing here is safe, and a token choosing is not.** A program
//! that names `Aes256Gcm` or `Ed25519` is choosing nothing an attacker
//! supplied, which is the distinction [`crate::jwt`]'s module doc draws: a
//! JWT's `alg` is attacker-supplied by construction, so that class reads the
//! field only to compare it against the key. The same line runs through the
//! members here that take a key rather than a cipher — a signature's scheme is
//! the pair's kind and is never a parameter — so an enum a call names selects a
//! primitive and never a verifier. [`crate::password`]'s module doc makes the
//! argument about cost parameters, where the roster is one entry and so the
//! argument is absent rather than closed.
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
//! AES-256-GCM is the **interop** cipher, and
//! `rule:core-classes/crypto-interop-tier` is the whole of why it is here:
//! `A256GCM` is what a browser's WebCrypto encrypts with and the one content
//! encryption JWE has, so a runtime that cannot produce those bytes cannot read
//! what the other end wrote. Which of the two a call names is the call's, and
//! where the choice is free the extended-nonce construction is the one to
//! prefer — its nonce carries the bound above, and its software path is
//! ChaCha20 rather than the constant-time bitslice AES falls back to on a
//! machine without AES-NI, of which a container host scheduling this runtime is
//! still one often enough to matter. That preference is written in
//! [`seal`](nvs_core_crypto_seal)'s reference card as advice and nowhere as a
//! default: nothing here picks a cipher for its caller.
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
//! # Every primitive has one home, and its callers sit on the near side of it
//!
//! [`cipher`], [`seal_under`] and [`open_under`] are `pub(crate)`, and
//! [`crate::signed_cookie`] — `rule:security/protocol-roster`
//! 's first roster entry — is their second caller. That is what makes a
//! signed cookie *this* AEAD with a key ring over it rather than a second
//! construction with its own nonce policy and its own opinion about tags: one
//! `XChaCha20Poly1305::new_from_slice` in `nvs-stdlib`, and everything above
//! reaches it through those three functions. Nothing outside this module reads
//! a *field* of an XChaCha-sealed message — a caller gets the whole buffer or
//! nothing.
//!
//! The interop cipher has the same three — [`gcm_cipher`], [`gcm_seal_under`]
//! and [`gcm_open_under`] — and the nonce is where they differ: this one is an
//! argument rather than a draw. A member draws one through
//! [`crate::random::draw`] before it calls, a token's header carries the one it
//! was written with, and a test replays WebCrypto's own input so the two
//! implementations can be compared octet for octet. A nonce is never reused
//! under one key, which is the caller's obligation here and the reason the
//! argument is a fixed-width array rather than a slice.
//!
//! The rest of the roster keeps the same shape. [`derive_key`] and
//! [`expand_key`] are the derivations, [`agree_x25519`] and [`agree_p256`] the
//! agreements, [`sign`] over a [`SigningKey`] and [`verify_signature`] over a
//! [`VerifyingKey`] the signatures, and [`wrap_key`], [`unwrap_key`] and
//! [`concat_kdf`] the JOSE constructions that are no member's surface. Each is
//! written once and takes its bounds with it, so a class that needs one calls
//! it rather than keeping a second copy that can disagree about them.
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
//!
//! # Four signatures, and the key is what names the algorithm
//!
//! [`VerifyingKey`] holds a public key's material *inside* the variant naming
//! its algorithm, and [`verify_signature`] takes nothing besides that key, its
//! message and the signature. `rule:security/algorithm-comes-from-the-key` is
//! therefore a property of the type rather than a check a call site performs:
//! there is no parameter a scheme could arrive in, so an `alg` read off a token
//! has nowhere to go but a comparison. RSA is two variants because it is one key
//! type carrying two algorithms, and which one a key is bound to is settled when
//! the key is read and never revisited.
//!
//! These four reach BoringSSL through `ring`, while every cipher, derivation and
//! agreement above them stays RustCrypto; [ADR 0179](/docs/decisions/0179.md)
//! § 8 is where that single exception is argued. An ECDSA signature here is the
//! 64-octet `r ‖ s` that JWS and WebCrypto both write, so the DER form most
//! other tooling prints is refused rather than re-encoded — one encoding on the
//! wire is one fewer place for two implementations to disagree.
//!
//! **Signing is the one thing in this module that draws outside
//! [`crate::random::draw`].** `ring`'s randomness trait is sealed to its own
//! implementations, so the OS source it ships is the only one its signers
//! accept, and PSS's salt and ECDSA's nonce come from there rather than through
//! the seam the nonce above is drawn through. It is the same class of source and
//! not a weaker one; what is given up is the single point a test could stand in
//! front of, which is why the two algorithms that write the same octets every
//! time — `RS256` and `EdDSA` — are the ones pinned to the frozen set octet for
//! octet, and the two that do not are held to the property that survives being
//! randomized.
//!
//! # One key, three encodings, and a thumbprint over the third
//!
//! [`PublicKey`] is the public half of any of [`KEY_KIND`]'s five kinds, read
//! out of and written back into [`KEY_FORMAT`]'s three encodings — the three a
//! browser's `SubtleCrypto.exportKey` writes. **Every check a public key gets
//! happens at the read**, which is
//! `rule:core-classes/crypto-interop-tier`'s *validated where it is read*: a
//! P-256 point goes through [`read_p256_point`] whichever encoding carried it,
//! an RSA modulus is held to the roster's width, and the kind a key is bound to
//! is the kind the call named. Nothing downstream re-checks any of it, because
//! a [`PublicKey`] that exists is a key.
//!
//! [`KeyRefusal`] is the one distinction the codec can draw and the member
//! above it cannot: an encoding a kind does not have is a bug in the program,
//! while octets that are not a key are a verdict on whoever sent them, and
//! which of those the second is depends on where the octets came from.
//!
//! The JWK written here is RFC 7638's — the members that kind requires, sorted,
//! no whitespace, nothing else — so [`PublicKey::thumbprint`] is a digest over
//! exactly what `write` answers rather than over a second canonicalization
//! nothing else uses. A browser's own export carries `ext`, `key_ops` and `alg`
//! beside the required members; those are read and ignored, and a `d` is
//! refused, a private key handed over as a public one being a bug rather than a
//! bad key.

use aes_gcm::Aes256Gcm;
use aes_kw::{KwAes128, KwAes256};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use chacha20poly1305::aead::{Aead, Nonce, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use p256::PublicKey as P256PublicKey;
use p256::SecretKey as P256SecretKey;
use p256::ecdh::diffie_hellman;
use p256::elliptic_curve::sec1::ToSec1Point;
use p256::pkcs8::der::asn1::{AnyRef, BitStringRef, OctetStringRef, UintRef};
use p256::pkcs8::der::{
    Decode, DecodeValue, Encode, EncodeValue, Error as DerError, Header, Length, Reader, Sequence,
    Writer,
};
use p256::pkcs8::spki::{
    AlgorithmIdentifierRef, ObjectIdentifier, SubjectPublicKeyInfo, SubjectPublicKeyInfoRef,
};
use p256::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, PrivateKeyInfoRef};
use rand::Rng;
use ring::rand::SystemRandom;
use ring::signature::{
    ECDSA_P256_SHA256_FIXED, ECDSA_P256_SHA256_FIXED_SIGNING, ED25519, EcdsaKeyPair,
    Ed25519KeyPair, KeyPair as _, RSA_PKCS1_2048_8192_SHA256, RSA_PKCS1_SHA256,
    RSA_PSS_2048_8192_SHA256, RSA_PSS_SHA256, RsaEncoding, RsaKeyPair, RsaPublicKeyComponents,
    UnparsedPublicKey,
};
// The 0.11 line of `sha2`, because `hkdf` and `pbkdf2` are generic over the
// digest family's 0.11 traits and `Core\Hash`'s `sha2` is the 0.10 one. The
// root manifest's row is where both majors being in this tree is argued.
use sha2_v11::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Crypto";

/// The cipher enum's name, once, for the row that takes it and the fatal that
/// reports a slot holding something else.
pub(crate) const CIPHER_NAME: &str = r"Core\Crypto\Cipher";

/// The key-kind enum's name, once, for the rows that take it and the fatal that
/// reports a slot holding something else.
pub(crate) const KEY_KIND_NAME: &str = r"Core\Crypto\KeyKind";

/// The key-format enum's name, once, for the same two places.
pub(crate) const KEY_FORMAT_NAME: &str = r"Core\Crypto\KeyFormat";

/// The public-key class's name, once, for its own refusals and for the row that
/// answers one.
pub(crate) const PUBLIC_KEY_NAME: &str = r"Core\Crypto\PublicKey";

/// [`PUBLIC_KEY`]'s first slot: the key's `SubjectPublicKeyInfo`, whichever
/// encoding it arrived in.
const PUBLIC_KEY_SPKI_SLOT: usize = 0;

/// [`PUBLIC_KEY`]'s second slot: [`KEY_KIND`]'s constant for the kind the read
/// was given, which is the one thing a `SubjectPublicKeyInfo` does not carry —
/// `rule:security/algorithm-comes-from-the-key` held for the one key type two
/// JWS algorithms share.
const PUBLIC_KEY_KIND_SLOT: usize = 1;

/// The key-pair class's name, once, for its own refusals and for the row that
/// answers one.
pub(crate) const KEY_PAIR_NAME: &str = r"Core\Crypto\KeyPair";

/// [`KEY_PAIR`]'s first slot: the pair's PKCS#8, as DER whichever spelling it
/// was read from.
const KEY_PAIR_PKCS8_SLOT: usize = 0;

/// [`KEY_PAIR`]'s second slot: [`KEY_KIND`]'s constant for the kind the read was
/// given, which a PKCS#8 no more carries than a `SubjectPublicKeyInfo` does.
const KEY_PAIR_KIND_SLOT: usize = 1;

// The two key classes have one layout between them — the encoded key, then its
// kind — which is what lets [`stored_key`] read either of them. A slot pair that
// drifted apart would make that function silently read a kind out of a key.
const _: () = assert!(
    PUBLIC_KEY_SPKI_SLOT == KEY_PAIR_PKCS8_SLOT && PUBLIC_KEY_KIND_SLOT == KEY_PAIR_KIND_SLOT,
    "the two key classes are read by one function, so their slots are one layout"
);

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

// An item below that still carries `#[expect(dead_code)]` is reached by its own
// tests and by nothing else: the `Core` row that will call it is not registered
// yet, and the expectation is what fails the day it is, so each marker deletes
// itself one row at a time. It is `cfg_attr(not(test), …)` because under
// `cfg(test)` the tests are the caller, so the lint does not fire and an
// unconditional expectation would be the unfulfilled one. The interop cipher
// carries none: [`CIPHER`]'s second case is what a program names to reach it.

/// AES-GCM's nonce length in octets, and an interop number rather than a
/// choice: it is what WebCrypto emits as an IV and what a JWE header carries.
pub(crate) const GCM_NONCE_LEN: usize = 12;

/// What an AES-GCM sealed message costs over its plaintext, and the shortest
/// buffer [`gcm_open_under`] could authenticate.
pub(crate) const GCM_OVERHEAD: usize = GCM_NONCE_LEN + TAG_LEN;

/// What a derivation answers, in octets — [`KEY_LEN`], for the module doc's
/// *two derivations* reason.
pub(crate) const DERIVED_LEN: usize = KEY_LEN;

/// The fewest PBKDF2 iterations this module will run.
pub(crate) const MIN_ITERATIONS: u32 = 100_000;

/// The most PBKDF2 iterations this module will run, and the bound that makes
/// PBES2 safe rather than the one that makes a password hard: the module doc's
/// *two derivations* section is why a ceiling exists at all.
pub(crate) const MAX_ITERATIONS: u32 = 2_000_000;

/// The shortest salt PBKDF2 will accept, in octets — enough that a table built
/// against one derivation is worthless against the next.
pub(crate) const MIN_SALT_LEN: usize = 16;

/// What a key agreement answers, in octets — X25519's u-coordinate and P-256's
/// x-coordinate are both this long, which is what lets one constant stand for
/// both and either answer reach [`expand_key`] unchanged.
///
/// It is [`KEY_LEN`] as a number and not as a meaning: a shared secret is a
/// coordinate, and the module doc's *two curves agree* section is why it is
/// never used as a key.
pub(crate) const SHARED_LEN: usize = 32;

/// The shorter key-encryption key [`wrap_key`] takes, in octets.
///
/// It is 128 bits because `PBES2-HS256+A128KW` names that width and the other
/// end implements the name, not because anything in this module encrypts under
/// AES-128: a wrap is not a cipher a program can reach, and the roster's own
/// ciphers are both 256-bit.
pub(crate) const KW_128_KEY_LEN: usize = 16;

/// What a wrapped [`KEY_LEN`]-octet key is, in octets — RFC 3394 adds one
/// 64-bit semiblock, which is the integrity check an unwrap verifies.
pub(crate) const WRAPPED_LEN: usize = KEY_LEN + 8;

/// A P-256 point's length in octets in the uncompressed SEC1 encoding: the
/// [`SEC1_UNCOMPRESSED`] tag and the two coordinates after it.
///
/// It is the one encoding this module hands anything a point in — a browser's
/// raw export, what a JWK's two coordinates assemble into, and what
/// [`VerifyingKey::P256`] reads — so a key holds these octets rather than
/// re-deriving them per call.
pub(crate) const P256_POINT_LEN: usize = 1 + 2 * P256_COORDINATE_LEN;

/// A P-256 coordinate's length in octets, which is the width a JWK's `x` and
/// `y` are each held to: two members that concatenate to a point of the right
/// length are still not a point unless each is its own field element wide.
pub(crate) const P256_COORDINATE_LEN: usize = 32;

/// The tag an uncompressed SEC1 point carries in front of its coordinates.
const SEC1_UNCOMPRESSED: u8 = 0x04;

/// A Curve25519 public key's length in octets, X25519's and Ed25519's alike:
/// each is one field element and neither curve has a second encoding.
pub(crate) const CURVE25519_POINT_LEN: usize = 32;

/// The narrowest RSA modulus the roster admits, in bits.
///
/// `rule:core-classes/crypto-interop-tier` fixes this and [`MAX_RSA_BITS`], and
/// both are checked where a key is read: below the floor is a key nobody should
/// still be verifying with, and above the ceiling is a modulus whose
/// verification is a CPU bill an attacker chose.
pub(crate) const MIN_RSA_BITS: u32 = 2048;

/// The widest RSA modulus the roster admits, in bits — [`MIN_RSA_BITS`]'s other
/// end, and the reason `ring`'s `2048_8192` verifiers are the ones this module
/// names.
pub(crate) const MAX_RSA_BITS: u32 = 8192;

/// The cipher a `seal` or an `open` names, and the argument that stands where
/// `openssl_encrypt`'s mode string stood.
///
/// Closed at the roster `rule:core-classes/crypto-interop-tier` fixes, so the
/// set of things a program can ask for is the set of things that authenticate,
/// and carrying no default anywhere: a call says which construction it is
/// under, and the module doc's *the cipher is a closed enum* section is why
/// that is a required argument rather than a convenience with a fallback.
///
/// The integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`]. They are ABI, in the sense that
/// [`keyed`] reads them back out of an argument slot: reordering this list is a
/// behaviour change, not a cosmetic one.
pub(crate) const CIPHER: CoreEnum = CoreEnum {
    name: CIPHER_NAME,
    cases: &[("XChaCha20Poly1305", 0), ("Aes256Gcm", 1)],
    doc: Some(&CIPHER_DOC),
};

/// [`CIPHER`]'s reference card — `rule:core-api/reference-card`. The module
/// doc owns why each construction is here; these are that argument condensed.
const CIPHER_DOC: EnumDoc = EnumDoc {
    short: "Which authenticated construction a `seal` or an `open` runs. There is no default: a \
            call names its cipher, and both cases authenticate, so neither choice can produce a \
            message an alteration would survive.",
    cases: &[
        CaseDoc {
            name: "XChaCha20Poly1305",
            desc: "ChaCha20-Poly1305 under a 192-bit nonce — the one to prefer when both ends \
                   are Novis, because a nonce that wide is never drawn twice and the cipher is \
                   fast on a host with no AES instructions. Seals as `nonce(24) ‖ ciphertext ‖ \
                   tag(16)`.",
        },
        CaseDoc {
            name: "Aes256Gcm",
            desc: "AES-256-GCM, the interop cipher: what a browser's WebCrypto encrypts with, so \
                   a sealed message crosses to the other end. Seals as `nonce(12) ‖ ciphertext ‖ \
                   tag(16)`, which is WebCrypto's own output with its IV in front. Its 96-bit \
                   nonce puts a birthday bound near 2^32 messages under one key; past that, seal \
                   under the other case.",
        },
    ],
};

/// Which asymmetric key a program is naming — what `generateKeyPair` makes,
/// what a stored key is read back as, and what a pair therefore signs or agrees
/// with.
///
/// Closed at the roster `rule:core-classes/crypto-interop-tier` fixes, and
/// carrying no default for the reason [`CIPHER`] carries none: a call says which
/// primitive it is under. The kinds are not interchangeable and the type does
/// not pretend they are — an X25519 pair agrees and signs nothing, an Ed25519 or
/// RSA pair signs and agrees nothing, and each wrong pairing is a `LogicError`
/// rather than a second-best answer.
///
/// RSA is one key type under two cases, which is
/// `rule:security/algorithm-comes-from-the-key` held at the only door where the
/// key itself cannot settle the question: PKCS#8 and SPKI both say *RSA* and
/// neither says `RS256` or `PS256`, so the scheme is fixed when the key is read
/// and is read back off the kind everywhere after. [`SigningKey`] is that same
/// choice inside the crate, which is why it has four variants where this has
/// five: X25519 signs nothing and so reaches no signer.
///
/// The integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`]. They are ABI: a member reads them
/// back out of an argument slot, so reordering this list is a behaviour change,
/// not a cosmetic one.
pub(crate) const KEY_KIND: CoreEnum = CoreEnum {
    name: KEY_KIND_NAME,
    cases: &[
        ("P256", 0),
        ("X25519", 1),
        ("Ed25519", 2),
        ("RsaPkcs1", 3),
        ("RsaPss", 4),
    ],
    doc: Some(&KEY_KIND_DOC),
};

/// [`KEY_KIND`]'s reference card — `rule:core-api/reference-card`. The module
/// doc owns why the roster stops where it does; these say what each case is for.
const KEY_KIND_DOC: EnumDoc = EnumDoc {
    short: "Which asymmetric key a member is naming — the curve or the RSA scheme. There is no \
            default, and the kinds do not substitute for one another: agreement is the two \
            Diffie-Hellman cases and signing is the other three.",
    cases: &[
        CaseDoc {
            name: "P256",
            desc: "NIST P-256, for agreement (ECDH) and for signing (ECDSA over SHA-256, as the \
                   64-octet `r ‖ s` a browser and JWS both use). The one curve every WebCrypto \
                   implementation has, so it is what an interop key pair is.",
        },
        CaseDoc {
            name: "X25519",
            desc: "Curve25519 key agreement, and agreement alone — a pair of this kind signs \
                   nothing. Prefer it over `P256` when both ends choose the curve, because \
                   nothing about it has to be checked at a call site to be safe.",
        },
        CaseDoc {
            name: "Ed25519",
            desc: "Ed25519 signatures, and signatures alone — a pair of this kind agrees \
                   nothing. The signing kind to prefer when both ends are Novis: short keys, \
                   short signatures, and no scheme left to choose.",
        },
        CaseDoc {
            name: "RsaPkcs1",
            desc: "An RSA key signing RSASSA-PKCS1-v1_5 over SHA-256 — JWS's `RS256`, and what \
                   most identity providers still issue. An RSA key is read, never generated, and \
                   this case is what fixes its scheme at the read.",
        },
        CaseDoc {
            name: "RsaPss",
            desc: "The same RSA key under RSASSA-PSS with SHA-256 and a 32-octet salt — JWS's \
                   `PS256`. It is a separate case rather than a separate key type because the \
                   key cannot say which of the two it is for.",
        },
    ],
};

/// Which encoding a public key is read out of or written back into — the three
/// a browser's `SubtleCrypto.exportKey` writes, under the names it writes them
/// under.
///
/// It reaches the public half alone. A private key crosses as PKCS#8 and
/// nothing else, so `KeyPair::write` takes no format argument: there is one
/// spelling for a stored pair, which is the one a service-account file already
/// holds.
///
/// No default, for the reason [`CIPHER`] and [`KEY_KIND`] carry none, and here
/// with a second edge: the encodings are not all defined for all kinds. `Raw` is
/// the key material with nothing around it — the uncompressed point for `P256`,
/// the 32 octets for `X25519` and `Ed25519` — and an RSA key has no such form,
/// so `Raw` against either RSA kind is a `LogicError` rather than a guess at
/// which DER was meant.
///
/// The integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`], and they are ABI for the same
/// reason [`CIPHER`]'s are: a member reads them back out of an argument slot.
pub(crate) const KEY_FORMAT: CoreEnum = CoreEnum {
    name: KEY_FORMAT_NAME,
    cases: &[("Raw", 0), ("Spki", 1), ("Jwk", 2)],
    doc: Some(&KEY_FORMAT_DOC),
};

/// [`KEY_FORMAT`]'s reference card — `rule:core-api/reference-card`.
const KEY_FORMAT_DOC: EnumDoc = EnumDoc {
    short: "Which encoding a public key is read from or written to. They are WebCrypto's own three \
            export formats under its own names, so a key crosses to a browser in whichever of them \
            the other end asked for.",
    cases: &[
        CaseDoc {
            name: "Raw",
            desc: "The key material alone: the 65-octet uncompressed point for `P256`, the 32 \
                   octets for `X25519` and `Ed25519`. An RSA key has no raw form, so this is a \
                   `LogicError` under either RSA kind.",
        },
        CaseDoc {
            name: "Spki",
            desc: "DER `SubjectPublicKeyInfo`, the one form every kind has — the key material \
                   under the algorithm identifier that names it. What a certificate and a `.pem` \
                   public key carry, and the only format an RSA key reads from or writes to \
                   besides `Jwk`.",
        },
        CaseDoc {
            name: "Jwk",
            desc: "The JSON Web Key as UTF-8 JSON octets. A read accepts what a browser exports, \
                   `ext` and `key_ops` included, and refuses one carrying `d`, because a private \
                   key handed over as a public one is a bug. A write answers RFC 7638's required \
                   members, sorted — the form a thumbprint is taken over.",
        },
    ],
};

/// `rule:core-api/tier-roster`'s AEAD-only surface, as registry rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
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
            names: &["message", "key", "cipher"],
            // The message is contagious on the `tainted` axis — the ciphertext
            // is made of it — and the key is neutral, because not one octet of
            // a key reaches the answer and its provenance says nothing about
            // the message's. `defaults: &[]` is the roster's rule rather than
            // this row's convenience: no algorithm argument in this class has
            // one.
            params: &[
                CoreTy::Blob(Qual::Contagious),
                CoreTy::SecretBlob(Qual::Neutral),
                CoreTy::Enum(CIPHER_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_crypto_seal",
            doc: Some(&SEAL_DOC),
        },
        CoreMethod {
            name: "open",
            names: &["sealed", "key", "cipher"],
            params: &[
                CoreTy::Blob(Qual::Contagious),
                CoreTy::SecretBlob(Qual::Neutral),
                CoreTy::Enum(CIPHER_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_crypto_open",
            doc: Some(&OPEN_DOC),
        },
        CoreMethod {
            name: "deriveKey",
            names: &["password", "salt", "iterations"],
            // Not one parameter here is contagious, and the reason is the same
            // for all three: a derivation answers a PRF output that carries no
            // octet of any argument, so a `tainted` password off a form and a
            // `tainted` salt read back out of a row both reach it and the key
            // is neither. The password is spelled `secret string` because that
            // is what a password *is* — the qualifier belongs to the value
            // rather than being a licence this member takes to reveal one,
            // which is what `Qual::Reveal` would have said instead.
            params: &[
                CoreTy::SecretText(Qual::Neutral),
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Uint,
            ],
            defaults: &[],
            return_ty: CoreTy::SecretBytes,
            symbol: "nvs_core_crypto_derive_key",
            doc: Some(&DERIVE_KEY_DOC),
        },
        CoreMethod {
            name: "expandKey",
            names: &["material", "salt", "info"],
            params: &[
                CoreTy::SecretBlob(Qual::Neutral),
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
            ],
            defaults: &[],
            return_ty: CoreTy::SecretBytes,
            symbol: "nvs_core_crypto_expand_key",
            doc: Some(&EXPAND_KEY_DOC),
        },
        CoreMethod {
            name: "generateKeyPair",
            names: &["kind"],
            params: &[CoreTy::Enum(KEY_KIND_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(KEY_PAIR_NAME),
            symbol: "nvs_core_crypto_generate_key_pair",
            doc: Some(&GENERATE_KEY_PAIR_DOC),
        },
        CoreMethod {
            name: "agree",
            // Neither key is contagious and the answer is `secret`: a shared
            // secret is a key, and no octet of either argument reaches it —
            // which is `deriveKey`'s reading of the same question, over an
            // object here rather than over octets.
            names: &["mine", "theirs"],
            params: &[
                CoreTy::Instance(KEY_PAIR_NAME),
                CoreTy::Instance(PUBLIC_KEY_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::SecretBytes,
            symbol: "nvs_core_crypto_agree",
            doc: Some(&AGREE_DOC),
        },
        CoreMethod {
            name: "sign",
            // The message is neutral rather than contagious for `deriveKey`'s
            // reason: a signature is a digest run through a key, so not one
            // octet of what was signed reaches it and a `tainted` body off the
            // wire signs without making the signature tainted. It is plain
            // `bytes` rather than `secret` because signing a secret is a
            // written `Core\Secret::revealBytes`, exactly as sealing one is.
            names: &["message", "key"],
            params: &[CoreTy::Blob(Qual::Neutral), CoreTy::Instance(KEY_PAIR_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_crypto_sign",
            doc: Some(&SIGN_DOC),
        },
        CoreMethod {
            name: "verify",
            // The signature is as neutral as the message, and for a stronger
            // reason than the row above: nothing is answered at all, so there
            // is no value for a qualifier to reach. Both arrive from whoever
            // sent them, which is the point of the member.
            names: &["message", "signature", "key"],
            params: &[
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Instance(PUBLIC_KEY_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_crypto_verify",
            doc: Some(&VERIFY_DOC),
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
    short: "Encrypts and authenticates `$message` under `$key` with the construction `$cipher` \
            names, drawing a fresh nonce per call. There is no mode, padding or IV argument, and \
            no cipher name in a string — the only choice is a `Core\\Crypto\\Cipher` case, and \
            every case authenticates.",
    params: &[
        ParamDoc {
            name: "message",
            desc: "The plaintext. A `secret` is refused here: sealing one is a written \
                   `Core\\Secret::revealBytes` call, which is what makes it greppable.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "A 32-octet key, as `generateKey` answers one. One width keys either cipher.",
            shape: &[],
        },
        ParamDoc {
            name: "cipher",
            desc: "Which construction to seal under. No default: `XChaCha20Poly1305` where both \
                   ends are Novis, `Aes256Gcm` where a browser has to read the result.",
            shape: &[],
        },
    ],
    ret: "The sealed message — 40 octets longer than `$message` under `XChaCha20Poly1305` and 28 \
          under `Aes256Gcm`, and different on every call for the same inputs, because each draws \
          its own nonce.",
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
    short: "Authenticates `$sealed` under `$key` and `$cipher` and answers the plaintext, or \
            throws. A message altered by one octet is refused rather than decrypted into \
            whatever is left of it, which is the whole reason the roster is AEAD only.",
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
        ParamDoc {
            name: "cipher",
            desc: "The construction `$sealed` was sealed under. Naming the other one is a \
                   forgery like any other, never plausible bytes.",
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
            desc: "`$sealed` is not an authentic message under `$key` and `$cipher` — it was \
                   altered, it is too short to be one at all, or the key or the cipher is the \
                   wrong one. They are one message on purpose: telling them apart tells a forger \
                   which half landed.",
        },
    ],
};

/// `Core\Crypto::deriveKey`'s reference card — `rule:core-api/reference-card`.
const DERIVE_KEY_DOC: MethodDoc = MethodDoc {
    short: "Derives a key from a password with PBKDF2-HMAC-SHA256, stretching `$password` and \
            `$salt` over `$iterations` rounds. This is the member for a value a person typed; \
            `expandKey` is the one for material that is already uniform, and handing a password \
            to it is the mistake no type here can catch.",
    params: &[
        ParamDoc {
            name: "password",
            desc: "The password to stretch. A `secret` is accepted and nothing is revealed: the \
                   answer is a `secret bytes` carrying no octet of it.",
            shape: &[],
        },
        ParamDoc {
            name: "salt",
            desc: "At least 16 octets, drawn once per password and stored beside the key it \
                   derived. `Core\\Random::bytes(16)` answers one; it is not a secret.",
            shape: &[],
        },
        ParamDoc {
            name: "iterations",
            desc: "How many rounds to stretch for, between 100,000 and 2,000,000. Required and \
                   without a default, because a key derived to be read back has to be derived \
                   under the count whoever wrote it chose.",
            shape: &[],
        },
    ],
    ret: "32 octets as a `secret bytes`, the width `seal` and `open` key under. The same three \
          arguments always answer the same key — that is what makes it a derivation rather than \
          a draw.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$iterations` is outside 100,000 to 2,000,000, or `$salt` is shorter than 16 \
               octets. Under the floor the answer is cheap to attack, and over the ceiling one \
               call is a denial of service against the process that made it.",
    }],
};

/// `Core\Crypto::expandKey`'s reference card — `rule:core-api/reference-card`.
const EXPAND_KEY_DOC: MethodDoc = MethodDoc {
    short: "Derives a key from material that is already uniform with HKDF-SHA256 — a shared \
            secret out of a key agreement, or a root key one service holds. `$info` is what \
            separates two keys derived from one secret, so a program names the use rather than \
            reusing the secret at two call sites.",
    params: &[
        ParamDoc {
            name: "material",
            desc: "The secret to expand. Uniform already: a password belongs at `deriveKey`, \
                   which stretches it.",
            shape: &[],
        },
        ParamDoc {
            name: "salt",
            desc: "The extract step's salt, which is not a secret and may be empty — RFC 5869's \
                   own default of a zero-filled one.",
            shape: &[],
        },
        ParamDoc {
            name: "info",
            desc: "The context string. Two calls over one `$material` with different `$info` \
                   answer unrelated keys, which is how one secret keys two things.",
            shape: &[],
        },
    ],
    ret: "32 octets as a `secret bytes`, the width `seal` and `open` key under, and the same for \
          the same three arguments.",
    errors: &[],
};

/// `Core\Crypto::generateKeyPair`'s reference card — `rule:core-api/reference-card`.
const GENERATE_KEY_PAIR_DOC: MethodDoc = MethodDoc {
    short: "Draws a fresh private key of the kind this call names and answers the pair it is, \
            from the same CSPRNG `Core\\Random` uses. There is no key size argument: each kind \
            has one size, and the kinds that would need one are the two this member refuses.",
    params: &[ParamDoc {
        name: "kind",
        desc: "Which key to draw. `P256` agrees and signs, `X25519` agrees only, `Ed25519` \
               signs only, and the two RSA kinds are read rather than drawn.",
        shape: &[],
    }],
    ret: "The pair, ready to sign or to agree with, and answering `write` with a PKCS#8 \
          `Core\\Crypto\\KeyPair::read` takes back.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$kind` is `RsaPkcs1` or `RsaPss`. An RSA key is never generated here — a \
               program is handed one by whoever issued it, and `Core\\Crypto\\KeyPair::read` \
               is the member that takes it.",
    }],
};

/// `Core\Crypto::agree`'s reference card — `rule:core-api/reference-card`.
const AGREE_DOC: MethodDoc = MethodDoc {
    short: "Agrees a shared secret with a peer over ECDH, from this program's own pair and the \
            public key the peer sent. The answer is the raw agreed value and is not a key: run it \
            through `expandKey` with a context string, which is what turns one agreement into the \
            keys a protocol needs.",
    params: &[
        ParamDoc {
            name: "mine",
            desc: "This program's pair, of kind `P256` or `X25519`. The other kinds sign rather \
                   than agree.",
            shape: &[],
        },
        ParamDoc {
            name: "theirs",
            desc: "The peer's public key, which has to be of the same kind: two keys on \
                   different curves are not two ends of one agreement.",
            shape: &[],
        },
    ],
    ret: "32 octets as a `secret bytes` — the x-coordinate for P-256 and the u-coordinate for \
          X25519, which is what WebCrypto's `deriveBits` answers over the same two keys.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The two keys are not two ends of one agreement: a pair of a kind that agrees \
                   nothing — `Ed25519` and both RSA kinds — or two keys on different curves.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The peer's X25519 point contributes nothing, so the secret would be all-zero \
                   whatever this program's scalar is. That is a point chosen by whoever sent it, \
                   so it is a verdict on them rather than a bug here.",
        },
    ],
};

/// `Core\Crypto::sign`'s reference card — `rule:core-api/reference-card`.
const SIGN_DOC: MethodDoc = MethodDoc {
    short: "Signs `$message` under `$key`'s private half. There is no algorithm or digest \
            argument: the scheme is the pair's own kind — `RsaPkcs1` and `RsaPss` over SHA-256, \
            `P256` as ECDSA over SHA-256, `Ed25519` as itself — so a call cannot name one the key \
            is not.",
    params: &[
        ParamDoc {
            name: "message",
            desc: "The octets to sign, whole. There is no pre-hashed spelling: a digest handed in \
                   as a message is a signature over a digest, which is a different statement.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The pair to sign with, of any kind but `X25519`, which agrees rather than \
                   signs.",
            shape: &[],
        },
    ],
    ret: "The signature: 64 octets for `P256` — the `r ‖ s` pair JWS and WebCrypto both use, never \
          DER — 64 for `Ed25519`, and as many octets as the modulus is long for either RSA kind. \
          `RsaPkcs1` and `Ed25519` sign a message the same way every time; `P256` and `RsaPss` \
          draw randomness, so two signatures over one message differ and both verify.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$key` is an `X25519` pair, which signs nothing — the kind is chosen when the pair \
               is generated or read, so this is the call to fix.",
    }],
};

/// `Core\Crypto::verify`'s reference card — `rule:core-api/reference-card`.
const VERIFY_DOC: MethodDoc = MethodDoc {
    short: "Checks that `$signature` is `$key`'s over `$message`, and throws when it is not. The \
            algorithm is the key's own, as `sign`'s is the pair's, so no part of what arrived \
            chooses how it is checked.",
    params: &[
        ParamDoc {
            name: "message",
            desc: "The octets the signature is supposed to cover. `tainted` is accepted and \
                   stays: checking what a peer sent is the point, and a signature that held says \
                   who sent the octets rather than what is in them.",
            shape: &[],
        },
        ParamDoc {
            name: "signature",
            desc: "The signature as `sign` answers one — `r ‖ s` for `P256`, never DER.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The public key to check against, of any kind but `X25519`.",
            shape: &[],
        },
    ],
    ret: "Nothing. A check that held returns and a check that failed throws, so there is no \
          falsy answer for a `==` to misread.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The signature is not this key's over this message — altered, the wrong length \
                   for the key's algorithm, or made under another key. Every one of those is the \
                   same sentence, so the refusal says nothing about which part was wrong.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` is an `X25519` key, which verifies nothing. That is the program's own \
                   key rather than anything that arrived, so it is a bug and not a verdict.",
        },
    ],
};

/// `rule:core-classes/crypto-interop-tier`'s public half, as registry rows: the
/// key a program was handed, read once and answered in whichever encoding the
/// other end asked for.
///
/// # Decision: the slots are the canonical SPKI and the kind, and every member re-reads
///
/// A `Core` instance slot holds a value Novis can already hold and never native
/// state ([`crate::instance`]), so the parsed key is not what this object keeps.
/// What it keeps instead is the one encoding every kind of the roster has — the
/// `SubjectPublicKeyInfo` [`PublicKey::write`] answers — beside the kind's
/// [`KEY_KIND`] constant, which is the fact that DER does not carry for the one
/// key type `RsaPkcs1` and `RsaPss` share. Every member then goes back through
/// [`PublicKey::read`], so a key is **never less validated than the first read
/// left it**: the DER parse and, on P-256, the on-curve check are paid again
/// rather than trusted.
///
/// The rejected alternative was holding the caller's own octets and its format:
/// it makes `write` in the arriving encoding a copy, and it makes two programs
/// holding one key hold two different slot values, so a JWK that arrived with
/// `ext` and `key_ops` on it would still be inside the object that read it.
///
/// **What it spends:** two slots per key, the wider of them the key's DER — a
/// little over a kilobyte at [`MAX_RSA_BITS`] and under a hundred octets on
/// every curve — plus one DER parse per member call. No member here is on a
/// request's hot path: a program reads a peer's key and then verifies or agrees
/// with it, and those are the calls that matter.
pub(crate) const PUBLIC_KEY: CoreClass = CoreClass {
    name: PUBLIC_KEY_NAME,
    doc: None,
    methods: &[CoreMethod {
        name: "read",
        names: &["encoded", "kind", "format"],
        // `$encoded` is neutral on the `tainted` axis because the answer is an
        // object, which carries no qualifier at all: what crosses out of a
        // public key off the wire is a key this member has already checked is
        // one, in the shape the roster admits, rather than the octets that
        // carried it.
        params: &[
            CoreTy::Blob(Qual::Neutral),
            CoreTy::Enum(KEY_KIND_NAME),
            CoreTy::Enum(KEY_FORMAT_NAME),
        ],
        defaults: &[],
        return_ty: CoreTy::Instance(PUBLIC_KEY_NAME),
        symbol: "nvs_core_crypto_public_key_read",
        doc: Some(&PUBLIC_KEY_READ_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "write",
            names: &["format"],
            params: &[CoreTy::Enum(KEY_FORMAT_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_crypto_public_key_write",
            doc: Some(&PUBLIC_KEY_WRITE_DOC),
        },
        CoreMethod {
            name: "kind",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(KEY_KIND_NAME),
            symbol: "nvs_core_crypto_public_key_kind",
            doc: Some(&PUBLIC_KEY_KIND_DOC),
        },
    ],
    slots: &["spki", "kind"],
    constants: &[],
};

/// `Core\Crypto\PublicKey::read`'s reference card — `rule:core-api/reference-card`.
const PUBLIC_KEY_READ_DOC: MethodDoc = MethodDoc {
    short: "Reads a public key out of `$encoded` and validates it: on the curve for `P256`, \
            inside the roster's width for either RSA kind, and a JWK's members against the kind \
            named. A key that does not check is refused here and nowhere later, so no member \
            that takes one can be handed a key nobody looked at.",
    params: &[
        ParamDoc {
            name: "encoded",
            desc: "The key's octets, in `$format`.",
            shape: &[],
        },
        ParamDoc {
            name: "kind",
            desc: "Which key this is. It is named rather than read out of the octets, because \
                   an RSA `SubjectPublicKeyInfo` says `rsaEncryption` whichever of `RS256` and \
                   `PS256` the key is for, so the program that was handed the key is the only \
                   party that can say.",
            shape: &[],
        },
        ParamDoc {
            name: "format",
            desc: "Which of WebCrypto's three export encodings `$encoded` is in.",
            shape: &[],
        },
    ],
    ret: "The key, ready to verify a signature or to agree with, and answering `kind` with the \
          case it was read as.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The call names an encoding the kind does not have — `Raw` against either RSA \
                   kind — or the JWK carries `d`, which is a private key handed over as a \
                   public one.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$encoded` is not a public key of that kind in that encoding: a point off \
                   the curve, a coordinate of the wrong width, a modulus outside the roster's \
                   range, a DER body that does not parse, or a JWK naming another key type. \
                   They are one message, because the key came from whoever sent it and a \
                   reason is a reply to them.",
        },
    ],
};

/// `Core\Crypto\PublicKey::write`'s reference card — `rule:core-api/reference-card`.
const PUBLIC_KEY_WRITE_DOC: MethodDoc = MethodDoc {
    short: "Answers this key in `$format` — the same encodings `read` accepts, so a key crosses \
            to a browser in whichever one the other end asked for. It is written from the key \
            rather than from the octets it arrived in, so one key has one spelling per \
            encoding however it was read.",
    params: &[ParamDoc {
        name: "format",
        desc: "Which encoding to write. `Jwk` answers RFC 7638's required members, sorted, \
               which is the form a thumbprint is taken over.",
        shape: &[],
    }],
    ret: "The key's octets, a JWK being its UTF-8 JSON.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`Raw` against either RSA kind: an RSA key has no encoding with nothing around \
               it, so there is nothing for this to answer.",
    }],
};

/// `Core\Crypto\PublicKey::kind`'s reference card — `rule:core-api/reference-card`.
const PUBLIC_KEY_KIND_DOC: MethodDoc = MethodDoc {
    short: "Reports which of `Core\\Crypto\\KeyKind`'s cases this key is, which is the kind its \
            read was given and never a second reading of the material.",
    params: &[],
    ret: "The case the key was read as. For an RSA key that is `RsaPkcs1` or `RsaPss` — the \
          scheme the key is bound to, settled at the read and not afterwards.",
    errors: &[],
};

/// The private half, as the object `rule:core-api/shape-rules` R14 asks for:
/// read once, kept for as long as the program holds it, and handed to whichever
/// member needs the half of it that member needs.
///
/// **Its slots are the PKCS#8 and the kind**, for [`PUBLIC_KEY`]'s reason and
/// with one difference. The reason is the same: a slot holds a value Novis can
/// already hold, so a parsed key is not what an object keeps, and reading the
/// stored octets back through [`PrivateKey::read`] leaves every member's key
/// exactly as validated as the first read left it. The difference is that a
/// PKCS#8 is stored as the pair received it rather than re-encoded — one PEM
/// block becomes its DER and nothing else moves, and a drawn key keeps the file
/// [`generated_pkcs8`] wrote — because `write`'s whole job is to hand a program
/// back the key it deployed or drew, and because this module reads RSA's
/// private components rather than writing them.
///
/// **The rejected alternative was a per-core cache of parsed keys**, the way
/// [`crate::regex`] keeps compiled patterns behind their source text. It buys
/// the parse back on a server that signs many tokens under one key, and it
/// costs a table of private key material keyed by that material, living past
/// the request that read it — O(keys seen) rather than O(in-flight), and a
/// second place a secret exists. That is priority 1 spent to buy priority 3,
/// which is the one direction [AGENTS.md](/AGENTS.md)'s ordering does not go.
///
/// **What it spends:** two slots per pair, the wider of them the key's DER — a
/// little over a kilobyte for RSA at [`MAX_RSA_BITS`] and under a hundred
/// octets on every curve — plus one PKCS#8 parse per member call that needs the
/// key. `write` needs none: it answers the slot.
pub(crate) const KEY_PAIR: CoreClass = CoreClass {
    name: KEY_PAIR_NAME,
    doc: None,
    methods: &[CoreMethod {
        name: "read",
        // `$pkcs8` is `secret bytes` because that is what a private key is, and
        // neutral on the `tainted` axis because nothing it carries reaches the
        // answer: what crosses out is an object, which carries no qualifier at
        // all.
        names: &["pkcs8", "kind"],
        params: &[
            CoreTy::SecretBlob(Qual::Neutral),
            CoreTy::Enum(KEY_KIND_NAME),
        ],
        defaults: &[],
        return_ty: CoreTy::Instance(KEY_PAIR_NAME),
        symbol: "nvs_core_crypto_key_pair_read",
        doc: Some(&KEY_PAIR_READ_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "write",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::SecretBytes,
            symbol: "nvs_core_crypto_key_pair_write",
            doc: Some(&KEY_PAIR_WRITE_DOC),
        },
        CoreMethod {
            name: "publicKey",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(PUBLIC_KEY_NAME),
            symbol: "nvs_core_crypto_key_pair_public_key",
            doc: Some(&KEY_PAIR_PUBLIC_KEY_DOC),
        },
    ],
    slots: &["pkcs8", "kind"],
    constants: &[],
};

/// `Core\Crypto\KeyPair::read`'s reference card — `rule:core-api/reference-card`.
const KEY_PAIR_READ_DOC: MethodDoc = MethodDoc {
    short: "Reads a stored private key back, as DER PKCS#8 or as one PEM `PRIVATE KEY` block — \
            which is what a service-account file carries. The key is parsed here, so a pair that \
            was read is a pair every later member can use without looking at it again.",
    params: &[
        ParamDoc {
            name: "pkcs8",
            desc: "The key's own octets, as `write` answered them or as whoever issued the key \
                   wrote them.",
            shape: &[],
        },
        ParamDoc {
            name: "kind",
            desc: "Which key this is. It is named for `Core\\Crypto\\PublicKey::read`'s reason: a \
                   PKCS#8 says the key is RSA and never which of `RS256` and `PS256` it is for.",
            shape: &[],
        },
    ],
    ret: "The pair, ready to sign or to agree with, and answering `publicKey` with the half that \
          is sent.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$pkcs8` is not a PKCS#8 private key of that kind: a key of another kind, a \
               PKCS#1 body, an encrypted PKCS#8 — which has a password this member takes no \
               argument for — an RSA key outside the roster's 2048 to 8192 bits, or anything \
               malformed. A private key is the program's own and never a peer's, so there is no \
               verdict here to pass on anyone and every refusal is a bug in what the program was \
               handed.",
    }],
};

/// `Core\Crypto\KeyPair::write`'s reference card — `rule:core-api/reference-card`.
const KEY_PAIR_WRITE_DOC: MethodDoc = MethodDoc {
    short: "Answers this pair as DER PKCS#8, so a server can keep its key across requests and \
            read it back with `read`. It is `secret bytes`, which is the only way a private key \
            leaves a pair.",
    params: &[],
    ret: "The PKCS#8, as DER whichever of the two spellings the pair was read from.",
    errors: &[],
};

/// `Core\Crypto\KeyPair::publicKey`'s reference card — `rule:core-api/reference-card`.
const KEY_PAIR_PUBLIC_KEY_DOC: MethodDoc = MethodDoc {
    short: "Answers the half of this pair that is sent — derived from the private key rather \
            than stored beside it, so it is this pair's public key and cannot be a different \
            key that arrived with it.",
    params: &[],
    ret: "The public key, of the same kind as the pair, and the same value \
          `Core\\Crypto\\PublicKey::read` answers for the key material a peer would receive.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_crypto_generate_key" => (nvs_core_crypto_generate_key as *const ()).cast(),
        "nvs_core_crypto_seal" => (nvs_core_crypto_seal as *const ()).cast(),
        "nvs_core_crypto_open" => (nvs_core_crypto_open as *const ()).cast(),
        "nvs_core_crypto_derive_key" => (nvs_core_crypto_derive_key as *const ()).cast(),
        "nvs_core_crypto_expand_key" => (nvs_core_crypto_expand_key as *const ()).cast(),
        "nvs_core_crypto_generate_key_pair" => {
            (nvs_core_crypto_generate_key_pair as *const ()).cast()
        }
        "nvs_core_crypto_agree" => (nvs_core_crypto_agree as *const ()).cast(),
        "nvs_core_crypto_sign" => (nvs_core_crypto_sign as *const ()).cast(),
        "nvs_core_crypto_verify" => (nvs_core_crypto_verify as *const ()).cast(),
        "nvs_core_crypto_public_key_read" => (nvs_core_crypto_public_key_read as *const ()).cast(),
        "nvs_core_crypto_public_key_write" => {
            (nvs_core_crypto_public_key_write as *const ()).cast()
        }
        "nvs_core_crypto_public_key_kind" => (nvs_core_crypto_public_key_kind as *const ()).cast(),
        "nvs_core_crypto_key_pair_read" => (nvs_core_crypto_key_pair_read as *const ()).cast(),
        "nvs_core_crypto_key_pair_write" => (nvs_core_crypto_key_pair_write as *const ()).cast(),
        "nvs_core_crypto_key_pair_public_key" => {
            (nvs_core_crypto_key_pair_public_key as *const ()).cast()
        }
        _ => return None,
    })
}

/// The `bytes` in slot `index`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: the slot's type is written in the row
/// above, so a value of another tag is a compiled-code bug rather than anything
/// a program can write — `nvs_types` answers `E0401` to a call that gets one
/// wrong before any of this runs. [`text_of`] and [`rounds_of`] are the same
/// reader on the class's other two parameter types.
fn bytes_of<'a>(args: &'a [Value], index: usize, member: &str) -> Result<&'a [u8], Fault> {
    args[index].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `bytes`, got tag {}",
            args[index].tag_byte()
        ))
    })
}

/// The `string` in slot `index`, for [`bytes_of`]'s reason.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member.
fn text_of<'a>(args: &'a [Value], index: usize, member: &str) -> Result<&'a str, Fault> {
    args[index].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `string`, got tag {}",
            args[index].tag_byte()
        ))
    })
}

/// The `uint` in slot `index`, for [`bytes_of`]'s reason.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member.
fn rounds_of(args: &[Value], index: usize, member: &str) -> Result<u64, Fault> {
    args[index].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `uint`, got tag {}",
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
/// `bound` is the AEAD's additional data: it is authenticated by the tag and
/// carried nowhere, so the answer only opens where the caller states the same
/// bytes again. Every caller writes what the ciphertext may not be moved away
/// from — the application and the entry's name for `Core\Cache\Store::getSecret`
/// — and `&[]` where nothing but the key binds it, which is the empty-associated-
/// data construction and therefore the same octets this member produced before
/// the parameter existed.
///
/// # Errors
///
/// A `RuntimeError` when the sealed message is larger than this construction
/// can produce or than this process can hold — both unreachable from source,
/// and both explained where they are raised.
pub(crate) fn seal_under(
    ctx: &mut nvs_runtime::Ctx,
    cipher: &XChaCha20Poly1305,
    bound: &[u8],
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
    let body = cipher
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: message,
                aad: bound,
            },
        )
        .map_err(|_| {
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
/// that does not verify, a buffer too short to hold a nonce and a tag, the
/// wrong key, and `bound` naming something other than what the seal named are
/// indistinguishable to the caller, which is the module doc's *a forgery
/// throws* section as a return type. It is also what lets `Core\SignedCookie`
/// try a key ring: a caller that could tell "wrong key" from "altered" apart
/// would learn which of a rotated pair a forgery was aimed at.
///
/// `bound` is [`seal_under`]'s additional data, which travels with neither the
/// ciphertext nor the key: the caller states it again from what it knows, and a
/// ciphertext lifted out of the context that produced it does not open here.
///
/// # Errors
///
/// A `RuntimeError` when this process cannot spare the plaintext's buffer.
/// `who` names the member for it.
pub(crate) fn open_under(
    cipher: &XChaCha20Poly1305,
    bound: &[u8],
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
    Ok(cipher
        .decrypt(
            &XNonce::from(*nonce),
            Payload {
                msg: body,
                aad: bound,
            },
        )
        .ok())
}

/// AES-256-GCM keyed by `key`, or `None` for a `bytes` that is not a key.
///
/// [`cipher`]'s counterpart for the interop cipher, and the one place in this
/// tree an `Aes256Gcm` is built: `Core\Crypto`'s AES case and `Core\Jwe`'s
/// `A256GCM` are both on the near side of it, so what a browser reads back was
/// assembled by one piece of code either way.
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
pub(crate) fn derive_key(
    password: &[u8],
    salt: &[u8],
    iterations: u64,
    who: &str,
) -> Result<[u8; DERIVED_LEN], Fault> {
    if !(u64::from(MIN_ITERATIONS)..=u64::from(MAX_ITERATIONS)).contains(&iterations) {
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

    // The count is taken in the width a caller can write one in — a `uint`
    // argument and a JWE header's `p2c` are both wider than this construction's
    // — so that a number past the ceiling is *reported* as the number it was
    // rather than wrapped into a plausible one on the way in. Past the check
    // above it is three orders of magnitude below `u32::MAX`.
    let rounds = u32::try_from(iterations).expect("the ceiling is far below u32::MAX");
    Ok(pbkdf2_sha256(password, salt, rounds))
}

/// HKDF-SHA256 over `material`, answering [`DERIVED_LEN`] octets.
///
/// The other derivation, for material that is already uniform: extract with
/// `salt`, then expand under `info`, which is the context string separating two
/// keys derived from one secret. An empty `salt` is RFC 5869's own default of a
/// zero-filled one, since HMAC pads either to the same block. Handing a password
/// to this rather than to [`derive_key`] is the mistake the module doc names,
/// and no type here can catch it.
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
pub(crate) fn agree_p256(mine: &[u8], theirs: &P256PublicKey) -> Option<[u8; SHARED_LEN]> {
    let secret = P256SecretKey::from_slice(mine).ok()?;
    let shared = diffie_hellman(secret.to_nonzero_scalar(), theirs.as_affine());
    let mut agreed = [0_u8; SHARED_LEN];
    agreed.copy_from_slice(shared.raw_secret_bytes().as_slice());
    Some(agreed)
}

/// Which asymmetric key a member is naming, as the Rust side of [`KEY_KIND`].
///
/// [`SigningKey`]'s variants are these minus `X25519`, and the two types do not
/// collapse into one: this is what a program can *name*, that one is what a
/// signature can be *made with*, and keeping them apart is what stops a kind
/// that signs nothing from reaching a signer at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {
    /// [`KEY_KIND`]'s `P256`: NIST P-256, for agreement and for ECDSA.
    P256,
    /// [`KEY_KIND`]'s `X25519`: agreement and nothing else.
    X25519,
    /// [`KEY_KIND`]'s `Ed25519`: signatures and nothing else.
    Ed25519,
    /// [`KEY_KIND`]'s `RsaPkcs1`: an RSA key bound to `RS256`.
    RsaPkcs1,
    /// [`KEY_KIND`]'s `RsaPss`: the same key type bound to `PS256`.
    RsaPss,
}

impl KeyKind {
    /// The case an argument slot's integer names, or `None` for an integer
    /// naming no case.
    ///
    /// [`KEY_KIND`]'s written-out constants are the ABI this reads back, so the
    /// two lists are one list in two places and
    /// `the_key_enums_cover_exactly_the_registry_enums_cases` is what holds
    /// them together. `None` is not reachable from a compiled program — an
    /// argument of a closed enum type is one of its cases — so the member above
    /// reports it as a [`Fault::fatal`], exactly as [`keyed`] reports a cipher
    /// slot holding something else.
    pub(crate) fn from_tag(tag: i64) -> Option<Self> {
        Some(match tag {
            0 => Self::P256,
            1 => Self::X25519,
            2 => Self::Ed25519,
            3 => Self::RsaPkcs1,
            4 => Self::RsaPss,
            _ => return None,
        })
    }

    /// This case's [`KEY_KIND`] constant — [`Self::from_tag`]'s inverse, and
    /// what a member writes into a slot or answers `kind()` with.
    pub(crate) fn tag(self) -> i64 {
        match self {
            Self::P256 => 0,
            Self::X25519 => 1,
            Self::Ed25519 => 2,
            Self::RsaPkcs1 => 3,
            Self::RsaPss => 4,
        }
    }

    /// The `kty` a JWK of this kind carries.
    fn key_type(self) -> &'static str {
        match self {
            Self::P256 => "EC",
            Self::X25519 | Self::Ed25519 => "OKP",
            Self::RsaPkcs1 | Self::RsaPss => "RSA",
        }
    }

    /// The `crv` a JWK of this kind carries, and `None` for the kinds whose JWK
    /// names no curve — which is both RSA cases, one key type wearing two
    /// schemes that a document cannot tell apart.
    fn curve(self) -> Option<&'static str> {
        Some(match self {
            Self::P256 => "P-256",
            Self::X25519 => "X25519",
            Self::Ed25519 => "Ed25519",
            Self::RsaPkcs1 | Self::RsaPss => return None,
        })
    }
}

/// Which encoding a public key crosses in, as the Rust side of [`KEY_FORMAT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyFormat {
    /// [`KEY_FORMAT`]'s `Raw`: the key material with nothing around it.
    Raw,
    /// [`KEY_FORMAT`]'s `Spki`: X.509's `SubjectPublicKeyInfo`, in DER.
    Spki,
    /// [`KEY_FORMAT`]'s `Jwk`: RFC 7517's JSON object, as its UTF-8 octets.
    Jwk,
}

impl KeyFormat {
    /// The case an argument slot's integer names, read the way
    /// [`KeyKind::from_tag`] reads [`KEY_KIND`]'s.
    pub(crate) fn from_tag(tag: i64) -> Option<Self> {
        Some(match tag {
            0 => Self::Raw,
            1 => Self::Spki,
            2 => Self::Jwk,
            _ => return None,
        })
    }
}

/// Why a public key was not read, in the one distinction the codec can draw and
/// the member above it cannot.
///
/// `rule:core-classes/crypto-interop-tier`'s last paragraph is the split: a key
/// off the wire that fails is a `RuntimeError`, a verdict on whoever sent it,
/// and a malformed key the program built is a `LogicError`, a bug. Which of
/// those [`Octets`] becomes depends on where the octets came from, which only
/// the member knows. [`Bug`] never does, because it is the *call* that is
/// wrong rather than anything in the key, so no call site can be handed a
/// choice it could get wrong.
///
/// [`Bug`]: KeyRefusal::Bug
/// [`Octets`]: KeyRefusal::Octets
#[derive(Debug)]
pub(crate) enum KeyRefusal {
    /// The call is wrong however sound the octets are: an encoding the kind
    /// does not have, which is `Raw` against either RSA case, or a JWK carrying
    /// a private key's `d`.
    Bug,
    /// The octets are not a public key of that kind in that encoding — a point
    /// off the curve, a coordinate of the wrong width, a modulus outside the
    /// roster's range, a DER body that does not parse, a JWK naming another
    /// key type.
    Octets,
}

/// The public half of an asymmetric key, validated, in the variant naming its
/// kind.
///
/// [`VerifyingKey`]'s owning counterpart, with the one kind that signs nothing
/// beside the ones that do: a `Crypto\PublicKey` holds one of these for as long
/// as the program holds the object, and every later use of it — a signature
/// check, an agreement, an export — reads material that was checked once, when
/// it was read.
///
/// P-256 keeps both halves of itself because both get read: the parsed point is
/// what an agreement multiplies by, and the uncompressed encoding is what a
/// signature check and all three exports are over. Neither is derivable from
/// the other without arithmetic, so the [`P256_POINT_LEN`] octets are held
/// rather than recomputed. An RSA key's two components are stored the way a
/// JWK writes them — big-endian, no leading zero — so a key read from a DER
/// `INTEGER` and the same key read from a JWK are the same key, down to the
/// thumbprint.
pub(crate) enum PublicKey {
    /// NIST P-256.
    P256 {
        /// The point, parsed — what [`agree_p256`] multiplies by.
        point: P256PublicKey,
        /// The same point as `04 ‖ x ‖ y`.
        uncompressed: [u8; P256_POINT_LEN],
    },
    /// X25519, as the 32 octets of a Montgomery u-coordinate. Nothing about
    /// them is checkable at a read: the refusal that matters is an all-zero
    /// shared secret, and [`agree_x25519`] is where it happens.
    X25519 {
        /// The u-coordinate.
        point: [u8; CURVE25519_POINT_LEN],
    },
    /// Ed25519, as the 32 octets a JWK carries as `x`. A point that is not on
    /// the curve fails inside [`verify_signature`], which is the only thing
    /// this kind reaches.
    Ed25519 {
        /// The compressed point.
        point: [u8; CURVE25519_POINT_LEN],
    },
    /// An RSA key bound to RSASSA-PKCS1-v1_5 over SHA-256.
    RsaPkcs1 {
        /// The modulus, `n`.
        modulus: Vec<u8>,
        /// The public exponent, `e`.
        exponent: Vec<u8>,
    },
    /// The same key type bound to RSASSA-PSS over SHA-256.
    RsaPss {
        /// The modulus, `n`.
        modulus: Vec<u8>,
        /// The public exponent, `e`.
        exponent: Vec<u8>,
    },
}

impl PublicKey {
    /// The key `encoded` is, read as `kind` out of `format`, or why it is not
    /// one.
    ///
    /// `kind` is an argument rather than something read out of the octets, for
    /// `rule:security/algorithm-comes-from-the-key`'s reason carried to a key
    /// file: an RSA `SubjectPublicKeyInfo` says `rsaEncryption` whichever of
    /// `RS256` and `PS256` the key is for, so the program that was handed the
    /// key is the only party that can say, and this is the last place it is
    /// asked.
    pub(crate) fn read(
        encoded: &[u8],
        kind: KeyKind,
        format: KeyFormat,
    ) -> Result<Self, KeyRefusal> {
        match format {
            KeyFormat::Raw => Self::read_raw(encoded, kind),
            KeyFormat::Spki => Self::read_spki(encoded, kind),
            KeyFormat::Jwk => Self::read_jwk(encoded, kind),
        }
    }

    /// This key in `format`, or [`KeyRefusal::Bug`] for an encoding its kind
    /// does not have.
    ///
    /// The three curve kinds are written by putting a constant
    /// `SubjectPublicKeyInfo` prefix in front of a fixed-width key, so nothing
    /// in those branches can fail. RSA's wrapper is assembled by the DER
    /// encoder, whose own refusal is unreachable for a modulus the read bounded
    /// at [`MAX_RSA_BITS`]; it is reported as a bug rather than as a verdict,
    /// because the octets it would be a verdict on are this program's own.
    pub(crate) fn write(&self, format: KeyFormat) -> Result<Vec<u8>, KeyRefusal> {
        match format {
            KeyFormat::Raw => self.raw().ok_or(KeyRefusal::Bug),
            KeyFormat::Spki => self.spki(),
            KeyFormat::Jwk => Ok(self.jwk().into_bytes()),
        }
    }

    /// Which of [`KEY_KIND`]'s cases this key is, which is the kind its read
    /// was given and never a second reading of the material.
    ///
    /// `Core\Crypto\PublicKey::kind` does not reach this: the kind is one of
    /// [`PUBLIC_KEY`]'s slots, so the member answers it without a parse. What
    /// reaches it is a member holding a key it has already read —
    /// [`public_key_instance`] filling that slot for a key derived from a pair,
    /// and later a signature check picking its scheme or an agreement refusing
    /// two different curves.
    pub(crate) fn kind(&self) -> KeyKind {
        match self {
            Self::P256 { .. } => KeyKind::P256,
            Self::X25519 { .. } => KeyKind::X25519,
            Self::Ed25519 { .. } => KeyKind::Ed25519,
            Self::RsaPkcs1 { .. } => KeyKind::RsaPkcs1,
            Self::RsaPss { .. } => KeyKind::RsaPss,
        }
    }

    /// RFC 7638's thumbprint: the base64url of SHA-256 over [`Self::jwk`].
    ///
    /// It is what a `kid` is when nobody assigned one, and it is the same
    /// string whichever encoding the key arrived in, because it is taken over
    /// the key's members rather than over the octets that carried them.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "a later stage registers the member")
    )]
    pub(crate) fn thumbprint(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(self.jwk().as_bytes());
        URL_SAFE_NO_PAD.encode(digest.finalize())
    }

    /// This key as the borrowed shape [`verify_signature`] takes, and `None`
    /// for the one kind that signs nothing.
    ///
    /// The variant is this key's own, settled when it was read, so a check
    /// cannot be asked to run an algorithm the key is not bound to:
    /// `rule:security/algorithm-comes-from-the-key` crosses from the read to
    /// the check as a type rather than as an argument.
    pub(crate) fn verifying(&self) -> Option<VerifyingKey<'_>> {
        Some(match self {
            Self::P256 { uncompressed, .. } => VerifyingKey::P256 {
                point: uncompressed,
            },
            Self::Ed25519 { point } => VerifyingKey::Ed25519 { point },
            Self::RsaPkcs1 { modulus, exponent } => VerifyingKey::RsaPkcs1 { modulus, exponent },
            Self::RsaPss { modulus, exponent } => VerifyingKey::RsaPss { modulus, exponent },
            Self::X25519 { .. } => return None,
        })
    }

    /// The key material with nothing around it, which is defined for the three
    /// curve kinds and for neither RSA case.
    fn read_raw(encoded: &[u8], kind: KeyKind) -> Result<Self, KeyRefusal> {
        match kind {
            KeyKind::P256 => Self::p256(encoded),
            KeyKind::X25519 => Ok(Self::X25519 {
                point: curve25519_point(encoded)?,
            }),
            KeyKind::Ed25519 => Ok(Self::Ed25519 {
                point: curve25519_point(encoded)?,
            }),
            KeyKind::RsaPkcs1 | KeyKind::RsaPss => Err(KeyRefusal::Bug),
        }
    }

    /// X.509's `SubjectPublicKeyInfo`, in DER.
    ///
    /// The two 25519 kinds have a wrapper that is constant up to their 32
    /// octets of key, so [`X25519_SPKI_PREFIX`] and [`ED25519_SPKI_PREFIX`] are
    /// compared rather than parsed — there is no field in either a producer
    /// could have chosen. P-256 goes through `p256`'s own reader, which asserts
    /// both OIDs and ends in [`read_p256_point`]'s validation, and which
    /// accepts the compressed point a peer is entitled to export. Only RSA is
    /// walked here: its wrapper is variable-width and its `BIT STRING` carries
    /// a second layer, PKCS#1's own [`RsaComponents`].
    fn read_spki(encoded: &[u8], kind: KeyKind) -> Result<Self, KeyRefusal> {
        match kind {
            KeyKind::P256 => Self::p256_from(
                P256PublicKey::from_public_key_der(encoded).map_err(|_| KeyRefusal::Octets)?,
            ),
            KeyKind::X25519 => Ok(Self::X25519 {
                point: curve25519_point(spki_body(encoded, &X25519_SPKI_PREFIX)?)?,
            }),
            KeyKind::Ed25519 => Ok(Self::Ed25519 {
                point: curve25519_point(spki_body(encoded, &ED25519_SPKI_PREFIX)?)?,
            }),
            KeyKind::RsaPkcs1 | KeyKind::RsaPss => {
                let spki =
                    SubjectPublicKeyInfoRef::from_der(encoded).map_err(|_| KeyRefusal::Octets)?;
                let described = spki.algorithm.oid == RSA_OID
                    && spki
                        .algorithm
                        .parameters
                        .is_none_or(|parameters| parameters.is_null());
                if !described {
                    return Err(KeyRefusal::Octets);
                }

                let body = spki
                    .subject_public_key
                    .as_bytes()
                    .ok_or(KeyRefusal::Octets)?;
                let components = RsaComponents::from_der(body).map_err(|_| KeyRefusal::Octets)?;
                Self::rsa(
                    components.modulus.as_bytes(),
                    components.exponent.as_bytes(),
                    kind,
                )
            }
        }
    }

    /// RFC 7517's JSON object, as its UTF-8 octets.
    ///
    /// What a browser's `exportKey` writes is accepted whole: `ext`, `key_ops`,
    /// `alg`, `use` and `kid` are read and ignored, because the algorithm a key
    /// is under is the kind the call named and never a member of the document.
    /// `kty` and `crv` are the two that are *compared* — a document describing
    /// another key type is not this key in another encoding.
    ///
    /// **A coordinate is held to its own width**, not merely to the length of
    /// the point they assemble into: a 31-octet `x` beside a 33-octet `y`
    /// concatenates to exactly the octets of some valid point, so a reader
    /// checking only the total would read one key as another.
    fn read_jwk(encoded: &[u8], kind: KeyKind) -> Result<Self, KeyRefusal> {
        let document: serde_json::Value =
            serde_json::from_slice(encoded).map_err(|_| KeyRefusal::Octets)?;
        let members = document.as_object().ok_or(KeyRefusal::Octets)?;
        if members.contains_key("d") {
            return Err(KeyRefusal::Bug);
        }

        let text = |name: &str| members.get(name).and_then(serde_json::Value::as_str);
        if text("kty") != Some(kind.key_type()) || text("crv") != kind.curve() {
            return Err(KeyRefusal::Octets);
        }

        let member = |name: &str| {
            text(name)
                .and_then(|value| URL_SAFE_NO_PAD.decode(value).ok())
                .ok_or(KeyRefusal::Octets)
        };

        match kind {
            KeyKind::P256 => {
                let (x, y) = (member("x")?, member("y")?);
                if x.len() != P256_COORDINATE_LEN || y.len() != P256_COORDINATE_LEN {
                    return Err(KeyRefusal::Octets);
                }

                let mut point = Vec::with_capacity(P256_POINT_LEN);
                point.push(SEC1_UNCOMPRESSED);
                point.extend_from_slice(&x);
                point.extend_from_slice(&y);
                Self::p256(&point)
            }
            KeyKind::X25519 => Ok(Self::X25519 {
                point: curve25519_point(&member("x")?)?,
            }),
            KeyKind::Ed25519 => Ok(Self::Ed25519 {
                point: curve25519_point(&member("x")?)?,
            }),
            KeyKind::RsaPkcs1 | KeyKind::RsaPss => Self::rsa(&member("n")?, &member("e")?, kind),
        }
    }

    /// The parsed point an agreement multiplies by, and `None` for every kind
    /// that is not P-256.
    ///
    /// It is handed out already on the curve, because this is where that was
    /// checked: `rule:core-classes/crypto-interop-tier`'s *validated where it
    /// is read* is what lets [`agree_p256`] take a point and no length, and the
    /// invalid-curve attack is closed by there being no other way to get one.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "a later stage registers the member")
    )]
    pub(crate) fn p256_point(&self) -> Option<&P256PublicKey> {
        match self {
            Self::P256 { point, .. } => Some(point),
            Self::X25519 { .. }
            | Self::Ed25519 { .. }
            | Self::RsaPkcs1 { .. }
            | Self::RsaPss { .. } => None,
        }
    }

    /// A P-256 key out of whichever SEC1 encoding carried it, validated.
    fn p256(encoded: &[u8]) -> Result<Self, KeyRefusal> {
        Self::p256_from(read_p256_point(encoded).ok_or(KeyRefusal::Octets)?)
    }

    /// A P-256 key from a point already read, with the uncompressed encoding
    /// taken once here rather than at every export.
    fn p256_from(point: P256PublicKey) -> Result<Self, KeyRefusal> {
        let sec1 = point.to_sec1_point(false);
        let uncompressed =
            <[u8; P256_POINT_LEN]>::try_from(sec1.as_bytes()).map_err(|_| KeyRefusal::Octets)?;
        Ok(Self::P256 {
            point,
            uncompressed,
        })
    }

    /// An RSA key of `kind` from its two components, held to the roster's
    /// width.
    ///
    /// Both components are trimmed first, because one number arrives with a
    /// leading zero from a DER `INTEGER` whose top bit is set and without one
    /// from a JWK. The stored form is the JWK's, which is what makes the same
    /// key read from either encoding write the same JWK and answer the same
    /// thumbprint. The width is checked here and nowhere later.
    fn rsa(modulus: &[u8], exponent: &[u8], kind: KeyKind) -> Result<Self, KeyRefusal> {
        let modulus = trimmed(modulus);
        let exponent = trimmed(exponent);
        let top = *modulus.first().ok_or(KeyRefusal::Octets)?;
        let bits = u32::try_from(modulus.len())
            .ok()
            .and_then(|octets| octets.checked_mul(8))
            .and_then(|whole| whole.checked_sub(top.leading_zeros()))
            .ok_or(KeyRefusal::Octets)?;
        if !(MIN_RSA_BITS..=MAX_RSA_BITS).contains(&bits) || exponent.is_empty() {
            return Err(KeyRefusal::Octets);
        }

        let (modulus, exponent) = (modulus.to_vec(), exponent.to_vec());
        match kind {
            KeyKind::RsaPkcs1 => Ok(Self::RsaPkcs1 { modulus, exponent }),
            KeyKind::RsaPss => Ok(Self::RsaPss { modulus, exponent }),
            KeyKind::P256 | KeyKind::X25519 | KeyKind::Ed25519 => Err(KeyRefusal::Bug),
        }
    }

    /// The key material with nothing around it, and `None` for the RSA cases,
    /// which have no such encoding to write.
    fn raw(&self) -> Option<Vec<u8>> {
        Some(match self {
            Self::P256 { uncompressed, .. } => uncompressed.to_vec(),
            Self::X25519 { point } | Self::Ed25519 { point } => point.to_vec(),
            Self::RsaPkcs1 { .. } | Self::RsaPss { .. } => return None,
        })
    }

    /// This key's `SubjectPublicKeyInfo`, in DER.
    fn spki(&self) -> Result<Vec<u8>, KeyRefusal> {
        match self {
            Self::P256 { uncompressed, .. } => Ok(der_over(&P256_SPKI_PREFIX, uncompressed)),
            Self::X25519 { point } => Ok(der_over(&X25519_SPKI_PREFIX, point)),
            Self::Ed25519 { point } => Ok(der_over(&ED25519_SPKI_PREFIX, point)),
            Self::RsaPkcs1 { modulus, exponent } | Self::RsaPss { modulus, exponent } => {
                let components = RsaComponents {
                    modulus: UintRef::new(modulus).map_err(|_| KeyRefusal::Bug)?,
                    exponent: UintRef::new(exponent).map_err(|_| KeyRefusal::Bug)?,
                };
                let body = components.to_der().map_err(|_| KeyRefusal::Bug)?;

                SubjectPublicKeyInfo {
                    algorithm: AlgorithmIdentifierRef {
                        oid: RSA_OID,
                        parameters: Some(AnyRef::NULL),
                    },
                    subject_public_key: BitStringRef::new(0, &body).map_err(|_| KeyRefusal::Bug)?,
                }
                .to_der()
                .map_err(|_| KeyRefusal::Bug)
            }
        }
    }

    /// This key as RFC 7638's JWK: the members its kind requires, sorted, with
    /// no whitespace and nothing else.
    ///
    /// It is both [`Self::write`]'s answer and [`Self::thumbprint`]'s input, so
    /// the thumbprint is a digest of something a program can see rather than of
    /// a canonicalization only this function knows.
    fn jwk(&self) -> String {
        let spelled = |octets: &[u8]| URL_SAFE_NO_PAD.encode(octets);
        match self {
            Self::P256 { uncompressed, .. } => {
                let (x, y) = uncompressed[1..].split_at(P256_COORDINATE_LEN);
                format!(
                    r#"{{"crv":"P-256","kty":"EC","x":"{}","y":"{}"}}"#,
                    spelled(x),
                    spelled(y)
                )
            }
            Self::X25519 { point } => {
                format!(r#"{{"crv":"X25519","kty":"OKP","x":"{}"}}"#, spelled(point))
            }
            Self::Ed25519 { point } => {
                format!(
                    r#"{{"crv":"Ed25519","kty":"OKP","x":"{}"}}"#,
                    spelled(point)
                )
            }
            Self::RsaPkcs1 { modulus, exponent } | Self::RsaPss { modulus, exponent } => format!(
                r#"{{"e":"{}","kty":"RSA","n":"{}"}}"#,
                spelled(exponent),
                spelled(modulus)
            ),
        }
    }
}

/// PKCS#1's `RSAPublicKey ::= SEQUENCE { modulus INTEGER, publicExponent
/// INTEGER }`, which is the one DER layer nothing in this graph walks for us.
///
/// It is borrowed on the way in and on the way out both, because a modulus is
/// the largest thing this module copies and neither direction needs a second
/// one: [`UintRef`] is the canonical form — leading zeros stripped on the way
/// in, the sign octet put back on the way out — so the stored components are a
/// JWK's members whichever encoding they were read from.
struct RsaComponents<'a> {
    /// The modulus, `n`.
    modulus: UintRef<'a>,
    /// The public exponent, `e`.
    exponent: UintRef<'a>,
}

impl<'a> DecodeValue<'a> for RsaComponents<'a> {
    type Error = DerError;

    fn decode_value<R: Reader<'a>>(reader: &mut R, header: Header) -> Result<Self, DerError> {
        reader.read_nested(header.length(), |reader| {
            Ok(Self {
                modulus: reader.decode()?,
                exponent: reader.decode()?,
            })
        })
    }
}

impl EncodeValue for RsaComponents<'_> {
    fn value_len(&self) -> Result<Length, DerError> {
        self.modulus.encoded_len()? + self.exponent.encoded_len()?
    }

    fn encode_value(&self, writer: &mut impl Writer) -> Result<(), DerError> {
        self.modulus.encode(writer)?;
        self.exponent.encode(writer)
    }
}

impl<'a> Sequence<'a> for RsaComponents<'a> {}

/// PKCS#1's `rsaEncryption`, 1.2.840.113549.1.1.1 — the one algorithm
/// identifier an RSA `SubjectPublicKeyInfo` carries, whatever the key signs
/// with, which is why a kind is an argument to a read rather than something
/// read out of the file.
const RSA_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");

/// A P-256 key's whole `SubjectPublicKeyInfo` up to its point: the outer
/// `SEQUENCE`, `id-ecPublicKey` with `prime256v1` as its parameter, and the
/// `BIT STRING` header. Every octet of it is fixed because the point's width
/// is, so writing one is a prefix and never an encoder's decision.
const P256_SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

/// X25519's, which RFC 8410 fixes at OID 1.3.101.110 with no parameters.
const X25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e, 0x03, 0x21, 0x00,
];

/// Ed25519's, which is X25519's with the other of RFC 8410's two OIDs —
/// 1.3.101.112 — and the one octet of difference between them.
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

/// An X25519 private key's whole RFC 5958 version-1 PKCS#8 up to its scalar:
/// the outer `SEQUENCE`, `version` 0, RFC 8410's parameterless algorithm
/// identifier, and the two `OCTET STRING` headers a `CurvePrivateKey` sits
/// under. It is a prefix for [`P256_SPKI_PREFIX`]'s reason — the scalar's width
/// is fixed, so every other octet is — and it is the shortest form of the file
/// rather than version 2 with the public key attached, because the point is
/// derived from the scalar wherever one is wanted. [`x25519_scalar`] reads back
/// what this writes, and it walks the structure rather than matching this
/// constant, so a file another producer wrote is read just as well.
const X25519_PKCS8_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e, 0x04, 0x22, 0x04, 0x20,
];

/// Ed25519's, the same file with RFC 8410's other OID, which `ring` reads
/// through `Ed25519KeyPair::from_pkcs8_maybe_unchecked` — the door
/// [`PrivateKey::read`] already takes for the version-1 file WebCrypto exports.
const ED25519_PKCS8_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// `prefix` then `body`, which is the whole of writing a DER wrapper whose
/// every other octet is constant — a `SubjectPublicKeyInfo` over a
/// fixed-width point, and an RFC 8410 PKCS#8 over a fixed-width scalar.
fn der_over(prefix: &[u8], body: &[u8]) -> Vec<u8> {
    let mut der = Vec::with_capacity(prefix.len() + body.len());
    der.extend_from_slice(prefix);
    der.extend_from_slice(body);
    der
}

/// The key inside a `SubjectPublicKeyInfo` that is `prefix` and then a key, or
/// a refusal when those octets are not that wrapper.
fn spki_body<'a>(encoded: &'a [u8], prefix: &[u8]) -> Result<&'a [u8], KeyRefusal> {
    encoded.strip_prefix(prefix).ok_or(KeyRefusal::Octets)
}

/// The 32 octets X25519 and Ed25519 both carry, or a refusal for any other
/// width.
fn curve25519_point(encoded: &[u8]) -> Result<[u8; CURVE25519_POINT_LEN], KeyRefusal> {
    <[u8; CURVE25519_POINT_LEN]>::try_from(encoded).map_err(|_| KeyRefusal::Octets)
}

/// `octets` without the leading zeros a DER `INTEGER` carries and a JWK member
/// does not, so one number has one stored form here.
fn trimmed(octets: &[u8]) -> &[u8] {
    let leading = octets.iter().take_while(|octet| **octet == 0).count();
    &octets[leading..]
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

/// The public half of a signing key, as the material a JWK carries, in the
/// variant that names what it signs with.
///
/// One variant per roster algorithm, and there is no variant without material
/// and no material without a variant, which is the module doc's *four
/// signatures* section made into a type. An RSA key appears twice because
/// `RS256` and `PS256` are one key type and two algorithms: a reader picks the
/// variant once, and no later site can pick again.
pub(crate) enum VerifyingKey<'a> {
    /// RSASSA-PKCS1-v1_5 over SHA-256, from a JWK's `n` and `e` as big-endian
    /// octets with no leading zeros — which is what the base64url of those two
    /// members decodes to.
    RsaPkcs1 {
        /// The modulus, `n`.
        modulus: &'a [u8],
        /// The public exponent, `e`.
        exponent: &'a [u8],
    },
    /// RSASSA-PSS over SHA-256, over the same two members, with the salt length
    /// `PS256` fixes at the digest's own.
    RsaPss {
        /// The modulus, `n`.
        modulus: &'a [u8],
        /// The public exponent, `e`.
        exponent: &'a [u8],
    },
    /// ECDSA over P-256 and SHA-256, over the uncompressed point `04 ‖ x ‖ y`
    /// that a browser's raw export is and that a JWK's two coordinates assemble
    /// into. The compressed form is not read here: a key this module answers is
    /// written one way.
    P256 {
        /// The point, uncompressed.
        point: &'a [u8],
    },
    /// Ed25519, over the 32 octets a JWK carries as `x`.
    Ed25519 {
        /// The point, in the curve's only encoding.
        point: &'a [u8],
    },
}

/// `Some(())` when `signature` is this key's over `message`, and `None` for
/// every way of it not being.
///
/// One answer covers a signature that does not verify, a signature of a form
/// this algorithm does not write and material that is not a key of its kind, for
/// [`open_under`]'s reason: a caller able to tell them apart would learn which
/// half of a forgery attempt landed. It is `Option<()>` and not a `bool` because
/// the member above it turns `None` into the one sentence
/// `rule:security/verification-throws-and-compares-in-constant-time` asks for,
/// and a `bool` is the shape a call site can drop on the floor.
///
/// RSA's parameters carry the same 2048–8192 bit range that a `read` enforces,
/// so a key narrower than the roster admits fails here too rather than relying
/// on the door having been shut.
pub(crate) fn verify_signature(
    key: &VerifyingKey<'_>,
    message: &[u8],
    signature: &[u8],
) -> Option<()> {
    let verified = match key {
        VerifyingKey::RsaPkcs1 { modulus, exponent } => RsaPublicKeyComponents {
            n: *modulus,
            e: *exponent,
        }
        .verify(&RSA_PKCS1_2048_8192_SHA256, message, signature),
        VerifyingKey::RsaPss { modulus, exponent } => RsaPublicKeyComponents {
            n: *modulus,
            e: *exponent,
        }
        .verify(&RSA_PSS_2048_8192_SHA256, message, signature),
        VerifyingKey::P256 { point } => {
            UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, point).verify(message, signature)
        }
        VerifyingKey::Ed25519 { point } => {
            UnparsedPublicKey::new(&ED25519, point).verify(message, signature)
        }
    };

    verified.ok()
}

/// The private half of an asymmetric key, parsed out of its PKCS#8, in the
/// variant naming its kind.
///
/// [`PublicKey`]'s counterpart over the same five kinds, and the one reader a
/// `Crypto\KeyPair` has: every kind of the roster is read here, including the
/// one that signs nothing, because a pair is read before anybody knows what it
/// will be asked for. What each variant holds is what its own later use needs —
/// a scalar for the two kinds that agree, `ring`'s own pair for the two RSA
/// cases and Ed25519, where the parse is the validation.
///
/// [`SigningKey`] is this narrowed to the four kinds that sign, reachable only
/// through [`Self::signing`], so an X25519 pair reaches no signer at all rather
/// than reaching one and being refused there.
pub(crate) enum PrivateKey {
    /// NIST P-256, as the scalar both ECDSA and ECDH start from.
    P256(P256SecretKey),
    /// X25519's scalar, which agrees and signs nothing.
    X25519(StaticSecret),
    /// Ed25519, as `ring`'s pair: the seed and the point it derives, together.
    Ed25519(Ed25519KeyPair),
    /// An RSA key bound to RSASSA-PKCS1-v1_5 over SHA-256.
    RsaPkcs1(RsaKeyPair),
    /// The same key type bound to RSASSA-PSS over SHA-256.
    RsaPss(RsaKeyPair),
}

impl PrivateKey {
    /// The key `der` is, read as `kind`, or `None` when those octets are not a
    /// PKCS#8 private key of that kind.
    ///
    /// The refusal covers a key of another kind, a PKCS#1 body, an encrypted
    /// PKCS#8 and anything malformed. `kind` is named rather than read out of
    /// the file for [`PublicKey::read`]'s reason: PKCS#8 says *RSA* and never
    /// which of `RS256` and `PS256` the key is for.
    ///
    /// **P-256 is read the long way round on purpose.** `ring`'s own PKCS#8
    /// reader requires the optional public key that RFC 5958 leaves out, and
    /// what WebCrypto exports leaves it out, so the scalar is read with `p256`
    /// and the point is derived from it where one is needed. Ed25519 has the
    /// same file and `from_pkcs8_maybe_unchecked` is `ring`'s door onto it.
    pub(crate) fn read(der: &[u8], kind: KeyKind) -> Option<Self> {
        Some(match kind {
            KeyKind::P256 => Self::P256(P256SecretKey::from_pkcs8_der(der).ok()?),
            KeyKind::X25519 => Self::X25519(StaticSecret::from(x25519_scalar(der)?)),
            KeyKind::Ed25519 => {
                Self::Ed25519(Ed25519KeyPair::from_pkcs8_maybe_unchecked(der).ok()?)
            }
            KeyKind::RsaPkcs1 => Self::RsaPkcs1(RsaKeyPair::from_pkcs8(der).ok()?),
            KeyKind::RsaPss => Self::RsaPss(RsaKeyPair::from_pkcs8(der).ok()?),
        })
    }

    /// The public half, derived from the private one, and `None` where that
    /// derivation could not be a key this module reads.
    ///
    /// `None` is not reachable from a program: every branch derives its point
    /// or its components from material that has already been parsed as a key of
    /// that kind, and `ring` holds an RSA pair to the same 2048–8192 bits
    /// [`PublicKey::rsa`] does. The answer goes through this module's own
    /// constructors rather than being assembled, so a public key derived here
    /// and the same key read off the wire are one value.
    pub(crate) fn public(&self) -> Option<PublicKey> {
        /// `n` and `e` out of the PKCS#1 `RSAPublicKey` `ring` serializes,
        /// which is the same body a `SubjectPublicKeyInfo` carries.
        fn rsa(pair: &RsaKeyPair, kind: KeyKind) -> Option<PublicKey> {
            let components = RsaComponents::from_der(pair.public().as_ref()).ok()?;
            PublicKey::rsa(
                components.modulus.as_bytes(),
                components.exponent.as_bytes(),
                kind,
            )
            .ok()
        }

        match self {
            Self::P256(secret) => PublicKey::p256_from(secret.public_key()).ok(),
            Self::X25519(secret) => Some(PublicKey::X25519 {
                point: X25519PublicKey::from(secret).to_bytes(),
            }),
            Self::Ed25519(pair) => Some(PublicKey::Ed25519 {
                point: curve25519_point(pair.public_key().as_ref()).ok()?,
            }),
            Self::RsaPkcs1(pair) => rsa(pair, KeyKind::RsaPkcs1),
            Self::RsaPss(pair) => rsa(pair, KeyKind::RsaPss),
        }
    }

    /// This key as the shape [`sign`] takes, and `None` for the one kind that
    /// signs nothing.
    ///
    /// The signer is assembled here rather than at the read because P-256 needs
    /// both halves to build one and only a signature wants it;
    /// `rule:security/algorithm-comes-from-the-key` crosses from the read to the
    /// signature as a type, exactly as [`PublicKey::verifying`] carries it to a
    /// check.
    pub(crate) fn signing(self) -> Option<SigningKey> {
        Some(match self {
            Self::RsaPkcs1(pair) => SigningKey::RsaPkcs1(pair),
            Self::RsaPss(pair) => SigningKey::RsaPss(pair),
            Self::Ed25519(pair) => SigningKey::Ed25519(pair),
            Self::P256(secret) => {
                let point = secret.public_key().to_sec1_point(false);
                SigningKey::P256(
                    EcdsaKeyPair::from_private_key_and_public_key(
                        &ECDSA_P256_SHA256_FIXED_SIGNING,
                        &secret.to_bytes(),
                        point.as_bytes(),
                        &SystemRandom::new(),
                    )
                    .ok()?,
                )
            }
            Self::X25519(_) => return None,
        })
    }
}

/// A signing key, in the variant naming what it signs with.
///
/// [`VerifyingKey`]'s owning counterpart, built by [`PrivateKey::signing`] and
/// by nothing else, so the four variants here are exactly the kinds of the
/// roster that sign: a key that signs nothing cannot be spelled as one of
/// these.
pub(crate) enum SigningKey {
    /// RSASSA-PKCS1-v1_5 over SHA-256.
    RsaPkcs1(RsaKeyPair),
    /// RSASSA-PSS over SHA-256.
    RsaPss(RsaKeyPair),
    /// ECDSA over P-256 and SHA-256.
    P256(EcdsaKeyPair),
    /// Ed25519.
    Ed25519(Ed25519KeyPair),
}

/// X25519's private-key OID, 1.3.101.110 — RFC 8410's, which fixes the whole
/// algorithm identifier and leaves no parameters.
const X25519_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.101.110");

/// The 32-octet scalar inside an RFC 8410 X25519 PKCS#8, or `None` when those
/// octets are not one.
///
/// The one kind of the roster nothing else in this graph reads a private key
/// for: `p256` and `ring` each bring their own PKCS#8 reader and
/// `x25519-dalek` brings none, so the structure is walked here. It is walked
/// rather than compared against a constant prefix — which is what the two
/// `SubjectPublicKeyInfo` readers above do — because a private key file has
/// fields a producer chooses: RFC 5958's version 2 attaches the public key, and
/// a reader that had frozen the prefix would refuse it.
fn x25519_scalar(der: &[u8]) -> Option<[u8; KEY_LEN]> {
    let info = PrivateKeyInfoRef::from_der(der).ok()?;
    if info.algorithm.oid != X25519_OID || info.algorithm.parameters.is_some() {
        return None;
    }

    // RFC 8410's `CurvePrivateKey` is an `OCTET STRING` inside the `OCTET
    // STRING` every PKCS#8 private key sits in, so the scalar is one layer
    // further down than the field.
    let scalar = <&OctetStringRef>::from_der(info.private_key.as_bytes()).ok()?;
    <[u8; KEY_LEN]>::try_from(scalar.as_bytes()).ok()
}

/// `octets` as DER PKCS#8: themselves when they already are, or the body of one
/// PEM `PRIVATE KEY` block when they are that.
///
/// A service-account file is handed out in either spelling, so both are read and
/// one is stored — which makes `write` answer DER whichever arrived, rather than
/// making a pair remember a transport. **The label is the refusal**: `RSA
/// PRIVATE KEY` is PKCS#1 and `ENCRYPTED PRIVATE KEY` is a file with a password
/// this member takes no argument for, and neither matches, so both fall through
/// to a DER read that cannot succeed either.
///
/// The base64 is decoded here rather than with a PEM reader because this graph
/// carries none this crate can reach: `nvs_host::tls`'s is `rustls`'s, and
/// `der`'s own is behind a feature nothing turns on.
fn pkcs8_der(octets: &[u8]) -> Option<Vec<u8>> {
    /// One PEM block's armour, which is also the whole of what is checked.
    const HEADER: &str = "-----BEGIN PRIVATE KEY-----";
    /// Its closing half.
    const FOOTER: &str = "-----END PRIVATE KEY-----";

    let text = str::from_utf8(octets)
        .ok()
        .map(str::trim)
        .filter(|text| text.starts_with(HEADER));
    let Some(text) = text else {
        return Some(octets.to_vec());
    };

    let body: String = text
        .strip_prefix(HEADER)?
        .strip_suffix(FOOTER)?
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    STANDARD.decode(body).ok()
}

/// How many P-256 scalars are drawn before the generator is called broken.
///
/// A uniform 32-octet string is a scalar of this curve with probability about
/// 1 − 2⁻³², so a second draw is already out of reach of anything that ever
/// runs and this bound is here to keep the loop finite rather than because it
/// is approached.
const P256_DRAWS: usize = 8;

/// A freshly drawn private key of `kind`, as the PKCS#8 a [`KEY_PAIR`] slot
/// holds.
///
/// **The scalar is drawn through [`crate::random::draw`]** rather than through
/// any crate's own generator, so this tree has one CSPRNG and a
/// `#[Test(seed: …)]` reproduces a key pair exactly as it reproduces a nonce.
/// The two RFC 8410 kinds take any 32 octets, so the draw *is* the key and the
/// file around it is [`X25519_PKCS8_PREFIX`]'s constant. P-256's scalars are
/// the integers below the group order, so a draw that is not one is discarded
/// and another taken: reducing instead would bias the low end of the range,
/// and `p256` is what says which draws are scalars.
///
/// # Errors
///
/// A `LogicError` for either RSA kind, which this member draws none of, and a
/// [`Fault::fatal`] where the encoder or the generator fails — neither
/// reachable from a program, and both explained where they are raised.
pub(crate) fn generated_pkcs8(
    ctx: &mut nvs_runtime::Ctx,
    kind: KeyKind,
    who: &str,
) -> Result<Vec<u8>, Fault> {
    let curve25519 = |ctx: &mut nvs_runtime::Ctx, prefix: &[u8]| {
        let mut scalar = [0_u8; KEY_LEN];
        crate::random::draw(ctx, |rng| rng.fill_bytes(&mut scalar));
        der_over(prefix, &scalar)
    };

    match kind {
        KeyKind::X25519 => Ok(curve25519(ctx, &X25519_PKCS8_PREFIX)),
        KeyKind::Ed25519 => Ok(curve25519(ctx, &ED25519_PKCS8_PREFIX)),
        KeyKind::P256 => {
            let mut drawn = [0_u8; P256_COORDINATE_LEN];
            for _ in 0..P256_DRAWS {
                crate::random::draw(ctx, |rng| rng.fill_bytes(&mut drawn));
                let Ok(secret) = P256SecretKey::from_slice(&drawn) else {
                    continue;
                };
                // Unreachable from source: the encoder writes a fixed structure
                // over a scalar and a point the crate has just produced itself.
                let der = secret.to_pkcs8_der().map_err(|_| {
                    Fault::fatal(format!(
                        "{who} could not write a PKCS#8 for the P-256 scalar it had just drawn"
                    ))
                })?;
                return Ok(der.as_bytes().to_vec());
            }
            Err(Fault::fatal(format!(
                "{who} drew {P256_DRAWS} strings and not one of them was a P-256 scalar"
            )))
        }
        KeyKind::RsaPkcs1 | KeyKind::RsaPss => Err(rsa_is_never_drawn()),
    }
}

/// The refusal both RSA kinds get from `generateKeyPair`.
///
/// A `LogicError` for [`pair_refused`]'s reason — there is nobody a verdict
/// could be about — and it names the member that does take an RSA key, because
/// the program that asked for one has a key file it was issued and a call that
/// cannot use it. `ring` generates no RSA key at all, so this is what the
/// roster's *never generated* is made of rather than a policy laid over a
/// generator that exists.
fn rsa_is_never_drawn() -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        "Core\\Crypto::generateKeyPair(): an RSA key is never generated here. A program is \
         handed its RSA key by whoever issued it, so read that file with \
         `Core\\Crypto\\KeyPair::read`."
            .to_owned(),
    )
}

/// Why two keys agreed on nothing.
///
/// [`KeyRefusal`]'s split, drawn over a pair of keys rather than over one key's
/// octets: which of the two a refusal is says who made the mistake, and that is
/// what the member above turns into a `LogicError` or a `RuntimeError`.
#[derive(Debug)]
pub(crate) enum NoAgreement {
    /// These two keys are not two ends of one agreement — a private key of a
    /// kind that agrees nothing, or two keys on different curves. The call is
    /// wrong however sound both keys are.
    Keys,
    /// The peer's point contributes nothing, so the secret is all-zero whatever
    /// this program's scalar is. A point is chosen by whoever sent it.
    Point,
}

/// The secret `mine` and `theirs` agree on, or why they agree on none.
///
/// Both curves of the roster and no other kind: Ed25519 signs and agrees
/// nothing, RSA has no agreement at all, and a pair of keys on two different
/// curves is not a pair. The two curves refuse in different places for
/// [`agree_x25519`]'s reason — P-256 has already refused a bad point at the
/// read, and X25519's refusal is on the way out — so the branch that can answer
/// [`NoAgreement::Point`] is the X25519 one, and P-256's `None` is a scalar
/// this module has itself already parsed twice and is therefore not reachable.
pub(crate) fn agree(
    mine: &PrivateKey,
    theirs: &PublicKey,
) -> Result<[u8; SHARED_LEN], NoAgreement> {
    match (mine, theirs) {
        (PrivateKey::P256(secret), PublicKey::P256 { point, .. }) => {
            agree_p256(&secret.to_bytes(), point).ok_or(NoAgreement::Keys)
        }
        (PrivateKey::X25519(secret), PublicKey::X25519 { point }) => {
            agree_x25519(&secret.to_bytes(), point).ok_or(NoAgreement::Point)
        }
        _ => Err(NoAgreement::Keys),
    }
}

/// The signature `key` writes over `message`, or `None` when it could not be
/// produced.
///
/// `None` here is not a verdict on anything — a key that reached this point is
/// already a key — so the member above it reports a failure of the machine
/// rather than of a message. It is reachable only through the entropy source
/// failing, which the two randomized algorithms below need and the other two
/// carry an unused argument for.
///
/// ECDSA answers the 64-octet `r ‖ s` rather than DER, for the module doc's
/// *four signatures* reason, and RSA answers as many octets as the modulus is
/// long.
pub(crate) fn sign(key: &SigningKey, message: &[u8]) -> Option<Vec<u8>> {
    /// One RSA signature under whichever padding the key is bound to, into a
    /// buffer the modulus's own width, which is the only width `ring` writes.
    fn rsa(
        pair: &RsaKeyPair,
        padding: &'static dyn RsaEncoding,
        rng: &SystemRandom,
        message: &[u8],
    ) -> Option<Vec<u8>> {
        let mut signature = vec![0_u8; pair.public().modulus_len()];
        pair.sign(padding, rng, message, &mut signature).ok()?;
        Some(signature)
    }

    let rng = SystemRandom::new();

    match key {
        SigningKey::RsaPkcs1(pair) => rsa(pair, &RSA_PKCS1_SHA256, &rng, message),
        SigningKey::RsaPss(pair) => rsa(pair, &RSA_PSS_SHA256, &rng, message),
        SigningKey::P256(pair) => pair
            .sign(&rng, message)
            .ok()
            .map(|signature| signature.as_ref().to_vec()),
        SigningKey::Ed25519(pair) => Some(pair.sign(message).as_ref().to_vec()),
    }
}

/// One of [`CIPHER`]'s constructions, keyed and ready to seal or open.
///
/// The two variants are different sizes — a round-key schedule against a
/// 32-octet key — and boxing the larger would buy an allocation on the request
/// path to save a few hundred bytes of stack that live for one call, which is
/// priority 3 spent on priority 5.
#[allow(clippy::large_enum_variant)]
enum Keyed {
    /// [`CIPHER`]'s `XChaCha20Poly1305`, through [`cipher`].
    Extended(XChaCha20Poly1305),
    /// [`CIPHER`]'s `Aes256Gcm`, through [`gcm_cipher`].
    Interop(Aes256Gcm),
}

/// The construction slot 2 names, keyed by slot 1, or the `LogicError` a
/// wrong-length key earns.
///
/// The length refusal is reachable from source despite the parameter's `secret
/// bytes`: the qualifier is about confidentiality and says nothing about
/// length, and a plain `bytes` of any size widens onto it.
/// `Core\Random::bytes(8)` is the one-line witness. The cipher slot is not
/// reachable that way — an argument of a closed enum type is one of its cases
/// or the program did not compile — so anything else there is a
/// [`Fault::fatal`], exactly as a wrong tag in [`bytes_of`] is.
///
/// One key length keys either construction, which is why `generateKey` answers
/// a key with no cipher named and this function checks the width once for both.
fn keyed(args: &[Value], member: &str) -> Result<Keyed, Fault> {
    let key = bytes_of(args, 1, member)?;
    let keyed = match args[2].as_int() {
        Some(0) => cipher(key).map(Keyed::Extended),
        Some(1) => gcm_cipher(key).map(Keyed::Interop),
        _ => {
            return Err(Fault::fatal(format!(
                "{NAME}::{member} expected a `{CIPHER_NAME}` case, got tag {} value {:?}",
                args[2].tag_byte(),
                args[2].as_int()
            )));
        }
    };
    keyed.ok_or_else(|| wrong_key_length(&format!("{NAME}::{member}"), "$key", key.len()))
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
    /// `Core\Crypto::seal(bytes $message, secret bytes $key, Cipher $cipher): bytes`
    /// — replacing `openssl_encrypt` and `sodium_crypto_aead_*_encrypt`, with
    /// the mode, the padding and the nonce taken off the call and the cipher
    /// left on it as a case rather than a string.
    ///
    /// The nonce is drawn before anything is allocated and prefixed to the
    /// answer, which is what lets [`nvs_core_crypto_open`] take no IV where
    /// PHP's pair takes one back as a further argument. Its width is the
    /// cipher's, and for the interop construction it is drawn here rather than
    /// inside [`gcm_seal_under`], whose caller in `Core\Jwe` has a nonce
    /// already written down in a header.
    fn nvs_core_crypto_seal(ctx, args: [3]) {
        let message = bytes_of(args, 0, "seal")?;
        let sealed = match keyed(args, "seal")? {
            Keyed::Extended(cipher) => {
                seal_under(ctx, &cipher, &[], message, "Core\\Crypto::seal")?
            }
            Keyed::Interop(cipher) => {
                let mut nonce = [0_u8; GCM_NONCE_LEN];
                crate::random::draw(ctx, |rng| rng.fill_bytes(&mut nonce));
                gcm_seal_under(&cipher, &nonce, message, "Core\\Crypto::seal")?
            }
        };
        Ok(Value::bytes(NvsStr::new(&sealed)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::open(bytes $sealed, secret bytes $key, Cipher $cipher): bytes`
    /// — replacing `openssl_decrypt`, which answers `false` on a forgery when
    /// it notices one at all.
    ///
    /// Every way `$sealed` can fail to be an authentic message under `$key` and
    /// `$cipher` is one throw with one sentence; the module doc's *a forgery
    /// throws* section is why, and it is a security property rather than a
    /// simplification. Naming the cipher the message was not sealed under is
    /// one of those ways: the nonce widths differ and the tag would not verify
    /// even if they agreed, so it lands on the same refusal rather than on
    /// plausible bytes.
    fn nvs_core_crypto_open(_ctx, args: [3]) {
        let sealed = bytes_of(args, 0, "open")?;

        // A buffer too short to hold a nonce and a tag, a tag that does not
        // verify, the wrong key and the wrong cipher are one `None` out of the
        // construction and one sentence here, deliberately not four.
        let opened = match keyed(args, "open")? {
            Keyed::Extended(cipher) => open_under(&cipher, &[], sealed, "Core\\Crypto::open")?,
            Keyed::Interop(cipher) => gcm_open_under(&cipher, sealed, "Core\\Crypto::open")?,
        };
        let plain = opened.ok_or_else(|| {
            Fault::thrown(
                "Core\\Crypto::open(): $sealed is not an authentic message under $key — it \
                 has been altered, it is too short to be one, or the key is not the one it \
                 was sealed under"
            )
        })?;

        Ok(Value::bytes(NvsStr::new(&plain)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::deriveKey(secret string $password, bytes $salt, uint $iterations): secret bytes`
    /// — replacing `hash_pbkdf2("sha256", …, $length, $raw)`, whose digest, output
    /// length and raw-or-hex flag are four ways to get one derivation wrong.
    ///
    /// The count is a required argument and the two bounds are
    /// [`derive_key`]'s, which is also PBES2's door onto the same construction:
    /// a `p2c` a token carries and a number a program wrote are the same
    /// question asked of one check.
    fn nvs_core_crypto_derive_key(_ctx, args: [3]) {
        let password = text_of(args, 0, "deriveKey")?;
        let salt = bytes_of(args, 1, "deriveKey")?;
        let iterations = rounds_of(args, 2, "deriveKey")?;

        let key = derive_key(
            password.as_bytes(),
            salt,
            iterations,
            "Core\\Crypto::deriveKey",
        )?;
        Ok(Value::bytes(NvsStr::new(&key)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::expandKey(secret bytes $material, bytes $salt, string $info): secret bytes`
    /// — replacing `hash_hkdf`, which is the one PHP twin here that is already
    /// the right shape, with its digest argument taken off the call for
    /// [`CIPHER`]'s reason.
    ///
    /// No bound to check and nothing to refuse: [`expand_key`] answers 32
    /// octets for any three arguments, and the mistake it cannot catch —
    /// a password handed to the derivation for uniform material — is named in
    /// both reference cards and in the module doc rather than reported here,
    /// because no type tells the two secrets apart.
    fn nvs_core_crypto_expand_key(_ctx, args: [3]) {
        let material = bytes_of(args, 0, "expandKey")?;
        let salt = bytes_of(args, 1, "expandKey")?;
        let info = text_of(args, 2, "expandKey")?;

        let key = expand_key(material, salt, info.as_bytes());
        Ok(Value::bytes(NvsStr::new(&key)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::generateKeyPair(Crypto\KeyKind $kind): Crypto\KeyPair` —
    /// replacing `openssl_pkey_new` and its configuration array, with the curve
    /// named by a case and nothing else on the call.
    ///
    /// The answer is built the way [`nvs_core_crypto_key_pair_read`] builds
    /// one, out of a PKCS#8 and the kind, so a generated pair and a read pair
    /// are one value: this member is a second producer of that object and not
    /// a second kind of it. [`generated_pkcs8`] owns where the scalar comes
    /// from and which kinds there are none for.
    fn nvs_core_crypto_generate_key_pair(ctx, args: [1]) {
        let kind = key_kind_of(args, 0, NAME, "generateKeyPair")?;
        let der = generated_pkcs8(ctx, kind, "Core\\Crypto::generateKeyPair")?;
        nvs_runtime::affordable(Some(der.len()), "Core\\Crypto::generateKeyPair")?;
        Ok(crate::instance::build(
            &KEY_PAIR,
            [Value::bytes(NvsStr::new(&der)), Value::int(kind.tag())],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::agree(Crypto\KeyPair $mine, Crypto\PublicKey $theirs): secret bytes`
    /// — replacing `openssl_pkey_derive`, with the curve coming from the two
    /// keys rather than from a call that could name a third.
    ///
    /// The answer is the raw agreed coordinate and is deliberately not a key:
    /// [`expand_key`] under a context string is what a protocol keys from, and
    /// the reference card says so. Which of the two refusals a failure is,
    /// [`agree`] decides and this body only spells — a `LogicError` about the
    /// program's own keys, and a `RuntimeError` about the point a peer chose.
    fn nvs_core_crypto_agree(_ctx, args: [2]) {
        let mine = pair_of(args, 0, "agree")?;
        let theirs = key_of(args, 1, "agree")?;
        let shared = agree(&mine, &theirs).map_err(|no| match no {
            NoAgreement::Keys => not_one_agreement(),
            NoAgreement::Point => point_contributes_nothing(),
        })?;
        Ok(Value::bytes(NvsStr::new(&shared)))
    }
}

/// The refusal `Core\Crypto::agree` gives two keys that are not two ends of one
/// agreement.
///
/// A `LogicError` because both keys are the program's own doing: the pair it
/// generated or read, and a peer's key it chose to hand to this member. The
/// sentence names the two curves rather than which of the two keys was wrong,
/// since either way the call is the thing to fix.
fn not_one_agreement() -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        "Core\\Crypto::agree(): these two keys are not two ends of one agreement. Both have to \
         be of one curve, `P256` or `X25519` — `Ed25519` signs and agrees nothing, and neither \
         RSA kind agrees at all."
            .to_owned(),
    )
}

/// The refusal `Core\Crypto::agree` gives a peer's X25519 point that drives the
/// secret to zero.
///
/// A `RuntimeError` because it is a verdict on whoever sent the point: a point
/// of small order makes the shared secret all-zero whatever this program's
/// scalar is, which is a peer choosing the key for both ends. The module doc's
/// *two curves check a peer's point in different places* owns why this is the
/// check X25519 gets and P-256 gets at its read.
fn point_contributes_nothing() -> Fault {
    Fault::thrown(
        "Core\\Crypto::agree(): this X25519 public key contributes nothing to the shared secret, \
         so the secret it would agree is not one this program's own key had any part in."
            .to_owned(),
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::sign(bytes $message, Crypto\KeyPair $key): bytes` —
    /// replacing `openssl_sign`, whose digest is a further argument that can
    /// disagree with the key it is written beside.
    ///
    /// There is nowhere here to name a scheme: [`PrivateKey::signing`] answers
    /// the variant the pair was read as, which is
    /// `rule:security/algorithm-comes-from-the-key` crossing from the read to
    /// the signature as a type. [`sign`] owns what each kind answers, including
    /// ECDSA's `r ‖ s` rather than DER.
    fn nvs_core_crypto_sign(_ctx, args: [2]) {
        let message = bytes_of(args, 0, "sign")?;
        let key = pair_of(args, 1, "sign")?.signing().ok_or_else(signs_nothing)?;

        // Unreachable from source: the two randomized algorithms are the only
        // ones that can answer `None` here and they do it only where `ring`'s
        // generator has failed, which is the machine and not the program. A key
        // that reached this line is already a key, so there is nothing a
        // `catch` could be about; [`sign`]'s own doc owns the rest.
        let signature = sign(&key, message).ok_or_else(|| {
            Fault::fatal("Core\\Crypto::sign could not draw the randomness a signature needs")
        })?;
        nvs_runtime::affordable(Some(signature.len()), "Core\\Crypto::sign")?;
        Ok(Value::bytes(NvsStr::new(&signature)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto::verify(bytes $message, bytes $signature, Crypto\PublicKey $key): void`
    /// — replacing `openssl_verify`, which answers `1`, `0` or `-1` and leaves
    /// the caller to tell the last two apart.
    ///
    /// Nothing is answered, so there is no value a loose comparison could
    /// misread and no raw check for a call site to write its own `==` over:
    /// `rule:security/verification-throws-and-compares-in-constant-time` is
    /// kept by the return type rather than by a caller's discipline. Which
    /// algorithm runs is [`PublicKey::verifying`]'s answer and never the
    /// signature's shape.
    fn nvs_core_crypto_verify(_ctx, args: [3]) {
        let message = bytes_of(args, 0, "verify")?;
        let signature = bytes_of(args, 1, "verify")?;
        let key = key_of(args, 2, "verify")?;
        let verifying = key.verifying().ok_or_else(verifies_nothing)?;
        verify_signature(&verifying, message, signature).ok_or_else(not_this_signature)?;
        Ok(Value::null())
    }
}

/// The refusal `Core\Crypto::sign` gives the one kind of pair that signs
/// nothing.
///
/// A `LogicError` because the pair is the program's own: it drew or read an
/// X25519 key and then asked that key for a signature, so the call is the thing
/// to fix. [`PrivateKey::signing`]'s `None` is the only way here.
fn signs_nothing() -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        "Core\\Crypto::sign(): an `X25519` pair signs nothing — it is a key for \
         `Core\\Crypto::agree` alone. `P256`, `Ed25519` and both RSA kinds sign."
            .to_owned(),
    )
}

/// The refusal `Core\Crypto::verify` gives the one kind of public key that
/// verifies nothing.
///
/// A `LogicError` rather than the verdict below it, on the same line
/// [`not_one_agreement`] draws: the key is one this program read and chose to
/// check against, and no signature could have made it the right kind.
fn verifies_nothing() -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        "Core\\Crypto::verify(): an `X25519` public key verifies nothing — it is a key for \
         `Core\\Crypto::agree` alone. `P256`, `Ed25519` and both RSA kinds verify."
            .to_owned(),
    )
}

/// The one sentence every check that did not hold gives.
///
/// A forgery, a signature of the wrong width for the key's algorithm and a
/// signature made under a different key are one refusal deliberately, for the
/// reason `rule:security/verification-throws-and-compares-in-constant-time`
/// gives and [`nvs_core_crypto_open`]'s refusal keeps for a sealed message:
/// a message naming which part was wrong is an oracle over the part.
fn not_this_signature() -> Fault {
    Fault::thrown(
        "Core\\Crypto::verify(): this signature is not $key's over $message — it has been \
         altered, it is not the shape the key's algorithm signs, or it was made under another \
         key."
            .to_owned(),
    )
}

/// The [`Fault`] a [`KeyRefusal`] becomes at the member that took the octets.
///
/// `rule:core-classes/crypto-interop-tier`'s split, written once: octets that
/// are not a key are a verdict on whoever sent them, and a call naming an
/// encoding its kind does not have is a bug in the program that wrote the call.
/// [`KeyRefusal`]'s own doc owns why the codec cannot draw that line itself.
fn key_refused(refusal: &KeyRefusal) -> Fault {
    match refusal {
        KeyRefusal::Bug => Fault::thrown_as(
            ThrownClass::Logic,
            "Core\\Crypto\\PublicKey::read(): this call names an encoding the kind does not \
             have, or hands a private key over as a public one — `Raw` is not a form either \
             RSA kind has, and a JWK carrying `d` is the private half."
                .to_owned(),
        ),
        KeyRefusal::Octets => Fault::thrown(
            "Core\\Crypto\\PublicKey::read(): these octets are not a public key of the kind \
             and encoding this call named."
                .to_owned(),
        ),
    }
}

/// The `bytes` in slot `index`, named for `class` rather than for [`CLASS`] —
/// the two key classes share every reader here.
///
/// # Errors
///
/// A [`Fault::fatal`], on [`bytes_of`]'s reading: the slot's type is written in
/// the row above, so a value of another tag is a compiled-code bug.
fn key_octets<'a>(
    args: &'a [Value],
    index: usize,
    class: &str,
    member: &str,
) -> Result<&'a [u8], Fault> {
    args[index].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{class}::{member} expected a `bytes`, got tag {}",
            args[index].tag_byte()
        ))
    })
}

/// The [`KEY_KIND`] case in slot `index`, read the way [`keyed`] reads a cipher.
///
/// # Errors
///
/// A [`Fault::fatal`] for [`key_octets`]'s reason: an argument of a closed enum
/// type is one of its cases before any of this runs.
fn key_kind_of(args: &[Value], index: usize, class: &str, member: &str) -> Result<KeyKind, Fault> {
    args[index]
        .as_int()
        .and_then(KeyKind::from_tag)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{class}::{member} expected a `{KEY_KIND_NAME}` case, got tag {}",
                args[index].tag_byte()
            ))
        })
}

/// The [`KEY_FORMAT`] case in slot `index`, for [`key_kind_of`]'s reason.
///
/// # Errors
///
/// A [`Fault::fatal`], as above.
fn key_format_of(args: &[Value], index: usize, member: &str) -> Result<KeyFormat, Fault> {
    args[index]
        .as_int()
        .and_then(KeyFormat::from_tag)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{PUBLIC_KEY_NAME}::{member} expected a `{KEY_FORMAT_NAME}` case, got tag {}",
                args[index].tag_byte()
            ))
        })
}

/// The key the instance in slot `index` holds, read back out of its own slots
/// — the receiver at `0` on an instance member, and an argument anywhere else.
///
/// The re-read is [`PUBLIC_KEY`]'s decision rather than this reader's
/// convenience, and that doc owns it: a slot holds a Novis value, so the parsed
/// key is not what the object keeps, and going back through [`PublicKey::read`]
/// is what leaves every later member's key exactly as validated as the first
/// read left it.
///
/// # Errors
///
/// A [`Fault::fatal`] on a receiver or a slot holding something else, which
/// compiled code cannot produce: both slots are written by
/// [`nvs_core_crypto_public_key_read`] and by nothing else.
fn key_of(args: &[Value], index: usize, member: &str) -> Result<PublicKey, Fault> {
    let receiver = crate::instance::receiver(args[index], &PUBLIC_KEY, member)?;
    let held = crate::instance::slot(receiver, PUBLIC_KEY_SPKI_SLOT);
    let encoded = held.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{PUBLIC_KEY_NAME}::{member} expected a `bytes` in its `{}` slot",
            PUBLIC_KEY.slots[PUBLIC_KEY_SPKI_SLOT]
        ))
    })?;
    let kind = crate::instance::slot(receiver, PUBLIC_KEY_KIND_SLOT)
        .as_int()
        .and_then(KeyKind::from_tag)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{PUBLIC_KEY_NAME}::{member} expected a `{KEY_KIND_NAME}` case in its `{}` slot",
                PUBLIC_KEY.slots[PUBLIC_KEY_KIND_SLOT]
            ))
        })?;
    // Unreachable from source: the slot holds what the read above wrote, after
    // a parse that had already succeeded over the same kind and encoding.
    PublicKey::read(encoded, kind, KeyFormat::Spki).map_err(|_| {
        Fault::fatal(format!(
            "{PUBLIC_KEY_NAME}::{member} expected a key in its `{}` slot",
            PUBLIC_KEY.slots[PUBLIC_KEY_SPKI_SLOT]
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto\PublicKey::read(bytes $encoded, Crypto\KeyKind $kind, Crypto\KeyFormat $format): Crypto\PublicKey`
    /// — the one door onto [`PublicKey::read`].
    ///
    /// What the object keeps is the canonical `SubjectPublicKeyInfo` and not the
    /// octets it was handed, so a key read from a JWK and the same key read from
    /// its DER are one value down to the slot. [`PUBLIC_KEY`]'s own doc owns the
    /// layout and what it spends.
    fn nvs_core_crypto_public_key_read(_ctx, args: [3]) {
        let encoded = key_octets(args, 0, PUBLIC_KEY_NAME, "read")?;
        let kind = key_kind_of(args, 1, PUBLIC_KEY_NAME, "read")?;
        let format = key_format_of(args, 2, "read")?;
        let key = PublicKey::read(encoded, kind, format).map_err(|refusal| key_refused(&refusal))?;
        public_key_instance(&key, "Core\\Crypto\\PublicKey::read")
    }
}

/// A `Core\Crypto\PublicKey` over `key`, which is the one place [`PUBLIC_KEY`]'s
/// two slots are written.
///
/// Both members that answer a public key come through here — the read of one a
/// peer sent and the derivation of one from a pair — so a key is one value
/// whichever way a program reached it, down to the slot.
///
/// # Errors
///
/// A [`Fault::fatal`] where the key cannot be written as a
/// `SubjectPublicKeyInfo`, and whatever `nvs_runtime::affordable` answers for a
/// request at its ceiling.
fn public_key_instance(key: &PublicKey, site: &str) -> Result<Value, Fault> {
    // Unreachable from source: writing a `SubjectPublicKeyInfo` refuses only
    // where the DER encoder does, over a modulus its reader has already held
    // inside the roster's range, and the curve kinds' branches are a constant
    // prefix in front of a fixed-width key.
    let spki = key
        .write(KeyFormat::Spki)
        .map_err(|_| Fault::fatal(format!("{site}() could not write the key it had just read")))?;
    nvs_runtime::affordable(Some(spki.len()), site)?;
    Ok(crate::instance::build(
        &PUBLIC_KEY,
        [
            Value::bytes(NvsStr::new(&spki)),
            Value::int(key.kind().tag()),
        ],
    ))
}

nvs_runtime::nvs_helper! {
    /// `$publicKey->write(Crypto\KeyFormat $format): bytes` — this key in any of
    /// the encodings `read` accepts.
    ///
    /// The answer is written from the key rather than copied out of the octets
    /// that carried it, which is what makes a JWK export the minimal one RFC
    /// 7638 defines whatever `ext` and `key_ops` the browser had put in the
    /// document this key was read from.
    fn nvs_core_crypto_public_key_write(_ctx, args: [2]) {
        let key = key_of(args, 0, "write")?;
        let format = key_format_of(args, 1, "write")?;
        let written = key.write(format).map_err(|_| {
            Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Crypto\\PublicKey::write(): an RSA key has no `Raw` form, so there is \
                 nothing to write — `Spki` and `Jwk` are the encodings it has."
                    .to_owned(),
            )
        })?;
        nvs_runtime::affordable(Some(written.len()), "Core\\Crypto\\PublicKey::write")?;
        Ok(Value::bytes(NvsStr::new(&written)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$publicKey->kind(): Crypto\KeyKind` — the case this key was read as.
    ///
    /// The one member here that does not re-read the key, because the kind is a
    /// slot rather than something the material says: an RSA
    /// `SubjectPublicKeyInfo` names `rsaEncryption` under either scheme, and
    /// `rule:security/algorithm-comes-from-the-key` is why the answer is the
    /// program's own word taken at the read.
    fn nvs_core_crypto_public_key_kind(_ctx, args: [1]) {
        let held = crate::instance::read_slot(args, &PUBLIC_KEY, PUBLIC_KEY_KIND_SLOT, "kind")?;
        Ok(held)
    }
}

/// The one refusal `Core\Crypto\KeyPair::read` has.
///
/// A `LogicError` rather than [`key_refused`]'s two-way split, and not because
/// the reasons could not be told apart: a private key is the program's own, so
/// there is nobody a verdict could be about, and every way of failing is a
/// mistake in what the program was deployed with. The sentence names what is
/// accepted rather than what was wrong with these octets, which is what makes
/// it actionable without being a report on a file this member will not read
/// twice.
fn pair_refused() -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        "Core\\Crypto\\KeyPair::read(): these octets are not a PKCS#8 private key of the kind \
         this call named — DER or one PEM `PRIVATE KEY` block, never a PKCS#1 body and never an \
         encrypted PKCS#8."
            .to_owned(),
    )
}

/// The pair the instance in slot `index` holds, read back out of its own slots
/// — [`key_of`]'s reading of a slot number, over the other key class.
///
/// The re-read is [`KEY_PAIR`]'s decision rather than this reader's
/// convenience, and that doc owns it: a slot holds a Novis value, so the parsed
/// key is not what the object keeps.
///
/// # Errors
///
/// A [`Fault::fatal`] on a receiver or a slot holding something else, which
/// compiled code cannot produce: both slots are written by
/// [`nvs_core_crypto_key_pair_read`] and by nothing else.
fn pair_of(args: &[Value], index: usize, member: &str) -> Result<PrivateKey, Fault> {
    let receiver = crate::instance::receiver(args[index], &KEY_PAIR, member)?;
    let slot_fault = |slot: usize, wanted: &str| {
        Fault::fatal(format!(
            "{KEY_PAIR_NAME}::{member} expected {wanted} in its `{}` slot",
            KEY_PAIR.slots[slot]
        ))
    };

    let held = crate::instance::slot(receiver, KEY_PAIR_PKCS8_SLOT);
    let der = held
        .as_bytes()
        .ok_or_else(|| slot_fault(KEY_PAIR_PKCS8_SLOT, "a `bytes`"))?;
    let kind = crate::instance::slot(receiver, KEY_PAIR_KIND_SLOT)
        .as_int()
        .and_then(KeyKind::from_tag)
        .ok_or_else(|| slot_fault(KEY_PAIR_KIND_SLOT, &format!("a `{KEY_KIND_NAME}` case")))?;

    // Unreachable from source: the slot holds the DER the read above stored,
    // after a parse that had already succeeded over the same kind.
    PrivateKey::read(der, kind).ok_or_else(|| slot_fault(KEY_PAIR_PKCS8_SLOT, "a private key"))
}

/// The encoded key and the kind a [`PUBLIC_KEY`] or [`KEY_PAIR`] receiver in
/// `args[index]` holds, without parsing either of them.
///
/// One function over both classes because they are one layout — the assertion
/// beside the slot constants is what holds that — and it is borrowed rather
/// than parsed because its caller is [`crate::jwe`], which **stores** the
/// octets in a `Jwe\Key` rather than using them: a key validated at
/// `Core\Crypto\PublicKey::read` is copied across as it stands, so nothing is
/// re-derived and nothing is less checked on the other side.
///
/// The [`Value`] is borrowed from the receiver, which the argument slot holds a
/// reference to for the length of the call, so the caller reads it in passing
/// and owes it nothing ([`crate::instance::slot`]).
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver or a kind slot of the wrong tag, neither of
/// which compiled code produces: the parameter's declared type is checked before
/// any of this runs, and the slots are this module's own.
pub(crate) fn stored_key(
    args: &[Value],
    index: usize,
    class: &'static CoreClass,
    member: &str,
) -> Result<(Value, KeyKind), Fault> {
    let receiver = crate::instance::receiver(args[index], class, member)?;
    let held = crate::instance::slot(receiver, PUBLIC_KEY_SPKI_SLOT);
    // Unreachable from source: both classes' readers write a `KEY_KIND`
    // constant into this slot and nothing else writes one at all.
    let kind = crate::instance::slot(receiver, PUBLIC_KEY_KIND_SLOT)
        .as_int()
        .and_then(KeyKind::from_tag)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{}::{member} expected a `{KEY_KIND_NAME}` case in its `{}` slot",
                class.name, class.slots[PUBLIC_KEY_KIND_SLOT]
            ))
        })?;
    Ok((held, kind))
}

/// The octets the [`stored_key`] value holds.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `bytes`, which [`stored_key`]'s
/// doc says no program reaches.
pub(crate) fn stored_octets<'a>(
    held: &'a Value,
    class: &'static CoreClass,
    member: &str,
) -> Result<&'a [u8], Fault> {
    // Unreachable from source: both classes' readers write a `Value::bytes`
    // into this slot, having parsed the key first.
    held.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{}::{member} expected a `bytes` in its `{}` slot, got tag {}",
            class.name,
            class.slots[PUBLIC_KEY_SPKI_SLOT],
            held.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Crypto\KeyPair::read(secret bytes $pkcs8, Crypto\KeyKind $kind): Crypto\KeyPair`
    /// — the one door onto [`PrivateKey::read`].
    ///
    /// The key is parsed to be refused, not to be kept: what the object holds is
    /// the DER, so a pair read from a PEM block and the same pair read from its
    /// DER are one value down to the slot. [`KEY_PAIR`]'s own doc owns the
    /// layout, what it spends, and the cache it turned down.
    fn nvs_core_crypto_key_pair_read(_ctx, args: [2]) {
        let stored = key_octets(args, 0, KEY_PAIR_NAME, "read")?;
        let kind = key_kind_of(args, 1, KEY_PAIR_NAME, "read")?;
        let der = pkcs8_der(stored).ok_or_else(pair_refused)?;
        PrivateKey::read(&der, kind).ok_or_else(pair_refused)?;
        nvs_runtime::affordable(Some(der.len()), "Core\\Crypto\\KeyPair::read")?;
        Ok(crate::instance::build(
            &KEY_PAIR,
            [Value::bytes(NvsStr::new(&der)), Value::int(kind.tag())],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$keyPair->write(): secret bytes` — this pair's PKCS#8, so a server keeps
    /// its key across requests.
    ///
    /// The one member of either key class that answers its slot rather than the
    /// key: a PKCS#8 is what was stored, and re-encoding one would mean writing
    /// RSA's private components, which this module reads and never writes.
    fn nvs_core_crypto_key_pair_write(_ctx, args: [1]) {
        let held = crate::instance::read_slot(args, &KEY_PAIR, KEY_PAIR_PKCS8_SLOT, "write")?;
        Ok(held)
    }
}

nvs_runtime::nvs_helper! {
    /// `$keyPair->publicKey(): Crypto\PublicKey` — the half of this pair that is
    /// sent.
    ///
    /// Derived from the private key rather than stored beside it, so a program
    /// cannot hold a pair whose public half is a different key: the two cannot
    /// disagree if only one of them is kept.
    fn nvs_core_crypto_key_pair_public_key(_ctx, args: [1]) {
        let pair = pair_of(args, 0, "publicKey")?;
        // Unreachable from source: every branch derives its point or its
        // components from material already parsed as a key of that kind, and
        // `ring` holds an RSA pair to the same width `PublicKey::rsa` does.
        let public = pair.public().ok_or_else(|| {
            Fault::fatal(format!(
                "{KEY_PAIR_NAME}::publicKey could not derive the public half of a key it had \
                 just read"
            ))
        })?;
        public_key_instance(&public, "Core\\Crypto\\KeyPair::publicKey")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::vectors as webcrypto;

    /// Every nonce, salt and private key the interop path draws comes through
    /// [`crate::random::draw`], which is the seam a second CSPRNG anywhere in
    /// this module would break while failing nothing else: fresh octets pass a
    /// round trip exactly as well as seeded ones.
    ///
    /// **Behavioural where the draw sits in a function a test can reach.** With
    /// a context's state armed as `#[Test(seed: …)]` arms it, two runs produce
    /// the same nonce and the same private key of every kind that is drawn at
    /// all, and a different seed produces different ones. Only a draw that goes
    /// through the seam does that — one taken from an entropy source directly
    /// answers different octets under either seed.
    ///
    /// **Structural for the draws that sit inside a member's own body**, here
    /// and in `Core\Jwe`, where the PBES2 salt and the content key are: the
    /// shipped half of both modules names no generator but the seam, so a draw
    /// added later to a member is caught as well. `ring`'s `SystemRandom` is
    /// named here and is the module doc's *signing is the one thing that draws
    /// outside* exception, so what is asserted of it is that no octet is ever
    /// taken **out** of it — `SecureRandom` is the trait that would have to be
    /// in scope to do that, and it is in neither module.
    #[test]
    fn every_interop_nonce_salt_and_private_key_is_drawn_through_core_random() {
        let drawn = |seed: u64| {
            let mut ctx = nvs_runtime::Ctx::buffered();
            ctx.set_random_state(seed);
            let cipher = XChaCha20Poly1305::new_from_slice(&[5_u8; KEY_LEN])
                .expect("the construction's own key length");
            let sealed = seal_under(&mut ctx, &cipher, &[], b"one message", "Core\\Crypto::seal")
                .expect("a short message seals");
            let keys: Vec<Vec<u8>> = [KeyKind::X25519, KeyKind::Ed25519, KeyKind::P256]
                .into_iter()
                .map(|kind| {
                    generated_pkcs8(&mut ctx, kind, "Core\\Crypto::generateKeyPair")
                        .expect("every kind but the two RSA ones is drawn here")
                })
                .collect();
            (sealed, keys)
        };

        assert_eq!(
            drawn(0x0005_eed1),
            drawn(0x0005_eed1),
            "a seeded context reproduces the nonce and every drawn key, which nothing outside \
             `crate::random::draw` can do"
        );
        assert_ne!(
            drawn(0x0005_eed1),
            drawn(0x0005_eed2),
            "and each of them is drawn rather than derived from the key or the message"
        );

        let shipped = |source: &'static str| {
            source
                .lines()
                .take_while(|line| line.trim_start() != "#[cfg(test)]")
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        for (module, source) in [
            ("crypto.rs", shipped(include_str!("crypto.rs"))),
            ("jwe.rs", shipped(include_str!("jwe.rs"))),
        ] {
            for generator in [
                "rand::rng",
                "OsRng",
                "thread_rng",
                "getrandom",
                "from_entropy",
                "SecureRandom",
            ] {
                assert!(
                    !source.contains(generator),
                    "`{module}` reaches `{generator}`; every nonce, salt and private key on this \
                     path is drawn through `crate::random::draw`"
                );
            }
            assert!(
                source.contains("crate::random::draw("),
                "`{module}` draws what it needs, through the seam"
            );
        }
    }

    /// A drawn key is [`KEY_LEN`] octets, a different one every call, and comes
    /// out of the context's generator like every other secret here.
    ///
    /// The member is driven through [`nvs_runtime::call`] rather than by
    /// calling [`crate::random::draw`] again, because the draw and the width
    /// *are* its whole body: a replay of them would assert this test's own
    /// code. The width is what makes a key a key — [`keyed_with`] is the only
    /// thing that reads it, and a member answering 31 octets would throw at
    /// every call site and nowhere here.
    ///
    /// Freshness is asserted over a batch rather than over one pair, since a
    /// generator stuck on one value still passes a single comparison whenever
    /// the value it is stuck on differs once. The seeded halves are the same
    /// claim as [`every_interop_nonce_salt_and_private_key_is_drawn_through_core_random`]
    /// makes of the nonces, over the member a program actually calls: one seed
    /// reproduces the keys and a second seed does not, which nothing drawing
    /// from an entropy source directly can do.
    // covers: Core\Crypto::generateKey
    #[test]
    fn a_generated_key_is_thirty_two_fresh_octets_drawn_through_core_random() {
        let keys = |seed: Option<u64>| {
            let mut ctx = nvs_runtime::Ctx::buffered();
            if let Some(state) = seed {
                ctx.set_random_state(state);
            }
            (0..16)
                .map(|_| {
                    nvs_runtime::call(nvs_core_crypto_generate_key, &mut ctx, &[])
                        .expect("the member takes no arguments and answers a key")
                        .as_bytes()
                        .expect("the row answers `secret bytes`, which is a `bytes` value")
                        .to_vec()
                })
                .collect::<Vec<_>>()
        };

        let drawn = keys(None);
        assert!(
            drawn.iter().all(|key| key.len() == KEY_LEN),
            "every key is {KEY_LEN} octets, which is the one width both ciphers take"
        );
        let mut distinct = drawn.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            drawn.len(),
            "no key is drawn twice, so no two records are sealed under one key by accident"
        );

        assert_eq!(
            keys(Some(0x0005_eed1)),
            keys(Some(0x0005_eed1)),
            "a seeded context reproduces the keys, which is what says they came through \
             `crate::random::draw`"
        );
        assert_ne!(
            keys(Some(0x0005_eed1)),
            keys(Some(0x0005_eed2)),
            "and a second seed draws different ones"
        );
    }

    /// A drawn pair is a private key of the kind the call named, a different one
    /// every call, and the two RSA kinds are refused.
    ///
    /// The member is driven through [`nvs_runtime::call`], with the kind as the
    /// case index a compiled call site passes, because what it answers is an
    /// object rather than a value: the slots are read with [`stored_key`], which
    /// is what every member taking a pair reads, and the DER is parsed with
    /// [`PrivateKey::read`], so a pair nothing could use again fails here rather
    /// than at the first signature.
    ///
    /// Freshness is asserted over a batch for
    /// [`a_generated_key_is_thirty_two_fresh_octets_drawn_through_core_random`]'s
    /// reason, and one seed reproducing the pairs while a second seed does not is
    /// what says the scalar came through [`crate::random::draw`] rather than out
    /// of an entropy source of this member's own.
    ///
    /// The refusal is read as its sentence, since the class a member threw is not
    /// on [`nvs_runtime::call`]'s error, and it has to name the member that does
    /// take an RSA key — a program holding one has a file to read, not a mistake
    /// to correct.
    // covers: Core\Crypto::generateKeyPair
    #[test]
    fn a_generated_pair_is_a_fresh_private_key_of_the_kind_that_was_named() {
        let drawn = |kind: KeyKind, seed: Option<u64>| {
            let mut ctx = nvs_runtime::Ctx::buffered();
            if let Some(state) = seed {
                ctx.set_random_state(state);
            }
            (0..8)
                .map(|_| {
                    let pair = nvs_runtime::call(
                        nvs_core_crypto_generate_key_pair,
                        &mut ctx,
                        &[Value::int(kind.tag())],
                    )
                    .expect("every kind but the two RSA ones is drawn here");
                    let (held, stored) = stored_key(&[pair], 0, &KEY_PAIR, "generateKeyPair")
                        .expect("the member fills both of the class's slots");
                    assert_eq!(stored, kind, "the pair is of the kind the call named");
                    let der = stored_octets(&held, &KEY_PAIR, "generateKeyPair")
                        .expect("the `pkcs8` slot holds the DER the member wrote")
                        .to_vec();
                    assert!(
                        PrivateKey::read(&der, kind).is_some(),
                        "the pair reads back as a private key, which is what every member \
                         taking one does with it"
                    );
                    #[expect(
                        unsafe_code,
                        reason = "the call answered this reference and nothing else holds it, \
                                  which is `Value::release`'s whole obligation"
                    )]
                    unsafe {
                        pair.release();
                    }
                    der
                })
                .collect::<Vec<_>>()
        };

        for kind in [KeyKind::P256, KeyKind::X25519, KeyKind::Ed25519] {
            let pairs = drawn(kind, None);
            let mut distinct = pairs.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(
                distinct.len(),
                pairs.len(),
                "no pair is drawn twice, so no two programs hold one private key by accident"
            );
            assert_eq!(
                drawn(kind, Some(0x000c_0ffe)),
                drawn(kind, Some(0x000c_0ffe)),
                "a seeded context reproduces the pairs, which is what says the scalar came \
                 through `crate::random::draw`"
            );
            assert_ne!(
                drawn(kind, Some(0x000c_0ffe)),
                drawn(kind, Some(0x000c_0fff)),
                "and a second seed draws different ones"
            );
        }

        let mut ctx = nvs_runtime::Ctx::buffered();
        for kind in [KeyKind::RsaPkcs1, KeyKind::RsaPss] {
            assert!(
                nvs_runtime::call(
                    nvs_core_crypto_generate_key_pair,
                    &mut ctx,
                    &[Value::int(kind.tag())],
                )
                .is_err(),
                "an RSA key is never drawn here"
            );
            let refusal = ctx
                .take_pending()
                .expect("a member that did not answer left its sentence")
                .to_string();
            assert!(
                refusal.contains("Core\\Crypto\\KeyPair::read"),
                "the refusal names the member that does take an RSA key, got {refusal:?}"
            );
        }
    }

    /// A signature is the pair's own scheme over the whole message, and an
    /// X25519 pair signs nothing.
    ///
    /// Driven through [`nvs_runtime::call`] from a generated pair, because the
    /// scheme is [`PrivateKey::signing`]'s answer rather than anything on the
    /// call: what this sees and a test of [`sign`] alone cannot is that the pair
    /// a program holds is the whole choice. Every signature is then checked
    /// through [`nvs_core_crypto_verify`] over the same key, since a member
    /// answering 64 plausible octets passes every length assertion ever written
    /// over it, and one octet of the message is changed so that a signature
    /// covering nothing in particular fails here.
    ///
    /// Ed25519 signing one message the same way twice while P-256 signs it
    /// differently is the reference card's own sentence, and it is the half a
    /// caller feels: two signatures that differ are both this key's.
    // covers: Core\Crypto::sign
    #[test]
    fn a_signature_is_the_pairs_own_scheme_and_an_x25519_pair_signs_nothing() {
        fn drew(ctx: &mut nvs_runtime::Ctx, kind: KeyKind) -> Value {
            nvs_runtime::call(
                nvs_core_crypto_generate_key_pair,
                ctx,
                &[Value::int(kind.tag())],
            )
            .expect("every kind but the two RSA ones is drawn here")
        }

        fn signed(
            ctx: &mut nvs_runtime::Ctx,
            message: &[u8],
            pair: Value,
        ) -> Result<Vec<u8>, String> {
            let args = [Value::bytes(NvsStr::new(message)), pair];
            match nvs_runtime::call(nvs_core_crypto_sign, ctx, &args) {
                Ok(answer) => Ok(answer.as_bytes().expect("the row answers `bytes`").to_vec()),
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a member that did not answer left its sentence")
                    .to_string()),
            }
        }

        fn held(ctx: &mut nvs_runtime::Ctx, message: &[u8], signature: &[u8], key: Value) -> bool {
            let args = [
                Value::bytes(NvsStr::new(message)),
                Value::bytes(NvsStr::new(signature)),
                key,
            ];
            let verdict = nvs_runtime::call(nvs_core_crypto_verify, ctx, &args).is_ok();
            if !verdict {
                // The sentence is this test's to clear rather than to read: the
                // refusals are `every_forgery_lands_on_one_sentence_and_an_x25519_key_verifies_nothing`'s
                // subject, and a pending one left here is the next call's.
                let _ = ctx.take_pending();
            }
            verdict
        }

        #[expect(
            unsafe_code,
            reason = "the call answered this reference and nothing else holds it, which is \
                      `Value::release`'s whole obligation"
        )]
        fn dropped(value: Value) {
            unsafe {
                value.release();
            }
        }

        let mut ctx = nvs_runtime::Ctx::buffered();
        let message = b"transfer 100 to account 7";

        for kind in [KeyKind::Ed25519, KeyKind::P256] {
            let pair = drew(&mut ctx, kind);
            let first = signed(&mut ctx, message, pair).expect("both of these kinds sign");
            let second = signed(&mut ctx, message, pair).expect("both of these kinds sign");
            assert_eq!(
                first.len(),
                64,
                "{kind:?} signs into the 64 octets JWS and WebCrypto read, never DER"
            );

            let public = nvs_runtime::call(nvs_core_crypto_key_pair_public_key, &mut ctx, &[pair])
                .expect("every pair answers the half that is sent");
            assert!(
                held(&mut ctx, message, &first, public) && held(&mut ctx, message, &second, public),
                "{kind:?} answers a signature its own public key checks out"
            );
            assert!(
                !held(&mut ctx, b"transfer 100 to account 8", &first, public),
                "{kind:?} signs these octets and no others"
            );

            if kind == KeyKind::Ed25519 {
                assert_eq!(
                    first, second,
                    "Ed25519 signs one message the same way every time"
                );
            } else {
                assert_ne!(
                    first, second,
                    "P-256 draws randomness, so two signatures over one message differ"
                );
            }
            dropped(public);
            dropped(pair);
        }

        let agreeing = drew(&mut ctx, KeyKind::X25519);
        let refusal =
            signed(&mut ctx, message, agreeing).expect_err("an X25519 pair signs nothing");
        assert!(
            refusal.contains("X25519"),
            "the refusal names the kind the program chose, got {refusal:?}"
        );
        dropped(agreeing);
    }

    /// Every forged signature lands on one sentence, and an X25519 key verifies
    /// nothing.
    ///
    /// The forgeries are the shapes a sender has: one octet changed at every
    /// position, the signature cut to every shorter length, a real signature made
    /// under another key, and this key's own signature over other octets.
    /// Throwing is not the whole assertion — what is pinned is that all of them
    /// land on **one** refusal, since a sentence naming which check failed tells
    /// a forger where to try next, and that is the leak
    /// `rule:security/verification-throws-and-compares-in-constant-time` exists
    /// to prevent.
    ///
    /// The X25519 key is what keeps the two refusals apart: it is the program's
    /// own key rather than anything that arrived, so it gets its own sentence and
    /// not the one every forgery gets. Both are read as sentences, since the
    /// class a member threw is not on [`nvs_runtime::call`]'s error.
    // covers: Core\Crypto::verify
    #[test]
    fn every_forgery_lands_on_one_sentence_and_an_x25519_key_verifies_nothing() {
        fn drew(ctx: &mut nvs_runtime::Ctx, kind: KeyKind) -> Value {
            nvs_runtime::call(
                nvs_core_crypto_generate_key_pair,
                ctx,
                &[Value::int(kind.tag())],
            )
            .expect("every kind but the two RSA ones is drawn here")
        }

        fn public_of(ctx: &mut nvs_runtime::Ctx, pair: Value) -> Value {
            nvs_runtime::call(nvs_core_crypto_key_pair_public_key, ctx, &[pair])
                .expect("every pair answers the half that is sent")
        }

        fn signed(ctx: &mut nvs_runtime::Ctx, message: &[u8], pair: Value) -> Vec<u8> {
            let args = [Value::bytes(NvsStr::new(message)), pair];
            nvs_runtime::call(nvs_core_crypto_sign, ctx, &args)
                .expect("this kind signs")
                .as_bytes()
                .expect("the row answers `bytes`")
                .to_vec()
        }

        /// The sentence the check was refused with, or `None` for one that held.
        fn refusal(
            ctx: &mut nvs_runtime::Ctx,
            message: &[u8],
            signature: &[u8],
            key: Value,
        ) -> Option<String> {
            let args = [
                Value::bytes(NvsStr::new(message)),
                Value::bytes(NvsStr::new(signature)),
                key,
            ];
            nvs_runtime::call(nvs_core_crypto_verify, ctx, &args).err()?;
            Some(
                ctx.take_pending()
                    .expect("a member that did not answer left its sentence")
                    .to_string(),
            )
        }

        #[expect(
            unsafe_code,
            reason = "the call answered this reference and nothing else holds it, which is \
                      `Value::release`'s whole obligation"
        )]
        fn dropped(value: Value) {
            unsafe {
                value.release();
            }
        }

        let mut ctx = nvs_runtime::Ctx::buffered();
        let message = b"transfer 100 to account 7";
        let pair = drew(&mut ctx, KeyKind::Ed25519);
        let key = public_of(&mut ctx, pair);
        let signature = signed(&mut ctx, message, pair);
        assert!(
            refusal(&mut ctx, message, &signature, key).is_none(),
            "the signature this key made over these octets checks out"
        );

        let mut sentences = std::collections::BTreeSet::new();
        for at in 0..signature.len() {
            let mut forged = signature.clone();
            forged[at] ^= 1;
            sentences.insert(
                refusal(&mut ctx, message, &forged, key)
                    .expect("a signature with an octet changed is not this key's"),
            );
        }
        for length in 0..signature.len() {
            sentences.insert(
                refusal(&mut ctx, message, &signature[..length], key)
                    .expect("a signature cut short is not a signature"),
            );
        }
        let stranger = drew(&mut ctx, KeyKind::P256);
        let theirs = signed(&mut ctx, message, stranger);
        sentences.insert(
            refusal(&mut ctx, message, &theirs, key)
                .expect("another key's signature over these octets is not this key's"),
        );
        sentences.insert(
            refusal(&mut ctx, b"transfer 100 to account 8", &signature, key)
                .expect("this key's signature covers the octets it was made over"),
        );
        assert_eq!(
            sentences.len(),
            1,
            "every forgery lands on one sentence, so the refusal says nothing about which \
             check failed: {sentences:?}"
        );

        let agreeing_pair = drew(&mut ctx, KeyKind::X25519);
        let agreeing = public_of(&mut ctx, agreeing_pair);
        let verdict = refusal(&mut ctx, message, &signature, agreeing)
            .expect("an X25519 key verifies nothing");
        assert!(
            verdict.contains("X25519"),
            "the refusal names the kind the program chose, got {verdict:?}"
        );
        assert!(
            !sentences.contains(&verdict),
            "a key that verifies nothing is the program's own mistake and gets its own sentence"
        );

        for value in [agreeing, agreeing_pair, stranger, key, pair] {
            dropped(value);
        }
    }

    /// `Core\Crypto::seal` answers a nonce, the body and a tag, with a fresh
    /// nonce every call, and throws on a key that is not [`KEY_LEN`] octets.
    ///
    /// Driven through [`nvs_runtime::call`], with the cipher as the case index
    /// a compiled call site passes, because the widths are this member's own
    /// arithmetic: [`NONCE_LEN`] plus [`TAG_LEN`] for the extended
    /// construction and [`GCM_NONCE_LEN`] plus [`TAG_LEN`] for the interop one.
    /// A member that drew the other cipher's nonce width answers a plausible
    /// buffer rather than failing, and only the length says so.
    ///
    /// The plaintext is searched for in the answer rather than assumed absent:
    /// a mode that forgot to encrypt would pass every length and freshness
    /// assertion above it. The refusal is read as its sentence, since the class
    /// a member threw is not on [`nvs_runtime::call`]'s error.
    // covers: Core\Crypto::seal
    #[test]
    fn seal_answers_a_fresh_nonce_a_body_and_a_tag_and_refuses_a_key_of_another_width() {
        let message = b"the vault code is 4711, and nothing else";
        let sealed = |cipher: i64, key: &[u8]| -> Result<Vec<u8>, String> {
            let mut ctx = nvs_runtime::Ctx::buffered();
            let args = [
                Value::bytes(NvsStr::new(message)),
                Value::bytes(NvsStr::new(key)),
                Value::int(cipher),
            ];
            match nvs_runtime::call(nvs_core_crypto_seal, &mut ctx, &args) {
                Ok(answer) => Ok(answer.as_bytes().expect("the row answers `bytes`").to_vec()),
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a member that did not answer left its sentence")
                    .to_string()),
            }
        };

        let key = [9_u8; KEY_LEN];
        for (cipher, overhead) in [(0, NONCE_LEN + TAG_LEN), (1, GCM_NONCE_LEN + TAG_LEN)] {
            let one = sealed(cipher, &key).expect("a 32-octet key seals under either case");
            let two = sealed(cipher, &key).expect("and does so again");
            assert_eq!(
                one.len(),
                message.len() + overhead,
                "case {cipher} answers the nonce, the body and the tag and nothing else"
            );
            assert_ne!(
                one, two,
                "case {cipher} draws its own nonce, so one message sealed twice is two answers"
            );
            let nonce = overhead - TAG_LEN;
            assert_ne!(
                one[..nonce],
                two[..nonce],
                "and the nonce is the part that moved"
            );
            assert!(
                !one.windows(message.len()).any(|slice| slice == message),
                "case {cipher} answers no octet run of the plaintext"
            );
        }

        for width in [0, 1, KEY_LEN - 1, KEY_LEN + 1] {
            let refusal = sealed(0, &vec![b'A'; width]).expect_err("a key of another width");
            assert!(
                refusal.contains("$key is") && refusal.contains("a key is 32"),
                "the sentence names the width it got and the one width a key has, got {refusal:?}"
            );
            assert!(
                !refusal.contains("AAAA"),
                "and carries no octet of the key itself, since a key does not belong in a log"
            );
        }
    }

    /// `Core\Crypto::open` answers the plaintext, and every way a message can
    /// fail to be an authentic one lands on a single sentence.
    ///
    /// The forgeries are the shapes an attacker has: one octet changed at every
    /// position, the buffer cut to every shorter length, the other cipher named
    /// over the same bytes, and another key. Throwing is not the whole
    /// assertion — a member that answered a plausible plaintext for a truncated
    /// buffer would throw nothing, and one that named *which* check failed
    /// would throw four sentences. That both halves hold is this member's own
    /// doc written as a test, since telling a forger which half landed is the
    /// leak the one sentence exists to prevent.
    // covers: Core\Crypto::open
    #[test]
    fn open_answers_the_plaintext_and_every_forgery_lands_on_one_sentence() {
        let message = b"pay 100 to Ada";
        let key = [9_u8; KEY_LEN];
        let called = |member: nvs_runtime::NvsFn,
                      subject: &[u8],
                      key: &[u8],
                      cipher: i64|
         -> Result<Vec<u8>, String> {
            let mut ctx = nvs_runtime::Ctx::buffered();
            let args = [
                Value::bytes(NvsStr::new(subject)),
                Value::bytes(NvsStr::new(key)),
                Value::int(cipher),
            ];
            match nvs_runtime::call(member, &mut ctx, &args) {
                Ok(answer) => Ok(answer.as_bytes().expect("the row answers `bytes`").to_vec()),
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a member that did not answer left its sentence")
                    .to_string()),
            }
        };

        for cipher in [0, 1] {
            let sealed = called(nvs_core_crypto_seal, message, &key, cipher)
                .expect("a 32-octet key seals under either case");
            assert_eq!(
                called(nvs_core_crypto_open, &sealed, &key, cipher)
                    .expect("what was sealed opens again"),
                message,
                "case {cipher} answers the plaintext byte for byte"
            );

            let mut sentences = std::collections::BTreeSet::new();
            for at in 0..sealed.len() {
                let mut forged = sealed.clone();
                forged[at] ^= 1;
                sentences.insert(
                    called(nvs_core_crypto_open, &forged, &key, cipher)
                        .expect_err("an octet changed at any position, the tag included"),
                );
            }
            for cut in 0..sealed.len() {
                sentences.insert(
                    called(nvs_core_crypto_open, &sealed[..cut], &key, cipher)
                        .expect_err("a message cut short, the empty one included"),
                );
            }
            sentences.insert(
                called(nvs_core_crypto_open, &sealed, &key, 1 - cipher)
                    .expect_err("the cipher the message was not sealed under"),
            );
            let mut another = key;
            another[0] ^= 1;
            sentences.insert(
                called(nvs_core_crypto_open, &sealed, &another, cipher)
                    .expect_err("a key of the right width that sealed nothing"),
            );

            assert_eq!(
                sentences.len(),
                1,
                "case {cipher} answers one sentence for every way a message is not authentic, \
                 got {sentences:?}"
            );
        }
    }

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

    /// Every AES-GCM message the frozen set records seals to exactly the octets
    /// WebCrypto produced and opens back to its own plaintext.
    ///
    /// The nonce is the recorded one rather than a draw, which is what
    /// [`gcm_seal_under`] takes it as an argument for: a construction that drew
    /// its own could only be held to a set that had agreed on the draw. So the
    /// layout the set's `about` states — `nonce(12) ‖ ciphertext ‖ tag(16)` — is
    /// asserted whole, as one comparison against the file's octets, rather than
    /// as three lengths that happen to add up.
    ///
    /// The forgeries the set records beside these messages are the primitive
    /// refusals test's, with the rest of the roster's.
    #[test]
    fn webcrypto_aes_gcm_vectors_open_and_reseal_byte_for_byte() {
        let sealing = "Core\\Crypto::seal";
        let opening = "Core\\Crypto::open";
        let vectors = webcrypto::vectors("aesGcm");
        assert!(
            !vectors.is_empty(),
            "the set carries AES-GCM vectors at all"
        );

        for vector in vectors {
            let name = webcrypto::text(vector, "/name");
            let cipher = gcm_cipher(&webcrypto::octets(vector, "/key"))
                .expect("the set keys AES-256 with 32 octets");
            let nonce =
                <[u8; GCM_NONCE_LEN]>::try_from(webcrypto::octets(vector, "/nonce").as_slice())
                    .expect("the set writes a 12-octet nonce");
            let message = webcrypto::octets(vector, "/plaintext");
            let sealed = webcrypto::octets(vector, "/sealed");

            assert_eq!(
                gcm_seal_under(&cipher, &nonce, &message, sealing).expect("the vector seals"),
                sealed,
                "{name} seals to the octets WebCrypto answered with"
            );
            assert_eq!(
                gcm_open_under(&cipher, &sealed, opening)
                    .expect("the plaintext is affordable")
                    .as_deref(),
                Some(message.as_slice()),
                "{name} opens back to its own plaintext"
            );
        }
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
            derive_key(b"correct horse", &salt, u64::from(MIN_ITERATIONS) - 1, who).is_err(),
            "one under the floor is refused"
        );
        assert!(
            derive_key(b"correct horse", &salt, u64::from(MAX_ITERATIONS) + 1, who).is_err(),
            "one over the ceiling is refused, and refused without running"
        );
        assert!(
            derive_key(
                b"correct horse",
                &salt[..MIN_SALT_LEN - 1],
                u64::from(MIN_ITERATIONS),
                who
            )
            .is_err(),
            "one octet short of a salt is refused"
        );
        assert_eq!(
            derive_key(b"correct horse", &salt, u64::from(MIN_ITERATIONS), who)
                .expect("the floor derives"),
            pbkdf2_sha256(b"correct horse", &salt, MIN_ITERATIONS),
            "inside the bounds it is the derivation above and nothing else"
        );
    }

    /// A derivation case's iteration count, in the width [`derive_key`] takes
    /// one in.
    fn rounds(case: &serde_json::Value) -> u64 {
        webcrypto::number(case, "/iterations")
    }

    /// Every key the frozen set's three derivations record, derived again from
    /// the input beside it: the two `Core\Crypto` members and the key wrap that
    /// sits under `PBES2`.
    ///
    /// Three sections in one test because they are one question asked of one
    /// file — whether this runtime turns the recorded input into the recorded
    /// key — and each section keeps its own empty-list assertion, so a section
    /// that goes missing from the set is caught rather than skipped.
    ///
    /// PBKDF2 is replayed through [`derive_key`] rather than through
    /// [`pbkdf2_sha256`], because that is the path a program takes and it is
    /// where the bounds sit; the refusals at those bounds are the test below. A
    /// password is UTF-8 octets, which is what a browser's `TextEncoder` hands
    /// `importKey`, and the set's second vector is not ASCII — so the encoding
    /// is pinned here rather than assumed, as HKDF's `info` context string is.
    /// The wrap is the one section with no member of its own: it is internal to
    /// `PBES2-HS256+A128KW` (`rule:security/jwe-compact-subset`), and the set is
    /// the only evidence for the 128-bit key-encryption key, which RFC 3394
    /// publishes no case for.
    // covers: Core\Crypto::deriveKey, Core\Crypto::expandKey
    #[test]
    fn webcrypto_pbkdf2_hkdf_and_aes_kw_vectors_derive_the_same_keys() {
        let who = "Core\\Crypto::deriveKey";
        let vectors = webcrypto::vectors("pbkdf2");
        assert!(!vectors.is_empty(), "the set carries PBKDF2 vectors at all");

        for vector in vectors {
            let name = webcrypto::text(vector, "/name");
            assert_eq!(
                derive_key(
                    webcrypto::text(vector, "/password").as_bytes(),
                    &webcrypto::octets(vector, "/salt"),
                    rounds(vector),
                    who,
                )
                .expect("a recorded derivation is inside the bounds")
                .as_slice(),
                webcrypto::octets(vector, "/key"),
                "{name} derives the key WebCrypto answered with"
            );
        }

        let vectors = webcrypto::vectors("hkdf");
        assert!(!vectors.is_empty(), "the set carries HKDF vectors at all");

        for vector in vectors {
            let name = webcrypto::text(vector, "/name");
            assert_eq!(
                expand_key(
                    &webcrypto::octets(vector, "/material"),
                    &webcrypto::octets(vector, "/salt"),
                    webcrypto::text(vector, "/info").as_bytes(),
                )
                .as_slice(),
                webcrypto::octets(vector, "/key"),
                "{name} expands to the key WebCrypto answered with"
            );
        }

        let vectors = webcrypto::vectors("aesKw");
        assert!(!vectors.is_empty(), "the set carries AES-KW vectors at all");

        for vector in vectors {
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
    }

    /// Every refusal the frozen set records against a primitive, refused by the
    /// member that owns the rule it breaks.
    ///
    /// An AES-GCM forgery — a tag flipped, a ciphertext flipped, a nonce
    /// flipped, a message too short to hold either, the right bytes under
    /// another key — is no plaintext at `Core\Crypto::open`, which is where the
    /// one forgery sentence
    /// `rule:security/verification-throws-and-compares-in-constant-time` asks
    /// for is said. A PBKDF2 count or salt outside the bounds is refused by
    /// `Core\Crypto::deriveKey` before the first HMAC, each case sitting one
    /// step outside one bound, so the file is what stops either number moving.
    ///
    /// A point is refused on each curve at the place the module doc's *two
    /// curves agree* section puts it, which is the reason one loop asks two
    /// members: P-256's off-curve point, point at infinity and short encoding
    /// are refused by `Crypto\PublicKey::read` before anything multiplies by
    /// them, while an X25519 small-order point is a well-formed encoding and is
    /// refused by `Crypto::agree` at the all-zero secret it produces. A refusal
    /// is a property of the point rather than of the scalar meeting it, so each
    /// case reuses the set's own private key for that curve rather than carrying
    /// one of its own.
    #[test]
    fn webcrypto_primitive_refusal_vectors_are_each_refused_by_the_member_that_owns_the_rule() {
        let refusals = webcrypto::refusals("aesGcm");
        assert!(!refusals.is_empty(), "the set records forgeries at all");

        for refusal in refusals {
            let name = webcrypto::text(refusal, "/name");
            let cipher = gcm_cipher(&webcrypto::octets(refusal, "/key"))
                .expect("the set keys AES-256 with 32 octets");
            assert!(
                gcm_open_under(
                    &cipher,
                    &webcrypto::octets(refusal, "/sealed"),
                    "Core\\Crypto::open",
                )
                .expect("a forgery costs the same buffer as a message")
                .is_none(),
                "{name} is not authentic under the key it is offered to"
            );
        }

        let refusals = webcrypto::refusals("pbkdf2");
        assert!(
            !refusals.is_empty(),
            "and the bounds it records at the edge"
        );

        for refusal in refusals {
            let name = webcrypto::text(refusal, "/name");
            assert!(
                derive_key(
                    webcrypto::text(refusal, "/password").as_bytes(),
                    &webcrypto::octets(refusal, "/salt"),
                    rounds(refusal),
                    "Core\\Crypto::deriveKey",
                )
                .is_err(),
                "{name} is outside a bound this derivation enforces"
            );
        }

        let refusals = webcrypto::refusals("ecdh");
        assert!(
            !refusals.is_empty(),
            "and the points it refused to agree on"
        );

        for refusal in refusals {
            let name = webcrypto::text(refusal, "/name");
            let curve = webcrypto::text(refusal, "/curve");
            let kind = kind_of(curve);
            let theirs = webcrypto::octets(refusal, "/theirs");
            let read = PublicKey::read(&theirs, kind, KeyFormat::Raw);

            if kind == KeyKind::P256 {
                assert!(read.is_err(), "{name}");
                continue;
            }

            let theirs = read.expect("a small-order point is a well-formed X25519 encoding");
            let holder = webcrypto::vectors("ecdh")
                .iter()
                .find(|vector| webcrypto::text(vector, "/curve") == curve)
                .expect("the set agrees over the curve it refuses points on");
            let mine = PrivateKey::read(&webcrypto::octets(holder, "/a/pkcs8"), kind)
                .expect("the set writes every private key as DER PKCS#8");
            assert!(agree(&mine, &theirs).is_err(), "{name}");
        }
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
    ///
    /// Then every form a browser's `exportKey` writes the peer's key in, through
    /// [`agree`] itself: `raw`, `spki` and `jwk` are three readers of one key,
    /// and a secret that comes out the same from all three is the evidence they
    /// built the same point rather than three plausible ones.
    // covers: Core\Crypto::agree
    #[test]
    fn webcrypto_ecdh_vectors_agree_the_same_secret_from_every_key_form() {
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

            // The same secret again out of the member's own seam, with the
            // peer's key arriving in each form a browser exports: `read` is
            // where a coordinate is dropped or a DER wrapper kept, and a key
            // that read almost right still agrees on *something* until three
            // encodings of it are made to agree on one secret.
            let kind = kind_of(webcrypto::text(vector, "/curve"));
            for (holder, sender) in [("/a", "/b"), ("/b", "/a")] {
                let mine =
                    PrivateKey::read(&webcrypto::octets(vector, &format!("{holder}/pkcs8")), kind)
                        .expect("the set writes every private key as DER PKCS#8");
                let party = vector
                    .pointer(sender)
                    .expect("the set nests a key pair under each party");

                for (form, encoded) in [
                    (KeyFormat::Raw, webcrypto::octets(party, "/raw")),
                    (KeyFormat::Spki, webcrypto::octets(party, "/spki")),
                    (
                        KeyFormat::Jwk,
                        webcrypto::text(party, "/jwk").as_bytes().to_vec(),
                    ),
                ] {
                    let theirs = PublicKey::read(&encoded, kind, form)
                        .unwrap_or_else(|_| panic!("{name}, whose peer key a browser exported"));
                    assert_eq!(
                        agree(&mine, &theirs)
                            .expect("the set's two parties are two ends of one agreement")
                            .as_slice(),
                        secret,
                        "{name}, from the peer's key read as {form:?}"
                    );
                }
            }
        }
    }

    /// The same exchange over the other curve against RFC 5903 § 8.1, which is
    /// P-256's own published evidence for it. The frozen set cannot supply this:
    /// it says what one implementation computes, and an agreement both ends of
    /// which are a document's literals says what the exchange *is*. The X25519
    /// half of the test above is RFC 7748 § 6.1 for the same reason.
    ///
    /// Each peer's point is multiplied up from the document's other scalar
    /// rather than copied out of its coordinate pair, because the point a scalar
    /// reaches is the point the exchange uses and a copied coordinate is a
    /// second literal that can be wrong on its own. Both directions are
    /// asserted, for the reason the test above gives.
    #[test]
    fn the_p256_agreement_matches_rfc_5903_from_either_end() {
        let initiator = hex("c88f01f510d9ac3f70a292daa2316de544e9aab8afe84049c62a9c57862d1433");
        let responder = hex("c6ef9c5d78ae012a011164acb397ce2088685d8f06bf9be0b283ab46476bee53");
        let published = hex("d6840f6b42f6edafd13116e0e12565202fef8e9ece7dce03812464d04b9442de");
        let point = |scalar: &[u8]| {
            P256SecretKey::from_slice(scalar)
                .expect("the document's key is a scalar of the curve it publishes")
                .public_key()
        };

        assert_eq!(
            agree_p256(&initiator, &point(&responder))
                .expect("the document's two keys are two ends of one agreement")
                .as_slice(),
            published,
            "RFC 5903 § 8.1, the initiator's scalar against the responder's point"
        );
        assert_eq!(
            agree_p256(&responder, &point(&initiator))
                .expect("the document's two keys are two ends of one agreement")
                .as_slice(),
            published,
            "RFC 5903 § 8.1, the same secret reached from the other end"
        );
    }

    /// The key wrap against RFC 3394 § 4.6 — the published case whose key data
    /// is the width every content key here is — and the refusals beside it,
    /// because an unwrap is the half that meets a token: a wrap with an octet
    /// changed, one a semiblock short and one under a key of no width this wraps
    /// under are each `None`, which is the one answer the module doc says they
    /// share.
    ///
    /// The frozen set's own wraps, which are the only evidence for the 128-bit
    /// key-encryption key `PBES2-HS256+A128KW` uses, are the derivation replay
    /// above.
    #[test]
    fn the_key_wrap_matches_rfc_3394_and_refuses_a_changed_wrap() {
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
    /// `bun nv webcrypto-vectors` reproduces this same output before it
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

    /// The public half of one of the set's JWS keys, as the [`VerifyingKey`] its
    /// own minimal JWK assembles into, handed to `body` because that key borrows
    /// the coordinates it was built out of.
    ///
    /// The variant comes from the key's recorded `alg` and never from the case
    /// under test, which is the direction
    /// `rule:security/algorithm-comes-from-the-key` makes a member run in: a
    /// refusal whose whole point is a signature written under another algorithm
    /// has to meet the key's own binding rather than its own claim.
    fn verifying<R>(id: &str, body: impl FnOnce(VerifyingKey<'_>) -> R) -> R {
        let key = webcrypto::node(&format!("/jws/keys/{id}"));
        let jwk: serde_json::Value = serde_json::from_str(webcrypto::text(key, "/jwkMinimal"))
            .expect("the set writes a minimal JWK as JSON text");
        let member = |name: &str| {
            URL_SAFE_NO_PAD
                .decode(jwk[name].as_str().expect("the JWK carries this member"))
                .expect("a JWK member is base64url")
        };

        match webcrypto::text(key, "/alg") {
            "RS256" => body(VerifyingKey::RsaPkcs1 {
                modulus: &member("n"),
                exponent: &member("e"),
            }),
            "PS256" => body(VerifyingKey::RsaPss {
                modulus: &member("n"),
                exponent: &member("e"),
            }),
            "ES256" => {
                let mut point = vec![4_u8];
                point.extend_from_slice(&member("x"));
                point.extend_from_slice(&member("y"));
                body(VerifyingKey::P256 { point: &point })
            }
            "EdDSA" => body(VerifyingKey::Ed25519 {
                point: &member("x"),
            }),
            alg => panic!("the set carries {alg}, which this module does not verify"),
        }
    }

    /// The private half of one of the set's JWS keys, read as the algorithm its
    /// own `alg` names.
    ///
    /// The same direction [`verifying`] reads the public half in, so no case can
    /// read one pair under two algorithms and call the disagreement a finding.
    fn signing(id: &str) -> SigningKey {
        let key = webcrypto::node(&format!("/jws/keys/{id}"));
        let kind = match webcrypto::text(key, "/alg") {
            "RS256" => KeyKind::RsaPkcs1,
            "PS256" => KeyKind::RsaPss,
            "ES256" => KeyKind::P256,
            "EdDSA" => KeyKind::Ed25519,
            alg => panic!("the set carries {alg}, which this module does not sign with"),
        };

        PrivateKey::read(&webcrypto::octets(key, "/pkcs8"), kind)
            .expect("the set writes every private key as DER PKCS#8")
            .signing()
            .expect("every key the set signs with is a kind that signs")
    }

    /// Every signature the set records a refusal for, refused: a flipped octet,
    /// another key's signature, a message edited after signing, a signature in a
    /// form the algorithm does not write, and one made under the algorithm the
    /// key is not bound to, which is the confusion
    /// `rule:security/algorithm-comes-from-the-key` exists to close.
    ///
    /// A refusal answers `None` here because this seam has no member to speak
    /// for: the one sentence
    /// `rule:security/verification-throws-and-compares-in-constant-time` asks
    /// for belongs to `Core\Crypto::verify`, which is what turns this `None`
    /// into it. What the set's refusals pin is that every way of not being the
    /// signature that key made over that message — the octets, the key, the
    /// message, the encoding and the algorithm — arrives there as the same
    /// answer, so there is one sentence to say.
    #[test]
    fn webcrypto_signature_refusals_are_each_refused_with_one_runtime_error() {
        for refusal in webcrypto::refusals("signatures") {
            let name = webcrypto::text(refusal, "/name");
            let message = webcrypto::octets(refusal, "/message");
            let signature = webcrypto::octets(refusal, "/signature");
            verifying(webcrypto::text(refusal, "/key"), |key| {
                assert!(
                    verify_signature(&key, &message, &signature).is_none(),
                    "{name}"
                );
            });
        }
    }

    /// Every signature the frozen WebCrypto set holds, in both directions: read
    /// from the public material a JWK carries, and written again where the
    /// algorithm writes one message's signature the same way twice.
    ///
    /// A case names its key rather than carrying one, so what is pinned is the
    /// pairing another implementation actually produced: the same key, the same
    /// message, the same octets.
    ///
    /// The two directions are one test because the set's `deterministic` flag is
    /// what divides them, and that is a property of the algorithm rather than of
    /// a case. `RS256` and `EdDSA` write the same octets every time, so those
    /// are held to the set itself: a token signed here under an issuer's key is
    /// byte for byte what WebCrypto would have written, which is what makes a
    /// signed token reproducible at all. The other two salt and nonce their
    /// signatures, so what is asserted there is what survives randomness — a
    /// fresh signature verifies under the key's own public half, and two over
    /// one message differ, which is the evidence the randomness reaches the
    /// algorithm rather than being a constant nobody noticed.
    #[test]
    fn webcrypto_signature_vectors_verify_and_deterministic_ones_resign_byte_for_byte() {
        for vector in webcrypto::vectors("signatures") {
            let name = webcrypto::text(vector, "/name");
            let id = webcrypto::text(vector, "/key");
            let message = webcrypto::octets(vector, "/message");
            let published = webcrypto::octets(vector, "/signature");
            verifying(id, |key| {
                assert_eq!(
                    verify_signature(&key, &message, &published),
                    Some(()),
                    "{name}"
                );
            });

            let written = sign(&signing(id), &message).expect("a key of the set signs");
            if vector["deterministic"]
                .as_bool()
                .expect("the set flags every signature one way or the other")
            {
                assert_eq!(written, published, "{name}");
            } else {
                let again = sign(&signing(id), &message).expect("a key of the set signs");
                assert_ne!(written, again, "{name}, twice over one message");
            }

            verifying(id, |key| {
                assert!(
                    verify_signature(&key, &message, &written).is_some(),
                    "{name}, verified against the key that wrote it"
                );
            });
        }
    }

    /// RS256 and ES256 against the tokens RFC 7515 publishes in its Appendices
    /// A.2 and A.3, each verified under the key that appendix prints beside it.
    ///
    /// The frozen set is one implementation's signatures over keys it generated
    /// itself, so a reader agreeing with that generator about an encoding they
    /// both got wrong still passes it. A document's own token under the
    /// document's own key is the evidence that cannot be produced that way.
    ///
    /// `Crypto\PublicKey::read` takes a JWK, so each appendix's JSON is the
    /// input with nothing assembled around it — minus the private members it
    /// prints beside `n` and `x`, because a JWK carrying `d` is refused on the
    /// way in and that refusal is one of this module's rules rather than an
    /// obstacle to work around. The payload is carried base64url rather than
    /// written out, because the appendix's own is the one with the `\r\n` in it.
    #[test]
    fn the_published_tokens_of_rfc_7515_verify_under_their_appendix_jwk() {
        let payload = "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFt\
                       cGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
        let rsa = serde_json::json!({
            "kty": "RSA",
            "n": "ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp\
                  -Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOW\
                  vG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUs\
                  LA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9k\
                  G4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2\
                  PAcDTW9gb54h4FRWyuXpoQ",
            "e": "AQAB",
        })
        .to_string();
        let ec = serde_json::json!({
            "kty": "EC",
            "crv": "P-256",
            "x": "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
            "y": "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0",
        })
        .to_string();

        let appendices = [
            (
                "A.2",
                KeyKind::RsaPkcs1,
                rsa,
                "eyJhbGciOiJSUzI1NiJ9",
                "cC4hiUPoj9Eetdgtv3hF80EGrhuB__dzERat0XF9g2VtQgr9PJbu3XOiZj5RZmh7\
                 AAuHIm4Bh-0Qc_lF5YKt_O8W2Fp5jujGbds9uJdbF9CUAr7t1dnZcAcQjbKBYNX4\
                 BAynRFdiuB--f_nZLgrnbyTyWzO75vRK5h6xBArLIARNPvkSjtQBMHlb1L07Qe7K\
                 0GarZRmB_eSN9383LcOLn6_dO--xi12jzDwusC-eOkHWEsqtFZESc6BfI7noOPqv\
                 hJ1phCnvWh6IeYI2w9QOYEUipUTI8np6LbgGY9Fs98rqVt5AXLIhWkWywlVmtVrB\
                 p0igcN_IoypGlUPQGe77Rw",
            ),
            (
                "A.3",
                KeyKind::P256,
                ec,
                "eyJhbGciOiJFUzI1NiJ9",
                "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSA\
                 pmWQxfKTUJqPP3-Kg6NU1Q",
            ),
        ];

        for (appendix, kind, jwk, header, signature) in appendices {
            let key = PublicKey::read(jwk.as_bytes(), kind, KeyFormat::Jwk)
                .expect("an appendix prints its public key as the JWK a browser exports");
            let signature = URL_SAFE_NO_PAD
                .decode(signature)
                .expect("a compact JWS carries its signature base64url");
            assert_eq!(
                verify_signature(
                    &key.verifying().expect("every kind of an appendix signs"),
                    format!("{header}.{payload}").as_bytes(),
                    &signature,
                ),
                Some(()),
                "RFC 7515 Appendix {appendix}, its own token under its own key"
            );
        }
    }

    /// Ed25519 against RFC 8037 Appendix A.4, which is the one published token
    /// of the roster whose signature is a function of the message alone — so the
    /// appendix is held on both sides: its token verifies under the public JWK
    /// it prints, and signing the same input under the `d` beside it writes the
    /// same octets back.
    ///
    /// The seed arrives as the 32 octets a JWK's `d` carries, so the PKCS#8
    /// [`PrivateKey::read`] takes is [`ED25519_PKCS8_PREFIX`] over it — the file
    /// [`generated_pkcs8`] writes for a drawn key, which is why a key that came
    /// out of a document needs no reader of its own.
    #[test]
    fn the_ed25519_token_of_rfc_8037_is_verified_and_signed_again_byte_for_byte() {
        let input = "eyJhbGciOiJFZERTQSJ9.RXhhbXBsZSBvZiBFZDI1NTE5IHNpZ25pbmc";
        let published = URL_SAFE_NO_PAD
            .decode(
                "hgyY0il_MGCjP0JzlnLWG1PPOt7-09PGcvMg3AIbQR6dWbhijcNR4ki4iylGjg5B\
                 hVsPt9g7sVvpAr_MuM0KAg",
            )
            .expect("a compact JWS carries its signature base64url");
        let jwk = serde_json::json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo",
        })
        .to_string();

        let key = PublicKey::read(jwk.as_bytes(), KeyKind::Ed25519, KeyFormat::Jwk)
            .expect("the appendix prints its public key as a JWK");
        assert_eq!(
            verify_signature(
                &key.verifying().expect("Ed25519 signs"),
                input.as_bytes(),
                &published,
            ),
            Some(()),
            "RFC 8037 Appendix A.4, the appendix's token under its own public key"
        );

        let seed = URL_SAFE_NO_PAD
            .decode("nWGxne_9WmC6hEr0kuwsxERJxWl7MmkZcDusAxyuf2A")
            .expect("the appendix prints its private key as the seed a JWK's `d` is");
        let pair = PrivateKey::read(&der_over(&ED25519_PKCS8_PREFIX, &seed), KeyKind::Ed25519)
            .expect("an RFC 8410 PKCS#8 over that seed is the file this module writes");
        assert_eq!(
            sign(
                &pair.signing().expect("an Ed25519 pair signs"),
                input.as_bytes(),
            ),
            Some(published),
            "RFC 8037 Appendix A.4's signature, written again from the seed beside it"
        );
    }

    /// A key read as an algorithm it cannot carry is refused at the read, so
    /// nothing downstream re-checks what a key is.
    ///
    /// The one pairing that is not a mistake is an RSA key read as either
    /// scheme: that is the single place `rule:security/algorithm-comes-from-the-key`
    /// leaves a choice, and it is made here and nowhere later.
    #[test]
    fn a_key_read_as_a_kind_it_is_not_is_refused_at_the_read() {
        let pkcs8 =
            |id: &str| webcrypto::octets(webcrypto::node(&format!("/jws/keys/{id}")), "/pkcs8");

        assert!(PrivateKey::read(&pkcs8("ed-1"), KeyKind::RsaPkcs1).is_none());
        assert!(PrivateKey::read(&pkcs8("ed-1"), KeyKind::P256).is_none());
        assert!(PrivateKey::read(&pkcs8("ed-1"), KeyKind::X25519).is_none());
        assert!(PrivateKey::read(&pkcs8("rsa-1"), KeyKind::Ed25519).is_none());
        assert!(PrivateKey::read(&pkcs8("ec-1"), KeyKind::RsaPss).is_none());
        assert!(PrivateKey::read(&pkcs8("ec-1"), KeyKind::Ed25519).is_none());
        assert!(
            PrivateKey::read(&pkcs8("rsa-1"), KeyKind::RsaPss).is_some(),
            "one key type carries both RSA algorithms, and the reader is where that is settled"
        );
    }

    /// The private half of every kind of the roster reads, answers the public
    /// half the set holds beside it, and signs only where the kind signs.
    ///
    /// The agreement shape: one question asked of all five kinds, counted, so a
    /// reader that carried the four signing kinds and lost X25519 — the shape
    /// the codec had before a `Crypto\KeyPair` needed all five — fails here
    /// while every line it does print still looks right.
    #[test]
    fn every_kind_of_the_roster_reads_a_private_key_and_derives_its_public_half() {
        let pairs = [
            ("/jws/keys/rsa-1", KeyKind::RsaPkcs1, true),
            ("/jws/keys/rsa-2", KeyKind::RsaPss, true),
            ("/jws/keys/ec-1", KeyKind::P256, true),
            ("/jws/keys/ed-1", KeyKind::Ed25519, true),
            ("/ecdh/vectors/1/a", KeyKind::X25519, false),
        ];

        let mut derived = 0;
        let mut signs = 0;
        for (path, kind, is_a_signer) in pairs {
            let node = webcrypto::node(path);
            let pair = PrivateKey::read(&webcrypto::octets(node, "/pkcs8"), kind)
                .expect("the set writes every private key as DER PKCS#8");
            let public = pair.public().expect("a pair that read has a public half");

            assert_eq!(public.kind(), kind, "{path}");
            if public
                .write(KeyFormat::Spki)
                .expect("every kind has a `SubjectPublicKeyInfo`")
                == webcrypto::octets(node, "/spki")
            {
                derived += 1;
            }
            if pair.signing().is_some() == is_a_signer {
                signs += 1;
            }
        }

        assert_eq!(
            derived,
            pairs.len(),
            "public halves derived from the private"
        );
        assert_eq!(signs, pairs.len(), "kinds that reach a signer");
    }

    /// One PEM `PRIVATE KEY` block is the same key its DER is, and neither a
    /// PKCS#1 body nor an encrypted PKCS#8 is read at all.
    ///
    /// The bound asserted on both sides: what `Core\Crypto\KeyPair::read`
    /// accepts is exactly one armour, so the case that is taken is named beside
    /// the two nearest ones that are refused.
    #[test]
    fn a_pem_private_key_block_reads_and_its_two_neighbours_do_not() {
        for id in ["rsa-1", "ec-1", "ed-1"] {
            let key = webcrypto::node(&format!("/jws/keys/{id}"));
            assert_eq!(
                pkcs8_der(webcrypto::text(key, "/pem").as_bytes()).as_deref(),
                Some(webcrypto::octets(key, "/pkcs8").as_slice()),
                "{id}'s PEM block carries its DER"
            );
        }

        let pkcs1 =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAK==\n-----END RSA PRIVATE KEY-----\n";
        let encrypted = "-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIBOgIBAAJBAK==\n-----END ENCRYPTED PRIVATE KEY-----\n";
        for armoured in [pkcs1, encrypted] {
            assert!(
                PrivateKey::read(
                    &pkcs8_der(armoured.as_bytes()).expect("an unrecognised label reads as DER"),
                    KeyKind::RsaPkcs1
                )
                .is_none(),
                "neither neighbour of `PRIVATE KEY` is a file this reads"
            );
        }
    }

    /// A key file reads into the pair's own two slots — the DER the file
    /// carried, and the kind the call named — whichever of the two spellings
    /// carried it.
    ///
    /// Driven through [`nvs_runtime::call`], because what the member answers is
    /// an object and the claim is about what that object holds. A `.nvst` case
    /// can ask a pair what it writes back; it cannot see that a PEM block is
    /// unwrapped *at the read*, so a member that stored the armoured text and
    /// unwrapped it again inside every later member would pass one and fail
    /// here.
    ///
    /// That is also the half [`a_pem_private_key_block_reads_and_its_two_neighbours_do_not`]
    /// leaves open: it reads [`pkcs8_der`] directly, one layer under the member.
    /// Both spellings of one key go in here, and the two pairs are compared slot
    /// for slot against the DER the frozen set holds.
    ///
    /// The stored DER is parsed back with [`PrivateKey::read`], since a slot
    /// holding the right octets under the wrong kind still answers every
    /// comparison above and fails at the first signature.
    // covers: Core\Crypto\KeyPair::read
    #[test]
    fn a_key_file_reads_into_the_der_and_the_kind_whichever_spelling_carried_it() {
        let slots = |octets: &[u8], kind: KeyKind| -> Option<(Vec<u8>, KeyKind)> {
            let mut ctx = nvs_runtime::Ctx::buffered();
            let pair = nvs_runtime::call(
                nvs_core_crypto_key_pair_read,
                &mut ctx,
                &[Value::bytes(NvsStr::new(octets)), Value::int(kind.tag())],
            )
            .ok()?;
            let (held, stored) = stored_key(&[pair], 0, &KEY_PAIR, "read")
                .expect("the member fills both of the class's slots");
            let der = stored_octets(&held, &KEY_PAIR, "read")
                .expect("the `pkcs8` slot holds the DER the member wrote")
                .to_vec();
            assert!(
                PrivateKey::read(&der, stored).is_some(),
                "the slots read back as a private key, which is what every member taking a pair \
                 does with them"
            );
            #[expect(
                unsafe_code,
                reason = "the call answered this reference and nothing else holds it, which is \
                          `Value::release`'s whole obligation"
            )]
            unsafe {
                pair.release();
            }
            Some((der, stored))
        };

        for (id, kind) in [
            ("rsa-1", KeyKind::RsaPkcs1),
            ("rsa-2", KeyKind::RsaPss),
            ("ec-1", KeyKind::P256),
            ("ed-1", KeyKind::Ed25519),
        ] {
            let node = webcrypto::node(&format!("/jws/keys/{id}"));
            let pkcs8 = webcrypto::octets(node, "/pkcs8");
            assert_eq!(
                slots(&pkcs8, kind),
                Some((pkcs8.clone(), kind)),
                "{id}'s DER is what the pair holds, under the kind the call named"
            );
            assert_eq!(
                slots(webcrypto::text(node, "/pem").as_bytes(), kind),
                Some((pkcs8, kind)),
                "{id}'s PEM block is unwrapped at the read, so no later member is handed armour"
            );
        }

        // The fifth kind of the roster, which the set files under the exchange
        // it is for and writes no PEM beside.
        let agreeing = webcrypto::octets(webcrypto::node("/ecdh/vectors/1/a"), "/pkcs8");
        assert_eq!(
            slots(&agreeing, KeyKind::X25519),
            Some((agreeing, KeyKind::X25519)),
            "every kind a program can hold a pair of reads here, not the four that sign"
        );
    }

    /// A pair writes the very octets in its own slot, and writing does not take
    /// them out of it.
    ///
    /// Driven through [`nvs_runtime::call`] over a pair
    /// [`nvs_core_crypto_key_pair_read`] built, because the claim is about the
    /// *slot* rather than about the value. A `.nvst` case asserts that what
    /// comes back reads as the same key, which a member re-encoding the key
    /// from its parsed components would pass as well — and that member is the
    /// one [`KEY_PAIR`]'s doc says this is not, since RSA's private components
    /// are read here and never written. The octets are compared by address,
    /// which only the slot's own buffer has, and that is also what the ledger's
    /// `allocations 0` records.
    ///
    /// Written twice, because a member moving the value out of the slot answers
    /// the right octets once and leaves a pair no later member can use.
    // covers: Core\Crypto\KeyPair::write
    #[test]
    fn a_pair_writes_the_octets_in_its_own_slot_and_keeps_them() {
        let pkcs8 = webcrypto::octets(webcrypto::node("/jws/keys/ed-1"), "/pkcs8");
        let mut ctx = nvs_runtime::Ctx::buffered();
        let pair = nvs_runtime::call(
            nvs_core_crypto_key_pair_read,
            &mut ctx,
            &[
                Value::bytes(NvsStr::new(&pkcs8)),
                Value::int(KeyKind::Ed25519.tag()),
            ],
        )
        .expect("the set's Ed25519 private key reads");
        let (held, _) = stored_key(&[pair], 0, &KEY_PAIR, "write")
            .expect("`read` fills both of the class's slots");
        let slot = stored_octets(&held, &KEY_PAIR, "write")
            .expect("the `pkcs8` slot holds the DER")
            .as_ptr();

        for turn in 1..=2 {
            let written = nvs_runtime::call(nvs_core_crypto_key_pair_write, &mut ctx, &[pair])
                .expect("a pair writes whenever it is asked");
            let octets = written.as_bytes().expect("`write` answers a `bytes`");
            assert_eq!(
                octets,
                pkcs8.as_slice(),
                "turn {turn}: the file the pair was read from"
            );
            assert_eq!(
                octets.as_ptr(),
                slot,
                "turn {turn}: the slot itself, rather than a copy of it"
            );
            #[expect(
                unsafe_code,
                reason = "`instance::read_slot` retained this reference for its caller, so \
                          releasing it is what the member's own caller owes"
            )]
            unsafe {
                written.release();
            }
        }

        #[expect(
            unsafe_code,
            reason = "the call answered this reference and nothing else holds it, which is \
                      `Value::release`'s whole obligation"
        )]
        unsafe {
            pair.release();
        }
    }

    /// The public half a pair answers is the one the frozen set exported beside
    /// its private key, for every kind, and each call answers a half of its own.
    ///
    /// Driven through [`nvs_runtime::call`], one layer above
    /// [`every_kind_of_the_roster_reads_a_private_key_and_derives_its_public_half`],
    /// which asks [`PrivateKey::public`] the same question directly: what this
    /// adds is that the member fills a `Crypto\PublicKey`'s own slots with that
    /// answer, so a derivation that is right and a class that is handed the
    /// wrong kind cannot pass together.
    ///
    /// Two halves are taken from one pair and the first is released while the
    /// second is still read, since the attack beside this keeps a hundred
    /// thousand of them: a member handing out one shared instance would answer
    /// every assertion above and leave the survivors reading freed octets.
    // covers: Core\Crypto\KeyPair::publicKey
    #[test]
    fn every_pair_answers_the_public_half_the_set_exported_and_a_fresh_one_each_call() {
        let mut derived = 0;
        let pairs = [
            ("/jws/keys/rsa-1", KeyKind::RsaPkcs1),
            ("/jws/keys/rsa-2", KeyKind::RsaPss),
            ("/jws/keys/ec-1", KeyKind::P256),
            ("/jws/keys/ed-1", KeyKind::Ed25519),
            ("/ecdh/vectors/1/a", KeyKind::X25519),
        ];
        for (path, kind) in pairs {
            let node = webcrypto::node(path);
            let mut ctx = nvs_runtime::Ctx::buffered();
            let pair = nvs_runtime::call(
                nvs_core_crypto_key_pair_read,
                &mut ctx,
                &[
                    Value::bytes(NvsStr::new(&webcrypto::octets(node, "/pkcs8"))),
                    Value::int(kind.tag()),
                ],
            )
            .expect("every kind of the roster reads");

            let half = |ctx: &mut nvs_runtime::Ctx| {
                nvs_runtime::call(nvs_core_crypto_key_pair_public_key, ctx, &[pair])
                    .expect("a pair that read has a public half")
            };
            let first = half(&mut ctx);
            let second = half(&mut ctx);
            #[expect(
                unsafe_code,
                reason = "the call answered this reference and nothing else holds it, which is \
                          `Value::release`'s whole obligation"
            )]
            unsafe {
                first.release();
            }

            let (spki, stored) = stored_key(&[second], 0, &PUBLIC_KEY, "publicKey")
                .expect("the member fills both of the class's slots");
            assert_eq!(stored, kind, "{path}: the half is of the pair's own kind");
            if stored_octets(&spki, &PUBLIC_KEY, "publicKey")
                .expect("the `spki` slot holds the encoding the member wrote")
                == webcrypto::octets(node, "/spki")
            {
                derived += 1;
            }
            #[expect(
                unsafe_code,
                reason = "the call answered this reference and nothing else holds it, which is \
                          `Value::release`'s whole obligation"
            )]
            unsafe {
                second.release();
            }
        }

        assert_eq!(
            derived,
            pairs.len(),
            "every kind's public half is the one the set exported, counted so that a member \
             carrying four kinds and losing the fifth fails here"
        );
    }

    /// The kind the set's own label for a key names, so no case here picks one
    /// for itself — the JWS keys are labelled by `alg` and the agreement pairs
    /// by `curve`.
    fn kind_of(label: &str) -> KeyKind {
        match label {
            "RS256" => KeyKind::RsaPkcs1,
            "PS256" => KeyKind::RsaPss,
            "ES256" | "P-256" => KeyKind::P256,
            "EdDSA" | "Ed25519" => KeyKind::Ed25519,
            "X25519" => KeyKind::X25519,
            other => panic!("the set labels a key {other}, which this module does not read"),
        }
    }

    /// Every public key the frozen set carries, as the node holding its
    /// encodings and the kind its label names.
    fn public_keys() -> Vec<(&'static serde_json::Value, KeyKind)> {
        let mut keys = Vec::new();
        for id in ["rsa-1", "rsa-2", "ec-1", "ed-1"] {
            let key = webcrypto::node(&format!("/jws/keys/{id}"));
            keys.push((key, kind_of(webcrypto::text(key, "/alg"))));
        }

        for vector in webcrypto::vectors("ecdh") {
            let kind = kind_of(webcrypto::text(vector, "/curve"));
            for side in ["/a", "/b"] {
                let party = vector
                    .pointer(side)
                    .expect("the set nests a key pair under each party");
                keys.push((party, kind));
            }
        }

        keys
    }

    /// The case index a call site writes for a [`KeyFormat`], taken from the
    /// registry's own enum rather than from a digit at each call.
    fn format_tag(format: KeyFormat) -> i64 {
        KEY_FORMAT
            .cases
            .iter()
            .map(|(_, case)| *case)
            .find(|case| KeyFormat::from_tag(*case) == Some(format))
            .expect("every encoding the codec reads is a case of the registry's enum")
    }

    /// A public key off the wire lands in the class's two slots as the set's own
    /// `SubjectPublicKeyInfo` and the kind the call named, whichever of the
    /// three encodings carried it.
    ///
    /// Driven through [`nvs_runtime::call`], one layer above
    /// [`webcrypto_jws_keys_read_in_every_form_and_write_their_minimal_jwk_and_thumbprint`],
    /// which asks the codec the same question. What this adds is the *slot*: a
    /// `.nvst` case sees only what `write` answers, so a member that kept the
    /// octets it was handed and re-encoded them inside every later member passes
    /// there and fails here. A JWK is read in the browser's own spelling, `ext`
    /// and `key_ops` included, which is the one a slot holding the arriving
    /// octets would carry into every member that reads it.
    ///
    /// The stored octets are parsed back with [`PublicKey::read`], since a slot
    /// holding the right encoding under the wrong kind answers every comparison
    /// here and fails at the first signature.
    // covers: Core\Crypto\PublicKey::read
    #[test]
    fn a_public_key_reads_into_the_canonical_spki_and_the_named_kind() {
        let slots = |octets: &[u8], kind: KeyKind, format: KeyFormat| {
            let mut ctx = nvs_runtime::Ctx::buffered();
            let key = nvs_runtime::call(
                nvs_core_crypto_public_key_read,
                &mut ctx,
                &[
                    Value::bytes(NvsStr::new(octets)),
                    Value::int(kind.tag()),
                    Value::int(format_tag(format)),
                ],
            )
            .expect("the set's own export of a key reads");
            let (held, stored) = stored_key(&[key], 0, &PUBLIC_KEY, "read")
                .expect("the member fills both of the class's slots");
            let spki = stored_octets(&held, &PUBLIC_KEY, "read")
                .expect("the `spki` slot holds the encoding the member wrote")
                .to_vec();
            assert!(
                PublicKey::read(&spki, stored, KeyFormat::Spki).is_ok(),
                "the slots read back as a public key, which is what every member taking one does \
                 with them"
            );
            #[expect(
                unsafe_code,
                reason = "the call answered this reference and nothing else holds it, which is \
                          `Value::release`'s whole obligation"
            )]
            unsafe {
                key.release();
            }
            (spki, stored)
        };

        let roster = public_keys();
        let mut encodings = 0;
        for (key, kind) in &roster {
            let (key, kind) = (*key, *kind);
            let spki = webcrypto::octets(key, "/spki");
            let minimal = webcrypto::text(key, "/jwkMinimal");
            let mut arrivals = vec![
                (spki.clone(), KeyFormat::Spki),
                (
                    webcrypto::text(key, "/jwk").as_bytes().to_vec(),
                    KeyFormat::Jwk,
                ),
            ];
            if key.pointer("/raw").is_some() {
                arrivals.push((webcrypto::octets(key, "/raw"), KeyFormat::Raw));
            }

            for (octets, format) in arrivals {
                assert_eq!(
                    slots(&octets, kind, format),
                    (spki.clone(), kind),
                    "{minimal} read from {format:?} holds the set's own SPKI under the kind the \
                     call named"
                );
                encodings += 1;
            }
        }

        let with_raw = roster
            .iter()
            .filter(|(key, _)| key.pointer("/raw").is_some())
            .count();
        assert_eq!(
            encodings,
            2 * roster.len() + with_raw,
            "every key of the set arrives in each encoding it was exported in, counted so that a \
             roster that stopped being read still fails"
        );
    }

    /// A key answers its own kind as the case index the registry's enum names,
    /// however the key was reached and however often it is asked.
    ///
    /// Driven through [`nvs_runtime::call`], because the claim is about the
    /// *representation*: what crosses is the integer a `Crypto\KeyKind` case is,
    /// which is what makes the member a slot read and what the ledger's
    /// `allocations 0` records. A `.nvst` case compares the answer against a
    /// case and sees the same thing whatever the member built to answer with.
    ///
    /// Both members that fill the slot are asked — the read of a key a peer
    /// sent, and the derivation from a pair — since a kind that is right on one
    /// route and wrong on the other still looks right wherever a program uses
    /// one route.
    // covers: Core\Crypto\PublicKey::kind
    #[test]
    fn a_key_answers_its_kind_as_the_registrys_own_case_however_it_was_reached() {
        let asked = |ctx: &mut nvs_runtime::Ctx, key: Value| {
            let answered = nvs_runtime::call(nvs_core_crypto_public_key_kind, ctx, &[key])
                .expect("a key that read knows what it is")
                .as_int()
                .expect("a `Crypto\\KeyKind` crosses as the integer its case is");
            assert!(
                KEY_KIND.cases.iter().any(|(_, case)| *case == answered),
                "the answer is a case of the registry's own enum, rather than any integer"
            );
            answered
        };

        for (node, kind) in public_keys() {
            let mut ctx = nvs_runtime::Ctx::buffered();
            let sent = nvs_runtime::call(
                nvs_core_crypto_public_key_read,
                &mut ctx,
                &[
                    Value::bytes(NvsStr::new(&webcrypto::octets(node, "/spki"))),
                    Value::int(kind.tag()),
                    Value::int(format_tag(KeyFormat::Spki)),
                ],
            )
            .expect("the set writes every public key as SPKI");
            // Asked twice, since a member taking the kind out of its slot
            // answers the first call and leaves an object no later member can
            // use.
            for turn in 1..=2 {
                assert_eq!(
                    asked(&mut ctx, sent),
                    kind.tag(),
                    "turn {turn}: the kind the read was given"
                );
            }

            let pair = nvs_runtime::call(
                nvs_core_crypto_key_pair_read,
                &mut ctx,
                &[
                    Value::bytes(NvsStr::new(&webcrypto::octets(node, "/pkcs8"))),
                    Value::int(kind.tag()),
                ],
            )
            .expect("the set writes every private key as PKCS#8");
            let derived = nvs_runtime::call(nvs_core_crypto_key_pair_public_key, &mut ctx, &[pair])
                .expect("a pair that read has a public half");
            assert_eq!(
                asked(&mut ctx, derived),
                kind.tag(),
                "a derived half is of the pair's own kind"
            );

            #[expect(
                unsafe_code,
                reason = "each call answered one of these references and nothing else holds them, \
                          which is `Value::release`'s whole obligation"
            )]
            unsafe {
                derived.release();
                pair.release();
                sent.release();
            }
        }
    }

    /// A key writes the set's own octets in every encoding it has, and writes
    /// them out of the key rather than out of the slot it was read into.
    ///
    /// Driven through [`nvs_runtime::call`] over a key
    /// [`nvs_core_crypto_public_key_read`] built, because the claim is about the
    /// *slot*: [`PUBLIC_KEY`]'s doc prices a write in the arriving encoding as a
    /// copy, and a `.nvst` case sees equal octets whether the member copied them
    /// or handed out the slot's own buffer. Each encoding is written twice,
    /// since a member moving the value out of the slot answers the right octets
    /// once and leaves an object no later member can use.
    // covers: Core\Crypto\PublicKey::write
    #[test]
    fn a_key_writes_a_copy_of_every_encoding_and_keeps_its_slot() {
        for (node, kind) in public_keys() {
            let spki = webcrypto::octets(node, "/spki");
            let minimal = webcrypto::text(node, "/jwkMinimal");
            let mut ctx = nvs_runtime::Ctx::buffered();
            let key = nvs_runtime::call(
                nvs_core_crypto_public_key_read,
                &mut ctx,
                &[
                    Value::bytes(NvsStr::new(&spki)),
                    Value::int(kind.tag()),
                    Value::int(format_tag(KeyFormat::Spki)),
                ],
            )
            .expect("the set writes every public key as SPKI");
            let (held, _) = stored_key(&[key], 0, &PUBLIC_KEY, "write")
                .expect("`read` fills both of the class's slots");
            let slot = stored_octets(&held, &PUBLIC_KEY, "write")
                .expect("the `spki` slot holds the encoding the member wrote")
                .as_ptr();

            let mut wanted = vec![
                (KeyFormat::Spki, spki.clone()),
                (KeyFormat::Jwk, minimal.as_bytes().to_vec()),
            ];
            if node.pointer("/raw").is_some() {
                wanted.push((KeyFormat::Raw, webcrypto::octets(node, "/raw")));
            }

            for turn in 1..=2 {
                for (format, want) in &wanted {
                    let written = nvs_runtime::call(
                        nvs_core_crypto_public_key_write,
                        &mut ctx,
                        &[key, Value::int(format_tag(*format))],
                    )
                    .expect("a key writes every encoding it has, whenever it is asked");
                    let octets = written.as_bytes().expect("`write` answers a `bytes`");
                    assert_eq!(
                        octets,
                        want.as_slice(),
                        "turn {turn}: {minimal} written as {format:?}"
                    );
                    assert_ne!(
                        octets.as_ptr(),
                        slot,
                        "turn {turn}: a copy, which is what the class's doc spends those octets on"
                    );
                    #[expect(
                        unsafe_code,
                        reason = "the call answered this reference and nothing else holds it, \
                                  which is `Value::release`'s whole obligation"
                    )]
                    unsafe {
                        written.release();
                    }
                }
            }

            assert_eq!(
                stored_octets(&held, &PUBLIC_KEY, "write")
                    .expect("the `spki` slot is still filled")
                    .as_ptr(),
                slot,
                "{minimal}: a write leaves the object's own octets where they were"
            );
            #[expect(
                unsafe_code,
                reason = "the call answered this reference and nothing else holds it, which is \
                          `Value::release`'s whole obligation"
            )]
            unsafe {
                key.release();
            }
        }
    }

    /// Every encoding a public key of the frozen set is exported in, read and
    /// written back, against WebCrypto's own octets rather than against this
    /// codec's — `raw`, `spki` and `jwk`, which are the forms a public key has.
    /// The private ones a key is also stored in are the two tests above, where
    /// `pkcs8` and its PEM armour are read.
    ///
    /// The shape is agreement: one key arrives in two or three encodings and
    /// has to become *one* key, so a reader that dropped a coordinate or kept a
    /// DER sign octet still looks right on its own line and fails here. The
    /// thumbprint is asserted twice on purpose — against RFC 7638's digest over
    /// the set's own canonical members, and against the value Node computed for
    /// the keys the set carries one for — because the first pins the digest and
    /// the second pins the canonicalization it is taken over.
    #[test]
    fn webcrypto_jws_keys_read_in_every_form_and_write_their_minimal_jwk_and_thumbprint() {
        for (key, kind) in public_keys() {
            let minimal = webcrypto::text(key, "/jwkMinimal");
            let spki = webcrypto::octets(key, "/spki");
            let from_spki = PublicKey::read(&spki, kind, KeyFormat::Spki)
                .expect("the set writes every public key as SPKI");
            let from_jwk = PublicKey::read(
                webcrypto::text(key, "/jwk").as_bytes(),
                kind,
                KeyFormat::Jwk,
            )
            .expect("the set writes what a browser's own exportKey wrote");
            let mut held = vec![from_spki, from_jwk];

            if key.pointer("/raw").is_some() {
                let raw = webcrypto::octets(key, "/raw");
                let from_raw = PublicKey::read(&raw, kind, KeyFormat::Raw)
                    .expect("the set's raw export is the key material");
                assert_eq!(
                    from_raw
                        .write(KeyFormat::Raw)
                        .expect("a curve kind has a raw form"),
                    raw,
                    "{minimal}"
                );
                held.push(from_raw);
            }

            let mut expected = Sha256::new();
            expected.update(minimal.as_bytes());
            let thumbprint = URL_SAFE_NO_PAD.encode(expected.finalize());

            for key_read in &held {
                assert_eq!(key_read.kind(), kind, "{minimal}");
                assert_eq!(
                    key_read.p256_point().is_some(),
                    kind == KeyKind::P256,
                    "the point an agreement takes is P-256's alone: {minimal}"
                );
                assert_eq!(
                    key_read
                        .write(KeyFormat::Spki)
                        .expect("every kind has a SubjectPublicKeyInfo"),
                    spki,
                    "{minimal}"
                );
                assert_eq!(
                    String::from_utf8(
                        key_read
                            .write(KeyFormat::Jwk)
                            .expect("every kind has a JWK")
                    )
                    .expect("a JWK is text"),
                    minimal,
                    "{minimal}"
                );
                assert_eq!(key_read.thumbprint(), thumbprint, "{minimal}");
            }

            if key.pointer("/thumbprint").is_some() {
                assert_eq!(
                    thumbprint,
                    webcrypto::text(key, "/thumbprint"),
                    "RFC 7638's thumbprint, as Node computed it for {minimal}"
                );
            }
        }
    }

    /// The roster's RSA width, asserted at the last modulus admitted and the
    /// first refused at each end.
    ///
    /// A member that stops one octet early prints plausibly against either half
    /// alone, which is why both edges are named together, and the widths come
    /// from [`MIN_RSA_BITS`] and [`MAX_RSA_BITS`] rather than from digits here.
    /// The set's own 1024-bit key is the witness that the bound is checked in
    /// both encodings and not just in the one a test happened to use.
    #[test]
    fn the_rsa_width_bound_is_asserted_on_both_sides() {
        let read = |octets: usize| {
            let jwk = format!(
                r#"{{"kty":"RSA","n":"{}","e":"AQAB"}}"#,
                URL_SAFE_NO_PAD.encode(vec![0xff_u8; octets])
            );
            PublicKey::read(jwk.as_bytes(), KeyKind::RsaPkcs1, KeyFormat::Jwk).is_ok()
        };

        let narrowest = usize::try_from(MIN_RSA_BITS / 8).expect("a width in octets");
        let widest = usize::try_from(MAX_RSA_BITS / 8).expect("a width in octets");
        for (octets, admitted) in [
            (narrowest - 1, false),
            (narrowest, true),
            (widest, true),
            (widest + 1, false),
        ] {
            assert_eq!(read(octets), admitted, "a modulus of {octets} octets");
        }

        let weak = webcrypto::node("/jws/keys/rsa-weak");
        assert!(
            PublicKey::read(
                &webcrypto::octets(weak, "/spki"),
                KeyKind::RsaPkcs1,
                KeyFormat::Spki,
            )
            .is_err(),
            "the set's own 1024-bit key is refused at the read, from its SPKI"
        );
        assert!(
            PublicKey::read(
                webcrypto::text(weak, "/jwk").as_bytes(),
                KeyKind::RsaPkcs1,
                KeyFormat::Jwk,
            )
            .is_err(),
            "and from its JWK, so neither door is the lenient one"
        );
    }

    /// Every refusal the codec makes, split the way
    /// `rule:core-classes/crypto-interop-tier` splits them: an encoding a kind
    /// does not have is a bug in the program, and octets that are not a key are
    /// a verdict on whoever sent them.
    ///
    /// The skewed-coordinate case is the one a total-length check would pass: a
    /// 31-octet `x` beside a 33-octet `y` concatenates to the very octets of
    /// this key's own point, so a reader checking only the point's width would
    /// read one JWK as a key it does not describe.
    #[test]
    fn an_encoding_a_kind_does_not_have_is_a_bug_and_bad_octets_are_a_verdict() {
        let ec = webcrypto::node("/jws/keys/ec-1");
        let rsa = webcrypto::node("/jws/keys/rsa-1");
        let montgomery = webcrypto::node("/ecdh/vectors/1/a");
        let point = webcrypto::octets(webcrypto::node("/ecdh/vectors/0/a"), "/raw");

        for kind in [KeyKind::RsaPkcs1, KeyKind::RsaPss] {
            assert!(
                matches!(
                    PublicKey::read(&point, kind, KeyFormat::Raw),
                    Err(KeyRefusal::Bug)
                ),
                "an RSA key has no raw form, and asking for one is a bug rather than a bad key"
            );
        }

        let rsa_key = PublicKey::read(
            webcrypto::text(rsa, "/jwk").as_bytes(),
            KeyKind::RsaPkcs1,
            KeyFormat::Jwk,
        )
        .expect("the set's RSA key reads from its own JWK");
        assert!(
            matches!(rsa_key.write(KeyFormat::Raw), Err(KeyRefusal::Bug)),
            "and writing one is the same bug at the other door"
        );
        assert!(
            rsa_key.verifying().is_some(),
            "the kinds that have no raw form are still the kinds that verify"
        );

        let x25519 = PublicKey::read(
            &webcrypto::octets(montgomery, "/raw"),
            KeyKind::X25519,
            KeyFormat::Raw,
        )
        .expect("the set's X25519 export is 32 octets of u-coordinate");
        assert!(
            x25519.verifying().is_none(),
            "the one kind that signs nothing reaches no verifier"
        );

        let private = format!(
            "{},\"d\":\"AA\"}}",
            webcrypto::text(montgomery, "/jwkMinimal").trim_end_matches('}')
        );
        assert!(
            matches!(
                PublicKey::read(private.as_bytes(), KeyKind::X25519, KeyFormat::Jwk),
                Err(KeyRefusal::Bug)
            ),
            "a JWK carrying `d` is a private key handed over as a public one"
        );

        let minimal: serde_json::Value = serde_json::from_str(webcrypto::text(ec, "/jwkMinimal"))
            .expect("the set writes a minimal JWK as JSON text");
        let coordinate = |name: &str| {
            URL_SAFE_NO_PAD
                .decode(
                    minimal[name]
                        .as_str()
                        .expect("an EC JWK carries this member"),
                )
                .expect("a JWK member is base64url")
        };
        let (x, y) = (coordinate("x"), coordinate("y"));
        let mut shifted = vec![x[0]];
        shifted.extend_from_slice(&y);
        let skewed = format!(
            r#"{{"crv":"P-256","kty":"EC","x":"{}","y":"{}"}}"#,
            URL_SAFE_NO_PAD.encode(&x[1..]),
            URL_SAFE_NO_PAD.encode(&shifted)
        );
        assert!(
            matches!(
                PublicKey::read(skewed.as_bytes(), KeyKind::P256, KeyFormat::Jwk),
                Err(KeyRefusal::Octets)
            ),
            "two coordinates that concatenate to this key's own point are still not this key"
        );

        assert!(
            matches!(
                PublicKey::read(
                    webcrypto::text(ec, "/jwk").as_bytes(),
                    KeyKind::Ed25519,
                    KeyFormat::Jwk,
                ),
                Err(KeyRefusal::Octets)
            ),
            "a document naming another key type is not this key in another encoding"
        );

        assert!(
            PublicKey::read(&point[..P256_POINT_LEN - 1], KeyKind::P256, KeyFormat::Raw).is_err(),
            "a point one octet short is refused before anything multiplies by it"
        );
        assert!(
            PublicKey::read(
                &webcrypto::octets(montgomery, "/raw")[..CURVE25519_POINT_LEN - 1],
                KeyKind::X25519,
                KeyFormat::Raw,
            )
            .is_err(),
            "and so is a u-coordinate one octet short"
        );
        assert!(
            PublicKey::read(
                &webcrypto::octets(ec, "/spki"),
                KeyKind::RsaPkcs1,
                KeyFormat::Spki
            )
            .is_err(),
            "an EC key's SubjectPublicKeyInfo read as RSA is refused on its algorithm identifier"
        );
    }

    /// A key this codec read verifies what WebCrypto signed, which is the two
    /// ways of assembling a [`VerifyingKey`] asserted to be one key.
    ///
    /// [`verifying`] builds one from the JWK's members by hand; this builds one
    /// through [`PublicKey::read`] and hands both the same frozen signatures.
    /// A codec that reordered an RSA component or dropped a point's tag would
    /// still round-trip its own encodings and fail every line of this.
    #[test]
    fn a_key_the_codec_read_verifies_what_webcrypto_signed() {
        for vector in webcrypto::vectors("signatures") {
            let name = webcrypto::text(vector, "/name");
            let key = webcrypto::node(&format!("/jws/keys/{}", webcrypto::text(vector, "/key")));
            let kind = kind_of(webcrypto::text(key, "/alg"));
            let held = PublicKey::read(&webcrypto::octets(key, "/spki"), kind, KeyFormat::Spki)
                .expect("every signing key of the set reads from its own SPKI");

            assert_eq!(
                verify_signature(
                    &held.verifying().expect("a signing kind lends a verifier"),
                    &webcrypto::octets(vector, "/message"),
                    &webcrypto::octets(vector, "/signature"),
                ),
                Some(()),
                "{name}"
            );
        }
    }

    /// The two Rust enums cover exactly the registry enums' cases, which is the
    /// only thing holding [`KeyKind::from_tag`]'s integers to [`KEY_KIND`]'s.
    ///
    /// A case added to one list and not the other is a program that compiles
    /// and then reads a tag nothing answers, so what is asserted is the whole
    /// list rather than a spelling of it.
    #[test]
    fn the_key_enums_cover_exactly_the_registry_enums_cases() {
        for (name, tag) in KEY_KIND.cases {
            assert_eq!(
                KeyKind::from_tag(*tag).map(KeyKind::tag),
                Some(*tag),
                "`{name}` is a case no Rust kind answers, or answers under another constant"
            );
        }
        for (name, tag) in KEY_FORMAT.cases {
            assert!(
                KeyFormat::from_tag(*tag).is_some(),
                "`{name}` is a case no Rust format answers"
            );
        }

        let kinds = i64::try_from(KEY_KIND.cases.len()).expect("a case count");
        let formats = i64::try_from(KEY_FORMAT.cases.len()).expect("a case count");
        assert!(
            KeyKind::from_tag(kinds).is_none() && KeyKind::from_tag(-1).is_none(),
            "the tags run 0 to one less than the count, and nothing outside them is a kind"
        );
        assert!(
            KeyFormat::from_tag(formats).is_none() && KeyFormat::from_tag(-1).is_none(),
            "and the same for the three encodings"
        );
        assert_eq!(KeyKind::from_tag(1), Some(KeyKind::X25519));
        assert_eq!(KeyFormat::from_tag(2), Some(KeyFormat::Jwk));
    }
}
