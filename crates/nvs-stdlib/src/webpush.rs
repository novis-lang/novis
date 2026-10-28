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

use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\WebPush";

/// The subscription class's name.
const SUBSCRIPTION_NAME: &str = r"Core\WebPush\Subscription";

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
    methods: &[CoreMethod {
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
    }],
    instance: &[],
    slots: &[],
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

/// The address of one of *this* module's symbols, or `None` for a symbol that belongs to another
/// domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_webpush_subscription" => (nvs_core_webpush_subscription as *const ()).cast(),
        "nvs_core_webpush_subscription_endpoint" => {
            (nvs_core_webpush_subscription_endpoint as *const ()).cast()
        }
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
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "a later stage registers the member")
)]
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
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "a later stage registers the member")
)]
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
}
