//! `Core\Jwe` — `rule:security/protocol-roster`'s JWE entry: compact JWE with
//! `A256GCM` alone, assembled out of [`crate::crypto`]'s primitives and holding
//! none of its own.
//!
//! `rule:security/jwe-compact-subset` is the rule and states the subset. What
//! belongs here is why a token's key arrives as a named constructor rather than
//! as a value, what the header is allowed to carry, and what a decryption
//! spends.
//!
//! # The static that built the key picks the algorithm
//!
//! `rule:security/algorithm-comes-from-the-key` applied to encryption. There is
//! no `alg` parameter and no inference from a value's type: [`KEY`]'s four
//! statics are the whole choice, and each one *is* an algorithm —
//! `Jwe\Key::shared` is `dir`, `Jwe\Key::password` is `PBES2-HS256+A128KW`, and
//! `Jwe\Key::recipient` and `Jwe\Key::own` are `ECDH-ES` with direct agreement,
//! no key wrap and empty `apu` and `apv`. So the header's `alg` is read only to
//! be **compared** with what the caller already named, exactly as
//! [`crate::jwt`]'s is, and a string out of a token reaches no `match` in this
//! module at all.
//!
//! A named constructor rather than a union parameter, which is the one design
//! question this class had: a union carries no `rule:security/unclassified-parameter-refuses-tainted`
//! classification, so a `secret bytes|Crypto\PublicKey|secret string` parameter
//! would refuse every `tainted` argument — and a password is tainted whether it
//! came off a form or out of `Core\Cli::secret`. `Core\Crypto::deriveKey`'s
//! whole-parameter `secret string` takes that same value today, which is what
//! [`KEY`]'s `password` static mirrors.
//!
//! # `Jwe\Key` is material and nothing else
//!
//! An instance holds which static built it, the octets that static was given,
//! and — for the two agreement kinds — the `Crypto\KeyKind` those octets were
//! read as. There is **no accessor**: a key goes in and a token comes out, so
//! the object is a way of naming an algorithm rather than a way of carrying a
//! secret around. That is the same shape `Core\Csrf` has, and it is why a
//! `secret bytes` may be stored in a slot at all — nothing is registered that
//! could read it back out.
//!
//! The stored material is the *same* material `Core\Crypto\PublicKey` and
//! `Core\Crypto\KeyPair` hold — an SPKI and a PKCS#8 — copied out of the key
//! the caller passed rather than reparsed into some second representation, so a
//! key validated at `Core\Crypto\PublicKey::read` is never less validated here.
//!
//! # The protected header is an allow-list, read after a length cap
//!
//! [`ALLOWED`] is every member a token may carry, and anything else is a
//! refusal — an unknown member in a ciphertext header is an extension this
//! class does not implement or an attack, with no third reading. `zip` is a
//! decompression bomb (`rule:core-classes/decompression-bound`), `jku`, `x5u`
//! and `x5c` are fetches the token is talking the program into, and `crit` and
//! a `jwk` other than `epk` ask the verifier to act on what the token brought.
//! The encoded header is capped at [`MAX_HEADER`] characters before anything
//! parses it.
//!
//! `p2s` and `p2c` are attacker-supplied and are held to
//! `Core\Crypto::deriveKey`'s own bounds — [`crypto::MIN_ITERATIONS`],
//! [`crypto::MAX_ITERATIONS`] and [`crypto::MIN_SALT_LEN`] — checked before the
//! first HMAC. Without the ceiling one token buys unbounded CPU on the request
//! path, and the ring rule below is the same bound one layer up.
//!
//! # What it spends
//!
//! Per call: the payload's own buffer and the token's, both charged to the
//! request through `nvs_runtime::affordable`, plus a few hundred octets of
//! cipher or curve state on the stack. A `Jwe\Key` holds one copy of its
//! material for as long as the program holds the object — 32 octets for a
//! shared key, a password's length, and a little under a hundred octets for
//! either curve's key. Nothing is held between calls, so it is O(in-flight).
//!
//! A `dir` decryption is one AES-GCM open per ring entry. An `ECDH-ES` one is
//! one agreement and one SHA-256 per entry. A PBES2 one is a full PBKDF2, which
//! is why a ring carrying a password key carries exactly one key: a ring an
//! attacker can lengthen is the iteration ceiling defeated by multiplying it.

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, Nonce, Payload};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::Rng as _;

use nvs_runtime::{Fault, NvsStr, Tag, ThrownClass, Value};

use crate::crypto::{self, KeyFormat, KeyKind, PrivateKey, PublicKey};
use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// `Core\Jwe`, as the spec's § 16 writes it.
const NAME: &str = r"Core\Jwe";

/// `Core\Jwe\Key`, the class the four named constructors answer.
pub(crate) const KEY_NAME: &str = r"Core\Jwe\Key";

/// The one content-encryption algorithm, in both directions.
const ENC: &str = "A256GCM";

/// `Jwe\Key::shared`'s algorithm: the key is the content-encryption key.
const DIR: &str = "dir";

/// `Jwe\Key::password`'s algorithm: PBKDF2-HMAC-SHA256 down to a 128-bit
/// key-encryption key, and AES Key Wrap over the drawn content key.
const PBES2: &str = "PBES2-HS256+A128KW";

/// `Jwe\Key::recipient`'s and `Jwe\Key::own`'s algorithm: the agreed secret
/// *is* the content-encryption key, through the Concat KDF and no key wrap.
const ECDH_ES: &str = "ECDH-ES";

/// Which static built a key, in [`KEY`]'s first slot.
const USE_SHARED: i64 = 0;
const USE_PASSWORD: i64 = 1;
const USE_RECIPIENT: i64 = 2;
const USE_OWN: i64 = 3;

/// The slot holding one of the four constants above.
const USE_SLOT: usize = 0;

/// The slot holding the octets the static was given: a key, a password, an
/// SPKI or a PKCS#8.
const MATERIAL_SLOT: usize = 1;

/// The slot holding the `Crypto\KeyKind` tag the two agreement kinds were read
/// as, and [`NO_KIND`] for the two that have no key kind at all.
const KIND_SLOT: usize = 2;

/// What [`KIND_SLOT`] holds for a shared key and for a password, neither of
/// which is a key of a *kind* — no `Crypto\KeyKind` case is negative, so this
/// names no case rather than naming the wrong one.
const NO_KIND: i64 = -1;

/// How long an encoded protected header may be before it is parsed, in
/// characters.
///
/// The widest header this class writes is `ECDH-ES` over P-256, whose `epk`
/// carries two base64url coordinates: a little over 150 characters encoded.
/// The margin above that is for a `kid`, a `typ` and a `cty` another
/// implementation wrote, and the cap is what stops a token from being a parse
/// of arbitrary size before a single check has run.
const MAX_HEADER: usize = 4096;

/// Every protected-header member a token may carry —
/// `rule:security/jwe-compact-subset`'s allow-list, and the whole of it.
///
/// Sorted, because that is the order they are written in and the order a reader
/// compares against the rule.
const ALLOWED: &[&str] = &["alg", "cty", "enc", "epk", "kid", "p2c", "p2s", "typ"];

/// AES-GCM's authentication tag, which compact serialization carries as its own
/// segment rather than after the ciphertext.
const TAG_LEN: usize = 16;

