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
//! under a pair is not one it reads at all; [`nvs_core_jwt_verify_issued`] is
//! the asymmetric half of the same rule, comparing a header's `alg` against
//! [`pair_alg`] of the kind the key it *found* was read as.
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
//! [`nvs_core_jwt_verify`] refuses a payload carrying no `exp`, and every
//! member that signs takes the lifetime as a **positional `Duration`** and
//! writes `exp` itself through [`registered_pair`] — so a caller cannot forget
//! it, cannot pass it as an option they leave out, and cannot supply their own:
//! `exp` and `iat` among the claims are a `LogicError` naming the parameter
//! that owns them, whichever shape those claims arrived in. A token
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
//! **The two members that sign are split on exactly that promise.**
//! [`nvs_core_jwt_sign`] takes `array<string>` for the reason above read from
//! the other side: both halves of the shared-key round trip agree about what a
//! claim is, so a token this class signs under a shared key never trips the
//! refusal above. [`nvs_core_jwt_sign_object`] takes an `object` and a key
//! pair, which is a token for somebody else's verifier — its claims are
//! written as `Core\Json::encode` writes them, and what that party accepts is
//! not this module's to promise. The split is what keeps the agreement in the
//! one direction it is about: a structured payload cannot be signed under a
//! shared key at all, so the member that reads a token back still only ever
//! meets the text claims `sign` wrote.
//!
//! `$claims` there is spelled `object` rather than `mixed`, so a value that is
//! not structured is refused where the call is written and the body owes no
//! sentence about one — [`crate::registry::CoreTy::Object`] is the home of
//! that choice. A claims shape built out of a request reaches it carrying the
//! per-field qualifiers `rule:security/tainted-qualifier` distributed onto it,
//! which is why a parameter that is neither `string` nor `bytes` needs no
//! classification of its own.
//!
//! # A token another party issued is checked in one order, and the line is the signature
//!
//! [`nvs_core_jwt_verify_issued`] is the member for a token this program did
//! not sign, and every rule it holds is a rule about *order*. Shape, header
//! policy, key, signature — and then the clock and the registered claims, which
//! are reached only under a signature that held. Everything before that line
//! answers with [`not_issued`]'s one sentence, because a forger chooses all of
//! it and telling one forgery from another is an oracle; everything after it
//! may name the claim, because the only reader who gets there is holding a
//! token the issuer really signed.
//!
//! **The header is read to refuse and to compare, never to choose.**
//! [`REFUSED_HEADER`] is a deny-list rather than an allow-list, and that is the
//! one place this member is deliberately looser than [`crate::jwe`]: an issuer
//! sends hints a verifier does not read, and refusing what is merely unknown
//! refuses real ID tokens. What is refused is what a verifier would have to
//! *act* on — a key or a fetch the token brought, an extension it would have to
//! understand, a payload that is not the one segment it looks like.
//!
//! **A key is found, never tried.** A `kid` is a lookup into the set and selects
//! nothing else, and a token carrying none is answered only by a set holding
//! exactly one key. So a token costs at most one signature check whatever the
//! set holds, which is what keeps an RSA verification off a request's budget
//! however many keys an issuer publishes.
//!
//! **The claims come back as a type, not as a table.** The payload is handed to
//! [`crate::json::decode_as`], so what a token may say is exactly what a JSON
//! document may say, and `rule:security/derived-codec-qualifiers` has already
//! made the call site declare `tainted` on every text field reachable from the
//! written type. That is how
//! `rule:security/verification-does-not-launder` survives a structured answer
//! without flattening every claim to text, which is the trade [`CLAIM`] makes
//! for the member above it.
//!
//! **`leeway` widens the window and never removes it.** It is bounded at
//! [`MAX_LEEWAY`] and refused below zero, because it is the one option here that
//! makes an expired token verify and `rule:security/jwt-expiry-is-mandatory`
//! leaves no room for a flag that removes the check.
//!
//! # A key set is an admission, and the two ways a document fails are not one
//!
//! [`KEY_SET`] is the second class here: the keys another party publishes, read
//! once out of a JWKS document and held so a token can be checked against the
//! one key its `kid` names. Reading it is where every decision about those keys
//! is made, so nothing downstream ever asks a question about a key again.
//!
//! A document fails in two ways that are deliberately different answers. A key
//! this roster has no use for — one marked for encryption, one of a kind or
//! under an algorithm outside `rule:security/protocol-roster`'s closed set — is
//! **skipped**, because a real JWKS carries keys for purposes we do not serve
//! and refusing the set over one of them would make every rotation an outage. A
//! document that is *wrong* — a private member where a public key goes, an RSA
//! key under [`crypto::MIN_RSA_BITS`], two keys under one name, an algorithm a
//! key's kind cannot carry — is **refused whole**, because it means the program
//! is pointed at something it should not be reading keys from at all.
//!
//! That refusal names what was wrong, where [`refused`] names nothing. The
//! reader of one is the operator who configured the URL, not a forger: a
//! document is fetched by this program, so there is no attacker on the other
//! end of the message to learn which check failed.
//!
//! **An RSA key is the one kind whose algorithm the key does not carry.** RS256
//! and PS256 are one key type under two schemes, so a key with no `alg` is read
//! under the `rsaScheme` option or refused — never guessed, which is
//! `rule:security/algorithm-comes-from-the-key` at the one place a kind is
//! genuinely ambiguous. A key's own `alg` always wins over the option.
//!
//! **What a set holds** is one slot, and a slot holds one value, so the
//! admitted keys are written into a single `bytes` in [`framed`]'s layout and
//! read back by [`keys_in`]. What is stored is each key's
//! `SubjectPublicKeyInfo` — a little over a kilobyte for RSA at
//! [`crypto::MAX_RSA_BITS`] and under a hundred octets for a curve — under the
//! `kid` the document gave it, so at [`MAX_KEYS`] keys a set is a few kilobytes
//! held for as long as the program holds the object, and nothing is held
//! between calls.
//!
//! # Constant time
//!
//! The signature comparison is `subtle::ConstantTimeEq` over the whole tag, on
//! [`crate::hash`]'s reasoning, which is that module's own doc. No member here
//! hands back a tag, a key or a signing input, which is `rule:security/algorithm-comes-from-the-key`'s "no API
//! exposes the raw value for the caller to compare themselves" for this entry.

use std::collections::BTreeSet;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use subtle::ConstantTimeEq as _;

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, ThrownClass, Value};

use crate::crypto::{self, KeyFormat, KeyKind, PrivateKey, PublicKey};
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

/// The keys `verifyIssued` checks a token against: a set to find one in by
/// `kid`, or the single key the program was handed directly.
///
/// Both arms are a key that has already been read, so the algorithm is settled
/// before the token is looked at — `rule:security/algorithm-comes-from-the-key`
/// crossing into this member as a type rather than as an argument.
const VERIFY_KEYS: CoreTy = CoreTy::Union(&[
    CoreTy::Instance(KEY_SET_NAME),
    CoreTy::Instance(crypto::PUBLIC_KEY_NAME),
]);

/// The longest token `verifyIssued` looks at, in octets, checked before the
/// first segment is decoded.
///
/// An ID token carrying a name, an email and a handful of registered claims is
/// a few hundred octets, and the header parameters that make a real token large
/// — a certificate chain under `x5c` — are refused outright by
/// [`REFUSED_HEADER`]. So the cap is reached only by something that is not a
/// token, and reaching it costs one length comparison rather than a base64
/// decode and a parse.
const MAX_TOKEN_LEN: usize = 8 * 1024;

