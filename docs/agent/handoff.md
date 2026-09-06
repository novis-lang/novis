# Handoff

## State

**Goal 8's stage 2 is on disk.** `nvs_config::cache::program_id` combines each unit's content hash
in program order with the environment digest and returns a `ProgramId` that displays as 64 lowercase
hex characters (`crates/nvs-config/src/cache.rs:150`); the four tests the TOML's stage 2 check names
are green. A `nvs run` computes it at its context wiring and writes it onto the `Ctx`
(`crates/nvs-cli/src/main.rs:1311`, `crates/nvs-runtime/src/ctx/wiring.rs:389`), so stage 3's member
has a slot to read and hashes nothing itself.

**Only the CLI run path writes one.** A served request's context gets an empty id: the "recomputed
at the hot-reload swap" half of the goal's standing decision needs a swap, and `nvs-cli`'s `Compiler`
still compiles once per written path and never revalidates (`docs/agent/loop-goal.toml:3740`). What
an unwritten id answers is stage 3's decision, not a blocker — the safe answer is that the member
refuses the way `Core\Command`'s name-less members do.

Nothing is blocked. `orient.py`'s `[context] modules` did not print `nvs-cli`'s `src/main.rs` or
`src/cache.rs`, which is where stage 2's threading actually landed; add both patterns.

## Next group

**Stage 3: the member** — one file set, `crates/nvs-stdlib/src/program.rs` plus the ADR, and the
first item is prose that decides the other two.

- [ ] **ADR 0061's folded amendment** — `docs/adr/0061-compile-time-autoload-and-program-discovery.md:177`
      (§ 5 is the last section; add a new one rather than renumbering §§ 4-5, which are cross-linked)
      and `:125` (§ 3, the sibling member's section, for the shape to match). Give `Core\Program` its
      first and only *runtime* member: the formula, the plain-`string` decision, and why the id
      cannot be a compile-time-folded constant — folding it into a unit changes that unit's bytes,
      hence its content hash, hence the id just folded. No new ADR number.
- [ ] **The row, the card and the body** — `crates/nvs-stdlib/src/program.rs:47` (`CLASS`), `:63`
      (the cards, in row order), `:1` (the module doc's "members never run" claim, which this member
      breaks) and `crates/nvs-stdlib/src/lib.rs:391` (`address`). The body reads
      `nvs_runtime::Ctx::program_id` (`crates/nvs-runtime/src/ctx/wiring.rs:389`) and formats
      nothing; `conventions.md`'s five edits is the checklist.
- [ ] **The cases** — three new `.nvst` under `tests/conformance/core/`, each asking a different
      question, and `examples/program-id.nvs`, whose `exact` check at
      `docs/agent/loop-goal.toml:4443` wants exactly `id is 64 lowercase hex characters` and `two
      reads within one run agree` on stdout. The registry's coverage floor is three
      (`crates/nvs-stdlib/tests/conformance_coverage.rs:1`).

## Backlog

- A served request writes no program id — see `## State`; it lands with ADR 0017's unit-table swap,
  which stage 7's own checks are also waiting on (`docs/agent/loop-goal.toml:3740`).
- Stage 3 also owes `docs/reference/core/Program.md` beside the registry card, per ADR 0117.
- When this goal's last check goes green the driver takes goal 9 — `Core\Db\Schema`.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
