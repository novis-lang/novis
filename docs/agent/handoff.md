# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: eleven of its seventeen findings are ticked,
D1 closed this session. The item-34 `[[check]]`'s first unwritten case is now its **eleventh**,
`tests/conformance/core/json-decodes-a-typed-list.nvst` (finding D35) — none of the four cases after
the tenth exists on disk, so each of them is one acceptance failure in the check's own order.

**A `Core` class satisfies an interface by carrying its member, and that is now a readable rule
rather than a comment.** `nvs_stdlib::registry::implements_comparable` asks the `compareTo` row —
an instance member taking one argument of this same class and answering `int`, which is exactly ADR
0013's signature — so there is no roster to keep in step with the rows; `nvs_types::core_lib` seeds
it as a `ClassSignature::implements` beside the `Iterable` one, and
`nvs_types::expr::operators::reaches_comparable` reads **two** tables because a `Core` class has no
`ClassGraph` entry at all. `Duration`, `Instant`, `Date`, `TimeOfDay` and `Core\Uri` order now.
`crates/nvs-stdlib/src/time.rs`'s module doc item 4 owns why an interface is satisfied by member.

**The comparison *is* that member, so for a `Core` class it lowers to a `CoreCall`, not a
`CallVirtual`** — a helper symbol has no entry in any compiled method table. The ownership rule
inverts with the branch: a `Core` member borrows both operands, so an operand read out of a binding
is retained by nobody and a freshly built one is this frame's temporary. `tools/leak-check.sh` is
green over a fixture comparing two fresh instants.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1067,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs; **0013 §§ 2-4** is this session's proven one, and **0046 §§ 2, 4-5**, **0053 §§ 1-3**,
**0007 §§ 2-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**
and **0096 §§ 1-1a** are the earlier ones. `[context] modules` misses every file this session edited
— `crates/nvs-stdlib/src/registry.rs`, `time.rs` is present but `crates/nvs-ir/src/lower/operator.rs`
and `crates/nvs-types/src/core_lib.rs` are not — and still misses
`crates/nvs-types/src/retrieval.rs`, `defaults.rs`, `consts.rs`, `expr/members.rs`, `generics.rs`,
`expr/assign.rs`, `expr/iteration.rs`, `expr/args.rs`, `check.rs`, `returns.rs`, `attributes.rs`,
`derive.rs`, `routes.rs`, `commands.rs`, `testing.rs`, `crates/nvs-stdlib/src/arr.rs`,
`serialize.rs`, `secret.rs`, `crates/nvs-hir/src/errors.rs`, `members.rs`,
`crates/nvs-ir/src/lower/generator.rs`, `closure.rs` and `call.rs`. `orient.py` itself still warns
that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**The item-34 `[[check]]`'s next two cases are one question — a typed decode reaching a shape the
folder cannot build — and share `crates/nvs-stdlib/src/json.rs` with the `#[Json\Derive]` half in
`crates/nvs-types/src/derive.rs`.**

- [ ] **D35 there is no typed decode of a JSON array** — `docs/reference/findings.md:300`.
      `Core\Json::decodeAs<array<U>>` is `E0465` where the type argument is checked,
      `crates/nvs-types/src/expr/args.rs:1131`; the registry row is
      `crates/nvs-stdlib/src/json.rs:154` and the body it reaches is
      `crates/nvs-stdlib/src/json.rs:896`. Case:
      `tests/conformance/core/json-decodes-a-typed-list.nvst`, named at
      `docs/agent/loop-goal.toml:338`.
- [ ] **D7/D8 `decodeAs` FATALs on a nested-class field, and a promoted constructor parameter is not
      a `#[Json\Derive]` field** — `docs/reference/findings.md:219`. The refusal is
      `crates/nvs-stdlib/src/json.rs:921`, the per-property classification that drops the promoted
      parameter is `crates/nvs-types/src/derive.rs:514` and the codec it records is
      `crates/nvs-types/src/derive.rs:377`. Case:
      `tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst`, named at
      `docs/agent/loop-goal.toml:339`.

## Backlog

- The rest of item 34 — P1, P4, D10, D12, D22, D23, D27, D33, U21, M1 — `docs/reference/findings.md`.
- Item 35, the cards that cite an ADR — `docs/agent/loop-goal.toml`'s `-p nvs-stdlib` check.
- `Core\Uri` gained `Comparable` by the same member rule and no case pins it — findings.md § D1.
- Stage 9: ADR 0119's items 21–23, anchored already — `docs/agent/loop-goal.md`.
- Stage 8: differential 206 of 210, six of eight written — `docs/implementation-plan.md`.
