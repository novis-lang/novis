# Handoff

## State

Milestone `dossier`, goal `core-encoding`. Nine of the eleven members in `docs/agent/loop-goal.md`
carry their feature proofs: `decodeText`, `encodeText`, `fromBase32`, `fromBase64`, `fromBase64Url`,
`fromHex`, `toBase64`, `toBase64Url` and `toHex`, each with `about.md`, three examples, one attack,
one bench and a Rust `#[test]` carrying its `covers:` marker. `toBase32` and `isValidText` are the
work left.

The three encoders proved clean: every example, attack and bench passed at its first run, and no
finding went to `rule:testing/a-failing-proof-is-fixed-or-recorded`.

No perf figure is recorded yet, on purpose. Every session in this goal edits
`crates/nvs-stdlib/src/encoding.rs`, and `rule:testing/member-perf-ledger` restales every
`Core\Encoding` figure the moment that file's text moves, so the eleven measurements are taken in
one release run at the goal's end. Nothing is blocked.

## Next group

**Feature proofs for the last two members — one file set:** `crates/nvs-stdlib/src/encoding.rs` (the
registry rows and the `mod tests` block), plus the two proof trees under `core/Encoding/<member>/`.
`rule:testing/feature-proofs` says what each item owes and
`rule:testing/a-failing-proof-is-fixed-or-recorded` says what a finding does. One slice is one
member with all five proofs together, and `toBase32`'s decoder already has an `about.md` and three
examples to point at rather than repeat.

- [ ] **`Core\Encoding::toBase32`** — `about.md`, three examples, one attack, one bench, a Rust
      `#[test]` carrying `// covers:`. It writes RFC 4648 § 6's alphabet padded to eight
      characters, so the five residues a group can end on are what the sweep is written around.
      `crates/nvs-stdlib/src/encoding.rs:405`
- [ ] **`Core\Encoding::isValidText`** — the same five. It answers a `bool` rather than throwing,
      so the attack is what a caller does with a `false`, not a refusal it can catch.
      `crates/nvs-stdlib/src/encoding.rs:360`
- [ ] **The eleven perf figures, in one release run once both land** — `python tools/dossier.py
      --record-perf --id 'Core\Encoding::<member>'` per member. This is what the driver's
      acceptance check has been red on all goal long, and it clears only after the last edit to
      `crates/nvs-stdlib/src/encoding.rs:405`

## Backlog

- The `[context] modules` manifest prints only `nvs-stdlib/src/encoding.rs`; a proof reaching
  `Core\Bytes` or `Core\Json` for a fixture reads their registry rows with `peek.py` instead.
- Signedness across `Core` differs by member and the diagnostic is the only notice: `Core\Bytes::fill`
  takes `uint` and `Core\Str::slice` takes `int`, so a `while` counter driving both needs a cast.
  Owned by each member's registry row.