/// The PBES2 salt input this class draws, in octets.
///
/// [`crypto::MIN_SALT_LEN`] is the floor both halves are held to, and `encrypt`
/// draws exactly it: a longer salt buys nothing against an attacker who has the
/// token, since the salt is in the header either way.
const P2S_LEN: usize = crypto::MIN_SALT_LEN;

/// The iteration count this class writes, which is the floor the member enforces
/// (`rule:security/jwe-compact-subset`).
///
/// The count is in the header, so it is the *recipient's* CPU a higher one
/// spends, and a token this class writes is one this class opens on the request
/// path. The floor is the number the bound is defined at; a program needing
/// more is one deriving its own key with `Core\Crypto::deriveKey` and sealing
/// under `Jwe\Key::shared`.
const P2C: u32 = crypto::MIN_ITERATIONS;

/// `Core\Jwe\Key`'s element type, for the ring `decrypt` takes.
const KEY_TY: CoreTy = CoreTy::Instance(KEY_NAME);

/// `rule:security/jwe-compact-subset`'s two members.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "encrypt",
            names: &["payload", "key"],
            // The payload is contagious — the token is made of it — and the key
            // is an object, which carries no qualifier for a row to classify.
            params: &[CoreTy::Text(Qual::Contagious), KEY_TY],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_jwe_encrypt",
            doc: Some(&ENCRYPT_DOC),
        },
        CoreMethod {
            name: "decrypt",
            names: &["token", "keys"],
            // The token is neutral on the way in because the payload is `tainted`
            // on the way out whatever it was: contagion has nothing left to carry,
            // and a contagious row would only refuse the token a request handed in
            // — `admits_tainted_argument` takes a `tainted` argument at such a row
            // only where the qualifier has somewhere new to go.
            // `Core\Jwt::verify` reads the same way for the same reason.
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Array(&KEY_TY)],
            defaults: &[],
            // `rule:security/verification-does-not-launder`: decrypting proves
            // who wrote the payload and never that it is safe for a sink.
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_jwe_decrypt",
            doc: Some(&DECRYPT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Jwe::encrypt`'s reference card — `rule:core-api/reference-card`.
const ENCRYPT_DOC: MethodDoc = MethodDoc {
    short: "Seals `$payload` into a compact JWE token under `$key`, with `A256GCM` content \
            encryption and the key-management algorithm the static that built `$key` names.",
    params: &[
        ParamDoc {
            name: "payload",
            desc: "The text to seal. It comes back from `decrypt` exactly as it went in, \
                   marked `tainted`.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The key, built by one of `Jwe\\Key`'s four statics — which is also what \
                   picks `alg`. A key built by `Jwe\\Key::own` seals to its own public half.",
            shape: &[],
        },
    ],
    ret: "Five dot-separated unpadded base64url segments: the protected header, the encrypted \
          key — empty under `dir` and `ECDH-ES` — the 12-octet IV, the ciphertext and the \
          16-octet tag. Different on every call for the same inputs, because each draws its \
          own IV.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$key` was built from a public key or a pair whose point contributes \
                   nothing to an agreement.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "This process cannot spare a buffer the size of the token.",
        },
    ],
};

/// `Core\Jwe::decrypt`'s reference card — `rule:core-api/reference-card`.
const DECRYPT_DOC: MethodDoc = MethodDoc {
    short: "Opens `$token` against each key in `$keys` in order and answers the payload that \
            was sealed, or throws. The answer is **`tainted`**: decrypting proves who wrote \
            the payload, never that it is safe for a sink.",
    params: &[
        ParamDoc {
            name: "token",
            desc: "The token text, as it arrived. A `tainted` value is accepted here — that \
                   is the point of the member.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The key ring, tried in order, so a token sealed under any key still in it \
                   opens. A ring holding a key built by `Jwe\\Key::password` holds exactly \
                   that one key, because every try costs a full derivation. A key built by \
                   `Jwe\\Key::recipient` is a public key and opens nothing.",
            shape: &[],
        },
    ],
    ret: "The payload, character for character, as `tainted string`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or it holds a `Jwe\\Key::recipient` key, or it holds a \
                   `Jwe\\Key::password` key beside anything else.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$token` is not a token any key in `$keys` opens — it is not five \
                   segments, its header is not the subset this class reads, its `alg` is not \
                   the one `$keys` names, or it was altered. All of them are one sentence on \
                   purpose: telling them apart tells a forger which half landed.",
        },
    ],
};

