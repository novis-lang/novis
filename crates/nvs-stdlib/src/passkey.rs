//! `Core\Passkey` — `rule:core-classes/passkey-verifies-a-webauthn-ceremony`: the options for a
//! WebAuthn registration, and the check of what the browser answers, for ES256 and Ed25519
//! credentials and without verifying attestation.
//!
//! The rule places the class and states what is checked. What belongs here is where each check
//! reads from, which crate parses which bytes, and what the class keeps between the two requests
//! of one ceremony.
//!
//! # The challenge is the one thing kept, and it is kept in the session
//!
//! [`nvs_core_passkey_register_options`] draws [`CHALLENGE_LEN`] octets through
//! [`crate::random::draw`] and saves them in the started session under [`REGISTER_KEY`], behind
//! the second they were issued at; [`nvs_core_passkey_sign_in_options`] does the same under
//! [`SIGN_IN_KEY`], so a registration and a sign-in in flight in one session do not take each
//! other's challenge. [`nvs_core_passkey_verify_registration`] and
//! [`nvs_core_passkey_verify_sign_in`] **take theirs out before they read a byte of the
//! response**, so a replayed response, a second check of one response and a
//! check that fails all find no challenge the next time. A challenge older than
//! [`TIMEOUT_SECONDS`] is refused; that is the `timeout` the options tell the browser, so a ceremony
//! the browser has abandoned cannot be completed later. The session is
//! `rule:core-classes/session-is-started-explicitly`'s, so a member called before
//! `Core\Session::start` throws naming it, as every `Core\Session` member does.
//!
//! **What this spends:** [`SAVED_LEN`] octets in the session record per ceremony in flight, which
//! is one per visitor and per ceremony kind, until it is checked.
//!
//! # Nothing the browser sent decides what is checked
//!
//! The origin and the relying-party id are the program's arguments. The client data's `origin`
//! is compared against the first, and the authenticator data's relying-party hash against
//! SHA-256 of the second. The client data's `type` is compared against the ceremony this member
//! is, and `crossOrigin: true` is refused, because a ceremony run inside another site's frame is
//! not one the program asked for. The user-present and user-verified flags are both required,
//! because the options ask for user verification and a passkey without it is a possession factor
//! alone.
//!
//! # CBOR is `ciborium`, and the key is `coset`
//!
//! The attestation object is CBOR (RFC 8949), read with `ciborium`, and its attestation
//! statement is read only far enough to be a map: attestation is not verified (the rule's
//! *Declined* list says why). The authenticator data is a fixed binary layout, read here. The
//! credential's public key inside it is a COSE key (RFC 9052), cut out of the authenticator data by
//! `ciborium` reading exactly one item and then read with `coset`. The COSE bytes are what the
//! [`CREDENTIAL`] keeps, unchanged, so a stored credential is the authenticator's own encoding.
//!
//! A key is ES256 (EC2, P-256, COSE algorithm −7) or Ed25519 (OKP, Ed25519, −8 or −19); any other
//! key is refused at registration. A P-256 point is checked to be on the curve by
//! [`crate::crypto::read_p256_point`], which is where every P-256 point in this crate is checked.
//! An Ed25519 point has no cheaper check than the signature it will verify, so a point that is not
//! one fails at sign-in.
//!
//! # A sign-in is one signature, checked by `Core\Crypto`'s code
//!
//! The assertion's signature covers `authenticatorData ‖ SHA-256(clientDataJSON)` and is checked
//! by [`crate::crypto::verify_signature`]. An ES256 assertion signs in ASN.1 DER, and JWS signs
//! in `r ‖ s`, so the key is [`crate::crypto::VerifyingKey::P256Der`] and not the variant a JWT
//! reads. The sign counter is the authenticator's clone detector: a count that does not rise
//! throws, unless both the stored and the sent count are zero, which is what a device that keeps
//! no counter sends every time.
//!
//! # A credential is an instance with four readers
//!
//! `Passkey\Credential` holds the id, the COSE key, the algorithm and the sign counter in four
//! slots, read back by four members of those names: a `Core` instance has no property a program
//! can reach ([`crate::registry::CoreTy::Instance`]). The application saves the four values
//! wherever it keeps users, and [`nvs_core_passkey_credential_of`] builds a credential from them
//! again, checking the key against the algorithm as registration did. The algorithm enum belongs
//! to the credential, which is the one class that takes or returns it
//! (`rule:enums/an-enum-lives-in-the-class-that-uses-it`).

use base64::Engine as _;
use base64::alphabet;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use ciborium::Value as Cbor;
use coset::iana::EnumI64 as _;
use coset::{CborSerializable as _, CoseKey, Label, RegisteredLabel, RegisteredLabelWithPrivate};
use rand::Rng as _;
use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;

use nvs_runtime::{Ctx, Fault, NvsStr, ThrownClass, Value};

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Passkey";

/// The credential value's class name.
const CREDENTIAL_NAME: &str = r"Core\Passkey\Credential";

/// The credential's algorithm enum, owned by the credential.
const ALGORITHM_NAME: &str = r"Core\Passkey\Credential::Algorithm";

/// The octets of a challenge: 256 bits, twice WebAuthn's floor of 16 octets.
const CHALLENGE_LEN: usize = 32;

/// The seconds a ceremony may take, which the options send the browser as `timeout`.
const TIMEOUT_SECONDS: i64 = 300;

/// The octets a challenge takes in the session: the second it was issued, big-endian, then the
/// challenge.
const SAVED_LEN: usize = 8 + CHALLENGE_LEN;

/// The session key a registration's challenge is saved under. It names the member that saves it,
/// so a program reading its own session's keys sees where this one came from.
const REGISTER_KEY: &str = r"Core\Passkey::registerOptions";

/// The session key a sign-in's challenge is saved under, named as [`REGISTER_KEY`] is.
const SIGN_IN_KEY: &str = r"Core\Passkey::signInOptions";

/// The longest response a member reads, in octets. A registration response with the longest
/// credential id WebAuthn allows and a large attestation certificate chain is under a quarter of
/// this, so the bound refuses only what no browser sends.
const MAX_RESPONSE: usize = 64 * 1024;

/// WebAuthn's longest credential id, in octets.
const MAX_ID: usize = 1023;

/// WebAuthn's longest user handle, in octets.
const MAX_USER_ID: usize = 64;

/// More than one `{"type":"public-key","id":…}` entry of the options costs beside its id: its
/// `serde_json` map with two keys and two values, and its text. [`credential_ids`] charges it.
const ENTRY_COST: usize = 256;

/// More than the options cost beside their ids and the program's text: the challenge, the fixed
/// keys and values, their `serde_json` nodes and the user handle.
const OPTIONS_COST: usize = 2048;

/// What the options cost for each byte of a text the program passed: one in the `serde_json` tree,
/// and up to six in the JSON, where a control character is written `\u0000`.
const TEXT_COST: usize = 7;

/// The authenticator data's user-present flag.
const USER_PRESENT: u8 = 0x01;

/// The authenticator data's user-verified flag.
const USER_VERIFIED: u8 = 0x04;

/// The authenticator data's flag for an attested credential that follows the counter.
const ATTESTED: u8 = 0x40;

/// The authenticator data's flag for an extensions map at its end.
const EXTENSIONS: u8 = 0x80;

/// The slots of a [`CREDENTIAL`], in order.
const ID_SLOT: usize = 0;
/// The credential's COSE key.
const KEY_SLOT: usize = 1;
/// The credential's [`ALGORITHM`] case.
const ALGORITHM_SLOT: usize = 2;
/// The credential's sign counter.
const COUNT_SLOT: usize = 3;

