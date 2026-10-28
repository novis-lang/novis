Returns the public half of your VAPID key as base64url text. The browser needs this text before it
can create a push subscription.

A VAPID key is a key pair that identifies your server to the push services of the browsers. It is a
`Core\Crypto\KeyPair` of kind `P256`. Create it once and store it. The page passes the text from
this method to `pushManager.subscribe()` as `applicationServerKey`. Later, `Core\WebPush::send`
signs every message with the same key pair. A message signed with another key is not delivered.

The text is 87 characters long, and it contains only the public key. The private half stays on
your server. A key pair of another kind throws a `LogicError`, and the error message does not
contain the key.

The examples show the length of the text, a route that gives the text to the page, and a key of
the wrong kind.
