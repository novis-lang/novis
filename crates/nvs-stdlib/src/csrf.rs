//! `Core\Csrf` — `rule:security/protocol-roster`
//! 's second roster entry: a token bound to one session, and a comparison
//! that is the only thing a caller can do with it.
//!
//! `rule:security/protocol-roster` places the class; what belongs here is which
//! session a token is bound to, why there is no member answering the expected
//! token, and why the construction is one [`nvs_runtime::csrf`] owns rather than
//! an HMAC of this class's own.
//!
//! # The comparison is the only exposed operation
//!
//! `rule:security/protocol-roster` asks for the tokens to be "compared in constant time, with the
//! comparison being the only exposed operation so a caller cannot write `==`".
//! That is a statement about the *roster*, not about an implementation
//! detail: there are three members, [`token`] and [`issue`] answer a fresh
//! token and [`verify`] answers `bool`, and **nothing here answers the token a
//! request was supposed to carry**. A caller who wanted to write `==` would
//! first have to obtain the expected value, and no member hands one over.
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
//! # Which session a token is bound to
//!
//! [`token`] binds to the session the door will check, which is the identifier
//! the response's `[session] cookie` carries, and [`issue`] binds to a string
//! its caller names. The door reads that cookie and nothing else
//! (`rule:security/csrf-is-on-by-default`), so a page that bound a token to
//! anything other than the identifier the next request presents would render a
//! form its own door refuses. That is the mistake [`token`] makes impossible:
//! [`crate::session::sent_id`] gives it the identifier, drawing one and sending
//! its cookie for a session nobody has written yet, and marks the session
//! written so the write-back stores the row. It needs `Core\Session::start`
//! first, because `rule:core-classes/session-is-started-explicitly` declines a
//! hidden start, and it reads the key with [`nvs_config::http::csrf_key`], the
//! door's own reader, so no program decodes base64.
//!
//! [`issue`] stays for a program that binds to something else: an account
//! identifier plus a form name is a legitimate narrower binding, and a check it
//! makes with [`verify`] is its own rather than the door's.
//!
//! [`issue`] and [`verify`] take their text as `Qual::Neutral`, so a session identifier read
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
//! `Core\SignedCookie::open` cannot even open a token: a cookie is sealed
//! under additional data that names it, and a token is sealed under none, so
//! neither opens as the other under a shared key
//! (`rule:security/protocol-roster`).
//!
//! # Constant time
//!
//! The one comparison is `subtle::ConstantTimeEq` over the opened plaintext, on
//! [`crate::hash`]'s reasoning, which is that module's own doc, and it is
//! [`nvs_runtime::csrf::Key::verify`]'s. No member here exposes a tag, a key or
//! a raw sealed buffer, which is `rule:security/algorithm-comes-from-the-key`'s
//! "no API exposes the raw value" for this entry.

use nvs_runtime::csrf::{Key, NONCE_LEN};
use nvs_runtime::{Fault, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Csrf";

/// The key both rows take: the 32 octets `Core\Crypto::generateKey()` answers,
/// written once so neither row can drift from the other.
const KEY: CoreTy = crate::keyring::KEY;

/// `Core\Csrf`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Protects forms against cross-site request forgery, a request that another site sends \
            in the user's name. `token` creates a token for the visitor's session, which the \
            server checks on the next request. `issue` creates a token for a value you choose, \
            and `verify` checks one.",
};

/// `rule:security/protocol-roster`'s second roster entry, as two rows.
///
/// `rule:security/protocol-roster`'s second roster entry, and the third caller of the one
/// construction `Core\SignedCookie` uses — [`crate::csrf`] seals a domain tag and a session
/// identifier where the cookie seals a payload, so this crate still holds
/// one AEAD and not three. Its own module doc owns why the session arrives
/// as an argument, why there is no key ring here, and why a class whose
/// whole point is a missing accessor is not `rule:core-api/shape-rules` R17's "reachable two
/// ways" against `Core\SignedCookie`.
// rank: 960
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "token",
            names: &[],
            // No argument: the session and the key are the ones the door
            // checks, read here, so a page cannot bind a token to anything the
            // next request does not present.
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_csrf_token",
            doc: Some(&TOKEN_DOC),
        },
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

/// `Core\Csrf::token`'s reference card — `rule:core-api/reference-card`.
const TOKEN_DOC: MethodDoc = MethodDoc {
    short: "Returns a CSRF token for the visitor's session. Put it in a form field named \
            `_csrf`, in the `x-csrf-token` header or in a `_csrf` query parameter. The server \
            checks it on the next request. Call `Core\\Session::start()` first. If the visitor \
            has no session yet, this call creates one and sends its cookie.",
    params: &[],
    ret: "A different string on every call. Each one is valid for the whole session. It uses \
          only `A-Za-z0-9-_`, so a form field, a header and a URL carry it unchanged.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The request has not called `Core\\Session::start()`, or `[http] csrf_key` is \
                   not set.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "The call runs inside a function passed to `Core\\Task::afterResponse`, after \
                   the response was sent. The session cookie cannot be sent then.",
        },
    ],
};

