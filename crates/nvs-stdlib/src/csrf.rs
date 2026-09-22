//! `Core\Csrf` — `rule:security/protocol-roster`
//! 's second roster entry: a token bound to one session, and a comparison
//! that is the only thing a caller can do with it.
//!
//! `rule:security/protocol-roster` places the class; what belongs here is why the session arrives as
//! an argument, why there is no member answering the expected token, and why the
//! construction is one [`nvs_runtime::csrf`] owns rather than an HMAC of this
//! class's own.
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
//! # The construction is [`nvs_runtime::csrf`]'s, one crate below this one
//!
//! The format — the domain tag, the sealed binding, the nonce prefix, the
//! base64 and the constant-time comparison — is
//! [`nvs_runtime::csrf::Key`], and both members here are callers of it.
//! `rule:security/csrf-is-on-by-default` is why it is not written here: the
//! server door refuses an unsafe verb whose token does not verify, and
//! `nvs-server` cannot see this crate. One format under both readers is what
//! stops an application issuing tokens its own door refuses.
//!
//! That module's doc is the home of what a token is and why the tag is in the
//! plaintext. What stays here is the *class*: two members, no accessor for an
//! expected token, and the argument that a caller cannot get anywhere with `==`.
//!
//! A CSRF token is conventionally an HMAC, and that would be a second keyed
//! primitive in a tree whose whole argument for `Core\SignedCookie` was that one
//! AEAD serves everything. It is the same AEAD here, reached one crate down.
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
//! [`crate::hash`]'s reasoning, which is that module's own doc, and it is
//! [`nvs_runtime::csrf::Key::verify`]'s. No member here exposes a tag, a key or
//! a raw sealed buffer, which is `rule:security/algorithm-comes-from-the-key`'s
//! "no API exposes the raw value" for this entry.

use nvs_runtime::csrf::{Key, NONCE_LEN};
use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Csrf";

/// The key both rows take: the 32 octets `Core\Crypto::generateKey()` answers,
/// written once so neither row can drift from the other.
const KEY: CoreTy = CoreTy::SecretBlob(Qual::Neutral);

