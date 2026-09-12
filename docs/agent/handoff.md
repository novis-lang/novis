# Handoff

## State

**Goal `webcrypto`, stage 6 is complete: `Core\Jwt::verifyIssued<T>` is on disk, registered and
reachable** (`crates/nvs-stdlib/src/jwt.rs:1702`). It checks in one order — shape, header policy,
key, signature, then the clock and the claims — and everything before the signature answers with
`not_issued()`'s one sentence (`crates/nvs-stdlib/src/jwt.rs:1494`), which is `verify`'s `refused()`
rewritten for this member's own set of ways to be unverifiable.

**A key is found, never tried** (`crates/nvs-stdlib/src/jwt.rs:1588`): `kid` is a lookup into the
set, a token naming none is answered only by a set of one, and a `Crypto\PublicKey` argument is that
same rule with the set of one written by the program. The payload is handed to
`crate::json::decode_as`, now `pub(crate)`, so what a token may say is what a JSON document may say.

**The one change outside `nvs-stdlib` landed too.** `verifyIssued` joined `jsonAs` on the decode-site
roster (`crates/nvs-types/src/expr/args.rs:1526`), and a `DecodeSite` now carries the member that
asked (`crates/nvs-types/src/derive.rs:785`) — the old message said "a decoded request body" for
whichever member recorded the site, which was about to be wrong for half of them. One reject case's
expected output moved with it.

`rule:security/protocol-roster`'s closing paragraph was stale on more than this member — it named
`Core\Jwe` and `Core\Signature` as having no member at all, and both are registered — so it now says
what is on disk. The failing acceptance check (`examples/webcrypto.nvs`) stays red until stage 7.

**A `[context]` gap, still open:** no field points at `crates/nvs-stdlib/tests/vectors/webcrypto.json`
or `crates/nvs-stdlib/src/tests/vectors.rs`, and the whole of the next group reads them. Add a
`files` selector for both.

## Next group

**Stage 7: replaying the frozen set's JWS half** — one file set:
`crates/nvs-stdlib/src/tests/vectors.rs`, `crates/nvs-stdlib/src/jwt.rs`, and
`crates/nvs-stdlib/tests/vectors/webcrypto.json` read-only. The goal's § *Stage 7* is the list; the
set is never edited by a session.

- [ ] **Split a crate-private entry point out of `crates/nvs-stdlib/src/jwt.rs:1702` that takes the
      clock as an argument.** The goal's § *Stage 7* asks for `verifyIssued` "at the crate-private
      level where the clock is an argument", and the helper reads it from `ctx` through
      `now_seconds` today, so `jws.vectors` and `jws.refusals` cannot be replayed at their recorded
      `clock` without one. Everything from the header policy down is already clock-free except the
      two window checks and `maxAge`.
- [ ] **Replay `jws.keys` and `jws.keySets` beside `crates/nvs-stdlib/src/tests/vectors.rs:42`.**
      Every key reads from `pkcs8`, `pem`, `spki` and `jwk`, the forms agree,
      `write(KeyFormat::Jwk)` answers `jwkMinimal`, `thumbprint` is base64url of its SHA-256
      (`crates/nvs-stdlib/src/crypto.rs:2010`), and `Jwt\KeySet::read` admits exactly `kids` or
      refuses the set whole.
- [ ] **Replay `jws.vectors` and `jws.refusals` through the new entry point.** Each payload's
      claims, the structured ones included; each refusal with its recorded kind — `policy` and
      `authenticity` the one sentence, `time` the expiry error, `claims` a refusal naming the claim,
      which is `claim_refused` at `crates/nvs-stdlib/src/jwt.rs:1507`.

## Backlog

- `jws.signs` and `signatures.*` replay — the goal's § *Stage 7*, after the JWS verify half.
- `examples/webcrypto.nvs`, the failing acceptance check — the goal's § *Stage 7*, last.
- `rule:security/protocol-roster` still carries the status *designed, not yet shipped* while every
  entry is registered with cases; whether that metadata moves is `docs/rules/security.json`'s.
- `MAX_TOKEN_LEN` is this member's alone; `Core\Jwe`'s protected-header cap is its own
  (`rule:security/jwe-compact-subset`).
