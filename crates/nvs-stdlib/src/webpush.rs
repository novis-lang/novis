//! `Core\WebPush` — `rule:core-classes/web-push-sends-an-encrypted-message`: one payload, encrypted
//! for a browser's push subscription as RFC 8291's `aes128gcm` encoding.
//!
//! # The encryption has no parameter
//!
//! RFC 8291 fixes every choice: the curve is P-256, the derivation is HKDF-SHA-256 keyed by the
//! subscription's `auth` secret, the cipher is AES-128-GCM, and a message is one record. What is
//! left is the salt and the sender's key pair, and [`seal`] draws both fresh for every message
//! through [`crate::random::draw`]. The content key and the nonce come out of HKDF over those two
//! and the subscription's keys, so a nonce is never written by a caller and never reused: two
//! messages share one only if they share a salt and a key pair, a collision of 384 random bits.
//!
//! **AES-128-GCM lives in this file and nowhere else.** It is not a case of `Core\Crypto\Cipher`,
//! whose members take a key the program holds and draw a nonce under it, which is the birthday
//! bound that file's module doc is written around. Here the key is derived per message and used
//! once, so the bound does not apply, and the rule's Declined list is why it stays private.
//!
//! # A subscription is an instance with three slots
//!
//! `subscription` reads the JSON a browser's `PushSubscription.toJSON()` writes, checks every
//! value in it once, and keeps the endpoint as text and the two keys as their decoded octets. A
//! `p256dh` that is not an uncompressed point on the curve, an `auth` that is not 16 octets and an
//! endpoint that is not an `https:` URL throw there, so [`seal`] reads keys it can trust. The
//! endpoint came from a browser, so `endpoint()` answers it `tainted`.
//!
//! # A message is signed for its endpoint's origin and posted through the client's door
//!
//! [`vapid_token`] writes RFC 8292's JWT: its audience is the endpoint's origin, it is valid for
//! [`VAPID_LIFETIME`], and it is signed ES256 by `Core\Crypto`'s signing code under the
//! application's `P256` pair, the code `Core\Jwt` signs with. The key never reaches a message or
//! an error. `send` posts through [`crate::http::push_send`], so the endpoint's host is pinned
//! under `net.connect` and the address policy as any `Core\Http\Client` call's is, and no
//! redirect carries the token to a host it does not name. A `404` or a `410` is `gone()`.
//!
//! **Memory:** a subscription is its endpoint plus 81 octets, and a message is the payload plus
//! [`OVERHEAD`] octets, built inside the call and charged to the request. Nothing is held between
//! calls.

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::{Aead as _, KeyInit as _, Nonce};
use base64::Engine as _;
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use hkdf::Hkdf;
use p256::SecretKey as P256SecretKey;
use p256::elliptic_curve::sec1::ToSec1Point as _;
use sha2::Sha256;

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::crypto::{KEY_PAIR, KEY_PAIR_NAME, KeyKind, PrivateKey};
use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
pub(crate) const NAME: &str = r"Core\WebPush";

/// The subscription class's name.
const SUBSCRIPTION_NAME: &str = r"Core\WebPush\Subscription";

/// The answer class's name.
const SENT_NAME: &str = r"Core\WebPush\Sent";

/// How long a VAPID token is valid for: 12 hours. RFC 8292 § 2 caps it at 24, and a token is
/// signed for each message, so the margin is for a clock that is behind.
const VAPID_LIFETIME: i64 = 12 * 60 * 60;

/// `send`'s `$ttl` when the call writes none: one day, in seconds.
const DEFAULT_TTL: i64 = 86_400;

/// The VAPID token's header, RFC 8292 § 2, as base64url of `{"typ":"JWT","alg":"ES256"}`.
const VAPID_HEADER: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiJ9";

/// The length of an uncompressed P-256 point, which is what `p256dh` and the sender's key are.
const POINT_LEN: usize = 65;

/// [`POINT_LEN`] as the header's one-octet key id length.
const KEY_ID_LEN: u8 = 65;

/// The length of the `auth` secret, RFC 8291 § 3.2.
const AUTH_LEN: usize = 16;

/// The length of the salt, RFC 8188 § 2.1.
const SALT_LEN: usize = 16;

/// The record size written into the header. RFC 8291 § 4 caps a push message at 4096 octets, and
/// one record that size holds the whole of it.
const RECORD_SIZE: u32 = 4096;

/// The header in front of the record: the salt, the record size, the key id's length and the key
/// id, which is the sender's public key.
const HEADER_LEN: usize = SALT_LEN + 4 + 1 + POINT_LEN;

/// AES-GCM's tag.
const TAG_LEN: usize = 16;

/// What a message adds to its payload: the header, the padding delimiter and the tag.
const OVERHEAD: usize = HEADER_LEN + 1 + TAG_LEN;

/// The largest payload one message carries: 3993 octets.
const MAX_PAYLOAD: usize = RECORD_SIZE as usize - OVERHEAD;

/// The largest subscription text `subscription` reads. A browser's is a few hundred octets.
const MAX_SUBSCRIPTION: usize = 8 * 1024;

/// The padding delimiter of the last record, RFC 8188 § 2.
const LAST_RECORD: u8 = 0x02;

/// The subscription's endpoint URL.
const ENDPOINT_SLOT: usize = 0;
/// The browser's public key, an uncompressed P-256 point.
const P256DH_SLOT: usize = 1;
/// The browser's `auth` secret.
const AUTH_SLOT: usize = 2;

/// The push service's status code.
const STATUS_SLOT: usize = 0;
/// Whether the push service no longer knows the subscription.
const GONE_SLOT: usize = 1;

