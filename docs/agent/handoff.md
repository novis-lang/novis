# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: nine of its seventeen findings are ticked,
D16 and D17 last, so the item-34 `[[check]]`'s first unwritten case is its ninth,
`tests/conformance/core/an-attribute-payload-holds-a-constant.nvst`, with five after it.

**Two rosters answer "does this class implement that interface", and a `Core` class is only in the
second.** `nvs_hir`'s `ClassGraph` holds what a *program* declared; what a `Core` class says about a
hierarchy is seeded straight into the signature table off `nvs_stdlib::registry::ITERABLES`.
`crate::expr::assign::class_satisfied` is that rule's home. Beside it,
`crate::generics::with_class_args` is now the one home of "what a class fixed for an interface is
written in the class's own type variables, and the receiver's arguments go in" — three callers, and
answering it three ways is what made `Core\Arr::from($set)` refuse a set `foreach` walks.

**`CoreTy::Instance` of a *generic* class interns at that class's own type variables**, never bare
(`nvs_types::core_lib`'s `lower`). That is what carries `<T>` through `ObjectSet::union`, and it
tightens the parameter with it: `union` on an `ObjectSet<Pin>` now refuses an `ObjectSet<Tag>`.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1065,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs; **0053 §§ 1-3** and **0007 §§ 2-3** are the proven ones, plus **0027 § 1**, **0031 § 3**,
**0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6** and **0096 §§ 1-1a**. `[context] modules` misses
every file this session actually edited — `crates/nvs-types/src/generics.rs`, `expr/assign.rs`,
`expr/iteration.rs`, `expr/args.rs` and `crates/nvs-stdlib/src/objset.rs` — and still misses
`crates/nvs-ir/src/lower/generator.rs`, `crates/nvs-hir/src/errors.rs` (spec § 10's tree),
`crates/nvs-stdlib/src/arr.rs`, `crates/nvs-hir/src/members.rs`,
`crates/nvs-types/src/attributes.rs`, `retrieval.rs`, `routes.rs`, `commands.rs`, `derive.rs`,
`testing.rs`, `consts.rs`, `generics.rs`, `crates/nvs-stdlib/src/serialize.rs` and `secret.rs`,
`crates/nvs-types/src/expr/args.rs`, `check.rs`, `returns.rs`, and
`crates/nvs-ir/src/lower/closure.rs` and `call.rs`. `orient.py` itself still warns that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 34's next three findings, in the order the item-34 `[[check]]` names their cases. The first
stands alone on the attribute retrieval path; the two after it are one question — a typed decode
reaching a shape the folder cannot build — and share `crates/nvs-stdlib/src/json.rs` with the
`#[Json\Derive]` half in `crates/nvs-types/src/derive.rs`.**

- [ ] **D21 an attribute payload holding a class constant or enum case cannot be retrieved** —
      findings.md § *Divergences* D21. Declaring it is fine; `Core\Attributes::get`/`all` is
      `E0731`, raised at `crates/nvs-types/src/retrieval.rs:413`, whose code is
      `crates/nvs-diagnostics/src/lib.rs:1814`. Case:
      `tests/conformance/core/an-attribute-payload-holds-a-constant.nvst`, named at
      `docs/agent/loop-goal.toml:336`.
- [ ] **D7/D8 `Core\Json::decodeAs` FATALs on a nested-class field, and a promoted constructor
      parameter is not a `#[Json\Derive]` field** — findings.md § *Divergences* D7 and D8. The row is
      `crates/nvs-stdlib/src/json.rs:154` and the gaps are named in that module's own doc at
      `crates/nvs-stdlib/src/json.rs:77`. Case:
      `tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst`.
- [ ] **D35 there is no typed decode of a JSON array** — findings.md § *Divergences* D35.
      `decodeAs<array<U>>` is `E0465`; same row, `crates/nvs-stdlib/src/json.rs:154`, and the
      helper's own gap list at `crates/nvs-stdlib/src/json.rs:832`. Case:
      `tests/conformance/core/json-decodes-a-typed-list.nvst`.

## Backlog

- D1 the time types do not implement `Comparable` — findings.md § *Divergences*, item 34.
- D12 `spawn script … with(args:)` has no reader — findings.md § *Divergences*, item 34.
- M1 `Core\Task::afterResponse` is unimplemented — findings.md § *Missing*, item 34.
- D10 `#[Api]` fields checked but absent from the emitted document — findings.md, item 34.
- Stage 9's expression `catch`, items 21–23 — `docs/agent/loop-goal.md` § *Stage 9*.
- Item 35, the cards that cite an ADR — `docs/reference/findings.md` § *Triage*.
