//! `Core\Csrf` — `rule:security/protocol-roster`
//! 's second roster entry: a token bound to one session, and a comparison
//! that is the only thing a caller can do with it.
//!
//! `rule:security/protocol-roster` places the class; what belongs here is why the session arrives as
//! an argument, why there is no member answering the expected token, why the
//! construction is [`crate::crypto`]'s third caller rather than an HMAC of its
//! own, and what the domain tag in the plaintext is for.
//!
//! # The comparison is the only exposed operation
//!
//! `rule:security/protocol-roster` asks for the tokens to be "compared in constant time, with the
//! comparison being the only exposed operation so a caller cannot write `==`".
//! That is a statement about the *roster*, not about an implementation
//! detail: there are two members, [`issue`] answers a token and [`verify`]
//! answers `bool`, and **nothing here answers the token a request was supposed
//! to carry**. A caller who wanted to write `==` would first have to obtain the
//! expected value, and no member hands one over.
//!
//! The construction makes that stronger than a missing accessor. Every token is
//! sealed under its own nonce, so two calls to [`issue`] for the same session
//! answer two different strings, and a caller who nevertheless compares one
//! token with another gets `false` from a pair that are both valid. The failure
//! mode `rule:security/algorithm-comes-from-the-key` wants unrepresentable is not merely undocumented here; it
//! visibly does not work.
//!
//! [`verify`] answers `false` for every way of not being this session's token —
//! an altered token, a token for another session, a token under a retired key,
//! and text that is not base64 at all. It throws only for a program bug (a
//! `$key` that was never a key). A forgery is not exceptional: it is the
//! ordinary answer to the question the member asks, and a member that threw
//! would put every CSRF check inside a `try` whose `catch` is the interesting
//! branch.
//!
//! # The session arrives as an argument
//!
//! `Core\Session` is not in this goal, so the identifier the token is bound to
//! is passed in rather than read. That is not a placeholder for a later
//! `Core\Csrf::issue()` with no arguments: binding is the whole content of the
//! entry, and a member that read an ambient session would be one that could
//! not be told what it bound to. Any stable per-session string does —
//! a session identifier is the obvious one, and an account identifier plus a
//! form name is a legitimate narrower binding.
//!
//! Both members take it as `Qual::Neutral`, so a session identifier read
//! straight out of a `tainted` cookie is accepted and the token is not itself
//! `tainted`. The answer's alphabet is base64's, which carries no injection
//! into any sink, exactly as a hash of a `secret` is not itself `secret` — this
//! is [`Qual::Neutral`]'s own "a hash of a secret" case rather than a hole in
//! `rule:security/tainted-qualifier`.
//!
//! # One key, and why no ring
//!
//! [`crate::signed_cookie`] takes a key *ring* because `rule:security/protocol-roster` asks for
//! rotation there; this entry's bullet does not, and the difference is what a
//! rotation costs. A cookie outlives a deploy — rotating without a ring logs
//! everyone out. A CSRF token outlives one rendered form, so rotating the key
//! costs at most one refused submission and a re-render, which is the same
//! thing a session timeout already does and which every application already
//! handles. One key keeps the surface at two arguments, and an operator who
//! wants a seamless rotation rotates the session key, not this one.
//!
//! # The construction is `Core\Crypto`'s, with a domain tag inside it
//!
//! A CSRF token is conventionally an HMAC, and that would be a second keyed
//! primitive in a crate whose whole argument for `Core\SignedCookie` was that
//! there is one AEAD in `nvs-stdlib` and everything is on the near side of it.
//! So this is [`crate::crypto::seal_under`] again: the sealed plaintext is
//! [`DOMAIN`] followed by the session identifier, and [`verify`] rebuilds
//! exactly that and compares it to what opened.
//!
//! [`DOMAIN`] is the part that is not decoration. A program will reasonably use
//! one `Core\Crypto::generateKey()` for its cookies and its CSRF tokens, and
//! without a tag in the plaintext a signed cookie carrying a session identifier
//! would *be* a valid CSRF token for that session, and the reverse. The tag
//! carries a version because a later change to what is bound must refuse
//! yesterday's tokens rather than accept them under a new reading. **What this
//! spends** is [`DOMAIN`]'s octets on every token — twelve, inside base64's
//! four-thirds — for a cross-protocol confusion that is otherwise a real
//! deployment away.
//!
//! `rule:core-api/shape-rules` R17 asks
//! whether this is `Core\SignedCookie` reached twice, and it is not: that class
//! answers *the payload* and this one answers a verdict it never lets go of.
//! A program that wrote `Core\SignedCookie::open($token, [$key]) == $session`
//! has written the comparison itself — in variable time, with `==`, and inside
//! a `try` because a forgery throws there — which is precisely the code ADR
//! 0060 § 1 exists to make unnecessary, not a second route to this member.
//!
//! # Constant time
//!
//! The one comparison is `subtle::ConstantTimeEq` over the opened plaintext, on
//! [`crate::hash`]'s reasoning, which is that module's own doc. Poly1305's tag
//! comparison underneath is the `chacha20poly1305` crate's, also through
//! `subtle`. No member here exposes a tag, a key or a raw sealed buffer, which
//! is `rule:security/algorithm-comes-from-the-key`'s "no API exposes the raw value" for this entry.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use subtle::ConstantTimeEq as _;

