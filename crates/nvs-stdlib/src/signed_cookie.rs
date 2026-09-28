//! `Core\SignedCookie` — `rule:security/protocol-roster`
//! 's first roster entry: [`crate::crypto`]'s construction with a key ring
//! over it and a cookie-safe spelling around it, and no second cipher anywhere.
//!
//! `rule:security/protocol-roster` places the class and § 2 says why the roster is closed at four;
//! what belongs here is which end of the ring is the newest key, why `open`
//! removes `tainted` when `rule:security/launderers-are-sink-named`
//! refuses that nearly everywhere else, and why two members named `seal`
//! and `open` are not a second way to reach `Core\Crypto`'s two.
//!
//! # The key ring is an `array<secret bytes>`, newest first
//!
//! [`crate::keyring`] is the home of what a ring is, which end of it is the
//! newest and what an unusable one earns, because this class is no longer the
//! only member that takes one. Here: [`seal`] takes the newest key and nothing
//! else, and [`open`] tries the whole ring in order.
//!
//! Order is also what makes the ring *cheap*: the newest key opens almost every
//! cookie, so the common path is one AEAD open and the loop is a rotation-window
//! tax rather than a per-request one. **What this spends** is one open attempt
//! per key until one authenticates, so a ring of `n` costs at most `n` opens of
//! a cookie-sized buffer, all inside the call and none held between calls.
//!
//! # `open` launders, and it is the only cookie in the language that does
//!
//! `rule:security/verification-does-not-launder`
//! is emphatic that a verified signature does not launder — JWT claims come
//! back `tainted` because a signature proves origin and not safety — and then
//! names this one exception: a cookie payload the application itself sealed
//! round-trips through our own AEAD unchanged, and comes back **unqualified**.
//! The distinction is not about the strength of the authentication, which is
//! identical; it is that the plaintext here was *ours and plain* when it went
//! in, so returning it qualified would mark a value the program already held
//! unmarked one line earlier.
//!
//! That makes [`open`]'s `$cookie` a [`Qual::Launder`] parameter, and the sink
//! it launders for is **none of them** — it does not make a value safe for
//! HTML, for SQL or for a shell, it restores the qualifier the value had before
//! `seal`. A program that sealed a `tainted` value gets a plain one back, which
//! is the one sharp edge of this design and is why `seal` is not reachable with
//! a `tainted` argument: the round trip cannot launder what it was never
//! allowed to seal.
//!
//! # `seal` and `open` here are not `Core\Crypto`'s, and R17 is why they can
//! share the names
//!
//! `rule:core-api/shape-rules` R17 refuses an
//! operation reachable two ways. These rows are not a second route to
//! `Core\Crypto::seal`: that member takes `bytes`, one key and a named cipher
//! and answers the raw sealed message, and these take a `string` and a *ring*,
//! seal under the one construction a cookie has ever used, and answer text a
//! `Set-Cookie` header can carry. Neither substitutes for the other, and a
//! program that writes the `Core\Crypto` pair plus a base64 call plus a loop
//! over keys has written this member badly rather than reached it twice. The
//! names are the same because the operation is the same one layer up, and
//! giving it a second vocabulary would be the actual confusion.
//!
//! # What a cookie is on the wire
//!
//! Unpadded URL-safe base64 (RFC 4648 § 5) of exactly the bytes
//! [`crate::crypto::seal_under`] produced. That alphabet is `A-Za-z0-9-_`,
//! every octet of which is an RFC 6265 `cookie-octet`, so the answer needs no
//! further escaping and no `=`. There is no version prefix, no key
//! identifier and no separator: a key hint would tell an attacker which key of
//! a rotating ring to attack, and every other field would be one more thing to
//! parse before the tag has been checked.
//!
//! The size is the plaintext's length plus 40 octets of AEAD overhead, then
//! base64's four-thirds — about `4/3 × (len + 40)` characters, which is the
//! number to hold against a browser's 4 KiB per-cookie limit.
//!
//! # One refusal, and it is constant-time underneath
//!
//! [`open`] answers the plaintext or throws, and **every way of not being an
//! authentic cookie is one `RuntimeError` with one sentence** — text that is
//! not base64, a payload too short to hold a nonce and a tag, one flipped bit,
//! and a cookie sealed under a key that has been retired past the end of the
//! ring. [`crate::crypto::open_under`]'s own docs are the home of why they are
//! indistinguishable; here there is a second reason, which is that a
//! distinguishable "wrong key" would say which key of the ring a forgery was
//! aimed at.
//!
//! `rule:security/algorithm-comes-from-the-key`'s constant-time rule is satisfied by the construction rather
//! than by anything in this module: Poly1305's tag comparison is the
//! `chacha20poly1305` crate's, done through `subtle`, and no member here
//! exposes a tag, a key or a raw sealed buffer for a caller to compare with
//! `==`. That is the whole of § 4's "no API exposes the raw value" for this
//! entry — the only thing a caller ever holds is the cookie text and the
//! plaintext.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use nvs_runtime::{Fault, NvsStr, Value};

