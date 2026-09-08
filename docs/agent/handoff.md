# Handoff

## State

**Goal 18's acceptance list has no known-red check left. Stage 5's three checks were each run in this
session and each passes**, so the status line claims `DONE`; the driver measures that claim against
its own acceptance run and records a `done-claim` if it disagrees.

The stage-5 failure was never missing work. `examples/input-shapes.nvs` and `examples/input-shapes.nvsr`
have been on disk and correct; the check ran the fixture as a bare `nvs run`, so `postAs()` answered
its CLI refusal — the right answer to the wrong question. The missing half was the check's own `args`,
now `docs/agent/loop-goal.toml:5817`, and `nvs run --request examples/input-shapes.nvsr
examples/input-shapes.nvs` prints the six `want` lines exactly, exit 0.

**The five roster gates are standing green** — `every_part_two_spec_member_is_registered`,
`every_part_two_member_has_a_conformance_case`, `every_core_class_has_a_conformance_floor_of_three`,
`every_registry_row_carries_a_reference_card` and
`every_registry_rows_names_are_the_specs_signature_column` all pass under `cargo test -p nvs-stdlib`,
with nothing below a floor and no card missing. `python tools/reference.py --check` passes: 291 of 291
examples hold.

**`docs/agent/goals/18-input-shapes.toml` had fallen three hunks behind its live copy** — the
`[context]` modules, the stage-3 rules overlay and stage 4's amended `nvs-suite` cases were all
written to `loop-goal.toml` alone by earlier sessions. The two files are now byte-identical modulo
line endings; the playbook bullet above owns why a plain `diff` hides this.

**What this session did not run:** the WSL leg and the valgrind sweep. Both were judged from the
driver's own report rather than executed here.

## Next group

**Stage 5: what the acceptance run says, and the one gap it will not catch** — one file set:
`docs/agent/loop-goal.toml`'s stage-5 block and `tools/loop.py`'s fixture runners. `rule:testing/four-proofs`
is the specification for what a fixture is meant to prove; the goal's stage-5 prose is
`docs/agent/loop-goal.md:"## Stage 5"`.

- [ ] **Read the driver's verdict before writing anything new** — `docs/agent/loop-goal.toml:5811` is
      the check that was red, and `:5817` is the `args` line that closes it. If the run went green the
      goal switched and this whole handoff is stale; if it went red the failure names the leg, and the
      WSL leg is the only one this session did not exercise.
- [ ] **The valgrind sweep runs the request fixture without its request** — `tools/loop.py:2394`
      (`cmd_for`) builds `valgrind … nvs run <file>` with no check `args`, so
      `examples/input-shapes.nvs` is leak-checked on its refusal path rather than on the hydration path
      it exists to prove. Either thread the owning check's `args` through the sweep, or say in
      `[valgrind] skip` that this fixture is covered elsewhere — the second is a smaller claim and
      needs the first written down as why.
- [ ] **Nothing checks a check's `args` fixture is on disk** — `tools/loop.py:1749` refuses a `file`
      outside `files`, but `examples/input-shapes.nvsr` is named only in `args`, and a `.nvsr` cannot
      join `files` because the valgrind sweep would `nvs run` it. A missing request fixture fails the
      check with a confusing message rather than at load.

## Backlog

- Goal 18's prose stage 5 is not re-read against the tree — `docs/agent/loop-goal.md`.
- `docs/reference/core/Request.md` and `Arr.md` carry the members; no session has re-read them since.
- The `[context]` manifest gap that sent this session to `tools/loop.py` six times: no `tools` field
  exists, and the driver's own check schema is what a stage-5 session reads — `docs/agent/loop-goal.toml`'s `[context]`.
