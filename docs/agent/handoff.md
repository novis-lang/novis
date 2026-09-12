# Handoff

## State

**Goal `webcrypto`, stage 7: every test name stage 7's `cargo-named` check lists now runs and is
green.** All fourteen at `docs/agent/loop-goal.toml:9145-9160` appear in `cargo test -p nvs-stdlib`'s
output, and `python tools/verify.py` is 11 of 11 green.

Two of them were not renames. `Core\Jwe::decrypt`'s verdict is now `plaintext`
(`crates/nvs-stdlib/src/jwe.rs:1134`), which leaves the member one `refused()` call site and gives
Rust a door onto the read direction: the set's payloads (`crates/nvs-stdlib/src/jwe.rs:1262`) and all
of its refusals (`:1473`) are now asserted through the member's own path, where before only the
`.nvst` side ever opened a token. The ECDH replay (`crates/nvs-stdlib/src/crypto.rs:4175`) gained the
every-key-form agreement its name claims and the goal's § *Stage 7* asks for — `raw`, `spki` and
`jwk` read and agreed through `agree`.

**`examples/webcrypto.nvs` is stage 7's last item**, and it is what the driver's acceptance sweep
dies on, so no `[[check]]` after it has run yet. Stage 8 — the rulebook — is behind it.

`nvs-host`'s two CPU-charging watchdog tests are the known flake (`crates/nvs-host/src/watchdog.rs:1051`
and `:1103`): one of them fails per run under load and passes alone, a different one each time. This
session's run was green, so a red one there is not evidence of a change in `nvs-stdlib`.

## Next group

**Stage 7: the example, which is the stage's last item** — one file set: `examples/webcrypto.nvs`,
new, with `docs/agent/loop-goal.toml:9162-9175`,
`tests/conformance/core/jwe-opens-every-token-webcrypto-sealed.nvst:1` and `examples/crypto.nvs:1`
read-only. `rule:testing/four-proofs`, and `docs/examples/README.md` owns what an example is.

- [ ] **Write `examples/webcrypto.nvs`.** The `exact` check freezes seven lines in order —
      `sealed`, `opened`, `tamper refused`, `jwe round trip`, `header swap refused`,
      `id token verified`, `algorithm swap refused` — at `docs/agent/loop-goal.toml:9166-9174`. The
      goal's § *Stage 7* says it opens one of the frozen set's own tokens and verifies one of its ID
      tokens, then refuses that same token with its `alg` swapped, so the example is itself a
      browser's output being read. `tests/conformance/core/jwe-opens-every-token-webcrypto-sealed.nvst:1`
      is where a frozen token is already inlined in Novis source to copy one from, and
      `examples/crypto.nvs:1` is the shape and comment register an example in this tree has. Nothing
      in `examples/` carries an `.out`, so there is nothing to bless — run it with
      `target/debug/nvs.exe examples/webcrypto.nvs` and read the seven lines back.
- [ ] **Then stage 8, the rulebook** — `rule:core-classes/crypto-interop-tier` still reads
      `designed` (`docs/agent/loop-goal.toml:9180`), which is a separate file set and its own group.

## Backlog

- The `structured` signing case as a `.nvst` case at a fixed clock — `docs/agent/loop-goal.md` §
  *Stage 7*, `jws.signs`; `object_payload_of` takes a Novis object, so no `#[test]` reaches it.
- `nvs-host`'s two CPU-charging watchdog tests need a measurement they own —
  `crates/nvs-host/src/watchdog.rs:1051` and `:1103`, outside this goal.