/// `Core\Csrf::issue`'s reference card — `rule:core-api/reference-card`.
const ISSUE_DOC: MethodDoc = MethodDoc {
    short: "Returns a CSRF token bound to `$session` under `$key`. Send it in a form field \
            named `_csrf`, in the `x-csrf-token` header or in a `_csrf` query parameter. A form \
            that uploads files must use the header or the query parameter. With `[http] \
            csrf_key` set, the server checks the token before the handler runs. `verify` checks \
            it with the same session and key.",
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
        "nvs_core_csrf_token" => (nvs_core_csrf_token as *const ()).cast(),
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
        let nonce = drawn_nonce(ctx);
        let token = key.issue(&nonce, session, "Core\\Csrf::issue")?;
        Ok(Value::text(&token))
    }
}

/// The key `[http] csrf_key` names, read with [`nvs_config::http::csrf_key`],
/// the reader the door uses.
///
/// # Errors
///
/// A thrown `RuntimeError` naming `[http] csrf_key` where the tree names no key.
/// A tree naming one that is not a key never boots, so that case is this one.
fn configured_key(ctx: &nvs_runtime::Ctx, member: &str) -> Result<Key, Fault> {
    ctx.config()
        .and_then(|config| nvs_config::http::csrf_key(&config.snapshot().config))
        .as_deref()
        .and_then(Key::new)
        .ok_or_else(|| {
            Fault::thrown(format!(
                "{member}(): `[http] csrf_key` is not set. Set it to a key from \
                 `Core\\Crypto::generateKey()`, written as base64. The server checks tokens \
                 with the same key."
            ))
        })
}

/// A fresh nonce, drawn on the side of `crate::random`'s seam where a
/// `#[Test(seed: …)]` still reaches: one CSPRNG in the tree, and a sealed
/// message a seeded run reproduces.
fn drawn_nonce(ctx: &mut nvs_runtime::Ctx) -> [u8; NONCE_LEN] {
    let mut nonce = [0_u8; NONCE_LEN];
    crate::random::draw(ctx, |rng| {
        use rand::Rng as _;

        rng.fill_bytes(&mut nonce);
    });
    nonce
}

