Reads the push subscription that a browser sends to your server. In the browser,
`PushSubscription.toJSON()` returns this JSON text. It has an `endpoint` URL and two keys,
`keys.p256dh` and `keys.auth`. The result is a `Core\WebPush\Subscription`, which you pass to
`Core\WebPush::send`.

`subscription` checks every value once. The endpoint must be an `https:` URL. `p256dh` must be a
public key on the P-256 curve, and `auth` must be 16 bytes. The text must be JSON and at most
8192 bytes. If a check fails, `subscription` throws a `RuntimeError`, and its message says which
check failed. Other fields, such as `expirationTime`, are ignored.

The browser wrote the endpoint, so `endpoint()` returns a `tainted` string.

Save the JSON text with the user, for example in a database column. Before you send a message,
call `subscription` on the saved text again.

The examples read a browser's JSON, catch the error for a text that is not valid, and save a
subscription with the user in a route.
