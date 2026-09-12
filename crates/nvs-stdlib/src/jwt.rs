//! `Core\Jwt` — `rule:security/protocol-roster`
//! 's fourth roster entry, and the one whose historical failures are all
//! failures of *choice*: an algorithm chosen by the token, an expiry chosen by
//! a flag, a verdict chosen by a falsy return.
//!
//! `rule:security/protocol-roster` places the class and § 4 states the three rules. What belongs here
//! is why each of them ends up as a property of a signature rather than as a
//! check inside a body, what a claim is on the way in and on the way out, and
//! what this entry deliberately refuses to carry.
//!
//! # The algorithm comes from the key, and the key is the only thing that names it
//!
//! `rule:security/algorithm-comes-from-the-key` asks that `alg` be "checked against the key's
//! algorithm and rejected on mismatch; never consulted to select one". This
//! class holds it by having no `alg` parameter at all: the value in
//! [`nvs_core_jwt_sign`]'s `$key` slot **is** the choice, and there is no
//! second argument that could disagree with it. A `secret bytes` is
//! HMAC-SHA-256 through [`crate::hash::hmac_sha256`], under the key
//! `Core\Crypto::generateKey()` already answers; a `Core\Crypto\KeyPair` is
//! the one JWS algorithm its kind can carry, which [`pair_alg`] is the whole
//! of — a total function of a kind settled when the pair was read, rather
//! than a lookup a call site steers.
//!
//! [`nvs_core_jwt_verify`] reads a header's `alg` only to **compare** it
//! against [`ALG`], and the comparison's failure is a refusal. There is no
//! code path in which a string out of a token reaches a `match`, so `alg:
//! none` and the RS256→HS256 confusion are not defended against here, they
//! are unwritable. That member takes a shared key alone, so a token signed
//! under a pair is not one it reads at all: the asymmetric half is for tokens
//! another party verifies.
//!
//! That is also why no member takes a `Digest`. `Core\Hash::hmac` takes one
//! because a program choosing a digest for its own protocol is choosing
//! nothing an attacker supplied; a JWT's algorithm field is attacker-supplied
//! by construction, and a member that accepted the caller's choice would have
//! put the same string back within one call site of the token it came from.
//!
//! # The header is written, sorted, and four members wide at most
//!
//! [`header_of`] writes `{alg, jwk?, kid?, typ}` — members sorted, no
//! whitespace — so a token this class signs is reproducible byte for byte
//! wherever the signature algorithm is itself deterministic, and can be held
//! to a frozen interoperability vector rather than only round-tripped against
//! this module. [`HEADER`] is what that function answers for a shared key and
//! an empty bag, written out so the bytes this class signs are visible in the
//! file, and `the_default_header_is_the_written_constant` holds the two
//! together.
//!
//! The trailing bag is the whole of what a caller may put in a header. `kid`
//! names the key for a recipient holding several, `typ` is the member RFC 9068
//! spells `at+jwt`, and `embedKey` writes the pair's public half as RFC 7638's
//! minimal JWK under `jwk` — what a DPoP proof carries. There is no option for
//! `alg`, none for a header member the two halves of the roster do not read,
//! and `embedKey` under a shared key is a `LogicError`: a shared secret has no
//! public half, and a member that wrote one anyway would be inventing exactly
//! the key-in-the-token shape `rule:security/algorithm-comes-from-the-key` removes.
//!
//! # Expiry is not optional at either end
//!
//! § 4's second bullet makes a token without `exp`, or past it, fail
//! verification with no flag to disable the check. This class holds it twice.
//! [`nvs_core_jwt_verify`] refuses a payload carrying no `exp`, and
//! [`nvs_core_jwt_sign`] takes the lifetime as a **positional `Duration`** and
//! writes `exp` itself — so a caller cannot forget it, cannot pass it as an
//! option they leave out, and cannot supply their own: `exp` and `iat` in
//! `$claims` are a `LogicError` naming the parameter that owns them. A token
//! this class signs is one this class verifies, which is not true of a design
//! where the caller assembles the registered claims.
//!
//! `nbf` is not written and not checked. It is a *third* clock rule to reason
//! about for the one case `exp` does not already cover — a token minted early
//! — and `rule:security/algorithm-comes-from-the-key` names only expiry. A program that needs it can put a
//! claim of its own in and compare it.
//!
//! # Verification returns claims or throws
//!
//! [`crate::csrf::nvs_core_csrf_verify`] answers `bool`, and this member
//! throws. The two are not inconsistent: a CSRF check asks a question whose
//! interesting answer is `false` on an ordinary request, while a JWT
//! verification's only useful continuation is *with the claims*, and a member
//! answering `?array` would put the entire failure surface behind a `??` that
//! a loose comparison can flatten. § 4's third bullet says so directly, and it
//! is the same reasoning `rule:core-classes/regex-two-tiers` applies to a budget exhaustion.
//!
//! Every way of not being a token this key signed is **one sentence**: three
//! dot-separated parts or not, a header that is not JSON, an `alg` that is not
//! ours, a signature under another key, a tampered payload. A forger learns
//! nothing from which half failed. Expiry is the deliberate exception, and it
//! is safe to distinguish precisely because it is checked *after* the
//! signature: only the holder of a genuinely signed token ever sees it, and
//! telling them to log in again rather than that they are being attacked is
//! the whole reason an application catches this at all.
//!
//! # A claim is text, and that is what this entry spends
//!
//! [`nvs_core_jwt_verify`] answers `array<tainted string>` — `rule:security/verification-does-not-launder`'s
//! qualifier, spelled in the row itself as
//! [`crate::registry::CoreTy::TaintedStr`] rather than left to a
//! `Qual::Contagious` that would only have tainted the claims when the *token*
//! was already tainted. Whether the token was written as a literal has nothing
//! to do with whether the issuer is trusted, which is § 5's point.
//!
//! The qualifier is what fixes the element type. `nvs_types` defines
//! `tainted` over `string` and `bytes` and over nothing else, so there is no
//! `tainted array<mixed>` to answer with; an `array<mixed>` of claims would
//! hand the program a value that had visibly been verified and invisibly been
//! laundered. So every claim comes back as text: a JSON string as itself, a
//! number in its own spelling, `true`/`false` as those words. **A claim whose
//! value is `null`, an object or an array is refused**, per
//! `rule:errors/ambiguous-input-refused`
//! — rendering a nested object as its JSON text would invent a spelling
//! nothing else in this crate reads back, and `null` and `""` have no honest
//! distinction once both are text.
//!
//! **What that spends** is interoperability with issuers whose tokens carry
//! structured claims — a directory server's role table is the usual one. It
//! buys a surface on which a claim cannot reach a sink unlaundered, which is
//! the priority-1 half of the trade and so the one that wins here. The
//! widening is a qualifier that survives a shape: a decoder carrying `tainted`
//! through into a declared shape lets a member answer that shape and keep
//! `rule:security/verification-does-not-launder`, which is the ground
//! `Core\Json::decodeAs<T>` already stands on and which a reader of a
//! third-party token is written onto rather than this member.
//!
//! [`nvs_core_jwt_sign`] takes `array<string>` for the same reason from the
//! other side: the two halves agree about what a claim is, so a token this
//! class signs under a shared key never trips the refusal above. Under a key
//! pair the token is for somebody else's verifier, and what that verifier
//! accepts is not this module's to promise — the claims are still
//! `array<string>`, so the agreement holds in the one direction it is about.
//!
//! # Constant time
//!
//! The signature comparison is `subtle::ConstantTimeEq` over the whole tag, on
//! [`crate::hash`]'s reasoning, which is that module's own doc. No member here
//! hands back a tag, a key or a signing input, which is `rule:security/algorithm-comes-from-the-key`'s "no API
//! exposes the raw value for the caller to compare themselves" for this entry.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use subtle::ConstantTimeEq as _;

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::crypto::{self, KeyFormat, KeyKind, PrivateKey};
use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Jwt";