/// `rule:security/jwe-compact-subset`'s named constructors — one per
/// key-management algorithm, and nothing to read a key back out with.
pub(crate) const KEY: CoreClass = CoreClass {
    name: KEY_NAME,
    methods: &[
        CoreMethod {
            name: "shared",
            names: &["key"],
            // Neutral on the `tainted` axis for `Core\Crypto\KeyPair::read`'s
            // reason: what crosses out is an object, which carries no qualifier
            // at all.
            params: &[CoreTy::SecretBlob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(KEY_NAME),
            symbol: "nvs_core_jwe_key_shared",
            doc: Some(&KEY_SHARED_DOC),
        },
        CoreMethod {
            name: "password",
            names: &["password"],
            // `secret string` because that is what a password *is*, and the one
            // spelling that admits `Core\Cli::secret`'s `secret tainted string`
            // — which is the whole reason this class is four statics rather
            // than one union parameter.
            params: &[CoreTy::SecretText(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(KEY_NAME),
            symbol: "nvs_core_jwe_key_password",
            doc: Some(&KEY_PASSWORD_DOC),
        },
        CoreMethod {
            name: "recipient",
            names: &["key"],
            params: &[CoreTy::Instance(crypto::PUBLIC_KEY_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(KEY_NAME),
            symbol: "nvs_core_jwe_key_recipient",
            doc: Some(&KEY_RECIPIENT_DOC),
        },
        CoreMethod {
            name: "own",
            names: &["key"],
            params: &[CoreTy::Instance(crypto::KEY_PAIR_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(KEY_NAME),
            symbol: "nvs_core_jwe_key_own",
            doc: Some(&KEY_OWN_DOC),
        },
    ],
    instance: &[],
    slots: &["use", "material", "kind"],
    constants: &[],
};

/// `Core\Jwe\Key::shared`'s reference card — `rule:core-api/reference-card`.
const KEY_SHARED_DOC: MethodDoc = MethodDoc {
    short: "A key for `dir`, where the shared key is the content-encryption key itself and \
            nothing is derived and nothing is wrapped.",
    params: &[ParamDoc {
        name: "key",
        desc: "32 octets both ends already hold — `Core\\Crypto::generateKey()` answers one.",
        shape: &[],
    }],
    ret: "A key `encrypt` writes `\"alg\":\"dir\"` for, and `decrypt` accepts only that.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$key` is not 32 octets — a `bytes` that was never a key.",
    }],
};

/// `Core\Jwe\Key::password`'s reference card — `rule:core-api/reference-card`.
const KEY_PASSWORD_DOC: MethodDoc = MethodDoc {
    short: "A key for `PBES2-HS256+A128KW`, where a person's password is stretched to a \
            key-encryption key and the content key is drawn and wrapped under it.",
    params: &[ParamDoc {
        name: "password",
        desc: "The password, of any length. `Core\\Cli::secret` and a form field both reach \
               this parameter; nothing about it is hashed for storage, which is \
               `Core\\Password`'s job and not this one's.",
        shape: &[],
    }],
    ret: "A key `encrypt` derives under at the iteration floor, writing the count and the \
          salt into the header for the other end to repeat.",
    errors: &[],
};

/// `Core\Jwe\Key::recipient`'s reference card — `rule:core-api/reference-card`.
const KEY_RECIPIENT_DOC: MethodDoc = MethodDoc {
    short: "A key for `ECDH-ES` in the sending direction: the other party's public key, which \
            seals a token only that party's pair opens.",
    params: &[ParamDoc {
        name: "key",
        desc: "A `P256` or `X25519` public key, already validated by \
               `Core\\Crypto\\PublicKey::read`.",
        shape: &[],
    }],
    ret: "A key `encrypt` agrees against under a fresh ephemeral pair, writing that pair's \
          public half into the header as `epk`. It opens nothing: a public key is not the \
          half that decrypts.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$key` is an `Ed25519` or RSA key, neither of which agrees on anything.",
    }],
};

/// `Core\Jwe\Key::own`'s reference card — `rule:core-api/reference-card`.
const KEY_OWN_DOC: MethodDoc = MethodDoc {
    short: "A key for `ECDH-ES` in the receiving direction: this program's own key pair, \
            which opens the tokens other parties sealed to its public half.",
    params: &[ParamDoc {
        name: "key",
        desc: "A `P256` or `X25519` key pair, read back with `Core\\Crypto\\KeyPair::read`.",
        shape: &[],
    }],
    ret: "A key `decrypt` agrees with against the token's `epk`, and that `encrypt` seals to \
          this pair's own public half.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$key` is an `Ed25519` or RSA pair, neither of which agrees on anything.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_jwe_encrypt" => (nvs_core_jwe_encrypt as *const ()).cast(),
        "nvs_core_jwe_decrypt" => (nvs_core_jwe_decrypt as *const ()).cast(),
        "nvs_core_jwe_key_shared" => (nvs_core_jwe_key_shared as *const ()).cast(),
        "nvs_core_jwe_key_password" => (nvs_core_jwe_key_password as *const ()).cast(),
        "nvs_core_jwe_key_recipient" => (nvs_core_jwe_key_recipient as *const ()).cast(),
        "nvs_core_jwe_key_own" => (nvs_core_jwe_key_own as *const ()).cast(),
        _ => return None,
    })
}

/// The one refusal `decrypt` has for a token, whatever was wrong with it.
///
/// One sentence for the shape, the header policy, the algorithm and the tag
/// alike, on [`crate::jwt`]'s reasoning: a forger who learns *which* check
/// failed learns which half of the next attempt to change.
fn refused() -> Fault {
    Fault::thrown(
        "Core\\Jwe::decrypt(): $token is not a token any key in $keys opens. Every way of not \
         being one — a shape that is not five base64url segments, a protected header outside \
         the subset this class reads, an `alg` or an `enc` that is not the one $keys names, \
         and an altered token — is this one sentence, so a forgery says nothing about which \
         part of it failed."
            .to_owned(),
    )
}

/// The refusal both agreement constructors give a key that agrees on nothing.
///
/// A `LogicError` rather than a verdict, for `Core\Crypto\KeyPair::read`'s
/// reason: the key is the program's own, so there is nobody a verdict could be
/// about. The sentence names the two kinds that do agree rather than what was
/// wrong with this one, which is what makes it actionable.
fn not_an_agreement_key(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "Core\\Jwe\\Key::{member}(): ECDH-ES agrees over P-256 and X25519, and this key is \
             neither. An Ed25519 key signs and an RSA key has no agreement at all, so there is \
             no shared secret for a content key to come out of."
        ),
    )
}

/// `Core\Jwe\Key`'s three slots, read back off `held`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver or a slot of the wrong tag, neither of
/// which compiled code produces: the parameter's declared type is checked at
/// `E0401` and the slots are this module's own.
fn opened(held: Value, member: &str) -> Result<(i64, Value, Option<KeyKind>), Fault> {
    let receiver = crate::instance::receiver(held, &KEY, member)?;
    let slot_fault = |slot: usize, wanted: &str| {
        Fault::fatal(format!(
            "{KEY_NAME}::{member} expected {wanted} in its `{}` slot",
            KEY.slots[slot]
        ))
    };

    // Unreachable from source: a `Jwe\Key` is built by the four members below
    // and by nothing else, so every slot holds what they put in it.
    let picked = crate::instance::slot(receiver, USE_SLOT)
        .as_int()
        .ok_or_else(|| slot_fault(USE_SLOT, "an `int`"))?;
    let material = crate::instance::slot(receiver, MATERIAL_SLOT);
    let tag = crate::instance::slot(receiver, KIND_SLOT)
        .as_int()
        .ok_or_else(|| slot_fault(KIND_SLOT, "an `int`"))?;
    let kind = if tag == NO_KIND {
        None
    } else {
        Some(KeyKind::from_tag(tag).ok_or_else(|| slot_fault(KIND_SLOT, "a `KeyKind` case"))?)
    };
    Ok((picked, material, kind))
}

/// The octets a slot holds.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `bytes`, which [`opened`]'s doc
/// says no program reaches.
fn material_of<'a>(held: &'a Value, member: &str) -> Result<&'a [u8], Fault> {
    // Unreachable from source: the four constructors below each write a
    // `Value::bytes` into this slot, and nothing else writes one at all.
    held.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{KEY_NAME}::{member} expected a `bytes` in its `{}` slot, got tag {}",
            KEY.slots[MATERIAL_SLOT],
            held.tag_byte()
        ))
    })
}

/// The `string` in argument slot `index`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: the slot is a `string` in the row, so
/// another tag is a compiled-code bug rather than anything a program can write.
fn text_at<'a>(
    args: &'a [Value],
    index: usize,
    member: &str,
    param: &str,
) -> Result<&'a str, Fault> {
    // Unreachable from source: `nvs_types` refuses a non-`string` argument at
    // `E0401` before any of this runs.
    args[index].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `string` for {param}, got tag {}",
            args[index].tag_byte()
        ))
    })
}

/// The key-encryption key PBES2 derives, which is `A128KW`'s 16 octets.
///
/// PBKDF2's output is a prefix of itself — the first block is the whole of a
/// 32-octet derivation and the first half of it is the whole of a 16-octet one
/// — so this is [`crypto::pbkdf2_sha256`] truncated rather than a second
/// derivation beside it. The salt is RFC 7518 § 4.8.1.1's, which is the
/// algorithm name, a zero octet, and then the salt input off the wire: two
/// tokens under one password and one salt input still derive different keys if
/// their `alg` differs.
fn pbes2_key(password: &[u8], salt_input: &[u8], iterations: u32) -> [u8; crypto::KW_128_KEY_LEN] {
    let mut salt = Vec::with_capacity(PBES2.len() + 1 + salt_input.len());
    salt.extend_from_slice(PBES2.as_bytes());
    salt.push(0);
    salt.extend_from_slice(salt_input);

    let derived = crypto::pbkdf2_sha256(password, &salt, iterations);
    let mut kek = [0_u8; crypto::KW_128_KEY_LEN];
    kek.copy_from_slice(&derived[..crypto::KW_128_KEY_LEN]);
    kek
}

