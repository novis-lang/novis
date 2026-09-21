# Handoff

## State

Goal `core-cli-and-2-more`, ten items landed: `Core\Cli::arguments`, `colorDepth`, `ask`, `confirm`,
`displayWidth` and `escape` before this session, and `write`, `isTty`, `width` and `height` in it.
Each carries `about.md`, three examples with blessed `.out` files, one attack, one bench and a Rust
test marked `covers:`. `python tools/verify.py` is green whole at this tree.

Five features in the group still owe their proofs: `select`, `multiSelect`, `secret`, `live` and
`progress`.

`Core\Cli::write`'s bench found a second allocation per call, which its declared `allocations 1`
caught: the substitution's `into_owned` sized the buffer exactly and the newline then grew it. The
`string` arm now sizes the buffer for the newline before it fills it, and the member's *What it
spends* paragraph at `crates/nvs-stdlib/src/cli.rs:1191` says what it costs now.

Every `Core\Cli` member but `write` reads `perf stale` from `python tools/dossier.py --id`, because
the ledger keys on the implementing file's text (`rule:testing/member-perf-ledger`) and every
session of this goal moves `crates/nvs-stdlib/src/cli.rs`. One
`python tools/dossier.py --record-perf --group 'Core\Cli'` at the end of the goal clears all of
them; a session taking another item would stale them again.

## Next group

**Stage: the goal's item list, in file order** — one file set: `crates/nvs-stdlib/src/cli.rs` for the
registry row, the body and the Rust test, plus `docs/examples/core/Cli/<member>/`,
`tests/hostile/core/Cli/<member>/` and `benches/members/core/Cli/<member>.nvs`. One slice is one
feature with all its feature proofs — `rule:testing/feature-proofs`.

- [ ] **`Core\Cli::select`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:1676`. An empty choice list is refused as a `Logic` throw
      whether or not anybody is watching, and an unattended run calls no `labels` callback, so that
      refusal is the attack's first step and an example's `.out` is the default choice.
- [ ] **`Core\Cli::multiSelect`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:1789`. One shape twice with `select`, over a set rather than one
      choice, so the two share their example ideas.
- [ ] **`Core\Cli::secret`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:1848`.

## Backlog

- `Core\Cli::live` owes every proof — `crates/nvs-stdlib/src/cli.rs:3084`.
- `Core\Cli::progress` owes every proof — `crates/nvs-stdlib/src/cli.rs:3229`.
- One `python tools/dossier.py --record-perf --group 'Core\Cli'` at the end of the goal, then
  `--perf-report` — `rule:testing/member-perf-ledger`.