/// Base64url as a browser's `toJSON()` writes it, without padding, read with or without it.
const LENIENT: GeneralPurpose = GeneralPurpose::new(
    &alphabet::URL_SAFE,
    GeneralPurposeConfig::new()
        .with_encode_padding(false)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// `rule:core-classes/passkey-verifies-a-webauthn-ceremony`'s class: both ceremonies.
///
/// Its module doc owns what is kept between the two requests of a ceremony and what each check
/// reads from.
// rank: 975
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "registerOptions",
            names: &["rpId", "rpName", "userId", "userName", "existing"],
            // Contagious: each argument is copied into the JSON the member returns.
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Blob(Qual::Contagious),
                CoreTy::Text(Qual::Contagious),
                CoreTy::Array(&CoreTy::Instance(CREDENTIAL_NAME)),
            ],
            defaults: &[Const::EmptyArray],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_passkey_register_options",
            doc: Some(&REGISTER_OPTIONS_DOC),
        },
        CoreMethod {
            name: "verifyRegistration",
            names: &["response", "rpId", "origin"],
            // The response arrives `tainted` from a request body, and the answer is an object,
            // which carries no qualifier: what crosses out is a key this member has checked. The
            // relying-party id and the origin are only compared.
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(CREDENTIAL_NAME),
            symbol: "nvs_core_passkey_verify_registration",
            doc: Some(&VERIFY_REGISTRATION_DOC),
        },
        CoreMethod {
            name: "signInOptions",
            names: &["rpId", "allowed"],
            // Contagious: the relying-party id is copied into the JSON the member returns.
            params: &[
                CoreTy::Text(Qual::Contagious),
                CoreTy::Array(&CoreTy::Instance(CREDENTIAL_NAME)),
            ],
            defaults: &[Const::EmptyArray],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_passkey_sign_in_options",
            doc: Some(&SIGN_IN_OPTIONS_DOC),
        },
        CoreMethod {
            name: "credentialId",
            names: &["response"],
            // Contagious: the id is the browser's claim, and stays `tainted` until a
            // verification checks the signature it came with.
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_passkey_credential_id_of",
            doc: Some(&CREDENTIAL_ID_DOC),
        },
        CoreMethod {
            name: "verifySignIn",
            names: &["response", "stored", "rpId", "origin"],
            // Neutral as `verifyRegistration`'s are: the answer is a credential whose signature
            // this member has checked.
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Instance(CREDENTIAL_NAME),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Neutral),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(CREDENTIAL_NAME),
            symbol: "nvs_core_passkey_verify_sign_in",
            doc: Some(&VERIFY_SIGN_IN_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Passkey\Credential`: one registered passkey, as the application stores it.
///
/// The module doc's *a credential is an instance with four readers* owns why its values are read
/// through members and built again with `of`.
// rank: 976
pub(crate) const CREDENTIAL: CoreClass = CoreClass {
    name: CREDENTIAL_NAME,
    doc: Some(&CREDENTIAL_CARD),
    methods: &[CoreMethod {
        name: "of",
        names: &["id", "publicKey", "algorithm", "signCount"],
        // Neutral for the octets because a stored credential comes back from wherever the
        // application keeps users, and the answer is an object this member has checked.
        params: &[
            CoreTy::Blob(Qual::Neutral),
            CoreTy::Blob(Qual::Neutral),
            CoreTy::Enum(ALGORITHM_NAME),
            CoreTy::Int,
        ],
        defaults: &[],
        return_ty: CoreTy::Instance(CREDENTIAL_NAME),
        symbol: "nvs_core_passkey_credential_of",
        doc: Some(&OF_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "id",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_passkey_credential_id",
            doc: Some(&ID_DOC),
        },
        CoreMethod {
            name: "publicKey",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_passkey_credential_public_key",
            doc: Some(&PUBLIC_KEY_DOC),
        },
        CoreMethod {
            name: "algorithm",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(ALGORITHM_NAME),
            symbol: "nvs_core_passkey_credential_algorithm",
            doc: Some(&ALGORITHM_DOC),
        },
        CoreMethod {
            name: "signCount",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_passkey_credential_sign_count",
            doc: Some(&SIGN_COUNT_DOC),
        },
    ],
    slots: &["id", "publicKey", "algorithm", "signCount"],
    constants: &[],
};

/// The two signature algorithms a passkey may have. The integers are ABI: they are what the
/// credential's slot holds and what `of` reads back.
pub(crate) const ALGORITHM: CoreEnum = CoreEnum {
    name: ALGORITHM_NAME,
    cases: &[("ES256", 0), ("Ed25519", 1)],
    doc: Some(&ALGORITHM_ENUM_DOC),
};

/// `Core\Passkey`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Passkey sign-in (WebAuthn). A passkey replaces a password. `registerOptions` and \
            `verifyRegistration` add a passkey to an account. `signInOptions` and \
            `verifySignIn` sign the user in with it. In the browser, JavaScript calls \
            `navigator.credentials` with the options.",
};

/// `Core\Passkey::registerOptions`'s reference card — `rule:core-api/reference-card`.
const REGISTER_OPTIONS_DOC: MethodDoc = MethodDoc {
    short: "Returns the options for `navigator.credentials.create()` as JSON. The options contain \
            a new random challenge. The challenge is saved in the session, so call \
            `Core\\Session::start()` first.",
    params: &[
        ParamDoc {
            name: "rpId",
            desc: "The domain of your site, such as `example.com`. The passkey works only on this \
                   domain and its subdomains.",
            shape: &[],
        },
        ParamDoc {
            name: "rpName",
            desc: "The name of your site. The browser shows it to the user.",
            shape: &[],
        },
        ParamDoc {
            name: "userId",
            desc: "A unique id for the account, from 1 to 64 bytes. Do not use an email address \
                   or a name. `Core\\Random::bytes(16)` makes a good id.",
            shape: &[],
        },
        ParamDoc {
            name: "userName",
            desc: "The name the user signs in with, such as an email address. The browser shows \
                   it in its list of passkeys.",
            shape: &[],
        },
        ParamDoc {
            name: "existing",
            desc: "The passkeys this account already has. A device that has one of them does not \
                   create a second one.",
            shape: &[],
        },
    ],
    ret: "A JSON string. In the browser, pass it to \
          `PublicKeyCredential.parseCreationOptionsFromJSON()`, and pass the result to \
          `navigator.credentials.create()`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The request has not started a session.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$rpId` is empty, or `$userId` is empty or longer than 64 bytes.",
        },
    ],
};

/// `Core\Passkey::verifyRegistration`'s reference card — `rule:core-api/reference-card`.
const VERIFY_REGISTRATION_DOC: MethodDoc = MethodDoc {
    short: "Checks the browser's answer to `registerOptions` and returns the new passkey. Save \
            the passkey with the account. A challenge can be checked only once, so a second call \
            with the same answer throws an error.",
    params: &[
        ParamDoc {
            name: "response",
            desc: "The credential that `navigator.credentials.create()` returned, as JSON. In the \
                   browser, `JSON.stringify(credential)` makes it.",
            shape: &[],
        },
        ParamDoc {
            name: "rpId",
            desc: "The same domain that you passed to `registerOptions`.",
            shape: &[],
        },
        ParamDoc {
            name: "origin",
            desc: "The address of your site in the browser, such as `https://example.com`. The \
                   answer must name exactly this origin.",
            shape: &[],
        },
    ],
    ret: "The new passkey, with its id, its public key, its algorithm and its sign counter.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The request has not started a session, no registration is waiting, or the \
               challenge is older than five minutes. It is also thrown when a check fails. The \
               checks are the challenge, the origin, the domain, the user verification and the \
               key algorithm. An answer longer than 64 KB also throws.",
    }],
};

/// `Core\Passkey::signInOptions`'s reference card — `rule:core-api/reference-card`.
const SIGN_IN_OPTIONS_DOC: MethodDoc = MethodDoc {
    short: "Returns the options for `navigator.credentials.get()` as JSON. The options contain a \
            new random challenge. The challenge is saved in the session, so call \
            `Core\\Session::start()` first.",
    params: &[
        ParamDoc {
            name: "rpId",
            desc: "The domain of your site, such as `example.com`. Use the same domain as for \
                   `registerOptions`.",
            shape: &[],
        },
        ParamDoc {
            name: "allowed",
            desc: "The passkeys of the account that signs in. Leave it empty when you do not know \
                   the account yet. The browser then shows every passkey it has for your site.",
            shape: &[],
        },
    ],
    ret: "A JSON string. In the browser, pass it to \
          `PublicKeyCredential.parseRequestOptionsFromJSON()`, and pass the result to \
          `navigator.credentials.get()`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The request has not started a session.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$rpId` is empty.",
        },
    ],
};

/// `Core\Passkey::credentialId`'s reference card — `rule:core-api/reference-card`.
const CREDENTIAL_ID_DOC: MethodDoc = MethodDoc {
    short: "Returns the passkey id in the browser's answer to `signInOptions`. Use it to find the \
            saved passkey. The id is not checked yet, so the result is `tainted`.",
    params: &[ParamDoc {
        name: "response",
        desc: "The credential that `navigator.credentials.get()` returned, as JSON. In the \
               browser, `JSON.stringify(credential)` makes it.",
        shape: &[],
    }],
    ret: "The passkey id, from 1 to 1023 bytes.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The answer is not JSON or is longer than 64 KB. It is also thrown when the \
               `id` is missing, is not base64url or is too long.",
    }],
};

/// `Core\Passkey::verifySignIn`'s reference card — `rule:core-api/reference-card`.
const VERIFY_SIGN_IN_DOC: MethodDoc = MethodDoc {
    short: "Checks the browser's answer to `signInOptions` against the saved passkey. It returns \
            the passkey with its new sign counter. Save it again. A challenge can be checked only \
            once, so a second call with the same answer throws an error.",
    params: &[
        ParamDoc {
            name: "response",
            desc: "The credential that `navigator.credentials.get()` returned, as JSON.",
            shape: &[],
        },
        ParamDoc {
            name: "stored",
            desc: "The saved passkey with the id that `credentialId` returned.",
            shape: &[],
        },
        ParamDoc {
            name: "rpId",
            desc: "The same domain that you passed to `signInOptions`.",
            shape: &[],
        },
        ParamDoc {
            name: "origin",
            desc: "The address of your site in the browser, such as `https://example.com`. The \
                   answer must name exactly this origin.",
            shape: &[],
        },
    ],
    ret: "The passkey with its new sign counter.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The request has not started a session, no sign-in is waiting, or the challenge \
               is older than five minutes. It is also thrown when a check fails. The checks are \
               the passkey id, the challenge, the origin, the domain, the user verification, the \
               signature and the sign counter. An answer longer than 64 KB also throws.",
    }],
};

/// `Core\Passkey\Credential`'s class card — `rule:core-api/reference-card`.
const CREDENTIAL_CARD: ClassDoc = ClassDoc {
    short: "A passkey that a user registered. Save its four values with the account, and create \
            it again with `of`. It contains no secret, because a public key can only check \
            signatures.",
};

/// `Core\Passkey\Credential::of`'s reference card — `rule:core-api/reference-card`.
const OF_DOC: MethodDoc = MethodDoc {
    short: "Creates a passkey from the four values that you saved. The public key is checked \
            against the algorithm.",
    params: &[
        ParamDoc {
            name: "id",
            desc: "The value that `id()` returned.",
            shape: &[],
        },
        ParamDoc {
            name: "publicKey",
            desc: "The value that `publicKey()` returned.",
            shape: &[],
        },
        ParamDoc {
            name: "algorithm",
            desc: "The value that `algorithm()` returned.",
            shape: &[],
        },
        ParamDoc {
            name: "signCount",
            desc: "The value that `signCount()` returned.",
            shape: &[],
        },
    ],
    ret: "The passkey.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$publicKey` is not a key of `$algorithm`, or `$id` is empty or longer than 1023 \
               bytes. It is also thrown when `$signCount` is below 0 or above 4294967295.",
    }],
};

/// `Core\Passkey\Credential::id`'s reference card — `rule:core-api/reference-card`.
const ID_DOC: MethodDoc = MethodDoc {
    short: "Returns the id of the passkey. The browser sends this id when the user signs in.",
    params: &[],
    ret: "From 1 to 1023 bytes.",
    errors: &[],
};

/// `Core\Passkey\Credential::publicKey`'s reference card — `rule:core-api/reference-card`.
const PUBLIC_KEY_DOC: MethodDoc = MethodDoc {
    short: "Returns the public key of the passkey, in the COSE format that WebAuthn uses.",
    params: &[],
    ret: "The key as bytes. Save them unchanged.",
    errors: &[],
};

/// `Core\Passkey\Credential::algorithm`'s reference card — `rule:core-api/reference-card`.
const ALGORITHM_DOC: MethodDoc = MethodDoc {
    short: "Returns the signature algorithm of the passkey.",
    params: &[],
    ret: "`ES256` or `Ed25519`.",
    errors: &[],
};

