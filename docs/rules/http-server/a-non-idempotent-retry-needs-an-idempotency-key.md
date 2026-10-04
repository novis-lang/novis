`GET`, `HEAD`, `PUT`, `DELETE`, `OPTIONS` and `TRACE` retry freely. **`POST` and `PATCH` require
`retryIdempotencyKey`**, sent as an `Idempotency-Key` header identical across every attempt — the
convention every payment API already implements, and the difference between a retried request
and a card charged twice.

Because the options bag is a compile-time-constant anonymous object (`rule:core-api/shape-rules`
R2) and the verb is the member's own name (`Client::post`), both halves are statically known at
an ordinary call site, and a `post` that asks for retries without the key is a **diagnostic**
naming the field. It is the verb that decides, asked of every member rather than of one.

Where the verb is genuinely dynamic — `Client::request($method, $url)` — the check
moves to the call and **throws before the first attempt** rather than before the second, so a test
run finds it rather than production finding it on the one retry that matters. A key supplied for
a verb that does not need one is accepted and sent; some servers want it regardless, and refusing
it would buy nothing. A key is never generated automatically: one minted per call is a different
key on the next request, which makes the header present and useless.