/// The content-encryption key an `ECDH-ES` agreement answers, through RFC 7518
/// § 4.6.2's Concat KDF with `A256GCM` as the algorithm identifier and empty
/// `apu` and `apv`.
///
/// Direct agreement, so the derived key *is* the content key and there is no
/// wrapped key segment — which is the shape a browser's WebCrypto produces and
/// the one the frozen vector set holds this class to.
fn agreed_key(mine: &PrivateKey, theirs: &PublicKey) -> Option<[u8; crypto::KEY_LEN]> {
    let shared = crypto::agree(mine, theirs).ok()?;
    let mut cek = [0_u8; crypto::KEY_LEN];
    crypto::concat_kdf(&shared, ENC, &[], &[], &mut cek);
    Some(cek)
}

/// The token's five segments, each still encoded.
///
/// `None` for anything that is not five of them: a compact JWE has exactly four
/// dots, and the JSON serialization has none.
fn segments(token: &str) -> Option<[&str; 5]> {
    let mut parts = token.split('.');
    let found = [
        parts.next()?,
        parts.next()?,
        parts.next()?,
        parts.next()?,
        parts.next()?,
    ];
    parts.next().is_none().then_some(found)
}

/// The protected header a token carries, parsed and held to the allow-list.
///
/// `None` for a header that is too long, is not base64url, is not a JSON
/// object, carries a member outside [`ALLOWED`], or names an `enc` other than
/// [`ENC`]. The `alg` comparison is the caller's, because only the caller knows
/// which static built the key.
fn header_of(encoded: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    if encoded.len() > MAX_HEADER {
        return None;
    }
    let json = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    let serde_json::Value::Object(header) = serde_json::from_slice(&json).ok()? else {
        return None;
    };
    if header.keys().any(|name| !ALLOWED.contains(&name.as_str())) {
        return None;
    }
    (text_member(&header, "enc") == Some(ENC)).then_some(header)
}

/// A header member as text, for the `alg` comparison and for `p2s`.
fn text_member<'a>(
    header: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Option<&'a str> {
    header.get(name).and_then(serde_json::Value::as_str)
}

/// The token assembled out of its five segments, charged to the request first.
///
/// # Errors
///
/// A `RuntimeError` when this process cannot spare the token's own buffer.
fn compact(protected: &str, encrypted_key: &str, iv: &str, body: &[u8]) -> Result<String, Fault> {
    let tag_at = body.len().saturating_sub(TAG_LEN);
    let ciphertext = URL_SAFE_NO_PAD.encode(&body[..tag_at]);
    let tag = URL_SAFE_NO_PAD.encode(&body[tag_at..]);

    let len = protected.len() + encrypted_key.len() + iv.len() + ciphertext.len() + tag.len() + 4;
    nvs_runtime::affordable(Some(len), "Core\\Jwe::encrypt")?;

    let mut token = String::with_capacity(len);
    for part in [protected, encrypted_key, iv, &ciphertext] {
        token.push_str(part);
        token.push('.');
    }
    token.push_str(&tag);
    Ok(token)
}

/// The token, sealed under `cek` with `iv`: the AEAD half of `encrypt`.
///
/// Every value that has to be random is an argument rather than a draw, so a
/// test reproducing a frozen token hands the vector's own IV in and gets the
/// token the member would have written.
///
/// `protected` is the header as it was **written**, not as it is encoded. The
/// additional data is the encoded form, which is what binds `alg`, `enc` and
/// the `epk` to the ciphertext: editing any of them after sealing makes the tag
/// fail rather than changing how the token is read.
///
/// # Errors
///
/// A `RuntimeError` when this process cannot spare the sealed payload's buffer
/// or the token's, and a [`Fault::fatal`] for a key or a payload no program
/// reaches.
fn sealed(
    cek: &[u8; crypto::KEY_LEN],
    iv: &[u8; crypto::GCM_NONCE_LEN],
    protected: &str,
    encrypted_key: &str,
    payload: &[u8],
) -> Result<String, Fault> {
    let protected = URL_SAFE_NO_PAD.encode(protected);

    // Unreachable from source: every caller answers exactly 32 octets, which is
    // the one length AES-256 has.
    let cipher = crypto::gcm_cipher(cek).ok_or_else(|| {
        Fault::fatal("Core\\Jwe::encrypt built no cipher from a 32-octet key".to_owned())
    })?;
    nvs_runtime::affordable(
        Some(payload.len().saturating_add(TAG_LEN)),
        "Core\\Jwe::encrypt",
    )?;
    let body = cipher
        .encrypt(
            &Nonce::<Aes256Gcm>::from(*iv),
            Payload {
                msg: payload,
                aad: protected.as_bytes(),
            },
        )
        .map_err(|_| {
            // Unreachable from source: AES-GCM stops at 2^36 - 32 octets under
            // one nonce, and no `string` a request can hold under any memory
            // cap comes near it.
            Fault::fatal(
                "Core\\Jwe::encrypt was given a payload larger than one nonce can seal".to_owned(),
            )
        })?;

    compact(
        &protected,
        encrypted_key,
        &URL_SAFE_NO_PAD.encode(iv),
        &body,
    )
}