nvs_runtime::nvs_helper! {
    /// `Core\Csrf::token(): string` — a token for the session the response
    /// sends, under the key the door checks with. The module doc's *which
    /// session a token is bound to* section is the home of why it reads both.
    ///
    /// Nothing changes until both checks pass: a request with no session and a
    /// tree with no key each throw with no identifier drawn and no cookie sent.
    /// Only then does [`crate::session::sent_id`] draw an identifier, for a
    /// session that has none.
    fn nvs_core_csrf_token(ctx, _args: [0]) {
        const MEMBER: &str = "Core\\Csrf::token";
        if ctx.session().is_none() {
            // `sent_id`'s own refusal, asked before the key, because a program
            // that called neither fixes the missing `start` first.
            crate::session::sent_id(ctx, MEMBER)?;
        }
        let key = configured_key(ctx, MEMBER)?;
        let session = crate::session::sent_id(ctx, MEMBER)?;
        let nonce = drawn_nonce(ctx);
        let token = key.issue(&nonce, &session, MEMBER)?;
        Ok(Value::text(&token))
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
    use nvs_runtime::NvsStr;
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

    /// A context under a tree whose `[http] csrf_key` is `key`, written as base64
    /// the way an operator writes it.
    fn keyed_ctx(key: &[u8]) -> nvs_runtime::Ctx {
        use base64::Engine as _;

        let written = format!(
            "[http]\ncsrf_key = \"{}\"\n",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key)
        );
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_random_state(5);
        ctx.set_config(crate::tests::granting(&written));
        ctx
    }

    /// The session `Core\Session::start` opens for a visitor who sent no cookie:
    /// no identifier and an empty record.
    fn fresh_session(ctx: &mut nvs_runtime::Ctx) {
        ctx.open_session(nvs_runtime::Session {
            id: String::new(),
            record: Vec::new(),
            dirty: false,
            write_back: |_| {},
        });
    }

    /// `Core\Csrf::token` called the way a program calls it, with the token as
    /// text, or the thrown message.
    fn token_of(ctx: &mut nvs_runtime::Ctx) -> Result<String, String> {
        match nvs_runtime::call(nvs_core_csrf_token, ctx, &[]) {
            Ok(value) => {
                let text = value.as_text().expect("a token is a `string`").to_owned();
                released(value);
                Ok(text)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refusal is a throw a program can catch")
                .into_owned()),
        }
    }

    /// The door's two calls over a second request: the session is the value of
    /// the `nvsid` cookie, read with `Inbound::cookie` as the door and
    /// `Core\Session::start` both read it, and the token is verified against it.
    fn door_accepts(key: &[u8], cookie: &str, token: &str) -> bool {
        let mut inbound = nvs_runtime::Inbound::new("POST", "/", "");
        inbound.push_header("cookie", cookie.as_bytes());
        let session = inbound
            .cookie(b"nvsid")
            .and_then(|value| std::str::from_utf8(value).ok())
            .filter(|value| !value.is_empty());
        let key = Key::new(key).expect("a 32-octet key keys");
        session.is_some_and(|session| key.verify(token, session))
    }

    /// `Core\Csrf::token` on a visitor's first request: the token passes the
    /// door on the next request carrying the cookie the first one sent, and the
    /// same token sent with another session's cookie is refused. The call draws
    /// the identifier and sends the cookie, marks the session written so its row
    /// is stored, and a second call keeps the same identifier.
    // covers: Core\Csrf::token
    #[test]
    fn a_first_requests_token_passes_the_door_with_the_cookie_it_sent() {
        let key = [7_u8; KEY_LEN];
        let mut ctx = keyed_ctx(&key);
        fresh_session(&mut ctx);

        let token = token_of(&mut ctx).expect("a started session with a key gets a token");
        let headers = ctx.take_headers();
        assert_eq!(headers.len(), 1, "the first token sends one cookie");
        let open = ctx.session().expect("the session stays open");
        assert!(open.dirty, "a token is a write, so the row is stored");
        let id = open.id.clone();
        assert!(
            headers[0].value.starts_with(&format!("nvsid={id};")),
            "{:?} carries the identifier the token is bound to",
            headers[0].value
        );

        let again = token_of(&mut ctx).expect("a second token on one request");
        assert_ne!(token, again, "every token has its own nonce");
        assert!(
            ctx.take_headers().is_empty(),
            "the identifier is already sent, so no second cookie is"
        );
        assert_eq!(ctx.session().expect("the session stays open").id, id);

        let cookie = format!("theme=dark; nvsid={id}");
        assert!(door_accepts(&key, &cookie, &token));
        assert!(door_accepts(&key, &cookie, &again));
        assert!(
            !door_accepts(&key, "nvsid=AAAAAAAAAAAAAAAAAAAAAA", &token),
            "a token sent with another session's cookie is refused"
        );
        assert!(
            !door_accepts(&key, "theme=dark", &token),
            "a request with no session passes no token"
        );
    }

    /// `Core\Csrf::token` throws before anything changes: before
    /// `Core\Session::start` it names `start`, and with no `[http] csrf_key` it
    /// names that key. Neither draws an identifier or sends a cookie.
    // covers: Core\Csrf::token
    #[test]
    fn a_token_needs_a_started_session_and_a_configured_key() {
        let mut unstarted = keyed_ctx(&[7_u8; KEY_LEN]);
        let refused = token_of(&mut unstarted).expect_err("no session was started");
        assert!(refused.contains(r"`Core\Session::start()`"), "{refused}");
        assert!(refused.starts_with(r"Core\Csrf::token()"), "{refused}");
        assert!(unstarted.take_headers().is_empty());

        let mut unkeyed = nvs_runtime::Ctx::buffered();
        unkeyed.set_config(crate::tests::granting("[http]\n"));
        fresh_session(&mut unkeyed);
        let refused = token_of(&mut unkeyed).expect_err("no key is configured");
        assert!(refused.contains("`[http] csrf_key`"), "{refused}");
        assert!(unkeyed.take_headers().is_empty(), "no cookie was sent");
        let open = unkeyed.session().expect("the session stays open");
        assert!(
            open.id.is_empty() && !open.dirty,
            "the session is as `start` left it"
        );

        let mut neither = nvs_runtime::Ctx::buffered();
        let refused = token_of(&mut neither).expect_err("neither is there");
        assert!(
            refused.contains(r"`Core\Session::start()`"),
            "the missing `start` is named first: {refused}"
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
        let session = Value::text("sid-ada");
        let other = Value::text("sid-grace");

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
        let session = Value::text("sid-ada");
        let elsewhere = Value::text("sid-grace");

        let issued = |ctx: &mut nvs_runtime::Ctx, session: Value, key: Value| {
            nvs_runtime::call(nvs_core_csrf_issue, ctx, &[session, key])
                .expect("a short identifier seals")
        };
        let mine = issued(&mut ctx, session, key);
        let text = mine.as_text().expect("a token is a `string`").to_owned();

        let forgeries = [
            Value::text(""),
            Value::text("not a token at all !!"),
            Value::text(&text[..text.len() - 1]),
            Value::text(&format!("{text}A")),
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