/// The one algorithm, in JWS's own spelling.
///
/// A constant rather than a match arm because it is only ever *compared*: the
/// module doc's own section is the home of why a `match` over this field is
/// the thing `rule:security/algorithm-comes-from-the-key` exists to prevent.
const ALG: &str = "HS256";

/// The JWS algorithm a pair of each kind carries, and `None` for the one kind
/// that signs nothing.
///
/// A function of the kind alone, which is the kind the pair was *read* as: the
/// module doc's first section is the home of why that is the whole of
/// `rule:security/algorithm-comes-from-the-key` for the asymmetric half. RSA's
/// two spellings are two kinds rather than one kind and a scheme argument, for
/// the reason [`crate::crypto::KeyKind`] gives.
const fn pair_alg(kind: KeyKind) -> Option<&'static str> {
    match kind {
        KeyKind::P256 => Some("ES256"),
        KeyKind::Ed25519 => Some("EdDSA"),
        KeyKind::RsaPkcs1 => Some("RS256"),
        KeyKind::RsaPss => Some("PS256"),
        KeyKind::X25519 => None,
    }
}

/// The `typ` a token carries unless the bag names another.
///
/// Written because RFC 7519 § 5.1 recommends it, and **not** checked on the way
/// in, because RFC 9068's access tokens spell it `at+jwt` and refusing those
/// would be this class inventing a rule `rule:security/protocol-roster` does not have.
const TYP: &str = "JWT";

/// The header a token signed under a shared key with an empty bag carries, byte
/// for byte.
///
/// Written out rather than assembled, so the bytes that get signed and the
/// bytes a reader of this file sees are the same bytes. [`header_of`] is what
/// actually writes one, and `the_default_header_is_the_written_constant` holds
/// this spelling and that function's answer together.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the written spelling is the pin, and the pin is what the test reads"
    )
)]
const HEADER: &str = r#"{"alg":"HS256","typ":"JWT"}"#;

