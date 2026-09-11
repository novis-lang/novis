# Handoff

## State

**Goal `resource-ceilings`: stage 3 is green.** Its second check's last name,
`the_limit_handler_runs_once_and_is_not_re_entered_when_it_overruns`, is now a unit test in
`crates/nvs-runtime/src/ctx/hooks.rs`, beside the two that were already there under the tree's own
names — `crates/nvs-runtime/src/ctx/safepoint.rs:591` and `crates/nvs-runtime/src/abi.rs:995`. The
check's `args = ["test", "-p", "nvs-runtime"]` was the right half: a closure value needs only
`ClassTable`, `MethodRow` and the `CLOSURE_*` constants, and that module's own `hook_of` already
builds one. Both TOML copies are byte-identical and their comment now says what the tree holds.

**What is left of the goal is stage 7.** Every test name stages 4, 5 and 6 list exists
(`crates/nvs-runtime/tests/allocator_ceiling.rs`, `.../refusal.rs`, `.../detached_accounting.rs`,
`crates/nvs-stdlib/src/cache.rs`), and both `.nvst` cases those stages name are on disk, so the
driver's own acceptance run is what confirms them. Stage 7 is the goal's record and the rule
fragments it creates, plus `python tools/rules.py --render`.

**One store stays unbracketed on purpose**, still the compiled-pattern cache —
`crates/nvs-stdlib/src/regex.rs`'s gap 4 is the finding, waiting on M6's arena.

## Next group

**Stage 7: the record and the rules it creates** — one file set, `docs/decisions/` and
`docs/rules/`, with nothing under `crates/` to touch. The goal's § *Standing decisions* is the
specification: one new record, no existing one amended, and its `changes.creates` names the refusal
and its degenerate return, the accounting boundary and stage 7's expansion rule.

- [ ] **Read what stage 7 asks of the record before writing a line of it** —
      `docs/agent/loop-goal.toml:8158` is the rulebook check and the one beside it is
      `tools/decisions.py --gate`; the goal's stage 7 prose in `docs/agent/loop-goal.md` is what the
      record has to say. `rule:programs/memory-priority` is the rule it works inside, and
      `docs/rules/programs/memory-priority.md:1` is that fragment.
- [ ] **Write the record at the next free number** — `docs/decisions/0174.md:1` is the newest
      neighbour to take the shape from, and `docs/agent/conventions.md` § *A decision record* is the
      shape itself. Re-derive the number from `docs/decisions/` immediately before creating the file:
      the goal's own text warns that any number named in it has since been claimed.
- [ ] **Write the fragments, wire `because`, and re-render** — `docs/rules/errors/on-limit.md:1` is
      the rule the record cites and the model for a fragment's voice; every rule under `creates` gets
      a topic-JSON entry whose `because` opens with the new number, and `python tools/rules.py
      --render` rewrites the three generated files. `session.py --wrap` runs `--check` for any
      session that touched `docs/rules/`.

## Backlog

- The compiled-pattern cache is unbracketed — `crates/nvs-stdlib/src/regex.rs` gap 4, waits on M6's
  arena (`docs/agent/carried-gaps.md`).
- Stage 3's `.nvst` case and stages 4–6's artefacts are all on disk; nothing here re-checks them, the
  driver's acceptance run does (`docs/agent/loop-goal.toml`).
