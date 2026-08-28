# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **867 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **2 unresolved of the 128 named test entries** `loop-goal.toml` holds, and
`nvs-ir (control flow)` is now **0 of 7** — closed, joining `nvs-ir (targets and tags)`,
`nvs-syntax (the last unparsed shapes)` and both `nvs-types` blocks. Both remaining names are in
the next group. The debt file's two counts stay derived off the tree by the pass its header
describes, never carried forward.

This session closed the last `nvs-ir` refcount name, and it was a lowering fix rather than a test
half. A transferred argument — and a transferred *receiver*, which is argument 0 — now rides
`Lowering::owned_temporaries` under the new `TemporaryKind::Transferred`, so a later argument's
throw releases it instead of leaking it. The whole subtlety is *when* it comes off:
`Lowering::forget_transferred_since` runs immediately **before** the call is emitted at each of the
three transferring sites, because `emit_fallible` builds the call's own fault edge from the stack
as it stands and the callee releases its parameters on its throwing edge too. The field's
`# Known gap` paragraph is gone; the mechanism is that field's doc plus
`forget_transferred_since`'s.

## Next group

**The two remaining guard names, both about what a *frame* owes on an exit that is not a return.**
They share no file with what this session had open, which is why this session stopped at one slice.
File set: `crates/nvs-codegen/tests/throwing.rs:198` (the neighbouring half, whose fixture shape the
first one reuses), `examples/fatal.nvs`, and `docs/agent/guard-name-debt.md:218` and `:320` for the
two reconciliations.

- [ ] **`a_fatal_releases_the_frames_locals`** — `docs/agent/guard-name-debt.md:218` says the work
      is genuinely not done: `throwing.rs` asserts that a throwing frame releases the strings it
      held (`crates/nvs-codegen/tests/throwing.rs:198`) and that a fatal is not caught, but nothing
      asserts what ADR 0020's `FATAL` does to the frame's locals, and the valgrind sweep cannot see
      it because `examples/fatal.nvs` is on its skip list for exiting non-zero by design. So the
      assertion has to be made in `nvs-codegen`'s own test, over the emitted code, the way its
      neighbour is.
- [ ] **`every_refusal_is_a_diagnostic_or_decided`** — Stage 8's own gate, and the bare entry at
      `docs/agent/guard-name-debt.md:320`. `python tools/holes.py` already reads every refusal site
      out of `nvs-ir` and `nvs-codegen` and attributes it; the test is that walk asserted, with the
      allowlist being `docs/agent/loop-goal.md` § *Standing decisions* and nothing else. That
      section says the allowlist may never grow without a decision taken in the same session.

## Backlog

- ADR 0007 § 2's `array<T> as array<U>` lowers, so the playbook bullets that route around it
  (`Core\Csv::format`'s unreachable column, the `array<mixed>` element read) are now stale on that
  clause — `docs/agent/playbook.md` owns them.
- `nvs-ir` gap 1: `Class::method(...)` as a first-class callable still panics — the crate's own
  module doc.
- The three fixtures under `examples/` are the only place a whole program is run — `docs/plan/m4.md`
  § *Verify*.
- `landing_block`'s remaining inline-release producers are gone from its doc; the next refcount
  question there is whichever new producer arrives, not a listed one.