/// Base64url as a browser's `toJSON()` writes it, without padding, read with or without it.
const LENIENT: GeneralPurpose = GeneralPurpose::new(
    &alphabet::URL_SAFE,
    GeneralPurposeConfig::new()
        .with_encode_padding(false)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// `rule:core-classes/web-push-sends-an-encrypted-message`'s class.
///
/// The module doc owns why the encryption takes no parameter.
// rank: 977
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "subscription",
            names: &["json"],
            // The text arrives `tainted` from a request body, and the answer is an object, which
            // carries no qualifier: what crosses out is keys this member has checked, and the
            // endpoint, which `endpoint()` answers `tainted` again.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(SUBSCRIPTION_NAME),
            symbol: "nvs_core_webpush_subscription",
            doc: Some(&SUBSCRIPTION_DOC),
        },
        CoreMethod {
            name: "applicationServerKey",
            names: &["vapid"],
            // The public half of the pair, which a page hands the browser: nothing secret leaves.
            params: &[CoreTy::Instance(KEY_PAIR_NAME)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_webpush_application_server_key",
            doc: Some(&APPLICATION_SERVER_KEY_DOC),
        },
        CoreMethod {
            name: "send",
            names: &["to", "payload", "vapid", "subject", "ttl"],
            // The payload is encrypted for one browser, so a `tainted` one is as safe to send as
            // any other. The subject is escaped into a signed claim. The answer is a status the
            // push service chose, so it carries neither.
            params: &[
                CoreTy::Instance(SUBSCRIPTION_NAME),
                CoreTy::Blob(Qual::Neutral),
                CoreTy::Instance(KEY_PAIR_NAME),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Int,
            ],
            defaults: &[Const::Int(DEFAULT_TTL)],
            return_ty: CoreTy::Instance(SENT_NAME),
            symbol: "nvs_core_webpush_send",
            doc: Some(&SEND_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\WebPush\Sent`: what the push service answered one message with.
// rank: 979
pub(crate) const SENT: CoreClass = CoreClass {
    name: SENT_NAME,
    doc: Some(&SENT_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "status",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_webpush_sent_status",
            doc: Some(&STATUS_DOC),
        },
        CoreMethod {
            name: "gone",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_webpush_sent_gone",
            doc: Some(&GONE_DOC),
        },
    ],
    slots: &["status", "gone"],
    constants: &[],
};

/// `Core\WebPush\Subscription`: one browser's push subscription, checked.
///
/// The module doc's *a subscription is an instance with three slots* owns what each slot is.
// rank: 978
pub(crate) const SUBSCRIPTION: CoreClass = CoreClass {
    name: SUBSCRIPTION_NAME,
    doc: Some(&SUBSCRIPTION_CARD),
    methods: &[],
    instance: &[CoreMethod {
        name: "endpoint",
        names: &[],
        params: &[],
        defaults: &[],
        // The browser wrote the URL, so the answer keeps the mark the JSON had.
        return_ty: CoreTy::TaintedStr,
        symbol: "nvs_core_webpush_subscription_endpoint",
        doc: Some(&ENDPOINT_DOC),
    }],
    slots: &["endpoint", "p256dh", "auth"],
    constants: &[],
};

/// `Core\WebPush`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Sends push notifications to a browser (Web Push). The browser gives your page a \
            subscription. Your server reads it with `subscription`. Each message is encrypted \
            for that one browser, and only that browser can read it.",
};

/// `Core\WebPush\Subscription`'s class card — `rule:core-api/reference-card`.
const SUBSCRIPTION_CARD: ClassDoc = ClassDoc {
    short: "A browser's push subscription, after `Core\\WebPush::subscription` checked it. It \
            contains the URL of the browser's push service and the two keys that encrypt a \
            message for this browser. Save the JSON text with the user, and read it again when \
            you send.",
};

/// `Core\WebPush::subscription`'s reference card — `rule:core-api/reference-card`.
const SUBSCRIPTION_DOC: MethodDoc = MethodDoc {
    short: "Reads a push subscription from the JSON that `PushSubscription.toJSON()` returns in \
            the browser. It checks the endpoint URL and both keys. A subscription that fails a \
            check throws an error here.",
    params: &[ParamDoc {
        name: "json",
        desc: "The JSON text the browser sent, such as `{\"endpoint\": \"https://…\", \"keys\": \
               {\"p256dh\": \"…\", \"auth\": \"…\"}}`. Other fields, such as `expirationTime`, \
               are ignored.",
        shape: &[],
    }],
    ret: "The subscription, ready to send a message to.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The text is not JSON or is longer than 8 KiB. Or the endpoint is not an `https:` \
               URL, `p256dh` is not a P-256 public key, or `auth` is not 16 bytes. The message \
               says which.",
    }],
};

/// `Core\WebPush\Subscription::endpoint`'s reference card — `rule:core-api/reference-card`.
const ENDPOINT_DOC: MethodDoc = MethodDoc {
    short: "Returns the URL of the browser's push service for this subscription. Use it to find \
            the subscription again, for example to delete it. The browser wrote this URL, so it \
            is `tainted`.",
    params: &[],
    ret: "The endpoint URL, such as `https://push.example.com/send/abc123`.",
    errors: &[],
};

/// `Core\WebPush\Sent`'s class card — `rule:core-api/reference-card`.
const SENT_CARD: ClassDoc = ClassDoc {
    short: "The answer of the browser's push service to one message that \
            `Core\\WebPush::send` sent. Check `gone()` to know when to delete a subscription.",
};

/// The `$vapid` parameter, which `applicationServerKey` and `send` share.
const VAPID_PARAM: ParamDoc = ParamDoc {
    name: "vapid",
    desc: "Your application's VAPID key: a `Core\\Crypto\\KeyPair` of kind `P256`. Create it once \
           with `Core\\Crypto::generateKeyPair`, keep it secret, and use the same key for every \
           message.",
    shape: &[],
};

/// The error both members throw for a key of another kind.
const NOT_P256: ErrorDoc = ErrorDoc {
    error: "LogicError",
    desc: "`$vapid` is not a key pair of kind `P256`.",
};

