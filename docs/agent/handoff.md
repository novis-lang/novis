# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 32 — the missing refusals — is closed.** Every finding in its triage row is ticked and every
case its `[[check]]` block names is written and green. P9 and P11 landed this session; P2, P3, P7 and
P8 were already fixed in the tree and only the doc was stale.

**A class named through a value is `E0496` at all three spellings.** `new $c()`, `$c::f()` and
`$x instanceof $c` share one report — `nvs_types::expr::members::reject_dynamic_class_name`, whose own
doc comment is the home of why one code covers three sites — so none of them reaches `nvs-ir`. An
enum case as an array key is `E0434` beside the `float`/`bool`/`null` keys, at all three sites that
write a key, and `$case as int` is the spelling for the number.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet, so `brief.py` still
reports it as the next free `E01xx`. The next free `E02xx` is `E0247`, the next `E07xx` is `E0782`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1042, six of eight
named cases written.

**`orient.py`'s `[context]` gaps, unchanged from the last session and still costing time.** No field
names `docs/adr/README.md` § *Decisions taken at project start*; `[context] modules` names neither
`crates/nvs-syntax/src/parser/decl.rs` nor `crates/nvs-types/src/consts.rs`; and a refusal's two
tables — `docs/reference/tools/30-php-differences.md` and `docs/adr/divergences.md` — are named by no
field at all. Four dead `modules` patterns remain: `crates/nvs-stdlib/src/capability.rs` (it is
`crates/nvs-config/src/capability.rs`), `crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs`
and `crates/nvs-types/src/literals.rs`.

## Next group

**Item 33's first three — a modifier parsed and not enforced, three times.** They share
`crates/nvs-types/`'s declaration side (`signatures.rs`, `conformance.rs`, `expr/calls.rs`) plus
`tests/conformance/reject/`, and each has its case name in `loop-goal.toml`'s item-33 `[[check]]`.

- [ ] **U1 `readonly`** — a write outside the constructor is refused (ADR 0038 § 1's contract).
      `crates/nvs-types/src/signatures.rs:1222` is the flag's only consumer today. Case:
      `tests/conformance/reject/readonly-is-written-once.nvst`.
- [ ] **U2 `final`** — extending a `final` class or overriding a `final` method is refused. PHP's own
      rule, priority 2, and no ADR is needed for it; the class-graph walk to hook is
      `crates/nvs-types/src/conformance.rs:57`. Case:
      `tests/conformance/reject/final-is-not-extended.nvst`.
- [ ] **U3 `abstract`** — `new` on an abstract class, a bodiless method in a non-abstract class, and a
      concrete class leaving an abstract method unimplemented, through the same `E0449` machinery at
      `crates/nvs-types/src/conformance.rs:57`; the `new` half sits in `check_new_target`,
      `crates/nvs-types/src/expr/calls.rs:1008`. Case:
      `tests/conformance/reject/abstract-is-not-instantiated.nvst`.

## Backlog

- Item 31's triage row still shows P15, D34, D5 and D25 unticked though the row is called closed — one
  scratch probe each says which half is stale (docs/reference/findings.md).
- Item 34's lowering gaps: P1's first-class-callable arm (crates/nvs-ir/src/lower/expr.rs:2874).
- Stage 9's items 21–23 — ADR 0119's expression `catch`, anchors already written (loop-goal.md).
- Stage 8: conformance 1042 against its 1050 floor, two named cases left (loop-goal.toml).
- `[context]` gaps above are fixed in docs/agent/loop-goal.toml, not by a session re-deriving them.
