# Handoff

## State

**Goal 63 — a `Core` class is a name a type test can walk — has Stage 2 landed, end to end for
`instanceof`.** `$v instanceof Core\Time\Date` compiles, answers at run time and narrows the subject on
the true edge, so a `mixed` holding a decoded `Core` instance is usable by naming its class
(`tests/conformance/lang/instanceof-answers-for-a-core-class.nvst`).

What moved, in one sentence each. `nvs_stdlib::instance::class_descriptors`
(`crates/nvs-stdlib/src/instance.rs:502`) now publishes one `(name, address)` pair per registered class
with instances rather than `Core\Html\Markup` alone, and `nvs_stdlib::class_has_instances` is that same
skip asked by name. `nvs_types::expr::members::testable_core_class`
(`crates/nvs-types/src/expr/members.rs:414`) is the checker's roster — a registered class with instances,
or a namespaced `nvs_hir::errors::TREE` entry such as `Core\Db\DbError`, which was refused before this
and is an ordinary exception class. `nvs_codegen`'s `emit_instanceof`
(`crates/nvs-codegen/src/emit.rs:2667`) accepts a label the unit does not declare when the process
publishes it, resolving it as the import a folded `` html`…` `` constant already used.

**Still refused, on purpose**: `mixed as Core\Time\Date` (`E0711`, Stage 3), the dynamic right-hand side,
an enum, and a `Core` **namespace** class — one declaring neither a slot nor an instance member, which
keeps `E0496` under new wording. `crates/nvs-runtime/src/graph.rs`'s gap still stands: it closes with
Stage 4, when the three spellings agree.

## Next group

**Stage 3: the conversion target** — one file set: `crates/nvs-types/src/expr/operators.rs`,
`crates/nvs-ir/src/lower/convert.rs`, `crates/nvs-codegen/src/emit.rs`.

- [ ] **`reject_unconvertible` stops refusing a `Core` class target** —
      `crates/nvs-types/src/expr/operators.rs:1847`'s `is_core()` arm sends every class but
      `Core\Html\Markup` to `reject_untestable_object_target`; it keeps `object`, a shape and a
      `callable` (`crates/nvs-types/src/expr/operators.rs:1841`) and admits a class
      `crate::expr::testable_core_class` answers for. `rule:types/conversion`'s `mixed`-to-class row is
      what the target then has to satisfy, and `rule:core-classes/html-auto-escape` keeps `Markup` a
      lift.
- [ ] **The downcast resolves a descriptor the unit does not own** —
      `crates/nvs-codegen/src/emit.rs:2378`'s `class_desc_const` refuses a label with no row in
      `self.classes`, which is the one thing standing between `lower_checked_downcast`
      (`crates/nvs-ir/src/lower/convert.rs:1171`) and a `Core` target. `emit_instanceof`
      (`crates/nvs-codegen/src/emit.rs:2667`) is the shape to follow: fall through to the published
      roster rather than declaring a second one. `E_UNTESTABLE_CONVERSION_TARGET`'s doc comment
      (`crates/nvs-diagnostics/src/lib.rs:2199`) states the current split and is rewritten here.
- [ ] **A conformance case asks all three spellings over one class** — `is`, `instanceof` and `as` over
      `Core\Time\Date` from a `mixed`, beside
      `tests/conformance/lang/instanceof-answers-for-a-core-class.nvst:13`. Stage 4's agreement check,
      and cheap once the row above exists: `rule:types/type-test` owns what `is` must already answer,
      and `crates/nvs-ir/src/lower/expr.rs:6193`'s `test_shape` is where a class row would go.

## Backlog

- `crates/nvs-runtime/src/graph.rs:72-86`'s `# Known gaps` block is rewritten as what the round trip does,
  at Stage 4 — `docs/agent/loop-goal.md` § *Stage 0* lists it with the two other catch-up sentences.
- `crates/nvs-ir/src/lib.rs:558-562`'s roster still names a `Core` class among the targets naming no
  class to test against; true until Stage 3 lands, stale the moment it does.
- `python tools/gaps.py` ranks what else the `Core` instance classes owe in cases per member.
