`Core\WebPush::send` encrypts one payload for a browser's push subscription as RFC 8291's `aes128gcm`
encoding, with the nonce derived inside and never a parameter, signs a VAPID token with the
application's P-256 key as RFC 8292 says, and sends the message through `Core`'s HTTP client under the
ordinary `net.connect` grant.

| Member | Does |
|---|---|
| `subscription(tainted string $json): WebPush\Subscription` | reads the browser's `PushSubscription.toJSON()`: the endpoint URL and its `p256dh` and `auth` values |
| `applicationServerKey(Crypto\KeyPair $vapid): string` | the public key, base64url, that the browser's `pushManager.subscribe()` needs |
| `send(WebPush\Subscription $to, bytes $payload, Crypto\KeyPair $vapid, string $subject, int $ttl = 86400): WebPush\Sent` | encrypts, signs and sends one message, and returns the push service's answer |

`WebPush\Sent` has the push service's `status` and `gone`, which is `true` for a subscription the
service no longer knows, so the application can drop it. The VAPID key is a `Core\Crypto\KeyPair` of
kind `P256` and stays `secret`; it never appears in a message or an error. The token is signed by the
code `Core\Jwt` already uses for ES256.

**The encryption is fixed.** Each message draws a fresh salt and a fresh P-256 key pair from the system
random source, agrees a secret with the subscription's `p256dh`, and derives the content key and the
nonce with HKDF and the `auth` secret, as RFC 8291 says. A payload larger than one record, 3993 bytes,
throws. A `p256dh` value not on the curve, an `auth` value of the wrong length and an endpoint that is
not `https:` each throw.

**AES-128-GCM is internal to `Core\WebPush`.** No `Core` cipher takes a caller-chosen nonce, and this
class adds none: AES-128-GCM is not a new case of `Core\Crypto\Cipher`, so the cipher roster of
`rule:core-classes/crypto-interop-tier` does not change.

**The endpoint came from a browser, so it is `tainted`**, and the HTTP client's outbound address table
refuses an internal host for it as for any request (`rule:security/net-address-policy`). A subscription
cannot make the server reach its own network.

`Core\WebPush` is admitted to the protocol roster (`rule:security/protocol-roster`). Its usual failure is
a silent library bug — a reused nonce or a wrong key derivation still produces bytes that look
encrypted — and a web application that notifies its users needs it. The registry notifies owners with
it (`rule:packaging/an-owner-is-notified-without-email`).

**Memory:** one encrypted message per call, freed when `send` returns.

**Declined:**
- **AES-128-GCM as a public cipher in `Core\Crypto`.** Its only caller needs a nonce derived by the
  protocol, and a public cipher would need a caller-chosen one.
- **A caller-chosen nonce, salt or record size.** Each is a way to break the encryption, and RFC 8291
  fixes all three.
- **Encryption only, with the sending left to the application.** The application would then build the
  VAPID header itself, which is the part most often wrong.
