# Handoff

## State

**Goal 47 — a browser and a Novis server read each other's encrypted data — has just started; nothing of it has landed yet.** Goal `template-format`'s whole list is this goal's Stage 1 floor.

**The design is settled with the user and is written into the goal's § *Standing decisions*; the record
that states it is not written yet.** Stage 2 writes it — one new record, the goal names no number — and
it transcribes those decisions rather than re-deriving them. The one thing not to re-decide: **one
streamlined API** — `generateKey`, `seal` and `open` take a `Cipher` enum defaulting to
XChaCha20-Poly1305, and a member exists per job, never per algorithm. Existing signatures may change
(nothing has shipped publicly); XChaCha as the default and every `Core\Digest` case stay.

## Next group

**Stage 2: the record** — one file set: `docs/decisions/`, `docs/rules/security*`,
`docs/rules/core-classes*`, `docs/spec/`.

- [ ] **The record** — the next free number in `docs/decisions/`, `changes.creates`
      `core-classes/crypto-interop-tier` and `security/jwe-compact-subset`, `changes.modifies`
      `security/protocol-roster`. Its body is the goal's standing decisions, argued, and it says which of
      the two fallbacks held (enum default, `secret` union) once the checker has been asked.
- [ ] **The two rule fragments and their JSON entries**, both `designed`, then `python tools/rules.py
      --render`.
- [ ] **`security/protocol-roster`** — `docs/rules/security/protocol-roster.md:1-3` names five
      protocols; JWE is the sixth. `:14-16` says `Core\Signature` is not on disk, which is already wrong.
      `docs/spec/01-core-library.md:1175` says "closed five-entry roster".
- [ ] **The spec rows** — `docs/spec/01-core-library.md:1173` (`Core\Crypto`) becomes the streamlined
      surface and a `Core\Jwe` row joins § 16; `docs/spec/02-php-migration.md:811-812` stop pointing at a
      member that does not exist. Do not add a `member` row the registry cannot answer yet —
      `every_migration_member_row_names_a_registered_member` would fail the build. Those two rows flip in
      stage 4.

## Backlog

- Stage 3 — the dependencies and the primitives under their published vectors. `Cargo.toml`,
  `crates/nvs-stdlib/Cargo.toml`, `crates/nvs-stdlib/src/crypto.rs`, `THIRD-PARTY-LICENSES.txt`. One
  session.
- Stage 4 — the `Core\Crypto` surface and the key classes. `crypto.rs`, `registry.rs`, `docs/spec/`,
  `tests/conformance/core/crypto-*`, and `examples/crypto.nvs` if `Cipher` had to become required.
  Shares `crypto.rs` with stage 3.
- Stage 5 — `Core\Jwe`. New `crates/nvs-stdlib/src/jwe.rs`, `registry.rs`,
  `tests/conformance/core/jwe-*`. Its own session.
- Stage 6 — the WebCrypto round trip and the example. `tools/webcrypto-interop.mjs` (new),
  `examples/webcrypto.nvs` (new). Cheap.
- Stage 7 — the flips. `docs/rules/` only.
- When this goal's last check goes green the driver takes goal `gap-zero`.