/// The header parameters a token another party issued may not carry.
///
/// A key or a fetch the token brings with it (`jwk`, `jku`, `x5u`, `x5c`), an
/// extension a verifier would have to understand to be safe (`crit`), and a
/// payload that is not the one segment it looks like (`b64`, `zip`, `cty`).
/// `zip` is a decompression bomb and the URL-bearing ones are outbound
/// requests a token got this program to make.
///
/// Every other member is **ignored**, which is looser than
/// [`crate::jwe`]'s allow-list on purpose: an issuer sends hints such as `x5t`
/// and `nonce`, and a verifier refusing what it does not read refuses real ID
/// tokens.
const REFUSED_HEADER: &[&str] = &["jku", "x5u", "x5c", "jwk", "crit", "b64", "zip", "cty"];

/// The `leeway` a call that names none gets, in seconds — RFC 7519 § 4.1.4's
/// "small leeway, no more than a few minutes", at the small end of it.
const DEFAULT_LEEWAY: i64 = 60;

/// The widest `leeway` a call may ask for, in seconds.
///
/// A bound rather than a caller's choice because leeway is the one knob here
/// that makes an expired token verify, and five minutes is already wider than
/// any clock a server has business disagreeing with. A caller wanting more is
/// asking for the check `rule:security/jwt-expiry-is-mandatory` refuses to have
/// a flag for.
const MAX_LEEWAY: i64 = 300;

/// `verifyIssued`'s trailing bag — what a verifier may relax, and what it may
/// additionally require.
///
/// Nothing here selects an algorithm or a key, which is the whole of why the
/// bag is safe to widen later: every option is a bound on a claim the token
/// already carries.
const VERIFY_ISSUED_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "leeway",
        ty: CoreTy::Instance(crate::time::DURATION_NAME),
        default: Const::Null,
    },
    // Neutral for the header bag's reason: each of these is *compared* against
    // a claim and written into nothing, so no byte of an argument crosses back
    // out in the answer.
    CoreOption {
        name: "typ",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "nonce",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "maxAge",
        ty: CoreTy::Instance(crate::time::DURATION_NAME),
        default: Const::Null,
    },
];

/// `rule:security/protocol-roster`'s fourth roster entry: the two ways of
/// signing a token, and the one way of reading back a token this program
/// signed under a shared key.
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
            name: "signObject",
            names: &["claims", "lifetime", "key"],
            // `object` rather than `mixed`, which is the whole of the
            // difference between this member and a `sign` widened by a union:
            // a value that is not structured is refused where the call is
            // written. The claims are neutral for the row above's reason, and
            // the shape they arrive in carries its own fields' qualifiers,
            // which is `rule:security/tainted-qualifier`'s distribution rather
            // than anything this row says.
            params: &[
                CoreTy::Object,
                CoreTy::Instance(crate::time::DURATION_NAME),
                CoreTy::Instance(crypto::KEY_PAIR_NAME),
                CoreTy::Options(SIGN_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_jwt_sign_object",
            doc: Some(&SIGN_OBJECT_DOC),
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
        CoreMethod {
            name: "verifyIssued",
            names: &["token", "keys", "issuer", "audience"],
            // Every parameter is neutral, and for `verify`'s reason rather than
            // because nothing of the token reaches the answer: the claims come
            // back inside a `T` whose text fields the call site has already been
            // made to declare `tainted`
            // (`rule:security/derived-codec-qualifiers`), so the qualifier is
            // carried by the written type and not by this row. `$issuer`,
            // `$audience` and the bag's two strings are compared and written
            // into nothing at all.
            params: &[
                CoreTy::Text(Qual::Neutral),
                VERIFY_KEYS,
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(VERIFY_ISSUED_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Written("T"),
            symbol: "nvs_core_jwt_verify_issued",
            doc: Some(&VERIFY_ISSUED_DOC),
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

/// `Core\Jwt::signObject`'s reference card — `rule:core-api/reference-card`.
const SIGN_OBJECT_DOC: MethodDoc = MethodDoc {
    short: "Signs a structured `$claims` into a JWT that expires `$lifetime` from now, under a \
            key pair and the one algorithm that pair has. The claims are written as \
            `Core\\Json::encode` writes them, with `iat` and `exp` appended.",
    params: &[
        ParamDoc {
            name: "claims",
            desc: "A shape or a class carrying `#[Core\\Json\\Derive]`, encoded as the JSON \
                   object it is. `exp` and `iat` are written by this member and are refused \
                   among its fields; a value with no JSON encoding is a bug in the program.",
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
            desc: "A `Core\\Crypto\\KeyPair`, which signs ES256, EdDSA, RS256 or PS256 by its \
                   kind. A shared secret is not accepted here: a structured payload is a token \
                   issued to another party, and `Core\\Jwt::verify` reads only the text claims \
                   `sign` writes.",
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
                   which is what a DPoP proof carries. Off by default.",
            shape: &[],
        },
    ],
    ret: "The three base64url parts and their two dots, exactly as `sign` answers — the payload \
          is the claims object with `iat` and `exp` as its last two members.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$claims` carries a field named `exp` or `iat`, which this member writes; or \
                   it has no JSON encoding at all — a class that carries no \
                   `#[Core\\Json\\Derive]`, a `secret` value, a `bytes`, or a cycle. `$key` is \
                   an `X25519` pair, which signs nothing, or `$lifetime` is zero or negative.",
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

/// `Core\Jwt::verifyIssued`'s reference card — `rule:core-api/reference-card`.
const VERIFY_ISSUED_DOC: MethodDoc = MethodDoc {
    short: "Verifies a token another party issued and answers its claims as the written type, \
            having checked the header's policy, found the one key that may have signed it, \
            checked the signature, and then the clock and the registered claims in that order. \
            It throws rather than answering an empty value.",
    params: &[
        ParamDoc {
            name: "token",
            desc: "The token as the request carried it, in its three-part form. It is refused \
                   unread past 8 KiB.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The issuer's key set, in which the header's `kid` names exactly one key, or \
                   one public key on its own. There is no try-every-key: a token naming no \
                   `kid` verifies only against a set holding exactly one. The token's `alg` is \
                   compared against the key's algorithm and never used to pick one.",
            shape: &[],
        },
        ParamDoc {
            name: "issuer",
            desc: "What the token's `iss` must equal, exactly.",
            shape: &[],
        },
        ParamDoc {
            name: "audience",
            desc: "What the token's `aud` must be, or must hold. A token listing more than one \
                   audience must also carry an `azp` equal to this.",
            shape: &[],
        },
        ParamDoc {
            name: "leeway",
            desc: "How far the clock may disagree, at both ends of the window: 60 seconds when \
                   omitted, and refused when negative or wider than 5 minutes. It widens the \
                   expiry check and never removes it.",
            shape: &[],
        },
        ParamDoc {
            name: "typ",
            desc: "The `typ` the header must carry, compared case-insensitively with any \
                   `application/` prefix removed — `at+jwt` for RFC 9068's access tokens. Any \
                   `typ` is accepted when this is omitted.",
            shape: &[],
        },
        ParamDoc {
            name: "nonce",
            desc: "The value the token's `nonce` claim must equal, compared in constant time. A \
                   token carrying none is refused when this is named.",
            shape: &[],
        },
        ParamDoc {
            name: "maxAge",
            desc: "How old the token's `auth_time` may be. A token carrying none is refused \
                   when this is named.",
            shape: &[],
        },
    ],
    ret: "An instance of the written type, decoded from the payload as `Core\\Json::decodeAs` \
          decodes a document. Its text fields must be declared `tainted`, which the call site is \
          held to: a signature proves who wrote a claim, not that it is safe for any sink.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`leeway` is negative or wider than 5 minutes; the written type is a list, \
                   which no token's payload is; or the key is an `X25519` one, which verifies \
                   nothing.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The token is not one this issuer's key signed, its header carries a \
                   parameter a verifier may not honour, or no key was found for it — one \
                   sentence for every way of not being verifiable; or it is authentic, and is \
                   outside its validity window or carries a registered claim that is not what \
                   was asked for, which names the claim.",
        },
    ],
};

/// The second class this module registers: the keys another party publishes.
pub(crate) const KEY_SET_NAME: &str = r"Core\Jwt\KeySet";

/// Which slot of a set holds its keys, which is the frame [`framed`] wrote and
/// [`keys_in`] reads back.
const KEY_SET_KEYS_SLOT: usize = 0;

/// The most keys one set holds, which is what keeps a verification's cost a
/// property of this program rather than of the document it fetched.
const MAX_KEYS: usize = 16;

/// The members a *public* key never carries, any one of which means the
/// document handed over a private one.
///
/// `k` is the symmetric case and the rest are RSA's and EC's private halves.
/// Checked by presence alone, before anything reads the key: what makes it a
/// refusal is that the document is publishing them, whatever else is true of
/// them.
const PRIVATE_MEMBERS: &[&str] = &["d", "p", "q", "dp", "dq", "qi", "k"];

/// `read`'s trailing bag, which is one option wide: the scheme an RSA key
/// carrying no `alg` is read under.
const READ_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "rsaScheme",
    ty: CoreTy::Enum(crypto::KEY_KIND_NAME),
    default: Const::Null,
}];

/// The set a token is verified against, as one row.
pub(crate) const KEY_SET: CoreClass = CoreClass {
    name: KEY_SET_NAME,
    methods: &[CoreMethod {
        name: "read",
        names: &["jwks"],
        // `Text` rather than `CoreTy::TaintedStr`, which in parameter position
        // demands nothing: what this row needs is to *accept* a `tainted`
        // argument, since a JWKS document is a fetch, and that is a
        // classification. `Neutral` because what crosses back out is an
        // object, which carries no qualifier at all.
        params: &[CoreTy::Text(Qual::Neutral), CoreTy::Options(READ_OPTIONS)],
        defaults: &[],
        return_ty: CoreTy::Instance(KEY_SET_NAME),
        symbol: "nvs_core_jwt_key_set_read",
        doc: Some(&KEY_SET_READ_DOC),
    }],
    instance: &[],
    slots: &["keys"],
    constants: &[],
};

/// `Core\Jwt\KeySet::read`'s reference card — `rule:core-api/reference-card`.
const KEY_SET_READ_DOC: MethodDoc = MethodDoc {
    short: "Admits the signing keys a JWKS document publishes, so a token can be checked against \
            the one key its `kid` names. A key this roster has no use for is skipped and a \
            document that is wrong is refused whole, which are different answers to different \
            questions.",
    params: &[
        ParamDoc {
            name: "jwks",
            desc: "The document as the issuer served it. It is read here and nowhere else: \
                   fetching it, caching it and rotating it are a package's job, and this member \
                   is stateless over the text it is handed.",
            shape: &[],
        },
        ParamDoc {
            name: "rsaScheme",
            desc: "Which scheme an RSA key carrying no `alg` is read under — `RsaPkcs1` for \
                   RS256 or `RsaPss` for PS256. A key's own `alg` always wins, and an RSA key \
                   with neither is refused rather than guessed. Every other kind names its own \
                   algorithm, so this reaches nothing else.",
            shape: &[],
        },
    ],
    ret: "A set of at most 16 public keys, each under the name the document gave it, ready for a \
          lookup by `kid` and never for a try of every key.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`{rsaScheme: …}` names a kind that is not one of the two RSA ones, and no \
                   other kind has two schemes to choose between.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$jwks` is not a document to read keys from: it is not JSON, it carries no \
                   `keys` array, one of its keys carries a private member or a `kid` that is not \
                   text, a key is not a key of the kind it claims — an RSA modulus outside \
                   2048–8192 bits included — an `alg` is one its key's kind cannot carry, two \
                   keys share a `kid`, or more than 16 keys are admitted.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_jwt_sign" => (nvs_core_jwt_sign as *const ()).cast(),
        "nvs_core_jwt_sign_object" => (nvs_core_jwt_sign_object as *const ()).cast(),
        "nvs_core_jwt_verify" => (nvs_core_jwt_verify as *const ()).cast(),
        "nvs_core_jwt_verify_issued" => (nvs_core_jwt_verify_issued as *const ()).cast(),
        "nvs_core_jwt_key_set_read" => (nvs_core_jwt_key_set_read as *const ()).cast(),
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

    let (pair, alg) = pair_at(args, slot, "sign")?;
    Ok(Signer::Pair(pair, alg))
}

/// The pair at `slot` and the one algorithm its kind signs — `sign`'s
/// asymmetric arm, and the whole of `signObject`'s key.
///
/// One function so the two members cannot come to disagree about which kinds
/// sign: `rule:security/algorithm-comes-from-the-key` is held by [`pair_alg`]
/// being the only thing either one asks.
///
/// # Errors
///
/// A `LogicError` for an `X25519` pair, which signs nothing, and a
/// [`Fault::fatal`] for octets that no longer parse — unreachable from source,
/// since they are the ones `Core\Crypto\KeyPair::read` already parsed under
/// this very kind.
fn pair_at(args: &[Value], slot: usize, member: &str) -> Result<(PrivateKey, &'static str), Fault> {
    let (held, kind) = crypto::stored_key(args, slot, &crypto::KEY_PAIR, member)?;
    let alg = pair_alg(kind).ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::{member}(): an `X25519` pair signs nothing — it is a key for \
                 Core\\Crypto::agree alone. A `P256` pair signs ES256, an `Ed25519` pair EdDSA, \
                 and the two RSA kinds RS256 and PS256."
            ),
        )
    })?;
    let der = crypto::stored_octets(&held, &crypto::KEY_PAIR, member)?;
    let pair = PrivateKey::read(der, kind).ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} held a `{}` that is no longer a private key of its own kind",
            crypto::KEY_PAIR_NAME
        ))
    })?;
    Ok((pair, alg))
}