/// `dir`'s header, which is the whole of that algorithm's key management: the
/// key the caller was already holding is the content key, so there is nothing
/// to draw and nothing to put in the encrypted-key segment.
fn direct() -> String {
    format!(r#"{{"alg":"{DIR}","enc":"{ENC}"}}"#)
}

/// `PBES2-HS256+A128KW`'s header and its encrypted key, over a salt and a
/// content key the caller drew.
///
/// # Errors
///
/// A [`Fault::fatal`] where the wrap fails, which no program reaches.
fn wrapped(
    password: &[u8],
    salt: &[u8; P2S_LEN],
    cek: &[u8; crypto::KEY_LEN],
) -> Result<(String, String), Fault> {
    let kek = pbes2_key(password, salt, P2C);
    // Unreachable from source: `wrap_key` refuses only a key-encryption key
    // that is neither 16 nor 32 octets, and `pbes2_key` answers exactly 16 of
    // them.
    let encrypted_key = crypto::wrap_key(&kek, cek).ok_or_else(|| {
        Fault::fatal(
            "Core\\Jwe::encrypt could not wrap a content key under a key-encryption key it had \
             just derived"
                .to_owned(),
        )
    })?;
    Ok((
        format!(
            r#"{{"alg":"{PBES2}","enc":"{ENC}","p2c":{P2C},"p2s":"{}"}}"#,
            URL_SAFE_NO_PAD.encode(salt)
        ),
        URL_SAFE_NO_PAD.encode(encrypted_key),
    ))
}

/// `ECDH-ES`'s header and content key, over an ephemeral pair the caller drew
/// as `der` and a recipient key of the same kind.
///
/// There is no encrypted-key segment to answer: the agreement is direct, so the
/// derived key *is* the content key ([`agreed_key`]).
///
/// # Errors
///
/// A `LogicError` for a recipient key that contributes no shared secret, and a
/// [`Fault::fatal`] where the drawn pair does not read back or its public half
/// does not write.
fn ephemeral(theirs: &PublicKey, der: &[u8]) -> Result<(String, [u8; crypto::KEY_LEN]), Fault> {
    // Unreachable from source: the DER was written by the generator over a
    // scalar of the recipient key's own kind.
    let (mine, epk) = PrivateKey::read(der, theirs.kind())
        .and_then(|mine| mine.public().map(|epk| (mine, epk)))
        .ok_or_else(|| {
            Fault::fatal(
                "Core\\Jwe::encrypt could not read back the ephemeral key it had just drawn"
                    .to_owned(),
            )
        })?;
    let cek = agreed_key(&mine, theirs).ok_or_else(|| {
        Fault::thrown_as(
            ThrownClass::Logic,
            "Core\\Jwe::encrypt(): this key agrees on nothing. Its point contributes no shared \
             secret whatever scalar is put against it, so there is no content key to seal under \
             — a key of that shape is not one a peer should have sent."
                .to_owned(),
        )
    })?;
    // Unreachable from source: a JWK is written from a key this module has
    // already read, and it is ASCII by construction.
    let jwk = epk
        .write(KeyFormat::Jwk)
        .ok()
        .and_then(|octets| String::from_utf8(octets).ok())
        .ok_or_else(|| {
            Fault::fatal(
                "Core\\Jwe::encrypt could not write a JWK for the ephemeral key it had just \
                 derived"
                    .to_owned(),
            )
        })?;
    Ok((
        format!(r#"{{"alg":"{ECDH_ES}","enc":"{ENC}","epk":{jwk}}}"#),
        cek,
    ))
}

/// The instance one of the four constructors answers.
fn built(picked: i64, material: &[u8], kind: i64) -> Value {
    crate::instance::build(
        &KEY,
        [
            Value::int(picked),
            Value::bytes(NvsStr::new(material)),
            Value::int(kind),
        ],
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwe\Key::shared(secret bytes $key): Jwe\Key` — the `dir` key.
    ///
    /// The length is checked here rather than at `encrypt`, so a program that
    /// built the wrong thing learns it where it built it. The shared helper
    /// `crate::crypto::wrong_key_length` is the message, which is the same one
    /// every other member of the roster gives a `bytes` that was never a key.
    fn nvs_core_jwe_key_shared(_ctx, args: [1]) {
        // Unreachable from source: the row declares `secret bytes`, so
        // `nvs_types` refuses another tag at `E0401`.
        let key = args[0].as_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "{KEY_NAME}::shared expected a `bytes` for $key, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        if key.len() != crypto::KEY_LEN {
            return Err(crypto::wrong_key_length(
                "Core\\Jwe\\Key::shared",
                "$key",
                key.len(),
            ));
        }
        Ok(built(USE_SHARED, key, NO_KIND))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwe\Key::password(secret string $password): Jwe\Key` — the
    /// `PBES2-HS256+A128KW` key.
    ///
    /// No length is refused: a password is whatever a person typed, and the
    /// bound that matters is the iteration count, which this class writes
    /// itself and holds a token to on the way back in.
    fn nvs_core_jwe_key_password(_ctx, args: [1]) {
        // Unreachable from source: the row declares `secret string`, so
        // `nvs_types` refuses another tag at `E0401`.
        let password = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{KEY_NAME}::password expected a `string` for $password, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        Ok(built(USE_PASSWORD, password.as_bytes(), NO_KIND))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwe\Key::recipient(Crypto\PublicKey $key): Jwe\Key` — the
    /// `ECDH-ES` key in the sending direction.
    ///
    /// What is stored is the `SubjectPublicKeyInfo` the key object already
    /// holds, so a key validated at `Core\Crypto\PublicKey::read` is exactly as
    /// validated here — the octets are copied, never re-derived.
    fn nvs_core_jwe_key_recipient(_ctx, args: [1]) {
        let (held, kind) = crypto::stored_key(args, 0, &crypto::PUBLIC_KEY, "recipient")?;
        let spki = crypto::stored_octets(&held, &crypto::PUBLIC_KEY, "recipient")?;
        match kind {
            KeyKind::P256 | KeyKind::X25519 => Ok(built(USE_RECIPIENT, spki, kind.tag())),
            KeyKind::Ed25519 | KeyKind::RsaPkcs1 | KeyKind::RsaPss => {
                Err(not_an_agreement_key("recipient"))
            }
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwe\Key::own(Crypto\KeyPair $key): Jwe\Key` — the `ECDH-ES` key in
    /// the receiving direction.
    ///
    /// [`nvs_core_jwe_key_recipient`]'s twin over the private half: the PKCS#8
    /// the pair already holds is what is stored, so the two objects hold one
    /// key between them rather than two encodings of it.
    fn nvs_core_jwe_key_own(_ctx, args: [1]) {
        let (held, kind) = crypto::stored_key(args, 0, &crypto::KEY_PAIR, "own")?;
        let pkcs8 = crypto::stored_octets(&held, &crypto::KEY_PAIR, "own")?;
        match kind {
            KeyKind::P256 | KeyKind::X25519 => Ok(built(USE_OWN, pkcs8, kind.tag())),
            KeyKind::Ed25519 | KeyKind::RsaPkcs1 | KeyKind::RsaPss => {
                Err(not_an_agreement_key("own"))
            }
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwe::encrypt(string $payload, Jwe\Key $key): string` — the write
    /// half of `rule:security/jwe-compact-subset`.
    ///
    /// The header is written **canonically** — members sorted, no whitespace,
    /// at every level, which the `epk` JWK is too because
    /// `Core\Crypto\PublicKey`'s JWK writer answers RFC 7638's required members
    /// in that order. That is what lets a token be held to the frozen vector
    /// set byte for byte rather than only round-tripped against this module.
    ///
    /// **The randomness is drawn here and nowhere below.** The IV, and the
    /// PBES2 salt and content key, and the ephemeral pair come out of
    /// [`crate::random::draw`] in this member; [`wrapped`], [`ephemeral`] and
    /// [`sealed`] take them as arguments, which is what lets a test seal under
    /// a vector's own values and get the token this member would have written.
    ///
    /// There is no `alg` argument to get wrong: which branch below runs is
    /// which static built `$key`, and the header states it rather than being
    /// asked for it.
    fn nvs_core_jwe_encrypt(ctx, args: [2]) {
        let payload = text_at(args, 0, "encrypt", "$payload")?;
        let (picked, held, kind) = opened(args[1], "encrypt")?;
        let material = material_of(&held, "encrypt")?;

        let (protected, encrypted_key, cek) = match picked {
            USE_PASSWORD => {
                let mut salt = [0_u8; P2S_LEN];
                let mut cek = [0_u8; crypto::KEY_LEN];
                crate::random::draw(ctx, |rng| {
                    rng.fill_bytes(&mut salt);
                    rng.fill_bytes(&mut cek);
                });
                let (protected, encrypted_key) = wrapped(material, &salt, &cek)?;
                (protected, encrypted_key, cek)
            }
            USE_RECIPIENT | USE_OWN => {
                let theirs = public_half(picked, material, kind, "encrypt")?;
                let der = crypto::generated_pkcs8(ctx, theirs.kind(), "Core\\Jwe::encrypt")?;
                let (protected, cek) = ephemeral(&theirs, &der)?;
                (protected, String::new(), cek)
            }
            _ => {
                // Unreachable from source: the four constructors write the four
                // constants above and a shared key is checked to be 32 octets
                // where it is built.
                let Ok(cek) = <[u8; crypto::KEY_LEN]>::try_from(material) else {
                    return Err(Fault::fatal(
                        "Core\\Jwe::encrypt held a shared key that is not 32 octets".to_owned(),
                    ));
                };
                (direct(), String::new(), cek)
            }
        };

        let mut iv = [0_u8; crypto::GCM_NONCE_LEN];
        crate::random::draw(ctx, |rng| rng.fill_bytes(&mut iv));

        let token = sealed(&cek, &iv, &protected, &encrypted_key, payload.as_bytes())?;
        Ok(Value::str(NvsStr::new(token.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Jwe::decrypt(string $token, array<Jwe\Key> $keys): tainted string`
    /// — the read half, and the allow-list applied before anything is believed.
    ///
    /// The order is load-bearing. The shape, the header's members and its `enc`
    /// and `alg` are settled before a single octet of key material is touched,
    /// and the payload is answered only under a tag that held — so the most
    /// expensive path, a PBES2 derivation, is reached only by a token that
    /// already claims to be one.
    fn nvs_core_jwe_decrypt(_ctx, args: [2]) {
        let token = text_at(args, 0, "decrypt", "$token")?;
        // Unreachable from source: the row declares `array<Jwe\Key>`, so
        // `nvs_types` refuses another tag at `E0401`.
        let raw = args[1].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::decrypt expected {:?} for $keys, got tag {}",
                Tag::Array,
                args[1].tag_byte()
            ))
        })?;
        let ring = crate::arr::borrowed(raw);
        let mut keys = Vec::new();
        for (_, entry) in crate::keyring::entries(&ring) {
            keys.push(opened(entry, "decrypt")?);
        }

        // The ring's *shape* is settled before the token is looked at, because
        // all three of these are bugs in the call rather than verdicts on a
        // token: a ring that cannot open anything would otherwise report the
        // one sentence a forgery gets, and a password ring holding a key that
        // happens to open first would hide the rule until the day it rotated.
        if keys.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Jwe::decrypt(): $keys is empty, and a key ring needs at least one key \
                 — one of Core\\Jwe\\Key's four statics answers one."
                    .to_owned(),
            ));
        }
        if keys.iter().any(|(picked, _, _)| *picked == USE_RECIPIENT) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Jwe::decrypt(): $keys holds a key built by Core\\Jwe\\Key::recipient, \
                 which is the other party's public key and opens nothing. The half that opens \
                 a token is the pair, through Core\\Jwe\\Key::own."
                    .to_owned(),
            ));
        }
        if keys.len() > 1 && keys.iter().any(|(picked, _, _)| *picked == USE_PASSWORD) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Jwe::decrypt(): $keys holds a key built by Core\\Jwe\\Key::password \
                 beside another key, and a password ring holds exactly one. Every try costs a \
                 full derivation, so a ring whoever sent the token can lengthen is the \
                 iteration ceiling defeated one layer up."
                    .to_owned(),
            ));
        }

        // Each key's octets are borrowed once, because [`plaintext`] walks the
        // ring per token rather than per slot and the material outlives it.
        let mut ring = Vec::new();
        for (picked, held, kind) in &keys {
            ring.push((*picked, material_of(held, "decrypt")?, *kind));
        }

        let Some(plain) = plaintext(&ring, token)? else {
            return Err(refused());
        };
        Ok(Value::str(NvsStr::new(&plain)))
    }
}