/// The shortest key HS256 accepts — RFC 7518 § 3.2's "a key of the same size
/// as the hash output", which is 256 bits.
///
/// A *minimum* rather than [`crate::crypto`]'s exact length, and that is the
/// one place this class parts company with the rest of the roster. Every other
/// entry keys a construction of ours, so it may insist on the 32 octets
/// `Core\Crypto::generateKey()` answers; a JWT is an interchange format, and
/// the shared secret on the other side of it belongs to a service that was
/// never going to ask us what length to make it.
const MIN_KEY_LEN: usize = 32;

/// The key `verify` takes: a shared secret and nothing else.
const KEY: CoreTy = CoreTy::SecretBlob(Qual::Neutral);

/// The key `sign` takes, which is the whole of its algorithm choice.
///
/// The secret arm is written [`CoreTy::SecretBlob`] rather than
/// `CoreTy::SecretBytes` because the registry's unclassified-parameter audit
/// walks a union's members and reads the bare spelling as unclassified.
const SIGN_KEY: CoreTy = CoreTy::Union(&[KEY, CoreTy::Instance(crypto::KEY_PAIR_NAME)]);

/// `sign`'s trailing bag — the whole of what a caller may put in a header, and
/// the module doc's *the header is written* section is the home of each.
const SIGN_OPTIONS: &[CoreOption] = &[
    // Neutral for the claims' reason: a header member is base64 and a dot by
    // the time it leaves, so it carries no argument's `tainted` into a sink.
    CoreOption {
        name: "kid",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "typ",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "embedKey",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// One verified claim, as `rule:security/verification-does-not-launder` requires it back.
const CLAIM: CoreTy = CoreTy::TaintedStr;

/// `rule:security/protocol-roster`'s fourth roster entry, as two rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "sign",
            names: &["claims", "lifetime", "key"],
            // The claims are neutral: the token is base64 and a dot, so it
            // carries no argument's `tainted` into any sink, exactly as
            // `Core\Csrf::issue`'s answer does not.
            params: &[
                CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
                CoreTy::Instance(crate::time::DURATION_NAME),
                SIGN_KEY,
                CoreTy::Options(SIGN_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_jwt_sign",
            doc: Some(&SIGN_DOC),
        },
        CoreMethod {
            name: "verify",
            names: &["token", "key"],
            // The token is neutral on the way in and the claims are `tainted`
            // on the way out whatever it was — the module doc's *a claim is
            // text* section is the home of why that is a promise rather than a
            // classification, and `rule:security/verification-does-not-launder` is the rule.
            params: &[CoreTy::Text(Qual::Neutral), KEY],
            defaults: &[],
            return_ty: CoreTy::Array(&CLAIM),
            symbol: "nvs_core_jwt_verify",
            doc: Some(&VERIFY_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Jwt::sign`'s reference card — `rule:core-api/reference-card`.
const SIGN_DOC: MethodDoc = MethodDoc {
    short: "Signs `$claims` into a JWT that expires `$lifetime` from now, under `$key` and the \
            one algorithm that key has. The expiry is written here rather than passed in, so a \
            token this member produces always carries one.",
    params: &[
        ParamDoc {
            name: "claims",
            desc: "The application's own claims, by name. `exp` and `iat` are written by this \
                   member and are refused here; every other name is carried through unchanged.",
            shape: &[],
        },
        ParamDoc {
            name: "lifetime",
            desc: "How long the token stays valid — `15m`, `1h`, `7d`. It must be positive: a \
                   token that has already expired is a program bug, not a token.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "A shared secret of at least 32 octets, which signs HS256 — \
                   `Core\\Crypto::generateKey()` answers one, and a longer secret agreed with \
                   another service is accepted as it stands. Or a `Core\\Crypto\\KeyPair`, which \
                   signs ES256, EdDSA, RS256 or PS256 by its kind. There is no algorithm \
                   argument: the key is the choice.",
            shape: &[],
        },
        ParamDoc {
            name: "kid",
            desc: "The key's name in the header, for a recipient holding several. Omitted by \
                   default, and written verbatim — it names a key and selects nothing.",
            shape: &[],
        },
        ParamDoc {
            name: "typ",
            desc: "The header's `typ`, `JWT` when omitted. RFC 9068's access tokens spell it \
                   `at+jwt`.",
            shape: &[],
        },
        ParamDoc {
            name: "embedKey",
            desc: "Writes the pair's public half into the header as RFC 7638's minimal JWK, \
                   which is what a DPoP proof carries. Off by default, and a bug under a shared \
                   secret, which has no public half.",
            shape: &[],
        },
    ],
    ret: "The three base64url parts and their two dots, as a header, a payload and a signature \
          — a value a header, a query string and a JSON body all carry unescaped.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` is a secret shorter than 32 octets, or an `X25519` pair, which signs \
                   nothing; `{embedKey: true}` was written under a shared secret; `$lifetime` \
                   is zero or negative; or `$claims` names `exp` or `iat`, which this member \
                   writes, or carries a positional entry, since a claim has a name.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "This process cannot spare a buffer the size of the token.",
        },
    ],
};

/// `Core\Jwt::verify`'s reference card — `rule:core-api/reference-card`.
const VERIFY_DOC: MethodDoc = MethodDoc {
    short: "Answers the claims `$token` carries, having checked that this key signed it and \
            that it has not expired. It throws rather than answering an empty value, so there \
            is no falsy result a comparison could mistake for a verified token.",
    params: &[
        ParamDoc {
            name: "token",
            desc: "The token as the request carried it, in its three-part form.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The shared secret `sign` was given, or the issuer's. The token's own `alg` \
                   is compared against this key's algorithm and never used to pick one.",
            shape: &[],
        },
    ],
    ret: "Every claim in the payload, by name, each one `tainted`: a signature proves who wrote \
          a value, not that it is safe for any sink. `exp` and `iat` are present in it, in their \
          own decimal spelling.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` is shorter than 32 octets — a value that was never a signing key.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The token is not one this key signed, which is one sentence for every way of \
                   not being one; or it is, and has expired, carries no `exp`, or carries a \
                   claim that is not text.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_jwt_sign" => (nvs_core_jwt_sign as *const ()).cast(),
        "nvs_core_jwt_verify" => (nvs_core_jwt_verify as *const ()).cast(),
        _ => return None,
    })
}

