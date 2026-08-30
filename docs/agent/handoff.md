# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open at D23** — P1 and P4 are both ticked, so the
driver's acceptance run now fails on the item-34 `[[check]]`'s third case,
`tests/conformance/core/a-named-closure-recurses.nvst`.

**A `: never` return lowers, and it lowers to `void`'s representation.** ADR 0007 § 3 makes `never`
return-only, and a frame that cannot come back hands its caller nothing — which is what `void`
already is. `nvs_ir::lower::erase_checked_ty`'s `CheckedTy::Never` arm is that decision's only home,
including why the call site keeps its ordinary fall-through instead of gaining a terminator of its
own: the callee's `throw` is the terminator, ADR 0002's unwind edge already carries it, and a mark
on the site would turn a checker hole into unreachable code that runs. Pinned by
`tests/conformance/core/a-never-method-is-a-terminator.nvst`.

**Two refusals the checker owes, found while deciding that and deliberately left open.**
`check_every_path_returns` (`crates/nvs-types/src/check.rs:701`) exempts `never` beside `void`, so a
`never` body that falls off its end compiles and comes back; and `nvs_types::returns` walks
statements syntactically, so a *call* to a `never` member does not count as leaving the frame the way
a `throw` does. Both are in `## Backlog`, and the erasure above is what makes the first harmless
rather than undefined. `nvs-ir`'s `KNOWN_ICE` is now **empty** — every atom ADR 0007 § 3 spells
reaches a diagnostic or an IR in both positions — and the crate's known gap 21 is deleted.

**One divergence is stated rather than hidden: `static::method(...)` binds the declaring class.** The
checker records `ResolvedCall::static_class` only for a written class name, and a class descriptor is
not a value a field slot can hold. `lower_callable`'s doc comment is that fact's home; the redesign
is in `## Backlog`.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is `E0794`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1059,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs: add **0027 § 1**, **0031 §§ 1-3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6** and
**0096 §§ 1-1a**; **0007 § 3** is the one this session needed and did not get. `[context] modules`
still misses `crates/nvs-types/src/attributes.rs`, `routes.rs`, `commands.rs`, `derive.rs`,
`testing.rs`, `consts.rs`, `retrieval.rs`, `crates/nvs-stdlib/src/serialize.rs` and `secret.rs`,
`crates/nvs-types/src/expr/args.rs`, `crates/nvs-types/src/check.rs` and `returns.rs`, and
`crates/nvs-ir/src/lower/closure.rs`, `expr.rs` and `call.rs`. `orient.py` itself still warns that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 34's two resolution gaps. Both are refusals written in the front end rather than
representations missing in `nvs-ir`, and they share
`crates/nvs-types/src/expr/` — take D23 first, since it is the next case the acceptance check
names.**

- [ ] **D23 a named closure recurses** — findings.md § *Divergences* D23, ADR 0031 § 3. `fn fact(int
      $n): int => $n <= 1 ? 1 : $n * fact($n - 1)` parses, but the recursive call resolves as a free
      function and is `E0320` at `crates/nvs-hir/src/members.rs:678`. The closure's own name has to
      be in scope inside its body, which is checked at
      `crates/nvs-types/src/expr/calls.rs:1241` (`check_fn_literal`) — decide there whether the name
      binds a local holding the closure or resolves specially, and say which in that function's doc
      comment. Case: `tests/conformance/core/a-named-closure-recurses.nvst`, named by the item-34
      `[[check]]` at `docs/agent/loop-goal.toml:330`.
- [ ] **D22 `inout` takes only a local** — findings.md § *Divergences* D22. `M::bump(inout $a["k"])`
      is `E0439` "cannot be passed to an `inout` parameter **yet**" at
      `crates/nvs-types/src/expr/args.rs:716`; the two neighbouring refusals at
      `crates/nvs-types/src/expr/args.rs:699` (a hooked property) and
      `crates/nvs-types/src/expr/args.rs:729` (anything that is not a place) stay whatever this
      decides. The question is whether an array element can be staged at all — the call site writes
      back into a copy-on-write array — so decide that before writing code, and record it in
      `args.rs`'s own doc comment.

## Backlog

- A `never` body that falls off its end is not refused — `crates/nvs-types/src/check.rs:701` exempts
  it beside `void`; `E0739`'s wording would need a `never` arm.
- A call to a `never` member does not count as leaving the frame —
  `crates/nvs-types/src/returns.rs:137` is syntactic and would need the expression table.
- `static::method(...)` as a first-class callable binds the declaring class —
  `nvs_ir::lower::closure::lower_callable`'s doc comment.
- The rest of item 34: D1, D7, D8, D10, D12, D16, D17, D21, D27, D33, D35, U21, M1 — findings.md
  § *Triage*.
- Stage 9, ADR 0119's expression `catch` — loop-goal items 21–23, resumes when stage 0c is green.
- Item 35, the doc-side findings — findings.md § *Triage*'s last open row.