use crate::keyring::KEY;
use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\SignedCookie";

/// `Core\SignedCookie::seal`, spelled the way a refusal names it.
const SEAL: &str = r"Core\SignedCookie::seal";

/// `Core\SignedCookie::open`, spelled the way a refusal names it.
const OPEN: &str = r"Core\SignedCookie::open";

/// `Core\SignedCookie`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Stores a value in a cookie. The browser keeps the cookie, but cannot read or change \
            the value. `seal` encrypts the value with a secret key, and `open` checks the cookie \
            and returns the value again.",
};

/// `rule:security/protocol-roster`'s first roster entry, as two rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "seal",
            names: &["value", "keys"],
            // The value is contagious — the cookie is made of it — and the
            // ring is neutral, because not one octet of a key reaches the
            // answer.
            params: &[CoreTy::Text(Qual::Contagious), CoreTy::Array(&KEY)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_signed_cookie_seal",
            doc: Some(&SEAL_DOC),
        },
        CoreMethod {
            name: "open",
            names: &["cookie", "keys"],
            // The cookie **launders**: the module doc's own section is the
            // home of why this one round trip may, when `rule:security/verification-does-not-launder` refuses
            // it for a verified signature everywhere else.
            params: &[CoreTy::Text(Qual::Launder), CoreTy::Array(&KEY)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_signed_cookie_open",
            doc: Some(&OPEN_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\SignedCookie::seal`'s reference card — `rule:core-api/reference-card`.
const SEAL_DOC: MethodDoc = MethodDoc {
    short: "Encrypts `$value` with the newest key in `$keys` and returns text for a cookie. \
            Nobody without the key can read or change the value. `Core\\SignedCookie::open` \
            returns the value again.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The value to store. `open` returns exactly this string.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The keys, newest first. `$keys[0]` encrypts the value. The older keys are \
                   there so that `open` still accepts cookies made before you added a new key. \
                   A list of one key is `[$key]`.",
            shape: &[],
        },
    ],
    ret: "The cookie text. It contains only `A-Z`, `a-z`, `0-9`, `-` and `_`, so a \
          `Set-Cookie` header can carry it without escaping. It is about 4/3 × (length + 40) \
          characters long. Each call returns different text for the same value, because each \
          call adds new random bytes.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or its first key is not 32 bytes long. \
                   `Core\\Crypto::generateKey()` returns a key of the right length.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The request does not have enough memory left for the cookie.",
        },
    ],
};

/// `Core\SignedCookie::open`'s reference card — `rule:core-api/reference-card`.
const OPEN_DOC: MethodDoc = MethodDoc {
    short: "Checks that `$cookie` was made by `Core\\SignedCookie::seal` with a key in `$keys`, \
            and returns the value stored in it. A changed cookie throws an error. The value is \
            returned without the `tainted` mark (the mark for input from outside), because \
            your own program stored it.",
    params: &[
        ParamDoc {
            name: "cookie",
            desc: "The cookie text as your program received it from the request. A `tainted` \
                   string is allowed here.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The keys you gave `seal`, newest first. A cookie made with any key in this \
                   list is accepted. A cookie made with a key you removed from the list is not.",
            shape: &[],
        },
    ],
    ret: "The value that was given to `seal`, exactly as it was.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or a key that `open` tries is not 32 bytes long. `open` \
                   tries the keys in order and stops at the first one that opens the cookie.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$cookie` was not made with any key in `$keys`. This includes a changed \
                   cookie, an empty cookie, text that is not a cookie, and a cookie made with a \
                   key you removed. All of these give the same message, so an attacker learns \
                   nothing from it.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_signed_cookie_seal" => (nvs_core_signed_cookie_seal as *const ()).cast(),
        "nvs_core_signed_cookie_open" => (nvs_core_signed_cookie_open as *const ()).cast(),
        _ => return None,
    })
}

