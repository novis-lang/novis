# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: ten of its seventeen findings are ticked,
D21 closed this session. The item-34 `[[check]]`'s first unwritten case is now its **tenth**,
`tests/conformance/core/the-time-types-are-comparable.nvst` (finding D1) — none of the five cases
after the ninth exists on disk, so each of them is one acceptance failure in the check's own order.

**An attribute payload is folded through the scope it was *written* in, and that is the whole of
D21.** `nvs_types::retrieval`'s `AttributeTable` now carries a `Scope` — namespace, `use` table,
enclosing class — per class declaration, and every `Site` indexes one; `site_ctx` turns it into the
`Ctx` that both types the payload (`matching`) and folds it (`fold_payload`), so what a name meant
when it was checked and what it folds to cannot come apart. The fold itself is
`crate::defaults::fold_const_reference`, which reads `signatures::resolve_const` — the same entry a
*read* of `Foo::CONST` inlines — where its sibling `const_reference_default` must read
`crate::consts`, because that one runs before signatures exist. **`E0731` is now only the residue**:
a class constant whose own declaration folds to nothing (`E0792`'s gap, one position along).

**A payload's diagnostics are emitted once per retrieval that reaches its site**, because `matching`
re-types every candidate site for every retrieval. Pre-existing and not a new cost, but it is why
`tests/conformance/reject/an-attribute-retrieval-is-refused-where-it-cannot-be-folded.nvst` attaches
its unfoldable payload to a class of its own: a second retrieval there would be a second copy of both
diagnostics rather than a second mistake.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1066,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs; **0046 §§ 2, 4-5** is this session's proven one, and **0053 §§ 1-3**, **0007 §§ 2-3**,
**0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6** and
**0096 §§ 1-1a** are the earlier ones. `[context] modules` misses every file this session edited —
`crates/nvs-types/src/retrieval.rs`, `defaults.rs`, `consts.rs` and `expr/members.rs` — and still
misses `crates/nvs-types/src/generics.rs`, `expr/assign.rs`, `expr/iteration.rs`, `expr/args.rs`,
`crates/nvs-stdlib/src/objset.rs`, `crates/nvs-ir/src/lower/generator.rs`,
`crates/nvs-hir/src/errors.rs` (spec § 10's tree), `crates/nvs-stdlib/src/arr.rs`,
`crates/nvs-hir/src/members.rs`, `crates/nvs-types/src/attributes.rs`, `routes.rs`, `commands.rs`,
`derive.rs`, `testing.rs`, `crates/nvs-stdlib/src/serialize.rs` and `secret.rs`,
`crates/nvs-types/src/check.rs`, `returns.rs`, and `crates/nvs-ir/src/lower/closure.rs` and
`call.rs`. `orient.py` itself still warns that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**The item-34 `[[check]]`'s next three cases, in its own order. The first stands alone on the
ordering path; the two after it are one question — a typed decode reaching a shape the folder cannot
build — and share `crates/nvs-stdlib/src/json.rs` with the `#[Json\Derive]` half in
`crates/nvs-types/src/derive.rs`.**

- [ ] **D1 the time types do not implement `Comparable`, so `<` on two of them is `E0411`** —
      findings.md § *Divergences* D1. The rule that `Comparable` is satisfied by *member* and not by a
      declared `implements` is that module's own doc at `crates/nvs-stdlib/src/time.rs:87`, and the
      check that reports `E0411` is `crates/nvs-types/src/expr/operators.rs:387`. Case:
      `tests/conformance/core/the-time-types-are-comparable.nvst`, named at
      `docs/agent/loop-goal.toml:337`.
- [ ] **D35 there is no typed decode of a JSON array** — findings.md § *Divergences* D35.
      `decodeAs<array<U>>` is `E0465`; the row is `crates/nvs-stdlib/src/json.rs:154` and the helper's
      own gap list is `crates/nvs-stdlib/src/json.rs:832`. Case:
      `tests/conformance/core/json-decodes-a-typed-list.nvst`.
- [ ] **D7/D8 `Core\Json::decodeAs` FATALs on a nested-class field, and a promoted constructor
      parameter is not a `#[Json\Derive]` field** — findings.md § *Divergences* D7 and D8. Same row,
      `crates/nvs-stdlib/src/json.rs:154`, and the gaps are named in that module's own doc at
      `crates/nvs-stdlib/src/json.rs:77`. Case:
      `tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst`.

## Backlog

- Item 34's last two cases after the group above: `script-args-are-read.nvst` and
  `task-after-response-runs.nvst`, both named in `docs/agent/loop-goal.toml`'s item-34 `[[check]]`.
- Item 35 — no reference card cites an ADR — `docs/reference/findings.md` § *Triage*.
- Stage 9's items 21–23, ADR 0119's lowering, once stage 0c is green.
- `matching` re-types a candidate payload once per retrieval; a memo keyed on the site would make a
  payload's own diagnostics report once. `crates/nvs-types/src/retrieval.rs`.
- ADR 0046 § 5's fold and `crate::consts` still disagree about arrays — a property default of
  `Limits::ROWS` is not foldable where a payload's now is. `crates/nvs-types/src/defaults.rs`.
