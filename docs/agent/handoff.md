# Handoff

## State

Goal `core-cli-and-2-more`, 15 of its 18 features carry their feature proofs. This session landed
`Core\Cli::secret`, `Core\Cli::live` and `Core\Cli::progress`, each with `about.md`, three examples
with blessed `.out` files, one attack, one bench and a Rust test marked `covers:`. `python
tools/verify.py` is green whole at this tree.

Three features are left, and **their trees are not under `core/Cli/`**: `Core\Cli\Color::index` and
`Core\Cli\Color::rgb` live at `core/Cli-Color/<member>`, `Core\Cli\Live::set` at `core/Cli-Live/set`.

Every `Core\Cli` feature reads `perf stale` from `python tools/dossier.py --id`, because the ledger
keys on the implementing file's text (`rule:testing/member-perf-ledger`) and every session of this
goal moves `crates/nvs-stdlib/src/cli.rs`. One `python tools/dossier.py --record-perf --group
'Core\Cli'` after the last three land clears all of them, and it is the only thing between this goal
and its gate.

Three facts this group cost time to establish. **A closure with statements is `fn(...) => { ... }`**;
`function (...) { }` is `E0222`. **A `secret tainted string` reaches almost nothing**: `Core\Str::length`
refuses it, `Core\Secret::reveal($value, $reason)` answers a `tainted string`,
`Core\Password::hash`/`verify` take it as it is, and `==` compares two of them. **A live region of a
million `Core\Cli\Text` rows reaches the memory ceiling**, so that step goes last under
`// hostile: ends-early`.

## Next group

**Stage: the goal's item list, in file order** — one file set: `crates/nvs-stdlib/src/cli.rs` for
the registry row, the body and the Rust test, plus `docs/examples/core/Cli-Color/<member>/`,
`docs/examples/core/Cli-Live/set/` and the matching `tests/hostile/` and `benches/members/` paths.
One slice is one feature with all its feature proofs — `rule:testing/feature-proofs`.

- [ ] **`Core\Cli\Color::index`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:2369`. A colour by its number in the 256-colour table, so an
      example is deterministic with no terminal at all.
- [ ] **`Core\Cli\Color::rgb`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:2378`. The sibling of the row above; the attack belongs on the
      boundaries of a channel.
- [ ] **`Core\Cli\Live::set`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:782`. It already has one conformance case and owes a second,
      and `tests/hostile/core/Cli/live/01-a-region-that-will-not-give-the-terminal-back.nvs` is
      where its refusals are already attacked, so the new attack goes at the rows themselves.

## Backlog

- `python tools/dossier.py --record-perf --group 'Core\Cli'` once the three above land — the goal's
  gate is `perf stale` on all 18 and nothing else (`rule:testing/member-perf-ledger`).
- `Core\Cli\Progress::advance` owes every proof and is not in this goal's item list; check where
  `python tools/dossier.py --emit-goals --dry-run` places it before reading that as a gap.
- `docs/examples/core/Cli/secret/03-log-in-at-the-terminal.nvs` hashes a password at run time, which
  is the one example in this group whose cost is argon2's; it measured well under a second.