/// The `string` in slot 0.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: slot 0 is a `string` in both rows, so
/// another tag is a compiled-code bug rather than anything a program can write.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `string`, got tag {}",
            args[0].tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\SignedCookie::seal(string $value, array<secret bytes> $keys): string`
    /// — the write half of `rule:security/protocol-roster`'s first entry, replacing the
    /// `hash_hmac` + `base64_encode` + `hash_equals` triple every PHP codebase
    /// grows its own slightly-different copy of.
    ///
    /// The newest key is the ring's first live slot, which for the list a
    /// program writes is `$keys[0]`; the module doc's own section is the home
    /// of why the newest end is the front and not the back.
    fn nvs_core_signed_cookie_seal(ctx, args: [2]) {
        let value = text_of(args, "seal")?;
        let ring = crate::keyring::borrow(args, 1, SEAL)?;

        let (slot, held) = crate::keyring::newest(&ring);
        let cipher = crate::keyring::cipher_at(&held, slot,SEAL)?;

        let sealed = crate::crypto::seal_under(ctx, &cipher, &[], value.as_bytes(), SEAL)?;
        Ok(Value::str(NvsStr::new(URL_SAFE_NO_PAD.encode(&sealed).as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\SignedCookie::open(string $cookie, array<secret bytes> $keys): string`
    /// — the read half, answering the sealed value **unqualified** per `rule:security/verification-does-not-launder`
    /// 's one named exception.
    ///
    /// Each key is keyed before the cookie is tried against it, so a ring
    /// entry that was never a key is a `LogicError` once the walk reaches it,
    /// whether the cookie decoded or not; an entry behind the key that opens
    /// the cookie is never reached. The decode is folded into the same one
    /// refusal as the tag check, so a cookie that is not base64 at all is not
    /// a distinguishable answer.
    ///
    /// The `string` the plaintext comes back as is safe to tag without
    /// re-validating: only this module's own `seal` produces bytes that
    /// authenticate, and it sealed a `string`. That is authenticity buying
    /// something concrete rather than being a formality.
    fn nvs_core_signed_cookie_open(_ctx, args: [2]) {
        let cookie = text_of(args, "open")?;
        let ring = crate::keyring::borrow(args, 1, OPEN)?;

        // A cookie that is not base64 is not authentic, and it is not a
        // *different* kind of not-authentic: the ring is still walked, so the
        // key-shape check below happens either way and the refusal is the one
        // sentence at the end.
        let sealed = URL_SAFE_NO_PAD.decode(cookie).ok();

        for (slot, held) in crate::keyring::entries(&ring) {
            let cipher = crate::keyring::cipher_at(&held, slot,OPEN)?;
            if let Some(sealed) = sealed.as_deref()
                && let Some(plain) = crate::crypto::open_under(&cipher, &[], sealed, OPEN)?
            {
                return Ok(Value::str(NvsStr::new(&plain)));
            }
        }

        Err(Fault::thrown(
            "Core\\SignedCookie::open(): $cookie is not an authentic cookie under any key in \
             $keys — it has been altered, it is not a cookie this application sealed, or the \
             key it was sealed under has been retired"
        ))
    }
}

#[cfg(test)]
mod tests {
    use chacha20poly1305::aead::Aead;
    use nvs_runtime::NvsArray;

    use super::*;

