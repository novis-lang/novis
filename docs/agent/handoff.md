# Handoff

## State

Milestone `dossier`, goal `core-encoding`. Ten of the eleven members in `docs/agent/loop-goal.md`
carry their feature proofs: `decodeText`, `encodeText`, `fromBase32`, `fromBase64`, `fromBase64Url`,
`fromHex`, `toBase32`, `toBase64`, `toBase64Url` and `toHex`, each with `about.md`, three examples,
one attack, one bench and a Rust `#[test]` carrying its `covers:` marker. `isValidText` is the last
member left.

`toBase32` proved clean: every example, the attack and the bench passed at their first run. It writes
RFC 4648 § 6 **unpadded**, as its reference card says; the earlier handoff line calling it padded was
wrong, and nothing in the tree said so.

No perf figure is recorded yet, on purpose. Every session in this goal edits
`crates/nvs-stdlib/src/encoding.rs`, and `rule:testing/member-perf-ledger` restales every
`Core\Encoding` figure the moment that file's text moves, so the eleven measurements are taken in
one release run at the goal's end. Nothing is blocked.

## Next group

**Feature proofs for the last member, then the perf figures — one file set:**
`crates/nvs-stdlib/src/encoding.rs` (the registry row and the `mod tests` block), plus the proof
trees under `core/Encoding/isValidText/`. `rule:testing/feature-proofs` says what the item owes and
`rule:testing/a-failing-proof-is-fixed-or-recorded` says what a finding does.

- [ ] **`Core\Encoding::isValidText`** — `about.md`, three examples, one attack, one bench, a Rust
      `#[test]` carrying `// covers:`. It answers a `bool` rather than throwing, so the attack is
      what a caller does with a `false`, not a refusal it can catch; its bench declares
      `// bench: allocations 0` if the member allocates nothing.
      `crates/nvs-stdlib/src/encoding.rs:360`
- [ ] **The eleven perf figures, in one release run once `isValidText` lands** — `python
      tools/dossier.py --record-perf --id 'Core\Encoding::<member>'` per member. This is what the
      driver's acceptance check has been red on all goal long, and it clears only after the last edit
      to `crates/nvs-stdlib/src/encoding.rs:360`

## Backlog

- The `[context] modules` manifest prints only `nvs-stdlib/src/encoding.rs`; a proof reaching
  `Core\Bytes`, `Core\Str` or `Core\Random` for a fixture reads their signatures from
  `nvs meta --json` (written to a file under `.agent-tmp/`; a PowerShell pipe adds a BOM).
- Signedness across `Core` differs by member and the diagnostic is the only notice: `Core\Bytes::fill`
  takes `uint` and `Core\Str::slice` takes `int`, so a `while` counter driving both needs a cast.
  Owned by each member's registry row.