use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Csrf";

/// What every token's plaintext begins with, so a sealed value produced for
/// another purpose under the same key is not one of these.
///
/// The trailing NUL is what keeps the tag a *prefix* rather than the start of
/// the identifier: without it, a session named `1x` under version `v1` and a
/// session named `x` under a hypothetical version `v11` would seal the same
/// bytes. The `1` is the binding's version, and changing what is bound changes
/// it.
const DOMAIN: &[u8] = b"nvs.csrf.v1\0";

/// The key both rows take: the 32 octets `Core\Crypto::generateKey()` answers,
/// written once so neither row can drift from the other.
const KEY: CoreTy = CoreTy::SecretBlob(Qual::Neutral);

/// `rule:security/protocol-roster`'s second roster entry, as two rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "issue",
            names: &["session", "key"],
            // Both neutral: the answer is base64 of a sealed buffer, so it
            // carries neither the session's `tainted` nor the key's `secret`.
            // The module doc's *the session arrives as an argument* section is
            // the home of why that is sound rather than convenient.
            params: &[CoreTy::Text(Qual::Neutral), KEY],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_csrf_issue",
            doc: Some(&ISSUE_DOC),
        },
        CoreMethod {
            name: "verify",
            names: &["token", "session", "key"],
            // The answer is a `bool`, so there is nothing for any of the three
            // to qualify — and the token is expected to arrive `tainted`,
            // because it came out of a request body.
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
                KEY,
            ],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_csrf_verify",
            doc: Some(&VERIFY_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Csrf::issue`'s reference card — `rule:core-api/reference-card`.
const ISSUE_DOC: MethodDoc = MethodDoc {
    short: "Answers a CSRF token bound to `$session` under `$key`. Put it in the form or the \
            header the next request will carry, and hand it back to `verify` with the same \
            session and key.",
    params: &[
        ParamDoc {
            name: "session",
            desc: "What the token is bound to — a session identifier, or anything else stable \
                   for as long as the token should be accepted. A token issued against one \
                   value never verifies against another.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "A 32-octet key, as `Core\\Crypto::generateKey()` answers. It never reaches \
                   the token, so the same key serves every session.",
            shape: &[],
        },
    ],
    ret: "Unpadded URL-safe base64 — `A-Za-z0-9-_`, which a hidden field, a header and a query \
          string all carry unescaped. Different on every call for the same inputs, so two \
          tokens for one session never compare equal and a caller cannot get anywhere with \
          `==`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` is not 32 octets long — a `bytes` that was never a key.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "This process cannot spare a buffer the size of the token.",
        },
    ],
};