/// `Core\WebPush::applicationServerKey`'s reference card — `rule:core-api/reference-card`.
const APPLICATION_SERVER_KEY_DOC: MethodDoc = MethodDoc {
    short: "Returns the public half of your VAPID key, as base64url text. Give it to \
            `pushManager.subscribe()` in the browser as `applicationServerKey`. The private half \
            is not in it.",
    params: &[VAPID_PARAM],
    ret: "The public key: 87 characters of base64url, such as `BCVxsr7N_eNg…`.",
    errors: &[NOT_P256],
};

/// `Core\WebPush::send`'s reference card — `rule:core-api/reference-card`.
const SEND_DOC: MethodDoc = MethodDoc {
    short: "Encrypts `$payload` for one browser and sends it to the browser's push service. The \
            message is signed with your VAPID key. The program needs `net.connect` for the \
            host of the endpoint.",
    params: &[
        ParamDoc {
            name: "to",
            desc: "The subscription that `Core\\WebPush::subscription` returned.",
            shape: &[],
        },
        ParamDoc {
            name: "payload",
            desc: "The message, at most 3993 bytes. The browser's service worker receives it in \
                   its `push` event.",
            shape: &[],
        },
        VAPID_PARAM,
        ParamDoc {
            name: "subject",
            desc: "A way for the push service to contact you: a `mailto:` or an `https:` URL, \
                   such as `mailto:admin@example.com`.",
            shape: &[],
        },
        ParamDoc {
            name: "ttl",
            desc: "How many seconds the push service keeps the message while the browser is \
                   offline. `0` means the message is sent only if the browser is online now.",
            shape: &[],
        },
    ],
    ret: "The push service's answer. Its `status()` is `201` when the service accepted the \
          message.",
    errors: &[
        NOT_P256,
        ErrorDoc {
            error: "LogicError",
            desc: "`$subject` does not start with `mailto:` or `https:`, or `$ttl` is negative.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The payload is longer than 3993 bytes, the program may not connect to the \
                   host, or the push service cannot be reached.",
        },
    ],
};

/// `Core\WebPush\Sent::status`'s reference card — `rule:core-api/reference-card`.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "Returns the HTTP status code of the push service's answer.",
    params: &[],
    ret: "The status code, such as `201` for a message the service accepted.",
    errors: &[],
};

/// `Core\WebPush\Sent::gone`'s reference card — `rule:core-api/reference-card`.
const GONE_DOC: MethodDoc = MethodDoc {
    short: "Returns `true` when the push service no longer knows the subscription. The status is \
            then `404` or `410`. Delete the subscription, because every later message fails too.",
    params: &[],
    ret: "`true` for a subscription the service has deleted, and `false` otherwise.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that belongs to another
/// domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_webpush_subscription" => (nvs_core_webpush_subscription as *const ()).cast(),
        "nvs_core_webpush_subscription_endpoint" => {
            (nvs_core_webpush_subscription_endpoint as *const ()).cast()
        }
        "nvs_core_webpush_application_server_key" => {
            (nvs_core_webpush_application_server_key as *const ()).cast()
        }
        "nvs_core_webpush_send" => (nvs_core_webpush_send as *const ()).cast(),
        "nvs_core_webpush_sent_status" => (nvs_core_webpush_sent_status as *const ()).cast(),
        "nvs_core_webpush_sent_gone" => (nvs_core_webpush_sent_gone as *const ()).cast(),
        _ => return None,
    })
}

/// A subscription's values, each checked.
#[derive(Debug, PartialEq, Eq)]
struct Subscription {
    endpoint: String,
    p256dh: [u8; POINT_LEN],
    auth: [u8; AUTH_LEN],
}

/// Reads and checks `PushSubscription.toJSON()`'s text.
///
/// # Errors
///
/// One sentence for the first value that fails, without the member prefix a throw carries.
fn read_subscription(text: &str) -> Result<Subscription, String> {
    if text.len() > MAX_SUBSCRIPTION {
        return Err(format!(
            "the subscription is {} bytes, and the limit is {MAX_SUBSCRIPTION}.",
            text.len()
        ));
    }
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|_| "the subscription is not JSON.".to_owned())?;
    let endpoint = json
        .get("endpoint")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "the subscription has no `endpoint` string.".to_owned())?;
    let scheme_ends = endpoint.find("://").unwrap_or(0);
    if !endpoint[..scheme_ends].eq_ignore_ascii_case("https")
        || endpoint.len() == scheme_ends + "://".len()
        || crate::uri::validate(endpoint).is_err()
    {
        return Err("the endpoint is not an `https:` URL.".to_owned());
    }
    let keys = json
        .get("keys")
        .ok_or_else(|| "the subscription has no `keys` object.".to_owned())?;
    let p256dh: [u8; POINT_LEN] = key(keys, "p256dh")?
        .try_into()
        .ok()
        .filter(|point: &[u8; POINT_LEN]| {
            point[0] == 0x04 && crate::crypto::read_p256_point(point).is_some()
        })
        .ok_or_else(|| "`p256dh` is not a P-256 public key.".to_owned())?;
    let auth: [u8; AUTH_LEN] = key(keys, "auth")?
        .try_into()
        .map_err(|_| format!("`auth` is not {AUTH_LEN} bytes."))?;
    Ok(Subscription {
        endpoint: endpoint.to_owned(),
        p256dh,
        auth,
    })
}

/// The base64url value `keys.<name>` decoded.
fn key(keys: &serde_json::Value, name: &str) -> Result<Vec<u8>, String> {
    keys.get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("the subscription has no `keys.{name}` string."))
        .and_then(|value| {
            LENIENT
                .decode(value)
                .map_err(|_| format!("`keys.{name}` is not base64url."))
        })
}

