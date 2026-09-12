# Handoff

## State

**Goal `webcrypto`: stage 3 carries a second and independent oracle beside the frozen WebCrypto set.**
P-256 agreement is held to RFC 5903 § 8.1, RS256 and ES256 to RFC 7515 Appendices A.2 and A.3, and
Ed25519 to RFC 8037 Appendix A.4 — `crates/nvs-stdlib/src/crypto.rs:4382`, `:4662` and `:4736`, each
named by stage 3's check in both `docs/agent/loop-goal.toml` and `docs/agent/goals/47-webcrypto.toml`.
Every row is additive: no vector file moved and no existing test changed.

None of the three needed a vector file because each is self-checking — the appendices' own literals
are in the test, and a digit wrong in a scalar, a key or a signature cannot agree or verify. The
playbook bullet under *Writing a test case* is where that, and the fetch that cannot reach a late
appendix, are written down.

**Stage 4 is what is open, and it is `.nvst` cases rather than any Rust.** Every member its check names
is registered and implemented (`crates/nvs-stdlib/src/crypto.rs:698` onward for the derivations,
agreement, signing and the two key classes), and the six cases that predate the cipher argument still
hold. Ten of the check's cases are not written yet; the group below is the first four of them, and the
driver's failing check is the first.

Three tests outside this goal fail under a full `verify.py` and pass alone, which `verify.py` itself
says of each: `nvs-host`'s two CPU-charging watchdog ones (`crates/nvs-host/src/watchdog.rs:1051` and
`:1103`) and now `nvs-server`'s `the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle`
(`crates/nvs-server/src/serve.rs:7958`). Nothing in this goal touches either crate.

## Next group

**Stage 4: the members' own conformance cases** — one file set: `tests/conformance/core/` and the member
bodies in `crates/nvs-stdlib/src/crypto.rs`. `rule:core-classes/crypto-interop-tier`, and the goal's
§ *Standing decisions* for what each member answers. The six cases already on disk are the shape to
follow; each item below is a case the stage 4 check names by path and nothing else is missing for it.

- [ ] **`crypto-derived-and-agreed-keys-are-secret-bytes.nvst`** — `deriveKey`, `expandKey` and `agree`
      each answer `secret bytes`, so a case proves the qualifier travels rather than just the octets:
      the members are at `crates/nvs-stdlib/src/crypto.rs:698`, `:719` and `:741`.
- [ ] **`crypto-agree-refuses-a-public-key-that-is-not-on-its-curve.nvst`** — the refusal is at the read
      for P-256 and at the all-zero secret for X25519, which is two members in one case;
      `crates/nvs-stdlib/src/crypto.rs:741` is `agree`'s row and `:1111` is `PublicKey::read`'s.
- [ ] **`crypto-public-keys-read-raw-spki-and-jwk-and-write-them-back.nvst`** — three forms in and the
      same three out, at `crates/nvs-stdlib/src/crypto.rs:1111` and the `write` row at `:1130`.
- [ ] **`crypto-a-key-pair-written-as-pkcs8-reads-back-and-agrees-the-same-secret.nvst`** — a pair
      round-trips through `write`/`read` and still agrees, at `crates/nvs-stdlib/src/crypto.rs:1258`.

## Backlog

- Six more stage 4 cases after the group above: the RSA and Ed25519 key kinds, PEM, `generateKeyPair`'s
  RSA refusal, `agree`'s refusal of a signing pair, and the two raw-signature cases — all listed by path
  in `docs/agent/loop-goal.toml`'s stage 4 check.
- Stage 4's second check wants `every_migration_member_row_has_a_conformance_case` green, which the ten
  cases above are what satisfies (`crates/nvs-stdlib` tests).
- JWE-encrypted ID tokens, JWK export of private keys and a key-set fetcher stay out of this goal
  (the goal's § *Not this goal*).