    /// Stage 4's cookie check — `rule:security/protocol-roster`'s first bullet and its M8
    /// verification line, which asks for three things in one breath: a round
    /// trip, a tampered cookie refused, and a cookie under a rotated-out key
    /// still opening while new ones use the newest.
    ///
    /// Asserted over the construction rather than through the registry, for
    /// [`crate::crypto`]'s reason: what could go wrong is the *layering* — the
    /// wrong end of the ring sealing, a decode that accepts a re-encoded
    /// forgery, a loop that stops at the first key — and every one of those is
    /// visible here without a compiler in front of it.
    #[test]
    fn a_signed_cookie_round_trips_and_a_tampered_one_is_refused() {
        let newest = crate::crypto::cipher(&[9_u8; 32]).expect("a 32-octet key keys");
        let retired = crate::crypto::cipher(&[4_u8; 32]).expect("a 32-octet key keys");

        // The round trip, through the same base64 spelling the member uses, so
        // an alphabet that drifted fails here rather than at a browser.
        let sealed = retired
            .encrypt(
                &chacha20poly1305::XNonce::from([1_u8; 24]),
                b"user=ada".as_ref(),
            )
            .expect("a short value seals");
        let mut wire = Vec::from([1_u8; 24]);
        wire.extend_from_slice(&sealed);
        let cookie = URL_SAFE_NO_PAD.encode(&wire);
        assert!(
            cookie
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "every octet of {cookie} is an RFC 6265 cookie-octet"
        );

        // A cookie under the *retired* key still opens — the ring is walked,
        // not just its head — while the newest key refuses it, which is what
        // makes the walk load-bearing rather than decoration.
        let raw = URL_SAFE_NO_PAD
            .decode(&cookie)
            .expect("its own encoding decodes");
        assert_eq!(
            crate::crypto::open_under(&retired, &[], &raw, "test")
                .expect("nothing is unaffordable here")
                .as_deref(),
            Some(b"user=ada".as_ref()),
            "a key still in the ring opens a cookie sealed under it"
        );
        assert!(
            crate::crypto::open_under(&newest, &[], &raw, "test")
                .expect("nothing is unaffordable here")
                .is_none(),
            "and the newest key alone would not have — so a ring of one is not this test"
        );

        // Tampering, at every position, including inside the nonce that
        // travels in the clear. A signed-but-not-encrypted cookie passes the
        // round trip above and fails here.
        for index in 0..raw.len() {
            let mut forged = raw.clone();
            forged[index] ^= 1;
            assert!(
                crate::crypto::open_under(&retired, &[], &forged, "test")
                    .expect("nothing is unaffordable here")
                    .is_none(),
                "one flipped bit at octet {index} of {} is refused",
                raw.len()
            );
        }

        // And the two shapes a forger reaches for before flipping a bit: a
        // truncated payload, and text that is not base64 at all.
        assert!(
            crate::crypto::open_under(&retired, &[], &raw[..raw.len() - 1], "test")
                .expect("nothing is unaffordable here")
                .is_none(),
            "a truncated cookie is refused"
        );
        assert!(
            URL_SAFE_NO_PAD.decode("not base64!").is_err(),
            "and text outside the alphabet never reaches the tag check at all"
        );
    }

    /// The ring's *order* is the whole of the rotation contract, and nothing a
    /// program can observe says which end is newest — so it is pinned here.
    ///
    /// `seal` takes `next_slot(0)`, which for the list a program writes is
    /// index 0. A member that took the last slot instead would round-trip
    /// perfectly, pass every assertion above, and quietly seal every new
    /// cookie under the oldest key an operator had been trying to retire.
    #[test]
    fn the_newest_key_is_the_rings_first_slot() {
        let mut ring = NvsArray::new();
        ring.append(Value::bytes(NvsStr::new(&[9_u8; 32])));
        ring.append(Value::bytes(NvsStr::new(&[4_u8; 32])));

        let newest = ring.next_slot(0).expect("two entries");
        assert_eq!(newest, 0, "the first live slot is the front of the list");
        let held = ring.value_at(newest).expect("a live slot holds a value");
        assert_eq!(
            held.as_bytes(),
            Some([9_u8; 32].as_ref()),
            "and it is the key written first, which is the one `seal` uses"
        );
    }