/// The header `jwk` a `{embedKey: true}` call writes: the pair's public half as
/// RFC 7638's minimal JWK.
///
/// # Errors
///
/// A [`Fault::fatal`] either way. Every kind that reaches this has a public half
/// and a JWK spelling for it, so both refusals are about this program's own
/// octets rather than about anything a call site wrote.
fn embedded_jwk(pair: &PrivateKey, member: &str) -> Result<String, Fault> {
    let broken = || {
        Fault::fatal(format!(
            "{NAME}::{member} held a pair with no public half to embed"
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

/// One pair's signature over a signing input, base64url with no padding.
///
/// # Errors
///
/// A [`Fault::fatal`] twice over, and each is the machine rather than the
/// program: `signing` answers `None` only for the kind [`pair_at`] already
/// refused, and `sign` only where `ring`'s generator has failed under the two
/// randomized algorithms. Both are unreachable from source.
fn pair_signature(pair: PrivateKey, input: &[u8], member: &str) -> Result<String, Fault> {
    let key = pair.signing().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} held a pair of a kind that signs nothing"
        ))
    })?;
    let written = crypto::sign(&key, input).ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} could not draw the randomness a signature needs"
        ))
    })?;
    Ok(URL_SAFE_NO_PAD.encode(written))
}

/// The two claims every signing member writes: the second it is issued in, and
/// that second plus `$lifetime`.
///
/// Whole seconds, and the truncation is deliberate: `exp` is a second count by
/// RFC 7519 § 4.1.4, so a `500ms` lifetime is not a token that lives half a
/// second — it is a token that has already expired, and it is refused as one
/// rather than silently rounded up.
///
/// One function so the members that sign cannot come to disagree about what a
/// lifetime buys, which is the bound
/// `rule:security/jwt-expiry-is-mandatory` is held by.
///
/// # Errors
///
/// A `LogicError` for a lifetime under one whole second and for one that runs
/// past the end of the representable range.
fn registered_pair(
    ctx: &nvs_runtime::Ctx,
    lifetime: i64,
    member: &str,
) -> Result<(i64, i64), Fault> {
    let seconds = lifetime / 1_000_000_000;
    if seconds <= 0 {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::{member}(): $lifetime is {lifetime}ns, and a token has to be valid for \
                 at least one whole second — `exp` counts seconds (RFC 7519 § 4.1.4), so \
                 anything shorter signs a token that is already past it."
            ),
        ));
    }

    let now = now_seconds(ctx, member)?;
    let exp = now.checked_add(seconds).ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::{member}(): $lifetime of {seconds}s runs past the end of the \
                 representable range from now."
            ),
        )
    })?;
    Ok((now, exp))
}

