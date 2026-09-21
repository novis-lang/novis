# Handoff

## State

Goal `core-cli-and-2-more` is met. All 18 of its features carry their feature proofs, and its three
acceptance checks — `dossier: Core\Cli`, `Core\Cli\Color`, `Core\Cli\Live` — each report `nothing
owed` with no failing example and no failing attack. This session landed `Core\Cli\Color::index`,
`Core\Cli\Color::rgb` and `Core\Cli\Live::set`, each with `about.md`, three examples with blessed
`.out` files, one attack, one bench and a Rust test marked `covers:`.

The perf ledger is current for the whole group: fifteen `Core\Cli*` features were re-measured after
the last slice landed, so nothing there reads `perf stale`. `python tools/verify.py` is green whole
(14 of 14) and `python tools/verify.py --doc` resolves every link.

Two facts this group cost time to establish. **A `Core\Cli\Color` is compared by identity**, so two
`index(1)` calls are never `==`, which is what lets a bench chain on the value it just built.
**`Core\Cli\Live::set` copies every row into a new frame before the region finds out it has no
terminal**, which is 9 allocations for a one-row frame — the bench declares that figure, and the
backlog holds the question.

## Next group

**Stage: the next chain entry, goal `core-cli-progress-and-6-more`** — one file set:
`crates/nvs-stdlib/src/cli.rs` for the registry row, the reference card and the Rust test, plus
`docs/examples/core/Cli-Progress/`, `docs/examples/core/Cli-Style/`, `docs/examples/core/Cli-Text/`
and the matching `tests/hostile/` and `benches/members/` paths. One slice is one feature with all
its feature proofs — `rule:testing/feature-proofs`.

- [ ] **`Core\Cli\Progress::advance`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:891`. The counter saturates rather than starting again, which is
      the boundary an attack is written around.
- [ ] **`Core\Cli\Style::of`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:2598`. Seven independent slots, all optional, so `of({})` is the
      identity a program passes where a style is required.
- [ ] **`Core\Cli\Text::plain`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:1941`. The carrier every other row is made of: it substitutes
      the control bytes, which is what a hostile case should try to get past it.

## Backlog

- `Core\Cli\Live::set` builds the whole frame and clones every row before `Region::set` returns
  early on a run with no terminal — 9 allocations per call that render nothing. An optimisation,
  not a bug: `crates/nvs-stdlib/src/cli.rs`, `nvs_core_cli_live_set`.
- `Core\Cli::multiSelect`, `select` and `write` were not re-measured by this session's group run;
  they were already current, and `python tools/dossier.py --verify --group 'Core\Cli'` agrees.