/// `Core\Csrf::verify`'s reference card — `rule:core-api/reference-card`.
const VERIFY_DOC: MethodDoc = MethodDoc {
    short: "Reports whether `$token` is a token this application issued for `$session` under \
            `$key`. This is the only comparison the class exposes: no member answers the token \
            that was expected, so there is nothing to write `==` against.",
    params: &[
        ParamDoc {
            name: "token",
            desc: "The token as the request carried it. A `tainted` value is expected here — \
                   that is where a token comes from.",
            shape: &[],
        },
        ParamDoc {
            name: "session",
            desc: "The value `issue` was given. A token for a different session answers \
                   `false`, which is what \"bound to the session\" means.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The key `issue` was given. Rotating it refuses every outstanding token.",
            shape: &[],
        },
    ],
    ret: "`true` for a token this key issued against this session, `false` for every other \
          text — altered, expired out of the key, issued for another session, or not base64 at \
          all. The comparison is constant-time, and the four cases are one answer so that a \
          forger learns nothing about which half landed.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` is not 32 octets long. A forged token is `false`, never a throw — \
                   only a program bug throws here.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "This process cannot spare the buffer the token would open into.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_csrf_issue" => (nvs_core_csrf_issue as *const ()).cast(),
        "nvs_core_csrf_verify" => (nvs_core_csrf_verify as *const ()).cast(),
        _ => return None,
    })
}

/// The `string` at `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: every text slot in both rows is a
/// `string`, so another tag is a compiled-code bug rather than anything a
/// program can write.
fn text_at<'a>(
    args: &'a [Value],
    slot: usize,
    member: &str,
    param: &str,
) -> Result<&'a str, Fault> {
    args[slot].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `string` for {param}, got tag {}",
            args[slot].tag_byte()
        ))
    })
}

/// The cipher keyed by the `bytes` at `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `bytes`, and
/// [`crate::crypto::wrong_key_length`]'s shared `LogicError` for one that is a
/// `bytes` of the wrong length — reachable from source despite the parameter's
/// `secret bytes`, because the qualifier says nothing about length and
/// `Core\Random::bytes(8)` widens onto it.
fn keyed(
    args: &[Value],
    slot: usize,
    member: &str,
) -> Result<chacha20poly1305::XChaCha20Poly1305, Fault> {
    let key = args[slot].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `bytes` for $key, got tag {}",
            args[slot].tag_byte()
        ))
    })?;
    crate::crypto::cipher(key).ok_or_else(|| {
        crate::crypto::wrong_key_length(&format!("{NAME}::{member}"), "$key", key.len())
    })
}

/// What a token for `session` seals: the domain tag, then the identifier.
///
/// One function so [`nvs_core_csrf_issue`] and [`nvs_core_csrf_verify`] cannot
/// disagree about the binding — the whole entry is that they do not.
fn bound(session: &str) -> Vec<u8> {
    let mut plain = Vec::with_capacity(DOMAIN.len() + session.len());
    plain.extend_from_slice(DOMAIN);
    plain.extend_from_slice(session.as_bytes());
    plain
}

