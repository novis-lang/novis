`Core\Jwt` verifies a token another party issued against a key it **finds** by `kid`, never one it tries,
and answers the claims as a `tainted` shape.

`Jwt::verifyIssued<T>` is the roster's reading half for a token this program did not sign. The algorithm
comes from the key — HS256 under a shared key, and ES256, EdDSA, RS256 or PS256 from a pair or public
key's kind — so the header's `alg` is read only to be compared, as `Core\Jwt` already does.
`rule:security/algorithm-comes-from-the-key` is the rule; what this adds is what happens either side of
the signature check.

**A key is found, never tried.** `kid` is a lookup into the `Jwt\KeySet` and selects nothing else, and a
token carrying no `kid` verifies only against a set holding exactly one key. There is no try-every-key
loop, so a token costs at most one signature check however large the set is — a bound that matters
because the token's sender chooses the `kid` and RSA verification is the dearest operation in the
roster.

**The header policy is deliberately looser than `rule:security/jwe-compact-subset`'s.** `jku`, `x5u`,
`x5c`, `jwk`, `crit`, `b64`, `zip` and `cty` are refused — a key or a fetch the token brings, an
extension, an unencoded or compressed payload, a nested token — and **every other member is ignored**,
because an issuer sends hints such as `x5t` and a verifier refusing them refuses real ID tokens. A JWE
header member we refuse is one we would have had to act on; a JWS header member we ignore cannot select
anything, the key having already decided the algorithm. The token is length-capped before it is parsed.

**The clock and the claims are reached only under a signature that held.** `exp` is required and `nbf` is
checked when present, both under a `leeway` that is `60s` by default and a `LogicError` when negative or
above `5m`. `iss` equals the issuer asked for; `aud` is the audience asked for or a list holding it, and
a list of more than one requires `azp` equal to that audience. A `nonce` asked for is compared in
constant time and an absent one is refused; `typ` is compared case-insensitively with any `application/`
prefix removed; `maxAge` requires an `auth_time` no older than `maxAge + leeway`.

**The order is shape, header policy, key, signature, then the clock and the claims**, and policy and
authenticity are the one `RuntimeError` sentence. Expiry keeps its own message, because only the holder
of a genuinely signed token ever sees it, and a claims refusal names the claim for the same reason.

**The claims come back as a shape, not as flattened text.** `T` is held to
`rule:security/derived-codec-qualifiers` at the call site exactly as `Core\Request::jsonAs<T>`'s is — an
inline shape must be `tainted {…}`, a class must declare `tainted` on every text field reachable from
it, and anything else is a diagnostic naming the field. That is how
`rule:security/verification-does-not-launder` is kept here: by the type, rather than by refusing every
structured claim. `Jwt::verify` is untouched and keeps its `array<tainted string>` answer.

**`Jwt\KeySet::read` skips what it has no use for and refuses what is wrong.** It skips a key marked
`use: enc` and a key whose `alg` or kind is off the roster, because a real JWKS carries keys for purposes
we do not serve and refusing the set over one of them makes every rotation an outage. It refuses the
whole document for a private member (`d`, `p`, `q`, `dp`, `dq`, `qi`, `k`), an RSA key under 2048 bits,
two keys under one `kid`, an `alg` its kind cannot carry, more than 16 keys, a body that is not JSON, and
an RSA key carrying no `alg` when no `rsaScheme` was named. Fetching, caching and discovering a key set
are a flow and therefore a package, not `Core` (`rule:security/protocol-admission-test`).

**Structured claims are signed only under a key pair**, by a member whose key parameter is a
`Crypto\KeyPair` alone, so a token this program signs under a shared key still carries only what
`Jwt::verify` reads back.