/// `Core\Passkey\Credential::signCount`'s reference card — `rule:core-api/reference-card`.
const SIGN_COUNT_DOC: MethodDoc = MethodDoc {
    short: "Returns the sign counter of the passkey. Some devices count every sign-in. Many \
            devices always send `0`.",
    params: &[],
    ret: "A whole number from 0 to 4294967295.",
    errors: &[],
};

/// [`ALGORITHM`]'s reference card — `rule:core-api/reference-card`.
const ALGORITHM_ENUM_DOC: EnumDoc = EnumDoc {
    short: "The signature algorithm of a passkey. A passkey with another algorithm cannot be \
            registered.",
    cases: &[
        CaseDoc {
            name: "ES256",
            desc: "ECDSA with the P-256 curve and SHA-256. Almost every device supports it.",
        },
        CaseDoc {
            name: "Ed25519",
            desc: "Ed25519 signatures. Some devices support it.",
        },
    ],
};

/// A passkey's signature algorithm, as [`ALGORITHM`]'s cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Algorithm {
    /// ECDSA over P-256 with SHA-256.
    Es256,
    /// Ed25519.
    Ed25519,
}

impl Algorithm {
    /// This case's [`ALGORITHM`] integer.
    fn tag(self) -> i64 {
        match self {
            Self::Es256 => 0,
            Self::Ed25519 => 1,
        }
    }

    /// The case [`ALGORITHM`]'s integer `tag` is, or `None` for one it has not.
    fn from_tag(tag: i64) -> Option<Self> {
        match tag {
            0 => Some(Self::Es256),
            1 => Some(Self::Ed25519),
            _ => None,
        }
    }
}

/// A credential a registration produced, before it is an instance.
#[derive(Debug, PartialEq, Eq)]
struct Registered {
    /// The credential id.
    id: Vec<u8>,
    /// The COSE key, as the authenticator encoded it.
    key: Vec<u8>,
    /// The key's algorithm.
    algorithm: Algorithm,
    /// The authenticator's sign counter.
    sign_count: u32,
}

/// The authenticator data's fixed head, and what follows it.
struct AuthData<'a> {
    /// SHA-256 of the relying-party id the authenticator used.
    rp_id_hash: &'a [u8],
    /// The flags octet.
    flags: u8,
    /// The sign counter.
    sign_count: u32,
    /// The attested credential data and the extensions, unread.
    rest: &'a [u8],
}

/// The credential the authenticator data attests.
#[derive(Debug, PartialEq, Eq)]
struct Attested<'a> {
    /// The credential id.
    id: &'a [u8],
    /// The COSE key's own octets.
    key: &'a [u8],
}

/// The address of one of *this* module's symbols, or `None` for a symbol that belongs to another
/// domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_passkey_register_options" => {
            (nvs_core_passkey_register_options as *const ()).cast()
        }
        "nvs_core_passkey_verify_registration" => {
            (nvs_core_passkey_verify_registration as *const ()).cast()
        }
        "nvs_core_passkey_sign_in_options" => {
            (nvs_core_passkey_sign_in_options as *const ()).cast()
        }
        "nvs_core_passkey_credential_id_of" => {
            (nvs_core_passkey_credential_id_of as *const ()).cast()
        }
        "nvs_core_passkey_verify_sign_in" => (nvs_core_passkey_verify_sign_in as *const ()).cast(),
        "nvs_core_passkey_credential_of" => (nvs_core_passkey_credential_of as *const ()).cast(),
        "nvs_core_passkey_credential_id" => (nvs_core_passkey_credential_id as *const ()).cast(),
        "nvs_core_passkey_credential_public_key" => {
            (nvs_core_passkey_credential_public_key as *const ()).cast()
        }
        "nvs_core_passkey_credential_algorithm" => {
            (nvs_core_passkey_credential_algorithm as *const ()).cast()
        }
        "nvs_core_passkey_credential_sign_count" => {
            (nvs_core_passkey_credential_sign_count as *const ()).cast()
        }
        _ => return None,
    })
}

/// The `string` argument at `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `string`, which the checker has already refused.
fn text_at<'a>(args: &'a [Value], slot: usize, who: &str, param: &str) -> Result<&'a str, Fault> {
    args[slot].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{who} expected a `string` for ${param}, got tag {}",
            args[slot].tag_byte()
        ))
    })
}

/// The `bytes` argument at `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `bytes`, which the checker has already refused.
fn bytes_at<'a>(args: &'a [Value], slot: usize, who: &str, param: &str) -> Result<&'a [u8], Fault> {
    args[slot].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{who} expected a `bytes` for ${param}, got tag {}",
            args[slot].tag_byte()
        ))
    })
}

/// The throw for a member called before `Core\Session::start`.
///
/// # Errors
///
/// A thrown `RuntimeError` while this request has started no session.
fn started(ctx: &Ctx, member: &str) -> Result<(), Fault> {
    if ctx.session().is_some() {
        return Ok(());
    }
    Err(Fault::thrown(format!(
        "{NAME}::{member}(): this request has not started a session. Call \
         `Core\\Session::start()` first, because the challenge is saved in the session."
    )))
}

/// The wall clock's second.
///
/// # Errors
///
/// A `LogicError` for a clock outside the range a timestamp reaches, which only a test's fixed
/// clock produces.
fn now(ctx: &Ctx, member: &str) -> Result<i64, Fault> {
    crate::time::wall_clock(ctx)
        .map(|at| at.as_second())
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("{NAME}::{member}(): the clock is outside the range of a date."),
            )
        })
}

/// A fresh challenge, saved in the session under `key` behind the second it was issued.
///
/// # Errors
///
/// [`started`]'s and [`now`]'s, and [`crate::session::put_bytes`]'s.
fn issue_challenge(ctx: &mut Ctx, key: &str, member: &str) -> Result<[u8; CHALLENGE_LEN], Fault> {
    started(ctx, member)?;
    let issued = now(ctx, member)?;
    let mut challenge = [0_u8; CHALLENGE_LEN];
    crate::random::draw(ctx, |rng| rng.fill_bytes(&mut challenge));

    let mut saved = Vec::with_capacity(SAVED_LEN);
    saved.extend_from_slice(&issued.to_be_bytes());
    saved.extend_from_slice(&challenge);
    crate::session::put_bytes(ctx, key, &saved)?;
    Ok(challenge)
}

/// The challenge saved under `key`, taken out of the session: the first thing a verification
/// does, so the challenge is gone whatever the checks after it find.
///
/// # Errors
///
/// [`started`]'s, a thrown `RuntimeError` for a session with no challenge waiting or one older
/// than [`TIMEOUT_SECONDS`], and [`crate::session::take_bytes`]'s.
fn take_challenge(ctx: &mut Ctx, key: &str, member: &str) -> Result<Vec<u8>, Fault> {
    started(ctx, member)?;
    let saved = crate::session::take_bytes(ctx, key)?.filter(|saved| saved.len() == SAVED_LEN);
    let Some(saved) = saved else {
        return Err(Fault::thrown(format!(
            "{NAME}::{member}(): no challenge is waiting in this session. Each challenge can be \
             checked only once, so call the options method again."
        )));
    };
    let (issued, challenge) = saved.split_at(8);
    let issued = i64::from_be_bytes(issued.try_into().expect("the split is eight octets"));
    if now(ctx, member)?.saturating_sub(issued) > TIMEOUT_SECONDS {
        return Err(Fault::thrown(format!(
            "{NAME}::{member}(): the challenge is older than five minutes. Call the options \
             method again."
        )));
    }
    Ok(challenge.to_vec())
}

/// The octets base64url `field` of `object` decodes to.
fn decoded(object: &serde_json::Value, field: &str) -> Result<Vec<u8>, String> {
    let text = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("the response has no `{field}` string."))?;
    LENIENT
        .decode(text)
        .map_err(|_| format!("`{field}` is not base64url."))
}

/// Checks the client data JSON against the ceremony `kind`, the challenge and the origin.
///
/// The challenge is compared in constant time. It is not a secret once the options are sent, but
/// a comparison that stops at the first difference is a habit nothing here needs.
fn client_data(encoded: &[u8], kind: &str, challenge: &[u8], origin: &str) -> Result<(), String> {
    let data: serde_json::Value =
        serde_json::from_slice(encoded).map_err(|_| "the client data is not JSON.".to_owned())?;
    let text = |field: &str| data.get(field).and_then(serde_json::Value::as_str);

    if text("type") != Some(kind) {
        return Err(format!("the client data's type is not `{kind}`."));
    }
    let sent = text("challenge").ok_or_else(|| "the client data has no challenge.".to_owned())?;
    let sent = LENIENT
        .decode(sent)
        .map_err(|_| "the client data's challenge is not base64url.".to_owned())?;
    if !bool::from(sent.as_slice().ct_eq(challenge)) {
        return Err("the challenge is not the one this session was given.".to_owned());
    }
    if text("origin") != Some(origin) {
        return Err(format!("the origin is not `{origin}`."));
    }
    if data.get("crossOrigin").and_then(serde_json::Value::as_bool) == Some(true) {
        return Err("the passkey was used in a frame of another site.".to_owned());
    }
    Ok(())
}

/// The `authData` of an attestation object. The format and the statement are read only far
/// enough to be a text and a map, because attestation is not verified.
fn attestation_auth_data(object: &[u8]) -> Result<Vec<u8>, String> {
    let not_one = || "the attestation object is not a WebAuthn attestation object.".to_owned();
    let mut reader = object;
    let value: Cbor = ciborium::de::from_reader(&mut reader).map_err(|_| not_one())?;
    if !reader.is_empty() {
        return Err(not_one());
    }
    let Cbor::Map(entries) = value else {
        return Err(not_one());
    };

    let mut format = false;
    let mut statement = false;
    let mut auth = None;
    for (key, value) in entries {
        match (key.as_text(), value) {
            (Some("fmt"), Cbor::Text(_)) => format = true,
            (Some("attStmt"), Cbor::Map(_)) => statement = true,
            (Some("authData"), Cbor::Bytes(bytes)) => auth = Some(bytes),
            _ => {}
        }
    }
    match auth {
        Some(auth) if format && statement => Ok(auth),
        _ => Err(not_one()),
    }
}