/// The one sentence every failed verification produces before the signature
/// has been believed.
///
/// One function so the four call sites cannot drift into four sentences, which
/// is the whole of what makes them indistinguishable.
fn refused() -> Fault {
    Fault::thrown(format!(
        "{NAME}::verify(): $token is not a token this key signed. Every way of not being one — \
         a shape that is not three base64url parts, a header this class did not write, an `alg` \
         that is not {ALG}, a signature under another key, and an altered payload — is this one \
         sentence, so a forgery says nothing about which half of it failed."
    ))
}

/// The signing key at `slot`, checked for length.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `bytes`, and a `LogicError` for
/// one shorter than [`MIN_KEY_LEN`] — reachable from source despite the
/// parameter's `secret bytes`, because a qualifier says nothing about length.
fn key_at<'a>(args: &'a [Value], slot: usize, member: &str) -> Result<&'a [u8], Fault> {
    let key = args[slot].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `bytes` for $key, got tag {}",
            args[slot].tag_byte()
        ))
    })?;
    if key.len() < MIN_KEY_LEN {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::{member}(): $key is {} octets, and {ALG} needs at least {MIN_KEY_LEN} \
                 (RFC 7518 § 3.2) — Core\\Crypto::generateKey() answers one. The value is not \
                 quoted here, because a key does not belong in a log.",
                key.len()
            ),
        ));
    }
    Ok(key)
}

/// The `string` at `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: the slot is a `string` in the row, so
/// another tag is a compiled-code bug rather than anything a program can
/// write.
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

/// The wall clock in whole seconds, which is the only resolution JWT has.
///
/// # Errors
///
/// A [`Fault::fatal`] for a clock outside the representable range, on
/// [`crate::time`]'s reasoning: both writers of the fixed clock prove the
/// value representable before storing it, so there is no program to write
/// against this.
fn now_seconds(ctx: &nvs_runtime::Ctx, member: &str) -> Result<i64, Fault> {
    crate::time::wall_clock(ctx)
        .map(|at| at.as_second())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::{member} found a fixed clock outside the representable range"
            ))
        })
}

/// `text` as a JSON string literal, escaped.
fn quoted(text: &str) -> String {
    serde_json::Value::String(text.to_owned()).to_string()
}

