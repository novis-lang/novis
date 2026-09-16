# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 38`,
`past-milestone: 8`, `goal-owned: 64`, `milestone-owned: 15`, `untagged: 0` and `unreasoned: 0`; the
stage wants the first at 0, and its other check, `python tools/owners.py --deferrals`, is green. That
count is the goal's whole remaining work: `owners.py` § *UNOWNED* lists all 38, each with its reason in
`docs/agent/carried-gaps.md`, and the § above it lists the 8 deferred to a milestone already passed.
Nothing is blocked.

**The LSP cursor thread is finished and `crates/nvs-lsp` records no `# Known gaps` at all** — it closes
no unowned item, so the count above is where the goal now stands.

## Next group

**Stage 6: `nvs-types`' own unowned gaps, each one built or given an owner** — one file set:
`crates/nvs-types/src/signatures.rs`, `crates/nvs-types/src/lib.rs`, and the `.nvst` cases under
`tests/conformance/`. Each item is the same decision, taken per the goal's § *Standing decisions*: build
it, state it as a bound (which strikes the gap), or defer it to an M9+ milestone whose plan states the
scope — never rewrite the gap down to what exists.

- [ ] **A promoted constructor parameter is a property here, visibility included** —
      `crates/nvs-types/src/signatures.rs:24` (gap 1), `rule:classes/promotion-is-constructor-only`,
      `rule:core-api/written-visibility`. The table this module builds skips a `constructor(public int
      $x)`, matching `nvs_hir::members`'s own member table, so `is_visible_from` is never reached for
      one and its `private` is not enforced. The method side has no such gap because the modifier is on
      the declaration; building this is the two tables together, in one slice.
- [ ] **What a variadic parameter's declared type means is a bound, not a hole** —
      `crates/nvs-types/src/signatures.rs:33` (gap 2). The type is matched against every argument from
      that position onward rather than modelled as `array<T>`, which is an element check and is what
      `crate::expr` uses. If that is the intended semantics, say so as the module's own prose with its
      limit and strike the gap; if it is not, the `array<T>` model is the build.
- [ ] **Equality-operand compatibility is one pass or none** — `crates/nvs-types/src/lib.rs:149`
      (gap 1), `rule:expressions/equality-semantics`, `rule:expressions/mixed-equality`. Nothing
      diagnoses `==` between two different enum types, or `int` against `uint`; the gap's own reasoning
      is that singling enums out leaves the operator inconsistent with itself. `rule:types/conversion`
      already refuses the `as` spelling, so the comparison is the whole hole.

## Backlog

- The other unowned clusters, largest first: `crates/nvs-ir/src/lib.rs` (11 items), `crates/nvs-cli/src/openapi.rs` (5), `crates/nvs-host/src/*` (3) — `python tools/owners.py` § *UNOWNED*.
- The 8 gaps deferred to a milestone the program has already passed, each needing a live owner — `owners.py` § *DEFERRED TO A MILESTONE THE PROGRAM HAS ALREADY PASSED`.
- `crates/nvs-types/src/lib.rs:156` gap 2 and `crates/nvs-types/src/locals.rs:84` gap 1 — exhaustive control-flow reachability, and what a falling-through `switch` case contributes; both in the group's file set if it runs short.
- `crates/nvs-lsp/src/hints.rs:64` gap 1 — the one unowned LSP item, its reason at `docs/agent/carried-gaps.md:627`.
