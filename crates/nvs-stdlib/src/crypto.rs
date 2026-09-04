//! `Core\Crypto` — [ADR 0051](/docs/adr/0051-standard-library-tiers.md)
//! § 3's "AEAD only, no ECB, no unauthenticated CBC, no cipher-name-as-string",
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
//! AES-GCM was the alternative and loses on two counts: its 96-bit nonce has
//! the bound above with no extended variant, and its software fallback is a
//! constant-time bitslice that is several times slower than ChaCha20 on any
//! machine without AES-NI — of which a container host scheduling this runtime
//! is still one often enough to matter.
//!
//! # What a sealed message is
//!
//! `nonce ‖ ciphertext ‖ tag` — [`NONCE_LEN`] octets, then as many as the
//! plaintext had, then [`TAG_LEN`]. The overhead is [`OVERHEAD`] octets flat,
//! and the layout is not a format anything else parses: nothing outside this
//! module reads a field of it, and a program that wants an interchange format
//! wants a protocol from [ADR 0060](/docs/adr/0060-application-security-protocols.md)'s
//! roster rather than this member's output. Stating it here is so that the
//! *size* is predictable, not so that it is depended on.
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
//! `?bytes`, which is [ADR 0063](/docs/adr/0063-core-api-conventions.md)'s
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
//! than one of ADR 0088 § 2's classifications — that variant's own docs are the
//! home of the difference. Two things follow, and they are the reason this
//! class was worth waiting for the spelling. A program cannot put a generated
//! key in a plain `bytes` variable: the assignment narrows a qualifier and the
//! checker refuses it, so a key stays out of `echo`, out of a log and out of a
//! `Throwable` message by construction rather than by review. And a `secret`
//! key reaches `seal` without any member removing the mark, so `Core\Crypto`
//! writes no [`Qual::Reveal`] and ADR 0033 § 3's launderer roster stays the two
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
//! into bytes that leave this process is the one event ADR 0033 exists to make
//! greppable, and encryption is not an exemption from it — it is the case it
//! was written for.
//!
//! # This construction has one home, and two classes are on the near side of it
//!
//! [`cipher`], [`seal_under`] and [`open_under`] are `pub(crate)`, and
//! [`crate::signed_cookie`] — [ADR 0060](/docs/adr/0060-application-security-protocols.md)
//! § 1's first roster entry — is their second caller. That is what makes a
//! signed cookie *this* AEAD with a key ring over it rather than a second
//! construction with its own nonce policy and its own opinion about tags: there
//! is exactly one `XChaCha20Poly1305::new_from_slice` in `nvs-stdlib`, and
//! everything above reaches it through those three functions. Nothing outside
//! this module reads a *field* of a sealed message, which is the sentence above
//! and is still true — a caller gets the whole buffer or nothing.
//!
//! # The nonce is drawn through `Core\Random`'s seam
//!
//! `chacha20poly1305`'s `getrandom` feature is off in `Cargo.toml`, for
//! [`crate::password`]'s reason and the same one again: every draw in `Core`
//! goes through [`crate::random::draw`], whose own doc comment is the home of
//! why, so this tree has one CSPRNG and a `#[Test(seed: …)]` reproduces a
//! sealed message byte for byte along with every other draw the test made.

use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::Rng;

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Crypto";

/// A key's length in octets — XChaCha20-Poly1305's only key size, so this is
/// the construction's number rather than a choice of ours.
const KEY_LEN: usize = 32;

/// The nonce's length in octets, and the module doc's *why the extended nonce*
/// section is the whole of why it is 24 and not 12.
const NONCE_LEN: usize = 24;

/// Poly1305's authentication tag, in octets.
const TAG_LEN: usize = 16;

/// What a sealed message costs over its plaintext, and the shortest buffer
/// [`nvs_core_crypto_open`] could authenticate — a nonce and a tag with an
/// empty message between them.
const OVERHEAD: usize = NONCE_LEN + TAG_LEN;

/// ADR 0051 § 3's AEAD-only surface, as three rows.
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

/// `Core\Crypto::generateKey`'s reference card — ADR 0117.
const GENERATE_KEY_DOC: MethodDoc = MethodDoc {
    short: "Draws a fresh key for `seal` and `open` from the same CSPRNG `Core\\Random` uses. \
            There is no key size argument: the construction has one key size.",
    params: &[],
    ret: "32 octets as a `secret bytes`. The qualifier is part of the type, so the key cannot \
          be assigned to a plain `bytes`, echoed, logged or put in a `Throwable` message.",
    errors: &[],
};

/// `Core\Crypto::seal`'s reference card — ADR 0117.
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

/// `Core\Crypto::open`'s reference card — ADR 0117.
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
/// reads it too, which is what makes ADR 0060 § 1's cookie entry *this*
/// construction with a key ring over it rather than a second one: there is one
/// `XChaCha20Poly1305::new_from_slice` in `nvs-stdlib` and both classes are on
/// the near side of it.
pub(crate) fn cipher(key: &[u8]) -> Option<XChaCha20Poly1305> {
    XChaCha20Poly1305::new_from_slice(key).ok()
}

/// The `LogicError` a key of the wrong length earns, in one sentence for every
/// member that keys [`cipher`].
///
/// `who` is the member, spelled `Core\Class::member`, and `param` the argument
/// as the caller wrote it — `$key` here and `$keys[0]` one class over, where a
/// key ring's entries are what get keyed.
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

    /// Every cipher this class can reach is authenticated — stage 4's first
    /// named check, asked of the construction rather than of a roster, because
    /// the roster is one entry and the claim is about what that entry does.
    ///
    /// Asserted by *forging*: a sealed message with one octet changed at every
    /// position in the tag, in the ciphertext and in the nonce has to be
    /// refused. An unauthenticated mode passes a round-trip test and fails
    /// every one of these, which is precisely the failure ADR 0051 § 3's
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
}