/// One `aes128gcm` message: `payload` encrypted for a browser's `p256dh` and `auth` under the
/// sender's `secret` scalar and `salt`, RFC 8291 § 3.4 and RFC 8188 § 2.
///
/// Every step is the RFC's, and its § 5 example is the test of this function. The salt and the
/// scalar are arguments so that example can fix them; [`seal`] is the one caller that is not a
/// test, and it draws both.
///
/// # Errors
///
/// A sentence for a payload over [`MAX_PAYLOAD`] octets, and for a `secret` that is not a scalar
/// of the curve or a `p256dh` that is not a point on it, which [`read_subscription`] and [`seal`]
/// have each already refused.
fn encrypt(
    payload: &[u8],
    p256dh: &[u8; POINT_LEN],
    auth: &[u8; AUTH_LEN],
    salt: &[u8; SALT_LEN],
    secret: &[u8],
) -> Result<Vec<u8>, String> {
    if payload.len() > MAX_PAYLOAD {
        return Err(format!(
            "the payload is {} bytes, and one message holds at most {MAX_PAYLOAD}.",
            payload.len()
        ));
    }
    let browser = crate::crypto::read_p256_point(p256dh)
        .ok_or_else(|| "`p256dh` is not a P-256 public key.".to_owned())?;
    let mine = P256SecretKey::from_slice(secret)
        .map_err(|_| "the sender's key is not a P-256 private key.".to_owned())?;
    let sender = mine.public_key().to_sec1_point(false);
    let shared = crate::crypto::agree_p256(secret, &browser)
        .ok_or_else(|| "the sender's key is not a P-256 private key.".to_owned())?;

    // RFC 8291 § 3.4: the input keying material, keyed by `auth` and bound to both public keys.
    let mut key_info = Vec::with_capacity(14 + 2 * POINT_LEN);
    key_info.extend_from_slice(b"WebPush: info\0");
    key_info.extend_from_slice(p256dh);
    key_info.extend_from_slice(sender.as_bytes());
    let mut ikm = [0_u8; 32];
    expand(auth, &shared, &key_info, &mut ikm);

    // RFC 8188 § 2.2 and § 2.3: the content key and the nonce, from the salt.
    let mut cek = [0_u8; 16];
    expand(salt, &ikm, b"Content-Encoding: aes128gcm\0", &mut cek);
    let mut nonce = [0_u8; 12];
    expand(salt, &ikm, b"Content-Encoding: nonce\0", &mut nonce);

    let mut plaintext = Vec::with_capacity(payload.len() + 1);
    plaintext.extend_from_slice(payload);
    plaintext.push(LAST_RECORD);
    let cipher = Aes128Gcm::new_from_slice(&cek).map_err(|_| "the content key".to_owned())?;
    // Unreachable: GCM refuses only a message past 2^36 octets, and this one is at most 3994.
    let record = cipher
        .encrypt(&Nonce::<Aes128Gcm>::from(nonce), plaintext.as_slice())
        .map_err(|_| "the payload could not be encrypted.".to_owned())?;

    let mut message = Vec::with_capacity(HEADER_LEN + record.len());
    message.extend_from_slice(salt);
    message.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    message.push(KEY_ID_LEN);
    message.extend_from_slice(sender.as_bytes());
    message.extend_from_slice(&record);
    Ok(message)
}

/// HKDF-SHA-256: extract `material` under `salt`, then expand under `info` into `out`.
fn expand(salt: &[u8], material: &[u8], info: &[u8], out: &mut [u8]) {
    Hkdf::<Sha256>::new(Some(salt), material)
        .expand(info, out)
        .expect("expand refuses only an output past 255 SHA-256 blocks, and these are at most one");
}

/// The values of the `WebPush\Subscription` in `value`, read back from its slots.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not a subscription, or a slot that does not hold what
/// `subscription` wrote into it. The checker has already refused the first, and nothing but
/// `subscription` builds one.
fn subscription_of(value: Value, member: &str) -> Result<Subscription, Fault> {
    let held = crate::instance::receiver(value, &SUBSCRIPTION, member)?;
    let broken = |slot: usize| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a subscription's `{}` in its slot",
            SUBSCRIPTION.slots[slot]
        ))
    };
    let endpoint = crate::instance::slot(held, ENDPOINT_SLOT)
        .as_text()
        .map(str::to_owned)
        .ok_or_else(|| broken(ENDPOINT_SLOT))?;
    let p256dh = crate::instance::slot(held, P256DH_SLOT)
        .as_bytes()
        .and_then(|octets| octets.try_into().ok())
        .ok_or_else(|| broken(P256DH_SLOT))?;
    let auth = crate::instance::slot(held, AUTH_SLOT)
        .as_bytes()
        .and_then(|octets| octets.try_into().ok())
        .ok_or_else(|| broken(AUTH_SLOT))?;
    Ok(Subscription {
        endpoint,
        p256dh,
        auth,
    })
}

/// `payload` encrypted for `to`, with a salt and a key pair drawn for this message alone.
///
/// The module doc's *the encryption has no parameter* is why both are drawn here and nowhere
/// else. A drawn scalar outside the curve's order is drawn again, which happens with odds of
/// about 2^-32.
///
/// # Errors
///
/// A `RuntimeError` naming `who` for a payload over [`MAX_PAYLOAD`] octets, and whatever
/// `nvs_runtime::affordable` answers for a request at its ceiling.
fn seal(
    ctx: &mut nvs_runtime::Ctx,
    payload: &[u8],
    to: &Subscription,
    who: &str,
) -> Result<Vec<u8>, Fault> {
    nvs_runtime::affordable(Some(payload.len().saturating_add(OVERHEAD)), who)?;
    let mut salt = [0_u8; SALT_LEN];
    let mut secret = [0_u8; 32];
    crate::random::draw(ctx, |rng| {
        use rand::Rng as _;
        rng.fill_bytes(&mut salt);
        loop {
            rng.fill_bytes(&mut secret);
            if P256SecretKey::from_slice(&secret).is_ok() {
                break;
            }
        }
    });
    let sealed = encrypt(payload, &to.p256dh, &to.auth, &salt, &secret)
        .map_err(|why| Fault::thrown(format!("{who}(): {why}")));
    secret.fill(0);
    sealed
}

