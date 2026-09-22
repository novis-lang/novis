# Handoff

## State

Milestone `dossier`, goal `core-encoding`. Six of the eleven members in `docs/agent/loop-goal.md`
carry their feature proofs: `decodeText`, `encodeText`, `fromBase32`, `fromBase64`, `fromBase64Url`
and `fromHex`, each with `about.md`, three examples, one attack, one bench and a Rust `#[test]`
carrying its `covers:` marker. The four remaining encoders and `isValidText` are the work left.

A proof found one defect and it is fixed: base64's truncation message counted `1 symbols` for a
single leftover symbol. `why_not_base64` now writes the singular, and
`tests/conformance/core/encoding-each-decoder-says-why-it-refused-and-where.nvst` pins it beside
the plural row the URL-safe form already had.

No perf figure is recorded yet, on purpose. Every session in this goal edits
`crates/nvs-stdlib/src/encoding.rs`, and `rule:testing/member-perf-ledger` restales every
`Core\Encoding` figure the moment that file's text moves, so the eleven measurements are taken in
one release run at the goal's end. Nothing is blocked.

## Next group

**Feature proofs for the four encoders — one file set:** `crates/nvs-stdlib/src/encoding.rs` (the
registry rows and the `mod tests` block), plus the four proof trees under `core/Encoding/<member>/`.
`rule:testing/feature-proofs` says what each item owes and
`rule:testing/a-failing-proof-is-fixed-or-recorded` says what a finding does. One slice is one
member with all five proofs together, and each encoder's decoder already has an `about.md` and
three examples to point at rather than repeat.

- [ ] **`Core\Encoding::toHex`** — `about.md`, three examples, one attack, one bench, a Rust
      `#[test]` carrying `// covers:`. `crates/nvs-stdlib/src/encoding.rs:422`
- [ ] **`Core\Encoding::toBase64`** — the same five. It writes the alphabet `fromBase64Url`
      refuses. `crates/nvs-stdlib/src/encoding.rs:368`
- [ ] **`Core\Encoding::toBase64Url`** — the same five. It writes no padding at all.
      `crates/nvs-stdlib/src/encoding.rs:386`
- [ ] **`Core\Encoding::toBase32`** — the same five. `crates/nvs-stdlib/src/encoding.rs:404`

## Backlog

- `Core\Encoding::isValidText` is the goal's eleventh member and the odd one out: it answers a
  bool rather than a buffer, so its attack is the part to think about. `docs/agent/loop-goal.md`
- Every `Core\Encoding` perf figure is unmeasured on purpose until the goal's last slice lands.
  `rule:testing/member-perf-ledger`
- An encoder cannot fail on its input, so what its attack asserts is a size rather than a refusal.
  `tests/hostile/README.md`