/// The payload `signObject` signs: the claims object as `Core\Json::encode`
/// writes it, then the two registered claims this class owns.
///
/// [`payload_of`]'s twin, and the same assembly by text for a related reason:
/// what is signed is the encoder's own output, so the members keep the order
/// that encoder chose — a deriving class's declaration order, a shape's sorted
/// field list — rather than whatever collation a parsed copy would be written
/// back out in. The parse here reads the field *names* alone, through
/// [`serde::de::IgnoredAny`], because the one question left is whether the
/// object already carries a claim this member writes.
///
/// # Errors
///
/// A `LogicError` for a claims object with no JSON encoding and for one
/// carrying `iat` or `exp`, and a [`Fault::fatal`] for encoder output that is
/// not a JSON object.
fn object_payload_of(claims: Value, now: i64, exp: i64) -> Result<String, Fault> {
    let written =
        serde_json::to_string(&crate::json::Encodable::document(claims)).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{NAME}::signObject(): $claims is written as `Core\\Json::encode` writes it, \
                     and {why}."
                ),
            )
        })?;
    // Unreachable from source: `object` admits a class instance and a shape and
    // nothing else, and `Core\Json`'s encoder writes each of those as a JSON
    // object or refuses it above.
    let named: std::collections::BTreeMap<String, serde::de::IgnoredAny> =
        serde_json::from_str(&written).map_err(|_| {
            Fault::fatal(format!(
                "{NAME}::signObject encoded $claims as a document that is not a JSON object"
            ))
        })?;
    for registered in ["iat", "exp"] {
        if named.contains_key(registered) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{NAME}::signObject(): $claims carries a field named `{registered}`, which \
                     this member writes from $lifetime — the clock is not the caller's to set, \
                     so that a token this member produces always carries an expiry it will \
                     accept."
                ),
            ));
        }
    }

    let registered = format!(r#""iat":{now},"exp":{exp}}}"#);
    let mut payload = String::with_capacity(written.len() + registered.len() + 1);
    // The encoder's text without its closing brace, which the registered pair
    // carries instead. An object that wrote no member of its own needs no comma
    // before them, and `named` is what says whether it wrote one.
    payload.push_str(&written[..written.len() - 1]);
    if !named.is_empty() {
        payload.push(',');
    }
    payload.push_str(&registered);
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
                    Some(embedded_jwk(pair, "sign")?)
                } else {
                    None
                };
                (*alg, jwk)
            }
        };
        let header = header_of(alg, jwk.as_deref(), kid, typ);
        let (now, exp) = registered_pair(ctx, lifetime, "sign")?;
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
            Signer::Pair(pair, _) => pair_signature(pair, signing_input.as_bytes(), "sign")?,
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
    /// `Core\Jwt::signObject(object $claims, Duration $lifetime, Crypto\KeyPair $key, {kid?, typ?, embedKey?}): string`
    /// — `sign` for a payload that is a structure rather than a table of text.
    ///
    /// A second member rather than a union arm on `sign`, because the two
    /// differ in their key as well as in their claims: this one takes a pair
    /// alone. `Core\Jwt::verify` reads back the text claims `sign` writes and
    /// refuses anything else, so a structured payload is a token issued to
    /// another party — and the key that signs one is the key that party can
    /// check, never the shared secret this program verifies with itself.
    ///
    /// The order is `sign`'s, for `sign`'s reason: the header is settled before
    /// anything is encoded, so every byte that gets signed is fixed by the key,
    /// the bag and the clock and by nothing read later.
    fn nvs_core_jwt_sign_object(ctx, args: [6]) {
        let claims = args[0];
        let lifetime = crate::time::nanos_of(args, 1, "signObject")?;
        let (pair, alg) = pair_at(args, 2, "signObject")?;
        let kid = args[3].as_text();
        let typ = args[4].as_text().unwrap_or(TYP);
        let embed = args[5].as_bool().unwrap_or(false);

        // No shared-key arm to refuse `embedKey` for: the row takes a pair, so
        // the public half the option writes always exists.
        let jwk = if embed {
            Some(embedded_jwk(&pair, "signObject")?)
        } else {
            None
        };
        let header = header_of(alg, jwk.as_deref(), kid, typ);
        let (now, exp) = registered_pair(ctx, lifetime, "signObject")?;
        let payload = object_payload_of(claims, now, exp)?;

        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&header),
            URL_SAFE_NO_PAD.encode(&payload)
        );
        let signature = pair_signature(pair, signing_input.as_bytes(), "signObject")?;

        // Asked once with the real number, as `sign` does: the answer's size is
        // known exactly here.
        let len = signing_input.len() + 1 + signature.len();
        nvs_runtime::affordable(Some(len), "Core\\Jwt::signObject")?;
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

/// The one sentence every token `verifyIssued` will not verify gets.
///
/// [`refused`]'s twin, and separate from it because the ways of not being
/// verifiable are not the same set: a header carrying a parameter a verifier may
/// not honour, a `kid` naming no key in the set, and an algorithm that is not
/// the one the found key carries. What the two share is the reason they are one
/// sentence — a forger picks every one of them, so telling them apart is an
/// oracle.
fn not_issued() -> Fault {
    Fault::thrown(format!(
        "{NAME}::verifyIssued(): $token is not a token this issuer's key signed. Every way of \
         not being one — a shape that is not three base64url parts, a header carrying a \
         parameter a verifier may not honour, a `kid` that names no key in the set, an `alg` \
         that is not the one that key carries, a signature under another key, and an altered \
         payload — is this one sentence, so a forgery says nothing about which half of it \
         failed."
    ))
}

/// The refusal a token that is outside its own validity window gets, saying
/// which end of it and by how much.
///
/// Its own sentence rather than [`refused`]'s, for the reason `verify`'s expiry
/// message has one: it is reached only under a signature that held, so the only
/// reader who ever sees it is holding a token the issuer really signed.
fn outside_the_window(said: &str) -> Fault {
    Fault::thrown(format!(
        "{NAME}::verifyIssued(): the token {said}. Expiry is not optional \
         (`rule:security/jwt-expiry-is-mandatory`) and `leeway` widens the window at both ends \
         rather than removing it."
    ))
}

/// The refusal a registered claim that is not what was asked for gets, naming
/// the claim.
///
/// Safe for [`outside_the_window`]'s reason and no other: it is reached past the
/// signature check, so it tells a forger nothing it could not have written
/// itself.
fn claim_refused(said: &str) -> Fault {
    Fault::thrown(format!(
        "{NAME}::verifyIssued(): the token is authentic and {said}. A claim this member checks \
         is one the issuer and this program had to agree on beforehand, so a mismatch is a \
         token meant for somebody else rather than a forgery."
    ))
}