/// The protected header a token carries: `{alg, jwk?, kid?, typ}`, members
/// sorted, no whitespace.
///
/// Assembled in that order by hand rather than through a `serde_json::Map`, for
/// [`payload_of`]'s reason turned the other way round — a map would sort these
/// too, and the point is that the sorting is a property a reader of this file
/// can check rather than one a serializer happens to have. `jwk` arrives
/// already written by `Core\Crypto\PublicKey`, which answers RFC 7638's
/// required members in the same order.
fn header_of(alg: &str, jwk: Option<&str>, kid: Option<&str>, typ: &str) -> String {
    let mut header = format!(r#"{{"alg":{}"#, quoted(alg));
    if let Some(jwk) = jwk {
        header.push_str(r#","jwk":"#);
        header.push_str(jwk);
    }
    if let Some(kid) = kid {
        header.push_str(r#","kid":"#);
        header.push_str(&quoted(kid));
    }
    header.push_str(r#","typ":"#);
    header.push_str(&quoted(typ));
    header.push('}');
    header
}

/// The key `sign` was handed, as one of the two things it is allowed to be.
///
/// The algorithm rides on the variant rather than beside it, so there is no
/// arrangement of this value in which a shared secret is about to be signed
/// with under an asymmetric `alg` — `rule:security/algorithm-comes-from-the-key`
/// held as a type, the way [`crate::crypto::SigningKey`] holds it one layer
/// down.
///
/// The two variants are different sizes — an RSA pair against a borrowed slice
/// — and boxing the larger would buy an allocation on the request path to save
/// a few hundred bytes of stack that live for one call, which is priority 3
/// spent on priority 5. [`crate::crypto`]'s own `Keyed` turns the same trade
/// down for the same reason.
#[allow(clippy::large_enum_variant)]
enum Signer<'a> {
    /// A shared secret, [`MIN_KEY_LEN`] octets or longer: [`ALG`].
    Shared(&'a [u8]),
    /// A pair, and the one algorithm [`pair_alg`] gives its kind.
    Pair(PrivateKey, &'static str),
}

/// The key at `slot`, read as whichever arm of the row's union it is.
///
/// # Errors
///
/// A `LogicError` for a secret shorter than [`MIN_KEY_LEN`] and for an `X25519`
/// pair, which signs nothing. A [`Fault::fatal`] for a pair whose stored DER no
/// longer parses, which is unreachable from source: the octets are the ones
/// `Core\Crypto\KeyPair::read` already parsed under the same kind.
fn signer_at(args: &[Value], slot: usize) -> Result<Signer<'_>, Fault> {
    if args[slot].as_bytes().is_some() {
        return Ok(Signer::Shared(key_at(args, slot, "sign")?));
    }

    let (held, kind) = crypto::stored_key(args, slot, &crypto::KEY_PAIR, "sign")?;
    let alg = pair_alg(kind).ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::sign(): an `X25519` pair signs nothing — it is a key for \
                 Core\\Crypto::agree alone. A `P256` pair signs ES256, an `Ed25519` pair EdDSA, \
                 and the two RSA kinds RS256 and PS256."
            ),
        )
    })?;
    let der = crypto::stored_octets(&held, &crypto::KEY_PAIR, "sign")?;
    let pair = PrivateKey::read(der, kind).ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::sign held a `{}` that is no longer a private key of its own kind",
            crypto::KEY_PAIR_NAME
        ))
    })?;
    Ok(Signer::Pair(pair, alg))
}

/// The header `jwk` a `{embedKey: true}` call writes: the pair's public half as
/// RFC 7638's minimal JWK.
///
/// # Errors
///
/// A [`Fault::fatal`] either way. Every kind that reaches this has a public half
/// and a JWK spelling for it, so both refusals are about this program's own
/// octets rather than about anything a call site wrote.
fn embedded_jwk(pair: &PrivateKey) -> Result<String, Fault> {
    let broken = || {
        Fault::fatal(format!(
            "{NAME}::sign held a pair with no public half to embed"
        ))
    };
    let public = pair.public().ok_or_else(broken)?;
    let written = public.write(KeyFormat::Jwk).map_err(|_| broken())?;
    String::from_utf8(written).map_err(|_| broken())
}

