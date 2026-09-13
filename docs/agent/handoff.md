# Handoff

## State

**Goal `gap-register` — one register reads every place a gap is written, and a milestone is an owner.
Stages 1–3 are green.** The two sides now agree on what an owner is: `tools/owners.py`'s module doc is
the rule, and `crates/nvs-stdlib/tests/spec_registry_coverage.rs` enforces it over the ratchet keys.

- `owner_problem` takes the plan's table and accepts three kinds: a live goal slug, `unowned`, and a
  milestone tag with a row in `docs/implementation-plan.md` that is M9 or later and not `done`. The
  three refusals are `tools/owners.py:@classify`'s three, in its order.
- `FIRST_FUTURE_MILESTONE` now has a second home, in Rust — no file both sides can read it from — and
  each is held by its own test. No ratchet key uses a milestone tag yet; the gate accepts one now.
- The floor check `no document in the tree still cites this goal's subject by ADR number`
  (`tools/check-links.py`) was red on three crate-relative paths in `tools/loop.py`'s partition comment
  and is green; the playbook carries the trap.

Nothing is blocked, and no gap was closed — this goal builds the register, it does not empty it.

## Next group

**Stage 4: owed work is written under one heading** — one file set: the module docs that record it
elsewhere, plus `tools/owners.py`.

- [ ] **`owners.py` warns when owed work is written outside a `# Known gaps` block** —
      `tools/owners.py:@report`. The line is `sections outside Known gaps: N`, which the stage 4 check
      wants at `0` (`docs/agent/loop-goal.toml:10111`); the sweep that finds them is
      `python tools/peek.py "crates/**/*.rs:re://! #+ .*(not yet|owe|still missing|not armed)"`, and the
      goal's stage 4 says to re-run it rather than trust the list. The rule is that stage and this
      module's own doc.
- [ ] **Two module docs move their owed work into their `# Known gaps` block** —
      `crates/nvs-stdlib/src/test.rs:138` (§ 18's in-process request; the block it moves into is
      `crates/nvs-stdlib/src/test.rs:92`, owner goal `m7-server-surface`) and
      `crates/nvs-cli/src/runner.rs:83` (§ 2's test parallelism, whose owner this stage decides). Numbered
      items with an owner tag, and the emptied heading rewritten to say what the module does rather than
      deleted — `AGENTS.md` rule 7 and `tools/owners.py`'s module doc § *Three owner kinds*.
- [ ] **The three that share one owner between two crates** — `crates/nvs-render/src/lib.rs:12` (the HTML
      rendering and three producers, goal `m8-stdlib-depth`), `crates/nvs-server/src/schedule.rs:77` and
      `crates/nvs-cli/src/serve.rs:763` (a `fleet` entry is never armed under `nvs serve`, goal
      `m7-server-surface`). Same shape as the item above.

## Backlog

- Eight more module docs the goal's stage 4 names are *read each*, and one recording no owed work is
  left alone — `docs/agent/loop-goal.md` § Stage 4.
- `crates/nvs-stdlib/src/process.rs` has no gap block at all and `Core\Process::spawn` is owed — stage 4
  gives it one.
- Stage 5 is `python tools/plan.py --past` over `owners.py --json`, then `--sync` writing `done` into a
  complete milestone's *Carried by* cell — `docs/agent/loop-goal.md` § Stage 5.
- The ratchet keys tagged `unowned` stay that way; re-tagging spec § 17's four classes to a milestone is
  goal `unowned-closures`', not this goal's — `docs/agent/loop-goal.md` § Standing decisions.
