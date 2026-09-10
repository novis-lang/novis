---
milestone: M8
---
# Loop goal 32 — signing a URL, and the payload behind it

`rule:core-api/signing-is-over-a-payload`, built.
One general member — `Core\Signature`, `rule:security/protocol-roster`'s
fifth and final roster entry — and the two doors onto it where a reader will actually look:
`$uri->sign`/`$uri->verifySignature`, and `Core\Router`'s pair for the one case a path cannot express.

**The design is finished and this goal does not re-open it.** `rule:core-api/signing-is-over-a-payload` is written, 0060's roster is
already five and 0077 § 4 already lists the two router members. What is missing is every line of
implementation, the spec rows are written against nothing, and
`crates/nvs-stdlib/tests/spec-members-outstanding.txt` names this goal as the owner of two of them.

**It sits after goal `unowned-sweep`** because it *adds surface* and 28 is the last entry that closes what is behind
it — and because stage 3's options bag is the one goal `unowned-sweep` stage 2 lands: `{keys, until}` needs an
optional-versus-written-`null` distinction the registry could not spell before it, and `until` being a
**required key holding a nullable value** is that same spelling read the other way round.

## Why here

It goes after goal `unowned-sweep` for a mechanical reason: `{keys, until}` is an options bag whose `until` is a
required key holding a nullable value, and telling an omitted key from a written `null` is precisely
what goal `input-shapes` spelled and goal `unowned-sweep` stage 2 is the second caller of.

## Stage 0 — the catch-up

1. **Three spec rows exist and nothing implements them.** `docs/spec/01-core-library.md` § 12 carries
   `$uri->sign` and `$uri->verifySignature`, § 16 carries `Core\Signature`. Each is struck from its
   outstanding-key file in the same slice that registers it, never before and never after —
   `spec-members-outstanding.txt` for the first two and `spec-classes-part-two-outstanding.txt` for the
   third, both keyed `# 29`.
2. **`crates/nvs-stdlib/src/uri.rs`'s module doc says the normalization "lives on `compareTo` and
   nowhere else".** That sentence is rewritten when stage 3 lands, not amended: after it, one function
   is reached by two members, which is the point rather than an exception.

## Stage 1 — the floor

Goal `unowned-sweep`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: `Core\Signature`, over a payload and never over text

```php
Core\Signature::sign(array<string, mixed> $payload,
                     {keys: array<secret bytes>, until: ?Time\Instant}): string;
Core\Signature::verify(string $token, array<secret bytes> $keys): array<string, mixed>;
```

1. **The canonical payload encoding is this stage's whole risk**, so it is written once, in
   `crates/nvs-stdlib/src/signature.rs`, and every other member in this goal calls it. Keys sorted,
   each value encoded with its type so `1` and `"1"` are different bytes, `until` inside the signed
   region rather than beside it.
2. **The key ring is `Core\SignedCookie`'s, not a second one** — `array<secret bytes>`, newest at
   `[0]`, sign under `$keys[0]` alone and verify down the ring in order. If the two classes end up
   with two ring walks, one of them is wrong; share the helper.
3. **The token is unpadded URL-safe base64**, RFC 4648 § 5, which `signed_cookie.rs` already emits for
   the same reason: every octet is legal in a query string and in a `Set-Cookie` alike, so nothing
   downstream escapes it twice.
4. **`verify` answers a `tainted` payload or throws.** `rule:security/verification-does-not-launder` is the rule and `rule:core-classes/signature` says
   why `Core\SignedCookie`'s laundering exemption does not reach here — the round trip may cross two
   services, so "the application authored this plaintext" is not a property the checker can see.

## Stage 3 — a `Uri` signs itself, over the form `compareTo` already defines

```php
$uri->sign({keys: array<secret bytes>, until: ?Time\Instant}): Uri;
$uri->verifySignature(array<secret bytes> $keys): void;
```

1. **`equivalent()` gains a second caller and is not copied.** `uri.rs`'s existing RFC 3986 § 6.2.2
   normalization is what is signed — the same function `compareTo` calls. A session that finds it
   easier to write a second normalization for the signature has written the bug this whole ADR exists
   to remove.