/// The authenticator data's fixed 37-octet head, split off what follows it.
fn auth_data(bytes: &[u8]) -> Result<AuthData<'_>, String> {
    let short = || "the authenticator data is shorter than 37 bytes.".to_owned();
    let (rp_id_hash, rest) = bytes.split_at_checked(32).ok_or_else(short)?;
    let (&flags, rest) = rest.split_first().ok_or_else(short)?;
    let (count, rest) = rest.split_first_chunk::<4>().ok_or_else(short)?;
    Ok(AuthData {
        rp_id_hash,
        flags,
        sign_count: u32::from_be_bytes(*count),
        rest,
    })
}

/// Checks the authenticator data's relying-party hash against `rp_id`, and that both the
/// user-present and the user-verified flags are set.
fn check_auth(auth: &AuthData<'_>, rp_id: &str) -> Result<(), String> {
    let expected = Sha256::digest(rp_id.as_bytes());
    if !bool::from(auth.rp_id_hash.ct_eq(expected.as_slice())) {
        return Err(format!("the passkey is for another domain than `{rp_id}`."));
    }
    if auth.flags & USER_PRESENT == 0 {
        return Err("the device did not confirm that the user was present.".to_owned());
    }
    if auth.flags & USER_VERIFIED == 0 {
        return Err(
            "the device did not verify the user with a PIN, a fingerprint or a face. \
                    The options ask for this check."
                .to_owned(),
        );
    }
    Ok(())
}

/// The credential id and the COSE key the attested credential data holds, with the extensions
/// after them read only to check that nothing else follows.
fn attested<'a>(auth: &AuthData<'a>) -> Result<Attested<'a>, String> {
    let short = || "the authenticator data ends inside the new passkey.".to_owned();
    if auth.flags & ATTESTED == 0 {
        return Err("the authenticator data contains no new passkey.".to_owned());
    }
    // The AAGUID names the authenticator's model, which only attestation could vouch for.
    let (_aaguid, rest) = auth.rest.split_at_checked(16).ok_or_else(short)?;
    let (length, rest) = rest.split_first_chunk::<2>().ok_or_else(short)?;
    let length = usize::from(u16::from_be_bytes(*length));
    if length == 0 || length > MAX_ID {
        return Err(format!(
            "the passkey's id is {length} bytes. It must be from 1 to {MAX_ID} bytes."
        ));
    }
    let (id, after) = rest.split_at_checked(length).ok_or_else(short)?;

    // `ciborium` reads exactly one item from a slice and advances it, so what it consumed is the
    // key's own octets.
    let not_cbor = || "the passkey's public key is not CBOR.".to_owned();
    let mut reader = after;
    let _: Cbor = ciborium::de::from_reader(&mut reader).map_err(|_| not_cbor())?;
    let key = &after[..after.len() - reader.len()];
    if auth.flags & EXTENSIONS != 0 {
        let _: Cbor = ciborium::de::from_reader(&mut reader)
            .map_err(|_| "the authenticator data's extensions are not CBOR.".to_owned())?;
    }
    if !reader.is_empty() {
        return Err("the authenticator data has bytes after the new passkey.".to_owned());
    }
    Ok(Attested { id, key })
}

/// The algorithm of a COSE key, and the point a signature check reads: `04 ‖ x ‖ y` for ES256
/// and `x` for Ed25519.
///
/// Every other key type, curve and algorithm is refused, naming the COSE algorithm number the
/// authenticator chose.
fn key_point(cose: &[u8]) -> Result<(Algorithm, Vec<u8>), String> {
    let not_one = || "the public key is not a COSE key.".to_owned();
    let key = CoseKey::from_slice(cose).map_err(|_| not_one())?;
    let algorithm = match &key.alg {
        Some(RegisteredLabelWithPrivate::Assigned(algorithm)) => algorithm.to_i64(),
        Some(RegisteredLabelWithPrivate::PrivateUse(algorithm)) => *algorithm,
        _ => return Err("the public key names no algorithm.".to_owned()),
    };
    let param = |label: i64| {
        key.params
            .iter()
            .find(|(found, _)| *found == Label::Int(label))
            .map(|(_, value)| value)
    };
    let curve = param(-1)
        .and_then(Cbor::as_integer)
        .and_then(|curve| i64::try_from(curve).ok());
    let coordinate = |label: i64| {
        param(label)
            .and_then(Cbor::as_bytes)
            .filter(|bytes| bytes.len() == 32)
    };
    let kty = match &key.kty {
        RegisteredLabel::Assigned(kty) => kty.to_i64(),
        RegisteredLabel::Text(_) => 0,
    };

    // EC2 is 2 and P-256 is 1; OKP is 1 and Ed25519 is 6 (RFC 9053 § 7).
    match (kty, algorithm) {
        (2, -7) => {
            let (Some(1), Some(x), Some(y)) = (curve, coordinate(-2), coordinate(-3)) else {
                return Err("the ES256 key is not a P-256 point.".to_owned());
            };
            let mut point = Vec::with_capacity(65);
            point.push(0x04);
            point.extend_from_slice(x);
            point.extend_from_slice(y);
            if crate::crypto::read_p256_point(&point).is_none() {
                return Err("the ES256 key is not a point on the P-256 curve.".to_owned());
            }
            Ok((Algorithm::Es256, point))
        }
        (1, -8 | -19) => {
            let (Some(6), Some(x)) = (curve, coordinate(-2)) else {
                return Err("the Ed25519 key is not a 32-byte Ed25519 key.".to_owned());
            };
            Ok((Algorithm::Ed25519, x.clone()))
        }
        _ => Err(format!(
            "the passkey uses COSE algorithm {algorithm}. Only ES256 (-7) and Ed25519 (-8 or \
             -19) are supported."
        )),
    }
}

/// Every check of a registration response, against a challenge already taken out of the session.
fn registration(
    response: &str,
    challenge: &[u8],
    rp_id: &str,
    origin: &str,
) -> Result<Registered, String> {
    let response: serde_json::Value =
        serde_json::from_str(response).map_err(|_| "the response is not JSON.".to_owned())?;
    if response.get("type").and_then(serde_json::Value::as_str) != Some("public-key") {
        return Err("the response's `type` is not `public-key`.".to_owned());
    }
    let inner = response
        .get("response")
        .ok_or_else(|| "the response has no `response` object.".to_owned())?;

    client_data(
        &decoded(inner, "clientDataJSON")?,
        "webauthn.create",
        challenge,
        origin,
    )?;
    let auth = attestation_auth_data(&decoded(inner, "attestationObject")?)?;
    let auth = auth_data(&auth)?;
    check_auth(&auth, rp_id)?;
    let attested = attested(&auth)?;
    let (algorithm, _point) = key_point(attested.key)?;
    if decoded(&response, "id")? != attested.id {
        return Err("the response's `id` is not the id of the new passkey.".to_owned());
    }

    Ok(Registered {
        id: attested.id.to_vec(),
        key: attested.key.to_vec(),
        algorithm,
        sign_count: auth.sign_count,
    })
}

/// The id of a sign-in response, from `id`, or from `rawId` where a response has no `id`. A
/// response with neither is refused naming `id`, the field every browser sends.
fn claimed_id(response: &serde_json::Value) -> Result<Vec<u8>, String> {
    let field = if response.get("id").is_none() && response.get("rawId").is_some() {
        "rawId"
    } else {
        "id"
    };
    let id = decoded(response, field)?;
    if id.is_empty() || id.len() > MAX_ID {
        return Err(format!(
            "the passkey's id is {} bytes. It must be from 1 to {MAX_ID} bytes.",
            id.len()
        ));
    }
    Ok(id)
}

/// Every check of a sign-in response against the `stored` credential and a challenge already
/// taken out of the session, and the sign counter the authenticator sent.
fn sign_in(
    response: &str,
    challenge: &[u8],
    stored: &Registered,
    rp_id: &str,
    origin: &str,
) -> Result<u32, String> {
    let response: serde_json::Value =
        serde_json::from_str(response).map_err(|_| "the response is not JSON.".to_owned())?;
    if response.get("type").and_then(serde_json::Value::as_str) != Some("public-key") {
        return Err("the response's `type` is not `public-key`.".to_owned());
    }
    if claimed_id(&response)? != stored.id {
        return Err("the response is for another passkey than `$stored`.".to_owned());
    }
    let inner = response
        .get("response")
        .ok_or_else(|| "the response has no `response` object.".to_owned())?;

    let client = decoded(inner, "clientDataJSON")?;
    client_data(&client, "webauthn.get", challenge, origin)?;
    let signed = decoded(inner, "authenticatorData")?;
    let auth = auth_data(&signed)?;
    check_auth(&auth, rp_id)?;

    let signature = decoded(inner, "signature")?;
    let (algorithm, point) = key_point(&stored.key)?;
    let key = match algorithm {
        Algorithm::Es256 => crate::crypto::VerifyingKey::P256Der { point: &point },
        Algorithm::Ed25519 => crate::crypto::VerifyingKey::Ed25519 { point: &point },
    };
    let mut message = signed.clone();
    message.extend_from_slice(&Sha256::digest(&client));
    crate::crypto::verify_signature(&key, &message, &signature)
        .ok_or_else(|| "the signature was not made by this passkey.".to_owned())?;

    let (sent, kept) = (auth.sign_count, stored.sign_count);
    if (sent != 0 || kept != 0) && sent <= kept {
        return Err(format!(
            "the sign counter is {sent}, and the saved passkey has {kept}. The counter must \
             increase, so the passkey may have been copied."
        ));
    }
    Ok(sent)
}

/// The four slots of the [`CREDENTIAL`] `value`, read back.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value or a slot of another type, which the checker and
/// [`nvs_core_passkey_credential_of`] have refused.
fn stored_credential(value: Value, member: &str) -> Result<Registered, Fault> {
    let held = crate::instance::receiver(value, &CREDENTIAL, member)?;
    let broken = |slot: &str| Fault::fatal(format!("{CREDENTIAL_NAME} has a broken `{slot}` slot"));
    let id = crate::instance::slot(held, ID_SLOT);
    let key = crate::instance::slot(held, KEY_SLOT);
    Ok(Registered {
        id: id.as_bytes().ok_or_else(|| broken("id"))?.to_vec(),
        key: key.as_bytes().ok_or_else(|| broken("publicKey"))?.to_vec(),
        algorithm: crate::instance::slot(held, ALGORITHM_SLOT)
            .as_int()
            .and_then(Algorithm::from_tag)
            .ok_or_else(|| broken("algorithm"))?,
        sign_count: crate::instance::slot(held, COUNT_SLOT)
            .as_int()
            .and_then(|count| u32::try_from(count).ok())
            .ok_or_else(|| broken("signCount"))?,
    })
}

