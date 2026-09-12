# Handoff

## State

**Goal `webcrypto`: stage 8 landed, and stage 3's check is the only red one left.** The three rules
read `shipped` and each names the conformance cases that hold it, so stage 8's three `rules.py --show`
checks pass. `docs/novis.md` needed no regeneration — `tools/reference.py` builds it from the binary's
registry and the hand-written chapters and reads no rule status, and `--check` says it is current.

**Stage 3's failure is a naming failure, not unwritten work.** Its `tests` list
(`docs/agent/loop-goal.toml:9027`) was drafted before the tests were written and none of its fourteen
names resolves, while thirteen of the claims under them are asserted in
`crates/nvs-stdlib/src/crypto.rs`. The mapping, each confirmed against the named test's doc comment or
body:

| Drafted claim | Asserted at |
|---|---|
| AES-256-GCM, NIST vectors + flipped tag | `aes_gcm_matches_the_vectors_published_with_the_mode` `:3757` |
| PBKDF2 RFC 7914, HKDF RFC 5869 | `the_two_derivations_match_their_published_vectors` `:3899` |
| X25519 RFC 7748, P-256 ECDH | `webcrypto_ecdh_vectors_agree_the_same_secret_from_every_key_form` `:4175` |
| their two refusals | `webcrypto_primitive_refusal_vectors_are_each_refused_by_the_member_that_owns_the_rule` `:4080` |
| AES Key Wrap RFC 3394 | `the_key_wrap_matches_rfc_3394_and_refuses_a_changed_wrap` `:4297` |
| Concat KDF RFC 7518 App C | `the_derivation_ecdh_es_runs_matches_rfc_7518_appendix_c` `:4348` |
| RS256, PS256, ES256, EdDSA | `webcrypto_signature_vectors_verify_and_deterministic_ones_resign_byte_for_byte` `:4491` |
| the ES256 DER refusal | `webcrypto_signature_refusals_are_each_refused_with_one_runtime_error` `:4458` |
| JWK thumbprint RFC 7638 | `webcrypto_jws_keys_read_in_every_form_and_write_their_minimal_jwk_and_thumbprint` `:4676` |
| the RSA 2048–8192 bound | `the_rsa_width_bound_is_asserted_on_both_sides` `:4754` |

**The oracle moved, deliberately, for five of those rows.** P-256 ECDH and the four signature rows are
held to the frozen WebCrypto set rather than to the RFC appendices stage 3's prose table names, and
`crypto.rs:4158-4162` is where that swap is argued. It is the goal's own § *Standing decisions* — "the
interop proof is WebCrypto's own output, frozen" — so re-pointing the check is the safe reading; adding
the RFC appendix vectors on top would be a second, independent oracle and is backlog, not a blocker.

**One claim has no Rust test**: `every_interop_nonce_salt_and_private_key_is_drawn_through_core_random`
is pinned only by `.nvst` cases (`tests/conformance/core/crypto-every-seal-draws-its-own-nonce-and-the-body-moves-with-it.nvst`
and the two `crypto-generate-key*-draws-*` cases), and a `cargo-named` check cannot see those.

`nvs-host`'s two CPU-charging watchdog tests stay the known flake (`crates/nvs-host/src/watchdog.rs:1051`
and `:1103`): one fails per run under load, a different one each time, and each passes alone.

## Next group

**Stage 3: the primitives, re-pointed** — one file set: `docs/agent/loop-goal.toml`,
`docs/agent/goals/47-webcrypto.toml` and `crates/nvs-stdlib/src/crypto.rs`.
`rule:core-classes/crypto-interop-tier`.

- [ ] **Write the one missing assertion as a `#[test]`** — that every nonce, salt and drawn private key
      in the interop path comes through `crate::random::draw`, beside the other seam tests at
      `crates/nvs-stdlib/src/crypto.rs:3676`. `rsa_is_never_drawn` (`crates/nvs-stdlib/src/crypto.rs:2861`)
      and the `generate_key` doc at `crates/nvs-stdlib/src/crypto.rs:2998` are the seam it asserts over.
- [ ] **Re-point stage 3's `tests` list to the names above** at `docs/agent/loop-goal.toml:9029`,
      keeping one entry per test rather than one per drafted claim, and say in the block's comment that
      five rows are held to the frozen set rather than to their RFC appendix.
- [ ] **Copy the same block into the goal file** at `docs/agent/goals/47-webcrypto.toml:9005` — the two
      are not byte-identical (the live copy carries closed goals' floor), so this is the same edit
      applied twice, and the next `goal-switch.py` restores the goal file over the live one.

## Backlog

- The RFC 7515 A.2 / 7520 § 4.2 / 8037 A.4 signature vectors as a second oracle beside the frozen set;
  no home yet, and `docs/agent/loop-goal.md` § *Stage 3* is where the table asking for them is.
- The Rust-side half of the tainted-token claim has no home: nothing under `crates/nvs-types/tests/`
  names a `Core` member, and `core_lib.rs`'s per-slot audit (`crates/nvs-types/src/core_lib.rs:938`)
  already pins the row's mark.
- JWE-encrypted ID tokens, key-set fetching and caching, and structured claims under a shared key are
  out of this goal by its § *Standing decisions*.
- `crates/nvs-host/src/watchdog.rs:1051` and `:1103` flake under load; nothing owns the repair.
