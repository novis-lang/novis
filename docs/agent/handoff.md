# Handoff

## State

**Goal `webcrypto`, stage 6: `Core\Jwt::signObject` is on disk, registered and reachable.**
`signObject(object $claims, Duration $lifetime, Crypto\KeyPair $key, {kid?, typ?, embedKey?})`
(`crates/nvs-stdlib/src/jwt.rs:324`) writes the claims as `Core\Json::encode` writes them, then
`iat` and `exp` in that order, and signs under the pair's own algorithm.

**The claims parameter is `object`, a new `CoreTy` variant**
(`crates/nvs-stdlib/src/registry.rs:415`), lowered to `Ty::Object` at
`crates/nvs-types/src/core_lib.rs:605`. `nvs_types::expr::is_assignable` satisfies `object` with a
class instance and a shape and with nothing else, so a scalar, an array and a `mixed` are refused
where the call is written — `CoreTy::Mixed` would have moved all three into the body. The variant's
own doc is the home of that, and of why a parameter that is neither `string` nor `bytes` owes
`rule:security/unclassified-parameter-refuses-tainted` no classification: a `tainted {…}` claims
shape reaches it carrying the per-field qualifiers `rule:security/tainted-qualifier` distributed
onto it. Four `.nvst` cases pin the member, one of them freezing the `E0401` the spelling produces.

**Three helpers now have one home each**, because two members sign and neither may drift from the
other: `pair_at` (which kinds sign), `registered_pair` (the lifetime bound and the two claims) and
`pair_signature`. Each takes the member's name, so every frozen message is unchanged.

`Jwt::verify`, `Jwt\KeySet` and `Jwe` are untouched. The failing acceptance check
(`examples/webcrypto.nvs`) stays red until stage 7; nothing is blocked.

**A `[context]` gap, still open from the last session:** no field points at
`crates/nvs-stdlib/tests/vectors/webcrypto.json` or at `crates/nvs-stdlib/src/tests/vectors.rs`,
and `/jws/refusals` is the fixture `verifyIssued` is judged by. Add a `files` selector for both.

## Next group

**Stage 6: JWS — `verifyIssued<T>`** — one file set: `crates/nvs-stdlib/src/jwt.rs`,
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/registry.rs`, and for the first item
`crates/nvs-types/src/expr/args.rs`. The goal's § *Standing decisions* and ADR 0179 § 7 fix the
whole surface; `rule:security/derived-codec-qualifiers` and
`rule:security/verification-does-not-launder` are what `T` is held to.

- [ ] **Register `Jwt::verifyIssued<T>(string $token, Jwt\KeySet|Crypto\PublicKey $keys, string
      $issuer, string $audience, {leeway?, typ?, nonce?, maxAge?}): T` beside
      `crates/nvs-stdlib/src/jwt.rs:345`.** The written type parameter is `CoreMethod::written` plus
      a row in `crates/nvs-stdlib/src/registry.rs:2745`'s `WRITTEN_CLASS_MEMBERS` if the helper needs
      the class in slot 0, as `Core\Json::decodeAs` does; the call-site qualifier check is
      `crates/nvs-types/src/expr/args.rs:1518`. Decide there whether `T` reaches the helper the way
      `decodeAs` gets it or the way `Core\Request::jsonAs` does.
- [ ] **The body's first half, beside `crates/nvs-stdlib/src/jwt.rs:1180`.** Length cap, shape,
      header policy — refuse `jku`, `x5u`, `x5c`, `jwk`, `crit`, `b64`, `zip`, `cty` and ignore every
      other member — then the key, found by `kid` through `keys_in`
      (`crates/nvs-stdlib/src/jwt.rs:1542`) and never tried, then one signature check. Everything to
      here is `refused()`'s one sentence (`crates/nvs-stdlib/src/jwt.rs:630`).
- [ ] **The clock and the claims, reached only under a signature that held.** `exp` under `leeway`
      keeps its own message for `crates/nvs-stdlib/src/jwt.rs:89`'s reason; `iss`, `aud`/`azp`,
      `nonce` in constant time, `typ` case-insensitively, `maxAge` against `auth_time` — each naming
      the claim. Then `T` is hydrated through `crates/nvs-stdlib/src/json.rs:1404`'s `check_codec`
      and `hydrate`, which is the door `Core\Request::jsonAs` already uses.

## Backlog
- `Jwe::encrypt`/`decrypt` and `Jwe\Key` — stage 7, `docs/agent/loop-goal.md` § *Standing decisions*.
- `examples/webcrypto.nvs`, the acceptance fixture — stage 7, `docs/agent/loop-goal.md`.
- ADR 0179 does not yet record that `signObject`'s claims are `object`; the reasoning lives in
  `CoreTy::Object`'s doc and `crate::jwt`'s module doc, which is where a rule's fragment points.
- A `[context] files` selector for `crates/nvs-stdlib/tests/vectors/webcrypto.json` and
  `crates/nvs-stdlib/src/tests/vectors.rs` — `docs/agent/loop-goal.toml`.
