# Handoff

## State

**Goal `webcrypto`: stage 4's member surface is complete and `tests/conformance/` is green** — 1827
passed, 0 failed, 0 skipped at this commit. The three cases this session wrote are the last of it: the
`secret` qualifier travelling with what `deriveKey`, `expandKey` and `agree` answer, `agree`'s refusal
of a peer's point on both curves — at `read` for P-256 and at `agree` for X25519 — and a PKCS#8 round
trip asserted by the shared secret rather than by the public half, which a PKCS#8 can carry.

**The stage 4 check's other eight paths were drafted names, not missing work**, and both
`docs/agent/loop-goal.toml` and `docs/agent/goals/47-webcrypto.toml` now name the cases the tree
carries. The playbook bullet under *Tooling* is how to spot the next one; nothing was renamed on disk.

`python tools/verify.py` is 11 of 11 green here, the three load-dependent tests earlier sessions saw
fail in a full run included — `nvs-host`'s two CPU-charging watchdog ones
(`crates/nvs-host/src/watchdog.rs:1051` and `:1103`) and `nvs-server`'s
`the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle`
(`crates/nvs-server/src/serve.rs:7958`). They are timing, not this goal, which touches neither crate.

## Next group

**Stage 5: `Core\Jwe`, whose check is in the same state stage 4's was** — one file set:
`tests/conformance/core/jwe-*.nvst`, `crates/nvs-stdlib/src/jwe.rs`, and the two goal files. Seven
`jwe-` cases are on disk and the check names six paths, none of which exists, so the first item is the
pairing and the rest are the claims nothing holds. `rule:security/protocol-roster`'s JWE entry and
`rule:security/algorithm-comes-from-the-key`, plus the goal's § *Standing decisions* for the subset.

- [ ] **Pair the stage 5 check's six paths against the seven cases on disk**, by `--TEST--` line, and
      rewrite in place the ones the tree already holds — the round trip under every key kind and the
      tainted payload are `jwe-round-trips-under-every-key-and-answers-a-tainted-payload.nvst`. The
      list is `docs/agent/loop-goal.toml:9114` and its copy `docs/agent/goals/47-webcrypto.toml:9090`;
      the members are `crates/nvs-stdlib/src/jwe.rs:175` and `:186`.
- [ ] **`jwe-refuses-zip-crit-and-every-unknown-header-with-one-sentence.nvst`** — the allowed header
      members are the goal's list and everything else is one `RuntimeError`, read at
      `crates/nvs-stdlib/src/jwe.rs:939`.
- [ ] **`jwe-pbes2-refuses-an-iteration-count-past-its-ceiling-before-deriving.nvst`** — `p2c` is
      attacker-supplied and bounded at both ends before the first HMAC, at
      `crates/nvs-stdlib/src/jwe.rs:939`.
- [ ] **`jwe-ecdh-es-refuses-an-ephemeral-key-off-the-curve.nvst`** — the `epk` a token carries is read
      through the same validation a peer's key gets, at `crates/nvs-stdlib/src/jwe.rs:939`.

## Backlog

- Stage 6 (JWS) is the same shape again: ten drafted `jwt-` paths, and `tests/conformance/core/jwt-*`
  already holds some of the claims — pair before writing (`docs/agent/loop-goal.toml`'s stage 6 check).
- Stage 7 is `-p nvs-stdlib` vector tests plus `examples/webcrypto.nvs`, which does not exist yet.
- JWE-encrypted ID tokens, JWK export of private keys and a key-set fetcher stay out of this goal
  (the goal's § *Not this goal*).