/// A [`CREDENTIAL`] instance holding `registered`.
///
/// # Errors
///
/// Whatever `nvs_runtime::affordable` answers for a request at its ceiling.
fn credential_instance(registered: &Registered, site: &str) -> Result<Value, Fault> {
    nvs_runtime::affordable(
        Some(registered.id.len().saturating_add(registered.key.len())),
        site,
    )?;
    Ok(crate::instance::build(
        &CREDENTIAL,
        [
            Value::bytes(NvsStr::new(&registered.id)),
            Value::bytes(NvsStr::new(&registered.key)),
            Value::int(registered.algorithm.tag()),
            Value::int(i64::from(registered.sign_count)),
        ],
    ))
}

/// The credential ids of the `array<Passkey\Credential>` at `slot`, base64url, for the options'
/// `excludeCredentials` and `allowCredentials`.
///
/// The options are Rust memory the request's heap budget does not count until the finished text
/// is copied into it: this list, the `serde_json` tree written from it and the text. So the
/// request's memory limit is checked before the first id and after each one, against `spent` (what
/// the rest of the options cost) plus each id three times and [`ENTRY_COST`] per entry, and a list
/// the request cannot afford stops the request before the tree is built.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot or an element of another type, which the checker has refused, and
/// for a list past the request's memory limit.
fn credential_ids(
    args: &[Value],
    slot: usize,
    member: &str,
    mut spent: usize,
) -> Result<Vec<String>, Fault> {
    let raw = args[slot].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected an array of `{CREDENTIAL_NAME}`, got tag {}",
            args[slot].tag_byte()
        ))
    })?;
    let list = crate::arr::borrowed(raw);
    let mut ids = Vec::new();
    let mut from = 0;
    loop {
        if !nvs_runtime::budget::affords(spent) {
            nvs_runtime::affordable(Some(spent), &format!("{NAME}::{member}"))?;
        }
        let Some(live) = list.next_slot(from) else {
            break;
        };
        from = live + 1;
        let element = list.value_at(live).expect("a live slot holds a value");
        let held = crate::instance::receiver(element, &CREDENTIAL, member)?;
        let id = crate::instance::slot(held, ID_SLOT);
        let id = id.as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{CREDENTIAL_NAME} expected a `bytes` in its `id` slot"
            ))
        })?;
        let encoded = id.len().div_ceil(3).saturating_mul(4);
        spent = spent
            .saturating_add(encoded.saturating_mul(3))
            .saturating_add(ENTRY_COST);
        ids.push(URL_SAFE_NO_PAD.encode(id));
    }
    Ok(ids)
}