/// A `Duration` option in whole seconds, and [`None`] for one the call omitted.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot holding something that is not a `Duration`,
/// which [`crate::time::nanos_of`]'s own doc says no program reaches.
fn seconds_option(args: &[Value], slot: usize, member: &str) -> Result<Option<i64>, Fault> {
    if matches!(args[slot].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    Ok(Some(
        crate::time::nanos_of(args, slot, member)? / 1_000_000_000,
    ))
}

/// `{leeway: …}` in whole seconds, bounded on both sides.
///
/// # Errors
///
/// A `LogicError` for a negative leeway — a window that closes before it opens
/// — and for one wider than [`MAX_LEEWAY`], which is the bound that keeps this
/// option from becoming the flag `rule:security/jwt-expiry-is-mandatory`
/// refuses to have.
fn leeway_of(args: &[Value], slot: usize) -> Result<i64, Fault> {
    let Some(seconds) = seconds_option(args, slot, "verifyIssued")? else {
        return Ok(DEFAULT_LEEWAY);
    };
    if !(0..=MAX_LEEWAY).contains(&seconds) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::verifyIssued(): {{leeway: …}} is {seconds}s, and it is a clock \
                 disagreement rather than an extension of the token's life — it is refused \
                 below zero and above {MAX_LEEWAY}s."
            ),
        ));
    }
    Ok(seconds)
}

/// The one key that may have signed this token, and the JWS algorithm its kind
/// carries.
///
/// **A key is found, never tried.** A `kid` is a lookup into the set and selects
/// nothing else, and a token carrying none is answered only by a set holding
/// exactly one key — so a token costs at most one signature check whatever the
/// set holds. A `Core\Crypto\PublicKey` argument is that same rule with the set
/// of one written by the program instead.
///
/// # Errors
///
/// [`not_issued`]'s one sentence where the set names no such key, a `LogicError`
/// for an `X25519` key, which verifies nothing, and a [`Fault::fatal`] for held
/// material that no longer parses — unreachable from source, since it is what a
/// reader of this module already read once.
fn verifying_key(
    args: &[Value],
    slot: usize,
    kid: Option<&str>,
) -> Result<(PublicKey, &'static str), Fault> {
    let (spki, kind) = if crate::instance::is_instance(args[slot], &KEY_SET) {
        let receiver = crate::instance::receiver(args[slot], &KEY_SET, "verifyIssued")?;
        let held = crate::instance::slot(receiver, KEY_SET_KEYS_SLOT);
        // Unreachable from source: `Core\Jwt\KeySet::read` is the only writer of
        // this slot and what it writes is the frame `framed` answered.
        let blob = held.as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::verifyIssued expected a `bytes` in the set's `keys` slot, got tag {}",
                held.tag_byte()
            ))
        })?;
        let keys = keys_in(blob).ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::verifyIssued held a `{KEY_SET_NAME}` whose frame it cannot read back"
            ))
        })?;
        let found = match kid {
            Some(kid) => keys.iter().find(|(named, _, _)| *named == Some(kid)),
            None => keys.first().filter(|_| keys.len() == 1),
        };
        let Some((_, kind, spki)) = found else {
            return Err(not_issued());
        };
        (spki.to_vec(), *kind)
    } else {
        let (held, kind) = crypto::stored_key(args, slot, &crypto::PUBLIC_KEY, "verifyIssued")?;
        let spki = crypto::stored_octets(&held, &crypto::PUBLIC_KEY, "verifyIssued")?;
        (spki.to_vec(), kind)
    };

    let alg = pair_alg(kind).ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::verifyIssued(): an `X25519` key verifies nothing — it is a key for \
                 Core\\Crypto::agree alone. A `P256` key checks ES256, an `Ed25519` key EdDSA, \
                 and the two RSA kinds RS256 and PS256."
            ),
        )
    })?;
    let key = PublicKey::read(&spki, kind, KeyFormat::Spki).map_err(|_| {
        Fault::fatal(format!(
            "{NAME}::verifyIssued held key material that is no longer a public key of its kind"
        ))
    })?;
    Ok((key, alg))
}

/// Whether a token's `typ` is the one a call asked for — RFC 7519 § 5.1's
/// comparison, which is case-insensitive with any `application/` prefix
/// dropped, so `at+jwt`, `AT+JWT` and `application/at+jwt` are one value.
fn same_typ(got: &str, want: &str) -> bool {
    let bare = |value: &str| {
        let lower = value.to_ascii_lowercase();
        lower
            .strip_prefix("application/")
            .unwrap_or(&lower)
            .to_owned()
    };
    bare(got) == bare(want)
}