/// The payload `sign` signs: the caller's claims in the order they were
/// written, then the two registered ones this class owns.
///
/// Assembled as text rather than through a `serde_json::Map`, so the claims
/// keep the array's insertion order — a map would sort them, and a token whose
/// claim order depends on the serializer's collation is one nobody can pin in
/// a test.
///
/// # Errors
///
/// A `LogicError` for a positional entry or for either registered name, and a
/// [`Fault::fatal`] for a claim that is not a `string`, which the row's
/// `array<string>` has already refused.
fn payload_of(claims: &NvsArray, now: i64, exp: i64) -> Result<String, Fault> {
    let mut payload = String::from("{");
    let mut from = 0_usize;
    while let Some(slot) = claims.next_slot(from) {
        from = slot + 1;
        let key = claims
            .key_at(slot)
            .expect("next_slot only names live entries");
        let name = std::str::from_utf8(key.as_bytes()).map_err(|_| {
            Fault::fatal(format!(
                "{NAME}::sign found an array key that is not UTF-8, which no source can write"
            ))
        })?;
        if matches!(name, "exp" | "iat") {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{NAME}::sign(): $claims names `{name}`, which this member writes from \
                     $lifetime — expiry is not the caller's to set, so that a token this member \
                     produces always carries one it will accept."
                ),
            ));
        }
        if name.parse::<i64>().is_ok() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{NAME}::sign(): $claims[{name}] has a positional key, and a claim has a \
                     name — write `[\"sub\" => \"ada\"]` rather than a list."
                ),
            ));
        }
        let held = claims
            .value_at(slot)
            .expect("next_slot only names live entries");
        let value = held.as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::sign expected a `string` for $claims[{name}], got tag {}",
                held.tag_byte()
            ))
        })?;
        payload.push_str(&quoted(name));
        payload.push(':');
        payload.push_str(&quoted(value));
        payload.push(',');
    }
    payload.push_str(&format!(r#""iat":{now},"exp":{exp}}}"#));
    Ok(payload)
}

/// One JSON value as the text a claim comes back as.
///
/// `None` for the three shapes that have no honest text — the module doc's *a
/// claim is text* section is the home of why they are refused rather than
/// rendered.
fn claim_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            None
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwt::sign(array<string> $claims, Duration $lifetime, secret bytes|Crypto\KeyPair $key, {kid?, typ?, embedKey?}): string`
    /// — the write half of `rule:security/protocol-roster`'s fourth entry, replacing the
    /// hand-rolled `base64_encode` + `hash_hmac` + `rtrim` triple and the
    /// several userland libraries that wrap it.
    ///
    /// `$lifetime` is positional and not an option, which is the member's
    /// whole answer to § 4's "there is no flag to disable the check": a caller
    /// cannot leave out an argument that is not optional. The module doc's own
    /// section is the home of why `exp` is refused in `$claims` as well.
    ///
    /// The order below is what makes a token reproducible: the header is
    /// settled before anything is encoded, so every byte that gets signed is
    /// fixed by the key, the bag and the clock and by nothing read later.
    fn nvs_core_jwt_sign(ctx, args: [6]) {
        let raw = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::sign expected {:?} for $claims, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let claims = crate::arr::borrowed(raw);
        let lifetime = crate::time::nanos_of(args, 1, "sign")?;
        let signer = signer_at(args, 2)?;
        let kid = args[3].as_text();
        let typ = args[4].as_text().unwrap_or(TYP);
        let embed = args[5].as_bool().unwrap_or(false);

        let (alg, jwk) = match &signer {
            Signer::Shared(_) => {
                if embed {
                    return Err(Fault::thrown_as(
                        ThrownClass::Logic,
                        format!(
                            "{NAME}::sign(): {{embedKey: true}} writes the signing key's public \
                             half into the header, and a shared secret has no public half — it \
                             is the key the recipient already holds. Sign under a \
                             `Core\\Crypto\\KeyPair` to embed one."
                        ),
                    ));
                }
                (ALG, None)
            }
            Signer::Pair(pair, alg) => {
                let jwk = if embed {
                    Some(embedded_jwk(pair)?)
                } else {
                    None
                };
                (*alg, jwk)
            }
        };
        let header = header_of(alg, jwk.as_deref(), kid, typ);

        // Whole seconds, and the truncation is deliberate: `exp` is a second
        // count by RFC 7519 § 4.1.4, so a `500ms` lifetime is not a token
        // that lives half a second — it is a token that has already expired,
        // and it is refused as one rather than silently rounded up.
        let seconds = lifetime / 1_000_000_000;
        if seconds <= 0 {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{NAME}::sign(): $lifetime is {lifetime}ns, and a token has to be valid for \
                     at least one whole second — `exp` counts seconds (RFC 7519 § 4.1.4), so \
                     anything shorter signs a token that is already past it."
                ),
            ));
        }

        let now = now_seconds(ctx, "sign")?;
        let exp = now.checked_add(seconds).ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{NAME}::sign(): $lifetime of {seconds}s runs past the end of the \
                     representable range from now."
                ),
            )
        })?;

        let payload = payload_of(&claims, now, exp)?;
        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&header),
            URL_SAFE_NO_PAD.encode(&payload)
        );
        let signature = match signer {
            Signer::Shared(key) => {
                URL_SAFE_NO_PAD.encode(crate::hash::hmac_sha256(key, signing_input.as_bytes()))
            }
            Signer::Pair(pair, _) => {
                // Unreachable from source twice over, and each is the machine
                // rather than the program: `signing` answers `None` only for
                // the kind `signer_at` already refused, and `sign` only where
                // `ring`'s generator has failed under the two randomized
                // algorithms.
                let key = pair.signing().ok_or_else(|| {
                    Fault::fatal(format!("{NAME}::sign held a pair of a kind that signs nothing"))
                })?;
                let written = crypto::sign(&key, signing_input.as_bytes()).ok_or_else(|| {
                    Fault::fatal(format!(
                        "{NAME}::sign could not draw the randomness a signature needs"
                    ))
                })?;
                URL_SAFE_NO_PAD.encode(written)
            }
        };

        // Asked once with the real number, as `crate::crypto::seal_under`
        // does: the answer's size is known exactly here.
        let len = signing_input.len() + 1 + signature.len();
        nvs_runtime::affordable(Some(len), "Core\\Jwt::sign")?;
        let mut token = String::with_capacity(len);
        token.push_str(&signing_input);
        token.push('.');
        token.push_str(&signature);
        Ok(Value::str(NvsStr::new(token.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwt::verify(string $token, secret bytes $key): array<tainted string>`
    /// — the read half, and `rule:security/algorithm-comes-from-the-key`'s three rules in one body.
    ///
    /// The order is load-bearing. `alg` is compared before anything is
    /// believed, the signature is checked before the payload is looked at at
    /// all, and expiry is checked last — so the only failure that gets its own
    /// sentence is the one whose message a forger can never provoke. The
    /// module doc's *verification returns claims or throws* section is the
    /// home of why.
    fn nvs_core_jwt_verify(ctx, args: [2]) {
        let token = text_at(args, 0, "verify", "$token")?;
        let key = key_at(args, 1, "verify")?;

        let mut parts = token.split('.');
        let (Some(header_b64), Some(payload_b64), Some(signature_b64), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(refused());
        };

        // The header is read to *compare*, never to choose. Anything other
        // than an object whose `alg` is the one algorithm this class has is
        // the same refusal as a bad signature.
        let header_json = URL_SAFE_NO_PAD.decode(header_b64).map_err(|_| refused())?;
        let header: serde_json::Value =
            serde_json::from_slice(&header_json).map_err(|_| refused())?;
        if header.get("alg").and_then(serde_json::Value::as_str) != Some(ALG) {
            return Err(refused());
        }

        let signature = URL_SAFE_NO_PAD.decode(signature_b64).map_err(|_| refused())?;
        let signing_input = &token[..header_b64.len() + 1 + payload_b64.len()];
        let tag = crate::hash::hmac_sha256(key, signing_input.as_bytes());
        if !bool::from(signature.ct_eq(&tag)) {
            return Err(refused());
        }

        // Past here the token is authentic, so a message may say what is
        // wrong with it: the only reader who can reach one of these already
        // holds a token this key signed.
        let payload_json = URL_SAFE_NO_PAD.decode(payload_b64).map_err(|_| refused())?;
        let payload: serde_json::Value =
            serde_json::from_slice(&payload_json).map_err(|_| refused())?;
        let Some(claims) = payload.as_object() else {
            return Err(refused());
        };

        let exp = claims
            .get("exp")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "{NAME}::verify(): the token carries no `exp`, and expiry is not optional \
                     (`rule:security/algorithm-comes-from-the-key`). There is no flag that accepts one, because a caller who \
                     wants a credential that never expires is not using JWT for what JWT is."
                ))
            })?;
        let now = now_seconds(ctx, "verify")?;
        if now >= exp {
            return Err(Fault::thrown(format!(
                "{NAME}::verify(): the token expired at {exp} and it is now {now}. This is the \
                 one refusal with its own sentence: it is reached only after the signature has \
                 been checked, so nobody but the holder of a real token ever sees it."
            )));
        }

        let mut out = NvsArray::new();
        for (name, value) in claims {
            let text = claim_text(value).ok_or_else(|| {
                Fault::thrown(format!(
                    "{NAME}::verify(): the claim `{name}` is not a string, a number or a \
                     boolean, and every claim comes back as `tainted string` so that `rule:security/protocol-roster` \
                     § 5's qualifier survives to the value a program reads. A null, an object \
                     and an array are refused rather than given a text spelling nothing else \
                     reads back."
                ))
            })?;
            out.set(
                NvsStr::new(name.as_bytes()),
                Value::str(NvsStr::new(text.as_bytes())),
            );
        }
        Ok(Value::array(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A token signed under `key`, with `payload` verbatim and `header`
    /// verbatim — the shape an attacker gets to build, so the test can build
    /// it too.
    fn token_of(header: &str, payload: &str, key: &[u8], sign: bool) -> String {
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header),
            URL_SAFE_NO_PAD.encode(payload)
        );
        let signature = if sign {
            URL_SAFE_NO_PAD.encode(crate::hash::hmac_sha256(key, input.as_bytes()))
        } else {
            String::new()
        };
        format!("{input}.{signature}")
    }

    /// Stage 4's JWT check — `rule:security/algorithm-comes-from-the-key`'s first bullet, which is the whole
    /// reason this entry is on the roster: `alg` is compared against the key's
    /// algorithm and never consulted to select one.
    ///
    /// Asserted over the construction rather than through the registry, for
    /// [`crate::csrf`]'s reason: what could go wrong is that a string out of
    /// the token reaches a decision, and the three tokens below are exactly
    /// the ones that would get somewhere if it did — `none` with no signature,
    /// `none` with the signature of the token it was altered from, and an
    /// algorithm we do not have. All three take the same path as a forgery.
    #[test]
    fn a_jwt_with_an_unexpected_algorithm_is_refused() {
        let key = [9_u8; MIN_KEY_LEN];
        let payload = r#"{"sub":"ada","iat":0,"exp":4102444800}"#;

        // The baseline: this header, this payload, this key. Whatever the
        // three below fail on, it is not the payload.
        let honest = token_of(HEADER, payload, &key, true);
        let parts = honest.split('.');
        let signature = parts.clone().nth(2).expect("three parts");
        assert_eq!(parts.clone().count(), 3, "a JWT is three parts");

        // `alg: none` with the signature stripped — the attack the bullet
        // names first. There is no arm to reach: the comparison against ALG
        // fails before anything is decoded.
        let stripped = token_of(r#"{"alg":"none","typ":"JWT"}"#, payload, &key, false);
        assert_eq!(
            stripped.split('.').nth(2),
            Some(""),
            "the attack is that an empty signature is accepted"
        );

        // `alg: none` carrying the *honest* token's signature, which a class
        // that selected its verifier from the header would not even look at.
        let carried = format!(
            "{}.{signature}",
            stripped.rsplit_once('.').expect("three parts").0
        );

        // And an algorithm nobody here has. A class that matched on the field
        // would need an arm; this one needs none, which is what makes the
        // refusal total rather than exhaustive.
        let other = token_of(r#"{"alg":"RS256","typ":"JWT"}"#, payload, &key, true);

        for (label, token) in [
            ("alg: none, no signature", &stripped),
            ("alg: none, borrowed signature", &carried),
            ("alg: RS256", &other),
        ] {
            let header = token.split('.').next().expect("a first part");
            let decoded = URL_SAFE_NO_PAD
                .decode(header)
                .expect("the header is base64url");
            let parsed: serde_json::Value =
                serde_json::from_slice(&decoded).expect("the header is JSON");
            let alg = parsed.get("alg").and_then(serde_json::Value::as_str);
            assert_ne!(alg, Some(ALG), "{label} does not claim our algorithm");
            assert_ne!(token, &honest, "{label} is not the honest token");
        }

        // The honest one does claim it, so the comparison above is doing work
        // rather than being true of every token.
        let header = URL_SAFE_NO_PAD
            .decode(honest.split('.').next().expect("a first part"))
            .expect("the header is base64url");
        let parsed: serde_json::Value =
            serde_json::from_slice(&header).expect("the header is JSON");
        assert_eq!(
            parsed.get("alg").and_then(serde_json::Value::as_str),
            Some(ALG),
            "the header this class writes is the one it accepts"
        );

        // The signature is over the header as well as the payload, which is
        // what makes swapping the header detectable at all: the borrowed
        // signature above is a tag for different bytes.
        let input = stripped.rsplit_once('.').expect("three parts").0;
        assert_ne!(
            URL_SAFE_NO_PAD.encode(crate::hash::hmac_sha256(&key, input.as_bytes())),
            signature,
            "the header is inside the signing input, so a swapped one does not verify"
        );
    }

    /// Every claim comes back as text or not at all — the module doc's *a
    /// claim is text* section, which is where `rule:security/verification-does-not-launder`'s qualifier forces
    /// the element type.
    #[test]
    fn a_claim_is_text_or_it_is_refused() {
        for (json, want) in [
            (r#""ada""#, Some("ada")),
            ("42", Some("42")),
            ("-1", Some("-1")),
            ("true", Some("true")),
            ("null", None),
            ("[1]", None),
            (r#"{"a":1}"#, None),
        ] {
            let value: serde_json::Value = serde_json::from_str(json).expect("the fixture is JSON");
            assert_eq!(
                claim_text(&value).as_deref(),
                want,
                "the claim value {json} comes back as {want:?}"
            );
        }
    }

    /// The hand-written header and the assembled one are the same bytes, so
    /// the spelling in the file stays the thing a reader can check the writer
    /// against.
    #[test]
    fn the_default_header_is_the_written_constant() {
        assert_eq!(header_of(ALG, None, None, TYP), HEADER);
    }

    /// `{alg, jwk?, kid?, typ}`, sorted, whatever order the bag was written in
    /// and whatever the members hold — a claim-shaped `kid` is escaped rather
    /// than closing the object it is inside.
    #[test]
    fn a_header_is_sorted_and_its_members_are_escaped() {
        assert_eq!(
            header_of("ES256", Some(r#"{"crv":"P-256"}"#), Some("2026"), "at+jwt"),
            r#"{"alg":"ES256","jwk":{"crv":"P-256"},"kid":"2026","typ":"at+jwt"}"#
        );
        assert_eq!(
            header_of("EdDSA", None, Some(r#"a"},"alg":"none"#), TYP),
            r#"{"alg":"EdDSA","kid":"a\"},\"alg\":\"none","typ":"JWT"}"#
        );
    }

    /// Every kind but the one that agrees carries exactly one JWS algorithm,
    /// and the four are distinct — a table that collapsed two kinds onto one
    /// `alg` would be a key signing under a scheme it was not read as.
    #[test]
    fn each_kind_carries_one_algorithm_and_x25519_carries_none() {
        let kinds = [
            KeyKind::P256,
            KeyKind::X25519,
            KeyKind::Ed25519,
            KeyKind::RsaPkcs1,
            KeyKind::RsaPss,
        ];
        let named: Vec<&str> = kinds.iter().copied().filter_map(pair_alg).collect();
        assert_eq!(named, ["ES256", "EdDSA", "RS256", "PS256"]);
        assert_eq!(pair_alg(KeyKind::X25519), None);
    }
}
