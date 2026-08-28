# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **867 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **3 unresolved of the 128 named test entries** `loop-goal.toml` holds, and
the `nvs-ir (targets and tags)` block is now **0 of 8** — closed, joining `nvs-syntax (the last
unparsed shapes)` and both `nvs-types` blocks. `nvs-ir (control flow)` is at **1 of 7**. The debt
file's two counts stay derived off the tree by the pass its header describes, never carried forward.

This session closed two of the `nvs-ir` names, and one of them was a lowering fix rather than a test
half. ADR 0007 § 2's `array<T> as array<U>` row **lowers today** — the debt entry claiming it panics
was stale about the tree — so its guard is written over `Lowering::lower_array_restamp`
(`crates/nvs-ir/src/lower/convert.rs:957`) as an agreement over the row's four spellings. The other
was real: a `match` subject, each of its labels, and an `unset` target's keys were released only on
the normal edge, so a throw out of a comparison or out of the fallible descent leaked them. Each now
rides `Lowering::owned_temporaries` for the length of the throwing region and is `forget`ten past
it — the multi-exit shape, since a `match` subject has one normal exit per arm and the stack
releases at one point.

## Next group

**The last `nvs-ir` refcount name, then the two `nvs-codegen`/Stage 8 ones.** The first is the one
`Lowering::owned_temporaries`' own `# Known gap` paragraph names, so its file set is the one this
session just had open: `crates/nvs-ir/src/lower/mod.rs:1257` (the field doc and its gap paragraph),
`crates/nvs-ir/src/lower/call.rs:900` (`own_temporary` and the four stack helpers beside it),
`crates/nvs-ir/src/lower/tests.rs:3236` (the sibling guard, whose fault-edge helpers this one reuses)
and `docs/agent/guard-name-debt.md:91` for the reconciliation.

- [ ] **`a_transferred_argument_is_released_when_a_later_one_throws`** — the field doc's own gap:
      `f($a, g())` retains `$a` for an `ArgOwnership::Transferred` parameter and leaks it when `g`
      throws. The paragraph at `crates/nvs-ir/src/lower/mod.rs:1257` states the fix it needs — a
      second entry kind on the stack, released on the error edge and *forgotten* on the normal one,
      plus one forget at each of the three transferring call sites. Note that
      `forget_temporaries_since`/`forget_temporary` (`call.rs:932`, `:953`) already give the forget
      half, so what is new is only the kind tag.
- [ ] **`a_fatal_releases_the_frames_locals`** — `docs/agent/guard-name-debt.md:220` says the work
      itself is not done, not just the name; read that entry before writing anything.
- [ ] **`every_refusal_is_a_diagnostic_or_decided`** — Stage 8's own gate,
      `docs/agent/guard-name-debt.md:315`. Its allowlist may never grow (loop-goal.md § *Standing
      decisions*), so this is the one that reads as an audit rather than a test.

## Backlog

- ADR 0007 § 2's `array<T> as array<U>` lowers, so the playbook bullets that route around it
  (`Core\Csv::format`'s unreachable column, the `array<mixed>` element read) are now stale on that
  clause — `docs/agent/playbook.md` owns them.
- `nvs-ir` gap 1: `Class::method(...)` as a first-class callable still panics — the crate's own
  module doc.
- The three fixtures under `examples/` are the only place a whole program is run — `docs/plan/m4.md`
  § *Verify*.
