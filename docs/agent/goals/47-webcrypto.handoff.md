# Handoff

## State

**Goal 47 — a browser and a Novis server read each other's encrypted data — has just started; nothing of it has landed yet.** Goal `template-format`'s whole list is this goal's Stage 1 floor.

**The design is settled with the user and is written into the goal's § *Standing decisions*; the record
that states it is not written yet.** Stage 2 writes it — one new record, the goal names no number — and
it transcribes those decisions rather than re-deriving them. The one thing not to re-decide: **this is
additive.** `Core\Crypto::generateKey`/`seal`/`open` stay XChaCha20-Poly1305, byte for byte, and every
digest `Core\Hash` has stays; the interop tier sits beside them.

## Next group

**Stage 2: the record** — one file set: `docs/decisions/`, `docs/rules/security*`,
`docs/rules/core-classes*`, `docs/spec/`.

- [ ] **The record** — the next free number in `docs/decisions/`, `changes.creates`
      `core-classes/crypto-interop-tier` and `security/jwe-compact-subset`, `changes.modifies`
      `security/protocol-roster`. Its body is the goal's standing decisions, argued.
- [ ] **The two rule fragments and their JSON entries**, both `designed`, then `python tools/rules.py
      --render`.
- [ ] **`security/protocol-roster`** — `docs/rules/security/protocol-roster.md:1-3` names five
      protocols; JWE is the sixth. `docs/spec/01-core-library.md:1175` says "closed five-entry roster".
- [ ] **The spec rows** — `docs/spec/01-core-library.md:1173` (`Core\Crypto`) gains the interop tier and
      a `Core\Jwe` row joins § 16; `docs/spec/02-php-migration.md:811-812` stop pointing at a member that
      does not exist. Leave the rows' member spellings to the record, and do not add a `member` row the
      registry cannot answer yet — `every_migration_member_row_names_a_registered_member` would fail the
      build. Those two rows flip in stage 4.

## Backlog

- Stage 3 — the dependencies and the primitives under their published vectors. `Cargo.toml`,
  `crates/nvs-stdlib/Cargo.toml`, `crates/nvs-stdlib/src/crypto.rs`, `THIRD-PARTY-LICENSES.txt`. One
  session.
- Stage 4 — the `Core\Crypto` members. `crypto.rs`, `registry.rs`, `docs/spec/`,
  `tests/conformance/core/crypto-*`. Shares `crypto.rs` with stage 3; take them together if stage 3
  came in small.
- Stage 5 — `Core\Jwe`. New `crates/nvs-stdlib/src/jwe.rs`, `registry.rs`,
  `tests/conformance/core/jwe-*`. Its own session.
- Stage 6 — the WebCrypto round trip and the example. `tools/webcrypto-interop.mjs` (new),
  `examples/webcrypto.nvs` (new). Cheap.
- Stage 7 — the flips. `docs/rules/` only.
- When this goal's last check goes green the driver takes goal `gap-zero`.
