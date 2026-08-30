# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: P1, P4, D23, D27, D33, D22 and now U21 are
ticked, so the item-34 `[[check]]`'s first unwritten case is its seventh,
`tests/conformance/core/arr-from-takes-a-core-collection.nvst`, with seven after it.

**A generator's `current()` outside the iteration protocol throws `LogicError`.**
`nvs_ir::lower::generator`'s module doc is that decision's only home — which § 10 class, and why a
caller's own control flow makes it the `LogicError` half of the split — and
`lower_generator_current` owns why one `gen#state >= 1` comparison is exactly ADR 0053 § 1's two
points and no third: the factory parks `0`, every suspension parks `index + 1`, `finish_generator`
parks `GEN_DONE`. `foreach` drives `advance()`/`current()` in lockstep, so it never reaches the
throw and pays only the compare.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1063,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs, and **0053 § 1** is now the proven one — this session had to slice it by hand to learn what
U21 owed. Add it, plus **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**,
**0086 § 6** and **0096 §§ 1-1a**. `[context] modules` still misses
`crates/nvs-ir/src/lower/generator.rs`, `crates/nvs-hir/src/errors.rs` (spec § 10's tree, which any
finding that throws needs), `crates/nvs-stdlib/src/arr.rs`, `crates/nvs-hir/src/members.rs`,
`crates/nvs-types/src/attributes.rs`, `routes.rs`, `commands.rs`, `derive.rs`, `testing.rs`,
`consts.rs`, `retrieval.rs`, `generics.rs`, `crates/nvs-stdlib/src/serialize.rs` and `secret.rs`,
`crates/nvs-types/src/expr/args.rs`, `check.rs`, `returns.rs`, and
`crates/nvs-ir/src/lower/closure.rs` and `call.rs`. `orient.py` itself still warns that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 34's next two findings, in the order the item-34 `[[check]]` names their cases. They are one
group because they are one question asked twice — a `Core` collection's element type surviving the
registry's `CoreTy` — and they share `crates/nvs-stdlib/src/registry.rs`, whose `CoreTy::Iterated`
and `Iterable<T>` element map are where both answers live.**

- [ ] **D16 `Core\Arr::from` refuses the `Core` collections** — findings.md § *Divergences* D16.
      `from($objectSet)` is `E0401 expected array<T>|Iterable<T>|Iterator<T>`, and on `ObjectMap`
      the message leaks the unsubstituted `array<K>`, although `foreach` over both works. The row is
      `crates/nvs-stdlib/src/arr.rs:518`, its parameter is `CoreTy::Iterated` at
      `crates/nvs-stdlib/src/registry.rs:419`, and the per-class element map that has to answer for
      a `Core` receiver is `crates/nvs-stdlib/src/registry.rs:1380`. Case:
      `tests/conformance/core/arr-from-takes-a-core-collection.nvst`, named at
      `docs/agent/loop-goal.toml:334`.
- [ ] **D17 `Core\ObjectSet::union` loses its element type** — findings.md § *Divergences* D17.
      `union`/`intersect`/`diff` answer a bare `Core\ObjectSet`, so `foreach … as Tag $t` is
      "expected Tag, found T". All three rows are `CoreTy::Instance(NAME)` on both sides at
      `crates/nvs-stdlib/src/objset.rs:97`, which is the spelling that drops the argument; the
      variant is declared at `crates/nvs-stdlib/src/registry.rs:419`'s neighbourhood. Case:
      `tests/conformance/core/objectset-union-keeps-its-element-type.nvst`, named at
      `docs/agent/loop-goal.toml:335`.

## Backlog

- Item 34's remaining findings after D16/D17 — D21/D35, D1, then the three `Core\Json` and script
  cases: `docs/agent/loop-goal.toml:336-341` is the order.
- Item 35, the last of stage 0c — `docs/agent/loop-goal.md` § *Stage 0c*.
- Stage 9's expression `catch` — ADR 0119, items 21–23, nothing implemented.
- Stage 8's differential gap — 206 of 210, six of eight written.