/// The public key an `ECDH-ES` encryption seals to: the recipient's own, or the
/// public half of the pair a program named as its own.
///
/// # Errors
///
/// A [`Fault::fatal`] for material that does not read back, which [`opened`]'s
/// doc says no program reaches: the octets were copied out of a key object that
/// had already parsed them.
fn public_half(
    picked: i64,
    material: &[u8],
    kind: Option<KeyKind>,
    member: &str,
) -> Result<PublicKey, Fault> {
    // Unreachable from source: the two agreement constructors are the only
    // writers of this slot pair, and each stores a kind beside the material it
    // took out of an already-validated key.
    let fatal = || {
        Fault::fatal(format!(
            "{NAME}::{member} could not read back the key material a Jwe\\Key holds"
        ))
    };
    let kind = kind.ok_or_else(fatal)?;
    if picked == USE_OWN {
        return PrivateKey::read(material, kind)
            .and_then(|mine| mine.public())
            .ok_or_else(fatal);
    }
    PublicKey::read(material, kind, KeyFormat::Spki).map_err(|_| fatal())
}

/// The content-encryption key this ring entry says the token was sealed under,
/// or `None` for a token this entry does not open at all.
///
/// `None` rather than a refusal per entry, because a ring is *tried*: an entry
/// whose `alg` is not the header's, whose `p2s` is out of bounds or whose `epk`
/// is not a point simply does not open this token, and it is the ring running
/// out that is the one sentence.
///
/// # Errors
///
/// The [`Fault::fatal`] [`public_half`] raises for material that does not read
/// back.
fn content_key(
    picked: i64,
    material: &[u8],
    kind: Option<KeyKind>,
    header: &serde_json::Map<String, serde_json::Value>,
    encrypted_key: &str,
) -> Result<Option<[u8; crypto::KEY_LEN]>, Fault> {
    let alg = text_member(header, "alg");
    match picked {
        USE_PASSWORD => {
            if alg != Some(PBES2) {
                return Ok(None);
            }
            let (Some(p2s), Some(p2c)) = (
                text_member(header, "p2s"),
                header.get("p2c").and_then(serde_json::Value::as_u64),
            ) else {
                return Ok(None);
            };
            let (Ok(salt), Ok(iterations)) = (URL_SAFE_NO_PAD.decode(p2s), u32::try_from(p2c))
            else {
                return Ok(None);
            };
            // The two bounds `Core\Crypto::deriveKey` holds its own arguments
            // to, held here against a header instead: `p2c` is chosen by
            // whoever sent the token, so the ceiling is what stops one token
            // from buying unbounded CPU.
            if salt.len() < crypto::MIN_SALT_LEN
                || !(crypto::MIN_ITERATIONS..=crypto::MAX_ITERATIONS).contains(&iterations)
            {
                return Ok(None);
            }
            let Ok(wrapped) = URL_SAFE_NO_PAD.decode(encrypted_key) else {
                return Ok(None);
            };
            let kek = pbes2_key(material, &salt, iterations);
            Ok(crypto::unwrap_key(&kek, &wrapped))
        }
        USE_OWN => {
            if alg != Some(ECDH_ES) || !encrypted_key.is_empty() {
                return Ok(None);
            }
            let Some(epk) = header.get("epk") else {
                return Ok(None);
            };
            // Unreachable from source: `Core\Jwe\Key::own` stores a kind beside
            // the PKCS#8 it took out of a pair that had already parsed it.
            let kind = kind.ok_or_else(|| {
                Fault::fatal(format!(
                    "{NAME}::decrypt held an agreement key with no kind"
                ))
            })?;
            // The kind is **this program's**, never the token's: an `epk` is
            // read as the curve the program's own key is on, so a token naming
            // another one is a key that does not read rather than a second
            // algorithm to select from.
            let Ok(theirs) = PublicKey::read(epk.to_string().as_bytes(), kind, KeyFormat::Jwk)
            else {
                return Ok(None);
            };
            let Some(mine) = PrivateKey::read(material, kind) else {
                return Ok(None);
            };
            Ok(agreed_key(&mine, &theirs))
        }
        _ => {
            if alg != Some(DIR) || !encrypted_key.is_empty() {
                return Ok(None);
            }
            Ok(<[u8; crypto::KEY_LEN]>::try_from(material).ok())
        }
    }
}