/// `rule:security/protocol-roster`'s second roster entry, as two rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
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
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$key` is not 32 octets long. A forged token is `false`, never a throw — \
                   only a program bug throws here.",
    }],
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

/// The token key made out of the `bytes` at `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `bytes`, and
/// [`crate::crypto::wrong_key_length`]'s shared `LogicError` for one that is a
/// `bytes` of the wrong length — reachable from source despite the parameter's
/// `secret bytes`, because the qualifier says nothing about length and
/// `Core\Random::bytes(8)` widens onto it. The sentence is `Core\Crypto`'s so
/// that every member of this tree taking a key says the same thing about one.
fn keyed(args: &[Value], slot: usize, member: &str) -> Result<Key, Fault> {
    let key = args[slot].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `bytes` for $key, got tag {}",
            args[slot].tag_byte()
        ))
    })?;
    Key::new(key).ok_or_else(|| {
        crate::crypto::wrong_key_length(&format!("{NAME}::{member}"), "$key", key.len())
    })
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
        let key = keyed(args, 1, "issue")?;

        // The nonce is drawn here rather than below, because this is the side of
        // `crate::random`'s seam where a `#[Test(seed: …)]` still reaches: one
        // CSPRNG in the tree, and a sealed message a seeded run reproduces.
        let mut nonce = [0_u8; NONCE_LEN];
        crate::random::draw(ctx, |rng| {
            use rand::Rng as _;

            rng.fill_bytes(&mut nonce);
        });
        let token = key.issue(&nonce, session, "Core\\Csrf::issue")?;
        Ok(Value::str(NvsStr::new(token.as_bytes())))
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
        let key = keyed(args, 2, "verify")?;

        Ok(Value::bool(key.verify(token, session)))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::csrf::KEY_LEN;

    use super::*;

    /// `rule:security/protocol-roster`'s second entry as *this crate* answers
    /// it: the token an application issues is the one every other reader of the
    /// format verifies, and the class hands out nothing else.
    ///
    /// The construction is asserted where it lives, in [`nvs_runtime::csrf`].
    /// What could go wrong *here* is the seam — a member that reached a
    /// different key, a different binding or a different alphabet than the door
    /// does — so what this pins is the round trip through the names these two
    /// rows call.
    #[test]
    fn the_token_the_class_issues_is_the_one_every_reader_verifies() {
        let key = Key::new(&[7_u8; KEY_LEN]).expect("a 32-octet key keys");
        let token = key
            .issue(&[5_u8; NONCE_LEN], "sid-ada", "Core\\Csrf::issue")
            .expect("a short identifier seals");

        assert!(
            token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "every octet of {token} goes into a hidden field unescaped"
        );
        assert!(key.verify(&token, "sid-ada"));
        assert!(
            !key.verify(&token, "sid-adam"),
            "a token is bound to one session, and an identifier that merely starts with \
             that one is another session"
        );
    }

    /// Drops the one reference this frame owns, exactly as a member's caller
    /// would.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the reference released here is the one the case took, and every \
                      other one is accounted for where it was taken"
        )]
        unsafe {
            value.release();
        }
    }

    /// `Core\Csrf::issue` as a program reaches it — through the symbol the
    /// registry row names rather than through [`Key`]. The session is read from
    /// slot 0 and the key from slot 1, the token it answers is `true` to this
    /// class's own `verify` for that session and `false` for another, and two
    /// calls for one session answer two different texts.
    ///
    /// The construction is pinned one crate below, in [`nvs_runtime::csrf`].
    /// What a member can get wrong is the seam — a slot read in the other
    /// order, a token whose reference the caller cannot hold, a refusal that
    /// leaves nothing pending — so both halves are driven the way a compiled
    /// call site drives them.
    // covers: Core\Csrf::issue
    #[test]
    fn the_member_issues_a_token_its_own_verify_accepts_and_never_repeats() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let key = Value::bytes(NvsStr::new(&[7_u8; KEY_LEN]));
        let session = Value::str(NvsStr::new(b"sid-ada"));
        let other = Value::str(NvsStr::new(b"sid-grace"));

        let first = nvs_runtime::call(nvs_core_csrf_issue, &mut ctx, &[session, key])
            .expect("a short identifier seals");
        let second = nvs_runtime::call(nvs_core_csrf_issue, &mut ctx, &[session, key])
            .expect("a short identifier seals");
        assert_ne!(
            first.as_text(),
            second.as_text(),
            "each token is sealed under its own nonce, so a caller who compares two of \
             them with `==` gets `false` from a pair that both verify"
        );

        for token in [first, second] {
            let verdict = |ctx: &mut nvs_runtime::Ctx, against: Value| {
                nvs_runtime::call(nvs_core_csrf_verify, ctx, &[token, against, key])
                    .expect("a verdict answers")
                    .as_bool()
            };
            assert_eq!(verdict(&mut ctx, session), Some(true));
            assert_eq!(
                verdict(&mut ctx, other),
                Some(false),
                "the binding is what the entry is, and a forgery is an answer rather \
                 than a throw"
            );
            released(token);
        }

        // A `bytes` that was never a key is a program bug, and the only thing
        // this member throws for.
        let short = Value::bytes(NvsStr::new(&[7_u8; KEY_LEN - 1]));
        nvs_runtime::call(nvs_core_csrf_issue, &mut ctx, &[session, short])
            .expect_err("a `bytes` of 31 octets is not a key");
        assert!(
            ctx.take_pending().is_some(),
            "the refusal is a throw a program can catch, carrying `Core\\Crypto`'s own \
             sentence about a key's length"
        );

        released(short);
        released(other);
        released(session);
        released(key);
    }

    /// `Core\Csrf::verify` answers **one** thing for every way of not being
    /// this session's token, counted over the whole table rather than read off
    /// a row: a member that told a decode failure apart from a failed tag would
    /// answer plausibly line by line and still be a forger's oracle. The rows
    /// are the halves a forgery can get wrong — the alphabet, the length, the
    /// key and the binding — and the real token beside them is what makes the
    /// count mean something, since a member answering `false` to everything
    /// passes the first half alone.
    ///
    /// A `$key` that was never a key is the one throw, and it is a program bug
    /// rather than a verdict, so a CSRF check needs no `try` around it.
    // covers: Core\Csrf::verify
    #[test]
    fn every_way_of_not_being_this_sessions_token_is_one_answer() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let key = Value::bytes(NvsStr::new(&[7_u8; KEY_LEN]));
        let retired = Value::bytes(NvsStr::new(&[3_u8; KEY_LEN]));
        let session = Value::str(NvsStr::new(b"sid-ada"));
        let elsewhere = Value::str(NvsStr::new(b"sid-grace"));

        let issued = |ctx: &mut nvs_runtime::Ctx, session: Value, key: Value| {
            nvs_runtime::call(nvs_core_csrf_issue, ctx, &[session, key])
                .expect("a short identifier seals")
        };
        let mine = issued(&mut ctx, session, key);
        let text = mine.as_text().expect("a token is a `string`").to_owned();

        let forgeries = [
            Value::str(NvsStr::new(b"")),
            Value::str(NvsStr::new(b"not a token at all !!")),
            Value::str(NvsStr::new(&text.as_bytes()[..text.len() - 1])),
            Value::str(NvsStr::new(format!("{text}A").as_bytes())),
            issued(&mut ctx, elsewhere, key),
            issued(&mut ctx, session, retired),
        ];

        let verdict = |ctx: &mut nvs_runtime::Ctx, token: Value| {
            nvs_runtime::call(nvs_core_csrf_verify, ctx, &[token, session, key])
                .expect("a verdict answers")
                .as_bool()
        };
        let mut refused = 0_usize;
        for forgery in forgeries {
            if verdict(&mut ctx, forgery) == Some(false) {
                refused += 1;
            }
            released(forgery);
        }
        assert_eq!(refused, forgeries.len());
        assert_eq!(
            verdict(&mut ctx, mine),
            Some(true),
            "the token this key issued for this session is the one thing that is accepted"
        );

        let short = Value::bytes(NvsStr::new(&[7_u8; KEY_LEN - 1]));
        nvs_runtime::call(nvs_core_csrf_verify, &mut ctx, &[mine, session, short])
            .expect_err("a `bytes` of 31 octets is not a key");
        assert!(ctx.take_pending().is_some());

        released(short);
        released(mine);
        released(elsewhere);
        released(session);
        released(retired);
        released(key);
    }
}
