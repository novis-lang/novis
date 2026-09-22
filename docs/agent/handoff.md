# Handoff

## State

Milestone `dossier`, goal `core-encoding`. Three of the eleven members in
`docs/agent/loop-goal.md` now carry their feature proofs: `decodeText`, `encodeText` and
`fromBase32`, each with `about.md`, three examples, one attack, one bench and a Rust `#[test]`
carrying its `covers:` marker.

A proof found one defect and it is fixed: base32's truncation message said the text *ends* at the
offset it names, but `data_encoding` reports where the short group **starts**, so the sentence was
false of every text whose truncated group is not its first.
`tests/conformance/core/encoding-each-decoder-says-why-it-refused-and-where.nvst` pins the
corrected wording at two offsets.

No perf figure is recorded yet, on purpose. Every session in this goal edits
`crates/nvs-stdlib/src/encoding.rs`, and `rule:testing/member-perf-ledger` restales every
`Core\Encoding` figure the moment that file's text moves, so eleven measurements taken now would be
thrown away eight times. Take them all in one release run at the goal's end. Nothing is blocked.

## Next group

**Feature proofs for the three decoders — one file set:** `crates/nvs-stdlib/src/encoding.rs` (the
registry rows, the `why_not_base64` message helper and the `mod tests` block), plus the four proof
trees under `core/Encoding/<member>/`. `rule:testing/feature-proofs` says what each item owes and
`rule:testing/a-failing-proof-is-fixed-or-recorded` says what a finding does. One slice is one
member with all five proofs together.

- [ ] **`Core\Encoding::fromBase64`** — `about.md`, three examples, one attack, one bench, a Rust
      `#[test]` carrying `// covers:`. `crates/nvs-stdlib/src/encoding.rs:377`
- [ ] **`Core\Encoding::fromBase64Url`** — the same five. It shares `why_not_base64` and its padding
      rule with the member above, so take it second. `crates/nvs-stdlib/src/encoding.rs:395`
- [ ] **`Core\Encoding::fromHex`** — the same five. Its refusals are the two in `why_not_hex`.
      `crates/nvs-stdlib/src/encoding.rs:431`

The shape to copy is `one_value_has_one_base32_spelling` at
`crates/nvs-stdlib/src/encoding.rs:1472`: it drives the member through `nvs_runtime::call` the way a
program reaches it, rather than the private conversion under it, and releases every `Value` it
builds. A decoder's strongest attack is **malleability** — two texts that decode to one value — and
`tests/hostile/core/Encoding/fromBase32/01-two-spellings-of-one-secret.nvs` is that attack written
out.

## Backlog

- The remaining five members of this goal: `isValidText`, `toBase32`, `toBase64`, `toBase64Url`,
  `toHex` — `docs/agent/loop-goal.md` § *The item list*.
- Record every `Core\Encoding` bench in one release run at the goal's end — `docs/perf/members.ndjson`.
- The `decodeText` attack reaches 8 MB and never the memory ceiling, because an `ends-early` step
  has to be a file's last — `tests/hostile/README.md`.