nvs_runtime::nvs_helper! {
    /// `Core\Csrf::issue(string $session, secret bytes $key): string` — the
    /// write half of `rule:security/protocol-roster`'s second entry, replacing the
    /// `random_bytes` + `$_SESSION['token']` + `hash_equals` triple every PHP
    /// codebase grows its own slightly-different copy of.
    ///
    /// Stateless on purpose: the binding is inside the token, so nothing has to
    /// be stored beside the session and a second tab does not invalidate the
    /// first one's form. The module doc's own section is the home of why the
    /// session is an argument.
    fn nvs_core_csrf_issue(ctx, args: [2]) {
        let session = text_at(args, 0, "issue", "$session")?;
        let cipher = keyed(args, 1, "issue")?;

        let sealed =
            crate::crypto::seal_under(ctx, &cipher, &[], &bound(session), "Core\\Csrf::issue")?;
        Ok(Value::str(NvsStr::new(URL_SAFE_NO_PAD.encode(&sealed).as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Csrf::verify(string $token, string $session, secret bytes $key): bool`
    /// — the read half, and the only comparison this class exposes.
    ///
    /// `false` for every way of not being this session's token, including text
    /// that is not base64: a decode failure is folded into the same answer as a
    /// failed tag check, so the member is not a base64 validator a forger can
    /// question separately. Only a `$key` that was never a key throws.
    ///
    /// The comparison is `subtle`'s over the whole plaintext rather than a
    /// prefix strip and a `==` on the tail, so a token whose domain tag is
    /// wrong and a token whose session is wrong take the same path as well as
    /// the same time.
    fn nvs_core_csrf_verify(_ctx, args: [3]) {
        let token = text_at(args, 0, "verify", "$token")?;
        let session = text_at(args, 1, "verify", "$session")?;
        let cipher = keyed(args, 2, "verify")?;

        let Ok(sealed) = URL_SAFE_NO_PAD.decode(token) else {
            return Ok(Value::bool(false));
        };
        let Some(plain) = crate::crypto::open_under(&cipher, &[], &sealed, "Core\\Csrf::verify")?
        else {
            return Ok(Value::bool(false));
        };

        Ok(Value::bool(bool::from(plain.ct_eq(&bound(session)))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stage 4's CSRF check — `rule:security/protocol-roster`'s second bullet, which asks for
    /// three things in one breath: a token bound to the session that issued it,
    /// a comparison that is the only exposed operation, and no way for a caller
    /// to hold the expected value.
    ///
    /// Asserted over the construction rather than through the registry, for
    /// [`crate::crypto`]'s reason: what could go wrong is the *binding* — a
    /// token that verifies against any session, a domain tag that is not in the
    /// plaintext, a comparison on a prefix — and every one of those is visible
    /// here without a compiler in front of it.
    #[test]
    fn a_csrf_token_is_bound_to_the_session_that_issued_it() {
        let key = crate::crypto::cipher(&[7_u8; 32]).expect("a 32-octet key keys");
        let other = crate::crypto::cipher(&[3_u8; 32]).expect("a 32-octet key keys");

        // A token is the sealed binding, and nothing else: what `issue` writes
        // is what `verify` rebuilds, so the two are pinned to one function.
        let plain = bound("sid-ada");
        assert!(
            plain.starts_with(DOMAIN),
            "the domain tag is in the plaintext, so a signed cookie of the same session \
             identifier under the same key is not a token"
        );

        // The round trip, through the member's own spelling of the wire form.
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let sealed = crate::crypto::seal_under(&mut ctx, &key, &[], &plain, "test")
            .expect("a short value seals");
        let token = URL_SAFE_NO_PAD.encode(&sealed);
        assert!(
            token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "every octet of {token} goes into a hidden field unescaped"
        );

        // Bound to *this* session: the same token against a neighbouring
        // identifier is refused, and a prefix of one is not a match either — a
        // comparison written as `starts_with` would pass the first and fail
        // here.
        let opened = crate::crypto::open_under(&key, &[], &sealed, "test")
            .expect("the buffer is affordable")
            .expect("its own key opens it");
        for (session, want) in [
            ("sid-ada", true),
            ("sid-ad", false),
            ("sid-adam", false),
            ("", false),
        ] {
            assert_eq!(
                bool::from(opened.ct_eq(&bound(session))),
                want,
                "a token for `sid-ada` against `{session}`"
            );
        }

        // A key that was rotated out refuses the token outright, which is the
        // member's `false` rather than a distinguishable answer.
        assert!(
            crate::crypto::open_under(&other, &[], &sealed, "test")
                .expect("the buffer is affordable")
                .is_none(),
            "a token under one key does not open under another"
        );

        // And two tokens for one session differ, so a caller who compares one
        // token with another gets `false` from a pair that are both valid.
        // This is what makes the missing accessor load-bearing rather than
        // decorative.
        let again = crate::crypto::seal_under(&mut ctx, &key, &[], &plain, "test")
            .expect("a short value seals");
        assert_ne!(
            token,
            URL_SAFE_NO_PAD.encode(&again),
            "each token seals under its own nonce, so `==` between two tokens is useless"
        );
    }
}