/// The VAPID key pair in `args[slot]`, which is of kind `P256`.
///
/// # Errors
///
/// A `LogicError` naming `member` for a pair of another kind, and a [`Fault::fatal`] for a pair
/// whose stored DER no longer parses, which `Core\Crypto\KeyPair::read` already parsed.
fn vapid_of(args: &[Value], slot: usize, member: &str) -> Result<PrivateKey, Fault> {
    let (held, kind) = crate::crypto::stored_key(args, slot, &KEY_PAIR, member)?;
    if !matches!(kind, KeyKind::P256) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{NAME}::{member}(): $vapid is not a key pair of kind `P256`."),
        ));
    }
    let der = crate::crypto::stored_octets(&held, &KEY_PAIR, member)?;
    match PrivateKey::read(der, kind) {
        Some(pair @ PrivateKey::P256(_)) => Ok(pair),
        _ => Err(Fault::fatal(format!(
            "{NAME}::{member} held a `{KEY_PAIR_NAME}` that is no longer a P-256 private key"
        ))),
    }
}

/// The public half of a pair [`vapid_of`] read, uncompressed and base64url, as a browser's
/// `applicationServerKey` and RFC 8292 § 3.2's `k` parameter both take it.
fn public_of(pair: &PrivateKey) -> String {
    let PrivateKey::P256(secret) = pair else {
        unreachable!("`vapid_of` answers a P-256 pair alone");
    };
    LENIENT.encode(secret.public_key().to_sec1_point(false).as_bytes())
}

/// The origin of an endpoint `read_subscription` checked: `https://` and its authority without
/// any user part, which is the audience RFC 8292 § 2 asks for.
fn origin_of(endpoint: &str) -> String {
    let rest = &endpoint[endpoint.find("://").map_or(0, |at| at + "://".len())..];
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();
    format!("https://{}", host.to_ascii_lowercase())
}

/// The VAPID token for one message to `endpoint`: RFC 8292 § 2's JWT, its audience the
/// endpoint's origin, valid for [`VAPID_LIFETIME`] from now, and signed ES256 by `pair` through
/// the signing code `Core\Jwt` uses.
///
/// # Errors
///
/// A `LogicError` for a `subject` that is not a `mailto:` or an `https:` URL, and a
/// [`Fault::fatal`] for a clock out of range or a signature that could not draw its randomness.
fn vapid_token(
    ctx: &nvs_runtime::Ctx,
    pair: PrivateKey,
    endpoint: &str,
    subject: &str,
    member: &str,
) -> Result<String, Fault> {
    let lowered = subject.to_ascii_lowercase();
    let contact = lowered
        .strip_prefix("mailto:")
        .or_else(|| lowered.strip_prefix("https://"));
    if contact.is_none_or(str::is_empty) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{NAME}::{member}(): $subject is not a `mailto:` or an `https:` URL."),
        ));
    }
    let now = crate::time::wall_clock(ctx)
        .map(|at| at.as_second())
        .ok_or_else(|| Fault::fatal(format!("{NAME}::{member} found a clock out of range")))?;
    let claims = serde_json::json!({
        "aud": origin_of(endpoint),
        "exp": now.saturating_add(VAPID_LIFETIME),
        "sub": subject,
    });
    let mut token = format!("{VAPID_HEADER}.{}", LENIENT.encode(claims.to_string()));
    let key = pair
        .signing()
        .ok_or_else(|| Fault::fatal(format!("{NAME}::{member} held a pair that signs nothing")))?;
    let signature = crate::crypto::sign(&key, token.as_bytes()).ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} could not draw the randomness a signature needs"
        ))
    })?;
    token.push('.');
    token.push_str(&LENIENT.encode(signature));
    Ok(token)
}