nvs_runtime::nvs_helper! {
    /// `Core\Passkey::registerOptions(string $rpId, string $rpName, bytes $userId,
    /// string $userName, array<Passkey\Credential> $existing = []): string` — the JSON a browser's
    /// `PublicKeyCredential.parseCreationOptionsFromJSON()` reads, with a fresh challenge saved in
    /// the session.
    ///
    /// The options ask for user verification, for a discoverable credential where the device can
    /// make one, and for no attestation. ES256 is listed before Ed25519 because nearly every
    /// authenticator has it.
    fn nvs_core_passkey_register_options(ctx, args: [5]) {
        let who = "Core\\Passkey::registerOptions";
        let rp_id = text_at(args, 0, who, "rpId")?;
        let rp_name = text_at(args, 1, who, "rpName")?;
        let user_id = bytes_at(args, 2, who, "userId")?;
        let user_name = text_at(args, 3, who, "userName")?;
        if rp_id.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{who}(): $rpId is empty. Pass the domain of your site."),
            ));
        }
        if user_id.is_empty() || user_id.len() > MAX_USER_ID {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{who}(): $userId is {} bytes. It must be from 1 to {MAX_USER_ID} bytes.",
                    user_id.len()
                ),
            ));
        }
        let texts = rp_id.len() + rp_name.len() + 2 * user_name.len();
        let spent = texts.saturating_mul(TEXT_COST).saturating_add(OPTIONS_COST);
        let excluded = credential_ids(args, 4, "registerOptions", spent)?;

        let challenge = issue_challenge(ctx, REGISTER_KEY, "registerOptions")?;
        let options = serde_json::json!({
            "challenge": URL_SAFE_NO_PAD.encode(challenge),
            "rp": { "id": rp_id, "name": rp_name },
            "user": {
                "id": URL_SAFE_NO_PAD.encode(user_id),
                "name": user_name,
                "displayName": user_name,
            },
            "pubKeyCredParams": [
                { "type": "public-key", "alg": -7 },
                { "type": "public-key", "alg": -8 },
            ],
            "timeout": TIMEOUT_SECONDS * 1000,
            "excludeCredentials": excluded
                .iter()
                .map(|id| serde_json::json!({ "type": "public-key", "id": id }))
                .collect::<Vec<_>>(),
            "authenticatorSelection": {
                "residentKey": "preferred",
                "userVerification": "required",
            },
            "attestation": "none",
        });
        let options = options.to_string();
        nvs_runtime::affordable(Some(options.len()), who)?;
        Ok(Value::text(&options))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Passkey::verifyRegistration(string $response, string $rpId, string $origin):
    /// Passkey\Credential` — every check of `rule:core-classes/passkey-verifies-a-webauthn-ceremony`
    /// for a registration, after the challenge is taken out of the session.
    ///
    /// Each failed check throws its own sentence. None of them is an oracle: a registration
    /// proves nothing about an account that exists, and the sentence is what tells a developer
    /// that `$origin` is written with the wrong port.
    fn nvs_core_passkey_verify_registration(ctx, args: [3]) {
        let who = "Core\\Passkey::verifyRegistration";
        let response = text_at(args, 0, who, "response")?;
        let rp_id = text_at(args, 1, who, "rpId")?;
        let origin = text_at(args, 2, who, "origin")?;

        let challenge = take_challenge(ctx, REGISTER_KEY, "verifyRegistration")?;
        if response.len() > MAX_RESPONSE {
            return Err(Fault::thrown(format!(
                "{who}(): the response is {} bytes, and the limit is {MAX_RESPONSE}.",
                response.len()
            )));
        }
        let registered = registration(response, &challenge, rp_id, origin)
            .map_err(|why| Fault::thrown(format!("{who}(): {why}")))?;
        credential_instance(&registered, who)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Passkey::signInOptions(string $rpId, array<Passkey\Credential> $allowed = []):
    /// string` — the JSON a browser's `PublicKeyCredential.parseRequestOptionsFromJSON()` reads,
    /// with a fresh challenge saved in the session.
    fn nvs_core_passkey_sign_in_options(ctx, args: [2]) {
        let who = "Core\\Passkey::signInOptions";
        let rp_id = text_at(args, 0, who, "rpId")?;
        if rp_id.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{who}(): $rpId is empty. Pass the domain of your site."),
            ));
        }
        let spent = rp_id.len().saturating_mul(TEXT_COST).saturating_add(OPTIONS_COST);
        let allowed = credential_ids(args, 1, "signInOptions", spent)?;

        let challenge = issue_challenge(ctx, SIGN_IN_KEY, "signInOptions")?;
        let options = serde_json::json!({
            "challenge": URL_SAFE_NO_PAD.encode(challenge),
            "rpId": rp_id,
            "timeout": TIMEOUT_SECONDS * 1000,
            "allowCredentials": allowed
                .iter()
                .map(|id| serde_json::json!({ "type": "public-key", "id": id }))
                .collect::<Vec<_>>(),
            "userVerification": "required",
        });
        let options = options.to_string();
        nvs_runtime::affordable(Some(options.len()), who)?;
        Ok(Value::text(&options))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Passkey::credentialId(string $response): bytes` — the id the browser claims, for the
    /// program to look up the stored credential with. Nothing about it is checked but its form:
    /// [`nvs_core_passkey_verify_sign_in`] compares it against the credential it is given.
    fn nvs_core_passkey_credential_id_of(_ctx, args: [1]) {
        let who = "Core\\Passkey::credentialId";
        let response = text_at(args, 0, who, "response")?;
        if response.len() > MAX_RESPONSE {
            return Err(Fault::thrown(format!(
                "{who}(): the response is {} bytes, and the limit is {MAX_RESPONSE}.",
                response.len()
            )));
        }
        let id = serde_json::from_str(response)
            .map_err(|_| "the response is not JSON.".to_owned())
            .and_then(|response| claimed_id(&response))
            .map_err(|why| Fault::thrown(format!("{who}(): {why}")))?;
        nvs_runtime::affordable(Some(id.len()), who)?;
        Ok(Value::bytes(NvsStr::new(&id)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Passkey::verifySignIn(string $response, Passkey\Credential $stored, string $rpId,
    /// string $origin): Passkey\Credential` — every check of
    /// `rule:core-classes/passkey-verifies-a-webauthn-ceremony` for a sign-in, after the challenge
    /// is taken out of the session, and `$stored` again with the counter the device sent.
    ///
    /// The signature's failure is one sentence for every way of not verifying, as
    /// [`crate::crypto::verify_signature`]'s is.
    fn nvs_core_passkey_verify_sign_in(ctx, args: [4]) {
        let who = "Core\\Passkey::verifySignIn";
        let response = text_at(args, 0, who, "response")?;
        let rp_id = text_at(args, 2, who, "rpId")?;
        let origin = text_at(args, 3, who, "origin")?;

        let challenge = take_challenge(ctx, SIGN_IN_KEY, "verifySignIn")?;
        if response.len() > MAX_RESPONSE {
            return Err(Fault::thrown(format!(
                "{who}(): the response is {} bytes, and the limit is {MAX_RESPONSE}.",
                response.len()
            )));
        }
        let mut stored = stored_credential(args[1], "verifySignIn")?;
        stored.sign_count = sign_in(response, &challenge, &stored, rp_id, origin)
            .map_err(|why| Fault::thrown(format!("{who}(): {why}")))?;
        credential_instance(&stored, who)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Passkey\Credential::of(bytes $id, bytes $publicKey, Passkey\Credential::Algorithm
    /// $algorithm, int $signCount): Passkey\Credential` — a stored credential, built again and
    /// checked as registration checked it.
    fn nvs_core_passkey_credential_of(_ctx, args: [4]) {
        let who = "Core\\Passkey\\Credential::of";
        let id = bytes_at(args, 0, who, "id")?;
        let key = bytes_at(args, 1, who, "publicKey")?;
        let algorithm = args[2].as_int().and_then(Algorithm::from_tag).ok_or_else(|| {
            Fault::fatal(format!(
                "{who} expected a `{ALGORITHM_NAME}` case, got tag {}",
                args[2].tag_byte()
            ))
        })?;
        let count = args[3].as_int().ok_or_else(|| {
            Fault::fatal(format!("{who} expected an `int` for $signCount, got tag {}", args[3].tag_byte()))
        })?;
        let logic = |why: String| Fault::thrown_as(ThrownClass::Logic, format!("{who}(): {why}"));

        if id.is_empty() || id.len() > MAX_ID {
            return Err(logic(format!(
                "$id is {} bytes. It must be from 1 to {MAX_ID} bytes.",
                id.len()
            )));
        }
        let sign_count = u32::try_from(count).map_err(|_| {
            logic(format!("$signCount is {count}. It must be from 0 to {}.", u32::MAX))
        })?;
        let (read, _point) = key_point(key).map_err(logic)?;
        if read != algorithm {
            return Err(logic("$publicKey is not a key of $algorithm.".to_owned()));
        }
        credential_instance(
            &Registered { id: id.to_vec(), key: key.to_vec(), algorithm, sign_count },
            who,
        )
    }
}

nvs_runtime::nvs_helper! {
    /// `$credential->id(): bytes`.
    fn nvs_core_passkey_credential_id(_ctx, args: [1]) {
        crate::instance::read_slot(args, &CREDENTIAL, ID_SLOT, "id")
    }
}

nvs_runtime::nvs_helper! {
    /// `$credential->publicKey(): bytes`, the COSE key as the authenticator encoded it.
    fn nvs_core_passkey_credential_public_key(_ctx, args: [1]) {
        crate::instance::read_slot(args, &CREDENTIAL, KEY_SLOT, "publicKey")
    }
}

nvs_runtime::nvs_helper! {
    /// `$credential->algorithm(): Passkey\Credential::Algorithm`.
    fn nvs_core_passkey_credential_algorithm(_ctx, args: [1]) {
        crate::instance::read_slot(args, &CREDENTIAL, ALGORITHM_SLOT, "algorithm")
    }
}

nvs_runtime::nvs_helper! {
    /// `$credential->signCount(): int`.
    fn nvs_core_passkey_credential_sign_count(_ctx, args: [1]) {
        crate::instance::read_slot(args, &CREDENTIAL, COUNT_SLOT, "signCount")
    }
}

#[cfg(test)]
mod tests {
    use ring::rand::SystemRandom;
    use ring::signature::{
        ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, Ed25519KeyPair, KeyPair as _,
    };

    use super::*;

    const RP_ID: &str = "example.org";
    const ORIGIN: &str = "https://example.org";

    /// The octets a hex string spells.
    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("the vector is hex"))
            .collect()
    }

    /// The private half of a software authenticator's key.
    enum Signer {
        /// A P-256 pair that signs in ASN.1 DER, as an authenticator does.
        Es256(EcdsaKeyPair),
        /// An Ed25519 pair.
        Ed25519(Ed25519KeyPair),
        /// The RSA key below, which signs nothing here.
        None,
    }

    impl Signer {
        /// This key's signature over `message`.
        fn sign(&self, message: &[u8]) -> Vec<u8> {
            match self {
                Self::Es256(pair) => pair
                    .sign(&SystemRandom::new(), message)
                    .expect("a P-256 signature is made")
                    .as_ref()
                    .to_vec(),
                Self::Ed25519(pair) => pair.sign(message).as_ref().to_vec(),
                Self::None => unreachable!("the RSA key is refused before it signs"),
            }
        }
    }

    /// A key a software authenticator made: its COSE encoding and its private half.
    fn authenticator(algorithm: i64) -> (Vec<u8>, Signer) {
        let rng = SystemRandom::new();
        let (map, signer) = match algorithm {
            -7 => {
                let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
                    .expect("a P-256 key generates");
                let pair =
                    EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng)
                        .expect("the key it generated reads");
                let point = pair.public_key().as_ref();
                let map = vec![
                    (Cbor::from(1), Cbor::from(2)),
                    (Cbor::from(3), Cbor::from(-7)),
                    (Cbor::from(-1), Cbor::from(1)),
                    (Cbor::from(-2), Cbor::Bytes(point[1..33].to_vec())),
                    (Cbor::from(-3), Cbor::Bytes(point[33..].to_vec())),
                ];
                (map, Signer::Es256(pair))
            }
            -8 => {
                let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("an Ed25519 key generates");
                let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("the key reads");
                let map = vec![
                    (Cbor::from(1), Cbor::from(1)),
                    (Cbor::from(3), Cbor::from(-8)),
                    (Cbor::from(-1), Cbor::from(6)),
                    (
                        Cbor::from(-2),
                        Cbor::Bytes(pair.public_key().as_ref().to_vec()),
                    ),
                ];
                (map, Signer::Ed25519(pair))
            }
            // RS256: an RSA key, which this class does not register. The modulus is not a real
            // one, because the algorithm is refused before it is read.
            _ => (
                vec![
                    (Cbor::from(1), Cbor::from(3)),
                    (Cbor::from(3), Cbor::from(algorithm)),
                    (Cbor::from(-1), Cbor::Bytes(vec![0xc5; 256])),
                    (Cbor::from(-2), Cbor::Bytes(vec![1, 0, 1])),
                ],
                Signer::None,
            ),
        };
        let mut written = Vec::new();
        ciborium::ser::into_writer(&Cbor::Map(map), &mut written).expect("a map writes");
        (written, signer)
    }

    /// A key a software authenticator made: its COSE encoding.
    fn cose(algorithm: i64) -> Vec<u8> {
        authenticator(algorithm).0
    }

    /// The sign-in response a browser would send for a software authenticator's key, with the
    /// sign counter `count`.
    fn assertion(challenge: &[u8], signer: &Signer, count: u32, flags: u8, id: &[u8]) -> String {
        let client = serde_json::json!({
            "type": "webauthn.get",
            "challenge": URL_SAFE_NO_PAD.encode(challenge),
            "origin": ORIGIN,
            "crossOrigin": false,
        })
        .to_string();
        let mut auth = Sha256::digest(RP_ID.as_bytes()).to_vec();
        auth.push(flags);
        auth.extend_from_slice(&count.to_be_bytes());
        let mut message = auth.clone();
        message.extend_from_slice(&Sha256::digest(client.as_bytes()));
        serde_json::json!({
            "id": URL_SAFE_NO_PAD.encode(id),
            "rawId": URL_SAFE_NO_PAD.encode(id),
            "type": "public-key",
            "response": {
                "clientDataJSON": URL_SAFE_NO_PAD.encode(client),
                "authenticatorData": URL_SAFE_NO_PAD.encode(auth),
                "signature": URL_SAFE_NO_PAD.encode(signer.sign(&message)),
                "userHandle": URL_SAFE_NO_PAD.encode(b"user-1"),
            },
        })
        .to_string()
    }

    /// The flags a passkey with user verification sets at sign-in.
    const SIGNED: u8 = USER_PRESENT | USER_VERIFIED;

    /// The registration response a browser would send for a software authenticator's key, with
    /// `none` attestation.
    fn response(challenge: &[u8], key: &[u8], flags: u8, id: &[u8]) -> String {
        let client = serde_json::json!({
            "type": "webauthn.create",
            "challenge": URL_SAFE_NO_PAD.encode(challenge),
            "origin": ORIGIN,
            "crossOrigin": false,
        })
        .to_string();

        let mut auth = Sha256::digest(RP_ID.as_bytes()).to_vec();
        auth.push(flags);
        auth.extend_from_slice(&7_u32.to_be_bytes());
        auth.extend_from_slice(&[0; 16]);
        auth.extend_from_slice(&u16::try_from(id.len()).expect("a short id").to_be_bytes());
        auth.extend_from_slice(id);
        auth.extend_from_slice(key);
        wrapped(client.as_bytes(), &auth, "none", Cbor::Map(Vec::new()), id)
    }

    /// The response JSON around a client data JSON and an authenticator data.
    fn wrapped(client: &[u8], auth: &[u8], format: &str, statement: Cbor, id: &[u8]) -> String {
        let object = Cbor::Map(vec![
            (Cbor::from("fmt"), Cbor::from(format)),
            (Cbor::from("attStmt"), statement),
            (Cbor::from("authData"), Cbor::Bytes(auth.to_vec())),
        ]);
        let mut written = Vec::new();
        ciborium::ser::into_writer(&object, &mut written).expect("a map writes");
        serde_json::json!({
            "id": URL_SAFE_NO_PAD.encode(id),
            "rawId": URL_SAFE_NO_PAD.encode(id),
            "type": "public-key",
            "response": {
                "clientDataJSON": URL_SAFE_NO_PAD.encode(client),
                "attestationObject": URL_SAFE_NO_PAD.encode(written),
            },
        })
        .to_string()
    }

    /// The flags a passkey with user verification sets at registration.
    const FLAGS: u8 = USER_PRESENT | USER_VERIFIED | ATTESTED;

    #[test]
    fn a_software_authenticators_registration_is_verified_for_each_algorithm() {
        for (cose_alg, algorithm) in [(-7, Algorithm::Es256), (-8, Algorithm::Ed25519)] {
            let challenge = [9_u8; CHALLENGE_LEN];
            let key = cose(cose_alg);
            let id = [3_u8; 20];
            let registered = registration(
                &response(&challenge, &key, FLAGS, &id),
                &challenge,
                RP_ID,
                ORIGIN,
            )
            .expect("a well-formed registration verifies");
            assert_eq!(
                registered,
                Registered {
                    id: id.to_vec(),
                    key: key.clone(),
                    algorithm,
                    sign_count: 7
                }
            );
            assert_eq!(
                key_point(&key).expect("the stored key reads again").0,
                algorithm
            );
        }
    }

    #[test]
    fn a_software_authenticators_sign_in_is_verified_for_each_algorithm() {
        for (cose_alg, algorithm) in [(-7, Algorithm::Es256), (-8, Algorithm::Ed25519)] {
            let challenge = [9_u8; CHALLENGE_LEN];
            let (key, signer) = authenticator(cose_alg);
            let id = [3_u8; 20];
            let mut stored = Registered {
                id: id.to_vec(),
                key,
                algorithm,
                sign_count: 7,
            };
            let sent = assertion(&challenge, &signer, 8, SIGNED, &id);
            assert_eq!(sign_in(&sent, &challenge, &stored, RP_ID, ORIGIN), Ok(8));

            // A device that keeps no counter sends zero every time, and so does the saved passkey.
            stored.sign_count = 0;
            let sent = assertion(&challenge, &signer, 0, SIGNED, &id);
            assert_eq!(sign_in(&sent, &challenge, &stored, RP_ID, ORIGIN), Ok(0));
        }
    }

    #[test]
    fn a_wrong_signature_is_refused() {
        let challenge = [9_u8; CHALLENGE_LEN];
        for cose_alg in [-7, -8] {
            let (key, signer) = authenticator(cose_alg);
            let (_, other) = authenticator(cose_alg);
            let (algorithm, _) = key_point(&key).expect("the key reads");
            let stored = Registered {
                id: vec![3; 20],
                key,
                algorithm,
                sign_count: 0,
            };

            let forged = assertion(&challenge, &other, 1, SIGNED, &[3; 20]);
            let refused = sign_in(&forged, &challenge, &stored, RP_ID, ORIGIN)
                .expect_err("another key's signature is refused");
            assert!(refused.contains("signature"), "{refused}");

            // A signature over another challenge does not verify over this one's client data.
            let mut sent: serde_json::Value =
                serde_json::from_str(&assertion(&challenge, &signer, 1, SIGNED, &[3; 20]))
                    .expect("JSON");
            let replayed = assertion(&[8; CHALLENGE_LEN], &signer, 1, SIGNED, &[3; 20]);
            let replayed: serde_json::Value = serde_json::from_str(&replayed).expect("JSON");
            sent["response"]["signature"] = replayed["response"]["signature"].clone();
            let refused = sign_in(&sent.to_string(), &challenge, &stored, RP_ID, ORIGIN)
                .expect_err("a signature over other client data is refused");
            assert!(refused.contains("signature"), "{refused}");
        }
    }

    #[test]
    fn a_counter_that_does_not_increase_throws() {
        let challenge = [9_u8; CHALLENGE_LEN];
        let (key, signer) = authenticator(-7);
        let stored = Registered {
            id: vec![3; 20],
            key,
            algorithm: Algorithm::Es256,
            sign_count: 7,
        };
        for count in [7, 3, 0] {
            let sent = assertion(&challenge, &signer, count, SIGNED, &[3; 20]);
            let refused = sign_in(&sent, &challenge, &stored, RP_ID, ORIGIN)
                .expect_err("a counter that does not rise is refused");
            assert!(refused.contains("sign counter is"), "{count}: {refused}");
        }
    }

    #[test]
    fn each_sign_in_check_refuses_what_it_reads() {
        let challenge = [9_u8; CHALLENGE_LEN];
        let (key, signer) = authenticator(-8);
        let stored = Registered {
            id: vec![3; 20],
            key,
            algorithm: Algorithm::Ed25519,
            sign_count: 0,
        };
        let good = assertion(&challenge, &signer, 1, SIGNED, &[3; 20]);

        let cases: [(String, &[u8], &str, &str, &str); 5] = [
            (
                assertion(&challenge, &signer, 1, SIGNED, &[4; 20]),
                &challenge,
                RP_ID,
                ORIGIN,
                "another passkey",
            ),
            (
                good.clone(),
                &[8; CHALLENGE_LEN],
                RP_ID,
                ORIGIN,
                "challenge",
            ),
            (
                good.clone(),
                &challenge,
                RP_ID,
                "https://example.org:8443",
                "origin",
            ),
            (good.clone(), &challenge, "example.com", ORIGIN, "domain"),
            (
                assertion(&challenge, &signer, 1, USER_PRESENT, &[3; 20]),
                &challenge,
                RP_ID,
                ORIGIN,
                "verify the user",
            ),
        ];
        for (sent, challenge, rp_id, origin, named) in cases {
            let refused = sign_in(&sent, challenge, &stored, rp_id, origin)
                .expect_err("a response that fails one check is refused");
            assert!(refused.contains(named), "{named}: {refused}");
        }

        // A registration's client data is not a sign-in's.
        let created = response(&challenge, &stored.key, FLAGS, &[3; 20]);
        let refused = sign_in(&created, &challenge, &stored, RP_ID, ORIGIN)
            .expect_err("a registration response is not a sign-in");
        assert!(refused.contains("webauthn.get"), "{refused}");
    }

    #[test]
    fn a_third_algorithm_is_refused_at_registration() {
        let challenge = [9_u8; CHALLENGE_LEN];
        let refused = registration(
            &response(&challenge, &cose(-257), FLAGS, &[3; 20]),
            &challenge,
            RP_ID,
            ORIGIN,
        )
        .expect_err("an RSA key is not registered");
        assert!(refused.contains("COSE algorithm -257"), "{refused}");
    }

    #[test]
    fn each_check_refuses_what_it_reads() {
        let challenge = [9_u8; CHALLENGE_LEN];
        let key = cose(-7);
        let good = response(&challenge, &key, FLAGS, &[3; 20]);

        let cases: [(String, &[u8], &str, &str, &str); 5] = [
            (
                good.clone(),
                &[8; CHALLENGE_LEN],
                RP_ID,
                ORIGIN,
                "challenge",
            ),
            (
                good.clone(),
                &challenge,
                RP_ID,
                "https://example.org:8443",
                "origin",
            ),
            (good.clone(), &challenge, "example.com", ORIGIN, "domain"),
            (
                response(&challenge, &key, USER_PRESENT | ATTESTED, &[3; 20]),
                &challenge,
                RP_ID,
                ORIGIN,
                "verify the user",
            ),
            (
                response(&challenge, &key, USER_VERIFIED | ATTESTED, &[3; 20]),
                &challenge,
                RP_ID,
                ORIGIN,
                "present",
            ),
        ];
        for (sent, challenge, rp_id, origin, named) in cases {
            let refused = registration(&sent, challenge, rp_id, origin)
                .expect_err("a response that fails one check is refused");
            assert!(refused.contains(named), "{named}: {refused}");
        }

        let mut trailing = key.clone();
        trailing.push(0);
        let refused = registration(
            &response(&challenge, &trailing, FLAGS, &[3; 20]),
            &challenge,
            RP_ID,
            ORIGIN,
        )
        .expect_err("an octet after the key is refused");
        assert!(refused.contains("after the new passkey"), "{refused}");
    }

    /// WebAuthn Level 3 § 16.2 (none, ES256) and § 16.11 (packed, Ed25519): the credential id and
    /// key the specification lists are what this module reads out of its attestation object, and
    /// the full check stops at the user-verified flag, which neither vector sets.
    #[test]
    fn the_specifications_vectors_read_and_stop_at_user_verification() {
        let vectors = [
            (
                "7b2274797065223a22776562617574686e2e637265617465222c226368616c6c656e6765223a22414d4d507434557878475453746e63647134313759447742466938767049612d7077386f4f755657345441222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a66616c73652c22657874726144617461223a22636c69656e74446174614a534f4e206d617920626520657874656e6465642077697468206164646974696f6e616c206669656c647320696e20746865206675747572652c207375636820617320746869733a20426b5165446a646354427258426941774a544c453551227d",
                "a363666d74646e6f6e656761747453746d74a068617574684461746158a4bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b559000000008446ccb9ab1db374750b2367ff6f3a1f0020f91f391db4c9b2fde0ea70189cba3fb63f579ba6122b33ad94ff3ec330084be4a5010203262001215820afefa16f97ca9b2d23eb86ccb64098d20db90856062eb249c33a9b672f26df61225820930a56b87a2fca66334b03458abf879717c12cc68ed73290af2e2664796b9220",
                "00c30fb78531c464d2b6771dab8d7b603c01162f2fa486bea70f283ae556e130",
                "f91f391db4c9b2fde0ea70189cba3fb63f579ba6122b33ad94ff3ec330084be4",
                Algorithm::Es256,
                "04afefa16f97ca9b2d23eb86ccb64098d20db90856062eb249c33a9b672f26df61930a56b87a2fca66334b03458abf879717c12cc68ed73290af2e2664796b9220",
            ),
            (
                "7b2274797065223a22776562617574686e2e637265617465222c226368616c6c656e6765223a22714b763532723347734e396a526d733576616e6f6f306f303459557a656c6e7878586d5a426e6254733730222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a66616c73652c22657874726144617461223a22636c69656e74446174614a534f4e206d617920626520657874656e6465642077697468206164646974696f6e616c206669656c647320696e20746865206675747572652c207375636820617320746869733a20425f44543567375a445f2d394f544c59583549764551227d",
                "a363666d74667061636b65646761747453746d74a363616c67266373696758483046022100d83f60bd80269537583218858aefb03ac57d45fa06e42feaae332d187f62da9f022100a02bd3cb6f7e1d283c93bad1f3f4b5a4c0494463da7fdbf256949116754d1f17637835638159022730820223308201c8a003020102021100b2cfc9ea33c8643b0e1a760463eaf164300a06082a8648ce3d0403023062311e301c06035504030c15576562417574686e207465737420766563746f7273310c300a060355040a0c0357334331253023060355040b0c1c41757468656e74696361746f72204174746573746174696f6e204341310b30090603550406130241413020170d3234303130313030303030305a180f33303234303130313030303030305a305f311e301c06035504030c15576562417574686e207465737420766563746f7273310c300a060355040a0c0357334331223020060355040b0c1941757468656e74696361746f72204174746573746174696f6e310b30090603550406130241413059301306072a8648ce3d020106082a8648ce3d03010703420004dd2b7a564b73b8c0b81c4c62e521925c4d1198ec9f583dbf1eebe364b65cd9c29a9bdf346aaa81fb6b9507e5249a52fdaf8e39e26b0b7dc45992a7e233b70f70a360305e300c0603551d130101ff04023000300e0603551d0f0101ff040403020780301d0603551d0e041604140ae27546bc7eccb1b4b597bd354f0c0b1f1f8f8e301f0603551d2304183016801445aff715b0dd786741fee996ebc16547a3931b1e300a06082a8648ce3d0403020349003046022100a0d434ecb5fc3bfd7da5f41904517ad2836249f561bd834ba7a438a8ab7a4ce8022100fac845bb7a02513b58e9f319654dbe49b0f02b95835bac568c71f8a18cdde9ab6861757468446174615881bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b54100000000d5aa33581e8ca478e20fe713f5d32ff20020ce9f840ed96599580cd140fbc7bb3230633f50f61041aff73308ae71caa8a2bda401010327200621582044e06ddd331c36a8dc667bab52bcae63486c916aa5e339e6acebaa84934bf832",
                "a8abf9dabdc6b0df63466b39bda9e8a34a34e185337a59f1c579990676d3b3bd",
                "ce9f840ed96599580cd140fbc7bb3230633f50f61041aff73308ae71caa8a2bd",
                Algorithm::Ed25519,
                "44e06ddd331c36a8dc667bab52bcae63486c916aa5e339e6acebaa84934bf832",
            ),
        ];
        for (client, object, challenge, id, algorithm, point) in vectors {
            let auth =
                attestation_auth_data(&hex(object)).expect("the vector is an attestation object");
            let parsed = auth_data(&auth).expect("the vector's authenticator data has its head");
            let read = attested(&parsed).expect("the vector attests a credential");
            assert_eq!(read.id, hex(id).as_slice());
            assert_eq!(
                key_point(read.key).expect("the vector's key reads"),
                (algorithm, hex(point))
            );
            client_data(&hex(client), "webauthn.create", &hex(challenge), ORIGIN)
                .expect("the vector's client data is for example.org");

            let sent = serde_json::json!({
                "id": URL_SAFE_NO_PAD.encode(hex(id)),
                "type": "public-key",
                "response": {
                    "clientDataJSON": URL_SAFE_NO_PAD.encode(hex(client)),
                    "attestationObject": URL_SAFE_NO_PAD.encode(hex(object)),
                },
            })
            .to_string();
            let refused = registration(&sent, &hex(challenge), RP_ID, ORIGIN)
                .expect_err("neither vector verifies the user");
            assert!(refused.contains("verify the user"), "{refused}");
        }
    }

    /// Releases a value a case took.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the reference released here is the one the case took"
        )]
        unsafe {
            value.release();
        }
    }

    /// A context with a started session that already has an identifier.
    fn session_ctx() -> Ctx {
        let mut ctx = Ctx::buffered();
        ctx.set_random_state(5);
        ctx.open_session(nvs_runtime::Session {
            id: "sid-test".to_owned(),
            record: Vec::new(),
            dirty: false,
            write_back: |_| {},
        });
        ctx
    }

    /// The thrown message of a member call that failed.
    fn thrown(ctx: &mut Ctx) -> String {
        ctx.take_pending()
            .expect("a refusal is a throw a program can catch")
            .into_owned()
    }

    /// The options are built in memory the request's budget does not count, so the limit is
    /// checked against what they will cost before they are built: long names, and many long ids.
    #[test]
    fn options_past_the_memory_limit_stop_before_they_are_built() {
        let mut ctx = session_ctx();
        ctx.set_config(crate::tests::granting("[limits]\nmemory = \"8M\"\n"));
        let long = "n".repeat(2 * 1024 * 1024);
        let named = [
            Value::text(RP_ID),
            Value::text(&long),
            Value::bytes(NvsStr::new(b"user-1")),
            Value::text("ada@example.org"),
            Value::array(nvs_runtime::NvsArray::new()),
        ];
        assert!(
            nvs_runtime::call(nvs_core_passkey_register_options, &mut ctx, &named).is_err(),
            "a 2 MiB name costs past 8 MiB of options"
        );

        let mut ctx = session_ctx();
        ctx.set_config(crate::tests::granting("[limits]\nmemory = \"8M\"\n"));
        let (key, _signer) = authenticator(-8);
        let mut existing = nvs_runtime::NvsArray::new();
        for at in 0..2048_u32 {
            let mut id = vec![0_u8; MAX_ID];
            id[..4].copy_from_slice(&at.to_be_bytes());
            let of_args = [
                Value::bytes(NvsStr::new(&id)),
                Value::bytes(NvsStr::new(&key)),
                Value::int(Algorithm::Ed25519.tag()),
                Value::int(0),
            ];
            existing.append(
                nvs_runtime::call(nvs_core_passkey_credential_of, &mut ctx, &of_args)
                    .expect("a credential under the limit is built"),
            );
        }
        let listed = [Value::text(RP_ID), Value::array(existing)];
        assert!(
            nvs_runtime::call(nvs_core_passkey_sign_in_options, &mut ctx, &listed).is_err(),
            "2048 ids of 1023 bytes cost past 8 MiB of options"
        );
    }

    // covers: Core\Passkey::registerOptions, Core\Passkey::verifyRegistration
    #[test]
    fn the_members_register_once_per_challenge() {
        let mut ctx = session_ctx();
        let options_args = [
            Value::text(RP_ID),
            Value::text("Example"),
            Value::bytes(NvsStr::new(b"user-1")),
            Value::text("ada@example.org"),
            Value::array(nvs_runtime::NvsArray::new()),
        ];
        let options = nvs_runtime::call(nvs_core_passkey_register_options, &mut ctx, &options_args)
            .expect("a started session gets options");
        let json: serde_json::Value =
            serde_json::from_str(options.as_text().expect("the options are text")).expect("JSON");
        released(options);
        assert_eq!(json["user"]["id"], URL_SAFE_NO_PAD.encode(b"user-1"));
        assert_eq!(
            json["authenticatorSelection"]["userVerification"],
            "required"
        );
        let challenge = LENIENT
            .decode(json["challenge"].as_str().expect("a challenge"))
            .expect("base64url");
        assert_eq!(challenge.len(), CHALLENGE_LEN);

        let sent = response(&challenge, &cose(-8), FLAGS, &[4; 16]);
        let verify_args = [Value::text(&sent), Value::text(RP_ID), Value::text(ORIGIN)];
        let credential =
            nvs_runtime::call(nvs_core_passkey_verify_registration, &mut ctx, &verify_args)
                .expect("the answer to this session's challenge verifies");
        let held = credential.obj_ptr().expect("a credential is an object");
        assert_eq!(
            crate::instance::slot(held, ID_SLOT).as_bytes(),
            Some(&[4_u8; 16][..])
        );
        assert_eq!(
            crate::instance::slot(held, ALGORITHM_SLOT).as_int(),
            Some(Algorithm::Ed25519.tag())
        );
        released(credential);

        assert!(
            nvs_runtime::call(nvs_core_passkey_verify_registration, &mut ctx, &verify_args)
                .is_err(),
            "a challenge is accepted once"
        );
        assert!(thrown(&mut ctx).contains("no challenge is waiting"));

        for value in options_args.into_iter().chain(verify_args) {
            released(value);
        }
    }

    // covers: Core\Passkey::signInOptions, Core\Passkey::credentialId, Core\Passkey::verifySignIn
    #[test]
    fn the_members_sign_in_once_per_challenge() {
        let mut ctx = session_ctx();
        let (key, signer) = authenticator(-7);
        let id = [4_u8; 16];
        let of_args = [
            Value::bytes(NvsStr::new(&id)),
            Value::bytes(NvsStr::new(&key)),
            Value::int(Algorithm::Es256.tag()),
            Value::int(7),
        ];
        let stored = nvs_runtime::call(nvs_core_passkey_credential_of, &mut ctx, &of_args)
            .expect("a stored credential is built again");
        let listed = nvs_runtime::call(nvs_core_passkey_credential_of, &mut ctx, &of_args)
            .expect("a stored credential is built again");
        let mut allowed = nvs_runtime::NvsArray::new();
        allowed.append(listed);
        let options_args = [Value::text(RP_ID), Value::array(allowed)];
        let options = nvs_runtime::call(nvs_core_passkey_sign_in_options, &mut ctx, &options_args)
            .expect("a started session gets options");
        let json: serde_json::Value =
            serde_json::from_str(options.as_text().expect("the options are text")).expect("JSON");
        released(options);
        assert_eq!(json["rpId"], RP_ID);
        assert_eq!(json["userVerification"], "required");
        assert_eq!(
            json["allowCredentials"][0]["id"],
            URL_SAFE_NO_PAD.encode(id)
        );
        let challenge = LENIENT
            .decode(json["challenge"].as_str().expect("a challenge"))
            .expect("base64url");

        let sent = assertion(&challenge, &signer, 9, SIGNED, &id);
        let id_args = [Value::text(&sent)];
        let claimed = nvs_runtime::call(nvs_core_passkey_credential_id_of, &mut ctx, &id_args)
            .expect("the response names its id");
        assert_eq!(claimed.as_bytes(), Some(&id[..]));
        released(claimed);

        let verify_args = [
            Value::text(&sent),
            stored,
            Value::text(RP_ID),
            Value::text(ORIGIN),
        ];
        let signed = nvs_runtime::call(nvs_core_passkey_verify_sign_in, &mut ctx, &verify_args)
            .expect("the answer to this session's challenge verifies");
        let held = signed.obj_ptr().expect("a credential is an object");
        assert_eq!(crate::instance::slot(held, COUNT_SLOT).as_int(), Some(9));
        assert_eq!(
            crate::instance::slot(held, ID_SLOT).as_bytes(),
            Some(&id[..])
        );
        released(signed);

        assert!(
            nvs_runtime::call(nvs_core_passkey_verify_sign_in, &mut ctx, &verify_args).is_err(),
            "a challenge is accepted once"
        );
        assert!(thrown(&mut ctx).contains("no challenge is waiting"));

        for value in of_args
            .into_iter()
            .chain(options_args)
            .chain(id_args)
            .chain(verify_args)
        {
            released(value);
        }
    }

    #[test]
    fn the_members_throw_before_a_session_is_started() {
        let mut ctx = Ctx::buffered();
        let args = [Value::text("{}"), Value::text(RP_ID), Value::text(ORIGIN)];
        assert!(nvs_runtime::call(nvs_core_passkey_verify_registration, &mut ctx, &args).is_err());
        assert!(thrown(&mut ctx).contains("Core\\Session::start()"));
        for value in args {
            released(value);
        }
    }
}