2. **`_sig` is reserved**, carries the tag and the lifetime together, is excluded from its own input,
   and a URL carrying two of them fails rather than resolving to one.
3. **Every component present is covered and the fragment is never covered.** Appending a query
   parameter invalidates; changing a fragment does not, because RFC 3986 § 3.5 fragments never reach
   the server. There is no option naming which parameters are signed — `rule:core-api/signing-is-over-a-payload`'s *Alternatives
   rejected* is the home of why that option is the bypass.
4. **`sign` answers a `Uri`** so it composes with `with` and `toString`; signing one that already
   carries `_sig` replaces it rather than nesting.
5. **`verifySignature` answers nothing and throws.** There are no claims to hand back — the claim is
   the URL the caller is holding.

## Stage 4 — the router pair, for the mount prefix and nothing else

```php
Core\Router::urlSigned(string $name, array<string, mixed> $params,
                       {keys: array<secret bytes>, until: ?Time\Instant}): string;
Core\Router::signedRoute(array<secret bytes> $keys): Router\Match;
```

1. **These sign the route's *identity*, not its path.** `rule:core-classes/router-signed-url`: one compiled table serves at
   `/ModuleA`, `/ModuleB` or `/` (`rule:http-server/a-mount-table-expands-at-boot`
   ), so a signature over an assembled path stops verifying when a mount moves and one over the
   route name and its typed parameters does not. **That property is the acceptance test**, not an
   aside.
2. **`signedRoute` verifies against `Core\Request::route()`** — the match the server already made
   (`rule:routing/matched-once-before-the-handler`
   ) — and re-parses nothing.
3. **Nothing verifies automatically.** `Core\Router` does not dispatch and this does not change it; the
   application calls `signedRoute` where it keeps its own refusal.
4. **`urlSigned` launders for the URL-path sink** exactly as `url` does, and prepends the mount prefix
   the same way.

## Stage 5 — one refusal, except expiry

1. **Every way of not being authentic is one error with one sentence** — not a token, too short, one
   flipped bit, a retired key, a missing `_sig`, two of them. `signed_cookie.rs` owns why a
   distinguishable "wrong key" leaks, and this is the same argument.
2. **Expiry is the one distinguishable failure, and the ordering is what makes it safe.** The signature
   is checked first and the clock only after, so `SignatureExpired` is reachable only by someone
   already holding a valid signature. **A test proves the ordering**: a token both forged *and* past
   its `until` throws the *invalid* error, never the expired one.
3. **`{until: null}` is the forever spelling and omitting the key does not compile** — `rule:core-api/a-lifetime-is-written`,
   which is `rule:security/access-is-checked-for-presence-not-meaning`
   's rule one surface over.

## Standing decisions

- **This goal opens no ADR number.** `rule:core-api/signing-is-over-a-payload` is the design, written before the goal existed. A session
  that finds a genuine hole in it folds the fix into 0146's body and says so in the handoff — it does
  not open 0147.
- **The canonical form is `compareTo`'s, and that is not a session's call to revisit.** A second
  normalization written for the signature alone is the defect this goal exists to not ship, however
  reasonable it looks at the call site.
- **Sign-and-reveal is not seal-and-hide.** `Core\SignedCookie` is AEAD and its payload is hidden; a
  signature's payload is visible and must be. The two share the key ring and the base64 and nothing
  else, and neither loses a row to the other (`rule:core-api/each-door-takes-a-different-thing`).
- **No options bag names which parameters are signed**, ever, at any of the three doors. If a caller
  needs a URL where some parameter is free, that parameter does not belong in the signed URL.
- **Verification never renders anything.** It throws; the application catches and decides. A session
  that finds itself writing a default error page has left this goal's scope.
- **What this spends**, per `rule:programs/memory-priority`: one signature
  computation per `sign`, one per key tried until one authenticates, all inside the call and nothing
  held between calls. A program that signs nothing pays nothing — no table, no registry walk, no
  per-request cost. The token adds about `4/3 × (payload + 40)` characters to a URL.
