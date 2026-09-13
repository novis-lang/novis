# Handoff

## State

**Goal `plan-truth`'s stage 5 is green.** `python tools/playbook.py --check` prints `none -- every
carried-gaps owner is live or struck`, the string the stage's `[[check]]` wants. Two things had to
change: the rows, and the tool that reports on them.

`docs/agent/carried-gaps.md:54` now names `m7-server-surface`, whose stage 7
(`docs/agent/goals/57-m7-server-surface.md:209-214`) gives `Core\Test::request` its `{headers?, body?}`
bag and spells the signature into spec § 13's cell. The old row was wrong twice: what is missing is
code, not prose — the registry row takes a method and a path only
(`crates/nvs-stdlib/src/test.rs:436-452`) — and the spec cell it cited as `:999` is `Core\Decimal`'s;
`Core\Test`'s is `docs/spec/01-core-library.md:1001`. The rule's elided `request(...)` is not a gap:
the signature has one home and it is the spec cell.

The untyped-grant row left § *Owned* for § *Unowned*. `crates/nvs-config/src/tree.rs:50`'s `Setting` is
`#[serde(untagged)]` and shared by every directive, and `:39-43` records that `memory = false` and
`exporter = false` are load-bearing second spellings, so narrowing the enum is not open — the arms a
directive accepts are per-directive and nothing declares them. `docs/plan/m6.md:52` promises the
`nvs config check` verb and not the refusal, so a milestone tag would not have been honest either.

`tools/playbook.py`'s carried-gaps section printed only on findings, unlike its three siblings; it now
prints the `none` line. That is the only code beyond `plan.py --stale` this goal has written, and it is
a report line, not behaviour.

Stage 5 was the driver's only red check, so its next sweep may reach the goal. This session did not
claim DONE, because it did not run the sweep — the driver's own green verdict is what advances the
chain, and a wrong DONE halts the run.

## Next group

**Stage 4: prose the tree contradicts** — one file set: `crates/nvs-types/src/` and the
`docs/rules/types/` fragments it cites.

- [ ] **`array<T>` is covariant in the checker and invariant in the rulebook** —
      `crates/nvs-types/src/expr/assign.rs:172` recurses element to element, so `array<int>` satisfies
      `array<int|float|decimal>`, and `:161-171` argues that is sound because an array is a
      copy-on-write value rather than an alias. `rule:types/arrays` states invariance flatly and prices
      the widening at an O(n) restamp, and `crates/nvs-types/src/expr/assign.rs:228` still speaks of
      that invariance as live. Read `crates/nvs-types/tests/arrays.rs` first: if the covariance is
      guarded it is the tree and the fragment is the edit (§ *Standing decisions*), and if nothing
      guards it this is a semantics bug and a `BLOCKED`. `rule:types/conversion`'s restamp sentence
      moves with it either way. Carried in `docs/agent/carried-gaps.md` § *Unowned* so it survives a
      goal switch.
- [ ] **A comment that reads as a changelog** — `crates/nvs-types/src/lib.rs:160`'s "Definite
      assignment itself is no longer conservative" puts history where the present tense goes
      (AGENTS.md rule 7, `docs/agent/conventions.md` § *A code comment*). Rewrite the sentence as what
      the pass does now, inside the same `# Known gaps` bullet.

## Backlog

- `rule:security/isolate-teardown-is-a-drain-then-a-sweep` tallies field slots only; the sweep also
  tallies a solely owned array (`crates/nvs-runtime/src/object.rs:1822`). Code ahead of the fragment,
  so the fragment is the edit, by the rule's own process.
- `crates/nvs-test/src/case.rs:351`'s `NOT_YET` reason string still names M6; correcting it is a code
  change, which this goal's § *Standing decisions* forbids.
- Stage 1 is goal `websocket-client`'s carried floor and nothing in it is known red.
