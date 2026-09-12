# Handoff

## State

**Goal `webcrypto`, stage 7: the sign direction replays.** `sign`'s clock-taking half is
`signed_token` over a `Signing` (`crates/nvs-stdlib/src/jwt.rs:1277` and `:1313`); the member keeps the
argument reads and `registered_pair` (`crates/nvs-stdlib/src/jwt.rs:1389`). The set's two deterministic
text-claims cases come back as WebCrypto's own tokens octet for octet
(`crates/nvs-stdlib/src/jwt.rs:2940`), and the ES256 DPoP proof, whose `deterministic` is false, matches
both encoded segments — the embedded JWK included — and verifies under the half its own pair derives
(`crates/nvs-stdlib/src/jwt.rs:2998`). `webcrypto::signs` (`crates/nvs-stdlib/src/tests/vectors.rs:54`)
is the door onto the section.

**The set's fourth signing case is not reachable from Rust.** Its claims are `structured`, and
`object_payload_of` (`crates/nvs-stdlib/src/jwt.rs:1204`) takes a Novis object, which only a program
builds — so that one is a `.nvst` case at a fixed clock rather than a `#[test]`. The byte-for-byte test
skips it out loud.

**Stage 7's `cargo-named` check names its tests verbatim and none of the landed ones did.** The four in
`jwt.rs` now carry the names `docs/agent/loop-goal.toml:9145-9160` lists; `crypto.rs`'s and `jwe.rs`'s
still do not, which is the next group. The trap itself is already `playbook.md`'s, twice.

Still red, and expected: `examples/webcrypto.nvs`, which the goal's § *Stage 7* puts last. It is also
what the driver's sweep dies on, so no `[[check]]` after it has run yet.

**`verify.py`'s `test` step is red on a crate this work does not touch**: one of `nvs-host`'s two
CPU-charging watchdog tests fails per run and passes alone, a different one each time. `nvs-stdlib`'s
own binary is green — the five `webcrypto_jws_*` tests included — and `cargo clippy -p nvs-stdlib
--all-targets` is clean. The playbook bullet is what stops the next session hunting its own change in
it.

## Next group

**Stage 7: the names stage 7's `cargo-named` check lists** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, with `crates/nvs-stdlib/src/jwe.rs` and
`docs/agent/loop-goal.toml:9145-9160` read-only. Every assertion below is green on disk already under
its own spelling; what is missing is the name, and `tools/loop.py:2434` matches it as a substring of the
run's output. `rule:testing/four-proofs`.

- [ ] **Rename the AES-GCM, ECDH, signature and JWE replays to the check's names.**
      `crates/nvs-stdlib/src/crypto.rs:3863` → `webcrypto_aes_gcm_vectors_open_and_reseal_byte_for_byte`,
      `:4105` → `webcrypto_ecdh_vectors_agree_the_same_secret_from_every_key_form` (with `:4616`, which
      holds the every-form half), `:4401` and `:4440` →
      `webcrypto_signature_vectors_verify_and_deterministic_ones_resign_byte_for_byte`, `:4415` →
      `webcrypto_signature_refusals_are_each_refused_with_one_runtime_error`, and
      `crates/nvs-stdlib/src/jwe.rs:1206`, `:1306`, `:1379` → the three `webcrypto_jwe_*` names. Two
      tests cannot share one required name, so where a name covers two of them, fold them or give it to
      the one it describes — `playbook.md`'s bullet at *a `cargo-named` check matches its name as a
      substring* is the rule.
- [ ] **Fold the three key-derivation replays into
      `webcrypto_pbkdf2_hkdf_and_aes_kw_vectors_derive_the_same_keys`** —
      `crates/nvs-stdlib/src/crypto.rs:4014` (PBKDF2), `:4065` (HKDF) and `:4226` (AES-KW) are the three
      halves, each green. One name for three sections is the check's own wording, so the fold is the
      design call it asks for rather than a twin beside them.
- [ ] **Give the refusal replays
      `webcrypto_primitive_refusal_vectors_are_each_refused_by_the_member_that_owns_the_rule`** —
      `crates/nvs-stdlib/src/crypto.rs:3896` (AES-GCM), `:3964` (PBKDF2's bounds) and `:4192`/`:4838`
      (a point off the curve, a low-order point) are what the name covers.

## Backlog

- `examples/webcrypto.nvs`, stage 7's last item — the seven lines `docs/agent/loop-goal.toml:9166-9174`
  freezes, one of them opening one of the set's own tokens.
- The `structured` signing case as a `.nvst` case at a fixed clock — `docs/agent/loop-goal.md` § *Stage
  7*, `jws.signs`.
- Stage 8, the rulebook: `rule:core-classes/crypto-interop-tier` still reads `designed`
  (`docs/agent/loop-goal.toml:9180`).
- `nvs-host`'s two CPU-charging watchdog tests need a measurement they own —
  `crates/nvs-host/src/watchdog.rs:1051` and `:1103`, outside this goal.