/// Whether `aud` names this audience, and whether an `azp` is owed for it.
///
/// A token addressed to more than one audience requires `azp` to equal the one
/// this program is: OpenID Connect's own rule, and the case where a token
/// issued for a different party's use would otherwise be accepted here.
fn addressed_to(claims: &serde_json::Map<String, serde_json::Value>, audience: &str) -> bool {
    match claims.get("aud") {
        Some(serde_json::Value::String(one)) => one == audience,
        Some(serde_json::Value::Array(many)) => {
            let named = many
                .iter()
                .filter_map(serde_json::Value::as_str)
                .any(|one| one == audience);
            let authorized = many.len() == 1
                || claims.get("azp").and_then(serde_json::Value::as_str) == Some(audience);
            named && authorized
        }
        _ => false,
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwt::verifyIssued<T>(string $token, Jwt\KeySet|Crypto\PublicKey $keys, string $issuer, string $audience, {leeway?, typ?, nonce?, maxAge?}): T`
    /// — the read half for a token this program did not sign.
    ///
    /// **Arguments 0 to 2 are what the call site wrote as its type argument**,
    /// not values: `crate::registry::WRITTEN_CLASS_MEMBERS` puts this member on
    /// the roster whose helper is handed a `nvs_runtime::ClassDesc`, the
    /// `array<...>` flag and an inline shape's wire contract ahead of its
    /// declared parameters, and that roster's docs own why. So the arity here is
    /// three more than the registry row's.
    ///
    /// **The order is the whole design.** Shape, header policy, key, signature —
    /// and then the clock and the claims, which are reached only under a
    /// signature that held. Everything before that line answers with
    /// [`not_issued`]'s one sentence, because a forger chooses what it sees and
    /// telling one forgery from another is an oracle; everything after it may
    /// say what is wrong, because the only reader who gets there is holding a
    /// token the issuer really signed.
    ///
    /// The claims are handed to `T` by [`crate::json::decode_as`] and not by a
    /// second decoder written here, so what a token's payload may say is exactly
    /// what a JSON document may say. `rule:security/derived-codec-qualifiers`
    /// has already made the call site declare `tainted` on every text field
    /// reachable from `T`, which is how
    /// `rule:security/verification-does-not-launder` survives a structured
    /// answer.
    fn nvs_core_jwt_verify_issued(ctx, args: [11]) {
        // Unreachable from source, on `Core\Json::decodeAs`'s reasoning:
        // arguments 0 to 2 are not a program's values but the constants
        // `nvs_ir::lower` writes out of the type argument, and a call naming
        // none is `E0442` before any of this runs.
        let class = args[0].as_class_desc().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Jwt::verifyIssued` was called with no class in argument 0",
        ))?;
        // Unreachable from source for the same reason and refused by the same
        // `E0442`: slot 1 is the `ConstBool` the lowering emits beside the
        // descriptor, so a call that has one has the other.
        let list = args[1].as_bool().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Jwt::verifyIssued` was called with no list flag in argument 1",
        ))?;
        let shape = args[2].as_shape_codec();
        let token = text_at(args, 3, "verifyIssued", "$token")?;
        let issuer = text_at(args, 5, "verifyIssued", "$issuer")?;
        let audience = text_at(args, 6, "verifyIssued", "$audience")?;
        let leeway = leeway_of(args, 7)?;
        let want_typ = args[8].as_text();
        let want_nonce = args[9].as_text();
        let max_age = seconds_option(args, 10, "verifyIssued")?;

        // The shape question, asked first because it is a question about the
        // program rather than about the token: a payload is one JSON object, so
        // there is no document a list could have decoded from.
        if list {
            return Err(Fault::thrown_as(ThrownClass::Logic, format!(
                "{NAME}::verifyIssued(): a token carries one payload object, so `<array<…>>` \
                 names a document no token has. Write the type one token's claims decode into."
            )));
        }
        if token.len() > MAX_TOKEN_LEN {
            return Err(not_issued());
        }

        let mut parts = token.split('.');
        let (Some(header_b64), Some(payload_b64), Some(signature_b64), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(not_issued());
        };

        // The header is read to *compare* and to refuse, never to choose.
        let header_json = URL_SAFE_NO_PAD.decode(header_b64).map_err(|_| not_issued())?;
        let header: serde_json::Value =
            serde_json::from_slice(&header_json).map_err(|_| not_issued())?;
        let Some(header) = header.as_object() else {
            return Err(not_issued());
        };
        if REFUSED_HEADER.iter().any(|name| header.contains_key(*name)) {
            return Err(not_issued());
        }
        let Some(alg) = header.get("alg").and_then(serde_json::Value::as_str) else {
            return Err(not_issued());
        };

        let kid = header.get("kid").and_then(serde_json::Value::as_str);
        let (key, want_alg) = verifying_key(args, 4, kid)?;
        if alg != want_alg {
            return Err(not_issued());
        }

        let signature = URL_SAFE_NO_PAD.decode(signature_b64).map_err(|_| not_issued())?;
        let signing_input = &token[..header_b64.len() + 1 + payload_b64.len()];
        let verifying = key.verifying().ok_or_else(not_issued)?;
        crypto::verify_signature(&verifying, signing_input.as_bytes(), &signature)
            .ok_or_else(not_issued)?;

        // Past here the token is authentic.
        let payload_json = URL_SAFE_NO_PAD.decode(payload_b64).map_err(|_| not_issued())?;
        let payload: serde_json::Value =
            serde_json::from_slice(&payload_json).map_err(|_| not_issued())?;
        let Some(claims) = payload.as_object() else {
            return Err(not_issued());
        };

        let now = now_seconds(ctx, "verifyIssued")?;
        let exp = claims
            .get("exp")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| outside_the_window("carries no `exp`"))?;
        if now >= exp.saturating_add(leeway) {
            return Err(outside_the_window(&format!(
                "expired at {exp}, and it is now {now} with {leeway}s of leeway"
            )));
        }
        if let Some(nbf) = claims.get("nbf").and_then(serde_json::Value::as_i64)
            && now < nbf.saturating_sub(leeway)
        {
            return Err(outside_the_window(&format!(
                "is not valid before {nbf}, and it is now {now} with {leeway}s of leeway"
            )));
        }

        if claims.get("iss").and_then(serde_json::Value::as_str) != Some(issuer) {
            return Err(claim_refused(&format!(
                "its `iss` is not `{issuer}`, which is the issuer this call named"
            )));
        }
        if !addressed_to(claims, audience) {
            return Err(claim_refused(&format!(
                "its `aud` does not name `{audience}` — or it names several audiences and its \
                 `azp` is not this one"
            )));
        }
        if let Some(want) = want_typ {
            let got = header.get("typ").and_then(serde_json::Value::as_str);
            if !got.is_some_and(|got| same_typ(got, want)) {
                return Err(claim_refused(&format!(
                    "its header's `typ` is not `{want}`"
                )));
            }
        }
        if let Some(want) = want_nonce {
            let got = claims.get("nonce").and_then(serde_json::Value::as_str);
            let matched = got
                .is_some_and(|got| bool::from(got.as_bytes().ct_eq(want.as_bytes())));
            if !matched {
                return Err(claim_refused("its `nonce` is not the one this call asked for"));
            }
        }
        if let Some(oldest) = max_age {
            let authenticated = claims
                .get("auth_time")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| claim_refused(
                    "carries no `auth_time`, which is what a `maxAge` is asked of",
                ))?;
            if now.saturating_sub(authenticated) > oldest.saturating_add(leeway) {
                return Err(claim_refused(&format!(
                    "was authenticated at {authenticated}, which is older than the {oldest}s \
                     this call allows"
                )));
            }
        }

        let document = std::str::from_utf8(&payload_json).map_err(|_| not_issued())?;
        #[expect(
            unsafe_code,
            reason = "the descriptor and the contract came out of the constants a \
                      compiled unit owns, so both outlive this call and every \
                      object made from it"
        )]
        unsafe {
            crate::json::decode_as(
                ctx,
                class,
                shape,
                document,
                crate::json::DEFAULT_MAX_DEPTH_U32,
                false,
                "Core\\Jwt::verifyIssued",
            )
        }
    }
}

/// One key a document offered and this class admitted.
struct Admitted {
    /// The name the document gave it, and `None` for a key carrying none —
    /// which is only ever found in a set holding exactly one key.
    kid: Option<String>,
    /// The kind it was read as, which is the whole of its algorithm.
    kind: KeyKind,
    /// Its `SubjectPublicKeyInfo`, which is what a set stores.
    spki: Vec<u8>,
}

/// The refusal a whole document gets, saying what is wrong with it.
///
/// Not [`refused`]'s one sentence, and the module doc's *a key set is an
/// admission* section is the home of why the two differ: a forged token is an
/// attacker's, so telling one forgery from another is an oracle, while a JWKS
/// document is one this program went and fetched.
fn not_a_key_set(said: &str) -> Fault {
    Fault::thrown(format!(
        "{KEY_SET_NAME}::read(): $jwks is not a document to read keys from — {said}. A key this \
         roster has no use for is skipped instead, so what is refused here is the document \
         rather than a key in it."
    ))
}

/// Whether `alg` is one of the four algorithms this roster signs with,
/// whatever key carries it.
///
/// Read off [`pair_alg`] rather than listed a second time, so the set of
/// algorithms a document may name and the set a key may be read under cannot
/// drift apart.
fn ours(alg: &str) -> bool {
    [
        KeyKind::P256,
        KeyKind::Ed25519,
        KeyKind::RsaPkcs1,
        KeyKind::RsaPss,
    ]
    .into_iter()
    .any(|kind| pair_alg(kind) == Some(alg))
}

/// One entry of a document's `keys` array: the key it is, `None` for a key this
/// roster has no use for, or the refusal that ends the whole document.
///
/// The order is what the two answers are worth. A private member is checked
/// before anything else, because a document publishing one is wrong whatever
/// the rest of the key says; the purpose and the kind come next, and both of
/// them only ever *skip*; and the material is read last, through
/// `Core\Crypto\PublicKey`'s own JWK reader, so a key admitted here is exactly
/// as validated as one the program read itself.
///
/// # Errors
///
/// A [`not_a_key_set`] for each of the document-wide refusals, and a
/// [`Fault::fatal`] for a key that parsed and will not write back out, which is
/// this crate's own codec rather than anything in the document.
fn admitted_key(
    entry: &serde_json::Value,
    scheme: Option<KeyKind>,
) -> Result<Option<Admitted>, Fault> {
    let members = entry
        .as_object()
        .ok_or_else(|| not_a_key_set("its `keys` array holds something that is not a key"))?;
    if let Some(name) = PRIVATE_MEMBERS
        .iter()
        .find(|name| members.contains_key(**name))
    {
        return Err(not_a_key_set(&format!(
            "a key in it carries `{name}`, which is a private key published as a public one"
        )));
    }

    let text = |name: &str| members.get(name).and_then(serde_json::Value::as_str);
    // A key marked for anything other than signing is one this class has no
    // use for. `enc` is the case a real document carries and the reasoning is
    // the same for any other: a purpose nothing here reads is a purpose
    // nothing here should be reading a key for.
    if members.contains_key("use") && text("use") != Some("sig") {
        return Ok(None);
    }

    let alg = text("alg");
    let kind = if text("kty") == Some("RSA") && text("crv").is_none() {
        match alg {
            Some("RS256") => KeyKind::RsaPkcs1,
            Some("PS256") => KeyKind::RsaPss,
            Some(alg) if ours(alg) => return Err(carries_a_foreign_alg(alg)),
            Some(_) => return Ok(None),
            None => scheme.ok_or_else(|| {
                not_a_key_set(
                    "an RSA key in it carries no `alg`, and RS256 and PS256 are one key type \
                     under two schemes — name one as `{rsaScheme: …}`, because a key read under \
                     the scheme it was not issued for verifies nothing and says nothing about \
                     why",
                )
            })?,
        }
    } else {
        let served = match (text("kty"), text("crv")) {
            (Some("EC"), Some("P-256")) => KeyKind::P256,
            (Some("OKP"), Some("Ed25519")) => KeyKind::Ed25519,
            // Every other kind — P-384, X25519, an `oct` secret — is one this
            // roster does not sign with, and the document keeps its other
            // keys.
            _ => return Ok(None),
        };
        match alg {
            None => served,
            Some(alg) if pair_alg(served) == Some(alg) => served,
            Some(alg) if ours(alg) => return Err(carries_a_foreign_alg(alg)),
            Some(_) => return Ok(None),
        }
    };

    let kid = match members.get("kid") {
        None => None,
        Some(serde_json::Value::String(kid)) => Some(kid.clone()),
        Some(_) => {
            return Err(not_a_key_set(
                "a key in it carries a `kid` that is not text, and a `kid` is the name a token \
                 asks for a key by",
            ));
        }
    };

    // Written back out and read through the one JWK reader this crate has,
    // rather than picked apart here: the curve check, the coordinate widths
    // and the RSA modulus bounds are that reader's, so a key in a set is
    // validated by the same code as a key a program read itself.
    let octets = serde_json::to_vec(entry).map_err(|_| {
        Fault::fatal(format!(
            "{KEY_SET_NAME}::read could not write back a key it had just parsed"
        ))
    })?;
    let key = PublicKey::read(&octets, kind, KeyFormat::Jwk).map_err(|_| {
        not_a_key_set(
            "a key in it is not a key of the kind it says it is — a point off the curve, a \
             coordinate of the wrong width, or an RSA modulus outside the 2048 to 8192 bits \
             this roster admits",
        )
    })?;
    let spki = key.write(KeyFormat::Spki).map_err(|_| {
        Fault::fatal(format!(
            "{KEY_SET_NAME}::read could not write the SPKI of a key it had just read"
        ))
    })?;
    Ok(Some(Admitted { kid, kind, spki }))
}

/// The refusal for a key marked with one of this roster's algorithms that its
/// own kind cannot carry.
///
/// A document disagreeing with itself, and the one `alg` reading that is not a
/// skip: an algorithm we do not have says the key is for something else, while
/// one we do have on a kind that cannot carry it says the document is wrong
/// about its own key.
fn carries_a_foreign_alg(alg: &str) -> Fault {
    not_a_key_set(&format!(
        "a key in it is marked `{alg}`, which is not the algorithm its own kind carries"
    ))
}

/// Every key a document offers, admitted, in the order it offered them.
///
/// # Errors
///
/// A [`not_a_key_set`] for a document that is not one, for any of
/// [`admitted_key`]'s refusals, and for the two the set as a whole has: two
/// keys under one name, and more keys than [`MAX_KEYS`].
fn admitted_set(jwks: &str, scheme: Option<KeyKind>) -> Result<Vec<Admitted>, Fault> {
    let document: serde_json::Value =
        serde_json::from_str(jwks).map_err(|_| not_a_key_set("it is not JSON"))?;
    let keys = document
        .get("keys")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            not_a_key_set(
                "it carries no `keys` array, which is the one member a JWKS document has (RFC \
                 7517 § 5)",
            )
        })?;

    let mut admitted: Vec<Admitted> = Vec::new();
    let mut named: BTreeSet<String> = BTreeSet::new();
    for entry in keys {
        let Some(key) = admitted_key(entry, scheme)? else {
            continue;
        };
        if let Some(kid) = &key.kid
            && !named.insert(kid.clone())
        {
            return Err(not_a_key_set(&format!(
                "two of its keys are named `{kid}`, so a token asking for that key names two, and \
                 which one signed it is not a question a verifier may answer by trying both"
            )));
        }
        if admitted.len() == MAX_KEYS {
            return Err(not_a_key_set(&format!(
                "it offers more than {MAX_KEYS} keys this class can use, and a set is capped \
                 there so that a token costs one signature check whatever the document holds"
            )));
        }
        admitted.push(key);
    }
    Ok(admitted)
}

/// The `kid` length no key has, which is how a key carrying none is written.
const NO_KID: u32 = u32::MAX;

/// A kind as the one octet the frame carries.
///
/// The frame's own spelling rather than [`KeyKind::tag`]'s, paired with
/// [`kind_of_octet`]: the ABI tag is what compiled code writes into an argument
/// slot, and a private encoding that borrowed it would be a second reason that
/// number can never change.
const fn kind_octet(kind: KeyKind) -> u8 {
    match kind {
        KeyKind::P256 => 0,
        KeyKind::X25519 => 1,
        KeyKind::Ed25519 => 2,
        KeyKind::RsaPkcs1 => 3,
        KeyKind::RsaPss => 4,
    }
}

/// [`kind_octet`]'s inverse, and `None` for an octet it never wrote.
const fn kind_of_octet(octet: u8) -> Option<KeyKind> {
    Some(match octet {
        0 => KeyKind::P256,
        1 => KeyKind::X25519,
        2 => KeyKind::Ed25519,
        3 => KeyKind::RsaPkcs1,
        4 => KeyKind::RsaPss,
        _ => return None,
    })
}

/// The admitted keys as the one `bytes` a set's slot holds, or `None` for a
/// `kid` or a key longer than a four-octet length can name.
///
/// Per key: [`kind_octet`], then the `kid` and the SPKI, each behind a
/// four-octet big-endian length, with an absent `kid` written as [`NO_KID`].
/// A slot holds one value and a set holds several keys, so they are written
/// into one rather than built as an array of objects nothing would be allowed
/// to read back out.
fn framed(keys: &[Admitted]) -> Option<Vec<u8>> {
    let mut blob = Vec::new();
    for key in keys {
        blob.push(kind_octet(key.kind));
        match &key.kid {
            Some(kid) => {
                let len = u32::try_from(kid.len()).ok().filter(|len| *len != NO_KID)?;
                blob.extend_from_slice(&len.to_be_bytes());
                blob.extend_from_slice(kid.as_bytes());
            }
            None => blob.extend_from_slice(&NO_KID.to_be_bytes()),
        }
        blob.extend_from_slice(&u32::try_from(key.spki.len()).ok()?.to_be_bytes());
        blob.extend_from_slice(&key.spki);
    }
    Some(blob)
}

/// The four-octet length at `at`, or `None` for a blob that ends inside it.
fn length_at(blob: &[u8], at: usize) -> Option<u32> {
    let four: [u8; 4] = blob.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_be_bytes(four))
}

/// One key as a set holds it: the name the document gave it, the kind it was
/// read as and its material, borrowed out of the frame rather than copied.
type Held<'a> = (Option<&'a str>, KeyKind, &'a [u8]);

/// The keys a set holds, as [`framed`] wrote them, and `None` for a blob it did
/// not write.
///
/// Borrowing rather than owning: a lookup reads one key out of a set and the
/// set outlives the call, so nothing here is copied to answer a question about
/// it.
fn keys_in(blob: &[u8]) -> Option<Vec<Held<'_>>> {
    let mut out = Vec::new();
    let mut at = 0_usize;
    while at < blob.len() {
        let kind = kind_of_octet(*blob.get(at)?)?;
        at = at.checked_add(1)?;
        let named = length_at(blob, at)?;
        at = at.checked_add(4)?;
        let kid = if named == NO_KID {
            None
        } else {
            let len = usize::try_from(named).ok()?;
            let kid = std::str::from_utf8(blob.get(at..at.checked_add(len)?)?).ok()?;
            at = at.checked_add(len)?;
            Some(kid)
        };
        let len = usize::try_from(length_at(blob, at)?).ok()?;
        at = at.checked_add(4)?;
        let spki = blob.get(at..at.checked_add(len)?)?;
        at = at.checked_add(len)?;
        out.push((kid, kind, spki));
    }
    Some(out)
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwt\KeySet::read(tainted string $jwks, {rsaScheme?}): Jwt\KeySet`
    /// — the admission, and the only place this class decides anything about a
    /// key.
    ///
    /// Every rule about what may be in the set is applied here, so a
    /// verification is a lookup and a signature check and nothing else. The
    /// module doc's *a key set is an admission* section is the home of why a
    /// key is skipped where a document is refused, and of what a set spends.
    fn nvs_core_jwt_key_set_read(_ctx, args: [2]) {
        // Unreachable from source: the row declares a `string`, so `nvs_types`
        // refuses another tag at `E0401`.
        let jwks = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{KEY_SET_NAME}::read expected a `string` for $jwks, got tag {}",
                args[0].tag_byte()
            ))
        })?;

        let scheme = match args[1].as_int() {
            None => None,
            Some(tag) => {
                // Unreachable from source for the reason above: the option is
                // a closed enum, so an argument is one of its cases.
                let kind = KeyKind::from_tag(tag).ok_or_else(|| {
                    Fault::fatal(format!(
                        "{KEY_SET_NAME}::read expected a `{}` case for $rsaScheme, got tag {tag}",
                        crypto::KEY_KIND_NAME
                    ))
                })?;
                if !matches!(kind, KeyKind::RsaPkcs1 | KeyKind::RsaPss) {
                    return Err(Fault::thrown_as(
                        ThrownClass::Logic,
                        format!(
                            "{KEY_SET_NAME}::read(): {{rsaScheme: …}} names a kind that is not \
                             an RSA one, and it is the scheme an RSA key carrying no `alg` is \
                             read under — RS256 and PS256 being the two one key type carries. \
                             Every other kind names its own algorithm, so there is nothing \
                             here for it to choose."
                        ),
                    ));
                }
                Some(kind)
            }
        };

        let admitted = admitted_set(jwks, scheme)?;
        let blob = framed(&admitted).ok_or_else(|| {
            Fault::fatal(format!(
                "{KEY_SET_NAME}::read admitted a key whose `kid` or SPKI is longer than a set \
                 can name"
            ))
        })?;
        // Asked once with the real number, as `sign` does: the set's size is
        // known exactly here, and it is what the object goes on holding.
        nvs_runtime::affordable(Some(blob.len()), "Core\\Jwt\\KeySet::read")?;
        Ok(crate::instance::build(&KEY_SET, [Value::bytes(NvsStr::new(&blob))]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::vectors as webcrypto;

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

    /// Every key set the frozen vector file carries is admitted or refused
    /// exactly as the script that wrote it judged, and an admitted one holds
    /// exactly the keys it names, in order.
    ///
    /// The file is the proof rather than a table here, for
    /// [`crate::tests::vectors`]'s reason: the set's JWKS documents are the
    /// ones a browser's own exports assemble into, and its `kids` are what an
    /// independent reading of the same rules admitted from them.
    #[test]
    fn every_frozen_key_set_is_admitted_or_refused_as_the_set_says() {
        let sets = webcrypto::node("/jws/keySets")
            .as_array()
            .expect("the vector set writes /jws/keySets as an array");
        assert!(!sets.is_empty(), "the set carries key-set cases at all");

        for case in sets {
            let name = webcrypto::text(case, "/name");
            let scheme = case
                .pointer("/rsaScheme")
                .and_then(serde_json::Value::as_str)
                .map(|alg| match alg {
                    "RS256" => KeyKind::RsaPkcs1,
                    "PS256" => KeyKind::RsaPss,
                    other => {
                        panic!("{name} names an rsaScheme this roster has no kind for: {other}")
                    }
                });
            let read = admitted_set(webcrypto::text(case, "/jwks"), scheme);

            if webcrypto::text(case, "/outcome") == "refused" {
                assert!(read.is_err(), "{name} is refused whole");
                continue;
            }

            let admitted = read.unwrap_or_else(|_| panic!("{name} is admitted"));
            let wanted: Vec<Option<&str>> = case
                .pointer("/kids")
                .and_then(serde_json::Value::as_array)
                .expect("an admitted case names the kids it admits")
                .iter()
                .map(serde_json::Value::as_str)
                .collect();
            let named: Vec<Option<&str>> = admitted.iter().map(|key| key.kid.as_deref()).collect();
            assert_eq!(named, wanted, "{name} admits exactly the keys it names");
        }
    }

    /// A set writes every key it admitted and reads all of them back — the
    /// `kid`, the kind and the material of each, and nothing between two keys
    /// that the frame cannot tell apart.
    #[test]
    fn a_set_reads_back_every_key_it_framed() {
        let case = webcrypto::node("/jws/keySets")
            .as_array()
            .and_then(|sets| sets.first())
            .expect("the first key-set case is the wide one");
        let admitted = admitted_set(webcrypto::text(case, "/jwks"), Some(KeyKind::RsaPss))
            .expect("the wide case is admitted");
        assert!(
            admitted.len() > 1,
            "the wide case holds several keys, so the frame is asked to separate them"
        );

        let blob = framed(&admitted).expect("a key set of this size frames");
        let read = keys_in(&blob).expect("what this module framed, this module reads");
        assert_eq!(read.len(), admitted.len(), "every key comes back");
        for (held, (kid, kind, spki)) in admitted.iter().zip(read) {
            assert_eq!(kid, held.kid.as_deref(), "the kid comes back as it went in");
            assert_eq!(kind, held.kind, "the kind comes back as it went in");
            assert_eq!(spki, held.spki, "the material comes back as it went in");
            // And it is still a key, read under the kind the frame carried:
            // storing an encoding nothing reads back would be a set that
            // admits keys it cannot use.
            assert!(
                PublicKey::read(spki, kind, KeyFormat::Spki).is_ok(),
                "a framed key is a key of its own kind"
            );
        }

        // A key carrying no `kid` is written as one, rather than as an empty
        // name a token could ask for.
        let none = framed(&[Admitted {
            kid: None,
            kind: KeyKind::P256,
            spki: vec![7, 7, 7],
        }])
        .expect("one key frames");
        let empty = framed(&[Admitted {
            kid: Some(String::new()),
            kind: KeyKind::P256,
            spki: vec![7, 7, 7],
        }])
        .expect("one key frames");
        assert_ne!(none, empty, "no kid and an empty kid are different keys");
        assert_eq!(keys_in(&none).expect("it reads back")[0].0, None);
        assert_eq!(keys_in(&empty).expect("it reads back")[0].0, Some(""));
    }

    /// The frame's octet and the kind it stands for are one relation, read in
    /// both directions, and no octet outside it reads as a kind.
    #[test]
    fn every_kind_survives_the_frames_own_octet() {
        let kinds = [
            KeyKind::P256,
            KeyKind::X25519,
            KeyKind::Ed25519,
            KeyKind::RsaPkcs1,
            KeyKind::RsaPss,
        ];
        let written: Vec<u8> = kinds.iter().copied().map(kind_octet).collect();
        let back: Vec<Option<KeyKind>> = written.iter().copied().map(kind_of_octet).collect();
        assert_eq!(
            back,
            kinds.map(Some).to_vec(),
            "every kind reads back as itself"
        );
        assert_eq!(
            written.iter().collect::<BTreeSet<_>>().len(),
            kinds.len(),
            "no two kinds share an octet"
        );
        assert_eq!(
            kind_of_octet(u8::MAX),
            None,
            "an octet it never wrote is not a kind"
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
