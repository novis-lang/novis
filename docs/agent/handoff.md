# Handoff

## State

Goal `core-cli-and-2-more`, items 1 to 3 landed: `Core\Cli::arguments`, `Core\Cli::colorDepth` and
`Core\Cli::ask` each carry `about.md`, three examples with blessed `.out` files, one attack, one
bench and a Rust test marked `covers:`. `python tools/verify.py` is green whole (14 of 14) at this
tree.

Every `Core\Cli` member now reads `perf stale` from `python tools/dossier.py --id`, because the
ledger keys on the implementing file's text (`rule:testing/member-perf-ledger`) and the three Rust
tests moved `crates/nvs-stdlib/src/cli.rs`. That is not a per-slice repair: one
`python tools/dossier.py --record-perf --group 'Core\Cli'` at the end of this goal clears all of
them, and any session taking another `Core\Cli` item would stale them again.

## Next group

**Stage: the goal's item list, in file order** — one file set: `crates/nvs-stdlib/src/cli.rs` for the
registry row, the body and the Rust test, plus `docs/examples/core/Cli/<member>/`,
`tests/hostile/core/Cli/<member>/` and `benches/members/core/Cli/<member>.nvs`. One slice is one
feature with all its feature proofs — `rule:testing/feature-proofs`.

- [ ] **`Core\Cli::confirm`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/cli.rs:257`
      It is `ask`'s shape with a `bool` answer, so `{default: false}` is what makes an example print
      without a terminal; `unanswered` at `crates/nvs-stdlib/src/cli.rs:1500` is the arm a Rust test
      can drive without blocking.
- [ ] **`Core\Cli::displayWidth`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/cli.rs:239`
      The one member of this class that answers the same number with every stream redirected, so an
      example may print a measured width.
- [ ] **`Core\Cli::escape`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/cli.rs:194`
      The launderer for the terminal sink; `terminal_output_substitutes_a_control_sequence_visibly`
      at `crates/nvs-stdlib/src/cli.rs:3870` already pins what it answers.

## Backlog

- Items 4 to 18 of this goal remain — `docs/agent/loop-goal.md` § *The item list, grouped by file set*.
- `Core\Cli` perf figures stay stale until one `--record-perf --group 'Core\Cli'` at the goal's end.
- `docs/examples/types/Cli-NotInteractive/` already shows catching a prompt nobody answered, so a
  prompt member's examples show the asking instead.