/// The payload one key of `ring` opens `token` to, or `None` for a token none
/// of them opens.
///
/// `decrypt`'s whole verdict, held apart from the member so that [`refused`] has
/// exactly one call site: a shape that is not five base64url segments, a header
/// outside the subset, a key that answers no content key and an AEAD that will
/// not open all arrive back here as the same `None`, and there is no branch left
/// where a second sentence could be written by accident. It is also the seam the
/// frozen set is replayed against, so the Rust side of
/// `rule:testing/feature-proofs` reads a token through the member's own path rather
/// than through a copy of it.
///
/// A ring entry is what `Jwe\Key` holds — which static built it, its octets, and
/// the kind those octets were read as — because the key objects themselves are
/// the member's business and a token's verdict does not depend on them.
///
/// # Errors
///
/// A [`Fault`] for a request that cannot afford the plaintext, and the fatals
/// [`content_key`] raises for material that does not read back.
fn plaintext(
    ring: &[(i64, &[u8], Option<KeyKind>)],
    token: &str,
) -> Result<Option<Vec<u8>>, Fault> {
    let Some([protected, encrypted_key, iv, ciphertext, tag]) = segments(token) else {
        return Ok(None);
    };
    let Some(header) = header_of(protected) else {
        return Ok(None);
    };
    let (Ok(iv), Ok(ciphertext), Ok(tag)) = (
        URL_SAFE_NO_PAD.decode(iv),
        URL_SAFE_NO_PAD.decode(ciphertext),
        URL_SAFE_NO_PAD.decode(tag),
    ) else {
        return Ok(None);
    };
    let Ok(iv): Result<[u8; crypto::GCM_NONCE_LEN], _> = iv.try_into() else {
        return Ok(None);
    };
    if tag.len() != TAG_LEN {
        return Ok(None);
    }

    // The AEAD reads the ciphertext and the tag as one buffer, which compact
    // serialization splits into two segments — so they are put back together
    // once, before the ring is walked, rather than once per key.
    let mut body = Vec::new();
    if body
        .try_reserve_exact(ciphertext.len() + tag.len())
        .is_err()
    {
        return Ok(None);
    }
    body.extend_from_slice(&ciphertext);
    body.extend_from_slice(&tag);
    nvs_runtime::affordable(Some(ciphertext.len()), "Core\\Jwe::decrypt")?;

    for (picked, material, kind) in ring {
        let Some(cek) = content_key(*picked, material, *kind, &header, encrypted_key)? else {
            continue;
        };
        // Unreachable from source: every branch of `content_key` answers 32
        // octets, which is the one length AES-256 has.
        let cipher = crypto::gcm_cipher(&cek).ok_or_else(|| {
            Fault::fatal("Core\\Jwe::decrypt built no cipher from a 32-octet key".to_owned())
        })?;
        let Ok(plain) = cipher.decrypt(
            &Nonce::<Aes256Gcm>::from(iv),
            Payload {
                msg: &body,
                aad: protected.as_bytes(),
            },
        ) else {
            continue;
        };
        return Ok(Some(plain));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::vectors as webcrypto;

    /// One of the set's octet strings as the fixed-width array a seam takes.
    fn fixed<const N: usize>(octets: &[u8], what: &str) -> [u8; N] {
        <[u8; N]>::try_from(octets)
            .unwrap_or_else(|_| panic!("the set's {what} is {N} octets, not {}", octets.len()))
    }

    /// The kind a vector's curve name stands for.
    fn curve(name: &str) -> KeyKind {
        match name {
            "P-256" => KeyKind::P256,
            "X25519" => KeyKind::X25519,
            curve => panic!("the set agrees over {curve}, which ECDH-ES here does not"),
        }
    }

    /// A token's protected header as the text it was encoded from, which is
    /// the form [`sealed`] takes and the form a header is canonical in.
    fn protected_of(token: &str) -> String {
        let encoded = segments(token).expect("the set writes compact tokens")[0];
        String::from_utf8(
            URL_SAFE_NO_PAD
                .decode(encoded)
                .expect("a header is base64url"),
        )
        .expect("a JOSE header is text")
    }

    /// The key ring a case names, in the form [`plaintext`] walks: which static
    /// would have built it, its octets, and the kind those octets read as.
    ///
    /// Every case in the set carries exactly one key, which is what makes the
    /// ring here one entry long — the rules about rings holding more than one
    /// are `decrypt`'s own and are a `.nvst` case's, since they are refusals of
    /// the *call* rather than of a token.
    fn ring_of(case: &serde_json::Value) -> (i64, Vec<u8>, Option<KeyKind>) {
        match webcrypto::text(case, "/key/kind") {
            "shared" => (USE_SHARED, webcrypto::octets(case, "/key/key"), None),
            "password" => (
                USE_PASSWORD,
                webcrypto::text(case, "/key/password").as_bytes().to_vec(),
                None,
            ),
            "keyPair" => (
                USE_OWN,
                webcrypto::octets(case, "/key/pkcs8"),
                Some(curve(webcrypto::text(case, "/key/curve"))),
            ),
            kind => panic!("the set keys a token with a {kind}, which Jwe\\Key does not build"),
        }
    }

    /// Every token the frozen set holds, opened under the key its own case
    /// names, to the payload another implementation sealed into it.
    ///
    /// This is the browser → Novis direction of
    /// `rule:security/jwe-compact-subset`, and all three algorithms of the
    /// subset run through one loop: `dir` reads the content key, `PBES2` derives
    /// it and unwraps it, and `ECDH-ES` agrees it out of the token's own `epk`.
    /// A payload that came back as text at all is not the assertion — the
    /// assertion is that it is *this* text, because an AEAD that opened under
    /// the wrong content key answers nothing rather than something else.
    #[test]
    fn webcrypto_jwe_vectors_decrypt_to_their_payloads() {
        for vector in webcrypto::vectors("jwe") {
            let name = webcrypto::text(vector, "/name");
            let (picked, material, kind) = ring_of(vector);
            let opened = plaintext(
                &[(picked, &material, kind)],
                webcrypto::text(vector, "/token"),
            )
            .expect("a vector's payload is affordable")
            .expect("a vector's token opens under its own key");
            assert_eq!(
                String::from_utf8(opened).expect("the set's payloads are text"),
                webcrypto::text(vector, "/payload"),
                "{name}"
            );
        }
    }

    /// Every `dir` token in the frozen set, written again out of the vector's
    /// own key, IV and payload, and compared to the last character.
    ///
    /// This is the assertion a `.nvst` case cannot make. `encrypt` draws its
    /// own IV, so a case can only open a token this module has just written —
    /// which holds whatever the module does. Holding the whole string to
    /// another implementation's pins the header's canonical order, the empty
    /// encrypted-key segment, and the split of the AEAD's output into a
    /// ciphertext and a tag, none of which a round trip can tell apart from
    /// its own mirror image.
    ///
    /// One of the set's `dir` vectors carries a `kid`, which this class does
    /// not write: `rule:security/jwe-compact-subset` gives `Jwe\Key`'s
    /// constructors one parameter each and none of them is a key name, so that
    /// token is one this class opens and cannot produce. What holds for it is
    /// the rest of its header — strike the member, and it is character for
    /// character the one written here.
    #[test]
    fn webcrypto_jwe_vectors_reencrypt_to_the_same_token_from_the_same_randomness_under_dir() {
        let mut written = 0;
        for vector in webcrypto::vectors("jwe") {
            if webcrypto::text(vector, "/key/kind") != "shared" {
                continue;
            }
            let name = webcrypto::text(vector, "/name");
            let token = webcrypto::text(vector, "/token");
            let header = header_of(segments(token).expect("the set writes compact tokens")[0])
                .expect("the set's header is one this class admits");

            if let Some(kid) = text_member(&header, "kid") {
                assert_eq!(
                    protected_of(token),
                    format!("{},\"kid\":\"{kid}\"}}", direct().trim_end_matches('}')),
                    "{name}, which is this header and the one member this class does not write"
                );
                continue;
            }

            assert_eq!(protected_of(token), direct(), "{name}, the header");
            assert_eq!(
                sealed(
                    &fixed(&webcrypto::octets(vector, "/key/key"), "content key"),
                    &fixed(&webcrypto::octets(vector, "/randomness/iv"), "IV"),
                    &direct(),
                    "",
                    webcrypto::text(vector, "/payload").as_bytes(),
                )
                .expect("a vector's token is affordable"),
                token,
                "{name}"
            );
            written += 1;
        }
        assert!(
            written > 0,
            "the set seals `dir` tokens, and this is the loop that writes them again"
        );
    }

    /// The set's PBES2 token, written again out of the password, the salt, the
    /// content key and the IV it was sealed under.
    ///
    /// Three things are in that one string: PBKDF2 over RFC 7518 § 4.8.1.1's
    /// prefixed salt, AES Key Wrap under the derivation truncated to 128 bits,
    /// and a header whose `p2c` is a number rather than text. The count is
    /// asserted against [`P2C`] first, because this class writes the floor and
    /// a vector taken at any other count would reproduce nothing here however
    /// right the derivation was.
    #[test]
    fn webcrypto_jwe_vectors_reencrypt_to_the_same_token_from_the_same_randomness_under_pbes2() {
        let vector = webcrypto::vectors("jwe")
            .iter()
            .find(|vector| webcrypto::text(vector, "/key/kind") == "password")
            .expect("the set seals one token under a password");
        let name = webcrypto::text(vector, "/name");
        let token = webcrypto::text(vector, "/token");
        let iterations = vector
            .pointer("/randomness/p2c")
            .and_then(serde_json::Value::as_u64)
            .expect("the set writes its iteration count as a number");
        assert_eq!(
            iterations,
            u64::from(P2C),
            "{name}, at the count this class writes"
        );

        let cek = fixed(&webcrypto::octets(vector, "/randomness/cek"), "content key");
        let (protected, encrypted_key) = wrapped(
            webcrypto::text(vector, "/key/password").as_bytes(),
            &fixed(&webcrypto::octets(vector, "/randomness/p2s"), "PBES2 salt"),
            &cek,
        )
        .expect("a derived key is one of the two widths the wrap takes");
        assert_eq!(protected, protected_of(token), "{name}, the header");

        assert_eq!(
            sealed(
                &cek,
                &fixed(&webcrypto::octets(vector, "/randomness/iv"), "IV"),
                &protected,
                &encrypted_key,
                webcrypto::text(vector, "/payload").as_bytes(),
            )
            .expect("a vector's token is affordable"),
            token,
            "{name}"
        );
    }

    /// Both `ECDH-ES` tokens in the set, written again out of the recipient's
    /// own public key and the ephemeral pair WebCrypto drew.
    ///
    /// The randomness here is a key pair rather than a value, which is why
    /// [`ephemeral`] takes a PKCS#8 instead of drawing one: handed the
    /// vector's pair, the `epk` is the set's `epk`, and the token then holds
    /// the Concat KDF, the JWK writer's member order and the empty
    /// encrypted-key segment to another implementation's answer at once.
    #[test]
    fn webcrypto_jwe_vectors_reencrypt_to_the_same_token_from_the_same_randomness_under_ecdh_es() {
        let mut written = 0;
        for vector in webcrypto::vectors("jwe") {
            if webcrypto::text(vector, "/key/kind") != "keyPair" {
                continue;
            }
            let name = webcrypto::text(vector, "/name");
            let token = webcrypto::text(vector, "/token");
            let kind = curve(webcrypto::text(vector, "/key/curve"));
            let theirs = PublicKey::read(
                &webcrypto::octets(vector, "/key/public"),
                kind,
                KeyFormat::Raw,
            )
            .unwrap_or_else(|_| panic!("{name}, whose recipient key is a point on its curve"));

            let (protected, cek) = ephemeral(
                &theirs,
                &webcrypto::octets(vector, "/randomness/ephemeralPkcs8"),
            )
            .expect("the set's ephemeral pair agrees with its recipient");
            assert_eq!(protected, protected_of(token), "{name}, the header");
            assert_eq!(
                sealed(
                    &cek,
                    &fixed(&webcrypto::octets(vector, "/randomness/iv"), "IV"),
                    &protected,
                    "",
                    webcrypto::text(vector, "/payload").as_bytes(),
                )
                .expect("a vector's token is affordable"),
                token,
                "{name}"
            );

            // The read half against the write half's own answer: the ring
            // entry holding the recipient's pair agrees on the content key
            // with the agreement that sealed under it, which is what makes
            // the refusals below `None` for the reason they say.
            let parts = segments(token).expect("the set writes compact tokens");
            let header = header_of(parts[0]).expect("the set's header passes the allow-list");
            assert!(
                matches!(
                    content_key(
                        USE_OWN,
                        &webcrypto::octets(vector, "/key/pkcs8"),
                        Some(kind),
                        &header,
                        parts[1],
                    ),
                    Ok(Some(found)) if found == cek
                ),
                "{name}, agreed again by the pair it was sealed for"
            );
            written += 1;
        }
        assert!(
            written > 0,
            "the set seals `ECDH-ES` tokens, and this is the loop that writes them again"
        );
    }

    /// Every token the set refuses, refused here — the policy half and the
    /// authenticity half alike, and each one held against the key its own case
    /// names rather than against a key that opens nothing anyway.
    ///
    /// The assertion is `None` rather than a sentence because `None` is what
    /// the one sentence is made of: [`plaintext`] is the whole verdict and
    /// [`refused`] is its single call site, so every way of not being a token
    /// this ring opens reaches the caller as the same `RuntimeError` by
    /// construction rather than by each branch remembering to say the same
    /// thing (`rule:security/verification-throws-and-compares-in-constant-time`).
    /// `tests/conformance/core/jwe-refuses-every-unopenable-token-with-one-sentence.nvst`
    /// is the side that reads the sentence itself.
    #[test]
    fn webcrypto_jwe_refusal_vectors_are_all_refused_with_one_runtime_error() {
        for case in webcrypto::refusals("jwe") {
            let name = webcrypto::text(case, "/name");
            let (picked, material, kind) = ring_of(case);
            assert!(
                plaintext(
                    &[(picked, &material, kind)],
                    webcrypto::text(case, "/token")
                )
                .expect("a refusal is refused rather than charged for a plaintext")
                .is_none(),
                "{name}"
            );
        }
    }
}
