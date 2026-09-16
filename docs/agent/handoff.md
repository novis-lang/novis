# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 35`,
`past-milestone: 8`, `untagged: 0`, `broken-tag: 0` and `unreasoned: 0`; the stage wants the first at 0,
and its other check, `python tools/owners.py --deferrals`, is green. `owners.py` § *UNOWNED* is the
roster of all 35, each with its reason in `docs/agent/carried-gaps.md` § *Unowned*.

**`nvs-types` now carries four unowned gaps, all in the § *Next group* below**, and the three struck this
session were each stale rather than open: the code was already ahead of the prose. `python
tools/verify.py` is green at 11 of 11 (conformance 1999). Nothing is blocked.

## Next group

**Stage 6: `nvs-types`' last four unowned gaps, each one built or given an owner** — one file set:
`crates/nvs-types/src/{error_lib.rs,intrinsics.rs,locals.rs,lib.rs}` and the `.nvst` cases under
`tests/conformance/`. Each item is the same decision, taken per the goal's § *Standing decisions*: build
it, state it as a bound (which strikes the gap), or defer it to an M9+ milestone whose plan states the
scope — never rewrite the gap down to what exists. **Read the code before believing the gap**: all three
items this session took described holes the tree had already closed, and `nvs.exe check` on a six-line
scratch file settled each one faster than the prose did.

- [ ] **A property read straight off `$e->previous` reaches its receiver, or the gap is `nvs-ir`'s** —
      `crates/nvs-types/src/error_lib.rs:57` (gap 1), `rule:expressions/nullable-conversion`. The chain
      is built and the property reads back as `Throwable|null`, but that erases to `nvs_ir::Ty::Tagged`,
      so `$e->previous->message` needs a local and a `!= null` narrowing first. Decide whether the
      tagged-receiver case is built (which is `nvs-ir`'s own known gap, not this crate's) or whether
      this module states the narrowing as its bound.
- [ ] **`nvs check` either reads a configuration or the host check is scoped to the callers that do** —
      `crates/nvs-types/src/intrinsics.rs:94` (gap 6), `rule:core-classes/db-literal-query-checking`.
      `crate::Env::grants` is the channel and `crate::check::check_program_granted` fills it; `nvs-cli`'s
      check path hands over no `nvs.toml`, so the refusal fires for nobody. A command that reads
      configuration is a command a broken `nvs.toml` can fail, which is the decision.
- [ ] **The two flow checks are one analysis or none** — `crates/nvs-types/src/lib.rs:162` (gap 1) and
      `crates/nvs-types/src/locals.rs:84` (gap 1). Exhaustive "every path returns" reachability, and a
      `switch` case falling through contributing nothing to the case it falls into, are the same
      question: whether the structural walk becomes a CFG, since a fall-through edge is exactly what a
      structural walk cannot carry. Take it as one decision or defer both to one M9+ milestone.

## Backlog

- `crates/nvs-ir/src/lib.rs` holds 11 of the 35 unowned gaps (`:209`, `:219`, `:261`, `:291`, `:302`,
  `:376`, `:421`, `:434`, `:470`, `:496`, `:532`) — the largest single file set left in stage 6.
- `crates/nvs-cli/src/openapi.rs` gaps 1–5 and `crates/nvs-host/src/{group,placed,worker}.rs` are the
  other two clusters; `python tools/owners.py` § *UNOWNED* is the roster.
- The 8 § *DEFERRED TO A MILESTONE THE PROGRAM HAS ALREADY PASSED* items are owed by a goal or nobody
  and are not counted by this stage's check — `docs/agent/carried-gaps.md` § *Unowned* is their index.
- `docs/agent/goals/52-plan-truth.md:107` names `crates/nvs-types/src/lib.rs` gaps 1–3 by number; that
  block now holds one gap, so the line is a position reference that has moved.