    /// Releases what a test built, so the crate's allocation gate sees a
    /// balanced run.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built"
        )]
        unsafe {
            value.release();
        }
    }

    /// The write half through the member itself: the cookie `seal` returns is
    /// cookie-safe text of the stated length, `open` returns the value under the
    /// same ring, and an empty ring is a `LogicError` before anything is sealed.
    // covers: Core\SignedCookie::seal
    #[test]
    fn seal_returns_cookie_text_open_reads_and_an_empty_ring_is_refused() {
        let keys = crate::keyring::tests::ring_of(&[&[9_u8; 32]]);
        let value = Value::str(NvsStr::new(b"user=ada"));

        let mut ctx = nvs_runtime::Ctx::buffered();
        let cookie = nvs_runtime::call(nvs_core_signed_cookie_seal, &mut ctx, &[value, keys])
            .expect("a ring of one well-formed key seals");
        let text = cookie.as_text().expect("seal returns a string").to_owned();
        assert!(
            text.bytes()
                .all(|octet| octet.is_ascii_alphanumeric() || octet == b'-' || octet == b'_'),
            "the cookie is unpadded URL-safe base64: {text}"
        );
        assert_eq!(
            text.len(),
            (8 + 40) * 4 / 3,
            "8 octets of value and 40 of overhead, in base64's four-thirds"
        );

        let back = nvs_runtime::call(nvs_core_signed_cookie_open, &mut ctx, &[cookie, keys])
            .expect("open reads what seal wrote");
        assert_eq!(
            back.as_text(),
            Some("user=ada"),
            "the value comes back as it went in"
        );
        dropped(back);
        dropped(cookie);

        let empty = crate::keyring::tests::ring_of(&[]);
        let refused = nvs_runtime::call(nvs_core_signed_cookie_seal, &mut ctx, &[value, empty]);
        let sentence = ctx.take_pending().map(std::borrow::Cow::into_owned);
        assert!(
            refused.is_err(),
            "an empty ring has no newest key to seal with"
        );
        assert!(
            sentence.is_some_and(|message| message.contains("$keys is empty")),
            "the refusal names the empty ring"
        );

        dropped(empty);
        dropped(value);
        dropped(keys);
    }

    /// The read half through the member itself: a cookie under an older key
    /// in the ring opens, and a changed cookie and a cookie under a key that
    /// left the ring are refused with one sentence.
    // covers: Core\SignedCookie::open
    #[test]
    fn open_walks_the_ring_and_refuses_a_forgery_and_a_retired_key_alike() {
        let old = crate::keyring::tests::ring_of(&[&[4_u8; 32]]);
        let rotated = crate::keyring::tests::ring_of(&[&[9_u8; 32], &[4_u8; 32]]);
        let retired = crate::keyring::tests::ring_of(&[&[9_u8; 32]]);
        let value = Value::str(NvsStr::new(b"cart=3"));

        let mut ctx = nvs_runtime::Ctx::buffered();
        let cookie = nvs_runtime::call(nvs_core_signed_cookie_seal, &mut ctx, &[value, old])
            .expect("a ring of one well-formed key seals");
        let back = nvs_runtime::call(nvs_core_signed_cookie_open, &mut ctx, &[cookie, rotated])
            .expect("the older key is still in the ring");
        assert_eq!(
            back.as_text(),
            Some("cart=3"),
            "the ring is walked past its head"
        );
        dropped(back);

        let mut forged = cookie.as_text().expect("seal returns a string").to_owned();
        let last = if forged.ends_with('A') { "B" } else { "A" };
        forged.replace_range(forged.len() - 1.., last);
        let forged = Value::str(NvsStr::new(forged.as_bytes()));

        let mut sentences = Vec::new();
        for (text, ring) in [(forged, rotated), (cookie, retired)] {
            let refused = nvs_runtime::call(nvs_core_signed_cookie_open, &mut ctx, &[text, ring]);
            assert!(
                refused.is_err(),
                "neither a forgery nor a retired key opens"
            );
            sentences.push(ctx.take_pending().map(std::borrow::Cow::into_owned));
        }
        assert!(
            sentences[0]
                .as_deref()
                .is_some_and(|message| message.contains("not an authentic cookie")),
            "the refusal says the cookie is not authentic: {sentences:?}"
        );
        assert_eq!(
            sentences[0], sentences[1],
            "and both refusals are one sentence"
        );

        dropped(forged);
        dropped(cookie);
        dropped(value);
        dropped(retired);
        dropped(rotated);
        dropped(old);
    }
}
