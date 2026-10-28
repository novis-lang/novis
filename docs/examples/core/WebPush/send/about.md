Sends one notification to one browser. `send` encrypts the payload for that browser and signs the
request with your VAPID key, a `Core\Crypto\KeyPair` of kind `P256`. Then it posts the message to
the push service of the browser.

The program needs the `net.connect` capability for the host of the endpoint. An endpoint on a
private or link-local address throws a `RuntimeError`. In a web request, a loopback address such
as `127.0.0.1` also throws. A payload over 3993 bytes throws a `RuntimeError` too. A wrong key
kind, a `$subject` that is not a `mailto:` or `https:` URL, or a negative `$ttl` throws a
`LogicError`. Each of these errors happens before `send` connects.

`status()` of the result is `201` when the push service accepted the message. `gone()` is `true`
for the status `404` or `410`. The push service has then deleted the subscription, so delete it
from your database too.

The examples show a wrong subject, a payload that is too long, and a function that deletes the
subscriptions that are gone.