nvs_runtime::nvs_helper! {
    /// `Core\WebPush::subscription(string $json): WebPush\Subscription` — the browser's
    /// subscription, every value checked once.
    fn nvs_core_webpush_subscription(_ctx, args: [1]) {
        let who = "Core\\WebPush::subscription";
        let text = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!("{who} expected a `string` for $json, got tag {}", args[0].tag_byte()))
        })?;
        let read = read_subscription(text).map_err(|why| Fault::thrown(format!("{who}(): {why}")))?;
        nvs_runtime::affordable(Some(read.endpoint.len() + POINT_LEN + AUTH_LEN), who)?;
        Ok(crate::instance::build(
            &SUBSCRIPTION,
            [
                Value::text(&read.endpoint),
                Value::bytes(NvsStr::new(&read.p256dh)),
                Value::bytes(NvsStr::new(&read.auth)),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$subscription->endpoint(): tainted string`.
    fn nvs_core_webpush_subscription_endpoint(_ctx, args: [1]) {
        crate::instance::read_slot(args, &SUBSCRIPTION, ENDPOINT_SLOT, "endpoint")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\WebPush::applicationServerKey(Crypto\KeyPair $vapid): string` — the public half,
    /// base64url.
    fn nvs_core_webpush_application_server_key(_ctx, args: [1]) {
        let pair = vapid_of(args, 0, "applicationServerKey")?;
        Ok(Value::text(&public_of(&pair)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\WebPush::send(WebPush\Subscription $to, bytes $payload, Crypto\KeyPair $vapid,
    /// string $subject, int $ttl = 86400): WebPush\Sent` — one message, encrypted, signed and
    /// sent.
    ///
    /// Every argument is judged before anything is drawn or sent, so a call that throws a
    /// `LogicError` has reached no host.
    fn nvs_core_webpush_send(ctx, args: [5]) {
        let who = "Core\\WebPush::send";
        let to = subscription_of(args[0], "send")?;
        let payload = args[1].as_bytes().ok_or_else(|| {
            Fault::fatal(format!("{who} expected `bytes` for $payload, got tag {}", args[1].tag_byte()))
        })?;
        let pair = vapid_of(args, 2, "send")?;
        let subject = args[3].as_text().ok_or_else(|| {
            Fault::fatal(format!("{who} expected a `string` for $subject, got tag {}", args[3].tag_byte()))
        })?;
        let ttl = args[4].as_int().ok_or_else(|| {
            Fault::fatal(format!("{who} expected an `int` for $ttl, got tag {}", args[4].tag_byte()))
        })?;
        if ttl < 0 {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("{who}(): $ttl is {ttl}, and a message is kept for 0 seconds or more."),
            ));
        }
        let key = public_of(&pair);
        let token = vapid_token(ctx, pair, &to.endpoint, subject, "send")?;
        let message = seal(ctx, payload, &to, who)?;
        let headers = vec![
            ("TTL".to_owned(), ttl.to_string()),
            ("Content-Encoding".to_owned(), "aes128gcm".to_owned()),
            ("Content-Type".to_owned(), "application/octet-stream".to_owned()),
            ("Authorization".to_owned(), format!("vapid t={token}, k={key}")),
        ];
        let status = crate::http::push_send(ctx, who, &to.endpoint, headers, message)?;
        Ok(crate::instance::build(
            &SENT,
            [Value::int(status), Value::bool(matches!(status, 404 | 410))],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$sent->status(): int`.
    fn nvs_core_webpush_sent_status(_ctx, args: [1]) {
        crate::instance::read_slot(args, &SENT, STATUS_SLOT, "status")
    }
}

nvs_runtime::nvs_helper! {
    /// `$sent->gone(): bool`.
    fn nvs_core_webpush_sent_gone(_ctx, args: [1]) {
        crate::instance::read_slot(args, &SENT, GONE_SLOT, "gone")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A value of RFC 8291 § 5's example, decoded.
    fn b64(text: &str) -> Vec<u8> {
        LENIENT
            .decode(text)
            .expect("the example's values are base64url")
    }

    /// The browser's public key of RFC 8291 § 5.
    const UA_PUBLIC: &str =
        "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
    /// The browser's `auth` secret of RFC 8291 § 5.
    const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";

    fn subscription_json(endpoint: &str, p256dh: &str, auth: &str) -> String {
        serde_json::json!({
            "endpoint": endpoint,
            "expirationTime": null,
            "keys": { "p256dh": p256dh, "auth": auth },
        })
        .to_string()
    }

    #[test]
    fn rfc_8291_example_message_matches_its_published_bytes() {
        let p256dh: [u8; POINT_LEN] = b64(UA_PUBLIC).try_into().unwrap();
        let auth: [u8; AUTH_LEN] = b64(AUTH).try_into().unwrap();
        let salt: [u8; SALT_LEN] = b64("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();
        let secret = b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw");

        let message = encrypt(
            b"When I grow up, I want to be a watermelon",
            &p256dh,
            &auth,
            &salt,
            &secret,
        )
        .unwrap();

        assert_eq!(
            message,
            b64(
                "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
            )
        );
    }

    #[test]
    fn a_payload_over_one_record_throws_and_one_that_fills_it_does_not() {
        let p256dh: [u8; POINT_LEN] = b64(UA_PUBLIC).try_into().unwrap();
        let auth: [u8; AUTH_LEN] = b64(AUTH).try_into().unwrap();
        let secret = b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw");

        let full = encrypt(&[7; MAX_PAYLOAD], &p256dh, &auth, &[1; SALT_LEN], &secret).unwrap();
        assert_eq!(full.len(), RECORD_SIZE as usize);
        assert_eq!(MAX_PAYLOAD, 3993);

        let over = encrypt(
            &[7; MAX_PAYLOAD + 1],
            &p256dh,
            &auth,
            &[1; SALT_LEN],
            &secret,
        );
        assert_eq!(
            over.unwrap_err(),
            "the payload is 3994 bytes, and one message holds at most 3993."
        );
    }

    #[test]
    fn a_subscription_reads_a_browsers_json_and_checks_each_value() {
        let read = read_subscription(&subscription_json(
            "https://push.example.com/send/abc",
            UA_PUBLIC,
            AUTH,
        ))
        .unwrap();
        assert_eq!(read.endpoint, "https://push.example.com/send/abc");
        assert_eq!(read.p256dh.to_vec(), b64(UA_PUBLIC));
        assert_eq!(read.auth.to_vec(), b64(AUTH));

        let refused = |json: &str| read_subscription(json).unwrap_err();
        assert_eq!(refused("{"), "the subscription is not JSON.");
        assert_eq!(
            refused(&subscription_json(
                "http://push.example.com/send/abc",
                UA_PUBLIC,
                AUTH
            )),
            "the endpoint is not an `https:` URL."
        );
        assert_eq!(
            refused(&subscription_json("https://", UA_PUBLIC, AUTH)),
            "the endpoint is not an `https:` URL."
        );
        // A point whose last octet is changed is off the curve.
        let mut off = b64(UA_PUBLIC);
        off[64] ^= 1;
        assert_eq!(
            refused(&subscription_json(
                "https://push.example.com/send/abc",
                &LENIENT.encode(&off),
                AUTH
            )),
            "`p256dh` is not a P-256 public key."
        );
        assert_eq!(
            refused(&subscription_json(
                "https://push.example.com/send/abc",
                UA_PUBLIC,
                "BTBZMqHH6r4Tts7J_aSI"
            )),
            "`auth` is not 16 bytes."
        );
        assert_eq!(
            refused(&"x".repeat(MAX_SUBSCRIPTION + 1)),
            "the subscription is 8193 bytes, and the limit is 8192."
        );
    }

    #[test]
    fn two_sealed_messages_never_share_a_salt_or_a_key() {
        let to = read_subscription(&subscription_json(
            "https://push.example.com/send/abc",
            UA_PUBLIC,
            AUTH,
        ))
        .unwrap();
        let built = crate::instance::build(
            &SUBSCRIPTION,
            [
                Value::text(&to.endpoint),
                Value::bytes(NvsStr::new(&to.p256dh)),
                Value::bytes(NvsStr::new(&to.auth)),
            ],
        );
        assert_eq!(subscription_of(built, "send").unwrap(), to);

        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
        let first = seal(&mut ctx, b"hello", &to, "test").unwrap();
        let second = seal(&mut ctx, b"hello", &to, "test").unwrap();
        assert_eq!(first.len(), 5 + OVERHEAD);
        assert_ne!(first[..SALT_LEN], second[..SALT_LEN]);
        assert_ne!(first[21..HEADER_LEN], second[21..HEADER_LEN]);
    }

    /// The browser's private key of RFC 8291 § 5, which [`UA_PUBLIC`] is the public half of.
    const UA_PRIVATE: &str = "q1dXpw3UpT5VOmu_cf_v6ih07Aems3njxI-JWgLcM94";

    /// A `Core\Crypto\KeyPair` of kind `P256` over a fixed scalar, as `KeyPair::read` stores one.
    fn vapid_pair() -> Value {
        use p256::pkcs8::EncodePrivateKey as _;
        let secret = P256SecretKey::from_slice(&b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw"))
            .expect("the scalar is on the curve");
        let der = secret.to_pkcs8_der().expect("a P-256 key encodes");
        crate::instance::build(
            &KEY_PAIR,
            [
                Value::bytes(NvsStr::new(der.as_bytes())),
                Value::int(KeyKind::P256.tag()),
            ],
        )
    }

    /// One request a loopback push service read: its head, lower-cased, and its body.
    struct Pushed {
        head: String,
        body: Vec<u8>,
    }

    /// A push service on loopback that terminates TLS under the `localhost` certificate the
    /// process's client trusts, reads one request whole, and answers `status` with no body.
    fn push_service(status: u16) -> (u16, std::thread::JoinHandle<Pushed>) {
        use std::io::{Read as _, Write as _};
        let (cert, key) = crate::tests::outbound_client().clone();
        let listener =
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).expect("a port");
        let port = listener.local_addr().expect("its own address").port();
        let served = std::thread::spawn(move || {
            let config = std::sync::Arc::new(
                rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .expect("the shipped versions")
                .with_no_client_auth()
                .with_single_cert(
                    vec![cert],
                    rustls::pki_types::PrivateKeyDer::Pkcs8(key.into()),
                )
                .expect("the certificate and its key"),
            );
            let (mut sock, _) = listener.accept().expect("the client connects");
            drop(sock.set_nonblocking(false));
            sock.set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .expect("a read timeout");
            let mut conn = rustls::ServerConnection::new(config).expect("a session");
            let mut tls = rustls::Stream::new(&mut conn, &mut sock);
            let mut read = Vec::new();
            let mut chunk = [0_u8; 4096];
            let (head, body) = loop {
                let got = tls.read(&mut chunk).expect("the request arrives");
                assert_ne!(got, 0, "the client closed before the request ended");
                read.extend_from_slice(&chunk[..got]);
                let Some(end) = read.windows(4).position(|four| four == b"\r\n\r\n") else {
                    continue;
                };
                let head = String::from_utf8_lossy(&read[..end]).into_owned();
                let length: usize = header_of(&head, "content-length")
                    .map_or(0, |value| value.parse().expect("a length"));
                if read.len() >= end + 4 + length {
                    break (head, read[end + 4..end + 4 + length].to_vec());
                }
            };
            write!(tls, "HTTP/1.1 {status} Pushed\r\nContent-Length: 0\r\n\r\n")
                .expect("the answer is written");
            tls.flush().expect("the answer is sent");
            Pushed { head, body }
        });
        (port, served)
    }

    /// What the browser does with a message: RFC 8291 § 3.4 run as the receiver.
    fn decrypted(message: &[u8]) -> Vec<u8> {
        let salt = &message[..SALT_LEN];
        assert_eq!(message[20], KEY_ID_LEN, "the key id is the sender's point");
        let sender = &message[21..HEADER_LEN];
        let shared = crate::crypto::agree_p256(
            &b64(UA_PRIVATE),
            &crate::crypto::read_p256_point(sender).expect("the sender's point"),
        )
        .expect("the agreement");
        let mut key_info = b"WebPush: info\0".to_vec();
        key_info.extend_from_slice(&b64(UA_PUBLIC));
        key_info.extend_from_slice(sender);
        let mut ikm = [0_u8; 32];
        expand(&b64(AUTH), &shared, &key_info, &mut ikm);
        let mut cek = [0_u8; 16];
        expand(salt, &ikm, b"Content-Encoding: aes128gcm\0", &mut cek);
        let mut nonce = [0_u8; 12];
        expand(salt, &ikm, b"Content-Encoding: nonce\0", &mut nonce);
        let mut plain = Aes128Gcm::new_from_slice(&cek)
            .expect("a 16-octet key")
            .decrypt(&Nonce::<Aes128Gcm>::from(nonce), &message[HEADER_LEN..])
            .expect("the record is authentic");
        assert_eq!(plain.pop(), Some(LAST_RECORD));
        plain
    }

    /// The value of header `name` in a request head, the name compared without case.
    fn header_of<'a>(head: &'a str, name: &str) -> Option<&'a str> {
        head.lines().find_map(|line| {
            let (written, value) = line.split_once(':')?;
            written.eq_ignore_ascii_case(name).then_some(value.trim())
        })
    }

    /// [`header_of`] for a header the request must carry.
    fn header<'a>(head: &'a str, name: &str) -> &'a str {
        header_of(head, name)
            .unwrap_or_else(|| panic!("the request has no `{name}` header:\n{head}"))
    }

    /// `send` to a push service on loopback, the way a program calls it, and the request the
    /// service read: the subscription's endpoint is `https://localhost`, reached under a grant.
    fn sent_to(status: u16, payload: &[u8]) -> (Value, Pushed, u16, nvs_runtime::Ctx) {
        let (port, served) = push_service(status);
        let mut ctx = nvs_runtime::Ctx::buffered();
        // Loopback is an internal address, so the test writes the operator's exception for it.
        ctx.set_config(crate::tests::granting(
            "[capabilities.net]\nconnect = [\"localhost\"]\ninternal = [\"127.0.0.1\", \"::1\"]\n",
        ));
        let json = subscription_json(
            &format!("https://localhost:{port}/push/abc"),
            UA_PUBLIC,
            AUTH,
        );
        let to = nvs_runtime::call(
            nvs_core_webpush_subscription,
            &mut ctx,
            &[Value::text(&json)],
        )
        .expect("the subscription reads");
        let sent = nvs_runtime::call(
            nvs_core_webpush_send,
            &mut ctx,
            &[
                to,
                Value::bytes(NvsStr::new(payload)),
                vapid_pair(),
                Value::text("mailto:admin@example.com"),
                Value::int(60),
            ],
        )
        .expect("the message is sent");
        (sent, served.join().expect("the push service"), port, ctx)
    }

    #[cfg_attr(
        miri,
        ignore = "`ring` runs C and assembly, which Miri cannot interpret"
    )]
    #[test]
    fn a_push_service_decrypts_what_send_posts_and_checks_its_vapid_token() {
        let (sent, pushed, port, mut ctx) = sent_to(201, b"New message from Shop");

        use ring::signature::{ECDSA_P256_SHA256_FIXED, UnparsedPublicKey};
        assert!(
            pushed.head.starts_with("POST /push/abc HTTP/1.1"),
            "{}",
            pushed.head
        );
        assert_eq!(header(&pushed.head, "ttl"), "60");
        assert_eq!(header(&pushed.head, "content-encoding"), "aes128gcm");
        assert_eq!(decrypted(&pushed.body), b"New message from Shop");

        // RFC 8292 § 3: the token and the public key it verifies under.
        let (token, key) = header(&pushed.head, "authorization")
            .strip_prefix("vapid t=")
            .and_then(|rest| rest.split_once(", k="))
            .expect("RFC 8292 § 3's `vapid t=…, k=…`");
        let published = nvs_runtime::call(
            nvs_core_webpush_application_server_key,
            &mut ctx,
            &[vapid_pair()],
        )
        .expect("the public key");
        assert_eq!(Some(key), published.as_text());
        let (signed, signature) = token.rsplit_once('.').expect("a signed token");
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, LENIENT.decode(key).unwrap())
            .verify(signed.as_bytes(), &LENIENT.decode(signature).unwrap())
            .expect("the service verifies the token under `k`");
        let claims = signed.split_once('.').expect("a header and claims").1;
        let claims: serde_json::Value =
            serde_json::from_slice(&LENIENT.decode(claims).unwrap()).unwrap();
        assert_eq!(claims["aud"], format!("https://localhost:{port}"));

        let status = nvs_runtime::call(nvs_core_webpush_sent_status, &mut ctx, &[sent]);
        assert_eq!(status.expect("a status").as_int(), Some(201));
        let gone = nvs_runtime::call(nvs_core_webpush_sent_gone, &mut ctx, &[sent]);
        assert_eq!(gone.expect("an answer").as_bool(), Some(false));
    }

    #[cfg_attr(
        miri,
        ignore = "`ring` runs C and assembly, which Miri cannot interpret"
    )]
    #[test]
    fn a_410_answer_is_a_subscription_that_is_gone() {
        let (sent, _, _, mut ctx) = sent_to(410, b"hello");
        let status = nvs_runtime::call(nvs_core_webpush_sent_status, &mut ctx, &[sent]);
        assert_eq!(status.expect("a status").as_int(), Some(410));
        let gone = nvs_runtime::call(nvs_core_webpush_sent_gone, &mut ctx, &[sent]);
        assert_eq!(gone.expect("an answer").as_bool(), Some(true));
    }

    #[cfg_attr(
        miri,
        ignore = "`ring` runs C and assembly, which Miri cannot interpret"
    )]
    #[test]
    fn a_vapid_token_names_the_endpoints_origin_and_verifies_under_the_public_key() {
        use ring::signature::{ECDSA_P256_SHA256_FIXED, UnparsedPublicKey};
        let args = [vapid_pair()];
        let pair = vapid_of(&args, 0, "send").unwrap();
        let key = LENIENT.decode(public_of(&pair)).unwrap();
        let ctx = nvs_runtime::Ctx::buffered();
        let token = vapid_token(
            &ctx,
            pair,
            "https://Push.Example.com:8443/send/abc?x=1",
            "mailto:admin@example.com",
            "send",
        )
        .unwrap();

        let (signed, signature) = token.rsplit_once('.').unwrap();
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, &key)
            .verify(signed.as_bytes(), &LENIENT.decode(signature).unwrap())
            .expect("the token verifies under the published key");
        let (header, claims) = signed.split_once('.').unwrap();
        assert_eq!(
            LENIENT.decode(header).unwrap(),
            br#"{"typ":"JWT","alg":"ES256"}"#
        );
        let claims: serde_json::Value =
            serde_json::from_slice(&LENIENT.decode(claims).unwrap()).unwrap();
        assert_eq!(claims["aud"], "https://push.example.com:8443");
        assert_eq!(claims["sub"], "mailto:admin@example.com");
        let now = crate::time::wall_clock(&ctx).unwrap().as_second();
        let exp = claims["exp"].as_i64().unwrap();
        assert!(
            exp > now && exp <= now + VAPID_LIFETIME,
            "{exp} against {now}"
        );

        let refused = vapid_token(
            &ctx,
            vapid_of(&args, 0, "send").unwrap(),
            "https://a.example/",
            "admin",
            "send",
        );
        assert!(
            matches!(refused, Err(Fault::Thrown(ThrownClass::Logic, ref why)) if why.contains("$subject")),
            "{refused:?}"
        );
    }
}
