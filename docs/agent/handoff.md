# Handoff

## State

**Goal `webcrypto`: stage 5 (`Core\Jwe`) is complete** — the check's ten case paths all exist and pass,
and stage 4 is still green. Three cases were written this session and four existing ones were paired
into the check's list by their `--TEST--` lines rather than renamed on disk.

What the three new ones pin, each a claim the seven cases already on disk did not hold: the protected
header's allow-list, asserted against tokens WebCrypto sealed *with* the header they are refused for —
so the tag holds over it and only the list can refuse them, with a `kid` token as the control that
opens; PBES2's `p2c` read before any derivation, asserted by the case finishing at all under counts of
4,294,967,295 and one that does not fit the counter, and by agreeing with the bounds
`Core\Crypto::deriveKey` holds its own arguments to; and an `epk` read through
`Core\Crypto\PublicKey::read` on the curve *this program's* key is on, with the same two JWKs handed to
that member answering the same way.

`python tools/verify.py` was collected green at this commit. The three load-dependent tests earlier
sessions saw fail in a full run — `crates/nvs-host/src/watchdog.rs:1051` and `:1103`, and
`crates/nvs-server/src/serve.rs:7958` — are timing, not this goal, which touches neither crate.

## Next group

**Stage 6: JWS — `Core\Jwt` over a key pair, `Jwt\KeySet`, and claims as a shape, whose check is in
exactly the state stage 5's was** — one file set: `tests/conformance/core/jwt-*.nvst`,
`crates/nvs-stdlib/src/jwt.rs`, and the two goal files. The members are landed and nineteen `jwt-`
cases are on disk, while the check names ten paths and none of them exists, so the first item is the
pairing and the rest are whatever it leaves unheld. `rule:security/algorithm-comes-from-the-key`,
`rule:security/verification-does-not-launder` and the goal's § *Standing decisions* for the JWS subset.

- [ ] **Pair the stage 6 check's ten paths against the nineteen `jwt-` cases on disk**, by `--TEST--`
      line, and rewrite the list in place the way stage 5's was — the list is
      `docs/agent/loop-goal.toml:9145` and its copy `docs/agent/goals/47-webcrypto.toml:9121`, and the
      cases on disk are `tests/conformance/core/jwt-*.nvst`.
- [ ] **`embedKey`'s proof of possession**, if the pairing leaves it unheld: the pair's public half
      written as RFC 7638's minimal JWK under `jwk`, and a `LogicError` under a shared key —
      `crates/nvs-stdlib/src/jwt.rs:428`.
- [ ] **The `verifyIssued` claims the pairing leaves unheld** — the header a verifier may not honour,
      the clock under a bounded leeway, and `azp` where `aud` is a list of more than one —
      `crates/nvs-stdlib/src/jwt.rs:479`.
- [ ] **`Jwt\KeySet::read`'s `rsaScheme`**, which reaches only an RSA key carrying no `alg` and loses
      to a key's own — `crates/nvs-stdlib/src/jwt.rs:2336`.

## Backlog

- No token in the frozen set carries `typ` or `cty`, so two of the eight allowed JWE header members
  are refused-by-nothing rather than accepted-by-a-case; regenerating the set is the user's call
  (goal § *Standing decisions*, `tools/webcrypto-vectors.mjs`).
- Nothing pins `MAX_HEADER`'s 4096-octet cap on a protected header — `crates/nvs-stdlib/src/jwe.rs:137`.
- Stage 7 is WebCrypto both directions and the example; stage 8 onward is untouched
  (`docs/agent/goals/47-webcrypto.md`).
